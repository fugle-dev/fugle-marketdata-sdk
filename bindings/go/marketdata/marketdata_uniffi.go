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

func RustBufferFromExternal(b RustBufferI) GoRustBuffer {
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
	bindingsContractVersion := 26
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
		if checksum != 56503 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_futopthistoricalclient_candles_sync: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_futopthistoricalclient_daily_sync()
		})
		if checksum != 10493 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_futopthistoricalclient_daily_sync: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_futopthistoricalclient_get_candles()
		})
		if checksum != 14488 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_futopthistoricalclient_get_candles: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_futopthistoricalclient_get_daily()
		})
		if checksum != 41351 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_futopthistoricalclient_get_daily: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_candles_sync()
		})
		if checksum != 60136 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_candles_sync: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_get_candles()
		})
		if checksum != 62970 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_get_candles: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_get_products()
		})
		if checksum != 61510 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_get_products: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_get_quote()
		})
		if checksum != 21333 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_get_quote: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_get_ticker()
		})
		if checksum != 30953 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_get_ticker: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_get_tickers()
		})
		if checksum != 29913 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_get_tickers: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_get_trades()
		})
		if checksum != 3561 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_get_trades: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_get_volumes()
		})
		if checksum != 5062 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_get_volumes: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_products_sync()
		})
		if checksum != 8976 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_products_sync: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_quote_sync()
		})
		if checksum != 33593 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_quote_sync: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_ticker_sync()
		})
		if checksum != 53319 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_ticker_sync: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_tickers_sync()
		})
		if checksum != 55649 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_tickers_sync: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_trades_sync()
		})
		if checksum != 987 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_trades_sync: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_volumes_sync()
		})
		if checksum != 25987 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_volumes_sync: UniFFI API checksum mismatch")
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
		if checksum != 38225 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stockcorporateactionsclient_capital_changes_sync: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stockcorporateactionsclient_dividends_sync()
		})
		if checksum != 26469 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stockcorporateactionsclient_dividends_sync: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stockcorporateactionsclient_get_capital_changes()
		})
		if checksum != 34953 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stockcorporateactionsclient_get_capital_changes: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stockcorporateactionsclient_get_dividends()
		})
		if checksum != 30186 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stockcorporateactionsclient_get_dividends: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stockcorporateactionsclient_get_listing_applicants()
		})
		if checksum != 41091 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stockcorporateactionsclient_get_listing_applicants: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stockcorporateactionsclient_listing_applicants_sync()
		})
		if checksum != 19487 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stockcorporateactionsclient_listing_applicants_sync: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stockhistoricalclient_candles_sync()
		})
		if checksum != 58660 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stockhistoricalclient_candles_sync: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stockhistoricalclient_get_candles()
		})
		if checksum != 18842 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stockhistoricalclient_get_candles: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stockhistoricalclient_get_stats()
		})
		if checksum != 19930 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stockhistoricalclient_get_stats: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stockhistoricalclient_stats_sync()
		})
		if checksum != 25283 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stockhistoricalclient_stats_sync: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stockintradayclient_candles_sync()
		})
		if checksum != 10535 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stockintradayclient_candles_sync: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stockintradayclient_get_candles()
		})
		if checksum != 20034 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stockintradayclient_get_candles: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stockintradayclient_get_quote()
		})
		if checksum != 64785 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stockintradayclient_get_quote: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stockintradayclient_get_ticker()
		})
		if checksum != 26620 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stockintradayclient_get_ticker: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stockintradayclient_get_tickers()
		})
		if checksum != 24176 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stockintradayclient_get_tickers: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stockintradayclient_get_trades()
		})
		if checksum != 48306 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stockintradayclient_get_trades: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stockintradayclient_get_volumes()
		})
		if checksum != 41478 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stockintradayclient_get_volumes: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stockintradayclient_quote_sync()
		})
		if checksum != 24390 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stockintradayclient_quote_sync: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stockintradayclient_ticker_sync()
		})
		if checksum != 22635 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stockintradayclient_ticker_sync: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stockintradayclient_tickers_sync()
		})
		if checksum != 3916 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stockintradayclient_tickers_sync: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stockintradayclient_trades_sync()
		})
		if checksum != 4040 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stockintradayclient_trades_sync: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stockintradayclient_volumes_sync()
		})
		if checksum != 8850 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stockintradayclient_volumes_sync: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stocksnapshotclient_actives_sync()
		})
		if checksum != 45448 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stocksnapshotclient_actives_sync: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stocksnapshotclient_get_actives()
		})
		if checksum != 31681 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stocksnapshotclient_get_actives: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stocksnapshotclient_get_movers()
		})
		if checksum != 54795 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stocksnapshotclient_get_movers: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stocksnapshotclient_get_quotes()
		})
		if checksum != 5150 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stocksnapshotclient_get_quotes: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stocksnapshotclient_movers_sync()
		})
		if checksum != 38625 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stocksnapshotclient_movers_sync: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stocksnapshotclient_quotes_sync()
		})
		if checksum != 7562 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stocksnapshotclient_quotes_sync: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_bb_sync()
		})
		if checksum != 50012 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_bb_sync: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_get_bb()
		})
		if checksum != 28523 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_get_bb: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_get_kdj()
		})
		if checksum != 47666 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_get_kdj: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_get_macd()
		})
		if checksum != 17293 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_get_macd: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_get_rsi()
		})
		if checksum != 23780 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_get_rsi: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_get_sma()
		})
		if checksum != 37856 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_get_sma: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_kdj_sync()
		})
		if checksum != 58302 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_kdj_sync: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_macd_sync()
		})
		if checksum != 32247 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_macd_sync: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_rsi_sync()
		})
		if checksum != 6527 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_rsi_sync: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_sma_sync()
		})
		if checksum != 18246 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_sma_sync: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_websocketclient_connect()
		})
		if checksum != 34522 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_websocketclient_connect: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_websocketclient_disconnect()
		})
		if checksum != 57258 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_websocketclient_disconnect: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_websocketclient_is_closed()
		})
		if checksum != 15116 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_websocketclient_is_closed: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_websocketclient_is_connected()
		})
		if checksum != 53625 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_websocketclient_is_connected: UniFFI API checksum mismatch")
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
		if checksum != 39559 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_websocketclient_subscribe: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_websocketclient_unsubscribe()
		})
		if checksum != 21735 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_websocketclient_unsubscribe: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_websocketlistener_on_connected()
		})
		if checksum != 56842 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_websocketlistener_on_connected: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_websocketlistener_on_disconnected()
		})
		if checksum != 54477 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_websocketlistener_on_disconnected: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_websocketlistener_on_message()
		})
		if checksum != 54327 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_websocketlistener_on_message: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_websocketlistener_on_error()
		})
		if checksum != 64085 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_websocketlistener_on_error: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_websocketlistener_on_reconnecting()
		})
		if checksum != 63646 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_websocketlistener_on_reconnecting: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_marketdata_uniffi_checksum_method_websocketlistener_on_reconnect_failed()
		})
		if checksum != 36342 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_method_websocketlistener_on_reconnect_failed: UniFFI API checksum mismatch")
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
		if checksum != 39727 {
			// If this happens try cleaning and rebuilding your project
			panic("marketdata_uniffi: uniffi_marketdata_uniffi_checksum_constructor_websocketclient_new_with_full_config: UniFFI API checksum mismatch")
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

type FfiConverterInt64 struct{}

var FfiConverterInt64INSTANCE = FfiConverterInt64{}

func (FfiConverterInt64) Lower(value int64) C.int64_t {
	return C.int64_t(value)
}

func (FfiConverterInt64) Write(writer io.Writer, value int64) {
	writeInt64(writer, value)
}

func (FfiConverterInt64) Lift(value C.int64_t) int64 {
	return int64(value)
}

func (FfiConverterInt64) Read(reader io.Reader) int64 {
	return readInt64(reader)
}

type FfiDestroyerInt64 struct{}

func (FfiDestroyerInt64) Destroy(_ int64) {}

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
	if err != nil {
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
	if err != nil {
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
	// Get historical candles for a contract (sync/blocking)
	CandlesSync(symbol string, from *string, to *string, timeframe *string, afterHours bool) (FutOptHistoricalCandlesResponse, *MarketDataError)
	// Get daily historical data for a contract (sync/blocking)
	DailySync(symbol string, from *string, to *string, afterHours bool) (FutOptDailyResponse, *MarketDataError)
	// Get historical candles for a contract (async)
	GetCandles(symbol string, from *string, to *string, timeframe *string, afterHours bool) (FutOptHistoricalCandlesResponse, *MarketDataError)
	// Get daily historical data for a contract (async)
	GetDaily(symbol string, from *string, to *string, afterHours bool) (FutOptDailyResponse, *MarketDataError)
}

// FutOpt historical data endpoints
//
// Provides access to historical candles and daily data for futures and options.
type FutOptHistoricalClient struct {
	ffiObject FfiObject
}

// Get historical candles for a contract (sync/blocking)
func (_self *FutOptHistoricalClient) CandlesSync(symbol string, from *string, to *string, timeframe *string, afterHours bool) (FutOptHistoricalCandlesResponse, *MarketDataError) {
	_pointer := _self.ffiObject.incrementPointer("*FutOptHistoricalClient")
	defer _self.ffiObject.decrementPointer()
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_method_futopthistoricalclient_candles_sync(
				_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterOptionalStringINSTANCE.Lower(from), FfiConverterOptionalStringINSTANCE.Lower(to), FfiConverterOptionalStringINSTANCE.Lower(timeframe), FfiConverterBoolINSTANCE.Lower(afterHours), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue FutOptHistoricalCandlesResponse
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterFutOptHistoricalCandlesResponseINSTANCE.Lift(_uniffiRV), _uniffiErr
	}
}

// Get daily historical data for a contract (sync/blocking)
func (_self *FutOptHistoricalClient) DailySync(symbol string, from *string, to *string, afterHours bool) (FutOptDailyResponse, *MarketDataError) {
	_pointer := _self.ffiObject.incrementPointer("*FutOptHistoricalClient")
	defer _self.ffiObject.decrementPointer()
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_method_futopthistoricalclient_daily_sync(
				_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterOptionalStringINSTANCE.Lower(from), FfiConverterOptionalStringINSTANCE.Lower(to), FfiConverterBoolINSTANCE.Lower(afterHours), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue FutOptDailyResponse
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterFutOptDailyResponseINSTANCE.Lift(_uniffiRV), _uniffiErr
	}
}

// Get historical candles for a contract (async)
func (_self *FutOptHistoricalClient) GetCandles(symbol string, from *string, to *string, timeframe *string, afterHours bool) (FutOptHistoricalCandlesResponse, *MarketDataError) {
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
		func(ffi RustBufferI) FutOptHistoricalCandlesResponse {
			return FfiConverterFutOptHistoricalCandlesResponseINSTANCE.Lift(ffi)
		},
		C.uniffi_marketdata_uniffi_fn_method_futopthistoricalclient_get_candles(
			_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterOptionalStringINSTANCE.Lower(from), FfiConverterOptionalStringINSTANCE.Lower(to), FfiConverterOptionalStringINSTANCE.Lower(timeframe), FfiConverterBoolINSTANCE.Lower(afterHours)),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_rust_buffer(handle)
		},
	)

	return res, err
}

// Get daily historical data for a contract (async)
func (_self *FutOptHistoricalClient) GetDaily(symbol string, from *string, to *string, afterHours bool) (FutOptDailyResponse, *MarketDataError) {
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
		func(ffi RustBufferI) FutOptDailyResponse {
			return FfiConverterFutOptDailyResponseINSTANCE.Lift(ffi)
		},
		C.uniffi_marketdata_uniffi_fn_method_futopthistoricalclient_get_daily(
			_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterOptionalStringINSTANCE.Lower(from), FfiConverterOptionalStringINSTANCE.Lower(to), FfiConverterBoolINSTANCE.Lower(afterHours)),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_rust_buffer(handle)
		},
	)

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

// FutOpt intraday endpoints with typed model returns
type FutOptIntradayClientInterface interface {
	// Get candlestick data for a contract (sync/blocking)
	CandlesSync(symbol string, timeframe string) (IntradayCandlesResponse, *MarketDataError)
	// Get candlestick data for a futures/options contract (async)
	GetCandles(symbol string, timeframe string) (IntradayCandlesResponse, *MarketDataError)
	// Get available products list (async)
	//
	// typ: "F" for futures, "O" for options
	GetProducts(typ string) (ProductsResponse, *MarketDataError)
	// Get quote for a futures/options contract (async)
	//
	// after_hours: true for after-hours session
	GetQuote(symbol string, afterHours bool) (FutOptQuote, *MarketDataError)
	// Get ticker info for a contract (async)
	GetTicker(symbol string, afterHours bool) (FutOptTicker, *MarketDataError)
	// Get batch tickers for futures/options (async)
	//
	// typ: "F" for futures, "O" for options
	GetTickers(typ string) ([]FutOptTicker, *MarketDataError)
	// Get trade history for a futures/options contract (async)
	GetTrades(symbol string) (TradesResponse, *MarketDataError)
	// Get volume breakdown by price for a futures/options contract (async)
	GetVolumes(symbol string) (VolumesResponse, *MarketDataError)
	// Get available products list (sync/blocking)
	ProductsSync(typ string) (ProductsResponse, *MarketDataError)
	// Get quote for a futures/options contract (sync/blocking)
	QuoteSync(symbol string, afterHours bool) (FutOptQuote, *MarketDataError)
	// Get ticker info for a contract (sync/blocking)
	TickerSync(symbol string, afterHours bool) (FutOptTicker, *MarketDataError)
	// Get batch tickers for futures/options (sync/blocking)
	//
	// typ: "F" for futures, "O" for options
	TickersSync(typ string) ([]FutOptTicker, *MarketDataError)
	// Get trade history for a contract (sync/blocking)
	TradesSync(symbol string) (TradesResponse, *MarketDataError)
	// Get volume breakdown by price for a contract (sync/blocking)
	VolumesSync(symbol string) (VolumesResponse, *MarketDataError)
}

// FutOpt intraday endpoints with typed model returns
type FutOptIntradayClient struct {
	ffiObject FfiObject
}

// Get candlestick data for a contract (sync/blocking)
func (_self *FutOptIntradayClient) CandlesSync(symbol string, timeframe string) (IntradayCandlesResponse, *MarketDataError) {
	_pointer := _self.ffiObject.incrementPointer("*FutOptIntradayClient")
	defer _self.ffiObject.decrementPointer()
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_method_futoptintradayclient_candles_sync(
				_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterStringINSTANCE.Lower(timeframe), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue IntradayCandlesResponse
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterIntradayCandlesResponseINSTANCE.Lift(_uniffiRV), _uniffiErr
	}
}

// Get candlestick data for a futures/options contract (async)
func (_self *FutOptIntradayClient) GetCandles(symbol string, timeframe string) (IntradayCandlesResponse, *MarketDataError) {
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
		func(ffi RustBufferI) IntradayCandlesResponse {
			return FfiConverterIntradayCandlesResponseINSTANCE.Lift(ffi)
		},
		C.uniffi_marketdata_uniffi_fn_method_futoptintradayclient_get_candles(
			_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterStringINSTANCE.Lower(timeframe)),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_rust_buffer(handle)
		},
	)

	return res, err
}

// Get available products list (async)
//
// typ: "F" for futures, "O" for options
func (_self *FutOptIntradayClient) GetProducts(typ string) (ProductsResponse, *MarketDataError) {
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
		func(ffi RustBufferI) ProductsResponse {
			return FfiConverterProductsResponseINSTANCE.Lift(ffi)
		},
		C.uniffi_marketdata_uniffi_fn_method_futoptintradayclient_get_products(
			_pointer, FfiConverterStringINSTANCE.Lower(typ)),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_rust_buffer(handle)
		},
	)

	return res, err
}

// Get quote for a futures/options contract (async)
//
// after_hours: true for after-hours session
func (_self *FutOptIntradayClient) GetQuote(symbol string, afterHours bool) (FutOptQuote, *MarketDataError) {
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
		func(ffi RustBufferI) FutOptQuote {
			return FfiConverterFutOptQuoteINSTANCE.Lift(ffi)
		},
		C.uniffi_marketdata_uniffi_fn_method_futoptintradayclient_get_quote(
			_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterBoolINSTANCE.Lower(afterHours)),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_rust_buffer(handle)
		},
	)

	return res, err
}

// Get ticker info for a contract (async)
func (_self *FutOptIntradayClient) GetTicker(symbol string, afterHours bool) (FutOptTicker, *MarketDataError) {
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
		func(ffi RustBufferI) FutOptTicker {
			return FfiConverterFutOptTickerINSTANCE.Lift(ffi)
		},
		C.uniffi_marketdata_uniffi_fn_method_futoptintradayclient_get_ticker(
			_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterBoolINSTANCE.Lower(afterHours)),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_rust_buffer(handle)
		},
	)

	return res, err
}

// Get batch tickers for futures/options (async)
//
// typ: "F" for futures, "O" for options
func (_self *FutOptIntradayClient) GetTickers(typ string) ([]FutOptTicker, *MarketDataError) {
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
		func(ffi RustBufferI) []FutOptTicker {
			return FfiConverterSequenceFutOptTickerINSTANCE.Lift(ffi)
		},
		C.uniffi_marketdata_uniffi_fn_method_futoptintradayclient_get_tickers(
			_pointer, FfiConverterStringINSTANCE.Lower(typ)),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_rust_buffer(handle)
		},
	)

	return res, err
}

// Get trade history for a futures/options contract (async)
func (_self *FutOptIntradayClient) GetTrades(symbol string) (TradesResponse, *MarketDataError) {
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
		func(ffi RustBufferI) TradesResponse {
			return FfiConverterTradesResponseINSTANCE.Lift(ffi)
		},
		C.uniffi_marketdata_uniffi_fn_method_futoptintradayclient_get_trades(
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

	return res, err
}

// Get volume breakdown by price for a futures/options contract (async)
func (_self *FutOptIntradayClient) GetVolumes(symbol string) (VolumesResponse, *MarketDataError) {
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
		func(ffi RustBufferI) VolumesResponse {
			return FfiConverterVolumesResponseINSTANCE.Lift(ffi)
		},
		C.uniffi_marketdata_uniffi_fn_method_futoptintradayclient_get_volumes(
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

	return res, err
}

// Get available products list (sync/blocking)
func (_self *FutOptIntradayClient) ProductsSync(typ string) (ProductsResponse, *MarketDataError) {
	_pointer := _self.ffiObject.incrementPointer("*FutOptIntradayClient")
	defer _self.ffiObject.decrementPointer()
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_method_futoptintradayclient_products_sync(
				_pointer, FfiConverterStringINSTANCE.Lower(typ), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue ProductsResponse
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterProductsResponseINSTANCE.Lift(_uniffiRV), _uniffiErr
	}
}

// Get quote for a futures/options contract (sync/blocking)
func (_self *FutOptIntradayClient) QuoteSync(symbol string, afterHours bool) (FutOptQuote, *MarketDataError) {
	_pointer := _self.ffiObject.incrementPointer("*FutOptIntradayClient")
	defer _self.ffiObject.decrementPointer()
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_method_futoptintradayclient_quote_sync(
				_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterBoolINSTANCE.Lower(afterHours), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue FutOptQuote
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterFutOptQuoteINSTANCE.Lift(_uniffiRV), _uniffiErr
	}
}

// Get ticker info for a contract (sync/blocking)
func (_self *FutOptIntradayClient) TickerSync(symbol string, afterHours bool) (FutOptTicker, *MarketDataError) {
	_pointer := _self.ffiObject.incrementPointer("*FutOptIntradayClient")
	defer _self.ffiObject.decrementPointer()
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_method_futoptintradayclient_ticker_sync(
				_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterBoolINSTANCE.Lower(afterHours), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue FutOptTicker
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterFutOptTickerINSTANCE.Lift(_uniffiRV), _uniffiErr
	}
}

// Get batch tickers for futures/options (sync/blocking)
//
// typ: "F" for futures, "O" for options
func (_self *FutOptIntradayClient) TickersSync(typ string) ([]FutOptTicker, *MarketDataError) {
	_pointer := _self.ffiObject.incrementPointer("*FutOptIntradayClient")
	defer _self.ffiObject.decrementPointer()
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_method_futoptintradayclient_tickers_sync(
				_pointer, FfiConverterStringINSTANCE.Lower(typ), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue []FutOptTicker
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterSequenceFutOptTickerINSTANCE.Lift(_uniffiRV), _uniffiErr
	}
}

// Get trade history for a contract (sync/blocking)
func (_self *FutOptIntradayClient) TradesSync(symbol string) (TradesResponse, *MarketDataError) {
	_pointer := _self.ffiObject.incrementPointer("*FutOptIntradayClient")
	defer _self.ffiObject.decrementPointer()
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_method_futoptintradayclient_trades_sync(
				_pointer, FfiConverterStringINSTANCE.Lower(symbol), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue TradesResponse
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterTradesResponseINSTANCE.Lift(_uniffiRV), _uniffiErr
	}
}

// Get volume breakdown by price for a contract (sync/blocking)
func (_self *FutOptIntradayClient) VolumesSync(symbol string) (VolumesResponse, *MarketDataError) {
	_pointer := _self.ffiObject.incrementPointer("*FutOptIntradayClient")
	defer _self.ffiObject.decrementPointer()
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_method_futoptintradayclient_volumes_sync(
				_pointer, FfiConverterStringINSTANCE.Lower(symbol), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue VolumesResponse
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterVolumesResponseINSTANCE.Lift(_uniffiRV), _uniffiErr
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
	// Access corporate actions endpoints
	CorporateActions() *StockCorporateActionsClient
	// Access historical data endpoints
	Historical() *StockHistoricalClient
	// Access intraday (real-time) endpoints
	Intraday() *StockIntradayClient
	// Access snapshot (market-wide) endpoints
	Snapshot() *StockSnapshotClient
	// Access technical indicator endpoints
	Technical() *StockTechnicalClient
}

// Stock market data client
type StockClient struct {
	ffiObject FfiObject
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
type StockCorporateActionsClientInterface interface {
	// Get capital structure changes (sync/blocking)
	CapitalChangesSync(date *string, startDate *string, endDate *string) (CapitalChangesResponse, *MarketDataError)
	// Get dividend announcements (sync/blocking)
	DividendsSync(date *string, startDate *string, endDate *string) (DividendsResponse, *MarketDataError)
	// Get capital structure changes (async)
	GetCapitalChanges(date *string, startDate *string, endDate *string) (CapitalChangesResponse, *MarketDataError)
	// Get dividend announcements (async)
	GetDividends(date *string, startDate *string, endDate *string) (DividendsResponse, *MarketDataError)
	// Get IPO listing applicants (async)
	GetListingApplicants(date *string, startDate *string, endDate *string) (ListingApplicantsResponse, *MarketDataError)
	// Get IPO listing applicants (sync/blocking)
	ListingApplicantsSync(date *string, startDate *string, endDate *string) (ListingApplicantsResponse, *MarketDataError)
}

// Stock corporate actions endpoints
//
// Provides access to capital changes, dividends, and listing applicants (IPO).
type StockCorporateActionsClient struct {
	ffiObject FfiObject
}

// Get capital structure changes (sync/blocking)
func (_self *StockCorporateActionsClient) CapitalChangesSync(date *string, startDate *string, endDate *string) (CapitalChangesResponse, *MarketDataError) {
	_pointer := _self.ffiObject.incrementPointer("*StockCorporateActionsClient")
	defer _self.ffiObject.decrementPointer()
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_method_stockcorporateactionsclient_capital_changes_sync(
				_pointer, FfiConverterOptionalStringINSTANCE.Lower(date), FfiConverterOptionalStringINSTANCE.Lower(startDate), FfiConverterOptionalStringINSTANCE.Lower(endDate), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue CapitalChangesResponse
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterCapitalChangesResponseINSTANCE.Lift(_uniffiRV), _uniffiErr
	}
}

// Get dividend announcements (sync/blocking)
func (_self *StockCorporateActionsClient) DividendsSync(date *string, startDate *string, endDate *string) (DividendsResponse, *MarketDataError) {
	_pointer := _self.ffiObject.incrementPointer("*StockCorporateActionsClient")
	defer _self.ffiObject.decrementPointer()
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_method_stockcorporateactionsclient_dividends_sync(
				_pointer, FfiConverterOptionalStringINSTANCE.Lower(date), FfiConverterOptionalStringINSTANCE.Lower(startDate), FfiConverterOptionalStringINSTANCE.Lower(endDate), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue DividendsResponse
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterDividendsResponseINSTANCE.Lift(_uniffiRV), _uniffiErr
	}
}

// Get capital structure changes (async)
func (_self *StockCorporateActionsClient) GetCapitalChanges(date *string, startDate *string, endDate *string) (CapitalChangesResponse, *MarketDataError) {
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
		func(ffi RustBufferI) CapitalChangesResponse {
			return FfiConverterCapitalChangesResponseINSTANCE.Lift(ffi)
		},
		C.uniffi_marketdata_uniffi_fn_method_stockcorporateactionsclient_get_capital_changes(
			_pointer, FfiConverterOptionalStringINSTANCE.Lower(date), FfiConverterOptionalStringINSTANCE.Lower(startDate), FfiConverterOptionalStringINSTANCE.Lower(endDate)),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_rust_buffer(handle)
		},
	)

	return res, err
}

// Get dividend announcements (async)
func (_self *StockCorporateActionsClient) GetDividends(date *string, startDate *string, endDate *string) (DividendsResponse, *MarketDataError) {
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
		func(ffi RustBufferI) DividendsResponse {
			return FfiConverterDividendsResponseINSTANCE.Lift(ffi)
		},
		C.uniffi_marketdata_uniffi_fn_method_stockcorporateactionsclient_get_dividends(
			_pointer, FfiConverterOptionalStringINSTANCE.Lower(date), FfiConverterOptionalStringINSTANCE.Lower(startDate), FfiConverterOptionalStringINSTANCE.Lower(endDate)),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_rust_buffer(handle)
		},
	)

	return res, err
}

// Get IPO listing applicants (async)
func (_self *StockCorporateActionsClient) GetListingApplicants(date *string, startDate *string, endDate *string) (ListingApplicantsResponse, *MarketDataError) {
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
		func(ffi RustBufferI) ListingApplicantsResponse {
			return FfiConverterListingApplicantsResponseINSTANCE.Lift(ffi)
		},
		C.uniffi_marketdata_uniffi_fn_method_stockcorporateactionsclient_get_listing_applicants(
			_pointer, FfiConverterOptionalStringINSTANCE.Lower(date), FfiConverterOptionalStringINSTANCE.Lower(startDate), FfiConverterOptionalStringINSTANCE.Lower(endDate)),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_rust_buffer(handle)
		},
	)

	return res, err
}

// Get IPO listing applicants (sync/blocking)
func (_self *StockCorporateActionsClient) ListingApplicantsSync(date *string, startDate *string, endDate *string) (ListingApplicantsResponse, *MarketDataError) {
	_pointer := _self.ffiObject.incrementPointer("*StockCorporateActionsClient")
	defer _self.ffiObject.decrementPointer()
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_method_stockcorporateactionsclient_listing_applicants_sync(
				_pointer, FfiConverterOptionalStringINSTANCE.Lower(date), FfiConverterOptionalStringINSTANCE.Lower(startDate), FfiConverterOptionalStringINSTANCE.Lower(endDate), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue ListingApplicantsResponse
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterListingApplicantsResponseINSTANCE.Lift(_uniffiRV), _uniffiErr
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

// Stock historical endpoints with typed model returns
//
// All methods have both async (get_*) and sync (*_sync) variants:
// - Async methods are preferred for best performance (non-blocking)
// - Sync methods block the calling thread (simpler API for scripting)
type StockHistoricalClientInterface interface {
	// Get historical candles for a symbol (sync/blocking)
	CandlesSync(symbol string, from *string, to *string, timeframe *string) (HistoricalCandlesResponse, *MarketDataError)
	// Get historical candles for a symbol (async)
	//
	// Parameters:
	// - symbol: Stock symbol (e.g., "2330")
	// - from: Start date (YYYY-MM-DD, optional)
	// - to: End date (YYYY-MM-DD, optional)
	// - timeframe: "D" (day), "W" (week), "M" (month), or intraday "1", "5", "10", "15", "30", "60"
	GetCandles(symbol string, from *string, to *string, timeframe *string) (HistoricalCandlesResponse, *MarketDataError)
	// Get historical stats for a symbol (async)
	//
	// Returns summary statistics including 52-week high/low
	GetStats(symbol string) (StatsResponse, *MarketDataError)
	// Get historical stats for a symbol (sync/blocking)
	StatsSync(symbol string) (StatsResponse, *MarketDataError)
}

// Stock historical endpoints with typed model returns
//
// All methods have both async (get_*) and sync (*_sync) variants:
// - Async methods are preferred for best performance (non-blocking)
// - Sync methods block the calling thread (simpler API for scripting)
type StockHistoricalClient struct {
	ffiObject FfiObject
}

// Get historical candles for a symbol (sync/blocking)
func (_self *StockHistoricalClient) CandlesSync(symbol string, from *string, to *string, timeframe *string) (HistoricalCandlesResponse, *MarketDataError) {
	_pointer := _self.ffiObject.incrementPointer("*StockHistoricalClient")
	defer _self.ffiObject.decrementPointer()
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_method_stockhistoricalclient_candles_sync(
				_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterOptionalStringINSTANCE.Lower(from), FfiConverterOptionalStringINSTANCE.Lower(to), FfiConverterOptionalStringINSTANCE.Lower(timeframe), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue HistoricalCandlesResponse
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterHistoricalCandlesResponseINSTANCE.Lift(_uniffiRV), _uniffiErr
	}
}

// Get historical candles for a symbol (async)
//
// Parameters:
// - symbol: Stock symbol (e.g., "2330")
// - from: Start date (YYYY-MM-DD, optional)
// - to: End date (YYYY-MM-DD, optional)
// - timeframe: "D" (day), "W" (week), "M" (month), or intraday "1", "5", "10", "15", "30", "60"
func (_self *StockHistoricalClient) GetCandles(symbol string, from *string, to *string, timeframe *string) (HistoricalCandlesResponse, *MarketDataError) {
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
		func(ffi RustBufferI) HistoricalCandlesResponse {
			return FfiConverterHistoricalCandlesResponseINSTANCE.Lift(ffi)
		},
		C.uniffi_marketdata_uniffi_fn_method_stockhistoricalclient_get_candles(
			_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterOptionalStringINSTANCE.Lower(from), FfiConverterOptionalStringINSTANCE.Lower(to), FfiConverterOptionalStringINSTANCE.Lower(timeframe)),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_rust_buffer(handle)
		},
	)

	return res, err
}

// Get historical stats for a symbol (async)
//
// Returns summary statistics including 52-week high/low
func (_self *StockHistoricalClient) GetStats(symbol string) (StatsResponse, *MarketDataError) {
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
		func(ffi RustBufferI) StatsResponse {
			return FfiConverterStatsResponseINSTANCE.Lift(ffi)
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

	return res, err
}

// Get historical stats for a symbol (sync/blocking)
func (_self *StockHistoricalClient) StatsSync(symbol string) (StatsResponse, *MarketDataError) {
	_pointer := _self.ffiObject.incrementPointer("*StockHistoricalClient")
	defer _self.ffiObject.decrementPointer()
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_method_stockhistoricalclient_stats_sync(
				_pointer, FfiConverterStringINSTANCE.Lower(symbol), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue StatsResponse
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterStatsResponseINSTANCE.Lift(_uniffiRV), _uniffiErr
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

// Stock intraday endpoints with typed model returns
//
// All methods have both async (get_*) and sync (*_sync) variants:
// - Async methods are preferred for best performance (non-blocking)
// - Sync methods block the calling thread (simpler API for scripting)
type StockIntradayClientInterface interface {
	// Get candlestick data for a symbol (sync/blocking)
	CandlesSync(symbol string, timeframe string) (IntradayCandlesResponse, *MarketDataError)
	// Get candlestick data for a symbol (async)
	//
	// timeframe: "1", "5", "10", "15", "30", "60" (minutes)
	// Returns typed IntradayCandlesResponse with OHLCV data.
	GetCandles(symbol string, timeframe string) (IntradayCandlesResponse, *MarketDataError)
	// Get quote for a symbol (async)
	//
	// Returns typed Quote model with all fields directly accessible.
	GetQuote(symbol string) (Quote, *MarketDataError)
	// Get ticker info for a symbol (async)
	//
	// Returns typed Ticker model with stock metadata.
	GetTicker(symbol string) (Ticker, *MarketDataError)
	// Get batch tickers for a security type (async)
	//
	// typ: Security type (e.g., "EQUITY", "INDEX", "ETF")
	GetTickers(typ string) ([]Ticker, *MarketDataError)
	// Get trade history for a symbol (async)
	//
	// Returns typed TradesResponse with list of trades.
	GetTrades(symbol string) (TradesResponse, *MarketDataError)
	// Get volume breakdown for a symbol (async)
	//
	// Returns typed VolumesResponse with volume at price data.
	GetVolumes(symbol string) (VolumesResponse, *MarketDataError)
	// Get quote for a symbol (sync/blocking)
	QuoteSync(symbol string) (Quote, *MarketDataError)
	// Get ticker info for a symbol (sync/blocking)
	TickerSync(symbol string) (Ticker, *MarketDataError)
	// Get batch tickers for a security type (sync/blocking)
	//
	// typ: Security type (e.g., "EQUITY", "INDEX", "ETF")
	TickersSync(typ string) ([]Ticker, *MarketDataError)
	// Get trade history for a symbol (sync/blocking)
	TradesSync(symbol string) (TradesResponse, *MarketDataError)
	// Get volume breakdown for a symbol (sync/blocking)
	VolumesSync(symbol string) (VolumesResponse, *MarketDataError)
}

// Stock intraday endpoints with typed model returns
//
// All methods have both async (get_*) and sync (*_sync) variants:
// - Async methods are preferred for best performance (non-blocking)
// - Sync methods block the calling thread (simpler API for scripting)
type StockIntradayClient struct {
	ffiObject FfiObject
}

// Get candlestick data for a symbol (sync/blocking)
func (_self *StockIntradayClient) CandlesSync(symbol string, timeframe string) (IntradayCandlesResponse, *MarketDataError) {
	_pointer := _self.ffiObject.incrementPointer("*StockIntradayClient")
	defer _self.ffiObject.decrementPointer()
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_method_stockintradayclient_candles_sync(
				_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterStringINSTANCE.Lower(timeframe), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue IntradayCandlesResponse
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterIntradayCandlesResponseINSTANCE.Lift(_uniffiRV), _uniffiErr
	}
}

// Get candlestick data for a symbol (async)
//
// timeframe: "1", "5", "10", "15", "30", "60" (minutes)
// Returns typed IntradayCandlesResponse with OHLCV data.
func (_self *StockIntradayClient) GetCandles(symbol string, timeframe string) (IntradayCandlesResponse, *MarketDataError) {
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
		func(ffi RustBufferI) IntradayCandlesResponse {
			return FfiConverterIntradayCandlesResponseINSTANCE.Lift(ffi)
		},
		C.uniffi_marketdata_uniffi_fn_method_stockintradayclient_get_candles(
			_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterStringINSTANCE.Lower(timeframe)),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_rust_buffer(handle)
		},
	)

	return res, err
}

// Get quote for a symbol (async)
//
// Returns typed Quote model with all fields directly accessible.
func (_self *StockIntradayClient) GetQuote(symbol string) (Quote, *MarketDataError) {
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
		func(ffi RustBufferI) Quote {
			return FfiConverterQuoteINSTANCE.Lift(ffi)
		},
		C.uniffi_marketdata_uniffi_fn_method_stockintradayclient_get_quote(
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

	return res, err
}

// Get ticker info for a symbol (async)
//
// Returns typed Ticker model with stock metadata.
func (_self *StockIntradayClient) GetTicker(symbol string) (Ticker, *MarketDataError) {
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
		func(ffi RustBufferI) Ticker {
			return FfiConverterTickerINSTANCE.Lift(ffi)
		},
		C.uniffi_marketdata_uniffi_fn_method_stockintradayclient_get_ticker(
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

	return res, err
}

// Get batch tickers for a security type (async)
//
// typ: Security type (e.g., "EQUITY", "INDEX", "ETF")
func (_self *StockIntradayClient) GetTickers(typ string) ([]Ticker, *MarketDataError) {
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
		func(ffi RustBufferI) []Ticker {
			return FfiConverterSequenceTickerINSTANCE.Lift(ffi)
		},
		C.uniffi_marketdata_uniffi_fn_method_stockintradayclient_get_tickers(
			_pointer, FfiConverterStringINSTANCE.Lower(typ)),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_rust_buffer(handle)
		},
	)

	return res, err
}

// Get trade history for a symbol (async)
//
// Returns typed TradesResponse with list of trades.
func (_self *StockIntradayClient) GetTrades(symbol string) (TradesResponse, *MarketDataError) {
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
		func(ffi RustBufferI) TradesResponse {
			return FfiConverterTradesResponseINSTANCE.Lift(ffi)
		},
		C.uniffi_marketdata_uniffi_fn_method_stockintradayclient_get_trades(
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

	return res, err
}

// Get volume breakdown for a symbol (async)
//
// Returns typed VolumesResponse with volume at price data.
func (_self *StockIntradayClient) GetVolumes(symbol string) (VolumesResponse, *MarketDataError) {
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
		func(ffi RustBufferI) VolumesResponse {
			return FfiConverterVolumesResponseINSTANCE.Lift(ffi)
		},
		C.uniffi_marketdata_uniffi_fn_method_stockintradayclient_get_volumes(
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

	return res, err
}

// Get quote for a symbol (sync/blocking)
func (_self *StockIntradayClient) QuoteSync(symbol string) (Quote, *MarketDataError) {
	_pointer := _self.ffiObject.incrementPointer("*StockIntradayClient")
	defer _self.ffiObject.decrementPointer()
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_method_stockintradayclient_quote_sync(
				_pointer, FfiConverterStringINSTANCE.Lower(symbol), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue Quote
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterQuoteINSTANCE.Lift(_uniffiRV), _uniffiErr
	}
}

// Get ticker info for a symbol (sync/blocking)
func (_self *StockIntradayClient) TickerSync(symbol string) (Ticker, *MarketDataError) {
	_pointer := _self.ffiObject.incrementPointer("*StockIntradayClient")
	defer _self.ffiObject.decrementPointer()
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_method_stockintradayclient_ticker_sync(
				_pointer, FfiConverterStringINSTANCE.Lower(symbol), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue Ticker
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterTickerINSTANCE.Lift(_uniffiRV), _uniffiErr
	}
}

// Get batch tickers for a security type (sync/blocking)
//
// typ: Security type (e.g., "EQUITY", "INDEX", "ETF")
func (_self *StockIntradayClient) TickersSync(typ string) ([]Ticker, *MarketDataError) {
	_pointer := _self.ffiObject.incrementPointer("*StockIntradayClient")
	defer _self.ffiObject.decrementPointer()
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_method_stockintradayclient_tickers_sync(
				_pointer, FfiConverterStringINSTANCE.Lower(typ), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue []Ticker
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterSequenceTickerINSTANCE.Lift(_uniffiRV), _uniffiErr
	}
}

// Get trade history for a symbol (sync/blocking)
func (_self *StockIntradayClient) TradesSync(symbol string) (TradesResponse, *MarketDataError) {
	_pointer := _self.ffiObject.incrementPointer("*StockIntradayClient")
	defer _self.ffiObject.decrementPointer()
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_method_stockintradayclient_trades_sync(
				_pointer, FfiConverterStringINSTANCE.Lower(symbol), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue TradesResponse
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterTradesResponseINSTANCE.Lift(_uniffiRV), _uniffiErr
	}
}

// Get volume breakdown for a symbol (sync/blocking)
func (_self *StockIntradayClient) VolumesSync(symbol string) (VolumesResponse, *MarketDataError) {
	_pointer := _self.ffiObject.incrementPointer("*StockIntradayClient")
	defer _self.ffiObject.decrementPointer()
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_method_stockintradayclient_volumes_sync(
				_pointer, FfiConverterStringINSTANCE.Lower(symbol), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue VolumesResponse
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterVolumesResponseINSTANCE.Lift(_uniffiRV), _uniffiErr
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

// Stock snapshot endpoints for market-wide data
//
// Provides access to quotes, movers (gainers/losers), and most active stocks
// across entire markets.
type StockSnapshotClientInterface interface {
	// Get most actively traded stocks (sync/blocking)
	ActivesSync(market string, trade *string) (ActivesResponse, *MarketDataError)
	// Get most actively traded stocks (async)
	//
	// Parameters:
	// - market: Market code (TSE, OTC)
	// - trade: "volume" or "value" (optional)
	GetActives(market string, trade *string) (ActivesResponse, *MarketDataError)
	// Get top movers (gainers/losers) in a market (async)
	//
	// Parameters:
	// - market: Market code (TSE, OTC)
	// - direction: "up" for gainers, "down" for losers (optional)
	// - change: "percent" or "value" (optional)
	GetMovers(market string, direction *string, change *string) (MoversResponse, *MarketDataError)
	// Get market-wide snapshot quotes (async)
	//
	// Parameters:
	// - market: Market code (TSE, OTC, ESB, TIB, PSB)
	// - type_filter: Optional filter (ALL, ALLBUT0999, COMMONSTOCK)
	GetQuotes(market string, typeFilter *string) (SnapshotQuotesResponse, *MarketDataError)
	// Get top movers (sync/blocking)
	MoversSync(market string, direction *string, change *string) (MoversResponse, *MarketDataError)
	// Get market-wide snapshot quotes (sync/blocking)
	QuotesSync(market string, typeFilter *string) (SnapshotQuotesResponse, *MarketDataError)
}

// Stock snapshot endpoints for market-wide data
//
// Provides access to quotes, movers (gainers/losers), and most active stocks
// across entire markets.
type StockSnapshotClient struct {
	ffiObject FfiObject
}

// Get most actively traded stocks (sync/blocking)
func (_self *StockSnapshotClient) ActivesSync(market string, trade *string) (ActivesResponse, *MarketDataError) {
	_pointer := _self.ffiObject.incrementPointer("*StockSnapshotClient")
	defer _self.ffiObject.decrementPointer()
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_method_stocksnapshotclient_actives_sync(
				_pointer, FfiConverterStringINSTANCE.Lower(market), FfiConverterOptionalStringINSTANCE.Lower(trade), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue ActivesResponse
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterActivesResponseINSTANCE.Lift(_uniffiRV), _uniffiErr
	}
}

// Get most actively traded stocks (async)
//
// Parameters:
// - market: Market code (TSE, OTC)
// - trade: "volume" or "value" (optional)
func (_self *StockSnapshotClient) GetActives(market string, trade *string) (ActivesResponse, *MarketDataError) {
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
		func(ffi RustBufferI) ActivesResponse {
			return FfiConverterActivesResponseINSTANCE.Lift(ffi)
		},
		C.uniffi_marketdata_uniffi_fn_method_stocksnapshotclient_get_actives(
			_pointer, FfiConverterStringINSTANCE.Lower(market), FfiConverterOptionalStringINSTANCE.Lower(trade)),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_rust_buffer(handle)
		},
	)

	return res, err
}

// Get top movers (gainers/losers) in a market (async)
//
// Parameters:
// - market: Market code (TSE, OTC)
// - direction: "up" for gainers, "down" for losers (optional)
// - change: "percent" or "value" (optional)
func (_self *StockSnapshotClient) GetMovers(market string, direction *string, change *string) (MoversResponse, *MarketDataError) {
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
		func(ffi RustBufferI) MoversResponse {
			return FfiConverterMoversResponseINSTANCE.Lift(ffi)
		},
		C.uniffi_marketdata_uniffi_fn_method_stocksnapshotclient_get_movers(
			_pointer, FfiConverterStringINSTANCE.Lower(market), FfiConverterOptionalStringINSTANCE.Lower(direction), FfiConverterOptionalStringINSTANCE.Lower(change)),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_rust_buffer(handle)
		},
	)

	return res, err
}

// Get market-wide snapshot quotes (async)
//
// Parameters:
// - market: Market code (TSE, OTC, ESB, TIB, PSB)
// - type_filter: Optional filter (ALL, ALLBUT0999, COMMONSTOCK)
func (_self *StockSnapshotClient) GetQuotes(market string, typeFilter *string) (SnapshotQuotesResponse, *MarketDataError) {
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
		func(ffi RustBufferI) SnapshotQuotesResponse {
			return FfiConverterSnapshotQuotesResponseINSTANCE.Lift(ffi)
		},
		C.uniffi_marketdata_uniffi_fn_method_stocksnapshotclient_get_quotes(
			_pointer, FfiConverterStringINSTANCE.Lower(market), FfiConverterOptionalStringINSTANCE.Lower(typeFilter)),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_rust_buffer(handle)
		},
	)

	return res, err
}

// Get top movers (sync/blocking)
func (_self *StockSnapshotClient) MoversSync(market string, direction *string, change *string) (MoversResponse, *MarketDataError) {
	_pointer := _self.ffiObject.incrementPointer("*StockSnapshotClient")
	defer _self.ffiObject.decrementPointer()
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_method_stocksnapshotclient_movers_sync(
				_pointer, FfiConverterStringINSTANCE.Lower(market), FfiConverterOptionalStringINSTANCE.Lower(direction), FfiConverterOptionalStringINSTANCE.Lower(change), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue MoversResponse
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterMoversResponseINSTANCE.Lift(_uniffiRV), _uniffiErr
	}
}

// Get market-wide snapshot quotes (sync/blocking)
func (_self *StockSnapshotClient) QuotesSync(market string, typeFilter *string) (SnapshotQuotesResponse, *MarketDataError) {
	_pointer := _self.ffiObject.incrementPointer("*StockSnapshotClient")
	defer _self.ffiObject.decrementPointer()
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_method_stocksnapshotclient_quotes_sync(
				_pointer, FfiConverterStringINSTANCE.Lower(market), FfiConverterOptionalStringINSTANCE.Lower(typeFilter), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue SnapshotQuotesResponse
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterSnapshotQuotesResponseINSTANCE.Lift(_uniffiRV), _uniffiErr
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
type StockTechnicalClientInterface interface {
	// Get Bollinger Bands (sync/blocking)
	BbSync(symbol string, from *string, to *string, timeframe *string, period *uint32, stddev *float64) (BbResponse, *MarketDataError)
	// Get Bollinger Bands (async)
	GetBb(symbol string, from *string, to *string, timeframe *string, period *uint32, stddev *float64) (BbResponse, *MarketDataError)
	// Get KDJ (Stochastic Oscillator) (async)
	GetKdj(symbol string, from *string, to *string, timeframe *string, period *uint32) (KdjResponse, *MarketDataError)
	// Get MACD indicator (async)
	GetMacd(symbol string, from *string, to *string, timeframe *string, fast *uint32, slow *uint32, signal *uint32) (MacdResponse, *MarketDataError)
	// Get Relative Strength Index (async)
	GetRsi(symbol string, from *string, to *string, timeframe *string, period *uint32) (RsiResponse, *MarketDataError)
	// Get Simple Moving Average (async)
	GetSma(symbol string, from *string, to *string, timeframe *string, period *uint32) (SmaResponse, *MarketDataError)
	// Get KDJ (sync/blocking)
	KdjSync(symbol string, from *string, to *string, timeframe *string, period *uint32) (KdjResponse, *MarketDataError)
	// Get MACD (sync/blocking)
	MacdSync(symbol string, from *string, to *string, timeframe *string, fast *uint32, slow *uint32, signal *uint32) (MacdResponse, *MarketDataError)
	// Get Relative Strength Index (sync/blocking)
	RsiSync(symbol string, from *string, to *string, timeframe *string, period *uint32) (RsiResponse, *MarketDataError)
	// Get Simple Moving Average (sync/blocking)
	SmaSync(symbol string, from *string, to *string, timeframe *string, period *uint32) (SmaResponse, *MarketDataError)
}

// Stock technical indicator endpoints
//
// Provides access to SMA, RSI, KDJ, MACD, and Bollinger Bands indicators.
type StockTechnicalClient struct {
	ffiObject FfiObject
}

// Get Bollinger Bands (sync/blocking)
func (_self *StockTechnicalClient) BbSync(symbol string, from *string, to *string, timeframe *string, period *uint32, stddev *float64) (BbResponse, *MarketDataError) {
	_pointer := _self.ffiObject.incrementPointer("*StockTechnicalClient")
	defer _self.ffiObject.decrementPointer()
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_method_stocktechnicalclient_bb_sync(
				_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterOptionalStringINSTANCE.Lower(from), FfiConverterOptionalStringINSTANCE.Lower(to), FfiConverterOptionalStringINSTANCE.Lower(timeframe), FfiConverterOptionalUint32INSTANCE.Lower(period), FfiConverterOptionalFloat64INSTANCE.Lower(stddev), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue BbResponse
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterBbResponseINSTANCE.Lift(_uniffiRV), _uniffiErr
	}
}

// Get Bollinger Bands (async)
func (_self *StockTechnicalClient) GetBb(symbol string, from *string, to *string, timeframe *string, period *uint32, stddev *float64) (BbResponse, *MarketDataError) {
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
		func(ffi RustBufferI) BbResponse {
			return FfiConverterBbResponseINSTANCE.Lift(ffi)
		},
		C.uniffi_marketdata_uniffi_fn_method_stocktechnicalclient_get_bb(
			_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterOptionalStringINSTANCE.Lower(from), FfiConverterOptionalStringINSTANCE.Lower(to), FfiConverterOptionalStringINSTANCE.Lower(timeframe), FfiConverterOptionalUint32INSTANCE.Lower(period), FfiConverterOptionalFloat64INSTANCE.Lower(stddev)),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_rust_buffer(handle)
		},
	)

	return res, err
}

// Get KDJ (Stochastic Oscillator) (async)
func (_self *StockTechnicalClient) GetKdj(symbol string, from *string, to *string, timeframe *string, period *uint32) (KdjResponse, *MarketDataError) {
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
		func(ffi RustBufferI) KdjResponse {
			return FfiConverterKdjResponseINSTANCE.Lift(ffi)
		},
		C.uniffi_marketdata_uniffi_fn_method_stocktechnicalclient_get_kdj(
			_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterOptionalStringINSTANCE.Lower(from), FfiConverterOptionalStringINSTANCE.Lower(to), FfiConverterOptionalStringINSTANCE.Lower(timeframe), FfiConverterOptionalUint32INSTANCE.Lower(period)),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_rust_buffer(handle)
		},
	)

	return res, err
}

// Get MACD indicator (async)
func (_self *StockTechnicalClient) GetMacd(symbol string, from *string, to *string, timeframe *string, fast *uint32, slow *uint32, signal *uint32) (MacdResponse, *MarketDataError) {
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
		func(ffi RustBufferI) MacdResponse {
			return FfiConverterMacdResponseINSTANCE.Lift(ffi)
		},
		C.uniffi_marketdata_uniffi_fn_method_stocktechnicalclient_get_macd(
			_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterOptionalStringINSTANCE.Lower(from), FfiConverterOptionalStringINSTANCE.Lower(to), FfiConverterOptionalStringINSTANCE.Lower(timeframe), FfiConverterOptionalUint32INSTANCE.Lower(fast), FfiConverterOptionalUint32INSTANCE.Lower(slow), FfiConverterOptionalUint32INSTANCE.Lower(signal)),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_rust_buffer(handle)
		},
	)

	return res, err
}

// Get Relative Strength Index (async)
func (_self *StockTechnicalClient) GetRsi(symbol string, from *string, to *string, timeframe *string, period *uint32) (RsiResponse, *MarketDataError) {
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
		func(ffi RustBufferI) RsiResponse {
			return FfiConverterRsiResponseINSTANCE.Lift(ffi)
		},
		C.uniffi_marketdata_uniffi_fn_method_stocktechnicalclient_get_rsi(
			_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterOptionalStringINSTANCE.Lower(from), FfiConverterOptionalStringINSTANCE.Lower(to), FfiConverterOptionalStringINSTANCE.Lower(timeframe), FfiConverterOptionalUint32INSTANCE.Lower(period)),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_rust_buffer(handle)
		},
	)

	return res, err
}

// Get Simple Moving Average (async)
func (_self *StockTechnicalClient) GetSma(symbol string, from *string, to *string, timeframe *string, period *uint32) (SmaResponse, *MarketDataError) {
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
		func(ffi RustBufferI) SmaResponse {
			return FfiConverterSmaResponseINSTANCE.Lift(ffi)
		},
		C.uniffi_marketdata_uniffi_fn_method_stocktechnicalclient_get_sma(
			_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterOptionalStringINSTANCE.Lower(from), FfiConverterOptionalStringINSTANCE.Lower(to), FfiConverterOptionalStringINSTANCE.Lower(timeframe), FfiConverterOptionalUint32INSTANCE.Lower(period)),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_rust_buffer(handle)
		},
	)

	return res, err
}

// Get KDJ (sync/blocking)
func (_self *StockTechnicalClient) KdjSync(symbol string, from *string, to *string, timeframe *string, period *uint32) (KdjResponse, *MarketDataError) {
	_pointer := _self.ffiObject.incrementPointer("*StockTechnicalClient")
	defer _self.ffiObject.decrementPointer()
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_method_stocktechnicalclient_kdj_sync(
				_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterOptionalStringINSTANCE.Lower(from), FfiConverterOptionalStringINSTANCE.Lower(to), FfiConverterOptionalStringINSTANCE.Lower(timeframe), FfiConverterOptionalUint32INSTANCE.Lower(period), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue KdjResponse
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterKdjResponseINSTANCE.Lift(_uniffiRV), _uniffiErr
	}
}

// Get MACD (sync/blocking)
func (_self *StockTechnicalClient) MacdSync(symbol string, from *string, to *string, timeframe *string, fast *uint32, slow *uint32, signal *uint32) (MacdResponse, *MarketDataError) {
	_pointer := _self.ffiObject.incrementPointer("*StockTechnicalClient")
	defer _self.ffiObject.decrementPointer()
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_method_stocktechnicalclient_macd_sync(
				_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterOptionalStringINSTANCE.Lower(from), FfiConverterOptionalStringINSTANCE.Lower(to), FfiConverterOptionalStringINSTANCE.Lower(timeframe), FfiConverterOptionalUint32INSTANCE.Lower(fast), FfiConverterOptionalUint32INSTANCE.Lower(slow), FfiConverterOptionalUint32INSTANCE.Lower(signal), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue MacdResponse
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterMacdResponseINSTANCE.Lift(_uniffiRV), _uniffiErr
	}
}

// Get Relative Strength Index (sync/blocking)
func (_self *StockTechnicalClient) RsiSync(symbol string, from *string, to *string, timeframe *string, period *uint32) (RsiResponse, *MarketDataError) {
	_pointer := _self.ffiObject.incrementPointer("*StockTechnicalClient")
	defer _self.ffiObject.decrementPointer()
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_method_stocktechnicalclient_rsi_sync(
				_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterOptionalStringINSTANCE.Lower(from), FfiConverterOptionalStringINSTANCE.Lower(to), FfiConverterOptionalStringINSTANCE.Lower(timeframe), FfiConverterOptionalUint32INSTANCE.Lower(period), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue RsiResponse
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterRsiResponseINSTANCE.Lift(_uniffiRV), _uniffiErr
	}
}

// Get Simple Moving Average (sync/blocking)
func (_self *StockTechnicalClient) SmaSync(symbol string, from *string, to *string, timeframe *string, period *uint32) (SmaResponse, *MarketDataError) {
	_pointer := _self.ffiObject.incrementPointer("*StockTechnicalClient")
	defer _self.ffiObject.decrementPointer()
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_marketdata_uniffi_fn_method_stocktechnicalclient_sma_sync(
				_pointer, FfiConverterStringINSTANCE.Lower(symbol), FfiConverterOptionalStringINSTANCE.Lower(from), FfiConverterOptionalStringINSTANCE.Lower(to), FfiConverterOptionalStringINSTANCE.Lower(timeframe), FfiConverterOptionalUint32INSTANCE.Lower(period), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue SmaResponse
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterSmaResponseINSTANCE.Lift(_uniffiRV), _uniffiErr
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
	Connect() *MarketDataError
	Disconnect()
	// Check if the client has been shut down
	IsClosed() bool
	// Check if the client is currently connected
	IsConnected() bool
	Ping(state *string) *MarketDataError
	QuerySubscriptions() *MarketDataError
	Subscribe(channel string, symbol string) *MarketDataError
	Unsubscribe(channel string, symbol string) *MarketDataError
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
func WebSocketClientNewWithFullConfig(apiKey string, listener WebSocketListener, endpoint WebSocketEndpoint, baseUrl *string, reconnectConfig *ReconnectConfigRecord, healthCheckConfig *HealthCheckConfigRecord, tls *TlsConfigRecord) *WebSocketClient {
	return FfiConverterWebSocketClientINSTANCE.Lift(rustCall(func(_uniffiStatus *C.RustCallStatus) unsafe.Pointer {
		return C.uniffi_marketdata_uniffi_fn_constructor_websocketclient_new_with_full_config(FfiConverterStringINSTANCE.Lower(apiKey), FfiConverterWebSocketListenerINSTANCE.Lower(listener), FfiConverterWebSocketEndpointINSTANCE.Lower(endpoint), FfiConverterOptionalStringINSTANCE.Lower(baseUrl), FfiConverterOptionalReconnectConfigRecordINSTANCE.Lower(reconnectConfig), FfiConverterOptionalHealthCheckConfigRecordINSTANCE.Lower(healthCheckConfig), FfiConverterOptionalTlsConfigRecordINSTANCE.Lower(tls), _uniffiStatus)
	}))
}

// Create a new WebSocket client with full configuration including custom base URL
func WebSocketClientNewWithUrl(apiKey string, listener WebSocketListener, endpoint WebSocketEndpoint, baseUrl string, reconnectConfig *ReconnectConfigRecord, healthCheckConfig *HealthCheckConfigRecord) *WebSocketClient {
	return FfiConverterWebSocketClientINSTANCE.Lift(rustCall(func(_uniffiStatus *C.RustCallStatus) unsafe.Pointer {
		return C.uniffi_marketdata_uniffi_fn_constructor_websocketclient_new_with_url(FfiConverterStringINSTANCE.Lower(apiKey), FfiConverterWebSocketListenerINSTANCE.Lower(listener), FfiConverterWebSocketEndpointINSTANCE.Lower(endpoint), FfiConverterStringINSTANCE.Lower(baseUrl), FfiConverterOptionalReconnectConfigRecordINSTANCE.Lower(reconnectConfig), FfiConverterOptionalHealthCheckConfigRecordINSTANCE.Lower(healthCheckConfig), _uniffiStatus)
	}))
}

func (_self *WebSocketClient) Connect() *MarketDataError {
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

	return err
}

func (_self *WebSocketClient) Disconnect() {
	_pointer := _self.ffiObject.incrementPointer("*WebSocketClient")
	defer _self.ffiObject.decrementPointer()
	uniffiRustCallAsync[struct{}](
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

// Check if the client has been shut down
func (_self *WebSocketClient) IsClosed() bool {
	_pointer := _self.ffiObject.incrementPointer("*WebSocketClient")
	defer _self.ffiObject.decrementPointer()
	return FfiConverterBoolINSTANCE.Lift(rustCall(func(_uniffiStatus *C.RustCallStatus) C.int8_t {
		return C.uniffi_marketdata_uniffi_fn_method_websocketclient_is_closed(
			_pointer, _uniffiStatus)
	}))
}

// Check if the client is currently connected
func (_self *WebSocketClient) IsConnected() bool {
	_pointer := _self.ffiObject.incrementPointer("*WebSocketClient")
	defer _self.ffiObject.decrementPointer()
	return FfiConverterBoolINSTANCE.Lift(rustCall(func(_uniffiStatus *C.RustCallStatus) C.int8_t {
		return C.uniffi_marketdata_uniffi_fn_method_websocketclient_is_connected(
			_pointer, _uniffiStatus)
	}))
}

func (_self *WebSocketClient) Ping(state *string) *MarketDataError {
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

	return err
}

func (_self *WebSocketClient) QuerySubscriptions() *MarketDataError {
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

	return err
}

func (_self *WebSocketClient) Subscribe(channel string, symbol string) *MarketDataError {
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
			_pointer, FfiConverterStringINSTANCE.Lower(channel), FfiConverterStringINSTANCE.Lower(symbol)),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_void(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_void(handle)
		},
	)

	return err
}

func (_self *WebSocketClient) Unsubscribe(channel string, symbol string) *MarketDataError {
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
			_pointer, FfiConverterStringINSTANCE.Lower(channel), FfiConverterStringINSTANCE.Lower(symbol)),
		// pollFn
		func(handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_poll_void(handle, continuation, data)
		},
		// freeFn
		func(handle C.uint64_t) {
			C.ffi_marketdata_uniffi_rust_future_free_void(handle)
		},
	)

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
// public void OnDisconnected() {
// Console.WriteLine("Disconnected");
// }
// public void OnMessage(StreamMessage message) {
// Console.WriteLine($"Got {message.Event} for {message.Symbol}");
// }
// public void OnError(string errorMessage) {
// Console.WriteLine($"Error: {errorMessage}");
// }
// }
// ```
type WebSocketListener interface {
	// Called when WebSocket connection is established
	OnConnected()
	// Called when WebSocket connection is closed
	OnDisconnected()
	// Called when a message is received
	OnMessage(message StreamMessage)
	// Called when an error occurs
	OnError(errorMessage string)
	// Called when a reconnection attempt starts
	OnReconnecting(attempt uint32)
	// Called when all reconnection attempts are exhausted
	OnReconnectFailed(attempts uint32)
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
// public void OnDisconnected() {
// Console.WriteLine("Disconnected");
// }
// public void OnMessage(StreamMessage message) {
// Console.WriteLine($"Got {message.Event} for {message.Symbol}");
// }
// public void OnError(string errorMessage) {
// Console.WriteLine($"Error: {errorMessage}");
// }
// }
// ```
type WebSocketListenerImpl struct {
	ffiObject FfiObject
}

// Called when WebSocket connection is established
func (_self *WebSocketListenerImpl) OnConnected() {
	_pointer := _self.ffiObject.incrementPointer("WebSocketListener")
	defer _self.ffiObject.decrementPointer()
	rustCall(func(_uniffiStatus *C.RustCallStatus) bool {
		C.uniffi_marketdata_uniffi_fn_method_websocketlistener_on_connected(
			_pointer, _uniffiStatus)
		return false
	})
}

// Called when WebSocket connection is closed
func (_self *WebSocketListenerImpl) OnDisconnected() {
	_pointer := _self.ffiObject.incrementPointer("WebSocketListener")
	defer _self.ffiObject.decrementPointer()
	rustCall(func(_uniffiStatus *C.RustCallStatus) bool {
		C.uniffi_marketdata_uniffi_fn_method_websocketlistener_on_disconnected(
			_pointer, _uniffiStatus)
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
func (_self *WebSocketListenerImpl) OnError(errorMessage string) {
	_pointer := _self.ffiObject.incrementPointer("WebSocketListener")
	defer _self.ffiObject.decrementPointer()
	rustCall(func(_uniffiStatus *C.RustCallStatus) bool {
		C.uniffi_marketdata_uniffi_fn_method_websocketlistener_on_error(
			_pointer, FfiConverterStringINSTANCE.Lower(errorMessage), _uniffiStatus)
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

// Called when all reconnection attempts are exhausted
func (_self *WebSocketListenerImpl) OnReconnectFailed(attempts uint32) {
	_pointer := _self.ffiObject.incrementPointer("WebSocketListener")
	defer _self.ffiObject.decrementPointer()
	rustCall(func(_uniffiStatus *C.RustCallStatus) bool {
		C.uniffi_marketdata_uniffi_fn_method_websocketlistener_on_reconnect_failed(
			_pointer, FfiConverterUint32INSTANCE.Lower(attempts), _uniffiStatus)
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
func marketdata_uniffi_cgo_dispatchCallbackInterfaceWebSocketListenerMethod1(uniffiHandle C.uint64_t, uniffiOutReturn *C.void, callStatus *C.RustCallStatus) {
	handle := uint64(uniffiHandle)
	uniffiObj, ok := FfiConverterWebSocketListenerINSTANCE.handleMap.tryGet(handle)
	if !ok {
		panic(fmt.Errorf("no callback in handle map: %d", handle))
	}

	uniffiObj.OnDisconnected()

}

//export marketdata_uniffi_cgo_dispatchCallbackInterfaceWebSocketListenerMethod2
func marketdata_uniffi_cgo_dispatchCallbackInterfaceWebSocketListenerMethod2(uniffiHandle C.uint64_t, message C.RustBuffer, uniffiOutReturn *C.void, callStatus *C.RustCallStatus) {
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

//export marketdata_uniffi_cgo_dispatchCallbackInterfaceWebSocketListenerMethod3
func marketdata_uniffi_cgo_dispatchCallbackInterfaceWebSocketListenerMethod3(uniffiHandle C.uint64_t, errorMessage C.RustBuffer, uniffiOutReturn *C.void, callStatus *C.RustCallStatus) {
	handle := uint64(uniffiHandle)
	uniffiObj, ok := FfiConverterWebSocketListenerINSTANCE.handleMap.tryGet(handle)
	if !ok {
		panic(fmt.Errorf("no callback in handle map: %d", handle))
	}

	uniffiObj.OnError(
		FfiConverterStringINSTANCE.Lift(GoRustBuffer{
			inner: errorMessage,
		}),
	)

}

//export marketdata_uniffi_cgo_dispatchCallbackInterfaceWebSocketListenerMethod4
func marketdata_uniffi_cgo_dispatchCallbackInterfaceWebSocketListenerMethod4(uniffiHandle C.uint64_t, attempt C.uint32_t, uniffiOutReturn *C.void, callStatus *C.RustCallStatus) {
	handle := uint64(uniffiHandle)
	uniffiObj, ok := FfiConverterWebSocketListenerINSTANCE.handleMap.tryGet(handle)
	if !ok {
		panic(fmt.Errorf("no callback in handle map: %d", handle))
	}

	uniffiObj.OnReconnecting(
		FfiConverterUint32INSTANCE.Lift(attempt),
	)

}

//export marketdata_uniffi_cgo_dispatchCallbackInterfaceWebSocketListenerMethod5
func marketdata_uniffi_cgo_dispatchCallbackInterfaceWebSocketListenerMethod5(uniffiHandle C.uint64_t, attempts C.uint32_t, uniffiOutReturn *C.void, callStatus *C.RustCallStatus) {
	handle := uint64(uniffiHandle)
	uniffiObj, ok := FfiConverterWebSocketListenerINSTANCE.handleMap.tryGet(handle)
	if !ok {
		panic(fmt.Errorf("no callback in handle map: %d", handle))
	}

	uniffiObj.OnReconnectFailed(
		FfiConverterUint32INSTANCE.Lift(attempts),
	)

}

var UniffiVTableCallbackInterfaceWebSocketListenerINSTANCE = C.UniffiVTableCallbackInterfaceWebSocketListener{
	onConnected:       (C.UniffiCallbackInterfaceWebSocketListenerMethod0)(C.marketdata_uniffi_cgo_dispatchCallbackInterfaceWebSocketListenerMethod0),
	onDisconnected:    (C.UniffiCallbackInterfaceWebSocketListenerMethod1)(C.marketdata_uniffi_cgo_dispatchCallbackInterfaceWebSocketListenerMethod1),
	onMessage:         (C.UniffiCallbackInterfaceWebSocketListenerMethod2)(C.marketdata_uniffi_cgo_dispatchCallbackInterfaceWebSocketListenerMethod2),
	onError:           (C.UniffiCallbackInterfaceWebSocketListenerMethod3)(C.marketdata_uniffi_cgo_dispatchCallbackInterfaceWebSocketListenerMethod3),
	onReconnecting:    (C.UniffiCallbackInterfaceWebSocketListenerMethod4)(C.marketdata_uniffi_cgo_dispatchCallbackInterfaceWebSocketListenerMethod4),
	onReconnectFailed: (C.UniffiCallbackInterfaceWebSocketListenerMethod5)(C.marketdata_uniffi_cgo_dispatchCallbackInterfaceWebSocketListenerMethod5),

	uniffiFree: (C.UniffiCallbackInterfaceFree)(C.marketdata_uniffi_cgo_dispatchCallbackInterfaceWebSocketListenerFree),
}

//export marketdata_uniffi_cgo_dispatchCallbackInterfaceWebSocketListenerFree
func marketdata_uniffi_cgo_dispatchCallbackInterfaceWebSocketListenerFree(handle C.uint64_t) {
	FfiConverterWebSocketListenerINSTANCE.handleMap.remove(uint64(handle))
}

func (c FfiConverterWebSocketListener) register() {
	C.uniffi_marketdata_uniffi_fn_init_callback_vtable_websocketlistener(&UniffiVTableCallbackInterfaceWebSocketListenerINSTANCE)
}

// Single active entry
type Active struct {
	DataType      *string
	Symbol        string
	Name          *string
	OpenPrice     *float64
	HighPrice     *float64
	LowPrice      *float64
	ClosePrice    *float64
	Change        *float64
	ChangePercent *float64
	TradeVolume   *int64
	TradeValue    *float64
	LastUpdated   *int64
}

func (r *Active) Destroy() {
	FfiDestroyerOptionalString{}.Destroy(r.DataType)
	FfiDestroyerString{}.Destroy(r.Symbol)
	FfiDestroyerOptionalString{}.Destroy(r.Name)
	FfiDestroyerOptionalFloat64{}.Destroy(r.OpenPrice)
	FfiDestroyerOptionalFloat64{}.Destroy(r.HighPrice)
	FfiDestroyerOptionalFloat64{}.Destroy(r.LowPrice)
	FfiDestroyerOptionalFloat64{}.Destroy(r.ClosePrice)
	FfiDestroyerOptionalFloat64{}.Destroy(r.Change)
	FfiDestroyerOptionalFloat64{}.Destroy(r.ChangePercent)
	FfiDestroyerOptionalInt64{}.Destroy(r.TradeVolume)
	FfiDestroyerOptionalFloat64{}.Destroy(r.TradeValue)
	FfiDestroyerOptionalInt64{}.Destroy(r.LastUpdated)
}

type FfiConverterActive struct{}

var FfiConverterActiveINSTANCE = FfiConverterActive{}

func (c FfiConverterActive) Lift(rb RustBufferI) Active {
	return LiftFromRustBuffer[Active](c, rb)
}

func (c FfiConverterActive) Read(reader io.Reader) Active {
	return Active{
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterOptionalInt64INSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterOptionalInt64INSTANCE.Read(reader),
	}
}

func (c FfiConverterActive) Lower(value Active) C.RustBuffer {
	return LowerIntoRustBuffer[Active](c, value)
}

func (c FfiConverterActive) Write(writer io.Writer, value Active) {
	FfiConverterOptionalStringINSTANCE.Write(writer, value.DataType)
	FfiConverterStringINSTANCE.Write(writer, value.Symbol)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Name)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.OpenPrice)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.HighPrice)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.LowPrice)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.ClosePrice)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.Change)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.ChangePercent)
	FfiConverterOptionalInt64INSTANCE.Write(writer, value.TradeVolume)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.TradeValue)
	FfiConverterOptionalInt64INSTANCE.Write(writer, value.LastUpdated)
}

type FfiDestroyerActive struct{}

func (_ FfiDestroyerActive) Destroy(value Active) {
	value.Destroy()
}

// Actives response
type ActivesResponse struct {
	Date   string
	Time   string
	Market string
	Data   []Active
}

func (r *ActivesResponse) Destroy() {
	FfiDestroyerString{}.Destroy(r.Date)
	FfiDestroyerString{}.Destroy(r.Time)
	FfiDestroyerString{}.Destroy(r.Market)
	FfiDestroyerSequenceActive{}.Destroy(r.Data)
}

type FfiConverterActivesResponse struct{}

var FfiConverterActivesResponseINSTANCE = FfiConverterActivesResponse{}

func (c FfiConverterActivesResponse) Lift(rb RustBufferI) ActivesResponse {
	return LiftFromRustBuffer[ActivesResponse](c, rb)
}

func (c FfiConverterActivesResponse) Read(reader io.Reader) ActivesResponse {
	return ActivesResponse{
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterSequenceActiveINSTANCE.Read(reader),
	}
}

func (c FfiConverterActivesResponse) Lower(value ActivesResponse) C.RustBuffer {
	return LowerIntoRustBuffer[ActivesResponse](c, value)
}

func (c FfiConverterActivesResponse) Write(writer io.Writer, value ActivesResponse) {
	FfiConverterStringINSTANCE.Write(writer, value.Date)
	FfiConverterStringINSTANCE.Write(writer, value.Time)
	FfiConverterStringINSTANCE.Write(writer, value.Market)
	FfiConverterSequenceActiveINSTANCE.Write(writer, value.Data)
}

type FfiDestroyerActivesResponse struct{}

func (_ FfiDestroyerActivesResponse) Destroy(value ActivesResponse) {
	value.Destroy()
}

// Bollinger Bands data point
type BbDataPoint struct {
	Date   string
	Upper  float64
	Middle float64
	Lower  float64
}

func (r *BbDataPoint) Destroy() {
	FfiDestroyerString{}.Destroy(r.Date)
	FfiDestroyerFloat64{}.Destroy(r.Upper)
	FfiDestroyerFloat64{}.Destroy(r.Middle)
	FfiDestroyerFloat64{}.Destroy(r.Lower)
}

type FfiConverterBbDataPoint struct{}

var FfiConverterBbDataPointINSTANCE = FfiConverterBbDataPoint{}

func (c FfiConverterBbDataPoint) Lift(rb RustBufferI) BbDataPoint {
	return LiftFromRustBuffer[BbDataPoint](c, rb)
}

func (c FfiConverterBbDataPoint) Read(reader io.Reader) BbDataPoint {
	return BbDataPoint{
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterFloat64INSTANCE.Read(reader),
		FfiConverterFloat64INSTANCE.Read(reader),
		FfiConverterFloat64INSTANCE.Read(reader),
	}
}

func (c FfiConverterBbDataPoint) Lower(value BbDataPoint) C.RustBuffer {
	return LowerIntoRustBuffer[BbDataPoint](c, value)
}

func (c FfiConverterBbDataPoint) Write(writer io.Writer, value BbDataPoint) {
	FfiConverterStringINSTANCE.Write(writer, value.Date)
	FfiConverterFloat64INSTANCE.Write(writer, value.Upper)
	FfiConverterFloat64INSTANCE.Write(writer, value.Middle)
	FfiConverterFloat64INSTANCE.Write(writer, value.Lower)
}

type FfiDestroyerBbDataPoint struct{}

func (_ FfiDestroyerBbDataPoint) Destroy(value BbDataPoint) {
	value.Destroy()
}

// Bollinger Bands response
type BbResponse struct {
	Symbol    string
	DataType  string
	Exchange  string
	Market    string
	Timeframe string
	Period    uint32
	Stddev    float64
	Data      []BbDataPoint
}

func (r *BbResponse) Destroy() {
	FfiDestroyerString{}.Destroy(r.Symbol)
	FfiDestroyerString{}.Destroy(r.DataType)
	FfiDestroyerString{}.Destroy(r.Exchange)
	FfiDestroyerString{}.Destroy(r.Market)
	FfiDestroyerString{}.Destroy(r.Timeframe)
	FfiDestroyerUint32{}.Destroy(r.Period)
	FfiDestroyerFloat64{}.Destroy(r.Stddev)
	FfiDestroyerSequenceBbDataPoint{}.Destroy(r.Data)
}

type FfiConverterBbResponse struct{}

var FfiConverterBbResponseINSTANCE = FfiConverterBbResponse{}

func (c FfiConverterBbResponse) Lift(rb RustBufferI) BbResponse {
	return LiftFromRustBuffer[BbResponse](c, rb)
}

func (c FfiConverterBbResponse) Read(reader io.Reader) BbResponse {
	return BbResponse{
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterUint32INSTANCE.Read(reader),
		FfiConverterFloat64INSTANCE.Read(reader),
		FfiConverterSequenceBbDataPointINSTANCE.Read(reader),
	}
}

func (c FfiConverterBbResponse) Lower(value BbResponse) C.RustBuffer {
	return LowerIntoRustBuffer[BbResponse](c, value)
}

func (c FfiConverterBbResponse) Write(writer io.Writer, value BbResponse) {
	FfiConverterStringINSTANCE.Write(writer, value.Symbol)
	FfiConverterStringINSTANCE.Write(writer, value.DataType)
	FfiConverterStringINSTANCE.Write(writer, value.Exchange)
	FfiConverterStringINSTANCE.Write(writer, value.Market)
	FfiConverterStringINSTANCE.Write(writer, value.Timeframe)
	FfiConverterUint32INSTANCE.Write(writer, value.Period)
	FfiConverterFloat64INSTANCE.Write(writer, value.Stddev)
	FfiConverterSequenceBbDataPointINSTANCE.Write(writer, value.Data)
}

type FfiDestroyerBbResponse struct{}

func (_ FfiDestroyerBbResponse) Destroy(value BbResponse) {
	value.Destroy()
}

// Capital change entry
type CapitalChange struct {
	Symbol          string
	Name            *string
	Date            string
	PreviousCapital *float64
	CurrentCapital  *float64
	ChangeType      *string
	Reason          *string
}

func (r *CapitalChange) Destroy() {
	FfiDestroyerString{}.Destroy(r.Symbol)
	FfiDestroyerOptionalString{}.Destroy(r.Name)
	FfiDestroyerString{}.Destroy(r.Date)
	FfiDestroyerOptionalFloat64{}.Destroy(r.PreviousCapital)
	FfiDestroyerOptionalFloat64{}.Destroy(r.CurrentCapital)
	FfiDestroyerOptionalString{}.Destroy(r.ChangeType)
	FfiDestroyerOptionalString{}.Destroy(r.Reason)
}

type FfiConverterCapitalChange struct{}

var FfiConverterCapitalChangeINSTANCE = FfiConverterCapitalChange{}

func (c FfiConverterCapitalChange) Lift(rb RustBufferI) CapitalChange {
	return LiftFromRustBuffer[CapitalChange](c, rb)
}

func (c FfiConverterCapitalChange) Read(reader io.Reader) CapitalChange {
	return CapitalChange{
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
	}
}

func (c FfiConverterCapitalChange) Lower(value CapitalChange) C.RustBuffer {
	return LowerIntoRustBuffer[CapitalChange](c, value)
}

func (c FfiConverterCapitalChange) Write(writer io.Writer, value CapitalChange) {
	FfiConverterStringINSTANCE.Write(writer, value.Symbol)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Name)
	FfiConverterStringINSTANCE.Write(writer, value.Date)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.PreviousCapital)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.CurrentCapital)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.ChangeType)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Reason)
}

type FfiDestroyerCapitalChange struct{}

func (_ FfiDestroyerCapitalChange) Destroy(value CapitalChange) {
	value.Destroy()
}

// Capital changes response
type CapitalChangesResponse struct {
	DataType string
	Exchange string
	Market   string
	Data     []CapitalChange
}

func (r *CapitalChangesResponse) Destroy() {
	FfiDestroyerString{}.Destroy(r.DataType)
	FfiDestroyerString{}.Destroy(r.Exchange)
	FfiDestroyerString{}.Destroy(r.Market)
	FfiDestroyerSequenceCapitalChange{}.Destroy(r.Data)
}

type FfiConverterCapitalChangesResponse struct{}

var FfiConverterCapitalChangesResponseINSTANCE = FfiConverterCapitalChangesResponse{}

func (c FfiConverterCapitalChangesResponse) Lift(rb RustBufferI) CapitalChangesResponse {
	return LiftFromRustBuffer[CapitalChangesResponse](c, rb)
}

func (c FfiConverterCapitalChangesResponse) Read(reader io.Reader) CapitalChangesResponse {
	return CapitalChangesResponse{
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterSequenceCapitalChangeINSTANCE.Read(reader),
	}
}

func (c FfiConverterCapitalChangesResponse) Lower(value CapitalChangesResponse) C.RustBuffer {
	return LowerIntoRustBuffer[CapitalChangesResponse](c, value)
}

func (c FfiConverterCapitalChangesResponse) Write(writer io.Writer, value CapitalChangesResponse) {
	FfiConverterStringINSTANCE.Write(writer, value.DataType)
	FfiConverterStringINSTANCE.Write(writer, value.Exchange)
	FfiConverterStringINSTANCE.Write(writer, value.Market)
	FfiConverterSequenceCapitalChangeINSTANCE.Write(writer, value.Data)
}

type FfiDestroyerCapitalChangesResponse struct{}

func (_ FfiDestroyerCapitalChangesResponse) Destroy(value CapitalChangesResponse) {
	value.Destroy()
}

// Dividend entry
type Dividend struct {
	Symbol         string
	Name           *string
	ExDividendDate *string
	PaymentDate    *string
	CashDividend   *float64
	StockDividend  *float64
	DividendYear   *string
}

func (r *Dividend) Destroy() {
	FfiDestroyerString{}.Destroy(r.Symbol)
	FfiDestroyerOptionalString{}.Destroy(r.Name)
	FfiDestroyerOptionalString{}.Destroy(r.ExDividendDate)
	FfiDestroyerOptionalString{}.Destroy(r.PaymentDate)
	FfiDestroyerOptionalFloat64{}.Destroy(r.CashDividend)
	FfiDestroyerOptionalFloat64{}.Destroy(r.StockDividend)
	FfiDestroyerOptionalString{}.Destroy(r.DividendYear)
}

type FfiConverterDividend struct{}

var FfiConverterDividendINSTANCE = FfiConverterDividend{}

func (c FfiConverterDividend) Lift(rb RustBufferI) Dividend {
	return LiftFromRustBuffer[Dividend](c, rb)
}

func (c FfiConverterDividend) Read(reader io.Reader) Dividend {
	return Dividend{
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
	}
}

func (c FfiConverterDividend) Lower(value Dividend) C.RustBuffer {
	return LowerIntoRustBuffer[Dividend](c, value)
}

func (c FfiConverterDividend) Write(writer io.Writer, value Dividend) {
	FfiConverterStringINSTANCE.Write(writer, value.Symbol)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Name)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.ExDividendDate)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.PaymentDate)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.CashDividend)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.StockDividend)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.DividendYear)
}

type FfiDestroyerDividend struct{}

func (_ FfiDestroyerDividend) Destroy(value Dividend) {
	value.Destroy()
}

// Dividends response
type DividendsResponse struct {
	DataType string
	Exchange string
	Market   string
	Data     []Dividend
}

func (r *DividendsResponse) Destroy() {
	FfiDestroyerString{}.Destroy(r.DataType)
	FfiDestroyerString{}.Destroy(r.Exchange)
	FfiDestroyerString{}.Destroy(r.Market)
	FfiDestroyerSequenceDividend{}.Destroy(r.Data)
}

type FfiConverterDividendsResponse struct{}

var FfiConverterDividendsResponseINSTANCE = FfiConverterDividendsResponse{}

func (c FfiConverterDividendsResponse) Lift(rb RustBufferI) DividendsResponse {
	return LiftFromRustBuffer[DividendsResponse](c, rb)
}

func (c FfiConverterDividendsResponse) Read(reader io.Reader) DividendsResponse {
	return DividendsResponse{
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterSequenceDividendINSTANCE.Read(reader),
	}
}

func (c FfiConverterDividendsResponse) Lower(value DividendsResponse) C.RustBuffer {
	return LowerIntoRustBuffer[DividendsResponse](c, value)
}

func (c FfiConverterDividendsResponse) Write(writer io.Writer, value DividendsResponse) {
	FfiConverterStringINSTANCE.Write(writer, value.DataType)
	FfiConverterStringINSTANCE.Write(writer, value.Exchange)
	FfiConverterStringINSTANCE.Write(writer, value.Market)
	FfiConverterSequenceDividendINSTANCE.Write(writer, value.Data)
}

type FfiDestroyerDividendsResponse struct{}

func (_ FfiDestroyerDividendsResponse) Destroy(value DividendsResponse) {
	value.Destroy()
}

// FutOpt daily data
type FutOptDailyData struct {
	Date            string
	Open            float64
	High            float64
	Low             float64
	Close           float64
	Volume          uint64
	OpenInterest    *uint64
	SettlementPrice *float64
}

func (r *FutOptDailyData) Destroy() {
	FfiDestroyerString{}.Destroy(r.Date)
	FfiDestroyerFloat64{}.Destroy(r.Open)
	FfiDestroyerFloat64{}.Destroy(r.High)
	FfiDestroyerFloat64{}.Destroy(r.Low)
	FfiDestroyerFloat64{}.Destroy(r.Close)
	FfiDestroyerUint64{}.Destroy(r.Volume)
	FfiDestroyerOptionalUint64{}.Destroy(r.OpenInterest)
	FfiDestroyerOptionalFloat64{}.Destroy(r.SettlementPrice)
}

type FfiConverterFutOptDailyData struct{}

var FfiConverterFutOptDailyDataINSTANCE = FfiConverterFutOptDailyData{}

func (c FfiConverterFutOptDailyData) Lift(rb RustBufferI) FutOptDailyData {
	return LiftFromRustBuffer[FutOptDailyData](c, rb)
}

func (c FfiConverterFutOptDailyData) Read(reader io.Reader) FutOptDailyData {
	return FutOptDailyData{
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterFloat64INSTANCE.Read(reader),
		FfiConverterFloat64INSTANCE.Read(reader),
		FfiConverterFloat64INSTANCE.Read(reader),
		FfiConverterFloat64INSTANCE.Read(reader),
		FfiConverterUint64INSTANCE.Read(reader),
		FfiConverterOptionalUint64INSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
	}
}

func (c FfiConverterFutOptDailyData) Lower(value FutOptDailyData) C.RustBuffer {
	return LowerIntoRustBuffer[FutOptDailyData](c, value)
}

func (c FfiConverterFutOptDailyData) Write(writer io.Writer, value FutOptDailyData) {
	FfiConverterStringINSTANCE.Write(writer, value.Date)
	FfiConverterFloat64INSTANCE.Write(writer, value.Open)
	FfiConverterFloat64INSTANCE.Write(writer, value.High)
	FfiConverterFloat64INSTANCE.Write(writer, value.Low)
	FfiConverterFloat64INSTANCE.Write(writer, value.Close)
	FfiConverterUint64INSTANCE.Write(writer, value.Volume)
	FfiConverterOptionalUint64INSTANCE.Write(writer, value.OpenInterest)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.SettlementPrice)
}

type FfiDestroyerFutOptDailyData struct{}

func (_ FfiDestroyerFutOptDailyData) Destroy(value FutOptDailyData) {
	value.Destroy()
}

// FutOpt daily response
type FutOptDailyResponse struct {
	Symbol   string
	DataType *string
	Exchange *string
	Data     []FutOptDailyData
}

func (r *FutOptDailyResponse) Destroy() {
	FfiDestroyerString{}.Destroy(r.Symbol)
	FfiDestroyerOptionalString{}.Destroy(r.DataType)
	FfiDestroyerOptionalString{}.Destroy(r.Exchange)
	FfiDestroyerSequenceFutOptDailyData{}.Destroy(r.Data)
}

type FfiConverterFutOptDailyResponse struct{}

var FfiConverterFutOptDailyResponseINSTANCE = FfiConverterFutOptDailyResponse{}

func (c FfiConverterFutOptDailyResponse) Lift(rb RustBufferI) FutOptDailyResponse {
	return LiftFromRustBuffer[FutOptDailyResponse](c, rb)
}

func (c FfiConverterFutOptDailyResponse) Read(reader io.Reader) FutOptDailyResponse {
	return FutOptDailyResponse{
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterSequenceFutOptDailyDataINSTANCE.Read(reader),
	}
}

func (c FfiConverterFutOptDailyResponse) Lower(value FutOptDailyResponse) C.RustBuffer {
	return LowerIntoRustBuffer[FutOptDailyResponse](c, value)
}

func (c FfiConverterFutOptDailyResponse) Write(writer io.Writer, value FutOptDailyResponse) {
	FfiConverterStringINSTANCE.Write(writer, value.Symbol)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.DataType)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Exchange)
	FfiConverterSequenceFutOptDailyDataINSTANCE.Write(writer, value.Data)
}

type FfiDestroyerFutOptDailyResponse struct{}

func (_ FfiDestroyerFutOptDailyResponse) Destroy(value FutOptDailyResponse) {
	value.Destroy()
}

// FutOpt historical candle
type FutOptHistoricalCandle struct {
	Date          string
	Open          float64
	High          float64
	Low           float64
	Close         float64
	Volume        uint64
	OpenInterest  *uint64
	Change        *float64
	ChangePercent *float64
}

func (r *FutOptHistoricalCandle) Destroy() {
	FfiDestroyerString{}.Destroy(r.Date)
	FfiDestroyerFloat64{}.Destroy(r.Open)
	FfiDestroyerFloat64{}.Destroy(r.High)
	FfiDestroyerFloat64{}.Destroy(r.Low)
	FfiDestroyerFloat64{}.Destroy(r.Close)
	FfiDestroyerUint64{}.Destroy(r.Volume)
	FfiDestroyerOptionalUint64{}.Destroy(r.OpenInterest)
	FfiDestroyerOptionalFloat64{}.Destroy(r.Change)
	FfiDestroyerOptionalFloat64{}.Destroy(r.ChangePercent)
}

type FfiConverterFutOptHistoricalCandle struct{}

var FfiConverterFutOptHistoricalCandleINSTANCE = FfiConverterFutOptHistoricalCandle{}

func (c FfiConverterFutOptHistoricalCandle) Lift(rb RustBufferI) FutOptHistoricalCandle {
	return LiftFromRustBuffer[FutOptHistoricalCandle](c, rb)
}

func (c FfiConverterFutOptHistoricalCandle) Read(reader io.Reader) FutOptHistoricalCandle {
	return FutOptHistoricalCandle{
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterFloat64INSTANCE.Read(reader),
		FfiConverterFloat64INSTANCE.Read(reader),
		FfiConverterFloat64INSTANCE.Read(reader),
		FfiConverterFloat64INSTANCE.Read(reader),
		FfiConverterUint64INSTANCE.Read(reader),
		FfiConverterOptionalUint64INSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
	}
}

func (c FfiConverterFutOptHistoricalCandle) Lower(value FutOptHistoricalCandle) C.RustBuffer {
	return LowerIntoRustBuffer[FutOptHistoricalCandle](c, value)
}

func (c FfiConverterFutOptHistoricalCandle) Write(writer io.Writer, value FutOptHistoricalCandle) {
	FfiConverterStringINSTANCE.Write(writer, value.Date)
	FfiConverterFloat64INSTANCE.Write(writer, value.Open)
	FfiConverterFloat64INSTANCE.Write(writer, value.High)
	FfiConverterFloat64INSTANCE.Write(writer, value.Low)
	FfiConverterFloat64INSTANCE.Write(writer, value.Close)
	FfiConverterUint64INSTANCE.Write(writer, value.Volume)
	FfiConverterOptionalUint64INSTANCE.Write(writer, value.OpenInterest)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.Change)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.ChangePercent)
}

type FfiDestroyerFutOptHistoricalCandle struct{}

func (_ FfiDestroyerFutOptHistoricalCandle) Destroy(value FutOptHistoricalCandle) {
	value.Destroy()
}

// FutOpt historical candles response
type FutOptHistoricalCandlesResponse struct {
	Symbol    string
	DataType  *string
	Exchange  *string
	Timeframe *string
	Candles   []FutOptHistoricalCandle
}

func (r *FutOptHistoricalCandlesResponse) Destroy() {
	FfiDestroyerString{}.Destroy(r.Symbol)
	FfiDestroyerOptionalString{}.Destroy(r.DataType)
	FfiDestroyerOptionalString{}.Destroy(r.Exchange)
	FfiDestroyerOptionalString{}.Destroy(r.Timeframe)
	FfiDestroyerSequenceFutOptHistoricalCandle{}.Destroy(r.Candles)
}

type FfiConverterFutOptHistoricalCandlesResponse struct{}

var FfiConverterFutOptHistoricalCandlesResponseINSTANCE = FfiConverterFutOptHistoricalCandlesResponse{}

func (c FfiConverterFutOptHistoricalCandlesResponse) Lift(rb RustBufferI) FutOptHistoricalCandlesResponse {
	return LiftFromRustBuffer[FutOptHistoricalCandlesResponse](c, rb)
}

func (c FfiConverterFutOptHistoricalCandlesResponse) Read(reader io.Reader) FutOptHistoricalCandlesResponse {
	return FutOptHistoricalCandlesResponse{
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterSequenceFutOptHistoricalCandleINSTANCE.Read(reader),
	}
}

func (c FfiConverterFutOptHistoricalCandlesResponse) Lower(value FutOptHistoricalCandlesResponse) C.RustBuffer {
	return LowerIntoRustBuffer[FutOptHistoricalCandlesResponse](c, value)
}

func (c FfiConverterFutOptHistoricalCandlesResponse) Write(writer io.Writer, value FutOptHistoricalCandlesResponse) {
	FfiConverterStringINSTANCE.Write(writer, value.Symbol)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.DataType)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Exchange)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Timeframe)
	FfiConverterSequenceFutOptHistoricalCandleINSTANCE.Write(writer, value.Candles)
}

type FfiDestroyerFutOptHistoricalCandlesResponse struct{}

func (_ FfiDestroyerFutOptHistoricalCandlesResponse) Destroy(value FutOptHistoricalCandlesResponse) {
	value.Destroy()
}

// FutOpt last trade info
type FutOptLastTrade struct {
	Price float64
	Size  int64
	Time  int64
}

func (r *FutOptLastTrade) Destroy() {
	FfiDestroyerFloat64{}.Destroy(r.Price)
	FfiDestroyerInt64{}.Destroy(r.Size)
	FfiDestroyerInt64{}.Destroy(r.Time)
}

type FfiConverterFutOptLastTrade struct{}

var FfiConverterFutOptLastTradeINSTANCE = FfiConverterFutOptLastTrade{}

func (c FfiConverterFutOptLastTrade) Lift(rb RustBufferI) FutOptLastTrade {
	return LiftFromRustBuffer[FutOptLastTrade](c, rb)
}

func (c FfiConverterFutOptLastTrade) Read(reader io.Reader) FutOptLastTrade {
	return FutOptLastTrade{
		FfiConverterFloat64INSTANCE.Read(reader),
		FfiConverterInt64INSTANCE.Read(reader),
		FfiConverterInt64INSTANCE.Read(reader),
	}
}

func (c FfiConverterFutOptLastTrade) Lower(value FutOptLastTrade) C.RustBuffer {
	return LowerIntoRustBuffer[FutOptLastTrade](c, value)
}

func (c FfiConverterFutOptLastTrade) Write(writer io.Writer, value FutOptLastTrade) {
	FfiConverterFloat64INSTANCE.Write(writer, value.Price)
	FfiConverterInt64INSTANCE.Write(writer, value.Size)
	FfiConverterInt64INSTANCE.Write(writer, value.Time)
}

type FfiDestroyerFutOptLastTrade struct{}

func (_ FfiDestroyerFutOptLastTrade) Destroy(value FutOptLastTrade) {
	value.Destroy()
}

// FutOpt price level
type FutOptPriceLevel struct {
	Price float64
	Size  int64
}

func (r *FutOptPriceLevel) Destroy() {
	FfiDestroyerFloat64{}.Destroy(r.Price)
	FfiDestroyerInt64{}.Destroy(r.Size)
}

type FfiConverterFutOptPriceLevel struct{}

var FfiConverterFutOptPriceLevelINSTANCE = FfiConverterFutOptPriceLevel{}

func (c FfiConverterFutOptPriceLevel) Lift(rb RustBufferI) FutOptPriceLevel {
	return LiftFromRustBuffer[FutOptPriceLevel](c, rb)
}

func (c FfiConverterFutOptPriceLevel) Read(reader io.Reader) FutOptPriceLevel {
	return FutOptPriceLevel{
		FfiConverterFloat64INSTANCE.Read(reader),
		FfiConverterInt64INSTANCE.Read(reader),
	}
}

func (c FfiConverterFutOptPriceLevel) Lower(value FutOptPriceLevel) C.RustBuffer {
	return LowerIntoRustBuffer[FutOptPriceLevel](c, value)
}

func (c FfiConverterFutOptPriceLevel) Write(writer io.Writer, value FutOptPriceLevel) {
	FfiConverterFloat64INSTANCE.Write(writer, value.Price)
	FfiConverterInt64INSTANCE.Write(writer, value.Size)
}

type FfiDestroyerFutOptPriceLevel struct{}

func (_ FfiDestroyerFutOptPriceLevel) Destroy(value FutOptPriceLevel) {
	value.Destroy()
}

// FutOpt quote
type FutOptQuote struct {
	Date          string
	ContractType  *string
	Exchange      *string
	Symbol        string
	Name          *string
	PreviousClose *float64
	OpenPrice     *float64
	OpenTime      *int64
	HighPrice     *float64
	HighTime      *int64
	LowPrice      *float64
	LowTime       *int64
	ClosePrice    *float64
	CloseTime     *int64
	LastPrice     *float64
	LastSize      *int64
	AvgPrice      *float64
	Change        *float64
	ChangePercent *float64
	Amplitude     *float64
	Bids          []FutOptPriceLevel
	Asks          []FutOptPriceLevel
	Total         *FutOptTotalStats
	LastTrade     *FutOptLastTrade
	LastUpdated   *int64
}

func (r *FutOptQuote) Destroy() {
	FfiDestroyerString{}.Destroy(r.Date)
	FfiDestroyerOptionalString{}.Destroy(r.ContractType)
	FfiDestroyerOptionalString{}.Destroy(r.Exchange)
	FfiDestroyerString{}.Destroy(r.Symbol)
	FfiDestroyerOptionalString{}.Destroy(r.Name)
	FfiDestroyerOptionalFloat64{}.Destroy(r.PreviousClose)
	FfiDestroyerOptionalFloat64{}.Destroy(r.OpenPrice)
	FfiDestroyerOptionalInt64{}.Destroy(r.OpenTime)
	FfiDestroyerOptionalFloat64{}.Destroy(r.HighPrice)
	FfiDestroyerOptionalInt64{}.Destroy(r.HighTime)
	FfiDestroyerOptionalFloat64{}.Destroy(r.LowPrice)
	FfiDestroyerOptionalInt64{}.Destroy(r.LowTime)
	FfiDestroyerOptionalFloat64{}.Destroy(r.ClosePrice)
	FfiDestroyerOptionalInt64{}.Destroy(r.CloseTime)
	FfiDestroyerOptionalFloat64{}.Destroy(r.LastPrice)
	FfiDestroyerOptionalInt64{}.Destroy(r.LastSize)
	FfiDestroyerOptionalFloat64{}.Destroy(r.AvgPrice)
	FfiDestroyerOptionalFloat64{}.Destroy(r.Change)
	FfiDestroyerOptionalFloat64{}.Destroy(r.ChangePercent)
	FfiDestroyerOptionalFloat64{}.Destroy(r.Amplitude)
	FfiDestroyerSequenceFutOptPriceLevel{}.Destroy(r.Bids)
	FfiDestroyerSequenceFutOptPriceLevel{}.Destroy(r.Asks)
	FfiDestroyerOptionalFutOptTotalStats{}.Destroy(r.Total)
	FfiDestroyerOptionalFutOptLastTrade{}.Destroy(r.LastTrade)
	FfiDestroyerOptionalInt64{}.Destroy(r.LastUpdated)
}

type FfiConverterFutOptQuote struct{}

var FfiConverterFutOptQuoteINSTANCE = FfiConverterFutOptQuote{}

func (c FfiConverterFutOptQuote) Lift(rb RustBufferI) FutOptQuote {
	return LiftFromRustBuffer[FutOptQuote](c, rb)
}

func (c FfiConverterFutOptQuote) Read(reader io.Reader) FutOptQuote {
	return FutOptQuote{
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterOptionalInt64INSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterOptionalInt64INSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterOptionalInt64INSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterOptionalInt64INSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterOptionalInt64INSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterSequenceFutOptPriceLevelINSTANCE.Read(reader),
		FfiConverterSequenceFutOptPriceLevelINSTANCE.Read(reader),
		FfiConverterOptionalFutOptTotalStatsINSTANCE.Read(reader),
		FfiConverterOptionalFutOptLastTradeINSTANCE.Read(reader),
		FfiConverterOptionalInt64INSTANCE.Read(reader),
	}
}

func (c FfiConverterFutOptQuote) Lower(value FutOptQuote) C.RustBuffer {
	return LowerIntoRustBuffer[FutOptQuote](c, value)
}

func (c FfiConverterFutOptQuote) Write(writer io.Writer, value FutOptQuote) {
	FfiConverterStringINSTANCE.Write(writer, value.Date)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.ContractType)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Exchange)
	FfiConverterStringINSTANCE.Write(writer, value.Symbol)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Name)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.PreviousClose)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.OpenPrice)
	FfiConverterOptionalInt64INSTANCE.Write(writer, value.OpenTime)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.HighPrice)
	FfiConverterOptionalInt64INSTANCE.Write(writer, value.HighTime)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.LowPrice)
	FfiConverterOptionalInt64INSTANCE.Write(writer, value.LowTime)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.ClosePrice)
	FfiConverterOptionalInt64INSTANCE.Write(writer, value.CloseTime)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.LastPrice)
	FfiConverterOptionalInt64INSTANCE.Write(writer, value.LastSize)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.AvgPrice)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.Change)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.ChangePercent)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.Amplitude)
	FfiConverterSequenceFutOptPriceLevelINSTANCE.Write(writer, value.Bids)
	FfiConverterSequenceFutOptPriceLevelINSTANCE.Write(writer, value.Asks)
	FfiConverterOptionalFutOptTotalStatsINSTANCE.Write(writer, value.Total)
	FfiConverterOptionalFutOptLastTradeINSTANCE.Write(writer, value.LastTrade)
	FfiConverterOptionalInt64INSTANCE.Write(writer, value.LastUpdated)
}

type FfiDestroyerFutOptQuote struct{}

func (_ FfiDestroyerFutOptQuote) Destroy(value FutOptQuote) {
	value.Destroy()
}

// FutOpt ticker
type FutOptTicker struct {
	Date             string
	ContractType     *string
	Exchange         *string
	Symbol           string
	Name             *string
	ReferencePrice   *float64
	StartDate        *string
	EndDate          *string
	SettlementDate   *string
	ContractSubType  *string
	IsDynamicBanding bool
	FlowGroup        *int32
}

func (r *FutOptTicker) Destroy() {
	FfiDestroyerString{}.Destroy(r.Date)
	FfiDestroyerOptionalString{}.Destroy(r.ContractType)
	FfiDestroyerOptionalString{}.Destroy(r.Exchange)
	FfiDestroyerString{}.Destroy(r.Symbol)
	FfiDestroyerOptionalString{}.Destroy(r.Name)
	FfiDestroyerOptionalFloat64{}.Destroy(r.ReferencePrice)
	FfiDestroyerOptionalString{}.Destroy(r.StartDate)
	FfiDestroyerOptionalString{}.Destroy(r.EndDate)
	FfiDestroyerOptionalString{}.Destroy(r.SettlementDate)
	FfiDestroyerOptionalString{}.Destroy(r.ContractSubType)
	FfiDestroyerBool{}.Destroy(r.IsDynamicBanding)
	FfiDestroyerOptionalInt32{}.Destroy(r.FlowGroup)
}

type FfiConverterFutOptTicker struct{}

var FfiConverterFutOptTickerINSTANCE = FfiConverterFutOptTicker{}

func (c FfiConverterFutOptTicker) Lift(rb RustBufferI) FutOptTicker {
	return LiftFromRustBuffer[FutOptTicker](c, rb)
}

func (c FfiConverterFutOptTicker) Read(reader io.Reader) FutOptTicker {
	return FutOptTicker{
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterBoolINSTANCE.Read(reader),
		FfiConverterOptionalInt32INSTANCE.Read(reader),
	}
}

func (c FfiConverterFutOptTicker) Lower(value FutOptTicker) C.RustBuffer {
	return LowerIntoRustBuffer[FutOptTicker](c, value)
}

func (c FfiConverterFutOptTicker) Write(writer io.Writer, value FutOptTicker) {
	FfiConverterStringINSTANCE.Write(writer, value.Date)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.ContractType)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Exchange)
	FfiConverterStringINSTANCE.Write(writer, value.Symbol)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Name)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.ReferencePrice)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.StartDate)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.EndDate)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.SettlementDate)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.ContractSubType)
	FfiConverterBoolINSTANCE.Write(writer, value.IsDynamicBanding)
	FfiConverterOptionalInt32INSTANCE.Write(writer, value.FlowGroup)
}

type FfiDestroyerFutOptTicker struct{}

func (_ FfiDestroyerFutOptTicker) Destroy(value FutOptTicker) {
	value.Destroy()
}

// FutOpt total stats
type FutOptTotalStats struct {
	TradeVolume   int64
	TotalBidMatch *int64
	TotalAskMatch *int64
}

func (r *FutOptTotalStats) Destroy() {
	FfiDestroyerInt64{}.Destroy(r.TradeVolume)
	FfiDestroyerOptionalInt64{}.Destroy(r.TotalBidMatch)
	FfiDestroyerOptionalInt64{}.Destroy(r.TotalAskMatch)
}

type FfiConverterFutOptTotalStats struct{}

var FfiConverterFutOptTotalStatsINSTANCE = FfiConverterFutOptTotalStats{}

func (c FfiConverterFutOptTotalStats) Lift(rb RustBufferI) FutOptTotalStats {
	return LiftFromRustBuffer[FutOptTotalStats](c, rb)
}

func (c FfiConverterFutOptTotalStats) Read(reader io.Reader) FutOptTotalStats {
	return FutOptTotalStats{
		FfiConverterInt64INSTANCE.Read(reader),
		FfiConverterOptionalInt64INSTANCE.Read(reader),
		FfiConverterOptionalInt64INSTANCE.Read(reader),
	}
}

func (c FfiConverterFutOptTotalStats) Lower(value FutOptTotalStats) C.RustBuffer {
	return LowerIntoRustBuffer[FutOptTotalStats](c, value)
}

func (c FfiConverterFutOptTotalStats) Write(writer io.Writer, value FutOptTotalStats) {
	FfiConverterInt64INSTANCE.Write(writer, value.TradeVolume)
	FfiConverterOptionalInt64INSTANCE.Write(writer, value.TotalBidMatch)
	FfiConverterOptionalInt64INSTANCE.Write(writer, value.TotalAskMatch)
}

type FfiDestroyerFutOptTotalStats struct{}

func (_ FfiDestroyerFutOptTotalStats) Destroy(value FutOptTotalStats) {
	value.Destroy()
}

// Health check configuration record for FFI
//
// All fields are optional — zero/false values mean "use default".
type HealthCheckConfigRecord struct {
	// Whether health check is enabled (default: false)
	Enabled bool
	// Interval between ping messages in milliseconds (default: 30000, min: 5000)
	IntervalMs uint64
	// Maximum missed pongs before disconnect (default: 2, min: 1)
	MaxMissedPongs uint64
}

func (r *HealthCheckConfigRecord) Destroy() {
	FfiDestroyerBool{}.Destroy(r.Enabled)
	FfiDestroyerUint64{}.Destroy(r.IntervalMs)
	FfiDestroyerUint64{}.Destroy(r.MaxMissedPongs)
}

type FfiConverterHealthCheckConfigRecord struct{}

var FfiConverterHealthCheckConfigRecordINSTANCE = FfiConverterHealthCheckConfigRecord{}

func (c FfiConverterHealthCheckConfigRecord) Lift(rb RustBufferI) HealthCheckConfigRecord {
	return LiftFromRustBuffer[HealthCheckConfigRecord](c, rb)
}

func (c FfiConverterHealthCheckConfigRecord) Read(reader io.Reader) HealthCheckConfigRecord {
	return HealthCheckConfigRecord{
		FfiConverterBoolINSTANCE.Read(reader),
		FfiConverterUint64INSTANCE.Read(reader),
		FfiConverterUint64INSTANCE.Read(reader),
	}
}

func (c FfiConverterHealthCheckConfigRecord) Lower(value HealthCheckConfigRecord) C.RustBuffer {
	return LowerIntoRustBuffer[HealthCheckConfigRecord](c, value)
}

func (c FfiConverterHealthCheckConfigRecord) Write(writer io.Writer, value HealthCheckConfigRecord) {
	FfiConverterBoolINSTANCE.Write(writer, value.Enabled)
	FfiConverterUint64INSTANCE.Write(writer, value.IntervalMs)
	FfiConverterUint64INSTANCE.Write(writer, value.MaxMissedPongs)
}

type FfiDestroyerHealthCheckConfigRecord struct{}

func (_ FfiDestroyerHealthCheckConfigRecord) Destroy(value HealthCheckConfigRecord) {
	value.Destroy()
}

// Single historical candle
type HistoricalCandle struct {
	Date     string
	Open     float64
	High     float64
	Low      float64
	Close    float64
	Volume   int64
	Turnover *float64
	Change   *float64
}

func (r *HistoricalCandle) Destroy() {
	FfiDestroyerString{}.Destroy(r.Date)
	FfiDestroyerFloat64{}.Destroy(r.Open)
	FfiDestroyerFloat64{}.Destroy(r.High)
	FfiDestroyerFloat64{}.Destroy(r.Low)
	FfiDestroyerFloat64{}.Destroy(r.Close)
	FfiDestroyerInt64{}.Destroy(r.Volume)
	FfiDestroyerOptionalFloat64{}.Destroy(r.Turnover)
	FfiDestroyerOptionalFloat64{}.Destroy(r.Change)
}

type FfiConverterHistoricalCandle struct{}

var FfiConverterHistoricalCandleINSTANCE = FfiConverterHistoricalCandle{}

func (c FfiConverterHistoricalCandle) Lift(rb RustBufferI) HistoricalCandle {
	return LiftFromRustBuffer[HistoricalCandle](c, rb)
}

func (c FfiConverterHistoricalCandle) Read(reader io.Reader) HistoricalCandle {
	return HistoricalCandle{
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterFloat64INSTANCE.Read(reader),
		FfiConverterFloat64INSTANCE.Read(reader),
		FfiConverterFloat64INSTANCE.Read(reader),
		FfiConverterFloat64INSTANCE.Read(reader),
		FfiConverterInt64INSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
	}
}

func (c FfiConverterHistoricalCandle) Lower(value HistoricalCandle) C.RustBuffer {
	return LowerIntoRustBuffer[HistoricalCandle](c, value)
}

func (c FfiConverterHistoricalCandle) Write(writer io.Writer, value HistoricalCandle) {
	FfiConverterStringINSTANCE.Write(writer, value.Date)
	FfiConverterFloat64INSTANCE.Write(writer, value.Open)
	FfiConverterFloat64INSTANCE.Write(writer, value.High)
	FfiConverterFloat64INSTANCE.Write(writer, value.Low)
	FfiConverterFloat64INSTANCE.Write(writer, value.Close)
	FfiConverterInt64INSTANCE.Write(writer, value.Volume)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.Turnover)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.Change)
}

type FfiDestroyerHistoricalCandle struct{}

func (_ FfiDestroyerHistoricalCandle) Destroy(value HistoricalCandle) {
	value.Destroy()
}

// Historical candles response
type HistoricalCandlesResponse struct {
	Symbol    string
	DataType  *string
	Exchange  *string
	Market    *string
	Timeframe *string
	Adjusted  *bool
	Data      []HistoricalCandle
}

func (r *HistoricalCandlesResponse) Destroy() {
	FfiDestroyerString{}.Destroy(r.Symbol)
	FfiDestroyerOptionalString{}.Destroy(r.DataType)
	FfiDestroyerOptionalString{}.Destroy(r.Exchange)
	FfiDestroyerOptionalString{}.Destroy(r.Market)
	FfiDestroyerOptionalString{}.Destroy(r.Timeframe)
	FfiDestroyerOptionalBool{}.Destroy(r.Adjusted)
	FfiDestroyerSequenceHistoricalCandle{}.Destroy(r.Data)
}

type FfiConverterHistoricalCandlesResponse struct{}

var FfiConverterHistoricalCandlesResponseINSTANCE = FfiConverterHistoricalCandlesResponse{}

func (c FfiConverterHistoricalCandlesResponse) Lift(rb RustBufferI) HistoricalCandlesResponse {
	return LiftFromRustBuffer[HistoricalCandlesResponse](c, rb)
}

func (c FfiConverterHistoricalCandlesResponse) Read(reader io.Reader) HistoricalCandlesResponse {
	return HistoricalCandlesResponse{
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalBoolINSTANCE.Read(reader),
		FfiConverterSequenceHistoricalCandleINSTANCE.Read(reader),
	}
}

func (c FfiConverterHistoricalCandlesResponse) Lower(value HistoricalCandlesResponse) C.RustBuffer {
	return LowerIntoRustBuffer[HistoricalCandlesResponse](c, value)
}

func (c FfiConverterHistoricalCandlesResponse) Write(writer io.Writer, value HistoricalCandlesResponse) {
	FfiConverterStringINSTANCE.Write(writer, value.Symbol)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.DataType)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Exchange)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Market)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Timeframe)
	FfiConverterOptionalBoolINSTANCE.Write(writer, value.Adjusted)
	FfiConverterSequenceHistoricalCandleINSTANCE.Write(writer, value.Data)
}

type FfiDestroyerHistoricalCandlesResponse struct{}

func (_ FfiDestroyerHistoricalCandlesResponse) Destroy(value HistoricalCandlesResponse) {
	value.Destroy()
}

// Single intraday candle
type IntradayCandle struct {
	Open    float64
	High    float64
	Low     float64
	Close   float64
	Volume  int64
	Average *float64
	Date    string
}

func (r *IntradayCandle) Destroy() {
	FfiDestroyerFloat64{}.Destroy(r.Open)
	FfiDestroyerFloat64{}.Destroy(r.High)
	FfiDestroyerFloat64{}.Destroy(r.Low)
	FfiDestroyerFloat64{}.Destroy(r.Close)
	FfiDestroyerInt64{}.Destroy(r.Volume)
	FfiDestroyerOptionalFloat64{}.Destroy(r.Average)
	FfiDestroyerString{}.Destroy(r.Date)
}

type FfiConverterIntradayCandle struct{}

var FfiConverterIntradayCandleINSTANCE = FfiConverterIntradayCandle{}

func (c FfiConverterIntradayCandle) Lift(rb RustBufferI) IntradayCandle {
	return LiftFromRustBuffer[IntradayCandle](c, rb)
}

func (c FfiConverterIntradayCandle) Read(reader io.Reader) IntradayCandle {
	return IntradayCandle{
		FfiConverterFloat64INSTANCE.Read(reader),
		FfiConverterFloat64INSTANCE.Read(reader),
		FfiConverterFloat64INSTANCE.Read(reader),
		FfiConverterFloat64INSTANCE.Read(reader),
		FfiConverterInt64INSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterStringINSTANCE.Read(reader),
	}
}

func (c FfiConverterIntradayCandle) Lower(value IntradayCandle) C.RustBuffer {
	return LowerIntoRustBuffer[IntradayCandle](c, value)
}

func (c FfiConverterIntradayCandle) Write(writer io.Writer, value IntradayCandle) {
	FfiConverterFloat64INSTANCE.Write(writer, value.Open)
	FfiConverterFloat64INSTANCE.Write(writer, value.High)
	FfiConverterFloat64INSTANCE.Write(writer, value.Low)
	FfiConverterFloat64INSTANCE.Write(writer, value.Close)
	FfiConverterInt64INSTANCE.Write(writer, value.Volume)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.Average)
	FfiConverterStringINSTANCE.Write(writer, value.Date)
}

type FfiDestroyerIntradayCandle struct{}

func (_ FfiDestroyerIntradayCandle) Destroy(value IntradayCandle) {
	value.Destroy()
}

// Intraday candles response
type IntradayCandlesResponse struct {
	Date      string
	DataType  *string
	Exchange  *string
	Market    *string
	Symbol    string
	Timeframe *string
	Data      []IntradayCandle
}

func (r *IntradayCandlesResponse) Destroy() {
	FfiDestroyerString{}.Destroy(r.Date)
	FfiDestroyerOptionalString{}.Destroy(r.DataType)
	FfiDestroyerOptionalString{}.Destroy(r.Exchange)
	FfiDestroyerOptionalString{}.Destroy(r.Market)
	FfiDestroyerString{}.Destroy(r.Symbol)
	FfiDestroyerOptionalString{}.Destroy(r.Timeframe)
	FfiDestroyerSequenceIntradayCandle{}.Destroy(r.Data)
}

type FfiConverterIntradayCandlesResponse struct{}

var FfiConverterIntradayCandlesResponseINSTANCE = FfiConverterIntradayCandlesResponse{}

func (c FfiConverterIntradayCandlesResponse) Lift(rb RustBufferI) IntradayCandlesResponse {
	return LiftFromRustBuffer[IntradayCandlesResponse](c, rb)
}

func (c FfiConverterIntradayCandlesResponse) Read(reader io.Reader) IntradayCandlesResponse {
	return IntradayCandlesResponse{
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterSequenceIntradayCandleINSTANCE.Read(reader),
	}
}

func (c FfiConverterIntradayCandlesResponse) Lower(value IntradayCandlesResponse) C.RustBuffer {
	return LowerIntoRustBuffer[IntradayCandlesResponse](c, value)
}

func (c FfiConverterIntradayCandlesResponse) Write(writer io.Writer, value IntradayCandlesResponse) {
	FfiConverterStringINSTANCE.Write(writer, value.Date)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.DataType)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Exchange)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Market)
	FfiConverterStringINSTANCE.Write(writer, value.Symbol)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Timeframe)
	FfiConverterSequenceIntradayCandleINSTANCE.Write(writer, value.Data)
}

type FfiDestroyerIntradayCandlesResponse struct{}

func (_ FfiDestroyerIntradayCandlesResponse) Destroy(value IntradayCandlesResponse) {
	value.Destroy()
}

// KDJ data point
type KdjDataPoint struct {
	Date string
	K    float64
	D    float64
	J    float64
}

func (r *KdjDataPoint) Destroy() {
	FfiDestroyerString{}.Destroy(r.Date)
	FfiDestroyerFloat64{}.Destroy(r.K)
	FfiDestroyerFloat64{}.Destroy(r.D)
	FfiDestroyerFloat64{}.Destroy(r.J)
}

type FfiConverterKdjDataPoint struct{}

var FfiConverterKdjDataPointINSTANCE = FfiConverterKdjDataPoint{}

func (c FfiConverterKdjDataPoint) Lift(rb RustBufferI) KdjDataPoint {
	return LiftFromRustBuffer[KdjDataPoint](c, rb)
}

func (c FfiConverterKdjDataPoint) Read(reader io.Reader) KdjDataPoint {
	return KdjDataPoint{
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterFloat64INSTANCE.Read(reader),
		FfiConverterFloat64INSTANCE.Read(reader),
		FfiConverterFloat64INSTANCE.Read(reader),
	}
}

func (c FfiConverterKdjDataPoint) Lower(value KdjDataPoint) C.RustBuffer {
	return LowerIntoRustBuffer[KdjDataPoint](c, value)
}

func (c FfiConverterKdjDataPoint) Write(writer io.Writer, value KdjDataPoint) {
	FfiConverterStringINSTANCE.Write(writer, value.Date)
	FfiConverterFloat64INSTANCE.Write(writer, value.K)
	FfiConverterFloat64INSTANCE.Write(writer, value.D)
	FfiConverterFloat64INSTANCE.Write(writer, value.J)
}

type FfiDestroyerKdjDataPoint struct{}

func (_ FfiDestroyerKdjDataPoint) Destroy(value KdjDataPoint) {
	value.Destroy()
}

// KDJ response
type KdjResponse struct {
	Symbol    string
	DataType  string
	Exchange  string
	Market    string
	Timeframe string
	Period    uint32
	Data      []KdjDataPoint
}

func (r *KdjResponse) Destroy() {
	FfiDestroyerString{}.Destroy(r.Symbol)
	FfiDestroyerString{}.Destroy(r.DataType)
	FfiDestroyerString{}.Destroy(r.Exchange)
	FfiDestroyerString{}.Destroy(r.Market)
	FfiDestroyerString{}.Destroy(r.Timeframe)
	FfiDestroyerUint32{}.Destroy(r.Period)
	FfiDestroyerSequenceKdjDataPoint{}.Destroy(r.Data)
}

type FfiConverterKdjResponse struct{}

var FfiConverterKdjResponseINSTANCE = FfiConverterKdjResponse{}

func (c FfiConverterKdjResponse) Lift(rb RustBufferI) KdjResponse {
	return LiftFromRustBuffer[KdjResponse](c, rb)
}

func (c FfiConverterKdjResponse) Read(reader io.Reader) KdjResponse {
	return KdjResponse{
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterUint32INSTANCE.Read(reader),
		FfiConverterSequenceKdjDataPointINSTANCE.Read(reader),
	}
}

func (c FfiConverterKdjResponse) Lower(value KdjResponse) C.RustBuffer {
	return LowerIntoRustBuffer[KdjResponse](c, value)
}

func (c FfiConverterKdjResponse) Write(writer io.Writer, value KdjResponse) {
	FfiConverterStringINSTANCE.Write(writer, value.Symbol)
	FfiConverterStringINSTANCE.Write(writer, value.DataType)
	FfiConverterStringINSTANCE.Write(writer, value.Exchange)
	FfiConverterStringINSTANCE.Write(writer, value.Market)
	FfiConverterStringINSTANCE.Write(writer, value.Timeframe)
	FfiConverterUint32INSTANCE.Write(writer, value.Period)
	FfiConverterSequenceKdjDataPointINSTANCE.Write(writer, value.Data)
}

type FfiDestroyerKdjResponse struct{}

func (_ FfiDestroyerKdjResponse) Destroy(value KdjResponse) {
	value.Destroy()
}

// Listing applicant entry
type ListingApplicant struct {
	Symbol          string
	Name            *string
	ApplicationDate *string
	ListingDate     *string
	Status          *string
	Industry        *string
}

func (r *ListingApplicant) Destroy() {
	FfiDestroyerString{}.Destroy(r.Symbol)
	FfiDestroyerOptionalString{}.Destroy(r.Name)
	FfiDestroyerOptionalString{}.Destroy(r.ApplicationDate)
	FfiDestroyerOptionalString{}.Destroy(r.ListingDate)
	FfiDestroyerOptionalString{}.Destroy(r.Status)
	FfiDestroyerOptionalString{}.Destroy(r.Industry)
}

type FfiConverterListingApplicant struct{}

var FfiConverterListingApplicantINSTANCE = FfiConverterListingApplicant{}

func (c FfiConverterListingApplicant) Lift(rb RustBufferI) ListingApplicant {
	return LiftFromRustBuffer[ListingApplicant](c, rb)
}

func (c FfiConverterListingApplicant) Read(reader io.Reader) ListingApplicant {
	return ListingApplicant{
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
	}
}

func (c FfiConverterListingApplicant) Lower(value ListingApplicant) C.RustBuffer {
	return LowerIntoRustBuffer[ListingApplicant](c, value)
}

func (c FfiConverterListingApplicant) Write(writer io.Writer, value ListingApplicant) {
	FfiConverterStringINSTANCE.Write(writer, value.Symbol)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Name)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.ApplicationDate)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.ListingDate)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Status)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Industry)
}

type FfiDestroyerListingApplicant struct{}

func (_ FfiDestroyerListingApplicant) Destroy(value ListingApplicant) {
	value.Destroy()
}

// Listing applicants response
type ListingApplicantsResponse struct {
	DataType string
	Exchange string
	Market   string
	Data     []ListingApplicant
}

func (r *ListingApplicantsResponse) Destroy() {
	FfiDestroyerString{}.Destroy(r.DataType)
	FfiDestroyerString{}.Destroy(r.Exchange)
	FfiDestroyerString{}.Destroy(r.Market)
	FfiDestroyerSequenceListingApplicant{}.Destroy(r.Data)
}

type FfiConverterListingApplicantsResponse struct{}

var FfiConverterListingApplicantsResponseINSTANCE = FfiConverterListingApplicantsResponse{}

func (c FfiConverterListingApplicantsResponse) Lift(rb RustBufferI) ListingApplicantsResponse {
	return LiftFromRustBuffer[ListingApplicantsResponse](c, rb)
}

func (c FfiConverterListingApplicantsResponse) Read(reader io.Reader) ListingApplicantsResponse {
	return ListingApplicantsResponse{
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterSequenceListingApplicantINSTANCE.Read(reader),
	}
}

func (c FfiConverterListingApplicantsResponse) Lower(value ListingApplicantsResponse) C.RustBuffer {
	return LowerIntoRustBuffer[ListingApplicantsResponse](c, value)
}

func (c FfiConverterListingApplicantsResponse) Write(writer io.Writer, value ListingApplicantsResponse) {
	FfiConverterStringINSTANCE.Write(writer, value.DataType)
	FfiConverterStringINSTANCE.Write(writer, value.Exchange)
	FfiConverterStringINSTANCE.Write(writer, value.Market)
	FfiConverterSequenceListingApplicantINSTANCE.Write(writer, value.Data)
}

type FfiDestroyerListingApplicantsResponse struct{}

func (_ FfiDestroyerListingApplicantsResponse) Destroy(value ListingApplicantsResponse) {
	value.Destroy()
}

// MACD data point
type MacdDataPoint struct {
	Date        string
	Macd        float64
	SignalValue float64
	Histogram   float64
}

func (r *MacdDataPoint) Destroy() {
	FfiDestroyerString{}.Destroy(r.Date)
	FfiDestroyerFloat64{}.Destroy(r.Macd)
	FfiDestroyerFloat64{}.Destroy(r.SignalValue)
	FfiDestroyerFloat64{}.Destroy(r.Histogram)
}

type FfiConverterMacdDataPoint struct{}

var FfiConverterMacdDataPointINSTANCE = FfiConverterMacdDataPoint{}

func (c FfiConverterMacdDataPoint) Lift(rb RustBufferI) MacdDataPoint {
	return LiftFromRustBuffer[MacdDataPoint](c, rb)
}

func (c FfiConverterMacdDataPoint) Read(reader io.Reader) MacdDataPoint {
	return MacdDataPoint{
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterFloat64INSTANCE.Read(reader),
		FfiConverterFloat64INSTANCE.Read(reader),
		FfiConverterFloat64INSTANCE.Read(reader),
	}
}

func (c FfiConverterMacdDataPoint) Lower(value MacdDataPoint) C.RustBuffer {
	return LowerIntoRustBuffer[MacdDataPoint](c, value)
}

func (c FfiConverterMacdDataPoint) Write(writer io.Writer, value MacdDataPoint) {
	FfiConverterStringINSTANCE.Write(writer, value.Date)
	FfiConverterFloat64INSTANCE.Write(writer, value.Macd)
	FfiConverterFloat64INSTANCE.Write(writer, value.SignalValue)
	FfiConverterFloat64INSTANCE.Write(writer, value.Histogram)
}

type FfiDestroyerMacdDataPoint struct{}

func (_ FfiDestroyerMacdDataPoint) Destroy(value MacdDataPoint) {
	value.Destroy()
}

// MACD response
type MacdResponse struct {
	Symbol    string
	DataType  string
	Exchange  string
	Market    string
	Timeframe string
	Fast      uint32
	Slow      uint32
	Signal    uint32
	Data      []MacdDataPoint
}

func (r *MacdResponse) Destroy() {
	FfiDestroyerString{}.Destroy(r.Symbol)
	FfiDestroyerString{}.Destroy(r.DataType)
	FfiDestroyerString{}.Destroy(r.Exchange)
	FfiDestroyerString{}.Destroy(r.Market)
	FfiDestroyerString{}.Destroy(r.Timeframe)
	FfiDestroyerUint32{}.Destroy(r.Fast)
	FfiDestroyerUint32{}.Destroy(r.Slow)
	FfiDestroyerUint32{}.Destroy(r.Signal)
	FfiDestroyerSequenceMacdDataPoint{}.Destroy(r.Data)
}

type FfiConverterMacdResponse struct{}

var FfiConverterMacdResponseINSTANCE = FfiConverterMacdResponse{}

func (c FfiConverterMacdResponse) Lift(rb RustBufferI) MacdResponse {
	return LiftFromRustBuffer[MacdResponse](c, rb)
}

func (c FfiConverterMacdResponse) Read(reader io.Reader) MacdResponse {
	return MacdResponse{
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterUint32INSTANCE.Read(reader),
		FfiConverterUint32INSTANCE.Read(reader),
		FfiConverterUint32INSTANCE.Read(reader),
		FfiConverterSequenceMacdDataPointINSTANCE.Read(reader),
	}
}

func (c FfiConverterMacdResponse) Lower(value MacdResponse) C.RustBuffer {
	return LowerIntoRustBuffer[MacdResponse](c, value)
}

func (c FfiConverterMacdResponse) Write(writer io.Writer, value MacdResponse) {
	FfiConverterStringINSTANCE.Write(writer, value.Symbol)
	FfiConverterStringINSTANCE.Write(writer, value.DataType)
	FfiConverterStringINSTANCE.Write(writer, value.Exchange)
	FfiConverterStringINSTANCE.Write(writer, value.Market)
	FfiConverterStringINSTANCE.Write(writer, value.Timeframe)
	FfiConverterUint32INSTANCE.Write(writer, value.Fast)
	FfiConverterUint32INSTANCE.Write(writer, value.Slow)
	FfiConverterUint32INSTANCE.Write(writer, value.Signal)
	FfiConverterSequenceMacdDataPointINSTANCE.Write(writer, value.Data)
}

type FfiDestroyerMacdResponse struct{}

func (_ FfiDestroyerMacdResponse) Destroy(value MacdResponse) {
	value.Destroy()
}

// Single mover entry
type Mover struct {
	DataType      *string
	Symbol        string
	Name          *string
	OpenPrice     *float64
	HighPrice     *float64
	LowPrice      *float64
	ClosePrice    *float64
	Change        *float64
	ChangePercent *float64
	TradeVolume   *int64
	TradeValue    *float64
	LastUpdated   *int64
}

func (r *Mover) Destroy() {
	FfiDestroyerOptionalString{}.Destroy(r.DataType)
	FfiDestroyerString{}.Destroy(r.Symbol)
	FfiDestroyerOptionalString{}.Destroy(r.Name)
	FfiDestroyerOptionalFloat64{}.Destroy(r.OpenPrice)
	FfiDestroyerOptionalFloat64{}.Destroy(r.HighPrice)
	FfiDestroyerOptionalFloat64{}.Destroy(r.LowPrice)
	FfiDestroyerOptionalFloat64{}.Destroy(r.ClosePrice)
	FfiDestroyerOptionalFloat64{}.Destroy(r.Change)
	FfiDestroyerOptionalFloat64{}.Destroy(r.ChangePercent)
	FfiDestroyerOptionalInt64{}.Destroy(r.TradeVolume)
	FfiDestroyerOptionalFloat64{}.Destroy(r.TradeValue)
	FfiDestroyerOptionalInt64{}.Destroy(r.LastUpdated)
}

type FfiConverterMover struct{}

var FfiConverterMoverINSTANCE = FfiConverterMover{}

func (c FfiConverterMover) Lift(rb RustBufferI) Mover {
	return LiftFromRustBuffer[Mover](c, rb)
}

func (c FfiConverterMover) Read(reader io.Reader) Mover {
	return Mover{
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterOptionalInt64INSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterOptionalInt64INSTANCE.Read(reader),
	}
}

func (c FfiConverterMover) Lower(value Mover) C.RustBuffer {
	return LowerIntoRustBuffer[Mover](c, value)
}

func (c FfiConverterMover) Write(writer io.Writer, value Mover) {
	FfiConverterOptionalStringINSTANCE.Write(writer, value.DataType)
	FfiConverterStringINSTANCE.Write(writer, value.Symbol)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Name)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.OpenPrice)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.HighPrice)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.LowPrice)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.ClosePrice)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.Change)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.ChangePercent)
	FfiConverterOptionalInt64INSTANCE.Write(writer, value.TradeVolume)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.TradeValue)
	FfiConverterOptionalInt64INSTANCE.Write(writer, value.LastUpdated)
}

type FfiDestroyerMover struct{}

func (_ FfiDestroyerMover) Destroy(value Mover) {
	value.Destroy()
}

// Movers response
type MoversResponse struct {
	Date   string
	Time   string
	Market string
	Data   []Mover
}

func (r *MoversResponse) Destroy() {
	FfiDestroyerString{}.Destroy(r.Date)
	FfiDestroyerString{}.Destroy(r.Time)
	FfiDestroyerString{}.Destroy(r.Market)
	FfiDestroyerSequenceMover{}.Destroy(r.Data)
}

type FfiConverterMoversResponse struct{}

var FfiConverterMoversResponseINSTANCE = FfiConverterMoversResponse{}

func (c FfiConverterMoversResponse) Lift(rb RustBufferI) MoversResponse {
	return LiftFromRustBuffer[MoversResponse](c, rb)
}

func (c FfiConverterMoversResponse) Read(reader io.Reader) MoversResponse {
	return MoversResponse{
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterSequenceMoverINSTANCE.Read(reader),
	}
}

func (c FfiConverterMoversResponse) Lower(value MoversResponse) C.RustBuffer {
	return LowerIntoRustBuffer[MoversResponse](c, value)
}

func (c FfiConverterMoversResponse) Write(writer io.Writer, value MoversResponse) {
	FfiConverterStringINSTANCE.Write(writer, value.Date)
	FfiConverterStringINSTANCE.Write(writer, value.Time)
	FfiConverterStringINSTANCE.Write(writer, value.Market)
	FfiConverterSequenceMoverINSTANCE.Write(writer, value.Data)
}

type FfiDestroyerMoversResponse struct{}

func (_ FfiDestroyerMoversResponse) Destroy(value MoversResponse) {
	value.Destroy()
}

// Bid/Ask price level for order book
type PriceLevel struct {
	Price float64
	Size  int64
}

func (r *PriceLevel) Destroy() {
	FfiDestroyerFloat64{}.Destroy(r.Price)
	FfiDestroyerInt64{}.Destroy(r.Size)
}

type FfiConverterPriceLevel struct{}

var FfiConverterPriceLevelINSTANCE = FfiConverterPriceLevel{}

func (c FfiConverterPriceLevel) Lift(rb RustBufferI) PriceLevel {
	return LiftFromRustBuffer[PriceLevel](c, rb)
}

func (c FfiConverterPriceLevel) Read(reader io.Reader) PriceLevel {
	return PriceLevel{
		FfiConverterFloat64INSTANCE.Read(reader),
		FfiConverterInt64INSTANCE.Read(reader),
	}
}

func (c FfiConverterPriceLevel) Lower(value PriceLevel) C.RustBuffer {
	return LowerIntoRustBuffer[PriceLevel](c, value)
}

func (c FfiConverterPriceLevel) Write(writer io.Writer, value PriceLevel) {
	FfiConverterFloat64INSTANCE.Write(writer, value.Price)
	FfiConverterInt64INSTANCE.Write(writer, value.Size)
}

type FfiDestroyerPriceLevel struct{}

func (_ FfiDestroyerPriceLevel) Destroy(value PriceLevel) {
	value.Destroy()
}

// FutOpt product
type Product struct {
	ProductType      *string
	Exchange         *string
	Symbol           string
	Name             *string
	UnderlyingSymbol *string
	ContractType     *string
	ContractSize     *int64
	UnderlyingType   *string
	StatusCode       *string
	TradingCurrency  *string
	QuoteAcceptable  bool
	CanBlockTrade    bool
	StartDate        *string
	ExpiryType       *string
	MarketCloseGroup *int32
	EndSession       *int32
}

func (r *Product) Destroy() {
	FfiDestroyerOptionalString{}.Destroy(r.ProductType)
	FfiDestroyerOptionalString{}.Destroy(r.Exchange)
	FfiDestroyerString{}.Destroy(r.Symbol)
	FfiDestroyerOptionalString{}.Destroy(r.Name)
	FfiDestroyerOptionalString{}.Destroy(r.UnderlyingSymbol)
	FfiDestroyerOptionalString{}.Destroy(r.ContractType)
	FfiDestroyerOptionalInt64{}.Destroy(r.ContractSize)
	FfiDestroyerOptionalString{}.Destroy(r.UnderlyingType)
	FfiDestroyerOptionalString{}.Destroy(r.StatusCode)
	FfiDestroyerOptionalString{}.Destroy(r.TradingCurrency)
	FfiDestroyerBool{}.Destroy(r.QuoteAcceptable)
	FfiDestroyerBool{}.Destroy(r.CanBlockTrade)
	FfiDestroyerOptionalString{}.Destroy(r.StartDate)
	FfiDestroyerOptionalString{}.Destroy(r.ExpiryType)
	FfiDestroyerOptionalInt32{}.Destroy(r.MarketCloseGroup)
	FfiDestroyerOptionalInt32{}.Destroy(r.EndSession)
}

type FfiConverterProduct struct{}

var FfiConverterProductINSTANCE = FfiConverterProduct{}

func (c FfiConverterProduct) Lift(rb RustBufferI) Product {
	return LiftFromRustBuffer[Product](c, rb)
}

func (c FfiConverterProduct) Read(reader io.Reader) Product {
	return Product{
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalInt64INSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterBoolINSTANCE.Read(reader),
		FfiConverterBoolINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalInt32INSTANCE.Read(reader),
		FfiConverterOptionalInt32INSTANCE.Read(reader),
	}
}

func (c FfiConverterProduct) Lower(value Product) C.RustBuffer {
	return LowerIntoRustBuffer[Product](c, value)
}

func (c FfiConverterProduct) Write(writer io.Writer, value Product) {
	FfiConverterOptionalStringINSTANCE.Write(writer, value.ProductType)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Exchange)
	FfiConverterStringINSTANCE.Write(writer, value.Symbol)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Name)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.UnderlyingSymbol)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.ContractType)
	FfiConverterOptionalInt64INSTANCE.Write(writer, value.ContractSize)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.UnderlyingType)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.StatusCode)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.TradingCurrency)
	FfiConverterBoolINSTANCE.Write(writer, value.QuoteAcceptable)
	FfiConverterBoolINSTANCE.Write(writer, value.CanBlockTrade)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.StartDate)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.ExpiryType)
	FfiConverterOptionalInt32INSTANCE.Write(writer, value.MarketCloseGroup)
	FfiConverterOptionalInt32INSTANCE.Write(writer, value.EndSession)
}

type FfiDestroyerProduct struct{}

func (_ FfiDestroyerProduct) Destroy(value Product) {
	value.Destroy()
}

// FutOpt products response
type ProductsResponse struct {
	Date         *string
	ProductType  *string
	Session      *string
	ContractType *string
	Status       *string
	Data         []Product
}

func (r *ProductsResponse) Destroy() {
	FfiDestroyerOptionalString{}.Destroy(r.Date)
	FfiDestroyerOptionalString{}.Destroy(r.ProductType)
	FfiDestroyerOptionalString{}.Destroy(r.Session)
	FfiDestroyerOptionalString{}.Destroy(r.ContractType)
	FfiDestroyerOptionalString{}.Destroy(r.Status)
	FfiDestroyerSequenceProduct{}.Destroy(r.Data)
}

type FfiConverterProductsResponse struct{}

var FfiConverterProductsResponseINSTANCE = FfiConverterProductsResponse{}

func (c FfiConverterProductsResponse) Lift(rb RustBufferI) ProductsResponse {
	return LiftFromRustBuffer[ProductsResponse](c, rb)
}

func (c FfiConverterProductsResponse) Read(reader io.Reader) ProductsResponse {
	return ProductsResponse{
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterSequenceProductINSTANCE.Read(reader),
	}
}

func (c FfiConverterProductsResponse) Lower(value ProductsResponse) C.RustBuffer {
	return LowerIntoRustBuffer[ProductsResponse](c, value)
}

func (c FfiConverterProductsResponse) Write(writer io.Writer, value ProductsResponse) {
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Date)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.ProductType)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Session)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.ContractType)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Status)
	FfiConverterSequenceProductINSTANCE.Write(writer, value.Data)
}

type FfiDestroyerProductsResponse struct{}

func (_ FfiDestroyerProductsResponse) Destroy(value ProductsResponse) {
	value.Destroy()
}

// Real-time stock quote
type Quote struct {
	Date             string
	DataType         *string
	Exchange         *string
	Market           *string
	Symbol           string
	Name             *string
	OpenPrice        *float64
	OpenTime         *int64
	HighPrice        *float64
	HighTime         *int64
	LowPrice         *float64
	LowTime          *int64
	ClosePrice       *float64
	CloseTime        *int64
	LastPrice        *float64
	LastSize         *int64
	AvgPrice         *float64
	Change           *float64
	ChangePercent    *float64
	Amplitude        *float64
	Bids             []PriceLevel
	Asks             []PriceLevel
	Total            *TotalStats
	LastTrade        *TradeInfo
	LastTrial        *TradeInfo
	TradingHalt      *TradingHalt
	IsLimitDownPrice bool
	IsLimitUpPrice   bool
	IsLimitDownBid   bool
	IsLimitUpBid     bool
	IsLimitDownAsk   bool
	IsLimitUpAsk     bool
	IsLimitDownHalt  bool
	IsLimitUpHalt    bool
	IsTrial          bool
	IsDelayedOpen    bool
	IsDelayedClose   bool
	IsContinuous     bool
	IsOpen           bool
	IsClose          bool
	LastUpdated      *int64
}

func (r *Quote) Destroy() {
	FfiDestroyerString{}.Destroy(r.Date)
	FfiDestroyerOptionalString{}.Destroy(r.DataType)
	FfiDestroyerOptionalString{}.Destroy(r.Exchange)
	FfiDestroyerOptionalString{}.Destroy(r.Market)
	FfiDestroyerString{}.Destroy(r.Symbol)
	FfiDestroyerOptionalString{}.Destroy(r.Name)
	FfiDestroyerOptionalFloat64{}.Destroy(r.OpenPrice)
	FfiDestroyerOptionalInt64{}.Destroy(r.OpenTime)
	FfiDestroyerOptionalFloat64{}.Destroy(r.HighPrice)
	FfiDestroyerOptionalInt64{}.Destroy(r.HighTime)
	FfiDestroyerOptionalFloat64{}.Destroy(r.LowPrice)
	FfiDestroyerOptionalInt64{}.Destroy(r.LowTime)
	FfiDestroyerOptionalFloat64{}.Destroy(r.ClosePrice)
	FfiDestroyerOptionalInt64{}.Destroy(r.CloseTime)
	FfiDestroyerOptionalFloat64{}.Destroy(r.LastPrice)
	FfiDestroyerOptionalInt64{}.Destroy(r.LastSize)
	FfiDestroyerOptionalFloat64{}.Destroy(r.AvgPrice)
	FfiDestroyerOptionalFloat64{}.Destroy(r.Change)
	FfiDestroyerOptionalFloat64{}.Destroy(r.ChangePercent)
	FfiDestroyerOptionalFloat64{}.Destroy(r.Amplitude)
	FfiDestroyerSequencePriceLevel{}.Destroy(r.Bids)
	FfiDestroyerSequencePriceLevel{}.Destroy(r.Asks)
	FfiDestroyerOptionalTotalStats{}.Destroy(r.Total)
	FfiDestroyerOptionalTradeInfo{}.Destroy(r.LastTrade)
	FfiDestroyerOptionalTradeInfo{}.Destroy(r.LastTrial)
	FfiDestroyerOptionalTradingHalt{}.Destroy(r.TradingHalt)
	FfiDestroyerBool{}.Destroy(r.IsLimitDownPrice)
	FfiDestroyerBool{}.Destroy(r.IsLimitUpPrice)
	FfiDestroyerBool{}.Destroy(r.IsLimitDownBid)
	FfiDestroyerBool{}.Destroy(r.IsLimitUpBid)
	FfiDestroyerBool{}.Destroy(r.IsLimitDownAsk)
	FfiDestroyerBool{}.Destroy(r.IsLimitUpAsk)
	FfiDestroyerBool{}.Destroy(r.IsLimitDownHalt)
	FfiDestroyerBool{}.Destroy(r.IsLimitUpHalt)
	FfiDestroyerBool{}.Destroy(r.IsTrial)
	FfiDestroyerBool{}.Destroy(r.IsDelayedOpen)
	FfiDestroyerBool{}.Destroy(r.IsDelayedClose)
	FfiDestroyerBool{}.Destroy(r.IsContinuous)
	FfiDestroyerBool{}.Destroy(r.IsOpen)
	FfiDestroyerBool{}.Destroy(r.IsClose)
	FfiDestroyerOptionalInt64{}.Destroy(r.LastUpdated)
}

type FfiConverterQuote struct{}

var FfiConverterQuoteINSTANCE = FfiConverterQuote{}

func (c FfiConverterQuote) Lift(rb RustBufferI) Quote {
	return LiftFromRustBuffer[Quote](c, rb)
}

func (c FfiConverterQuote) Read(reader io.Reader) Quote {
	return Quote{
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterOptionalInt64INSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterOptionalInt64INSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterOptionalInt64INSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterOptionalInt64INSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterOptionalInt64INSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterSequencePriceLevelINSTANCE.Read(reader),
		FfiConverterSequencePriceLevelINSTANCE.Read(reader),
		FfiConverterOptionalTotalStatsINSTANCE.Read(reader),
		FfiConverterOptionalTradeInfoINSTANCE.Read(reader),
		FfiConverterOptionalTradeInfoINSTANCE.Read(reader),
		FfiConverterOptionalTradingHaltINSTANCE.Read(reader),
		FfiConverterBoolINSTANCE.Read(reader),
		FfiConverterBoolINSTANCE.Read(reader),
		FfiConverterBoolINSTANCE.Read(reader),
		FfiConverterBoolINSTANCE.Read(reader),
		FfiConverterBoolINSTANCE.Read(reader),
		FfiConverterBoolINSTANCE.Read(reader),
		FfiConverterBoolINSTANCE.Read(reader),
		FfiConverterBoolINSTANCE.Read(reader),
		FfiConverterBoolINSTANCE.Read(reader),
		FfiConverterBoolINSTANCE.Read(reader),
		FfiConverterBoolINSTANCE.Read(reader),
		FfiConverterBoolINSTANCE.Read(reader),
		FfiConverterBoolINSTANCE.Read(reader),
		FfiConverterBoolINSTANCE.Read(reader),
		FfiConverterOptionalInt64INSTANCE.Read(reader),
	}
}

func (c FfiConverterQuote) Lower(value Quote) C.RustBuffer {
	return LowerIntoRustBuffer[Quote](c, value)
}

func (c FfiConverterQuote) Write(writer io.Writer, value Quote) {
	FfiConverterStringINSTANCE.Write(writer, value.Date)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.DataType)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Exchange)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Market)
	FfiConverterStringINSTANCE.Write(writer, value.Symbol)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Name)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.OpenPrice)
	FfiConverterOptionalInt64INSTANCE.Write(writer, value.OpenTime)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.HighPrice)
	FfiConverterOptionalInt64INSTANCE.Write(writer, value.HighTime)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.LowPrice)
	FfiConverterOptionalInt64INSTANCE.Write(writer, value.LowTime)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.ClosePrice)
	FfiConverterOptionalInt64INSTANCE.Write(writer, value.CloseTime)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.LastPrice)
	FfiConverterOptionalInt64INSTANCE.Write(writer, value.LastSize)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.AvgPrice)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.Change)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.ChangePercent)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.Amplitude)
	FfiConverterSequencePriceLevelINSTANCE.Write(writer, value.Bids)
	FfiConverterSequencePriceLevelINSTANCE.Write(writer, value.Asks)
	FfiConverterOptionalTotalStatsINSTANCE.Write(writer, value.Total)
	FfiConverterOptionalTradeInfoINSTANCE.Write(writer, value.LastTrade)
	FfiConverterOptionalTradeInfoINSTANCE.Write(writer, value.LastTrial)
	FfiConverterOptionalTradingHaltINSTANCE.Write(writer, value.TradingHalt)
	FfiConverterBoolINSTANCE.Write(writer, value.IsLimitDownPrice)
	FfiConverterBoolINSTANCE.Write(writer, value.IsLimitUpPrice)
	FfiConverterBoolINSTANCE.Write(writer, value.IsLimitDownBid)
	FfiConverterBoolINSTANCE.Write(writer, value.IsLimitUpBid)
	FfiConverterBoolINSTANCE.Write(writer, value.IsLimitDownAsk)
	FfiConverterBoolINSTANCE.Write(writer, value.IsLimitUpAsk)
	FfiConverterBoolINSTANCE.Write(writer, value.IsLimitDownHalt)
	FfiConverterBoolINSTANCE.Write(writer, value.IsLimitUpHalt)
	FfiConverterBoolINSTANCE.Write(writer, value.IsTrial)
	FfiConverterBoolINSTANCE.Write(writer, value.IsDelayedOpen)
	FfiConverterBoolINSTANCE.Write(writer, value.IsDelayedClose)
	FfiConverterBoolINSTANCE.Write(writer, value.IsContinuous)
	FfiConverterBoolINSTANCE.Write(writer, value.IsOpen)
	FfiConverterBoolINSTANCE.Write(writer, value.IsClose)
	FfiConverterOptionalInt64INSTANCE.Write(writer, value.LastUpdated)
}

type FfiDestroyerQuote struct{}

func (_ FfiDestroyerQuote) Destroy(value Quote) {
	value.Destroy()
}

// Reconnection configuration record for FFI
//
// All fields are optional — zero/false values mean "use default".
type ReconnectConfigRecord struct {
	// Maximum reconnection attempts (default: 5, min: 1)
	MaxAttempts uint32
	// Initial reconnection delay in milliseconds (default: 1000, min: 100)
	InitialDelayMs uint64
	// Maximum reconnection delay in milliseconds (default: 60000)
	MaxDelayMs uint64
}

func (r *ReconnectConfigRecord) Destroy() {
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
		FfiConverterUint32INSTANCE.Read(reader),
		FfiConverterUint64INSTANCE.Read(reader),
		FfiConverterUint64INSTANCE.Read(reader),
	}
}

func (c FfiConverterReconnectConfigRecord) Lower(value ReconnectConfigRecord) C.RustBuffer {
	return LowerIntoRustBuffer[ReconnectConfigRecord](c, value)
}

func (c FfiConverterReconnectConfigRecord) Write(writer io.Writer, value ReconnectConfigRecord) {
	FfiConverterUint32INSTANCE.Write(writer, value.MaxAttempts)
	FfiConverterUint64INSTANCE.Write(writer, value.InitialDelayMs)
	FfiConverterUint64INSTANCE.Write(writer, value.MaxDelayMs)
}

type FfiDestroyerReconnectConfigRecord struct{}

func (_ FfiDestroyerReconnectConfigRecord) Destroy(value ReconnectConfigRecord) {
	value.Destroy()
}

// RSI data point
type RsiDataPoint struct {
	Date string
	Rsi  float64
}

func (r *RsiDataPoint) Destroy() {
	FfiDestroyerString{}.Destroy(r.Date)
	FfiDestroyerFloat64{}.Destroy(r.Rsi)
}

type FfiConverterRsiDataPoint struct{}

var FfiConverterRsiDataPointINSTANCE = FfiConverterRsiDataPoint{}

func (c FfiConverterRsiDataPoint) Lift(rb RustBufferI) RsiDataPoint {
	return LiftFromRustBuffer[RsiDataPoint](c, rb)
}

func (c FfiConverterRsiDataPoint) Read(reader io.Reader) RsiDataPoint {
	return RsiDataPoint{
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterFloat64INSTANCE.Read(reader),
	}
}

func (c FfiConverterRsiDataPoint) Lower(value RsiDataPoint) C.RustBuffer {
	return LowerIntoRustBuffer[RsiDataPoint](c, value)
}

func (c FfiConverterRsiDataPoint) Write(writer io.Writer, value RsiDataPoint) {
	FfiConverterStringINSTANCE.Write(writer, value.Date)
	FfiConverterFloat64INSTANCE.Write(writer, value.Rsi)
}

type FfiDestroyerRsiDataPoint struct{}

func (_ FfiDestroyerRsiDataPoint) Destroy(value RsiDataPoint) {
	value.Destroy()
}

// RSI response
type RsiResponse struct {
	Symbol    string
	DataType  string
	Exchange  string
	Market    string
	Timeframe string
	Period    uint32
	Data      []RsiDataPoint
}

func (r *RsiResponse) Destroy() {
	FfiDestroyerString{}.Destroy(r.Symbol)
	FfiDestroyerString{}.Destroy(r.DataType)
	FfiDestroyerString{}.Destroy(r.Exchange)
	FfiDestroyerString{}.Destroy(r.Market)
	FfiDestroyerString{}.Destroy(r.Timeframe)
	FfiDestroyerUint32{}.Destroy(r.Period)
	FfiDestroyerSequenceRsiDataPoint{}.Destroy(r.Data)
}

type FfiConverterRsiResponse struct{}

var FfiConverterRsiResponseINSTANCE = FfiConverterRsiResponse{}

func (c FfiConverterRsiResponse) Lift(rb RustBufferI) RsiResponse {
	return LiftFromRustBuffer[RsiResponse](c, rb)
}

func (c FfiConverterRsiResponse) Read(reader io.Reader) RsiResponse {
	return RsiResponse{
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterUint32INSTANCE.Read(reader),
		FfiConverterSequenceRsiDataPointINSTANCE.Read(reader),
	}
}

func (c FfiConverterRsiResponse) Lower(value RsiResponse) C.RustBuffer {
	return LowerIntoRustBuffer[RsiResponse](c, value)
}

func (c FfiConverterRsiResponse) Write(writer io.Writer, value RsiResponse) {
	FfiConverterStringINSTANCE.Write(writer, value.Symbol)
	FfiConverterStringINSTANCE.Write(writer, value.DataType)
	FfiConverterStringINSTANCE.Write(writer, value.Exchange)
	FfiConverterStringINSTANCE.Write(writer, value.Market)
	FfiConverterStringINSTANCE.Write(writer, value.Timeframe)
	FfiConverterUint32INSTANCE.Write(writer, value.Period)
	FfiConverterSequenceRsiDataPointINSTANCE.Write(writer, value.Data)
}

type FfiDestroyerRsiResponse struct{}

func (_ FfiDestroyerRsiResponse) Destroy(value RsiResponse) {
	value.Destroy()
}

// SMA data point
type SmaDataPoint struct {
	Date string
	Sma  float64
}

func (r *SmaDataPoint) Destroy() {
	FfiDestroyerString{}.Destroy(r.Date)
	FfiDestroyerFloat64{}.Destroy(r.Sma)
}

type FfiConverterSmaDataPoint struct{}

var FfiConverterSmaDataPointINSTANCE = FfiConverterSmaDataPoint{}

func (c FfiConverterSmaDataPoint) Lift(rb RustBufferI) SmaDataPoint {
	return LiftFromRustBuffer[SmaDataPoint](c, rb)
}

func (c FfiConverterSmaDataPoint) Read(reader io.Reader) SmaDataPoint {
	return SmaDataPoint{
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterFloat64INSTANCE.Read(reader),
	}
}

func (c FfiConverterSmaDataPoint) Lower(value SmaDataPoint) C.RustBuffer {
	return LowerIntoRustBuffer[SmaDataPoint](c, value)
}

func (c FfiConverterSmaDataPoint) Write(writer io.Writer, value SmaDataPoint) {
	FfiConverterStringINSTANCE.Write(writer, value.Date)
	FfiConverterFloat64INSTANCE.Write(writer, value.Sma)
}

type FfiDestroyerSmaDataPoint struct{}

func (_ FfiDestroyerSmaDataPoint) Destroy(value SmaDataPoint) {
	value.Destroy()
}

// SMA response
type SmaResponse struct {
	Symbol    string
	DataType  string
	Exchange  string
	Market    string
	Timeframe string
	Period    uint32
	Data      []SmaDataPoint
}

func (r *SmaResponse) Destroy() {
	FfiDestroyerString{}.Destroy(r.Symbol)
	FfiDestroyerString{}.Destroy(r.DataType)
	FfiDestroyerString{}.Destroy(r.Exchange)
	FfiDestroyerString{}.Destroy(r.Market)
	FfiDestroyerString{}.Destroy(r.Timeframe)
	FfiDestroyerUint32{}.Destroy(r.Period)
	FfiDestroyerSequenceSmaDataPoint{}.Destroy(r.Data)
}

type FfiConverterSmaResponse struct{}

var FfiConverterSmaResponseINSTANCE = FfiConverterSmaResponse{}

func (c FfiConverterSmaResponse) Lift(rb RustBufferI) SmaResponse {
	return LiftFromRustBuffer[SmaResponse](c, rb)
}

func (c FfiConverterSmaResponse) Read(reader io.Reader) SmaResponse {
	return SmaResponse{
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterUint32INSTANCE.Read(reader),
		FfiConverterSequenceSmaDataPointINSTANCE.Read(reader),
	}
}

func (c FfiConverterSmaResponse) Lower(value SmaResponse) C.RustBuffer {
	return LowerIntoRustBuffer[SmaResponse](c, value)
}

func (c FfiConverterSmaResponse) Write(writer io.Writer, value SmaResponse) {
	FfiConverterStringINSTANCE.Write(writer, value.Symbol)
	FfiConverterStringINSTANCE.Write(writer, value.DataType)
	FfiConverterStringINSTANCE.Write(writer, value.Exchange)
	FfiConverterStringINSTANCE.Write(writer, value.Market)
	FfiConverterStringINSTANCE.Write(writer, value.Timeframe)
	FfiConverterUint32INSTANCE.Write(writer, value.Period)
	FfiConverterSequenceSmaDataPointINSTANCE.Write(writer, value.Data)
}

type FfiDestroyerSmaResponse struct{}

func (_ FfiDestroyerSmaResponse) Destroy(value SmaResponse) {
	value.Destroy()
}

// Single snapshot quote
type SnapshotQuote struct {
	DataType      *string
	Symbol        string
	Name          *string
	OpenPrice     *float64
	HighPrice     *float64
	LowPrice      *float64
	ClosePrice    *float64
	Change        *float64
	ChangePercent *float64
	TradeVolume   *int64
	TradeValue    *float64
	LastUpdated   *int64
}

func (r *SnapshotQuote) Destroy() {
	FfiDestroyerOptionalString{}.Destroy(r.DataType)
	FfiDestroyerString{}.Destroy(r.Symbol)
	FfiDestroyerOptionalString{}.Destroy(r.Name)
	FfiDestroyerOptionalFloat64{}.Destroy(r.OpenPrice)
	FfiDestroyerOptionalFloat64{}.Destroy(r.HighPrice)
	FfiDestroyerOptionalFloat64{}.Destroy(r.LowPrice)
	FfiDestroyerOptionalFloat64{}.Destroy(r.ClosePrice)
	FfiDestroyerOptionalFloat64{}.Destroy(r.Change)
	FfiDestroyerOptionalFloat64{}.Destroy(r.ChangePercent)
	FfiDestroyerOptionalInt64{}.Destroy(r.TradeVolume)
	FfiDestroyerOptionalFloat64{}.Destroy(r.TradeValue)
	FfiDestroyerOptionalInt64{}.Destroy(r.LastUpdated)
}

type FfiConverterSnapshotQuote struct{}

var FfiConverterSnapshotQuoteINSTANCE = FfiConverterSnapshotQuote{}

func (c FfiConverterSnapshotQuote) Lift(rb RustBufferI) SnapshotQuote {
	return LiftFromRustBuffer[SnapshotQuote](c, rb)
}

func (c FfiConverterSnapshotQuote) Read(reader io.Reader) SnapshotQuote {
	return SnapshotQuote{
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterOptionalInt64INSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterOptionalInt64INSTANCE.Read(reader),
	}
}

func (c FfiConverterSnapshotQuote) Lower(value SnapshotQuote) C.RustBuffer {
	return LowerIntoRustBuffer[SnapshotQuote](c, value)
}

func (c FfiConverterSnapshotQuote) Write(writer io.Writer, value SnapshotQuote) {
	FfiConverterOptionalStringINSTANCE.Write(writer, value.DataType)
	FfiConverterStringINSTANCE.Write(writer, value.Symbol)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Name)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.OpenPrice)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.HighPrice)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.LowPrice)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.ClosePrice)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.Change)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.ChangePercent)
	FfiConverterOptionalInt64INSTANCE.Write(writer, value.TradeVolume)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.TradeValue)
	FfiConverterOptionalInt64INSTANCE.Write(writer, value.LastUpdated)
}

type FfiDestroyerSnapshotQuote struct{}

func (_ FfiDestroyerSnapshotQuote) Destroy(value SnapshotQuote) {
	value.Destroy()
}

// Snapshot quotes response
type SnapshotQuotesResponse struct {
	Date   string
	Time   string
	Market string
	Data   []SnapshotQuote
}

func (r *SnapshotQuotesResponse) Destroy() {
	FfiDestroyerString{}.Destroy(r.Date)
	FfiDestroyerString{}.Destroy(r.Time)
	FfiDestroyerString{}.Destroy(r.Market)
	FfiDestroyerSequenceSnapshotQuote{}.Destroy(r.Data)
}

type FfiConverterSnapshotQuotesResponse struct{}

var FfiConverterSnapshotQuotesResponseINSTANCE = FfiConverterSnapshotQuotesResponse{}

func (c FfiConverterSnapshotQuotesResponse) Lift(rb RustBufferI) SnapshotQuotesResponse {
	return LiftFromRustBuffer[SnapshotQuotesResponse](c, rb)
}

func (c FfiConverterSnapshotQuotesResponse) Read(reader io.Reader) SnapshotQuotesResponse {
	return SnapshotQuotesResponse{
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterSequenceSnapshotQuoteINSTANCE.Read(reader),
	}
}

func (c FfiConverterSnapshotQuotesResponse) Lower(value SnapshotQuotesResponse) C.RustBuffer {
	return LowerIntoRustBuffer[SnapshotQuotesResponse](c, value)
}

func (c FfiConverterSnapshotQuotesResponse) Write(writer io.Writer, value SnapshotQuotesResponse) {
	FfiConverterStringINSTANCE.Write(writer, value.Date)
	FfiConverterStringINSTANCE.Write(writer, value.Time)
	FfiConverterStringINSTANCE.Write(writer, value.Market)
	FfiConverterSequenceSnapshotQuoteINSTANCE.Write(writer, value.Data)
}

type FfiDestroyerSnapshotQuotesResponse struct{}

func (_ FfiDestroyerSnapshotQuotesResponse) Destroy(value SnapshotQuotesResponse) {
	value.Destroy()
}

// Historical stats response
type StatsResponse struct {
	Date          string
	DataType      string
	Exchange      string
	Market        string
	Symbol        string
	Name          string
	OpenPrice     float64
	HighPrice     float64
	LowPrice      float64
	ClosePrice    float64
	Change        float64
	ChangePercent float64
	TradeVolume   int64
	TradeValue    float64
	PreviousClose float64
	Week52High    float64
	Week52Low     float64
}

func (r *StatsResponse) Destroy() {
	FfiDestroyerString{}.Destroy(r.Date)
	FfiDestroyerString{}.Destroy(r.DataType)
	FfiDestroyerString{}.Destroy(r.Exchange)
	FfiDestroyerString{}.Destroy(r.Market)
	FfiDestroyerString{}.Destroy(r.Symbol)
	FfiDestroyerString{}.Destroy(r.Name)
	FfiDestroyerFloat64{}.Destroy(r.OpenPrice)
	FfiDestroyerFloat64{}.Destroy(r.HighPrice)
	FfiDestroyerFloat64{}.Destroy(r.LowPrice)
	FfiDestroyerFloat64{}.Destroy(r.ClosePrice)
	FfiDestroyerFloat64{}.Destroy(r.Change)
	FfiDestroyerFloat64{}.Destroy(r.ChangePercent)
	FfiDestroyerInt64{}.Destroy(r.TradeVolume)
	FfiDestroyerFloat64{}.Destroy(r.TradeValue)
	FfiDestroyerFloat64{}.Destroy(r.PreviousClose)
	FfiDestroyerFloat64{}.Destroy(r.Week52High)
	FfiDestroyerFloat64{}.Destroy(r.Week52Low)
}

type FfiConverterStatsResponse struct{}

var FfiConverterStatsResponseINSTANCE = FfiConverterStatsResponse{}

func (c FfiConverterStatsResponse) Lift(rb RustBufferI) StatsResponse {
	return LiftFromRustBuffer[StatsResponse](c, rb)
}

func (c FfiConverterStatsResponse) Read(reader io.Reader) StatsResponse {
	return StatsResponse{
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterFloat64INSTANCE.Read(reader),
		FfiConverterFloat64INSTANCE.Read(reader),
		FfiConverterFloat64INSTANCE.Read(reader),
		FfiConverterFloat64INSTANCE.Read(reader),
		FfiConverterFloat64INSTANCE.Read(reader),
		FfiConverterFloat64INSTANCE.Read(reader),
		FfiConverterInt64INSTANCE.Read(reader),
		FfiConverterFloat64INSTANCE.Read(reader),
		FfiConverterFloat64INSTANCE.Read(reader),
		FfiConverterFloat64INSTANCE.Read(reader),
		FfiConverterFloat64INSTANCE.Read(reader),
	}
}

func (c FfiConverterStatsResponse) Lower(value StatsResponse) C.RustBuffer {
	return LowerIntoRustBuffer[StatsResponse](c, value)
}

func (c FfiConverterStatsResponse) Write(writer io.Writer, value StatsResponse) {
	FfiConverterStringINSTANCE.Write(writer, value.Date)
	FfiConverterStringINSTANCE.Write(writer, value.DataType)
	FfiConverterStringINSTANCE.Write(writer, value.Exchange)
	FfiConverterStringINSTANCE.Write(writer, value.Market)
	FfiConverterStringINSTANCE.Write(writer, value.Symbol)
	FfiConverterStringINSTANCE.Write(writer, value.Name)
	FfiConverterFloat64INSTANCE.Write(writer, value.OpenPrice)
	FfiConverterFloat64INSTANCE.Write(writer, value.HighPrice)
	FfiConverterFloat64INSTANCE.Write(writer, value.LowPrice)
	FfiConverterFloat64INSTANCE.Write(writer, value.ClosePrice)
	FfiConverterFloat64INSTANCE.Write(writer, value.Change)
	FfiConverterFloat64INSTANCE.Write(writer, value.ChangePercent)
	FfiConverterInt64INSTANCE.Write(writer, value.TradeVolume)
	FfiConverterFloat64INSTANCE.Write(writer, value.TradeValue)
	FfiConverterFloat64INSTANCE.Write(writer, value.PreviousClose)
	FfiConverterFloat64INSTANCE.Write(writer, value.Week52High)
	FfiConverterFloat64INSTANCE.Write(writer, value.Week52Low)
}

type FfiDestroyerStatsResponse struct{}

func (_ FfiDestroyerStatsResponse) Destroy(value StatsResponse) {
	value.Destroy()
}

// Streaming message (simplified for FFI)
type StreamMessage struct {
	Event        string
	Channel      *string
	Symbol       *string
	Id           *string
	DataJson     *string
	ErrorCode    *int32
	ErrorMessage *string
}

func (r *StreamMessage) Destroy() {
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

func (c FfiConverterStreamMessage) Write(writer io.Writer, value StreamMessage) {
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

// Stock ticker info
type Ticker struct {
	Date                        string
	DataType                    *string
	Exchange                    *string
	Market                      *string
	Symbol                      string
	Name                        *string
	NameEn                      *string
	Industry                    *string
	SecurityType                *string
	ReferencePrice              *float64
	LimitUpPrice                *float64
	LimitDownPrice              *float64
	PreviousClose               *float64
	CanDayTrade                 bool
	CanBuyDayTrade              bool
	CanBelowFlatMarginShortSell bool
	CanBelowFlatSblShortSell    bool
	IsAttention                 bool
	IsDisposition               bool
	IsUnusuallyRecommended      bool
	IsSpecificAbnormally        bool
	IsNewlyCompiled             bool
	MatchingInterval            *int32
	SecurityStatus              *string
	BoardLot                    *int32
	TradingCurrency             *string
	ExercisePrice               *float64
	ExercisedVolume             *int64
	CancelledVolume             *int64
	RemainingVolume             *int64
	ExerciseRatio               *float64
	CapPrice                    *float64
	FloorPrice                  *float64
	MaturityDate                *string
	OpenTime                    *string
	CloseTime                   *string
}

func (r *Ticker) Destroy() {
	FfiDestroyerString{}.Destroy(r.Date)
	FfiDestroyerOptionalString{}.Destroy(r.DataType)
	FfiDestroyerOptionalString{}.Destroy(r.Exchange)
	FfiDestroyerOptionalString{}.Destroy(r.Market)
	FfiDestroyerString{}.Destroy(r.Symbol)
	FfiDestroyerOptionalString{}.Destroy(r.Name)
	FfiDestroyerOptionalString{}.Destroy(r.NameEn)
	FfiDestroyerOptionalString{}.Destroy(r.Industry)
	FfiDestroyerOptionalString{}.Destroy(r.SecurityType)
	FfiDestroyerOptionalFloat64{}.Destroy(r.ReferencePrice)
	FfiDestroyerOptionalFloat64{}.Destroy(r.LimitUpPrice)
	FfiDestroyerOptionalFloat64{}.Destroy(r.LimitDownPrice)
	FfiDestroyerOptionalFloat64{}.Destroy(r.PreviousClose)
	FfiDestroyerBool{}.Destroy(r.CanDayTrade)
	FfiDestroyerBool{}.Destroy(r.CanBuyDayTrade)
	FfiDestroyerBool{}.Destroy(r.CanBelowFlatMarginShortSell)
	FfiDestroyerBool{}.Destroy(r.CanBelowFlatSblShortSell)
	FfiDestroyerBool{}.Destroy(r.IsAttention)
	FfiDestroyerBool{}.Destroy(r.IsDisposition)
	FfiDestroyerBool{}.Destroy(r.IsUnusuallyRecommended)
	FfiDestroyerBool{}.Destroy(r.IsSpecificAbnormally)
	FfiDestroyerBool{}.Destroy(r.IsNewlyCompiled)
	FfiDestroyerOptionalInt32{}.Destroy(r.MatchingInterval)
	FfiDestroyerOptionalString{}.Destroy(r.SecurityStatus)
	FfiDestroyerOptionalInt32{}.Destroy(r.BoardLot)
	FfiDestroyerOptionalString{}.Destroy(r.TradingCurrency)
	FfiDestroyerOptionalFloat64{}.Destroy(r.ExercisePrice)
	FfiDestroyerOptionalInt64{}.Destroy(r.ExercisedVolume)
	FfiDestroyerOptionalInt64{}.Destroy(r.CancelledVolume)
	FfiDestroyerOptionalInt64{}.Destroy(r.RemainingVolume)
	FfiDestroyerOptionalFloat64{}.Destroy(r.ExerciseRatio)
	FfiDestroyerOptionalFloat64{}.Destroy(r.CapPrice)
	FfiDestroyerOptionalFloat64{}.Destroy(r.FloorPrice)
	FfiDestroyerOptionalString{}.Destroy(r.MaturityDate)
	FfiDestroyerOptionalString{}.Destroy(r.OpenTime)
	FfiDestroyerOptionalString{}.Destroy(r.CloseTime)
}

type FfiConverterTicker struct{}

var FfiConverterTickerINSTANCE = FfiConverterTicker{}

func (c FfiConverterTicker) Lift(rb RustBufferI) Ticker {
	return LiftFromRustBuffer[Ticker](c, rb)
}

func (c FfiConverterTicker) Read(reader io.Reader) Ticker {
	return Ticker{
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterBoolINSTANCE.Read(reader),
		FfiConverterBoolINSTANCE.Read(reader),
		FfiConverterBoolINSTANCE.Read(reader),
		FfiConverterBoolINSTANCE.Read(reader),
		FfiConverterBoolINSTANCE.Read(reader),
		FfiConverterBoolINSTANCE.Read(reader),
		FfiConverterBoolINSTANCE.Read(reader),
		FfiConverterBoolINSTANCE.Read(reader),
		FfiConverterBoolINSTANCE.Read(reader),
		FfiConverterOptionalInt32INSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalInt32INSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterOptionalInt64INSTANCE.Read(reader),
		FfiConverterOptionalInt64INSTANCE.Read(reader),
		FfiConverterOptionalInt64INSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
	}
}

func (c FfiConverterTicker) Lower(value Ticker) C.RustBuffer {
	return LowerIntoRustBuffer[Ticker](c, value)
}

func (c FfiConverterTicker) Write(writer io.Writer, value Ticker) {
	FfiConverterStringINSTANCE.Write(writer, value.Date)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.DataType)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Exchange)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Market)
	FfiConverterStringINSTANCE.Write(writer, value.Symbol)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Name)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.NameEn)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Industry)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.SecurityType)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.ReferencePrice)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.LimitUpPrice)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.LimitDownPrice)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.PreviousClose)
	FfiConverterBoolINSTANCE.Write(writer, value.CanDayTrade)
	FfiConverterBoolINSTANCE.Write(writer, value.CanBuyDayTrade)
	FfiConverterBoolINSTANCE.Write(writer, value.CanBelowFlatMarginShortSell)
	FfiConverterBoolINSTANCE.Write(writer, value.CanBelowFlatSblShortSell)
	FfiConverterBoolINSTANCE.Write(writer, value.IsAttention)
	FfiConverterBoolINSTANCE.Write(writer, value.IsDisposition)
	FfiConverterBoolINSTANCE.Write(writer, value.IsUnusuallyRecommended)
	FfiConverterBoolINSTANCE.Write(writer, value.IsSpecificAbnormally)
	FfiConverterBoolINSTANCE.Write(writer, value.IsNewlyCompiled)
	FfiConverterOptionalInt32INSTANCE.Write(writer, value.MatchingInterval)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.SecurityStatus)
	FfiConverterOptionalInt32INSTANCE.Write(writer, value.BoardLot)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.TradingCurrency)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.ExercisePrice)
	FfiConverterOptionalInt64INSTANCE.Write(writer, value.ExercisedVolume)
	FfiConverterOptionalInt64INSTANCE.Write(writer, value.CancelledVolume)
	FfiConverterOptionalInt64INSTANCE.Write(writer, value.RemainingVolume)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.ExerciseRatio)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.CapPrice)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.FloorPrice)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.MaturityDate)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.OpenTime)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.CloseTime)
}

type FfiDestroyerTicker struct{}

func (_ FfiDestroyerTicker) Destroy(value Ticker) {
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

func (c FfiConverterTlsConfigRecord) Write(writer io.Writer, value TlsConfigRecord) {
	FfiConverterOptionalBytesINSTANCE.Write(writer, value.RootCertPem)
	FfiConverterBoolINSTANCE.Write(writer, value.AcceptInvalidCerts)
}

type FfiDestroyerTlsConfigRecord struct{}

func (_ FfiDestroyerTlsConfigRecord) Destroy(value TlsConfigRecord) {
	value.Destroy()
}

// Total trading statistics
type TotalStats struct {
	TradeValue       float64
	TradeVolume      int64
	TradeVolumeAtBid *int64
	TradeVolumeAtAsk *int64
	Transaction      *int64
	Time             *int64
}

func (r *TotalStats) Destroy() {
	FfiDestroyerFloat64{}.Destroy(r.TradeValue)
	FfiDestroyerInt64{}.Destroy(r.TradeVolume)
	FfiDestroyerOptionalInt64{}.Destroy(r.TradeVolumeAtBid)
	FfiDestroyerOptionalInt64{}.Destroy(r.TradeVolumeAtAsk)
	FfiDestroyerOptionalInt64{}.Destroy(r.Transaction)
	FfiDestroyerOptionalInt64{}.Destroy(r.Time)
}

type FfiConverterTotalStats struct{}

var FfiConverterTotalStatsINSTANCE = FfiConverterTotalStats{}

func (c FfiConverterTotalStats) Lift(rb RustBufferI) TotalStats {
	return LiftFromRustBuffer[TotalStats](c, rb)
}

func (c FfiConverterTotalStats) Read(reader io.Reader) TotalStats {
	return TotalStats{
		FfiConverterFloat64INSTANCE.Read(reader),
		FfiConverterInt64INSTANCE.Read(reader),
		FfiConverterOptionalInt64INSTANCE.Read(reader),
		FfiConverterOptionalInt64INSTANCE.Read(reader),
		FfiConverterOptionalInt64INSTANCE.Read(reader),
		FfiConverterOptionalInt64INSTANCE.Read(reader),
	}
}

func (c FfiConverterTotalStats) Lower(value TotalStats) C.RustBuffer {
	return LowerIntoRustBuffer[TotalStats](c, value)
}

func (c FfiConverterTotalStats) Write(writer io.Writer, value TotalStats) {
	FfiConverterFloat64INSTANCE.Write(writer, value.TradeValue)
	FfiConverterInt64INSTANCE.Write(writer, value.TradeVolume)
	FfiConverterOptionalInt64INSTANCE.Write(writer, value.TradeVolumeAtBid)
	FfiConverterOptionalInt64INSTANCE.Write(writer, value.TradeVolumeAtAsk)
	FfiConverterOptionalInt64INSTANCE.Write(writer, value.Transaction)
	FfiConverterOptionalInt64INSTANCE.Write(writer, value.Time)
}

type FfiDestroyerTotalStats struct{}

func (_ FfiDestroyerTotalStats) Destroy(value TotalStats) {
	value.Destroy()
}

// Single trade execution
type Trade struct {
	Bid   *float64
	Ask   *float64
	Price float64
	Size  int64
	Time  int64
}

func (r *Trade) Destroy() {
	FfiDestroyerOptionalFloat64{}.Destroy(r.Bid)
	FfiDestroyerOptionalFloat64{}.Destroy(r.Ask)
	FfiDestroyerFloat64{}.Destroy(r.Price)
	FfiDestroyerInt64{}.Destroy(r.Size)
	FfiDestroyerInt64{}.Destroy(r.Time)
}

type FfiConverterTrade struct{}

var FfiConverterTradeINSTANCE = FfiConverterTrade{}

func (c FfiConverterTrade) Lift(rb RustBufferI) Trade {
	return LiftFromRustBuffer[Trade](c, rb)
}

func (c FfiConverterTrade) Read(reader io.Reader) Trade {
	return Trade{
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterFloat64INSTANCE.Read(reader),
		FfiConverterInt64INSTANCE.Read(reader),
		FfiConverterInt64INSTANCE.Read(reader),
	}
}

func (c FfiConverterTrade) Lower(value Trade) C.RustBuffer {
	return LowerIntoRustBuffer[Trade](c, value)
}

func (c FfiConverterTrade) Write(writer io.Writer, value Trade) {
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.Bid)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.Ask)
	FfiConverterFloat64INSTANCE.Write(writer, value.Price)
	FfiConverterInt64INSTANCE.Write(writer, value.Size)
	FfiConverterInt64INSTANCE.Write(writer, value.Time)
}

type FfiDestroyerTrade struct{}

func (_ FfiDestroyerTrade) Destroy(value Trade) {
	value.Destroy()
}

// Trade execution info
type TradeInfo struct {
	Bid   *float64
	Ask   *float64
	Price float64
	Size  int64
	Time  int64
}

func (r *TradeInfo) Destroy() {
	FfiDestroyerOptionalFloat64{}.Destroy(r.Bid)
	FfiDestroyerOptionalFloat64{}.Destroy(r.Ask)
	FfiDestroyerFloat64{}.Destroy(r.Price)
	FfiDestroyerInt64{}.Destroy(r.Size)
	FfiDestroyerInt64{}.Destroy(r.Time)
}

type FfiConverterTradeInfo struct{}

var FfiConverterTradeInfoINSTANCE = FfiConverterTradeInfo{}

func (c FfiConverterTradeInfo) Lift(rb RustBufferI) TradeInfo {
	return LiftFromRustBuffer[TradeInfo](c, rb)
}

func (c FfiConverterTradeInfo) Read(reader io.Reader) TradeInfo {
	return TradeInfo{
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterOptionalFloat64INSTANCE.Read(reader),
		FfiConverterFloat64INSTANCE.Read(reader),
		FfiConverterInt64INSTANCE.Read(reader),
		FfiConverterInt64INSTANCE.Read(reader),
	}
}

func (c FfiConverterTradeInfo) Lower(value TradeInfo) C.RustBuffer {
	return LowerIntoRustBuffer[TradeInfo](c, value)
}

func (c FfiConverterTradeInfo) Write(writer io.Writer, value TradeInfo) {
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.Bid)
	FfiConverterOptionalFloat64INSTANCE.Write(writer, value.Ask)
	FfiConverterFloat64INSTANCE.Write(writer, value.Price)
	FfiConverterInt64INSTANCE.Write(writer, value.Size)
	FfiConverterInt64INSTANCE.Write(writer, value.Time)
}

type FfiDestroyerTradeInfo struct{}

func (_ FfiDestroyerTradeInfo) Destroy(value TradeInfo) {
	value.Destroy()
}

// Trades response
type TradesResponse struct {
	Date     string
	DataType *string
	Exchange *string
	Market   *string
	Symbol   string
	Data     []Trade
}

func (r *TradesResponse) Destroy() {
	FfiDestroyerString{}.Destroy(r.Date)
	FfiDestroyerOptionalString{}.Destroy(r.DataType)
	FfiDestroyerOptionalString{}.Destroy(r.Exchange)
	FfiDestroyerOptionalString{}.Destroy(r.Market)
	FfiDestroyerString{}.Destroy(r.Symbol)
	FfiDestroyerSequenceTrade{}.Destroy(r.Data)
}

type FfiConverterTradesResponse struct{}

var FfiConverterTradesResponseINSTANCE = FfiConverterTradesResponse{}

func (c FfiConverterTradesResponse) Lift(rb RustBufferI) TradesResponse {
	return LiftFromRustBuffer[TradesResponse](c, rb)
}

func (c FfiConverterTradesResponse) Read(reader io.Reader) TradesResponse {
	return TradesResponse{
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterSequenceTradeINSTANCE.Read(reader),
	}
}

func (c FfiConverterTradesResponse) Lower(value TradesResponse) C.RustBuffer {
	return LowerIntoRustBuffer[TradesResponse](c, value)
}

func (c FfiConverterTradesResponse) Write(writer io.Writer, value TradesResponse) {
	FfiConverterStringINSTANCE.Write(writer, value.Date)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.DataType)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Exchange)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Market)
	FfiConverterStringINSTANCE.Write(writer, value.Symbol)
	FfiConverterSequenceTradeINSTANCE.Write(writer, value.Data)
}

type FfiDestroyerTradesResponse struct{}

func (_ FfiDestroyerTradesResponse) Destroy(value TradesResponse) {
	value.Destroy()
}

// Trading halt status
type TradingHalt struct {
	IsHalted bool
	Time     *int64
}

func (r *TradingHalt) Destroy() {
	FfiDestroyerBool{}.Destroy(r.IsHalted)
	FfiDestroyerOptionalInt64{}.Destroy(r.Time)
}

type FfiConverterTradingHalt struct{}

var FfiConverterTradingHaltINSTANCE = FfiConverterTradingHalt{}

func (c FfiConverterTradingHalt) Lift(rb RustBufferI) TradingHalt {
	return LiftFromRustBuffer[TradingHalt](c, rb)
}

func (c FfiConverterTradingHalt) Read(reader io.Reader) TradingHalt {
	return TradingHalt{
		FfiConverterBoolINSTANCE.Read(reader),
		FfiConverterOptionalInt64INSTANCE.Read(reader),
	}
}

func (c FfiConverterTradingHalt) Lower(value TradingHalt) C.RustBuffer {
	return LowerIntoRustBuffer[TradingHalt](c, value)
}

func (c FfiConverterTradingHalt) Write(writer io.Writer, value TradingHalt) {
	FfiConverterBoolINSTANCE.Write(writer, value.IsHalted)
	FfiConverterOptionalInt64INSTANCE.Write(writer, value.Time)
}

type FfiDestroyerTradingHalt struct{}

func (_ FfiDestroyerTradingHalt) Destroy(value TradingHalt) {
	value.Destroy()
}

// Volume at a specific price level
type VolumeAtPrice struct {
	Price       float64
	Volume      int64
	VolumeAtBid *int64
	VolumeAtAsk *int64
}

func (r *VolumeAtPrice) Destroy() {
	FfiDestroyerFloat64{}.Destroy(r.Price)
	FfiDestroyerInt64{}.Destroy(r.Volume)
	FfiDestroyerOptionalInt64{}.Destroy(r.VolumeAtBid)
	FfiDestroyerOptionalInt64{}.Destroy(r.VolumeAtAsk)
}

type FfiConverterVolumeAtPrice struct{}

var FfiConverterVolumeAtPriceINSTANCE = FfiConverterVolumeAtPrice{}

func (c FfiConverterVolumeAtPrice) Lift(rb RustBufferI) VolumeAtPrice {
	return LiftFromRustBuffer[VolumeAtPrice](c, rb)
}

func (c FfiConverterVolumeAtPrice) Read(reader io.Reader) VolumeAtPrice {
	return VolumeAtPrice{
		FfiConverterFloat64INSTANCE.Read(reader),
		FfiConverterInt64INSTANCE.Read(reader),
		FfiConverterOptionalInt64INSTANCE.Read(reader),
		FfiConverterOptionalInt64INSTANCE.Read(reader),
	}
}

func (c FfiConverterVolumeAtPrice) Lower(value VolumeAtPrice) C.RustBuffer {
	return LowerIntoRustBuffer[VolumeAtPrice](c, value)
}

func (c FfiConverterVolumeAtPrice) Write(writer io.Writer, value VolumeAtPrice) {
	FfiConverterFloat64INSTANCE.Write(writer, value.Price)
	FfiConverterInt64INSTANCE.Write(writer, value.Volume)
	FfiConverterOptionalInt64INSTANCE.Write(writer, value.VolumeAtBid)
	FfiConverterOptionalInt64INSTANCE.Write(writer, value.VolumeAtAsk)
}

type FfiDestroyerVolumeAtPrice struct{}

func (_ FfiDestroyerVolumeAtPrice) Destroy(value VolumeAtPrice) {
	value.Destroy()
}

// Volumes response
type VolumesResponse struct {
	Date     string
	DataType *string
	Exchange *string
	Market   *string
	Symbol   string
	Data     []VolumeAtPrice
}

func (r *VolumesResponse) Destroy() {
	FfiDestroyerString{}.Destroy(r.Date)
	FfiDestroyerOptionalString{}.Destroy(r.DataType)
	FfiDestroyerOptionalString{}.Destroy(r.Exchange)
	FfiDestroyerOptionalString{}.Destroy(r.Market)
	FfiDestroyerString{}.Destroy(r.Symbol)
	FfiDestroyerSequenceVolumeAtPrice{}.Destroy(r.Data)
}

type FfiConverterVolumesResponse struct{}

var FfiConverterVolumesResponseINSTANCE = FfiConverterVolumesResponse{}

func (c FfiConverterVolumesResponse) Lift(rb RustBufferI) VolumesResponse {
	return LiftFromRustBuffer[VolumesResponse](c, rb)
}

func (c FfiConverterVolumesResponse) Read(reader io.Reader) VolumesResponse {
	return VolumesResponse{
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterOptionalStringINSTANCE.Read(reader),
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterSequenceVolumeAtPriceINSTANCE.Read(reader),
	}
}

func (c FfiConverterVolumesResponse) Lower(value VolumesResponse) C.RustBuffer {
	return LowerIntoRustBuffer[VolumesResponse](c, value)
}

func (c FfiConverterVolumesResponse) Write(writer io.Writer, value VolumesResponse) {
	FfiConverterStringINSTANCE.Write(writer, value.Date)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.DataType)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Exchange)
	FfiConverterOptionalStringINSTANCE.Write(writer, value.Market)
	FfiConverterStringINSTANCE.Write(writer, value.Symbol)
	FfiConverterSequenceVolumeAtPriceINSTANCE.Write(writer, value.Data)
}

type FfiDestroyerVolumesResponse struct{}

func (_ FfiDestroyerVolumesResponse) Destroy(value VolumesResponse) {
	value.Destroy()
}

// Error type for UniFFI bindings
//
// Maps to MarketDataError in the UDL file. Each variant becomes an exception
// in the target language with the error message preserved.
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
var ErrMarketDataErrorNetworkError = fmt.Errorf("MarketDataErrorNetworkError")
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
type MarketDataErrorNetworkError struct {
	Msg string
}

func NewMarketDataErrorNetworkError(
	msg string,
) *MarketDataError {
	return &MarketDataError{err: &MarketDataErrorNetworkError{
		Msg: msg}}
}

func (e MarketDataErrorNetworkError) destroy() {
	FfiDestroyerString{}.Destroy(e.Msg)
}

func (err MarketDataErrorNetworkError) Error() string {
	return fmt.Sprint("NetworkError",
		": ",

		"Msg=",
		err.Msg,
	)
}

func (self MarketDataErrorNetworkError) Is(target error) bool {
	return target == ErrMarketDataErrorNetworkError
}

type MarketDataErrorAuthError struct {
	Msg string
}

func NewMarketDataErrorAuthError(
	msg string,
) *MarketDataError {
	return &MarketDataError{err: &MarketDataErrorAuthError{
		Msg: msg}}
}

func (e MarketDataErrorAuthError) destroy() {
	FfiDestroyerString{}.Destroy(e.Msg)
}

func (err MarketDataErrorAuthError) Error() string {
	return fmt.Sprint("AuthError",
		": ",

		"Msg=",
		err.Msg,
	)
}

func (self MarketDataErrorAuthError) Is(target error) bool {
	return target == ErrMarketDataErrorAuthError
}

type MarketDataErrorRateLimitError struct {
	Msg string
}

func NewMarketDataErrorRateLimitError(
	msg string,
) *MarketDataError {
	return &MarketDataError{err: &MarketDataErrorRateLimitError{
		Msg: msg}}
}

func (e MarketDataErrorRateLimitError) destroy() {
	FfiDestroyerString{}.Destroy(e.Msg)
}

func (err MarketDataErrorRateLimitError) Error() string {
	return fmt.Sprint("RateLimitError",
		": ",

		"Msg=",
		err.Msg,
	)
}

func (self MarketDataErrorRateLimitError) Is(target error) bool {
	return target == ErrMarketDataErrorRateLimitError
}

type MarketDataErrorInvalidSymbol struct {
	Msg string
}

func NewMarketDataErrorInvalidSymbol(
	msg string,
) *MarketDataError {
	return &MarketDataError{err: &MarketDataErrorInvalidSymbol{
		Msg: msg}}
}

func (e MarketDataErrorInvalidSymbol) destroy() {
	FfiDestroyerString{}.Destroy(e.Msg)
}

func (err MarketDataErrorInvalidSymbol) Error() string {
	return fmt.Sprint("InvalidSymbol",
		": ",

		"Msg=",
		err.Msg,
	)
}

func (self MarketDataErrorInvalidSymbol) Is(target error) bool {
	return target == ErrMarketDataErrorInvalidSymbol
}

type MarketDataErrorParseError struct {
	Msg string
}

func NewMarketDataErrorParseError(
	msg string,
) *MarketDataError {
	return &MarketDataError{err: &MarketDataErrorParseError{
		Msg: msg}}
}

func (e MarketDataErrorParseError) destroy() {
	FfiDestroyerString{}.Destroy(e.Msg)
}

func (err MarketDataErrorParseError) Error() string {
	return fmt.Sprint("ParseError",
		": ",

		"Msg=",
		err.Msg,
	)
}

func (self MarketDataErrorParseError) Is(target error) bool {
	return target == ErrMarketDataErrorParseError
}

type MarketDataErrorTimeoutError struct {
	Msg string
}

func NewMarketDataErrorTimeoutError(
	msg string,
) *MarketDataError {
	return &MarketDataError{err: &MarketDataErrorTimeoutError{
		Msg: msg}}
}

func (e MarketDataErrorTimeoutError) destroy() {
	FfiDestroyerString{}.Destroy(e.Msg)
}

func (err MarketDataErrorTimeoutError) Error() string {
	return fmt.Sprint("TimeoutError",
		": ",

		"Msg=",
		err.Msg,
	)
}

func (self MarketDataErrorTimeoutError) Is(target error) bool {
	return target == ErrMarketDataErrorTimeoutError
}

type MarketDataErrorWebSocketError struct {
	Msg string
}

func NewMarketDataErrorWebSocketError(
	msg string,
) *MarketDataError {
	return &MarketDataError{err: &MarketDataErrorWebSocketError{
		Msg: msg}}
}

func (e MarketDataErrorWebSocketError) destroy() {
	FfiDestroyerString{}.Destroy(e.Msg)
}

func (err MarketDataErrorWebSocketError) Error() string {
	return fmt.Sprint("WebSocketError",
		": ",

		"Msg=",
		err.Msg,
	)
}

func (self MarketDataErrorWebSocketError) Is(target error) bool {
	return target == ErrMarketDataErrorWebSocketError
}

type MarketDataErrorClientClosed struct {
}

func NewMarketDataErrorClientClosed() *MarketDataError {
	return &MarketDataError{err: &MarketDataErrorClientClosed{}}
}

func (e MarketDataErrorClientClosed) destroy() {
}

func (err MarketDataErrorClientClosed) Error() string {
	return fmt.Sprint("ClientClosed")
}

func (self MarketDataErrorClientClosed) Is(target error) bool {
	return target == ErrMarketDataErrorClientClosed
}

type MarketDataErrorConfigError struct {
	Msg string
}

func NewMarketDataErrorConfigError(
	msg string,
) *MarketDataError {
	return &MarketDataError{err: &MarketDataErrorConfigError{
		Msg: msg}}
}

func (e MarketDataErrorConfigError) destroy() {
	FfiDestroyerString{}.Destroy(e.Msg)
}

func (err MarketDataErrorConfigError) Error() string {
	return fmt.Sprint("ConfigError",
		": ",

		"Msg=",
		err.Msg,
	)
}

func (self MarketDataErrorConfigError) Is(target error) bool {
	return target == ErrMarketDataErrorConfigError
}

type MarketDataErrorApiError struct {
	Msg string
}

func NewMarketDataErrorApiError(
	msg string,
) *MarketDataError {
	return &MarketDataError{err: &MarketDataErrorApiError{
		Msg: msg}}
}

func (e MarketDataErrorApiError) destroy() {
	FfiDestroyerString{}.Destroy(e.Msg)
}

func (err MarketDataErrorApiError) Error() string {
	return fmt.Sprint("ApiError",
		": ",

		"Msg=",
		err.Msg,
	)
}

func (self MarketDataErrorApiError) Is(target error) bool {
	return target == ErrMarketDataErrorApiError
}

type MarketDataErrorOther struct {
	Msg string
}

func NewMarketDataErrorOther(
	msg string,
) *MarketDataError {
	return &MarketDataError{err: &MarketDataErrorOther{
		Msg: msg}}
}

func (e MarketDataErrorOther) destroy() {
	FfiDestroyerString{}.Destroy(e.Msg)
}

func (err MarketDataErrorOther) Error() string {
	return fmt.Sprint("Other",
		": ",

		"Msg=",
		err.Msg,
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

func (c FfiConverterMarketDataError) Read(reader io.Reader) *MarketDataError {
	errorID := readUint32(reader)

	switch errorID {
	case 1:
		return &MarketDataError{&MarketDataErrorNetworkError{
			Msg: FfiConverterStringINSTANCE.Read(reader),
		}}
	case 2:
		return &MarketDataError{&MarketDataErrorAuthError{
			Msg: FfiConverterStringINSTANCE.Read(reader),
		}}
	case 3:
		return &MarketDataError{&MarketDataErrorRateLimitError{
			Msg: FfiConverterStringINSTANCE.Read(reader),
		}}
	case 4:
		return &MarketDataError{&MarketDataErrorInvalidSymbol{
			Msg: FfiConverterStringINSTANCE.Read(reader),
		}}
	case 5:
		return &MarketDataError{&MarketDataErrorParseError{
			Msg: FfiConverterStringINSTANCE.Read(reader),
		}}
	case 6:
		return &MarketDataError{&MarketDataErrorTimeoutError{
			Msg: FfiConverterStringINSTANCE.Read(reader),
		}}
	case 7:
		return &MarketDataError{&MarketDataErrorWebSocketError{
			Msg: FfiConverterStringINSTANCE.Read(reader),
		}}
	case 8:
		return &MarketDataError{&MarketDataErrorClientClosed{}}
	case 9:
		return &MarketDataError{&MarketDataErrorConfigError{
			Msg: FfiConverterStringINSTANCE.Read(reader),
		}}
	case 10:
		return &MarketDataError{&MarketDataErrorApiError{
			Msg: FfiConverterStringINSTANCE.Read(reader),
		}}
	case 11:
		return &MarketDataError{&MarketDataErrorOther{
			Msg: FfiConverterStringINSTANCE.Read(reader),
		}}
	default:
		panic(fmt.Sprintf("Unknown error code %d in FfiConverterMarketDataError.Read()", errorID))
	}
}

func (c FfiConverterMarketDataError) Write(writer io.Writer, value *MarketDataError) {
	switch variantValue := value.err.(type) {
	case *MarketDataErrorNetworkError:
		writeInt32(writer, 1)
		FfiConverterStringINSTANCE.Write(writer, variantValue.Msg)
	case *MarketDataErrorAuthError:
		writeInt32(writer, 2)
		FfiConverterStringINSTANCE.Write(writer, variantValue.Msg)
	case *MarketDataErrorRateLimitError:
		writeInt32(writer, 3)
		FfiConverterStringINSTANCE.Write(writer, variantValue.Msg)
	case *MarketDataErrorInvalidSymbol:
		writeInt32(writer, 4)
		FfiConverterStringINSTANCE.Write(writer, variantValue.Msg)
	case *MarketDataErrorParseError:
		writeInt32(writer, 5)
		FfiConverterStringINSTANCE.Write(writer, variantValue.Msg)
	case *MarketDataErrorTimeoutError:
		writeInt32(writer, 6)
		FfiConverterStringINSTANCE.Write(writer, variantValue.Msg)
	case *MarketDataErrorWebSocketError:
		writeInt32(writer, 7)
		FfiConverterStringINSTANCE.Write(writer, variantValue.Msg)
	case *MarketDataErrorClientClosed:
		writeInt32(writer, 8)
	case *MarketDataErrorConfigError:
		writeInt32(writer, 9)
		FfiConverterStringINSTANCE.Write(writer, variantValue.Msg)
	case *MarketDataErrorApiError:
		writeInt32(writer, 10)
		FfiConverterStringINSTANCE.Write(writer, variantValue.Msg)
	case *MarketDataErrorOther:
		writeInt32(writer, 11)
		FfiConverterStringINSTANCE.Write(writer, variantValue.Msg)
	default:
		_ = variantValue
		panic(fmt.Sprintf("invalid error value `%v` in FfiConverterMarketDataError.Write", value))
	}
}

type FfiDestroyerMarketDataError struct{}

func (_ FfiDestroyerMarketDataError) Destroy(value *MarketDataError) {
	switch variantValue := value.err.(type) {
	case MarketDataErrorNetworkError:
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

type FfiConverterOptionalInt64 struct{}

var FfiConverterOptionalInt64INSTANCE = FfiConverterOptionalInt64{}

func (c FfiConverterOptionalInt64) Lift(rb RustBufferI) *int64 {
	return LiftFromRustBuffer[*int64](c, rb)
}

func (_ FfiConverterOptionalInt64) Read(reader io.Reader) *int64 {
	if readInt8(reader) == 0 {
		return nil
	}
	temp := FfiConverterInt64INSTANCE.Read(reader)
	return &temp
}

func (c FfiConverterOptionalInt64) Lower(value *int64) C.RustBuffer {
	return LowerIntoRustBuffer[*int64](c, value)
}

func (_ FfiConverterOptionalInt64) Write(writer io.Writer, value *int64) {
	if value == nil {
		writeInt8(writer, 0)
	} else {
		writeInt8(writer, 1)
		FfiConverterInt64INSTANCE.Write(writer, *value)
	}
}

type FfiDestroyerOptionalInt64 struct{}

func (_ FfiDestroyerOptionalInt64) Destroy(value *int64) {
	if value != nil {
		FfiDestroyerInt64{}.Destroy(*value)
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

type FfiConverterOptionalFutOptLastTrade struct{}

var FfiConverterOptionalFutOptLastTradeINSTANCE = FfiConverterOptionalFutOptLastTrade{}

func (c FfiConverterOptionalFutOptLastTrade) Lift(rb RustBufferI) *FutOptLastTrade {
	return LiftFromRustBuffer[*FutOptLastTrade](c, rb)
}

func (_ FfiConverterOptionalFutOptLastTrade) Read(reader io.Reader) *FutOptLastTrade {
	if readInt8(reader) == 0 {
		return nil
	}
	temp := FfiConverterFutOptLastTradeINSTANCE.Read(reader)
	return &temp
}

func (c FfiConverterOptionalFutOptLastTrade) Lower(value *FutOptLastTrade) C.RustBuffer {
	return LowerIntoRustBuffer[*FutOptLastTrade](c, value)
}

func (_ FfiConverterOptionalFutOptLastTrade) Write(writer io.Writer, value *FutOptLastTrade) {
	if value == nil {
		writeInt8(writer, 0)
	} else {
		writeInt8(writer, 1)
		FfiConverterFutOptLastTradeINSTANCE.Write(writer, *value)
	}
}

type FfiDestroyerOptionalFutOptLastTrade struct{}

func (_ FfiDestroyerOptionalFutOptLastTrade) Destroy(value *FutOptLastTrade) {
	if value != nil {
		FfiDestroyerFutOptLastTrade{}.Destroy(*value)
	}
}

type FfiConverterOptionalFutOptTotalStats struct{}

var FfiConverterOptionalFutOptTotalStatsINSTANCE = FfiConverterOptionalFutOptTotalStats{}

func (c FfiConverterOptionalFutOptTotalStats) Lift(rb RustBufferI) *FutOptTotalStats {
	return LiftFromRustBuffer[*FutOptTotalStats](c, rb)
}

func (_ FfiConverterOptionalFutOptTotalStats) Read(reader io.Reader) *FutOptTotalStats {
	if readInt8(reader) == 0 {
		return nil
	}
	temp := FfiConverterFutOptTotalStatsINSTANCE.Read(reader)
	return &temp
}

func (c FfiConverterOptionalFutOptTotalStats) Lower(value *FutOptTotalStats) C.RustBuffer {
	return LowerIntoRustBuffer[*FutOptTotalStats](c, value)
}

func (_ FfiConverterOptionalFutOptTotalStats) Write(writer io.Writer, value *FutOptTotalStats) {
	if value == nil {
		writeInt8(writer, 0)
	} else {
		writeInt8(writer, 1)
		FfiConverterFutOptTotalStatsINSTANCE.Write(writer, *value)
	}
}

type FfiDestroyerOptionalFutOptTotalStats struct{}

func (_ FfiDestroyerOptionalFutOptTotalStats) Destroy(value *FutOptTotalStats) {
	if value != nil {
		FfiDestroyerFutOptTotalStats{}.Destroy(*value)
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

type FfiConverterOptionalTotalStats struct{}

var FfiConverterOptionalTotalStatsINSTANCE = FfiConverterOptionalTotalStats{}

func (c FfiConverterOptionalTotalStats) Lift(rb RustBufferI) *TotalStats {
	return LiftFromRustBuffer[*TotalStats](c, rb)
}

func (_ FfiConverterOptionalTotalStats) Read(reader io.Reader) *TotalStats {
	if readInt8(reader) == 0 {
		return nil
	}
	temp := FfiConverterTotalStatsINSTANCE.Read(reader)
	return &temp
}

func (c FfiConverterOptionalTotalStats) Lower(value *TotalStats) C.RustBuffer {
	return LowerIntoRustBuffer[*TotalStats](c, value)
}

func (_ FfiConverterOptionalTotalStats) Write(writer io.Writer, value *TotalStats) {
	if value == nil {
		writeInt8(writer, 0)
	} else {
		writeInt8(writer, 1)
		FfiConverterTotalStatsINSTANCE.Write(writer, *value)
	}
}

type FfiDestroyerOptionalTotalStats struct{}

func (_ FfiDestroyerOptionalTotalStats) Destroy(value *TotalStats) {
	if value != nil {
		FfiDestroyerTotalStats{}.Destroy(*value)
	}
}

type FfiConverterOptionalTradeInfo struct{}

var FfiConverterOptionalTradeInfoINSTANCE = FfiConverterOptionalTradeInfo{}

func (c FfiConverterOptionalTradeInfo) Lift(rb RustBufferI) *TradeInfo {
	return LiftFromRustBuffer[*TradeInfo](c, rb)
}

func (_ FfiConverterOptionalTradeInfo) Read(reader io.Reader) *TradeInfo {
	if readInt8(reader) == 0 {
		return nil
	}
	temp := FfiConverterTradeInfoINSTANCE.Read(reader)
	return &temp
}

func (c FfiConverterOptionalTradeInfo) Lower(value *TradeInfo) C.RustBuffer {
	return LowerIntoRustBuffer[*TradeInfo](c, value)
}

func (_ FfiConverterOptionalTradeInfo) Write(writer io.Writer, value *TradeInfo) {
	if value == nil {
		writeInt8(writer, 0)
	} else {
		writeInt8(writer, 1)
		FfiConverterTradeInfoINSTANCE.Write(writer, *value)
	}
}

type FfiDestroyerOptionalTradeInfo struct{}

func (_ FfiDestroyerOptionalTradeInfo) Destroy(value *TradeInfo) {
	if value != nil {
		FfiDestroyerTradeInfo{}.Destroy(*value)
	}
}

type FfiConverterOptionalTradingHalt struct{}

var FfiConverterOptionalTradingHaltINSTANCE = FfiConverterOptionalTradingHalt{}

func (c FfiConverterOptionalTradingHalt) Lift(rb RustBufferI) *TradingHalt {
	return LiftFromRustBuffer[*TradingHalt](c, rb)
}

func (_ FfiConverterOptionalTradingHalt) Read(reader io.Reader) *TradingHalt {
	if readInt8(reader) == 0 {
		return nil
	}
	temp := FfiConverterTradingHaltINSTANCE.Read(reader)
	return &temp
}

func (c FfiConverterOptionalTradingHalt) Lower(value *TradingHalt) C.RustBuffer {
	return LowerIntoRustBuffer[*TradingHalt](c, value)
}

func (_ FfiConverterOptionalTradingHalt) Write(writer io.Writer, value *TradingHalt) {
	if value == nil {
		writeInt8(writer, 0)
	} else {
		writeInt8(writer, 1)
		FfiConverterTradingHaltINSTANCE.Write(writer, *value)
	}
}

type FfiDestroyerOptionalTradingHalt struct{}

func (_ FfiDestroyerOptionalTradingHalt) Destroy(value *TradingHalt) {
	if value != nil {
		FfiDestroyerTradingHalt{}.Destroy(*value)
	}
}

type FfiConverterSequenceActive struct{}

var FfiConverterSequenceActiveINSTANCE = FfiConverterSequenceActive{}

func (c FfiConverterSequenceActive) Lift(rb RustBufferI) []Active {
	return LiftFromRustBuffer[[]Active](c, rb)
}

func (c FfiConverterSequenceActive) Read(reader io.Reader) []Active {
	length := readInt32(reader)
	if length == 0 {
		return nil
	}
	result := make([]Active, 0, length)
	for i := int32(0); i < length; i++ {
		result = append(result, FfiConverterActiveINSTANCE.Read(reader))
	}
	return result
}

func (c FfiConverterSequenceActive) Lower(value []Active) C.RustBuffer {
	return LowerIntoRustBuffer[[]Active](c, value)
}

func (c FfiConverterSequenceActive) Write(writer io.Writer, value []Active) {
	if len(value) > math.MaxInt32 {
		panic("[]Active is too large to fit into Int32")
	}

	writeInt32(writer, int32(len(value)))
	for _, item := range value {
		FfiConverterActiveINSTANCE.Write(writer, item)
	}
}

type FfiDestroyerSequenceActive struct{}

func (FfiDestroyerSequenceActive) Destroy(sequence []Active) {
	for _, value := range sequence {
		FfiDestroyerActive{}.Destroy(value)
	}
}

type FfiConverterSequenceBbDataPoint struct{}

var FfiConverterSequenceBbDataPointINSTANCE = FfiConverterSequenceBbDataPoint{}

func (c FfiConverterSequenceBbDataPoint) Lift(rb RustBufferI) []BbDataPoint {
	return LiftFromRustBuffer[[]BbDataPoint](c, rb)
}

func (c FfiConverterSequenceBbDataPoint) Read(reader io.Reader) []BbDataPoint {
	length := readInt32(reader)
	if length == 0 {
		return nil
	}
	result := make([]BbDataPoint, 0, length)
	for i := int32(0); i < length; i++ {
		result = append(result, FfiConverterBbDataPointINSTANCE.Read(reader))
	}
	return result
}

func (c FfiConverterSequenceBbDataPoint) Lower(value []BbDataPoint) C.RustBuffer {
	return LowerIntoRustBuffer[[]BbDataPoint](c, value)
}

func (c FfiConverterSequenceBbDataPoint) Write(writer io.Writer, value []BbDataPoint) {
	if len(value) > math.MaxInt32 {
		panic("[]BbDataPoint is too large to fit into Int32")
	}

	writeInt32(writer, int32(len(value)))
	for _, item := range value {
		FfiConverterBbDataPointINSTANCE.Write(writer, item)
	}
}

type FfiDestroyerSequenceBbDataPoint struct{}

func (FfiDestroyerSequenceBbDataPoint) Destroy(sequence []BbDataPoint) {
	for _, value := range sequence {
		FfiDestroyerBbDataPoint{}.Destroy(value)
	}
}

type FfiConverterSequenceCapitalChange struct{}

var FfiConverterSequenceCapitalChangeINSTANCE = FfiConverterSequenceCapitalChange{}

func (c FfiConverterSequenceCapitalChange) Lift(rb RustBufferI) []CapitalChange {
	return LiftFromRustBuffer[[]CapitalChange](c, rb)
}

func (c FfiConverterSequenceCapitalChange) Read(reader io.Reader) []CapitalChange {
	length := readInt32(reader)
	if length == 0 {
		return nil
	}
	result := make([]CapitalChange, 0, length)
	for i := int32(0); i < length; i++ {
		result = append(result, FfiConverterCapitalChangeINSTANCE.Read(reader))
	}
	return result
}

func (c FfiConverterSequenceCapitalChange) Lower(value []CapitalChange) C.RustBuffer {
	return LowerIntoRustBuffer[[]CapitalChange](c, value)
}

func (c FfiConverterSequenceCapitalChange) Write(writer io.Writer, value []CapitalChange) {
	if len(value) > math.MaxInt32 {
		panic("[]CapitalChange is too large to fit into Int32")
	}

	writeInt32(writer, int32(len(value)))
	for _, item := range value {
		FfiConverterCapitalChangeINSTANCE.Write(writer, item)
	}
}

type FfiDestroyerSequenceCapitalChange struct{}

func (FfiDestroyerSequenceCapitalChange) Destroy(sequence []CapitalChange) {
	for _, value := range sequence {
		FfiDestroyerCapitalChange{}.Destroy(value)
	}
}

type FfiConverterSequenceDividend struct{}

var FfiConverterSequenceDividendINSTANCE = FfiConverterSequenceDividend{}

func (c FfiConverterSequenceDividend) Lift(rb RustBufferI) []Dividend {
	return LiftFromRustBuffer[[]Dividend](c, rb)
}

func (c FfiConverterSequenceDividend) Read(reader io.Reader) []Dividend {
	length := readInt32(reader)
	if length == 0 {
		return nil
	}
	result := make([]Dividend, 0, length)
	for i := int32(0); i < length; i++ {
		result = append(result, FfiConverterDividendINSTANCE.Read(reader))
	}
	return result
}

func (c FfiConverterSequenceDividend) Lower(value []Dividend) C.RustBuffer {
	return LowerIntoRustBuffer[[]Dividend](c, value)
}

func (c FfiConverterSequenceDividend) Write(writer io.Writer, value []Dividend) {
	if len(value) > math.MaxInt32 {
		panic("[]Dividend is too large to fit into Int32")
	}

	writeInt32(writer, int32(len(value)))
	for _, item := range value {
		FfiConverterDividendINSTANCE.Write(writer, item)
	}
}

type FfiDestroyerSequenceDividend struct{}

func (FfiDestroyerSequenceDividend) Destroy(sequence []Dividend) {
	for _, value := range sequence {
		FfiDestroyerDividend{}.Destroy(value)
	}
}

type FfiConverterSequenceFutOptDailyData struct{}

var FfiConverterSequenceFutOptDailyDataINSTANCE = FfiConverterSequenceFutOptDailyData{}

func (c FfiConverterSequenceFutOptDailyData) Lift(rb RustBufferI) []FutOptDailyData {
	return LiftFromRustBuffer[[]FutOptDailyData](c, rb)
}

func (c FfiConverterSequenceFutOptDailyData) Read(reader io.Reader) []FutOptDailyData {
	length := readInt32(reader)
	if length == 0 {
		return nil
	}
	result := make([]FutOptDailyData, 0, length)
	for i := int32(0); i < length; i++ {
		result = append(result, FfiConverterFutOptDailyDataINSTANCE.Read(reader))
	}
	return result
}

func (c FfiConverterSequenceFutOptDailyData) Lower(value []FutOptDailyData) C.RustBuffer {
	return LowerIntoRustBuffer[[]FutOptDailyData](c, value)
}

func (c FfiConverterSequenceFutOptDailyData) Write(writer io.Writer, value []FutOptDailyData) {
	if len(value) > math.MaxInt32 {
		panic("[]FutOptDailyData is too large to fit into Int32")
	}

	writeInt32(writer, int32(len(value)))
	for _, item := range value {
		FfiConverterFutOptDailyDataINSTANCE.Write(writer, item)
	}
}

type FfiDestroyerSequenceFutOptDailyData struct{}

func (FfiDestroyerSequenceFutOptDailyData) Destroy(sequence []FutOptDailyData) {
	for _, value := range sequence {
		FfiDestroyerFutOptDailyData{}.Destroy(value)
	}
}

type FfiConverterSequenceFutOptHistoricalCandle struct{}

var FfiConverterSequenceFutOptHistoricalCandleINSTANCE = FfiConverterSequenceFutOptHistoricalCandle{}

func (c FfiConverterSequenceFutOptHistoricalCandle) Lift(rb RustBufferI) []FutOptHistoricalCandle {
	return LiftFromRustBuffer[[]FutOptHistoricalCandle](c, rb)
}

func (c FfiConverterSequenceFutOptHistoricalCandle) Read(reader io.Reader) []FutOptHistoricalCandle {
	length := readInt32(reader)
	if length == 0 {
		return nil
	}
	result := make([]FutOptHistoricalCandle, 0, length)
	for i := int32(0); i < length; i++ {
		result = append(result, FfiConverterFutOptHistoricalCandleINSTANCE.Read(reader))
	}
	return result
}

func (c FfiConverterSequenceFutOptHistoricalCandle) Lower(value []FutOptHistoricalCandle) C.RustBuffer {
	return LowerIntoRustBuffer[[]FutOptHistoricalCandle](c, value)
}

func (c FfiConverterSequenceFutOptHistoricalCandle) Write(writer io.Writer, value []FutOptHistoricalCandle) {
	if len(value) > math.MaxInt32 {
		panic("[]FutOptHistoricalCandle is too large to fit into Int32")
	}

	writeInt32(writer, int32(len(value)))
	for _, item := range value {
		FfiConverterFutOptHistoricalCandleINSTANCE.Write(writer, item)
	}
}

type FfiDestroyerSequenceFutOptHistoricalCandle struct{}

func (FfiDestroyerSequenceFutOptHistoricalCandle) Destroy(sequence []FutOptHistoricalCandle) {
	for _, value := range sequence {
		FfiDestroyerFutOptHistoricalCandle{}.Destroy(value)
	}
}

type FfiConverterSequenceFutOptPriceLevel struct{}

var FfiConverterSequenceFutOptPriceLevelINSTANCE = FfiConverterSequenceFutOptPriceLevel{}

func (c FfiConverterSequenceFutOptPriceLevel) Lift(rb RustBufferI) []FutOptPriceLevel {
	return LiftFromRustBuffer[[]FutOptPriceLevel](c, rb)
}

func (c FfiConverterSequenceFutOptPriceLevel) Read(reader io.Reader) []FutOptPriceLevel {
	length := readInt32(reader)
	if length == 0 {
		return nil
	}
	result := make([]FutOptPriceLevel, 0, length)
	for i := int32(0); i < length; i++ {
		result = append(result, FfiConverterFutOptPriceLevelINSTANCE.Read(reader))
	}
	return result
}

func (c FfiConverterSequenceFutOptPriceLevel) Lower(value []FutOptPriceLevel) C.RustBuffer {
	return LowerIntoRustBuffer[[]FutOptPriceLevel](c, value)
}

func (c FfiConverterSequenceFutOptPriceLevel) Write(writer io.Writer, value []FutOptPriceLevel) {
	if len(value) > math.MaxInt32 {
		panic("[]FutOptPriceLevel is too large to fit into Int32")
	}

	writeInt32(writer, int32(len(value)))
	for _, item := range value {
		FfiConverterFutOptPriceLevelINSTANCE.Write(writer, item)
	}
}

type FfiDestroyerSequenceFutOptPriceLevel struct{}

func (FfiDestroyerSequenceFutOptPriceLevel) Destroy(sequence []FutOptPriceLevel) {
	for _, value := range sequence {
		FfiDestroyerFutOptPriceLevel{}.Destroy(value)
	}
}

type FfiConverterSequenceFutOptTicker struct{}

var FfiConverterSequenceFutOptTickerINSTANCE = FfiConverterSequenceFutOptTicker{}

func (c FfiConverterSequenceFutOptTicker) Lift(rb RustBufferI) []FutOptTicker {
	return LiftFromRustBuffer[[]FutOptTicker](c, rb)
}

func (c FfiConverterSequenceFutOptTicker) Read(reader io.Reader) []FutOptTicker {
	length := readInt32(reader)
	if length == 0 {
		return nil
	}
	result := make([]FutOptTicker, 0, length)
	for i := int32(0); i < length; i++ {
		result = append(result, FfiConverterFutOptTickerINSTANCE.Read(reader))
	}
	return result
}

func (c FfiConverterSequenceFutOptTicker) Lower(value []FutOptTicker) C.RustBuffer {
	return LowerIntoRustBuffer[[]FutOptTicker](c, value)
}

func (c FfiConverterSequenceFutOptTicker) Write(writer io.Writer, value []FutOptTicker) {
	if len(value) > math.MaxInt32 {
		panic("[]FutOptTicker is too large to fit into Int32")
	}

	writeInt32(writer, int32(len(value)))
	for _, item := range value {
		FfiConverterFutOptTickerINSTANCE.Write(writer, item)
	}
}

type FfiDestroyerSequenceFutOptTicker struct{}

func (FfiDestroyerSequenceFutOptTicker) Destroy(sequence []FutOptTicker) {
	for _, value := range sequence {
		FfiDestroyerFutOptTicker{}.Destroy(value)
	}
}

type FfiConverterSequenceHistoricalCandle struct{}

var FfiConverterSequenceHistoricalCandleINSTANCE = FfiConverterSequenceHistoricalCandle{}

func (c FfiConverterSequenceHistoricalCandle) Lift(rb RustBufferI) []HistoricalCandle {
	return LiftFromRustBuffer[[]HistoricalCandle](c, rb)
}

func (c FfiConverterSequenceHistoricalCandle) Read(reader io.Reader) []HistoricalCandle {
	length := readInt32(reader)
	if length == 0 {
		return nil
	}
	result := make([]HistoricalCandle, 0, length)
	for i := int32(0); i < length; i++ {
		result = append(result, FfiConverterHistoricalCandleINSTANCE.Read(reader))
	}
	return result
}

func (c FfiConverterSequenceHistoricalCandle) Lower(value []HistoricalCandle) C.RustBuffer {
	return LowerIntoRustBuffer[[]HistoricalCandle](c, value)
}

func (c FfiConverterSequenceHistoricalCandle) Write(writer io.Writer, value []HistoricalCandle) {
	if len(value) > math.MaxInt32 {
		panic("[]HistoricalCandle is too large to fit into Int32")
	}

	writeInt32(writer, int32(len(value)))
	for _, item := range value {
		FfiConverterHistoricalCandleINSTANCE.Write(writer, item)
	}
}

type FfiDestroyerSequenceHistoricalCandle struct{}

func (FfiDestroyerSequenceHistoricalCandle) Destroy(sequence []HistoricalCandle) {
	for _, value := range sequence {
		FfiDestroyerHistoricalCandle{}.Destroy(value)
	}
}

type FfiConverterSequenceIntradayCandle struct{}

var FfiConverterSequenceIntradayCandleINSTANCE = FfiConverterSequenceIntradayCandle{}

func (c FfiConverterSequenceIntradayCandle) Lift(rb RustBufferI) []IntradayCandle {
	return LiftFromRustBuffer[[]IntradayCandle](c, rb)
}

func (c FfiConverterSequenceIntradayCandle) Read(reader io.Reader) []IntradayCandle {
	length := readInt32(reader)
	if length == 0 {
		return nil
	}
	result := make([]IntradayCandle, 0, length)
	for i := int32(0); i < length; i++ {
		result = append(result, FfiConverterIntradayCandleINSTANCE.Read(reader))
	}
	return result
}

func (c FfiConverterSequenceIntradayCandle) Lower(value []IntradayCandle) C.RustBuffer {
	return LowerIntoRustBuffer[[]IntradayCandle](c, value)
}

func (c FfiConverterSequenceIntradayCandle) Write(writer io.Writer, value []IntradayCandle) {
	if len(value) > math.MaxInt32 {
		panic("[]IntradayCandle is too large to fit into Int32")
	}

	writeInt32(writer, int32(len(value)))
	for _, item := range value {
		FfiConverterIntradayCandleINSTANCE.Write(writer, item)
	}
}

type FfiDestroyerSequenceIntradayCandle struct{}

func (FfiDestroyerSequenceIntradayCandle) Destroy(sequence []IntradayCandle) {
	for _, value := range sequence {
		FfiDestroyerIntradayCandle{}.Destroy(value)
	}
}

type FfiConverterSequenceKdjDataPoint struct{}

var FfiConverterSequenceKdjDataPointINSTANCE = FfiConverterSequenceKdjDataPoint{}

func (c FfiConverterSequenceKdjDataPoint) Lift(rb RustBufferI) []KdjDataPoint {
	return LiftFromRustBuffer[[]KdjDataPoint](c, rb)
}

func (c FfiConverterSequenceKdjDataPoint) Read(reader io.Reader) []KdjDataPoint {
	length := readInt32(reader)
	if length == 0 {
		return nil
	}
	result := make([]KdjDataPoint, 0, length)
	for i := int32(0); i < length; i++ {
		result = append(result, FfiConverterKdjDataPointINSTANCE.Read(reader))
	}
	return result
}

func (c FfiConverterSequenceKdjDataPoint) Lower(value []KdjDataPoint) C.RustBuffer {
	return LowerIntoRustBuffer[[]KdjDataPoint](c, value)
}

func (c FfiConverterSequenceKdjDataPoint) Write(writer io.Writer, value []KdjDataPoint) {
	if len(value) > math.MaxInt32 {
		panic("[]KdjDataPoint is too large to fit into Int32")
	}

	writeInt32(writer, int32(len(value)))
	for _, item := range value {
		FfiConverterKdjDataPointINSTANCE.Write(writer, item)
	}
}

type FfiDestroyerSequenceKdjDataPoint struct{}

func (FfiDestroyerSequenceKdjDataPoint) Destroy(sequence []KdjDataPoint) {
	for _, value := range sequence {
		FfiDestroyerKdjDataPoint{}.Destroy(value)
	}
}

type FfiConverterSequenceListingApplicant struct{}

var FfiConverterSequenceListingApplicantINSTANCE = FfiConverterSequenceListingApplicant{}

func (c FfiConverterSequenceListingApplicant) Lift(rb RustBufferI) []ListingApplicant {
	return LiftFromRustBuffer[[]ListingApplicant](c, rb)
}

func (c FfiConverterSequenceListingApplicant) Read(reader io.Reader) []ListingApplicant {
	length := readInt32(reader)
	if length == 0 {
		return nil
	}
	result := make([]ListingApplicant, 0, length)
	for i := int32(0); i < length; i++ {
		result = append(result, FfiConverterListingApplicantINSTANCE.Read(reader))
	}
	return result
}

func (c FfiConverterSequenceListingApplicant) Lower(value []ListingApplicant) C.RustBuffer {
	return LowerIntoRustBuffer[[]ListingApplicant](c, value)
}

func (c FfiConverterSequenceListingApplicant) Write(writer io.Writer, value []ListingApplicant) {
	if len(value) > math.MaxInt32 {
		panic("[]ListingApplicant is too large to fit into Int32")
	}

	writeInt32(writer, int32(len(value)))
	for _, item := range value {
		FfiConverterListingApplicantINSTANCE.Write(writer, item)
	}
}

type FfiDestroyerSequenceListingApplicant struct{}

func (FfiDestroyerSequenceListingApplicant) Destroy(sequence []ListingApplicant) {
	for _, value := range sequence {
		FfiDestroyerListingApplicant{}.Destroy(value)
	}
}

type FfiConverterSequenceMacdDataPoint struct{}

var FfiConverterSequenceMacdDataPointINSTANCE = FfiConverterSequenceMacdDataPoint{}

func (c FfiConverterSequenceMacdDataPoint) Lift(rb RustBufferI) []MacdDataPoint {
	return LiftFromRustBuffer[[]MacdDataPoint](c, rb)
}

func (c FfiConverterSequenceMacdDataPoint) Read(reader io.Reader) []MacdDataPoint {
	length := readInt32(reader)
	if length == 0 {
		return nil
	}
	result := make([]MacdDataPoint, 0, length)
	for i := int32(0); i < length; i++ {
		result = append(result, FfiConverterMacdDataPointINSTANCE.Read(reader))
	}
	return result
}

func (c FfiConverterSequenceMacdDataPoint) Lower(value []MacdDataPoint) C.RustBuffer {
	return LowerIntoRustBuffer[[]MacdDataPoint](c, value)
}

func (c FfiConverterSequenceMacdDataPoint) Write(writer io.Writer, value []MacdDataPoint) {
	if len(value) > math.MaxInt32 {
		panic("[]MacdDataPoint is too large to fit into Int32")
	}

	writeInt32(writer, int32(len(value)))
	for _, item := range value {
		FfiConverterMacdDataPointINSTANCE.Write(writer, item)
	}
}

type FfiDestroyerSequenceMacdDataPoint struct{}

func (FfiDestroyerSequenceMacdDataPoint) Destroy(sequence []MacdDataPoint) {
	for _, value := range sequence {
		FfiDestroyerMacdDataPoint{}.Destroy(value)
	}
}

type FfiConverterSequenceMover struct{}

var FfiConverterSequenceMoverINSTANCE = FfiConverterSequenceMover{}

func (c FfiConverterSequenceMover) Lift(rb RustBufferI) []Mover {
	return LiftFromRustBuffer[[]Mover](c, rb)
}

func (c FfiConverterSequenceMover) Read(reader io.Reader) []Mover {
	length := readInt32(reader)
	if length == 0 {
		return nil
	}
	result := make([]Mover, 0, length)
	for i := int32(0); i < length; i++ {
		result = append(result, FfiConverterMoverINSTANCE.Read(reader))
	}
	return result
}

func (c FfiConverterSequenceMover) Lower(value []Mover) C.RustBuffer {
	return LowerIntoRustBuffer[[]Mover](c, value)
}

func (c FfiConverterSequenceMover) Write(writer io.Writer, value []Mover) {
	if len(value) > math.MaxInt32 {
		panic("[]Mover is too large to fit into Int32")
	}

	writeInt32(writer, int32(len(value)))
	for _, item := range value {
		FfiConverterMoverINSTANCE.Write(writer, item)
	}
}

type FfiDestroyerSequenceMover struct{}

func (FfiDestroyerSequenceMover) Destroy(sequence []Mover) {
	for _, value := range sequence {
		FfiDestroyerMover{}.Destroy(value)
	}
}

type FfiConverterSequencePriceLevel struct{}

var FfiConverterSequencePriceLevelINSTANCE = FfiConverterSequencePriceLevel{}

func (c FfiConverterSequencePriceLevel) Lift(rb RustBufferI) []PriceLevel {
	return LiftFromRustBuffer[[]PriceLevel](c, rb)
}

func (c FfiConverterSequencePriceLevel) Read(reader io.Reader) []PriceLevel {
	length := readInt32(reader)
	if length == 0 {
		return nil
	}
	result := make([]PriceLevel, 0, length)
	for i := int32(0); i < length; i++ {
		result = append(result, FfiConverterPriceLevelINSTANCE.Read(reader))
	}
	return result
}

func (c FfiConverterSequencePriceLevel) Lower(value []PriceLevel) C.RustBuffer {
	return LowerIntoRustBuffer[[]PriceLevel](c, value)
}

func (c FfiConverterSequencePriceLevel) Write(writer io.Writer, value []PriceLevel) {
	if len(value) > math.MaxInt32 {
		panic("[]PriceLevel is too large to fit into Int32")
	}

	writeInt32(writer, int32(len(value)))
	for _, item := range value {
		FfiConverterPriceLevelINSTANCE.Write(writer, item)
	}
}

type FfiDestroyerSequencePriceLevel struct{}

func (FfiDestroyerSequencePriceLevel) Destroy(sequence []PriceLevel) {
	for _, value := range sequence {
		FfiDestroyerPriceLevel{}.Destroy(value)
	}
}

type FfiConverterSequenceProduct struct{}

var FfiConverterSequenceProductINSTANCE = FfiConverterSequenceProduct{}

func (c FfiConverterSequenceProduct) Lift(rb RustBufferI) []Product {
	return LiftFromRustBuffer[[]Product](c, rb)
}

func (c FfiConverterSequenceProduct) Read(reader io.Reader) []Product {
	length := readInt32(reader)
	if length == 0 {
		return nil
	}
	result := make([]Product, 0, length)
	for i := int32(0); i < length; i++ {
		result = append(result, FfiConverterProductINSTANCE.Read(reader))
	}
	return result
}

func (c FfiConverterSequenceProduct) Lower(value []Product) C.RustBuffer {
	return LowerIntoRustBuffer[[]Product](c, value)
}

func (c FfiConverterSequenceProduct) Write(writer io.Writer, value []Product) {
	if len(value) > math.MaxInt32 {
		panic("[]Product is too large to fit into Int32")
	}

	writeInt32(writer, int32(len(value)))
	for _, item := range value {
		FfiConverterProductINSTANCE.Write(writer, item)
	}
}

type FfiDestroyerSequenceProduct struct{}

func (FfiDestroyerSequenceProduct) Destroy(sequence []Product) {
	for _, value := range sequence {
		FfiDestroyerProduct{}.Destroy(value)
	}
}

type FfiConverterSequenceRsiDataPoint struct{}

var FfiConverterSequenceRsiDataPointINSTANCE = FfiConverterSequenceRsiDataPoint{}

func (c FfiConverterSequenceRsiDataPoint) Lift(rb RustBufferI) []RsiDataPoint {
	return LiftFromRustBuffer[[]RsiDataPoint](c, rb)
}

func (c FfiConverterSequenceRsiDataPoint) Read(reader io.Reader) []RsiDataPoint {
	length := readInt32(reader)
	if length == 0 {
		return nil
	}
	result := make([]RsiDataPoint, 0, length)
	for i := int32(0); i < length; i++ {
		result = append(result, FfiConverterRsiDataPointINSTANCE.Read(reader))
	}
	return result
}

func (c FfiConverterSequenceRsiDataPoint) Lower(value []RsiDataPoint) C.RustBuffer {
	return LowerIntoRustBuffer[[]RsiDataPoint](c, value)
}

func (c FfiConverterSequenceRsiDataPoint) Write(writer io.Writer, value []RsiDataPoint) {
	if len(value) > math.MaxInt32 {
		panic("[]RsiDataPoint is too large to fit into Int32")
	}

	writeInt32(writer, int32(len(value)))
	for _, item := range value {
		FfiConverterRsiDataPointINSTANCE.Write(writer, item)
	}
}

type FfiDestroyerSequenceRsiDataPoint struct{}

func (FfiDestroyerSequenceRsiDataPoint) Destroy(sequence []RsiDataPoint) {
	for _, value := range sequence {
		FfiDestroyerRsiDataPoint{}.Destroy(value)
	}
}

type FfiConverterSequenceSmaDataPoint struct{}

var FfiConverterSequenceSmaDataPointINSTANCE = FfiConverterSequenceSmaDataPoint{}

func (c FfiConverterSequenceSmaDataPoint) Lift(rb RustBufferI) []SmaDataPoint {
	return LiftFromRustBuffer[[]SmaDataPoint](c, rb)
}

func (c FfiConverterSequenceSmaDataPoint) Read(reader io.Reader) []SmaDataPoint {
	length := readInt32(reader)
	if length == 0 {
		return nil
	}
	result := make([]SmaDataPoint, 0, length)
	for i := int32(0); i < length; i++ {
		result = append(result, FfiConverterSmaDataPointINSTANCE.Read(reader))
	}
	return result
}

func (c FfiConverterSequenceSmaDataPoint) Lower(value []SmaDataPoint) C.RustBuffer {
	return LowerIntoRustBuffer[[]SmaDataPoint](c, value)
}

func (c FfiConverterSequenceSmaDataPoint) Write(writer io.Writer, value []SmaDataPoint) {
	if len(value) > math.MaxInt32 {
		panic("[]SmaDataPoint is too large to fit into Int32")
	}

	writeInt32(writer, int32(len(value)))
	for _, item := range value {
		FfiConverterSmaDataPointINSTANCE.Write(writer, item)
	}
}

type FfiDestroyerSequenceSmaDataPoint struct{}

func (FfiDestroyerSequenceSmaDataPoint) Destroy(sequence []SmaDataPoint) {
	for _, value := range sequence {
		FfiDestroyerSmaDataPoint{}.Destroy(value)
	}
}

type FfiConverterSequenceSnapshotQuote struct{}

var FfiConverterSequenceSnapshotQuoteINSTANCE = FfiConverterSequenceSnapshotQuote{}

func (c FfiConverterSequenceSnapshotQuote) Lift(rb RustBufferI) []SnapshotQuote {
	return LiftFromRustBuffer[[]SnapshotQuote](c, rb)
}

func (c FfiConverterSequenceSnapshotQuote) Read(reader io.Reader) []SnapshotQuote {
	length := readInt32(reader)
	if length == 0 {
		return nil
	}
	result := make([]SnapshotQuote, 0, length)
	for i := int32(0); i < length; i++ {
		result = append(result, FfiConverterSnapshotQuoteINSTANCE.Read(reader))
	}
	return result
}

func (c FfiConverterSequenceSnapshotQuote) Lower(value []SnapshotQuote) C.RustBuffer {
	return LowerIntoRustBuffer[[]SnapshotQuote](c, value)
}

func (c FfiConverterSequenceSnapshotQuote) Write(writer io.Writer, value []SnapshotQuote) {
	if len(value) > math.MaxInt32 {
		panic("[]SnapshotQuote is too large to fit into Int32")
	}

	writeInt32(writer, int32(len(value)))
	for _, item := range value {
		FfiConverterSnapshotQuoteINSTANCE.Write(writer, item)
	}
}

type FfiDestroyerSequenceSnapshotQuote struct{}

func (FfiDestroyerSequenceSnapshotQuote) Destroy(sequence []SnapshotQuote) {
	for _, value := range sequence {
		FfiDestroyerSnapshotQuote{}.Destroy(value)
	}
}

type FfiConverterSequenceTicker struct{}

var FfiConverterSequenceTickerINSTANCE = FfiConverterSequenceTicker{}

func (c FfiConverterSequenceTicker) Lift(rb RustBufferI) []Ticker {
	return LiftFromRustBuffer[[]Ticker](c, rb)
}

func (c FfiConverterSequenceTicker) Read(reader io.Reader) []Ticker {
	length := readInt32(reader)
	if length == 0 {
		return nil
	}
	result := make([]Ticker, 0, length)
	for i := int32(0); i < length; i++ {
		result = append(result, FfiConverterTickerINSTANCE.Read(reader))
	}
	return result
}

func (c FfiConverterSequenceTicker) Lower(value []Ticker) C.RustBuffer {
	return LowerIntoRustBuffer[[]Ticker](c, value)
}

func (c FfiConverterSequenceTicker) Write(writer io.Writer, value []Ticker) {
	if len(value) > math.MaxInt32 {
		panic("[]Ticker is too large to fit into Int32")
	}

	writeInt32(writer, int32(len(value)))
	for _, item := range value {
		FfiConverterTickerINSTANCE.Write(writer, item)
	}
}

type FfiDestroyerSequenceTicker struct{}

func (FfiDestroyerSequenceTicker) Destroy(sequence []Ticker) {
	for _, value := range sequence {
		FfiDestroyerTicker{}.Destroy(value)
	}
}

type FfiConverterSequenceTrade struct{}

var FfiConverterSequenceTradeINSTANCE = FfiConverterSequenceTrade{}

func (c FfiConverterSequenceTrade) Lift(rb RustBufferI) []Trade {
	return LiftFromRustBuffer[[]Trade](c, rb)
}

func (c FfiConverterSequenceTrade) Read(reader io.Reader) []Trade {
	length := readInt32(reader)
	if length == 0 {
		return nil
	}
	result := make([]Trade, 0, length)
	for i := int32(0); i < length; i++ {
		result = append(result, FfiConverterTradeINSTANCE.Read(reader))
	}
	return result
}

func (c FfiConverterSequenceTrade) Lower(value []Trade) C.RustBuffer {
	return LowerIntoRustBuffer[[]Trade](c, value)
}

func (c FfiConverterSequenceTrade) Write(writer io.Writer, value []Trade) {
	if len(value) > math.MaxInt32 {
		panic("[]Trade is too large to fit into Int32")
	}

	writeInt32(writer, int32(len(value)))
	for _, item := range value {
		FfiConverterTradeINSTANCE.Write(writer, item)
	}
}

type FfiDestroyerSequenceTrade struct{}

func (FfiDestroyerSequenceTrade) Destroy(sequence []Trade) {
	for _, value := range sequence {
		FfiDestroyerTrade{}.Destroy(value)
	}
}

type FfiConverterSequenceVolumeAtPrice struct{}

var FfiConverterSequenceVolumeAtPriceINSTANCE = FfiConverterSequenceVolumeAtPrice{}

func (c FfiConverterSequenceVolumeAtPrice) Lift(rb RustBufferI) []VolumeAtPrice {
	return LiftFromRustBuffer[[]VolumeAtPrice](c, rb)
}

func (c FfiConverterSequenceVolumeAtPrice) Read(reader io.Reader) []VolumeAtPrice {
	length := readInt32(reader)
	if length == 0 {
		return nil
	}
	result := make([]VolumeAtPrice, 0, length)
	for i := int32(0); i < length; i++ {
		result = append(result, FfiConverterVolumeAtPriceINSTANCE.Read(reader))
	}
	return result
}

func (c FfiConverterSequenceVolumeAtPrice) Lower(value []VolumeAtPrice) C.RustBuffer {
	return LowerIntoRustBuffer[[]VolumeAtPrice](c, value)
}

func (c FfiConverterSequenceVolumeAtPrice) Write(writer io.Writer, value []VolumeAtPrice) {
	if len(value) > math.MaxInt32 {
		panic("[]VolumeAtPrice is too large to fit into Int32")
	}

	writeInt32(writer, int32(len(value)))
	for _, item := range value {
		FfiConverterVolumeAtPriceINSTANCE.Write(writer, item)
	}
}

type FfiDestroyerSequenceVolumeAtPrice struct{}

func (FfiDestroyerSequenceVolumeAtPrice) Destroy(sequence []VolumeAtPrice) {
	for _, value := range sequence {
		FfiDestroyerVolumeAtPrice{}.Destroy(value)
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
func NewRestClientWithApiKey(apiKey string) (*RestClient, *MarketDataError) {
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) unsafe.Pointer {
		return C.uniffi_marketdata_uniffi_fn_func_new_rest_client_with_api_key(FfiConverterStringINSTANCE.Lower(apiKey), _uniffiStatus)
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue *RestClient
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterRestClientINSTANCE.Lift(_uniffiRV), _uniffiErr
	}
}

// Create a REST client with API key authentication, custom base URL, and TLS config
func NewRestClientWithApiKeyAndTls(apiKey string, baseUrl *string, tls TlsConfigRecord) (*RestClient, *MarketDataError) {
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) unsafe.Pointer {
		return C.uniffi_marketdata_uniffi_fn_func_new_rest_client_with_api_key_and_tls(FfiConverterStringINSTANCE.Lower(apiKey), FfiConverterOptionalStringINSTANCE.Lower(baseUrl), FfiConverterTlsConfigRecordINSTANCE.Lower(tls), _uniffiStatus)
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue *RestClient
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterRestClientINSTANCE.Lift(_uniffiRV), _uniffiErr
	}
}

// Create a REST client with bearer token authentication
//
// # Arguments
// * `bearer_token` - OAuth bearer token
//
// # Returns
// A RestClient instance wrapped in Arc for thread-safe access
func NewRestClientWithBearerToken(bearerToken string) (*RestClient, *MarketDataError) {
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) unsafe.Pointer {
		return C.uniffi_marketdata_uniffi_fn_func_new_rest_client_with_bearer_token(FfiConverterStringINSTANCE.Lower(bearerToken), _uniffiStatus)
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue *RestClient
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterRestClientINSTANCE.Lift(_uniffiRV), _uniffiErr
	}
}

// Create a REST client with bearer token authentication, custom base URL, and TLS config
func NewRestClientWithBearerTokenAndTls(bearerToken string, baseUrl *string, tls TlsConfigRecord) (*RestClient, *MarketDataError) {
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) unsafe.Pointer {
		return C.uniffi_marketdata_uniffi_fn_func_new_rest_client_with_bearer_token_and_tls(FfiConverterStringINSTANCE.Lower(bearerToken), FfiConverterOptionalStringINSTANCE.Lower(baseUrl), FfiConverterTlsConfigRecordINSTANCE.Lower(tls), _uniffiStatus)
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue *RestClient
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterRestClientINSTANCE.Lift(_uniffiRV), _uniffiErr
	}
}

// Create a REST client with SDK token authentication
//
// # Arguments
// * `sdk_token` - Fugle SDK token
//
// # Returns
// A RestClient instance wrapped in Arc for thread-safe access
func NewRestClientWithSdkToken(sdkToken string) (*RestClient, *MarketDataError) {
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) unsafe.Pointer {
		return C.uniffi_marketdata_uniffi_fn_func_new_rest_client_with_sdk_token(FfiConverterStringINSTANCE.Lower(sdkToken), _uniffiStatus)
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue *RestClient
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterRestClientINSTANCE.Lift(_uniffiRV), _uniffiErr
	}
}

// Create a REST client with SDK token authentication, custom base URL, and TLS config
func NewRestClientWithSdkTokenAndTls(sdkToken string, baseUrl *string, tls TlsConfigRecord) (*RestClient, *MarketDataError) {
	_uniffiRV, _uniffiErr := rustCallWithError[MarketDataError](FfiConverterMarketDataError{}, func(_uniffiStatus *C.RustCallStatus) unsafe.Pointer {
		return C.uniffi_marketdata_uniffi_fn_func_new_rest_client_with_sdk_token_and_tls(FfiConverterStringINSTANCE.Lower(sdkToken), FfiConverterOptionalStringINSTANCE.Lower(baseUrl), FfiConverterTlsConfigRecordINSTANCE.Lower(tls), _uniffiStatus)
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue *RestClient
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterRestClientINSTANCE.Lift(_uniffiRV), _uniffiErr
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
