"""``raw_message`` and ``messages(raw=True)`` hand over the frame's text (#246).

``raw_message`` callbacks get each message as the ``str`` the server sent;
``message`` still gets the dict, built only when a ``message`` callback is
registered. Either kind of callback takes the messages from the iterators.
``messages(raw=True)`` is the same for iteration.

Debug builds (``maturin develop``) panic where the dict is about to be built
when ``FUGLE_MARKETDATA_TEST_PANIC=ws_message_dict``, which is how the tests
see that a ``raw_message``-only client never builds it. A release build has
no injection sites, so those two tests are skipped there: the first would
pass whatever the binding did, the second would wait for an error that
never comes.
"""
import json
import os
import sys
import threading
import time
import typing

import pytest

from fugle_marketdata import MessageIterator, WebSocketError
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


_debug_build = None


def _is_debug_build():
    """Whether the installed binding has the test injection sites.

    Probed with ``ws_callback_poison``, the one site that needs no
    connection: a debug build panics in ``off()``, a release build returns.
    """
    global _debug_build
    if _debug_build is None:
        previous = os.environ.get(PANIC_ENV)
        os.environ[PANIC_ENV] = "ws_callback_poison"
        try:
            # The registry reads the variable when the client is created.
            product_ws("ws://127.0.0.1:9", "stock").off("reconnect")
            _debug_build = False
        except BaseException as panic:  # PanicException is a BaseException
            _debug_build = "ws_callback_poison" in str(panic)
        finally:
            if previous is None:
                del os.environ[PANIC_ENV]
            else:
                os.environ[PANIC_ENV] = previous
    return _debug_build


@pytest.fixture
def injection_sites():
    if not _is_debug_build():
        pytest.skip(
            "release build: FUGLE_MARKETDATA_TEST_PANIC injection sites exist only in "
            "debug builds (`maturin develop`), and this test proves nothing without them"
        )


class Frames:
    """Records what the ``raw_message`` and ``message`` callbacks receive, in
    delivery order, as ``(event, payload)``."""

    def __init__(self, ws, events):
        self.calls = []
        self._cond = threading.Condition()
        for event in events:
            ws.on(event, self._handler(event))

    def _handler(self, event):
        def record(payload):
            with self._cond:
                self.calls.append((event, payload))
                self._cond.notify_all()

        return record

    def of(self, event):
        with self._cond:
            return [payload for name, payload in self.calls if name == event]

    def wait_for_data(self, event, symbol):
        """Wait for the ``data`` frame of ``symbol`` on ``event``."""

        def arrived():
            for name, payload in self.calls:
                if name != event:
                    continue
                frame = json.loads(payload) if isinstance(payload, str) else payload
                if frame.get("event") == "data" and frame["data"].get("symbol") == symbol:
                    return True
            return False

        with self._cond:
            hit = self._cond.wait_for(arrived, timeout=TIMEOUT_S)
        assert hit, f'no data frame for {symbol} on "{event}"; got {self.calls}'


def _next_data(read):
    """The first ``data`` frame ``read()`` returns, as the iterator gave it."""
    deadline = time.monotonic() + TIMEOUT_S
    while time.monotonic() < deadline:
        item = read()
        if item is None:
            time.sleep(0.01)
            continue
        frame = json.loads(item) if isinstance(item, str) else item
        if frame.get("event") == "data":
            return item
    raise AssertionError("no data frame from the iterator")


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_raw_message_is_the_text_of_the_message_dict(server, product):
    ws = product_ws(server.url, product)
    frames = Frames(ws, ("raw_message", "message"))
    try:
        ws.connect()
        ws.subscribe("trades", "2330")
        frames.wait_for_data("message", "2330")
    finally:
        disconnect_quietly(ws)

    raws, dicts = frames.of("raw_message"), frames.of("message")
    assert dicts and len(raws) == len(dicts)
    assert all(type(raw) is str for raw in raws)
    assert [json.loads(raw) for raw in raws] == dicts
    # Each frame goes to `raw_message` first, then to `message`.
    assert [name for name, _ in frames.calls] == ["raw_message", "message"] * len(dicts)


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_raw_message_is_the_frame_verbatim(server, product):
    # The loopback server writes its frames with `json.dumps` defaults, so
    # with a space after every `,` and `:`. serde_json writes none, so a
    # binding that re-serialised the frame would not reproduce this text.
    sent = (
        '{"event": "data", "data": {"symbol": "2330", "price": 100}, '
        '"id": "trades-2330", "channel": "trades"}'
    )
    assert json.dumps(json.loads(sent)) == sent
    ws = product_ws(server.url, product)
    frames = Frames(ws, ("raw_message",))
    try:
        ws.connect()
        ws.subscribe("trades", "2330")
        frames.wait_for_data("raw_message", "2330")
    finally:
        disconnect_quietly(ws)
    assert sent in frames.of("raw_message"), frames.of("raw_message")


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_messages_raw_yields_the_frame_verbatim(server, product):
    sent = (
        '{"event": "data", "data": {"symbol": "2330", "price": 100}, '
        '"id": "trades-2330", "channel": "trades"}'
    )
    ws = product_ws(server.url, product)
    try:
        ws.connect()
        ws.subscribe("trades", "2330")
        assert _next_data(ws.messages(raw=True).__next__) == sent
    finally:
        disconnect_quietly(ws)


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_raw_message_alone_never_builds_the_dict(server, product, monkeypatch, injection_sites):
    monkeypatch.setenv(PANIC_ENV, "ws_message_dict")
    ws = product_ws(server.url, product)
    recorder = Recorder(ws)
    frames = Frames(ws, ("raw_message",))
    try:
        ws.connect()
        ws.subscribe("trades", "2330")
        frames.wait_for_data("raw_message", "2330")
        ws.subscribe("trades", "2317")
        frames.wait_for_data("raw_message", "2317")
        assert "error" not in recorder.names(), recorder.calls
    finally:
        disconnect_quietly(ws)


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_message_callback_builds_the_dict(server, product, monkeypatch, injection_sites):
    # The counterpart of the test above: the injection site is reached as
    # soon as a `message` callback is registered too.
    monkeypatch.setenv(PANIC_ENV, "ws_message_dict")
    ws = product_ws(server.url, product)
    recorder = Recorder(ws)
    ws.on("raw_message", lambda raw: None)
    ws.on("message", lambda msg: None)
    try:
        ws.connect()
        recorder.wait_for("error", TIMEOUT_S)
        (err,) = recorder.args_of("error")[0]
        assert isinstance(err, WebSocketError)
        message, code = err.args
        assert code == -1
        assert message.startswith("WebSocket message thread panicked: injected test panic"), message
    finally:
        disconnect_quietly(ws)


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_raw_message_callback_takes_messages_from_iterators(server, product):
    ws = product_ws(server.url, product)
    frames = Frames(ws, ("raw_message",))
    try:
        ws.connect()
        ws.subscribe("trades", "2330")
        frames.wait_for_data("raw_message", "2330")
        assert ws.messages().try_recv() is None
        assert ws.messages(raw=True).try_recv() is None
    finally:
        disconnect_quietly(ws)


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_off_raw_message_removes_the_callbacks(server, product):
    ws = product_ws(server.url, product)
    frames = Frames(ws, ("raw_message",))
    try:
        ws.connect()
        ws.subscribe("trades", "2330")
        frames.wait_for_data("raw_message", "2330")

        ws.off("raw_message")
        seen = len(frames.calls)
        ws.subscribe("trades", "2317")
        # With no callback left, the messages wait for an iterator.
        data = _next_data(ws.messages().__next__)
        assert data["data"]["symbol"] == "2317"
        assert len(frames.calls) == seen
    finally:
        disconnect_quietly(ws)


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_failing_raw_message_callback_is_reported_under_its_name(server, product):
    ws = product_ws(server.url, product)
    recorder = Recorder(ws)

    def fail(raw):
        raise ValueError("boom")

    ws.on("raw_message", fail)
    try:
        ws.connect()
        ws.subscribe("trades", "2330")
        recorder.wait_for("error", TIMEOUT_S)
        (err,) = recorder.args_of("error")[0]
        assert err.code == 3004
        assert err.event == "raw_message"
        assert isinstance(err.__cause__, ValueError)
    finally:
        disconnect_quietly(ws)


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_messages_raw_yields_the_text(server, product):
    ws = product_ws(server.url, product)
    try:
        ws.connect()
        ws.subscribe("trades", "2330")
        raw = _next_data(ws.messages(raw=True).__next__)
        assert type(raw) is str
        assert json.loads(raw)["data"]["symbol"] == "2330"

        # The mode belongs to the iterator, not to the connection.
        ws.subscribe("trades", "2317")
        msg = _next_data(ws.messages().__next__)
        assert isinstance(msg, dict) and msg["data"]["symbol"] == "2317"
    finally:
        disconnect_quietly(ws)


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_messages_raw_try_recv_and_recv_timeout_yield_the_text(server, product):
    ws = product_ws(server.url, product)
    try:
        ws.connect()
        messages = ws.messages(raw=True)
        ws.subscribe("trades", "2330")
        raw = _next_data(lambda: messages.recv_timeout(100))
        assert type(raw) is str and json.loads(raw)["data"]["symbol"] == "2330"

        ws.subscribe("trades", "2317")
        raw = _next_data(messages.try_recv)
        assert type(raw) is str and json.loads(raw)["data"]["symbol"] == "2317"
    finally:
        disconnect_quietly(ws)


@hard_timeout
async def test_messages_raw_async_iteration_yields_the_text(server):
    ws = product_ws(server.url, "stock")
    await ws.connect_async()
    try:
        ws.subscribe("trades", "2330")
        async for raw in ws.messages(raw=True):
            assert type(raw) is str
            frame = json.loads(raw)
            if frame["event"] == "data":
                assert frame["data"]["symbol"] == "2330"
                break
    finally:
        await ws.disconnect_async()


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_messages_raw_is_keyword_only(server, product):
    ws = product_ws(server.url, product)
    try:
        ws.connect()
        with pytest.raises(TypeError):
            ws.messages(None, True)
        with pytest.raises(TypeError):
            ws.messages(raw="yes")
    finally:
        disconnect_quietly(ws)


def test_raw_message_has_no_alias():
    ws = product_ws("ws://127.0.0.1:9", "stock")
    ws.on("raw_message", lambda raw: None)
    ws.off("raw_message")
    for name in ("raw", "raw_data", "message_raw"):
        with pytest.raises(ValueError, match="raw_message.*messages_dropped"):
            ws.on(name, lambda raw: None)


def test_message_iterator_is_subscriptable():
    # The stubs declare it generic over what it yields.
    alias = MessageIterator[str]
    if sys.version_info >= (3, 9):
        assert typing.get_origin(alias) is MessageIterator
        assert typing.get_args(alias) == (str,)
    else:
        # No `types.GenericAlias` before 3.9: the class itself.
        assert alias is MessageIterator
