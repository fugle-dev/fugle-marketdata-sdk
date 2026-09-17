"""Minimal loopback WebSocket server for tests that need no API key.

Standard library only, so CI needs no extra dependency. It speaks just enough
RFC 6455 for the SDK client: the opening handshake, unfragmented text frames
from the client (masked) and to it (unmasked), ping/pong and close.

The protocol mirrors ``js/tests/ws-worker.test.js``: ``auth`` is acked with
``authenticated``, or answered with ``error`` for ``REJECTED_API_KEY``; ``subscribe`` is answered with ``subscribed`` plus one
``data`` frame. The ``subscribed`` ack echoes ``intradayOddLot`` / ``afterHours``
and marks them in the id, as ``<channel>-<symbol>[-odd][-ah]``. With ``flood`` it keeps sending ``data`` frames until the peer
closes. With ``burst_on_close`` it answers the client's Close with that many
``data`` frames before its own Close: frames written after ``disconnect()``
started, and read by the client before the close completes.

``LoopbackServer`` runs the server in a child process; ``InProcessLoopbackServer``
runs it on threads of the test process, which only works while the blocking
client calls release the GIL (#39).
"""
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
        # ``data`` of every ``unsubscribe`` frame received, in arrival order.
        self.unsubscribe_data = []
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

    def _accept_loop(self):
        with self._listener:
            while not self._stopped.is_set():
                try:
                    conn, _ = self._listener.accept()
                except socket.timeout:
                    continue
                conn.settimeout(None)
                threading.Thread(target=self._serve, args=(conn,), daemon=True).start()

    def _serve(self, conn):
        send_lock = threading.Lock()
        closed = threading.Event()

        def send(opcode, payload=b""):
            with send_lock:
                _send_frame(conn, opcode, payload)

        try:
            _handshake(conn)
            while not self._stopped.is_set():
                opcode, payload = _read_frame(conn)
                if opcode == OP_TEXT:
                    frame = json.loads(payload)
                    if frame.get("event") == "auth":
                        self.auth_data.append(frame.get("data"))
                    if frame.get("event") == "unsubscribe":
                        self.unsubscribe_data.append(frame.get("data"))
                    for reply in self._replies(frame):
                        send(OP_TEXT, json.dumps(reply).encode())
                    if self._flood and frame.get("event") == "subscribe":
                        threading.Thread(
                            target=self._flood_data, args=(send, closed, frame), daemon=True
                        ).start()
                elif opcode == OP_PING:
                    send(OP_PONG, payload)
                elif opcode == OP_CLOSE:
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
            conn.close()

    def _flood_data(self, send, closed, frame):
        reply = self._replies(frame)[-1]
        payload = json.dumps(reply).encode()
        try:
            while not closed.is_set() and not self._stopped.is_set():
                send(OP_TEXT, payload)
        except OSError:
            return

    @staticmethod
    def _replies(frame):
        event = frame.get("event")
        if event == "auth":
            if (frame.get("data") or {}).get("apikey") == REJECTED_API_KEY:
                return [{"event": "error", "data": {"message": "Invalid authentication credentials"}}]
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
    def unsubscribe_data(self):
        """``data`` of every ``unsubscribe`` frame the server received."""
        return list(self._server.unsubscribe_data)

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


if __name__ == "__main__":
    burst = next(
        (int(arg.split("=", 1)[1]) for arg in sys.argv[1:] if arg.startswith("--burst-on-close=")),
        0,
    )
    server = _Server(flood="--flood" in sys.argv[1:], burst_on_close=burst)
    server.start()
    print(server.port, flush=True)
    sys.stdin.read()
