//! Error mapping from MarketDataError to napi::Error
//!
//! This module provides conversion from marketdata-core's MarketDataError
//! to napi::Error, preserving error codes for JavaScript consumers.

use marketdata_core::MarketDataError;

/// Convert MarketDataError to napi::Error
///
/// The error code is embedded in the error message for JavaScript access.
/// JavaScript consumers can parse the code from `error.message`.
///
/// Message format: "[code] original_message"
/// Example: "[1001] Invalid symbol: TEST"
pub fn to_napi_error(err: MarketDataError) -> napi::Error {
    let code = err.to_error_code();
    let message = err.to_string();

    // Create error with code and message
    // The error code will be accessible in JavaScript via error.message parsing
    napi::Error::from_reason(format!("[{}] {}", code, message))
}

/// Result type alias for NAPI operations
pub type NapiResult<T> = napi::Result<T>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_symbol_error_conversion() {
        let err = MarketDataError::InvalidSymbol {
            symbol: "TEST".to_string(),
        };
        let napi_err = to_napi_error(err);

        let reason = napi_err.reason.clone();
        assert!(reason.contains("[1001]"));
        assert!(reason.contains("Invalid symbol: TEST"));
    }

    #[test]
    fn test_config_error_conversion() {
        let err = MarketDataError::ConfigError("missing required field".to_string());
        let napi_err = to_napi_error(err);

        let reason = napi_err.reason.clone();
        assert!(reason.contains("[1004]"));
        assert!(reason.contains("missing required field"));
    }

    #[test]
    fn test_auth_error_conversion() {
        let err = MarketDataError::AuthError {
            msg: "invalid token".to_string(),
        };
        let napi_err = to_napi_error(err);

        let reason = napi_err.reason.clone();
        assert!(reason.contains("[2002]"));
        assert!(reason.contains("invalid token"));
    }

    #[test]
    fn test_api_error_conversion() {
        let err = MarketDataError::ApiError {
            status: 404,
            message: "not found".to_string(),
        };
        let napi_err = to_napi_error(err);

        let reason = napi_err.reason.clone();
        assert!(reason.contains("[2003]"));
        assert!(reason.contains("status 404"));
        assert!(reason.contains("not found"));
    }

    #[test]
    fn test_connection_error_conversion() {
        let err = MarketDataError::ConnectionError {
            msg: "connection refused".to_string(),
        };
        let napi_err = to_napi_error(err);

        let reason = napi_err.reason.clone();
        assert!(reason.contains("[2001]"));
        assert!(reason.contains("connection refused"));
    }

    #[test]
    fn test_timeout_error_conversion() {
        let err = MarketDataError::TimeoutError {
            operation: "quote request".to_string(),
        };
        let napi_err = to_napi_error(err);

        let reason = napi_err.reason.clone();
        assert!(reason.contains("[3001]"));
        assert!(reason.contains("quote request"));
    }
}
