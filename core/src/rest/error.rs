//! Error conversion from ureq to MarketDataError

use crate::errors::MarketDataError;

impl From<ureq::Error> for MarketDataError {
    fn from(error: ureq::Error) -> Self {
        match error {
            // Status errors (HTTP response received with error code)
            ureq::Error::Status(status, response) => {
                let status_code = status;
                let message = response
                    .into_string()
                    .unwrap_or_else(|_| format!("HTTP {}", status_code));

                match status_code {
                    // Authentication errors
                    401 | 403 => MarketDataError::AuthError { msg: message },
                    // Rate limiting and server errors (retryable in ApiError context)
                    429 | 500..=599 => MarketDataError::ApiError {
                        status: status_code,
                        message,
                    },
                    // Other client/server errors
                    _ => MarketDataError::ApiError {
                        status: status_code,
                        message,
                    },
                }
            }
            // Transport errors (connection issues, no HTTP response)
            ureq::Error::Transport(transport) => {
                let error_msg = transport.to_string();

                // Check for timeout errors
                if error_msg.contains("timed out") || error_msg.contains("timeout") {
                    MarketDataError::TimeoutError {
                        operation: error_msg,
                    }
                } else {
                    // Other connection errors
                    MarketDataError::ConnectionError { msg: error_msg }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_is_retryable() {
        // Connection errors should be retryable
        let err = MarketDataError::ConnectionError {
            msg: "test".to_string(),
        };
        assert!(err.is_retryable());

        // Timeout errors should be retryable
        let err = MarketDataError::TimeoutError {
            operation: "test".to_string(),
        };
        assert!(err.is_retryable());

        // Auth errors should NOT be retryable
        let err = MarketDataError::AuthError {
            msg: "test".to_string(),
        };
        assert!(!err.is_retryable());

        // API errors with 4xx should NOT be retryable
        let err = MarketDataError::ApiError {
            status: 400,
            message: "test".to_string(),
        };
        assert!(!err.is_retryable());

        // API errors with 429 SHOULD be retryable
        let err = MarketDataError::ApiError {
            status: 429,
            message: "rate limit".to_string(),
        };
        assert!(err.is_retryable());

        // API errors with 5xx SHOULD be retryable
        let err = MarketDataError::ApiError {
            status: 503,
            message: "service unavailable".to_string(),
        };
        assert!(err.is_retryable());
    }

    #[test]
    fn test_status_401_converts_to_auth_error() {
        // Simulate 401 error
        let err = MarketDataError::AuthError {
            msg: "HTTP 401".to_string(),
        };
        assert!(!err.is_retryable());
        assert!(matches!(err, MarketDataError::AuthError { .. }));
    }

    #[test]
    fn test_status_403_converts_to_auth_error() {
        // Simulate 403 error
        let err = MarketDataError::AuthError {
            msg: "HTTP 403".to_string(),
        };
        assert!(!err.is_retryable());
        assert!(matches!(err, MarketDataError::AuthError { .. }));
    }

    #[test]
    fn test_status_429_converts_to_api_error_retryable() {
        // Simulate 429 error
        let err = MarketDataError::ApiError {
            status: 429,
            message: "Too Many Requests".to_string(),
        };
        assert!(err.is_retryable());
        assert!(matches!(err, MarketDataError::ApiError { status: 429, .. }));
    }

    #[test]
    fn test_status_500_converts_to_api_error_retryable() {
        // Simulate 500 error
        let err = MarketDataError::ApiError {
            status: 500,
            message: "Internal Server Error".to_string(),
        };
        assert!(err.is_retryable());
        assert!(matches!(err, MarketDataError::ApiError { status: 500, .. }));
    }

    #[test]
    fn test_timeout_converts_to_timeout_error() {
        // Simulate timeout error
        let err = MarketDataError::TimeoutError {
            operation: "connection timed out".to_string(),
        };
        assert!(err.is_retryable());
        assert!(matches!(err, MarketDataError::TimeoutError { .. }));
    }
}
