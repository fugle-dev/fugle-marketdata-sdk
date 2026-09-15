//! Error conversion from HTTP outcomes to MarketDataError

use crate::errors::MarketDataError;

/// Map an HTTP error status and its response body to a `MarketDataError`.
pub(crate) fn status_error(status: u16, message: String) -> MarketDataError {
    match status {
        // Authentication errors
        401 | 403 => MarketDataError::AuthError { msg: message },
        // Everything else keeps the status; 429 and 5xx are retryable via
        // `MarketDataError::is_retryable`.
        _ => MarketDataError::ApiError { status, message },
    }
}

/// Map a transport-level failure (no HTTP response) to a `MarketDataError`.
///
/// Timeouts become `TimeoutError`; everything else is a `ConnectionError`.
/// The request URL is prefixed so the message says where the call went.
pub(crate) fn transport_error(url: &str, error: ureq::Error) -> MarketDataError {
    let is_timeout = match &error {
        ureq::Error::Timeout(_) | ureq::Error::BodyStalled => true,
        ureq::Error::Io(io) => matches!(io.kind(), std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock),
        _ => false,
    };
    let message = format!("{url}: {error}");
    if is_timeout {
        MarketDataError::TimeoutError { operation: message }
    } else {
        MarketDataError::ConnectionError { msg: message }
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

    #[test]
    fn status_error_maps_auth_and_api_errors() {
        assert!(matches!(status_error(401, "x".into()), MarketDataError::AuthError { .. }));
        assert!(matches!(status_error(403, "x".into()), MarketDataError::AuthError { .. }));
        assert!(matches!(status_error(404, "x".into()), MarketDataError::ApiError { status: 404, .. }));
        assert!(status_error(429, "x".into()).is_retryable());
        assert!(status_error(502, "x".into()).is_retryable());
        assert!(!status_error(400, "x".into()).is_retryable());
    }

    #[test]
    fn transport_error_maps_timeouts() {
        let t = transport_error("http://h/p", ureq::Error::Timeout(ureq::Timeout::RecvResponse));
        assert!(matches!(t, MarketDataError::TimeoutError { .. }));
        let io = transport_error("http://h/p", ureq::Error::Io(std::io::Error::from(std::io::ErrorKind::TimedOut)));
        assert!(matches!(io, MarketDataError::TimeoutError { .. }));
        let refused = transport_error("http://h/p", ureq::Error::ConnectionFailed);
        match refused {
            MarketDataError::ConnectionError { msg } => assert!(msg.starts_with("http://h/p: ")),
            other => panic!("expected ConnectionError, got {other:?}"),
        }
    }
}
