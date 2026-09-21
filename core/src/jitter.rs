//! Backoff jitter shared by REST retry and WebSocket reconnection.

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Pseudo-random jitter in the range `[0, ceiling)`.
///
/// Each call hashes the wall clock with a fresh `RandomState`, whose keys
/// are seeded randomly per thread and advance on every construction, so
/// the result differs between processes, threads and calls. Good enough to
/// spread backoff across clients without pulling in `rand` as a runtime
/// dependency.
pub(crate) fn jitter(ceiling: Duration) -> Duration {
    let nanos_ceil = ceiling.as_nanos().min(u128::from(u64::MAX)) as u64;
    if nanos_ceil == 0 {
        return Duration::ZERO;
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos() as u64);
    let mut hasher = RandomState::new().build_hasher();
    hasher.write_u64(now);
    Duration::from_nanos(hasher.finish() % nanos_ceil)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jitter_within_ceiling() {
        let ceiling = Duration::from_millis(500);
        for _ in 0..1000 {
            assert!(jitter(ceiling) < ceiling);
        }
    }

    #[test]
    fn test_jitter_zero_ceiling() {
        assert_eq!(jitter(Duration::ZERO), Duration::ZERO);
    }
}
