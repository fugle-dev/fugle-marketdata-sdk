//! Throttling for repeated reports (#83): `MessagesDropped`, and the
//! bindings' reports of a callback that failed.
//!
//! The first occurrence is reported at once; later ones at most once per
//! [`REPORT_INTERVAL`], each report carrying how many occurred since the
//! previous one. Nothing reports occurrences left over when they stop: they
//! are only counted into the next report.

use std::time::{Duration, Instant};

/// Minimum spacing between two throttled reports.
pub const REPORT_INTERVAL: Duration = Duration::from_secs(1);

/// Counts occurrences and decides when they are due to be reported.
///
/// Not synchronized: keep it behind the lock that guards what it counts.
#[derive(Debug, Default, Clone)]
pub struct ReportThrottle {
    /// Occurrences not covered by a report yet.
    pending: u64,
    /// When the last report was due.
    last_report: Option<Instant>,
}

impl ReportThrottle {
    /// A throttle with nothing counted and no report made.
    pub fn new() -> Self {
        Self::default()
    }

    /// Count one occurrence at `now` and return the count to report if a
    /// report is due (see [`Self::take_due`]).
    pub fn record(&mut self, now: Instant) -> Option<u64> {
        self.count();
        self.take_due(now, false)
    }

    /// Count one occurrence without checking whether a report is due.
    pub fn count(&mut self) {
        self.pending = self.pending.saturating_add(1);
    }

    /// Occurrences not covered by a report yet.
    pub fn pending(&self) -> u64 {
        self.pending
    }

    /// If anything is pending and a report is due — none made yet, or
    /// [`REPORT_INTERVAL`] since the last one, or `force` — start a new
    /// interval at `now` and return the pending count, resetting it.
    pub fn take_due(&mut self, now: Instant, force: bool) -> Option<u64> {
        if self.pending == 0 {
            return None;
        }
        let throttled = self
            .last_report
            .is_some_and(|last| now.saturating_duration_since(last) < REPORT_INTERVAL);
        if throttled && !force {
            return None;
        }
        self.last_report = Some(now);
        Some(std::mem::take(&mut self.pending))
    }

    /// Forget the pending count and the last report: the next occurrence is
    /// reported at once.
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_occurrence_is_reported_at_once() {
        let mut throttle = ReportThrottle::new();
        assert_eq!(throttle.record(Instant::now()), Some(1));
    }

    #[test]
    fn occurrences_within_the_interval_count_into_the_next_report() {
        let start = Instant::now();
        let mut throttle = ReportThrottle::new();
        assert_eq!(throttle.record(start), Some(1));
        assert_eq!(throttle.record(start + Duration::from_millis(100)), None);
        assert_eq!(throttle.record(start + Duration::from_millis(900)), None);
        assert_eq!(throttle.pending(), 2);
        assert_eq!(throttle.record(start + REPORT_INTERVAL), Some(3));
        assert_eq!(throttle.pending(), 0);
    }

    #[test]
    fn interval_restarts_at_each_report() {
        let start = Instant::now();
        let mut throttle = ReportThrottle::new();
        assert_eq!(throttle.record(start), Some(1));
        let second = start + Duration::from_millis(1500);
        assert_eq!(throttle.record(second), Some(1));
        assert_eq!(throttle.record(second + Duration::from_millis(999)), None);
        assert_eq!(throttle.record(second + REPORT_INTERVAL), Some(2));
    }

    #[test]
    fn force_reports_within_the_interval_and_nothing_pending_reports_nothing() {
        let start = Instant::now();
        let mut throttle = ReportThrottle::new();
        assert_eq!(throttle.take_due(start, true), None);
        assert_eq!(throttle.record(start), Some(1));
        throttle.count();
        assert_eq!(throttle.take_due(start, false), None);
        assert_eq!(throttle.take_due(start, true), Some(1));
    }

    #[test]
    fn reset_reports_the_next_occurrence_at_once() {
        let start = Instant::now();
        let mut throttle = ReportThrottle::new();
        assert_eq!(throttle.record(start), Some(1));
        throttle.count();
        throttle.reset();
        assert_eq!(throttle.pending(), 0);
        assert_eq!(throttle.record(start), Some(1));
    }
}
