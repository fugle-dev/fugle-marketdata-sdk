package tw.com.fugle.marketdata;

import java.util.function.LongSupplier;

/**
 * Throttling for repeated reports (#83): a listener callback that keeps
 * throwing should not flood {@code onError}. Mirrors the semantics of the
 * Rust core's {@code ReportThrottle} (core/src/websocket/report_throttle.rs).
 *
 * <p>The first occurrence is reported at once; later ones at most once per
 * {@link #REPORT_INTERVAL_MS}, each report carrying how many occurred since
 * the previous one. Occurrences left over when they stop are never flushed
 * — only counted into the next report, if any.
 *
 * <p>Thread-safe: {@link #record()} may be called concurrently from
 * multiple callback threads.
 */
final class ReportThrottle {
    /** Minimum spacing between two throttled reports, in milliseconds. */
    static final long REPORT_INTERVAL_MS = 1000L;

    private final LongSupplier nowMillis;
    private long pending;
    private Long lastReportMillis;

    ReportThrottle() {
        // Monotonic: unaffected by wall-clock adjustments.
        this(() -> System.nanoTime() / 1_000_000L);
    }

    /**
     * @param nowMillis clock to use, injectable for tests. Must be
     *     monotonically non-decreasing for the throttle's semantics to hold.
     */
    ReportThrottle(LongSupplier nowMillis) {
        this.nowMillis = nowMillis;
    }

    /**
     * Count one occurrence and return the count to report — never made yet,
     * or {@link #REPORT_INTERVAL_MS} since the last report — or {@code null}
     * when the occurrence is suppressed (folded into the next report).
     */
    synchronized Long record() {
        long now = nowMillis.getAsLong();
        pending++;

        boolean due = lastReportMillis == null || (now - lastReportMillis) >= REPORT_INTERVAL_MS;
        if (!due) {
            return null;
        }

        lastReportMillis = now;
        long count = pending;
        pending = 0;
        return count;
    }
}
