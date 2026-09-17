// Throttling for repeated reports (#83): a listener callback that keeps
// throwing should not flood OnError. Mirrors the semantics of the Rust
// core's `ReportThrottle` (core/src/websocket/report_throttle.rs).
using System;
using System.Diagnostics;
using System.Runtime.CompilerServices;

[assembly: InternalsVisibleTo("MarketdataUniffi.Tests")]

namespace FugleMarketData
{
    /// <summary>
    /// Counts occurrences and decides when they are due to be reported.
    ///
    /// The first occurrence is reported at once; later ones at most once per
    /// <see cref="ReportInterval"/>, each report carrying how many occurred
    /// since the previous one. Occurrences left over when they stop are
    /// never flushed — only counted into the next report, if any.
    ///
    /// Thread-safe: <see cref="Record"/> may be called concurrently from
    /// multiple callback threads.
    /// </summary>
    internal sealed class ReportThrottle
    {
        /// <summary>Minimum spacing between two throttled reports.</summary>
        public static readonly TimeSpan ReportInterval = TimeSpan.FromSeconds(1);

        /// <summary>Monotonic clock for the default constructor, unaffected by wall-clock adjustments.</summary>
        private static readonly Stopwatch Clock = Stopwatch.StartNew();

        private readonly Func<TimeSpan> _now;
        private readonly object _lock = new object();
        private ulong _pending;
        private TimeSpan? _lastReport;

        /// <param name="now">
        /// Monotonic clock (time elapsed since an arbitrary start), injectable
        /// for tests. Defaults to a <see cref="Stopwatch"/>.
        /// </param>
        public ReportThrottle(Func<TimeSpan>? now = null)
        {
            _now = now ?? (() => Clock.Elapsed);
        }

        /// <summary>
        /// Count one occurrence and return the count to report — never made
        /// yet, or <see cref="ReportInterval"/> since the last report — or
        /// <c>null</c> when the occurrence is suppressed (folded into the
        /// next report).
        /// </summary>
        public ulong? Record()
        {
            lock (_lock)
            {
                var now = _now();
                _pending++;

                var due = _lastReport == null || (now - _lastReport.Value) >= ReportInterval;
                if (!due)
                {
                    return null;
                }

                _lastReport = now;
                var count = _pending;
                _pending = 0;
                return count;
            }
        }
    }
}
