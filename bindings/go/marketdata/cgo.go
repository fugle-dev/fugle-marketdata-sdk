// cgo.go - CGO configuration for linking the UniFFI library
package marketdata_uniffi

/*
#cgo LDFLAGS: -L${SRCDIR}/../../../target/release -lmarketdata_uniffi
#cgo darwin LDFLAGS: -Wl,-rpath,${SRCDIR}/../../../target/release
#cgo linux LDFLAGS: -Wl,-rpath,${SRCDIR}/../../../target/release -Wl,--no-as-needed
*/
import "C"
