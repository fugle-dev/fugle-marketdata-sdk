"""2.x callback API in 3.0 (#304): ``on()`` registers an equal callback
once, as 2.x's ``pyee`` did; ``off(event, listener)`` removes that listener
only (2.x documented it but raised ``AttributeError``); and the ``error``
callback's single argument prints as its message (#299)."""

import threading

import pytest

from fugle_marketdata import WebSocketError
from tests.ws_loopback import TIMEOUT_S, LoopbackServer, Recorder, disconnect_quietly, product_ws

PRODUCTS = [pytest.param("stock", id="stock"), pytest.param("futopt", id="futopt")]
SUBSCRIPTIONS = {"stock": {"channel": "trades", "symbol": "2330"}, "futopt": {"channel": "trades", "symbol": "TXFA4"}}

hard_timeout = pytest.mark.timeout(20, method="thread")


@pytest.fixture
def server():
    with LoopbackServer() as srv:
        yield srv


class _Listener:
    def __init__(self):
        self.calls = 0

    def on_connect(self):
        self.calls += 1


def _connect_and_wait(ws):
    recorder = Recorder(ws)
    ws.connect()
    # Callbacks run in order on one thread: once `authenticated` is
    # recorded, every `connect` callback has run.
    recorder.wait_for("authenticated", TIMEOUT_S)
    return recorder


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_off_with_listener_removes_only_that_listener(server, product):
    ws = product_ws(server.url, product)
    removed, kept = [], []
    ws.on("connect", lambda: removed.append(1))
    first = lambda: removed.append(1)  # noqa: E731
    ws.on("connect", first)
    ws.on("connect", lambda: kept.append(1))
    ws.off("connect", first)
    try:
        _connect_and_wait(ws)
        # The first lambda is a different object and stays registered.
        assert (len(removed), len(kept)) == (1, 1)
    finally:
        disconnect_quietly(ws)


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_registering_the_same_callback_twice_calls_it_once(server, product):
    ws = product_ws(server.url, product)
    calls = []
    on_connect = lambda: calls.append(1)  # noqa: E731
    ws.on("connect", on_connect)
    ws.on("connect", on_connect)
    listener = _Listener()
    ws.on("connect", listener.on_connect)
    ws.on("connect", listener.on_connect)  # a new bound method, == to the first
    try:
        _connect_and_wait(ws)
        assert (calls, listener.calls) == ([1], 1)
    finally:
        disconnect_quietly(ws)


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_setup_run_again_handles_each_message_once(server, product):
    """A setup function that runs several times (say, on every reconnect
    of the caller's own) registers its handler once: each message is
    handled once, as in 2.x."""
    ws = product_ws(server.url, product)
    handled = []
    done = threading.Event()

    def on_message(msg):
        handled.append(msg)
        if msg["event"] == "subscribed":
            done.set()

    def setup():
        ws.on("message", on_message)

    for _ in range(3):
        setup()
    try:
        _connect_and_wait(ws)
        ws.subscribe(SUBSCRIPTIONS[product])
        assert done.wait(TIMEOUT_S), handled
        events = [msg["event"] for msg in handled]
        assert events.count("authenticated") == 1, events
        assert events.count("subscribed") == 1, events
    finally:
        disconnect_quietly(ws)


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_on_propagates_a_failing_eq_and_adds_nothing(server, product):
    class BadCallable:
        def __call__(self):
            raise AssertionError("never registered")

        def __eq__(self, other):
            raise RuntimeError("eq boom")

        __hash__ = object.__hash__

    ws = product_ws(server.url, product)
    ws.on("connect", lambda: None)
    with pytest.raises(RuntimeError, match="eq boom"):
        ws.on("connect", BadCallable())
    try:
        recorder = _connect_and_wait(ws)
        assert "error" not in recorder.names()
    finally:
        disconnect_quietly(ws)


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_off_matches_bound_methods_by_equality(server, product):
    """``obj.method`` is a new object each time; 2.x (pyee) matched it by
    ``==``, so ``off(event, obj.method)`` still removes it."""
    ws = product_ws(server.url, product)
    listener = _Listener()
    ws.on("connect", listener.on_connect)
    ws.off("connect", listener.on_connect)
    try:
        _connect_and_wait(ws)
        assert listener.calls == 0
    finally:
        disconnect_quietly(ws)


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_off_with_unregistered_listener_or_none(server, product):
    ws = product_ws(server.url, product)
    calls = []
    ws.on("connect", lambda: calls.append("a"))
    ws.off("connect", lambda: None)  # not registered: ignored
    ws.off("message", lambda: None)  # nothing registered for the event
    try:
        _connect_and_wait(ws)
        assert calls == ["a"]
    finally:
        disconnect_quietly(ws)

    ws = product_ws(server.url, product)
    ws.on("connect", lambda: calls.append("b"))
    ws.off("connect", None)  # None: every callback, as off(event)
    try:
        _connect_and_wait(ws)
        assert calls == ["a"]
    finally:
        disconnect_quietly(ws)


def test_off_rejects_unknown_event():
    ws = product_ws("ws://127.0.0.1:9", "stock")
    with pytest.raises(ValueError):
        ws.off("nope", lambda: None)


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_error_callback_argument_prints_its_message(server, product):
    """2.x handlers log the error with ``str(err)`` or ``print(err)``."""
    ws = product_ws(server.url, product)
    errors = []

    def boom():
        raise RuntimeError("boom")

    ws.on("connect", boom)
    ws.on("error", errors.append)
    try:
        recorder = _connect_and_wait(ws)
        recorder.wait_for("error", TIMEOUT_S)
    finally:
        disconnect_quietly(ws)

    err = errors[0]
    assert isinstance(err, WebSocketError)
    assert str(err) == err.message == err.args[0]
    assert str(err) == "'connect' callback raised RuntimeError: boom"


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_off_propagates_a_failing_eq_and_removes_nothing(server, product):
    class Bad:
        def __eq__(self, other):
            raise RuntimeError("eq boom")

        __hash__ = object.__hash__

    ws = product_ws(server.url, product)
    calls = []
    ws.on("connect", lambda: calls.append(1))
    with pytest.raises(RuntimeError, match="eq boom"):
        ws.off("connect", Bad())
    try:
        _connect_and_wait(ws)
        assert calls == [1]
    finally:
        disconnect_quietly(ws)
