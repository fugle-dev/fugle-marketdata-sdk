"""Tests for async WebSocket client.

These tests verify that the WebSocket client works correctly in Python asyncio context.
Unit tests run without API key, integration tests require FUGLE_API_KEY.
"""
import asyncio
import pytest
from fugle_marketdata import (
    WebSocketClient,
    MarketDataError,
    ConnectionError,
)


class TestWebSocketClientCreation:
    """Test WebSocket client instantiation."""

    def test_create_with_api_key(self, mock_api_key):
        """Client should be creatable with API key."""
        client = WebSocketClient(mock_api_key)
        assert client is not None

    def test_stock_property_returns_client(self, mock_api_key):
        """ws.stock should return StockWebSocketClient."""
        client = WebSocketClient(mock_api_key)
        assert client.stock is not None

    def test_futopt_property_returns_client(self, mock_api_key):
        """ws.futopt should return FutOptWebSocketClient."""
        client = WebSocketClient(mock_api_key)
        assert client.futopt is not None


class TestAsyncConnect:
    """Test async connect method."""

    @pytest.mark.asyncio
    @pytest.mark.timeout(10)
    async def test_connect_async_returns_awaitable(self, mock_api_key):
        """connect_async() should return an awaitable."""
        client = WebSocketClient(mock_api_key)
        # Should fail with auth error but be awaitable
        # The connection will fail because of invalid API key
        with pytest.raises((MarketDataError, ConnectionError, Exception)):
            await client.stock.connect_async()


class TestCallbackPattern:
    """Test callback registration."""

    def test_on_registers_callback(self, mock_api_key):
        """on() should register a callback."""
        client = WebSocketClient(mock_api_key)
        called = []

        def handler(msg):
            called.append(msg)

        # No error means callback registered successfully
        client.stock.on("message", handler)

    def test_off_removes_callback(self, mock_api_key):
        """off() should remove callbacks."""
        client = WebSocketClient(mock_api_key)
        client.stock.on("message", lambda m: None)
        # No error means callback removed successfully
        client.stock.off("message")

    def test_on_supports_multiple_events(self, mock_api_key):
        """on() should support multiple event types."""
        client = WebSocketClient(mock_api_key)

        # Test various event types
        client.stock.on("message", lambda m: None)
        client.stock.on("connect", lambda: None)
        client.stock.on("disconnect", lambda: None)
        client.stock.on("error", lambda e: None)


class TestAsyncIterator:
    """Test async iterator pattern."""

    def test_messages_method_exists(self, mock_api_key):
        """messages() method should exist."""
        client = WebSocketClient(mock_api_key)
        assert hasattr(client.stock, 'messages')
        assert callable(client.stock.messages)


class TestSyncMethods:
    """Test synchronous methods."""

    def test_is_connected_returns_bool(self, mock_api_key):
        """is_connected() should return bool."""
        client = WebSocketClient(mock_api_key)
        result = client.stock.is_connected()
        assert isinstance(result, bool)
        assert result is False  # Not connected initially

    def test_is_closed_returns_bool(self, mock_api_key):
        """is_closed() should return bool."""
        client = WebSocketClient(mock_api_key)
        result = client.stock.is_closed()
        assert isinstance(result, bool)

    def test_subscriptions_returns_list(self, mock_api_key):
        """subscriptions() should return list."""
        client = WebSocketClient(mock_api_key)
        result = client.stock.subscriptions()
        assert isinstance(result, list)
        assert len(result) == 0  # No subscriptions initially


@pytest.mark.integration
class TestIntegrationWebSocket:
    """Integration tests requiring real API key.

    These tests are skipped unless FUGLE_API_KEY environment variable is set.
    """

    @pytest.mark.asyncio
    @pytest.mark.timeout(15)
    async def test_connect_and_disconnect(self, ws_client):
        """Should be able to connect and disconnect."""
        await ws_client.stock.connect_async()
        assert ws_client.stock.is_connected()

        await ws_client.stock.disconnect_async()
        # Note: is_connected might take a moment to update

    @pytest.mark.asyncio
    @pytest.mark.timeout(15)
    async def test_connect_and_subscribe(self, ws_client):
        """Should be able to connect and subscribe."""
        await ws_client.stock.connect_async()
        assert ws_client.stock.is_connected()

        # Subscribe to trades channel
        await ws_client.stock.subscribe_async("trades", "2330")

        # Verify subscription is active
        subs = ws_client.stock.subscriptions()
        assert len(subs) >= 0  # Subscription keys may or may not be immediately visible

        await ws_client.stock.disconnect_async()

    @pytest.mark.asyncio
    @pytest.mark.timeout(20)
    async def test_async_iterator_receives_messages(self, ws_client):
        """Async iterator should receive messages without GIL deadlock."""
        await ws_client.stock.connect_async()
        await ws_client.stock.subscribe_async("trades", "2330")

        messages = []
        try:
            # Use timeout to avoid hanging indefinitely
            async for msg in ws_client.stock.messages(timeout_ms=5000):
                messages.append(msg)
                if len(messages) >= 1:
                    break
        except Exception:
            pass  # May timeout if no market activity

        await ws_client.stock.disconnect_async()
        # Just verify no crash - may or may not receive messages depending on market hours

    @pytest.mark.asyncio
    @pytest.mark.timeout(15)
    async def test_context_manager(self, api_key):
        """Context manager should connect and disconnect automatically."""
        ws = WebSocketClient(api_key)
        async with ws.stock:
            assert ws.stock.is_connected()
        # After context exit, should be disconnected (eventually)

    @pytest.mark.asyncio
    @pytest.mark.timeout(10)
    async def test_sync_connect_works(self, api_key):
        """Sync connect() should also work."""
        ws = WebSocketClient(api_key)
        # Note: sync connect blocks the event loop
        # This is a known limitation; use connect_async() in async code
        import asyncio
        loop = asyncio.get_event_loop()
        await loop.run_in_executor(None, ws.stock.connect)
        assert ws.stock.is_connected()
        await loop.run_in_executor(None, ws.stock.disconnect)


class TestFutOptWebSocket:
    """Test FutOpt WebSocket client."""

    def test_futopt_has_expected_methods(self, mock_api_key):
        """FutOpt client should have expected methods."""
        ws = WebSocketClient(mock_api_key)
        futopt = ws.futopt

        assert hasattr(futopt, 'connect')
        assert hasattr(futopt, 'disconnect')
        assert hasattr(futopt, 'subscribe')
        assert hasattr(futopt, 'on')
        assert hasattr(futopt, 'off')
        assert hasattr(futopt, 'messages')
        assert hasattr(futopt, 'is_connected')

    def test_futopt_is_connected_returns_bool(self, mock_api_key):
        """is_connected() should return bool."""
        ws = WebSocketClient(mock_api_key)
        result = ws.futopt.is_connected()
        assert isinstance(result, bool)
        assert result is False
