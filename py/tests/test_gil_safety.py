"""GIL safety stress tests for async REST calls and WebSocket streaming.

These tests verify that async operations properly release the GIL, preventing deadlocks.
GIL deadlock would cause test timeouts or hangs. All tests use timeouts as deadlock detection.
"""
import asyncio
import pytest
import time
from concurrent.futures import ThreadPoolExecutor

from fugle_marketdata import AuthError, RestClient
from tests.ws_loopback import LoopbackServer, disconnect_quietly, product_ws


class TestGilSafety:
    """Tests to verify GIL is released during async operations."""

    @pytest.fixture
    def mock_api_key(self):
        """Provide a mock API key for testing."""
        return "mock_api_key_for_testing"

    @pytest.mark.asyncio
    @pytest.mark.timeout(10)
    async def test_concurrent_async_tasks(self, mock_api_key, rest_server):
        """Multiple concurrent async tasks should not deadlock.

        This test spawns multiple concurrent async tasks. If the GIL is held
        during await operations, tasks would block each other and timeout.
        The loopback server runs on a thread of this process, so it could not
        answer either (#71).
        """
        client = RestClient(api_key=mock_api_key, base_url=rest_server.url)

        # Run 10 concurrent requests - would deadlock if GIL held during await
        results = await asyncio.gather(
            *(client.stock.intraday.quote_async("2330") for _ in range(10)),
            return_exceptions=True,
        )

        assert all(isinstance(r, AuthError) for r in results), results
        assert len(rest_server.requests) == 10

    @pytest.mark.asyncio
    @pytest.mark.timeout(10)
    async def test_async_with_thread_pool(self, mock_api_key, rest_server):
        """Async operations should work alongside thread pool executor.

        This test mixes async and threaded sync operations. If GIL handling
        is incorrect, thread pool tasks would deadlock with async tasks.
        """
        client = RestClient(api_key=mock_api_key, base_url=rest_server.url)

        def sync_work():
            """Simulate CPU-bound work in thread."""
            time.sleep(0.1)
            return "done"

        # Run async and sync concurrently
        with ThreadPoolExecutor(max_workers=4) as executor:
            loop = asyncio.get_running_loop()

            # Mix of async and sync tasks
            async_request = client.stock.intraday.quote_async("2330")
            thread_future = loop.run_in_executor(executor, sync_work)

            results = await asyncio.gather(async_request, thread_future, return_exceptions=True)

        assert isinstance(results[0], AuthError), results
        assert results[1] == "done"
        assert len(rest_server.requests) == 1

    @pytest.mark.asyncio
    @pytest.mark.timeout(15)
    async def test_websocket_iterator_concurrent_recv(self, mock_api_key):
        """WebSocket async iteration should not hold GIL.

        This tests that the async iterator's __anext__ releases GIL properly.
        If GIL is held during recv(), other async tasks would be blocked.
        """
        # A local server in a child process, so the test needs no network and
        # the server does not compete for this process's GIL (#66).
        with LoopbackServer() as srv:
            ws = product_ws(srv.url, "stock", api_key=mock_api_key)

            async def other_work():
                """Other async work that should run concurrently."""
                for _ in range(5):
                    await asyncio.sleep(0.1)
                return "other_done"

            try:
                # Both tasks should run concurrently without GIL deadlock
                results = await asyncio.gather(ws.connect_async(), other_work())
            finally:
                disconnect_quietly(ws)

        assert results[1] == "other_done"

    @pytest.mark.asyncio
    @pytest.mark.timeout(15)
    async def test_async_iterator_no_gil_hold(self, mock_api_key):
        """Async iterator should release GIL during message receive.

        This is a more direct test of the async iterator pattern.
        Creates a mock scenario where we test concurrent execution.
        """
        completed_tasks = []

        async def monitor_task(task_id):
            """A task that monitors concurrent execution."""
            for _ in range(3):
                await asyncio.sleep(0.05)
                completed_tasks.append(task_id)

        # Local server as above (#66).
        with LoopbackServer() as srv:
            ws = product_ws(srv.url, "stock", api_key=mock_api_key)
            try:
                # Run WebSocket task alongside monitor tasks
                await asyncio.gather(
                    ws.connect_async(),
                    monitor_task("monitor_1"),
                    monitor_task("monitor_2"),
                )
            finally:
                disconnect_quietly(ws)

        # Monitor tasks should complete (at least some iterations)
        # If GIL was held, monitors would be blocked
        assert len(completed_tasks) >= 3, f"Only {len(completed_tasks)} monitor iterations completed"

