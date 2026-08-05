//! Centralized endpoint URL constants for the Fugle marketdata SDK.
//!
//! All production endpoints are defined here so version bumps (e.g. `v1.0`
//! → `v2.0`) are a single-line change. Internal callers (`ConnectionConfig`,
//! `RestClient`) read from this module instead of inlining literal strings.
//!
//! # Custom endpoints
//!
//! A caller-supplied base URL carries the **host and path prefix and nothing
//! else** — the version segment is always appended by the SDK. A base URL that
//! already ends in a version segment is rejected by [`with_version`] rather
//! than swapped or appended on top of.
//!
//! Letting two options decide the same path segment is what forces precedence
//! rules, and those rules mean anyone who only wants to change host is made to
//! manage the version by hand. For WebSocket the version comes from the
//! [`StockVersion`](crate::websocket::StockVersion) /
//! [`FutOptVersion`](crate::websocket::FutOptVersion) options; REST serves a
//! single version, so there is no option to choose it with.
//!
//! - REST: chain [`RestClient::base_url`](crate::RestClient::base_url) onto
//!   [`RestClient::new`](crate::RestClient::new) to point at a custom host.
//! - WebSocket: chain
//!   [`WebSocketFactory::base_url`](crate::WebSocketFactory::base_url).
//!
//! # ⚠️ Breaking change in 0.8.0
//!
//! 0.6.0 through 0.7.x required the caller to *include* the version segment.
//! 0.8.0 reverses that: including it is now an error. This realigns with the
//! official Node / Python SDKs (`@fugle/marketdata` 1.5.0, `fugle-marketdata`
//! 2.5.0). See `MIGRATION-0.8.md`.

use crate::MarketDataError;

// ---- Full production endpoints (default values used by convenience constructors) ----

/// Stock market WebSocket streaming endpoint, at the default stock version
/// ([`StockVersion::V1_0`](crate::websocket::StockVersion::V1_0)).
pub const STOCK_WS: &str = "wss://api.fugle.tw/marketdata/v1.0/stock/streaming";

/// Futures and options WebSocket streaming endpoint, at the default futopt
/// version ([`FutOptVersion::V1_1`](crate::websocket::FutOptVersion::V1_1)).
///
/// # ⚠️ Behaviour change in 0.8.0
///
/// This moved from `v1.0` to `v1.1`, so connections built from it now receive
/// trial-matching (試撮) frames on `trades` / `books`. Branch on the frame's
/// `is_trial` before acting on a price. Pin
/// [`FutOptVersion::V1_0`](crate::websocket::FutOptVersion::V1_0) on
/// [`WebSocketFactory`](crate::WebSocketFactory) to opt out.
pub const FUTOPT_WS: &str = "wss://api.fugle.tw/marketdata/v1.1/futopt/streaming";

/// REST API base URL (host + version, no trailing slash).
pub const REST_BASE: &str = "https://api.fugle.tw/marketdata/v1.0";

// ---- Host roots (no version) — what `base_url` overrides now take ----

/// REST API host root (no version segment).
///
/// This is the shape [`RestClient::base_url`](crate::RestClient::base_url)
/// expects: host and path prefix only.
pub const REST_BASE_ROOT: &str = "https://api.fugle.tw/marketdata";

/// WebSocket host root (no version segment).
///
/// This is the shape
/// [`WebSocketFactory::base_url`](crate::WebSocketFactory::base_url) expects:
/// host and path prefix only.
pub const WS_BASE_ROOT: &str = "wss://api.fugle.tw/marketdata";

/// API version segment used by REST.
///
/// WebSocket versions are per-product and live in
/// [`crate::websocket::version`]; this constant covers REST, which serves a
/// single version.
pub const API_VERSION: &str = "v1.0";

/// Return the trailing `/vMAJOR.MINOR` segment of `s`, if it has one.
///
/// Equivalent to the official SDK's `/\/v\d+\.\d+$/` test, without pulling in
/// a regex dependency.
fn trailing_version_segment(s: &str) -> Option<&str> {
    let slash = s.rfind('/')?;
    let segment = &s[slash..];
    let body = segment.strip_prefix("/v")?;
    let (major, minor) = body.split_once('.')?;

    let numeric = |part: &str| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit());
    (numeric(major) && numeric(minor)).then_some(segment)
}

/// Join a caller-supplied `base` with the version the SDK resolved.
///
/// `base` carries the host and path prefix and nothing else; the version
/// segment is always appended here. Trailing slashes are stripped.
///
/// `hint` is appended to the rejection message to point at whichever option
/// owns the version for this transport — pass `""` when there isn't one.
///
/// # Errors
///
/// Returns [`MarketDataError::ConfigError`] if `base` already ends in a
/// version segment. It is rejected rather than swapped or appended on top of,
/// because letting two options decide the same path segment is what forces
/// precedence rules.
pub fn with_version(base: &str, version: &str, hint: &str) -> Result<String, MarketDataError> {
    let trimmed = base.trim_end_matches('/');

    if let Some(existing) = trailing_version_segment(trimmed) {
        let prefix = &trimmed[..trimmed.len() - existing.len()];
        return Err(MarketDataError::ConfigError(format!(
            "base_url must not include a version segment (found '{existing}'). \
             Pass the host and path prefix only: '{prefix}'.{}",
            if hint.is_empty() {
                String::new()
            } else {
                format!(" {hint}")
            }
        )));
    }

    Ok(format!("{trimmed}/{version}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stock_ws() {
        assert_eq!(STOCK_WS, "wss://api.fugle.tw/marketdata/v1.0/stock/streaming");
    }

    #[test]
    fn test_futopt_ws() {
        // 0.8.0: the futopt default moved to v1.1 (trial frames).
        assert_eq!(FUTOPT_WS, "wss://api.fugle.tw/marketdata/v1.1/futopt/streaming");
    }

    #[test]
    fn test_rest_base() {
        assert_eq!(REST_BASE, "https://api.fugle.tw/marketdata/v1.0");
    }

    #[test]
    fn test_roots_match_full_urls() {
        // Sanity: roots + version + channel must reconstruct the full URLs so
        // the canonical constants and the per-product version defaults agree.
        // A drift here means `ConnectionConfig::stock()` / `::futopt()` would
        // resolve to a different endpoint than `WebSocketFactory`.
        use crate::websocket::{FutOptVersion, StockVersion};

        assert_eq!(
            format!(
                "{}/{}/stock/streaming",
                WS_BASE_ROOT,
                StockVersion::default().as_str()
            ),
            STOCK_WS
        );
        assert_eq!(
            format!(
                "{}/{}/futopt/streaming",
                WS_BASE_ROOT,
                FutOptVersion::default().as_str()
            ),
            FUTOPT_WS
        );
        assert_eq!(format!("{}/{}", REST_BASE_ROOT, API_VERSION), REST_BASE);
    }

    #[test]
    fn test_with_version_appends() {
        assert_eq!(
            with_version("https://api.fugle.tw/marketdata", "v1.0", "").unwrap(),
            "https://api.fugle.tw/marketdata/v1.0"
        );
    }

    #[test]
    fn test_with_version_strips_trailing_slashes() {
        assert_eq!(
            with_version("https://example.com/marketdata///", "v1.0", "").unwrap(),
            "https://example.com/marketdata/v1.0"
        );
    }

    #[test]
    fn test_with_version_rejects_version_segment() {
        let err = with_version("https://api.fugle.tw/marketdata/v1.0", "v1.0", "")
            .expect_err("a base ending in a version segment must be rejected");
        let msg = err.to_string();
        assert!(msg.contains("/v1.0"), "message names the offending segment: {msg}");
        assert!(
            msg.contains("'https://api.fugle.tw/marketdata'"),
            "message names the prefix to use instead: {msg}"
        );
    }

    #[test]
    fn test_with_version_rejects_any_version_segment() {
        // Not just the current one — a base pinned to a different version is
        // equally ambiguous.
        assert!(with_version("wss://example.com/md/v2.11", "v1.0", "").is_err());
    }

    #[test]
    fn test_with_version_rejection_carries_hint() {
        let err = with_version("wss://example.com/md/v1.0", "v1.0", "Use the version option.")
            .expect_err("must reject");
        assert!(err.to_string().ends_with("Use the version option."));
    }

    #[test]
    fn test_with_version_allows_version_like_prefixes() {
        // Only a *trailing* `/vN.N` is a version segment. These are not.
        for base in [
            "https://example.com/v1.0/marketdata",
            "https://example.com/v1",
            "https://example.com/version",
            "https://example.com/v1.0.1",
            "https://v1.0.example.com/md",
        ] {
            assert!(
                with_version(base, "v1.0", "").is_ok(),
                "{base} should not be treated as ending in a version segment"
            );
        }
    }
}
