"""SDK exceptions carry the unified error fields (#81) and the core → Python
class mapping holds (#219).

``code``, ``source_kind``, ``message``, ``status``, ``body``, ``request_id``
and ``headers``, plus the 2.4.1 aliases ``status_code`` / ``response_text``.
"""

from __future__ import annotations

import json
import threading
from http.server import BaseHTTPRequestHandler, HTTPServer

import pytest

from fugle_marketdata import (
    ApiError,
    AuthError,
    ConnectionError,
    MarketDataError,
    RateLimitError,
    ReconnectConfig,
    RestClient,
    WebSocketClient,
    WebSocketError,
)


class _Server:
    """Answer every GET with ``status``, ``body`` and ``headers``."""

    def __init__(self, status: int, body: str, headers: dict[str, str]):
        payload = body.encode()

        class Handler(BaseHTTPRequestHandler):
            def do_GET(self):  # noqa: N802 - name fixed by BaseHTTPRequestHandler
                self.send_response(status)
                self.send_header("Content-Type", "application/json")
                for name, value in headers.items():
                    self.send_header(name, value)
                self.send_header("Content-Length", str(len(payload)))
                self.end_headers()
                self.wfile.write(payload)

            def log_message(self, *_args):
                pass

        self._httpd = HTTPServer(("127.0.0.1", 0), Handler)
        self._thread = threading.Thread(target=self._httpd.serve_forever, daemon=True)
        self._thread.start()

    @property
    def base_url(self) -> str:
        host, port = self._httpd.server_address[:2]
        return f"http://{host}:{port}"

    def close(self) -> None:
        self._httpd.shutdown()
        self._httpd.server_close()
        self._thread.join(timeout=5)


@pytest.fixture
def error_client():
    servers: list[_Server] = []

    def _make(status: int, body: str, headers: dict[str, str] | None = None):
        server = _Server(status, body, headers or {})
        servers.append(server)
        return RestClient(api_key="test-key", base_url=server.base_url)

    yield _make
    for s in servers:
        s.close()


@pytest.mark.parametrize(
    ("status", "exc_type", "code", "source_kind", "prefix"),
    [
        (401, AuthError, 2002, "auth", "Authentication error: "),
        (404, ApiError, 2003, "client", "API error (status 404): "),
        (429, RateLimitError, 2003, "rate_limit", "API error (status 429): "),
        (503, ApiError, 2003, "network", "API error (status 503): "),
    ],
)
def test_http_error_carries_response_details(error_client, status, exc_type, code, source_kind, prefix):
    body = json.dumps({"message": "nope", "statusCode": status})
    client = error_client(status, body, {"X-Request-Id": "req-1", "X-RateLimit-Remaining": "0"})

    with pytest.raises(exc_type) as info:
        client.stock.intraday.quote(symbol="2330")
    e = info.value

    assert e.code == code
    assert e.args == (prefix + body, code)
    assert e.source_kind == source_kind
    assert e.message == str(e.args[0]) == prefix + body
    assert e.status == status
    assert e.body == body
    assert e.request_id == "req-1"
    assert e.headers["x-request-id"] == "req-1"
    assert e.headers["x-ratelimit-remaining"] == "0"
    assert e.headers["content-type"] == "application/json"
    # 2.4.1 aliases
    assert e.status_code == status
    assert e.response_text == body
    assert e.url is None and e.params is None


def test_transport_error_is_connection_error_without_http_details():
    """REST cannot reach the server: ``ConnectionError`` (2001), not
    ``WebSocketError`` (#219)."""
    client = RestClient(api_key="test-key", base_url="http://127.0.0.1:9")

    with pytest.raises(ConnectionError) as info:
        client.stock.intraday.quote(symbol="2330")
    e = info.value

    assert isinstance(e, MarketDataError)
    assert not isinstance(e, WebSocketError)
    assert e.code == 2001
    assert e.source_kind == "network"
    assert (e.status, e.body, e.request_id, e.headers) == (None, None, None, {})
    assert (e.status_code, e.response_text) == (None, None)


@pytest.mark.timeout(20, method="thread")
def test_websocket_connect_failure_stays_websocket_error():
    """A WebSocket connect that cannot reach the server stays ``WebSocketError``
    (3002); only core's ``ConnectionError`` moved to ``ConnectionError`` (#219)."""
    ws = WebSocketClient(
        api_key="test-key", base_url="ws://127.0.0.1:9", reconnect=ReconnectConfig.disabled()
    ).stock

    with pytest.raises(WebSocketError) as info:
        ws.connect()
    e = info.value

    assert isinstance(e, MarketDataError)
    assert not isinstance(e, ConnectionError)
    assert e.code == 3002
    assert e.source_kind == "network"


def test_websocket_command_before_connect_is_runtime_error():
    """Pinned: a command before ``connect()`` is the binding's own
    ``RuntimeError``, not a ``ConnectionError`` (2001) nor a ``WebSocketError``."""
    ws = WebSocketClient(api_key="test-key", reconnect=ReconnectConfig.disabled()).stock

    with pytest.raises(RuntimeError, match="Not connected") as info:
        ws.subscribe("trades", "2330")
    assert not isinstance(info.value, MarketDataError)
