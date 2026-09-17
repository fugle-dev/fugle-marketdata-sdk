"""Connection-event callbacks against a loopback server (no API key).

The binding only forwards core's connection events (#55, #56): ``connect``,
``authenticated(data)`` / ``unauthenticated(data)`` and ``disconnect`` all come
from core's stream, and ``disconnect()`` / a failed ``connect()`` return only
once the stream reader has delivered what core queued (#54). Messages arrive
on the same stream, so ``message`` keeps its place among the events (#68).
"""
import asyncio
import threading
import time

import pytest

from fugle_marketdata import AuthError, HealthCheckConfig
from tests.ws_loopback import (
    REJECTED_API_KEY,
    TIMEOUT_S,
    InProcessLoopbackServer,
    LoopbackServer,
    Recorder,
    disconnect_quietly,
    product_ws,
)

PRODUCTS = [pytest.param("stock", id="stock"), pytest.param("futopt", id="futopt")]

# The heartbeat floor is 5 s, so leave room for one timeout plus teardown.
hard_timeout = pytest.mark.timeout(20, method="thread")


@pytest.fixture
def server():
    with LoopbackServer() as srv:
        yield srv


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_authenticated_receives_server_data(server, product):
    ws = product_ws(server.url, product)
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
        disconnect_quietly(ws)


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_rejected_key_fires_unauthenticated_before_raising(server, product):
    ws = product_ws(server.url, product, api_key=REJECTED_API_KEY)
    recorder = Recorder(ws)

    with pytest.raises(AuthError):
        ws.connect()

    # No waiting: connect() raises only after the stream reader delivered.
    assert recorder.args_of("unauthenticated") == [
        ({"message": "Invalid authentication credentials"},)
    ]
    assert recorder.args_of("connect") == [()]
    assert "authenticated" not in recorder.names()


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_disconnect_callback_reads_the_closed_state_of_a_lost_connection(product):
    # Core records the close before reporting it, so a callback reading the
    # state sees it closed (#86), on either product's client (#94).
    seen = []
    recorded = threading.Event()

    def record_state(*_):
        seen.append((ws.is_connected(), ws.is_closed()))
        recorded.set()

    with LoopbackServer() as srv:
        ws = product_ws(srv.url, product)
        recorder = Recorder(ws)
        ws.on("disconnect", record_state)
        ws.connect()
    # Leaving the block stops the server, which drops the connection.
    try:
        # Callbacks run in registration order, so the recorder hears the
        # disconnect before `record_state` has appended; wait for the latter.
        assert recorded.wait(TIMEOUT_S), recorder.calls
        assert seen == [(False, True)], recorder.calls
    finally:
        disconnect_quietly(ws)


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_heartbeat_timeout_then_disconnect_fires_disconnect_once(server, product):
    ws = product_ws(
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
        disconnect_quietly(ws)


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_disconnect_callback_has_fired_when_disconnect_returns(server, product):
    ws = product_ws(server.url, product)
    recorder = Recorder(ws)
    ws.connect()
    recorder.wait_for("authenticated", TIMEOUT_S)

    ws.disconnect()

    assert recorder.args_of("disconnect") == [(1000, "Normal closure")]


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_disconnect_from_disconnect_callback_still_waits(server, product):
    ws = product_ws(server.url, product)
    recorder = Recorder(ws)
    finished = []

    def disconnect_again(code, reason):
        # Runs on the stream reader: must neither self-join nor stop the outer
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
@pytest.mark.parametrize("product", PRODUCTS)
def test_message_callback_can_call_client_methods(server, product):
    # Callbacks run on the stream reader's thread, which may call back into
    # the client on either product (#94): the first `subscribed` ack
    # subscribes again from inside the callback.
    first, second = SUBSCRIPTIONS[product], dict(SUBSCRIPTIONS[product], symbol="OTHER")
    ws = product_ws(server.url, product)
    errors = []
    seen = []
    called = threading.Event()

    def subscribe_again(message):
        if message.get("event") != "subscribed" or called.is_set():
            return
        try:
            seen.append(ws.is_connected())
            ws.subscribe(second)
        except BaseException as exc:  # an unsendable client panics here
            errors.append(exc)
        finally:
            called.set()

    ws.on("error", errors.append)
    ws.on("message", subscribe_again)
    recorder = Recorder(ws, messages=True)
    try:
        ws.connect()
        ws.subscribe(first)
        recorder.wait_until(
            lambda calls: any(
                name == "message"
                and args[0].get("event") == "subscribed"
                and args[0]["data"]["symbol"] == "OTHER"
                for name, args in calls
            ),
            TIMEOUT_S,
            "the second subscribed ack",
        )
    finally:
        disconnect_quietly(ws)

    assert errors == []
    assert seen == [True]


@hard_timeout
async def test_connect_async_forwards_events(server):
    ws = product_ws(server.url, "stock")
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
    ws = product_ws(server.url, "stock", api_key=REJECTED_API_KEY)
    recorder = Recorder(ws)

    with pytest.raises(AuthError):
        await ws.connect_async()

    assert recorder.args_of("unauthenticated") == [
        ({"message": "Invalid authentication credentials"},)
    ]



SUBSCRIPTION = {"channel": "trades", "symbol": "2330"}
SUBSCRIPTIONS = {"stock": SUBSCRIPTION, "futopt": {"channel": "trades", "symbol": "TXFA4"}}


def _names(calls):
    return [name for name, _ in calls]


@hard_timeout
def test_messages_arrive_between_authenticated_and_disconnect_while_disconnecting():
    # Messages still flowing when disconnect() is called reach the callback,
    # and all of them before `disconnect` (#68).
    with LoopbackServer(flood=True) as srv:
        ws = product_ws(srv.url, "stock")
        recorder = Recorder(ws, messages=True)
        try:
            ws.connect()
            ws.subscribe(SUBSCRIPTION)
            recorder.wait_until(
                lambda calls: _names(calls).count("message") >= 50, TIMEOUT_S, "50 messages"
            )
            ws.disconnect()
        finally:
            disconnect_quietly(ws)

    names = _names(recorder.calls)
    assert names[:2] == ["connect", "authenticated"], names[:5]
    assert names[-1] == "disconnect", names[-5:]
    assert names.count("disconnect") == 1
    assert set(names[2:-1]) == {"message"}, [n for n in names[2:-1] if n != "message"]


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_messages_arriving_during_disconnect_reach_the_callback(product):
    # The server answers the client's Close with frames before its own Close:
    # they are written after disconnect() started and still belong to the
    # connection, so every one reaches `message`, before `disconnect` (#68).
    burst = 100
    with LoopbackServer(burst_on_close=burst) as srv:
        ws = product_ws(srv.url, product)
        recorder = Recorder(ws, messages=True)
        try:
            ws.connect()
            ws.disconnect()
        finally:
            disconnect_quietly(ws)

    data = [args for name, args in recorder.calls if name == "message" and args[0]["event"] == "data"]
    assert [args[0]["data"]["i"] for args in data] == list(range(burst))
    names = recorder.names()
    assert names[-1] == "disconnect", names[-5:]
    assert names.count("disconnect") == 1


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_rejected_authentication_frame_never_reaches_message(server, product):
    ws = product_ws(server.url, product, api_key=REJECTED_API_KEY)
    recorder = Recorder(ws, messages=True)
    with pytest.raises(AuthError):
        ws.connect()

    assert "message" not in recorder.names(), recorder.calls


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_message_callback_registered_after_connect_receives_messages(server, product):
    # Whether a `message` callback exists is checked per message (#68).
    ws = product_ws(server.url, product)
    try:
        ws.connect()
        recorder = Recorder(ws, messages=True)
        ws.subscribe(SUBSCRIPTIONS[product])
        recorder.wait_for("message", TIMEOUT_S)
    finally:
        disconnect_quietly(ws)


@hard_timeout
def test_disconnect_returns_while_an_unread_iterator_holds_up_the_stream():
    # Nobody iterates, so the handoff to `messages()` fills and the reader
    # waits; disconnect() must still return and fire `disconnect` (#54, #68).
    with LoopbackServer(flood=True) as srv:
        ws = product_ws(srv.url, "stock")
        recorder = Recorder(ws)
        try:
            ws.connect()
            ws.subscribe(SUBSCRIPTION)
            time.sleep(1)
            started = time.monotonic()
            ws.disconnect()
            assert time.monotonic() - started < TIMEOUT_S
            assert recorder.args_of("disconnect") == [(1000, "Normal closure")]
        finally:
            disconnect_quietly(ws)


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_iterator_waits_without_holding_the_gil(product):
    # The server answers from a Python thread of this process, and the
    # subscribe comes from another thread while the iterator waits: both need
    # the GIL the waiting iterator must have released (#68).
    with InProcessLoopbackServer() as srv:
        ws = product_ws(srv.url, product)
        try:
            ws.connect()
            messages = ws.messages()
            assert next(messages)["event"] == "authenticated"
            threading.Timer(0.2, ws.subscribe, args=(SUBSCRIPTIONS[product],)).start()
            assert next(messages)["event"] == "subscribed"
        finally:
            disconnect_quietly(ws)


# --- Message queue settings and drop reports (#46).


@pytest.mark.parametrize(
    "kwargs",
    [
        {"message_overflow": "dropNewest"},
        {"message_overflow": ""},
        {"message_buffer": 0},
        {"message_buffer": -1},
    ],
)
def test_message_queue_settings_reject_bad_values(kwargs):
    from fugle_marketdata import WebSocketClient

    with pytest.raises(ValueError):
        WebSocketClient(api_key="k", **kwargs)


def _slow_flood_ws(srv, **kwargs):
    """A stock client whose message callback is slower than the flood."""
    ws = product_ws(srv.url, "stock", **kwargs)
    ws.on("message", lambda msg: time.sleep(0.001))
    return ws


@hard_timeout
def test_messages_dropped_reports_add_up_to_the_count_after_disconnect():
    with LoopbackServer(flood=True) as srv:
        ws = _slow_flood_ws(srv, message_buffer=16)
        reports = []
        ws.on("messages_dropped", lambda dropped, total: reports.append((dropped, total)))
        assert ws.messages_dropped_total() == 0
        try:
            ws.connect()
            ws.subscribe(SUBSCRIPTION)
            deadline = time.monotonic() + TIMEOUT_S
            while not reports and time.monotonic() < deadline:
                time.sleep(0.05)
            ws.disconnect()
        finally:
            disconnect_quietly(ws)

    total = ws.messages_dropped_total()
    assert reports, "no messages_dropped report"
    assert total > 0
    # Every drop is reported before `disconnect` returns, and the count stays
    # readable after it.
    assert sum(dropped for dropped, _ in reports) == total
    assert reports[-1][1] == total


@hard_timeout
def test_unbounded_message_overflow_never_drops():
    # A burst far larger than `message_buffer` reaches a slow callback in full.
    # (A bounded flood would do too, but everything queued before
    # `disconnect` is delivered before disconnect() returns.)
    burst = 500
    with LoopbackServer(burst_on_close=burst) as srv:
        ws = _slow_flood_ws(srv, message_overflow="unbounded", message_buffer=16)
        reports = []
        data = []
        ws.on("messages_dropped", lambda dropped, total: reports.append((dropped, total)))
        ws.on("message", lambda msg: data.append(msg) if msg["event"] == "data" else None)
        try:
            ws.connect()
            ws.disconnect()
        finally:
            disconnect_quietly(ws)

    assert len(data) == burst
    assert reports == []
    assert ws.messages_dropped_total() == 0
