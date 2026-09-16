"""Connection-event callbacks against a loopback server (no API key).

The binding only forwards core's connection events (#55, #56): ``connect``,
``authenticated(data)`` / ``unauthenticated(data)`` and ``disconnect`` all come
from core's event channel, and ``disconnect()`` / a failed ``connect()`` return
only once the event thread has delivered what core queued (#54).
"""
import asyncio
import threading
import time

import pytest

from fugle_marketdata import AuthError, HealthCheckConfig, ReconnectConfig, WebSocketClient
from tests.ws_loopback import REJECTED_API_KEY, LoopbackServer

PRODUCTS = [pytest.param("stock", id="stock"), pytest.param("futopt", id="futopt")]

TIMEOUT_S = 5

# The heartbeat floor is 5 s, so leave room for one timeout plus teardown.
hard_timeout = pytest.mark.timeout(20, method="thread")


class Recorder:
    """Records ``(event, args)`` for every connection callback."""

    EVENTS = ("connect", "authenticated", "unauthenticated", "disconnect", "reconnect", "error")

    def __init__(self, ws):
        self.calls = []
        self._cond = threading.Condition()
        for event in self.EVENTS:
            ws.on(event, self._handler(event))

    def _handler(self, event):
        def record(*args):
            with self._cond:
                self.calls.append((event, args))
                self._cond.notify_all()

        return record

    def names(self):
        with self._cond:
            return [name for name, _ in self.calls]

    def args_of(self, event):
        with self._cond:
            return [args for name, args in self.calls if name == event]

    def wait_for(self, event, timeout):
        with self._cond:
            hit = self._cond.wait_for(lambda: event in [n for n, _ in self.calls], timeout=timeout)
        assert hit, f'no "{event}" callback within {timeout}s; got {self.calls}'


@pytest.fixture
def server():
    with LoopbackServer() as srv:
        yield srv


def _product_ws(url, product, api_key="test-key", **kwargs):
    client = WebSocketClient(
        api_key=api_key, base_url=url, reconnect=ReconnectConfig.disabled(), **kwargs
    )
    return getattr(client, product)


def _disconnect_quietly(ws):
    try:
        ws.disconnect()
    except Exception:
        pass  # never connected, or already gone


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_authenticated_receives_server_data(server, product):
    ws = _product_ws(server.url, product)
    recorder = Recorder(ws)
    try:
        ws.connect()
        recorder.wait_for("authenticated", TIMEOUT_S)

        names = recorder.names()
        assert names.index("connect") < names.index("authenticated"), recorder.calls
        assert recorder.args_of("connect") == [()]
        assert recorder.args_of("authenticated") == [
            ({"message": "Authenticated successfully"},)
        ]
    finally:
        _disconnect_quietly(ws)


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_rejected_key_fires_unauthenticated_before_raising(server, product):
    ws = _product_ws(server.url, product, api_key=REJECTED_API_KEY)
    recorder = Recorder(ws)

    with pytest.raises(AuthError):
        ws.connect()

    # No waiting: connect() raises only after the event thread delivered.
    assert recorder.args_of("unauthenticated") == [
        ({"message": "Invalid authentication credentials"},)
    ]
    assert recorder.args_of("connect") == [()]
    assert "authenticated" not in recorder.names()


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_heartbeat_timeout_then_disconnect_fires_disconnect_once(server, product):
    ws = _product_ws(
        server.url, product, health_check=HealthCheckConfig(heartbeat_timeout_ms=5000)
    )
    recorder = Recorder(ws)
    try:
        ws.connect()
        # The loopback server stays silent after the auth ack.
        recorder.wait_for("disconnect", timeout=10)
        ws.disconnect()

        assert len(recorder.args_of("disconnect")) == 1, recorder.calls
    finally:
        _disconnect_quietly(ws)


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_disconnect_callback_has_fired_when_disconnect_returns(server, product):
    ws = _product_ws(server.url, product)
    recorder = Recorder(ws)
    ws.connect()
    recorder.wait_for("authenticated", TIMEOUT_S)

    ws.disconnect()

    assert recorder.args_of("disconnect") == [(1000, "Normal closure")]


@hard_timeout
def test_disconnect_from_disconnect_callback_still_waits(server):
    # Stock only: futopt is `unsendable`, so a callback cannot call back into it.
    ws = _product_ws(server.url, "stock")
    recorder = Recorder(ws)
    finished = []

    def disconnect_again(code, reason):
        # Runs on the event thread: must neither self-join nor stop the outer
        # disconnect() from waiting for the callbacks after this one.
        ws.disconnect()
        time.sleep(0.2)
        finished.append(code)

    ws.on("disconnect", disconnect_again)
    ws.connect()
    recorder.wait_for("authenticated", TIMEOUT_S)

    ws.disconnect()

    assert finished == [1000]
    assert recorder.args_of("disconnect") == [(1000, "Normal closure")]


@hard_timeout
async def test_connect_async_forwards_events(server):
    ws = _product_ws(server.url, "stock")
    recorder = Recorder(ws)
    await ws.connect_async()
    try:
        deadline = time.monotonic() + TIMEOUT_S
        while "authenticated" not in recorder.names() and time.monotonic() < deadline:
            await asyncio.sleep(0.02)
        assert recorder.args_of("connect") == [()]
        assert recorder.args_of("authenticated") == [
            ({"message": "Authenticated successfully"},)
        ]
    finally:
        await ws.disconnect_async()

    assert recorder.args_of("disconnect") == [(1000, "Normal closure")]


@hard_timeout
async def test_connect_async_rejected_key_fires_unauthenticated_before_raising(server):
    ws = _product_ws(server.url, "stock", api_key=REJECTED_API_KEY)
    recorder = Recorder(ws)

    with pytest.raises(AuthError):
        await ws.connect_async()

    assert recorder.args_of("unauthenticated") == [
        ({"message": "Invalid authentication credentials"},)
    ]

