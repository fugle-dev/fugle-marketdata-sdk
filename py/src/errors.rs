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
/// The PyErr's exception instance exposes the following attributes, matching
/// the 2.4.1 SDK's `FugleAPIError` contract:
///
/// - `message` — human-readable error message (str)
/// - `status_code` — HTTP status for ApiError variants, else None
/// - `url` — request URL (None; requires plumbing from HTTP client)
/// - `params` — request params (None; requires plumbing from HTTP client)
/// - `response_text` — raw response body (None; requires plumbing from HTTP client)
///
/// The legacy `args[0]` (message) and `args[1]` (internal error_code) are
/// preserved for existing code paths.
///
/// ```python
/// try:
///     quote = client.stock.intraday.quote("INVALID")
/// except FugleAPIError as e:
///     print(e.message)      # same as str(e)
///     print(e.status_code)  # HTTP status for API errors, else None
/// ```
pub fn to_py_err(err: marketdata_core::MarketDataError) -> PyErr {
    use marketdata_core::MarketDataError as CoreError;

    let error_code = err.to_error_code();
    let message = err.to_string();

    // Extract status_code before consuming `err` in the match below.
    let status_code: Option<u16> = match &err {
        CoreError::ApiError { status, .. } => Some(*status),
        _ => None,
    };

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
        | CoreError::ClientClosed => WebSocketError::new_err((message.clone(), error_code)),
        _ => MarketDataError::new_err((message.clone(), error_code)),
    };

    // Attach 2.4.1-compatible attributes on the exception instance so
    // `except FugleAPIError as e: e.status_code` works drop-in.
    Python::attach(|py| {
        let inst = pyerr.value(py);
        let _ = inst.setattr("message", &message);
        let _ = inst.setattr("status_code", status_code);
        let _ = inst.setattr("url", py.None());
        let _ = inst.setattr("params", py.None());
        let _ = inst.setattr("response_text", py.None());
    });

    pyerr
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
        };
        assert_eq!(get_error_code(&err), 2002);

        let err = CoreError::ApiError {
            status: 404,
            message: "not found".to_string(),
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
