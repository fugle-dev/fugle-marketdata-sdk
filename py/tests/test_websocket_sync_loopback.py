"""Sync WebSocket ``connect()`` against a loopback server (no API key).

The blocking ``connect()`` runs on a plain Python thread, not a tokio one.
Anything it does outside the runtime context — ``messages()`` spawning its
bridge task, for one — panics there. The core crate's tests all run inside
``#[tokio::test]`` and cannot see that, so these go through the real binding
(#13, #24).
"""
import asyncio
import os
import signal
import threading
import time
import warnings

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
    for msg in ws.messages():
        seen.append(msg)
        if msg.get("event") == "data":
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


# The iterators end once the connection is gone: the reader closes their
# queue when core's stream closes. A closed queue that yielded None instead
# would spin forever; `GUARD` turns that into a failure instead of a hang.
GUARD = 1000


@hard_timeout
@pytest.mark.parametrize("product_case", PRODUCTS)
def test_for_loop_over_messages_ends_after_disconnect(server, product_case):
    product, _ = product_case
    ws = _product_ws(server.url, product)
    ws.connect()
    messages = ws.messages()
    seen = []
    done = threading.Event()

    def consume():
        for msg in messages:
            seen.append(msg)
            if len(seen) >= GUARD:
                break
        done.set()

    threading.Thread(target=consume, daemon=True).start()
    try:
        time.sleep(0.2)
        ws.disconnect()
        assert done.wait(TIMEOUT_S), "for loop did not end after disconnect()"
    finally:
        _disconnect_quietly(ws)
    assert [m["event"] for m in seen] == ["authenticated"], seen[:5]


@hard_timeout
async def test_async_for_over_messages_ends_after_disconnect(server):
    ws = _product_ws(server.url, "stock")
    await ws.connect_async()
    seen = []

    async def consume():
        async for msg in ws.messages():
            seen.append(msg)
            if len(seen) >= GUARD:
                break

    task = asyncio.create_task(consume())
    try:
        await asyncio.sleep(0.2)
        await ws.disconnect_async()
        await asyncio.wait_for(task, TIMEOUT_S)
    finally:
        task.cancel()
    assert [m["event"] for m in seen] == ["authenticated"], seen[:5]


@hard_timeout
@pytest.mark.parametrize("timeout_ms", [None, 50], ids=["default", "deprecated-timeout_ms"])
def test_iteration_waits_through_quiet_periods_and_yields_only_messages(server, timeout_ms):
    # Nothing arrives for a while: iteration neither yields None nor ends, and
    # the next message still comes through (#68). `timeout_ms` changes nothing.
    ws = _product_ws(server.url, "stock")
    try:
        ws.connect()
        with warnings.catch_warnings():
            warnings.simplefilter("ignore", DeprecationWarning)
            messages = ws.messages() if timeout_ms is None else ws.messages(timeout_ms=timeout_ms)
        assert next(messages)["event"] == "authenticated"
        threading.Timer(0.5, ws.subscribe, args=({"channel": "trades", "symbol": "2330"},)).start()
        started = time.monotonic()
        msg = next(messages)
        assert msg is not None
        assert msg["event"] == "subscribed"
        assert time.monotonic() - started >= 0.4
    finally:
        _disconnect_quietly(ws)


@hard_timeout
async def test_async_iteration_waits_through_quiet_periods_and_yields_only_messages(server):
    ws = _product_ws(server.url, "stock")
    await ws.connect_async()
    seen = []

    async def subscribe_later():
        await asyncio.sleep(0.5)
        await ws.subscribe_async("trades", "2330")

    subscriber = asyncio.create_task(subscribe_later())
    try:
        async def consume():
            async for msg in ws.messages():
                seen.append(msg)
                if len(seen) == 2:
                    return

        await asyncio.wait_for(consume(), TIMEOUT_S)
    finally:
        await subscriber
        await ws.disconnect_async()
    assert [m["event"] for m in seen] == ["authenticated", "subscribed"], seen


@pytest.mark.parametrize("product_case", PRODUCTS)
def test_messages_timeout_ms_is_deprecated(server, product_case):
    product, _ = product_case
    ws = _product_ws(server.url, product)
    try:
        ws.connect()
        with pytest.warns(DeprecationWarning, match="timeout_ms"):
            ws.messages(timeout_ms=100)
        with warnings.catch_warnings():
            warnings.simplefilter("error", DeprecationWarning)
            ws.messages()
    finally:
        _disconnect_quietly(ws)


class _Interrupted(Exception):
    pass


@hard_timeout
@pytest.mark.skipif(not hasattr(signal, "SIGUSR1"), reason="needs SIGUSR1")
def test_sync_iteration_wait_lets_signal_handlers_interrupt_it(server):
    # Like Ctrl+C: a signal handler that raises must interrupt a wait with no
    # data, rather than run only once a message arrives (#68). The subscribe
    # after 3 s bounds the wait if the handler never gets to run.
    ws = _product_ws(server.url, "stock")

    def interrupt(signum, frame):
        raise _Interrupted()

    previous = signal.signal(signal.SIGUSR1, interrupt)
    kill = threading.Timer(0.3, os.kill, args=(os.getpid(), signal.SIGUSR1))
    bound = threading.Timer(3, ws.subscribe, args=({"channel": "trades", "symbol": "2330"},))
    try:
        ws.connect()
        messages = ws.messages()
        assert next(messages)["event"] == "authenticated"
        kill.start()
        bound.start()
        started = time.monotonic()
        with pytest.raises(_Interrupted):
            next(messages)
        assert time.monotonic() - started < 2, "the signal was only handled once data arrived"
    finally:
        # Never restore the default handler while our signal may still come:
        # SIGUSR1's default action ends the process.
        kill.cancel()
        bound.cancel()
        kill.join()
        try:
            time.sleep(0.1)  # a signal sent just before cancel() runs its handler now
        except _Interrupted:
            pass
        signal.signal(signal.SIGUSR1, previous)
        _disconnect_quietly(ws)
