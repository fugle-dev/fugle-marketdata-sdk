"""A panic on a WebSocket background thread is reported, not silent (#25).

Debug builds (``maturin develop``) panic on demand where
``FUGLE_MARKETDATA_TEST_PANIC`` says, read when ``connect()`` starts the
stream reader: ``ws_events`` on its first event, ``ws_messages`` on its first
message (the ``authenticated`` ack), reported as the event and the message
thread. The ``error`` callbacks receive a ``WebSocketError`` with code -1, and
``disconnect()`` still returns. Both kinds share one reader since #68, so a
panic stops the delivery of events and messages alike.
"""
import asyncio
import time

import pytest

from fugle_marketdata import WebSocketError
from tests.ws_loopback import (
    TIMEOUT_S,
    LoopbackServer,
    Recorder,
    disconnect_quietly,
    product_ws,
)

PRODUCTS = [pytest.param("stock", id="stock"), pytest.param("futopt", id="futopt")]

PANIC_ENV = "FUGLE_MARKETDATA_TEST_PANIC"

hard_timeout = pytest.mark.timeout(20, method="thread")


@pytest.fixture
def server():
    with LoopbackServer() as srv:
        yield srv


def _assert_panic_error(recorder, thread):
    errors = recorder.args_of("error")
    assert len(errors) == 1, recorder.calls
    (err,) = errors[0]
    assert isinstance(err, WebSocketError)
    message, code = err.args
    assert code == -1
    assert message.startswith(f"WebSocket {thread} thread panicked: injected test panic"), message


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_event_thread_panic_fires_error(server, product, monkeypatch):
    monkeypatch.setenv(PANIC_ENV, "ws_events")
    ws = product_ws(server.url, product)
    recorder = Recorder(ws)
    try:
        ws.connect()
        recorder.wait_for("error", TIMEOUT_S)
        _assert_panic_error(recorder, "event")
    finally:
        started = time.monotonic()
        disconnect_quietly(ws)
        # The panicked stream reader has already ended; joining it must not hang.
        assert time.monotonic() - started < TIMEOUT_S


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_message_thread_panic_fires_error(server, product, monkeypatch):
    monkeypatch.setenv(PANIC_ENV, "ws_messages")
    ws = product_ws(server.url, product)
    recorder = Recorder(ws)
    ws.on("message", lambda msg: None)
    try:
        ws.connect()
        recorder.wait_for("error", TIMEOUT_S)
        _assert_panic_error(recorder, "message")
    finally:
        started = time.monotonic()
        disconnect_quietly(ws)
        assert time.monotonic() - started < TIMEOUT_S


@hard_timeout
async def test_connect_async_message_thread_panic_fires_error(server, monkeypatch):
    monkeypatch.setenv(PANIC_ENV, "ws_messages")
    ws = product_ws(server.url, "stock")
    recorder = Recorder(ws)
    ws.on("message", lambda msg: None)
    await ws.connect_async()
    try:
        deadline = time.monotonic() + TIMEOUT_S
        while "error" not in recorder.names() and time.monotonic() < deadline:
            await asyncio.sleep(0.02)
        _assert_panic_error(recorder, "message")
    finally:
        await ws.disconnect_async()


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_callbacks_still_fire_after_registry_lock_poisoned(product, monkeypatch):
    # A panic while the callback registry's write lock is held poisons it.
    # The registry must keep working, or reporting an error through it would
    # panic again and kill the reporting thread silently.
    monkeypatch.setenv(PANIC_ENV, "ws_callback_poison")
    # Nothing listens here, so connect() fails and core emits `Error`.
    ws = product_ws("ws://127.0.0.1:9", product)
    recorder = Recorder(ws)
    with pytest.raises(BaseException, match="ws_callback_poison"):
        ws.off("reconnect")

    ws.on("message", lambda msg: None)  # write lock, still usable
    with pytest.raises(Exception):
        ws.connect()

    recorder.wait_for("error", TIMEOUT_S)
    (err,) = recorder.args_of("error")[0]
    assert isinstance(err, WebSocketError)
    assert err.args[1] != -1, "the error came from a panicked thread, not core"
