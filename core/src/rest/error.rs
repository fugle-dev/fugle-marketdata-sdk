//! Error conversion from HTTP outcomes to MarketDataError

use crate::errors::{HttpErrorContext, MarketDataError};

/// Map an HTTP error response to a `MarketDataError`.
///
/// The message is the body, or `HTTP <status>` when the body could not be
/// read; the full response rides along as [`HttpErrorContext`].
pub(crate) fn status_error(http: HttpErrorContext) -> MarketDataError {
    let status = http.status;
    let message = http.body.clone().unwrap_or_else(|| format!("HTTP {status}"));
    let http = Some(Box::new(http));
    match status {
        // Authentication errors
        401 | 403 => MarketDataError::AuthError { msg: message, http },
        // Everything else keeps the status; 429 and 5xx are retryable via
        // `MarketDataError::is_retryable`.
        _ => MarketDataError::ApiError { status, message, http },
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

/// Whether the peer closed the connection before a full response header
/// arrived, so an idempotent request can safely be sent once more.
///
/// This is how a pooled keep-alive connection fails when the server (or a
/// load balancer with a shorter idle timeout) closed it just after ureq's
/// liveness probe: the request goes out and the reply is an EOF or reset.
/// ureq does not say whether the connection came from the pool, nor whether
/// part of the header had arrived, so the check is on the failure alone.
/// `ConnectionAborted` is how Windows reports the same close.
///
/// `InvalidInput` is on the list for macOS. ureq sets the socket's write
/// timeout before writing the request and its read timeout before reading
/// the response (`maybe_update_timeout` in ureq's
/// `unversioned/transport/tcp.rs`), and macOS answers that `setsockopt`
/// with `EINVAL` when the peer has already reset the socket. Depending on
/// which call fails, the request may or may not have been written. Either
/// way resending is safe: GET is idempotent and it is resent only once —
/// do not rely on the request being unsent. The kind is broad, but in a
/// loopback stress run it only appeared on reused connections, never when
/// the server closed each connection itself.
///
/// `ConnectionFailed` and `ConnectionRefused` mean no connection was made at
/// all; sending again would not help, so they are excluded.
pub(crate) fn dropped_before_response(error: &ureq::Error) -> bool {
    match error {
        ureq::Error::Io(io) => matches!(
            io.kind(),
            std::io::ErrorKind::UnexpectedEof
                | std::io::ErrorKind::ConnectionReset
                | std::io::ErrorKind::ConnectionAborted
                | std::io::ErrorKind::BrokenPipe
                | std::io::ErrorKind::InvalidInput
        ),
        _ => false,
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
            http: None,
        };
        assert!(!err.is_retryable());

        // API errors with 4xx should NOT be retryable
        let err = MarketDataError::ApiError {
            status: 400,
            message: "test".to_string(),
            http: None,
        };
        assert!(!err.is_retryable());

        // API errors with 429 SHOULD be retryable
        let err = MarketDataError::ApiError {
            status: 429,
            message: "rate limit".to_string(),
            http: None,
        };
        assert!(err.is_retryable());

        // API errors with 5xx SHOULD be retryable
        let err = MarketDataError::ApiError {
            status: 503,
            message: "service unavailable".to_string(),
            http: None,
        };
        assert!(err.is_retryable());
    }

    #[test]
    fn test_status_401_converts_to_auth_error() {
        // Simulate 401 error
        let err = MarketDataError::AuthError {
            msg: "HTTP 401".to_string(),
            http: None,
        };
        assert!(!err.is_retryable());
        assert!(matches!(err, MarketDataError::AuthError { .. }));
    }

    #[test]
    fn test_status_403_converts_to_auth_error() {
        // Simulate 403 error
        let err = MarketDataError::AuthError {
            msg: "HTTP 403".to_string(),
            http: None,
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
            http: None,
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
            http: None,
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
        let err = |status| status_error(HttpErrorContext::new(status, Some("x".into()), [("X-Request-Id", "r1")]));
        assert!(matches!(err(401), MarketDataError::AuthError { .. }));
        assert!(matches!(err(403), MarketDataError::AuthError { .. }));
        assert!(matches!(err(404), MarketDataError::ApiError { status: 404, .. }));
        assert!(err(429).is_retryable());
        assert!(err(502).is_retryable());
        assert!(!err(400).is_retryable());
    }

    #[test]
    fn status_error_keeps_the_response_in_info() {
        for status in [401, 404] {
            let info = status_error(HttpErrorContext::new(
                status,
                Some(r#"{"message":"nope"}"#.into()),
                [("X-Request-Id", "r1"), ("Retry-After", "5")],
            ))
            .info();
            assert_eq!(info.status, Some(status));
            assert_eq!(info.body.as_deref(), Some(r#"{"message":"nope"}"#));
            assert_eq!(info.request_id.as_deref(), Some("r1"));
            assert_eq!(info.headers.get("retry-after").map(String::as_str), Some("5"));
        }
    }

    #[test]
    fn unreadable_body_falls_back_to_status_message() {
        let err = status_error(HttpErrorContext::new(500, None, Vec::<(&str, &str)>::new()));
        assert_eq!(err.to_string(), "API error (status 500): HTTP 500");
        let info = err.info();
        assert_eq!(info.status, Some(500));
        assert_eq!(info.body, None);
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

    // The macOS `InvalidInput` (EINVAL) case is only classified here: it
    // depends on OS timing on a reused socket, so no test drives it through
    // the connection pool. It was verified by a manual loopback stress run
    // (#106); CI does not cover that path end to end.
    #[test]
    fn dropped_before_response_covers_peer_closes_only() {
        use std::io::ErrorKind;
        let io = |kind| ureq::Error::Io(std::io::Error::from(kind));
        for kind in [
            ErrorKind::UnexpectedEof,
            ErrorKind::ConnectionReset,
            ErrorKind::ConnectionAborted,
            ErrorKind::BrokenPipe,
            ErrorKind::InvalidInput,
        ] {
            assert!(dropped_before_response(&io(kind)), "{kind:?}");
        }
        for kind in [ErrorKind::ConnectionRefused, ErrorKind::TimedOut, ErrorKind::Other] {
            assert!(!dropped_before_response(&io(kind)), "{kind:?}");
        }
        assert!(!dropped_before_response(&ureq::Error::ConnectionFailed));
        assert!(!dropped_before_response(&ureq::Error::Timeout(ureq::Timeout::RecvResponse)));
    }
}
