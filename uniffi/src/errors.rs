//! Error types for UniFFI bindings
//!
//! This module defines a flat error enum that maps to the UDL MarketDataError.
//! All variants include a message string for detailed error information.

use marketdata_core::MarketDataError as CoreError;

/// Error type for UniFFI bindings
///
/// Maps to MarketDataError in the UDL file. Each variant becomes an exception
/// in the target language with the error message preserved.
///
/// Note: This is a FLAT enum per UniFFI constraints - no nested error types.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum MarketDataError {
    #[error("Network error: {msg}")]
    NetworkError { msg: String },

    #[error("Authentication error: {msg}")]
    AuthError { msg: String },

    #[error("Rate limit exceeded: {msg}")]
    RateLimitError { msg: String },

    #[error("Invalid symbol: {msg}")]
    InvalidSymbol { msg: String },

    #[error("Parse error: {msg}")]
    ParseError { msg: String },

    #[error("Timeout: {msg}")]
    TimeoutError { msg: String },

    #[error("WebSocket error: {msg}")]
    WebSocketError { msg: String },

    #[error("Client already closed")]
    ClientClosed,

    #[error("Configuration error: {msg}")]
    ConfigError { msg: String },

    #[error("API error: {msg}")]
    ApiError { msg: String },

    #[error("Other error: {msg}")]
    Other { msg: String },
}

impl From<CoreError> for MarketDataError {
    fn from(err: CoreError) -> Self {
        match err {
            CoreError::InvalidSymbol { symbol } => MarketDataError::InvalidSymbol { msg: symbol },
            CoreError::DeserializationError { source } => {
                MarketDataError::ParseError {
                    msg: source.to_string(),
                }
            }
            CoreError::RuntimeError { msg } => MarketDataError::Other { msg },
            CoreError::ConfigError(msg) => MarketDataError::ConfigError { msg },
            CoreError::ConnectionError { msg } => MarketDataError::NetworkError { msg },
            CoreError::AuthError { msg } => MarketDataError::AuthError { msg },
            CoreError::ApiError { status, message } => {
                // Check if this is a rate limit error (429)
                if status == 429 {
                    MarketDataError::RateLimitError {
                        msg: format!("HTTP {}: {}", status, message),
                    }
                } else {
                    MarketDataError::ApiError {
                        msg: format!("HTTP {}: {}", status, message),
                    }
                }
            }
            CoreError::TimeoutError { operation } => {
                MarketDataError::TimeoutError { msg: operation }
            }
            CoreError::HeartbeatTimeout { elapsed } => MarketDataError::TimeoutError {
                msg: format!("Heartbeat timeout: no inbound frames for {:?}", elapsed),
            },
            CoreError::WebSocketError { msg } => MarketDataError::WebSocketError { msg },
            CoreError::ClientClosed => MarketDataError::ClientClosed,
            CoreError::InvalidParameter { name, reason } => MarketDataError::ApiError {
                msg: format!("Invalid parameter '{}': {}", name, reason),
            },
            CoreError::Other(err) => MarketDataError::Other {
                msg: err.to_string(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = MarketDataError::NetworkError {
            msg: "connection refused".to_string(),
        };
        assert_eq!(err.to_string(), "Network error: connection refused");
    }

    #[test]
    fn test_client_closed_display() {
        let err = MarketDataError::ClientClosed;
        assert_eq!(err.to_string(), "Client already closed");
    }

    #[test]
    fn test_rate_limit_from_api_error() {
        let core_err = CoreError::ApiError {
            status: 429,
            message: "Too many requests".to_string(),
        };
        let uniffi_err: MarketDataError = core_err.into();
        assert!(matches!(uniffi_err, MarketDataError::RateLimitError { .. }));
    }

    #[test]
    fn test_api_error_non_rate_limit() {
        let core_err = CoreError::ApiError {
            status: 500,
            message: "Internal server error".to_string(),
        };
        let uniffi_err: MarketDataError = core_err.into();
        assert!(matches!(uniffi_err, MarketDataError::ApiError { .. }));
    }
}
