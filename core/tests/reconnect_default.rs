//! Pin the reconnect defaults every language shares (#149).
//!
//! Bindings no longer override the core default: a client created without a
//! reconnect config auto-reconnects in Rust, Python, Node.js, C#, Go, Java and
//! C++ alike, retrying without an attempt limit. `ReconnectionConfig::disabled()`
//! is how each of them turns it off.
//!
//! If a future core bump silently changes either default, this test fails.

use marketdata_core::websocket::ReconnectionManager;
use marketdata_core::{
    AuthRequest, ConnectionConfig, ReconnectionConfig, WebSocketClient, DEFAULT_MAX_ATTEMPTS,
};

#[test]
fn default_is_enabled() {
    assert!(
        ReconnectionConfig::default().enabled,
        "ReconnectionConfig::default() must have enabled = true: every binding \
         passes it through, so flipping it turns auto-reconnect off in every language."
    );
}

#[test]
fn default_max_attempts_is_unlimited() {
    assert_eq!(DEFAULT_MAX_ATTEMPTS, 0);
    let config = ReconnectionConfig::default();
    assert_eq!(config.max_attempts, 0);
    assert_eq!(ReconnectionManager::new(config).attempts_remaining(), None);
}

#[test]
fn explicit_disabled_constructor_still_works() {
    let config = ReconnectionConfig::disabled();
    assert!(
        !config.enabled,
        "ReconnectionConfig::disabled() is how every language turns auto-reconnect off"
    );
    let _client = WebSocketClient::with_reconnection_config(
        ConnectionConfig::fugle_stock(AuthRequest::with_api_key("test-key")),
        config,
    );
}
