//! WebSocket reconnection logic with exponential backoff

use std::time::Duration;

use crate::MarketDataError;

/// Default maximum reconnection attempts: `0`, meaning unlimited (#149).
///
/// With no attempt limit the client keeps retrying and [`DEFAULT_MAX_DELAY_MS`]
/// caps each wait, so a long outage is retried about once a minute instead of
/// being given up on.
pub const DEFAULT_MAX_ATTEMPTS: u32 = 0;

/// Default initial reconnection delay in milliseconds (VAL-02)
pub const DEFAULT_INITIAL_DELAY_MS: u64 = 1000;

/// Default maximum reconnection delay in milliseconds (VAL-02)
pub const DEFAULT_MAX_DELAY_MS: u64 = 60000;

/// Minimum allowed initial delay to prevent connection storms
pub const MIN_INITIAL_DELAY_MS: u64 = 100;

/// Reconnection configuration
///
/// Controls automatic reconnection behavior after connection drops.
///
/// Construct via the derived [`ReconnectionConfig::builder`] (no validation,
/// fields default to the [`DEFAULT_*`](DEFAULT_MAX_ATTEMPTS) constants) or
/// via the validating positional constructor [`ReconnectionConfig::new`].
///
/// `enabled` defaults to **`true`** and `max_attempts` to `0` (unlimited), in
/// every language: bindings pass the core default through when the caller
/// configures nothing (#149). Use [`ReconnectionConfig::disabled`] to turn
/// auto-reconnect off.
#[derive(Debug, Clone, bon::Builder)]
pub struct ReconnectionConfig {
    /// Whether auto-reconnect is active. When `false`, [`ReconnectionManager::should_reconnect`]
    /// always returns `false` regardless of how the connection ended.
    #[builder(default = true)]
    pub enabled: bool,
    /// Maximum reconnection attempts before giving up; `0` means unlimited
    /// (the default). `ReconnectFailed` is only emitted when this is non-zero.
    #[builder(default = DEFAULT_MAX_ATTEMPTS)]
    pub max_attempts: u32,
    /// Initial delay before first reconnection attempt
    #[builder(default = Duration::from_millis(DEFAULT_INITIAL_DELAY_MS))]
    pub initial_delay: Duration,
    /// Maximum delay between reconnection attempts
    #[builder(default = Duration::from_millis(DEFAULT_MAX_DELAY_MS))]
    pub max_delay: Duration,
}

impl Default for ReconnectionConfig {
    fn default() -> Self {
        Self::builder().build()
    }
}

impl ReconnectionConfig {
    /// Create a new reconnection config with validation
    ///
    /// # Errors
    /// Returns `MarketDataError::ConfigError` if:
    /// - `initial_delay` is less than 100ms
    /// - `max_delay` is less than `initial_delay`
    ///
    /// `max_attempts == 0` means unlimited attempts.
    ///
    /// The returned config has `enabled: true`. To get a disabled config use
    /// [`ReconnectionConfig::disabled`].
    pub fn new(
        max_attempts: u32,
        initial_delay: Duration,
        max_delay: Duration,
    ) -> Result<Self, MarketDataError> {
        if initial_delay < Duration::from_millis(MIN_INITIAL_DELAY_MS) {
            return Err(MarketDataError::ConfigError(format!(
                "initial_delay must be >= {}ms (got {}ms)",
                MIN_INITIAL_DELAY_MS,
                initial_delay.as_millis()
            )));
        }

        if max_delay < initial_delay {
            return Err(MarketDataError::ConfigError(format!(
                "max_delay ({}ms) must be >= initial_delay ({}ms)",
                max_delay.as_millis(),
                initial_delay.as_millis()
            )));
        }

        Ok(Self {
            enabled: true,
            max_attempts,
            initial_delay,
            max_delay,
        })
    }

    /// Build an explicitly disabled reconnection config.
    ///
    /// `should_reconnect()` will always return `false` regardless of how the
    /// connection ended.
    ///
    /// # Stability
    ///
    /// **Stable public API.** This is how every language turns
    /// auto-reconnect off — see the `tests/reconnect_default.rs` gate. The
    /// function's name and signature will be preserved across every 0.x
    /// release; downstream code can rely on it.
    #[must_use]
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Self::default()
        }
    }

}

/// Manages reconnection attempts with exponential backoff
///
/// Tracks reconnection state and determines:
/// - Whether a lost connection is retried (see [`Self::should_reconnect`])
/// - Delay before next reconnection attempt
/// - When max attempts have been reached
pub struct ReconnectionManager {
    config: ReconnectionConfig,
    current_attempt: u32,
}

impl ReconnectionManager {
    /// Create a new reconnection manager
    pub fn new(config: ReconnectionConfig) -> Self {
        Self {
            config,
            current_attempt: 0,
        }
    }

    /// Whether to reconnect after a connection ended with `close_code` (the
    /// peer's Close frame code, `None` for a Close without one, a dropped
    /// transport or a heartbeat timeout) and `last_error_code`, the `code`
    /// of the last `error` frame the connection delivered, if any.
    ///
    /// **Not reconnecting is the enumerated case; everything else
    /// reconnects** (#201). The set is what the server
    /// (`fugle-realtime/apps/streamer`) actually expresses as "do not come
    /// back":
    ///
    /// | Condition | Reconnect |
    /// |---|---|
    /// | [`ReconnectionConfig::enabled`] is `false` | no |
    /// | close `1000` (normal closure) | no |
    /// | `last_error_code == 1000`: the server rejected the credentials, then closed without a code | no |
    /// | anything else | yes |
    ///
    /// Close codes the server sends: `1001` (maintenance restart,
    /// connection limit, no auth request within 60 s) and `1008` (too many
    /// auth messages on one connection; unreachable from this SDK, which
    /// authenticates once per connection — revisit if it ever re-auths).
    /// Both reconnect, as do `1006`, an absent code and any unknown code;
    /// the server never sends 4xxx. Credentials rejected is the one case
    /// where retrying cannot help: the server says so with `error{1000}`
    /// followed by a Close with no code, hence the second parameter.
    ///
    /// Adding an exception is one row here and one in
    /// `will_reconnect_after_matrix` (`connection_event.rs`). Shared by the
    /// sync and async clients, through
    /// `connection_event::will_reconnect_after`.
    pub fn should_reconnect(&self, close_code: Option<u16>, last_error_code: Option<i32>) -> bool {
        if !self.config.enabled {
            return false;
        }
        if close_code == Some(1000) {
            return false;
        }
        if last_error_code == Some(crate::websocket::protocol::AUTH_REJECTED_CODE) {
            return false;
        }
        true
    }

    /// Calculate next reconnection delay with exponential backoff and jitter
    ///
    /// Attempt `n` waits `base × (1 + U[0, 0.5))`, where
    /// `base = min(initial_delay × 2^(n-1), max_delay)`, and never longer
    /// than `max_delay`. The jitter is random per call (#227), so clients
    /// dropped at the same moment do not all come back at the same moment;
    /// one client's delays still increase, since the jitter is under 100%.
    ///
    /// Returns None if max attempts reached, Some(duration) otherwise; with
    /// `max_attempts == 0` (unlimited) it always returns Some.
    /// Increments attempt counter.
    pub fn next_delay(&mut self) -> Option<Duration> {
        if self.config.max_attempts != 0 && self.current_attempt >= self.config.max_attempts {
            return None;
        }

        self.current_attempt = self.current_attempt.saturating_add(1);

        let base = self.base_delay(self.current_attempt);
        let delay = base.saturating_add(crate::jitter::jitter(base / 2));
        Some(delay.min(self.config.max_delay))
    }

    /// Backoff before jitter for `attempt` (1-indexed):
    /// `initial_delay × 2^(attempt-1)`, capped at `max_delay`.
    fn base_delay(&self, attempt: u32) -> Duration {
        // 2^10 is past any sane cap and keeps the multiplication in range.
        let exponent = attempt.saturating_sub(1).min(10);
        self.config
            .initial_delay
            .saturating_mul(1 << exponent)
            .min(self.config.max_delay)
    }

    /// Reset reconnection state
    ///
    /// Clears attempt counter, allowing fresh reconnection.
    /// Used after successful reconnection or manual reconnect() call.
    pub fn reset(&mut self) {
        self.current_attempt = 0;
    }

    /// Get number of remaining reconnection attempts; `None` when attempts
    /// are unlimited (`max_attempts == 0`).
    pub fn attempts_remaining(&self) -> Option<u32> {
        match self.config.max_attempts {
            0 => None,
            max => Some(max.saturating_sub(self.current_attempt)),
        }
    }

    /// Get current attempt number
    pub fn current_attempt(&self) -> u32 {
        self.current_attempt
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: build an explicitly enabled config for the close-code tests.
    fn enabled_config() -> ReconnectionConfig {
        ReconnectionConfig::new(5, Duration::from_secs(1), Duration::from_secs(60))
            .expect("test config is valid")
    }

    #[test]
    fn test_reconnection_config_default() {
        let config = ReconnectionConfig::default();
        assert!(
            config.enabled,
            "auto-reconnect is on by default in every language (#149)"
        );
        assert_eq!(config.max_attempts, 0, "unlimited attempts by default (#149)");
        assert_eq!(config.initial_delay, Duration::from_secs(1));
        assert_eq!(config.max_delay, Duration::from_secs(60));
    }

    #[test]
    fn test_reconnection_config_new_is_enabled() {
        // Explicit construction means the caller wants reconnect on
        let config = enabled_config();
        assert!(config.enabled);
    }

    #[test]
    fn test_reconnection_config_disabled_constructor() {
        let config = ReconnectionConfig::disabled();
        assert!(!config.enabled);
    }

    #[test]
    fn test_disabled_config_never_reconnects() {
        // `ReconnectionConfig::disabled()` short-circuits `should_reconnect`
        // even on codes the close-code logic considers retriable
        // (1006, 1001, …). It is how every language turns reconnect off.
        let manager = ReconnectionManager::new(ReconnectionConfig::disabled());
        assert!(!manager.should_reconnect(Some(1006), None));
        assert!(!manager.should_reconnect(Some(1001), None));
        assert!(!manager.should_reconnect(None, None));
    }

    #[test]
    fn test_reconnection_config_builder() {
        let config = ReconnectionConfig::builder()
            .max_attempts(10)
            .initial_delay(Duration::from_secs(2))
            .max_delay(Duration::from_secs(120))
            .build();

        assert_eq!(config.max_attempts, 10);
        assert_eq!(config.initial_delay, Duration::from_secs(2));
        assert_eq!(config.max_delay, Duration::from_secs(120));
        assert!(
            config.enabled,
            "builder defaults `enabled` to true, matching Default"
        );
    }

    #[test]
    fn test_reconnection_config_builder_defaults_match_default() {
        let via_builder = ReconnectionConfig::builder().build();
        let via_default = ReconnectionConfig::default();
        assert_eq!(via_builder.enabled, via_default.enabled);
        assert_eq!(via_builder.max_attempts, via_default.max_attempts);
        assert_eq!(via_builder.initial_delay, via_default.initial_delay);
        assert_eq!(via_builder.max_delay, via_default.max_delay);
    }

    /// The full decision table (#201). Not reconnecting is the enumerated
    /// case; every other row reconnects. Mirrored, with the intent and
    /// shutdown inputs, by `connection_event::will_reconnect_after_matrix`.
    #[test]
    fn test_should_reconnect_matrix() {
        let manager = ReconnectionManager::new(enabled_config());

        // Normal closure is final.
        assert!(!manager.should_reconnect(Some(1000), None));
        assert!(!manager.should_reconnect(Some(1000), Some(1003)));
        // Credentials rejected: `error{1000}`, then a Close without a code
        // (what the server does), or with one, or the transport dropped.
        assert!(!manager.should_reconnect(None, Some(1000)));
        assert!(!manager.should_reconnect(Some(1001), Some(1000)));
        assert!(!manager.should_reconnect(Some(1006), Some(1000)));

        // Codes the server sends.
        assert!(manager.should_reconnect(Some(1001), None)); // going away
        assert!(manager.should_reconnect(Some(1008), None)); // policy violation
        // Codes the transport produces.
        assert!(manager.should_reconnect(Some(1006), None)); // abnormal closure
        assert!(manager.should_reconnect(None, None)); // no Close frame / no code
        // Unknown codes reconnect by default, including 4xxx: the server
        // never sends them, and a code the SDK does not know is not a
        // reason to give up.
        assert!(manager.should_reconnect(Some(1002), None));
        assert!(manager.should_reconnect(Some(1003), None));
        assert!(manager.should_reconnect(Some(4000), None));
        assert!(manager.should_reconnect(Some(4001), None));
        assert!(manager.should_reconnect(Some(4999), None));
        // Any other last error is not a verdict on the credentials: `1004`
        // (no auth request seen) precedes the server's 1001, `1011` (auth
        // service down) is transient, `1003` is a bad request.
        assert!(manager.should_reconnect(None, Some(1004)));
        assert!(manager.should_reconnect(Some(1001), Some(1004)));
        assert!(manager.should_reconnect(None, Some(1011)));
        assert!(manager.should_reconnect(None, Some(1003)));
    }

    #[test]
    fn test_exponential_backoff_delays() {
        let config = ReconnectionConfig::builder().max_attempts(5).build();
        let mut manager = ReconnectionManager::new(config);

        // First delay should be returned
        let delay1 = manager.next_delay();
        assert!(delay1.is_some());
        assert_eq!(manager.current_attempt(), 1);

        // Delays should increase (exponential backoff)
        let delay2 = manager.next_delay();
        assert!(delay2.is_some());
        assert_eq!(manager.current_attempt(), 2);

        // Continue getting delays up to max_attempts
        let _ = manager.next_delay();
        let _ = manager.next_delay();
        let _ = manager.next_delay();

        // After max_attempts (5), should return None
        let delay_final = manager.next_delay();
        assert!(delay_final.is_none());
    }

    #[test]
    fn test_reset_clears_attempts() {
        let config = ReconnectionConfig::builder().max_attempts(5).build();
        let mut manager = ReconnectionManager::new(config);

        // Exhaust attempts
        let _ = manager.next_delay();
        let _ = manager.next_delay();
        assert_eq!(manager.current_attempt(), 2);

        // Reset should clear attempts
        manager.reset();
        assert_eq!(manager.current_attempt(), 0);
        assert_eq!(manager.attempts_remaining(), Some(5));

        // Should be able to get delays again
        let delay = manager.next_delay();
        assert!(delay.is_some());
    }

    #[test]
    fn test_max_attempts_reached() {
        let config = ReconnectionConfig::builder().max_attempts(3).build();
        let mut manager = ReconnectionManager::new(config);

        // Get 3 delays
        assert!(manager.next_delay().is_some());
        assert!(manager.next_delay().is_some());
        assert!(manager.next_delay().is_some());

        // 4th attempt should return None
        assert!(manager.next_delay().is_none());
        assert_eq!(manager.attempts_remaining(), Some(0));
    }

    #[test]
    fn test_attempts_remaining() {
        let config = ReconnectionConfig::builder().max_attempts(5).build();
        let mut manager = ReconnectionManager::new(config);

        assert_eq!(manager.attempts_remaining(), Some(5));

        let _ = manager.next_delay();
        assert_eq!(manager.attempts_remaining(), Some(4));

        let _ = manager.next_delay();
        assert_eq!(manager.attempts_remaining(), Some(3));
    }

    #[test]
    fn test_unlimited_attempts_never_give_up() {
        let mut manager = ReconnectionManager::new(ReconnectionConfig::default());
        assert_eq!(manager.attempts_remaining(), None);

        let max_delay = Duration::from_millis(DEFAULT_MAX_DELAY_MS);
        let mut last = Duration::ZERO;
        for _ in 0..100 {
            last = manager.next_delay().expect("unlimited attempts never run out");
            // max_delay is a hard cap, jitter included.
            assert!(last <= max_delay, "{last:?}");
        }
        assert_eq!(manager.current_attempt(), 100);
        assert_eq!(manager.attempts_remaining(), None);
        assert_eq!(last, max_delay, "backoff reaches the cap");
    }

    /// Clients dropped at the same moment do not share a first delay (#227).
    /// The jitter is random, so only require that the 100 values are not all
    /// equal; that cannot flake.
    #[test]
    fn test_jitter_spreads_first_delay_across_clients() {
        let config = ReconnectionConfig::default();
        let first: std::collections::HashSet<Duration> = (0..100)
            .map(|_| {
                ReconnectionManager::new(config.clone())
                    .next_delay()
                    .expect("unlimited attempts")
            })
            .collect();
        assert!(first.len() >= 2, "every client got {first:?}");
    }

    /// Each delay lies in `[base, base × 1.5]` and within `max_delay`, and a
    /// client's delays never decrease (#227).
    #[test]
    fn test_delay_within_jitter_bounds_and_non_decreasing() {
        let config = ReconnectionConfig::default();
        let max_delay = config.max_delay;
        for _ in 0..100 {
            let mut manager = ReconnectionManager::new(config.clone());
            let mut previous = Duration::ZERO;
            for attempt in 1..=12 {
                let base = config
                    .initial_delay
                    .saturating_mul(1 << (attempt - 1))
                    .min(max_delay);
                let delay = manager.next_delay().expect("unlimited attempts");
                assert!(delay >= base, "attempt {attempt}: {delay:?} < {base:?}");
                assert!(
                    delay <= base + base / 2,
                    "attempt {attempt}: {delay:?} > 1.5 × {base:?}"
                );
                assert!(delay <= max_delay, "attempt {attempt}: {delay:?}");
                assert!(
                    delay >= previous,
                    "attempt {attempt}: {delay:?} < {previous:?}"
                );
                previous = delay;
            }
        }
    }

    #[test]
    fn test_reconnection_config_default_uses_constants() {
        let config = ReconnectionConfig::default();
        assert_eq!(config.max_attempts, DEFAULT_MAX_ATTEMPTS);
        assert_eq!(
            config.initial_delay,
            Duration::from_millis(DEFAULT_INITIAL_DELAY_MS)
        );
        assert_eq!(
            config.max_delay,
            Duration::from_millis(DEFAULT_MAX_DELAY_MS)
        );
    }

    #[test]
    fn test_new_accepts_zero_max_attempts_as_unlimited() {
        let config = ReconnectionConfig::new(0, Duration::from_secs(1), Duration::from_secs(60))
            .expect("0 means unlimited");
        assert_eq!(config.max_attempts, 0);
        assert_eq!(ReconnectionManager::new(config).attempts_remaining(), None);
    }

    #[test]
    fn test_new_rejects_too_small_initial_delay() {
        let result = ReconnectionConfig::new(5, Duration::from_millis(50), Duration::from_secs(60));
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("initial_delay"),
            "Error should mention field name: {}",
            err
        );
        assert!(
            err.contains("100ms") || err.contains("50ms"),
            "Error should show values: {}",
            err
        );
    }

    #[test]
    fn test_new_rejects_max_delay_less_than_initial() {
        let result = ReconnectionConfig::new(5, Duration::from_secs(10), Duration::from_secs(5));
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("max_delay"),
            "Error should mention field name: {}",
            err
        );
        assert!(
            err.contains("initial_delay"),
            "Error should mention constraint relationship: {}",
            err
        );
    }

    #[test]
    fn test_new_accepts_valid_config() {
        let result =
            ReconnectionConfig::new(3, Duration::from_millis(500), Duration::from_secs(30));
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.max_attempts, 3);
        assert_eq!(config.initial_delay, Duration::from_millis(500));
        assert_eq!(config.max_delay, Duration::from_secs(30));
    }

    // Validation is now exclusively on `ReconnectionConfig::new(...)`.
    // The `with_*` chainable validators were replaced by the unvalidated
    // bon-derived `ReconnectionConfig::builder()` setters; users who want
    // validation construct via `new()` and surface `MarketDataError` to
    // their caller. The `test_new_rejects_*` tests above already cover
    // the validation matrix.
}
