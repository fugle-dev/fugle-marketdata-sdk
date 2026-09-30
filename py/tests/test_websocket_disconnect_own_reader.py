"""``disconnect()`` waits for the stream reader of the connection it closed (#277).

It waits for that reader so the ``disconnect`` callback has fired by the time
it returns (#54). The binding kept the reader in one slot per client, which a
``connect()`` from another thread overwrote while the ``disconnect()`` was
still closing: the ``disconnect()`` then waited for the new connection's
reader, until that connection was closed too, and the old reader was waited
for by no one.

The server holds its answer to the Close, so the ``connect()`` lands inside
the ``disconnect()`` every time.

A ``connect()`` that replaces a connection that was lost leaves that
connection's reader for the next ``disconnect()`` to wait for, and its
``messages()`` iterators still get what core queued for them. A
``disconnect()`` from a callback waits for no stream reader, so two readers
disconnecting from callbacks never wait for each other. Neither does a
``disconnect_async()`` created in a callback and waited for there, with
``asyncio.run()`` say: it used to wait, on a runtime thread, for the very
reader waiting for it (#280).
"""
import asyncio
import threading
import time

import pytest

from fugle_marketdata import WebSocketError
from tests.ws_loopback import (
    TIMEOUT_S,
    InProcessLoopbackServer,
    Recorder,
    disconnect_quietly,
    product_ws,
    to_thread,
)

PRODUCTS = [pytest.param("stock", id="stock"), pytest.param("futopt", id="futopt")]
SYMBOLS = {"stock": "2330", "futopt": "TXF1!"}

hard_timeout = pytest.mark.timeout(30, method="thread")

# How long the rejected connection's `connect` callback holds its reader, so
# its `unauthenticated` callback comes after the `connect()` has failed. Not
# what the test depends on: see there.
REJECTED_READER_BUSY_S = 0.3

# How long a replaced-connection test waits for what must not happen: a
# `disconnect()` returning, or a callback firing, too early.
RETURN_WAIT_S = 1.0

BUFFER = 16


@pytest.fixture
def server():
    with InProcessLoopbackServer() as srv:
        yield srv


def disconnects(recorder):
    return recorder.names().count("disconnect")


def wait_until(predicate, what):
    deadline = time.monotonic() + TIMEOUT_S
    while not predicate():
        assert time.monotonic() < deadline, f"no {what} within {TIMEOUT_S}s"
        time.sleep(0.01)


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_disconnect_does_not_wait_for_connection_opened_during_it(server, product):
    ws = product_ws(server.url, product)
    recorder = Recorder(ws)
    ws.connect()
    server.hold_next_close()
    seen_on_return = []

    def disconnect():
        ws.disconnect()
        seen_on_return.append(disconnects(recorder))

    closing = threading.Thread(target=disconnect, daemon=True)
    closing.start()
    try:
        assert server.wait_close_held(TIMEOUT_S), "the server never got the Close"
        ws.connect()
        # The connect() came while the disconnect() was closing.
        assert closing.is_alive()
        server.release_close()
        closing.join(TIMEOUT_S)
        assert not closing.is_alive(), "disconnect() waited for the connection opened after it"
        # Its own connection's reader was waited for (#54).
        assert seen_on_return == [1]
        assert ws.is_connected()
        # The new connection's reader was left to the disconnect() closing it.
        ws.disconnect()
        assert disconnects(recorder) == 2
    finally:
        server.release_close()
        disconnect_quietly(ws)
        closing.join(TIMEOUT_S)


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
async def test_disconnect_async_does_not_wait_for_connection_opened_during_it(server, product):
    ws = product_ws(server.url, product)
    recorder = Recorder(ws)
    await ws.connect_async()
    server.hold_next_close()
    seen_on_return = []

    async def disconnect():
        await ws.disconnect_async()
        seen_on_return.append(disconnects(recorder))

    closing = asyncio.ensure_future(disconnect())
    try:
        assert await to_thread(server.wait_close_held, TIMEOUT_S), "the server never got the Close"
        await ws.connect_async()
        assert not closing.done()
        server.release_close()
        done, _ = await asyncio.wait([closing], timeout=TIMEOUT_S)
        assert done, "disconnect_async() waited for the connection opened after it"
        assert seen_on_return == [1]
        assert ws.is_connected()
        await ws.disconnect_async()
        assert disconnects(recorder) == 2
    finally:
        server.release_close()
        try:
            await ws.disconnect_async()
        except Exception:
            pass
        await asyncio.wait([closing], timeout=TIMEOUT_S)


class LostConnectionCallback:
    """Keeps the `disconnect` callback of the first connection — fired when it
    is lost — running until the next connection's `disconnect` callback, then
    for up to RETURN_WAIT_S more, recording whether the `disconnect()` under
    test had returned meanwhile."""

    def __init__(self, ws):
        self._cond = threading.Condition()
        self._calls = 0
        self._finished = threading.Event()
        self.disconnect_returned = threading.Event()
        self.returned_before_it_finished = []
        ws.on("disconnect", self._on_disconnect)

    def _on_disconnect(self, *args):
        with self._cond:
            self._calls += 1
            self._cond.notify_all()
            if self._calls > 1:
                return
            self._cond.wait_for(lambda: self._calls > 1, timeout=TIMEOUT_S)
        self.returned_before_it_finished.append(self.disconnect_returned.wait(RETURN_WAIT_S))
        self._finished.set()

    def wait_running(self):
        with self._cond:
            assert self._cond.wait_for(lambda: self._calls > 0, timeout=TIMEOUT_S), (
                "the connection was not lost"
            )

    def assert_waited_for(self):
        """Call once the `disconnect()` has returned."""
        self.disconnect_returned.set()
        assert self._finished.wait(TIMEOUT_S)
        assert self.returned_before_it_finished == [False], (
            "disconnect() returned before the replaced connection's reader finished"
        )

    def release(self):
        with self._cond:
            self._calls += 1
            self._cond.notify_all()
        self.disconnect_returned.set()


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_disconnect_waits_for_reader_of_replaced_lost_connection(server, product):
    ws = product_ws(server.url, product)
    lost = LostConnectionCallback(ws)
    ws.connect()
    try:
        server.drop_connections()
        lost.wait_running()
        # Replaces the lost connection while its reader is still busy.
        ws.connect()
        ws.disconnect()
        lost.assert_waited_for()
    finally:
        lost.release()
        disconnect_quietly(ws)


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
async def test_disconnect_async_waits_for_reader_of_replaced_lost_connection(server, product):
    ws = product_ws(server.url, product)
    lost = LostConnectionCallback(ws)
    await ws.connect_async()
    try:
        server.drop_connections()
        await to_thread(lost.wait_running)
        await ws.connect_async()
        await ws.disconnect_async()
        await to_thread(lost.assert_waited_for)
    finally:
        lost.release()
        try:
            await ws.disconnect_async()
        except Exception:
            pass


def connect_replacing_lost(ws):
    """connect() once the lost connection is seen closed; 2011 until then."""
    deadline = time.monotonic() + TIMEOUT_S
    while True:
        try:
            ws.connect()
            return
        except WebSocketError as e:
            if e.code != 2011 or time.monotonic() > deadline:
                raise
        time.sleep(0.01)


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_connect_replacing_lost_connection_keeps_its_queued_messages(product):
    with InProcessLoopbackServer(flood=True) as srv:
        ws = product_ws(srv.url, product, message_buffer=BUFFER)
        lost_reported = threading.Event()
        ws.on("disconnect", lambda *args: lost_reported.set())
        ws.connect()
        try:
            unread = ws.messages()
            ws.subscribe({"channel": "trades", "symbol": "2330"})
            # The iterator's buffer is full, and so is core's queue behind it.
            wait_until(lambda: ws.messages_dropped_total() > 0, "dropped messages")
            srv.drop_connections()
            replacing = threading.Thread(target=connect_replacing_lost, args=(ws,))
            replacing.start()
            replacing.join(TIMEOUT_S)
            assert ws.is_connected()
            # The lost connection's reader still waits for the iterator to make
            # room: its `disconnect` callback comes after the messages it holds.
            assert not lost_reported.wait(RETURN_WAIT_S), (
                "the replaced connection's reader dropped the messages it held"
            )
            read = []
            drain = threading.Thread(target=lambda: read.extend(unread), daemon=True)
            drain.start()
            drain.join(TIMEOUT_S)
            assert not drain.is_alive()
            # Both buffers were full: the iterator's, and core's behind it. What
            # core still held was handed over too, not dropped.
            assert len(read) >= 2 * BUFFER
        finally:
            disconnect_quietly(ws)


async def connect_async_replacing_lost(ws):
    """connect_async() once the lost connection is seen closed."""
    deadline = time.monotonic() + TIMEOUT_S
    while True:
        try:
            await ws.connect_async()
            return
        except WebSocketError as e:
            if e.code != 2011 or time.monotonic() > deadline:
                raise
        await asyncio.sleep(0.01)


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
async def test_connect_async_replacing_lost_connection_keeps_its_queued_messages(product):
    with InProcessLoopbackServer(flood=True) as srv:
        ws = product_ws(srv.url, product, message_buffer=BUFFER)
        lost_reported = threading.Event()
        ws.on("disconnect", lambda *args: lost_reported.set())
        await ws.connect_async()
        try:
            unread = ws.messages()
            await ws.subscribe_async({"channel": "trades", "symbol": SYMBOLS[product]})
            await to_thread(wait_until, lambda: ws.messages_dropped_total() > 0, "dropped messages")
            srv.drop_connections()
            await connect_async_replacing_lost(ws)
            assert ws.is_connected()
            # See the blocking version.
            assert not await to_thread(lost_reported.wait, RETURN_WAIT_S), (
                "the replaced connection's reader dropped the messages it held"
            )
            read = await asyncio.wait_for(to_thread(list, unread), TIMEOUT_S)
            assert len(read) >= 2 * BUFFER
        finally:
            try:
                await ws.disconnect_async()
            except Exception:
                pass


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
@pytest.mark.parametrize("new_reader_event", ["connect", "authenticated"])
def test_readers_disconnecting_from_callbacks_do_not_wait_for_each_other(
    server, product, new_reader_event
):
    # The lost connection's reader connects again from its `disconnect`
    # callback, then disconnects. The new connection's reader disconnects from
    # its `connect` callback — during the handshake, so it aborts that
    # connect — or from `authenticated`, before or after the connect installs
    # the connection.
    ws = product_ws(server.url, product)
    lock = threading.Lock()
    calls = {"disconnect": 0, new_reader_event: 0}
    lost_done, new_done = threading.Event(), threading.Event()

    def nth(event):
        with lock:
            calls[event] += 1
            return calls[event]

    def on_disconnect(*args):
        if nth("disconnect") != 1:
            return
        try:
            ws.connect()
        except WebSocketError:
            pass  # 2010: the new reader's disconnect() aborted it
        ws.disconnect()
        lost_done.set()

    def on_new_reader_event(*args):
        if nth(new_reader_event) == 2:
            ws.disconnect()
            new_done.set()

    ws.on("disconnect", on_disconnect)
    ws.on(new_reader_event, on_new_reader_event)
    ws.connect()
    try:
        server.drop_connections()
        assert lost_done.wait(TIMEOUT_S) and new_done.wait(TIMEOUT_S), (
            "stream readers disconnecting from callbacks waited for each other"
        )
    finally:
        # Waits for the readers the callbacks' disconnect() left.
        closing = threading.Thread(target=disconnect_quietly, args=(ws,), daemon=True)
        closing.start()
        closing.join(TIMEOUT_S)
    assert not closing.is_alive()


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_reader_disconnecting_while_a_failed_connect_waits_for_it(server, product):
    # The lost connection's reader connects again from its `disconnect`
    # callback, and the credentials are rejected: that connect() waits for
    # the new connection's reader before it raises. The new reader
    # disconnects from its `unauthenticated` callback, closing the lost
    # connection — whose reader is the one waiting for it.
    #
    # The new reader's `connect` callback holds it until the connect() has
    # failed, the order that used to hang both. The other order passes
    # before and after the fix, so the wait only makes the test able to fail.
    ws = product_ws(server.url, product)
    lock = threading.Lock()
    calls = {"disconnect": 0, "connect": 0, "unauthenticated": 0}
    connect_errors = []
    lost_done, new_done = threading.Event(), threading.Event()

    def nth(event):
        with lock:
            calls[event] += 1
            return calls[event]

    def on_disconnect(*args):
        if nth("disconnect") != 1:
            return
        server.reject_auth()
        try:
            ws.connect()
        except Exception as e:
            connect_errors.append(e)
        lost_done.set()

    def on_connect(*args):
        if nth("connect") == 2:
            time.sleep(REJECTED_READER_BUSY_S)

    def on_unauthenticated(*args):
        if nth("unauthenticated") == 1:
            ws.disconnect()
            new_done.set()

    ws.on("disconnect", on_disconnect)
    ws.on("connect", on_connect)
    ws.on("unauthenticated", on_unauthenticated)
    ws.connect()
    try:
        server.drop_connections()
        assert lost_done.wait(TIMEOUT_S) and new_done.wait(TIMEOUT_S), (
            "stream readers disconnecting from callbacks waited for each other"
        )
        assert len(connect_errors) == 1
    finally:
        server.reject_auth(False)
        closing = threading.Thread(target=disconnect_quietly, args=(ws,), daemon=True)
        closing.start()
        closing.join(TIMEOUT_S)
    assert not closing.is_alive()


def disconnect_async_blocking(ws):
    """Wait for disconnect_async() synchronously, as from a callback."""

    async def close():
        await ws.disconnect_async()

    asyncio.run(close())


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
@pytest.mark.parametrize("event", ["connect", "authenticated"])
def test_disconnect_async_waited_for_from_callback_waits_for_no_reader(server, product, event):
    # The awaitable used to join, on a runtime thread, the reader of the
    # connection it closed — the reader waiting for it in the callback (#280).
    # `connect` fires during the handshake, so it aborts that connect.
    ws = product_ws(server.url, product)
    recorder = Recorder(ws)
    calls = []
    errors = []
    returned = threading.Event()

    def on_event(*args):
        calls.append(event)
        if len(calls) != 1:
            return
        try:
            disconnect_async_blocking(ws)
        except Exception as e:
            errors.append(e)
        returned.set()

    ws.on(event, on_event)
    try:
        ws.connect()
    except WebSocketError:
        pass  # 2010: the callback's disconnect_async() aborted it
    try:
        assert returned.wait(TIMEOUT_S), "disconnect_async() waited for the reader waiting for it"
        assert errors == []
        assert not ws.is_connected()
    finally:
        # Waits for the reader the callback's disconnect_async() left: its
        # `disconnect` callback has fired once this returns.
        closing = threading.Thread(target=disconnect_async_blocking, args=(ws,), daemon=True)
        closing.start()
        closing.join(TIMEOUT_S)
    assert not closing.is_alive()
    assert disconnects(recorder) == 1
