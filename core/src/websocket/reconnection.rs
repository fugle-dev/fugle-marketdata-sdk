//! WebSocket reconnection logic with exponential backoff

use std::time::Duration;

use crate::MarketDataError;

/// Default maximum reconnection attempts (VAL-02)
pub const DEFAULT_MAX_ATTEMPTS: u32 = 5;

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
/// `enabled` defaults to `false` so the binding layer matches the historical
/// `fugle-marketdata-{python,node}` SDKs (no auto-reconnect unless the caller
/// asks for it). Explicitly constructing via [`ReconnectionConfig::new`] sets
/// `enabled: true` because the caller has expressed intent.
#[derive(Debug, Clone)]
pub struct ReconnectionConfig {
    /// Whether auto-reconnect is active. When `false`, [`ReconnectionManager::should_reconnect`]
    /// always returns `false` regardless of the close code.
    pub enabled: bool,
    /// Maximum reconnection attempts before giving up
    pub max_attempts: u32,
    /// Initial delay before first reconnection attempt
    pub initial_delay: Duration,
    /// Maximum delay between reconnection attempts
    pub max_delay: Duration,
}

impl Default for ReconnectionConfig {
    fn default() -> Self {
        Self {
            // Off by default — matches old fugle-marketdata SDKs that have no
            // auto-reconnect at all. Bindings flip this on when the user
            // explicitly passes a `reconnect:` option block.
            enabled: false,
            max_attempts: DEFAULT_MAX_ATTEMPTS,
            initial_delay: Duration::from_millis(DEFAULT_INITIAL_DELAY_MS),
            max_delay: Duration::from_millis(DEFAULT_MAX_DELAY_MS),
        }
    }
}

impl ReconnectionConfig {
    /// Create a new reconnection config with validation
    ///
    /// # Errors
    /// Returns `MarketDataError::ConfigError` if:
    /// - `max_attempts` is 0 (must be >= 1)
    /// - `initial_delay` is less than 100ms
    /// - `max_delay` is less than `initial_delay`
    ///
    /// Constructing via `new()` is treated as explicit opt-in, so the
    /// returned config has `enabled: true`. To get a disabled config (e.g.
    /// to fall back to "no reconnect at all") use [`ReconnectionConfig::disabled`]
    /// or `ReconnectionConfig::default()`.
    pub fn new(
        max_attempts: u32,
        initial_delay: Duration,
        max_delay: Duration,
    ) -> Result<Self, MarketDataError> {
        if max_attempts == 0 {
            return Err(MarketDataError::ConfigError(
                "max_attempts must be >= 1".to_string(),
            ));
        }

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

    /// Build an explicitly disabled reconnection config
    ///
    /// `should_reconnect()` will always return `false` regardless of close code.
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Self::default()
        }
    }

    /// Builder: set max attempts with validation
    ///
    /// # Errors
    /// Returns `MarketDataError::ConfigError` if `max_attempts` is 0
    pub fn with_max_attempts(mut self, max_attempts: u32) -> Result<Self, MarketDataError> {
        if max_attempts == 0 {
            return Err(MarketDataError::ConfigError(
                "max_attempts must be >= 1".to_string(),
            ));
        }
        self.max_attempts = max_attempts;
        Ok(self)
    }

    /// Builder: set initial delay with validation
    ///
    /// # Errors
    /// Returns `MarketDataError::ConfigError` if `initial_delay` is less than 100ms
    pub fn with_initial_delay(mut self, initial_delay: Duration) -> Result<Self, MarketDataError> {
        if initial_delay < Duration::from_millis(MIN_INITIAL_DELAY_MS) {
            return Err(MarketDataError::ConfigError(format!(
                "initial_delay must be >= {}ms (got {}ms)",
                MIN_INITIAL_DELAY_MS,
                initial_delay.as_millis()
            )));
        }
        self.initial_delay = initial_delay;
        Ok(self)
    }

    /// Builder: set max delay with validation
    ///
    /// # Errors
    /// Returns `MarketDataError::ConfigError` if `max_delay` is less than `initial_delay`
    pub fn with_max_delay(mut self, max_delay: Duration) -> Result<Self, MarketDataError> {
        if max_delay < self.initial_delay {
            return Err(MarketDataError::ConfigError(format!(
                "max_delay ({}ms) must be >= initial_delay ({}ms)",
                max_delay.as_millis(),
                self.initial_delay.as_millis()
            )));
        }
        self.max_delay = max_delay;
        Ok(self)
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
    /// flag is `false` (the default — matches old `fugle-marketdata` SDKs).
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
    /// Returns None if max attempts reached, Some(duration) otherwise.
    /// Increments attempt counter.
    pub fn next_delay(&mut self) -> Option<Duration> {
        if self.current_attempt >= self.config.max_attempts {
            return None;
        }

        self.current_attempt += 1;

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

    /// Get number of remaining reconnection attempts
    pub fn attempts_remaining(&self) -> u32 {
        self.config.max_attempts.saturating_sub(self.current_attempt)
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
    /// `default()` is now disabled (matches old fugle-marketdata SDKs), so
    /// any test that exercises the close-code logic must opt in.
    fn enabled_config() -> ReconnectionConfig {
        ReconnectionConfig::new(5, Duration::from_secs(1), Duration::from_secs(60))
            .expect("test config is valid")
    }

    #[test]
    fn test_reconnection_config_default() {
        let config = ReconnectionConfig::default();
        assert!(!config.enabled, "default must be disabled to match old SDK behaviour");
        assert_eq!(config.max_attempts, 5);
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
    fn test_default_config_never_reconnects() {
        // The default config must short-circuit `should_reconnect` even on
        // codes the close-code logic considers retriable (1006, 1001, …).
        let manager = ReconnectionManager::new(ReconnectionConfig::default());
        assert!(!manager.should_reconnect(Some(1006)));
        assert!(!manager.should_reconnect(Some(1001)));
        assert!(!manager.should_reconnect(None));
    }

    #[test]
    fn test_reconnection_config_builder() {
        let config = ReconnectionConfig::default()
            .with_max_attempts(10)
            .unwrap()
            .with_initial_delay(Duration::from_secs(2))
            .unwrap()
            .with_max_delay(Duration::from_secs(120))
            .unwrap();

        assert_eq!(config.max_attempts, 10);
        assert_eq!(config.initial_delay, Duration::from_secs(2));
        assert_eq!(config.max_delay, Duration::from_secs(120));
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
        let config = ReconnectionConfig::default();
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
        let config = ReconnectionConfig::default();
        let mut manager = ReconnectionManager::new(config);

        // Exhaust attempts
        let _ = manager.next_delay();
        let _ = manager.next_delay();
        assert_eq!(manager.current_attempt(), 2);

        // Reset should clear attempts
        manager.reset();
        assert_eq!(manager.current_attempt(), 0);
        assert_eq!(manager.attempts_remaining(), 5);

        // Should be able to get delays again
        let delay = manager.next_delay();
        assert!(delay.is_some());
    }

    #[test]
    fn test_max_attempts_reached() {
        let config = ReconnectionConfig::default().with_max_attempts(3).unwrap();
        let mut manager = ReconnectionManager::new(config);

        // Get 3 delays
        assert!(manager.next_delay().is_some());
        assert!(manager.next_delay().is_some());
        assert!(manager.next_delay().is_some());

        // 4th attempt should return None
        assert!(manager.next_delay().is_none());
        assert_eq!(manager.attempts_remaining(), 0);
    }

    #[test]
    fn test_attempts_remaining() {
        let config = ReconnectionConfig::default().with_max_attempts(5).unwrap();
        let mut manager = ReconnectionManager::new(config);

        assert_eq!(manager.attempts_remaining(), 5);

        let _ = manager.next_delay();
        assert_eq!(manager.attempts_remaining(), 4);

        let _ = manager.next_delay();
        assert_eq!(manager.attempts_remaining(), 3);
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
    fn test_new_rejects_zero_max_attempts() {
        let result = ReconnectionConfig::new(0, Duration::from_secs(1), Duration::from_secs(60));
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("max_attempts"),
            "Error should mention field name: {}",
            err
        );
        assert!(
            err.contains(">= 1") || err.contains("must be"),
            "Error should mention constraint: {}",
            err
        );
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

    #[test]
    fn test_builder_rejects_zero_max_attempts() {
        let result = ReconnectionConfig::default().with_max_attempts(0);
        assert!(result.is_err());
    }

    #[test]
    fn test_builder_rejects_too_small_initial_delay() {
        let result = ReconnectionConfig::default().with_initial_delay(Duration::from_millis(50));
        assert!(result.is_err());
    }

    #[test]
    fn test_builder_rejects_max_delay_less_than_initial() {
        // First set a larger initial_delay, then try to set smaller max_delay
        let config = ReconnectionConfig::default()
            .with_initial_delay(Duration::from_secs(30))
            .unwrap();
        let result = config.with_max_delay(Duration::from_secs(10));
        assert!(result.is_err());
    }
}
