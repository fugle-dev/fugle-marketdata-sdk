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
//! // Custom base (staging / mock server). Both stock + futopt share the
//! // base; the factory derives /v1.0/{type}/streaming for each.
//! let factory = WebSocketFactory::new()
//!     .base_url("wss://staging.fugle.tw/marketdata")
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
    /// Override the WebSocket base URL (host root, no version segment).
    ///
    /// Available in any state. The factory appends
    /// `/{API_VERSION}/{type}/streaming` to every derived endpoint;
    /// trailing slashes on `base` are stripped.
    #[must_use]
    pub fn base_url(mut self, base: impl Into<String>) -> Self {
        self.base_url = Some(base.into());
        self
    }

    fn endpoint_for(&self, kind: &str) -> String {
        let base = self
            .base_url
            .as_deref()
            .unwrap_or(crate::urls::WS_BASE_ROOT);
        format!(
            "{}/{}/{}/streaming",
            base.trim_end_matches('/'),
            crate::urls::API_VERSION,
            kind,
        )
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
        let factory = WebSocketFactory::new()
            .base_url("wss://staging.fugle.tw/marketdata")
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
            .base_url("ws://localhost:8080")
            .auth(AuthRequest::with_api_key("k"));
        let cfg = factory.futopt().build();
        assert_eq!(cfg.url, "ws://localhost:8080/v1.0/futopt/streaming");
    }

    #[test]
    fn test_base_url_strips_trailing_slashes() {
        let factory = WebSocketFactory::new()
            .base_url("wss://example.com/marketdata///")
            .auth(AuthRequest::with_api_key("k"));
        let cfg = factory.stock().build();
        assert_eq!(
            cfg.url,
            "wss://example.com/marketdata/v1.0/stock/streaming",
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
            .base_url("wss://staging.fugle.tw/marketdata")
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
