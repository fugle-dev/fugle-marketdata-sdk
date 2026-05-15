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
pub use errors::MarketDataError;

// Re-export client types (FutOpt now consolidated in client module)
pub use client::{RestClient, StockClient, StockIntradayClient, FutOptClient, FutOptIntradayClient};

// Re-export TLS record
pub use tls::TlsConfigRecord;

// Re-export WebSocket types
pub use websocket::{WebSocketClient, WebSocketListener, WebSocketEndpoint};

// Setup UniFFI scaffolding using proc macros
// This replaces include_scaffolding!() and allows using derive macros for types
uniffi::setup_scaffolding!();

/// Create a REST client with API key authentication
///
/// # Arguments
/// * `api_key` - The Fugle API key
///
/// # Returns
/// A RestClient instance wrapped in Arc for thread-safe access
#[uniffi::export]
pub fn new_rest_client_with_api_key(api_key: String) -> Result<Arc<RestClient>, MarketDataError> {
    Ok(Arc::new(RestClient::new(Auth::ApiKey(api_key))))
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
    Ok(Arc::new(RestClient::new(Auth::BearerToken(bearer_token))))
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
    Ok(Arc::new(RestClient::new(Auth::SdkToken(sdk_token))))
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
    let mut client = RestClient::with_tls(auth, tls.to_core())?;
    if let Some(url) = base_url {
        client = client.with_base_url(&url);
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
