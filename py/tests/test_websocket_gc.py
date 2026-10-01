"""A callback that refers back to its client does not keep it alive (#313).

``ws`` -> ``ws.stock`` -> its callbacks -> a closure over ``ws``: the cycle
goes through the registry the stream reader shares, so the GC has to be
shown the callbacks to collect it. While connected, the reader keeps them
alive and calling, as it does with no cycle.
"""
import asyncio
import gc
import subprocess
import sys
import textwrap
import threading
import weakref

import pytest

from fugle_marketdata import WebSocketClient
from tests.ws_loopback import InProcessLoopbackServer

PRODUCTS = [
    pytest.param(("stock", {"channel": "trades", "symbol": "2330"}), id="stock"),
    pytest.param(("futopt", {"channel": "trades", "symbol": "TXF1!", "afterHours": True}), id="futopt"),
]

TIMEOUT_S = 5

hard_timeout = pytest.mark.timeout(20, method="thread")


class DataCounter:
    """Lets the test wait until a number of `data` messages has arrived."""

    def __init__(self):
        self.count = 0
        self._cond = threading.Condition()

    def record(self, msg):
        if msg.get("event") != "data":
            return
        with self._cond:
            self.count += 1
            self._cond.notify_all()

    def wait_for(self, count):
        with self._cond:
            return self._cond.wait_for(lambda: self.count >= count, timeout=TIMEOUT_S)


def _register_closure_over_client(product, base_url=None, counter=None):
    """Register a callback that closes over a local ``ws``; return a weakref
    to the callback, which lives as long as the cycle does. The callback
    keeps the product client as ``client``, for the test to reach it."""
    ws = WebSocketClient(api_key="test-key", base_url=base_url)

    def on_message(msg):
        if counter is not None:
            counter.record(msg)
        return ws

    on_message.client = getattr(ws, product)
    on_message.client.on("message", on_message)
    return weakref.ref(on_message)


@pytest.mark.parametrize("product_case", PRODUCTS)
def test_closure_over_client_is_collected(product_case):
    product, _ = product_case
    callback = _register_closure_over_client(product)
    gc.collect()
    assert callback() is None


@pytest.mark.parametrize("product_case", PRODUCTS)
def test_closure_over_local_ws_only_is_collected(product_case):
    # The cycle as reported: nothing but the closure over `ws` leads back
    # to the client.
    product, _ = product_case

    def register():
        ws = WebSocketClient(api_key="test-key")

        def on_message(msg):
            return ws, msg

        getattr(ws, product).on("message", on_message)
        return weakref.ref(on_message)

    callback = register()
    gc.collect()
    assert callback() is None


@pytest.mark.parametrize("product_case", PRODUCTS)
def test_closure_over_client_is_freed_after_off(product_case):
    # Not the cycle itself: `off()` still lets go of the callback, which
    # reference counting then frees.
    product, _ = product_case
    callback = _register_closure_over_client(product)
    callback().client.off("message")
    assert callback() is None


@hard_timeout
@pytest.mark.parametrize("product_case", PRODUCTS)
def test_closure_over_client_is_collected_after_disconnect(product_case):
    product, subscription = product_case
    counter = DataCounter()
    with InProcessLoopbackServer() as srv:
        callback = _register_closure_over_client(product, srv.url, counter)
        client = callback().client
        client.connect()
        client.subscribe(subscription)
        assert counter.wait_for(1)
        client.disconnect()
        del client
        gc.collect()
        assert callback() is None


@pytest.mark.asyncio
@hard_timeout
@pytest.mark.parametrize("product_case", PRODUCTS)
async def test_closure_over_client_is_collected_after_disconnect_async(product_case):
    product, subscription = product_case
    counter = DataCounter()
    loop = asyncio.get_running_loop()
    with InProcessLoopbackServer() as srv:
        callback = _register_closure_over_client(product, srv.url, counter)
        client = callback().client
        await client.connect_async()
        client.subscribe(subscription)
        assert await loop.run_in_executor(None, counter.wait_for, 1)
        await client.disconnect_async()
        del client
        gc.collect()
        assert callback() is None


@hard_timeout
@pytest.mark.parametrize("product_case", PRODUCTS)
def test_connected_client_keeps_calling_its_callbacks(product_case):
    product, subscription = product_case
    counter = DataCounter()
    with InProcessLoopbackServer(flood=True) as srv:
        callback = _register_closure_over_client(product, srv.url, counter)
        callback().client.connect()
        callback().client.subscribe(subscription)
        assert counter.wait_for(1)

        gc.collect()
        assert callback() is not None
        delivered = counter.count
        assert counter.wait_for(delivered + 10)

        callback().client.disconnect()
        gc.collect()
        assert callback() is None


# A deadlock here holds the GIL, which no in-process timeout gets past.
OFF_FINALIZER_SCRIPT = textwrap.dedent("""
    import sys
    from fugle_marketdata import WebSocketClient

    product, how = sys.argv[1:]
    client = getattr(WebSocketClient(api_key="test-key"), product)

    class Handler:
        def __call__(self, msg):
            pass

        def __eq__(self, other):
            return isinstance(other, Handler)

        __hash__ = object.__hash__

        def __del__(self):
            client.on("connect", print)

    client.on("message", Handler())
    if how == "all":
        client.off("message")
    else:
        client.off("message", Handler())  # an equal one, not the registered one
""")


@pytest.mark.parametrize("product_case", PRODUCTS)
@pytest.mark.parametrize("how", ["all", "listener"])
def test_off_drops_callbacks_after_releasing_the_lock(product_case, how):
    # A finalizer of the callback `off()` removes may call `on()`; dropping
    # the callback under the registry's lock deadlocked.
    product, _ = product_case
    args = [sys.executable, "-c", OFF_FINALIZER_SCRIPT, product, how]
    result = subprocess.run(args, capture_output=True, text=True, timeout=30)
    assert result.returncode == 0, result.stderr
    assert "Exception ignored" not in result.stderr, result.stderr
