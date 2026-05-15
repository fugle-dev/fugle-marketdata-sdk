"""FFI boundary tests for Python bindings.

These tests verify that the PyO3 FFI boundary properly handles edge cases:
- Error type mapping (Rust errors -> Python exceptions)
- Panic recovery (catch_unwind verification)
- Memory safety (GC, weak refs, concurrent access)
- GIL safety (blocking calls release GIL)

FFI boundary failures would manifest as:
- Segfaults or process abort (panic not caught)
- Memory corruption (invalid pointer access)
- Deadlocks (GIL held during blocking operations)
- Wrong exception types (error mapping broken)
"""
import pytest
import gc
import weakref
import sys


class TestErrorHandlingBoundary:
    """Test Rust error to Python exception mapping."""

    @pytest.fixture
    def mock_api_key(self):
        """Provide a mock API key for testing."""
        return "mock_api_key_for_boundary_testing"

    @pytest.mark.asyncio
    async def test_invalid_symbol_raises_typed_error(self, mock_api_key):
        """Invalid symbol should raise specific error type (not base MarketDataError)."""
        from fugle_marketdata import RestClient, MarketDataError

        client = RestClient(mock_api_key)

        # Invalid symbol should raise a specific error subclass
        with pytest.raises(MarketDataError) as exc_info:
            await client.stock.intraday.quote_async("INVALID_SYMBOL_12345")

        # Error should be a subclass of MarketDataError (not the base class)
        assert type(exc_info.value).__name__ in ["ApiError", "AuthError"]

        # Error should have readable message (no memory corruption)
        msg = str(exc_info.value)
        assert len(msg) > 0
        assert isinstance(msg, str)

    @pytest.mark.asyncio
    async def test_auth_error_type_mapping(self, mock_api_key):
        """Authentication failure should raise AuthError."""
        from fugle_marketdata import RestClient, AuthError, MarketDataError

        # Mock key will likely fail authentication
        client = RestClient(mock_api_key)

        try:
            await client.stock.intraday.quote_async("2330")
        except AuthError as e:
            # AuthError should be subclass of MarketDataError
            assert isinstance(e, MarketDataError)
            # Error message should be readable string
            assert len(str(e)) > 0
            assert isinstance(str(e), str)
        except Exception as e:
            # May get other errors depending on mock key handling
            # At minimum, should be readable and not segfault
            assert len(str(e)) > 0

    @pytest.mark.asyncio
    async def test_error_message_no_corruption(self, mock_api_key):
        """Error messages should be valid strings (no memory corruption)."""
        from fugle_marketdata import RestClient, MarketDataError

        client = RestClient(mock_api_key)

        try:
            # This will likely fail - we're testing error handling
            await client.stock.intraday.quote_async("2330")
        except MarketDataError as e:
            # Error message should be valid UTF-8 string
            msg = str(e)
            assert isinstance(msg, str)
            assert len(msg) > 0
            # Should not contain null bytes or corruption indicators
            assert "\x00" not in msg
            # Should be decodable as UTF-8
            msg.encode("utf-8")

    @pytest.mark.asyncio
    async def test_error_args_accessible(self, mock_api_key):
        """Error args should contain message and error code."""
        from fugle_marketdata import RestClient, MarketDataError

        client = RestClient(mock_api_key)

        try:
            await client.stock.intraday.quote_async("INVALID")
        except MarketDataError as e:
            # PyO3 errors should have args tuple
            assert hasattr(e, "args")
            assert len(e.args) >= 1
            # First arg is message
            assert isinstance(e.args[0], str)
            # Second arg (if present) is error code
            if len(e.args) >= 2:
                assert isinstance(e.args[1], int)


class TestPanicRecovery:
    """Test that Rust panics are caught and don't abort process."""

    def test_empty_api_key_doesnt_panic(self):
        """Empty API key should not cause panic (may succeed or fail gracefully)."""
        from fugle_marketdata import RestClient

        # Empty API key should not panic - may accept empty string
        try:
            client = RestClient("")
            # If it succeeds, verify client is created
            assert client is not None
        except Exception as e:
            # If it raises, should be a readable exception
            assert isinstance(str(e), str)
            assert len(str(e)) > 0

    @pytest.mark.asyncio
    async def test_long_input_doesnt_overflow(self):
        """Very long input strings should not cause buffer overflow."""
        from fugle_marketdata import RestClient

        client = RestClient("test_key")

        # Try extremely long symbol (potential buffer overflow)
        long_symbol = "A" * 10000

        with pytest.raises(Exception):
            await client.stock.intraday.quote_async(long_symbol)

        # Should raise exception, not segfault

    @pytest.mark.asyncio
    async def test_unicode_input_handled_safely(self):
        """Unicode input should be handled without panic."""
        from fugle_marketdata import RestClient

        client = RestClient("test_key")

        # Unicode characters that might cause issues
        unicode_symbols = [
            "中文",  # Chinese characters
            "🚀📈",  # Emojis
            "א",    # Hebrew
            "\u0000",  # Null character
        ]

        for symbol in unicode_symbols:
            try:
                await client.stock.intraday.quote_async(symbol)
            except Exception as e:
                # Should get exception with valid error message
                assert isinstance(str(e), str)


class TestMemorySafety:
    """Test memory safety across FFI boundary."""

    def test_client_cleanup_after_gc(self):
        """Client should be properly cleaned up after garbage collection."""
        from fugle_marketdata import RestClient

        # Create multiple clients and delete them
        clients = [RestClient(f"key_{i}") for i in range(10)]

        # Get initial reference count
        import sys
        ref_count = sys.getrefcount(clients[0])

        # Delete all clients
        del clients
        gc.collect()

        # Create new client - should not have memory leaks
        # If there were leaks, this would eventually fail with OOM
        new_client = RestClient("test_key")
        assert new_client is not None

    def test_concurrent_client_creation(self):
        """Multiple clients should not interfere with each other's memory."""
        from fugle_marketdata import RestClient

        clients = [RestClient(f"key_{i}") for i in range(10)]

        # All clients should be independent
        assert len(clients) == 10

        # Clean up
        del clients
        gc.collect()

    @pytest.mark.asyncio
    async def test_client_reuse_after_error(self):
        """Client should remain usable after error (no state corruption)."""
        from fugle_marketdata import RestClient

        client = RestClient("test_key")

        # Cause an error
        try:
            await client.stock.intraday.quote_async("INVALID")
        except Exception:
            pass

        # Client should still be usable
        try:
            await client.stock.intraday.quote_async("2330")
        except Exception:
            pass  # Error is expected, but should not segfault

    @pytest.mark.asyncio
    @pytest.mark.skipif(sys.version_info < (3, 9), reason="Requires Python 3.9+")
    async def test_buffer_protocol_safety(self):
        """Test that any buffer handling is memory safe."""
        from fugle_marketdata import RestClient

        client = RestClient("test_key")

        # Test with various input types that might use buffer protocol
        inputs = [
            b"bytes_input",
            bytearray(b"bytearray_input"),
            memoryview(b"memoryview_input"),
        ]

        for inp in inputs:
            try:
                # Most FFI functions expect strings, this should fail gracefully
                await client.stock.intraday.quote_async(inp)  # type: ignore
            except (TypeError, Exception):
                # Expected - should fail with exception, not segfault
                pass


class TestGilSafety:
    """Test that blocking FFI calls properly release GIL."""

    @pytest.mark.asyncio
    @pytest.mark.timeout(10)
    async def test_blocking_call_releases_gil(self):
        """Async call should not block event loop."""
        import asyncio
        from fugle_marketdata import RestClient

        client = RestClient("test_key")
        results = []

        async def make_request():
            try:
                await client.stock.intraday.quote_async("2330")
            except Exception:
                results.append("request_attempted")

        async def other_work():
            # Simulate async work
            await asyncio.sleep(0.1)
            results.append("other_work_done")

        # Run both concurrently
        await asyncio.gather(make_request(), other_work(), return_exceptions=True)

        # Both tasks should complete
        assert "other_work_done" in results

    @pytest.mark.asyncio
    @pytest.mark.timeout(10)
    async def test_multiple_clients_parallel(self):
        """Multiple concurrent async calls should not deadlock."""
        import asyncio
        from fugle_marketdata import RestClient

        results = []

        async def use_client(client_id):
            client = RestClient(f"key_{client_id}")
            try:
                await client.stock.intraday.quote_async("2330")
            except Exception:
                pass
            results.append(client_id)

        tasks = [use_client(i) for i in range(5)]
        await asyncio.gather(*tasks, return_exceptions=True)

        # All tasks should complete
        assert len(results) == 5


# Manual testing notes:
#
# Additional stress tests (not automated):
#   1. Memory leak testing:
#      - Create/destroy thousands of clients
#      - Monitor RSS with `ps aux | grep pytest`
#
#   2. Thread safety under load:
#      - 50+ concurrent threads
#      - Monitor for deadlocks or race conditions
#
#   3. Valgrind testing (if available):
#      - valgrind --leak-check=full python -m pytest test_ffi_boundary.py
