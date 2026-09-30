"""Blocking ``connect()`` racing ``disconnect()`` on a connected client (#271).

``connect()`` decides what to do with the GIL released, so a ``disconnect()``
can take the connection and the runtime in between. The connect then opens a
new connection, as when the disconnect came first; it used to raise
``RuntimeError("Runtime not initialized")``.

Whichever call wins, ``connect()`` either succeeds or is refused with a
``WebSocketError``: 2011 while the connection is still stored, 2010 when the
disconnect aborts the connection it is opening.
"""
import threading

import pytest

from fugle_marketdata import WebSocketError
from tests.ws_loopback import InProcessLoopbackServer, disconnect_quietly, product_ws

PRODUCTS = [pytest.param("stock", id="stock"), pytest.param("futopt", id="futopt")]

hard_timeout = pytest.mark.timeout(120, method="thread")

# The disconnect lands in the window in about 1 round in 100.
ROUNDS = 1000


@pytest.fixture
def server():
    with InProcessLoopbackServer() as srv:
        yield srv


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_connect_racing_disconnect_succeeds_or_is_refused(server, product):
    ws = product_ws(server.url, product)
    unexpected = []
    try:
        for _ in range(ROUNDS):
            ws.connect()
            start = threading.Barrier(2)

            def connect():
                start.wait()
                try:
                    ws.connect()
                except WebSocketError as e:
                    if e.code not in (2010, 2011):
                        unexpected.append(e)
                except Exception as e:
                    unexpected.append(e)

            def disconnect():
                start.wait()
                ws.disconnect()

            # The last thread to reach the barrier runs first: connect(), so
            # disconnect() takes over where connect() releases the GIL.
            threads = [threading.Thread(target=f, daemon=True) for f in (disconnect, connect)]
            for thread in threads:
                thread.start()
            for thread in threads:
                thread.join(1)
            # Closes what the connect opened, if it came second.
            ws.disconnect()
            for thread in threads:
                thread.join(10)
            assert unexpected == []
    finally:
        disconnect_quietly(ws)
