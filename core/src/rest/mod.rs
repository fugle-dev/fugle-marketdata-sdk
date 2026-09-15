//! REST client module for Fugle marketdata API
//!
//! This module provides:
//! - Authentication mechanisms (API Key, Bearer Token, SDK Token)
//! - HTTP client with connection pooling via ureq
//! - Error conversion from ureq to MarketDataError
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
/// Reads the whole (already decompressed) body into memory and parses it with
/// `serde_json::from_slice`. ureq's `Response::into_json` parses straight from
/// the socket reader, and `serde_json::from_reader` works byte by byte through
/// `io::Read`; buffering first is 1.2-3.5x faster per request, most of all for
/// large or gzip-encoded bodies such as historical candles. See
/// `benches/rest_decode.rs`.
///
/// Read and parse failures both map to `MarketDataError::Other`, the variant
/// `into_json` produced.
pub(crate) fn read_json<T: serde::de::DeserializeOwned>(
    response: ureq::Response,
) -> Result<T, crate::errors::MarketDataError> {
    use std::io::Read;

    // Content-Length is the compressed size for gzip bodies, so it is only a
    // starting capacity. Capped so a hostile header cannot force a huge
    // allocation up front.
    const MAX_PREALLOC: usize = 8 * 1024 * 1024;
    let capacity = response
        .header("Content-Length")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(8 * 1024)
        .min(MAX_PREALLOC);

    let mut body = Vec::with_capacity(capacity);
    response
        .into_reader()
        .read_to_end(&mut body)
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
