"""WebSocket ``connect()`` on a client that is connected or still connecting.

Refused with ``WebSocketError`` code 2011 (``ALREADY_CONNECTED``), as core and
the other bindings do (#119, #130), and the live connection is left alone.
Only the stock client has ``connect_async()``.
"""
import asyncio
import socket
import threading

import pytest

from fugle_marketdata import MarketDataError, WebSocketError
from tests.ws_loopback import TIMEOUT_S, LoopbackServer, disconnect_quietly, product_ws

PRODUCTS = [pytest.param("stock", id="stock"), pytest.param("futopt", id="futopt")]

hard_timeout = pytest.mark.timeout(20, method="thread")


class StallingServer:
    """Accepts TCP connections and never answers the WebSocket handshake, so a
    ``connect()`` against it stays in progress until ``release()``."""

    def __init__(self):
        self._listener = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self._listener.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        self._listener.bind(("127.0.0.1", 0))
        self._listener.listen()
        self.url = f"ws://127.0.0.1:{self._listener.getsockname()[1]}"
        self.accepted = threading.Event()
        self._conns = []
        self._thread = threading.Thread(target=self._accept_loop, daemon=True)

    def _accept_loop(self):
        while True:
            try:
                conn, _ = self._listener.accept()
            except OSError:
                return
            self._conns.append(conn)
            self.accepted.set()

    def release(self):
        """Close every held connection, failing the pending handshake."""
        for conn in self._conns:
            conn.close()

    def __enter__(self):
        self._thread.start()
        return self

    def __exit__(self, *exc):
        self._listener.close()
        self.release()


def assert_already_connected(excinfo):
    err = excinfo.value
    assert isinstance(err, MarketDataError)
    assert err.code == 2011
    assert err.source_kind == "client"


@pytest.fixture
def server():
    with LoopbackServer() as srv:
        yield srv


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_connect_while_connected_raises_2011_and_keeps_connection(server, product):
    ws = product_ws(server.url, product)
    try:
        ws.connect()
        with pytest.raises(WebSocketError) as excinfo:
            ws.connect()
        assert_already_connected(excinfo)

        assert ws.is_connected()
        # The live connection still takes commands and delivers messages.
        ws.subscribe("trades", "TXF1!" if product == "futopt" else "2330")
        for msg in ws.messages():
            if msg.get("event") == "subscribed":
                break
    finally:
        disconnect_quietly(ws)


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_connect_while_connecting_raises_2011(product):
    with StallingServer() as stall:
        ws = product_ws(stall.url, product)
        first = {}

        def connect():
            try:
                ws.connect()
            except Exception as e:  # the released handshake fails
                first["error"] = e

        thread = threading.Thread(target=connect)
        thread.start()
        try:
            assert stall.accepted.wait(TIMEOUT_S), "first connect() never reached the server"
            with pytest.raises(WebSocketError) as excinfo:
                ws.connect()
            assert_already_connected(excinfo)
        finally:
            stall.release()
            thread.join(TIMEOUT_S)
        assert not thread.is_alive()
        # The first connect() failed on its own, not with 2011.
        assert "error" in first, "first connect() did not fail after release"
        assert getattr(first["error"], "code", None) != 2011


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_connect_after_disconnect_succeeds(server, product):
    ws = product_ws(server.url, product)
    try:
        ws.connect()
        ws.disconnect()
        ws.connect()
        assert ws.is_connected()
    finally:
        disconnect_quietly(ws)


@hard_timeout
async def test_connect_async_while_connected_raises_2011(server):
    ws = product_ws(server.url, "stock")
    try:
        await ws.connect_async()
        with pytest.raises(WebSocketError) as excinfo:
            await ws.connect_async()
        assert_already_connected(excinfo)
        # Sync and async connects share the check.
        with pytest.raises(WebSocketError) as excinfo:
            ws.connect()
        assert_already_connected(excinfo)
        assert ws.is_connected()
    finally:
        await ws.disconnect_async()


@hard_timeout
async def test_connect_async_while_connecting_raises_2011():
    with StallingServer() as stall:
        ws = product_ws(stall.url, "stock")
        first = asyncio.ensure_future(ws.connect_async())
        try:
            assert await asyncio.to_thread(stall.accepted.wait, TIMEOUT_S)
            with pytest.raises(WebSocketError) as excinfo:
                await ws.connect_async()
            assert_already_connected(excinfo)
        finally:
            stall.release()
            with pytest.raises(Exception) as first_err:
                await asyncio.wait_for(first, TIMEOUT_S)
        assert getattr(first_err.value, "code", None) != 2011
