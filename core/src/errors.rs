//! Error types for marketdata-core
//!
//! Error code ranges:
//! - 1000-1999: Client errors (bad input, deserialization)
//! - 2000-2999: Server/API errors (auth, connection, HTTP)
//! - 3000-3999: Network errors (timeout, WebSocket)
//! - 9000-9999: Internal errors (unexpected failures)

use std::time::Duration;
use thiserror::Error;

/// Main error type for marketdata-core operations
#[derive(Error, Debug)]
pub enum MarketDataError {
    /// Invalid symbol format or unsupported symbol
    #[error("Invalid symbol: {symbol}")]
    InvalidSymbol {
        /// The offending symbol string that failed validation.
        symbol: String,
    },

    /// Invalid or missing parameter
    #[error("Invalid parameter '{name}': {reason}")]
    InvalidParameter {
        /// Parameter name that failed validation.
        name: String,
        /// Human-readable explanation of why the parameter was rejected.
        reason: String,
    },

    /// JSON deserialization failed
    #[error("Deserialization failed: {source}")]
    DeserializationError {
        /// Underlying `serde_json` error.
        #[from]
        source: serde_json::Error,
    },

    /// Runtime operation failed
    #[error("Runtime error: {msg}")]
    RuntimeError {
        /// Diagnostic message describing the runtime failure.
        msg: String,
    },

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigError(
        /// Diagnostic message identifying the misconfiguration.
        String,
    ),

    /// Connection to server failed
    #[error("Connection error: {msg}")]
    ConnectionError {
        /// Diagnostic message describing the connection failure.
        msg: String,
    },

    /// Authentication failed
    #[error("Authentication error: {msg}")]
    AuthError {
        /// Diagnostic message describing the authentication failure.
        msg: String,
    },

    /// API returned error response
    #[error("API error (status {status}): {message}")]
    ApiError {
        /// HTTP status code returned by the server.
        status: u16,
        /// Server-provided error message.
        message: String,
    },

    /// Operation timed out
    #[error("Timeout error: {operation}")]
    TimeoutError {
        /// Human-readable name of the operation that timed out.
        operation: String,
    },

    /// WebSocket error
    #[error("WebSocket error: {msg}")]
    WebSocketError {
        /// Diagnostic message describing the WebSocket failure.
        msg: String,
    },

    /// Inbound activity timed out: no frame received within the
    /// configured `heartbeat_timeout` window.
    #[error("Heartbeat timeout: no inbound frames for {elapsed:?}")]
    HeartbeatTimeout {
        /// Wall-clock interval that elapsed since the last inbound frame.
        elapsed: Duration,
    },

    /// Client has been closed and cannot be reused
    #[error("Client already closed")]
    ClientClosed,

    /// Other unexpected errors
    #[error(transparent)]
    Other(
        /// Underlying error wrapped via `anyhow`.
        #[from]
        anyhow::Error,
    ),
}

impl From<tungstenite::Error> for MarketDataError {
    fn from(err: tungstenite::Error) -> Self {
        use tungstenite::Error as WsError;

        match err {
            // Retryable connection errors
            WsError::ConnectionClosed | WsError::Io(_) => {
                Self::ConnectionError {
                    msg: format!("WebSocket connection error: {}", err),
                }
            }
            // Fatal WebSocket protocol errors
            WsError::AlreadyClosed | WsError::Protocol(_) | WsError::Capacity(_) => {
                Self::WebSocketError {
                    msg: format!("WebSocket protocol error: {}", err),
                }
            }
            // TLS/certificate errors are auth errors (often cert issues)
            WsError::Tls(_) => Self::AuthError {
                msg: format!("TLS/certificate error: {}", err),
            },
            // HTTP errors (e.g., 401, 403, 404)
            WsError::Http(response) => {
                let status = response.status().as_u16();
                match status {
                    401 | 403 => Self::AuthError {
                        msg: format!("HTTP {} during WebSocket handshake", status),
                    },
                    _ => Self::ConnectionError {
                        msg: format!("HTTP {} during WebSocket handshake", status),
                    },
                }
            }
            // Other errors (URL parsing, UTF-8, etc.) are WebSocket errors
            _ => Self::WebSocketError {
                msg: format!("WebSocket error: {}", err),
            },
        }
    }
}

impl MarketDataError {
    /// Get numeric error code for FFI consumers
    pub fn to_error_code(&self) -> i32 {
        match self {
            Self::InvalidSymbol { .. } => 1001,
            Self::InvalidParameter { .. } => 1005,
            Self::DeserializationError { .. } => 1002,
            Self::RuntimeError { .. } => 1003,
            Self::ConfigError(_) => 1004,
            Self::ConnectionError { .. } => 2001,
            Self::AuthError { .. } => 2002,
            Self::ApiError { .. } => 2003,
            Self::TimeoutError { .. } => 3001,
            Self::WebSocketError { .. } => 3002,
            Self::HeartbeatTimeout { .. } => 3003,
            Self::ClientClosed => 2010,
            Self::Other(_) => 9999,
        }
    }

    /// Check if error is retryable
    pub fn is_retryable(&self) -> bool {
        match self {
            // Network errors are always retryable
            Self::ConnectionError { .. }
            | Self::TimeoutError { .. }
            | Self::WebSocketError { .. }
            | Self::HeartbeatTimeout { .. } => true,
            // API errors with 429 or 5xx status codes are retryable
            Self::ApiError { status, .. } => *status == 429 || (500..=599).contains(status),
            // Parameter errors are never retryable (user must fix input)
            Self::InvalidParameter { .. } => false,
            // All other errors are not retryable
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = MarketDataError::InvalidSymbol {
            symbol: "INVALID".to_string(),
        };
        assert_eq!(err.to_string(), "Invalid symbol: INVALID");

        let err = MarketDataError::RuntimeError {
            msg: "test message".to_string(),
        };
        assert_eq!(err.to_string(), "Runtime error: test message");

        let err = MarketDataError::ConfigError("missing key".to_string());
        assert_eq!(err.to_string(), "Configuration error: missing key");

        let err = MarketDataError::ApiError {
            status: 404,
            message: "not found".to_string(),
        };
        assert_eq!(err.to_string(), "API error (status 404): not found");

        let err = MarketDataError::ClientClosed;
        assert_eq!(err.to_string(), "Client already closed");
    }

    #[test]
    fn test_error_codes() {
        let err = MarketDataError::InvalidSymbol {
            symbol: "test".to_string(),
        };
        assert_eq!(err.to_error_code(), 1001);

        let err = MarketDataError::RuntimeError {
            msg: "test".to_string(),
        };
        assert_eq!(err.to_error_code(), 1003);

        let err = MarketDataError::ConfigError("test".to_string());
        assert_eq!(err.to_error_code(), 1004);

        let err = MarketDataError::ConnectionError {
            msg: "test".to_string(),
        };
        assert_eq!(err.to_error_code(), 2001);

        let err = MarketDataError::AuthError {
            msg: "test".to_string(),
        };
        assert_eq!(err.to_error_code(), 2002);

        let err = MarketDataError::ApiError {
            status: 500,
            message: "test".to_string(),
        };
        assert_eq!(err.to_error_code(), 2003);

        let err = MarketDataError::TimeoutError {
            operation: "test".to_string(),
        };
        assert_eq!(err.to_error_code(), 3001);

        let err = MarketDataError::WebSocketError {
            msg: "test".to_string(),
        };
        assert_eq!(err.to_error_code(), 3002);

        let err = MarketDataError::HeartbeatTimeout {
            elapsed: Duration::from_secs(35),
        };
        assert_eq!(err.to_error_code(), 3003);

        let err = MarketDataError::ClientClosed;
        assert_eq!(err.to_error_code(), 2010);

        let err = MarketDataError::Other(anyhow::anyhow!("test"));
        assert_eq!(err.to_error_code(), 9999);
    }

    #[test]
    fn test_retryable_classification() {
        // Retryable errors
        let err = MarketDataError::ConnectionError {
            msg: "test".to_string(),
        };
        assert!(err.is_retryable());

        let err = MarketDataError::TimeoutError {
            operation: "test".to_string(),
        };
        assert!(err.is_retryable());

        let err = MarketDataError::WebSocketError {
            msg: "test".to_string(),
        };
        assert!(err.is_retryable());

        let err = MarketDataError::HeartbeatTimeout {
            elapsed: Duration::from_secs(35),
        };
        assert!(err.is_retryable());

        // Non-retryable errors
        let err = MarketDataError::InvalidSymbol {
            symbol: "test".to_string(),
        };
        assert!(!err.is_retryable());

        let err = MarketDataError::RuntimeError {
            msg: "test".to_string(),
        };
        assert!(!err.is_retryable());

        let err = MarketDataError::ConfigError("test".to_string());
        assert!(!err.is_retryable());

        let err = MarketDataError::AuthError {
            msg: "test".to_string(),
        };
        assert!(!err.is_retryable());

        let err = MarketDataError::ApiError {
            status: 400,
            message: "test".to_string(),
        };
        assert!(!err.is_retryable());

        // ApiError with 429 should be retryable
        let err = MarketDataError::ApiError {
            status: 429,
            message: "rate limit".to_string(),
        };
        assert!(err.is_retryable());

        // ApiError with 5xx should be retryable
        let err = MarketDataError::ApiError {
            status: 503,
            message: "service unavailable".to_string(),
        };
        assert!(err.is_retryable());

        let err = MarketDataError::ClientClosed;
        assert!(!err.is_retryable());

        let err = MarketDataError::Other(anyhow::anyhow!("test"));
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_heartbeat_timeout_display() {
        let err = MarketDataError::HeartbeatTimeout {
            elapsed: Duration::from_secs(35),
        };
        assert!(err.to_string().contains("35s"));
        assert!(err.to_string().starts_with("Heartbeat timeout"));
    }

    #[test]
    fn test_from_serde_json_error() {
        let json_err = serde_json::from_str::<serde_json::Value>("{invalid json")
            .unwrap_err();
        let err: MarketDataError = json_err.into();

        assert_eq!(err.to_error_code(), 1002);
        assert!(matches!(err, MarketDataError::DeserializationError { .. }));
    }

    #[test]
    fn test_from_anyhow_error() {
        let anyhow_err = anyhow::anyhow!("test error");
        let err: MarketDataError = anyhow_err.into();

        assert_eq!(err.to_error_code(), 9999);
        assert!(matches!(err, MarketDataError::Other(_)));
    }

    #[test]
    fn test_from_tungstenite_connection_closed() {
        use tokio_tungstenite::tungstenite::Error as WsError;

        let ws_err = WsError::ConnectionClosed;
        let err: MarketDataError = ws_err.into();

        assert_eq!(err.to_error_code(), 2001);
        assert!(matches!(err, MarketDataError::ConnectionError { .. }));
        assert!(err.is_retryable());
    }

    #[test]
    fn test_from_tungstenite_protocol_error() {
        use tokio_tungstenite::tungstenite::Error as WsError;
        use tokio_tungstenite::tungstenite::error::ProtocolError;

        let ws_err = WsError::Protocol(ProtocolError::ResetWithoutClosingHandshake);
        let err: MarketDataError = ws_err.into();

        assert_eq!(err.to_error_code(), 3002);
        assert!(matches!(err, MarketDataError::WebSocketError { .. }));
        assert!(err.is_retryable()); // WebSocket errors are retryable
    }

    #[test]
    fn test_from_tungstenite_already_closed() {
        use tokio_tungstenite::tungstenite::Error as WsError;

        let ws_err = WsError::AlreadyClosed;
        let err: MarketDataError = ws_err.into();

        assert_eq!(err.to_error_code(), 3002);
        assert!(matches!(err, MarketDataError::WebSocketError { .. }));
    }
}
