"""Own reconnect code alongside auto-reconnect is warned about (#226).

2.x code often recovers a lost connection from a background thread:
``disconnect()`` then ``connect()`` after the ``disconnect`` callback set a
flag. With auto-reconnect on, that ``disconnect()`` closes the connection the
SDK has just restored, which fires ``disconnect`` again: the two keep
triggering each other. The SDK cannot tell that code apart from a deliberate
close, so it only warns: a ``RuntimeWarning`` (not an ``error`` callback),
once per client, when ``disconnect()`` closes a connection an automatic
reconnect restored less than 30 seconds earlier.
"""
import threading
import time
import warnings

import pytest

from fugle_marketdata import ReconnectConfig, WebSocketClient
from tests.ws_loopback import TIMEOUT_S, InProcessLoopbackServer, Recorder, disconnect_quietly

hard_timeout = pytest.mark.timeout(30, method="thread")


@pytest.fixture
def server():
    with InProcessLoopbackServer() as srv:
        yield srv


def reconnecting_stock(url):
    client = WebSocketClient(
        api_key="test-key",
        base_url=url,
        reconnect=ReconnectConfig(enabled=True, initial_delay_ms=100, max_delay_ms=100),
    )
    return client.stock


def conflict_warnings(caught):
    return [
        w
        for w in caught
        if issubclass(w.category, RuntimeWarning) and "automatic reconnect" in str(w.message)
    ]


@hard_timeout
def test_own_reconnect_thread_is_warned_about_once(server):
    ws = reconnecting_stock(server.url)
    recorder = Recorder(ws)
    reconnect_needed = threading.Event()
    rounds = []
    stop = threading.Event()

    def reconnect_loop():
        # The 2.x pattern from #226, for a few rounds. The sleep lets the
        # automatic reconnect (100 ms) finish first, as the 2 s of #226 did.
        while not stop.is_set():
            if not reconnect_needed.wait(0.05):
                continue
            reconnect_needed.clear()
            time.sleep(0.5)
            ws.disconnect()
            ws.connect()
            rounds.append(1)
            if len(rounds) == 1:
                # Lose the connection of the next core client too: its
                # automatic reconnect is closed the same way, and the
                # warning must not repeat.
                server.drop_connections()

    ws.on("disconnect", lambda code, message: reconnect_needed.set())
    thread = threading.Thread(target=reconnect_loop, daemon=True)
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        try:
            ws.connect()
            thread.start()
            server.drop_connections()
            # Lost, reconnected, then closed and reopened by the thread: the
            # loop goes on with a new client each round.
            deadline = time.monotonic() + TIMEOUT_S * 2
            while len(rounds) < 3:
                assert time.monotonic() < deadline, f"only {len(rounds)} rounds"
                time.sleep(0.05)
        finally:
            stop.set()
            thread.join(TIMEOUT_S)
            disconnect_quietly(ws)

    # Two connections restored by automatic reconnect were closed.
    assert server.connections_accepted >= 5, server.connections_accepted
    found = conflict_warnings(caught)
    assert len(found) == 1, [str(w.message) for w in caught]
    message = str(found[0].message)
    assert "ReconnectConfig.disabled()" in message, message
    # Not reported through the `error` callback.
    assert [args for args in recorder.args_of("error") if args[0].code == 3006] == []


@hard_timeout
def test_disconnect_of_a_connection_the_caller_opened_is_not_warned_about(server):
    ws = reconnecting_stock(server.url)
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        ws.connect()
        ws.disconnect()
        ws.connect()
        ws.disconnect()
    assert conflict_warnings(caught) == []
