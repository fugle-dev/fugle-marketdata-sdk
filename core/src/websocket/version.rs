//! Per-product WebSocket streaming versions.
//!
//! Each product serves its own set of streaming versions, and the sets are not
//! the same. The official Node / Python SDKs express this as a per-product map
//! (`{ futopt: 'v1.1' }`) validated at runtime, because a bare version string
//! would be ambiguous about which product it applied to.
//!
//! Rust doesn't need the runtime check: a distinct enum per product makes an
//! unsupported pairing unrepresentable, so `stock` can never be asked for a
//! version it doesn't serve.
//!
//! ```compile_fail
//! use marketdata_core::websocket::{FutOptVersion, WebSocketFactory};
//! use marketdata_core::AuthRequest;
//!
//! // `FutOptVersion` is not a `StockVersion` — futopt-only versions cannot
//! // reach the stock endpoint.
//! let _ = WebSocketFactory::new()
//!     .auth(AuthRequest::with_api_key("k"))
//!     .stock_version(FutOptVersion::V1_1);
//! ```

/// Appended to `base_url` rejections, pointing at the options that own the
/// version segment for streaming.
pub(crate) const VERSION_OPTION_HINT: &str =
    "The version comes from the streaming version options, \
     e.g. .futopt_version(FutOptVersion::V1_1).";

/// Streaming versions served by the stock product.
///
/// Stock has no v1.1: its trial-matching (試撮) frames have always been
/// streamed, so there was no compatibility break to gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum StockVersion {
    /// `v1.0` — the only version stock streaming serves.
    #[default]
    V1_0,
}

impl StockVersion {
    /// The URL path segment for this version.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::V1_0 => "v1.0",
        }
    }
}

/// Streaming versions served by the futures/options product.
///
/// `V1_1` is `V1_0` plus trial-matching (試撮, TAIFEX I022/I082) frames on the
/// `trades` and `books` channels, which carry a top-level `isTrial: true`.
///
/// Trial frames on `trades` / `books` only reach clients connected to v1.1.
/// They are *not* the only place trial data surfaces, though: `aggregates` is
/// not version-gated, and during a trial session its `lastPrice` / `lastSize`
/// are the trial values on every version — only the top-level `isTrial`
/// distinguishes them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum FutOptVersion {
    /// `v1.0` — no trial-matching frames on `trades` / `books`.
    V1_0,
    /// `v1.1` — adds trial-matching frames. **This is the default.**
    #[default]
    V1_1,
}

impl FutOptVersion {
    /// The URL path segment for this version.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::V1_0 => "v1.0",
            Self::V1_1 => "v1.1",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stock_default_is_v1_0() {
        assert_eq!(StockVersion::default(), StockVersion::V1_0);
        assert_eq!(StockVersion::default().as_str(), "v1.0");
    }

    #[test]
    fn test_futopt_default_is_v1_1() {
        // Behaviour change in 0.8.0: futopt defaults to the latest version,
        // which means trial frames arrive without opting in.
        assert_eq!(FutOptVersion::default(), FutOptVersion::V1_1);
        assert_eq!(FutOptVersion::default().as_str(), "v1.1");
    }

    #[test]
    fn test_futopt_v1_0_still_reachable() {
        assert_eq!(FutOptVersion::V1_0.as_str(), "v1.0");
    }
}
