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
//!     .stock()
//!     .build();
//!
//! // Custom base (staging / mock server). `base_url` follows the OpenAI /
//! // Stripe / AWS convention — caller passes the FULL prefix including
//! // the API version segment; the factory appends only /{type}/streaming.
//! let factory = WebSocketFactory::new()
//!     .base_url("wss://staging.fugle.tw/marketdata/v1.0")
//!     .auth(AuthRequest::with_api_key("k"));
//!
//! let stock_cfg = factory.stock().build();
//! let futopt_cfg = factory.futopt().build();
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
use crate::websocket::config::{ConnectionConfig, ConnectionConfigBuilder};

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
        }
    }

    /// Set the authentication credential, advancing the typestate to
    /// [`WithAuth`].
    #[must_use]
    pub fn auth(self, auth: AuthRequest) -> WebSocketFactory<WithAuth> {
        WebSocketFactory {
            state: WithAuth(auth),
            base_url: self.base_url,
        }
    }
}

impl<S> WebSocketFactory<S> {
    /// Override the WebSocket base URL.
    ///
    /// Available in any state. **`base` MUST include the API version
    /// segment** (e.g. `"wss://api.fugle.tw/marketdata/v1.0"`). The factory
    /// appends only `/{stock|futopt}/streaming` to whatever you pass;
    /// it does NOT inject `/{API_VERSION}` for caller-supplied bases.
    /// Trailing slashes are stripped.
    ///
    /// # ⚠️ Silent breaking change in 0.6.0
    ///
    /// Pre-0.6.0 this method accepted a host root (no version) and the
    /// factory silently appended `/{API_VERSION}`. As of 0.6.0 the
    /// caller owns the full prefix, matching the OpenAI / Stripe / AWS
    /// SDK convention. Code that worked in 0.5.x will compile against
    /// 0.6.0 but produce a 404 on first connect because the URL lacks
    /// the `/v1.0` segment. See `MIGRATION-0.6.md`.
    #[must_use]
    pub fn base_url(mut self, base: impl Into<String>) -> Self {
        self.base_url = Some(base.into());
        self
    }

    fn endpoint_for(&self, kind: &str) -> String {
        // No-override path: use the canonical full endpoints from
        // `crate::urls` directly. These already include the version
        // segment, so no concatenation is needed.
        let canonical = match kind {
            "stock" => crate::urls::STOCK_WS,
            "futopt" => crate::urls::FUTOPT_WS,
            _ => unreachable!("endpoint_for only called with \"stock\" / \"futopt\""),
        };
        match self.base_url.as_deref() {
            None => canonical.to_string(),
            // Custom-base path: append only the channel suffix; the caller
            // owns everything up to and including the version segment.
            Some(base) => format!("{}/{}/streaming", base.trim_end_matches('/'), kind),
        }
    }
}

impl WebSocketFactory<WithAuth> {
    /// Derived stock-streaming endpoint as a [`ConnectionConfigBuilder`].
    ///
    /// Chain further setters (`.message_buffer(...)`, etc.) then
    /// `.build()` to obtain the [`ConnectionConfig`]. Only available once
    /// `auth(...)` has been called.
    pub fn stock(&self) -> ConnectionConfigBuilder {
        ConnectionConfig::builder(self.endpoint_for("stock"), self.state.0.clone())
    }

    /// Derived futures/options streaming endpoint as a
    /// [`ConnectionConfigBuilder`]. Only available once `auth(...)` has
    /// been called.
    pub fn futopt(&self) -> ConnectionConfigBuilder {
        ConnectionConfig::builder(self.endpoint_for("futopt"), self.state.0.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::urls::{FUTOPT_WS, STOCK_WS};

    #[test]
    fn test_default_stock_endpoint() {
        let factory = WebSocketFactory::new().auth(AuthRequest::with_api_key("k"));
        let cfg = factory.stock().build();
        assert_eq!(cfg.url, STOCK_WS);
    }

    #[test]
    fn test_default_futopt_endpoint() {
        let factory = WebSocketFactory::new().auth(AuthRequest::with_api_key("k"));
        let cfg = factory.futopt().build();
        assert_eq!(cfg.url, FUTOPT_WS);
    }

    #[test]
    fn test_custom_base_url_applied_to_stock() {
        // 0.6.0: base_url MUST include the API version segment.
        let factory = WebSocketFactory::new()
            .base_url("wss://staging.fugle.tw/marketdata/v1.0")
            .auth(AuthRequest::with_api_key("k"));
        let cfg = factory.stock().build();
        assert_eq!(
            cfg.url,
            "wss://staging.fugle.tw/marketdata/v1.0/stock/streaming",
        );
    }

    #[test]
    fn test_custom_base_url_applied_to_futopt() {
        let factory = WebSocketFactory::new()
            .base_url("ws://localhost:8080/v1.0")
            .auth(AuthRequest::with_api_key("k"));
        let cfg = factory.futopt().build();
        assert_eq!(cfg.url, "ws://localhost:8080/v1.0/futopt/streaming");
    }

    #[test]
    fn test_base_url_strips_trailing_slashes() {
        let factory = WebSocketFactory::new()
            .base_url("wss://example.com/marketdata/v1.0///")
            .auth(AuthRequest::with_api_key("k"));
        let cfg = factory.stock().build();
        assert_eq!(
            cfg.url,
            "wss://example.com/marketdata/v1.0/stock/streaming",
        );
    }

    #[test]
    fn test_different_api_version_is_honored() {
        // The factory does NOT force `urls::API_VERSION` onto user-supplied
        // bases. Whatever segment the caller types is what ends up in the URL.
        let factory = WebSocketFactory::new()
            .base_url("wss://api.fugle.tw/marketdata/v2.0")
            .auth(AuthRequest::with_api_key("k"));
        let cfg = factory.stock().build();
        assert_eq!(
            cfg.url,
            "wss://api.fugle.tw/marketdata/v2.0/stock/streaming",
        );
    }

    #[test]
    fn test_legacy_host_root_caller_produces_non_canonical_url() {
        // 0.5.x callers passed a host root without /v1.0 and the SDK
        // injected the version. 0.6.0 stops doing that — the URL ends up
        // missing /v1.0 and the gateway responds 404. This test pins the
        // SILENT BREAKING semantic so MIGRATION-0.6.md stays honest.
        let factory = WebSocketFactory::new()
            .base_url("wss://api.fugle.tw/marketdata") // <-- missing /v1.0 on purpose
            .auth(AuthRequest::with_api_key("k"));
        let cfg = factory.stock().build();
        assert_eq!(
            cfg.url,
            "wss://api.fugle.tw/marketdata/stock/streaming",
            "Pinned non-canonical URL — confirms 0.5.x host-root callers \
             must be updated to include /v1.0 in their base_url string"
        );
    }

    #[test]
    fn test_factory_yields_independent_builders() {
        // Builders returned from .stock() / .futopt() must not share state
        // with the factory; chaining a setter on one MUST NOT affect the
        // other (otherwise a single factory could not produce multiple
        // distinct configurations).
        let factory = WebSocketFactory::new().auth(AuthRequest::with_api_key("k"));
        let a = factory.stock().message_buffer(2048).build();
        let b = factory.stock().message_buffer(4096).build();
        assert_eq!(a.message_buffer, 2048);
        assert_eq!(b.message_buffer, 4096);
    }

    #[test]
    fn test_chained_setters_compose_with_factory() {
        let factory = WebSocketFactory::new()
            .base_url("wss://staging.fugle.tw/marketdata/v1.0")
            .auth(AuthRequest::with_api_key("k"));
        let cfg = factory
            .stock()
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
