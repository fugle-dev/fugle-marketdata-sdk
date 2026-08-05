//! Convenience factory mirroring the JS / Python SDK shape, with
//! compile-time enforcement of required-field ordering.
//!
//! Usage:
//!
//! ```rust
//! use marketdata_core::websocket::WebSocketFactory;
//! use marketdata_core::AuthRequest;
//!
//! // Production default endpoint:
//! let stock_cfg = WebSocketFactory::new()
//!     .auth(AuthRequest::with_api_key("k"))
//!     .stock()?
//!     .build();
//!
//! // Custom base (staging / mock server). `base_url` carries the host and
//! // path prefix and nothing else — the factory appends the version segment
//! // and /{type}/streaming.
//! let factory = WebSocketFactory::new()
//!     .base_url("wss://staging.fugle.tw/marketdata")
//!     .auth(AuthRequest::with_api_key("k"));
//!
//! let stock_cfg = factory.stock()?.build();
//! let futopt_cfg = factory.futopt()?.build();
//! # Ok::<(), marketdata_core::MarketDataError>(())
//! ```
//!
//! Streaming versions are per-product options rather than something written
//! into `base_url`:
//!
//! ```rust
//! use marketdata_core::websocket::{FutOptVersion, WebSocketFactory};
//! use marketdata_core::AuthRequest;
//!
//! // futopt defaults to v1.1 (trial frames). Pin it to v1.0 to opt out.
//! let cfg = WebSocketFactory::new()
//!     .futopt_version(FutOptVersion::V1_0)
//!     .auth(AuthRequest::with_api_key("k"))
//!     .futopt()?
//!     .build();
//!
//! assert!(cfg.url.contains("/v1.0/futopt/streaming"));
//! # Ok::<(), marketdata_core::MarketDataError>(())
//! ```
//!
//! Mirrors `fugle-marketdata-node/src/websocket/factory.ts` and
//! `fugle-marketdata-python/fugle_marketdata/websocket/factory.py`, but
//! the Rust shape is typestate-enforced: `stock()` / `futopt()` only
//! become callable once `auth(...)` has been set. Calling them on a
//! freshly-constructed factory is a compile-time error:
//!
//! ```compile_fail
//! use marketdata_core::websocket::WebSocketFactory;
//! // No `.auth(...)` — `.stock()` is not defined on `WebSocketFactory<Unset>`.
//! let _ = WebSocketFactory::new().stock();
//! ```
//!
//! ```compile_fail
//! use marketdata_core::websocket::WebSocketFactory;
//! // Same gate for `.futopt()`.
//! let _ = WebSocketFactory::new().futopt();
//! ```

use crate::models::AuthRequest;
use crate::urls;
use crate::websocket::config::{ConnectionConfig, ConnectionConfigBuilder};
use crate::websocket::version::{FutOptVersion, StockVersion, VERSION_OPTION_HINT};
use crate::MarketDataError;

/// Initial typestate of [`WebSocketFactory`]: `auth` is not yet set.
///
/// Calling `.stock()` / `.futopt()` on a factory in this state is a
/// compile-time error.
#[derive(Debug, Clone, Default)]
pub struct Unset;

/// Typestate of [`WebSocketFactory`] after `auth(...)` has populated the
/// credential.
#[derive(Debug, Clone)]
pub struct WithAuth(AuthRequest);

/// Factory that derives stock + futopt WebSocket endpoint configurations
/// from a single authentication credential and an optional shared base URL.
///
/// The factory is generic over a typestate marker (`Unset` / `WithAuth`)
/// so the compiler enforces that `.auth(...)` is called before `.stock()`
/// / `.futopt()`.
#[derive(Clone, Debug)]
pub struct WebSocketFactory<S = Unset> {
    state: S,
    base_url: Option<String>,
    stock_version: StockVersion,
    futopt_version: FutOptVersion,
}

impl Default for WebSocketFactory<Unset> {
    fn default() -> Self {
        Self::new()
    }
}

impl WebSocketFactory<Unset> {
    /// Create a fresh factory pointing at the production WebSocket base URL
    /// ([`crate::urls::WS_BASE_ROOT`]). Returns a typestate-`Unset`
    /// instance; chain [`auth`](Self::auth) before
    /// [`stock`](WebSocketFactory::stock) / [`futopt`](WebSocketFactory::futopt).
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: Unset,
            base_url: None,
            stock_version: StockVersion::default(),
            futopt_version: FutOptVersion::default(),
        }
    }

    /// Set the authentication credential, advancing the typestate to
    /// [`WithAuth`].
    #[must_use]
    pub fn auth(self, auth: AuthRequest) -> WebSocketFactory<WithAuth> {
        WebSocketFactory {
            state: WithAuth(auth),
            base_url: self.base_url,
            stock_version: self.stock_version,
            futopt_version: self.futopt_version,
        }
    }
}

impl<S> WebSocketFactory<S> {
    /// Override the WebSocket base URL.
    ///
    /// Available in any state. `base` carries the **host and path prefix and
    /// nothing else** (e.g. `"wss://staging.fugle.tw/marketdata"`); the
    /// factory appends the version segment and `/{stock|futopt}/streaming`.
    /// Trailing slashes are stripped.
    ///
    /// A `base` that already ends in a version segment is rejected by
    /// [`stock`](WebSocketFactory::stock) /
    /// [`futopt`](WebSocketFactory::futopt) — the version comes from
    /// [`stock_version`](Self::stock_version) /
    /// [`futopt_version`](Self::futopt_version) instead.
    ///
    /// # ⚠️ Breaking change in 0.8.0
    ///
    /// 0.6.0 through 0.7.x required the opposite: `base` had to *include*
    /// `/v1.0`. Passing a 0.6-era base URL now fails with a
    /// [`MarketDataError::ConfigError`] naming the prefix to use instead.
    /// See `MIGRATION-0.8.md`.
    #[must_use]
    pub fn base_url(mut self, base: impl Into<String>) -> Self {
        self.base_url = Some(base.into());
        self
    }

    /// Pin the stock streaming version.
    ///
    /// Stock serves a single version, so this exists for symmetry and
    /// forward-compatibility rather than because there is a choice to make
    /// today.
    #[must_use]
    pub fn stock_version(mut self, version: StockVersion) -> Self {
        self.stock_version = version;
        self
    }

    /// Pin the futures/options streaming version.
    ///
    /// Defaults to [`FutOptVersion::V1_1`], which delivers trial-matching
    /// (試撮) frames on `trades` / `books`. Pass [`FutOptVersion::V1_0`] to
    /// opt out.
    #[must_use]
    pub fn futopt_version(mut self, version: FutOptVersion) -> Self {
        self.futopt_version = version;
        self
    }

    fn endpoint_for(&self, kind: &str, version: &str) -> Result<String, MarketDataError> {
        match self.base_url.as_deref() {
            // No-override path: use the canonical full endpoints from
            // `crate::urls` when the resolved version is also the default,
            // otherwise build from the host root.
            None => Ok(format!(
                "{}/{}/{}/streaming",
                urls::WS_BASE_ROOT,
                version,
                kind
            )),
            // Custom-base path: the SDK owns the version segment, so a base
            // that already carries one is ambiguous and gets rejected.
            Some(base) => {
                let prefix = urls::with_version(base, version, VERSION_OPTION_HINT)?;
                Ok(format!("{prefix}/{kind}/streaming"))
            }
        }
    }
}

impl WebSocketFactory<WithAuth> {
    /// Derived stock-streaming endpoint as a [`ConnectionConfigBuilder`].
    ///
    /// Chain further setters (`.message_buffer(...)`, etc.) then
    /// `.build()` to obtain the [`ConnectionConfig`]. Only available once
    /// `auth(...)` has been called.
    ///
    /// # Errors
    ///
    /// Returns [`MarketDataError::ConfigError`] if [`base_url`](Self::base_url)
    /// was given a prefix that already ends in a version segment.
    pub fn stock(&self) -> Result<ConnectionConfigBuilder, MarketDataError> {
        let url = self.endpoint_for("stock", self.stock_version.as_str())?;
        Ok(ConnectionConfig::builder(url, self.state.0.clone()))
    }

    /// Derived futures/options streaming endpoint as a
    /// [`ConnectionConfigBuilder`]. Only available once `auth(...)` has
    /// been called.
    ///
    /// Resolves to [`FutOptVersion::V1_1`] unless
    /// [`futopt_version`](Self::futopt_version) says otherwise.
    ///
    /// # Errors
    ///
    /// Returns [`MarketDataError::ConfigError`] if [`base_url`](Self::base_url)
    /// was given a prefix that already ends in a version segment.
    pub fn futopt(&self) -> Result<ConnectionConfigBuilder, MarketDataError> {
        let url = self.endpoint_for("futopt", self.futopt_version.as_str())?;
        Ok(ConnectionConfig::builder(url, self.state.0.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::urls::{FUTOPT_WS, STOCK_WS};

    #[test]
    fn test_default_stock_endpoint() {
        let factory = WebSocketFactory::new().auth(AuthRequest::with_api_key("k"));
        let cfg = factory.stock().unwrap().build();
        assert_eq!(cfg.url, STOCK_WS);
    }

    #[test]
    fn test_default_futopt_endpoint_is_v1_1() {
        // 0.8.0 behaviour change: futopt defaults to the latest version, so
        // trial frames arrive without opting in.
        let factory = WebSocketFactory::new().auth(AuthRequest::with_api_key("k"));
        let cfg = factory.futopt().unwrap().build();
        assert_eq!(cfg.url, FUTOPT_WS);
        assert!(cfg.url.contains("/v1.1/futopt/streaming"), "{}", cfg.url);
    }

    #[test]
    fn test_futopt_version_can_be_pinned_to_v1_0() {
        let factory = WebSocketFactory::new()
            .futopt_version(FutOptVersion::V1_0)
            .auth(AuthRequest::with_api_key("k"));
        let cfg = factory.futopt().unwrap().build();
        assert_eq!(
            cfg.url,
            "wss://api.fugle.tw/marketdata/v1.0/futopt/streaming"
        );
    }

    #[test]
    fn test_futopt_version_does_not_leak_into_stock() {
        // The two products resolve independently; pinning futopt must leave
        // the stock endpoint alone.
        let factory = WebSocketFactory::new()
            .futopt_version(FutOptVersion::V1_0)
            .auth(AuthRequest::with_api_key("k"));
        assert_eq!(factory.stock().unwrap().build().url, STOCK_WS);
    }

    #[test]
    fn test_custom_base_url_applied_to_stock() {
        // 0.8.0: base_url is host + path prefix only; the SDK appends /v1.0.
        let factory = WebSocketFactory::new()
            .base_url("wss://staging.fugle.tw/marketdata")
            .auth(AuthRequest::with_api_key("k"));
        let cfg = factory.stock().unwrap().build();
        assert_eq!(
            cfg.url,
            "wss://staging.fugle.tw/marketdata/v1.0/stock/streaming",
        );
    }

    #[test]
    fn test_custom_base_url_applied_to_futopt_with_version() {
        // The version segment on a custom base comes from the version option,
        // not from the base URL string.
        let factory = WebSocketFactory::new()
            .base_url("ws://localhost:8080")
            .auth(AuthRequest::with_api_key("k"));
        let cfg = factory.futopt().unwrap().build();
        assert_eq!(cfg.url, "ws://localhost:8080/v1.1/futopt/streaming");

        let pinned = WebSocketFactory::new()
            .base_url("ws://localhost:8080")
            .futopt_version(FutOptVersion::V1_0)
            .auth(AuthRequest::with_api_key("k"));
        assert_eq!(
            pinned.futopt().unwrap().build().url,
            "ws://localhost:8080/v1.0/futopt/streaming"
        );
    }

    #[test]
    fn test_base_url_strips_trailing_slashes() {
        let factory = WebSocketFactory::new()
            .base_url("wss://example.com/marketdata///")
            .auth(AuthRequest::with_api_key("k"));
        let cfg = factory.stock().unwrap().build();
        assert_eq!(
            cfg.url,
            "wss://example.com/marketdata/v1.0/stock/streaming",
        );
    }

    #[test]
    fn test_legacy_0_6_base_url_is_rejected() {
        // 0.6.0-0.7.x required base_url to INCLUDE the version segment.
        // 0.8.0 reverses that, and the rejection is loud rather than silent:
        // the previous behaviour would have produced a doubled version
        // segment. This pins the migration story for MIGRATION-0.8.md.
        let factory = WebSocketFactory::new()
            .base_url("wss://staging.fugle.tw/marketdata/v1.0") // <-- 0.6-era form
            .auth(AuthRequest::with_api_key("k"));

        let msg = match factory.stock() {
            Err(err) => err.to_string(),
            Ok(_) => panic!("0.6-era base_url must be rejected"),
        };
        assert!(msg.contains("/v1.0"), "names the offending segment: {msg}");
        assert!(
            msg.contains("'wss://staging.fugle.tw/marketdata'"),
            "names the prefix to use instead: {msg}"
        );
        assert!(
            msg.contains("futopt_version"),
            "points at the option that owns the version: {msg}"
        );
    }

    #[test]
    fn test_legacy_base_url_rejected_for_both_products() {
        let factory = WebSocketFactory::new()
            .base_url("wss://example.com/md/v1.1")
            .auth(AuthRequest::with_api_key("k"));
        assert!(factory.stock().is_err());
        assert!(factory.futopt().is_err());
    }

    #[test]
    fn test_factory_yields_independent_builders() {
        // Builders returned from .stock() / .futopt() must not share state
        // with the factory; chaining a setter on one MUST NOT affect the
        // other (otherwise a single factory could not produce multiple
        // distinct configurations).
        let factory = WebSocketFactory::new().auth(AuthRequest::with_api_key("k"));
        let a = factory.stock().unwrap().message_buffer(2048).build();
        let b = factory.stock().unwrap().message_buffer(4096).build();
        assert_eq!(a.message_buffer, 2048);
        assert_eq!(b.message_buffer, 4096);
    }

    #[test]
    fn test_chained_setters_compose_with_factory() {
        let factory = WebSocketFactory::new()
            .base_url("wss://staging.fugle.tw/marketdata")
            .auth(AuthRequest::with_api_key("k"));
        let cfg = factory
            .stock()
            .unwrap()
            .message_buffer(8192)
            .event_buffer(256)
            .build();
        assert_eq!(
            cfg.url,
            "wss://staging.fugle.tw/marketdata/v1.0/stock/streaming",
        );
        assert_eq!(cfg.message_buffer, 8192);
        assert_eq!(cfg.event_buffer, 256);
    }

    #[test]
    fn test_base_url_before_auth_compiles() {
        // base_url is callable in either state; chaining order is flexible.
        let _ = WebSocketFactory::new()
            .base_url("ws://example.com")
            .auth(AuthRequest::with_api_key("k"));
    }

    /// Typestate guard: `.stock()` must not exist on `WebSocketFactory<Unset>`.
    ///
    /// ```compile_fail
    /// use marketdata_core::websocket::WebSocketFactory;
    /// // This must fail because `stock()` is only implemented for
    /// // `WebSocketFactory<WithAuth>`.
    /// let _ = WebSocketFactory::new().stock();
    /// ```
    #[allow(dead_code, reason = "compile-fail doctest, never executed")]
    fn _stock_before_auth_must_not_compile() {}

    /// Typestate guard: `.futopt()` must not exist on `WebSocketFactory<Unset>`.
    ///
    /// ```compile_fail
    /// use marketdata_core::websocket::WebSocketFactory;
    /// let _ = WebSocketFactory::new().futopt();
    /// ```
    #[allow(dead_code, reason = "compile-fail doctest, never executed")]
    fn _futopt_before_auth_must_not_compile() {}
}
