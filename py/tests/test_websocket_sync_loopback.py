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
from tests.ws_loopback import LoopbackServer

PRODUCTS = [
    pytest.param(("stock", {"channel": "trades", "symbol": "2330"}), id="stock"),
    pytest.param(("futopt", {"channel": "trades", "symbol": "TXF1!", "afterHours": True}), id="futopt"),
]

TIMEOUT_S = 5

# ``disconnect()`` joins the message thread with the GIL held; if that thread is
# waiting for the GIL to deliver a frame the two deadlock inside native code,
# where the default signal-based timeout never fires. The thread method still
# kills the run (#39).
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


@pytest.fixture(params=PRODUCTS)
def product_ws(request, server):
    product, subscription = request.param
    client = WebSocketClient(api_key="test-key", base_url=server.url)
    ws = getattr(client, product)
    yield ws, subscription
    try:
        ws.disconnect()
    except Exception:
        pass  # never connected, or already gone


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
    # Drain the last frame so teardown's disconnect() cannot race its delivery (#39).
    collector.wait_for("data")
    assert ws.is_connected()
