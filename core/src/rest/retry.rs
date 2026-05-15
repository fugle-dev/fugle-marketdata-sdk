//! Opt-in retry policy for [`RestClient`].
//!
//! Retry is OFF by default — observability use cases need visibility into
//! transient failures. Callers opt in via
//! [`RestClient::with_retry`](super::RestClient::with_retry).
//!
//! Only errors for which [`MarketDataError::is_retryable`] returns `true`
//! are retried (HTTP 429 + 5xx, transport timeouts, connection errors).
//! All other errors propagate to the caller on the first attempt.

use crate::errors::MarketDataError;
use std::time::Duration;

/// Configuration for transparent retry of REST requests.
///
/// Construct via the derived [`RetryPolicy::builder`] for full control, use
/// [`RetryPolicy::conservative`] / [`RetryPolicy::aggressive`] for typical
/// presets, or keep [`RetryPolicy::new`] for the legacy positional form. Pass
/// to [`RestClient::with_retry`](super::RestClient::with_retry) to enable.
#[derive(Debug, Clone, Copy, bon::Builder)]
pub struct RetryPolicy {
    /// Maximum number of attempts (including the first). A value of `1`
    /// disables retry behaviour.
    pub max_attempts: u32,

    /// Base delay before the second attempt. Subsequent attempts double the
    /// delay until [`max_backoff`](Self::max_backoff) is hit.
    pub initial_backoff: Duration,

    /// Hard cap on the inter-attempt delay (jitter notwithstanding).
    pub max_backoff: Duration,
}

impl RetryPolicy {
    /// Build a custom policy via positional arguments.
    ///
    /// Equivalent to the derived
    /// `RetryPolicy::builder().max_attempts(..).initial_backoff(..).max_backoff(..).build()`.
    pub fn new(max_attempts: u32, initial_backoff: Duration, max_backoff: Duration) -> Self {
        Self {
            max_attempts,
            initial_backoff,
            max_backoff,
        }
    }

    /// Conservative preset: 3 attempts, 100 ms initial backoff, 2 s ceiling.
    /// Safe default for monitoring or critical-path REST calls.
    pub fn conservative() -> Self {
        Self::new(3, Duration::from_millis(100), Duration::from_secs(2))
    }

    /// Aggressive preset: 5 attempts, 250 ms initial backoff, 10 s ceiling.
    /// Use when the upstream is known to be flaky and the caller can
    /// tolerate longer total latency.
    pub fn aggressive() -> Self {
        Self::new(5, Duration::from_millis(250), Duration::from_secs(10))
    }

    /// Compute the delay before attempt `attempt` (1-indexed).
    ///
    /// Attempt 1 returns `Duration::ZERO` (no pre-attempt delay). Subsequent
    /// attempts use exponential backoff capped at `max_backoff`, plus a
    /// uniform random jitter in `[0, initial_backoff)`.
    pub(crate) fn delay_for_attempt(&self, attempt: u32) -> Duration {
        if attempt <= 1 {
            return Duration::ZERO;
        }
        let exp = attempt.saturating_sub(1).min(31);
        let multiplier = 1u64 << exp;
        let raw_nanos = self
            .initial_backoff
            .as_nanos()
            .saturating_mul(u128::from(multiplier));
        let capped = raw_nanos.min(self.max_backoff.as_nanos());
        let base = Duration::from_nanos(capped.min(u128::from(u64::MAX)) as u64);
        base + jitter(self.initial_backoff)
    }
}

/// Pseudo-random jitter in the range `[0, ceiling)`.
///
/// Uses a per-call `RandomState` hash of the current monotonic timestamp —
/// good enough for backoff smoothing and avoids pulling in `rand` as a
/// runtime dependency.
fn jitter(ceiling: Duration) -> Duration {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    use std::time::Instant;

    let nanos_ceil = ceiling.as_nanos().min(u128::from(u64::MAX)) as u64;
    if nanos_ceil == 0 {
        return Duration::ZERO;
    }
    let now = Instant::now().elapsed().as_nanos() as u64;
    let mut hasher = RandomState::new().build_hasher();
    hasher.write_u64(now);
    let pseudo_random = hasher.finish() % nanos_ceil;
    Duration::from_nanos(pseudo_random)
}

/// Run `op` under `policy`, retrying on retryable errors.
///
/// `op` is invoked at least once and at most `policy.max_attempts` times.
/// On a retryable failure the function sleeps according to
/// [`RetryPolicy::delay_for_attempt`] and tries again. On a non-retryable
/// failure or success, returns immediately. Exhausted retries return the
/// last error verbatim — no synthetic wrapper.
pub(crate) fn run<T>(
    policy: &RetryPolicy,
    mut op: impl FnMut() -> Result<T, MarketDataError>,
) -> Result<T, MarketDataError> {
    let mut last_err: Option<MarketDataError> = None;
    for attempt in 1..=policy.max_attempts {
        let delay = policy.delay_for_attempt(attempt);
        if !delay.is_zero() {
            std::thread::sleep(delay);
        }
        match op() {
            Ok(value) => return Ok(value),
            Err(err) => {
                if !err.is_retryable() || attempt == policy.max_attempts {
                    return Err(err);
                }
                last_err = Some(err);
            }
        }
    }
    // Unreachable when max_attempts >= 1, but keeps the compiler happy.
    Err(last_err.unwrap_or(MarketDataError::RuntimeError {
        msg: "retry loop exited without error or success".into(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn test_conservative_preset() {
        let p = RetryPolicy::conservative();
        assert_eq!(p.max_attempts, 3);
        assert_eq!(p.initial_backoff, Duration::from_millis(100));
        assert_eq!(p.max_backoff, Duration::from_secs(2));
    }

    #[test]
    fn test_aggressive_preset() {
        let p = RetryPolicy::aggressive();
        assert_eq!(p.max_attempts, 5);
    }

    #[test]
    fn test_bon_builder_matches_new() {
        let via_builder = RetryPolicy::builder()
            .max_attempts(4)
            .initial_backoff(Duration::from_millis(50))
            .max_backoff(Duration::from_secs(5))
            .build();
        let via_new = RetryPolicy::new(4, Duration::from_millis(50), Duration::from_secs(5));
        assert_eq!(via_builder.max_attempts, via_new.max_attempts);
        assert_eq!(via_builder.initial_backoff, via_new.initial_backoff);
        assert_eq!(via_builder.max_backoff, via_new.max_backoff);
    }

    #[test]
    fn test_first_attempt_no_delay() {
        let p = RetryPolicy::conservative();
        assert_eq!(p.delay_for_attempt(1), Duration::ZERO);
    }

    #[test]
    fn test_delay_capped_at_max() {
        let p = RetryPolicy::new(20, Duration::from_millis(100), Duration::from_millis(500));
        for attempt in 1..=10 {
            let d = p.delay_for_attempt(attempt);
            // base capped at 500ms; jitter ceiling = 100ms.
            assert!(d <= Duration::from_millis(600), "attempt {} = {:?}", attempt, d);
        }
    }

    #[test]
    fn test_run_succeeds_first_attempt() {
        let p = RetryPolicy::new(3, Duration::from_millis(1), Duration::from_millis(10));
        let result: Result<i32, MarketDataError> = run(&p, || Ok(42));
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_run_retries_retryable() {
        let p = RetryPolicy::new(3, Duration::from_millis(1), Duration::from_millis(10));
        let attempts = Cell::new(0u32);
        let result: Result<i32, MarketDataError> = run(&p, || {
            let n = attempts.get() + 1;
            attempts.set(n);
            if n < 2 {
                Err(MarketDataError::ApiError {
                    status: 503,
                    message: "transient".into(),
                })
            } else {
                Ok(42)
            }
        });
        assert_eq!(result.unwrap(), 42);
        assert_eq!(attempts.get(), 2);
    }

    #[test]
    fn test_run_does_not_retry_non_retryable() {
        let p = RetryPolicy::new(5, Duration::from_millis(1), Duration::from_millis(10));
        let attempts = Cell::new(0u32);
        let result: Result<i32, MarketDataError> = run(&p, || {
            attempts.set(attempts.get() + 1);
            Err(MarketDataError::ApiError {
                status: 401,
                message: "unauthorized".into(),
            })
        });
        assert!(result.is_err());
        assert_eq!(attempts.get(), 1);
    }

    #[test]
    fn test_run_exhausts_and_returns_last_error() {
        let p = RetryPolicy::new(3, Duration::from_millis(1), Duration::from_millis(10));
        let attempts = Cell::new(0u32);
        let result: Result<i32, MarketDataError> = run(&p, || {
            attempts.set(attempts.get() + 1);
            Err(MarketDataError::ApiError {
                status: 503,
                message: "still down".into(),
            })
        });
        assert_eq!(attempts.get(), 3);
        match result.unwrap_err() {
            MarketDataError::ApiError { status, message } => {
                assert_eq!(status, 503);
                assert_eq!(message, "still down");
            }
            other => panic!("expected ApiError, got {:?}", other),
        }
    }
}
