"""``subscribe_async()`` sends the product's session modifier (#272).

``odd_lot=True`` (stock) and ``after_hours=True`` (futopt), or the dict's
``oddLot`` / ``afterHours``, reach the ``subscribe`` frame as
``intradayOddLot`` / ``afterHours``; without them the frame carries neither.
"""
import asyncio
import time

import pytest

from tests.ws_loopback import TIMEOUT_S, InProcessLoopbackServer, product_ws

hard_timeout = pytest.mark.timeout(20, method="thread")

# product, symbol, kwarg, dict key, frame key
PRODUCTS = [
    pytest.param("stock", "2330", "odd_lot", "oddLot", "intradayOddLot", id="stock"),
    pytest.param("futopt", "TXF1!", "after_hours", "afterHours", "afterHours", id="futopt"),
]


async def wait_for_subscribes(srv, count):
    deadline = time.monotonic() + TIMEOUT_S
    while len(srv.subscribe_data) < count:
        assert time.monotonic() < deadline, f"no {count} subscribe frames within {TIMEOUT_S}s"
        await asyncio.sleep(0.01)
    return srv.subscribe_data


@hard_timeout
@pytest.mark.parametrize("product, symbol, kwarg, key, frame_key", PRODUCTS)
async def test_subscribe_async_sends_the_modifier(product, symbol, kwarg, key, frame_key):
    with InProcessLoopbackServer() as srv:
        ws = product_ws(srv.url, product)
        try:
            await ws.connect_async()
            await ws.subscribe_async("trades", symbol, **{kwarg: True})
            await ws.subscribe_async({"channel": "books", "symbol": symbol, key: True})
            await ws.subscribe_async("candles", symbol)

            frames = await wait_for_subscribes(srv, 3)
            assert [(f["channel"], f.get(frame_key)) for f in frames] == [
                ("trades", True),
                ("books", True),
                ("candles", None),
            ]
        finally:
            await ws.disconnect_async()
