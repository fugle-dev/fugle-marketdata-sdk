"""Minimal loopback WebSocket server for tests that need no API key.

Standard library only, so CI needs no extra dependency. It speaks just enough
RFC 6455 for the SDK client: the opening handshake, unfragmented text frames
from the client (masked) and to it (unmasked), ping/pong and close.

The protocol mirrors ``js/tests/ws-worker.test.js``: ``auth`` is acked with
``authenticated``; ``subscribe`` is answered with ``subscribed`` plus one
``data`` frame.

The server runs in a child process: the blocking ``connect()`` holds the GIL
while it waits for the handshake, so a server thread in the test process could
never answer it (#39).
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
    """Fugle-shaped WebSocket server on ``127.0.0.1`` with an ephemeral port.

    Runs only inside the child process, whose exit reaps the daemon threads.
    """

    def __init__(self):
        self._listener = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self._listener.bind(("127.0.0.1", 0))
        self._listener.listen()
        self.port = self._listener.getsockname()[1]

    def start(self):
        threading.Thread(target=self._accept_loop, daemon=True).start()

    def _accept_loop(self):
        while True:
            conn, _ = self._listener.accept()
            threading.Thread(target=self._serve, args=(conn,), daemon=True).start()

    def _serve(self, conn):
        try:
            _handshake(conn)
            while True:
                opcode, payload = _read_frame(conn)
                if opcode == OP_TEXT:
                    for reply in self._replies(json.loads(payload)):
                        _send_frame(conn, OP_TEXT, json.dumps(reply).encode())
                elif opcode == OP_PING:
                    _send_frame(conn, OP_PONG, payload)
                elif opcode == OP_CLOSE:
                    _send_frame(conn, OP_CLOSE, payload[:2])
                    return
        except (ConnectionError, OSError, ValueError):
            return

    @staticmethod
    def _replies(frame):
        event = frame.get("event")
        if event == "auth":
            return [{"event": "authenticated", "data": {"message": "Authenticated successfully"}}]
        if event == "subscribe":
            data = frame.get("data") or {}
            channel, symbol = data.get("channel"), data.get("symbol")
            sub_id = f"{channel}-{symbol}"
            return [
                {"event": "subscribed", "data": {"id": sub_id, "channel": channel, "symbol": symbol}},
                {"event": "data", "data": {"symbol": symbol, "price": 100}, "id": sub_id, "channel": channel},
            ]
        return []


class LoopbackServer:
    """Run ``_Server`` in a child process for the duration of a ``with`` block.

    ``url`` is the ``base_url`` to hand the client.
    """

    def __init__(self):
        self._proc = None
        self.url = None

    def __enter__(self):
        self._proc = subprocess.Popen(
            [sys.executable, os.path.abspath(__file__)],
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


if __name__ == "__main__":
    server = _Server()
    server.start()
    print(server.port, flush=True)
    sys.stdin.read()
