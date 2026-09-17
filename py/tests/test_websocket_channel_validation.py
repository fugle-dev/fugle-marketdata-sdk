"""subscribe() rejects an unknown channel name with 1005 INVALID_PARAMETER (#114).

The name is checked before the connection, so no server is needed: an unknown
channel is 1005 whether or not the client is connected, and a known one (in
any case) gets past the check to "Not connected".
"""
import pytest
from fugle_marketdata import MarketDataError, WebSocketClient

STOCK_CHANNELS = "trades, candles, books, aggregates, indices"
FUTOPT_CHANNELS = "trades, candles, books, aggregates"


def unknown_channel(name, valid):
    return f"Invalid parameter 'channel': unknown channel '{name}'. Valid channels: {valid}"


def assert_invalid_channel(excinfo, name, valid):
    assert excinfo.value.code == 1005
    assert excinfo.value.source_kind == "client"
    assert excinfo.value.message == unknown_channel(name, valid)


@pytest.fixture
def ws(mock_api_key):
    return WebSocketClient(api_key=mock_api_key)


@pytest.mark.parametrize(
    "args",
    [
        pytest.param(("trade", "2330"), id="positional"),
        pytest.param(({"channel": "trade", "symbol": "2330"},), id="dict"),
    ],
)
def test_stock_subscribe_rejects_unknown_channel(ws, args):
    with pytest.raises(MarketDataError) as excinfo:
        ws.stock.subscribe(*args)
    assert_invalid_channel(excinfo, "trade", STOCK_CHANNELS)


def test_futopt_subscribe_rejects_indices(ws):
    with pytest.raises(MarketDataError) as excinfo:
        ws.futopt.subscribe({"channel": "indices", "symbol": "TXFC4"})
    assert_invalid_channel(excinfo, "indices", FUTOPT_CHANNELS)


@pytest.mark.asyncio
async def test_stock_subscribe_async_raises_on_await(ws):
    # Calling it does not raise; the error comes with the awaitable.
    awaitable = ws.stock.subscribe_async("trade", "2330")
    with pytest.raises(MarketDataError) as excinfo:
        await awaitable
    assert_invalid_channel(excinfo, "trade", STOCK_CHANNELS)


def test_known_channel_in_any_case_reaches_connection_check(ws):
    with pytest.raises(RuntimeError, match="Not connected"):
        ws.stock.subscribe("Trades", "2330")
    with pytest.raises(RuntimeError, match="Not connected"):
        ws.futopt.subscribe("BOOKS", "TXFC4")


@pytest.mark.asyncio
async def test_known_channel_async_reaches_connection_check(ws):
    with pytest.raises(RuntimeError, match="Not connected"):
        await ws.stock.subscribe_async("Candles", "2330")
