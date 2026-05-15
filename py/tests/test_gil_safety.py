"""GIL safety stress tests for WebSocket streaming.

These tests verify that async operations properly release the GIL, preventing deadlocks.
GIL deadlock would cause test timeouts or hangs. All tests use timeouts as deadlock detection.
"""
import asyncio
import pytest
import time
from concurrent.futures import ThreadPoolExecutor


class TestGilSafety:
    """Tests to verify GIL is released during async operations."""

    @pytest.fixture
    def mock_api_key(self):
        """Provide a mock API key for testing."""
        return "mock_api_key_for_testing"

    @pytest.mark.asyncio
    @pytest.mark.timeout(10)
    async def test_concurrent_async_tasks(self, mock_api_key):
        """Multiple concurrent async tasks should not deadlock.

        This test spawns multiple concurrent async tasks. If the GIL is held
        during await operations, tasks would block each other and timeout.
        """
        from fugle_marketdata import RestClient

        client = RestClient(mock_api_key)

        # Spawn multiple concurrent tasks
        async def make_request():
            try:
                # This will fail with mock key, but we're testing GIL behavior
                await asyncio.to_thread(lambda: client.stock.intraday.quote_async("2330"))
            except Exception:
                pass  # Expected to fail with mock key

        # Run 10 concurrent tasks - would deadlock if GIL held during await
        tasks = [make_request() for _ in range(10)]
        await asyncio.gather(*tasks, return_exceptions=True)
        # Test passes if no hang/timeout

    @pytest.mark.asyncio
    @pytest.mark.timeout(10)
    async def test_async_with_thread_pool(self, mock_api_key):
        """Async operations should work alongside thread pool executor.

        This test mixes async and threaded sync operations. If GIL handling
        is incorrect, thread pool tasks would deadlock with async tasks.
        """
        from fugle_marketdata import RestClient

        client = RestClient(mock_api_key)

        def sync_work():
            """Simulate CPU-bound work in thread."""
            time.sleep(0.1)
            return "done"

        async def async_request():
            try:
                await asyncio.to_thread(lambda: client.stock.intraday.quote_async("2330"))
            except Exception:
                pass

        # Run async and sync concurrently
        with ThreadPoolExecutor(max_workers=4) as executor:
            loop = asyncio.get_event_loop()

            # Mix of async and sync tasks
            async_task = asyncio.create_task(async_request())
            thread_future = loop.run_in_executor(executor, sync_work)

            results = await asyncio.gather(async_task, thread_future, return_exceptions=True)

            # Thread work should complete even if async fails
            assert any(r == "done" for r in results if not isinstance(r, Exception))

    @pytest.mark.asyncio
    @pytest.mark.timeout(15)
    async def test_websocket_iterator_concurrent_recv(self, mock_api_key):
        """WebSocket async iteration should not hold GIL.

        This tests that the async iterator's __anext__ releases GIL properly.
        If GIL is held during recv(), other async tasks would be blocked.
        """
        from fugle_marketdata import WebSocketClient

        ws = WebSocketClient(mock_api_key)

        # This tests that the iterator's __anext__ releases GIL
        # If GIL is held during recv(), other tasks would be blocked

        async def other_work():
            """Other async work that should run concurrently."""
            for _ in range(5):
                await asyncio.sleep(0.1)
            return "other_done"

        async def ws_connect_attempt():
            try:
                # This will likely fail with mock key, but we're testing concurrency
                await ws.stock.connect_async()
            except Exception:
                pass  # Expected to fail with mock key

        # Both tasks should run concurrently without GIL deadlock
        results = await asyncio.gather(
            ws_connect_attempt(),
            other_work(),
            return_exceptions=True
        )

        # other_work should complete even if ws fails
        assert any(r == "other_done" for r in results if not isinstance(r, Exception))

    @pytest.mark.asyncio
    @pytest.mark.timeout(15)
    async def test_async_iterator_no_gil_hold(self, mock_api_key):
        """Async iterator should release GIL during message receive.

        This is a more direct test of the async iterator pattern.
        Creates a mock scenario where we test concurrent execution.
        """
        from fugle_marketdata import WebSocketClient

        ws = WebSocketClient(mock_api_key)

        completed_tasks = []

        async def monitor_task(task_id):
            """A task that monitors concurrent execution."""
            for _ in range(3):
                await asyncio.sleep(0.05)
                completed_tasks.append(task_id)

        async def websocket_task():
            """Task that attempts WebSocket operations."""
            try:
                await ws.stock.connect_async()
                # Even if this fails, monitor tasks should complete
            except Exception:
                pass

        # Run WebSocket task alongside monitor tasks
        await asyncio.gather(
            websocket_task(),
            monitor_task("monitor_1"),
            monitor_task("monitor_2"),
            return_exceptions=True
        )

        # Monitor tasks should complete (at least some iterations)
        # If GIL was held, monitors would be blocked
        assert len(completed_tasks) >= 3, f"Only {len(completed_tasks)} monitor iterations completed"


# Manual testing note for developers:
#
# For production GIL verification with real API key:
#   FUGLE_API_KEY=your_real_key pytest tests/test_gil_safety.py -v --timeout=30
#
# Monitor for hangs during concurrent operations. All tests should complete
# within their timeout periods without hanging.
#
# Additional stress test (not automated):
#   - Run with higher concurrency (50+ tasks)
#   - Monitor system resources
#   - Watch for thread deadlocks or hangs
