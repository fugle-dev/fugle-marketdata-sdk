//! Authentication mechanisms for REST API

use crate::errors::MarketDataError;
use std::fmt;

/// Authentication method for REST API requests.
///
/// `Debug` is implemented manually to redact the secret value — printing
/// an `Auth` (directly or via `tracing::debug!(?config)`) emits
/// `Auth::ApiKey(***)` instead of the raw token, preventing accidental
/// secret leakage to logs.
#[derive(Clone)]
pub enum Auth {
    /// API Key authentication (X-API-KEY header)
    ApiKey(String),
    /// Bearer token authentication (Authorization: Bearer header)
    BearerToken(String),
    /// SDK token authentication (X-SDK-TOKEN header)
    SdkToken(String),
}

impl fmt::Debug for Auth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Auth::ApiKey(_) => f.write_str("Auth::ApiKey(***)"),
            Auth::BearerToken(_) => f.write_str("Auth::BearerToken(***)"),
            Auth::SdkToken(_) => f.write_str("Auth::SdkToken(***)"),
        }
    }
}

impl Auth {
    /// Apply authentication to a ureq request
    pub fn apply_to_request(&self, request: ureq::Request) -> ureq::Request {
        match self {
            Auth::ApiKey(key) => request.set("X-API-KEY", key),
            Auth::BearerToken(token) => request.set("Authorization", &format!("Bearer {}", token)),
            Auth::SdkToken(token) => request.set("X-SDK-TOKEN", token),
        }
    }

    /// Resolve authentication from the process environment.
    ///
    /// Probes the variables `FUGLE_API_KEY`, `FUGLE_BEARER_TOKEN`, and
    /// `FUGLE_SDK_TOKEN` in that order and returns the first non-empty
    /// match wrapped in the corresponding `Auth` variant. Empty strings are
    /// treated as unset.
    ///
    /// # Errors
    ///
    /// Returns [`MarketDataError::ConfigError`] when none of the three
    /// variables is set to a non-empty value.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use marketdata_core::Auth;
    ///
    /// // FUGLE_API_KEY=my-key cargo run
    /// let auth = Auth::from_env().expect("set FUGLE_API_KEY");
    /// ```
    pub fn from_env() -> Result<Self, MarketDataError> {
        const VARS: &[(&str, fn(String) -> Auth)] = &[
            ("FUGLE_API_KEY", Auth::ApiKey),
            ("FUGLE_BEARER_TOKEN", Auth::BearerToken),
            ("FUGLE_SDK_TOKEN", Auth::SdkToken),
        ];

        for (name, ctor) in VARS {
            if let Ok(value) = std::env::var(name) {
                if !value.is_empty() {
                    return Ok(ctor(value));
                }
            }
        }

        Err(MarketDataError::ConfigError(format!(
            "No authentication credentials found in environment. \
             Set one of: {}, {}, or {}.",
            VARS[0].0, VARS[1].0, VARS[2].0
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_apply_to_request() {
        // Test ApiKey
        let auth = Auth::ApiKey("test_key".to_string());
        let request = ureq::get("http://example.com");
        let _request = auth.apply_to_request(request);
        // Note: ureq doesn't expose headers for inspection in tests,
        // so we just verify it compiles and doesn't panic

        // Test BearerToken
        let auth = Auth::BearerToken("test_token".to_string());
        let request = ureq::get("http://example.com");
        let _request = auth.apply_to_request(request);

        // Test SdkToken
        let auth = Auth::SdkToken("test_sdk_token".to_string());
        let request = ureq::get("http://example.com");
        let _request = auth.apply_to_request(request);
    }

    #[test]
    fn test_debug_redacts_api_key() {
        let auth = Auth::ApiKey("super-secret-key-12345".to_string());
        let rendered = format!("{:?}", auth);
        assert_eq!(rendered, "Auth::ApiKey(***)");
        assert!(!rendered.contains("super-secret-key-12345"));
    }

    #[test]
    fn test_debug_redacts_bearer_token() {
        let auth = Auth::BearerToken("eyJhbGciOiJIUzI1NiJ9.payload.sig".to_string());
        let rendered = format!("{:?}", auth);
        assert_eq!(rendered, "Auth::BearerToken(***)");
        assert!(!rendered.contains("eyJ"));
    }

    #[test]
    fn test_debug_redacts_sdk_token() {
        let auth = Auth::SdkToken("sdk-token-abc".to_string());
        let rendered = format!("{:?}", auth);
        assert_eq!(rendered, "Auth::SdkToken(***)");
        assert!(!rendered.contains("sdk-token-abc"));
    }

    /// Mutex serialises env-mutating tests so `from_env` always observes
    /// a clean variable set. Without this each test races against the others
    /// and `cargo test --jobs N` produces flakes.
    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        use std::sync::Mutex;
        static LOCK: Mutex<()> = Mutex::new(());
        LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn clear_auth_env() {
        std::env::remove_var("FUGLE_API_KEY");
        std::env::remove_var("FUGLE_BEARER_TOKEN");
        std::env::remove_var("FUGLE_SDK_TOKEN");
    }

    #[test]
    fn test_from_env_api_key_precedence() {
        let _guard = env_lock();
        clear_auth_env();
        std::env::set_var("FUGLE_API_KEY", "k1");
        std::env::set_var("FUGLE_BEARER_TOKEN", "t1");

        let auth = Auth::from_env().expect("api key path");
        match auth {
            Auth::ApiKey(v) => assert_eq!(v, "k1"),
            other => panic!("expected ApiKey precedence, got {:?}", other),
        }

        clear_auth_env();
    }

    #[test]
    fn test_from_env_bearer_fallback() {
        let _guard = env_lock();
        clear_auth_env();
        std::env::set_var("FUGLE_BEARER_TOKEN", "t1");

        let auth = Auth::from_env().expect("bearer path");
        match auth {
            Auth::BearerToken(v) => assert_eq!(v, "t1"),
            other => panic!("expected BearerToken, got {:?}", other),
        }

        clear_auth_env();
    }

    #[test]
    fn test_from_env_sdk_fallback() {
        let _guard = env_lock();
        clear_auth_env();
        std::env::set_var("FUGLE_SDK_TOKEN", "s1");

        let auth = Auth::from_env().expect("sdk path");
        match auth {
            Auth::SdkToken(v) => assert_eq!(v, "s1"),
            other => panic!("expected SdkToken, got {:?}", other),
        }

        clear_auth_env();
    }

    #[test]
    fn test_from_env_empty_treated_as_unset() {
        let _guard = env_lock();
        clear_auth_env();
        std::env::set_var("FUGLE_API_KEY", "");
        std::env::set_var("FUGLE_BEARER_TOKEN", "t1");

        let auth = Auth::from_env().expect("bearer fallback when api key empty");
        match auth {
            Auth::BearerToken(v) => assert_eq!(v, "t1"),
            other => panic!("expected BearerToken, got {:?}", other),
        }

        clear_auth_env();
    }

    #[test]
    fn test_from_env_none_set_returns_config_error() {
        let _guard = env_lock();
        clear_auth_env();

        let err = Auth::from_env().expect_err("no auth env vars");
        match err {
            MarketDataError::ConfigError(msg) => {
                assert!(msg.contains("FUGLE_API_KEY"));
                assert!(msg.contains("FUGLE_BEARER_TOKEN"));
                assert!(msg.contains("FUGLE_SDK_TOKEN"));
            }
            other => panic!("expected ConfigError, got {:?}", other),
        }
    }
}
