"""Sync WebSocket ``connect()`` against a loopback server (no API key).

The blocking ``connect()`` runs on a plain Python thread, not a tokio one.
Anything it does outside the runtime context — ``messages()`` spawning its
bridge task, for one — panics there. The core crate's tests all run inside
``#[tokio::test]`` and cannot see that, so these go through the real binding
(#13, #24).
"""
import threading
import time

import pytest

from fugle_marketdata import WebSocketClient
from tests.ws_loopback import InProcessLoopbackServer, LoopbackServer

PRODUCTS = [
    pytest.param(("stock", {"channel": "trades", "symbol": "2330"}), id="stock"),
    pytest.param(("futopt", {"channel": "trades", "symbol": "TXF1!", "afterHours": True}), id="futopt"),
]

TIMEOUT_S = 5

# A hang inside native code never lets the default signal-based timeout fire;
# the thread method catches it as long as the GIL is free. A hang that keeps the
# GIL (#39) stalls the run until the CI job times out.
hard_timeout = pytest.mark.timeout(20, method="thread")


class Collector:
    """Message callback that records frames and lets the test wait on them."""

    def __init__(self):
        self.seen = []
        self._cond = threading.Condition()

    def __call__(self, msg):
        with self._cond:
            self.seen.append(msg)
            self._cond.notify_all()

    def wait_for(self, event, timeout=TIMEOUT_S):
        with self._cond:
            hit = self._cond.wait_for(
                lambda: next((m for m in self.seen if m.get("event") == event), None),
                timeout=timeout,
            )
        assert hit, f'no "{event}" message within {timeout}s; got {self.seen}'
        return hit


@pytest.fixture
def server():
    with LoopbackServer() as srv:
        yield srv


def _product_ws(url, product):
    return getattr(WebSocketClient(api_key="test-key", base_url=url), product)


def _disconnect_quietly(ws):
    try:
        ws.disconnect()
    except Exception:
        pass  # never connected, or already gone


@pytest.fixture(params=PRODUCTS)
def product_case(request):
    return request.param


@pytest.fixture
def product_ws(product_case, server):
    product, subscription = product_case
    ws = _product_ws(server.url, product)
    yield ws, subscription
    _disconnect_quietly(ws)


@hard_timeout
def test_callbacks_receive_messages_after_connect(product_ws):
    ws, subscription = product_ws
    collector = Collector()
    ws.on("message", collector)

    ws.connect()
    ws.subscribe(subscription)

    collector.wait_for("authenticated")
    collector.wait_for("subscribed")
    data = collector.wait_for("data")
    assert data["data"]["symbol"] == subscription["symbol"]
    assert ws.is_connected()


@hard_timeout
def test_iterator_receives_messages_after_connect(product_ws):
    ws, subscription = product_ws

    ws.connect()
    ws.subscribe(subscription)

    seen = []
    deadline = time.monotonic() + TIMEOUT_S
    for msg in ws.messages(timeout_ms=100):
        if msg is not None:
            seen.append(msg)
            if msg.get("event") == "data":
                break
        if time.monotonic() > deadline:
            break

    events = [m.get("event") for m in seen]
    assert events[:3] == ["authenticated", "subscribed", "data"], seen
    assert seen[2]["data"]["symbol"] == subscription["symbol"]


@hard_timeout
def test_still_accepts_commands_well_after_connect(product_ws):
    ws, subscription = product_ws
    collector = Collector()
    ws.on("message", collector)

    ws.connect()
    # A dead background task drops its channels; give it time to die.
    time.sleep(0.5)

    ws.subscribe(subscription)
    collector.wait_for("subscribed")
    assert ws.is_connected()


@hard_timeout
def test_disconnect_returns_while_messages_flow(product_case):
    product, subscription = product_case
    # The stream reader is delivering frames non-stop, so disconnect() must
    # join it without holding the GIL that thread needs (#39).
    with LoopbackServer(flood=True) as srv:
        ws = _product_ws(srv.url, product)
        delivered = threading.Semaphore(0)
        ws.on("message", lambda msg: delivered.release())
        try:
            ws.connect()
            ws.subscribe(subscription)
            for _ in range(50):
                assert delivered.acquire(timeout=TIMEOUT_S), "data frames stopped flowing"

            started = time.monotonic()
            ws.disconnect()
            assert time.monotonic() - started < TIMEOUT_S
            assert not ws.is_connected()
        finally:
            _disconnect_quietly(ws)


@hard_timeout
def test_blocking_calls_work_with_server_in_same_process(product_case):
    product, subscription = product_case
    # The server answers from a Python thread of this process, so every
    # blocking call has to release the GIL while it waits on the network (#39).
    with InProcessLoopbackServer() as srv:
        ws = _product_ws(srv.url, product)
        collector = Collector()
        ws.on("message", collector)
        try:
            ws.connect()
            ws.subscribe(subscription)
            collector.wait_for("data")
            assert ws.is_connected()

            started = time.monotonic()
            ws.disconnect()
            # Waiting out the close-ack timeout also means the server never got the GIL.
            assert time.monotonic() - started < 2
        finally:
            _disconnect_quietly(ws)
