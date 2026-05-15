//! Pin the 0.4.0 reconnect-default flip.
//!
//! These assertions are the workspace-level CI gate for risk **R8** in
//! `openspec/changes/sdk-04-improvements/design.md`: every binding wrapper
//! that constructs `ReconnectionConfig` MUST explicitly call
//! `ReconnectionConfig::disabled()` so its end users observe no behaviour
//! change. The Rust core, on the other hand, defaults to `enabled = true`.
//!
//! If a future core bump silently flips this back, this test fails.

use marketdata_core::ReconnectionConfig;

#[test]
fn rust_core_default_is_enabled() {
    let config = ReconnectionConfig::default();
    assert!(
        config.enabled,
        "core::ReconnectionConfig::default() must have enabled = true (0.4.0+). \
         If this assertion is flipped back to false, every binding's effective \
         default also flips and end users start seeing auto-reconnect they did \
         not opt into."
    );
}

#[test]
fn explicit_disabled_constructor_still_works() {
    let config = ReconnectionConfig::disabled();
    assert!(
        !config.enabled,
        "ReconnectionConfig::disabled() is the binding-side compensation API: \
         flipping this would break every Python / Node / UniFFI wrapper that \
         relies on it to preserve historical 'no auto-reconnect' behaviour."
    );
}

#[test]
fn default_max_attempts_unchanged() {
    // Sanity: the flip changed only `enabled`, not the other fields.
    let config = ReconnectionConfig::default();
    assert_eq!(config.max_attempts, 5);
}
