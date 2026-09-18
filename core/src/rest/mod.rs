//! REST client module for Fugle marketdata API
//!
//! This module provides:
//! - Authentication mechanisms (API Key, Bearer Token, SDK Token)
//! - HTTP client with connection pooling (ureq)
//! - Error conversion from HTTP outcomes to MarketDataError
//! - Stock endpoints (intraday, historical)
//! - FutOpt endpoints (futures and options)

pub(crate) mod auth;
mod client;
mod error;
mod retry;
mod symbol_path;

pub(crate) use symbol_path::encode_symbol;

/// Format one `key=value` query pair with `value` form-urlencoded.
///
/// Without encoding, a value carrying `&`, `=`, `#`, `+` or a space would
/// split into extra params, be truncated, or make the URL invalid. `key` is
/// not encoded — every typed builder passes a literal param name.
pub(crate) fn query_pair(key: &str, value: impl std::fmt::Display) -> String {
    let value = value.to_string();
    let encoded: String = url::form_urlencoded::byte_serialize(value.as_bytes()).collect();
    format!("{key}={encoded}")
}

#[cfg(test)]
mod query_pair_tests {
    use super::query_pair;

    #[test]
    fn test_plain_values_are_unchanged() {
        assert_eq!(query_pair("from", "2026-09-15"), "from=2026-09-15");
        assert_eq!(query_pair("timeframe", "D"), "timeframe=D");
        assert_eq!(query_pair("limit", 50u32), "limit=50");
        assert_eq!(query_pair("isTrial", true), "isTrial=true");
        assert_eq!(query_pair("gte", 2.5f64), "gte=2.5");
        assert_eq!(query_pair("contractMonth", "202609"), "contractMonth=202609");
    }

    #[test]
    fn test_reserved_characters_are_encoded() {
        assert_eq!(query_pair("fields", "open,close"), "fields=open%2Cclose");
        assert_eq!(query_pair("industry", "24&type=ETF"), "industry=24%26type%3DETF");
        assert_eq!(query_pair("date", "2026-09-15#x"), "date=2026-09-15%23x");
        assert_eq!(query_pair("to", "a+b c"), "to=a%2Bb+c");
        assert_eq!(query_pair("contractMonth", "1!"), "contractMonth=1%21");
        assert_eq!(query_pair("q", "100%"), "q=100%25");
        assert_eq!(query_pair("name", "台積電"), "name=%E5%8F%B0%E7%A9%8D%E9%9B%BB");
    }
}

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

// Which query keys each endpoint accepts — for the bindings, like
// `RestClient::get_json`; hidden from the docs and the public-API baseline.
#[doc(hidden)]
pub mod params;

// Re-export public types
pub use auth::Auth;
pub use client::{IntradayClient, RestClient, StockClient};
pub use futopt::{FutOptClient, FutOptIntradayClient};
pub use retry::RetryPolicy;
pub use stock::snapshot::SnapshotClient;

#[cfg(test)]
mod tests {
    /// Long decimals must land on the same double as a correctly-rounded parser
    /// (and JavaScript's `JSON.parse`). serde_json's default fast float parser
    /// can pick an adjacent double; the `float_roundtrip` feature fixes that.
    #[test]
    fn json_floats_parse_correctly_rounded() {
        let body = br#"[51.708947112827516, -9.419062495727303, 2441.4726242670918]"#;
        let parsed: Vec<f64> = serde_json::from_slice(body).unwrap();
        let expected = [
            51.708947112827516_f64,
            -9.419062495727303,
            2441.4726242670918,
        ];
        for (got, want) in parsed.iter().zip(expected) {
            assert_eq!(got.to_bits(), want.to_bits(), "got {got}, want {want}");
        }
    }
}
