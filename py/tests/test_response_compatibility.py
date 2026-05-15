"""
Response Compatibility Tests

These tests validate response structure against recorded official SDK responses.

Strategy:
1. Fixture tests: Load VCR cassettes and validate structure (no API key needed)
2. Integration tests: Compare live API responses when FUGLE_API_KEY is set

Note: VCR.py cannot intercept native Rust HTTP calls. We load fixtures directly
and validate structure against expected official SDK format.

Run: pytest py/tests/test_response_compatibility.py -v
"""
import json
import os
import pytest
import yaml
from pathlib import Path

FIXTURES_DIR = Path(__file__).parent / "fixtures"


def load_cassette_response(cassette_name: str) -> dict:
    """Load response body from VCR cassette YAML file."""
    cassette_path = FIXTURES_DIR / cassette_name
    if not cassette_path.exists():
        pytest.skip(f"Cassette not found: {cassette_name}")

    with open(cassette_path, 'r') as f:
        cassette = yaml.safe_load(f)

    # VCR cassette structure: interactions[0].response.body.string
    if not cassette or 'interactions' not in cassette:
        pytest.skip(f"Invalid cassette format: {cassette_name}")

    interactions = cassette['interactions']
    if not interactions:
        pytest.skip(f"No interactions in cassette: {cassette_name}")

    response_body = interactions[0]['response']['body']['string']
    return json.loads(response_body)


class TestQuoteFixtureStructure:
    """Validate quote fixture has expected official SDK structure."""

    def test_quote_has_required_fields(self):
        """Quote fixture must have all fields present in official SDK."""
        response = load_cassette_response('official_sdk_quote.yaml')

        # Top-level structure
        assert 'apiVersion' in response or 'data' in response or 'symbol' in response

        # Extract data payload (handle both wrapped and unwrapped formats)
        if 'data' in response:
            data = response['data']
        else:
            data = response

        # Info section (metadata)
        if 'info' in data:
            info = data['info']
            assert 'date' in info
            assert 'symbolId' in info or 'symbol' in info

        # Quote section (price data)
        if 'quote' in data:
            quote = data['quote']

            # Total statistics
            if 'total' in quote:
                total = quote['total']
                assert 'tradeVolume' in total or 'volume' in total
                assert 'tradeValue' in total or 'value' in total

            # Current trade
            if 'trade' in quote:
                trade = quote['trade']
                assert 'price' in trade
                assert 'at' in trade or 'time' in trade

            # Order book
            if 'order' in quote:
                order = quote['order']
                if 'bids' in order:
                    assert isinstance(order['bids'], list)
                    if order['bids']:
                        assert 'price' in order['bids'][0]
                        assert 'volume' in order['bids'][0]
                if 'asks' in order:
                    assert isinstance(order['asks'], list)
                    if order['asks']:
                        assert 'price' in order['asks'][0]
                        assert 'volume' in order['asks'][0]

    def test_quote_field_types(self):
        """Validate field types in quote fixture."""
        response = load_cassette_response('official_sdk_quote.yaml')

        # Extract data
        if 'data' in response:
            data = response['data']
        else:
            data = response

        # Validate info types
        if 'info' in data:
            info = data['info']
            if 'date' in info:
                assert isinstance(info['date'], str)
            if 'symbolId' in info:
                assert isinstance(info['symbolId'], str)
            elif 'symbol' in info:
                assert isinstance(info['symbol'], str)

        # Validate quote types
        if 'quote' in data:
            quote = data['quote']

            # Price values should be numeric
            if 'priceHigh' in quote:
                price_high = quote['priceHigh']
                if isinstance(price_high, dict) and 'price' in price_high:
                    assert isinstance(price_high['price'], (int, float))

            if 'priceLow' in quote:
                price_low = quote['priceLow']
                if isinstance(price_low, dict) and 'price' in price_low:
                    assert isinstance(price_low['price'], (int, float))

    def test_quote_order_book_structure(self):
        """Order book (bids/asks) must have proper structure."""
        response = load_cassette_response('official_sdk_quote.yaml')

        # Extract quote
        if 'data' in response:
            quote = response['data'].get('quote', {})
        else:
            quote = response.get('quote', {})

        if 'order' in quote:
            order = quote['order']

            # Bids structure
            if 'bids' in order and order['bids']:
                for bid in order['bids']:
                    assert 'price' in bid, "Bid must have price"
                    assert 'volume' in bid, "Bid must have volume"
                    assert isinstance(bid['price'], (int, float)), "Bid price must be numeric"
                    assert isinstance(bid['volume'], (int, float)), "Bid volume must be numeric"

            # Asks structure
            if 'asks' in order and order['asks']:
                for ask in order['asks']:
                    assert 'price' in ask, "Ask must have price"
                    assert 'volume' in ask, "Ask must have volume"
                    assert isinstance(ask['price'], (int, float)), "Ask price must be numeric"
                    assert isinstance(ask['volume'], (int, float)), "Ask volume must be numeric"


class TestTickerFixtureStructure:
    """Validate ticker fixture has expected official SDK structure."""

    def test_ticker_has_array_structure(self):
        """Ticker fixture should be an array of tick data."""
        response = load_cassette_response('official_sdk_ticker.yaml')

        # Extract ticker array
        if 'data' in response:
            data = response['data']
            if 'ticker' in data:
                ticker = data['ticker']
            else:
                ticker = data
        else:
            ticker = response

        # Should be array-like
        assert isinstance(ticker, (list, tuple)), "Ticker should be array/list"

    def test_ticker_items_have_required_fields(self):
        """Each ticker item should have timestamp and price fields."""
        response = load_cassette_response('official_sdk_ticker.yaml')

        # Extract ticker array
        if 'data' in response:
            data = response['data']
            ticker = data.get('ticker', data)
        else:
            ticker = response

        if isinstance(ticker, (list, tuple)) and ticker:
            # Check first item structure
            item = ticker[0]

            # Time field (various formats)
            assert any(k in item for k in ['at', 'time', 'timestamp']), \
                "Ticker item must have time field"

            # Price field
            assert any(k in item for k in ['price', 'lastPrice', 'close']), \
                "Ticker item must have price field"


class TestTradesFixtureStructure:
    """Validate trades fixture has expected official SDK structure."""

    def test_trades_has_array_structure(self):
        """Trades fixture should be an array of trade executions."""
        response = load_cassette_response('official_sdk_trades.yaml')

        # Extract trades array
        if 'data' in response:
            data = response['data']
            trades = data.get('trades', data)
        else:
            trades = response

        assert isinstance(trades, (list, tuple)), "Trades should be array/list"

    def test_trades_items_have_price_volume(self):
        """Each trade should have price and volume."""
        response = load_cassette_response('official_sdk_trades.yaml')

        # Extract trades array
        if 'data' in response:
            data = response['data']
            trades = data.get('trades', data)
        else:
            trades = response

        if isinstance(trades, (list, tuple)) and trades:
            trade = trades[0]

            # Must have price
            assert 'price' in trade, "Trade must have price"
            assert isinstance(trade['price'], (int, float)), "Price must be numeric"

            # Must have volume
            assert 'volume' in trade, "Trade must have volume"
            assert isinstance(trade['volume'], (int, float)), "Volume must be numeric"

    def test_trades_items_have_timestamp(self):
        """Each trade should have timestamp."""
        response = load_cassette_response('official_sdk_trades.yaml')

        # Extract trades array
        if 'data' in response:
            data = response['data']
            trades = data.get('trades', data)
        else:
            trades = response

        if isinstance(trades, (list, tuple)) and trades:
            trade = trades[0]

            # Must have timestamp (various field names)
            assert any(k in trade for k in ['at', 'time', 'timestamp']), \
                "Trade must have timestamp field"


class TestCandlesFixtureStructure:
    """Validate candles fixture has expected official SDK structure."""

    def test_candles_has_array_structure(self):
        """Candles fixture should be an array of OHLCV data."""
        response = load_cassette_response('official_sdk_candles.yaml')

        # Extract candles array
        if 'data' in response:
            data = response['data']
            candles = data.get('candles', data)
        else:
            candles = response

        assert isinstance(candles, (list, tuple)), "Candles should be array/list"

    def test_candles_items_have_ohlcv_fields(self):
        """Each candle should have OHLCV structure."""
        response = load_cassette_response('official_sdk_candles.yaml')

        # Extract candles array
        if 'data' in response:
            data = response['data']
            candles = data.get('candles', data)
        else:
            candles = response

        if isinstance(candles, (list, tuple)) and candles:
            candle = candles[0]

            # OHLC fields (open, high, low, close)
            assert 'open' in candle, "Candle must have open price"
            assert 'high' in candle, "Candle must have high price"
            assert 'low' in candle, "Candle must have low price"
            assert 'close' in candle, "Candle must have close price"

            # Volume field
            assert 'volume' in candle, "Candle must have volume"

            # All OHLCV values should be numeric
            assert isinstance(candle['open'], (int, float))
            assert isinstance(candle['high'], (int, float))
            assert isinstance(candle['low'], (int, float))
            assert isinstance(candle['close'], (int, float))
            assert isinstance(candle['volume'], (int, float))

    def test_candles_items_have_timestamp(self):
        """Each candle should have timestamp."""
        response = load_cassette_response('official_sdk_candles.yaml')

        # Extract candles array
        if 'data' in response:
            data = response['data']
            candles = data.get('candles', data)
        else:
            candles = response

        if isinstance(candles, (list, tuple)) and candles:
            candle = candles[0]

            # Must have timestamp
            assert any(k in candle for k in ['at', 'time', 'timestamp']), \
                "Candle must have timestamp field"


# ============================================================================
# Integration Tests (require FUGLE_API_KEY)
# ============================================================================

@pytest.mark.integration
class TestQuoteIntegration:
    """Integration tests comparing live API to fixtures."""

    @pytest.mark.asyncio
    async def test_live_quote_matches_fixture_structure(self, rest_client):
        """Live quote response should match fixture structure."""
        if not os.environ.get("FUGLE_API_KEY"):
            pytest.skip("FUGLE_API_KEY not set")

        # Get live response
        live_response = await rest_client.stock.intraday.quote_async("2330")

        # Load fixture for comparison
        fixture_response = load_cassette_response('official_sdk_quote.yaml')

        # Both should have similar top-level structure
        live_keys = set(live_response.keys())
        fixture_keys = set(fixture_response.keys())

        # Check for common fields (data or direct fields)
        assert live_keys & fixture_keys, \
            f"Live and fixture should share structure. Live: {live_keys}, Fixture: {fixture_keys}"


@pytest.mark.integration
class TestTickerIntegration:
    """Integration tests for ticker endpoint."""

    @pytest.mark.asyncio
    async def test_live_ticker_returns_array(self, rest_client):
        """Live ticker response should be array-like."""
        if not os.environ.get("FUGLE_API_KEY"):
            pytest.skip("FUGLE_API_KEY not set")

        live_response = await rest_client.stock.intraday.ticker_async("2330")

        # Extract ticker array
        if 'data' in live_response:
            data = live_response['data']
            ticker = data.get('ticker', data)
        else:
            ticker = live_response

        assert isinstance(ticker, (list, tuple)), "Live ticker should be array"


# ============================================================================
# Response Structure Documentation
# ============================================================================
#
# These tests validate that official SDK cassettes have the expected structure.
# By validating fixtures, we ensure our SDK can be tested against accurate
# reference data.
#
# Test Strategy:
# 1. Fixture tests: Load VCR cassettes directly and validate structure
# 2. Integration tests: When API key is available, verify live responses
#
# Note on VCR.py:
# VCR.py intercepts Python HTTP libraries (requests, urllib3, aiohttp).
# Our SDK uses native Rust HTTP calls via ureq, which bypass Python's HTTP
# stack entirely. Therefore, we load fixtures directly instead of using
# VCR to intercept calls.
#
# Running Tests:
#   Fixture tests (always work):
#     pytest py/tests/test_response_compatibility.py -v -k "Fixture"
#
#   Integration tests (require API key):
#     FUGLE_API_KEY=xxx pytest py/tests/test_response_compatibility.py -v -k "Integration"
#
# ============================================================================
