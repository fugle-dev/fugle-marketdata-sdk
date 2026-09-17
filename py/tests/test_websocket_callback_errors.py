"""A callback that raises neither stops later events nor goes unnoticed (#83).

A callback raising an ``Exception`` is reported to the ``error`` callbacks as
a ``WebSocketError`` (code 3004) whose ``__cause__`` is the exception —
throttled: the first at once, later ones at most once per second with their
``count``. Without an ``error`` callback, or when it raises too, the failure
goes to ``sys.unraisablehook``. ``async def`` callbacks are refused.
"""
import sys
import threading
import time

import pytest

from fugle_marketdata import ReconnectConfig, WebSocketClient, WebSocketError
from tests.ws_loopback import TIMEOUT_S, LoopbackServer, disconnect_quietly, product_ws

PRODUCTS = [pytest.param("stock", id="stock"), pytest.param("futopt", id="futopt")]
SUBSCRIPTION = {"channel": "trades", "symbol": "2330"}

hard_timeout = pytest.mark.timeout(20, method="thread")


@pytest.fixture
def server():
    with LoopbackServer() as srv:
        yield srv


@pytest.fixture
def unraisable(monkeypatch):
    """What reached ``sys.unraisablehook``, as ``(exc_value, object)``."""
    seen = []
    monkeypatch.setattr(sys, "unraisablehook", lambda hook: seen.append((hook.exc_value, hook.object)))
    return seen


def wait_until(predicate, what, timeout=TIMEOUT_S):
    deadline = time.monotonic() + timeout
    while not predicate():
        assert time.monotonic() < deadline, f"timed out waiting for {what}"
        time.sleep(0.01)


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_raising_message_callback_is_reported_and_later_messages_arrive(server, product, unraisable):
    ws = product_ws(server.url, product)
    received = []
    errors = []

    def on_message(msg):
        received.append(msg)
        raise ValueError("boom")

    ws.on("message", on_message)
    ws.on("error", errors.append)
    try:
        ws.connect()
        ws.subscribe(SUBSCRIPTION)
        # authenticated, subscribed and data: three failures within a second.
        wait_until(lambda: len(received) >= 3, "three messages")
        time.sleep(0.1)
    finally:
        disconnect_quietly(ws)

    assert unraisable == []
    assert len(errors) == 1, errors
    (err,) = errors
    assert isinstance(err, WebSocketError)
    assert err.code == 3004
    assert err.args == (err.message, 3004)
    assert err.source_kind == "client"
    assert err.event == "message"
    assert err.count == 1
    assert isinstance(err.__cause__, ValueError)
    assert str(err.__cause__) == "boom"
    assert err.message == "'message' callback raised ValueError: boom"


@hard_timeout
def test_later_failures_are_reported_once_per_second_with_their_count(server):
    ws = product_ws(server.url, "stock")
    received = []
    reports = []

    def on_message(msg):
        received.append(msg)
        raise ValueError(f"boom {len(received)}")

    ws.on("message", on_message)
    # The report runs right after the failing callback: record how many
    # messages had failed by then.
    ws.on("error", lambda err: reports.append((err, len(received))))
    try:
        ws.connect()
        ws.subscribe(SUBSCRIPTION)
        wait_until(lambda: len(received) >= 3, "three messages")
        time.sleep(1.1)
        ws.subscribe({"channel": "books", "symbol": "2330"})
        wait_until(lambda: len(reports) == 2, "second report")
    finally:
        disconnect_quietly(ws)

    (first, first_at), (second, second_at) = reports
    assert (first.count, first_at) == (1, 1)
    assert second.count == second_at - first_at
    assert second.count > 1
    assert str(second.__cause__) == f"boom {second_at}"


@hard_timeout
def test_without_error_callback_the_failure_goes_to_unraisablehook(server, unraisable):
    ws = product_ws(server.url, "stock")
    received = []

    def on_message(msg):
        received.append(msg)
        raise ValueError("unheard boom")

    ws.on("message", on_message)
    try:
        ws.connect()
        ws.subscribe(SUBSCRIPTION)
        wait_until(lambda: len(received) >= 3, "three messages")
    finally:
        disconnect_quietly(ws)

    assert len(unraisable) == 1, unraisable
    exc, obj = unraisable[0]
    assert isinstance(exc, WebSocketError)
    assert exc.code == 3004
    assert str(exc.__cause__) == "unheard boom"
    assert obj is on_message


@hard_timeout
def test_raising_error_callback_is_printed_not_re_reported(server, unraisable):
    ws = product_ws(server.url, "stock")
    received = []
    error_calls = []

    def on_message(msg):
        received.append(msg)
        raise ValueError("first boom")

    def on_error(err):
        error_calls.append(err)
        raise RuntimeError("error callback boom")

    ws.on("message", on_message)
    ws.on("error", on_error)
    try:
        ws.connect()
        ws.subscribe(SUBSCRIPTION)
        wait_until(lambda: len(received) >= 3, "three messages")
        time.sleep(0.1)
    finally:
        disconnect_quietly(ws)

    assert len(error_calls) == 1
    assert len(unraisable) == 1, unraisable
    exc, obj = unraisable[0]
    assert isinstance(exc, RuntimeError)
    assert obj is on_error
    assert exc.__context__ is error_calls[0]


@hard_timeout
def test_base_exception_is_only_printed(server, unraisable):
    ws = product_ws(server.url, "stock")
    received = []
    errors = []

    def on_message(msg):
        received.append(msg)
        raise KeyboardInterrupt

    ws.on("message", on_message)
    ws.on("error", errors.append)
    try:
        ws.connect()
        ws.subscribe(SUBSCRIPTION)
        wait_until(lambda: len(received) >= 3, "three messages")
    finally:
        disconnect_quietly(ws)

    assert errors == []
    assert len(unraisable) == len(received)
    assert all(isinstance(exc, KeyboardInterrupt) for exc, _ in unraisable)


@hard_timeout
def test_raising_connect_callback_does_not_fail_connect(server):
    ws = product_ws(server.url, "stock")
    errors = []

    def on_connect():
        raise ValueError("connect boom")

    ws.on("connect", on_connect)
    ws.on("error", errors.append)
    try:
        ws.connect()
        wait_until(lambda: errors, "error report")
    finally:
        disconnect_quietly(ws)

    assert [(e.code, e.event, str(e.__cause__)) for e in errors] == [(3004, "connect", "connect boom")]


def test_async_def_callback_is_refused():
    ws = WebSocketClient(api_key="test-key").stock

    async def on_message(msg):
        pass

    with pytest.raises(TypeError, match="async def callbacks are not supported"):
        ws.on("message", on_message)


@hard_timeout
def test_callback_returning_a_coroutine_is_reported(server, recwarn):
    ws = product_ws(server.url, "stock")
    errors = []

    async def handle(msg):
        pass

    ws.on("message", lambda msg: handle(msg))
    ws.on("error", errors.append)
    try:
        ws.connect()
        wait_until(lambda: errors, "error report")
    finally:
        disconnect_quietly(ws)

    assert errors[0].code == 3004
    assert isinstance(errors[0].__cause__, TypeError)
    assert "coroutine" in str(errors[0].__cause__)
    assert not [w for w in recwarn if "never awaited" in str(w.message)]


@hard_timeout
def test_reconnection_failure_carries_code_3005():
    errors = []
    connected = threading.Event()
    srv = LoopbackServer().__enter__()
    try:
        ws = WebSocketClient(
            api_key="test-key",
            base_url=srv.url,
            reconnect=ReconnectConfig(max_attempts=1, initial_delay_ms=100, max_delay_ms=100),
        ).stock
        ws.on("authenticated", lambda data: connected.set())
        ws.on("error", errors.append)
        ws.connect()
        assert connected.wait(TIMEOUT_S)
    finally:
        # Stop the server: the connection drops and the reconnect is refused.
        srv.__exit__(None, None, None)
    try:
        wait_until(lambda: any(e.code == 3005 for e in errors), "reconnection failure", timeout=10)
    finally:
        disconnect_quietly(ws)

    failure = next(e for e in errors if e.code == 3005)
    assert failure.source_kind == "network"
    assert failure.args == ("Reconnection failed after 1 attempts", 3005)
