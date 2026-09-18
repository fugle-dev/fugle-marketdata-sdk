//! WebSocket connection liveness detection — configuration.
//!
//! Two modes, chosen by [`HealthCheckConfig::probe_enabled`]:
//!
//! - **Passive** (default): if no inbound frame arrives within
//!   `heartbeat_timeout`, the connection is declared dead and the reconnect
//!   path takes over. The verdict is a guess — a server heartbeat that is
//!   merely late looks the same as a dead connection.
//! - **Probe**: once the connection has been silent for `idle_probe_after`,
//!   the SDK sends one application-level `{"event":"ping"}`; if nothing
//!   arrives within `probe_timeout` after that, the connection is declared
//!   dead. The verdict is confirmed. `heartbeat_timeout` does not apply.
//!
//! In both modes *any* inbound frame (heartbeat, data, pong) resets the
//! silence, so while market data flows no ping is ever sent. The timer is
//! enforced at the read site of each client (`aio::dispatch`,
//! `sync::owner_thread`) through the shared `liveness::Liveness` state
//! machine — no background polling task.
//!
//! The defaults make "just turn `probe_enabled` on" free: `idle_probe_after`
//! (30s) matches the server's 30-second heartbeat, so a punctual heartbeat
//! keeps the silence below it and no ping is sent. Only a *late* heartbeat —
//! the case that causes false disconnects in passive mode — triggers one
//! ping, and detection stays at 30s + 5s = 35s, the passive default.
//!
//! Probe mode does **not** cover a half-open connection where the server can
//! still send but our writes no longer reach it: the server's broadcast
//! heartbeat keeps arriving and resetting the silence, so the probe never
//! fires.
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
//! | `healthCheck.interval` (ping cadence) | no fixed cadence: with [`HealthCheckConfig::probe_enabled`], one ping after [`HealthCheckConfig::idle_probe_after`] of silence |
//! | `healthCheck.maxMissedPongs` | no equivalent — nothing is counted |
//! | `interval × maxMissedPongs` (effective deadline) | [`HealthCheckConfig::heartbeat_timeout`], or `idle_probe_after + probe_timeout` with probing |
//! | `disconnect` event with `{ reason: 'health-check-timeout' }` | [`ConnectionEvent::HeartbeatTimeout`](crate::websocket::ConnectionEvent::HeartbeatTimeout), then `Disconnected { intent: Network }` |
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

/// Default silence before a probe is sent (probe mode): the Fugle server's
/// 30s heartbeat period (`@Cron('*/30 * * * * *')`). A punctual heartbeat
/// keeps the silence below it, so with this default a ping is sent only when
/// the heartbeat is late — and detection stays at 30s + 5s, the same as the
/// passive default.
pub const DEFAULT_IDLE_PROBE_AFTER_MS: u64 = 30_000;

/// Floor for [`HealthCheckConfig::idle_probe_after`]. Each connection sends
/// a ping every `idle_probe_after` of silence, so this bounds the load a
/// client can put on the server during quiet periods.
pub const MIN_IDLE_PROBE_AFTER_MS: u64 = 5_000;

/// Default wait for any inbound frame after a probe is sent (probe mode).
pub const DEFAULT_PROBE_TIMEOUT_MS: u64 = 5_000;

/// Floor for [`HealthCheckConfig::probe_timeout`].
pub const MIN_PROBE_TIMEOUT_MS: u64 = 1_000;

/// Configuration for WebSocket connection liveness detection.
///
/// When the connection is declared dead the dispatch path emits
/// [`ConnectionEvent::HeartbeatTimeout`](crate::websocket::ConnectionEvent::HeartbeatTimeout)
/// followed by `Disconnected { intent: Network }` and exits, which lets the
/// reconnect manager take over. See the [module docs](self) for the two
/// modes and what probe mode does not cover.
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
    /// **Passive mode only: does not apply when `probe_enabled` is true**,
    /// where detection is `idle_probe_after + probe_timeout` instead.
    ///
    /// Default: 35s (the Fugle server emits a heartbeat every 30s;
    /// 5s buffer absorbs network jitter). Use [`HealthCheckConfig::with_timeout`]
    /// to construct with validation.
    pub heartbeat_timeout: Duration,

    /// Confirm a silent connection with a ping instead of declaring it dead
    /// on `heartbeat_timeout`. Default: `false`.
    ///
    /// When true, `heartbeat_timeout` does not apply: after
    /// `idle_probe_after` of silence one `{"event":"ping"}` is sent, and the
    /// connection is declared dead if nothing arrives within
    /// `probe_timeout`. Detection time is `idle_probe_after + probe_timeout`
    /// (35s with the defaults, the same as passive mode). A probe that
    /// cannot be written within `probe_timeout` counts as no answer.
    pub probe_enabled: bool,

    /// Silence before a probe is sent (probe mode only). `None` means
    /// [`DEFAULT_IDLE_PROBE_AFTER_MS`] (30s, the server's heartbeat period);
    /// floor [`MIN_IDLE_PROBE_AFTER_MS`].
    ///
    /// Below 30s a ping is sent in every gap between server heartbeats
    /// while no market data flows — one per `idle_probe_after`, per
    /// connection.
    pub idle_probe_after: Option<Duration>,

    /// Wait for any inbound frame after a probe is sent (probe mode only).
    /// `None` means [`DEFAULT_PROBE_TIMEOUT_MS`] (5s); floor
    /// [`MIN_PROBE_TIMEOUT_MS`].
    pub probe_timeout: Option<Duration>,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            enabled: DEFAULT_HEALTH_CHECK_ENABLED,
            heartbeat_timeout: Duration::from_millis(DEFAULT_HEARTBEAT_TIMEOUT_MS),
            probe_enabled: false,
            idle_probe_after: None,
            probe_timeout: None,
        }
    }
}

/// `ConfigError` unless `value` is absent or at least `min_ms`.
fn check_floor(name: &str, value: Option<Duration>, min_ms: u64) -> Result<(), MarketDataError> {
    match value {
        Some(value) if value < Duration::from_millis(min_ms) => Err(MarketDataError::ConfigError(
            format!("{name} must be >= {min_ms}ms (got {value:?})"),
        )),
        _ => Ok(()),
    }
}

impl HealthCheckConfig {
    /// Build a config from every setting at once, validating each against
    /// its floor. `None` means that setting's default. This is the
    /// constructor the language bindings convert into.
    ///
    /// # Errors
    /// Returns [`MarketDataError::ConfigError`] if a given value is below its
    /// floor ([`MIN_HEARTBEAT_TIMEOUT_MS`], [`MIN_IDLE_PROBE_AFTER_MS`],
    /// [`MIN_PROBE_TIMEOUT_MS`]). Values are checked even for a mode that
    /// is not in use, so a bad setting surfaces before it is switched on.
    pub fn from_parts(
        enabled: bool,
        heartbeat_timeout: Option<Duration>,
        probe_enabled: bool,
        idle_probe_after: Option<Duration>,
        probe_timeout: Option<Duration>,
    ) -> Result<Self, MarketDataError> {
        check_floor("heartbeat_timeout", heartbeat_timeout, MIN_HEARTBEAT_TIMEOUT_MS)?;
        check_floor("idle_probe_after", idle_probe_after, MIN_IDLE_PROBE_AFTER_MS)?;
        check_floor("probe_timeout", probe_timeout, MIN_PROBE_TIMEOUT_MS)?;
        Ok(Self {
            enabled,
            heartbeat_timeout: heartbeat_timeout
                .unwrap_or(Duration::from_millis(DEFAULT_HEARTBEAT_TIMEOUT_MS)),
            probe_enabled,
            idle_probe_after,
            probe_timeout,
        })
    }

    /// Construct an enabled config in probe mode (see
    /// [`probe_enabled`](Self::probe_enabled)): a ping after `idle_probe_after`
    /// of silence, dead if nothing arrives within `probe_timeout` of it.
    ///
    /// # Errors
    /// Returns [`MarketDataError::ConfigError`] if `idle_probe_after` is below
    /// [`MIN_IDLE_PROBE_AFTER_MS`] or `probe_timeout` below
    /// [`MIN_PROBE_TIMEOUT_MS`].
    pub fn with_probe(
        idle_probe_after: Duration,
        probe_timeout: Duration,
    ) -> Result<Self, MarketDataError> {
        Self::from_parts(true, None, true, Some(idle_probe_after), Some(probe_timeout))
    }

    /// The silence before a probe is sent, defaulted.
    pub fn idle_probe_after_or_default(&self) -> Duration {
        self.idle_probe_after
            .unwrap_or(Duration::from_millis(DEFAULT_IDLE_PROBE_AFTER_MS))
    }

    /// The wait after a probe is sent, defaulted.
    pub fn probe_timeout_or_default(&self) -> Duration {
        self.probe_timeout
            .unwrap_or(Duration::from_millis(DEFAULT_PROBE_TIMEOUT_MS))
    }

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
        Self::from_parts(true, Some(timeout), false, None, None)
    }

    /// Construct a disabled config. Without liveness detection a
    /// stalled connection won't surface until the OS times out the
    /// underlying TCP — typically hours on Linux defaults.
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Self::default()
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

    #[test]
    fn test_probe_defaults() {
        let config = HealthCheckConfig::default();
        assert!(!config.probe_enabled, "probe mode is opt-in");
        assert_eq!(config.idle_probe_after_or_default(), Duration::from_secs(30));
        assert_eq!(config.probe_timeout_or_default(), Duration::from_secs(5));
    }

    #[test]
    fn test_default_probe_detection_matches_passive_default() {
        // Turning probe mode on alone must not slow detection down.
        let config = HealthCheckConfig::default();
        assert_eq!(
            config.idle_probe_after_or_default() + config.probe_timeout_or_default(),
            config.heartbeat_timeout,
        );
    }

    #[test]
    fn test_with_probe() {
        let config =
            HealthCheckConfig::with_probe(Duration::from_secs(5), Duration::from_secs(5)).unwrap();
        assert!(config.enabled && config.probe_enabled);
        assert_eq!(config.idle_probe_after, Some(Duration::from_secs(5)));
        assert_eq!(config.probe_timeout, Some(Duration::from_secs(5)));
    }

    #[test]
    fn test_probe_floors() {
        assert!(HealthCheckConfig::with_probe(Duration::from_millis(5_000), Duration::from_millis(1_000)).is_ok());
        let idle = HealthCheckConfig::with_probe(Duration::from_millis(4_999), Duration::from_secs(5));
        assert!(matches!(idle, Err(MarketDataError::ConfigError(_))));
        let timeout = HealthCheckConfig::with_probe(Duration::from_secs(5), Duration::from_millis(999));
        assert!(matches!(timeout, Err(MarketDataError::ConfigError(_))));
    }

    #[test]
    fn test_from_parts_validates_unused_mode() {
        // A bad probe setting is rejected even with probe mode off.
        let result = HealthCheckConfig::from_parts(
            true,
            None,
            false,
            None,
            Some(Duration::from_millis(10)),
        );
        assert!(result.is_err());
    }
}
