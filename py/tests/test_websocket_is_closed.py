"""``is_closed()`` after ``disconnect()``.

``disconnect()`` takes and drops the client's ``state``, so ``is_closed()``
used to read ``None`` as "never connected" and return ``False`` — the exact
opposite of what its own docstring promises. #146.
"""
import asyncio

import pytest

from tests.ws_loopback import TIMEOUT_S, LoopbackServer, product_ws

PRODUCTS = [pytest.param("stock", id="stock"), pytest.param("futopt", id="futopt")]

hard_timeout = pytest.mark.timeout(20, method="thread")


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_is_closed_is_false_before_any_connect(product):
    """A fresh client was never closed, so the flag must not start set."""
    with LoopbackServer() as server:
        ws = product_ws(server.url, product)
        assert ws.is_closed() is False


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_is_closed_is_true_once_disconnect_returns(product):
    with LoopbackServer() as server:
        ws = product_ws(server.url, product)
        ws.connect()
        assert ws.is_closed() is False

        ws.disconnect()
        # No polling: `disconnect()` only returns once the client is closed,
        # so the answer must be right immediately.
        assert ws.is_closed() is True


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_reconnecting_the_same_client_clears_closed(product):
    """`connect()` installs a fresh core client, so the old close must not
    keep showing through."""
    with LoopbackServer() as server:
        ws = product_ws(server.url, product)
        ws.connect()
        ws.disconnect()
        assert ws.is_closed() is True

        ws.connect()
        assert ws.is_closed() is False
        ws.disconnect()
        assert ws.is_closed() is True


@hard_timeout
def test_is_closed_is_true_after_disconnect_async():
    """The async path drops `state` the same way, so it needs its own check."""
    with LoopbackServer() as server:
        ws = product_ws(server.url, "stock")

        async def run():
            await ws.connect_async()
            assert ws.is_closed() is False
            await ws.disconnect_async()
            return ws.is_closed()

        assert asyncio.run(run()) is True


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_disconnect_without_connect_leaves_is_closed_false(product):
    """Nothing was closed, so nothing should be reported as closed — the flag
    tracks a client that actually existed, not the bare call."""
    with LoopbackServer() as server:
        ws = product_ws(server.url, product)
        ws.disconnect()
        assert ws.is_closed() is False
        # Still usable.
        ws.connect()
        assert ws.is_closed() is False
        ws.disconnect()
        assert ws.is_closed() is True


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_is_connected_stays_false_after_disconnect(product):
    """`is_connected()` reads `None` as "not connected", which is already
    correct — guard it so the #146 fix does not flip it."""
    with LoopbackServer() as server:
        ws = product_ws(server.url, product)
        ws.connect()
        ws.disconnect()
        assert ws.is_connected() is False
