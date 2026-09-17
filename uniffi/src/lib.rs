//! UniFFI bindings for Fugle marketdata-core
//!
//! This crate provides FFI bindings for C#, Go, C++, and other languages using UniFFI.
//!
//! # Architecture
//!
//! All methods return TYPED models (not JSON strings) for compile-time safety.
//! This enables native IDE support in target languages (IntelliSense in C#, etc.)
//!
//! # Error Handling
//!
//! Errors are mapped to the `MarketDataError` enum, which becomes exceptions in target languages:
//! - C#: `MarketDataException` with typed variants
//! - Go: `error` type with specific error types
//! - C++: `std::exception` subclasses
//!
//! # Example (C#)
//!
//! ```csharp
//! using MarketdataUniffi;
//!
//! var client = MarketdataUniffi.NewRestClientWithSdkToken("your-token");
//! var quote = await client.Stock().Intraday().GetQuoteAsync("2330");
//! Console.WriteLine(quote.LastPrice); // Strongly typed access
//! ```

// UniFFI exports each optional argument as a separate parameter — there is no
// way to express an options object that reads naturally in C#, Go, Java and
// C++ at once. Collapsing them into a record to satisfy the lint would change
// the generated API in all four languages.
#![allow(clippy::too_many_arguments)]
// Every `MarketDataError` variant carries an `ErrorInfo` record (#81), which
// makes the error large. UniFFI errors cannot hold a `Box`, and the variant
// shape is the generated API, so the size is accepted.
#![allow(clippy::result_large_err)]

mod client;
mod errors;
mod models;
mod tls;
mod websocket;

use std::sync::Arc;
use marketdata_core::Auth;

// Re-export model types for UniFFI scaffolding
pub use models::*;

// Re-export error type
pub use errors::{ErrorInfo, ErrorSourceKind, MarketDataError};

// Re-export client types (FutOpt now consolidated in client module)
pub use client::{RestClient, StockClient, StockIntradayClient, FutOptClient, FutOptIntradayClient};

// Re-export TLS record
pub use tls::TlsConfigRecord;

// Re-export WebSocket types
pub use websocket::{WebSocketClient, WebSocketListener, WebSocketEndpoint};

// Setup UniFFI scaffolding using proc macros
// This replaces include_scaffolding!() and allows using derive macros for types
uniffi::setup_scaffolding!();

/// Which credential [`validate_credentials`] accepted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum CredentialKind {
    /// `api_key` was the credential provided.
    ApiKey,
    /// `bearer_token` was the credential provided.
    BearerToken,
    /// `sdk_token` was the credential provided.
    SdkToken,
}

/// Check a set of credentials the way every client constructor does.
///
/// A value that is empty or only whitespace counts as not provided; exactly
/// one of the three must remain. Wrappers that accept all three options call
/// this and pass the value of the returned kind to the matching constructor,
/// so the rule and the error (a `ConfigError`, code 1004) come from the core.
#[uniffi::export]
pub fn validate_credentials(
    api_key: Option<String>,
    bearer_token: Option<String>,
    sdk_token: Option<String>,
) -> Result<CredentialKind, MarketDataError> {
    Ok(match Auth::from_credentials(api_key, bearer_token, sdk_token)? {
        Auth::ApiKey(_) => CredentialKind::ApiKey,
        Auth::BearerToken(_) => CredentialKind::BearerToken,
        Auth::SdkToken(_) => CredentialKind::SdkToken,
    })
}

/// Validate a single credential and build a REST client from it.
fn build_rest_client(auth: Auth) -> Result<Arc<RestClient>, MarketDataError> {
    auth.validate()?;
    Ok(Arc::new(RestClient::new(auth)))
}

/// Create a REST client with API key authentication
///
/// # Arguments
/// * `api_key` - The Fugle API key
///
/// # Returns
/// A RestClient instance wrapped in Arc for thread-safe access
#[uniffi::export]
pub fn new_rest_client_with_api_key(api_key: String) -> Result<Arc<RestClient>, MarketDataError> {
    build_rest_client(Auth::ApiKey(api_key))
}

/// Create a REST client with bearer token authentication
///
/// # Arguments
/// * `bearer_token` - OAuth bearer token
///
/// # Returns
/// A RestClient instance wrapped in Arc for thread-safe access
#[uniffi::export]
pub fn new_rest_client_with_bearer_token(bearer_token: String) -> Result<Arc<RestClient>, MarketDataError> {
    build_rest_client(Auth::BearerToken(bearer_token))
}

/// Create a REST client with SDK token authentication
///
/// # Arguments
/// * `sdk_token` - Fugle SDK token
///
/// # Returns
/// A RestClient instance wrapped in Arc for thread-safe access
#[uniffi::export]
pub fn new_rest_client_with_sdk_token(sdk_token: String) -> Result<Arc<RestClient>, MarketDataError> {
    build_rest_client(Auth::SdkToken(sdk_token))
}

// ============================================================================
// TLS-aware REST factories — rc.9
//
// Additive. The three original factories above stay untouched for existing
// consumers. These variants expose `base_url` override (previously only WS
// had it) and a `TlsConfigRecord` so Java/C#/Go/C++ callers can pin a
// custom CA or disable cert checking — parity with the py binding kwargs.
// ============================================================================

fn build_rest_client_with_tls(
    auth: Auth,
    base_url: Option<String>,
    tls: TlsConfigRecord,
) -> Result<Arc<RestClient>, MarketDataError> {
    auth.validate()?;
    let mut client = RestClient::with_tls(auth, tls.to_core())?;
    if let Some(url) = base_url {
        client = client.with_base_url(&url)?;
    }
    Ok(Arc::new(client))
}

/// Create a REST client with API key authentication, custom base URL, and TLS config
#[uniffi::export]
pub fn new_rest_client_with_api_key_and_tls(
    api_key: String,
    base_url: Option<String>,
    tls: TlsConfigRecord,
) -> Result<Arc<RestClient>, MarketDataError> {
    build_rest_client_with_tls(Auth::ApiKey(api_key), base_url, tls)
}

/// Create a REST client with bearer token authentication, custom base URL, and TLS config
#[uniffi::export]
pub fn new_rest_client_with_bearer_token_and_tls(
    bearer_token: String,
    base_url: Option<String>,
    tls: TlsConfigRecord,
) -> Result<Arc<RestClient>, MarketDataError> {
    build_rest_client_with_tls(Auth::BearerToken(bearer_token), base_url, tls)
}

/// Create a REST client with SDK token authentication, custom base URL, and TLS config
#[uniffi::export]
pub fn new_rest_client_with_sdk_token_and_tls(
    sdk_token: String,
    base_url: Option<String>,
    tls: TlsConfigRecord,
) -> Result<Arc<RestClient>, MarketDataError> {
    build_rest_client_with_tls(Auth::SdkToken(sdk_token), base_url, tls)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_config_error<T>(result: Result<T, MarketDataError>) {
        match result {
            Err(MarketDataError::ConfigError { info, .. }) => {
                assert_eq!(info.code, marketdata_core::error_code::CONFIG)
            }
            Err(other) => panic!("expected ConfigError, got {other:?}"),
            Ok(_) => panic!("expected ConfigError, got Ok"),
        }
    }

    #[test]
    fn validate_credentials_requires_exactly_one_non_blank() {
        assert_eq!(
            validate_credentials(Some(" ".into()), Some("t".into()), None).unwrap(),
            CredentialKind::BearerToken
        );
        assert_eq!(
            validate_credentials(None, Some("".into()), Some("s".into())).unwrap(),
            CredentialKind::SdkToken
        );
        assert_eq!(
            validate_credentials(Some("k".into()), None, None).unwrap(),
            CredentialKind::ApiKey
        );
        assert_config_error(validate_credentials(None, None, None));
        assert_config_error(validate_credentials(Some("".into()), None, Some("\t".into())));
        assert_config_error(validate_credentials(Some("k".into()), Some("t".into()), None));
    }

    #[test]
    fn rest_factories_reject_blank_credentials() {
        assert_config_error(new_rest_client_with_api_key(String::new()));
        assert_config_error(new_rest_client_with_bearer_token("  ".into()));
        assert_config_error(new_rest_client_with_sdk_token_and_tls(
            String::new(),
            None,
            TlsConfigRecord { root_cert_pem: None, accept_invalid_certs: false },
        ));
        assert!(new_rest_client_with_api_key("k".into()).is_ok());
    }
}
