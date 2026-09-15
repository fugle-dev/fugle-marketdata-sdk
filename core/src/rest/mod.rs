//! REST client module for Fugle marketdata API
//!
//! This module provides:
//! - Authentication mechanisms (API Key, Bearer Token, SDK Token)
//! - HTTP client with connection pooling (ureq)
//! - Error conversion from HTTP outcomes to MarketDataError
//! - Stock endpoints (intraday, historical)
//! - FutOpt endpoints (futures and options)

mod auth;
mod client;
mod error;
mod retry;
mod symbol_path;

pub(crate) use symbol_path::encode_symbol;

#[cfg(test)]
mod http_tests;

/// Decode a JSON response body.
///
/// Reads the whole (already decompressed) body, then parses it with
/// `serde_json::from_slice`. Parsing from a reader works byte by byte through
/// `io::Read` and is 1.2-3.5x slower per request, most of all for large or
/// gzip-encoded bodies such as historical candles. See `benches/rest_decode.rs`.
///
/// There is no body size limit, matching the SDK's behaviour before ureq 3,
/// whose default limit is 10 MB.
///
/// Read and parse failures both map to `MarketDataError::Other`.
pub(crate) fn read_json<T: serde::de::DeserializeOwned>(
    mut response: client::HttpResponse,
) -> Result<T, crate::errors::MarketDataError> {
    let body = response
        .body_mut()
        .with_config()
        .limit(u64::MAX)
        .read_to_vec()
        .map_err(|e| crate::errors::MarketDataError::Other(e.into()))?;
    serde_json::from_slice(&body).map_err(|e| crate::errors::MarketDataError::Other(e.into()))
}

// Stock endpoints module
pub mod stock;

// FutOpt (Futures and Options) endpoints module
pub mod futopt;

// Re-export public types
pub use auth::Auth;
pub use client::{IntradayClient, RestClient, StockClient};
pub use futopt::{FutOptClient, FutOptIntradayClient};
pub use retry::RetryPolicy;
pub use stock::snapshot::SnapshotClient;
