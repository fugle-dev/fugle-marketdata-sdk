"""Loopback HTTP server for REST tests that must not reach the network (#71).

Standard library only. Every request is answered the way the Fugle API answers
an invalid key: ``401`` with ``{"message":"Unauthorized","statusCode":401}``.

Every response carries ``Connection: close``. The handler speaks HTTP/1.0 and
closes the socket after each reply; without the header the client pools that
connection and can reuse it before the close lands, failing with "peer
disconnected" instead of the expected status (#103).

The server runs on a thread of the test process, so a client call that holds
the GIL while waiting on the response would starve it and the test would time
out instead of passing.
"""
import json
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

UNAUTHORIZED = {"message": "Unauthorized", "statusCode": 401}


class RestLoopbackServer:
    """Serve ``status``/``body`` on every path for a ``with`` block.

    ``url`` is the ``base_url`` to hand the client; ``requests`` lists the
    paths received, in order.
    """

    def __init__(self, status=401, body=UNAUTHORIZED):
        payload = json.dumps(body).encode()
        requests = self.requests = []

        class Handler(BaseHTTPRequestHandler):
            def do_GET(self):  # noqa: N802 - name fixed by BaseHTTPRequestHandler
                requests.append(self.path)
                self.send_response(status)
                self.send_header("Content-Type", "application/json")
                self.send_header("Content-Length", str(len(payload)))
                self.send_header("Connection", "close")
                self.end_headers()
                self.wfile.write(payload)

            def log_message(self, *_args):
                pass

        self._httpd = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        self._thread = None
        self.url = f"http://127.0.0.1:{self._httpd.server_address[1]}"

    def __enter__(self):
        self._thread = threading.Thread(target=self._httpd.serve_forever, daemon=True)
        self._thread.start()
        return self

    def __exit__(self, *exc):
        self._httpd.shutdown()
        self._httpd.server_close()
        self._thread.join(timeout=5)

