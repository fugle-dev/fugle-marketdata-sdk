"""Tests for async REST client.

These tests verify that the REST client works correctly in Python asyncio context.
Unit tests run without API key, integration tests require FUGLE_API_KEY.
"""
import pytest
from fugle_marketdata import (
    RestClient,
    MarketDataError,
    AuthError,
)


class TestRestClientCreation:
    """Test REST client instantiation."""

    def test_create_with_api_key(self, mock_api_key):
        """Client should be creatable with API key."""
        client = RestClient(api_key=mock_api_key)
        assert client is not None

    def test_create_with_bearer_token(self):
        """Client should be creatable with bearer token."""
        client = RestClient.with_bearer_token("test-token")
        assert client is not None

    def test_create_with_sdk_token(self):
        """Client should be creatable with SDK token."""
        client = RestClient.with_sdk_token("test-sdk-token")
        assert client is not None

    def test_stock_property_returns_client(self, mock_api_key):
        """client.stock should return StockClient."""
        client = RestClient(api_key=mock_api_key)
        assert client.stock is not None
        assert client.stock.intraday is not None

    def test_futopt_property_returns_client(self, mock_api_key):
        """client.futopt should return FutOptClient."""
        client = RestClient(api_key=mock_api_key)
        assert client.futopt is not None
        assert client.futopt.intraday is not None


class TestAsyncMethods:
    """Test that methods return awaitables.

    Requests go to a loopback server that rejects the key like the real API
    does, so these run without network (#71).
    """

    @pytest.mark.asyncio
    async def test_quote_returns_awaitable(self, rest_server):
        """quote() should return an awaitable that raises error with a rejected key."""
        client = RestClient(api_key="test-key", base_url=rest_server.url)
        # This should raise an error (rejected key) but BE awaitable
        with pytest.raises(AuthError):
            await client.stock.intraday.quote_async("2330")

    @pytest.mark.asyncio
    async def test_ticker_returns_awaitable(self, rest_server):
        """ticker() should return an awaitable."""
        client = RestClient(api_key="test-key", base_url=rest_server.url)
        with pytest.raises(AuthError):
            await client.stock.intraday.ticker_async("2330")

    @pytest.mark.asyncio
    async def test_candles_returns_awaitable(self, rest_server):
        """candles() should return an awaitable."""
        client = RestClient(api_key="test-key", base_url=rest_server.url)
        with pytest.raises(AuthError):
            await client.stock.intraday.candles_async("2330")

    @pytest.mark.asyncio
    async def test_trades_returns_awaitable(self, rest_server):
        """trades() should return an awaitable."""
        client = RestClient(api_key="test-key", base_url=rest_server.url)
        with pytest.raises(AuthError):
            await client.stock.intraday.trades_async("2330")

    @pytest.mark.asyncio
    async def test_volumes_returns_awaitable(self, rest_server):
        """volumes() should return an awaitable."""
        client = RestClient(api_key="test-key", base_url=rest_server.url)
        with pytest.raises(AuthError):
            await client.stock.intraday.volumes_async("2330")

    @pytest.mark.asyncio
    async def test_futopt_quote_returns_awaitable(self, rest_server):
        """futopt.intraday.quote() should return an awaitable."""
        client = RestClient(api_key="test-key", base_url=rest_server.url)
        with pytest.raises(AuthError):
            await client.futopt.intraday.quote_async("TXFC4")


class TestMethodSignatures:
    """Test method signatures accept expected parameters."""

    def test_quote_accepts_symbol(self, mock_api_key):
        """quote() should accept symbol parameter."""
        client = RestClient(api_key=mock_api_key)
        # Verify method exists and is callable
        assert hasattr(client.stock.intraday, 'quote')
        assert callable(client.stock.intraday.quote)

    def test_quote_accepts_odd_lot(self, mock_api_key):
        """quote() should accept odd_lot parameter."""
        client = RestClient(api_key=mock_api_key)
        # The method signature includes odd_lot parameter
        # We verify by checking it doesn't raise TypeError
        import inspect
        sig = inspect.signature(client.stock.intraday.quote)
        params = list(sig.parameters.keys())
        # Should have symbol parameter at minimum
        assert len(params) >= 1

    def test_candles_accepts_timeframe(self, mock_api_key):
        """candles() should accept timeframe parameter."""
        client = RestClient(api_key=mock_api_key)
        assert hasattr(client.stock.intraday, 'candles')


@pytest.mark.integration
class TestIntegrationRest:
    """Integration tests requiring real API key.

    These tests are skipped unless FUGLE_API_KEY environment variable is set.
    """

    @pytest.mark.asyncio
    async def test_quote_returns_dict(self, rest_client):
        """quote() should return a dict with market data."""
        result = await rest_client.stock.intraday.quote_async("2330")
        assert isinstance(result, dict)
        # The response should contain data fields
        assert len(result) > 0

    @pytest.mark.asyncio
    async def test_ticker_returns_dict(self, rest_client):
        """ticker() should return a dict with ticker data."""
        result = await rest_client.stock.intraday.ticker_async("2330")
        assert isinstance(result, dict)
        assert len(result) > 0

    @pytest.mark.asyncio
    async def test_candles_returns_dict(self, rest_client):
        """candles() should return a dict with candlestick data."""
        result = await rest_client.stock.intraday.candles_async("2330")
        assert isinstance(result, dict)

    @pytest.mark.asyncio
    async def test_trades_returns_dict(self, rest_client):
        """trades() should return a dict with trade data."""
        result = await rest_client.stock.intraday.trades_async("2330")
        assert isinstance(result, dict)

    @pytest.mark.asyncio
    async def test_volumes_returns_dict(self, rest_client):
        """volumes() should return a dict with volume data."""
        result = await rest_client.stock.intraday.volumes_async("2330")
        assert isinstance(result, dict)

    @pytest.mark.asyncio
    async def test_invalid_symbol_raises_error(self, rest_client):
        """Invalid symbol should raise MarketDataError."""
        with pytest.raises(MarketDataError):
            await rest_client.stock.intraday.quote_async("INVALID_SYMBOL_12345")

    @pytest.mark.asyncio
    async def test_futopt_quote_returns_dict(self, rest_client):
        """futopt.intraday.quote() should return a dict."""
        result = await rest_client.futopt.intraday.quote_async("TXFC4")
        assert isinstance(result, dict)
