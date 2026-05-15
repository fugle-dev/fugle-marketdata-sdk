//! Documents the API contract that downstream language bindings rely
//! on to compensate for the 0.4 reconnect-default flip.
//!
//! - `ReconnectionConfig::default().enabled` MUST stay `true` so Rust
//!   callers on the happy path get auto-reconnect.
//! - `ReconnectionConfig::disabled().enabled` MUST stay `false` so
//!   each binding wrapper can call it at its FFI boundary to preserve
//!   the historical "no auto-reconnect" semantic exposed to end users.
//! - `WebSocketClient::with_reconnection_config(_, disabled())` MUST
//!   accept the disabled config without panicking — this is the call
//!   shape every binding takes when no reconnect record was supplied.
//!
//! See `MIGRATION-0.4.md` section "Bindings" for the migration rationale.

use marketdata_core::{
    AuthRequest, ConnectionConfig, ReconnectionConfig, WebSocketClient,
};

#[test]
fn rust_core_default_stays_enabled() {
    assert!(
        ReconnectionConfig::default().enabled,
        "Rust default flipped — bindings rely on this being true \
         while the binding wrapper explicitly downgrades to disabled()"
    );
}

#[test]
fn binding_compensation_call_shape_compiles() {
    // This test demonstrates the call shape every binding wrapper
    // takes when the user did not opt in to auto-reconnect:
    //   1. Build a `ConnectionConfig` from user-supplied auth.
    //   2. Pass `ReconnectionConfig::disabled()` so end users keep
    //      the historical "no auto-reconnect" behaviour.
    //
    // Compile is the assertion — if either function signature changes
    // in a breaking way, every binding wrapper breaks the same day.
    let auth = AuthRequest::with_api_key("test-key");
    let config = ConnectionConfig::fugle_stock(auth);
    let _client = WebSocketClient::with_reconnection_config(
        config,
        ReconnectionConfig::disabled(),
    );
}

#[test]
fn disabled_constructor_yields_enabled_false() {
    let cfg = ReconnectionConfig::disabled();
    assert!(
        !cfg.enabled,
        "disabled() must produce enabled=false; binding wrappers \
         depend on this to short-circuit reconnect at the FFI boundary"
    );
}
