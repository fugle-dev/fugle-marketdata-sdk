//! Centralized endpoint URL constants for the Fugle marketdata SDK.
//!
//! All production endpoints are defined here so version bumps (e.g. `v1.0`
//! → `v2.0`) are a single-line change. Internal callers (`ConnectionConfig`,
//! `RestClient`) read from this module instead of inlining literal strings.
//!
//! # Custom endpoints
//!
//! Two override paths are supported:
//!
//! - REST: chain [`RestClient::base_url`](crate::RestClient::base_url) onto
//!   [`RestClient::new`](crate::RestClient::new) to point at a custom host.
//! - WebSocket: pass a custom URL directly to
//!   [`ConnectionConfig::new`](crate::websocket::ConnectionConfig::new), or
//!   build via [`WebSocketFactory::new`](crate::WebSocketFactory::new) and
//!   chain `.base_url(...)` to point at a custom host root; the factory
//!   appends `{API_VERSION}/{type}/streaming` automatically.

// ---- Full production endpoints (default values used by convenience constructors) ----

/// Stock market WebSocket streaming endpoint.
pub const STOCK_WS: &str = "wss://api.fugle.tw/marketdata/v1.0/stock/streaming";

/// Futures and options WebSocket streaming endpoint.
pub const FUTOPT_WS: &str = "wss://api.fugle.tw/marketdata/v1.0/futopt/streaming";

/// REST API base URL (host + version, no trailing slash).
pub const REST_BASE: &str = "https://api.fugle.tw/marketdata/v1.0";

// ---- Host roots (no version) used for staging / mock-server overrides ----

/// REST API host root (no version segment).
///
/// Combine with [`API_VERSION`] to derive a custom REST base.
pub const REST_BASE_ROOT: &str = "https://api.fugle.tw/marketdata";

/// WebSocket host root (no version segment).
///
/// Combine with [`API_VERSION`] and a channel suffix to derive a custom
/// WebSocket endpoint, or pass directly to
/// [`WebSocketFactory::base_url`](crate::WebSocketFactory::base_url) for
/// staging / mock-server overrides.
pub const WS_BASE_ROOT: &str = "wss://api.fugle.tw/marketdata";

/// API version segment shared by REST and WebSocket endpoints.
///
/// Bump in lockstep with [`STOCK_WS`] / [`FUTOPT_WS`] / [`REST_BASE`] when
/// the marketdata service publishes a new major version.
pub const API_VERSION: &str = "v1.0";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stock_ws() {
        assert_eq!(STOCK_WS, "wss://api.fugle.tw/marketdata/v1.0/stock/streaming");
    }

    #[test]
    fn test_futopt_ws() {
        assert_eq!(FUTOPT_WS, "wss://api.fugle.tw/marketdata/v1.0/futopt/streaming");
    }

    #[test]
    fn test_rest_base() {
        assert_eq!(REST_BASE, "https://api.fugle.tw/marketdata/v1.0");
    }

    #[test]
    fn test_roots_match_full_urls() {
        // Sanity: roots + version + channel must reconstruct the full URLs
        // so the "_at" helpers and the canonical constants agree.
        assert_eq!(
            format!("{}/{}/stock/streaming", WS_BASE_ROOT, API_VERSION),
            STOCK_WS
        );
        assert_eq!(
            format!("{}/{}/futopt/streaming", WS_BASE_ROOT, API_VERSION),
            FUTOPT_WS
        );
        assert_eq!(format!("{}/{}", REST_BASE_ROOT, API_VERSION), REST_BASE);
    }
}
