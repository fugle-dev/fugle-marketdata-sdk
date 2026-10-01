//! Error mapping from marketdata-core to Python exceptions
//!
//! Maps MarketDataError variants to Python exceptions with error_code attribute.

use pyo3::prelude::*;
use pyo3::exceptions::{PyException, PyUserWarning};
use pyo3::create_exception;

// Create a custom Python exception hierarchy for market data errors

// Base exception for all market data errors
create_exception!(fugle_marketdata, MarketDataError, PyException);

// API-related errors
create_exception!(fugle_marketdata, ApiError, MarketDataError, "API request failed");
create_exception!(fugle_marketdata, RateLimitError, ApiError, "Rate limit exceeded");

// Authentication errors
create_exception!(fugle_marketdata, AuthError, MarketDataError, "Authentication failed");

// Configuration errors (core `ConfigError`, code 1004): credentials,
// `ReconnectConfig`, `HealthCheckConfig`. Not a `ValueError` (#171).
create_exception!(fugle_marketdata, ConfigError, MarketDataError, "Invalid configuration");

// Connection errors (core `ConnectionError`, code 2001): a REST request
// cannot reach the server, a WebSocket command is sent while not connected,
// or the WebSocket auth handshake fails for a reason other than rejected
// credentials; also core `ConnectionLimit` (code 2012, #300). Not a
// `WebSocketError` (#219).
create_exception!(fugle_marketdata, ConnectionError, MarketDataError, "Connection failed");
create_exception!(fugle_marketdata, TimeoutError, MarketDataError, "Operation timed out");

// WebSocket errors
create_exception!(fugle_marketdata, WebSocketError, MarketDataError, "WebSocket operation failed");

// Issued when `HealthCheckConfig` is given the 2.x fields `ping_interval` /
// `max_missed_pongs`, which 3.0 ignores (#304). A `UserWarning`, so it is
// shown by default; `code` matches Node's process warning.
create_exception!(
    fugle_marketdata,
    FugleHealthCheckWarning,
    PyUserWarning,
    "HealthCheckConfig was given 2.x fields that 3.0 ignores"
);

/// The `code` attribute of [`FugleHealthCheckWarning`], as on Node's
/// `FugleHealthCheckWarning`.
pub const HEALTH_CHECK_LEGACY_OPTIONS_CODE: &str = "FUGLE_HEALTH_CHECK_LEGACY_OPTIONS";

/// Warn that the 2.x `HealthCheckConfig` fields in `fields` were ignored.
/// Every call warns; Python's warning filters decide what is shown (by
/// default once per calling line). A filter that turns it into an error
/// makes this return that error.
pub fn warn_legacy_health_check_fields(py: Python<'_>, fields: &[&str]) -> PyResult<()> {
    if fields.is_empty() {
        return Ok(());
    }
    let (verb, was) = if fields.len() == 1 { ("does", "was") } else { ("do", "were") };
    let message = format!(
        "HealthCheckConfig {} {verb} not exist in fugle-marketdata 3.0 and {was} ignored. Use \
         heartbeat_timeout_ms (how long without any inbound frame before the connection is declared \
         dead, default 35000 ms), or probe_enabled with idle_probe_after_ms and probe_timeout_ms to \
         have the SDK ping a silent connection.",
        fields.join(" and "),
    );
    let message = std::ffi::CString::new(message).expect("no NUL in the message");
    PyErr::warn(py, &py.get_type::<FugleHealthCheckWarning>(), &message, 1)
}

/// Give `MarketDataError` (and so every subclass) a `__str__` that returns
/// `message`, the way the docs describe it: the instances are built with
/// `args == (message, code)`, so `BaseException.__str__` would print that
/// tuple (#299). An instance without a `str` `message` (one the caller
/// raised as `MarketDataError("...")`) keeps `BaseException.__str__`.
pub fn install_str(py: Python<'_>) -> PyResult<()> {
    // `py.run` into a scratch namespace rather than `PyModule::from_code`,
    // which would leave a module in `sys.modules`.
    let namespace = pyo3::types::PyDict::new(py);
    py.run(
        c"def __str__(self):\n    message = getattr(self, 'message', None)\n    if isinstance(message, str):\n        return message\n    return BaseException.__str__(self)\n",
        Some(&namespace),
        None,
    )?;
    let str_fn = namespace.get_item("__str__")?.expect("defined just above");
    str_fn.setattr("__qualname__", "MarketDataError.__str__")?;
    str_fn.setattr("__module__", "fugle_marketdata")?;
    py.get_type::<MarketDataError>().setattr("__str__", str_fn)
}

/// Convert marketdata_core error to PyErr with specific exception types
///
/// Maps MarketDataError variants to specific Python exception types:
/// - AuthError → AuthError
/// - ConfigError → ConfigError
/// - ApiError → ApiError (or RateLimitError for 429 status)
/// - TimeoutError / HeartbeatTimeout → TimeoutError
/// - ConnectionError / ConnectionLimit → ConnectionError
/// - WebSocketError / ClientClosed / ConnectionAborted / AlreadyConnected / ReconnectFailed → WebSocketError
/// - Other errors → MarketDataError (base exception)
///
/// The exception instance carries the unified error fields (core's
/// `ErrorInfo`, see `docs/errors.md`):
///
/// - `code` — numeric error code (int); also `args[1]`
/// - `source_kind` — `"network"`, `"protocol"`, `"auth"`, `"rate_limit"` or `"client"`
/// - `message` — human-readable message (str); also `args[0]` and `str(e)` (#299)
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
        CoreError::ConfigError(_) => ConfigError::new_err((message.clone(), error_code)),
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
        // The connection limit is a `ConnectionError` too, so existing
        // `except ConnectionError` blocks still catch it; `code` 2012 tells
        // it apart (#300).
        CoreError::ConnectionError { .. } | CoreError::ConnectionLimit { .. } => {
            ConnectionError::new_err((message.clone(), error_code))
        }
        CoreError::WebSocketError { .. }
        | CoreError::ClientClosed
        | CoreError::ConnectionAborted
        | CoreError::AlreadyConnected
        | CoreError::ReconnectFailed { .. } => WebSocketError::new_err((message.clone(), error_code)),
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
