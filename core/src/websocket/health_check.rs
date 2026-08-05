//! WebSocket connection liveness detection — configuration only.
//!
//! In 3.0 the SDK uses a single timeout window: if no inbound frame
//! arrives within `heartbeat_timeout`, the connection is declared dead
//! and the reconnect path takes over. The actual timeout enforcement
//! lives at the read site in `crate::websocket::message::dispatch_messages`,
//! wrapped via `tokio::time::timeout(heartbeat_timeout, ws_read.next())`.
//! No background polling task, no atomic activity timestamps — just
//! plain async-native pre-emption.
//!
//! # Mapping from the official SDKs
//!
//! The official Node / Python SDKs poll on a timer and count missed pongs.
//! Their 1.5.0 / 2.5.0 release reworked that into a freshness check ("did
//! anything arrive since our last ping?") and added a disconnect reason —
//! converging on what this module has done since 0.3.0. The knobs do not
//! correspond one-to-one, so if you are porting configuration across:
//!
//! | Official option / behaviour | Here |
//! |---|---|
//! | `healthCheck.enabled` | [`HealthCheckConfig::enabled`] |
//! | `healthCheck.interval` (ping cadence) | no equivalent — nothing is sent |
//! | `healthCheck.maxMissedPongs` | no equivalent — nothing is counted |
//! | `interval × maxMissedPongs` (effective deadline) | [`HealthCheckConfig::heartbeat_timeout`] |
//! | `disconnect` event with `{ reason: 'health-check-timeout' }` | [`ConnectionEvent::HeartbeatTimeout`](crate::websocket::ConnectionEvent::HeartbeatTimeout) |
//!
//! The `maxMissedPongs`-of-0 bug the official SDKs clamped in 1.5.0 (a zero
//! would disconnect a healthy connection on the first tick) cannot occur
//! here: there is no counter to zero out, only a timeout with a floor of
//! [`MIN_HEARTBEAT_TIMEOUT_MS`].

use crate::MarketDataError;
use std::time::Duration;

/// Liveness detection enabled by default in 3.0 (was opt-in in 2.x).
/// Silent-by-default lets a stalled connection sit unnoticed until the
/// OS eventually times out the underlying TCP — typically hours.
pub const DEFAULT_HEALTH_CHECK_ENABLED: bool = true;

/// Default heartbeat timeout: Fugle server's 30s heartbeat period plus
/// 5s buffer to absorb network jitter. Mirrors Databento's
/// `heartbeat_interval + 5` convention.
pub const DEFAULT_HEARTBEAT_TIMEOUT_MS: u64 = 35_000;

/// Absolute floor for [`HealthCheckConfig::heartbeat_timeout`]. This is
/// a sanity floor, **not** a "safe value" — values below the actual
/// server heartbeat period (currently 30s) will cause repeated false
/// disconnects. Settings under 35s only make sense in tests, or once
/// the server supports negotiated heartbeat interval (Phase 2.3 in the
/// SDK roadmap; see `WEBSOCKET-SERVER-RECOMMENDATIONS.md`).
pub const MIN_HEARTBEAT_TIMEOUT_MS: u64 = 5_000;

/// Configuration for WebSocket connection liveness detection.
///
/// A single timeout window controls when the SDK declares the
/// connection dead: if no inbound frame (heartbeat, data, anything)
/// arrives within `heartbeat_timeout`, the dispatch path emits
/// [`ConnectionEvent::HeartbeatTimeout`](crate::websocket::ConnectionEvent::HeartbeatTimeout)
/// and exits, which lets the reconnect manager take over.
#[derive(Debug, Clone)]
pub struct HealthCheckConfig {
    /// Whether liveness detection is active. Default: `true` (changed
    /// from `false` in 2.x). Use [`HealthCheckConfig::disabled`] to
    /// opt out — discouraged outside test environments because a
    /// silent connection won't surface until the OS times out the
    /// underlying TCP, typically hours later.
    pub enabled: bool,

    /// Maximum allowed gap between inbound frames before declaring
    /// the connection dead.
    ///
    /// Default: 35s (the Fugle server emits a heartbeat every 30s;
    /// 5s buffer absorbs network jitter). Use [`HealthCheckConfig::with_timeout`]
    /// to construct with validation.
    pub heartbeat_timeout: Duration,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            enabled: DEFAULT_HEALTH_CHECK_ENABLED,
            heartbeat_timeout: Duration::from_millis(DEFAULT_HEARTBEAT_TIMEOUT_MS),
        }
    }
}

impl HealthCheckConfig {
    /// Construct an enabled config with the given timeout.
    ///
    /// Returns [`MarketDataError::ConfigError`] if `timeout` is below
    /// the absolute sanity floor ([`MIN_HEARTBEAT_TIMEOUT_MS`]).
    /// Note: this only enforces a floor, not a value that's actually
    /// safe against the live server's heartbeat period. See the
    /// constant's docs.
    ///
    /// # Errors
    /// Returns [`MarketDataError`] on transport, protocol, deserialization,
    /// validation, or peer-initiated failures.
    pub fn with_timeout(timeout: Duration) -> Result<Self, MarketDataError> {
        if timeout < Duration::from_millis(MIN_HEARTBEAT_TIMEOUT_MS) {
            return Err(MarketDataError::ConfigError(format!(
                "heartbeat_timeout must be >= {}ms (got {:?})",
                MIN_HEARTBEAT_TIMEOUT_MS, timeout
            )));
        }
        Ok(Self {
            enabled: true,
            heartbeat_timeout: timeout,
        })
    }

    /// Construct a disabled config. Without liveness detection a
    /// stalled connection won't surface until the OS times out the
    /// underlying TCP — typically hours on Linux defaults.
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            heartbeat_timeout: Duration::from_millis(DEFAULT_HEARTBEAT_TIMEOUT_MS),
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = HealthCheckConfig::default();
        assert!(config.enabled, "3.0 default is enabled=true");
        assert_eq!(config.heartbeat_timeout, Duration::from_secs(35));
    }

    #[test]
    fn test_default_config_timeout_is_35s() {
        let config = HealthCheckConfig::default();
        assert_eq!(config.heartbeat_timeout, Duration::from_secs(35));
    }

    #[test]
    fn test_disabled_factory() {
        let config = HealthCheckConfig::disabled();
        assert!(!config.enabled);
        // heartbeat_timeout is still set (to default) but unused when disabled.
        assert_eq!(config.heartbeat_timeout, Duration::from_secs(35));
    }

    #[test]
    fn test_with_timeout_accepts_60s() {
        let config = HealthCheckConfig::with_timeout(Duration::from_secs(60)).unwrap();
        assert!(config.enabled);
        assert_eq!(config.heartbeat_timeout, Duration::from_secs(60));
    }

    #[test]
    fn test_with_timeout_accepts_5s_minimum() {
        let result = HealthCheckConfig::with_timeout(Duration::from_millis(5000));
        assert!(result.is_ok(), "5s is at the floor and must be accepted");
    }

    #[test]
    fn test_with_timeout_rejects_below_5s() {
        let result = HealthCheckConfig::with_timeout(Duration::from_millis(4_999));
        assert!(result.is_err(), "below 5s floor must be rejected");
    }

}
