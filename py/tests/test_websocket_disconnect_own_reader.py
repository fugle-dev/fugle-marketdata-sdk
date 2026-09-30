"""``disconnect()`` waits for the stream reader of the connection it closed (#277).

It waits for that reader so the ``disconnect`` callback has fired by the time
it returns (#54). The binding kept the reader in one slot per client, which a
``connect()`` from another thread overwrote while the ``disconnect()`` was
still closing: the ``disconnect()`` then waited for the new connection's
reader, until that connection was closed too, and the old reader was waited
for by no one.

The server holds its answer to the Close, so the ``connect()`` lands inside
the ``disconnect()`` every time. Only the stock client has the async methods.

A ``connect()`` that replaces a connection that was lost leaves that
connection's reader for the next ``disconnect()`` to wait for.
"""
import asyncio
import threading
import time

import pytest

from tests.ws_loopback import (
    TIMEOUT_S,
    InProcessLoopbackServer,
    Recorder,
    disconnect_quietly,
    product_ws,
    to_thread,
)

PRODUCTS = [pytest.param("stock", id="stock"), pytest.param("futopt", id="futopt")]

hard_timeout = pytest.mark.timeout(30, method="thread")

# How long the lost connection's `disconnect` callback stays busy after it is
# released. Not what orders the calls: see the test.
BUSY_S = 0.3


@pytest.fixture
def server():
    with InProcessLoopbackServer() as srv:
        yield srv


def disconnects(recorder):
    return recorder.names().count("disconnect")


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
async def test_disconnect_async_does_not_wait_for_connection_opened_during_it(server):
    ws = product_ws(server.url, "stock")
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


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_disconnect_waits_for_reader_of_replaced_lost_connection(server, product):
    ws = product_ws(server.url, product)
    calls = []
    cond = threading.Condition()
    lost_callback_returned = threading.Event()

    def on_disconnect(*args):
        with cond:
            first = not calls
            calls.append(args)
            cond.notify_all()
            if first:
                # The lost connection's reader stays in this callback until
                # the next connection's `disconnect` callback runs.
                cond.wait_for(lambda: len(calls) > 1, timeout=TIMEOUT_S)
        if first:
            # Still busy once released. Only makes a disconnect() that does
            # not wait for this reader return first; one that does waits
            # however long this takes.
            time.sleep(BUSY_S)
            lost_callback_returned.set()

    ws.on("disconnect", on_disconnect)
    ws.connect()
    try:
        server.drop_connections()
        with cond:
            assert cond.wait_for(lambda: calls, timeout=TIMEOUT_S), "the connection was not lost"
        # Replaces the lost connection while its reader is still busy.
        ws.connect()
        ws.disconnect()
        assert lost_callback_returned.is_set(), (
            "disconnect() returned before the replaced connection's reader finished"
        )
    finally:
        with cond:
            calls.append(None)
            cond.notify_all()
        disconnect_quietly(ws)
