"""``disconnect()`` during a ``connect()`` handshake aborts that connect.

Core stops a ``connect()`` in progress when ``disconnect()`` is called: it
raises ``WebSocketError`` code 2010 (``ConnectionAborted``) and closes the
socket it opened (#121). The binding used to hold the connecting core client
in a local variable only, so ``disconnect()`` found nothing to close and the
connect went on to install a live connection (#143).

The server here never answers ``auth``, so the connect stays in the handshake
until it is aborted. Only the stock client has ``connect_async()``.
"""
import asyncio
import threading
import time

import pytest

from fugle_marketdata import MarketDataError, WebSocketError
from tests.ws_loopback import (
    SILENT_API_KEY,
    TIMEOUT_S,
    LoopbackServer,
    Recorder,
    disconnect_quietly,
    product_ws,
)

PRODUCTS = [pytest.param("stock", id="stock"), pytest.param("futopt", id="futopt")]

hard_timeout = pytest.mark.timeout(20, method="thread")

# Core's auth timeout is 10 s; an abort must come well before it.
ABORT_WITHIN_S = 3


@pytest.fixture
def server():
    with LoopbackServer() as srv:
        yield srv


def assert_aborted(err):
    assert isinstance(err, WebSocketError), repr(err)
    assert isinstance(err, MarketDataError)
    assert err.code == 2010


def assert_closed_after_abort(ws, rec):
    assert ws.is_connected() is False
    assert ws.is_closed() is True
    rec.wait_for("disconnect", TIMEOUT_S)
    # Give a duplicate the chance to show up.
    time.sleep(0.2)
    assert rec.names().count("disconnect") == 1, rec.calls
    assert "authenticated" not in rec.names()


def connect_in_thread(ws):
    """Run ``ws.connect()`` on a thread; returns ``(thread, outcome)`` where
    ``outcome`` gets ``error`` (or ``None``) and ``elapsed``."""
    outcome = {}

    def run():
        start = time.monotonic()
        try:
            ws.connect()
            outcome["error"] = None
        except Exception as e:  # noqa: BLE001 - asserted by the caller
            outcome["error"] = e
        outcome["elapsed"] = time.monotonic() - start

    thread = threading.Thread(target=run, daemon=True)
    thread.start()
    return thread, outcome


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_disconnect_during_handshake_aborts_connect(server, product):
    ws = product_ws(server.url, product, api_key=SILENT_API_KEY)
    rec = Recorder(ws)
    thread, outcome = connect_in_thread(ws)
    try:
        # The socket is open and the auth frame is on its way.
        rec.wait_for("connect", TIMEOUT_S)
        aborted_at = time.monotonic()
        ws.disconnect()
        # `disconnect()` waits for the aborted connection's callbacks (#54).
        assert rec.names().count("disconnect") == 1, rec.calls

        thread.join(ABORT_WITHIN_S)
        assert not thread.is_alive(), "connect() was not aborted"
        assert time.monotonic() - aborted_at < ABORT_WITHIN_S
        assert_aborted(outcome["error"])
        assert_closed_after_abort(ws, rec)
    finally:
        disconnect_quietly(ws)
        thread.join(TIMEOUT_S)


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_client_connects_again_after_an_aborted_connect(server, product):
    """The abort releases the connect gate and leaves nothing behind, and the
    next `connect()` clears `is_closed()` as usual."""
    silent = product_ws(server.url, product, api_key=SILENT_API_KEY)
    rec = Recorder(silent)
    thread, outcome = connect_in_thread(silent)
    rec.wait_for("connect", TIMEOUT_S)
    silent.disconnect()
    thread.join(ABORT_WITHIN_S)
    assert_aborted(outcome["error"])

    # A second connect on the same client is not refused as in progress; it
    # hangs in the handshake again, so abort it once more.
    thread, outcome = connect_in_thread(silent)
    rec.wait_until(lambda calls: [n for n, _ in calls].count("connect") == 2, TIMEOUT_S, "second connect")
    assert silent.is_closed() is True
    silent.disconnect()
    thread.join(ABORT_WITHIN_S)
    assert_aborted(outcome["error"])

    ws = product_ws(server.url, product)
    try:
        ws.connect()
        assert ws.is_connected() is True
        assert ws.is_closed() is False
    finally:
        disconnect_quietly(ws)


@hard_timeout
def test_disconnect_async_during_handshake_aborts_connect_async(server):
    ws = product_ws(server.url, "stock", api_key=SILENT_API_KEY)
    rec = Recorder(ws)

    async def scenario():
        connect = asyncio.ensure_future(ws.connect_async())
        await asyncio.to_thread(rec.wait_for, "connect", TIMEOUT_S)
        await ws.disconnect_async()
        assert rec.names().count("disconnect") == 1, rec.calls
        with pytest.raises(WebSocketError) as excinfo:
            await asyncio.wait_for(connect, ABORT_WITHIN_S)
        return excinfo.value

    try:
        assert_aborted(asyncio.run(scenario()))
        assert_closed_after_abort(ws, rec)
    finally:
        disconnect_quietly(ws)


@hard_timeout
def test_sync_disconnect_aborts_connect_async(server):
    """`connect_async()` runs on pyo3-async-runtimes' runtime, not the one the
    sync `disconnect()` blocks on."""
    ws = product_ws(server.url, "stock", api_key=SILENT_API_KEY)
    rec = Recorder(ws)

    async def scenario():
        connect = asyncio.ensure_future(ws.connect_async())
        await asyncio.to_thread(rec.wait_for, "connect", TIMEOUT_S)
        await asyncio.to_thread(ws.disconnect)
        with pytest.raises(WebSocketError) as excinfo:
            await asyncio.wait_for(connect, ABORT_WITHIN_S)
        return excinfo.value

    try:
        assert_aborted(asyncio.run(scenario()))
        assert_closed_after_abort(ws, rec)
    finally:
        disconnect_quietly(ws)


@hard_timeout
def test_cancelled_connect_async_leaves_nothing_to_abort(server):
    """A `connect_async()` cancelled mid-handshake drops its client itself;
    a later `disconnect()` finds nothing to close."""
    ws = product_ws(server.url, "stock", api_key=SILENT_API_KEY)
    rec = Recorder(ws)

    async def scenario():
        connect = asyncio.ensure_future(ws.connect_async())
        await asyncio.to_thread(rec.wait_for, "connect", TIMEOUT_S)
        connect.cancel()
        with pytest.raises(asyncio.CancelledError):
            await connect

    asyncio.run(scenario())
    # pyo3-async-runtimes drops the Rust future on its own runtime after the
    # Python future is cancelled.
    time.sleep(0.5)
    ws.disconnect()
    assert ws.is_closed() is False
    assert ws.is_connected() is False
