"""Minimal loopback WebSocket server for tests that need no API key.

Standard library only, so CI needs no extra dependency. It speaks just enough
RFC 6455 for the SDK client: the opening handshake, unfragmented text frames
from the client (masked) and to it (unmasked), ping/pong and close.

The protocol mirrors ``js/tests/ws-worker.test.js``: ``auth`` is acked with
``authenticated``, or answered with ``error`` for ``REJECTED_API_KEY``, or with a
Close frame (1001, ``LIMIT_CLOSE_REASON``) for ``LIMITED_API_KEY``, (1013) for
``LIMITED_1013_API_KEY``, (1001, ``RESTART_CLOSE_REASON``) for ``RESTARTING_API_KEY``; ``subscribe`` is answered with ``subscribed`` plus one
``data`` frame. The ``subscribed`` ack echoes ``intradayOddLot`` / ``afterHours``
and marks them in the id, as ``<channel>-<symbol>[-odd][-ah]``. With ``flood`` it keeps sending ``data`` frames until the peer
closes. With ``burst_on_close`` it answers the client's Close with that many
``data`` frames before its own Close: frames written after ``disconnect()``
started, and read by the client before the close completes.

For reconnect tests the in-process server counts the connections it accepts,
can cut them without a Close frame (``drop_connections()``), refuse new ones
(``refuse_connections``) or reject every ``auth`` (``reject_auth``), and logs
each ``subscribe`` with the index of the connection it came on. It can also
hold its answer to the next client Close (``hold_next_close()``) until the
test releases it, keeping that ``disconnect()`` in its close meanwhile, and
close every open connection itself with a Close frame
(``close_connections(code, reason)``).

``LoopbackServer`` runs the server in a child process; ``InProcessLoopbackServer``
runs it on threads of the test process, which only works while the blocking
client calls release the GIL (#39).

``is_debug_build()`` tells whether the installed binding has the
``FUGLE_MARKETDATA_TEST_PANIC`` injection sites, by triggering one: only a
call that returns normally counts as a release build, and an exception other
than the injected panic is raised, not taken for one.
"""
import asyncio
import base64
import hashlib
import json
import os
import socket
import struct
import subprocess
import sys
import threading

_GUID = b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11"

# ``auth`` with this API key is rejected the way the Fugle server does it.
REJECTED_API_KEY = "rejected-key"
# ``auth`` with this API key is never answered, so a ``connect()`` stays in
# the handshake until the client gives up.
SILENT_API_KEY = "silent-key"

# Authenticated with a bare `{"event":"authenticated"}`, no `data` (#304).
BARE_AUTH_API_KEY = "bare-auth-key"
# ``auth`` with this API key is answered with a Close frame, the way the
# server refuses a connection over its limit (#292).
LIMITED_API_KEY = "limited-key"
LIMIT_CLOSE_REASON = "Maximum number of connections reached"
# The limit Close of a server with fugle-realtime !640 (#300).
LIMITED_1013_API_KEY = "limited-1013-key"
# `1001` with another reason: a restart, not the limit (#300).
RESTARTING_API_KEY = "restarting-key"
RESTART_CLOSE_REASON = "Server restarting"
# `auth` answered with `error{1003}` then a Close: the limit as a server with
# fugle-realtime !640 says it (Close 1013), and a failed validation, which has
# the same code and another message (Close without a code) (#300).
LIMIT_ERROR_API_KEY = "limit-error-key"
VALIDATION_ERROR_API_KEY = "validation-error-key"
VALIDATION_ERROR_MESSAGE = "apikey should not be empty"
# What each of those keys' `auth` is answered with: (code, message, close code).
AUTH_ERRORS = {
    LIMIT_ERROR_API_KEY: (1003, LIMIT_CLOSE_REASON, 1013),
    VALIDATION_ERROR_API_KEY: (1003, VALIDATION_ERROR_MESSAGE, None),
}
# What each of those keys' `auth` is answered with: (close code, reason).
AUTH_CLOSES = {
    LIMITED_API_KEY: (1001, LIMIT_CLOSE_REASON),
    LIMITED_1013_API_KEY: (1013, ""),
    RESTARTING_API_KEY: (1001, RESTART_CLOSE_REASON),
}

OP_TEXT = 0x1
OP_CLOSE = 0x8
OP_PING = 0x9
OP_PONG = 0xA


def _recv_exact(conn, n):
    buf = b""
    while len(buf) < n:
        chunk = conn.recv(n - len(buf))
        if not chunk:
            raise ConnectionError("peer closed")
        buf += chunk
    return buf


def _handshake(conn):
    request = b""
    while b"\r\n\r\n" not in request:
        chunk = conn.recv(4096)
        if not chunk:
            raise ConnectionError("peer closed during handshake")
        request += chunk
    key = None
    for line in request.split(b"\r\n")[1:]:
        name, _, value = line.partition(b":")
        if name.strip().lower() == b"sec-websocket-key":
            key = value.strip()
    if key is None:
        raise ConnectionError("missing Sec-WebSocket-Key")
    accept = base64.b64encode(hashlib.sha1(key + _GUID).digest())
    conn.sendall(
        b"HTTP/1.1 101 Switching Protocols\r\n"
        b"Upgrade: websocket\r\n"
        b"Connection: Upgrade\r\n"
        b"Sec-WebSocket-Accept: " + accept + b"\r\n\r\n"
    )


def _read_frame(conn):
    b0, b1 = _recv_exact(conn, 2)
    opcode = b0 & 0x0F
    length = b1 & 0x7F
    if length == 126:
        (length,) = struct.unpack("!H", _recv_exact(conn, 2))
    elif length == 127:
        (length,) = struct.unpack("!Q", _recv_exact(conn, 8))
    mask = _recv_exact(conn, 4) if b1 & 0x80 else None
    payload = _recv_exact(conn, length)
    if mask:
        payload = bytes(b ^ mask[i % 4] for i, b in enumerate(payload))
    return opcode, payload


def _send_frame(conn, opcode, payload=b""):
    header = bytes([0x80 | opcode])
    length = len(payload)
    if length < 126:
        header += bytes([length])
    elif length < 1 << 16:
        header += bytes([126]) + struct.pack("!H", length)
    else:
        header += bytes([127]) + struct.pack("!Q", length)
    conn.sendall(header + payload)


class _Server:
    """Fugle-shaped WebSocket server on ``127.0.0.1`` with an ephemeral port."""

    def __init__(self, flood=False, burst_on_close=0):
        self._flood = flood
        self._burst_on_close = burst_on_close
        self._stopped = threading.Event()
        # ``data`` of every ``auth`` frame received, in arrival order.
        self.auth_data = []
        # ``data`` of every ``subscribe`` frame received, in arrival order.
        self.subscribe_data = []
        # ``data`` of every ``unsubscribe`` frame received, in arrival order.
        self.unsubscribe_data = []
        # ``data`` of every ``ping`` frame received, in arrival order.
        self.ping_data = []
        # Connections accepted so far, refused ones included.
        self.connections_accepted = 0
        # ``(connection index, symbol)`` of every ``subscribe``, in arrival order.
        self.subscribe_log = []
        # Close every new connection at once, so reconnect attempts fail.
        self.refuse_connections = False
        # Answer every ``auth`` with the rejection, whatever the key.
        self.reject_auth = False
        # Hold the answer to the next client Close until ``release_close()``.
        self._hold_close = False
        self.close_held = threading.Event()
        self._close_released = threading.Event()
        self._open_conns = []
        # The send function of each open connection, for ``close_connections``.
        self._senders = {}
        self._conns_lock = threading.Lock()
        self._listener = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self._listener.bind(("127.0.0.1", 0))
        self._listener.listen()
        # A blocked accept() is not reliably woken by close(); poll instead.
        self._listener.settimeout(0.1)
        self.port = self._listener.getsockname()[1]

    def start(self):
        threading.Thread(target=self._accept_loop, daemon=True).start()

    def stop(self):
        self._stopped.set()
        self._close_released.set()

    def hold_next_close(self):
        with self._conns_lock:
            self._hold_close = True
            self.close_held.clear()
            self._close_released.clear()

    def release_close(self):
        self._close_released.set()

    def _take_close_hold(self):
        with self._conns_lock:
            hold, self._hold_close = self._hold_close, False
            return hold

    def _accept_loop(self):
        with self._listener:
            while not self._stopped.is_set():
                try:
                    conn, _ = self._listener.accept()
                except socket.timeout:
                    continue
                conn.settimeout(None)
                with self._conns_lock:
                    index = self.connections_accepted
                    self.connections_accepted += 1
                    if self.refuse_connections:
                        conn.close()
                        continue
                    self._open_conns.append(conn)
                threading.Thread(target=self._serve, args=(conn, index), daemon=True).start()

    def drop_connections(self):
        """Cut every open connection without a Close frame: the client sees
        the transport end (1006) and, if enabled, reconnects."""
        with self._conns_lock:
            conns, self._open_conns = self._open_conns, []
        for conn in conns:
            try:
                conn.shutdown(socket.SHUT_RDWR)
            except OSError:
                pass

    def close_connections(self, code=None, reason=""):
        """Send a Close frame (with ``code`` and ``reason``, or neither) on
        every open connection: the client sees the server close it."""
        payload = b"" if code is None else struct.pack("!H", code) + reason.encode()
        with self._conns_lock:
            senders = list(self._senders.values())
        for send in senders:
            try:
                send(OP_CLOSE, payload, server_close=True)
            except OSError:
                pass

    def _serve(self, conn, index=0):
        send_lock = threading.Lock()
        closed = threading.Event()
        # Set once this side sent its Close: the client's Close is the answer.
        server_closed = threading.Event()

        def send(opcode, payload=b"", server_close=False):
            with send_lock:
                if server_closed.is_set():
                    return
                if server_close:
                    server_closed.set()
                _send_frame(conn, opcode, payload)

        try:
            _handshake(conn)
            with self._conns_lock:
                self._senders[conn] = send
            while not self._stopped.is_set():
                opcode, payload = _read_frame(conn)
                if opcode == OP_TEXT:
                    frame = json.loads(payload)
                    if frame.get("event") == "auth":
                        self.auth_data.append(frame.get("data"))
                        apikey = (frame.get("data") or {}).get("apikey")
                        refused = apikey in AUTH_ERRORS or apikey in AUTH_CLOSES
                        if apikey in AUTH_ERRORS:
                            code, message, close_code = AUTH_ERRORS[apikey]
                            error = {"event": "error", "code": code, "data": {"message": message}}
                            send(OP_TEXT, json.dumps(error).encode())
                            close = b"" if close_code is None else struct.pack("!H", close_code)
                            send(OP_CLOSE, close)
                        elif apikey in AUTH_CLOSES:
                            code, reason = AUTH_CLOSES[apikey]
                            send(OP_CLOSE, struct.pack("!H", code) + reason.encode())
                        if refused:
                            # Until the client goes: its reply Close, which needs
                            # no answer, or the end of the stream, on which
                            # _read_frame raises.
                            while _read_frame(conn)[0] != OP_CLOSE:
                                pass
                            return
                    if frame.get("event") == "unsubscribe":
                        self.unsubscribe_data.append(frame.get("data"))
                    if frame.get("event") == "ping":
                        self.ping_data.append(frame.get("data"))
                    if frame.get("event") == "subscribe":
                        self.subscribe_data.append(frame.get("data"))
                        symbol = (frame.get("data") or {}).get("symbol")
                        self.subscribe_log.append((index, symbol))
                    for reply in self._replies(frame):
                        send(OP_TEXT, json.dumps(reply).encode())
                    if self._flood and frame.get("event") == "subscribe":
                        threading.Thread(
                            target=self._flood_data, args=(send, closed, frame), daemon=True
                        ).start()
                elif opcode == OP_PING:
                    send(OP_PONG, payload)
                elif opcode == OP_CLOSE and server_closed.is_set():
                    return
                elif opcode == OP_CLOSE:
                    if self._take_close_hold():
                        self.close_held.set()
                        self._close_released.wait()
                    closed.set()
                    for i in range(self._burst_on_close):
                        data = {"event": "data", "data": {"i": i}, "channel": "trades"}
                        send(OP_TEXT, json.dumps(data).encode())
                    send(OP_CLOSE, payload[:2])
                    return
        except (ConnectionError, OSError, ValueError):
            return
        finally:
            closed.set()
            with self._conns_lock:
                if conn in self._open_conns:
                    self._open_conns.remove(conn)
                self._senders.pop(conn, None)
            conn.close()

    def _flood_data(self, send, closed, frame):
        reply = self._replies(frame)[-1]
        payload = json.dumps(reply).encode()
        try:
            while not closed.is_set() and not self._stopped.is_set():
                send(OP_TEXT, payload)
        except OSError:
            return

    def _replies(self, frame):
        event = frame.get("event")
        if event == "auth":
            if (frame.get("data") or {}).get("apikey") == SILENT_API_KEY:
                return []
            if self.reject_auth or (frame.get("data") or {}).get("apikey") == REJECTED_API_KEY:
                # The server's rejection shape: `code` 1000 at the top level (#201).
                return [{"event": "error", "code": 1000, "data": {"message": "Invalid authentication credentials"}}]
            if (frame.get("data") or {}).get("apikey") == BARE_AUTH_API_KEY:
                return [{"event": "authenticated"}]
            return [{"event": "authenticated", "data": {"message": "Authenticated successfully"}}]
        if event == "subscribe":
            data = frame.get("data") or {}
            channel, symbol = data.get("channel"), data.get("symbol")
            modifiers = {k: True for k in ("intradayOddLot", "afterHours") if data.get(k)}
            sub_id = f"{channel}-{symbol}"
            sub_id += "-odd" if "intradayOddLot" in modifiers else ""
            sub_id += "-ah" if "afterHours" in modifiers else ""
            return [
                {
                    "event": "subscribed",
                    "data": {"id": sub_id, "channel": channel, "symbol": symbol, **modifiers},
                },
                {"event": "data", "data": {"symbol": symbol, "price": 100}, "id": sub_id, "channel": channel},
            ]
        if event == "ping":
            # Like the server: echo `state`, with the server's time.
            state = (frame.get("data") or {}).get("state")
            return [{"event": "pong", "data": {"time": 0, "state": state}}]
        return []


class LoopbackServer:
    """Run ``_Server`` in a child process for the duration of a ``with`` block.

    ``url`` is the ``base_url`` to hand the client.
    """

    def __init__(self, flood=False, burst_on_close=0):
        self._flood = flood
        self._burst_on_close = burst_on_close
        self._proc = None
        self.url = None

    def __enter__(self):
        args = [sys.executable, os.path.abspath(__file__)]
        if self._flood:
            args.append("--flood")
        if self._burst_on_close:
            args.append(f"--burst-on-close={self._burst_on_close}")
        self._proc = subprocess.Popen(
            args,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            text=True,
        )
        port = self._proc.stdout.readline().strip()
        if not port:
            self.__exit__()
            raise RuntimeError("loopback server failed to start")
        self.url = f"ws://127.0.0.1:{port}"
        return self

    def __exit__(self, *exc):
        # Closing stdin tells the child to stop.
        self._proc.stdin.close()
        try:
            self._proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            self._proc.kill()
            self._proc.wait()
        self._proc.stdout.close()


class InProcessLoopbackServer:
    """Run ``_Server`` on threads of this process for a ``with`` block."""

    def __init__(self, flood=False):
        self._server = _Server(flood=flood)
        self.url = f"ws://127.0.0.1:{self._server.port}"

    @property
    def auth_data(self):
        """``data`` of every ``auth`` frame the server received."""
        return list(self._server.auth_data)

    @property
    def subscribe_data(self):
        """``data`` of every ``subscribe`` frame the server received."""
        return list(self._server.subscribe_data)

    @property
    def unsubscribe_data(self):
        """``data`` of every ``unsubscribe`` frame the server received."""
        return list(self._server.unsubscribe_data)

    @property
    def ping_data(self):
        """``data`` of every ``ping`` frame the server received."""
        return list(self._server.ping_data)

    @property
    def connections_accepted(self):
        """Connections the server accepted, refused ones included."""
        return self._server.connections_accepted

    @property
    def subscribe_log(self):
        """``(connection index, symbol)`` of every ``subscribe`` received."""
        return list(self._server.subscribe_log)

    def drop_connections(self):
        """Cut every open connection without a Close frame."""
        self._server.drop_connections()

    def close_connections(self, code=None, reason=""):
        """Close every open connection from the server with a Close frame."""
        self._server.close_connections(code, reason)

    def refuse_connections(self, refuse=True):
        """Close new connections at once, so reconnect attempts fail."""
        self._server.refuse_connections = refuse

    def reject_auth(self, reject=True):
        """Reject every ``auth`` from now on."""
        self._server.reject_auth = reject

    def hold_next_close(self):
        """Answer the next client Close only once ``release_close()`` is
        called; the client's ``disconnect()`` waits for that answer."""
        self._server.hold_next_close()

    def wait_close_held(self, timeout):
        """Wait until a Close is being held; whether one is."""
        return self._server.close_held.wait(timeout)

    def release_close(self):
        """Answer the held Close."""
        self._server.release_close()

    def __enter__(self):
        self._server.start()
        return self

    def __exit__(self, *exc):
        self._server.stop()


# --- Client-side helpers shared by the loopback tests. The SDK is imported
# inside the functions, so running this file as the server needs only the
# standard library.

TIMEOUT_S = 5


class Recorder:
    """Records ``(event, args)`` for every connection callback, and with
    ``messages`` for the ``message`` callback too, in delivery order."""

    EVENTS = ("connect", "authenticated", "unauthenticated", "disconnect", "reconnect", "error")

    def __init__(self, ws, messages=False):
        self.calls = []
        self._cond = threading.Condition()
        for event in self.EVENTS + (("message",) if messages else ()):
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

    def wait_until(self, predicate, timeout, what):
        """Wait until ``predicate(calls)`` holds."""
        with self._cond:
            hit = self._cond.wait_for(lambda: predicate(self.calls), timeout=timeout)
        assert hit, f"no {what} within {timeout}s; got {len(self.calls)} calls"


def product_ws(url, product, api_key="test-key", **kwargs):
    """The ``product`` client of a WebSocketClient for ``url``, reconnect off."""
    from fugle_marketdata import ReconnectConfig, WebSocketClient

    client = WebSocketClient(
        api_key=api_key, base_url=url, reconnect=ReconnectConfig.disabled(), **kwargs
    )
    return getattr(client, product)


def disconnect_quietly(ws):
    try:
        ws.disconnect()
    except Exception:
        pass  # never connected, or already gone


async def to_thread(func, *args):
    """``asyncio.to_thread``, which Python 3.8 lacks."""
    return await asyncio.get_running_loop().run_in_executor(None, func, *args)


# Names the injection site where a debug build of the binding panics on demand.
PANIC_ENV = "FUGLE_MARKETDATA_TEST_PANIC"

_debug_build = None


def is_debug_build():
    """Whether the installed binding has the test injection sites.

    Probed with ``ws_callback_poison``, the one site that needs no
    connection: a debug build panics in ``off()``, a release build returns.
    Any other exception is raised and nothing is cached, so the next call
    probes again.
    """
    global _debug_build
    if _debug_build is None:
        previous = os.environ.get(PANIC_ENV)
        os.environ[PANIC_ENV] = "ws_callback_poison"
        try:
            # The registry reads the variable when the client is created.
            product_ws("ws://127.0.0.1:9", "stock").off("reconnect")
            _debug_build = False
        except (KeyboardInterrupt, SystemExit):
            raise
        except BaseException as panic:  # PanicException is a BaseException
            # Anything but the injected panic is a failed probe, not a release
            # build: raise it rather than let the tests skip quietly.
            if "ws_callback_poison" not in str(panic):
                raise
            _debug_build = True
        finally:
            if previous is None:
                del os.environ[PANIC_ENV]
            else:
                os.environ[PANIC_ENV] = previous
    return _debug_build


if __name__ == "__main__":
    burst = next(
        (int(arg.split("=", 1)[1]) for arg in sys.argv[1:] if arg.startswith("--burst-on-close=")),
        0,
    )
    server = _Server(flood="--flood" in sys.argv[1:], burst_on_close=burst)
    server.start()
    print(server.port, flush=True)
    sys.stdin.read()
