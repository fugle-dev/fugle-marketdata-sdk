"""unsubscribe() sends the id the server issued (#136).

Besides the server id (positional, ``ids=``, ``{"id"}`` / ``{"ids"}``), it
takes the arguments given to ``subscribe()``: a dict with ``channel`` and
``symbol`` / ``symbols``, or ``channel=`` with ``symbol=`` / ``symbols=``,
plus the product's modifier. Either way the frame carries the id from the
``subscribed`` ack. Naming a channel together with an id is 1005.
"""
import time

import pytest

from fugle_marketdata import MarketDataError, WebSocketClient
from tests.ws_loopback import InProcessLoopbackServer, disconnect_quietly, product_ws

hard_timeout = pytest.mark.timeout(20, method="thread")

# product, dict modifier key, kwarg modifier, id suffix
PRODUCTS = [
    pytest.param("stock", "oddLot", "odd_lot", "odd", id="stock"),
    pytest.param("futopt", "afterHours", "after_hours", "ah", id="futopt"),
]


def wait_for_unsubscribes(srv, count, timeout=5):
    deadline = time.monotonic() + timeout
    while len(srv.unsubscribe_data) < count and time.monotonic() < deadline:
        time.sleep(0.02)
    return srv.unsubscribe_data


@hard_timeout
@pytest.mark.parametrize("product, key, kwarg, suffix", PRODUCTS)
def test_unsubscribe_by_subscribe_arguments_sends_server_id(product, key, kwarg, suffix):
    with InProcessLoopbackServer() as srv:
        ws = product_ws(srv.url, product)
        try:
            ws.connect()
            ws.subscribe("trades", "2330")
            ws.subscribe("books", "2330")
            ws.subscribe("candles", "2330", **{kwarg: True})
            ws.subscribe("aggregates", "2330")

            ws.unsubscribe({"channel": "trades", "symbol": "2330"})
            ws.unsubscribe(channel="books", symbols=["2330"])
            ws.unsubscribe({"channel": "candles", "symbol": "2330", key: True})
            ws.unsubscribe(channel="aggregates", symbol="2330", **{kwarg: False})

            assert wait_for_unsubscribes(srv, 4) == [
                {"id": "trades-2330"},
                {"id": "books-2330"},
                {"id": f"candles-2330-{suffix}"},
                {"id": "aggregates-2330"},
            ]
        finally:
            disconnect_quietly(ws)


@hard_timeout
@pytest.mark.parametrize("product", ["stock", "futopt"])
def test_unsubscribe_by_server_id_sends_it(product):
    with InProcessLoopbackServer() as srv:
        ws = product_ws(srv.url, product)
        try:
            ws.connect()
            ws.unsubscribe("id-a")
            ws.unsubscribe(ids=["id-b", "id-c"])
            ws.unsubscribe({"id": "id-d"})

            assert wait_for_unsubscribes(srv, 3) == [
                {"id": "id-a"},
                {"ids": ["id-b", "id-c"]},
                {"id": "id-d"},
            ]
        finally:
            disconnect_quietly(ws)


@pytest.fixture
def ws(mock_api_key):
    return WebSocketClient(api_key=mock_api_key)


CHANNEL_WITH_ID = [
    pytest.param((), {"channel": "trades", "symbol": "2330", "ids": ["id-a"]}, id="kwargs-ids"),
    pytest.param(("id-a",), {"channel": "trades", "symbol": "2330"}, id="positional-id"),
    pytest.param(({"channel": "trades", "symbol": "2330", "id": "id-a"},), {}, id="dict-id"),
    pytest.param(({"channel": "trades", "symbol": "2330", "ids": ["id-a"]},), {}, id="dict-ids"),
]


@pytest.mark.parametrize("product", ["stock", "futopt"])
@pytest.mark.parametrize("args, kwargs", CHANNEL_WITH_ID)
def test_channel_with_id_is_invalid_parameter(ws, product, args, kwargs):
    with pytest.raises(MarketDataError) as excinfo:
        getattr(ws, product).unsubscribe(*args, **kwargs)
    assert excinfo.value.code == 1005
    assert excinfo.value.message == "Invalid parameter 'channel': cannot be combined with 'id' or 'ids'"


@pytest.mark.parametrize("product", ["stock", "futopt"])
def test_unknown_channel_is_invalid_parameter_before_connecting(ws, product):
    for call in (
        lambda: getattr(ws, product).unsubscribe(channel="trade", symbol="2330"),
        lambda: getattr(ws, product).unsubscribe({"channel": "trade", "symbol": "2330"}),
    ):
        with pytest.raises(MarketDataError) as excinfo:
            call()
        assert excinfo.value.code == 1005


def test_known_channel_reaches_connection_check(ws):
    with pytest.raises(RuntimeError, match="Not connected"):
        ws.stock.unsubscribe(channel="Trades", symbol="2330")
    with pytest.raises(RuntimeError, match="Not connected"):
        ws.futopt.unsubscribe({"channel": "books", "symbol": "TXFC4", "afterHours": True})


@pytest.mark.parametrize(
    "kwargs, message",
    [
        pytest.param({"channel": "trades"}, "unsubscribe() requires either `symbol` or `symbols`", id="no-symbol"),
        pytest.param(
            {"channel": "trades", "symbol": "2330", "symbols": ["2330"]},
            "unsubscribe() accepts either `symbol` or `symbols`, not both",
            id="both-symbols",
        ),
        pytest.param(
            {"symbol": "2330"},
            "unsubscribe(): `symbol`, `symbols` and `odd_lot` require `channel`",
            id="symbol-without-channel",
        ),
    ],
)
def test_channel_arguments_are_checked(ws, kwargs, message):
    with pytest.raises(ValueError, match=message.replace("(", r"\(").replace(")", r"\)")):
        ws.stock.unsubscribe(**kwargs)
