"""A panic on a WebSocket background thread is reported, not silent (#25).

Debug builds (``maturin develop``) panic on demand where
``FUGLE_MARKETDATA_TEST_PANIC`` says, read when ``connect()`` starts the
threads: ``ws_events`` on the event thread's first event, ``ws_messages`` on
the message thread's first frame (the ``authenticated`` ack). The ``error``
callbacks receive a ``WebSocketError`` with code -1, and ``disconnect()``
still returns.
"""
import asyncio
import time

import pytest

from fugle_marketdata import WebSocketError
from tests.test_websocket_events_loopback import (
    PRODUCTS,
    TIMEOUT_S,
    Recorder,
    _disconnect_quietly,
    _product_ws,
    hard_timeout,
)
from tests.ws_loopback import LoopbackServer

PANIC_ENV = "FUGLE_MARKETDATA_TEST_PANIC"


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
    ws = _product_ws(server.url, product)
    recorder = Recorder(ws)
    try:
        ws.connect()
        recorder.wait_for("error", TIMEOUT_S)
        _assert_panic_error(recorder, "event")
    finally:
        started = time.monotonic()
        _disconnect_quietly(ws)
        # The panicked event thread has already ended; joining it must not hang.
        assert time.monotonic() - started < TIMEOUT_S


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_message_thread_panic_fires_error(server, product, monkeypatch):
    monkeypatch.setenv(PANIC_ENV, "ws_messages")
    ws = _product_ws(server.url, product)
    recorder = Recorder(ws)
    ws.on("message", lambda msg: None)
    try:
        ws.connect()
        recorder.wait_for("error", TIMEOUT_S)
        _assert_panic_error(recorder, "message")
    finally:
        started = time.monotonic()
        _disconnect_quietly(ws)
        assert time.monotonic() - started < TIMEOUT_S


@hard_timeout
async def test_connect_async_message_thread_panic_fires_error(server, monkeypatch):
    monkeypatch.setenv(PANIC_ENV, "ws_messages")
    ws = _product_ws(server.url, "stock")
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
