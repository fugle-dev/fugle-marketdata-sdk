"""``connect()`` during an automatic reconnect waits for it (#230).

The 1.x / 2.x way to recover a lost connection — ``connect()`` then
``subscribe()`` in the ``disconnect`` callback — keeps working with
auto-reconnect on: ``connect()`` joins the reconnect instead of raising 2011
or opening a second connection. It returns once the reconnect is up and its
subscription replay is queued; it raises 2010 when ``disconnect()`` is called,
3005 when the attempts run out and ``AuthError`` when the credentials are
rejected. A connected client still refuses it with 2011.
"""
import asyncio
import threading
import time

import pytest

from fugle_marketdata import AuthError, ReconnectConfig, WebSocketClient, WebSocketError
from tests.ws_loopback import TIMEOUT_S, InProcessLoopbackServer, Recorder, disconnect_quietly

PRODUCTS = [pytest.param("stock", id="stock"), pytest.param("futopt", id="futopt")]
SYMBOLS = {"stock": "2330", "futopt": "TXF1!"}

hard_timeout = pytest.mark.timeout(30, method="thread")


@pytest.fixture
def server():
    with InProcessLoopbackServer() as srv:
        yield srv


def reconnecting_ws(url, product, max_attempts=0, delay_ms=100):
    """The ``product`` client of a WebSocketClient for ``url``, reconnect on."""
    client = WebSocketClient(
        api_key="test-key",
        base_url=url,
        reconnect=ReconnectConfig(
            enabled=True, max_attempts=max_attempts, initial_delay_ms=delay_ms, max_delay_ms=delay_ms
        ),
    )
    return getattr(client, product)


def wait_until(predicate, what):
    deadline = time.monotonic() + TIMEOUT_S
    while not predicate():
        assert time.monotonic() < deadline, f"no {what} within {TIMEOUT_S}s"
        time.sleep(0.01)


def lose_connection(server, recorder):
    """Cut the connection and wait until the ``disconnect`` callback ran, so
    a ``connect()`` from here on sees the connection as lost."""
    server.drop_connections()
    recorder.wait_for("disconnect", TIMEOUT_S)


def connect_in_thread(ws):
    """Run ``ws.connect()`` on a thread; returns the thread and its outcome."""
    outcome = {}

    def run():
        try:
            ws.connect()
            outcome["ok"] = True
        except Exception as e:
            outcome["error"] = e

    thread = threading.Thread(target=run, daemon=True)
    thread.start()
    return thread, outcome


def assert_code(err, cls, code):
    assert isinstance(err, cls), repr(err)
    assert err.code == code, repr(err)


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_connect_in_disconnect_callback_joins_the_reconnect(server, product):
    symbol = SYMBOLS[product]
    ws = reconnecting_ws(server.url, product)
    recorder = Recorder(ws)
    outcome = {}
    handled = threading.Event()

    def on_disconnect(*_args):
        # The 1.x pattern, without a try/except; only for the lost connection.
        if handled.is_set():
            return
        try:
            ws.connect()
            ws.subscribe("trades", symbol)
            outcome["ok"] = True
        except Exception as e:
            outcome["error"] = e
        finally:
            handled.set()

    ws.on("disconnect", on_disconnect)
    try:
        ws.connect()
        ws.subscribe("trades", symbol)
        wait_until(lambda: len(server.subscribe_log) == 1, "subscribe")

        server.drop_connections()
        assert handled.wait(TIMEOUT_S), "disconnect callback never ran"
        assert outcome == {"ok": True}

        # The replay comes before the callback's own subscribe, both on the
        # one new connection.
        wait_until(lambda: len(server.subscribe_log) == 3, "replay and subscribe")
        assert server.subscribe_log == [(0, symbol), (1, symbol), (1, symbol)]
        assert server.connections_accepted == 2

        # Once the reconnect's `authenticated` is delivered, it is refused again.
        recorder.wait_until(
            lambda calls: [n for n, _ in calls].count("authenticated") == 2, TIMEOUT_S, "second authenticated"
        )
        with pytest.raises(WebSocketError) as excinfo:
            ws.connect()
        assert_code(excinfo.value, WebSocketError, 2011)
        assert server.connections_accepted == 2
    finally:
        disconnect_quietly(ws)


@hard_timeout
async def test_connect_async_joins_the_reconnect(server):
    # Long enough a backoff that connect_async() is called within it.
    ws = reconnecting_ws(server.url, "stock", delay_ms=1500)
    recorder = Recorder(ws)
    try:
        await ws.connect_async()
        ws.subscribe("trades", "2330")
        await asyncio.to_thread(wait_until, lambda: len(server.subscribe_log) == 1, "subscribe")

        await asyncio.to_thread(lose_connection, server, recorder)
        await asyncio.wait_for(ws.connect_async(), TIMEOUT_S)
        ws.subscribe("trades", "2330")

        await asyncio.to_thread(wait_until, lambda: len(server.subscribe_log) == 3, "replay and subscribe")
        assert server.subscribe_log == [(0, "2330"), (1, "2330"), (1, "2330")]
        assert server.connections_accepted == 2
        assert ws.is_connected()
    finally:
        await ws.disconnect_async()


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_disconnect_during_backoff_ends_the_join_with_2010(server, product):
    ws = reconnecting_ws(server.url, product, delay_ms=5000)
    recorder = Recorder(ws)
    try:
        ws.connect()
        server.refuse_connections()
        lose_connection(server, recorder)

        thread, outcome = connect_in_thread(ws)
        time.sleep(0.3)
        assert thread.is_alive(), f"connect() did not wait on the reconnect: {outcome}"

        started = time.monotonic()
        ws.disconnect()
        thread.join(TIMEOUT_S)
        assert not thread.is_alive()
        assert time.monotonic() - started < 2, "disconnect() waited out the backoff"
        assert_code(outcome.get("error"), WebSocketError, 2010)
    finally:
        disconnect_quietly(ws)


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_disconnect_while_a_callback_waits_on_the_reconnect(server, product):
    # The join runs on the stream reader, which disconnect() waits for.
    ws = reconnecting_ws(server.url, product, delay_ms=5000)
    entered = threading.Event()
    outcome = {}

    def on_disconnect(*_args):
        if entered.is_set():
            return
        entered.set()
        try:
            ws.connect()
            outcome["ok"] = True
        except Exception as e:
            outcome["error"] = e

    ws.on("disconnect", on_disconnect)
    try:
        ws.connect()
        server.refuse_connections()
        server.drop_connections()
        assert entered.wait(TIMEOUT_S), "disconnect callback never ran"
        time.sleep(0.3)
        assert outcome == {}, "connect() did not wait on the reconnect"

        started = time.monotonic()
        ws.disconnect()
        assert time.monotonic() - started < 2, "disconnect() waited out the backoff"
        # disconnect() waited for the callbacks, this one included.
        assert_code(outcome.get("error"), WebSocketError, 2010)
    finally:
        disconnect_quietly(ws)


@hard_timeout
async def test_disconnect_async_during_backoff_ends_the_async_join_with_2010(server):
    ws = reconnecting_ws(server.url, "stock", delay_ms=5000)
    recorder = Recorder(ws)
    try:
        await ws.connect_async()
        server.refuse_connections()
        await asyncio.to_thread(lose_connection, server, recorder)

        join = asyncio.ensure_future(ws.connect_async())
        await asyncio.sleep(0.3)
        assert not join.done(), "connect_async() did not wait on the reconnect"

        await ws.disconnect_async()
        with pytest.raises(WebSocketError) as excinfo:
            await asyncio.wait_for(join, TIMEOUT_S)
        assert_code(excinfo.value, WebSocketError, 2010)
    finally:
        await ws.disconnect_async()


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_join_raises_3005_when_the_attempts_run_out(server, product):
    ws = reconnecting_ws(server.url, product, max_attempts=1, delay_ms=1000)
    recorder = Recorder(ws)
    try:
        ws.connect()
        server.refuse_connections()
        lose_connection(server, recorder)

        with pytest.raises(WebSocketError) as excinfo:
            ws.connect()
        assert_code(excinfo.value, WebSocketError, 3005)
        # The attempt it waited on was the only new connection.
        assert server.connections_accepted == 2
    finally:
        disconnect_quietly(ws)


@hard_timeout
async def test_async_join_raises_3005_when_the_attempts_run_out(server):
    ws = reconnecting_ws(server.url, "stock", max_attempts=1, delay_ms=1000)
    recorder = Recorder(ws)
    try:
        await ws.connect_async()
        server.refuse_connections()
        await asyncio.to_thread(lose_connection, server, recorder)

        with pytest.raises(WebSocketError) as excinfo:
            await asyncio.wait_for(ws.connect_async(), TIMEOUT_S)
        assert_code(excinfo.value, WebSocketError, 3005)
    finally:
        await ws.disconnect_async()


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_join_raises_auth_error_when_the_reconnect_is_rejected(server, product):
    ws = reconnecting_ws(server.url, product, delay_ms=1000)
    recorder = Recorder(ws)
    try:
        ws.connect()
        server.reject_auth()
        lose_connection(server, recorder)

        with pytest.raises(AuthError) as excinfo:
            ws.connect()
        assert_code(excinfo.value, AuthError, 2002)
        # Rejected on the reconnect's attempt, not on a connection of its own.
        assert server.connections_accepted == 2
    finally:
        disconnect_quietly(ws)


@hard_timeout
async def test_async_join_raises_auth_error_when_the_reconnect_is_rejected(server):
    ws = reconnecting_ws(server.url, "stock", delay_ms=1000)
    recorder = Recorder(ws)
    try:
        await ws.connect_async()
        server.reject_auth()
        await asyncio.to_thread(lose_connection, server, recorder)

        with pytest.raises(AuthError) as excinfo:
            await asyncio.wait_for(ws.connect_async(), TIMEOUT_S)
        assert_code(excinfo.value, AuthError, 2002)
        assert server.connections_accepted == 2
    finally:
        await ws.disconnect_async()


def slow_connect_callback(ws):
    """Hold up the stream reader in the ``connect`` callback, so it has not
    delivered ``authenticated`` when ``connect()`` returns."""
    ws.on("connect", lambda: time.sleep(0.3))


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_second_connect_is_refused_before_authenticated_is_delivered(server, product):
    ws = reconnecting_ws(server.url, product)
    slow_connect_callback(ws)
    try:
        ws.connect()
        with pytest.raises(WebSocketError) as excinfo:
            ws.connect()
        assert_code(excinfo.value, WebSocketError, 2011)
        assert server.connections_accepted == 1
    finally:
        disconnect_quietly(ws)


@hard_timeout
async def test_second_connect_async_is_refused_before_authenticated_is_delivered(server):
    ws = reconnecting_ws(server.url, "stock")
    slow_connect_callback(ws)
    try:
        await ws.connect_async()
        with pytest.raises(WebSocketError) as excinfo:
            await ws.connect_async()
        assert_code(excinfo.value, WebSocketError, 2011)
        assert server.connections_accepted == 1
    finally:
        await ws.disconnect_async()


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_concurrent_joins_both_return_once_reconnected(server, product):
    # Like core's client: every caller waits on the one reconnect.
    ws = reconnecting_ws(server.url, product, delay_ms=1000)
    recorder = Recorder(ws)
    try:
        ws.connect()
        lose_connection(server, recorder)

        joins = [connect_in_thread(ws) for _ in range(2)]
        for thread, _ in joins:
            thread.join(TIMEOUT_S)
        assert [outcome for _, outcome in joins] == [{"ok": True}, {"ok": True}]
        assert server.connections_accepted == 2
    finally:
        disconnect_quietly(ws)


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_concurrent_joins_both_raise_3005_when_the_attempts_run_out(server, product):
    ws = reconnecting_ws(server.url, product, max_attempts=1, delay_ms=1000)
    recorder = Recorder(ws)
    try:
        ws.connect()
        server.refuse_connections()
        lose_connection(server, recorder)

        joins = [connect_in_thread(ws) for _ in range(2)]
        for thread, _ in joins:
            thread.join(TIMEOUT_S)
        for _, outcome in joins:
            assert_code(outcome.get("error"), WebSocketError, 3005)
        assert server.connections_accepted == 2
    finally:
        disconnect_quietly(ws)


@hard_timeout
async def test_concurrent_async_joins_both_return_once_reconnected(server):
    ws = reconnecting_ws(server.url, "stock", delay_ms=1000)
    recorder = Recorder(ws)
    try:
        await ws.connect_async()
        await asyncio.to_thread(lose_connection, server, recorder)

        # A blocking connect() on a thread joins alongside.
        thread, outcome = connect_in_thread(ws)
        await asyncio.wait_for(asyncio.gather(ws.connect_async(), ws.connect_async()), TIMEOUT_S)
        await asyncio.to_thread(thread.join, TIMEOUT_S)
        assert outcome == {"ok": True}
        assert server.connections_accepted == 2
    finally:
        await ws.disconnect_async()
