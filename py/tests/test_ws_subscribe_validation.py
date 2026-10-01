"""`subscribe()` / `unsubscribe()` refuse what they would not use (#294).

The dict form used to read the keys it knew and drop the rest, and the
arguments next to a dict were dropped too, so `afterHours` on the stock
client subscribed board-lot data without a word. Each of these is now a
`TypeError`, raised before the connection is looked at, so the tests run on a
client that never connects: a valid call gets "Not connected" instead.
"""
import pytest

from fugle_marketdata import WebSocketClient

STOCK_KEYS = "channel, symbol, symbols, oddLot, odd_lot, intradayOddLot"
FUTOPT_KEYS = "channel, symbol, symbols, afterHours, after_hours"


@pytest.fixture
def ws():
    return WebSocketClient(api_key="key")


@pytest.mark.parametrize(
    "product, call, message",
    [
        ("stock", lambda c: c.subscribe({"channel": "trades", "symbol": "2330", "foo": 1}),
         f"subscribe(dict): unknown key 'foo' (accepted: {STOCK_KEYS})"),
        ("stock", lambda c: c.subscribe({"channel": "trades", "symbol": "2330", "afterHours": True}),
         "subscribe(dict): unknown key 'afterHours': it is a futopt option, the stock client takes "
         f"intradayOddLot (accepted: {STOCK_KEYS})"),
        ("futopt", lambda c: c.subscribe({"channel": "trades", "symbol": "TXFA6", "oddLot": True}),
         "subscribe(dict): unknown key 'oddLot': it is a stock option, the futopt client takes "
         f"afterHours (accepted: {FUTOPT_KEYS})"),
        ("stock", lambda c: c.subscribe({"channel": "trades", "symbol": "2330", "oddLot": "true"}),
         "subscribe(dict): 'oddLot' must be a boolean, got str"),
        ("futopt", lambda c: c.subscribe({"channel": "trades", "symbol": "TXFA6", "afterHours": None}),
         "subscribe(dict): 'afterHours' must be a boolean, got NoneType"),
        ("stock", lambda c: c.subscribe({"channel": "trades", "symbol": "2330", "oddLot": True, "intradayOddLot": True}),
         "subscribe(dict): 'intradayOddLot' and 'oddLot' are the same option; give one"),
        ("stock", lambda c: c.subscribe({"channel": "trades", "symbol": "2330"}, "2317"),
         "subscribe(): pass either a dict or the channel with symbol / symbols / odd_lot, not both"),
        ("stock", lambda c: c.subscribe({"channel": "trades", "symbol": "2330"}, odd_lot=False),
         "subscribe(): pass either a dict or the channel with symbol / symbols / odd_lot, not both"),
        ("futopt", lambda c: c.subscribe({"channel": "trades", "symbol": "TXFA6"}, symbols=["TXFB6"]),
         "subscribe(): pass either a dict or the channel with symbol / symbols / after_hours, not both"),
        ("stock", lambda c: c.unsubscribe({"id": "abc", "foo": 1}),
         "unsubscribe(dict): unknown key 'foo' (accepted: id, ids)"),
        ("stock", lambda c: c.unsubscribe({"channel": "trades", "symbol": "2330", "afterHours": True}),
         "unsubscribe(dict): unknown key 'afterHours': it is a futopt option, the stock client takes "
         f"intradayOddLot (accepted: {STOCK_KEYS})"),
        ("stock", lambda c: c.unsubscribe({"id": "abc"}, ids=["def"]),
         "unsubscribe(): pass either a dict or ids=, not both"),
    ],
)
def test_refused(ws, product, call, message):
    with pytest.raises(TypeError) as exc_info:
        call(getattr(ws, product))
    assert str(exc_info.value) == message


@pytest.mark.parametrize(
    "product, call",
    [
        ("stock", lambda c: c.subscribe({"channel": "trades", "symbol": "2330", "intradayOddLot": True})),
        ("stock", lambda c: c.subscribe({"channel": "trades", "symbol": "2330", "odd_lot": False})),
        ("stock", lambda c: c.subscribe("trades", "2330", odd_lot=True)),
        ("futopt", lambda c: c.subscribe({"channel": "books", "symbol": "TXFA6", "afterHours": True})),
        ("stock", lambda c: c.unsubscribe({"channel": "trades", "symbol": "2330", "intradayOddLot": True})),
        ("stock", lambda c: c.unsubscribe({"ids": ["abc"]})),
    ],
)
def test_passes_the_checks(ws, product, call):
    with pytest.raises(RuntimeError, match="Not connected"):
        call(getattr(ws, product))
