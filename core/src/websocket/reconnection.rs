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
    /// always returns `false` regardless of the close code.
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
    /// `should_reconnect()` will always return `false` regardless of close code.
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
/// - Whether a close code is retriable
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

    /// Determine if reconnection should be attempted based on close code
    ///
    /// From CONTEXT.md decisions:
    /// - 1001 (Going away) → reconnect
    /// - 1006 (Abnormal closure) → reconnect
    /// - 4001 (Auth failure) → don't reconnect
    /// - 4000-4999 (Application errors) → don't reconnect
    /// - 1000 (Normal closure) → don't reconnect
    /// - Others → reconnect by default
    ///
    /// Always returns `false` if the underlying [`ReconnectionConfig::enabled`]
    /// flag is `false` (see [`ReconnectionConfig::disabled`]).
    pub fn should_reconnect(&self, close_code: Option<u16>) -> bool {
        if !self.config.enabled {
            return false;
        }
        match close_code {
            Some(1000) => false, // Normal closure
            Some(1001) => true,  // Going away
            Some(1006) => true,  // Abnormal closure
            Some(4001) => false, // Auth failure
            Some(code) if (4000..=4999).contains(&code) => false, // Application errors
            _ => true, // Default: reconnect on unknown errors
        }
    }

    /// Calculate next reconnection delay with exponential backoff and jitter
    ///
    /// Returns None if max attempts reached, Some(duration) otherwise; with
    /// `max_attempts == 0` (unlimited) it always returns Some.
    /// Increments attempt counter.
    pub fn next_delay(&mut self) -> Option<Duration> {
        if self.config.max_attempts != 0 && self.current_attempt >= self.config.max_attempts {
            return None;
        }

        self.current_attempt = self.current_attempt.saturating_add(1);

        // Calculate exponential backoff: initial * 2^(attempt-1)
        let exponential_millis = self.config.initial_delay.as_millis()
            * 2_u128.pow((self.current_attempt - 1).min(10)); // Cap at 2^10 to avoid overflow

        // Apply max_delay cap
        let capped_millis = exponential_millis.min(self.config.max_delay.as_millis());

        // Add simple deterministic jitter based on attempt number (0-15% of delay)
        // This avoids thundering herd without requiring rand dependency
        let jitter_percent = (self.current_attempt * 3) % 16; // 0-15%
        let jitter = (capped_millis * jitter_percent as u128) / 100;
        let final_millis = capped_millis.saturating_add(jitter);

        Some(Duration::from_millis(final_millis as u64))
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
        assert!(!manager.should_reconnect(Some(1006)));
        assert!(!manager.should_reconnect(Some(1001)));
        assert!(!manager.should_reconnect(None));
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

    #[test]
    fn test_should_reconnect_on_1006() {
        let manager = ReconnectionManager::new(enabled_config());

        // 1006 (Abnormal closure) should reconnect
        assert!(manager.should_reconnect(Some(1006)));
    }

    #[test]
    fn test_should_reconnect_on_1001() {
        let manager = ReconnectionManager::new(enabled_config());

        // 1001 (Going away) should reconnect
        assert!(manager.should_reconnect(Some(1001)));
    }

    #[test]
    fn test_should_not_reconnect_on_4001() {
        let manager = ReconnectionManager::new(enabled_config());

        // 4001 (Auth failure) should not reconnect
        assert!(!manager.should_reconnect(Some(4001)));
    }

    #[test]
    fn test_should_not_reconnect_on_1000() {
        let manager = ReconnectionManager::new(enabled_config());

        // 1000 (Normal closure) should not reconnect
        assert!(!manager.should_reconnect(Some(1000)));
    }

    #[test]
    fn test_should_not_reconnect_on_4xxx() {
        let manager = ReconnectionManager::new(enabled_config());

        // Application errors (4000-4999) should not reconnect
        assert!(!manager.should_reconnect(Some(4000)));
        assert!(!manager.should_reconnect(Some(4500)));
        assert!(!manager.should_reconnect(Some(4999)));
    }

    #[test]
    fn test_should_reconnect_on_unknown() {
        let manager = ReconnectionManager::new(enabled_config());

        // Unknown errors should reconnect by default
        assert!(manager.should_reconnect(Some(1002)));
        assert!(manager.should_reconnect(Some(1003)));
        assert!(manager.should_reconnect(None));
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
            // max_delay caps the backoff; jitter adds at most 15% on top.
            assert!(last <= max_delay + max_delay * 15 / 100, "{last:?}");
        }
        assert_eq!(manager.current_attempt(), 100);
        assert_eq!(manager.attempts_remaining(), None);
        assert!(last >= max_delay, "backoff reaches the cap: {last:?}");
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
