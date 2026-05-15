//! Convenience factory mirroring the JS / Python SDK shape.
//!
//! Usage:
//!
//! ```rust
//! use marketdata_core::websocket::WebSocketFactory;
//! use marketdata_core::AuthRequest;
//!
//! // Production default endpoint:
//! let stock_cfg = WebSocketFactory::new(AuthRequest::with_api_key("k"))
//!     .stock()
//!     .build();
//!
//! // Custom base (staging / mock server). Both stock + futopt share the
//! // base; the factory derives /v1.0/{type}/streaming for each.
//! let factory = WebSocketFactory::new(AuthRequest::with_api_key("k"))
//!     .base_url("wss://staging.fugle.tw/marketdata");
//!
//! let stock_cfg = factory.stock().build();
//! let futopt_cfg = factory.futopt().build();
//! ```
//!
//! Mirrors `fugle-marketdata-node/src/websocket/factory.ts` and
//! `fugle-marketdata-python/fugle_marketdata/websocket/factory.py`.

use crate::models::AuthRequest;
use crate::websocket::config::{ConnectionConfig, ConnectionConfigBuilder};

/// Factory that derives stock + futopt WebSocket endpoint configurations
/// from a single authentication credential and an optional shared base URL.
#[derive(Clone)]
pub struct WebSocketFactory {
    auth: AuthRequest,
    base_url: Option<String>,
}

impl WebSocketFactory {
    /// Create a factory with the given auth credential and the production
    /// default WebSocket base URL ([`crate::urls::WS_BASE_ROOT`]).
    pub fn new(auth: AuthRequest) -> Self {
        Self {
            auth,
            base_url: None,
        }
    }

    /// Override the WebSocket base URL (host root, no version segment).
    ///
    /// The factory appends `/{API_VERSION}/{type}/streaming` to every
    /// derived endpoint. Trailing slashes on `base` are stripped.
    pub fn base_url(mut self, base: impl Into<String>) -> Self {
        self.base_url = Some(base.into());
        self
    }

    /// Derived stock-streaming endpoint as a [`ConnectionConfigBuilder`].
    /// Chain further setters (`.message_buffer(...)`, etc.)
    /// then `.build()` to obtain the [`ConnectionConfig`].
    pub fn stock(&self) -> ConnectionConfigBuilder {
        ConnectionConfig::builder(self.endpoint_for("stock"), self.auth.clone())
    }

    /// Derived futures/options streaming endpoint as a
    /// [`ConnectionConfigBuilder`].
    pub fn futopt(&self) -> ConnectionConfigBuilder {
        ConnectionConfig::builder(self.endpoint_for("futopt"), self.auth.clone())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::urls::{FUTOPT_WS, STOCK_WS};

    #[test]
    fn test_default_stock_endpoint() {
        let factory = WebSocketFactory::new(AuthRequest::with_api_key("k"));
        let cfg = factory.stock().build();
        assert_eq!(cfg.url, STOCK_WS);
    }

    #[test]
    fn test_default_futopt_endpoint() {
        let factory = WebSocketFactory::new(AuthRequest::with_api_key("k"));
        let cfg = factory.futopt().build();
        assert_eq!(cfg.url, FUTOPT_WS);
    }

    #[test]
    fn test_custom_base_url_applied_to_stock() {
        let factory = WebSocketFactory::new(AuthRequest::with_api_key("k"))
            .base_url("wss://staging.fugle.tw/marketdata");
        let cfg = factory.stock().build();
        assert_eq!(
            cfg.url,
            "wss://staging.fugle.tw/marketdata/v1.0/stock/streaming",
        );
    }

    #[test]
    fn test_custom_base_url_applied_to_futopt() {
        let factory = WebSocketFactory::new(AuthRequest::with_api_key("k"))
            .base_url("ws://localhost:8080");
        let cfg = factory.futopt().build();
        assert_eq!(cfg.url, "ws://localhost:8080/v1.0/futopt/streaming");
    }

    #[test]
    fn test_base_url_strips_trailing_slashes() {
        let factory = WebSocketFactory::new(AuthRequest::with_api_key("k"))
            .base_url("wss://example.com/marketdata///");
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
        let factory = WebSocketFactory::new(AuthRequest::with_api_key("k"));
        let a = factory.stock().message_buffer(2048).build();
        let b = factory.stock().message_buffer(4096).build();
        assert_eq!(a.message_buffer, 2048);
        assert_eq!(b.message_buffer, 4096);
    }

    #[test]
    fn test_chained_setters_compose_with_factory() {
        let factory = WebSocketFactory::new(AuthRequest::with_api_key("k"))
            .base_url("wss://staging.fugle.tw/marketdata");
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
}
