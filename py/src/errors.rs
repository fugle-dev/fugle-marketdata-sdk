//! Error mapping from marketdata-core to Python exceptions
//!
//! Maps MarketDataError variants to Python exceptions with error_code attribute.

use pyo3::prelude::*;
use pyo3::exceptions::PyException;
use pyo3::create_exception;

// Create a custom Python exception hierarchy for market data errors

// Base exception for all market data errors
create_exception!(fugle_marketdata, MarketDataError, PyException);

// API-related errors
create_exception!(fugle_marketdata, ApiError, MarketDataError, "API request failed");
create_exception!(fugle_marketdata, RateLimitError, ApiError, "Rate limit exceeded");

// Authentication errors
create_exception!(fugle_marketdata, AuthError, MarketDataError, "Authentication failed");

// Connection errors
create_exception!(fugle_marketdata, ConnectionError, MarketDataError, "Connection failed");
create_exception!(fugle_marketdata, TimeoutError, MarketDataError, "Operation timed out");

// WebSocket errors
create_exception!(fugle_marketdata, WebSocketError, MarketDataError, "WebSocket operation failed");

/// Convert marketdata_core error to PyErr with specific exception types
///
/// Maps MarketDataError variants to specific Python exception types:
/// - AuthError → AuthError
/// - ApiError → ApiError (or RateLimitError for 429 status)
/// - TimeoutError → TimeoutError
/// - Connection/WebSocket errors → WebSocketError
/// - Other errors → MarketDataError (base exception)
///
/// The exception instance carries the unified error fields (core's
/// `ErrorInfo`, see `docs/errors.md`):
///
/// - `code` — numeric error code (int); also `args[1]`
/// - `source_kind` — `"network"`, `"protocol"`, `"auth"`, `"rate_limit"` or `"client"`
/// - `message` — human-readable message (str); also `args[0]` and `str(e)`
/// - `status` — HTTP status (int) when the error came from an HTTP response, else None
/// - `body` — raw HTTP response body (str, REST only), else None
/// - `request_id` — server-assigned request id (`x-request-id`), else None
/// - `headers` — HTTP response headers with lowercase names (dict, REST only; else empty)
///
/// Aliases kept from the 2.4.1 SDK's `FugleAPIError`: `status_code` (= `status`),
/// `response_text` (= `body`); `url` and `params` are always None.
///
/// ```python
/// try:
///     quote = client.stock.intraday.quote("2330")
/// except MarketDataError as e:
///     print(e.code, e.source_kind, e.status, e.body)
/// ```
pub fn to_py_err(err: marketdata_core::MarketDataError) -> PyErr {
    use marketdata_core::MarketDataError as CoreError;

    let info = err.info();
    let error_code = info.code;
    let message = info.message.clone();

    // Map to specific exception types based on error variant
    let pyerr = match err {
        CoreError::AuthError { .. } => AuthError::new_err((message.clone(), error_code)),
        CoreError::ApiError { status, .. } => {
            if status == 429 {
                RateLimitError::new_err((message.clone(), error_code))
            } else {
                ApiError::new_err((message.clone(), error_code))
            }
        }
        CoreError::TimeoutError { .. } | CoreError::HeartbeatTimeout { .. } => {
            TimeoutError::new_err((message.clone(), error_code))
        }
        CoreError::ConnectionError { .. }
        | CoreError::WebSocketError { .. }
        | CoreError::ClientClosed
        | CoreError::ConnectionAborted
        | CoreError::AlreadyConnected => WebSocketError::new_err((message.clone(), error_code)),
        _ => MarketDataError::new_err((message.clone(), error_code)),
    };

    Python::attach(|py| set_info_attrs(pyerr.value(py).as_any(), &info));

    pyerr
}

/// A `WebSocketError((message, code))` carrying `info`'s fields, as the
/// WebSocket `error` callbacks receive it.
pub fn websocket_error(py: Python<'_>, info: &marketdata_core::ErrorInfo) -> PyErr {
    let err = WebSocketError::new_err((info.message.clone(), info.code));
    set_info_attrs(err.value(py).as_any(), info);
    err
}

/// Set the unified error fields of `info` (and the 2.4.1 aliases) on `inst`.
fn set_info_attrs(inst: &Bound<'_, PyAny>, info: &marketdata_core::ErrorInfo) {
    let py = inst.py();
    let _ = inst.setattr("code", info.code);
    let _ = inst.setattr("source_kind", info.source_kind.as_str());
    let _ = inst.setattr("message", &info.message);
    let _ = inst.setattr("status", info.status);
    let _ = inst.setattr("body", info.body.as_deref());
    let _ = inst.setattr("request_id", info.request_id.as_deref());
    let _ = inst.setattr("headers", &info.headers);
    // 2.4.1 `FugleAPIError` aliases.
    let _ = inst.setattr("status_code", info.status);
    let _ = inst.setattr("response_text", info.body.as_deref());
    let _ = inst.setattr("url", py.None());
    let _ = inst.setattr("params", py.None());
}

/// Helper to get error_code from a MarketDataError
#[allow(dead_code)]
pub fn get_error_code(err: &marketdata_core::MarketDataError) -> i32 {
    err.to_error_code()
}

#[cfg(test)]
mod tests {
    use super::*;
    use marketdata_core::MarketDataError as CoreError;

    #[test]
    fn test_error_code_mapping() {
        let err = CoreError::InvalidSymbol {
            symbol: "TEST".to_string(),
        };
        assert_eq!(get_error_code(&err), 1001);

        let err = CoreError::AuthError {
            msg: "test".to_string(),
            http: None,
        };
        assert_eq!(get_error_code(&err), 2002);

        let err = CoreError::ApiError {
            status: 404,
            message: "not found".to_string(),
            http: None,
        };
        assert_eq!(get_error_code(&err), 2003);

        let err = CoreError::TimeoutError {
            operation: "test".to_string(),
        };
        assert_eq!(get_error_code(&err), 3001);
    }

    #[test]
    fn test_error_message() {
        let err = CoreError::InvalidSymbol {
            symbol: "BAD".to_string(),
        };
        assert_eq!(err.to_string(), "Invalid symbol: BAD");
    }

    #[test]
    fn test_client_closed_error_code() {
        let err = CoreError::ClientClosed;
        assert_eq!(get_error_code(&err), 2010);
        assert_eq!(err.to_string(), "Client already closed");
    }
}
