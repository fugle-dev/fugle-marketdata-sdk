// errors.go - Unified error info helpers (#81)
//
// The generated MarketDataError variants and the channel-based
// StreamingClient's Errors() both carry the cross-language ErrorInfo
// (code, source kind, message, HTTP details). ErrorInfoOf gives callers one
// way to reach it regardless of which of the two they got.
package marketdata_uniffi

import "errors"

// StreamError is delivered on StreamingClient.Errors() for a WebSocket
// error event (the generated WebSocketListener.OnError callback), carrying
// the unified ErrorInfo instead of a bare formatted string.
type StreamError struct {
	Info ErrorInfo
}

// Error implements the error interface, returning the human-readable message.
func (e *StreamError) Error() string {
	return e.Info.Message
}

// ErrorInfoOf extracts the unified ErrorInfo from an error this package
// returned, if it carries one.
//
// Recognises:
//   - the generated *MarketDataError (returned by RestClient / WebSocketClient
//     methods), unwrapped to whichever variant it holds
//   - *StreamError (delivered on StreamingClient.Errors() by OnError)
//
// Returns false for any other error, including nil.
func ErrorInfoOf(err error) (ErrorInfo, bool) {
	var mde *MarketDataError
	if errors.As(err, &mde) {
		switch variant := mde.err.(type) {
		case *MarketDataErrorConnectionError:
			return variant.Info, true
		case *MarketDataErrorAuthError:
			return variant.Info, true
		case *MarketDataErrorRateLimitError:
			return variant.Info, true
		case *MarketDataErrorInvalidSymbol:
			return variant.Info, true
		case *MarketDataErrorParseError:
			return variant.Info, true
		case *MarketDataErrorTimeoutError:
			return variant.Info, true
		case *MarketDataErrorWebSocketError:
			return variant.Info, true
		case *MarketDataErrorClientClosed:
			return variant.Info, true
		case *MarketDataErrorConfigError:
			return variant.Info, true
		case *MarketDataErrorApiError:
			return variant.Info, true
		case *MarketDataErrorOther:
			return variant.Info, true
		}
		return ErrorInfo{}, false
	}

	var se *StreamError
	if errors.As(err, &se) {
		return se.Info, true
	}

	return ErrorInfo{}, false
}
