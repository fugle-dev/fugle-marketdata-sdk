//! Error types for UniFFI bindings
//!
//! This module defines a flat error enum that maps to the UDL MarketDataError.
//! All variants include a message string for detailed error information, plus
//! a structured [`ErrorInfo`] carrying the cross-language error fields shared
//! by every binding (code, source kind, HTTP details).

use std::collections::HashMap;

use marketdata_core::MarketDataError as CoreError;

/// Coarse-grained classification of the source of a [`MarketDataError`].
///
/// Mirrors `marketdata_core::ErrorKind`. That core enum is `#[non_exhaustive]`
/// so a future variant this crate doesn't know about yet maps to `Client`
/// (see the `From` impl below) rather than failing to compile.
#[derive(Debug, Clone, uniffi::Enum)]
pub enum ErrorSourceKind {
    /// Transport-level transient failure: connection reset, timeout,
    /// heartbeat gap, server outage (5xx). Generally safe to retry with
    /// backoff.
    Network,
    /// Protocol-level violation or unclassified WebSocket failure. Indicates
    /// an SDK / version mismatch or a server-side bug; retry is unlikely to
    /// help.
    Protocol,
    /// Authentication / authorization failure: bad credentials, 401/403,
    /// expired token, TLS cert failure. Human intervention required.
    Auth,
    /// Server is rejecting requests because the caller is exceeding its
    /// rate budget (HTTP 429).
    RateLimit,
    /// Caller-side problem: invalid input, configuration error, client
    /// already closed, serialization failure, non-auth/non-throttle 4xx.
    Client,
}

impl From<marketdata_core::ErrorKind> for ErrorSourceKind {
    fn from(kind: marketdata_core::ErrorKind) -> Self {
        match kind {
            marketdata_core::ErrorKind::Network => Self::Network,
            marketdata_core::ErrorKind::Protocol => Self::Protocol,
            marketdata_core::ErrorKind::Auth => Self::Auth,
            marketdata_core::ErrorKind::RateLimit => Self::RateLimit,
            marketdata_core::ErrorKind::Client => Self::Client,
            // `ErrorKind` is `#[non_exhaustive]`: a future core variant this
            // crate doesn't know about yet falls back to `Client` instead of
            // failing to compile.
            _ => Self::Client,
        }
    }
}

/// The cross-language view of an error: the fields every binding exposes
/// under the same names. Mirrors `marketdata_core::ErrorInfo`.
#[derive(Debug, Clone, uniffi::Record)]
pub struct ErrorInfo {
    /// Numeric code from `marketdata_core::error_code`, stable across
    /// languages and releases.
    pub code: i32,
    /// Category of the failure.
    pub source_kind: ErrorSourceKind,
    /// Human-readable message.
    pub message: String,
    /// HTTP status, when the error came from an HTTP response (REST, or the
    /// WebSocket upgrade).
    pub status: Option<u16>,
    /// Raw HTTP response body (REST only).
    pub body: Option<String>,
    /// Server-assigned request id (`x-request-id`), when present.
    pub request_id: Option<String>,
    /// HTTP response headers (REST only; empty otherwise).
    pub headers: HashMap<String, String>,
}

impl From<&marketdata_core::ErrorInfo> for ErrorInfo {
    fn from(info: &marketdata_core::ErrorInfo) -> Self {
        Self {
            code: info.code,
            source_kind: info.source_kind.into(),
            message: info.message.clone(),
            status: info.status,
            body: info.body.clone(),
            request_id: info.request_id.clone(),
            headers: info
                .headers
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        }
    }
}

/// Error type for UniFFI bindings
///
/// Maps to MarketDataError in the UDL file. Each variant becomes an exception
/// in the target language with the error message preserved, plus an `info`
/// field carrying the unified [`ErrorInfo`].
///
/// Note: This is a FLAT enum per UniFFI constraints - no nested error types.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum MarketDataError {
    #[error("Connection error: {msg}")]
    ConnectionError { msg: String, info: ErrorInfo },

    #[error("Authentication error: {msg}")]
    AuthError { msg: String, info: ErrorInfo },

    #[error("Rate limit exceeded: {msg}")]
    RateLimitError { msg: String, info: ErrorInfo },

    #[error("Invalid symbol: {msg}")]
    InvalidSymbol { msg: String, info: ErrorInfo },

    #[error("Parse error: {msg}")]
    ParseError { msg: String, info: ErrorInfo },

    #[error("Timeout: {msg}")]
    TimeoutError { msg: String, info: ErrorInfo },

    #[error("WebSocket error: {msg}")]
    WebSocketError { msg: String, info: ErrorInfo },

    #[error("Client already closed")]
    ClientClosed { info: ErrorInfo },

    #[error("Configuration error: {msg}")]
    ConfigError { msg: String, info: ErrorInfo },

    #[error("API error: {msg}")]
    ApiError { msg: String, info: ErrorInfo },

    #[error("Other error: {msg}")]
    Other { msg: String, info: ErrorInfo },
}

/// `MarketDataError::ConfigError` for a caller-side configuration mistake
/// caught at the FFI boundary (e.g. an unrecognised channel name or streaming
/// version string). `info` is what core reports for the same mistake.
pub(crate) fn config_error(message: impl Into<String>) -> MarketDataError {
    let msg = message.into();
    let info = ErrorInfo::from(&CoreError::ConfigError(msg.clone()).info());
    MarketDataError::ConfigError { msg, info }
}

/// `MarketDataError::ConnectionError` for an operation attempted before
/// `connect()`: the same variant and `info` (code 2001) as core's own
/// `ConnectionError`, so a not-connected `subscribe()` and a refused REST
/// connection are caught by one handler (#223).
pub(crate) fn not_connected_error(message: impl Into<String>) -> MarketDataError {
    MarketDataError::from(CoreError::ConnectionError { msg: message.into() })
}

/// `MarketDataError::Other` for a failure outside core (a
/// `tokio::task::JoinError` from a panicked `spawn_blocking` task, or a JSON
/// encode failure at the FFI boundary); `info` is core's `Other` (9999).
pub(crate) fn other_error(message: impl Into<String>) -> MarketDataError {
    let msg = message.into();
    let info = ErrorInfo::from(&marketdata_core::ErrorInfo::new(
        marketdata_core::error_code::OTHER,
        marketdata_core::ErrorKind::Client,
        msg.clone(),
    ));
    MarketDataError::Other { msg, info }
}

impl From<CoreError> for MarketDataError {
    fn from(err: CoreError) -> Self {
        // Computed before the match below consumes `err`.
        let info = ErrorInfo::from(&err.info());
        match err {
            CoreError::InvalidSymbol { symbol } => MarketDataError::InvalidSymbol { msg: symbol, info },
            CoreError::DeserializationError { source } => {
                MarketDataError::ParseError {
                    msg: source.to_string(),
                    info,
                }
            }
            CoreError::RuntimeError { msg } => MarketDataError::Other { msg, info },
            CoreError::ConfigError(msg) => MarketDataError::ConfigError { msg, info },
            CoreError::ConnectionError { msg } => MarketDataError::ConnectionError { msg, info },
            CoreError::AuthError { msg, .. } => MarketDataError::AuthError { msg, info },
            CoreError::ApiError { status, message, .. } => {
                // Check if this is a rate limit error (429)
                if status == 429 {
                    MarketDataError::RateLimitError {
                        msg: format!("HTTP {}: {}", status, message),
                        info,
                    }
                } else {
                    MarketDataError::ApiError {
                        msg: format!("HTTP {}: {}", status, message),
                        info,
                    }
                }
            }
            CoreError::TimeoutError { operation } => {
                MarketDataError::TimeoutError { msg: operation, info }
            }
            CoreError::HeartbeatTimeout { elapsed } => MarketDataError::TimeoutError {
                msg: format!("Heartbeat timeout: no inbound frames for {:?}", elapsed),
                info,
            },
            // 0.6.0: core's WebSocketError gained a structured `kind` field;
            // uniffi's shadow stays string-only because non-exhaustive enums
            // don't translate cleanly across all FFI targets (Go / Java).
            // Stringify the kind into the message to preserve detail.
            CoreError::WebSocketError { kind, msg } => MarketDataError::WebSocketError {
                msg: format!("[{kind:?}] {msg}"),
                info,
            },
            // Same code (2010), its own message (#121).
            CoreError::ClientClosed | CoreError::ConnectionAborted => {
                MarketDataError::ClientClosed { info }
            }
            // A dedicated exception would change every generated binding;
            // `info.code` (2011) identifies it (#119).
            CoreError::AlreadyConnected => MarketDataError::WebSocketError {
                msg: info.message.clone(),
                info,
            },
            CoreError::InvalidParameter { name, reason } => MarketDataError::ApiError {
                msg: format!("Invalid parameter '{}': {}", name, reason),
                info,
            },
            CoreError::Other(err) => MarketDataError::Other {
                msg: err.to_string(),
                info,
            },
            other => MarketDataError::Other {
                msg: other.to_string(),
                info,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_info() -> ErrorInfo {
        ErrorInfo {
            code: 0,
            source_kind: ErrorSourceKind::Client,
            message: String::new(),
            status: None,
            body: None,
            request_id: None,
            headers: HashMap::new(),
        }
    }

    #[test]
    fn test_error_display() {
        let err = MarketDataError::ConnectionError {
            msg: "connection refused".to_string(),
            info: test_info(),
        };
        assert_eq!(err.to_string(), "Connection error: connection refused");
    }

    /// A not-connected WebSocket command and a refused REST connection are
    /// the same variant with the same code (#223).
    #[test]
    fn not_connected_and_core_connection_error_share_a_variant() {
        let not_connected = not_connected_error("Not connected");
        let refused: MarketDataError = CoreError::ConnectionError {
            msg: "connection refused".to_string(),
        }
        .into();
        for err in [&not_connected, &refused] {
            match err {
                MarketDataError::ConnectionError { info, .. } => {
                    assert_eq!(info.code, marketdata_core::error_code::CONNECTION, "{err:?}");
                    assert!(matches!(info.source_kind, ErrorSourceKind::Network), "{err:?}");
                }
                other => panic!("expected ConnectionError, got {other:?}"),
            }
        }
        assert_eq!(not_connected.to_string(), "Connection error: Not connected");
    }

    #[test]
    fn test_client_closed_display() {
        let err = MarketDataError::ClientClosed { info: test_info() };
        assert_eq!(err.to_string(), "Client already closed");
    }

    #[test]
    fn test_rate_limit_from_api_error() {
        let core_err = CoreError::ApiError {
            status: 429,
            message: "Too many requests".to_string(),
            http: None,
        };
        let uniffi_err: MarketDataError = core_err.into();
        assert!(matches!(uniffi_err, MarketDataError::RateLimitError { .. }));
    }

    #[test]
    fn test_api_error_non_rate_limit() {
        let core_err = CoreError::ApiError {
            status: 500,
            message: "Internal server error".to_string(),
            http: None,
        };
        let uniffi_err: MarketDataError = core_err.into();
        assert!(matches!(uniffi_err, MarketDataError::ApiError { .. }));
    }

    #[test]
    fn api_error_info_carries_http_context() {
        let http = marketdata_core::HttpErrorContext::new(
            404,
            Some("not found".to_string()),
            [("x-request-id", "r1")],
        );
        let core_err = CoreError::ApiError {
            status: 404,
            message: "not found".to_string(),
            http: Some(Box::new(http)),
        };
        let uniffi_err: MarketDataError = core_err.into();
        match uniffi_err {
            MarketDataError::ApiError { info, .. } => {
                assert_eq!(info.code, marketdata_core::error_code::API);
                assert!(matches!(info.source_kind, ErrorSourceKind::Client));
                assert_eq!(info.status, Some(404));
                assert_eq!(info.body.as_deref(), Some("not found"));
                assert_eq!(info.request_id.as_deref(), Some("r1"));
                assert_eq!(
                    info.headers.get("x-request-id").map(String::as_str),
                    Some("r1")
                );
            }
            other => panic!("expected ApiError, got {other:?}"),
        }
    }

    #[test]
    fn auth_error_info_carries_http_context() {
        let http = marketdata_core::HttpErrorContext::new(
            401,
            Some("denied".to_string()),
            [("x-request-id", "r2")],
        );
        let core_err = CoreError::AuthError {
            msg: "denied".to_string(),
            http: Some(Box::new(http)),
        };
        let uniffi_err: MarketDataError = core_err.into();
        match uniffi_err {
            MarketDataError::AuthError { info, .. } => {
                assert_eq!(info.code, marketdata_core::error_code::AUTH);
                assert!(matches!(info.source_kind, ErrorSourceKind::Auth));
                assert_eq!(info.status, Some(401));
                assert_eq!(info.body.as_deref(), Some("denied"));
                assert_eq!(info.request_id.as_deref(), Some("r2"));
            }
            other => panic!("expected AuthError, got {other:?}"),
        }
    }

    #[test]
    fn client_closed_has_code_2010() {
        let uniffi_err: MarketDataError = CoreError::ClientClosed.into();
        match uniffi_err {
            MarketDataError::ClientClosed { info } => {
                assert_eq!(info.code, 2010);
            }
            other => panic!("expected ClientClosed, got {other:?}"),
        }
    }
}
