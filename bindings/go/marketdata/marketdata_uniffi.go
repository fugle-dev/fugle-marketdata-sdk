package marketdata_uniffi

// #include <marketdata_uniffi.h>
import "C"

import (
	"bytes"
	"encoding/binary"
	"fmt"
	"io"
	"math"
	"runtime"
	"runtime/cgo"
	"sync"
	"sync/atomic"
	"unsafe"
)

// This is needed, because as of go 1.24
// type RustBuffer C.RustBuffer cannot have methods,
// RustBuffer is treated as non-local type
type GoRustBuffer struct {
	inner C.RustBuffer
}

type RustBufferI interface {
	AsReader() *bytes.Reader
	Free()
	ToGoBytes() []byte
	Data() unsafe.Pointer
	Len() uint64
	Capacity() uint64
}

// C.RustBuffer fields exposed as an interface so they can be accessed in different Go packages.
// See https://github.com/golang/go/issues/13467
type ExternalCRustBuffer interface {
	Data() unsafe.Pointer
	Len() uint64
	Capacity() uint64
}

func RustBufferFromC(b C.RustBuffer) ExternalCRustBuffer {
	return GoRustBuffer{
		inner: b,
	}
}

func CFromRustBuffer(b ExternalCRustBuffer) C.RustBuffer {
	return C.RustBuffer{
		capacity: C.uint64_t(b.Capacity()),
		len:      C.uint64_t(b.Len()),
		data:     (*C.uchar)(b.Data()),
	}
}

func RustBufferFromExternal(b ExternalCRustBuffer) GoRustBuffer {
	return GoRustBuffer{
		inner: C.RustBuffer{
			capacity: C.uint64_t(b.Capacity()),
			len:      C.uint64_t(b.Len()),
			data:     (*C.uchar)(b.Data()),
		},
	}
}

func (cb GoRustBuffer) Capacity() uint64 {
	return uint64(cb.inner.capacity)
}

func (cb GoRustBuffer) Len() uint64 {
	return uint64(cb.inner.len)
}

func (cb GoRustBuffer) Data() unsafe.Pointer {
	return unsafe.Pointer(cb.inner.data)
}

func (cb GoRustBuffer) AsReader() *bytes.Reader {
	b := unsafe.Slice((*byte)(cb.inner.data), C.uint64_t(cb.inner.len))
	return bytes.NewReader(b)
}

func (cb GoRustBuffer) Free() {
	rustCall(func(status *C.RustCallStatus) bool {
		C.ffi_marketdata_uniffi_rustbuffer_free(cb.inner, status)
		return false
	})
}

func (cb GoRustBuffer) ToGoBytes() []byte {
	return C.GoBytes(unsafe.Pointer(cb.inner.data), C.int(cb.inner.len))
}

func stringToRustBuffer(str string) C.RustBuffer {
	return bytesToRustBuffer([]byte(str))
}

func bytesToRustBuffer(b []byte) C.RustBuffer {
	if len(b) == 0 {
		return C.RustBuffer{}
	}
	// We can pass the pointer along here, as it is pinned
	// for the duration of this call
	foreign := C.ForeignBytes{
		len:  C.int(len(b)),
		data: (*C.uchar)(unsafe.Pointer(&b[0])),
	}

	return rustCall(func(status *C.RustCallStatus) C.RustBuffer {
		return C.ffi_marketdata_uniffi_rustbuffer_from_bytes(foreign, status)
	})
}

type BufLifter[GoType any] interface {
	Lift(value RustBufferI) GoType
}

type BufLowerer[GoType any] interface {
	Lower(value GoType) C.RustBuffer
}

type BufReader[GoType any] interface {
	Read(reader io.Reader) GoType
}

type BufWriter[GoType any] interface {
	Write(writer io.Writer, value GoType)
}

func LowerIntoRustBuffer[GoType any](bufWriter BufWriter[GoType], value GoType) C.RustBuffer {
	// This might be not the most efficient way but it does not require knowing allocation size
	// beforehand
	var buffer bytes.Buffer
	bufWriter.Write(&buffer, value)

	bytes, err := io.ReadAll(&buffer)
	if err != nil {
		panic(fmt.Errorf("reading written data: %w", err))
	}
	return bytesToRustBuffer(bytes)
}

func LiftFromRustBuffer[GoType any](bufReader BufReader[GoType], rbuf RustBufferI) GoType {
	defer rbuf.Free()
	reader := rbuf.AsReader()
	item := bufReader.Read(reader)
	if reader.Len() > 0 {
		// TODO: Remove this
		leftover, _ := io.ReadAll(reader)
		panic(fmt.Errorf("Junk remaining in buffer after lifting: %s", string(leftover)))
	}
	return item
}

func rustCallWithError[E any, U any](converter BufReader[*E], callback func(*C.RustCallStatus) U) (U, *E) {
	var status C.RustCallStatus
	returnValue := callback(&status)
	err := checkCallStatus(converter, status)
	return returnValue, err
}

func checkCallStatus[E any](converter BufReader[*E], status C.RustCallStatus) *E {
	switch status.code {
	case 0:
		return nil
	case 1:
		return LiftFromRustBuffer(converter, GoRustBuffer{inner: status.errorBuf})
	case 2:
		// when the rust code sees a panic, it tries to construct a rustBuffer
		// with the message.  but if that code panics, then it just sends back
		// an empty buffer.
		if status.errorBuf.len > 0 {
			panic(fmt.Errorf("%s", FfiConverterStringINSTANCE.Lift(GoRustBuffer{inner: status.errorBuf})))
		} else {
			panic(fmt.Errorf("Rust panicked while handling Rust panic"))
		}
	default:
		panic(fmt.Errorf("unknown status code: %d", status.code))
	}
}

func checkCallStatusUnknown(status C.RustCallStatus) error {
	switch status.code {
	case 0:
		return nil
	case 1:
		panic(fmt.Errorf("function not returning an error returned an error"))
	case 2:
		// when the rust code sees a panic, it tries to construct a C.RustBuffer
		// with the message.  but if that code panics, then it just sends back
		// an empty buffer.
		if status.errorBuf.len > 0 {
			panic(fmt.Errorf("%s", FfiConverterStringINSTANCE.Lift(GoRustBuffer{
				inner: status.errorBuf,
			})))
		} else {
			panic(fmt.Errorf("Rust panicked while handling Rust panic"))
		}
	default:
		return fmt.Errorf("unknown status code: %d", status.code)
	}
}

func rustCall[U any](callback func(*C.RustCallStatus) U) U {
	returnValue, err := rustCallWithError[error](nil, callback)
	if err != nil {
		panic(err)
	}
	return returnValue
}

type NativeError interface {
	AsError() error
}

func writeInt8(writer io.Writer, value int8) {
	if err := binary.Write(writer, binary.BigEndian, value); err != nil {
		panic(err)
	}
}

func writeUint8(writer io.Writer, value uint8) {
	if err := binary.Write(writer, binary.BigEndian, value); err != nil {
		panic(err)
	}
}

func writeInt16(writer io.Writer, value int16) {
	if err := binary.Write(writer, binary.BigEndian, value); err != nil {
		panic(err)
	}
}

func writeUint16(writer io.Writer, value uint16) {
	if err := binary.Write(writer, binary.BigEndian, value); err != nil {
		panic(err)
	}
}

func writeInt32(writer io.Writer, value int32) {
	if err := binary.Write(writer, binary.BigEndian, value); err != nil {
		panic(err)
	}
}

func writeUint32(writer io.Writer, value uint32) {
	if err := binary.Write(writer, binary.BigEndian, value); err != nil {
		panic(err)
	}
}

func writeInt64(writer io.Writer, value int64) {
	if err := binary.Write(writer, binary.BigEndian, value); err != nil {
		panic(err)
	}
}

func writeUint64(writer io.Writer, value uint64) {
	if err := binary.Write(writer, binary.BigEndian, value); err != nil {
		panic(err)
	}
}

func writeFloat32(writer io.Writer, value float32) {
	if err := binary.Write(writer, binary.BigEndian, value); err != nil {
		panic(err)
	}
}

func writeFloat64(writer io.Writer, value float64) {
	if err := binary.Write(writer, binary.BigEndian, value); err != nil {
		panic(err)
	}
}

func readInt8(reader io.Reader) int8 {
	var result int8
	if err := binary.Read(reader, binary.BigEndian, &result); err != nil {
		panic(err)
	}
	return result
}

func readUint8(reader io.Reader) uint8 {
	var result uint8
	if err := binary.Read(reader, binary.BigEndian, &result); err != nil {
		panic(err)
	}
	return result
}

func readInt16(reader io.Reader) int16 {
	var result int16
	if err := binary.Read(reader, binary.BigEndian, &result); err != nil {
		panic(err)
	}
	return result
}

func readUint16(reader io.Reader) uint16 {
	var result uint16
	if err := binary.Read(reader, binary.BigEndian, &result); err != nil {
		panic(err)
	}
	return result
}

func readInt32(reader io.Reader) int32 {
	var result int32
	if err := binary.Read(reader, binary.BigEndian, &result); err != nil {
		panic(err)
	}
	return result
}

func readUint32(reader io.Reader) uint32 {
	var result uint32
	if err := binary.Read(reader, binary.BigEndian, &result); err != nil {
		panic(err)
	}
	return result
}

func readInt64(reader io.Reader) int64 {
	var result int64
	if err := binary.Read(reader, binary.BigEndian, &result); err != nil {
		panic(err)
	}
	return result
}

func readUint64(reader io.Reader) uint64 {
	var result uint64
	if err := binary.Read(reader, binary.BigEndian, &result); err != nil {
		panic(err)
	}
	return result
}

func readFloat32(reader io.Reader) float32 {
	var result float32
	if err := binary.Read(reader, binary.BigEndian, &result); err != nil {
		panic(err)
	}
	return result
}

func readFloat64(reader io.Reader) float64 {
	var result float64
	if err := binary.Read(reader, binary.BigEndian, &result); err != nil {
		panic(err)
	}
	return result
}

func init() {

	FfiConverterWebSocketListenerINSTANCE.register()
	uniffiCheckChecksums()
}

func uniffiCheckChecksums() {
	// Get the bindings contract version from our ComponentInterface
	bindingsContractVersion := 29
	// Get the scaffolding contract version by calling the into the dylib
	scaffoldingContractVersion := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint32_t {
		return C.ffi_marketdata_uniffi_uniffi_contract_version()
	})
	if bindingsContractVersion != int(scaffoldingContractVersion) {
		// If this happens try cleaning and rebuilding your project
		panic("marketdata_uniffi: UniFFI contract version mismatch")
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_func_new_rest_client_with_api_key()
		})
		if checksum != 2560 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_func_new_rest_client_with_api_key: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_func_new_rest_client_with_api_key_and_tls()
		})
		if checksum != 17616 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_func_new_rest_client_with_api_key_and_tls: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_func_new_rest_client_with_bearer_token()
		})
		if checksum != 30582 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_func_new_rest_client_with_bearer_token: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_func_new_rest_client_with_bearer_token_and_tls()
		})
		if checksum != 21309 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_func_new_rest_client_with_bearer_token_and_tls: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_func_new_rest_client_with_sdk_token()
		})
		if checksum != 14209 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_func_new_rest_client_with_sdk_token: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_func_new_rest_client_with_sdk_token_and_tls()
		})
		if checksum != 25673 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_func_new_rest_client_with_sdk_token_and_tls: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_func_new_websocket_client()
		})
		if checksum != 17568 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_func_new_websocket_client: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_func_new_websocket_client_with_config()
		})
		if checksum != 19180 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_func_new_websocket_client_with_config: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_func_new_websocket_client_with_endpoint()
		})
		if checksum != 15148 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_func_new_websocket_client_with_endpoint: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_func_validate_credentials()
		})
		if checksum != 23718 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_func_validate_credentials: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_futoptclient_historical()
		})
		if checksum != 18194 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_futoptclient_historical: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_futoptclient_intraday()
		})
		if checksum != 43120 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_futoptclient_intraday: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_futopthistoricalclient_candles_sync()
		})
		if checksum != 8321 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_futopthistoricalclient_candles_sync: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_futopthistoricalclient_daily_sync()
		})
		if checksum != 43568 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_futopthistoricalclient_daily_sync: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_futopthistoricalclient_get_candles()
		})
		if checksum != 6749 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_futopthistoricalclient_get_candles: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_futopthistoricalclient_get_daily()
		})
		if checksum != 61915 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_futopthistoricalclient_get_daily: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_candles_sync()
		})
		if checksum != 15435 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_candles_sync: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_get_candles()
		})
		if checksum != 18846 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_get_candles: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_get_products()
		})
		if checksum != 28718 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_get_products: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_get_quote()
		})
		if checksum != 60925 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_get_quote: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_get_ticker()
		})
		if checksum != 6755 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_get_ticker: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_get_tickers()
		})
		if checksum != 2210 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_get_tickers: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_get_trades()
		})
		if checksum != 18238 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_get_trades: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_get_volumes()
		})
		if checksum != 46935 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_get_volumes: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_products_sync()
		})
		if checksum != 21712 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_products_sync: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_quote_sync()
		})
		if checksum != 49258 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_quote_sync: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_ticker_sync()
		})
		if checksum != 25885 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_ticker_sync: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_tickers_sync()
		})
		if checksum != 32515 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_tickers_sync: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_trades_sync()
		})
		if checksum != 4933 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_trades_sync: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_volumes_sync()
		})
		if checksum != 40116 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_volumes_sync: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_restclient_base_url()
		})
		if checksum != 36384 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_restclient_base_url: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_restclient_futopt()
		})
		if checksum != 65348 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_restclient_futopt: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_restclient_stock()
		})
		if checksum != 18733 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_restclient_stock: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stockclient_base_url()
		})
		if checksum != 28231 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stockclient_base_url: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stockclient_corporate_actions()
		})
		if checksum != 38783 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stockclient_corporate_actions: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stockclient_historical()
		})
		if checksum != 45578 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stockclient_historical: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stockclient_intraday()
		})
		if checksum != 53228 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stockclient_intraday: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stockclient_ownership()
		})
		if checksum != 26642 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stockclient_ownership: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stockclient_snapshot()
		})
		if checksum != 49856 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stockclient_snapshot: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stockclient_technical()
		})
		if checksum != 10974 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stockclient_technical: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stockcorporateactionsclient_capital_changes_sync()
		})
		if checksum != 31988 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stockcorporateactionsclient_capital_changes_sync: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stockcorporateactionsclient_dividends_sync()
		})
		if checksum != 37175 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stockcorporateactionsclient_dividends_sync: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stockcorporateactionsclient_get_capital_changes()
		})
		if checksum != 22794 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stockcorporateactionsclient_get_capital_changes: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stockcorporateactionsclient_get_dividends()
		})
		if checksum != 1657 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stockcorporateactionsclient_get_dividends: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stockcorporateactionsclient_get_listing_applicants()
		})
		if checksum != 1735 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stockcorporateactionsclient_get_listing_applicants: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stockcorporateactionsclient_listing_applicants_sync()
		})
		if checksum != 4098 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stockcorporateactionsclient_listing_applicants_sync: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stockhistoricalclient_candles_sync()
		})
		if checksum != 16718 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stockhistoricalclient_candles_sync: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stockhistoricalclient_get_candles()
		})
		if checksum != 30527 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stockhistoricalclient_get_candles: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stockhistoricalclient_get_stats()
		})
		if checksum != 37563 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stockhistoricalclient_get_stats: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stockhistoricalclient_stats_sync()
		})
		if checksum != 20776 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stockhistoricalclient_stats_sync: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stockintradayclient_candles_sync()
		})
		if checksum != 43276 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stockintradayclient_candles_sync: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stockintradayclient_get_candles()
		})
		if checksum != 27303 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stockintradayclient_get_candles: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stockintradayclient_get_quote()
		})
		if checksum != 4800 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stockintradayclient_get_quote: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stockintradayclient_get_ticker()
		})
		if checksum != 11469 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stockintradayclient_get_ticker: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stockintradayclient_get_tickers()
		})
		if checksum != 25432 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stockintradayclient_get_tickers: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stockintradayclient_get_trades()
		})
		if checksum != 49659 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stockintradayclient_get_trades: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stockintradayclient_get_volumes()
		})
		if checksum != 7081 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stockintradayclient_get_volumes: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stockintradayclient_quote_sync()
		})
		if checksum != 14450 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stockintradayclient_quote_sync: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stockintradayclient_ticker_sync()
		})
		if checksum != 12061 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stockintradayclient_ticker_sync: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stockintradayclient_tickers_sync()
		})
		if checksum != 50959 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stockintradayclient_tickers_sync: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stockintradayclient_trades_sync()
		})
		if checksum != 17544 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stockintradayclient_trades_sync: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stockintradayclient_volumes_sync()
		})
		if checksum != 53696 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stockintradayclient_volumes_sync: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stockownershipclient_director_holdings_sync()
		})
		if checksum != 42397 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stockownershipclient_director_holdings_sync: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stockownershipclient_etf_holdings_sync()
		})
		if checksum != 9047 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stockownershipclient_etf_holdings_sync: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stockownershipclient_get_director_holdings()
		})
		if checksum != 9436 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stockownershipclient_get_director_holdings: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stockownershipclient_get_etf_holdings()
		})
		if checksum != 32666 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stockownershipclient_get_etf_holdings: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stockownershipclient_get_institutional_trades()
		})
		if checksum != 44140 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stockownershipclient_get_institutional_trades: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stockownershipclient_get_tdcc_distribution()
		})
		if checksum != 55570 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stockownershipclient_get_tdcc_distribution: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stockownershipclient_institutional_trades_sync()
		})
		if checksum != 27652 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stockownershipclient_institutional_trades_sync: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stockownershipclient_tdcc_distribution_sync()
		})
		if checksum != 41522 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stockownershipclient_tdcc_distribution_sync: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stocksnapshotclient_actives_sync()
		})
		if checksum != 34682 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stocksnapshotclient_actives_sync: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stocksnapshotclient_get_actives()
		})
		if checksum != 54146 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stocksnapshotclient_get_actives: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stocksnapshotclient_get_movers()
		})
		if checksum != 19121 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stocksnapshotclient_get_movers: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stocksnapshotclient_get_quotes()
		})
		if checksum != 18220 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stocksnapshotclient_get_quotes: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stocksnapshotclient_movers_sync()
		})
		if checksum != 58532 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stocksnapshotclient_movers_sync: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stocksnapshotclient_quotes_sync()
		})
		if checksum != 8259 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stocksnapshotclient_quotes_sync: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_bb_sync()
		})
		if checksum != 60077 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_bb_sync: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_get_bb()
		})
		if checksum != 16142 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_get_bb: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_get_kdj()
		})
		if checksum != 15872 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_get_kdj: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_get_macd()
		})
		if checksum != 61798 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_get_macd: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_get_rsi()
		})
		if checksum != 3410 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_get_rsi: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_get_sma()
		})
		if checksum != 28284 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_get_sma: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_kdj_sync()
		})
		if checksum != 7023 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_kdj_sync: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_macd_sync()
		})
		if checksum != 3187 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_macd_sync: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_rsi_sync()
		})
		if checksum != 43008 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_rsi_sync: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_sma_sync()
		})
		if checksum != 21533 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_sma_sync: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_websocketclient_connect()
		})
		if checksum != 2768 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_websocketclient_connect: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_websocketclient_disconnect()
		})
		if checksum != 58180 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_websocketclient_disconnect: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_websocketclient_is_closed()
		})
		if checksum != 1028 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_websocketclient_is_closed: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_websocketclient_is_connected()
		})
		if checksum != 18665 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_websocketclient_is_connected: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_websocketclient_measure_latency()
		})
		if checksum != 53522 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_websocketclient_measure_latency: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_websocketclient_messages_dropped_total()
		})
		if checksum != 28793 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_websocketclient_messages_dropped_total: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_websocketclient_ping()
		})
		if checksum != 51664 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_websocketclient_ping: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_websocketclient_query_subscriptions()
		})
		if checksum != 20069 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_websocketclient_query_subscriptions: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_websocketclient_subscribe()
		})
		if checksum != 12456 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_websocketclient_subscribe: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_websocketclient_unsubscribe()
		})
		if checksum != 6177 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_websocketclient_unsubscribe: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_websocketclient_unsubscribe_ids()
		})
		if checksum != 5738 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_websocketclient_unsubscribe_ids: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_websocketlistener_on_connected()
		})
		if checksum != 42437 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_websocketlistener_on_connected: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_websocketlistener_on_authenticated()
		})
		if checksum != 51034 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_websocketlistener_on_authenticated: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_websocketlistener_on_unauthenticated()
		})
		if checksum != 41202 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_websocketlistener_on_unauthenticated: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_websocketlistener_on_disconnected()
		})
		if checksum != 44379 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_websocketlistener_on_disconnected: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_websocketlistener_on_message()
		})
		if checksum != 4936 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_websocketlistener_on_message: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_websocketlistener_on_error()
		})
		if checksum != 8035 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_websocketlistener_on_error: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_websocketlistener_on_reconnecting()
		})
		if checksum != 12322 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_websocketlistener_on_reconnecting: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_websocketlistener_on_reconnect_failed()
		})
		if checksum != 51040 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_websocketlistener_on_reconnect_failed: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_websocketlistener_on_messages_dropped()
		})
		if checksum != 34523 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_websocketlistener_on_messages_dropped: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_constructor_websocketclient_new()
		})
		if checksum != 36225 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_constructor_websocketclient_new: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_constructor_websocketclient_new_with_config()
		})
		if checksum != 8956 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_constructor_websocketclient_new_with_config: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_constructor_websocketclient_new_with_credentials()
		})
		if checksum != 53902 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_constructor_websocketclient_new_with_credentials: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_constructor_websocketclient_new_with_endpoint()
		})
		if checksum != 35702 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_constructor_websocketclient_new_with_endpoint: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_constructor_websocketclient_new_with_full_config()
		})
		if checksum != 32798 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_constructor_websocketclient_new_with_full_config: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_constructor_websocketclient_new_with_options()
		})
		if checksum != 2558 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_constructor_websocketclient_new_with_options: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_constructor_websocketclient_new_with_url()
		})
		if checksum != 63549 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_constructor_websocketclient_new_with_url: UniFFI API checksum mismatch")
		}
	}
}

type FfiConverterUint16 struct{}

var FfiConverterUint16INSTANCE = FfiConverterUint16{}

func (FfiConverterUint16) Lower(value uint16) C.uint16_t {
	return C.uint16_t(value)
}

func (FfiConverterUint16) Write(writer io.Writer, value uint16) {
	writeUint16(writer, value)
}

func (FfiConverterUint16) Lift(value C.uint16_t) uint16 {
	return uint16(value)
}

func (FfiConverterUint16) Read(reader io.Reader) uint16 {
	return readUint16(reader)
}

type FfiDestroyerUint16 struct{}

func (FfiDestroyerUint16) Destroy(_ uint16) {}

type FfiConverterUint32 struct{}

var FfiConverterUint32INSTANCE = FfiConverterUint32{}

func (FfiConverterUint32) Lower(value uint32) C.uint32_t {
	return C.uint32_t(value)
}

func (FfiConverterUint32) Write(writer io.Writer, value uint32) {
	writeUint32(writer, value)
}

func (FfiConverterUint32) Lift(value C.uint32_t) uint32 {
	return uint32(value)
}

func (FfiConverterUint32) Read(reader io.Reader) uint32 {
	return readUint32(reader)
}

type FfiDestroyerUint32 struct{}

func (FfiDestroyerUint32) Destroy(_ uint32) {}

type FfiConverterInt32 struct{}

var FfiConverterInt32INSTANCE = FfiConverterInt32{}

func (FfiConverterInt32) Lower(value int32) C.int32_t {
	return C.int32_t(value)
}

func (FfiConverterInt32) Write(writer io.Writer, value int32) {
	writeInt32(writer, value)
}

func (FfiConverterInt32) Lift(value C.int32_t) int32 {
	return int32(value)
}

func (FfiConverterInt32) Read(reader io.Reader) int32 {
	return readInt32(reader)
}

type FfiDestroyerInt32 struct{}

func (FfiDestroyerInt32) Destroy(_ int32) {}

type FfiConverterUint64 struct{}

var FfiConverterUint64INSTANCE = FfiConverterUint64{}

func (FfiConverterUint64) Lower(value uint64) C.uint64_t {
	return C.uint64_t(value)
}

func (FfiConverterUint64) Write(writer io.Writer, value uint64) {
	writeUint64(writer, value)
}

func (FfiConverterUint64) Lift(value C.uint64_t) uint64 {
	return uint64(value)
}

func (FfiConverterUint64) Read(reader io.Reader) uint64 {
	return readUint64(reader)
}

type FfiDestroyerUint64 struct{}

func (FfiDestroyerUint64) Destroy(_ uint64) {}

type FfiConverterFloat64 struct{}

var FfiConverterFloat64INSTANCE = FfiConverterFloat64{}

func (FfiConverterFloat64) Lower(value float64) C.double {
	return C.double(value)
}

func (FfiConverterFloat64) Write(writer io.Writer, value float64) {
	writeFloat64(writer, value)
}

func (FfiConverterFloat64) Lift(value C.double) float64 {
	return float64(value)
}

func (FfiConverterFloat64) Read(reader io.Reader) float64 {
	return readFloat64(reader)
}

type FfiDestroyerFloat64 struct{}

func (FfiDestroyerFloat64) Destroy(_ float64) {}

type FfiConverterBool struct{}

var FfiConverterBoolINSTANCE = FfiConverterBool{}

func (FfiConverterBool) Lower(value bool) C.int8_t {
	if value {
		return C.int8_t(1)
	}
	return C.int8_t(0)
}

func (FfiConverterBool) Write(writer io.Writer, value bool) {
	if value {
		writeInt8(writer, 1)
	} else {
		writeInt8(writer, 0)
	}
}

func (FfiConverterBool) Lift(value C.int8_t) bool {
	return value != 0
}

func (FfiConverterBool) Read(reader io.Reader) bool {
	return readInt8(reader) != 0
}

type FfiDestroyerBool struct{}

func (FfiDestroyerBool) Destroy(_ bool) {}

type FfiConverterString struct{}

var FfiConverterStringINSTANCE = FfiConverterString{}

func (FfiConverterString) Lift(rb RustBufferI) string {
	defer rb.Free()
	reader := rb.AsReader()
	b, err := io.ReadAll(reader)
	if err != nil {
		panic(fmt.Errorf("reading reader: %w", err))
	}
	return string(b)
}

func (FfiConverterString) Read(reader io.Reader) string {
	length := readInt32(reader)
	buffer := make([]byte, length)
	read_length, err := reader.Read(buffer)
	if err != nil && err != io.EOF {
		panic(err)
	}
	if read_length != int(length) {
		panic(fmt.Errorf("bad read length when reading string, expected %d, read %d", length, read_length))
	}
	return string(buffer)
}

func (FfiConverterString) Lower(value string) C.RustBuffer {
	return stringToRustBuffer(value)
}

func (c FfiConverterString) LowerExternal(value string) ExternalCRustBuffer {
	return RustBufferFromC(stringToRustBuffer(value))
}

func (FfiConverterString) Write(writer io.Writer, value string) {
	if len(value) > math.MaxInt32 {
		panic("String is too large to fit into Int32")
	}

	writeInt32(writer, int32(len(value)))
	write_length, err := io.WriteString(writer, value)
	if err != nil {
		panic(err)
	}
	if write_length != len(value) {
		panic(fmt.Errorf("bad write length when writing string, expected %d, written %d", len(value), write_length))
	}
}

type FfiDestroyerString struct{}

func (FfiDestroyerString) Destroy(_ string) {}

type FfiConverterBytes struct{}

var FfiConverterBytesINSTANCE = FfiConverterBytes{}

func (c FfiConverterBytes) Lower(value []byte) C.RustBuffer {
	return LowerIntoRustBuffer[[]byte](c, value)
}

func (c FfiConverterBytes) LowerExternal(value []byte) ExternalCRustBuffer {
	return RustBufferFromC(c.Lower(value))
}

func (c FfiConverterBytes) Write(writer io.Writer, value []byte) {
	if len(value) > math.MaxInt32 {
		panic("[]byte is too large to fit into Int32")
	}

	writeInt32(writer, int32(len(value)))
	write_length, err := writer.Write(value)
	if err != nil {
		panic(err)
	}
	if write_length != len(value) {
		panic(fmt.Errorf("bad write length when writing []byte, expected %d, written %d", len(value), write_length))
	}
}

func (c FfiConverterBytes) Lift(rb RustBufferI) []byte {
	return LiftFromRustBuffer[[]byte](c, rb)
}

func (c FfiConverterBytes) Read(reader io.Reader) []byte {
	length := readInt32(reader)
	buffer := make([]byte, length)
	read_length, err := reader.Read(buffer)
	if err != nil && err != io.EOF {
		panic(err)
	}
	if read_length != int(length) {
		panic(fmt.Errorf("bad read length when reading []byte, expected %d, read %d", length, read_length))
	}
	return buffer
}

type FfiDestroyerBytes struct{}

func (FfiDestroyerBytes) Destroy(_ []byte) {}

// Below is an implementation of synchronization requirements outlined in the link.
// https://github.com/mozilla/uniffi-rs/blob/0dc031132d9493ca812c3af6e7dd60ad2ea95bf0/uniffi_bindgen/src/bindings/kotlin/templates/ObjectRuntime.kt#L31

type FfiObject struct {
	pointer       unsafe.Pointer
	callCounter   atomic.Int64
	cloneFunction func(unsafe.Pointer, *C.RustCallStatus) unsafe.Pointer
	freeFunction  func(unsafe.Pointer, *C.RustCallStatus)
	destroyed     atomic.Bool
}

func newFfiObject(
	pointer unsafe.Pointer,
	cloneFunction func(unsafe.Pointer, *C.RustCallStatus) unsafe.Pointer,
	freeFunction func(unsafe.Pointer, *C.RustCallStatus),
) FfiObject {
	return FfiObject{
		pointer:       pointer,
		cloneFunction: cloneFunction,
		freeFunction:  freeFunction,
	}
}

func (ffiObject *FfiObject) incrementPointer(debugName string) unsafe.Pointer {
	for {
		counter := ffiObject.callCounter.Load()
		if counter <= -1 {
			panic(fmt.Errorf("%v object has already been destroyed", debugName))
		}
		if counter == math.MaxInt64 {
			panic(fmt.Errorf("%v object call counter would overflow", debugName))
		}
		if ffiObject.callCounter.CompareAndSwap(counter, counter+1) {
			break
		}
	}

	return rustCall(func(status *C.RustCallStatus) unsafe.Pointer {
		return ffiObject.cloneFunction(ffiObject.pointer, status)
	})
}

func (ffiObject *FfiObject) decrementPointer() {
	if ffiObject.callCounter.Add(-1) == -1 {
		ffiObject.freeRustArcPtr()
	}
}

func (ffiObject *FfiObject) destroy() {
	if ffiObject.destroyed.CompareAndSwap(false, true) {
		if ffiObject.callCounter.Add(-1) == -1 {
			ffiObject.freeRustArcPtr()
		}
	}
}

func (ffiObject *FfiObject) freeRustArcPtr() {
	rustCall(func(status *C.RustCallStatus) int32 {
		ffiObject.freeFunction(ffiObject.pointer, status)
		return 0
	})
}

// FutOpt market data client
type FutOptClientInterface interface {
	// Access historical data endpoints
	Historical() *FutOptHistoricalClient
	// Access intraday (real-time) endpoints
	Intraday() *FutOptIntradayClient
}

// FutOpt market data client
type FutOptClient struct {
	ffiObject FfiObject
}

// Access historical data endpoints
func (_self *FutOptClient) Historical() *FutOptHistoricalClient {
	_pointer := _self.ffiObject.incrementPointer("*FutOptClient")
	defer _self.ffiObject.decrementPointer()
	return FfiConverterFutOptHistoricalClientINSTANCE.Lift(rustCall(func(_uniffiStatus *C.RustCallStatus) unsafe.Pointer {
		return C.uniffi_marketdata_uniffi_fn_method_futoptclient_historical(
			_pointer, _uniffiStatus)
	}))
}

// Access intraday (real-time) endpoints
func (_self *FutOptClient) Intraday() *FutOptIntradayClient {
	_pointer := _self.ffiObject.incrementPointer("*FutOptClient")
	defer _self.ffiObject.decrementPointer()
	return FfiConverterFutOptIntradayClientINSTANCE.Lift(rustCall(func(_uniffiStatus *C.RustCallStatus) unsafe.Pointer {
		return C.uniffi_marketdata_uniffi_fn_method_futoptclient_intraday(
			_pointer, _uniffiStatus)
	}))
}
func (object *FutOptClient) Destroy() {
	runtime.SetFinalizer(object, nil)
	object.ffiObject.destroy()
}

type FfiConverterFutOptClient struct{}

var FfiConverterFutOptClientINSTANCE = FfiConverterFutOptClient{}

func (c FfiConverterFutOptClient) Lift(pointer unsafe.Pointer) *FutOptClient {
	result := &FutOptClient{
		newFfiObject(
			pointer,
			func(pointer unsafe.Pointer, status *C.RustCallStatus) unsafe.Pointer {
				return C.uniffi_marketdata_uniffi_fn_clone_futoptclient(pointer, status)
			},
			func(pointer unsafe.Pointer, status *C.RustCallStatus) {
				C.uniffi_marketdata_uniffi_fn_free_futoptclient(pointer, status)
			},
		),
	}
	runtime.SetFinalizer(result, (*FutOptClient).Destroy)
	return result
}

func (c FfiConverterFutOptClient) Read(reader io.Reader) *FutOptClient {
	return c.Lift(unsafe.Pointer(uintptr(readUint64(reader))))
}

func (c FfiConverterFutOptClient) Lower(value *FutOptClient) unsafe.Pointer {
	// TODO: this is bad - all synchronization from ObjectRuntime.go is discarded here,
	// because the pointer will be decremented immediately after this function returns,
	// and someone will be left holding onto a non-locked pointer.
	pointer := value.ffiObject.incrementPointer("*FutOptClient")
	defer value.ffiObject.decrementPointer()
	return pointer

}

func (c FfiConverterFutOptClient) Write(writer io.Writer, value *FutOptClient) {
	writeUint64(writer, uint64(uintptr(c.Lower(value))))
}

type FfiDestroyerFutOptClient struct{}

func (_ FfiDestroyerFutOptClient) Destroy(value *FutOptClient) {
	value.Destroy()
}

// FutOpt historical data endpoints
//
// Provides access to historical candles and daily data for futures and options.
type FutOptHistoricalClientInterface interface {
	// Get historical candles for a product such as "TXF" (sync/blocking)
	CandlesSync(symbol string, params *FutOptHistoricalCandlesParams) (string, error)
	// Get one trading day's daily quotes for every contract month of a product such as "TXF" (sync/blocking)
	DailySync(symbol string, params *FutOptDailyParams) (string, error)
	// Get historical candles for a product such as "TXF" (async)
	GetCandles(symbol string, params *FutOptHistoricalCandlesParams) (string, error)
	// Get one trading day's daily quotes for every contract month of a product such as "TXF" (async)
	GetDaily(symbol string, params *FutOptDailyParams) (string, error)
}

// FutOpt historical data endpoints
//
// Provides access to historical candles and daily data for futures and options.
type FutOptHistoricalClient struct {
	ffiObject FfiObject
}

// Get historical candles for a product such as "TXF" (sync/blocking)
func (_self *FutOptHistoricalClient) CandlesSync(symbol string, params *FutOptHistoricalCandlesParams) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*FutOptHistoricalClient")
	defer _self.ffiObject.decrementPointer()
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_method_futopthistoricalclient_candles_sync(
				_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterOptionalFutOptHistoricalCandlesParamsINSTANCE.Lower(params), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue string
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterStringINSTANCE.Lift(_uniffiRV), nil
	}
}

// Get one trading day's daily quotes for every contract month of a product such as "TXF" (sync/blocking)
func (_self *FutOptHistoricalClient) DailySync(symbol string, params *FutOptDailyParams) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*FutOptHistoricalClient")
	defer _self.ffiObject.decrementPointer()
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_method_futopthistoricalclient_daily_sync(
				_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterOptionalFutOptDailyParamsINSTANCE.Lower(params), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue string
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterStringINSTANCE.Lift(_uniffiRV), nil
	}
}

// Get historical candles for a product such as "TXF" (async)
func (_self *FutOptHistoricalClient) GetCandles(symbol string, params *FutOptHistoricalCandlesParams) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*FutOptHistoricalClient")
	defer _self.ffiObject.decrementPointer()
	res, err := uniffiRustCallAsync[MarketDataError](
		FfiConverterMarketDataErrorINSTANCE,
		// completeFn
		func(handle C.uint64_t, status *C.RustCallStatus) RustBufferI {
			res := C.ffi_marketdata_uniffi_rust_future_complete_rust_buffer(handle, status)
			return GoRustBuffer{
				inner: res,
			}
		},
		// liftFn
		func(ffi RustBufferI) string {
			return FfiConverterStringINSTANCE.Lift(ffi)
		},
		C.uniffi_marketdata_uniffi_fn_method_futopthistoricalclient_get_candles(
			_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterOptionalFutOptHistoricalCandlesParamsINSTANCE.Lower(params)),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_rust_buffer(handle)
		},
	)

	if err == nil {
		return res, nil
	}

	return res, err
}

// Get one trading day's daily quotes for every contract month of a product such as "TXF" (async)
func (_self *FutOptHistoricalClient) GetDaily(symbol string, params *FutOptDailyParams) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*FutOptHistoricalClient")
	defer _self.ffiObject.decrementPointer()
	res, err := uniffiRustCallAsync[MarketDataError](
		FfiConverterMarketDataErrorINSTANCE,
		// completeFn
		func(handle C.uint64_t, status *C.RustCallStatus) RustBufferI {
			res := C.ffi_marketdata_uniffi_rust_future_complete_rust_buffer(handle, status)
			return GoRustBuffer{
				inner: res,
			}
		},
		// liftFn
		func(ffi RustBufferI) string {
			return FfiConverterStringINSTANCE.Lift(ffi)
		},
		C.uniffi_marketdata_uniffi_fn_method_futopthistoricalclient_get_daily(
			_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterOptionalFutOptDailyParamsINSTANCE.Lower(params)),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_rust_buffer(handle)
		},
	)

	if err == nil {
		return res, nil
	}

	return res, err
}
func (object *FutOptHistoricalClient) Destroy() {
	runtime.SetFinalizer(object, nil)
	object.ffiObject.destroy()
}

type FfiConverterFutOptHistoricalClient struct{}

var FfiConverterFutOptHistoricalClientINSTANCE = FfiConverterFutOptHistoricalClient{}

func (c FfiConverterFutOptHistoricalClient) Lift(pointer unsafe.Pointer) *FutOptHistoricalClient {
	result := &FutOptHistoricalClient{
		newFfiObject(
			pointer,
			func(pointer unsafe.Pointer, status *C.RustCallStatus) unsafe.Pointer {
				return C.uniffi_marketdata_uniffi_fn_clone_futopthistoricalclient(pointer, status)
			},
			func(pointer unsafe.Pointer, status *C.RustCallStatus) {
				C.uniffi_marketdata_uniffi_fn_free_futopthistoricalclient(pointer, status)
			},
		),
	}
	runtime.SetFinalizer(result, (*FutOptHistoricalClient).Destroy)
	return result
}

func (c FfiConverterFutOptHistoricalClient) Read(reader io.Reader) *FutOptHistoricalClient {
	return c.Lift(unsafe.Pointer(uintptr(readUint64(reader))))
}

func (c FfiConverterFutOptHistoricalClient) Lower(value *FutOptHistoricalClient) unsafe.Pointer {
	// TODO: this is bad - all synchronization from ObjectRuntime.go is discarded here,
	// because the pointer will be decremented immediately after this function returns,
	// and someone will be left holding onto a non-locked pointer.
	pointer := value.ffiObject.incrementPointer("*FutOptHistoricalClient")
	defer value.ffiObject.decrementPointer()
	return pointer

}

func (c FfiConverterFutOptHistoricalClient) Write(writer io.Writer, value *FutOptHistoricalClient) {
	writeUint64(writer, uint64(uintptr(c.Lower(value))))
}

type FfiDestroyerFutOptHistoricalClient struct{}

func (_ FfiDestroyerFutOptHistoricalClient) Destroy(value *FutOptHistoricalClient) {
	value.Destroy()
}

// FutOpt intraday endpoints
type FutOptIntradayClientInterface interface {
	// Get candlestick data for a contract (sync/blocking)
	CandlesSync(symbol string, params *FutOptCandlesParams) (string, error)
	// Get candlestick data for a futures/options contract (async)
	GetCandles(symbol string, params *FutOptCandlesParams) (string, error)
	// Get available products list (async)
	//
	// typ: "F" for futures, "O" for options
	GetProducts(typ string, params *FutOptProductsParams) (string, error)
	// Get quote for a futures/options contract (async)
	GetQuote(symbol string, params *AfterHoursParams) (string, error)
	// Get ticker info for a contract (async)
	GetTicker(symbol string, params *AfterHoursParams) (string, error)
	// Get batch tickers for futures/options (async)
	//
	// typ: "F" for futures, "O" for options
	GetTickers(typ string, params *FutOptTickersParams) (string, error)
	// Get trade history for a futures/options contract (async)
	GetTrades(symbol string, params *FutOptTradesParams) (string, error)
	// Get volume breakdown by price for a futures/options contract (async)
	GetVolumes(symbol string, params *AfterHoursParams) (string, error)
	// Get available products list (sync/blocking)
	//
	// typ: "F" for futures, "O" for options
	ProductsSync(typ string, params *FutOptProductsParams) (string, error)
	// Get quote for a futures/options contract (sync/blocking)
	QuoteSync(symbol string, params *AfterHoursParams) (string, error)
	// Get ticker info for a contract (sync/blocking)
	TickerSync(symbol string, params *AfterHoursParams) (string, error)
	// Get batch tickers for futures/options (sync/blocking)
	//
	// typ: "F" for futures, "O" for options
	TickersSync(typ string, params *FutOptTickersParams) (string, error)
	// Get trade history for a contract (sync/blocking)
	TradesSync(symbol string, params *FutOptTradesParams) (string, error)
	// Get volume breakdown by price for a contract (sync/blocking)
	VolumesSync(symbol string, params *AfterHoursParams) (string, error)
}

// FutOpt intraday endpoints
type FutOptIntradayClient struct {
	ffiObject FfiObject
}

// Get candlestick data for a contract (sync/blocking)
func (_self *FutOptIntradayClient) CandlesSync(symbol string, params *FutOptCandlesParams) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*FutOptIntradayClient")
	defer _self.ffiObject.decrementPointer()
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_method_futoptintradayclient_candles_sync(
				_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterOptionalFutOptCandlesParamsINSTANCE.Lower(params), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue string
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterStringINSTANCE.Lift(_uniffiRV), nil
	}
}

// Get candlestick data for a futures/options contract (async)
func (_self *FutOptIntradayClient) GetCandles(symbol string, params *FutOptCandlesParams) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*FutOptIntradayClient")
	defer _self.ffiObject.decrementPointer()
	res, err := uniffiRustCallAsync[MarketDataError](
		FfiConverterMarketDataErrorINSTANCE,
		// completeFn
		func(handle C.uint64_t, status *C.RustCallStatus) RustBufferI {
			res := C.ffi_marketdata_uniffi_rust_future_complete_rust_buffer(handle, status)
			return GoRustBuffer{
				inner: res,
			}
		},
		// liftFn
		func(ffi RustBufferI) string {
			return FfiConverterStringINSTANCE.Lift(ffi)
		},
		C.uniffi_marketdata_uniffi_fn_method_futoptintradayclient_get_candles(
			_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterOptionalFutOptCandlesParamsINSTANCE.Lower(params)),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_rust_buffer(handle)
		},
	)

	if err == nil {
		return res, nil
	}

	return res, err
}

// Get available products list (async)
//
// typ: "F" for futures, "O" for options
func (_self *FutOptIntradayClient) GetProducts(typ string, params *FutOptProductsParams) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*FutOptIntradayClient")
	defer _self.ffiObject.decrementPointer()
	res, err := uniffiRustCallAsync[MarketDataError](
		FfiConverterMarketDataErrorINSTANCE,
		// completeFn
		func(handle C.uint64_t, status *C.RustCallStatus) RustBufferI {
			res := C.ffi_marketdata_uniffi_rust_future_complete_rust_buffer(handle, status)
			return GoRustBuffer{
				inner: res,
			}
		},
		// liftFn
		func(ffi RustBufferI) string {
			return FfiConverterStringINSTANCE.Lift(ffi)
		},
		C.uniffi_marketdata_uniffi_fn_method_futoptintradayclient_get_products(
			_pointer, FfiConverterStringINSTANCE.Lower(typ), FfiConverterOptionalFutOptProductsParamsINSTANCE.Lower(params)),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_rust_buffer(handle)
		},
	)

	if err == nil {
		return res, nil
	}

	return res, err
}

// Get quote for a futures/options contract (async)
func (_self *FutOptIntradayClient) GetQuote(symbol string, params *AfterHoursParams) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*FutOptIntradayClient")
	defer _self.ffiObject.decrementPointer()
	res, err := uniffiRustCallAsync[MarketDataError](
		FfiConverterMarketDataErrorINSTANCE,
		// completeFn
		func(handle C.uint64_t, status *C.RustCallStatus) RustBufferI {
			res := C.ffi_marketdata_uniffi_rust_future_complete_rust_buffer(handle, status)
			return GoRustBuffer{
				inner: res,
			}
		},
		// liftFn
		func(ffi RustBufferI) string {
			return FfiConverterStringINSTANCE.Lift(ffi)
		},
		C.uniffi_marketdata_uniffi_fn_method_futoptintradayclient_get_quote(
			_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterOptionalAfterHoursParamsINSTANCE.Lower(params)),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_rust_buffer(handle)
		},
	)

	if err == nil {
		return res, nil
	}

	return res, err
}

// Get ticker info for a contract (async)
func (_self *FutOptIntradayClient) GetTicker(symbol string, params *AfterHoursParams) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*FutOptIntradayClient")
	defer _self.ffiObject.decrementPointer()
	res, err := uniffiRustCallAsync[MarketDataError](
		FfiConverterMarketDataErrorINSTANCE,
		// completeFn
		func(handle C.uint64_t, status *C.RustCallStatus) RustBufferI {
			res := C.ffi_marketdata_uniffi_rust_future_complete_rust_buffer(handle, status)
			return GoRustBuffer{
				inner: res,
			}
		},
		// liftFn
		func(ffi RustBufferI) string {
			return FfiConverterStringINSTANCE.Lift(ffi)
		},
		C.uniffi_marketdata_uniffi_fn_method_futoptintradayclient_get_ticker(
			_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterOptionalAfterHoursParamsINSTANCE.Lower(params)),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_rust_buffer(handle)
		},
	)

	if err == nil {
		return res, nil
	}

	return res, err
}

// Get batch tickers for futures/options (async)
//
// typ: "F" for futures, "O" for options
func (_self *FutOptIntradayClient) GetTickers(typ string, params *FutOptTickersParams) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*FutOptIntradayClient")
	defer _self.ffiObject.decrementPointer()
	res, err := uniffiRustCallAsync[MarketDataError](
		FfiConverterMarketDataErrorINSTANCE,
		// completeFn
		func(handle C.uint64_t, status *C.RustCallStatus) RustBufferI {
			res := C.ffi_marketdata_uniffi_rust_future_complete_rust_buffer(handle, status)
			return GoRustBuffer{
				inner: res,
			}
		},
		// liftFn
		func(ffi RustBufferI) string {
			return FfiConverterStringINSTANCE.Lift(ffi)
		},
		C.uniffi_marketdata_uniffi_fn_method_futoptintradayclient_get_tickers(
			_pointer, FfiConverterStringINSTANCE.Lower(typ), FfiConverterOptionalFutOptTickersParamsINSTANCE.Lower(params)),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_rust_buffer(handle)
		},
	)

	if err == nil {
		return res, nil
	}

	return res, err
}

// Get trade history for a futures/options contract (async)
func (_self *FutOptIntradayClient) GetTrades(symbol string, params *FutOptTradesParams) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*FutOptIntradayClient")
	defer _self.ffiObject.decrementPointer()
	res, err := uniffiRustCallAsync[MarketDataError](
		FfiConverterMarketDataErrorINSTANCE,
		// completeFn
		func(handle C.uint64_t, status *C.RustCallStatus) RustBufferI {
			res := C.ffi_marketdata_uniffi_rust_future_complete_rust_buffer(handle, status)
			return GoRustBuffer{
				inner: res,
			}
		},
		// liftFn
		func(ffi RustBufferI) string {
			return FfiConverterStringINSTANCE.Lift(ffi)
		},
		C.uniffi_marketdata_uniffi_fn_method_futoptintradayclient_get_trades(
			_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterOptionalFutOptTradesParamsINSTANCE.Lower(params)),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_rust_buffer(handle)
		},
	)

	if err == nil {
		return res, nil
	}

	return res, err
}

// Get volume breakdown by price for a futures/options contract (async)
func (_self *FutOptIntradayClient) GetVolumes(symbol string, params *AfterHoursParams) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*FutOptIntradayClient")
	defer _self.ffiObject.decrementPointer()
	res, err := uniffiRustCallAsync[MarketDataError](
		FfiConverterMarketDataErrorINSTANCE,
		// completeFn
		func(handle C.uint64_t, status *C.RustCallStatus) RustBufferI {
			res := C.ffi_marketdata_uniffi_rust_future_complete_rust_buffer(handle, status)
			return GoRustBuffer{
				inner: res,
			}
		},
		// liftFn
		func(ffi RustBufferI) string {
			return FfiConverterStringINSTANCE.Lift(ffi)
		},
		C.uniffi_marketdata_uniffi_fn_method_futoptintradayclient_get_volumes(
			_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterOptionalAfterHoursParamsINSTANCE.Lower(params)),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_rust_buffer(handle)
		},
	)

	if err == nil {
		return res, nil
	}

	return res, err
}

// Get available products list (sync/blocking)
//
// typ: "F" for futures, "O" for options
func (_self *FutOptIntradayClient) ProductsSync(typ string, params *FutOptProductsParams) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*FutOptIntradayClient")
	defer _self.ffiObject.decrementPointer()
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_method_futoptintradayclient_products_sync(
				_pointer, FfiConverterStringINSTANCE.Lower(typ), FfiConverterOptionalFutOptProductsParamsINSTANCE.Lower(params), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue string
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterStringINSTANCE.Lift(_uniffiRV), nil
	}
}

// Get quote for a futures/options contract (sync/blocking)
func (_self *FutOptIntradayClient) QuoteSync(symbol string, params *AfterHoursParams) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*FutOptIntradayClient")
	defer _self.ffiObject.decrementPointer()
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_method_futoptintradayclient_quote_sync(
				_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterOptionalAfterHoursParamsINSTANCE.Lower(params), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue string
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterStringINSTANCE.Lift(_uniffiRV), nil
	}
}

// Get ticker info for a contract (sync/blocking)
func (_self *FutOptIntradayClient) TickerSync(symbol string, params *AfterHoursParams) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*FutOptIntradayClient")
	defer _self.ffiObject.decrementPointer()
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_method_futoptintradayclient_ticker_sync(
				_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterOptionalAfterHoursParamsINSTANCE.Lower(params), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue string
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterStringINSTANCE.Lift(_uniffiRV), nil
	}
}

// Get batch tickers for futures/options (sync/blocking)
//
// typ: "F" for futures, "O" for options
func (_self *FutOptIntradayClient) TickersSync(typ string, params *FutOptTickersParams) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*FutOptIntradayClient")
	defer _self.ffiObject.decrementPointer()
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_method_futoptintradayclient_tickers_sync(
				_pointer, FfiConverterStringINSTANCE.Lower(typ), FfiConverterOptionalFutOptTickersParamsINSTANCE.Lower(params), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue string
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterStringINSTANCE.Lift(_uniffiRV), nil
	}
}

// Get trade history for a contract (sync/blocking)
func (_self *FutOptIntradayClient) TradesSync(symbol string, params *FutOptTradesParams) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*FutOptIntradayClient")
	defer _self.ffiObject.decrementPointer()
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_method_futoptintradayclient_trades_sync(
				_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterOptionalFutOptTradesParamsINSTANCE.Lower(params), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue string
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterStringINSTANCE.Lift(_uniffiRV), nil
	}
}

// Get volume breakdown by price for a contract (sync/blocking)
func (_self *FutOptIntradayClient) VolumesSync(symbol string, params *AfterHoursParams) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*FutOptIntradayClient")
	defer _self.ffiObject.decrementPointer()
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_method_futoptintradayclient_volumes_sync(
				_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterOptionalAfterHoursParamsINSTANCE.Lower(params), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue string
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterStringINSTANCE.Lift(_uniffiRV), nil
	}
}
func (object *FutOptIntradayClient) Destroy() {
	runtime.SetFinalizer(object, nil)
	object.ffiObject.destroy()
}

type FfiConverterFutOptIntradayClient struct{}

var FfiConverterFutOptIntradayClientINSTANCE = FfiConverterFutOptIntradayClient{}

func (c FfiConverterFutOptIntradayClient) Lift(pointer unsafe.Pointer) *FutOptIntradayClient {
	result := &FutOptIntradayClient{
		newFfiObject(
			pointer,
			func(pointer unsafe.Pointer, status *C.RustCallStatus) unsafe.Pointer {
				return C.uniffi_marketdata_uniffi_fn_clone_futoptintradayclient(pointer, status)
			},
			func(pointer unsafe.Pointer, status *C.RustCallStatus) {
				C.uniffi_marketdata_uniffi_fn_free_futoptintradayclient(pointer, status)
			},
		),
	}
	runtime.SetFinalizer(result, (*FutOptIntradayClient).Destroy)
	return result
}

func (c FfiConverterFutOptIntradayClient) Read(reader io.Reader) *FutOptIntradayClient {
	return c.Lift(unsafe.Pointer(uintptr(readUint64(reader))))
}

func (c FfiConverterFutOptIntradayClient) Lower(value *FutOptIntradayClient) unsafe.Pointer {
	// TODO: this is bad - all synchronization from ObjectRuntime.go is discarded here,
	// because the pointer will be decremented immediately after this function returns,
	// and someone will be left holding onto a non-locked pointer.
	pointer := value.ffiObject.incrementPointer("*FutOptIntradayClient")
	defer value.ffiObject.decrementPointer()
	return pointer

}

func (c FfiConverterFutOptIntradayClient) Write(writer io.Writer, value *FutOptIntradayClient) {
	writeUint64(writer, uint64(uintptr(c.Lower(value))))
}

type FfiDestroyerFutOptIntradayClient struct{}

func (_ FfiDestroyerFutOptIntradayClient) Destroy(value *FutOptIntradayClient) {
	value.Destroy()
}

// REST client for UniFFI bindings
//
// Wraps the core RestClient and provides Arc-wrapped sub-clients for FFI safety.
type RestClientInterface interface {
	// The prefix every request from this client is built on, fully resolved —
	// host, path prefix and version segment.
	//
	// The version segment is chosen by the SDK rather than written by the
	// caller, so this is the only way to see what a client resolved to.
	BaseUrl() string
	// Access FutOpt (futures and options) endpoints
	Futopt() *FutOptClient
	// Access stock-related endpoints
	Stock() *StockClient
}

// REST client for UniFFI bindings
//
// Wraps the core RestClient and provides Arc-wrapped sub-clients for FFI safety.
type RestClient struct {
	ffiObject FfiObject
}

// The prefix every request from this client is built on, fully resolved —
// host, path prefix and version segment.
//
// The version segment is chosen by the SDK rather than written by the
// caller, so this is the only way to see what a client resolved to.
func (_self *RestClient) BaseUrl() string {
	_pointer := _self.ffiObject.incrementPointer("*RestClient")
	defer _self.ffiObject.decrementPointer()
	return FfiConverterStringINSTANCE.Lift(rustCall(func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_method_restclient_base_url(
				_pointer, _uniffiStatus),
		}
	}))
}

// Access FutOpt (futures and options) endpoints
func (_self *RestClient) Futopt() *FutOptClient {
	_pointer := _self.ffiObject.incrementPointer("*RestClient")
	defer _self.ffiObject.decrementPointer()
	return FfiConverterFutOptClientINSTANCE.Lift(rustCall(func(_uniffiStatus *C.RustCallStatus) unsafe.Pointer {
		return C.uniffi_marketdata_uniffi_fn_method_restclient_futopt(
			_pointer, _uniffiStatus)
	}))
}

// Access stock-related endpoints
func (_self *RestClient) Stock() *StockClient {
	_pointer := _self.ffiObject.incrementPointer("*RestClient")
	defer _self.ffiObject.decrementPointer()
	return FfiConverterStockClientINSTANCE.Lift(rustCall(func(_uniffiStatus *C.RustCallStatus) unsafe.Pointer {
		return C.uniffi_marketdata_uniffi_fn_method_restclient_stock(
			_pointer, _uniffiStatus)
	}))
}
func (object *RestClient) Destroy() {
	runtime.SetFinalizer(object, nil)
	object.ffiObject.destroy()
}

type FfiConverterRestClient struct{}

var FfiConverterRestClientINSTANCE = FfiConverterRestClient{}

func (c FfiConverterRestClient) Lift(pointer unsafe.Pointer) *RestClient {
	result := &RestClient{
		newFfiObject(
			pointer,
			func(pointer unsafe.Pointer, status *C.RustCallStatus) unsafe.Pointer {
				return C.uniffi_marketdata_uniffi_fn_clone_restclient(pointer, status)
			},
			func(pointer unsafe.Pointer, status *C.RustCallStatus) {
				C.uniffi_marketdata_uniffi_fn_free_restclient(pointer, status)
			},
		),
	}
	runtime.SetFinalizer(result, (*RestClient).Destroy)
	return result
}

func (c FfiConverterRestClient) Read(reader io.Reader) *RestClient {
	return c.Lift(unsafe.Pointer(uintptr(readUint64(reader))))
}

func (c FfiConverterRestClient) Lower(value *RestClient) unsafe.Pointer {
	// TODO: this is bad - all synchronization from ObjectRuntime.go is discarded here,
	// because the pointer will be decremented immediately after this function returns,
	// and someone will be left holding onto a non-locked pointer.
	pointer := value.ffiObject.incrementPointer("*RestClient")
	defer value.ffiObject.decrementPointer()
	return pointer

}

func (c FfiConverterRestClient) Write(writer io.Writer, value *RestClient) {
	writeUint64(writer, uint64(uintptr(c.Lower(value))))
}

type FfiDestroyerRestClient struct{}

func (_ FfiDestroyerRestClient) Destroy(value *RestClient) {
	value.Destroy()
}

// Stock market data client
type StockClientInterface interface {
	// The fully resolved request prefix for this product client.
	BaseUrl() string
	// Access corporate actions endpoints
	CorporateActions() *StockCorporateActionsClient
	// Access historical data endpoints
	Historical() *StockHistoricalClient
	// Access intraday (real-time) endpoints
	Intraday() *StockIntradayClient
	// Access ownership endpoints (ETF holdings, institutional trades, director
	// holdings, TDCC distribution)
	Ownership() *StockOwnershipClient
	// Access snapshot (market-wide) endpoints
	Snapshot() *StockSnapshotClient
	// Access technical indicator endpoints
	Technical() *StockTechnicalClient
}

// Stock market data client
type StockClient struct {
	ffiObject FfiObject
}

// The fully resolved request prefix for this product client.
func (_self *StockClient) BaseUrl() string {
	_pointer := _self.ffiObject.incrementPointer("*StockClient")
	defer _self.ffiObject.decrementPointer()
	return FfiConverterStringINSTANCE.Lift(rustCall(func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_method_stockclient_base_url(
				_pointer, _uniffiStatus),
		}
	}))
}

// Access corporate actions endpoints
func (_self *StockClient) CorporateActions() *StockCorporateActionsClient {
	_pointer := _self.ffiObject.incrementPointer("*StockClient")
	defer _self.ffiObject.decrementPointer()
	return FfiConverterStockCorporateActionsClientINSTANCE.Lift(rustCall(func(_uniffiStatus *C.RustCallStatus) unsafe.Pointer {
		return C.uniffi_marketdata_uniffi_fn_method_stockclient_corporate_actions(
			_pointer, _uniffiStatus)
	}))
}

// Access historical data endpoints
func (_self *StockClient) Historical() *StockHistoricalClient {
	_pointer := _self.ffiObject.incrementPointer("*StockClient")
	defer _self.ffiObject.decrementPointer()
	return FfiConverterStockHistoricalClientINSTANCE.Lift(rustCall(func(_uniffiStatus *C.RustCallStatus) unsafe.Pointer {
		return C.uniffi_marketdata_uniffi_fn_method_stockclient_historical(
			_pointer, _uniffiStatus)
	}))
}

// Access intraday (real-time) endpoints
func (_self *StockClient) Intraday() *StockIntradayClient {
	_pointer := _self.ffiObject.incrementPointer("*StockClient")
	defer _self.ffiObject.decrementPointer()
	return FfiConverterStockIntradayClientINSTANCE.Lift(rustCall(func(_uniffiStatus *C.RustCallStatus) unsafe.Pointer {
		return C.uniffi_marketdata_uniffi_fn_method_stockclient_intraday(
			_pointer, _uniffiStatus)
	}))
}

// Access ownership endpoints (ETF holdings, institutional trades, director
// holdings, TDCC distribution)
func (_self *StockClient) Ownership() *StockOwnershipClient {
	_pointer := _self.ffiObject.incrementPointer("*StockClient")
	defer _self.ffiObject.decrementPointer()
	return FfiConverterStockOwnershipClientINSTANCE.Lift(rustCall(func(_uniffiStatus *C.RustCallStatus) unsafe.Pointer {
		return C.uniffi_marketdata_uniffi_fn_method_stockclient_ownership(
			_pointer, _uniffiStatus)
	}))
}

// Access snapshot (market-wide) endpoints
func (_self *StockClient) Snapshot() *StockSnapshotClient {
	_pointer := _self.ffiObject.incrementPointer("*StockClient")
	defer _self.ffiObject.decrementPointer()
	return FfiConverterStockSnapshotClientINSTANCE.Lift(rustCall(func(_uniffiStatus *C.RustCallStatus) unsafe.Pointer {
		return C.uniffi_marketdata_uniffi_fn_method_stockclient_snapshot(
			_pointer, _uniffiStatus)
	}))
}

// Access technical indicator endpoints
func (_self *StockClient) Technical() *StockTechnicalClient {
	_pointer := _self.ffiObject.incrementPointer("*StockClient")
	defer _self.ffiObject.decrementPointer()
	return FfiConverterStockTechnicalClientINSTANCE.Lift(rustCall(func(_uniffiStatus *C.RustCallStatus) unsafe.Pointer {
		return C.uniffi_marketdata_uniffi_fn_method_stockclient_technical(
			_pointer, _uniffiStatus)
	}))
}
func (object *StockClient) Destroy() {
	runtime.SetFinalizer(object, nil)
	object.ffiObject.destroy()
}

type FfiConverterStockClient struct{}

var FfiConverterStockClientINSTANCE = FfiConverterStockClient{}

func (c FfiConverterStockClient) Lift(pointer unsafe.Pointer) *StockClient {
	result := &StockClient{
		newFfiObject(
			pointer,
			func(pointer unsafe.Pointer, status *C.RustCallStatus) unsafe.Pointer {
				return C.uniffi_marketdata_uniffi_fn_clone_stockclient(pointer, status)
			},
			func(pointer unsafe.Pointer, status *C.RustCallStatus) {
				C.uniffi_marketdata_uniffi_fn_free_stockclient(pointer, status)
			},
		),
	}
	runtime.SetFinalizer(result, (*StockClient).Destroy)
	return result
}

func (c FfiConverterStockClient) Read(reader io.Reader) *StockClient {
	return c.Lift(unsafe.Pointer(uintptr(readUint64(reader))))
}

func (c FfiConverterStockClient) Lower(value *StockClient) unsafe.Pointer {
	// TODO: this is bad - all synchronization from ObjectRuntime.go is discarded here,
	// because the pointer will be decremented immediately after this function returns,
	// and someone will be left holding onto a non-locked pointer.
	pointer := value.ffiObject.incrementPointer("*StockClient")
	defer value.ffiObject.decrementPointer()
	return pointer

}

func (c FfiConverterStockClient) Write(writer io.Writer, value *StockClient) {
	writeUint64(writer, uint64(uintptr(c.Lower(value))))
}

type FfiDestroyerStockClient struct{}

func (_ FfiDestroyerStockClient) Destroy(value *StockClient) {
	value.Destroy()
}

// Stock corporate actions endpoints
//
// Provides access to capital changes, dividends, and listing applicants (IPO).
// One record serves all three; `capital-changes` has no `exchange`, so
// setting it there is 1005 `INVALID_PARAMETER`.
type StockCorporateActionsClientInterface interface {
	// Get capital structure changes (sync/blocking)
	CapitalChangesSync(params *CorporateActionsParams) (string, error)
	// Get dividend announcements (sync/blocking)
	DividendsSync(params *CorporateActionsParams) (string, error)
	// Get capital structure changes (async)
	GetCapitalChanges(params *CorporateActionsParams) (string, error)
	// Get dividend announcements (async)
	GetDividends(params *CorporateActionsParams) (string, error)
	// Get IPO listing applicants (async)
	GetListingApplicants(params *CorporateActionsParams) (string, error)
	// Get IPO listing applicants (sync/blocking)
	ListingApplicantsSync(params *CorporateActionsParams) (string, error)
}

// Stock corporate actions endpoints
//
// Provides access to capital changes, dividends, and listing applicants (IPO).
// One record serves all three; `capital-changes` has no `exchange`, so
// setting it there is 1005 `INVALID_PARAMETER`.
type StockCorporateActionsClient struct {
	ffiObject FfiObject
}

// Get capital structure changes (sync/blocking)
func (_self *StockCorporateActionsClient) CapitalChangesSync(params *CorporateActionsParams) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*StockCorporateActionsClient")
	defer _self.ffiObject.decrementPointer()
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_method_stockcorporateactionsclient_capital_changes_sync(
				_pointer, FfiConverterOptionalCorporateActionsParamsINSTANCE.Lower(params), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue string
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterStringINSTANCE.Lift(_uniffiRV), nil
	}
}

// Get dividend announcements (sync/blocking)
func (_self *StockCorporateActionsClient) DividendsSync(params *CorporateActionsParams) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*StockCorporateActionsClient")
	defer _self.ffiObject.decrementPointer()
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_method_stockcorporateactionsclient_dividends_sync(
				_pointer, FfiConverterOptionalCorporateActionsParamsINSTANCE.Lower(params), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue string
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterStringINSTANCE.Lift(_uniffiRV), nil
	}
}

// Get capital structure changes (async)
func (_self *StockCorporateActionsClient) GetCapitalChanges(params *CorporateActionsParams) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*StockCorporateActionsClient")
	defer _self.ffiObject.decrementPointer()
	res, err := uniffiRustCallAsync[MarketDataError](
		FfiConverterMarketDataErrorINSTANCE,
		// completeFn
		func(handle C.uint64_t, status *C.RustCallStatus) RustBufferI {
			res := C.ffi_marketdata_uniffi_rust_future_complete_rust_buffer(handle, status)
			return GoRustBuffer{
				inner: res,
			}
		},
		// liftFn
		func(ffi RustBufferI) string {
			return FfiConverterStringINSTANCE.Lift(ffi)
		},
		C.uniffi_marketdata_uniffi_fn_method_stockcorporateactionsclient_get_capital_changes(
			_pointer, FfiConverterOptionalCorporateActionsParamsINSTANCE.Lower(params)),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_rust_buffer(handle)
		},
	)

	if err == nil {
		return res, nil
	}

	return res, err
}

// Get dividend announcements (async)
func (_self *StockCorporateActionsClient) GetDividends(params *CorporateActionsParams) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*StockCorporateActionsClient")
	defer _self.ffiObject.decrementPointer()
	res, err := uniffiRustCallAsync[MarketDataError](
		FfiConverterMarketDataErrorINSTANCE,
		// completeFn
		func(handle C.uint64_t, status *C.RustCallStatus) RustBufferI {
			res := C.ffi_marketdata_uniffi_rust_future_complete_rust_buffer(handle, status)
			return GoRustBuffer{
				inner: res,
			}
		},
		// liftFn
		func(ffi RustBufferI) string {
			return FfiConverterStringINSTANCE.Lift(ffi)
		},
		C.uniffi_marketdata_uniffi_fn_method_stockcorporateactionsclient_get_dividends(
			_pointer, FfiConverterOptionalCorporateActionsParamsINSTANCE.Lower(params)),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_rust_buffer(handle)
		},
	)

	if err == nil {
		return res, nil
	}

	return res, err
}

// Get IPO listing applicants (async)
func (_self *StockCorporateActionsClient) GetListingApplicants(params *CorporateActionsParams) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*StockCorporateActionsClient")
	defer _self.ffiObject.decrementPointer()
	res, err := uniffiRustCallAsync[MarketDataError](
		FfiConverterMarketDataErrorINSTANCE,
		// completeFn
		func(handle C.uint64_t, status *C.RustCallStatus) RustBufferI {
			res := C.ffi_marketdata_uniffi_rust_future_complete_rust_buffer(handle, status)
			return GoRustBuffer{
				inner: res,
			}
		},
		// liftFn
		func(ffi RustBufferI) string {
			return FfiConverterStringINSTANCE.Lift(ffi)
		},
		C.uniffi_marketdata_uniffi_fn_method_stockcorporateactionsclient_get_listing_applicants(
			_pointer, FfiConverterOptionalCorporateActionsParamsINSTANCE.Lower(params)),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_rust_buffer(handle)
		},
	)

	if err == nil {
		return res, nil
	}

	return res, err
}

// Get IPO listing applicants (sync/blocking)
func (_self *StockCorporateActionsClient) ListingApplicantsSync(params *CorporateActionsParams) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*StockCorporateActionsClient")
	defer _self.ffiObject.decrementPointer()
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_method_stockcorporateactionsclient_listing_applicants_sync(
				_pointer, FfiConverterOptionalCorporateActionsParamsINSTANCE.Lower(params), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue string
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterStringINSTANCE.Lift(_uniffiRV), nil
	}
}
func (object *StockCorporateActionsClient) Destroy() {
	runtime.SetFinalizer(object, nil)
	object.ffiObject.destroy()
}

type FfiConverterStockCorporateActionsClient struct{}

var FfiConverterStockCorporateActionsClientINSTANCE = FfiConverterStockCorporateActionsClient{}

func (c FfiConverterStockCorporateActionsClient) Lift(pointer unsafe.Pointer) *StockCorporateActionsClient {
	result := &StockCorporateActionsClient{
		newFfiObject(
			pointer,
			func(pointer unsafe.Pointer, status *C.RustCallStatus) unsafe.Pointer {
				return C.uniffi_marketdata_uniffi_fn_clone_stockcorporateactionsclient(pointer, status)
			},
			func(pointer unsafe.Pointer, status *C.RustCallStatus) {
				C.uniffi_marketdata_uniffi_fn_free_stockcorporateactionsclient(pointer, status)
			},
		),
	}
	runtime.SetFinalizer(result, (*StockCorporateActionsClient).Destroy)
	return result
}

func (c FfiConverterStockCorporateActionsClient) Read(reader io.Reader) *StockCorporateActionsClient {
	return c.Lift(unsafe.Pointer(uintptr(readUint64(reader))))
}

func (c FfiConverterStockCorporateActionsClient) Lower(value *StockCorporateActionsClient) unsafe.Pointer {
	// TODO: this is bad - all synchronization from ObjectRuntime.go is discarded here,
	// because the pointer will be decremented immediately after this function returns,
	// and someone will be left holding onto a non-locked pointer.
	pointer := value.ffiObject.incrementPointer("*StockCorporateActionsClient")
	defer value.ffiObject.decrementPointer()
	return pointer

}

func (c FfiConverterStockCorporateActionsClient) Write(writer io.Writer, value *StockCorporateActionsClient) {
	writeUint64(writer, uint64(uintptr(c.Lower(value))))
}

type FfiDestroyerStockCorporateActionsClient struct{}

func (_ FfiDestroyerStockCorporateActionsClient) Destroy(value *StockCorporateActionsClient) {
	value.Destroy()
}

// Stock historical endpoints
type StockHistoricalClientInterface interface {
	// Get historical candles for a symbol (sync/blocking)
	CandlesSync(symbol string, params *StockHistoricalCandlesParams) (string, error)
	// Get historical candles for a symbol (async)
	GetCandles(symbol string, params *StockHistoricalCandlesParams) (string, error)
	// Get historical stats for a symbol (async)
	//
	// Returns summary statistics including 52-week high/low
	GetStats(symbol string) (string, error)
	// Get historical stats for a symbol (sync/blocking)
	StatsSync(symbol string) (string, error)
}

// Stock historical endpoints
type StockHistoricalClient struct {
	ffiObject FfiObject
}

// Get historical candles for a symbol (sync/blocking)
func (_self *StockHistoricalClient) CandlesSync(symbol string, params *StockHistoricalCandlesParams) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*StockHistoricalClient")
	defer _self.ffiObject.decrementPointer()
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_method_stockhistoricalclient_candles_sync(
				_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterOptionalStockHistoricalCandlesParamsINSTANCE.Lower(params), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue string
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterStringINSTANCE.Lift(_uniffiRV), nil
	}
}

// Get historical candles for a symbol (async)
func (_self *StockHistoricalClient) GetCandles(symbol string, params *StockHistoricalCandlesParams) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*StockHistoricalClient")
	defer _self.ffiObject.decrementPointer()
	res, err := uniffiRustCallAsync[MarketDataError](
		FfiConverterMarketDataErrorINSTANCE,
		// completeFn
		func(handle C.uint64_t, status *C.RustCallStatus) RustBufferI {
			res := C.ffi_marketdata_uniffi_rust_future_complete_rust_buffer(handle, status)
			return GoRustBuffer{
				inner: res,
			}
		},
		// liftFn
		func(ffi RustBufferI) string {
			return FfiConverterStringINSTANCE.Lift(ffi)
		},
		C.uniffi_marketdata_uniffi_fn_method_stockhistoricalclient_get_candles(
			_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterOptionalStockHistoricalCandlesParamsINSTANCE.Lower(params)),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_rust_buffer(handle)
		},
	)

	if err == nil {
		return res, nil
	}

	return res, err
}

// Get historical stats for a symbol (async)
//
// Returns summary statistics including 52-week high/low
func (_self *StockHistoricalClient) GetStats(symbol string) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*StockHistoricalClient")
	defer _self.ffiObject.decrementPointer()
	res, err := uniffiRustCallAsync[MarketDataError](
		FfiConverterMarketDataErrorINSTANCE,
		// completeFn
		func(handle C.uint64_t, status *C.RustCallStatus) RustBufferI {
			res := C.ffi_marketdata_uniffi_rust_future_complete_rust_buffer(handle, status)
			return GoRustBuffer{
				inner: res,
			}
		},
		// liftFn
		func(ffi RustBufferI) string {
			return FfiConverterStringINSTANCE.Lift(ffi)
		},
		C.uniffi_marketdata_uniffi_fn_method_stockhistoricalclient_get_stats(
			_pointer, FfiConverterStringINSTANCE.Lower(symbol)),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_rust_buffer(handle)
		},
	)

	if err == nil {
		return res, nil
	}

	return res, err
}

// Get historical stats for a symbol (sync/blocking)
func (_self *StockHistoricalClient) StatsSync(symbol string) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*StockHistoricalClient")
	defer _self.ffiObject.decrementPointer()
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_method_stockhistoricalclient_stats_sync(
				_pointer, FfiConverterStringINSTANCE.Lower(symbol), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue string
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterStringINSTANCE.Lift(_uniffiRV), nil
	}
}
func (object *StockHistoricalClient) Destroy() {
	runtime.SetFinalizer(object, nil)
	object.ffiObject.destroy()
}

type FfiConverterStockHistoricalClient struct{}

var FfiConverterStockHistoricalClientINSTANCE = FfiConverterStockHistoricalClient{}

func (c FfiConverterStockHistoricalClient) Lift(pointer unsafe.Pointer) *StockHistoricalClient {
	result := &StockHistoricalClient{
		newFfiObject(
			pointer,
			func(pointer unsafe.Pointer, status *C.RustCallStatus) unsafe.Pointer {
				return C.uniffi_marketdata_uniffi_fn_clone_stockhistoricalclient(pointer, status)
			},
			func(pointer unsafe.Pointer, status *C.RustCallStatus) {
				C.uniffi_marketdata_uniffi_fn_free_stockhistoricalclient(pointer, status)
			},
		),
	}
	runtime.SetFinalizer(result, (*StockHistoricalClient).Destroy)
	return result
}

func (c FfiConverterStockHistoricalClient) Read(reader io.Reader) *StockHistoricalClient {
	return c.Lift(unsafe.Pointer(uintptr(readUint64(reader))))
}

func (c FfiConverterStockHistoricalClient) Lower(value *StockHistoricalClient) unsafe.Pointer {
	// TODO: this is bad - all synchronization from ObjectRuntime.go is discarded here,
	// because the pointer will be decremented immediately after this function returns,
	// and someone will be left holding onto a non-locked pointer.
	pointer := value.ffiObject.incrementPointer("*StockHistoricalClient")
	defer value.ffiObject.decrementPointer()
	return pointer

}

func (c FfiConverterStockHistoricalClient) Write(writer io.Writer, value *StockHistoricalClient) {
	writeUint64(writer, uint64(uintptr(c.Lower(value))))
}

type FfiDestroyerStockHistoricalClient struct{}

func (_ FfiDestroyerStockHistoricalClient) Destroy(value *StockHistoricalClient) {
	value.Destroy()
}

// Stock intraday endpoints
//
// All methods have both async (get_*) and sync (*_sync) variants:
// - Async methods are preferred for best performance (non-blocking)
// - Sync methods block the calling thread (simpler API for scripting)
type StockIntradayClientInterface interface {
	// Get candlestick data for a symbol (sync/blocking)
	CandlesSync(symbol string, params *StockCandlesParams) (string, error)
	// Get candlestick data for a symbol (async)
	//
	// `timeframe` is in the record: unset takes the server default.
	GetCandles(symbol string, params *StockCandlesParams) (string, error)
	// Get quote for a symbol (async)
	GetQuote(symbol string, params *OddLotParams) (string, error)
	// Get ticker info for a symbol (async)
	GetTicker(symbol string, params *OddLotParams) (string, error)
	// Get batch tickers for a security type (async)
	//
	// typ: Security type (e.g., "EQUITY", "INDEX", "ETF")
	GetTickers(typ string, params *StockTickersParams) (string, error)
	// Get trade history for a symbol (async)
	GetTrades(symbol string, params *StockTradesParams) (string, error)
	// Get volume breakdown for a symbol (async)
	GetVolumes(symbol string, params *OddLotParams) (string, error)
	// Get quote for a symbol (sync/blocking)
	QuoteSync(symbol string, params *OddLotParams) (string, error)
	// Get ticker info for a symbol (sync/blocking)
	TickerSync(symbol string, params *OddLotParams) (string, error)
	// Get batch tickers for a security type (sync/blocking)
	//
	// typ: Security type (e.g., "EQUITY", "INDEX", "ETF")
	TickersSync(typ string, params *StockTickersParams) (string, error)
	// Get trade history for a symbol (sync/blocking)
	TradesSync(symbol string, params *StockTradesParams) (string, error)
	// Get volume breakdown for a symbol (sync/blocking)
	VolumesSync(symbol string, params *OddLotParams) (string, error)
}

// Stock intraday endpoints
//
// All methods have both async (get_*) and sync (*_sync) variants:
// - Async methods are preferred for best performance (non-blocking)
// - Sync methods block the calling thread (simpler API for scripting)
type StockIntradayClient struct {
	ffiObject FfiObject
}

// Get candlestick data for a symbol (sync/blocking)
func (_self *StockIntradayClient) CandlesSync(symbol string, params *StockCandlesParams) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*StockIntradayClient")
	defer _self.ffiObject.decrementPointer()
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_method_stockintradayclient_candles_sync(
				_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterOptionalStockCandlesParamsINSTANCE.Lower(params), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue string
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterStringINSTANCE.Lift(_uniffiRV), nil
	}
}

// Get candlestick data for a symbol (async)
//
// `timeframe` is in the record: unset takes the server default.
func (_self *StockIntradayClient) GetCandles(symbol string, params *StockCandlesParams) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*StockIntradayClient")
	defer _self.ffiObject.decrementPointer()
	res, err := uniffiRustCallAsync[MarketDataError](
		FfiConverterMarketDataErrorINSTANCE,
		// completeFn
		func(handle C.uint64_t, status *C.RustCallStatus) RustBufferI {
			res := C.ffi_marketdata_uniffi_rust_future_complete_rust_buffer(handle, status)
			return GoRustBuffer{
				inner: res,
			}
		},
		// liftFn
		func(ffi RustBufferI) string {
			return FfiConverterStringINSTANCE.Lift(ffi)
		},
		C.uniffi_marketdata_uniffi_fn_method_stockintradayclient_get_candles(
			_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterOptionalStockCandlesParamsINSTANCE.Lower(params)),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_rust_buffer(handle)
		},
	)

	if err == nil {
		return res, nil
	}

	return res, err
}

// Get quote for a symbol (async)
func (_self *StockIntradayClient) GetQuote(symbol string, params *OddLotParams) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*StockIntradayClient")
	defer _self.ffiObject.decrementPointer()
	res, err := uniffiRustCallAsync[MarketDataError](
		FfiConverterMarketDataErrorINSTANCE,
		// completeFn
		func(handle C.uint64_t, status *C.RustCallStatus) RustBufferI {
			res := C.ffi_marketdata_uniffi_rust_future_complete_rust_buffer(handle, status)
			return GoRustBuffer{
				inner: res,
			}
		},
		// liftFn
		func(ffi RustBufferI) string {
			return FfiConverterStringINSTANCE.Lift(ffi)
		},
		C.uniffi_marketdata_uniffi_fn_method_stockintradayclient_get_quote(
			_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterOptionalOddLotParamsINSTANCE.Lower(params)),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_rust_buffer(handle)
		},
	)

	if err == nil {
		return res, nil
	}

	return res, err
}

// Get ticker info for a symbol (async)
func (_self *StockIntradayClient) GetTicker(symbol string, params *OddLotParams) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*StockIntradayClient")
	defer _self.ffiObject.decrementPointer()
	res, err := uniffiRustCallAsync[MarketDataError](
		FfiConverterMarketDataErrorINSTANCE,
		// completeFn
		func(handle C.uint64_t, status *C.RustCallStatus) RustBufferI {
			res := C.ffi_marketdata_uniffi_rust_future_complete_rust_buffer(handle, status)
			return GoRustBuffer{
				inner: res,
			}
		},
		// liftFn
		func(ffi RustBufferI) string {
			return FfiConverterStringINSTANCE.Lift(ffi)
		},
		C.uniffi_marketdata_uniffi_fn_method_stockintradayclient_get_ticker(
			_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterOptionalOddLotParamsINSTANCE.Lower(params)),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_rust_buffer(handle)
		},
	)

	if err == nil {
		return res, nil
	}

	return res, err
}

// Get batch tickers for a security type (async)
//
// typ: Security type (e.g., "EQUITY", "INDEX", "ETF")
func (_self *StockIntradayClient) GetTickers(typ string, params *StockTickersParams) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*StockIntradayClient")
	defer _self.ffiObject.decrementPointer()
	res, err := uniffiRustCallAsync[MarketDataError](
		FfiConverterMarketDataErrorINSTANCE,
		// completeFn
		func(handle C.uint64_t, status *C.RustCallStatus) RustBufferI {
			res := C.ffi_marketdata_uniffi_rust_future_complete_rust_buffer(handle, status)
			return GoRustBuffer{
				inner: res,
			}
		},
		// liftFn
		func(ffi RustBufferI) string {
			return FfiConverterStringINSTANCE.Lift(ffi)
		},
		C.uniffi_marketdata_uniffi_fn_method_stockintradayclient_get_tickers(
			_pointer, FfiConverterStringINSTANCE.Lower(typ), FfiConverterOptionalStockTickersParamsINSTANCE.Lower(params)),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_rust_buffer(handle)
		},
	)

	if err == nil {
		return res, nil
	}

	return res, err
}

// Get trade history for a symbol (async)
func (_self *StockIntradayClient) GetTrades(symbol string, params *StockTradesParams) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*StockIntradayClient")
	defer _self.ffiObject.decrementPointer()
	res, err := uniffiRustCallAsync[MarketDataError](
		FfiConverterMarketDataErrorINSTANCE,
		// completeFn
		func(handle C.uint64_t, status *C.RustCallStatus) RustBufferI {
			res := C.ffi_marketdata_uniffi_rust_future_complete_rust_buffer(handle, status)
			return GoRustBuffer{
				inner: res,
			}
		},
		// liftFn
		func(ffi RustBufferI) string {
			return FfiConverterStringINSTANCE.Lift(ffi)
		},
		C.uniffi_marketdata_uniffi_fn_method_stockintradayclient_get_trades(
			_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterOptionalStockTradesParamsINSTANCE.Lower(params)),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_rust_buffer(handle)
		},
	)

	if err == nil {
		return res, nil
	}

	return res, err
}

// Get volume breakdown for a symbol (async)
func (_self *StockIntradayClient) GetVolumes(symbol string, params *OddLotParams) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*StockIntradayClient")
	defer _self.ffiObject.decrementPointer()
	res, err := uniffiRustCallAsync[MarketDataError](
		FfiConverterMarketDataErrorINSTANCE,
		// completeFn
		func(handle C.uint64_t, status *C.RustCallStatus) RustBufferI {
			res := C.ffi_marketdata_uniffi_rust_future_complete_rust_buffer(handle, status)
			return GoRustBuffer{
				inner: res,
			}
		},
		// liftFn
		func(ffi RustBufferI) string {
			return FfiConverterStringINSTANCE.Lift(ffi)
		},
		C.uniffi_marketdata_uniffi_fn_method_stockintradayclient_get_volumes(
			_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterOptionalOddLotParamsINSTANCE.Lower(params)),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_rust_buffer(handle)
		},
	)

	if err == nil {
		return res, nil
	}

	return res, err
}

// Get quote for a symbol (sync/blocking)
func (_self *StockIntradayClient) QuoteSync(symbol string, params *OddLotParams) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*StockIntradayClient")
	defer _self.ffiObject.decrementPointer()
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_method_stockintradayclient_quote_sync(
				_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterOptionalOddLotParamsINSTANCE.Lower(params), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue string
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterStringINSTANCE.Lift(_uniffiRV), nil
	}
}

// Get ticker info for a symbol (sync/blocking)
func (_self *StockIntradayClient) TickerSync(symbol string, params *OddLotParams) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*StockIntradayClient")
	defer _self.ffiObject.decrementPointer()
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_method_stockintradayclient_ticker_sync(
				_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterOptionalOddLotParamsINSTANCE.Lower(params), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue string
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterStringINSTANCE.Lift(_uniffiRV), nil
	}
}

// Get batch tickers for a security type (sync/blocking)
//
// typ: Security type (e.g., "EQUITY", "INDEX", "ETF")
func (_self *StockIntradayClient) TickersSync(typ string, params *StockTickersParams) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*StockIntradayClient")
	defer _self.ffiObject.decrementPointer()
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_method_stockintradayclient_tickers_sync(
				_pointer, FfiConverterStringINSTANCE.Lower(typ), FfiConverterOptionalStockTickersParamsINSTANCE.Lower(params), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue string
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterStringINSTANCE.Lift(_uniffiRV), nil
	}
}

// Get trade history for a symbol (sync/blocking)
func (_self *StockIntradayClient) TradesSync(symbol string, params *StockTradesParams) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*StockIntradayClient")
	defer _self.ffiObject.decrementPointer()
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_method_stockintradayclient_trades_sync(
				_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterOptionalStockTradesParamsINSTANCE.Lower(params), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue string
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterStringINSTANCE.Lift(_uniffiRV), nil
	}
}

// Get volume breakdown for a symbol (sync/blocking)
func (_self *StockIntradayClient) VolumesSync(symbol string, params *OddLotParams) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*StockIntradayClient")
	defer _self.ffiObject.decrementPointer()
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_method_stockintradayclient_volumes_sync(
				_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterOptionalOddLotParamsINSTANCE.Lower(params), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue string
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterStringINSTANCE.Lift(_uniffiRV), nil
	}
}
func (object *StockIntradayClient) Destroy() {
	runtime.SetFinalizer(object, nil)
	object.ffiObject.destroy()
}

type FfiConverterStockIntradayClient struct{}

var FfiConverterStockIntradayClientINSTANCE = FfiConverterStockIntradayClient{}

func (c FfiConverterStockIntradayClient) Lift(pointer unsafe.Pointer) *StockIntradayClient {
	result := &StockIntradayClient{
		newFfiObject(
			pointer,
			func(pointer unsafe.Pointer, status *C.RustCallStatus) unsafe.Pointer {
				return C.uniffi_marketdata_uniffi_fn_clone_stockintradayclient(pointer, status)
			},
			func(pointer unsafe.Pointer, status *C.RustCallStatus) {
				C.uniffi_marketdata_uniffi_fn_free_stockintradayclient(pointer, status)
			},
		),
	}
	runtime.SetFinalizer(result, (*StockIntradayClient).Destroy)
	return result
}

func (c FfiConverterStockIntradayClient) Read(reader io.Reader) *StockIntradayClient {
	return c.Lift(unsafe.Pointer(uintptr(readUint64(reader))))
}

func (c FfiConverterStockIntradayClient) Lower(value *StockIntradayClient) unsafe.Pointer {
	// TODO: this is bad - all synchronization from ObjectRuntime.go is discarded here,
	// because the pointer will be decremented immediately after this function returns,
	// and someone will be left holding onto a non-locked pointer.
	pointer := value.ffiObject.incrementPointer("*StockIntradayClient")
	defer value.ffiObject.decrementPointer()
	return pointer

}

func (c FfiConverterStockIntradayClient) Write(writer io.Writer, value *StockIntradayClient) {
	writeUint64(writer, uint64(uintptr(c.Lower(value))))
}

type FfiDestroyerStockIntradayClient struct{}

func (_ FfiDestroyerStockIntradayClient) Destroy(value *StockIntradayClient) {
	value.Destroy()
}

// Stock ownership endpoints client
type StockOwnershipClientInterface interface {
	// Get monthly holdings and pledges disclosed by directors and supervisors (sync/blocking)
	DirectorHoldingsSync(symbol string, params *OwnershipParams) (string, error)
	// Get the constituents an ETF held over a date range (sync/blocking)
	EtfHoldingsSync(symbol string, params *OwnershipParams) (string, error)
	// Get monthly holdings and pledges disclosed by directors and supervisors (async)
	GetDirectorHoldings(symbol string, params *OwnershipParams) (string, error)
	// Get the constituents an ETF held over a date range (async)
	GetEtfHoldings(symbol string, params *OwnershipParams) (string, error)
	// Get daily trading by the three major institutional investors (async)
	GetInstitutionalTrades(symbol string, params *OwnershipParams) (string, error)
	// Get the weekly TDCC shareholder distribution by holding-size bracket (async)
	GetTdccDistribution(symbol string, params *OwnershipParams) (string, error)
	// Get daily trading by the three major institutional investors (sync/blocking)
	InstitutionalTradesSync(symbol string, params *OwnershipParams) (string, error)
	// Get the weekly TDCC shareholder distribution by holding-size bracket (sync/blocking)
	TdccDistributionSync(symbol string, params *OwnershipParams) (string, error)
}

// Stock ownership endpoints client
type StockOwnershipClient struct {
	ffiObject FfiObject
}

// Get monthly holdings and pledges disclosed by directors and supervisors (sync/blocking)
func (_self *StockOwnershipClient) DirectorHoldingsSync(symbol string, params *OwnershipParams) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*StockOwnershipClient")
	defer _self.ffiObject.decrementPointer()
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_method_stockownershipclient_director_holdings_sync(
				_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterOptionalOwnershipParamsINSTANCE.Lower(params), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue string
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterStringINSTANCE.Lift(_uniffiRV), nil
	}
}

// Get the constituents an ETF held over a date range (sync/blocking)
func (_self *StockOwnershipClient) EtfHoldingsSync(symbol string, params *OwnershipParams) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*StockOwnershipClient")
	defer _self.ffiObject.decrementPointer()
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_method_stockownershipclient_etf_holdings_sync(
				_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterOptionalOwnershipParamsINSTANCE.Lower(params), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue string
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterStringINSTANCE.Lift(_uniffiRV), nil
	}
}

// Get monthly holdings and pledges disclosed by directors and supervisors (async)
func (_self *StockOwnershipClient) GetDirectorHoldings(symbol string, params *OwnershipParams) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*StockOwnershipClient")
	defer _self.ffiObject.decrementPointer()
	res, err := uniffiRustCallAsync[MarketDataError](
		FfiConverterMarketDataErrorINSTANCE,
		// completeFn
		func(handle C.uint64_t, status *C.RustCallStatus) RustBufferI {
			res := C.ffi_marketdata_uniffi_rust_future_complete_rust_buffer(handle, status)
			return GoRustBuffer{
				inner: res,
			}
		},
		// liftFn
		func(ffi RustBufferI) string {
			return FfiConverterStringINSTANCE.Lift(ffi)
		},
		C.uniffi_marketdata_uniffi_fn_method_stockownershipclient_get_director_holdings(
			_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterOptionalOwnershipParamsINSTANCE.Lower(params)),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_rust_buffer(handle)
		},
	)

	if err == nil {
		return res, nil
	}

	return res, err
}

// Get the constituents an ETF held over a date range (async)
func (_self *StockOwnershipClient) GetEtfHoldings(symbol string, params *OwnershipParams) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*StockOwnershipClient")
	defer _self.ffiObject.decrementPointer()
	res, err := uniffiRustCallAsync[MarketDataError](
		FfiConverterMarketDataErrorINSTANCE,
		// completeFn
		func(handle C.uint64_t, status *C.RustCallStatus) RustBufferI {
			res := C.ffi_marketdata_uniffi_rust_future_complete_rust_buffer(handle, status)
			return GoRustBuffer{
				inner: res,
			}
		},
		// liftFn
		func(ffi RustBufferI) string {
			return FfiConverterStringINSTANCE.Lift(ffi)
		},
		C.uniffi_marketdata_uniffi_fn_method_stockownershipclient_get_etf_holdings(
			_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterOptionalOwnershipParamsINSTANCE.Lower(params)),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_rust_buffer(handle)
		},
	)

	if err == nil {
		return res, nil
	}

	return res, err
}

// Get daily trading by the three major institutional investors (async)
func (_self *StockOwnershipClient) GetInstitutionalTrades(symbol string, params *OwnershipParams) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*StockOwnershipClient")
	defer _self.ffiObject.decrementPointer()
	res, err := uniffiRustCallAsync[MarketDataError](
		FfiConverterMarketDataErrorINSTANCE,
		// completeFn
		func(handle C.uint64_t, status *C.RustCallStatus) RustBufferI {
			res := C.ffi_marketdata_uniffi_rust_future_complete_rust_buffer(handle, status)
			return GoRustBuffer{
				inner: res,
			}
		},
		// liftFn
		func(ffi RustBufferI) string {
			return FfiConverterStringINSTANCE.Lift(ffi)
		},
		C.uniffi_marketdata_uniffi_fn_method_stockownershipclient_get_institutional_trades(
			_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterOptionalOwnershipParamsINSTANCE.Lower(params)),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_rust_buffer(handle)
		},
	)

	if err == nil {
		return res, nil
	}

	return res, err
}

// Get the weekly TDCC shareholder distribution by holding-size bracket (async)
func (_self *StockOwnershipClient) GetTdccDistribution(symbol string, params *OwnershipParams) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*StockOwnershipClient")
	defer _self.ffiObject.decrementPointer()
	res, err := uniffiRustCallAsync[MarketDataError](
		FfiConverterMarketDataErrorINSTANCE,
		// completeFn
		func(handle C.uint64_t, status *C.RustCallStatus) RustBufferI {
			res := C.ffi_marketdata_uniffi_rust_future_complete_rust_buffer(handle, status)
			return GoRustBuffer{
				inner: res,
			}
		},
		// liftFn
		func(ffi RustBufferI) string {
			return FfiConverterStringINSTANCE.Lift(ffi)
		},
		C.uniffi_marketdata_uniffi_fn_method_stockownershipclient_get_tdcc_distribution(
			_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterOptionalOwnershipParamsINSTANCE.Lower(params)),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_rust_buffer(handle)
		},
	)

	if err == nil {
		return res, nil
	}

	return res, err
}

// Get daily trading by the three major institutional investors (sync/blocking)
func (_self *StockOwnershipClient) InstitutionalTradesSync(symbol string, params *OwnershipParams) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*StockOwnershipClient")
	defer _self.ffiObject.decrementPointer()
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_method_stockownershipclient_institutional_trades_sync(
				_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterOptionalOwnershipParamsINSTANCE.Lower(params), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue string
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterStringINSTANCE.Lift(_uniffiRV), nil
	}
}

// Get the weekly TDCC shareholder distribution by holding-size bracket (sync/blocking)
func (_self *StockOwnershipClient) TdccDistributionSync(symbol string, params *OwnershipParams) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*StockOwnershipClient")
	defer _self.ffiObject.decrementPointer()
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_method_stockownershipclient_tdcc_distribution_sync(
				_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterOptionalOwnershipParamsINSTANCE.Lower(params), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue string
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterStringINSTANCE.Lift(_uniffiRV), nil
	}
}
func (object *StockOwnershipClient) Destroy() {
	runtime.SetFinalizer(object, nil)
	object.ffiObject.destroy()
}

type FfiConverterStockOwnershipClient struct{}

var FfiConverterStockOwnershipClientINSTANCE = FfiConverterStockOwnershipClient{}

func (c FfiConverterStockOwnershipClient) Lift(pointer unsafe.Pointer) *StockOwnershipClient {
	result := &StockOwnershipClient{
		newFfiObject(
			pointer,
			func(pointer unsafe.Pointer, status *C.RustCallStatus) unsafe.Pointer {
				return C.uniffi_marketdata_uniffi_fn_clone_stockownershipclient(pointer, status)
			},
			func(pointer unsafe.Pointer, status *C.RustCallStatus) {
				C.uniffi_marketdata_uniffi_fn_free_stockownershipclient(pointer, status)
			},
		),
	}
	runtime.SetFinalizer(result, (*StockOwnershipClient).Destroy)
	return result
}

func (c FfiConverterStockOwnershipClient) Read(reader io.Reader) *StockOwnershipClient {
	return c.Lift(unsafe.Pointer(uintptr(readUint64(reader))))
}

func (c FfiConverterStockOwnershipClient) Lower(value *StockOwnershipClient) unsafe.Pointer {
	// TODO: this is bad - all synchronization from ObjectRuntime.go is discarded here,
	// because the pointer will be decremented immediately after this function returns,
	// and someone will be left holding onto a non-locked pointer.
	pointer := value.ffiObject.incrementPointer("*StockOwnershipClient")
	defer value.ffiObject.decrementPointer()
	return pointer

}

func (c FfiConverterStockOwnershipClient) Write(writer io.Writer, value *StockOwnershipClient) {
	writeUint64(writer, uint64(uintptr(c.Lower(value))))
}

type FfiDestroyerStockOwnershipClient struct{}

func (_ FfiDestroyerStockOwnershipClient) Destroy(value *StockOwnershipClient) {
	value.Destroy()
}

// Stock snapshot endpoints for market-wide data
//
// Provides access to quotes, movers (gainers/losers), and most active stocks
// across entire markets.
type StockSnapshotClientInterface interface {
	// Get most actively traded stocks (sync/blocking)
	ActivesSync(market string, trade string, params *SnapshotParams) (string, error)
	// Get most actively traded stocks (async)
	//
	// trade: "volume" or "value"
	GetActives(market string, trade string, params *SnapshotParams) (string, error)
	// Get top movers (gainers/losers) in a market (async)
	//
	// direction: "up" for gainers, "down" for losers;
	// change: "percent" or "value"
	GetMovers(market string, direction string, change string, params *MoversParams) (string, error)
	// Get market-wide snapshot quotes (async)
	//
	// market: TSE, OTC, ESB, TIB or PSB
	GetQuotes(market string, params *SnapshotParams) (string, error)
	// Get top movers (sync/blocking)
	MoversSync(market string, direction string, change string, params *MoversParams) (string, error)
	// Get market-wide snapshot quotes (sync/blocking)
	QuotesSync(market string, params *SnapshotParams) (string, error)
}

// Stock snapshot endpoints for market-wide data
//
// Provides access to quotes, movers (gainers/losers), and most active stocks
// across entire markets.
type StockSnapshotClient struct {
	ffiObject FfiObject
}

// Get most actively traded stocks (sync/blocking)
func (_self *StockSnapshotClient) ActivesSync(market string, trade string, params *SnapshotParams) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*StockSnapshotClient")
	defer _self.ffiObject.decrementPointer()
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_method_stocksnapshotclient_actives_sync(
				_pointer, FfiConverterStringINSTANCE.Lower(market), FfiConverterStringINSTANCE.Lower(trade), FfiConverterOptionalSnapshotParamsINSTANCE.Lower(params), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue string
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterStringINSTANCE.Lift(_uniffiRV), nil
	}
}

// Get most actively traded stocks (async)
//
// trade: "volume" or "value"
func (_self *StockSnapshotClient) GetActives(market string, trade string, params *SnapshotParams) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*StockSnapshotClient")
	defer _self.ffiObject.decrementPointer()
	res, err := uniffiRustCallAsync[MarketDataError](
		FfiConverterMarketDataErrorINSTANCE,
		// completeFn
		func(handle C.uint64_t, status *C.RustCallStatus) RustBufferI {
			res := C.ffi_marketdata_uniffi_rust_future_complete_rust_buffer(handle, status)
			return GoRustBuffer{
				inner: res,
			}
		},
		// liftFn
		func(ffi RustBufferI) string {
			return FfiConverterStringINSTANCE.Lift(ffi)
		},
		C.uniffi_marketdata_uniffi_fn_method_stocksnapshotclient_get_actives(
			_pointer, FfiConverterStringINSTANCE.Lower(market), FfiConverterStringINSTANCE.Lower(trade), FfiConverterOptionalSnapshotParamsINSTANCE.Lower(params)),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_rust_buffer(handle)
		},
	)

	if err == nil {
		return res, nil
	}

	return res, err
}

// Get top movers (gainers/losers) in a market (async)
//
// direction: "up" for gainers, "down" for losers;
// change: "percent" or "value"
func (_self *StockSnapshotClient) GetMovers(market string, direction string, change string, params *MoversParams) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*StockSnapshotClient")
	defer _self.ffiObject.decrementPointer()
	res, err := uniffiRustCallAsync[MarketDataError](
		FfiConverterMarketDataErrorINSTANCE,
		// completeFn
		func(handle C.uint64_t, status *C.RustCallStatus) RustBufferI {
			res := C.ffi_marketdata_uniffi_rust_future_complete_rust_buffer(handle, status)
			return GoRustBuffer{
				inner: res,
			}
		},
		// liftFn
		func(ffi RustBufferI) string {
			return FfiConverterStringINSTANCE.Lift(ffi)
		},
		C.uniffi_marketdata_uniffi_fn_method_stocksnapshotclient_get_movers(
			_pointer, FfiConverterStringINSTANCE.Lower(market), FfiConverterStringINSTANCE.Lower(direction), FfiConverterStringINSTANCE.Lower(change), FfiConverterOptionalMoversParamsINSTANCE.Lower(params)),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_rust_buffer(handle)
		},
	)

	if err == nil {
		return res, nil
	}

	return res, err
}

// Get market-wide snapshot quotes (async)
//
// market: TSE, OTC, ESB, TIB or PSB
func (_self *StockSnapshotClient) GetQuotes(market string, params *SnapshotParams) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*StockSnapshotClient")
	defer _self.ffiObject.decrementPointer()
	res, err := uniffiRustCallAsync[MarketDataError](
		FfiConverterMarketDataErrorINSTANCE,
		// completeFn
		func(handle C.uint64_t, status *C.RustCallStatus) RustBufferI {
			res := C.ffi_marketdata_uniffi_rust_future_complete_rust_buffer(handle, status)
			return GoRustBuffer{
				inner: res,
			}
		},
		// liftFn
		func(ffi RustBufferI) string {
			return FfiConverterStringINSTANCE.Lift(ffi)
		},
		C.uniffi_marketdata_uniffi_fn_method_stocksnapshotclient_get_quotes(
			_pointer, FfiConverterStringINSTANCE.Lower(market), FfiConverterOptionalSnapshotParamsINSTANCE.Lower(params)),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_rust_buffer(handle)
		},
	)

	if err == nil {
		return res, nil
	}

	return res, err
}

// Get top movers (sync/blocking)
func (_self *StockSnapshotClient) MoversSync(market string, direction string, change string, params *MoversParams) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*StockSnapshotClient")
	defer _self.ffiObject.decrementPointer()
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_method_stocksnapshotclient_movers_sync(
				_pointer, FfiConverterStringINSTANCE.Lower(market), FfiConverterStringINSTANCE.Lower(direction), FfiConverterStringINSTANCE.Lower(change), FfiConverterOptionalMoversParamsINSTANCE.Lower(params), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue string
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterStringINSTANCE.Lift(_uniffiRV), nil
	}
}

// Get market-wide snapshot quotes (sync/blocking)
func (_self *StockSnapshotClient) QuotesSync(market string, params *SnapshotParams) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*StockSnapshotClient")
	defer _self.ffiObject.decrementPointer()
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_method_stocksnapshotclient_quotes_sync(
				_pointer, FfiConverterStringINSTANCE.Lower(market), FfiConverterOptionalSnapshotParamsINSTANCE.Lower(params), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue string
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterStringINSTANCE.Lift(_uniffiRV), nil
	}
}
func (object *StockSnapshotClient) Destroy() {
	runtime.SetFinalizer(object, nil)
	object.ffiObject.destroy()
}

type FfiConverterStockSnapshotClient struct{}

var FfiConverterStockSnapshotClientINSTANCE = FfiConverterStockSnapshotClient{}

func (c FfiConverterStockSnapshotClient) Lift(pointer unsafe.Pointer) *StockSnapshotClient {
	result := &StockSnapshotClient{
		newFfiObject(
			pointer,
			func(pointer unsafe.Pointer, status *C.RustCallStatus) unsafe.Pointer {
				return C.uniffi_marketdata_uniffi_fn_clone_stocksnapshotclient(pointer, status)
			},
			func(pointer unsafe.Pointer, status *C.RustCallStatus) {
				C.uniffi_marketdata_uniffi_fn_free_stocksnapshotclient(pointer, status)
			},
		),
	}
	runtime.SetFinalizer(result, (*StockSnapshotClient).Destroy)
	return result
}

func (c FfiConverterStockSnapshotClient) Read(reader io.Reader) *StockSnapshotClient {
	return c.Lift(unsafe.Pointer(uintptr(readUint64(reader))))
}

func (c FfiConverterStockSnapshotClient) Lower(value *StockSnapshotClient) unsafe.Pointer {
	// TODO: this is bad - all synchronization from ObjectRuntime.go is discarded here,
	// because the pointer will be decremented immediately after this function returns,
	// and someone will be left holding onto a non-locked pointer.
	pointer := value.ffiObject.incrementPointer("*StockSnapshotClient")
	defer value.ffiObject.decrementPointer()
	return pointer

}

func (c FfiConverterStockSnapshotClient) Write(writer io.Writer, value *StockSnapshotClient) {
	writeUint64(writer, uint64(uintptr(c.Lower(value))))
}

type FfiDestroyerStockSnapshotClient struct{}

func (_ FfiDestroyerStockSnapshotClient) Destroy(value *StockSnapshotClient) {
	value.Destroy()
}

// Stock technical indicator endpoints
//
// Provides access to SMA, RSI, KDJ, MACD, and Bollinger Bands indicators.
// The periods are required by the server and so are positional; the date
// range is the record.
type StockTechnicalClientInterface interface {
	// Get Bollinger Bands (sync/blocking)
	BbSync(symbol string, period uint32, params *TechnicalParams) (string, error)
	// Get Bollinger Bands (async)
	GetBb(symbol string, period uint32, params *TechnicalParams) (string, error)
	// Get KDJ (Stochastic Oscillator) (async)
	GetKdj(symbol string, rPeriod uint32, kPeriod uint32, dPeriod uint32, params *TechnicalParams) (string, error)
	// Get MACD indicator (async)
	GetMacd(symbol string, fast uint32, slow uint32, signal uint32, params *TechnicalParams) (string, error)
	// Get Relative Strength Index (async)
	GetRsi(symbol string, period uint32, params *TechnicalParams) (string, error)
	// Get Simple Moving Average (async)
	GetSma(symbol string, period uint32, params *TechnicalParams) (string, error)
	// Get KDJ (sync/blocking)
	KdjSync(symbol string, rPeriod uint32, kPeriod uint32, dPeriod uint32, params *TechnicalParams) (string, error)
	// Get MACD (sync/blocking)
	MacdSync(symbol string, fast uint32, slow uint32, signal uint32, params *TechnicalParams) (string, error)
	// Get Relative Strength Index (sync/blocking)
	RsiSync(symbol string, period uint32, params *TechnicalParams) (string, error)
	// Get Simple Moving Average (sync/blocking)
	SmaSync(symbol string, period uint32, params *TechnicalParams) (string, error)
}

// Stock technical indicator endpoints
//
// Provides access to SMA, RSI, KDJ, MACD, and Bollinger Bands indicators.
// The periods are required by the server and so are positional; the date
// range is the record.
type StockTechnicalClient struct {
	ffiObject FfiObject
}

// Get Bollinger Bands (sync/blocking)
func (_self *StockTechnicalClient) BbSync(symbol string, period uint32, params *TechnicalParams) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*StockTechnicalClient")
	defer _self.ffiObject.decrementPointer()
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_method_stocktechnicalclient_bb_sync(
				_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterUint32INSTANCE.Lower(period), FfiConverterOptionalTechnicalParamsINSTANCE.Lower(params), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue string
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterStringINSTANCE.Lift(_uniffiRV), nil
	}
}

// Get Bollinger Bands (async)
func (_self *StockTechnicalClient) GetBb(symbol string, period uint32, params *TechnicalParams) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*StockTechnicalClient")
	defer _self.ffiObject.decrementPointer()
	res, err := uniffiRustCallAsync[MarketDataError](
		FfiConverterMarketDataErrorINSTANCE,
		// completeFn
		func(handle C.uint64_t, status *C.RustCallStatus) RustBufferI {
			res := C.ffi_marketdata_uniffi_rust_future_complete_rust_buffer(handle, status)
			return GoRustBuffer{
				inner: res,
			}
		},
		// liftFn
		func(ffi RustBufferI) string {
			return FfiConverterStringINSTANCE.Lift(ffi)
		},
		C.uniffi_marketdata_uniffi_fn_method_stocktechnicalclient_get_bb(
			_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterUint32INSTANCE.Lower(period), FfiConverterOptionalTechnicalParamsINSTANCE.Lower(params)),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_rust_buffer(handle)
		},
	)

	if err == nil {
		return res, nil
	}

	return res, err
}

// Get KDJ (Stochastic Oscillator) (async)
func (_self *StockTechnicalClient) GetKdj(symbol string, rPeriod uint32, kPeriod uint32, dPeriod uint32, params *TechnicalParams) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*StockTechnicalClient")
	defer _self.ffiObject.decrementPointer()
	res, err := uniffiRustCallAsync[MarketDataError](
		FfiConverterMarketDataErrorINSTANCE,
		// completeFn
		func(handle C.uint64_t, status *C.RustCallStatus) RustBufferI {
			res := C.ffi_marketdata_uniffi_rust_future_complete_rust_buffer(handle, status)
			return GoRustBuffer{
				inner: res,
			}
		},
		// liftFn
		func(ffi RustBufferI) string {
			return FfiConverterStringINSTANCE.Lift(ffi)
		},
		C.uniffi_marketdata_uniffi_fn_method_stocktechnicalclient_get_kdj(
			_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterUint32INSTANCE.Lower(rPeriod), FfiConverterUint32INSTANCE.Lower(kPeriod), FfiConverterUint32INSTANCE.Lower(dPeriod), FfiConverterOptionalTechnicalParamsINSTANCE.Lower(params)),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_rust_buffer(handle)
		},
	)

	if err == nil {
		return res, nil
	}

	return res, err
}

// Get MACD indicator (async)
func (_self *StockTechnicalClient) GetMacd(symbol string, fast uint32, slow uint32, signal uint32, params *TechnicalParams) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*StockTechnicalClient")
	defer _self.ffiObject.decrementPointer()
	res, err := uniffiRustCallAsync[MarketDataError](
		FfiConverterMarketDataErrorINSTANCE,
		// completeFn
		func(handle C.uint64_t, status *C.RustCallStatus) RustBufferI {
			res := C.ffi_marketdata_uniffi_rust_future_complete_rust_buffer(handle, status)
			return GoRustBuffer{
				inner: res,
			}
		},
		// liftFn
		func(ffi RustBufferI) string {
			return FfiConverterStringINSTANCE.Lift(ffi)
		},
		C.uniffi_marketdata_uniffi_fn_method_stocktechnicalclient_get_macd(
			_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterUint32INSTANCE.Lower(fast), FfiConverterUint32INSTANCE.Lower(slow), FfiConverterUint32INSTANCE.Lower(signal), FfiConverterOptionalTechnicalParamsINSTANCE.Lower(params)),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_rust_buffer(handle)
		},
	)

	if err == nil {
		return res, nil
	}

	return res, err
}

// Get Relative Strength Index (async)
func (_self *StockTechnicalClient) GetRsi(symbol string, period uint32, params *TechnicalParams) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*StockTechnicalClient")
	defer _self.ffiObject.decrementPointer()
	res, err := uniffiRustCallAsync[MarketDataError](
		FfiConverterMarketDataErrorINSTANCE,
		// completeFn
		func(handle C.uint64_t, status *C.RustCallStatus) RustBufferI {
			res := C.ffi_marketdata_uniffi_rust_future_complete_rust_buffer(handle, status)
			return GoRustBuffer{
				inner: res,
			}
		},
		// liftFn
		func(ffi RustBufferI) string {
			return FfiConverterStringINSTANCE.Lift(ffi)
		},
		C.uniffi_marketdata_uniffi_fn_method_stocktechnicalclient_get_rsi(
			_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterUint32INSTANCE.Lower(period), FfiConverterOptionalTechnicalParamsINSTANCE.Lower(params)),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_rust_buffer(handle)
		},
	)

	if err == nil {
		return res, nil
	}

	return res, err
}

// Get Simple Moving Average (async)
func (_self *StockTechnicalClient) GetSma(symbol string, period uint32, params *TechnicalParams) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*StockTechnicalClient")
	defer _self.ffiObject.decrementPointer()
	res, err := uniffiRustCallAsync[MarketDataError](
		FfiConverterMarketDataErrorINSTANCE,
		// completeFn
		func(handle C.uint64_t, status *C.RustCallStatus) RustBufferI {
			res := C.ffi_marketdata_uniffi_rust_future_complete_rust_buffer(handle, status)
			return GoRustBuffer{
				inner: res,
			}
		},
		// liftFn
		func(ffi RustBufferI) string {
			return FfiConverterStringINSTANCE.Lift(ffi)
		},
		C.uniffi_marketdata_uniffi_fn_method_stocktechnicalclient_get_sma(
			_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterUint32INSTANCE.Lower(period), FfiConverterOptionalTechnicalParamsINSTANCE.Lower(params)),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_rust_buffer(handle)
		},
	)

	if err == nil {
		return res, nil
	}

	return res, err
}

// Get KDJ (sync/blocking)
func (_self *StockTechnicalClient) KdjSync(symbol string, rPeriod uint32, kPeriod uint32, dPeriod uint32, params *TechnicalParams) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*StockTechnicalClient")
	defer _self.ffiObject.decrementPointer()
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_method_stocktechnicalclient_kdj_sync(
				_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterUint32INSTANCE.Lower(rPeriod), FfiConverterUint32INSTANCE.Lower(kPeriod), FfiConverterUint32INSTANCE.Lower(dPeriod), FfiConverterOptionalTechnicalParamsINSTANCE.Lower(params), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue string
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterStringINSTANCE.Lift(_uniffiRV), nil
	}
}

// Get MACD (sync/blocking)
func (_self *StockTechnicalClient) MacdSync(symbol string, fast uint32, slow uint32, signal uint32, params *TechnicalParams) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*StockTechnicalClient")
	defer _self.ffiObject.decrementPointer()
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_method_stocktechnicalclient_macd_sync(
				_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterUint32INSTANCE.Lower(fast), FfiConverterUint32INSTANCE.Lower(slow), FfiConverterUint32INSTANCE.Lower(signal), FfiConverterOptionalTechnicalParamsINSTANCE.Lower(params), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue string
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterStringINSTANCE.Lift(_uniffiRV), nil
	}
}

// Get Relative Strength Index (sync/blocking)
func (_self *StockTechnicalClient) RsiSync(symbol string, period uint32, params *TechnicalParams) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*StockTechnicalClient")
	defer _self.ffiObject.decrementPointer()
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_method_stocktechnicalclient_rsi_sync(
				_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterUint32INSTANCE.Lower(period), FfiConverterOptionalTechnicalParamsINSTANCE.Lower(params), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue string
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterStringINSTANCE.Lift(_uniffiRV), nil
	}
}

// Get Simple Moving Average (sync/blocking)
func (_self *StockTechnicalClient) SmaSync(symbol string, period uint32, params *TechnicalParams) (string, error) {
	_pointer := _self.ffiObject.incrementPointer("*StockTechnicalClient")
	defer _self.ffiObject.decrementPointer()
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_method_stocktechnicalclient_sma_sync(
				_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterUint32INSTANCE.Lower(period), FfiConverterOptionalTechnicalParamsINSTANCE.Lower(params), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue string
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterStringINSTANCE.Lift(_uniffiRV), nil
	}
}
func (object *StockTechnicalClient) Destroy() {
	runtime.SetFinalizer(object, nil)
	object.ffiObject.destroy()
}

type FfiConverterStockTechnicalClient struct{}

var FfiConverterStockTechnicalClientINSTANCE = FfiConverterStockTechnicalClient{}

func (c FfiConverterStockTechnicalClient) Lift(pointer unsafe.Pointer) *StockTechnicalClient {
	result := &StockTechnicalClient{
		newFfiObject(
			pointer,
			func(pointer unsafe.Pointer, status *C.RustCallStatus) unsafe.Pointer {
				return C.uniffi_marketdata_uniffi_fn_clone_stocktechnicalclient(pointer, status)
			},
			func(pointer unsafe.Pointer, status *C.RustCallStatus) {
				C.uniffi_marketdata_uniffi_fn_free_stocktechnicalclient(pointer, status)
			},
		),
	}
	runtime.SetFinalizer(result, (*StockTechnicalClient).Destroy)
	return result
}

func (c FfiConverterStockTechnicalClient) Read(reader io.Reader) *StockTechnicalClient {
	return c.Lift(unsafe.Pointer(uintptr(readUint64(reader))))
}

func (c FfiConverterStockTechnicalClient) Lower(value *StockTechnicalClient) unsafe.Pointer {
	// TODO: this is bad - all synchronization from ObjectRuntime.go is discarded here,
	// because the pointer will be decremented immediately after this function returns,
	// and someone will be left holding onto a non-locked pointer.
	pointer := value.ffiObject.incrementPointer("*StockTechnicalClient")
	defer value.ffiObject.decrementPointer()
	return pointer

}

func (c FfiConverterStockTechnicalClient) Write(writer io.Writer, value *StockTechnicalClient) {
	writeUint64(writer, uint64(uintptr(c.Lower(value))))
}

type FfiDestroyerStockTechnicalClient struct{}

func (_ FfiDestroyerStockTechnicalClient) Destroy(value *StockTechnicalClient) {
	value.Destroy()
}

// WebSocket client for real-time market data streaming
//
// Wraps the core WebSocketClient and forwards messages to the provided
// WebSocketListener implementation via a background task.
type WebSocketClientInterface interface {
	// Connect to the WebSocket server and authenticate.
	//
	// Refused with code 2011 (`ALREADY_CONNECTED`) while connected or while
	// another `connect()` is in progress.
	//
	// During an automatic reconnect it opens no connection of its own: it
	// waits for that reconnect and returns once the connection is back and
	// the subscriptions are re-sent, so a `subscribe()` afterwards follows
	// them. The wait fails with 2010 (`ClientClosed`) if `disconnect()` is
	// called, 3005 (`RECONNECT_FAILED`) if the reconnect runs out of
	// attempts, and `AuthError` (2002) if its credentials are rejected.
	// Called from a listener method, it holds up the listener until the
	// reconnect ends.
	Connect() error
	// Disconnect, returning once the listener has handled the connection's
	// remaining events, `on_disconnected` included.
	//
	// There is no timeout on that wait: a listener method that blocks keeps
	// `disconnect()` waiting for as long as it does.
	//
	// Called from a listener method, it returns without that wait: those
	// events are delivered on the thread running the method, after it
	// returns.
	Disconnect()
	// Check if the connection has ended
	//
	// Reads core's connection state: true after `disconnect()`, and after
	// the server closes the connection when no reconnect follows (disabled
	// or attempts exhausted). False while reconnecting and before the first
	// `connect()`.
	IsClosed() bool
	// Check if the client is currently connected
	//
	// Reads core's connection state, so it is false while reconnecting and
	// right after the connection drops, without waiting for the event thread.
	IsConnected() bool
	// Measure the round trip to the server: send a ping, wait for its pong,
	// and return the time between the two in milliseconds.
	//
	// Unlike `ping()` (fire and forget, pong delivered to `on_message`),
	// this waits for the answer, and its pong is not delivered. Works
	// whether or not `probe_enabled` is set, and sends nothing in the
	// background. `timeout_ms` defaults to 5000 when `None`.
	//
	// Errors: `ClientClosed` (2010) when not connected, `ConnectionError`
	// (2001) when the connection closes before the pong, `TimeoutError`
	// (3001) when no pong arrives within `timeout_ms`, and
	// `InvalidParameter` (1005) for a `timeout_ms` of 0.
	MeasureLatency(timeoutMs *uint64) (float64, error)
	// Messages dropped because they arrived while the message queue held
	// `buffer` unread messages (`MessageOverflowRecord::DropNewest`).
	//
	// Counted from the start of the current connection (every `connect()` or
	// reconnect restarts it); after `disconnect()` it still reads the last
	// connection's count. 0 before the first `connect()`.
	MessagesDroppedTotal() uint64
	Ping(state *string) error
	QuerySubscriptions() error
	// Subscribe to a channel for one or more symbols.
	//
	// One symbol is sent as `symbol`, several as `symbols` in one frame;
	// each symbol is its own subscription afterwards. An empty list is
	// 1005 `INVALID_PARAMETER`.
	//
	// `opts` selects the session: `intraday_odd_lot` is Stock only and
	// `after_hours` is FutOpt only; setting either on the other endpoint,
	// to any value, is 1005 `INVALID_PARAMETER`.
	Subscribe(channel string, symbols []string, opts *SubscribeOptions) error
	// Unsubscribe from a channel for one or more symbols.
	//
	// Pass the same options as the `subscribe` call: an odd-lot or
	// after-hours subscription is a separate subscription from the regular
	// one.
	Unsubscribe(channel string, symbols []string, opts *SubscribeOptions) error
	// Unsubscribe by the ids the server issued in its `subscribed` messages.
	//
	// Removes the subscriptions those ids name, so a reconnect does not
	// restore them. An empty list is 1005 `INVALID_PARAMETER`.
	UnsubscribeIds(ids []string) error
}

// WebSocket client for real-time market data streaming
//
// Wraps the core WebSocketClient and forwards messages to the provided
// WebSocketListener implementation via a background task.
type WebSocketClient struct {
	ffiObject FfiObject
}

// Create a new WebSocket client for stock market data
//
// # Arguments
// * `api_key` - Fugle API key for authentication
// * `listener` - Callback interface for receiving WebSocket events
func NewWebSocketClient(apiKey string, listener WebSocketListener) *WebSocketClient {
	return FfiConverterWebSocketClientINSTANCE.Lift(rustCall(func(_uniffiStatus *C.RustCallStatus) unsafe.Pointer {
		return C.uniffi_marketdata_uniffi_fn_constructor_websocketclient_new(FfiConverterStringINSTANCE.Lower(apiKey), FfiConverterWebSocketListenerINSTANCE.Lower(listener), _uniffiStatus)
	}))
}

// Create a new WebSocket client with full configuration
//
// # Arguments
// * `api_key` - Fugle API key for authentication
// * `listener` - Callback interface for receiving WebSocket events
// * `endpoint` - The market data endpoint (Stock or FutOpt)
// * `reconnect_config` - Optional reconnection configuration
// * `health_check_config` - Optional health check configuration
func WebSocketClientNewWithConfig(apiKey string, listener WebSocketListener, endpoint WebSocketEndpoint, reconnectConfig *ReconnectConfigRecord, healthCheckConfig *HealthCheckConfigRecord) *WebSocketClient {
	return FfiConverterWebSocketClientINSTANCE.Lift(rustCall(func(_uniffiStatus *C.RustCallStatus) unsafe.Pointer {
		return C.uniffi_marketdata_uniffi_fn_constructor_websocketclient_new_with_config(FfiConverterStringINSTANCE.Lower(apiKey), FfiConverterWebSocketListenerINSTANCE.Lower(listener), FfiConverterWebSocketEndpointINSTANCE.Lower(endpoint), FfiConverterOptionalReconnectConfigRecordINSTANCE.Lower(reconnectConfig), FfiConverterOptionalHealthCheckConfigRecordINSTANCE.Lower(healthCheckConfig), _uniffiStatus)
	}))
}

// Create a new WebSocket client from whichever credential was given.
//
// Takes the same three credentials as the REST client: exactly one must
// be non-empty (an empty or whitespace-only value counts as not
// provided), otherwise this returns a `ConfigError` (code 1004). The
// auth frame then carries it as `apikey`, `token` or `sdkToken`.
// The other arguments are those of `new_with_options`.
//
// The credentials are one record rather than three arguments: with three
// more buffers than `new_with_options` the Java binding (JNA) passed
// garbage to Rust on macOS arm64.
func WebSocketClientNewWithCredentials(credentials CredentialsRecord, listener WebSocketListener, endpoint WebSocketEndpoint, baseUrl *string, reconnectConfig *ReconnectConfigRecord, healthCheckConfig *HealthCheckConfigRecord, tls *TlsConfigRecord, version *StreamingVersionRecord, messageQueue *MessageQueueConfigRecord, connection *ConnectionConfigRecord) (*WebSocketClient, error) {
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) unsafe.Pointer {
		return C.uniffi_marketdata_uniffi_fn_constructor_websocketclient_new_with_credentials(FfiConverterCredentialsRecordINSTANCE.Lower(credentials), FfiConverterWebSocketListenerINSTANCE.Lower(listener), FfiConverterWebSocketEndpointINSTANCE.Lower(endpoint), FfiConverterOptionalStringINSTANCE.Lower(baseUrl), FfiConverterOptionalReconnectConfigRecordINSTANCE.Lower(reconnectConfig), FfiConverterOptionalHealthCheckConfigRecordINSTANCE.Lower(healthCheckConfig), FfiConverterOptionalTlsConfigRecordINSTANCE.Lower(tls), FfiConverterOptionalStreamingVersionRecordINSTANCE.Lower(version), FfiConverterOptionalMessageQueueConfigRecordINSTANCE.Lower(messageQueue), FfiConverterOptionalConnectionConfigRecordINSTANCE.Lower(connection), _uniffiStatus)
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue *WebSocketClient
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterWebSocketClientINSTANCE.Lift(_uniffiRV), nil
	}
}

// Create a new WebSocket client for a specific endpoint
//
// # Arguments
// * `api_key` - Fugle API key for authentication
// * `listener` - Callback interface for receiving WebSocket events
// * `endpoint` - The market data endpoint (Stock or FutOpt)
func WebSocketClientNewWithEndpoint(apiKey string, listener WebSocketListener, endpoint WebSocketEndpoint) *WebSocketClient {
	return FfiConverterWebSocketClientINSTANCE.Lift(rustCall(func(_uniffiStatus *C.RustCallStatus) unsafe.Pointer {
		return C.uniffi_marketdata_uniffi_fn_constructor_websocketclient_new_with_endpoint(FfiConverterStringINSTANCE.Lower(apiKey), FfiConverterWebSocketListenerINSTANCE.Lower(listener), FfiConverterWebSocketEndpointINSTANCE.Lower(endpoint), _uniffiStatus)
	}))
}

// Create a new WebSocket client with full configuration including TLS.
//
// All optional parameters can be None to use defaults. This is the
// TLS-aware variant of `new_with_url` — use this when you need to
// pin a custom CA or disable cert verification.
//
// # Arguments
// * `api_key` - Fugle API key for authentication
// * `listener` - Callback interface for receiving WebSocket events
// * `endpoint` - The market data endpoint (Stock or FutOpt)
// * `base_url` - Optional base URL override
// * `reconnect_config` - Optional reconnection configuration
// * `health_check_config` - Optional health check configuration
// * `tls` - Optional TLS customization (custom CA or accept_invalid_certs)
func WebSocketClientNewWithFullConfig(apiKey string, listener WebSocketListener, endpoint WebSocketEndpoint, baseUrl *string, reconnectConfig *ReconnectConfigRecord, healthCheckConfig *HealthCheckConfigRecord, tls *TlsConfigRecord, version *StreamingVersionRecord) *WebSocketClient {
	return FfiConverterWebSocketClientINSTANCE.Lift(rustCall(func(_uniffiStatus *C.RustCallStatus) unsafe.Pointer {
		return C.uniffi_marketdata_uniffi_fn_constructor_websocketclient_new_with_full_config(FfiConverterStringINSTANCE.Lower(apiKey), FfiConverterWebSocketListenerINSTANCE.Lower(listener), FfiConverterWebSocketEndpointINSTANCE.Lower(endpoint), FfiConverterOptionalStringINSTANCE.Lower(baseUrl), FfiConverterOptionalReconnectConfigRecordINSTANCE.Lower(reconnectConfig), FfiConverterOptionalHealthCheckConfigRecordINSTANCE.Lower(healthCheckConfig), FfiConverterOptionalTlsConfigRecordINSTANCE.Lower(tls), FfiConverterOptionalStreamingVersionRecordINSTANCE.Lower(version), _uniffiStatus)
	}))
}

// Create a new WebSocket client with full configuration plus the
// message queue and connection settings.
//
// Same as `new_with_full_config`, with `message_queue` choosing what
// happens while `on_message` falls behind (None for the defaults:
// `DropNewest`, 4096 messages) and `connection` setting the
// connection's own timeouts (None for the defaults: 10 s auth
// timeout).
//
// # Arguments
// * `api_key` - Fugle API key for authentication
// * `listener` - Callback interface for receiving WebSocket events
// * `endpoint` - The market data endpoint (Stock or FutOpt)
// * `base_url` - Optional base URL override
// * `reconnect_config` - Optional reconnection configuration
// * `health_check_config` - Optional health check configuration
// * `tls` - Optional TLS customization (custom CA or accept_invalid_certs)
// * `version` - Optional per-product streaming version
// * `message_queue` - Optional message queue configuration
// * `connection` - Optional connection configuration (auth timeout)
func WebSocketClientNewWithOptions(apiKey string, listener WebSocketListener, endpoint WebSocketEndpoint, baseUrl *string, reconnectConfig *ReconnectConfigRecord, healthCheckConfig *HealthCheckConfigRecord, tls *TlsConfigRecord, version *StreamingVersionRecord, messageQueue *MessageQueueConfigRecord, connection *ConnectionConfigRecord) *WebSocketClient {
	return FfiConverterWebSocketClientINSTANCE.Lift(rustCall(func(_uniffiStatus *C.RustCallStatus) unsafe.Pointer {
		return C.uniffi_marketdata_uniffi_fn_constructor_websocketclient_new_with_options(FfiConverterStringINSTANCE.Lower(apiKey), FfiConverterWebSocketListenerINSTANCE.Lower(listener), FfiConverterWebSocketEndpointINSTANCE.Lower(endpoint), FfiConverterOptionalStringINSTANCE.Lower(baseUrl), FfiConverterOptionalReconnectConfigRecordINSTANCE.Lower(reconnectConfig), FfiConverterOptionalHealthCheckConfigRecordINSTANCE.Lower(healthCheckConfig), FfiConverterOptionalTlsConfigRecordINSTANCE.Lower(tls), FfiConverterOptionalStreamingVersionRecordINSTANCE.Lower(version), FfiConverterOptionalMessageQueueConfigRecordINSTANCE.Lower(messageQueue), FfiConverterOptionalConnectionConfigRecordINSTANCE.Lower(connection), _uniffiStatus)
	}))
}

// Create a new WebSocket client with full configuration including custom base URL
func WebSocketClientNewWithUrl(apiKey string, listener WebSocketListener, endpoint WebSocketEndpoint, baseUrl string, reconnectConfig *ReconnectConfigRecord, healthCheckConfig *HealthCheckConfigRecord) *WebSocketClient {
	return FfiConverterWebSocketClientINSTANCE.Lift(rustCall(func(_uniffiStatus *C.RustCallStatus) unsafe.Pointer {
		return C.uniffi_marketdata_uniffi_fn_constructor_websocketclient_new_with_url(FfiConverterStringINSTANCE.Lower(apiKey), FfiConverterWebSocketListenerINSTANCE.Lower(listener), FfiConverterWebSocketEndpointINSTANCE.Lower(endpoint), FfiConverterStringINSTANCE.Lower(baseUrl), FfiConverterOptionalReconnectConfigRecordINSTANCE.Lower(reconnectConfig), FfiConverterOptionalHealthCheckConfigRecordINSTANCE.Lower(healthCheckConfig), _uniffiStatus)
	}))
}

// Connect to the WebSocket server and authenticate.
//
// Refused with code 2011 (`ALREADY_CONNECTED`) while connected or while
// another `connect()` is in progress.
//
// During an automatic reconnect it opens no connection of its own: it
// waits for that reconnect and returns once the connection is back and
// the subscriptions are re-sent, so a `subscribe()` afterwards follows
// them. The wait fails with 2010 (`ClientClosed`) if `disconnect()` is
// called, 3005 (`RECONNECT_FAILED`) if the reconnect runs out of
// attempts, and `AuthError` (2002) if its credentials are rejected.
// Called from a listener method, it holds up the listener until the
// reconnect ends.
func (_self *WebSocketClient) Connect() error {
	_pointer := _self.ffiObject.incrementPointer("*WebSocketClient")
	defer _self.ffiObject.decrementPointer()
	_, err := uniffiRustCallAsync[MarketDataError](
		FfiConverterMarketDataErrorINSTANCE,
		// completeFn
		func(handle C.uint64_t, status *C.RustCallStatus) struct{} {
			C.ffi_marketdata_uniffi_rust_future_complete_void(handle, status)
			return struct{}{}
		},
		// liftFn
		func(_ struct{}) struct{} { return struct{}{} },
		C.uniffi_marketdata_uniffi_fn_method_websocketclient_connect(
			_pointer),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_void(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_void(handle)
		},
	)

	if err == nil {
		return nil
	}

	return err
}

// Disconnect, returning once the listener has handled the connection's
// remaining events, `on_disconnected` included.
//
// There is no timeout on that wait: a listener method that blocks keeps
// `disconnect()` waiting for as long as it does.
//
// Called from a listener method, it returns without that wait: those
// events are delivered on the thread running the method, after it
// returns.
func (_self *WebSocketClient) Disconnect() {
	_pointer := _self.ffiObject.incrementPointer("*WebSocketClient")
	defer _self.ffiObject.decrementPointer()
	uniffiRustCallAsync[error](
		nil,
		// completeFn
		func(handle C.uint64_t, status *C.RustCallStatus) struct{} {
			C.ffi_marketdata_uniffi_rust_future_complete_void(handle, status)
			return struct{}{}
		},
		// liftFn
		func(_ struct{}) struct{} { return struct{}{} },
		C.uniffi_marketdata_uniffi_fn_method_websocketclient_disconnect(
			_pointer),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_void(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_void(handle)
		},
	)

}

// Check if the connection has ended
//
// Reads core's connection state: true after `disconnect()`, and after
// the server closes the connection when no reconnect follows (disabled
// or attempts exhausted). False while reconnecting and before the first
// `connect()`.
func (_self *WebSocketClient) IsClosed() bool {
	_pointer := _self.ffiObject.incrementPointer("*WebSocketClient")
	defer _self.ffiObject.decrementPointer()
	return FfiConverterBoolINSTANCE.Lift(rustCall(func(_uniffiStatus *C.RustCallStatus) C.int8_t {
		return C.uniffi_marketdata_uniffi_fn_method_websocketclient_is_closed(
			_pointer, _uniffiStatus)
	}))
}

// Check if the client is currently connected
//
// Reads core's connection state, so it is false while reconnecting and
// right after the connection drops, without waiting for the event thread.
func (_self *WebSocketClient) IsConnected() bool {
	_pointer := _self.ffiObject.incrementPointer("*WebSocketClient")
	defer _self.ffiObject.decrementPointer()
	return FfiConverterBoolINSTANCE.Lift(rustCall(func(_uniffiStatus *C.RustCallStatus) C.int8_t {
		return C.uniffi_marketdata_uniffi_fn_method_websocketclient_is_connected(
			_pointer, _uniffiStatus)
	}))
}

// Measure the round trip to the server: send a ping, wait for its pong,
// and return the time between the two in milliseconds.
//
// Unlike `ping()` (fire and forget, pong delivered to `on_message`),
// this waits for the answer, and its pong is not delivered. Works
// whether or not `probe_enabled` is set, and sends nothing in the
// background. `timeout_ms` defaults to 5000 when `None`.
//
// Errors: `ClientClosed` (2010) when not connected, `ConnectionError`
// (2001) when the connection closes before the pong, `TimeoutError`
// (3001) when no pong arrives within `timeout_ms`, and
// `InvalidParameter` (1005) for a `timeout_ms` of 0.
func (_self *WebSocketClient) MeasureLatency(timeoutMs *uint64) (float64, error) {
	_pointer := _self.ffiObject.incrementPointer("*WebSocketClient")
	defer _self.ffiObject.decrementPointer()
	res, err := uniffiRustCallAsync[MarketDataError](
		FfiConverterMarketDataErrorINSTANCE,
		// completeFn
		func(handle C.uint64_t, status *C.RustCallStatus) C.double {
			res := C.ffi_marketdata_uniffi_rust_future_complete_f64(handle, status)
			return res
		},
		// liftFn
		func(ffi C.double) float64 {
			return FfiConverterFloat64INSTANCE.Lift(ffi)
		},
		C.uniffi_marketdata_uniffi_fn_method_websocketclient_measure_latency(
			_pointer, FfiConverterOptionalUint64INSTANCE.Lower(timeoutMs)),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_f64(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_f64(handle)
		},
	)

	if err == nil {
		return res, nil
	}

	return res, err
}

// Messages dropped because they arrived while the message queue held
// `buffer` unread messages (`MessageOverflowRecord::DropNewest`).
//
// Counted from the start of the current connection (every `connect()` or
// reconnect restarts it); after `disconnect()` it still reads the last
// connection's count. 0 before the first `connect()`.
func (_self *WebSocketClient) MessagesDroppedTotal() uint64 {
	_pointer := _self.ffiObject.incrementPointer("*WebSocketClient")
	defer _self.ffiObject.decrementPointer()
	return FfiConverterUint64INSTANCE.Lift(rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint64_t {
		return C.uniffi_marketdata_uniffi_fn_method_websocketclient_messages_dropped_total(
			_pointer, _uniffiStatus)
	}))
}

func (_self *WebSocketClient) Ping(state *string) error {
	_pointer := _self.ffiObject.incrementPointer("*WebSocketClient")
	defer _self.ffiObject.decrementPointer()
	_, err := uniffiRustCallAsync[MarketDataError](
		FfiConverterMarketDataErrorINSTANCE,
		// completeFn
		func(handle C.uint64_t, status *C.RustCallStatus) struct{} {
			C.ffi_marketdata_uniffi_rust_future_complete_void(handle, status)
			return struct{}{}
		},
		// liftFn
		func(_ struct{}) struct{} { return struct{}{} },
		C.uniffi_marketdata_uniffi_fn_method_websocketclient_ping(
			_pointer, FfiConverterOptionalStringINSTANCE.Lower(state)),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_void(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_void(handle)
		},
	)

	if err == nil {
		return nil
	}

	return err
}

func (_self *WebSocketClient) QuerySubscriptions() error {
	_pointer := _self.ffiObject.incrementPointer("*WebSocketClient")
	defer _self.ffiObject.decrementPointer()
	_, err := uniffiRustCallAsync[MarketDataError](
		FfiConverterMarketDataErrorINSTANCE,
		// completeFn
		func(handle C.uint64_t, status *C.RustCallStatus) struct{} {
			C.ffi_marketdata_uniffi_rust_future_complete_void(handle, status)
			return struct{}{}
		},
		// liftFn
		func(_ struct{}) struct{} { return struct{}{} },
		C.uniffi_marketdata_uniffi_fn_method_websocketclient_query_subscriptions(
			_pointer),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_void(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_void(handle)
		},
	)

	if err == nil {
		return nil
	}

	return err
}

// Subscribe to a channel for one or more symbols.
//
// One symbol is sent as `symbol`, several as `symbols` in one frame;
// each symbol is its own subscription afterwards. An empty list is
// 1005 `INVALID_PARAMETER`.
//
// `opts` selects the session: `intraday_odd_lot` is Stock only and
// `after_hours` is FutOpt only; setting either on the other endpoint,
// to any value, is 1005 `INVALID_PARAMETER`.
func (_self *WebSocketClient) Subscribe(channel string, symbols []string, opts *SubscribeOptions) error {
	_pointer := _self.ffiObject.incrementPointer("*WebSocketClient")
	defer _self.ffiObject.decrementPointer()
	_, err := uniffiRustCallAsync[MarketDataError](
		FfiConverterMarketDataErrorINSTANCE,
		// completeFn
		func(handle C.uint64_t, status *C.RustCallStatus) struct{} {
			C.ffi_marketdata_uniffi_rust_future_complete_void(handle, status)
			return struct{}{}
		},
		// liftFn
		func(_ struct{}) struct{} { return struct{}{} },
		C.uniffi_marketdata_uniffi_fn_method_websocketclient_subscribe(
			_pointer, FfiConverterStringINSTANCE.Lower(channel), FfiConverterSequenceStringINSTANCE.Lower(symbols), FfiConverterOptionalSubscribeOptionsINSTANCE.Lower(opts)),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_void(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_void(handle)
		},
	)

	if err == nil {
		return nil
	}

	return err
}

// Unsubscribe from a channel for one or more symbols.
//
// Pass the same options as the `subscribe` call: an odd-lot or
// after-hours subscription is a separate subscription from the regular
// one.
func (_self *WebSocketClient) Unsubscribe(channel string, symbols []string, opts *SubscribeOptions) error {
	_pointer := _self.ffiObject.incrementPointer("*WebSocketClient")
	defer _self.ffiObject.decrementPointer()
	_, err := uniffiRustCallAsync[MarketDataError](
		FfiConverterMarketDataErrorINSTANCE,
		// completeFn
		func(handle C.uint64_t, status *C.RustCallStatus) struct{} {
			C.ffi_marketdata_uniffi_rust_future_complete_void(handle, status)
			return struct{}{}
		},
		// liftFn
		func(_ struct{}) struct{} { return struct{}{} },
		C.uniffi_marketdata_uniffi_fn_method_websocketclient_unsubscribe(
			_pointer, FfiConverterStringINSTANCE.Lower(channel), FfiConverterSequenceStringINSTANCE.Lower(symbols), FfiConverterOptionalSubscribeOptionsINSTANCE.Lower(opts)),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_void(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_void(handle)
		},
	)

	if err == nil {
		return nil
	}

	return err
}

// Unsubscribe by the ids the server issued in its `subscribed` messages.
//
// Removes the subscriptions those ids name, so a reconnect does not
// restore them. An empty list is 1005 `INVALID_PARAMETER`.
func (_self *WebSocketClient) UnsubscribeIds(ids []string) error {
	_pointer := _self.ffiObject.incrementPointer("*WebSocketClient")
	defer _self.ffiObject.decrementPointer()
	_, err := uniffiRustCallAsync[MarketDataError](
		FfiConverterMarketDataErrorINSTANCE,
		// completeFn
		func(handle C.uint64_t, status *C.RustCallStatus) struct{} {
			C.ffi_marketdata_uniffi_rust_future_complete_void(handle, status)
			return struct{}{}
		},
		// liftFn
		func(_ struct{}) struct{} { return struct{}{} },
		C.uniffi_marketdata_uniffi_fn_method_websocketclient_unsubscribe_ids(
			_pointer, FfiConverterSequenceStringINSTANCE.Lower(ids)),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_void(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_void(handle)
		},
	)

	if err == nil {
		return nil
	}

	return err
}
func (object *WebSocketClient) Destroy() {
	runtime.SetFinalizer(object, nil)
	object.ffiObject.destroy()
}

type FfiConverterWebSocketClient struct{}

var FfiConverterWebSocketClientINSTANCE = FfiConverterWebSocketClient{}

func (c FfiConverterWebSocketClient) Lift(pointer unsafe.Pointer) *WebSocketClient {
	result := &WebSocketClient{
		newFfiObject(
			pointer,
			func(pointer unsafe.Pointer, status *C.RustCallStatus) unsafe.Pointer {
				return C.uniffi_marketdata_uniffi_fn_clone_websocketclient(pointer, status)
			},
			func(pointer unsafe.Pointer, status *C.RustCallStatus) {
				C.uniffi_marketdata_uniffi_fn_free_websocketclient(pointer, status)
			},
		),
	}
	runtime.SetFinalizer(result, (*WebSocketClient).Destroy)
	return result
}

func (c FfiConverterWebSocketClient) Read(reader io.Reader) *WebSocketClient {
	return c.Lift(unsafe.Pointer(uintptr(readUint64(reader))))
}

func (c FfiConverterWebSocketClient) Lower(value *WebSocketClient) unsafe.Pointer {
	// TODO: this is bad - all synchronization from ObjectRuntime.go is discarded here,
	// because the pointer will be decremented immediately after this function returns,
	// and someone will be left holding onto a non-locked pointer.
	pointer := value.ffiObject.incrementPointer("*WebSocketClient")
	defer value.ffiObject.decrementPointer()
	return pointer

}

func (c FfiConverterWebSocketClient) Write(writer io.Writer, value *WebSocketClient) {
	writeUint64(writer, uint64(uintptr(c.Lower(value))))
}

type FfiDestroyerWebSocketClient struct{}

func (_ FfiDestroyerWebSocketClient) Destroy(value *WebSocketClient) {
	value.Destroy()
}

// Callback interface for WebSocket events
//
// Foreign code (C#, Go) implements this trait to receive WebSocket events.
// The implementation must be thread-safe (Send + Sync) as callbacks may be
// invoked from background tokio tasks.
//
// # Example (C#)
//
// ```csharp
// class MyListener : IWebSocketListener {
// public void OnConnected() {
// Console.WriteLine("Connected!");
// }
// public void OnAuthenticated(string? dataJson) {
// Console.WriteLine("Authenticated");
// }
// public void OnUnauthenticated(string? dataJson) {
// Console.WriteLine($"Rejected: {dataJson}");
// }
// public void OnDisconnected(bool willReconnect) {
// Console.WriteLine($"Disconnected (will reconnect: {willReconnect})");
// }
// public void OnMessage(StreamMessage message) {
// Console.WriteLine($"Got {message.Event} for {message.Symbol}");
// }
// public void OnError(ErrorInfo error) {
// Console.WriteLine($"Error: {error.Message}");
// }
// }
// ```
type WebSocketListener interface {
	// Called when the transport is established, before the server has
	// answered the auth frame. Fires again on every successful reconnect.
	// Wait for `on_authenticated` before treating the connection as usable.
	OnConnected()
	// Called when the server accepts the credentials.
	//
	// `data_json` is the `data` member of the server's `authenticated`
	// frame, still encoded as JSON, or `None` when the frame has none.
	OnAuthenticated(dataJson *string)
	// Called when the server rejects the credentials: it answered the auth
	// frame with an `error` of code 1000. On `connect()` the call also
	// fails with an auth error; no `on_error` is emitted for the rejection.
	// During an auto-reconnect, `on_reconnect_failed` follows at once: the
	// same credentials would be rejected again, so the client stops and
	// stays closed (#201). An auth-phase `error` with any other code (1011
	// auth service unavailable, 1004 no auth request received) is not a
	// rejection: it is reported to `on_error` (code 2001) and a reconnect
	// goes on.
	//
	// `data_json` is the `data` member of the server's rejection frame
	// (the server's message is under `message`), still encoded as JSON, or
	// `None` when the frame has none.
	OnUnauthenticated(dataJson *string)
	// Called when the connection is closed, at most once per connection.
	//
	// `will_reconnect` is `true` when the client will try to reconnect
	// (`on_reconnecting` follows unless `disconnect()` is called first) and
	// `false` when this connection's lifecycle has ended.
	OnDisconnected(willReconnect bool)
	// Called when a message is received
	OnMessage(message StreamMessage)
	// Called when an error occurs
	//
	// Also carries one warning, code 3006 (`RECONNECT_CONFLICT`), at most
	// once per client: `connect()` was called less than 30 seconds after
	// `disconnect()` closed a connection that automatic reconnect had
	// restored less than 30 seconds before, which is what code that also
	// reconnects on its own does (#226, #242). It comes from that
	// `connect()`, which goes ahead; the message says how to resolve it.
	OnError(error ErrorInfo)
	// Called when a reconnection attempt starts
	OnReconnecting(attempt uint32)
	// Called when the reconnect gives up: all attempts are exhausted, or an
	// attempt's credentials were rejected (`on_unauthenticated` precedes
	// it, #201). Terminal: no further lifecycle callbacks follow for this
	// connection.
	OnReconnectFailed(attempts uint32)
	// Called when messages were dropped because `on_message` fell behind
	// while the client's message queue held `buffer` unread messages
	// (`MessageOverflowRecord::DropNewest`).
	//
	// `count` is the number dropped since the previous call. The first drop
	// on a connection is reported at once, later ones at most once per
	// second, and the rest before `on_disconnected`. The connection's total
	// is `WebSocketClient::messages_dropped_total()`.
	OnMessagesDropped(count uint64)
}

// Callback interface for WebSocket events
//
// Foreign code (C#, Go) implements this trait to receive WebSocket events.
// The implementation must be thread-safe (Send + Sync) as callbacks may be
// invoked from background tokio tasks.
//
// # Example (C#)
//
// ```csharp
// class MyListener : IWebSocketListener {
// public void OnConnected() {
// Console.WriteLine("Connected!");
// }
// public void OnAuthenticated(string? dataJson) {
// Console.WriteLine("Authenticated");
// }
// public void OnUnauthenticated(string? dataJson) {
// Console.WriteLine($"Rejected: {dataJson}");
// }
// public void OnDisconnected(bool willReconnect) {
// Console.WriteLine($"Disconnected (will reconnect: {willReconnect})");
// }
// public void OnMessage(StreamMessage message) {
// Console.WriteLine($"Got {message.Event} for {message.Symbol}");
// }
// public void OnError(ErrorInfo error) {
// Console.WriteLine($"Error: {error.Message}");
// }
// }
// ```
type WebSocketListenerImpl struct {
	ffiObject FfiObject
}

// Called when the transport is established, before the server has
// answered the auth frame. Fires again on every successful reconnect.
// Wait for `on_authenticated` before treating the connection as usable.
func (_self *WebSocketListenerImpl) OnConnected() {
	_pointer := _self.ffiObject.incrementPointer("WebSocketListener")
	defer _self.ffiObject.decrementPointer()
	rustCall(func(_uniffiStatus *C.RustCallStatus) bool {
		C.uniffi_marketdata_uniffi_fn_method_websocketlistener_on_connected(
			_pointer, _uniffiStatus)
		return false
	})
}

// Called when the server accepts the credentials.
//
// `data_json` is the `data` member of the server's `authenticated`
// frame, still encoded as JSON, or `None` when the frame has none.
func (_self *WebSocketListenerImpl) OnAuthenticated(dataJson *string) {
	_pointer := _self.ffiObject.incrementPointer("WebSocketListener")
	defer _self.ffiObject.decrementPointer()
	rustCall(func(_uniffiStatus *C.RustCallStatus) bool {
		C.uniffi_marketdata_uniffi_fn_method_websocketlistener_on_authenticated(
			_pointer, FfiConverterOptionalStringINSTANCE.Lower(dataJson), _uniffiStatus)
		return false
	})
}

// Called when the server rejects the credentials: it answered the auth
// frame with an `error` of code 1000. On `connect()` the call also
// fails with an auth error; no `on_error` is emitted for the rejection.
// During an auto-reconnect, `on_reconnect_failed` follows at once: the
// same credentials would be rejected again, so the client stops and
// stays closed (#201). An auth-phase `error` with any other code (1011
// auth service unavailable, 1004 no auth request received) is not a
// rejection: it is reported to `on_error` (code 2001) and a reconnect
// goes on.
//
// `data_json` is the `data` member of the server's rejection frame
// (the server's message is under `message`), still encoded as JSON, or
// `None` when the frame has none.
func (_self *WebSocketListenerImpl) OnUnauthenticated(dataJson *string) {
	_pointer := _self.ffiObject.incrementPointer("WebSocketListener")
	defer _self.ffiObject.decrementPointer()
	rustCall(func(_uniffiStatus *C.RustCallStatus) bool {
		C.uniffi_marketdata_uniffi_fn_method_websocketlistener_on_unauthenticated(
			_pointer, FfiConverterOptionalStringINSTANCE.Lower(dataJson), _uniffiStatus)
		return false
	})
}

// Called when the connection is closed, at most once per connection.
//
// `will_reconnect` is `true` when the client will try to reconnect
// (`on_reconnecting` follows unless `disconnect()` is called first) and
// `false` when this connection's lifecycle has ended.
func (_self *WebSocketListenerImpl) OnDisconnected(willReconnect bool) {
	_pointer := _self.ffiObject.incrementPointer("WebSocketListener")
	defer _self.ffiObject.decrementPointer()
	rustCall(func(_uniffiStatus *C.RustCallStatus) bool {
		C.uniffi_marketdata_uniffi_fn_method_websocketlistener_on_disconnected(
			_pointer, FfiConverterBoolINSTANCE.Lower(willReconnect), _uniffiStatus)
		return false
	})
}

// Called when a message is received
func (_self *WebSocketListenerImpl) OnMessage(message StreamMessage) {
	_pointer := _self.ffiObject.incrementPointer("WebSocketListener")
	defer _self.ffiObject.decrementPointer()
	rustCall(func(_uniffiStatus *C.RustCallStatus) bool {
		C.uniffi_marketdata_uniffi_fn_method_websocketlistener_on_message(
			_pointer, FfiConverterStreamMessageINSTANCE.Lower(message), _uniffiStatus)
		return false
	})
}

// Called when an error occurs
//
// Also carries one warning, code 3006 (`RECONNECT_CONFLICT`), at most
// once per client: `connect()` was called less than 30 seconds after
// `disconnect()` closed a connection that automatic reconnect had
// restored less than 30 seconds before, which is what code that also
// reconnects on its own does (#226, #242). It comes from that
// `connect()`, which goes ahead; the message says how to resolve it.
func (_self *WebSocketListenerImpl) OnError(error ErrorInfo) {
	_pointer := _self.ffiObject.incrementPointer("WebSocketListener")
	defer _self.ffiObject.decrementPointer()
	rustCall(func(_uniffiStatus *C.RustCallStatus) bool {
		C.uniffi_marketdata_uniffi_fn_method_websocketlistener_on_error(
			_pointer, FfiConverterErrorInfoINSTANCE.Lower(error), _uniffiStatus)
		return false
	})
}

// Called when a reconnection attempt starts
func (_self *WebSocketListenerImpl) OnReconnecting(attempt uint32) {
	_pointer := _self.ffiObject.incrementPointer("WebSocketListener")
	defer _self.ffiObject.decrementPointer()
	rustCall(func(_uniffiStatus *C.RustCallStatus) bool {
		C.uniffi_marketdata_uniffi_fn_method_websocketlistener_on_reconnecting(
			_pointer, FfiConverterUint32INSTANCE.Lower(attempt), _uniffiStatus)
		return false
	})
}

// Called when the reconnect gives up: all attempts are exhausted, or an
// attempt's credentials were rejected (`on_unauthenticated` precedes
// it, #201). Terminal: no further lifecycle callbacks follow for this
// connection.
func (_self *WebSocketListenerImpl) OnReconnectFailed(attempts uint32) {
	_pointer := _self.ffiObject.incrementPointer("WebSocketListener")
	defer _self.ffiObject.decrementPointer()
	rustCall(func(_uniffiStatus *C.RustCallStatus) bool {
		C.uniffi_marketdata_uniffi_fn_method_websocketlistener_on_reconnect_failed(
			_pointer, FfiConverterUint32INSTANCE.Lower(attempts), _uniffiStatus)
		return false
	})
}

// Called when messages were dropped because `on_message` fell behind
// while the client's message queue held `buffer` unread messages
// (`MessageOverflowRecord::DropNewest`).
//
// `count` is the number dropped since the previous call. The first drop
// on a connection is reported at once, later ones at most once per
// second, and the rest before `on_disconnected`. The connection's total
// is `WebSocketClient::messages_dropped_total()`.
func (_self *WebSocketListenerImpl) OnMessagesDropped(count uint64) {
	_pointer := _self.ffiObject.incrementPointer("WebSocketListener")
	defer _self.ffiObject.decrementPointer()
	rustCall(func(_uniffiStatus *C.RustCallStatus) bool {
		C.uniffi_marketdata_uniffi_fn_method_websocketlistener_on_messages_dropped(
			_pointer, FfiConverterUint64INSTANCE.Lower(count), _uniffiStatus)
		return false
	})
}
func (object *WebSocketListenerImpl) Destroy() {
	runtime.SetFinalizer(object, nil)
	object.ffiObject.destroy()
}

type FfiConverterWebSocketListener struct {
	handleMap *concurrentHandleMap[WebSocketListener]
}

var FfiConverterWebSocketListenerINSTANCE = FfiConverterWebSocketListener{
	handleMap: newConcurrentHandleMap[WebSocketListener](),
}

func (c FfiConverterWebSocketListener) Lift(pointer unsafe.Pointer) WebSocketListener {
	result := &WebSocketListenerImpl{
		newFfiObject(
			pointer,
			func(pointer unsafe.Pointer, status *C.RustCallStatus) unsafe.Pointer {
				return C.uniffi_marketdata_uniffi_fn_clone_websocketlistener(pointer, status)
			},
			func(pointer unsafe.Pointer, status *C.RustCallStatus) {
				C.uniffi_marketdata_uniffi_fn_free_websocketlistener(pointer, status)
			},
		),
	}
	runtime.SetFinalizer(result, (*WebSocketListenerImpl).Destroy)
	return result
}

func (c FfiConverterWebSocketListener) Read(reader io.Reader) WebSocketListener {
	return c.Lift(unsafe.Pointer(uintptr(readUint64(reader))))
}

func (c FfiConverterWebSocketListener) Lower(value WebSocketListener) unsafe.Pointer {
	// TODO: this is bad - all synchronization from ObjectRuntime.go is discarded here,
	// because the pointer will be decremented immediately after this function returns,
	// and someone will be left holding onto a non-locked pointer.
	pointer := unsafe.Pointer(uintptr(c.handleMap.insert(value)))
	return pointer

}

func (c FfiConverterWebSocketListener) Write(writer io.Writer, value WebSocketListener) {
	writeUint64(writer, uint64(uintptr(c.Lower(value))))
}

type FfiDestroyerWebSocketListener struct{}

func (_ FfiDestroyerWebSocketListener) Destroy(value WebSocketListener) {
	if val, ok := value.(*WebSocketListenerImpl); ok {
		val.Destroy()
	} else {
		panic("Expected *WebSocketListenerImpl")
	}
}

type uniffiCallbackResult C.int8_t

const (
	uniffiIdxCallbackFree               uniffiCallbackResult = 0
	uniffiCallbackResultSuccess         uniffiCallbackResult = 0
	uniffiCallbackResultError           uniffiCallbackResult = 1
	uniffiCallbackUnexpectedResultError uniffiCallbackResult = 2
	uniffiCallbackCancelled             uniffiCallbackResult = 3
)

type concurrentHandleMap[T any] struct {
	handles       map[uint64]T
	currentHandle uint64
	lock          sync.RWMutex
}

func newConcurrentHandleMap[T any]() *concurrentHandleMap[T] {
	return &concurrentHandleMap[T]{
		handles: map[uint64]T{},
	}
}

func (cm *concurrentHandleMap[T]) insert(obj T) uint64 {
	cm.lock.Lock()
	defer cm.lock.Unlock()

	cm.currentHandle = cm.currentHandle + 1
	cm.handles[cm.currentHandle] = obj
	return cm.currentHandle
}

func (cm *concurrentHandleMap[T]) remove(handle uint64) {
	cm.lock.Lock()
	defer cm.lock.Unlock()

	delete(cm.handles, handle)
}

func (cm *concurrentHandleMap[T]) tryGet(handle uint64) (T, bool) {
	cm.lock.RLock()
	defer cm.lock.RUnlock()

	val, ok := cm.handles[handle]
	return val, ok
}

//export marketdata_uniffi_cgo_dispatchCallbackInterfaceWebSocketListenerMethod0
func marketdata_uniffi_cgo_dispatchCallbackInterfaceWebSocketListenerMethod0(uniffiHandle C.uint64_t, uniffiOutReturn *C.void, callStatus *C.RustCallStatus) {
	handle := uint64(uniffiHandle)
	uniffiObj, ok := FfiConverterWebSocketListenerINSTANCE.handleMap.tryGet(handle)
	if !ok {
		panic(fmt.Errorf("no callback in handle map: %d", handle))
	}

	uniffiObj.OnConnected()

}

//export marketdata_uniffi_cgo_dispatchCallbackInterfaceWebSocketListenerMethod1
func marketdata_uniffi_cgo_dispatchCallbackInterfaceWebSocketListenerMethod1(uniffiHandle C.uint64_t, dataJson C.RustBuffer, uniffiOutReturn *C.void, callStatus *C.RustCallStatus) {
	handle := uint64(uniffiHandle)
	uniffiObj, ok := FfiConverterWebSocketListenerINSTANCE.handleMap.tryGet(handle)
	if !ok {
		panic(fmt.Errorf("no callback in handle map: %d", handle))
	}

	uniffiObj.OnAuthenticated(
		FfiConverterOptionalStringINSTANCE.Lift(GoRustBuffer{
			inner: dataJson,
		}),
	)

}

//export marketdata_uniffi_cgo_dispatchCallbackInterfaceWebSocketListenerMethod2
func marketdata_uniffi_cgo_dispatchCallbackInterfaceWebSocketListenerMethod2(uniffiHandle C.uint64_t, dataJson C.RustBuffer, uniffiOutReturn *C.void, callStatus *C.RustCallStatus) {
	handle := uint64(uniffiHandle)
	uniffiObj, ok := FfiConverterWebSocketListenerINSTANCE.handleMap.tryGet(handle)
	if !ok {
		panic(fmt.Errorf("no callback in handle map: %d", handle))
	}

	uniffiObj.OnUnauthenticated(
		FfiConverterOptionalStringINSTANCE.Lift(GoRustBuffer{
			inner: dataJson,
		}),
	)

}

//export marketdata_uniffi_cgo_dispatchCallbackInterfaceWebSocketListenerMethod3
func marketdata_uniffi_cgo_dispatchCallbackInterfaceWebSocketListenerMethod3(uniffiHandle C.uint64_t, willReconnect C.int8_t, uniffiOutReturn *C.void, callStatus *C.RustCallStatus) {
	handle := uint64(uniffiHandle)
	uniffiObj, ok := FfiConverterWebSocketListenerINSTANCE.handleMap.tryGet(handle)
	if !ok {
		panic(fmt.Errorf("no callback in handle map: %d", handle))
	}

	uniffiObj.OnDisconnected(
		FfiConverterBoolINSTANCE.Lift(willReconnect),
	)

}

//export marketdata_uniffi_cgo_dispatchCallbackInterfaceWebSocketListenerMethod4
func marketdata_uniffi_cgo_dispatchCallbackInterfaceWebSocketListenerMethod4(uniffiHandle C.uint64_t, message C.RustBuffer, uniffiOutReturn *C.void, callStatus *C.RustCallStatus) {
	handle := uint64(uniffiHandle)
	uniffiObj, ok := FfiConverterWebSocketListenerINSTANCE.handleMap.tryGet(handle)
	if !ok {
		panic(fmt.Errorf("no callback in handle map: %d", handle))
	}

	uniffiObj.OnMessage(
		FfiConverterStreamMessageINSTANCE.Lift(GoRustBuffer{
			inner: message,
		}),
	)

}

//export marketdata_uniffi_cgo_dispatchCallbackInterfaceWebSocketListenerMethod5
func marketdata_uniffi_cgo_dispatchCallbackInterfaceWebSocketListenerMethod5(uniffiHandle C.uint64_t, error C.RustBuffer, uniffiOutReturn *C.void, callStatus *C.RustCallStatus) {
	handle := uint64(uniffiHandle)
	uniffiObj, ok := FfiConverterWebSocketListenerINSTANCE.handleMap.tryGet(handle)
	if !ok {
		panic(fmt.Errorf("no callback in handle map: %d", handle))
	}

	uniffiObj.OnError(
		FfiConverterErrorInfoINSTANCE.Lift(GoRustBuffer{
			inner: error,
		}),
	)

}

//export marketdata_uniffi_cgo_dispatchCallbackInterfaceWebSocketListenerMethod6
func marketdata_uniffi_cgo_dispatchCallbackInterfaceWebSocketListenerMethod6(uniffiHandle C.uint64_t, attempt C.uint32_t, uniffiOutReturn *C.void, callStatus *C.RustCallStatus) {
	handle := uint64(uniffiHandle)
	uniffiObj, ok := FfiConverterWebSocketListenerINSTANCE.handleMap.tryGet(handle)
	if !ok {
		panic(fmt.Errorf("no callback in handle map: %d", handle))
	}

	uniffiObj.OnReconnecting(
		FfiConverterUint32INSTANCE.Lift(attempt),
	)

}

//export marketdata_uniffi_cgo_dispatchCallbackInterfaceWebSocketListenerMethod7
func marketdata_uniffi_cgo_dispatchCallbackInterfaceWebSocketListenerMethod7(uniffiHandle C.uint64_t, attempts C.uint32_t, uniffiOutReturn *C.void, callStatus *C.RustCallStatus) {
	handle := uint64(uniffiHandle)
	uniffiObj, ok := FfiConverterWebSocketListenerINSTANCE.handleMap.tryGet(handle)
	if !ok {
		panic(fmt.Errorf("no callback in handle map: %d", handle))
	}

	uniffiObj.OnReconnectFailed(
		FfiConverterUint32INSTANCE.Lift(attempts),
	)

}

//export marketdata_uniffi_cgo_dispatchCallbackInterfaceWebSocketListenerMethod8
func marketdata_uniffi_cgo_dispatchCallbackInterfaceWebSocketListenerMethod8(uniffiHandle C.uint64_t, count C.uint64_t, uniffiOutReturn *C.void, callStatus *C.RustCallStatus) {
	handle := uint64(uniffiHandle)
	uniffiObj, ok := FfiConverterWebSocketListenerINSTANCE.handleMap.tryGet(handle)
	if !ok {
		panic(fmt.Errorf("no callback in handle map: %d", handle))
	}

	uniffiObj.OnMessagesDropped(
		FfiConverterUint64INSTANCE.Lift(count),
	)

}

var UniffiVTableCallbackInterfaceWebSocketListenerINSTANCE = C.UniffiVTableCallbackInterfaceWebSocketListener{
	onConnected:       (C.UniffiCallbackInterfaceWebSocketListenerMethod0)(C.marketdata_uniffi_cgo_dispatchCallbackInterfaceWebSocketListenerMethod0),
	onAuthenticated:   (C.UniffiCallbackInterfaceWebSocketListenerMethod1)(C.marketdata_uniffi_cgo_dispatchCallbackInterfaceWebSocketListenerMethod1),
	onUnauthenticated: (C.UniffiCallbackInterfaceWebSocketListenerMethod2)(C.marketdata_uniffi_cgo_dispatchCallbackInterfaceWebSocketListenerMethod2),
	onDisconnected:    (C.UniffiCallbackInterfaceWebSocketListenerMethod3)(C.marketdata_uniffi_cgo_dispatchCallbackInterfaceWebSocketListenerMethod3),
	onMessage:         (C.UniffiCallbackInterfaceWebSocketListenerMethod4)(C.marketdata_uniffi_cgo_dispatchCallbackInterfaceWebSocketListenerMethod4),
	onError:           (C.UniffiCallbackInterfaceWebSocketListenerMethod5)(C.marketdata_uniffi_cgo_dispatchCallbackInterfaceWebSocketListenerMethod5),
	onReconnecting:    (C.UniffiCallbackInterfaceWebSocketListenerMethod6)(C.marketdata_uniffi_cgo_dispatchCallbackInterfaceWebSocketListenerMethod6),
	onReconnectFailed: (C.UniffiCallbackInterfaceWebSocketListenerMethod7)(C.marketdata_uniffi_cgo_dispatchCallbackInterfaceWebSocketListenerMethod7),
	onMessagesDropped: (C.UniffiCallbackInterfaceWebSocketListenerMethod8)(C.marketdata_uniffi_cgo_dispatchCallbackInterfaceWebSocketListenerMethod8),

	uniffiFree: (C.UniffiCallbackInterfaceFree)(C.marketdata_uniffi_cgo_dispatchCallbackInterfaceWebSocketListenerFree),
}

//export marketdata_uniffi_cgo_dispatchCallbackInterfaceWebSocketListenerFree
func marketdata_uniffi_cgo_dispatchCallbackInterfaceWebSocketListenerFree(handle C.uint64_t) {
	FfiConverterWebSocketListenerINSTANCE.handleMap.remove(uint64(handle))
}

func (c FfiConverterWebSocketListener) register() {
	C.uniffi_marketdata_uniffi_fn_init_callback_vtable_websocketlistener(&UniffiVTableCallbackInterfaceWebSocketListenerINSTANCE)
}

// The after-hours session flag for `futopt/intraday/ticker`, `quote` and
// `volumes`.
type AfterHoursParams struct {
	// `true` asks for the after-hours session (`session=afterhours`);
	// unset or `false` is the regular session.
	AfterHours *bool
}

func (r *AfterHoursParams) Destroy() {
	FfiDestroyerOptionalBool{}.Destroy(r.AfterHours)
}

type FfiConverterAfterHoursParams struct{}

var FfiConverterAfterHoursParamsINSTANCE = FfiConverterAfterHoursParams{}

func (c FfiConverterAfterHoursParams) Lift(rb RustBufferI) AfterHoursParams {
	return LiftFromRustBuffer[AfterHoursParams](c, rb)
}

func (c FfiConverterAfterHoursParams) Read(reader io.Reader) AfterHoursParams {
	return AfterHoursParams{
		FfiConverterOptionalBoolINSTANCE.Read(reader),
	}
}

func (c FfiConverterAfterHoursParams) Lower(value AfterHoursParams) C.RustBuffer {
	return LowerIntoRustBuffer[AfterHoursParams](c, value)
}

func (c FfiConverterAfterHoursParams) LowerExternal(value AfterHoursParams) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[AfterHoursParams](c, value))
}

func (c FfiConverterAfterHoursParams) Write(writer io.Writer, value AfterHoursParams) {
	FfiConverterOptionalBoolINSTANCE.Write(writer, value.AfterHours)
}

type FfiDestroyerAfterHoursParams struct{}

func (_ FfiDestroyerAfterHoursParams) Destroy(value AfterHoursParams) {
	value.Destroy()
}

// Connection configuration record for FFI: the timeouts of the connection
// itself (#199).
//
// Every field's zero value means "use default", so a zero-initialized
// record (C++ `ConnectionConfigRecord{}`, a Go `ConnectionConfigRecord{}`
// literal) is the full default. Omitting the record gives the same result.
type ConnectionConfigRecord struct {
	// How long the auth handshake may take once the WebSocket is open, in
	// milliseconds: from the auth frame being sent until the server's
	// verdict. Default 10000. Pass 0 to use the default. Applies to the
	// first `connect()` and to every reconnect; elapsing it fails the
	// attempt with a `TimeoutError` (3001). The server itself allows 60 s.
	AuthTimeoutMs uint64
}

func (r *ConnectionConfigRecord) Destroy() {
	FfiDestroyerUint64{}.Destroy(r.AuthTimeoutMs)
}

type FfiConverterConnectionConfigRecord struct{}

var FfiConverterConnectionConfigRecordINSTANCE = FfiConverterConnectionConfigRecord{}

func (c FfiConverterConnectionConfigRecord) Lift(rb RustBufferI) ConnectionConfigRecord {
	return LiftFromRustBuffer[ConnectionConfigRecord](c, rb)
}

func (c FfiConverterConnectionConfigRecord) Read(reader io.Reader) ConnectionConfigRecord {
	return ConnectionConfigRecord{
		FfiConverterUint64INSTANCE.Read(reader),
	}
}

func (c FfiConverterConnectionConfigRecord) Lower(value ConnectionConfigRecord) C.RustBuffer {
	return LowerIntoRustBuffer[ConnectionConfigRecord](c, value)
}

func (c FfiConverterConnectionConfigRecord) LowerExternal(value ConnectionConfigRecord) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[ConnectionConfigRecord](c, value))
}

func (c FfiConverterConnectionConfigRecord) Write(writer io.Writer, value ConnectionConfigRecord) {
	FfiConverterUint64INSTANCE.Write(writer, value.AuthTimeoutMs)
}

type FfiDestroyerConnectionConfigRecord struct{}

func (_ FfiDestroyerConnectionConfigRecord) Destroy(value ConnectionConfigRecord) {
	value.Destroy()
}

// Parameters for the three `stock/corporate-actions` endpoints.
//
// `capital-changes` has no `exchange`: setting it there is 1005
// `INVALID_PARAMETER`.
type CorporateActionsParams struct {
	// `YYYY-MM-DD`.
	StartDate *string
	// `YYYY-MM-DD`.
	EndDate *string
	// `TWSE` or `TPEx` (dividends and listing-applicants only).
	Exchange *string
	// `asc` or `desc`.
	Sort *string
}

func (r *CorporateActionsParams) Destroy() {
	FfiDestroyerOptionalString{}.Destroy(r.StartDate)
	FfiDestroyerOptionalString{}.Destroy(r.EndDate)
	FfiDestroyerOptionalString{}.Destroy(r.Exchange)
	FfiDestroyerOptionalString{}.Destroy(r.Sort)
}

type FfiConverterCorporateActionsParams struct{}

var FfiConverterCorporateActionsParamsINSTANCE = FfiConverterCorporateActionsParams{}

func (c FfiConverterCorporateActionsParams) Lift(rb RustBufferI) CorporateActionsParams {
	return LiftFromRustBuffer[CorporateActionsParams](c, rb)
}

func (c FfiConverterCorporateActionsParams) Read(reader io.Reader) CorporateActionsParams {
	return CorporateActionsParams{
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
	}
}

func (c FfiConverterCorporateActionsParams) Lower(value CorporateActionsParams) C.RustBuffer {
	return LowerIntoRustBuffer[CorporateActionsParams](c, value)
}

func (c FfiConverterCorporateActionsParams) LowerExternal(value CorporateActionsParams) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[CorporateActionsParams](c, value))
}

func (c FfiConverterCorporateActionsParams) Write(writer io.Writer, value CorporateActionsParams) {
	FfiConverterOptionalStringINSTANCE.Write(writer, value.StartDate)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.EndDate)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Exchange)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Sort)
}

type FfiDestroyerCorporateActionsParams struct{}

func (_ FfiDestroyerCorporateActionsParams) Destroy(value CorporateActionsParams) {
	value.Destroy()
}

// The credentials a WebSocket client authenticates with.
//
// Exactly one must be non-empty; an empty or whitespace-only value counts
// as not provided.
//
// Its fields are secrets: do not log this record. `Debug` here redacts
// them, but the generated types may not — a C# record's `ToString()` and
// Go's `fmt` `%v` print every field.
type CredentialsRecord struct {
	// Fugle API key, sent as `apikey`
	ApiKey *string
	// OAuth bearer token, sent as `token`
	BearerToken *string
	// Fugle SDK token, sent as `sdkToken`
	SdkToken *string
}

func (r *CredentialsRecord) Destroy() {
	FfiDestroyerOptionalString{}.Destroy(r.ApiKey)
	FfiDestroyerOptionalString{}.Destroy(r.BearerToken)
	FfiDestroyerOptionalString{}.Destroy(r.SdkToken)
}

type FfiConverterCredentialsRecord struct{}

var FfiConverterCredentialsRecordINSTANCE = FfiConverterCredentialsRecord{}

func (c FfiConverterCredentialsRecord) Lift(rb RustBufferI) CredentialsRecord {
	return LiftFromRustBuffer[CredentialsRecord](c, rb)
}

func (c FfiConverterCredentialsRecord) Read(reader io.Reader) CredentialsRecord {
	return CredentialsRecord{
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
	}
}

func (c FfiConverterCredentialsRecord) Lower(value CredentialsRecord) C.RustBuffer {
	return LowerIntoRustBuffer[CredentialsRecord](c, value)
}

func (c FfiConverterCredentialsRecord) LowerExternal(value CredentialsRecord) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[CredentialsRecord](c, value))
}

func (c FfiConverterCredentialsRecord) Write(writer io.Writer, value CredentialsRecord) {
	FfiConverterOptionalStringINSTANCE.Write(writer, value.ApiKey)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.BearerToken)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.SdkToken)
}

type FfiDestroyerCredentialsRecord struct{}

func (_ FfiDestroyerCredentialsRecord) Destroy(value CredentialsRecord) {
	value.Destroy()
}

// The cross-language view of an error: the fields every binding exposes
// under the same names. Mirrors `marketdata_core::ErrorInfo`.
type ErrorInfo struct {
	// Numeric code from `marketdata_core::error_code`, stable across
	// languages and releases.
	Code int32
	// Category of the failure.
	SourceKind ErrorSourceKind
	// Human-readable message.
	Message string
	// HTTP status, when the error came from an HTTP response (REST, or the
	// WebSocket upgrade).
	Status *uint16
	// Raw HTTP response body (REST only).
	Body *string
	// Server-assigned request id (`x-request-id`), when present.
	RequestId *string
	// HTTP response headers (REST only; empty otherwise).
	Headers map[string]string
}

func (r *ErrorInfo) Destroy() {
	FfiDestroyerInt32{}.Destroy(r.Code)
	FfiDestroyerErrorSourceKind{}.Destroy(r.SourceKind)
	FfiDestroyerString{}.Destroy(r.Message)
	FfiDestroyerOptionalUint16{}.Destroy(r.Status)
	FfiDestroyerOptionalString{}.Destroy(r.Body)
	FfiDestroyerOptionalString{}.Destroy(r.RequestId)
	FfiDestroyerMapStringString{}.Destroy(r.Headers)
}

type FfiConverterErrorInfo struct{}

var FfiConverterErrorInfoINSTANCE = FfiConverterErrorInfo{}

func (c FfiConverterErrorInfo) Lift(rb RustBufferI) ErrorInfo {
	return LiftFromRustBuffer[ErrorInfo](c, rb)
}

func (c FfiConverterErrorInfo) Read(reader io.Reader) ErrorInfo {
	return ErrorInfo{
		FfiConverterInt32INSTANCE.Read(reader),
		FfiConverterErrorSourceKindINSTANCE.Read(reader),
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterOptionalUint16INSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterMapStringStringINSTANCE.Read(reader),
	}
}

func (c FfiConverterErrorInfo) Lower(value ErrorInfo) C.RustBuffer {
	return LowerIntoRustBuffer[ErrorInfo](c, value)
}

func (c FfiConverterErrorInfo) LowerExternal(value ErrorInfo) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[ErrorInfo](c, value))
}

func (c FfiConverterErrorInfo) Write(writer io.Writer, value ErrorInfo) {
	FfiConverterInt32INSTANCE.Write(writer, value.Code)
	FfiConverterErrorSourceKindINSTANCE.Write(writer, value.SourceKind)
	FfiConverterStringINSTANCE.Write(writer, value.Message)
	FfiConverterOptionalUint16INSTANCE.Write(writer, value.Status)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Body)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.RequestId)
	FfiConverterMapStringStringINSTANCE.Write(writer, value.Headers)
}

type FfiDestroyerErrorInfo struct{}

func (_ FfiDestroyerErrorInfo) Destroy(value ErrorInfo) {
	value.Destroy()
}

// Parameters for `futopt/intraday/candles`.
type FutOptCandlesParams struct {
	// `true` asks for the after-hours session (`session=afterhours`).
	AfterHours *bool
	// `1`, `5`, `10`, `15`, `30` or `60` minutes.
	Timeframe *string
}

func (r *FutOptCandlesParams) Destroy() {
	FfiDestroyerOptionalBool{}.Destroy(r.AfterHours)
	FfiDestroyerOptionalString{}.Destroy(r.Timeframe)
}

type FfiConverterFutOptCandlesParams struct{}

var FfiConverterFutOptCandlesParamsINSTANCE = FfiConverterFutOptCandlesParams{}

func (c FfiConverterFutOptCandlesParams) Lift(rb RustBufferI) FutOptCandlesParams {
	return LiftFromRustBuffer[FutOptCandlesParams](c, rb)
}

func (c FfiConverterFutOptCandlesParams) Read(reader io.Reader) FutOptCandlesParams {
	return FutOptCandlesParams{
		FfiConverterOptionalBoolINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
	}
}

func (c FfiConverterFutOptCandlesParams) Lower(value FutOptCandlesParams) C.RustBuffer {
	return LowerIntoRustBuffer[FutOptCandlesParams](c, value)
}

func (c FfiConverterFutOptCandlesParams) LowerExternal(value FutOptCandlesParams) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[FutOptCandlesParams](c, value))
}

func (c FfiConverterFutOptCandlesParams) Write(writer io.Writer, value FutOptCandlesParams) {
	FfiConverterOptionalBoolINSTANCE.Write(writer, value.AfterHours)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Timeframe)
}

type FfiDestroyerFutOptCandlesParams struct{}

func (_ FfiDestroyerFutOptCandlesParams) Destroy(value FutOptCandlesParams) {
	value.Destroy()
}

// Parameters for `futopt/historical/daily`.
type FutOptDailyParams struct {
	// `YYYY-MM-DD`.
	Date *string
	// `true` asks for the after-hours session (`session=afterhours`).
	AfterHours *bool
}

func (r *FutOptDailyParams) Destroy() {
	FfiDestroyerOptionalString{}.Destroy(r.Date)
	FfiDestroyerOptionalBool{}.Destroy(r.AfterHours)
}

type FfiConverterFutOptDailyParams struct{}

var FfiConverterFutOptDailyParamsINSTANCE = FfiConverterFutOptDailyParams{}

func (c FfiConverterFutOptDailyParams) Lift(rb RustBufferI) FutOptDailyParams {
	return LiftFromRustBuffer[FutOptDailyParams](c, rb)
}

func (c FfiConverterFutOptDailyParams) Read(reader io.Reader) FutOptDailyParams {
	return FutOptDailyParams{
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalBoolINSTANCE.Read(reader),
	}
}

func (c FfiConverterFutOptDailyParams) Lower(value FutOptDailyParams) C.RustBuffer {
	return LowerIntoRustBuffer[FutOptDailyParams](c, value)
}

func (c FfiConverterFutOptDailyParams) LowerExternal(value FutOptDailyParams) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[FutOptDailyParams](c, value))
}

func (c FfiConverterFutOptDailyParams) Write(writer io.Writer, value FutOptDailyParams) {
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Date)
	FfiConverterOptionalBoolINSTANCE.Write(writer, value.AfterHours)
}

type FfiDestroyerFutOptDailyParams struct{}

func (_ FfiDestroyerFutOptDailyParams) Destroy(value FutOptDailyParams) {
	value.Destroy()
}

// Parameters for `futopt/historical/candles`.
//
// `strike_price` is `f64`, and 0 is a strike like any other: unlike the
// config records, an unset field is `None`, not 0.
type FutOptHistoricalCandlesParams struct {
	// Start date, `YYYY-MM-DD`.
	From *string
	// End date, `YYYY-MM-DD`.
	To *string
	// `YYYYMM`, or a continuous contract: `1!` (the server default), `2!`,
	// `3!`.
	ContractMonth *string
	// Comma-separated field names.
	Fields *string
	// `D`, or `1`, `5`, `10`, `15`, `30`, `60` minutes.
	Timeframe *string
	// `asc` or `desc`.
	Sort *string
	// Options only.
	StrikePrice *float64
	// Options only: `C` or `P`.
	CallPut *string
	// `true` asks for the after-hours session (`session=afterhours`).
	AfterHours *bool
}

func (r *FutOptHistoricalCandlesParams) Destroy() {
	FfiDestroyerOptionalString{}.Destroy(r.From)
	FfiDestroyerOptionalString{}.Destroy(r.To)
	FfiDestroyerOptionalString{}.Destroy(r.ContractMonth)
	FfiDestroyerOptionalString{}.Destroy(r.Fields)
	FfiDestroyerOptionalString{}.Destroy(r.Timeframe)
	FfiDestroyerOptionalString{}.Destroy(r.Sort)
	FfiDestroyerOptionalFloat64{}.Destroy(r.StrikePrice)
	FfiDestroyerOptionalString{}.Destroy(r.CallPut)
	FfiDestroyerOptionalBool{}.Destroy(r.AfterHours)
}

type FfiConverterFutOptHistoricalCandlesParams struct{}

var FfiConverterFutOptHistoricalCandlesParamsINSTANCE = FfiConverterFutOptHistoricalCandlesParams{}

func (c FfiConverterFutOptHistoricalCandlesParams) Lift(rb RustBufferI) FutOptHistoricalCandlesParams {
	return LiftFromRustBuffer[FutOptHistoricalCandlesParams](c, rb)
}

func (c FfiConverterFutOptHistoricalCandlesParams) Read(reader io.Reader) FutOptHistoricalCandlesParams {
	return FutOptHistoricalCandlesParams{
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalBoolINSTANCE.Read(reader),
	}
}

func (c FfiConverterFutOptHistoricalCandlesParams) Lower(value FutOptHistoricalCandlesParams) C.RustBuffer {
	return LowerIntoRustBuffer[FutOptHistoricalCandlesParams](c, value)
}

func (c FfiConverterFutOptHistoricalCandlesParams) LowerExternal(value FutOptHistoricalCandlesParams) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[FutOptHistoricalCandlesParams](c, value))
}

func (c FfiConverterFutOptHistoricalCandlesParams) Write(writer io.Writer, value FutOptHistoricalCandlesParams) {
	FfiConverterOptionalStringINSTANCE.Write(writer, value.From)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.To)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.ContractMonth)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Fields)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Timeframe)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Sort)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.StrikePrice)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.CallPut)
	FfiConverterOptionalBoolINSTANCE.Write(writer, value.AfterHours)
}

type FfiDestroyerFutOptHistoricalCandlesParams struct{}

func (_ FfiDestroyerFutOptHistoricalCandlesParams) Destroy(value FutOptHistoricalCandlesParams) {
	value.Destroy()
}

// Filters for `futopt/intraday/products`; `type` is the method's argument.
type FutOptProductsParams struct {
	// `TAIFEX`.
	Exchange *string
	// `true` asks for the after-hours session (`session=AFTERHOURS`);
	// unset or `false` is the regular session.
	AfterHours *bool
	// `I`, `R`, `B`, `C`, `S` or `E`.
	ContractType *string
	// `N` (normal) or `U` (unlisted).
	Status *string
}

func (r *FutOptProductsParams) Destroy() {
	FfiDestroyerOptionalString{}.Destroy(r.Exchange)
	FfiDestroyerOptionalBool{}.Destroy(r.AfterHours)
	FfiDestroyerOptionalString{}.Destroy(r.ContractType)
	FfiDestroyerOptionalString{}.Destroy(r.Status)
}

type FfiConverterFutOptProductsParams struct{}

var FfiConverterFutOptProductsParamsINSTANCE = FfiConverterFutOptProductsParams{}

func (c FfiConverterFutOptProductsParams) Lift(rb RustBufferI) FutOptProductsParams {
	return LiftFromRustBuffer[FutOptProductsParams](c, rb)
}

func (c FfiConverterFutOptProductsParams) Read(reader io.Reader) FutOptProductsParams {
	return FutOptProductsParams{
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalBoolINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
	}
}

func (c FfiConverterFutOptProductsParams) Lower(value FutOptProductsParams) C.RustBuffer {
	return LowerIntoRustBuffer[FutOptProductsParams](c, value)
}

func (c FfiConverterFutOptProductsParams) LowerExternal(value FutOptProductsParams) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[FutOptProductsParams](c, value))
}

func (c FfiConverterFutOptProductsParams) Write(writer io.Writer, value FutOptProductsParams) {
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Exchange)
	FfiConverterOptionalBoolINSTANCE.Write(writer, value.AfterHours)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.ContractType)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Status)
}

type FfiDestroyerFutOptProductsParams struct{}

func (_ FfiDestroyerFutOptProductsParams) Destroy(value FutOptProductsParams) {
	value.Destroy()
}

// Filters for `futopt/intraday/tickers`; `type` is the method's argument.
type FutOptTickersParams struct {
	// `TAIFEX`.
	Exchange *string
	// `true` asks for the after-hours session (`session=AFTERHOURS`);
	// unset or `false` is the regular session.
	AfterHours *bool
	// Product code, `TXF`.
	Product *string
	// `I`, `R`, `B`, `C`, `S` or `E`.
	ContractType *string
	IsSpread     *bool
}

func (r *FutOptTickersParams) Destroy() {
	FfiDestroyerOptionalString{}.Destroy(r.Exchange)
	FfiDestroyerOptionalBool{}.Destroy(r.AfterHours)
	FfiDestroyerOptionalString{}.Destroy(r.Product)
	FfiDestroyerOptionalString{}.Destroy(r.ContractType)
	FfiDestroyerOptionalBool{}.Destroy(r.IsSpread)
}

type FfiConverterFutOptTickersParams struct{}

var FfiConverterFutOptTickersParamsINSTANCE = FfiConverterFutOptTickersParams{}

func (c FfiConverterFutOptTickersParams) Lift(rb RustBufferI) FutOptTickersParams {
	return LiftFromRustBuffer[FutOptTickersParams](c, rb)
}

func (c FfiConverterFutOptTickersParams) Read(reader io.Reader) FutOptTickersParams {
	return FutOptTickersParams{
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalBoolINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalBoolINSTANCE.Read(reader),
	}
}

func (c FfiConverterFutOptTickersParams) Lower(value FutOptTickersParams) C.RustBuffer {
	return LowerIntoRustBuffer[FutOptTickersParams](c, value)
}

func (c FfiConverterFutOptTickersParams) LowerExternal(value FutOptTickersParams) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[FutOptTickersParams](c, value))
}

func (c FfiConverterFutOptTickersParams) Write(writer io.Writer, value FutOptTickersParams) {
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Exchange)
	FfiConverterOptionalBoolINSTANCE.Write(writer, value.AfterHours)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Product)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.ContractType)
	FfiConverterOptionalBoolINSTANCE.Write(writer, value.IsSpread)
}

type FfiDestroyerFutOptTickersParams struct{}

func (_ FfiDestroyerFutOptTickersParams) Destroy(value FutOptTickersParams) {
	value.Destroy()
}

// Parameters for `futopt/intraday/trades`.
type FutOptTradesParams struct {
	// `true` asks for the after-hours session (`session=afterhours`).
	AfterHours *bool
	Offset     *uint32
	Limit      *uint32
	IsTrial    *bool
}

func (r *FutOptTradesParams) Destroy() {
	FfiDestroyerOptionalBool{}.Destroy(r.AfterHours)
	FfiDestroyerOptionalUint32{}.Destroy(r.Offset)
	FfiDestroyerOptionalUint32{}.Destroy(r.Limit)
	FfiDestroyerOptionalBool{}.Destroy(r.IsTrial)
}

type FfiConverterFutOptTradesParams struct{}

var FfiConverterFutOptTradesParamsINSTANCE = FfiConverterFutOptTradesParams{}

func (c FfiConverterFutOptTradesParams) Lift(rb RustBufferI) FutOptTradesParams {
	return LiftFromRustBuffer[FutOptTradesParams](c, rb)
}

func (c FfiConverterFutOptTradesParams) Read(reader io.Reader) FutOptTradesParams {
	return FutOptTradesParams{
		FfiConverterOptionalBoolINSTANCE.Read(reader),
		FfiConverterOptionalUint32INSTANCE.Read(reader),
		FfiConverterOptionalUint32INSTANCE.Read(reader),
		FfiConverterOptionalBoolINSTANCE.Read(reader),
	}
}

func (c FfiConverterFutOptTradesParams) Lower(value FutOptTradesParams) C.RustBuffer {
	return LowerIntoRustBuffer[FutOptTradesParams](c, value)
}

func (c FfiConverterFutOptTradesParams) LowerExternal(value FutOptTradesParams) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[FutOptTradesParams](c, value))
}

func (c FfiConverterFutOptTradesParams) Write(writer io.Writer, value FutOptTradesParams) {
	FfiConverterOptionalBoolINSTANCE.Write(writer, value.AfterHours)
	FfiConverterOptionalUint32INSTANCE.Write(writer, value.Offset)
	FfiConverterOptionalUint32INSTANCE.Write(writer, value.Limit)
	FfiConverterOptionalBoolINSTANCE.Write(writer, value.IsTrial)
}

type FfiDestroyerFutOptTradesParams struct{}

func (_ FfiDestroyerFutOptTradesParams) Destroy(value FutOptTradesParams) {
	value.Destroy()
}

// Health check configuration record for FFI
//
// Every field's zero value means "use default", so a zero-initialized
// record (C++ `HealthCheckConfigRecord{}`, a Go `HealthCheckConfigRecord{}`
// literal) is the full default: detection on, no probe, 35 s timeout
// (#158, #161).
type HealthCheckConfigRecord struct {
	// Whether liveness detection is active; `false` turns it off. Unset
	// (the zero value) takes the core default, which is on.
	Enabled *bool
	// Maximum allowed gap between inbound frames before declaring the
	// connection dead, in milliseconds. Default 35000; floor 5000.
	// Pass 0 to use the default. Does not apply when `probe_enabled` is
	// true.
	HeartbeatTimeoutMs uint64
	// Confirm a silent connection with a ping before declaring it dead
	// (default: false). After `idle_probe_after_ms` of silence one ping is
	// sent; if nothing arrives within `probe_timeout_ms` the connection is
	// declared dead.
	ProbeEnabled bool
	// Silence before the probe, in milliseconds. Default 30000 (the
	// server's heartbeat period); floor 5000. Pass 0 to use the default.
	IdleProbeAfterMs uint64
	// Wait for any inbound frame after the probe, in milliseconds.
	// Default 5000; floor 1000. Pass 0 to use the default.
	ProbeTimeoutMs uint64
}

func (r *HealthCheckConfigRecord) Destroy() {
	FfiDestroyerOptionalBool{}.Destroy(r.Enabled)
	FfiDestroyerUint64{}.Destroy(r.HeartbeatTimeoutMs)
	FfiDestroyerBool{}.Destroy(r.ProbeEnabled)
	FfiDestroyerUint64{}.Destroy(r.IdleProbeAfterMs)
	FfiDestroyerUint64{}.Destroy(r.ProbeTimeoutMs)
}

type FfiConverterHealthCheckConfigRecord struct{}

var FfiConverterHealthCheckConfigRecordINSTANCE = FfiConverterHealthCheckConfigRecord{}

func (c FfiConverterHealthCheckConfigRecord) Lift(rb RustBufferI) HealthCheckConfigRecord {
	return LiftFromRustBuffer[HealthCheckConfigRecord](c, rb)
}

func (c FfiConverterHealthCheckConfigRecord) Read(reader io.Reader) HealthCheckConfigRecord {
	return HealthCheckConfigRecord{
		FfiConverterOptionalBoolINSTANCE.Read(reader),
		FfiConverterUint64INSTANCE.Read(reader),
		FfiConverterBoolINSTANCE.Read(reader),
		FfiConverterUint64INSTANCE.Read(reader),
		FfiConverterUint64INSTANCE.Read(reader),
	}
}

func (c FfiConverterHealthCheckConfigRecord) Lower(value HealthCheckConfigRecord) C.RustBuffer {
	return LowerIntoRustBuffer[HealthCheckConfigRecord](c, value)
}

func (c FfiConverterHealthCheckConfigRecord) LowerExternal(value HealthCheckConfigRecord) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[HealthCheckConfigRecord](c, value))
}

func (c FfiConverterHealthCheckConfigRecord) Write(writer io.Writer, value HealthCheckConfigRecord) {
	FfiConverterOptionalBoolINSTANCE.Write(writer, value.Enabled)
	FfiConverterUint64INSTANCE.Write(writer, value.HeartbeatTimeoutMs)
	FfiConverterBoolINSTANCE.Write(writer, value.ProbeEnabled)
	FfiConverterUint64INSTANCE.Write(writer, value.IdleProbeAfterMs)
	FfiConverterUint64INSTANCE.Write(writer, value.ProbeTimeoutMs)
}

type FfiDestroyerHealthCheckConfigRecord struct{}

func (_ FfiDestroyerHealthCheckConfigRecord) Destroy(value HealthCheckConfigRecord) {
	value.Destroy()
}

// Message queue configuration record for FFI
//
// `buffer` is 0 for the default (4096).
type MessageQueueConfigRecord struct {
	// What happens to new messages while `buffer` are unread
	Overflow MessageOverflowRecord
	// Unread messages held (default 4096; 0 means default)
	Buffer uint32
}

func (r *MessageQueueConfigRecord) Destroy() {
	FfiDestroyerMessageOverflowRecord{}.Destroy(r.Overflow)
	FfiDestroyerUint32{}.Destroy(r.Buffer)
}

type FfiConverterMessageQueueConfigRecord struct{}

var FfiConverterMessageQueueConfigRecordINSTANCE = FfiConverterMessageQueueConfigRecord{}

func (c FfiConverterMessageQueueConfigRecord) Lift(rb RustBufferI) MessageQueueConfigRecord {
	return LiftFromRustBuffer[MessageQueueConfigRecord](c, rb)
}

func (c FfiConverterMessageQueueConfigRecord) Read(reader io.Reader) MessageQueueConfigRecord {
	return MessageQueueConfigRecord{
		FfiConverterMessageOverflowRecordINSTANCE.Read(reader),
		FfiConverterUint32INSTANCE.Read(reader),
	}
}

func (c FfiConverterMessageQueueConfigRecord) Lower(value MessageQueueConfigRecord) C.RustBuffer {
	return LowerIntoRustBuffer[MessageQueueConfigRecord](c, value)
}

func (c FfiConverterMessageQueueConfigRecord) LowerExternal(value MessageQueueConfigRecord) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[MessageQueueConfigRecord](c, value))
}

func (c FfiConverterMessageQueueConfigRecord) Write(writer io.Writer, value MessageQueueConfigRecord) {
	FfiConverterMessageOverflowRecordINSTANCE.Write(writer, value.Overflow)
	FfiConverterUint32INSTANCE.Write(writer, value.Buffer)
}

type FfiDestroyerMessageQueueConfigRecord struct{}

func (_ FfiDestroyerMessageQueueConfigRecord) Destroy(value MessageQueueConfigRecord) {
	value.Destroy()
}

// Parameters for `stock/snapshot/movers`; `direction` and `change` are the
// method's arguments.
//
// The price bounds are `f64`, and 0 is a bound like any other: unlike the
// config records, an unset field is `None`, not 0.
type MoversParams struct {
	// `type`: `ALL`, `ALLBUT0999` or `COMMONSTOCK`.
	TypeFilter *string
	// Change greater than.
	Gt *float64
	// Change greater than or equal to.
	Gte *float64
	// Change less than.
	Lt *float64
	// Change less than or equal to.
	Lte *float64
	// Change equal to.
	Eq *float64
}

func (r *MoversParams) Destroy() {
	FfiDestroyerOptionalString{}.Destroy(r.TypeFilter)
	FfiDestroyerOptionalFloat64{}.Destroy(r.Gt)
	FfiDestroyerOptionalFloat64{}.Destroy(r.Gte)
	FfiDestroyerOptionalFloat64{}.Destroy(r.Lt)
	FfiDestroyerOptionalFloat64{}.Destroy(r.Lte)
	FfiDestroyerOptionalFloat64{}.Destroy(r.Eq)
}

type FfiConverterMoversParams struct{}

var FfiConverterMoversParamsINSTANCE = FfiConverterMoversParams{}

func (c FfiConverterMoversParams) Lift(rb RustBufferI) MoversParams {
	return LiftFromRustBuffer[MoversParams](c, rb)
}

func (c FfiConverterMoversParams) Read(reader io.Reader) MoversParams {
	return MoversParams{
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
	}
}

func (c FfiConverterMoversParams) Lower(value MoversParams) C.RustBuffer {
	return LowerIntoRustBuffer[MoversParams](c, value)
}

func (c FfiConverterMoversParams) LowerExternal(value MoversParams) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[MoversParams](c, value))
}

func (c FfiConverterMoversParams) Write(writer io.Writer, value MoversParams) {
	FfiConverterOptionalStringINSTANCE.Write(writer, value.TypeFilter)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.Gt)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.Gte)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.Lt)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.Lte)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.Eq)
}

type FfiDestroyerMoversParams struct{}

func (_ FfiDestroyerMoversParams) Destroy(value MoversParams) {
	value.Destroy()
}

// The odd-lot session flag for `stock/intraday/ticker`, `quote` and `volumes`.
type OddLotParams struct {
	// `true` asks for the intraday odd-lot session (`type=oddlot`).
	OddLot *bool
}

func (r *OddLotParams) Destroy() {
	FfiDestroyerOptionalBool{}.Destroy(r.OddLot)
}

type FfiConverterOddLotParams struct{}

var FfiConverterOddLotParamsINSTANCE = FfiConverterOddLotParams{}

func (c FfiConverterOddLotParams) Lift(rb RustBufferI) OddLotParams {
	return LiftFromRustBuffer[OddLotParams](c, rb)
}

func (c FfiConverterOddLotParams) Read(reader io.Reader) OddLotParams {
	return OddLotParams{
		FfiConverterOptionalBoolINSTANCE.Read(reader),
	}
}

func (c FfiConverterOddLotParams) Lower(value OddLotParams) C.RustBuffer {
	return LowerIntoRustBuffer[OddLotParams](c, value)
}

func (c FfiConverterOddLotParams) LowerExternal(value OddLotParams) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[OddLotParams](c, value))
}

func (c FfiConverterOddLotParams) Write(writer io.Writer, value OddLotParams) {
	FfiConverterOptionalBoolINSTANCE.Write(writer, value.OddLot)
}

type FfiDestroyerOddLotParams struct{}

func (_ FfiDestroyerOddLotParams) Destroy(value OddLotParams) {
	value.Destroy()
}

// Parameters for the four `stock/ownership` endpoints.
type OwnershipParams struct {
	// Start date, `YYYY-MM-DD`.
	From *string
	// End date, `YYYY-MM-DD`.
	To *string
	// `asc` or `desc`.
	Sort *string
}

func (r *OwnershipParams) Destroy() {
	FfiDestroyerOptionalString{}.Destroy(r.From)
	FfiDestroyerOptionalString{}.Destroy(r.To)
	FfiDestroyerOptionalString{}.Destroy(r.Sort)
}

type FfiConverterOwnershipParams struct{}

var FfiConverterOwnershipParamsINSTANCE = FfiConverterOwnershipParams{}

func (c FfiConverterOwnershipParams) Lift(rb RustBufferI) OwnershipParams {
	return LiftFromRustBuffer[OwnershipParams](c, rb)
}

func (c FfiConverterOwnershipParams) Read(reader io.Reader) OwnershipParams {
	return OwnershipParams{
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
	}
}

func (c FfiConverterOwnershipParams) Lower(value OwnershipParams) C.RustBuffer {
	return LowerIntoRustBuffer[OwnershipParams](c, value)
}

func (c FfiConverterOwnershipParams) LowerExternal(value OwnershipParams) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[OwnershipParams](c, value))
}

func (c FfiConverterOwnershipParams) Write(writer io.Writer, value OwnershipParams) {
	FfiConverterOptionalStringINSTANCE.Write(writer, value.From)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.To)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Sort)
}

type FfiDestroyerOwnershipParams struct{}

func (_ FfiDestroyerOwnershipParams) Destroy(value OwnershipParams) {
	value.Destroy()
}

// Reconnection configuration record for FFI
//
// Every field's zero value means "use default", so a zero-initialized
// record (C++ `ReconnectConfigRecord{}`, a Go `ReconnectConfigRecord{}`
// literal) is the full default: auto-reconnect on with the core delays
// (#158, #161). Omitting the record gives the same result.
type ReconnectConfigRecord struct {
	// Whether auto-reconnect is active; `false` turns it off. Unset (the
	// zero value) takes the core default, which is on.
	Enabled *bool
	// Maximum reconnection attempts; 0 means unlimited (the default)
	MaxAttempts uint32
	// Initial reconnection delay in milliseconds (default: 1000, min: 100)
	InitialDelayMs uint64
	// Maximum reconnection delay in milliseconds (default: 60000)
	MaxDelayMs uint64
}

func (r *ReconnectConfigRecord) Destroy() {
	FfiDestroyerOptionalBool{}.Destroy(r.Enabled)
	FfiDestroyerUint32{}.Destroy(r.MaxAttempts)
	FfiDestroyerUint64{}.Destroy(r.InitialDelayMs)
	FfiDestroyerUint64{}.Destroy(r.MaxDelayMs)
}

type FfiConverterReconnectConfigRecord struct{}

var FfiConverterReconnectConfigRecordINSTANCE = FfiConverterReconnectConfigRecord{}

func (c FfiConverterReconnectConfigRecord) Lift(rb RustBufferI) ReconnectConfigRecord {
	return LiftFromRustBuffer[ReconnectConfigRecord](c, rb)
}

func (c FfiConverterReconnectConfigRecord) Read(reader io.Reader) ReconnectConfigRecord {
	return ReconnectConfigRecord{
		FfiConverterOptionalBoolINSTANCE.Read(reader),
		FfiConverterUint32INSTANCE.Read(reader),
		FfiConverterUint64INSTANCE.Read(reader),
		FfiConverterUint64INSTANCE.Read(reader),
	}
}

func (c FfiConverterReconnectConfigRecord) Lower(value ReconnectConfigRecord) C.RustBuffer {
	return LowerIntoRustBuffer[ReconnectConfigRecord](c, value)
}

func (c FfiConverterReconnectConfigRecord) LowerExternal(value ReconnectConfigRecord) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[ReconnectConfigRecord](c, value))
}

func (c FfiConverterReconnectConfigRecord) Write(writer io.Writer, value ReconnectConfigRecord) {
	FfiConverterOptionalBoolINSTANCE.Write(writer, value.Enabled)
	FfiConverterUint32INSTANCE.Write(writer, value.MaxAttempts)
	FfiConverterUint64INSTANCE.Write(writer, value.InitialDelayMs)
	FfiConverterUint64INSTANCE.Write(writer, value.MaxDelayMs)
}

type FfiDestroyerReconnectConfigRecord struct{}

func (_ FfiDestroyerReconnectConfigRecord) Destroy(value ReconnectConfigRecord) {
	value.Destroy()
}

// Parameters for `stock/snapshot/quotes` and `actives`.
type SnapshotParams struct {
	// `type`: `ALL`, `ALLBUT0999` or `COMMONSTOCK`.
	TypeFilter *string
}

func (r *SnapshotParams) Destroy() {
	FfiDestroyerOptionalString{}.Destroy(r.TypeFilter)
}

type FfiConverterSnapshotParams struct{}

var FfiConverterSnapshotParamsINSTANCE = FfiConverterSnapshotParams{}

func (c FfiConverterSnapshotParams) Lift(rb RustBufferI) SnapshotParams {
	return LiftFromRustBuffer[SnapshotParams](c, rb)
}

func (c FfiConverterSnapshotParams) Read(reader io.Reader) SnapshotParams {
	return SnapshotParams{
		FfiConverterOptionalStringINSTANCE.Read(reader),
	}
}

func (c FfiConverterSnapshotParams) Lower(value SnapshotParams) C.RustBuffer {
	return LowerIntoRustBuffer[SnapshotParams](c, value)
}

func (c FfiConverterSnapshotParams) LowerExternal(value SnapshotParams) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[SnapshotParams](c, value))
}

func (c FfiConverterSnapshotParams) Write(writer io.Writer, value SnapshotParams) {
	FfiConverterOptionalStringINSTANCE.Write(writer, value.TypeFilter)
}

type FfiDestroyerSnapshotParams struct{}

func (_ FfiDestroyerSnapshotParams) Destroy(value SnapshotParams) {
	value.Destroy()
}

// Parameters for `stock/intraday/candles`.
type StockCandlesParams struct {
	// `1`, `5`, `10`, `15`, `30` or `60` minutes; unset takes the server
	// default.
	Timeframe *string
	// `true` asks for the intraday odd-lot session (`type=oddlot`).
	OddLot *bool
	// `asc` or `desc`.
	Sort *string
}

func (r *StockCandlesParams) Destroy() {
	FfiDestroyerOptionalString{}.Destroy(r.Timeframe)
	FfiDestroyerOptionalBool{}.Destroy(r.OddLot)
	FfiDestroyerOptionalString{}.Destroy(r.Sort)
}

type FfiConverterStockCandlesParams struct{}

var FfiConverterStockCandlesParamsINSTANCE = FfiConverterStockCandlesParams{}

func (c FfiConverterStockCandlesParams) Lift(rb RustBufferI) StockCandlesParams {
	return LiftFromRustBuffer[StockCandlesParams](c, rb)
}

func (c FfiConverterStockCandlesParams) Read(reader io.Reader) StockCandlesParams {
	return StockCandlesParams{
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalBoolINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
	}
}

func (c FfiConverterStockCandlesParams) Lower(value StockCandlesParams) C.RustBuffer {
	return LowerIntoRustBuffer[StockCandlesParams](c, value)
}

func (c FfiConverterStockCandlesParams) LowerExternal(value StockCandlesParams) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[StockCandlesParams](c, value))
}

func (c FfiConverterStockCandlesParams) Write(writer io.Writer, value StockCandlesParams) {
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Timeframe)
	FfiConverterOptionalBoolINSTANCE.Write(writer, value.OddLot)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Sort)
}

type FfiDestroyerStockCandlesParams struct{}

func (_ FfiDestroyerStockCandlesParams) Destroy(value StockCandlesParams) {
	value.Destroy()
}

// Parameters for `stock/historical/candles`.
type StockHistoricalCandlesParams struct {
	// Start date, `YYYY-MM-DD`.
	From *string
	// End date, `YYYY-MM-DD`.
	To *string
	// `D`, `W`, `M`, or `1`, `5`, `10`, `15`, `30`, `60` minutes.
	Timeframe *string
	// Comma-separated field names, `open,high,low,close,volume`.
	Fields *string
	// `asc` or `desc`.
	Sort *string
	// Adjusted prices.
	Adjusted *bool
}

func (r *StockHistoricalCandlesParams) Destroy() {
	FfiDestroyerOptionalString{}.Destroy(r.From)
	FfiDestroyerOptionalString{}.Destroy(r.To)
	FfiDestroyerOptionalString{}.Destroy(r.Timeframe)
	FfiDestroyerOptionalString{}.Destroy(r.Fields)
	FfiDestroyerOptionalString{}.Destroy(r.Sort)
	FfiDestroyerOptionalBool{}.Destroy(r.Adjusted)
}

type FfiConverterStockHistoricalCandlesParams struct{}

var FfiConverterStockHistoricalCandlesParamsINSTANCE = FfiConverterStockHistoricalCandlesParams{}

func (c FfiConverterStockHistoricalCandlesParams) Lift(rb RustBufferI) StockHistoricalCandlesParams {
	return LiftFromRustBuffer[StockHistoricalCandlesParams](c, rb)
}

func (c FfiConverterStockHistoricalCandlesParams) Read(reader io.Reader) StockHistoricalCandlesParams {
	return StockHistoricalCandlesParams{
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalBoolINSTANCE.Read(reader),
	}
}

func (c FfiConverterStockHistoricalCandlesParams) Lower(value StockHistoricalCandlesParams) C.RustBuffer {
	return LowerIntoRustBuffer[StockHistoricalCandlesParams](c, value)
}

func (c FfiConverterStockHistoricalCandlesParams) LowerExternal(value StockHistoricalCandlesParams) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[StockHistoricalCandlesParams](c, value))
}

func (c FfiConverterStockHistoricalCandlesParams) Write(writer io.Writer, value StockHistoricalCandlesParams) {
	FfiConverterOptionalStringINSTANCE.Write(writer, value.From)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.To)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Timeframe)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Fields)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Sort)
	FfiConverterOptionalBoolINSTANCE.Write(writer, value.Adjusted)
}

type FfiDestroyerStockHistoricalCandlesParams struct{}

func (_ FfiDestroyerStockHistoricalCandlesParams) Destroy(value StockHistoricalCandlesParams) {
	value.Destroy()
}

// Filters for `stock/intraday/tickers`; `type` is the method's argument.
type StockTickersParams struct {
	// `TWSE` or `TPEx`.
	Exchange *string
	// `TSE`, `OTC`, `ESB`, `TIB` or `PSB`.
	Market *string
	// Industry code.
	Industry      *string
	IsNormal      *bool
	IsAttention   *bool
	IsDisposition *bool
	IsHalted      *bool
	// Symbol prefix.
	Symbol *string
}

func (r *StockTickersParams) Destroy() {
	FfiDestroyerOptionalString{}.Destroy(r.Exchange)
	FfiDestroyerOptionalString{}.Destroy(r.Market)
	FfiDestroyerOptionalString{}.Destroy(r.Industry)
	FfiDestroyerOptionalBool{}.Destroy(r.IsNormal)
	FfiDestroyerOptionalBool{}.Destroy(r.IsAttention)
	FfiDestroyerOptionalBool{}.Destroy(r.IsDisposition)
	FfiDestroyerOptionalBool{}.Destroy(r.IsHalted)
	FfiDestroyerOptionalString{}.Destroy(r.Symbol)
}

type FfiConverterStockTickersParams struct{}

var FfiConverterStockTickersParamsINSTANCE = FfiConverterStockTickersParams{}

func (c FfiConverterStockTickersParams) Lift(rb RustBufferI) StockTickersParams {
	return LiftFromRustBuffer[StockTickersParams](c, rb)
}

func (c FfiConverterStockTickersParams) Read(reader io.Reader) StockTickersParams {
	return StockTickersParams{
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalBoolINSTANCE.Read(reader),
		FfiConverterOptionalBoolINSTANCE.Read(reader),
		FfiConverterOptionalBoolINSTANCE.Read(reader),
		FfiConverterOptionalBoolINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
	}
}

func (c FfiConverterStockTickersParams) Lower(value StockTickersParams) C.RustBuffer {
	return LowerIntoRustBuffer[StockTickersParams](c, value)
}

func (c FfiConverterStockTickersParams) LowerExternal(value StockTickersParams) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[StockTickersParams](c, value))
}

func (c FfiConverterStockTickersParams) Write(writer io.Writer, value StockTickersParams) {
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Exchange)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Market)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Industry)
	FfiConverterOptionalBoolINSTANCE.Write(writer, value.IsNormal)
	FfiConverterOptionalBoolINSTANCE.Write(writer, value.IsAttention)
	FfiConverterOptionalBoolINSTANCE.Write(writer, value.IsDisposition)
	FfiConverterOptionalBoolINSTANCE.Write(writer, value.IsHalted)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Symbol)
}

type FfiDestroyerStockTickersParams struct{}

func (_ FfiDestroyerStockTickersParams) Destroy(value StockTickersParams) {
	value.Destroy()
}

// Parameters for `stock/intraday/trades`.
type StockTradesParams struct {
	// `true` asks for the intraday odd-lot session (`type=oddlot`).
	OddLot *bool
	Offset *uint32
	Limit  *uint32
	// `asc` or `desc`.
	Sort    *string
	IsTrial *bool
}

func (r *StockTradesParams) Destroy() {
	FfiDestroyerOptionalBool{}.Destroy(r.OddLot)
	FfiDestroyerOptionalUint32{}.Destroy(r.Offset)
	FfiDestroyerOptionalUint32{}.Destroy(r.Limit)
	FfiDestroyerOptionalString{}.Destroy(r.Sort)
	FfiDestroyerOptionalBool{}.Destroy(r.IsTrial)
}

type FfiConverterStockTradesParams struct{}

var FfiConverterStockTradesParamsINSTANCE = FfiConverterStockTradesParams{}

func (c FfiConverterStockTradesParams) Lift(rb RustBufferI) StockTradesParams {
	return LiftFromRustBuffer[StockTradesParams](c, rb)
}

func (c FfiConverterStockTradesParams) Read(reader io.Reader) StockTradesParams {
	return StockTradesParams{
		FfiConverterOptionalBoolINSTANCE.Read(reader),
		FfiConverterOptionalUint32INSTANCE.Read(reader),
		FfiConverterOptionalUint32INSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalBoolINSTANCE.Read(reader),
	}
}

func (c FfiConverterStockTradesParams) Lower(value StockTradesParams) C.RustBuffer {
	return LowerIntoRustBuffer[StockTradesParams](c, value)
}

func (c FfiConverterStockTradesParams) LowerExternal(value StockTradesParams) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[StockTradesParams](c, value))
}

func (c FfiConverterStockTradesParams) Write(writer io.Writer, value StockTradesParams) {
	FfiConverterOptionalBoolINSTANCE.Write(writer, value.OddLot)
	FfiConverterOptionalUint32INSTANCE.Write(writer, value.Offset)
	FfiConverterOptionalUint32INSTANCE.Write(writer, value.Limit)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Sort)
	FfiConverterOptionalBoolINSTANCE.Write(writer, value.IsTrial)
}

type FfiDestroyerStockTradesParams struct{}

func (_ FfiDestroyerStockTradesParams) Destroy(value StockTradesParams) {
	value.Destroy()
}

// An inbound streaming frame.
//
// `raw` is the frame exactly as the server sent it — decode that when you
// want the payload. The other fields are the routing subset this SDK parses
// out so callbacks can dispatch without decoding the whole frame first; they
// are a convenience, not the source of truth.
type StreamMessage struct {
	// The frame verbatim, as received on the wire.
	Raw string
	// Event type: "data", "subscribed", "error", "authenticated", "pong".
	Event string
	// Channel name, for data events.
	Channel *string
	// Symbol, for data events.
	Symbol *string
	// Subscription id, for subscribed events.
	Id *string
	// The `data` member of the frame, still encoded as JSON.
	DataJson *string
	// Server error code, for error events: `1000` credentials rejected,
	// `1001` subscription limit exceeded, `1002` command before
	// authentication, `1003` request validation failed, `1004` no auth
	// request within 60 s, `1011` auth service unavailable. Absent when the
	// server sent an error frame without a code.
	ErrorCode *int32
	// Error message, for error events: the frame's `data.message`, or its
	// top-level `message` when the server sent the code-less shape.
	ErrorMessage *string
}

func (r *StreamMessage) Destroy() {
	FfiDestroyerString{}.Destroy(r.Raw)
	FfiDestroyerString{}.Destroy(r.Event)
	FfiDestroyerOptionalString{}.Destroy(r.Channel)
	FfiDestroyerOptionalString{}.Destroy(r.Symbol)
	FfiDestroyerOptionalString{}.Destroy(r.Id)
	FfiDestroyerOptionalString{}.Destroy(r.DataJson)
	FfiDestroyerOptionalInt32{}.Destroy(r.ErrorCode)
	FfiDestroyerOptionalString{}.Destroy(r.ErrorMessage)
}

type FfiConverterStreamMessage struct{}

var FfiConverterStreamMessageINSTANCE = FfiConverterStreamMessage{}

func (c FfiConverterStreamMessage) Lift(rb RustBufferI) StreamMessage {
	return LiftFromRustBuffer[StreamMessage](c, rb)
}

func (c FfiConverterStreamMessage) Read(reader io.Reader) StreamMessage {
	return StreamMessage{
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalInt32INSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
	}
}

func (c FfiConverterStreamMessage) Lower(value StreamMessage) C.RustBuffer {
	return LowerIntoRustBuffer[StreamMessage](c, value)
}

func (c FfiConverterStreamMessage) LowerExternal(value StreamMessage) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[StreamMessage](c, value))
}

func (c FfiConverterStreamMessage) Write(writer io.Writer, value StreamMessage) {
	FfiConverterStringINSTANCE.Write(writer, value.Raw)
	FfiConverterStringINSTANCE.Write(writer, value.Event)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Channel)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Symbol)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Id)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.DataJson)
	FfiConverterOptionalInt32INSTANCE.Write(writer, value.ErrorCode)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.ErrorMessage)
}

type FfiDestroyerStreamMessage struct{}

func (_ FfiDestroyerStreamMessage) Destroy(value StreamMessage) {
	value.Destroy()
}

// Per-product streaming version selection.
//
// UniFFI has no way to express core's one-enum-per-product typing across
// C#/Go/Java/C++ at once, so this carries optional strings and validates
// them — the same shape the official SDK's version map has.
type StreamingVersionRecord struct {
	// Stock streaming version. Only "v1.0" is served. None means latest.
	Stock *string
	// FutOpt streaming version: "v1.0" or "v1.1". None means latest (v1.1).
	//
	// v1.1 adds trial-matching (試撮) frames on trades / books — check the
	// frame's `isTrial` before acting on a price.
	Futopt *string
}

func (r *StreamingVersionRecord) Destroy() {
	FfiDestroyerOptionalString{}.Destroy(r.Stock)
	FfiDestroyerOptionalString{}.Destroy(r.Futopt)
}

type FfiConverterStreamingVersionRecord struct{}

var FfiConverterStreamingVersionRecordINSTANCE = FfiConverterStreamingVersionRecord{}

func (c FfiConverterStreamingVersionRecord) Lift(rb RustBufferI) StreamingVersionRecord {
	return LiftFromRustBuffer[StreamingVersionRecord](c, rb)
}

func (c FfiConverterStreamingVersionRecord) Read(reader io.Reader) StreamingVersionRecord {
	return StreamingVersionRecord{
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
	}
}

func (c FfiConverterStreamingVersionRecord) Lower(value StreamingVersionRecord) C.RustBuffer {
	return LowerIntoRustBuffer[StreamingVersionRecord](c, value)
}

func (c FfiConverterStreamingVersionRecord) LowerExternal(value StreamingVersionRecord) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[StreamingVersionRecord](c, value))
}

func (c FfiConverterStreamingVersionRecord) Write(writer io.Writer, value StreamingVersionRecord) {
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Stock)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Futopt)
}

type FfiDestroyerStreamingVersionRecord struct{}

func (_ FfiDestroyerStreamingVersionRecord) Destroy(value StreamingVersionRecord) {
	value.Destroy()
}

// Session options for `subscribe` / `unsubscribe` (#202).
//
// Unset is the regular session, so an omitted or default record subscribes
// as before. Each option belongs to one endpoint — `intraday_odd_lot`
// (盤中零股) to Stock, `after_hours` (盤後) to FutOpt — and setting it on
// the other, to any value, is 1005 `INVALID_PARAMETER`.
type SubscribeOptions struct {
	// FutOpt only: `true` subscribes to the after-hours session.
	AfterHours *bool
	// Stock only: `true` subscribes to the intraday odd-lot session.
	IntradayOddLot *bool
}

func (r *SubscribeOptions) Destroy() {
	FfiDestroyerOptionalBool{}.Destroy(r.AfterHours)
	FfiDestroyerOptionalBool{}.Destroy(r.IntradayOddLot)
}

type FfiConverterSubscribeOptions struct{}

var FfiConverterSubscribeOptionsINSTANCE = FfiConverterSubscribeOptions{}

func (c FfiConverterSubscribeOptions) Lift(rb RustBufferI) SubscribeOptions {
	return LiftFromRustBuffer[SubscribeOptions](c, rb)
}

func (c FfiConverterSubscribeOptions) Read(reader io.Reader) SubscribeOptions {
	return SubscribeOptions{
		FfiConverterOptionalBoolINSTANCE.Read(reader),
		FfiConverterOptionalBoolINSTANCE.Read(reader),
	}
}

func (c FfiConverterSubscribeOptions) Lower(value SubscribeOptions) C.RustBuffer {
	return LowerIntoRustBuffer[SubscribeOptions](c, value)
}

func (c FfiConverterSubscribeOptions) LowerExternal(value SubscribeOptions) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[SubscribeOptions](c, value))
}

func (c FfiConverterSubscribeOptions) Write(writer io.Writer, value SubscribeOptions) {
	FfiConverterOptionalBoolINSTANCE.Write(writer, value.AfterHours)
	FfiConverterOptionalBoolINSTANCE.Write(writer, value.IntradayOddLot)
}

type FfiDestroyerSubscribeOptions struct{}

func (_ FfiDestroyerSubscribeOptions) Destroy(value SubscribeOptions) {
	value.Destroy()
}

// The date range for the `stock/technical` endpoints; the periods are the
// method's arguments.
type TechnicalParams struct {
	// Start date, `YYYY-MM-DD`.
	From *string
	// End date, `YYYY-MM-DD`.
	To *string
	// `D`, `W`, `M`, or `1`, `5`, `10`, `15`, `30`, `60` minutes.
	Timeframe *string
}

func (r *TechnicalParams) Destroy() {
	FfiDestroyerOptionalString{}.Destroy(r.From)
	FfiDestroyerOptionalString{}.Destroy(r.To)
	FfiDestroyerOptionalString{}.Destroy(r.Timeframe)
}

type FfiConverterTechnicalParams struct{}

var FfiConverterTechnicalParamsINSTANCE = FfiConverterTechnicalParams{}

func (c FfiConverterTechnicalParams) Lift(rb RustBufferI) TechnicalParams {
	return LiftFromRustBuffer[TechnicalParams](c, rb)
}

func (c FfiConverterTechnicalParams) Read(reader io.Reader) TechnicalParams {
	return TechnicalParams{
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
	}
}

func (c FfiConverterTechnicalParams) Lower(value TechnicalParams) C.RustBuffer {
	return LowerIntoRustBuffer[TechnicalParams](c, value)
}

func (c FfiConverterTechnicalParams) LowerExternal(value TechnicalParams) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[TechnicalParams](c, value))
}

func (c FfiConverterTechnicalParams) Write(writer io.Writer, value TechnicalParams) {
	FfiConverterOptionalStringINSTANCE.Write(writer, value.From)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.To)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Timeframe)
}

type FfiDestroyerTechnicalParams struct{}

func (_ FfiDestroyerTechnicalParams) Destroy(value TechnicalParams) {
	value.Destroy()
}

// Optional TLS customization exposed to foreign languages.
//
// When all fields are default the SDK uses the OS trust store
// (loaded by `rustls-native-certs`). Provide `root_cert_pem` to pin
// an additional CA, or set `accept_invalid_certs` to disable all
// verification (dev/testing only — exposes MITM risk).
type TlsConfigRecord struct {
	// PEM-encoded additional root CA bytes. Appended to the OS trust
	// store; chains signed by either this CA or any OS-trusted root
	// are accepted.
	RootCertPem *[]byte
	// Disable ALL TLS verification (chain + hostname + expiry).
	// Equivalent to `curl -k` / `wscat --no-check`. Do not use in
	// production.
	AcceptInvalidCerts bool
}

func (r *TlsConfigRecord) Destroy() {
	FfiDestroyerOptionalBytes{}.Destroy(r.RootCertPem)
	FfiDestroyerBool{}.Destroy(r.AcceptInvalidCerts)
}

type FfiConverterTlsConfigRecord struct{}

var FfiConverterTlsConfigRecordINSTANCE = FfiConverterTlsConfigRecord{}

func (c FfiConverterTlsConfigRecord) Lift(rb RustBufferI) TlsConfigRecord {
	return LiftFromRustBuffer[TlsConfigRecord](c, rb)
}

func (c FfiConverterTlsConfigRecord) Read(reader io.Reader) TlsConfigRecord {
	return TlsConfigRecord{
		FfiConverterOptionalBytesINSTANCE.Read(reader),
		FfiConverterBoolINSTANCE.Read(reader),
	}
}

func (c FfiConverterTlsConfigRecord) Lower(value TlsConfigRecord) C.RustBuffer {
	return LowerIntoRustBuffer[TlsConfigRecord](c, value)
}

func (c FfiConverterTlsConfigRecord) LowerExternal(value TlsConfigRecord) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[TlsConfigRecord](c, value))
}

func (c FfiConverterTlsConfigRecord) Write(writer io.Writer, value TlsConfigRecord) {
	FfiConverterOptionalBytesINSTANCE.Write(writer, value.RootCertPem)
	FfiConverterBoolINSTANCE.Write(writer, value.AcceptInvalidCerts)
}

type FfiDestroyerTlsConfigRecord struct{}

func (_ FfiDestroyerTlsConfigRecord) Destroy(value TlsConfigRecord) {
	value.Destroy()
}

// Which credential [`validate_credentials`] accepted.
type CredentialKind uint

const (
	// `api_key` was the credential provided.
	CredentialKindApiKey CredentialKind = 1
	// `bearer_token` was the credential provided.
	CredentialKindBearerToken CredentialKind = 2
	// `sdk_token` was the credential provided.
	CredentialKindSdkToken CredentialKind = 3
)

type FfiConverterCredentialKind struct{}

var FfiConverterCredentialKindINSTANCE = FfiConverterCredentialKind{}

func (c FfiConverterCredentialKind) Lift(rb RustBufferI) CredentialKind {
	return LiftFromRustBuffer[CredentialKind](c, rb)
}

func (c FfiConverterCredentialKind) Lower(value CredentialKind) C.RustBuffer {
	return LowerIntoRustBuffer[CredentialKind](c, value)
}

func (c FfiConverterCredentialKind) LowerExternal(value CredentialKind) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[CredentialKind](c, value))
}
func (FfiConverterCredentialKind) Read(reader io.Reader) CredentialKind {
	id := readInt32(reader)
	return CredentialKind(id)
}

func (FfiConverterCredentialKind) Write(writer io.Writer, value CredentialKind) {
	writeInt32(writer, int32(value))
}

type FfiDestroyerCredentialKind struct{}

func (_ FfiDestroyerCredentialKind) Destroy(value CredentialKind) {
}

// Coarse-grained classification of the source of a [`MarketDataError`].
//
// Mirrors `marketdata_core::ErrorKind`. That core enum is `#[non_exhaustive]`
// so a future variant this crate doesn't know about yet maps to `Client`
// (see the `From` impl below) rather than failing to compile.
type ErrorSourceKind uint

const (
	// Transport-level transient failure: connection reset, timeout,
	// heartbeat gap, server outage (5xx). Generally safe to retry with
	// backoff.
	ErrorSourceKindNetwork ErrorSourceKind = 1
	// Protocol-level violation or unclassified WebSocket failure. Indicates
	// an SDK / version mismatch or a server-side bug; retry is unlikely to
	// help.
	ErrorSourceKindProtocol ErrorSourceKind = 2
	// Authentication / authorization failure: bad credentials, 401/403,
	// expired token, TLS cert failure. Human intervention required.
	ErrorSourceKindAuth ErrorSourceKind = 3
	// Server is rejecting requests because the caller is exceeding its
	// rate budget (HTTP 429).
	ErrorSourceKindRateLimit ErrorSourceKind = 4
	// Caller-side problem: invalid input, configuration error, client
	// already closed, serialization failure, non-auth/non-throttle 4xx.
	ErrorSourceKindClient ErrorSourceKind = 5
)

type FfiConverterErrorSourceKind struct{}

var FfiConverterErrorSourceKindINSTANCE = FfiConverterErrorSourceKind{}

func (c FfiConverterErrorSourceKind) Lift(rb RustBufferI) ErrorSourceKind {
	return LiftFromRustBuffer[ErrorSourceKind](c, rb)
}

func (c FfiConverterErrorSourceKind) Lower(value ErrorSourceKind) C.RustBuffer {
	return LowerIntoRustBuffer[ErrorSourceKind](c, value)
}

func (c FfiConverterErrorSourceKind) LowerExternal(value ErrorSourceKind) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[ErrorSourceKind](c, value))
}
func (FfiConverterErrorSourceKind) Read(reader io.Reader) ErrorSourceKind {
	id := readInt32(reader)
	return ErrorSourceKind(id)
}

func (FfiConverterErrorSourceKind) Write(writer io.Writer, value ErrorSourceKind) {
	writeInt32(writer, int32(value))
}

type FfiDestroyerErrorSourceKind struct{}

func (_ FfiDestroyerErrorSourceKind) Destroy(value ErrorSourceKind) {
}

// Error type for UniFFI bindings
//
// Maps to MarketDataError in the UDL file. Each variant becomes an exception
// in the target language with the error message preserved, plus an `info`
// field carrying the unified [`ErrorInfo`].
//
// Note: This is a FLAT enum per UniFFI constraints - no nested error types.
type MarketDataError struct {
	err error
}

// Convience method to turn *MarketDataError into error
// Avoiding treating nil pointer as non nil error interface
func (err *MarketDataError) AsError() error {
	if err == nil {
		return nil
	} else {
		return err
	}
}

func (err MarketDataError) Error() string {
	return fmt.Sprintf("MarketDataError: %s", err.err.Error())
}

func (err MarketDataError) Unwrap() error {
	return err.err
}

// Err* are used for checking error type with `errors.Is`
var ErrMarketDataErrorConnectionError = fmt.Errorf("MarketDataErrorConnectionError")
var ErrMarketDataErrorAuthError = fmt.Errorf("MarketDataErrorAuthError")
var ErrMarketDataErrorRateLimitError = fmt.Errorf("MarketDataErrorRateLimitError")
var ErrMarketDataErrorInvalidSymbol = fmt.Errorf("MarketDataErrorInvalidSymbol")
var ErrMarketDataErrorParseError = fmt.Errorf("MarketDataErrorParseError")
var ErrMarketDataErrorTimeoutError = fmt.Errorf("MarketDataErrorTimeoutError")
var ErrMarketDataErrorWebSocketError = fmt.Errorf("MarketDataErrorWebSocketError")
var ErrMarketDataErrorClientClosed = fmt.Errorf("MarketDataErrorClientClosed")
var ErrMarketDataErrorConfigError = fmt.Errorf("MarketDataErrorConfigError")
var ErrMarketDataErrorApiError = fmt.Errorf("MarketDataErrorApiError")
var ErrMarketDataErrorOther = fmt.Errorf("MarketDataErrorOther")

// Variant structs
type MarketDataErrorConnectionError struct {
	Msg  string
	Info ErrorInfo
}

func NewMarketDataErrorConnectionError(
	msg string,
	info ErrorInfo,
) *MarketDataError {
	return &MarketDataError{err: &MarketDataErrorConnectionError{
		Msg:  msg,
		Info: info}}
}

func (e MarketDataErrorConnectionError) destroy() {
	FfiDestroyerString{}.Destroy(e.Msg)
	FfiDestroyerErrorInfo{}.Destroy(e.Info)
}

func (err MarketDataErrorConnectionError) Error() string {
	return fmt.Sprint("ConnectionError",
		": ",

		"Msg=",
		err.Msg,
		", ",
		"Info=",
		err.Info,
	)
}

func (self MarketDataErrorConnectionError) Is(target error) bool {
	return target == ErrMarketDataErrorConnectionError
}

type MarketDataErrorAuthError struct {
	Msg  string
	Info ErrorInfo
}

func NewMarketDataErrorAuthError(
	msg string,
	info ErrorInfo,
) *MarketDataError {
	return &MarketDataError{err: &MarketDataErrorAuthError{
		Msg:  msg,
		Info: info}}
}

func (e MarketDataErrorAuthError) destroy() {
	FfiDestroyerString{}.Destroy(e.Msg)
	FfiDestroyerErrorInfo{}.Destroy(e.Info)
}

func (err MarketDataErrorAuthError) Error() string {
	return fmt.Sprint("AuthError",
		": ",

		"Msg=",
		err.Msg,
		", ",
		"Info=",
		err.Info,
	)
}

func (self MarketDataErrorAuthError) Is(target error) bool {
	return target == ErrMarketDataErrorAuthError
}

type MarketDataErrorRateLimitError struct {
	Msg  string
	Info ErrorInfo
}

func NewMarketDataErrorRateLimitError(
	msg string,
	info ErrorInfo,
) *MarketDataError {
	return &MarketDataError{err: &MarketDataErrorRateLimitError{
		Msg:  msg,
		Info: info}}
}

func (e MarketDataErrorRateLimitError) destroy() {
	FfiDestroyerString{}.Destroy(e.Msg)
	FfiDestroyerErrorInfo{}.Destroy(e.Info)
}

func (err MarketDataErrorRateLimitError) Error() string {
	return fmt.Sprint("RateLimitError",
		": ",

		"Msg=",
		err.Msg,
		", ",
		"Info=",
		err.Info,
	)
}

func (self MarketDataErrorRateLimitError) Is(target error) bool {
	return target == ErrMarketDataErrorRateLimitError
}

type MarketDataErrorInvalidSymbol struct {
	Msg  string
	Info ErrorInfo
}

func NewMarketDataErrorInvalidSymbol(
	msg string,
	info ErrorInfo,
) *MarketDataError {
	return &MarketDataError{err: &MarketDataErrorInvalidSymbol{
		Msg:  msg,
		Info: info}}
}

func (e MarketDataErrorInvalidSymbol) destroy() {
	FfiDestroyerString{}.Destroy(e.Msg)
	FfiDestroyerErrorInfo{}.Destroy(e.Info)
}

func (err MarketDataErrorInvalidSymbol) Error() string {
	return fmt.Sprint("InvalidSymbol",
		": ",

		"Msg=",
		err.Msg,
		", ",
		"Info=",
		err.Info,
	)
}

func (self MarketDataErrorInvalidSymbol) Is(target error) bool {
	return target == ErrMarketDataErrorInvalidSymbol
}

type MarketDataErrorParseError struct {
	Msg  string
	Info ErrorInfo
}

func NewMarketDataErrorParseError(
	msg string,
	info ErrorInfo,
) *MarketDataError {
	return &MarketDataError{err: &MarketDataErrorParseError{
		Msg:  msg,
		Info: info}}
}

func (e MarketDataErrorParseError) destroy() {
	FfiDestroyerString{}.Destroy(e.Msg)
	FfiDestroyerErrorInfo{}.Destroy(e.Info)
}

func (err MarketDataErrorParseError) Error() string {
	return fmt.Sprint("ParseError",
		": ",

		"Msg=",
		err.Msg,
		", ",
		"Info=",
		err.Info,
	)
}

func (self MarketDataErrorParseError) Is(target error) bool {
	return target == ErrMarketDataErrorParseError
}

type MarketDataErrorTimeoutError struct {
	Msg  string
	Info ErrorInfo
}

func NewMarketDataErrorTimeoutError(
	msg string,
	info ErrorInfo,
) *MarketDataError {
	return &MarketDataError{err: &MarketDataErrorTimeoutError{
		Msg:  msg,
		Info: info}}
}

func (e MarketDataErrorTimeoutError) destroy() {
	FfiDestroyerString{}.Destroy(e.Msg)
	FfiDestroyerErrorInfo{}.Destroy(e.Info)
}

func (err MarketDataErrorTimeoutError) Error() string {
	return fmt.Sprint("TimeoutError",
		": ",

		"Msg=",
		err.Msg,
		", ",
		"Info=",
		err.Info,
	)
}

func (self MarketDataErrorTimeoutError) Is(target error) bool {
	return target == ErrMarketDataErrorTimeoutError
}

type MarketDataErrorWebSocketError struct {
	Msg  string
	Info ErrorInfo
}

func NewMarketDataErrorWebSocketError(
	msg string,
	info ErrorInfo,
) *MarketDataError {
	return &MarketDataError{err: &MarketDataErrorWebSocketError{
		Msg:  msg,
		Info: info}}
}

func (e MarketDataErrorWebSocketError) destroy() {
	FfiDestroyerString{}.Destroy(e.Msg)
	FfiDestroyerErrorInfo{}.Destroy(e.Info)
}

func (err MarketDataErrorWebSocketError) Error() string {
	return fmt.Sprint("WebSocketError",
		": ",

		"Msg=",
		err.Msg,
		", ",
		"Info=",
		err.Info,
	)
}

func (self MarketDataErrorWebSocketError) Is(target error) bool {
	return target == ErrMarketDataErrorWebSocketError
}

type MarketDataErrorClientClosed struct {
	Info ErrorInfo
}

func NewMarketDataErrorClientClosed(
	info ErrorInfo,
) *MarketDataError {
	return &MarketDataError{err: &MarketDataErrorClientClosed{
		Info: info}}
}

func (e MarketDataErrorClientClosed) destroy() {
	FfiDestroyerErrorInfo{}.Destroy(e.Info)
}

func (err MarketDataErrorClientClosed) Error() string {
	return fmt.Sprint("ClientClosed",
		": ",

		"Info=",
		err.Info,
	)
}

func (self MarketDataErrorClientClosed) Is(target error) bool {
	return target == ErrMarketDataErrorClientClosed
}

type MarketDataErrorConfigError struct {
	Msg  string
	Info ErrorInfo
}

func NewMarketDataErrorConfigError(
	msg string,
	info ErrorInfo,
) *MarketDataError {
	return &MarketDataError{err: &MarketDataErrorConfigError{
		Msg:  msg,
		Info: info}}
}

func (e MarketDataErrorConfigError) destroy() {
	FfiDestroyerString{}.Destroy(e.Msg)
	FfiDestroyerErrorInfo{}.Destroy(e.Info)
}

func (err MarketDataErrorConfigError) Error() string {
	return fmt.Sprint("ConfigError",
		": ",

		"Msg=",
		err.Msg,
		", ",
		"Info=",
		err.Info,
	)
}

func (self MarketDataErrorConfigError) Is(target error) bool {
	return target == ErrMarketDataErrorConfigError
}

type MarketDataErrorApiError struct {
	Msg  string
	Info ErrorInfo
}

func NewMarketDataErrorApiError(
	msg string,
	info ErrorInfo,
) *MarketDataError {
	return &MarketDataError{err: &MarketDataErrorApiError{
		Msg:  msg,
		Info: info}}
}

func (e MarketDataErrorApiError) destroy() {
	FfiDestroyerString{}.Destroy(e.Msg)
	FfiDestroyerErrorInfo{}.Destroy(e.Info)
}

func (err MarketDataErrorApiError) Error() string {
	return fmt.Sprint("ApiError",
		": ",

		"Msg=",
		err.Msg,
		", ",
		"Info=",
		err.Info,
	)
}

func (self MarketDataErrorApiError) Is(target error) bool {
	return target == ErrMarketDataErrorApiError
}

type MarketDataErrorOther struct {
	Msg  string
	Info ErrorInfo
}

func NewMarketDataErrorOther(
	msg string,
	info ErrorInfo,
) *MarketDataError {
	return &MarketDataError{err: &MarketDataErrorOther{
		Msg:  msg,
		Info: info}}
}

func (e MarketDataErrorOther) destroy() {
	FfiDestroyerString{}.Destroy(e.Msg)
	FfiDestroyerErrorInfo{}.Destroy(e.Info)
}

func (err MarketDataErrorOther) Error() string {
	return fmt.Sprint("Other",
		": ",

		"Msg=",
		err.Msg,
		", ",
		"Info=",
		err.Info,
	)
}

func (self MarketDataErrorOther) Is(target error) bool {
	return target == ErrMarketDataErrorOther
}

type FfiConverterMarketDataError struct{}

var FfiConverterMarketDataErrorINSTANCE = FfiConverterMarketDataError{}

func (c FfiConverterMarketDataError) Lift(eb RustBufferI) *MarketDataError {
	return LiftFromRustBuffer[*MarketDataError](c, eb)
}

func (c FfiConverterMarketDataError) Lower(value *MarketDataError) C.RustBuffer {
	return LowerIntoRustBuffer[*MarketDataError](c, value)
}

func (c FfiConverterMarketDataError) LowerExternal(value *MarketDataError) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[*MarketDataError](c, value))
}

func (c FfiConverterMarketDataError) Read(reader io.Reader) *MarketDataError {
	errorID := readUint32(reader)

	switch errorID {
	case 1:
		return &MarketDataError{&MarketDataErrorConnectionError{
			Msg:  FfiConverterStringINSTANCE.Read(reader),
			Info: FfiConverterErrorInfoINSTANCE.Read(reader),
		}}
	case 2:
		return &MarketDataError{&MarketDataErrorAuthError{
			Msg:  FfiConverterStringINSTANCE.Read(reader),
			Info: FfiConverterErrorInfoINSTANCE.Read(reader),
		}}
	case 3:
		return &MarketDataError{&MarketDataErrorRateLimitError{
			Msg:  FfiConverterStringINSTANCE.Read(reader),
			Info: FfiConverterErrorInfoINSTANCE.Read(reader),
		}}
	case 4:
		return &MarketDataError{&MarketDataErrorInvalidSymbol{
			Msg:  FfiConverterStringINSTANCE.Read(reader),
			Info: FfiConverterErrorInfoINSTANCE.Read(reader),
		}}
	case 5:
		return &MarketDataError{&MarketDataErrorParseError{
			Msg:  FfiConverterStringINSTANCE.Read(reader),
			Info: FfiConverterErrorInfoINSTANCE.Read(reader),
		}}
	case 6:
		return &MarketDataError{&MarketDataErrorTimeoutError{
			Msg:  FfiConverterStringINSTANCE.Read(reader),
			Info: FfiConverterErrorInfoINSTANCE.Read(reader),
		}}
	case 7:
		return &MarketDataError{&MarketDataErrorWebSocketError{
			Msg:  FfiConverterStringINSTANCE.Read(reader),
			Info: FfiConverterErrorInfoINSTANCE.Read(reader),
		}}
	case 8:
		return &MarketDataError{&MarketDataErrorClientClosed{
			Info: FfiConverterErrorInfoINSTANCE.Read(reader),
		}}
	case 9:
		return &MarketDataError{&MarketDataErrorConfigError{
			Msg:  FfiConverterStringINSTANCE.Read(reader),
			Info: FfiConverterErrorInfoINSTANCE.Read(reader),
		}}
	case 10:
		return &MarketDataError{&MarketDataErrorApiError{
			Msg:  FfiConverterStringINSTANCE.Read(reader),
			Info: FfiConverterErrorInfoINSTANCE.Read(reader),
		}}
	case 11:
		return &MarketDataError{&MarketDataErrorOther{
			Msg:  FfiConverterStringINSTANCE.Read(reader),
			Info: FfiConverterErrorInfoINSTANCE.Read(reader),
		}}
	default:
		panic(fmt.Sprintf("Unknown error code %d in FfiConverterMarketDataError.Read()", errorID))
	}
}

func (c FfiConverterMarketDataError) Write(writer io.Writer, value *MarketDataError) {
	switch variantValue := value.err.(type) {
	case *MarketDataErrorConnectionError:
		writeInt32(writer, 1)
		FfiConverterStringINSTANCE.Write(writer, variantValue.Msg)
		FfiConverterErrorInfoINSTANCE.Write(writer, variantValue.Info)
	case *MarketDataErrorAuthError:
		writeInt32(writer, 2)
		FfiConverterStringINSTANCE.Write(writer, variantValue.Msg)
		FfiConverterErrorInfoINSTANCE.Write(writer, variantValue.Info)
	case *MarketDataErrorRateLimitError:
		writeInt32(writer, 3)
		FfiConverterStringINSTANCE.Write(writer, variantValue.Msg)
		FfiConverterErrorInfoINSTANCE.Write(writer, variantValue.Info)
	case *MarketDataErrorInvalidSymbol:
		writeInt32(writer, 4)
		FfiConverterStringINSTANCE.Write(writer, variantValue.Msg)
		FfiConverterErrorInfoINSTANCE.Write(writer, variantValue.Info)
	case *MarketDataErrorParseError:
		writeInt32(writer, 5)
		FfiConverterStringINSTANCE.Write(writer, variantValue.Msg)
		FfiConverterErrorInfoINSTANCE.Write(writer, variantValue.Info)
	case *MarketDataErrorTimeoutError:
		writeInt32(writer, 6)
		FfiConverterStringINSTANCE.Write(writer, variantValue.Msg)
		FfiConverterErrorInfoINSTANCE.Write(writer, variantValue.Info)
	case *MarketDataErrorWebSocketError:
		writeInt32(writer, 7)
		FfiConverterStringINSTANCE.Write(writer, variantValue.Msg)
		FfiConverterErrorInfoINSTANCE.Write(writer, variantValue.Info)
	case *MarketDataErrorClientClosed:
		writeInt32(writer, 8)
		FfiConverterErrorInfoINSTANCE.Write(writer, variantValue.Info)
	case *MarketDataErrorConfigError:
		writeInt32(writer, 9)
		FfiConverterStringINSTANCE.Write(writer, variantValue.Msg)
		FfiConverterErrorInfoINSTANCE.Write(writer, variantValue.Info)
	case *MarketDataErrorApiError:
		writeInt32(writer, 10)
		FfiConverterStringINSTANCE.Write(writer, variantValue.Msg)
		FfiConverterErrorInfoINSTANCE.Write(writer, variantValue.Info)
	case *MarketDataErrorOther:
		writeInt32(writer, 11)
		FfiConverterStringINSTANCE.Write(writer, variantValue.Msg)
		FfiConverterErrorInfoINSTANCE.Write(writer, variantValue.Info)
	default:
		_ = variantValue
		panic(fmt.Sprintf("invalid error value `%v` in FfiConverterMarketDataError.Write", value))
	}
}

type FfiDestroyerMarketDataError struct{}

func (_ FfiDestroyerMarketDataError) Destroy(value *MarketDataError) {
	switch variantValue := value.err.(type) {
	case MarketDataErrorConnectionError:
		variantValue.destroy()
	case MarketDataErrorAuthError:
		variantValue.destroy()
	case MarketDataErrorRateLimitError:
		variantValue.destroy()
	case MarketDataErrorInvalidSymbol:
		variantValue.destroy()
	case MarketDataErrorParseError:
		variantValue.destroy()
	case MarketDataErrorTimeoutError:
		variantValue.destroy()
	case MarketDataErrorWebSocketError:
		variantValue.destroy()
	case MarketDataErrorClientClosed:
		variantValue.destroy()
	case MarketDataErrorConfigError:
		variantValue.destroy()
	case MarketDataErrorApiError:
		variantValue.destroy()
	case MarketDataErrorOther:
		variantValue.destroy()
	default:
		_ = variantValue
		panic(fmt.Sprintf("invalid error value `%v` in FfiDestroyerMarketDataError.Destroy", value))
	}
}

// What the client does with an inbound message while its queue already
// holds `buffer` unread messages.
type MessageOverflowRecord uint

const (
	// Drop new messages and report them through `on_messages_dropped`.
	MessageOverflowRecordDropNewest MessageOverflowRecord = 1
	// Never drop: the queue grows while `on_message` lags.
	MessageOverflowRecordUnbounded MessageOverflowRecord = 2
)

type FfiConverterMessageOverflowRecord struct{}

var FfiConverterMessageOverflowRecordINSTANCE = FfiConverterMessageOverflowRecord{}

func (c FfiConverterMessageOverflowRecord) Lift(rb RustBufferI) MessageOverflowRecord {
	return LiftFromRustBuffer[MessageOverflowRecord](c, rb)
}

func (c FfiConverterMessageOverflowRecord) Lower(value MessageOverflowRecord) C.RustBuffer {
	return LowerIntoRustBuffer[MessageOverflowRecord](c, value)
}

func (c FfiConverterMessageOverflowRecord) LowerExternal(value MessageOverflowRecord) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[MessageOverflowRecord](c, value))
}
func (FfiConverterMessageOverflowRecord) Read(reader io.Reader) MessageOverflowRecord {
	id := readInt32(reader)
	return MessageOverflowRecord(id)
}

func (FfiConverterMessageOverflowRecord) Write(writer io.Writer, value MessageOverflowRecord) {
	writeInt32(writer, int32(value))
}

type FfiDestroyerMessageOverflowRecord struct{}

func (_ FfiDestroyerMessageOverflowRecord) Destroy(value MessageOverflowRecord) {
}

// Endpoint type for WebSocket connection
type WebSocketEndpoint uint

const (
	// Stock market data endpoint
	WebSocketEndpointStock WebSocketEndpoint = 1
	// Futures and options market data endpoint
	WebSocketEndpointFutOpt WebSocketEndpoint = 2
)

type FfiConverterWebSocketEndpoint struct{}

var FfiConverterWebSocketEndpointINSTANCE = FfiConverterWebSocketEndpoint{}

func (c FfiConverterWebSocketEndpoint) Lift(rb RustBufferI) WebSocketEndpoint {
	return LiftFromRustBuffer[WebSocketEndpoint](c, rb)
}

func (c FfiConverterWebSocketEndpoint) Lower(value WebSocketEndpoint) C.RustBuffer {
	return LowerIntoRustBuffer[WebSocketEndpoint](c, value)
}

func (c FfiConverterWebSocketEndpoint) LowerExternal(value WebSocketEndpoint) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[WebSocketEndpoint](c, value))
}
func (FfiConverterWebSocketEndpoint) Read(reader io.Reader) WebSocketEndpoint {
	id := readInt32(reader)
	return WebSocketEndpoint(id)
}

func (FfiConverterWebSocketEndpoint) Write(writer io.Writer, value WebSocketEndpoint) {
	writeInt32(writer, int32(value))
}

type FfiDestroyerWebSocketEndpoint struct{}

func (_ FfiDestroyerWebSocketEndpoint) Destroy(value WebSocketEndpoint) {
}

type FfiConverterOptionalUint16 struct{}

var FfiConverterOptionalUint16INSTANCE = FfiConverterOptionalUint16{}

func (c FfiConverterOptionalUint16) Lift(rb RustBufferI) *uint16 {
	return LiftFromRustBuffer[*uint16](c, rb)
}

func (_ FfiConverterOptionalUint16) Read(reader io.Reader) *uint16 {
	if readInt8(reader) == 0 {
		return nil
	}
	temp := FfiConverterUint16INSTANCE.Read(reader)
	return &temp
}

func (c FfiConverterOptionalUint16) Lower(value *uint16) C.RustBuffer {
	return LowerIntoRustBuffer[*uint16](c, value)
}

func (c FfiConverterOptionalUint16) LowerExternal(value *uint16) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[*uint16](c, value))
}

func (_ FfiConverterOptionalUint16) Write(writer io.Writer, value *uint16) {
	if value == nil {
		writeInt8(writer, 0)
	} else {
		writeInt8(writer, 1)
		FfiConverterUint16INSTANCE.Write(writer, *value)
	}
}

type FfiDestroyerOptionalUint16 struct{}

func (_ FfiDestroyerOptionalUint16) Destroy(value *uint16) {
	if value != nil {
		FfiDestroyerUint16{}.Destroy(*value)
	}
}

type FfiConverterOptionalUint32 struct{}

var FfiConverterOptionalUint32INSTANCE = FfiConverterOptionalUint32{}

func (c FfiConverterOptionalUint32) Lift(rb RustBufferI) *uint32 {
	return LiftFromRustBuffer[*uint32](c, rb)
}

func (_ FfiConverterOptionalUint32) Read(reader io.Reader) *uint32 {
	if readInt8(reader) == 0 {
		return nil
	}
	temp := FfiConverterUint32INSTANCE.Read(reader)
	return &temp
}

func (c FfiConverterOptionalUint32) Lower(value *uint32) C.RustBuffer {
	return LowerIntoRustBuffer[*uint32](c, value)
}

func (c FfiConverterOptionalUint32) LowerExternal(value *uint32) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[*uint32](c, value))
}

func (_ FfiConverterOptionalUint32) Write(writer io.Writer, value *uint32) {
	if value == nil {
		writeInt8(writer, 0)
	} else {
		writeInt8(writer, 1)
		FfiConverterUint32INSTANCE.Write(writer, *value)
	}
}

type FfiDestroyerOptionalUint32 struct{}

func (_ FfiDestroyerOptionalUint32) Destroy(value *uint32) {
	if value != nil {
		FfiDestroyerUint32{}.Destroy(*value)
	}
}

type FfiConverterOptionalInt32 struct{}

var FfiConverterOptionalInt32INSTANCE = FfiConverterOptionalInt32{}

func (c FfiConverterOptionalInt32) Lift(rb RustBufferI) *int32 {
	return LiftFromRustBuffer[*int32](c, rb)
}

func (_ FfiConverterOptionalInt32) Read(reader io.Reader) *int32 {
	if readInt8(reader) == 0 {
		return nil
	}
	temp := FfiConverterInt32INSTANCE.Read(reader)
	return &temp
}

func (c FfiConverterOptionalInt32) Lower(value *int32) C.RustBuffer {
	return LowerIntoRustBuffer[*int32](c, value)
}

func (c FfiConverterOptionalInt32) LowerExternal(value *int32) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[*int32](c, value))
}

func (_ FfiConverterOptionalInt32) Write(writer io.Writer, value *int32) {
	if value == nil {
		writeInt8(writer, 0)
	} else {
		writeInt8(writer, 1)
		FfiConverterInt32INSTANCE.Write(writer, *value)
	}
}

type FfiDestroyerOptionalInt32 struct{}

func (_ FfiDestroyerOptionalInt32) Destroy(value *int32) {
	if value != nil {
		FfiDestroyerInt32{}.Destroy(*value)
	}
}

type FfiConverterOptionalUint64 struct{}

var FfiConverterOptionalUint64INSTANCE = FfiConverterOptionalUint64{}

func (c FfiConverterOptionalUint64) Lift(rb RustBufferI) *uint64 {
	return LiftFromRustBuffer[*uint64](c, rb)
}

func (_ FfiConverterOptionalUint64) Read(reader io.Reader) *uint64 {
	if readInt8(reader) == 0 {
		return nil
	}
	temp := FfiConverterUint64INSTANCE.Read(reader)
	return &temp
}

func (c FfiConverterOptionalUint64) Lower(value *uint64) C.RustBuffer {
	return LowerIntoRustBuffer[*uint64](c, value)
}

func (c FfiConverterOptionalUint64) LowerExternal(value *uint64) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[*uint64](c, value))
}

func (_ FfiConverterOptionalUint64) Write(writer io.Writer, value *uint64) {
	if value == nil {
		writeInt8(writer, 0)
	} else {
		writeInt8(writer, 1)
		FfiConverterUint64INSTANCE.Write(writer, *value)
	}
}

type FfiDestroyerOptionalUint64 struct{}

func (_ FfiDestroyerOptionalUint64) Destroy(value *uint64) {
	if value != nil {
		FfiDestroyerUint64{}.Destroy(*value)
	}
}

type FfiConverterOptionalFloat64 struct{}

var FfiConverterOptionalFloat64INSTANCE = FfiConverterOptionalFloat64{}

func (c FfiConverterOptionalFloat64) Lift(rb RustBufferI) *float64 {
	return LiftFromRustBuffer[*float64](c, rb)
}

func (_ FfiConverterOptionalFloat64) Read(reader io.Reader) *float64 {
	if readInt8(reader) == 0 {
		return nil
	}
	temp := FfiConverterFloat64INSTANCE.Read(reader)
	return &temp
}

func (c FfiConverterOptionalFloat64) Lower(value *float64) C.RustBuffer {
	return LowerIntoRustBuffer[*float64](c, value)
}

func (c FfiConverterOptionalFloat64) LowerExternal(value *float64) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[*float64](c, value))
}

func (_ FfiConverterOptionalFloat64) Write(writer io.Writer, value *float64) {
	if value == nil {
		writeInt8(writer, 0)
	} else {
		writeInt8(writer, 1)
		FfiConverterFloat64INSTANCE.Write(writer, *value)
	}
}

type FfiDestroyerOptionalFloat64 struct{}

func (_ FfiDestroyerOptionalFloat64) Destroy(value *float64) {
	if value != nil {
		FfiDestroyerFloat64{}.Destroy(*value)
	}
}

type FfiConverterOptionalBool struct{}

var FfiConverterOptionalBoolINSTANCE = FfiConverterOptionalBool{}

func (c FfiConverterOptionalBool) Lift(rb RustBufferI) *bool {
	return LiftFromRustBuffer[*bool](c, rb)
}

func (_ FfiConverterOptionalBool) Read(reader io.Reader) *bool {
	if readInt8(reader) == 0 {
		return nil
	}
	temp := FfiConverterBoolINSTANCE.Read(reader)
	return &temp
}

func (c FfiConverterOptionalBool) Lower(value *bool) C.RustBuffer {
	return LowerIntoRustBuffer[*bool](c, value)
}

func (c FfiConverterOptionalBool) LowerExternal(value *bool) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[*bool](c, value))
}

func (_ FfiConverterOptionalBool) Write(writer io.Writer, value *bool) {
	if value == nil {
		writeInt8(writer, 0)
	} else {
		writeInt8(writer, 1)
		FfiConverterBoolINSTANCE.Write(writer, *value)
	}
}

type FfiDestroyerOptionalBool struct{}

func (_ FfiDestroyerOptionalBool) Destroy(value *bool) {
	if value != nil {
		FfiDestroyerBool{}.Destroy(*value)
	}
}

type FfiConverterOptionalString struct{}

var FfiConverterOptionalStringINSTANCE = FfiConverterOptionalString{}

func (c FfiConverterOptionalString) Lift(rb RustBufferI) *string {
	return LiftFromRustBuffer[*string](c, rb)
}

func (_ FfiConverterOptionalString) Read(reader io.Reader) *string {
	if readInt8(reader) == 0 {
		return nil
	}
	temp := FfiConverterStringINSTANCE.Read(reader)
	return &temp
}

func (c FfiConverterOptionalString) Lower(value *string) C.RustBuffer {
	return LowerIntoRustBuffer[*string](c, value)
}

func (c FfiConverterOptionalString) LowerExternal(value *string) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[*string](c, value))
}

func (_ FfiConverterOptionalString) Write(writer io.Writer, value *string) {
	if value == nil {
		writeInt8(writer, 0)
	} else {
		writeInt8(writer, 1)
		FfiConverterStringINSTANCE.Write(writer, *value)
	}
}

type FfiDestroyerOptionalString struct{}

func (_ FfiDestroyerOptionalString) Destroy(value *string) {
	if value != nil {
		FfiDestroyerString{}.Destroy(*value)
	}
}

type FfiConverterOptionalBytes struct{}

var FfiConverterOptionalBytesINSTANCE = FfiConverterOptionalBytes{}

func (c FfiConverterOptionalBytes) Lift(rb RustBufferI) *[]byte {
	return LiftFromRustBuffer[*[]byte](c, rb)
}

func (_ FfiConverterOptionalBytes) Read(reader io.Reader) *[]byte {
	if readInt8(reader) == 0 {
		return nil
	}
	temp := FfiConverterBytesINSTANCE.Read(reader)
	return &temp
}

func (c FfiConverterOptionalBytes) Lower(value *[]byte) C.RustBuffer {
	return LowerIntoRustBuffer[*[]byte](c, value)
}

func (c FfiConverterOptionalBytes) LowerExternal(value *[]byte) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[*[]byte](c, value))
}

func (_ FfiConverterOptionalBytes) Write(writer io.Writer, value *[]byte) {
	if value == nil {
		writeInt8(writer, 0)
	} else {
		writeInt8(writer, 1)
		FfiConverterBytesINSTANCE.Write(writer, *value)
	}
}

type FfiDestroyerOptionalBytes struct{}

func (_ FfiDestroyerOptionalBytes) Destroy(value *[]byte) {
	if value != nil {
		FfiDestroyerBytes{}.Destroy(*value)
	}
}

type FfiConverterOptionalAfterHoursParams struct{}

var FfiConverterOptionalAfterHoursParamsINSTANCE = FfiConverterOptionalAfterHoursParams{}

func (c FfiConverterOptionalAfterHoursParams) Lift(rb RustBufferI) *AfterHoursParams {
	return LiftFromRustBuffer[*AfterHoursParams](c, rb)
}

func (_ FfiConverterOptionalAfterHoursParams) Read(reader io.Reader) *AfterHoursParams {
	if readInt8(reader) == 0 {
		return nil
	}
	temp := FfiConverterAfterHoursParamsINSTANCE.Read(reader)
	return &temp
}

func (c FfiConverterOptionalAfterHoursParams) Lower(value *AfterHoursParams) C.RustBuffer {
	return LowerIntoRustBuffer[*AfterHoursParams](c, value)
}

func (c FfiConverterOptionalAfterHoursParams) LowerExternal(value *AfterHoursParams) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[*AfterHoursParams](c, value))
}

func (_ FfiConverterOptionalAfterHoursParams) Write(writer io.Writer, value *AfterHoursParams) {
	if value == nil {
		writeInt8(writer, 0)
	} else {
		writeInt8(writer, 1)
		FfiConverterAfterHoursParamsINSTANCE.Write(writer, *value)
	}
}

type FfiDestroyerOptionalAfterHoursParams struct{}

func (_ FfiDestroyerOptionalAfterHoursParams) Destroy(value *AfterHoursParams) {
	if value != nil {
		FfiDestroyerAfterHoursParams{}.Destroy(*value)
	}
}

type FfiConverterOptionalConnectionConfigRecord struct{}

var FfiConverterOptionalConnectionConfigRecordINSTANCE = FfiConverterOptionalConnectionConfigRecord{}

func (c FfiConverterOptionalConnectionConfigRecord) Lift(rb RustBufferI) *ConnectionConfigRecord {
	return LiftFromRustBuffer[*ConnectionConfigRecord](c, rb)
}

func (_ FfiConverterOptionalConnectionConfigRecord) Read(reader io.Reader) *ConnectionConfigRecord {
	if readInt8(reader) == 0 {
		return nil
	}
	temp := FfiConverterConnectionConfigRecordINSTANCE.Read(reader)
	return &temp
}

func (c FfiConverterOptionalConnectionConfigRecord) Lower(value *ConnectionConfigRecord) C.RustBuffer {
	return LowerIntoRustBuffer[*ConnectionConfigRecord](c, value)
}

func (c FfiConverterOptionalConnectionConfigRecord) LowerExternal(value *ConnectionConfigRecord) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[*ConnectionConfigRecord](c, value))
}

func (_ FfiConverterOptionalConnectionConfigRecord) Write(writer io.Writer, value *ConnectionConfigRecord) {
	if value == nil {
		writeInt8(writer, 0)
	} else {
		writeInt8(writer, 1)
		FfiConverterConnectionConfigRecordINSTANCE.Write(writer, *value)
	}
}

type FfiDestroyerOptionalConnectionConfigRecord struct{}

func (_ FfiDestroyerOptionalConnectionConfigRecord) Destroy(value *ConnectionConfigRecord) {
	if value != nil {
		FfiDestroyerConnectionConfigRecord{}.Destroy(*value)
	}
}

type FfiConverterOptionalCorporateActionsParams struct{}

var FfiConverterOptionalCorporateActionsParamsINSTANCE = FfiConverterOptionalCorporateActionsParams{}

func (c FfiConverterOptionalCorporateActionsParams) Lift(rb RustBufferI) *CorporateActionsParams {
	return LiftFromRustBuffer[*CorporateActionsParams](c, rb)
}

func (_ FfiConverterOptionalCorporateActionsParams) Read(reader io.Reader) *CorporateActionsParams {
	if readInt8(reader) == 0 {
		return nil
	}
	temp := FfiConverterCorporateActionsParamsINSTANCE.Read(reader)
	return &temp
}

func (c FfiConverterOptionalCorporateActionsParams) Lower(value *CorporateActionsParams) C.RustBuffer {
	return LowerIntoRustBuffer[*CorporateActionsParams](c, value)
}

func (c FfiConverterOptionalCorporateActionsParams) LowerExternal(value *CorporateActionsParams) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[*CorporateActionsParams](c, value))
}

func (_ FfiConverterOptionalCorporateActionsParams) Write(writer io.Writer, value *CorporateActionsParams) {
	if value == nil {
		writeInt8(writer, 0)
	} else {
		writeInt8(writer, 1)
		FfiConverterCorporateActionsParamsINSTANCE.Write(writer, *value)
	}
}

type FfiDestroyerOptionalCorporateActionsParams struct{}

func (_ FfiDestroyerOptionalCorporateActionsParams) Destroy(value *CorporateActionsParams) {
	if value != nil {
		FfiDestroyerCorporateActionsParams{}.Destroy(*value)
	}
}

type FfiConverterOptionalFutOptCandlesParams struct{}

var FfiConverterOptionalFutOptCandlesParamsINSTANCE = FfiConverterOptionalFutOptCandlesParams{}

func (c FfiConverterOptionalFutOptCandlesParams) Lift(rb RustBufferI) *FutOptCandlesParams {
	return LiftFromRustBuffer[*FutOptCandlesParams](c, rb)
}

func (_ FfiConverterOptionalFutOptCandlesParams) Read(reader io.Reader) *FutOptCandlesParams {
	if readInt8(reader) == 0 {
		return nil
	}
	temp := FfiConverterFutOptCandlesParamsINSTANCE.Read(reader)
	return &temp
}

func (c FfiConverterOptionalFutOptCandlesParams) Lower(value *FutOptCandlesParams) C.RustBuffer {
	return LowerIntoRustBuffer[*FutOptCandlesParams](c, value)
}

func (c FfiConverterOptionalFutOptCandlesParams) LowerExternal(value *FutOptCandlesParams) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[*FutOptCandlesParams](c, value))
}

func (_ FfiConverterOptionalFutOptCandlesParams) Write(writer io.Writer, value *FutOptCandlesParams) {
	if value == nil {
		writeInt8(writer, 0)
	} else {
		writeInt8(writer, 1)
		FfiConverterFutOptCandlesParamsINSTANCE.Write(writer, *value)
	}
}

type FfiDestroyerOptionalFutOptCandlesParams struct{}

func (_ FfiDestroyerOptionalFutOptCandlesParams) Destroy(value *FutOptCandlesParams) {
	if value != nil {
		FfiDestroyerFutOptCandlesParams{}.Destroy(*value)
	}
}

type FfiConverterOptionalFutOptDailyParams struct{}

var FfiConverterOptionalFutOptDailyParamsINSTANCE = FfiConverterOptionalFutOptDailyParams{}

func (c FfiConverterOptionalFutOptDailyParams) Lift(rb RustBufferI) *FutOptDailyParams {
	return LiftFromRustBuffer[*FutOptDailyParams](c, rb)
}

func (_ FfiConverterOptionalFutOptDailyParams) Read(reader io.Reader) *FutOptDailyParams {
	if readInt8(reader) == 0 {
		return nil
	}
	temp := FfiConverterFutOptDailyParamsINSTANCE.Read(reader)
	return &temp
}

func (c FfiConverterOptionalFutOptDailyParams) Lower(value *FutOptDailyParams) C.RustBuffer {
	return LowerIntoRustBuffer[*FutOptDailyParams](c, value)
}

func (c FfiConverterOptionalFutOptDailyParams) LowerExternal(value *FutOptDailyParams) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[*FutOptDailyParams](c, value))
}

func (_ FfiConverterOptionalFutOptDailyParams) Write(writer io.Writer, value *FutOptDailyParams) {
	if value == nil {
		writeInt8(writer, 0)
	} else {
		writeInt8(writer, 1)
		FfiConverterFutOptDailyParamsINSTANCE.Write(writer, *value)
	}
}

type FfiDestroyerOptionalFutOptDailyParams struct{}

func (_ FfiDestroyerOptionalFutOptDailyParams) Destroy(value *FutOptDailyParams) {
	if value != nil {
		FfiDestroyerFutOptDailyParams{}.Destroy(*value)
	}
}

type FfiConverterOptionalFutOptHistoricalCandlesParams struct{}

var FfiConverterOptionalFutOptHistoricalCandlesParamsINSTANCE = FfiConverterOptionalFutOptHistoricalCandlesParams{}

func (c FfiConverterOptionalFutOptHistoricalCandlesParams) Lift(rb RustBufferI) *FutOptHistoricalCandlesParams {
	return LiftFromRustBuffer[*FutOptHistoricalCandlesParams](c, rb)
}

func (_ FfiConverterOptionalFutOptHistoricalCandlesParams) Read(reader io.Reader) *FutOptHistoricalCandlesParams {
	if readInt8(reader) == 0 {
		return nil
	}
	temp := FfiConverterFutOptHistoricalCandlesParamsINSTANCE.Read(reader)
	return &temp
}

func (c FfiConverterOptionalFutOptHistoricalCandlesParams) Lower(value *FutOptHistoricalCandlesParams) C.RustBuffer {
	return LowerIntoRustBuffer[*FutOptHistoricalCandlesParams](c, value)
}

func (c FfiConverterOptionalFutOptHistoricalCandlesParams) LowerExternal(value *FutOptHistoricalCandlesParams) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[*FutOptHistoricalCandlesParams](c, value))
}

func (_ FfiConverterOptionalFutOptHistoricalCandlesParams) Write(writer io.Writer, value *FutOptHistoricalCandlesParams) {
	if value == nil {
		writeInt8(writer, 0)
	} else {
		writeInt8(writer, 1)
		FfiConverterFutOptHistoricalCandlesParamsINSTANCE.Write(writer, *value)
	}
}

type FfiDestroyerOptionalFutOptHistoricalCandlesParams struct{}

func (_ FfiDestroyerOptionalFutOptHistoricalCandlesParams) Destroy(value *FutOptHistoricalCandlesParams) {
	if value != nil {
		FfiDestroyerFutOptHistoricalCandlesParams{}.Destroy(*value)
	}
}

type FfiConverterOptionalFutOptProductsParams struct{}

var FfiConverterOptionalFutOptProductsParamsINSTANCE = FfiConverterOptionalFutOptProductsParams{}

func (c FfiConverterOptionalFutOptProductsParams) Lift(rb RustBufferI) *FutOptProductsParams {
	return LiftFromRustBuffer[*FutOptProductsParams](c, rb)
}

func (_ FfiConverterOptionalFutOptProductsParams) Read(reader io.Reader) *FutOptProductsParams {
	if readInt8(reader) == 0 {
		return nil
	}
	temp := FfiConverterFutOptProductsParamsINSTANCE.Read(reader)
	return &temp
}

func (c FfiConverterOptionalFutOptProductsParams) Lower(value *FutOptProductsParams) C.RustBuffer {
	return LowerIntoRustBuffer[*FutOptProductsParams](c, value)
}

func (c FfiConverterOptionalFutOptProductsParams) LowerExternal(value *FutOptProductsParams) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[*FutOptProductsParams](c, value))
}

func (_ FfiConverterOptionalFutOptProductsParams) Write(writer io.Writer, value *FutOptProductsParams) {
	if value == nil {
		writeInt8(writer, 0)
	} else {
		writeInt8(writer, 1)
		FfiConverterFutOptProductsParamsINSTANCE.Write(writer, *value)
	}
}

type FfiDestroyerOptionalFutOptProductsParams struct{}

func (_ FfiDestroyerOptionalFutOptProductsParams) Destroy(value *FutOptProductsParams) {
	if value != nil {
		FfiDestroyerFutOptProductsParams{}.Destroy(*value)
	}
}

type FfiConverterOptionalFutOptTickersParams struct{}

var FfiConverterOptionalFutOptTickersParamsINSTANCE = FfiConverterOptionalFutOptTickersParams{}

func (c FfiConverterOptionalFutOptTickersParams) Lift(rb RustBufferI) *FutOptTickersParams {
	return LiftFromRustBuffer[*FutOptTickersParams](c, rb)
}

func (_ FfiConverterOptionalFutOptTickersParams) Read(reader io.Reader) *FutOptTickersParams {
	if readInt8(reader) == 0 {
		return nil
	}
	temp := FfiConverterFutOptTickersParamsINSTANCE.Read(reader)
	return &temp
}

func (c FfiConverterOptionalFutOptTickersParams) Lower(value *FutOptTickersParams) C.RustBuffer {
	return LowerIntoRustBuffer[*FutOptTickersParams](c, value)
}

func (c FfiConverterOptionalFutOptTickersParams) LowerExternal(value *FutOptTickersParams) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[*FutOptTickersParams](c, value))
}

func (_ FfiConverterOptionalFutOptTickersParams) Write(writer io.Writer, value *FutOptTickersParams) {
	if value == nil {
		writeInt8(writer, 0)
	} else {
		writeInt8(writer, 1)
		FfiConverterFutOptTickersParamsINSTANCE.Write(writer, *value)
	}
}

type FfiDestroyerOptionalFutOptTickersParams struct{}

func (_ FfiDestroyerOptionalFutOptTickersParams) Destroy(value *FutOptTickersParams) {
	if value != nil {
		FfiDestroyerFutOptTickersParams{}.Destroy(*value)
	}
}

type FfiConverterOptionalFutOptTradesParams struct{}

var FfiConverterOptionalFutOptTradesParamsINSTANCE = FfiConverterOptionalFutOptTradesParams{}

func (c FfiConverterOptionalFutOptTradesParams) Lift(rb RustBufferI) *FutOptTradesParams {
	return LiftFromRustBuffer[*FutOptTradesParams](c, rb)
}

func (_ FfiConverterOptionalFutOptTradesParams) Read(reader io.Reader) *FutOptTradesParams {
	if readInt8(reader) == 0 {
		return nil
	}
	temp := FfiConverterFutOptTradesParamsINSTANCE.Read(reader)
	return &temp
}

func (c FfiConverterOptionalFutOptTradesParams) Lower(value *FutOptTradesParams) C.RustBuffer {
	return LowerIntoRustBuffer[*FutOptTradesParams](c, value)
}

func (c FfiConverterOptionalFutOptTradesParams) LowerExternal(value *FutOptTradesParams) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[*FutOptTradesParams](c, value))
}

func (_ FfiConverterOptionalFutOptTradesParams) Write(writer io.Writer, value *FutOptTradesParams) {
	if value == nil {
		writeInt8(writer, 0)
	} else {
		writeInt8(writer, 1)
		FfiConverterFutOptTradesParamsINSTANCE.Write(writer, *value)
	}
}

type FfiDestroyerOptionalFutOptTradesParams struct{}

func (_ FfiDestroyerOptionalFutOptTradesParams) Destroy(value *FutOptTradesParams) {
	if value != nil {
		FfiDestroyerFutOptTradesParams{}.Destroy(*value)
	}
}

type FfiConverterOptionalHealthCheckConfigRecord struct{}

var FfiConverterOptionalHealthCheckConfigRecordINSTANCE = FfiConverterOptionalHealthCheckConfigRecord{}

func (c FfiConverterOptionalHealthCheckConfigRecord) Lift(rb RustBufferI) *HealthCheckConfigRecord {
	return LiftFromRustBuffer[*HealthCheckConfigRecord](c, rb)
}

func (_ FfiConverterOptionalHealthCheckConfigRecord) Read(reader io.Reader) *HealthCheckConfigRecord {
	if readInt8(reader) == 0 {
		return nil
	}
	temp := FfiConverterHealthCheckConfigRecordINSTANCE.Read(reader)
	return &temp
}

func (c FfiConverterOptionalHealthCheckConfigRecord) Lower(value *HealthCheckConfigRecord) C.RustBuffer {
	return LowerIntoRustBuffer[*HealthCheckConfigRecord](c, value)
}

func (c FfiConverterOptionalHealthCheckConfigRecord) LowerExternal(value *HealthCheckConfigRecord) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[*HealthCheckConfigRecord](c, value))
}

func (_ FfiConverterOptionalHealthCheckConfigRecord) Write(writer io.Writer, value *HealthCheckConfigRecord) {
	if value == nil {
		writeInt8(writer, 0)
	} else {
		writeInt8(writer, 1)
		FfiConverterHealthCheckConfigRecordINSTANCE.Write(writer, *value)
	}
}

type FfiDestroyerOptionalHealthCheckConfigRecord struct{}

func (_ FfiDestroyerOptionalHealthCheckConfigRecord) Destroy(value *HealthCheckConfigRecord) {
	if value != nil {
		FfiDestroyerHealthCheckConfigRecord{}.Destroy(*value)
	}
}

type FfiConverterOptionalMessageQueueConfigRecord struct{}

var FfiConverterOptionalMessageQueueConfigRecordINSTANCE = FfiConverterOptionalMessageQueueConfigRecord{}

func (c FfiConverterOptionalMessageQueueConfigRecord) Lift(rb RustBufferI) *MessageQueueConfigRecord {
	return LiftFromRustBuffer[*MessageQueueConfigRecord](c, rb)
}

func (_ FfiConverterOptionalMessageQueueConfigRecord) Read(reader io.Reader) *MessageQueueConfigRecord {
	if readInt8(reader) == 0 {
		return nil
	}
	temp := FfiConverterMessageQueueConfigRecordINSTANCE.Read(reader)
	return &temp
}

func (c FfiConverterOptionalMessageQueueConfigRecord) Lower(value *MessageQueueConfigRecord) C.RustBuffer {
	return LowerIntoRustBuffer[*MessageQueueConfigRecord](c, value)
}

func (c FfiConverterOptionalMessageQueueConfigRecord) LowerExternal(value *MessageQueueConfigRecord) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[*MessageQueueConfigRecord](c, value))
}

func (_ FfiConverterOptionalMessageQueueConfigRecord) Write(writer io.Writer, value *MessageQueueConfigRecord) {
	if value == nil {
		writeInt8(writer, 0)
	} else {
		writeInt8(writer, 1)
		FfiConverterMessageQueueConfigRecordINSTANCE.Write(writer, *value)
	}
}

type FfiDestroyerOptionalMessageQueueConfigRecord struct{}

func (_ FfiDestroyerOptionalMessageQueueConfigRecord) Destroy(value *MessageQueueConfigRecord) {
	if value != nil {
		FfiDestroyerMessageQueueConfigRecord{}.Destroy(*value)
	}
}

type FfiConverterOptionalMoversParams struct{}

var FfiConverterOptionalMoversParamsINSTANCE = FfiConverterOptionalMoversParams{}

func (c FfiConverterOptionalMoversParams) Lift(rb RustBufferI) *MoversParams {
	return LiftFromRustBuffer[*MoversParams](c, rb)
}

func (_ FfiConverterOptionalMoversParams) Read(reader io.Reader) *MoversParams {
	if readInt8(reader) == 0 {
		return nil
	}
	temp := FfiConverterMoversParamsINSTANCE.Read(reader)
	return &temp
}

func (c FfiConverterOptionalMoversParams) Lower(value *MoversParams) C.RustBuffer {
	return LowerIntoRustBuffer[*MoversParams](c, value)
}

func (c FfiConverterOptionalMoversParams) LowerExternal(value *MoversParams) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[*MoversParams](c, value))
}

func (_ FfiConverterOptionalMoversParams) Write(writer io.Writer, value *MoversParams) {
	if value == nil {
		writeInt8(writer, 0)
	} else {
		writeInt8(writer, 1)
		FfiConverterMoversParamsINSTANCE.Write(writer, *value)
	}
}

type FfiDestroyerOptionalMoversParams struct{}

func (_ FfiDestroyerOptionalMoversParams) Destroy(value *MoversParams) {
	if value != nil {
		FfiDestroyerMoversParams{}.Destroy(*value)
	}
}

type FfiConverterOptionalOddLotParams struct{}

var FfiConverterOptionalOddLotParamsINSTANCE = FfiConverterOptionalOddLotParams{}

func (c FfiConverterOptionalOddLotParams) Lift(rb RustBufferI) *OddLotParams {
	return LiftFromRustBuffer[*OddLotParams](c, rb)
}

func (_ FfiConverterOptionalOddLotParams) Read(reader io.Reader) *OddLotParams {
	if readInt8(reader) == 0 {
		return nil
	}
	temp := FfiConverterOddLotParamsINSTANCE.Read(reader)
	return &temp
}

func (c FfiConverterOptionalOddLotParams) Lower(value *OddLotParams) C.RustBuffer {
	return LowerIntoRustBuffer[*OddLotParams](c, value)
}

func (c FfiConverterOptionalOddLotParams) LowerExternal(value *OddLotParams) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[*OddLotParams](c, value))
}

func (_ FfiConverterOptionalOddLotParams) Write(writer io.Writer, value *OddLotParams) {
	if value == nil {
		writeInt8(writer, 0)
	} else {
		writeInt8(writer, 1)
		FfiConverterOddLotParamsINSTANCE.Write(writer, *value)
	}
}

type FfiDestroyerOptionalOddLotParams struct{}

func (_ FfiDestroyerOptionalOddLotParams) Destroy(value *OddLotParams) {
	if value != nil {
		FfiDestroyerOddLotParams{}.Destroy(*value)
	}
}

type FfiConverterOptionalOwnershipParams struct{}

var FfiConverterOptionalOwnershipParamsINSTANCE = FfiConverterOptionalOwnershipParams{}

func (c FfiConverterOptionalOwnershipParams) Lift(rb RustBufferI) *OwnershipParams {
	return LiftFromRustBuffer[*OwnershipParams](c, rb)
}

func (_ FfiConverterOptionalOwnershipParams) Read(reader io.Reader) *OwnershipParams {
	if readInt8(reader) == 0 {
		return nil
	}
	temp := FfiConverterOwnershipParamsINSTANCE.Read(reader)
	return &temp
}

func (c FfiConverterOptionalOwnershipParams) Lower(value *OwnershipParams) C.RustBuffer {
	return LowerIntoRustBuffer[*OwnershipParams](c, value)
}

func (c FfiConverterOptionalOwnershipParams) LowerExternal(value *OwnershipParams) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[*OwnershipParams](c, value))
}

func (_ FfiConverterOptionalOwnershipParams) Write(writer io.Writer, value *OwnershipParams) {
	if value == nil {
		writeInt8(writer, 0)
	} else {
		writeInt8(writer, 1)
		FfiConverterOwnershipParamsINSTANCE.Write(writer, *value)
	}
}

type FfiDestroyerOptionalOwnershipParams struct{}

func (_ FfiDestroyerOptionalOwnershipParams) Destroy(value *OwnershipParams) {
	if value != nil {
		FfiDestroyerOwnershipParams{}.Destroy(*value)
	}
}

type FfiConverterOptionalReconnectConfigRecord struct{}

var FfiConverterOptionalReconnectConfigRecordINSTANCE = FfiConverterOptionalReconnectConfigRecord{}

func (c FfiConverterOptionalReconnectConfigRecord) Lift(rb RustBufferI) *ReconnectConfigRecord {
	return LiftFromRustBuffer[*ReconnectConfigRecord](c, rb)
}

func (_ FfiConverterOptionalReconnectConfigRecord) Read(reader io.Reader) *ReconnectConfigRecord {
	if readInt8(reader) == 0 {
		return nil
	}
	temp := FfiConverterReconnectConfigRecordINSTANCE.Read(reader)
	return &temp
}

func (c FfiConverterOptionalReconnectConfigRecord) Lower(value *ReconnectConfigRecord) C.RustBuffer {
	return LowerIntoRustBuffer[*ReconnectConfigRecord](c, value)
}

func (c FfiConverterOptionalReconnectConfigRecord) LowerExternal(value *ReconnectConfigRecord) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[*ReconnectConfigRecord](c, value))
}

func (_ FfiConverterOptionalReconnectConfigRecord) Write(writer io.Writer, value *ReconnectConfigRecord) {
	if value == nil {
		writeInt8(writer, 0)
	} else {
		writeInt8(writer, 1)
		FfiConverterReconnectConfigRecordINSTANCE.Write(writer, *value)
	}
}

type FfiDestroyerOptionalReconnectConfigRecord struct{}

func (_ FfiDestroyerOptionalReconnectConfigRecord) Destroy(value *ReconnectConfigRecord) {
	if value != nil {
		FfiDestroyerReconnectConfigRecord{}.Destroy(*value)
	}
}

type FfiConverterOptionalSnapshotParams struct{}

var FfiConverterOptionalSnapshotParamsINSTANCE = FfiConverterOptionalSnapshotParams{}

func (c FfiConverterOptionalSnapshotParams) Lift(rb RustBufferI) *SnapshotParams {
	return LiftFromRustBuffer[*SnapshotParams](c, rb)
}

func (_ FfiConverterOptionalSnapshotParams) Read(reader io.Reader) *SnapshotParams {
	if readInt8(reader) == 0 {
		return nil
	}
	temp := FfiConverterSnapshotParamsINSTANCE.Read(reader)
	return &temp
}

func (c FfiConverterOptionalSnapshotParams) Lower(value *SnapshotParams) C.RustBuffer {
	return LowerIntoRustBuffer[*SnapshotParams](c, value)
}

func (c FfiConverterOptionalSnapshotParams) LowerExternal(value *SnapshotParams) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[*SnapshotParams](c, value))
}

func (_ FfiConverterOptionalSnapshotParams) Write(writer io.Writer, value *SnapshotParams) {
	if value == nil {
		writeInt8(writer, 0)
	} else {
		writeInt8(writer, 1)
		FfiConverterSnapshotParamsINSTANCE.Write(writer, *value)
	}
}

type FfiDestroyerOptionalSnapshotParams struct{}

func (_ FfiDestroyerOptionalSnapshotParams) Destroy(value *SnapshotParams) {
	if value != nil {
		FfiDestroyerSnapshotParams{}.Destroy(*value)
	}
}

type FfiConverterOptionalStockCandlesParams struct{}

var FfiConverterOptionalStockCandlesParamsINSTANCE = FfiConverterOptionalStockCandlesParams{}

func (c FfiConverterOptionalStockCandlesParams) Lift(rb RustBufferI) *StockCandlesParams {
	return LiftFromRustBuffer[*StockCandlesParams](c, rb)
}

func (_ FfiConverterOptionalStockCandlesParams) Read(reader io.Reader) *StockCandlesParams {
	if readInt8(reader) == 0 {
		return nil
	}
	temp := FfiConverterStockCandlesParamsINSTANCE.Read(reader)
	return &temp
}

func (c FfiConverterOptionalStockCandlesParams) Lower(value *StockCandlesParams) C.RustBuffer {
	return LowerIntoRustBuffer[*StockCandlesParams](c, value)
}

func (c FfiConverterOptionalStockCandlesParams) LowerExternal(value *StockCandlesParams) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[*StockCandlesParams](c, value))
}

func (_ FfiConverterOptionalStockCandlesParams) Write(writer io.Writer, value *StockCandlesParams) {
	if value == nil {
		writeInt8(writer, 0)
	} else {
		writeInt8(writer, 1)
		FfiConverterStockCandlesParamsINSTANCE.Write(writer, *value)
	}
}

type FfiDestroyerOptionalStockCandlesParams struct{}

func (_ FfiDestroyerOptionalStockCandlesParams) Destroy(value *StockCandlesParams) {
	if value != nil {
		FfiDestroyerStockCandlesParams{}.Destroy(*value)
	}
}

type FfiConverterOptionalStockHistoricalCandlesParams struct{}

var FfiConverterOptionalStockHistoricalCandlesParamsINSTANCE = FfiConverterOptionalStockHistoricalCandlesParams{}

func (c FfiConverterOptionalStockHistoricalCandlesParams) Lift(rb RustBufferI) *StockHistoricalCandlesParams {
	return LiftFromRustBuffer[*StockHistoricalCandlesParams](c, rb)
}

func (_ FfiConverterOptionalStockHistoricalCandlesParams) Read(reader io.Reader) *StockHistoricalCandlesParams {
	if readInt8(reader) == 0 {
		return nil
	}
	temp := FfiConverterStockHistoricalCandlesParamsINSTANCE.Read(reader)
	return &temp
}

func (c FfiConverterOptionalStockHistoricalCandlesParams) Lower(value *StockHistoricalCandlesParams) C.RustBuffer {
	return LowerIntoRustBuffer[*StockHistoricalCandlesParams](c, value)
}

func (c FfiConverterOptionalStockHistoricalCandlesParams) LowerExternal(value *StockHistoricalCandlesParams) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[*StockHistoricalCandlesParams](c, value))
}

func (_ FfiConverterOptionalStockHistoricalCandlesParams) Write(writer io.Writer, value *StockHistoricalCandlesParams) {
	if value == nil {
		writeInt8(writer, 0)
	} else {
		writeInt8(writer, 1)
		FfiConverterStockHistoricalCandlesParamsINSTANCE.Write(writer, *value)
	}
}

type FfiDestroyerOptionalStockHistoricalCandlesParams struct{}

func (_ FfiDestroyerOptionalStockHistoricalCandlesParams) Destroy(value *StockHistoricalCandlesParams) {
	if value != nil {
		FfiDestroyerStockHistoricalCandlesParams{}.Destroy(*value)
	}
}

type FfiConverterOptionalStockTickersParams struct{}

var FfiConverterOptionalStockTickersParamsINSTANCE = FfiConverterOptionalStockTickersParams{}

func (c FfiConverterOptionalStockTickersParams) Lift(rb RustBufferI) *StockTickersParams {
	return LiftFromRustBuffer[*StockTickersParams](c, rb)
}

func (_ FfiConverterOptionalStockTickersParams) Read(reader io.Reader) *StockTickersParams {
	if readInt8(reader) == 0 {
		return nil
	}
	temp := FfiConverterStockTickersParamsINSTANCE.Read(reader)
	return &temp
}

func (c FfiConverterOptionalStockTickersParams) Lower(value *StockTickersParams) C.RustBuffer {
	return LowerIntoRustBuffer[*StockTickersParams](c, value)
}

func (c FfiConverterOptionalStockTickersParams) LowerExternal(value *StockTickersParams) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[*StockTickersParams](c, value))
}

func (_ FfiConverterOptionalStockTickersParams) Write(writer io.Writer, value *StockTickersParams) {
	if value == nil {
		writeInt8(writer, 0)
	} else {
		writeInt8(writer, 1)
		FfiConverterStockTickersParamsINSTANCE.Write(writer, *value)
	}
}

type FfiDestroyerOptionalStockTickersParams struct{}

func (_ FfiDestroyerOptionalStockTickersParams) Destroy(value *StockTickersParams) {
	if value != nil {
		FfiDestroyerStockTickersParams{}.Destroy(*value)
	}
}

type FfiConverterOptionalStockTradesParams struct{}

var FfiConverterOptionalStockTradesParamsINSTANCE = FfiConverterOptionalStockTradesParams{}

func (c FfiConverterOptionalStockTradesParams) Lift(rb RustBufferI) *StockTradesParams {
	return LiftFromRustBuffer[*StockTradesParams](c, rb)
}

func (_ FfiConverterOptionalStockTradesParams) Read(reader io.Reader) *StockTradesParams {
	if readInt8(reader) == 0 {
		return nil
	}
	temp := FfiConverterStockTradesParamsINSTANCE.Read(reader)
	return &temp
}

func (c FfiConverterOptionalStockTradesParams) Lower(value *StockTradesParams) C.RustBuffer {
	return LowerIntoRustBuffer[*StockTradesParams](c, value)
}

func (c FfiConverterOptionalStockTradesParams) LowerExternal(value *StockTradesParams) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[*StockTradesParams](c, value))
}

func (_ FfiConverterOptionalStockTradesParams) Write(writer io.Writer, value *StockTradesParams) {
	if value == nil {
		writeInt8(writer, 0)
	} else {
		writeInt8(writer, 1)
		FfiConverterStockTradesParamsINSTANCE.Write(writer, *value)
	}
}

type FfiDestroyerOptionalStockTradesParams struct{}

func (_ FfiDestroyerOptionalStockTradesParams) Destroy(value *StockTradesParams) {
	if value != nil {
		FfiDestroyerStockTradesParams{}.Destroy(*value)
	}
}

type FfiConverterOptionalStreamingVersionRecord struct{}

var FfiConverterOptionalStreamingVersionRecordINSTANCE = FfiConverterOptionalStreamingVersionRecord{}

func (c FfiConverterOptionalStreamingVersionRecord) Lift(rb RustBufferI) *StreamingVersionRecord {
	return LiftFromRustBuffer[*StreamingVersionRecord](c, rb)
}

func (_ FfiConverterOptionalStreamingVersionRecord) Read(reader io.Reader) *StreamingVersionRecord {
	if readInt8(reader) == 0 {
		return nil
	}
	temp := FfiConverterStreamingVersionRecordINSTANCE.Read(reader)
	return &temp
}

func (c FfiConverterOptionalStreamingVersionRecord) Lower(value *StreamingVersionRecord) C.RustBuffer {
	return LowerIntoRustBuffer[*StreamingVersionRecord](c, value)
}

func (c FfiConverterOptionalStreamingVersionRecord) LowerExternal(value *StreamingVersionRecord) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[*StreamingVersionRecord](c, value))
}

func (_ FfiConverterOptionalStreamingVersionRecord) Write(writer io.Writer, value *StreamingVersionRecord) {
	if value == nil {
		writeInt8(writer, 0)
	} else {
		writeInt8(writer, 1)
		FfiConverterStreamingVersionRecordINSTANCE.Write(writer, *value)
	}
}

type FfiDestroyerOptionalStreamingVersionRecord struct{}

func (_ FfiDestroyerOptionalStreamingVersionRecord) Destroy(value *StreamingVersionRecord) {
	if value != nil {
		FfiDestroyerStreamingVersionRecord{}.Destroy(*value)
	}
}

type FfiConverterOptionalSubscribeOptions struct{}

var FfiConverterOptionalSubscribeOptionsINSTANCE = FfiConverterOptionalSubscribeOptions{}

func (c FfiConverterOptionalSubscribeOptions) Lift(rb RustBufferI) *SubscribeOptions {
	return LiftFromRustBuffer[*SubscribeOptions](c, rb)
}

func (_ FfiConverterOptionalSubscribeOptions) Read(reader io.Reader) *SubscribeOptions {
	if readInt8(reader) == 0 {
		return nil
	}
	temp := FfiConverterSubscribeOptionsINSTANCE.Read(reader)
	return &temp
}

func (c FfiConverterOptionalSubscribeOptions) Lower(value *SubscribeOptions) C.RustBuffer {
	return LowerIntoRustBuffer[*SubscribeOptions](c, value)
}

func (c FfiConverterOptionalSubscribeOptions) LowerExternal(value *SubscribeOptions) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[*SubscribeOptions](c, value))
}

func (_ FfiConverterOptionalSubscribeOptions) Write(writer io.Writer, value *SubscribeOptions) {
	if value == nil {
		writeInt8(writer, 0)
	} else {
		writeInt8(writer, 1)
		FfiConverterSubscribeOptionsINSTANCE.Write(writer, *value)
	}
}

type FfiDestroyerOptionalSubscribeOptions struct{}

func (_ FfiDestroyerOptionalSubscribeOptions) Destroy(value *SubscribeOptions) {
	if value != nil {
		FfiDestroyerSubscribeOptions{}.Destroy(*value)
	}
}

type FfiConverterOptionalTechnicalParams struct{}

var FfiConverterOptionalTechnicalParamsINSTANCE = FfiConverterOptionalTechnicalParams{}

func (c FfiConverterOptionalTechnicalParams) Lift(rb RustBufferI) *TechnicalParams {
	return LiftFromRustBuffer[*TechnicalParams](c, rb)
}

func (_ FfiConverterOptionalTechnicalParams) Read(reader io.Reader) *TechnicalParams {
	if readInt8(reader) == 0 {
		return nil
	}
	temp := FfiConverterTechnicalParamsINSTANCE.Read(reader)
	return &temp
}

func (c FfiConverterOptionalTechnicalParams) Lower(value *TechnicalParams) C.RustBuffer {
	return LowerIntoRustBuffer[*TechnicalParams](c, value)
}

func (c FfiConverterOptionalTechnicalParams) LowerExternal(value *TechnicalParams) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[*TechnicalParams](c, value))
}

func (_ FfiConverterOptionalTechnicalParams) Write(writer io.Writer, value *TechnicalParams) {
	if value == nil {
		writeInt8(writer, 0)
	} else {
		writeInt8(writer, 1)
		FfiConverterTechnicalParamsINSTANCE.Write(writer, *value)
	}
}

type FfiDestroyerOptionalTechnicalParams struct{}

func (_ FfiDestroyerOptionalTechnicalParams) Destroy(value *TechnicalParams) {
	if value != nil {
		FfiDestroyerTechnicalParams{}.Destroy(*value)
	}
}

type FfiConverterOptionalTlsConfigRecord struct{}

var FfiConverterOptionalTlsConfigRecordINSTANCE = FfiConverterOptionalTlsConfigRecord{}

func (c FfiConverterOptionalTlsConfigRecord) Lift(rb RustBufferI) *TlsConfigRecord {
	return LiftFromRustBuffer[*TlsConfigRecord](c, rb)
}

func (_ FfiConverterOptionalTlsConfigRecord) Read(reader io.Reader) *TlsConfigRecord {
	if readInt8(reader) == 0 {
		return nil
	}
	temp := FfiConverterTlsConfigRecordINSTANCE.Read(reader)
	return &temp
}

func (c FfiConverterOptionalTlsConfigRecord) Lower(value *TlsConfigRecord) C.RustBuffer {
	return LowerIntoRustBuffer[*TlsConfigRecord](c, value)
}

func (c FfiConverterOptionalTlsConfigRecord) LowerExternal(value *TlsConfigRecord) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[*TlsConfigRecord](c, value))
}

func (_ FfiConverterOptionalTlsConfigRecord) Write(writer io.Writer, value *TlsConfigRecord) {
	if value == nil {
		writeInt8(writer, 0)
	} else {
		writeInt8(writer, 1)
		FfiConverterTlsConfigRecordINSTANCE.Write(writer, *value)
	}
}

type FfiDestroyerOptionalTlsConfigRecord struct{}

func (_ FfiDestroyerOptionalTlsConfigRecord) Destroy(value *TlsConfigRecord) {
	if value != nil {
		FfiDestroyerTlsConfigRecord{}.Destroy(*value)
	}
}

type FfiConverterSequenceString struct{}

var FfiConverterSequenceStringINSTANCE = FfiConverterSequenceString{}

func (c FfiConverterSequenceString) Lift(rb RustBufferI) []string {
	return LiftFromRustBuffer[[]string](c, rb)
}

func (c FfiConverterSequenceString) Read(reader io.Reader) []string {
	length := readInt32(reader)
	if length == 0 {
		return nil
	}
	result := make([]string, 0, length)
	for i := int32(0); i < length; i++ {
		result = append(result, FfiConverterStringINSTANCE.Read(reader))
	}
	return result
}

func (c FfiConverterSequenceString) Lower(value []string) C.RustBuffer {
	return LowerIntoRustBuffer[[]string](c, value)
}

func (c FfiConverterSequenceString) LowerExternal(value []string) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[[]string](c, value))
}

func (c FfiConverterSequenceString) Write(writer io.Writer, value []string) {
	if len(value) > math.MaxInt32 {
		panic("[]string is too large to fit into Int32")
	}

	writeInt32(writer, int32(len(value)))
	for _, item := range value {
		FfiConverterStringINSTANCE.Write(writer, item)
	}
}

type FfiDestroyerSequenceString struct{}

func (FfiDestroyerSequenceString) Destroy(sequence []string) {
	for _, value := range sequence {
		FfiDestroyerString{}.Destroy(value)
	}
}

type FfiConverterMapStringString struct{}

var FfiConverterMapStringStringINSTANCE = FfiConverterMapStringString{}

func (c FfiConverterMapStringString) Lift(rb RustBufferI) map[string]string {
	return LiftFromRustBuffer[map[string]string](c, rb)
}

func (_ FfiConverterMapStringString) Read(reader io.Reader) map[string]string {
	result := make(map[string]string)
	length := readInt32(reader)
	for i := int32(0); i < length; i++ {
		key := FfiConverterStringINSTANCE.Read(reader)
		value := FfiConverterStringINSTANCE.Read(reader)
		result[key] = value
	}
	return result
}

func (c FfiConverterMapStringString) Lower(value map[string]string) C.RustBuffer {
	return LowerIntoRustBuffer[map[string]string](c, value)
}

func (c FfiConverterMapStringString) LowerExternal(value map[string]string) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[map[string]string](c, value))
}

func (_ FfiConverterMapStringString) Write(writer io.Writer, mapValue map[string]string) {
	if len(mapValue) > math.MaxInt32 {
		panic("map[string]string is too large to fit into Int32")
	}

	writeInt32(writer, int32(len(mapValue)))
	for key, value := range mapValue {
		FfiConverterStringINSTANCE.Write(writer, key)
		FfiConverterStringINSTANCE.Write(writer, value)
	}
}

type FfiDestroyerMapStringString struct{}

func (_ FfiDestroyerMapStringString) Destroy(mapValue map[string]string) {
	for key, value := range mapValue {
		FfiDestroyerString{}.Destroy(key)
		FfiDestroyerString{}.Destroy(value)
	}
}

const (
	uniffiRustFuturePollReady      int8 = 0
	uniffiRustFuturePollMaybeReady int8 = 1
)

type rustFuturePollFunc func(C.uint64_t, C.UniffiRustFutureContinuationCallback, C.uint64_t)
type rustFutureCompleteFunc[T any] func(C.uint64_t, *C.RustCallStatus) T
type rustFutureFreeFunc func(C.uint64_t)

//export marketdata_uniffi_uniffiFutureContinuationCallback
func marketdata_uniffi_uniffiFutureContinuationCallback(data C.uint64_t, pollResult C.int8_t) {
	h := cgo.Handle(uintptr(data))
	waiter := h.Value().(chan int8)
	waiter <- int8(pollResult)
}

func uniffiRustCallAsync[E any, T any, F any](
	errConverter BufReader[*E],
	completeFunc rustFutureCompleteFunc[F],
	liftFunc func(F) T,
	rustFuture C.uint64_t,
	pollFunc rustFuturePollFunc,
	freeFunc rustFutureFreeFunc,
) (T, *E) {
	defer freeFunc(rustFuture)

	pollResult := int8(-1)
	waiter := make(chan int8, 1)

	chanHandle := cgo.NewHandle(waiter)
	defer chanHandle.Delete()

	for pollResult != uniffiRustFuturePollReady {
		pollFunc(
			rustFuture,
			(C.UniffiRustFutureContinuationCallback)(C.marketdata_uniffi_uniffiFutureContinuationCallback),
			C.uint64_t(chanHandle),
		)
		pollResult = <-waiter
	}

	var goValue T
	var ffiValue F
	var err *E

	ffiValue, err = rustCallWithError(errConverter, func(status *C.RustCallStatus) F {
		return completeFunc(rustFuture, status)
	})
	if err != nil {
		return goValue, err
	}
	return liftFunc(ffiValue), nil
}

//export marketdata_uniffi_uniffiFreeGorutine
func marketdata_uniffi_uniffiFreeGorutine(data C.uint64_t) {
	handle := cgo.Handle(uintptr(data))
	defer handle.Delete()

	guard := handle.Value().(chan struct{})
	guard <- struct{}{}
}

// Create a REST client with API key authentication
//
// # Arguments
// * `api_key` - The Fugle API key
//
// # Returns
// A RestClient instance wrapped in Arc for thread-safe access
func NewRestClientWithApiKey(apiKey string) (*RestClient, error) {
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) unsafe.Pointer {
		return C.uniffi_marketdata_uniffi_fn_func_new_rest_client_with_api_key(FfiConverterStringINSTANCE.Lower(apiKey), _uniffiStatus)
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue *RestClient
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterRestClientINSTANCE.Lift(_uniffiRV), nil
	}
}

// Create a REST client with API key authentication, custom base URL, and TLS config
func NewRestClientWithApiKeyAndTls(apiKey string, baseUrl *string, tls TlsConfigRecord) (*RestClient, error) {
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) unsafe.Pointer {
		return C.uniffi_marketdata_uniffi_fn_func_new_rest_client_with_api_key_and_tls(FfiConverterStringINSTANCE.Lower(apiKey), FfiConverterOptionalStringINSTANCE.Lower(baseUrl), FfiConverterTlsConfigRecordINSTANCE.Lower(tls), _uniffiStatus)
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue *RestClient
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterRestClientINSTANCE.Lift(_uniffiRV), nil
	}
}

// Create a REST client with bearer token authentication
//
// # Arguments
// * `bearer_token` - OAuth bearer token
//
// # Returns
// A RestClient instance wrapped in Arc for thread-safe access
func NewRestClientWithBearerToken(bearerToken string) (*RestClient, error) {
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) unsafe.Pointer {
		return C.uniffi_marketdata_uniffi_fn_func_new_rest_client_with_bearer_token(FfiConverterStringINSTANCE.Lower(bearerToken), _uniffiStatus)
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue *RestClient
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterRestClientINSTANCE.Lift(_uniffiRV), nil
	}
}

// Create a REST client with bearer token authentication, custom base URL, and TLS config
func NewRestClientWithBearerTokenAndTls(bearerToken string, baseUrl *string, tls TlsConfigRecord) (*RestClient, error) {
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) unsafe.Pointer {
		return C.uniffi_marketdata_uniffi_fn_func_new_rest_client_with_bearer_token_and_tls(FfiConverterStringINSTANCE.Lower(bearerToken), FfiConverterOptionalStringINSTANCE.Lower(baseUrl), FfiConverterTlsConfigRecordINSTANCE.Lower(tls), _uniffiStatus)
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue *RestClient
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterRestClientINSTANCE.Lift(_uniffiRV), nil
	}
}

// Create a REST client with SDK token authentication
//
// # Arguments
// * `sdk_token` - Fugle SDK token
//
// # Returns
// A RestClient instance wrapped in Arc for thread-safe access
func NewRestClientWithSdkToken(sdkToken string) (*RestClient, error) {
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) unsafe.Pointer {
		return C.uniffi_marketdata_uniffi_fn_func_new_rest_client_with_sdk_token(FfiConverterStringINSTANCE.Lower(sdkToken), _uniffiStatus)
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue *RestClient
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterRestClientINSTANCE.Lift(_uniffiRV), nil
	}
}

// Create a REST client with SDK token authentication, custom base URL, and TLS config
func NewRestClientWithSdkTokenAndTls(sdkToken string, baseUrl *string, tls TlsConfigRecord) (*RestClient, error) {
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) unsafe.Pointer {
		return C.uniffi_marketdata_uniffi_fn_func_new_rest_client_with_sdk_token_and_tls(FfiConverterStringINSTANCE.Lower(sdkToken), FfiConverterOptionalStringINSTANCE.Lower(baseUrl), FfiConverterTlsConfigRecordINSTANCE.Lower(tls), _uniffiStatus)
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue *RestClient
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterRestClientINSTANCE.Lift(_uniffiRV), nil
	}
}

// Create a new WebSocket client for stock market data
//
// # Arguments
// * `api_key` - Fugle API key for authentication
// * `listener` - Callback interface for receiving WebSocket events
//
// # Returns
// A WebSocketClient instance wrapped in Arc for thread-safe access
func NewWebsocketClient(apiKey string, listener WebSocketListener) *WebSocketClient {
	return FfiConverterWebSocketClientINSTANCE.Lift(rustCall(func(_uniffiStatus *C.RustCallStatus) unsafe.Pointer {
		return C.uniffi_marketdata_uniffi_fn_func_new_websocket_client(FfiConverterStringINSTANCE.Lower(apiKey), FfiConverterWebSocketListenerINSTANCE.Lower(listener), _uniffiStatus)
	}))
}

// Create a new WebSocket client with full configuration
//
// # Arguments
// * `api_key` - Fugle API key for authentication
// * `listener` - Callback interface for receiving WebSocket events
// * `endpoint` - The market data endpoint (Stock or FutOpt)
// * `reconnect_config` - Optional reconnection configuration
// * `health_check_config` - Optional health check configuration
//
// # Returns
// A WebSocketClient instance wrapped in Arc for thread-safe access
func NewWebsocketClientWithConfig(apiKey string, listener WebSocketListener, endpoint WebSocketEndpoint, reconnectConfig *ReconnectConfigRecord, healthCheckConfig *HealthCheckConfigRecord) *WebSocketClient {
	return FfiConverterWebSocketClientINSTANCE.Lift(rustCall(func(_uniffiStatus *C.RustCallStatus) unsafe.Pointer {
		return C.uniffi_marketdata_uniffi_fn_func_new_websocket_client_with_config(FfiConverterStringINSTANCE.Lower(apiKey), FfiConverterWebSocketListenerINSTANCE.Lower(listener), FfiConverterWebSocketEndpointINSTANCE.Lower(endpoint), FfiConverterOptionalReconnectConfigRecordINSTANCE.Lower(reconnectConfig), FfiConverterOptionalHealthCheckConfigRecordINSTANCE.Lower(healthCheckConfig), _uniffiStatus)
	}))
}

// Create a new WebSocket client for a specific endpoint
//
// # Arguments
// * `api_key` - Fugle API key for authentication
// * `listener` - Callback interface for receiving WebSocket events
// * `endpoint` - The market data endpoint (Stock or FutOpt)
//
// # Returns
// A WebSocketClient instance wrapped in Arc for thread-safe access
func NewWebsocketClientWithEndpoint(apiKey string, listener WebSocketListener, endpoint WebSocketEndpoint) *WebSocketClient {
	return FfiConverterWebSocketClientINSTANCE.Lift(rustCall(func(_uniffiStatus *C.RustCallStatus) unsafe.Pointer {
		return C.uniffi_marketdata_uniffi_fn_func_new_websocket_client_with_endpoint(FfiConverterStringINSTANCE.Lower(apiKey), FfiConverterWebSocketListenerINSTANCE.Lower(listener), FfiConverterWebSocketEndpointINSTANCE.Lower(endpoint), _uniffiStatus)
	}))
}

// Check a set of credentials the way every client constructor does.
//
// A value that is empty or only whitespace counts as not provided; exactly
// one of the three must remain. Wrappers that accept all three options call
// this and pass the value of the returned kind to the matching constructor,
// so the rule and the error (a `ConfigError`, code 1004) come from the core.
func ValidateCredentials(apiKey *string, bearerToken *string, sdkToken *string) (CredentialKind, error) {
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_func_validate_credentials(FfiConverterOptionalStringINSTANCE.Lower(apiKey), FfiConverterOptionalStringINSTANCE.Lower(bearerToken), FfiConverterOptionalStringINSTANCE.Lower(sdkToken), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue CredentialKind
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterCredentialKindINSTANCE.Lift(_uniffiRV), nil
	}
}
