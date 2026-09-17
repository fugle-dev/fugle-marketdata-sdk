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

/// Message for a missing, blank or ambiguous credential set.
const CREDENTIALS_MESSAGE: &str =
    "Provide exactly one non-empty credential: API key, bearer token, or SDK token";

/// A credential that is empty or only whitespace counts as not provided.
fn is_provided(value: &str) -> bool {
    !value.trim().is_empty()
}

/// Require exactly one provided credential among the three kinds.
///
/// Shared by [`Auth`] and [`AuthRequest`](crate::AuthRequest) so REST and
/// WebSocket clients reject the same inputs with the same error.
pub(crate) fn check_credentials(
    api_key: Option<&str>,
    bearer_token: Option<&str>,
    sdk_token: Option<&str>,
) -> Result<(), MarketDataError> {
    let provided = [api_key, bearer_token, sdk_token]
        .into_iter()
        .flatten()
        .filter(|value| is_provided(value))
        .count();
    if provided == 1 {
        Ok(())
    } else {
        Err(MarketDataError::ConfigError(CREDENTIALS_MESSAGE.to_string()))
    }
}

impl Auth {
    /// Build the credential from the three optional inputs a binding exposes.
    ///
    /// A value that is empty or only whitespace counts as not provided;
    /// exactly one of the three must remain. The returned variant records
    /// which kind was given.
    ///
    /// # Errors
    ///
    /// Returns [`MarketDataError::ConfigError`] when none or more than one
    /// credential is provided.
    ///
    /// # Example
    ///
    /// ```rust
    /// use marketdata_core::Auth;
    ///
    /// let auth = Auth::from_credentials(None, Some("token".into()), Some("".into())).unwrap();
    /// assert!(matches!(auth, Auth::BearerToken(_)));
    /// assert!(Auth::from_credentials(Some("  ".into()), None, None).is_err());
    /// ```
    pub fn from_credentials(
        api_key: Option<String>,
        bearer_token: Option<String>,
        sdk_token: Option<String>,
    ) -> Result<Self, MarketDataError> {
        check_credentials(api_key.as_deref(), bearer_token.as_deref(), sdk_token.as_deref())?;
        let provided = |value: &Option<String>| value.as_deref().is_some_and(is_provided);
        Ok(if provided(&api_key) {
            Auth::ApiKey(api_key.unwrap_or_default())
        } else if provided(&bearer_token) {
            Auth::BearerToken(bearer_token.unwrap_or_default())
        } else {
            Auth::SdkToken(sdk_token.unwrap_or_default())
        })
    }

    /// Check that the credential is not empty or only whitespace.
    ///
    /// # Errors
    ///
    /// Returns [`MarketDataError::ConfigError`] for a blank credential.
    pub fn validate(&self) -> Result<(), MarketDataError> {
        check_credentials(Some(self.secret()), None, None)
    }

    fn secret(&self) -> &str {
        match self {
            Auth::ApiKey(value) | Auth::BearerToken(value) | Auth::SdkToken(value) => value,
        }
    }

    /// The HTTP header carrying this credential.
    pub(crate) fn header(&self) -> (&'static str, String) {
        match self {
            Auth::ApiKey(key) => ("X-API-KEY", key.clone()),
            Auth::BearerToken(token) => ("Authorization", format!("Bearer {token}")),
            Auth::SdkToken(token) => ("X-SDK-TOKEN", token.clone()),
        }
    }

    /// Resolve authentication from the process environment.
    ///
    /// Probes the variables `FUGLE_API_KEY`, `FUGLE_BEARER_TOKEN`, and
    /// `FUGLE_SDK_TOKEN` in that order and returns the first non-empty
    /// match wrapped in the corresponding `Auth` variant. Empty or
    /// whitespace-only values are treated as unset.
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
        #[allow(clippy::type_complexity, reason = "trivial env-var probe table")]
        const VARS: &[(&str, fn(String) -> Auth)] = &[
            ("FUGLE_API_KEY", Auth::ApiKey),
            ("FUGLE_BEARER_TOKEN", Auth::BearerToken),
            ("FUGLE_SDK_TOKEN", Auth::SdkToken),
        ];

        for (name, ctor) in VARS {
            if let Ok(value) = std::env::var(name) {
                if is_provided(&value) {
                    return Ok(ctor(value));
                }
            }
        }

        Err(MarketDataError::ConfigError(format!(
            "No authentication credentials found in environment. \
             Set one of: {}, {}, or {} to a non-empty value.",
            VARS[0].0, VARS[1].0, VARS[2].0
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_header() {
        assert_eq!(
            Auth::ApiKey("test_key".into()).header(),
            ("X-API-KEY", "test_key".to_string())
        );
        assert_eq!(
            Auth::BearerToken("test_token".into()).header(),
            ("Authorization", "Bearer test_token".to_string())
        );
        assert_eq!(
            Auth::SdkToken("test_sdk_token".into()).header(),
            ("X-SDK-TOKEN", "test_sdk_token".to_string())
        );
    }

    #[test]
    fn test_from_credentials_keeps_kind() {
        let key = Auth::from_credentials(Some("k".into()), None, None).unwrap();
        assert!(matches!(key, Auth::ApiKey(ref v) if v == "k"));
        let bearer = Auth::from_credentials(None, Some("t".into()), None).unwrap();
        assert!(matches!(bearer, Auth::BearerToken(ref v) if v == "t"));
        let sdk = Auth::from_credentials(None, None, Some("s".into())).unwrap();
        assert!(matches!(sdk, Auth::SdkToken(ref v) if v == "s"));
    }

    #[test]
    fn test_from_credentials_ignores_blank_values() {
        let auth = Auth::from_credentials(Some("".into()), Some(" \t".into()), Some("s".into()))
            .unwrap();
        assert!(matches!(auth, Auth::SdkToken(ref v) if v == "s"));
    }

    #[test]
    fn test_from_credentials_rejects_none_blank_or_multiple() {
        for (api_key, bearer_token, sdk_token) in [
            (None, None, None),
            (Some(""), None, None),
            (Some("   "), None, None),
            (None, Some("\n"), Some("")),
            (Some("k"), Some("t"), None),
            (Some("k"), None, Some("s")),
        ] {
            let err = Auth::from_credentials(
                api_key.map(String::from),
                bearer_token.map(String::from),
                sdk_token.map(String::from),
            )
            .expect_err("credentials should be rejected");
            assert!(matches!(err, MarketDataError::ConfigError(_)), "{err:?}");
            assert_eq!(err.info().code, crate::error_code::CONFIG);
            assert!(err.to_string().contains("exactly one non-empty credential"));
        }
    }

    #[test]
    fn test_validate_rejects_blank_credential() {
        assert!(Auth::ApiKey("k".into()).validate().is_ok());
        for auth in [
            Auth::ApiKey(String::new()),
            Auth::BearerToken("  ".into()),
            Auth::SdkToken("\t".into()),
        ] {
            assert!(matches!(auth.validate(), Err(MarketDataError::ConfigError(_))));
        }
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
        std::env::set_var("FUGLE_BEARER_TOKEN", "  ");
        std::env::set_var("FUGLE_SDK_TOKEN", "s1");

        let auth = Auth::from_env().expect("sdk fallback when others are blank");
        match auth {
            Auth::SdkToken(v) => assert_eq!(v, "s1"),
            other => panic!("expected SdkToken, got {:?}", other),
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
