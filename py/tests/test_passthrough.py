"""Passthrough tests: what the server sends is what the caller gets.

These replace ``test_response_compatibility.py``, whose 11 tests asserted
against VCR cassettes rather than against anything the SDK produced, wrapped
nearly every assertion in ``if key in obj:`` so a missing key skipped silently,
and recorded the v0.3-era API whose schema this SDK never parsed. Nothing in
that file could fail.

The SDK issues HTTP from Rust, so ``responses``/``vcrpy`` cannot intercept it.
We stand up a real loopback server instead and point ``base_url`` at it, which
exercises the whole chain: HTTP, JSON decode, and conversion into Python
objects.
"""

from __future__ import annotations

import json
import threading
from http.server import BaseHTTPRequestHandler, HTTPServer

import pytest

from fugle_marketdata import RestClient

# A real ``GET /stock/intraday/quote/2330`` body, captured 2026-09-16.
QUOTE_2330 = {
    "date": "2026-09-16",
    "type": "EQUITY",
    "exchange": "TWSE",
    "market": "TSE",
    "symbol": "2330",
    "name": "台積電",
    "referencePrice": 2380,
    "previousClose": 2385,
    "openPrice": 2375,
    "openTime": 1789520409721819,
    "highPrice": 2385,
    "lowPrice": 2375,
    "closePrice": 2385,
    "avgPrice": 2378.51,
    "change": 5,
    "changePercent": 0.21,
    "amplitude": 0.42,
    "lastPrice": 2385,
    "lastSize": 1,
    "bids": [{"price": 2380, "size": 185}, {"price": 2375, "size": 1407}],
    "asks": [{"price": 2385, "size": 199}, {"price": 2390, "size": 556}],
    "total": {
        "tradeValue": 14566025000,
        "tradeVolume": 6124,
        "tradeVolumeAtBid": 1962,
        "tradeVolumeAtAsk": 2686,
        "transaction": 1922,
        "time": 1789525853959140,
    },
    "lastTrade": {
        "bid": 2380,
        "ask": 2385,
        "price": 2385,
        "size": 1,
        "time": 1789525853959140,
        "serial": 6132837,
    },
    "isContinuous": True,
    "serial": 6152257,
    "lastUpdated": 1789525882301247,
}


class _Server:
    """Serve one fixed JSON body on every path."""

    def __init__(self, body: dict):
        payload = json.dumps(body).encode()

        class Handler(BaseHTTPRequestHandler):
            def do_GET(self):  # noqa: N802 - name fixed by BaseHTTPRequestHandler
                self.send_response(200)
                self.send_header("Content-Type", "application/json")
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
def quote_client():
    """A client pointed at a loopback server returning ``QUOTE_2330``."""

    def _make(body=None):
        server = _Server(body if body is not None else QUOTE_2330)
        servers.append(server)
        return RestClient(api_key="test-key", base_url=server.base_url)

    servers: list[_Server] = []
    yield _make
    for s in servers:
        s.close()


def test_response_is_exactly_what_the_server_sent(quote_client):
    quote = quote_client().stock.intraday.quote(symbol="2330")

    assert quote == QUOTE_2330
    # Nothing added, nothing removed.
    assert set(quote) == set(QUOTE_2330)


def test_key_order_matches_the_server(quote_client):
    quote = quote_client().stock.intraday.quote(symbol="2330")

    # Previously the JSON object went through a sorted map, so callers saw
    # amplitude, asks, avgPrice, bids, ... instead of the server's ordering.
    assert list(quote) == list(QUOTE_2330)
    assert next(iter(quote)) == "date"


def test_reference_price_survives_and_is_the_basis_for_change(quote_client):
    quote = quote_client().stock.intraday.quote(symbol="2330")

    assert quote["referencePrice"] == 2380
    assert quote["lastPrice"] - quote["referencePrice"] == quote["change"]
    # Deriving it from previousClose would have given the wrong answer.
    assert quote["lastPrice"] - quote["previousClose"] != quote["change"]


def test_last_trade_is_no_longer_dropped(quote_client):
    quote = quote_client().stock.intraday.quote(symbol="2330")

    # quote_to_dict used to omit lastTrade, lastTrial and tradingHalt outright,
    # so Python callers simply could not reach them.
    assert quote["lastTrade"] == QUOTE_2330["lastTrade"]
    assert quote["total"]["tradeVolumeAtBid"] == 1962
    assert quote["total"]["tradeVolumeAtAsk"] == 2686
    assert quote["total"]["time"] == 1789525853959140


def test_omitted_fields_stay_absent(quote_client):
    quote = quote_client().stock.intraday.quote(symbol="2330")

    # The server sent only isContinuous. The rest used to be materialised as
    # False, which callers could not tell apart from a real False.
    assert quote["isContinuous"] is True
    for absent in ("isOpen", "isClose", "isTrial", "isLimitUpPrice", "tradingHalt"):
        assert absent not in quote


def test_unknown_field_still_reaches_the_caller(quote_client):
    body = dict(QUOTE_2330, someFieldAddedLater={"nested": [1, 2]})
    quote = quote_client(body).stock.intraday.quote(symbol="2330")

    assert quote["someFieldAddedLater"] == {"nested": [1, 2]}


def test_serial_keeps_the_type_the_server_used(quote_client):
    quote = quote_client().stock.intraday.quote(symbol="2330")

    # The typed Rust models normalise this to a string because futopt sends a
    # zero-padded one. Passthrough does not: the wire type is preserved.
    assert isinstance(quote["serial"], int)
    assert isinstance(quote["lastTrade"]["serial"], int)


def test_tickers_keeps_the_envelope(quote_client):
    envelope = {
        "date": "2026-09-16",
        "type": "EQUITY",
        "exchange": "TWSE",
        "market": "TSE",
        "data": [{"symbol": "2330", "name": "台積電"}],
    }
    result = quote_client(envelope).stock.intraday.tickers(type="EQUITY")

    # Earlier releases returned just `data`, losing the sibling metadata.
    assert result == envelope
    assert not isinstance(result, list)
    assert len(result["data"]) == 1
