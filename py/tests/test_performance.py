"""
Performance Benchmark Tests

Measures latency and throughput for REST client operations.
Uses pytest-benchmark for statistical analysis and CI integration.
Compares against official fugle-marketdata-python SDK baseline.

Run: pytest py/tests/test_performance.py -v --benchmark-only
Output JSON: pytest py/tests/test_performance.py --benchmark-json=results.json
"""
import pytest
import asyncio
import json
from pathlib import Path
from fugle_marketdata import RestClient

# Skip if native library not available
pytestmark = pytest.mark.benchmark

# Load official SDK baseline if available
BASELINE_PATH = Path(__file__).parent / "benchmarks" / "baseline.json"
OFFICIAL_BASELINE = None
if BASELINE_PATH.exists():
    with open(BASELINE_PATH) as f:
        OFFICIAL_BASELINE = json.load(f)

# Performance thresholds from SC #4
PYTHON_THRESHOLD_MULTIPLIER = 2.0  # Within 2x of official SDK


class TestRestClientPerformance:
    """Benchmark REST client operations."""

    @pytest.fixture
    def client(self):
        """Create test client for benchmarks."""
        return RestClient("benchmark-test-key")

    def test_client_creation_latency(self, benchmark):
        """Benchmark client instantiation time."""
        def create_client():
            client = RestClient("test-key")
            return client

        result = benchmark(create_client)
        assert result is not None

    @pytest.mark.asyncio
    async def test_quote_ffi_overhead(self, benchmark, client):
        """Benchmark quote method call overhead (FFI boundary crossing).

        Note: This measures Python -> Rust -> Python overhead only.
        The call will fail without valid API key, but we're measuring
        the FFI boundary crossing time, not the network request.
        """
        def sync_wrapper():
            try:
                loop = asyncio.new_event_loop()
                try:
                    coro = client.stock.intraday.quote("2330")
                    loop.run_until_complete(coro)
                except Exception:
                    pass  # Expected without valid API key
                finally:
                    loop.close()
            except Exception:
                pass

        benchmark(sync_wrapper)


@pytest.mark.integration
class TestRestClientPerformanceIntegration:
    """Integration benchmarks requiring FUGLE_API_KEY."""

    @pytest.fixture
    def live_client(self):
        """Create client with real API key."""
        import os
        api_key = os.environ.get("FUGLE_API_KEY")
        if not api_key:
            pytest.skip("FUGLE_API_KEY not set")
        return RestClient(api_key)

    def test_quote_latency_live(self, benchmark, live_client):
        """Benchmark actual quote API call latency."""
        def sync_wrapper():
            loop = asyncio.new_event_loop()
            try:
                coro = live_client.stock.intraday.quote("2330")
                result = loop.run_until_complete(coro)
                return result
            finally:
                loop.close()

        result = benchmark(sync_wrapper)
        assert result is not None
        assert result.get('symbol') == '2330'

    def test_ticker_latency_live(self, benchmark, live_client):
        """Benchmark ticker API call latency."""
        def sync_wrapper():
            loop = asyncio.new_event_loop()
            try:
                coro = live_client.stock.intraday.ticker("2330")
                result = loop.run_until_complete(coro)
                return result
            finally:
                loop.close()

        result = benchmark(sync_wrapper)
        assert result is not None


@pytest.mark.integration
class TestOfficialSdkComparison:
    """Compare performance against official SDK baseline.

    Validates SC #4: "Performance benchmarks demonstrate competitive speed
    compared to official SDKs (within 2x for Python)"
    """

    @pytest.fixture
    def live_client(self):
        """Create client with real API key."""
        import os
        api_key = os.environ.get("FUGLE_API_KEY")
        if not api_key:
            pytest.skip("FUGLE_API_KEY not set")
        return RestClient(api_key)

    @pytest.fixture
    def baseline(self):
        """Load official SDK baseline."""
        if OFFICIAL_BASELINE is None:
            pytest.skip(
                "Official SDK baseline not recorded. "
                "Run: python tests/benchmarks/record_official_baseline.py"
            )
        return OFFICIAL_BASELINE

    def _measure_latency(self, operation, rounds=10, warmup=3):
        """Measure median latency for an operation."""
        import time
        import statistics

        # Warmup
        for _ in range(warmup):
            try:
                loop = asyncio.new_event_loop()
                loop.run_until_complete(operation())
                loop.close()
            except Exception:
                pass
            time.sleep(0.5)

        # Measure
        latencies = []
        for _ in range(rounds):
            loop = asyncio.new_event_loop()
            try:
                start = time.perf_counter()
                loop.run_until_complete(operation())
                elapsed = (time.perf_counter() - start) * 1000  # ms
                latencies.append(elapsed)
            except Exception:
                pass
            finally:
                loop.close()
            time.sleep(0.5)

        return statistics.median(latencies) if latencies else None

    def test_quote_within_2x_of_official(self, live_client, baseline):
        """Quote latency must be within 2x of official SDK."""
        official_median = baseline["operations"]["quote"]["median_ms"]

        our_median = self._measure_latency(
            lambda: live_client.stock.intraday.quote("2330")
        )

        assert our_median is not None, "Failed to measure our SDK latency"

        ratio = our_median / official_median
        threshold = PYTHON_THRESHOLD_MULTIPLIER

        print(f"\nQuote Performance:")
        print(f"  Official SDK: {official_median:.2f}ms")
        print(f"  Our SDK: {our_median:.2f}ms")
        print(f"  Ratio: {ratio:.2f}x (threshold: {threshold}x)")

        assert ratio <= threshold, (
            f"Quote latency {our_median:.2f}ms is {ratio:.2f}x "
            f"of official SDK {official_median:.2f}ms "
            f"(exceeds {threshold}x threshold)"
        )

    def test_ticker_within_2x_of_official(self, live_client, baseline):
        """Ticker latency must be within 2x of official SDK."""
        official_median = baseline["operations"]["ticker"]["median_ms"]

        our_median = self._measure_latency(
            lambda: live_client.stock.intraday.ticker("2330")
        )

        assert our_median is not None, "Failed to measure our SDK latency"

        ratio = our_median / official_median
        threshold = PYTHON_THRESHOLD_MULTIPLIER

        print(f"\nTicker Performance:")
        print(f"  Official SDK: {official_median:.2f}ms")
        print(f"  Our SDK: {our_median:.2f}ms")
        print(f"  Ratio: {ratio:.2f}x (threshold: {threshold}x)")

        assert ratio <= threshold, (
            f"Ticker latency {our_median:.2f}ms is {ratio:.2f}x "
            f"of official SDK {official_median:.2f}ms "
            f"(exceeds {threshold}x threshold)"
        )

    def test_trades_within_2x_of_official(self, live_client, baseline):
        """Trades latency must be within 2x of official SDK."""
        official_median = baseline["operations"]["trades"]["median_ms"]

        our_median = self._measure_latency(
            lambda: live_client.stock.intraday.trades("2330")
        )

        assert our_median is not None, "Failed to measure our SDK latency"

        ratio = our_median / official_median
        threshold = PYTHON_THRESHOLD_MULTIPLIER

        print(f"\nTrades Performance:")
        print(f"  Official SDK: {official_median:.2f}ms")
        print(f"  Our SDK: {our_median:.2f}ms")
        print(f"  Ratio: {ratio:.2f}x (threshold: {threshold}x)")

        assert ratio <= threshold, (
            f"Trades latency {our_median:.2f}ms is {ratio:.2f}x "
            f"of official SDK {official_median:.2f}ms "
            f"(exceeds {threshold}x threshold)"
        )
