"""FutOpt historical endpoints — URL shape per fugle-realtime #727.

Runs against a local HTTP server, so no API key or network is needed.
"""
import http.server
import threading
from urllib.parse import parse_qs, urlsplit

import pytest

from fugle_marketdata import RestClient


@pytest.fixture
def server():
    requests = []

    class Handler(http.server.BaseHTTPRequestHandler):
        def do_GET(self):  # noqa: N802
            requests.append(self.path)
            payload = b'{"product":"TXF","data":[]}'
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(payload)))
            self.end_headers()
            self.wfile.write(payload)

        def log_message(self, *args):
            pass

    httpd = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    thread = threading.Thread(target=httpd.serve_forever, daemon=True)
    thread.start()
    try:
        yield f"http://127.0.0.1:{httpd.server_address[1]}", requests
    finally:
        httpd.shutdown()


def last_request(requests):
    url = urlsplit(requests[-1])
    return url.path, {k: v[0] for k, v in parse_qs(url.query).items()}


@pytest.fixture
def historical(server):
    base_url, _ = server
    return RestClient(api_key="test-key", base_url=base_url).futopt.historical


def test_daily_sends_date_and_session(historical, server):
    historical.daily("TXF", date="2026-09-15", after_hours=True)
    assert last_request(server[1]) == (
        "/v1.0/futopt/historical/daily/TXF",
        {"date": "2026-09-15", "session": "afterhours"},
    )


def test_daily_regular_session_sends_no_params(historical, server):
    historical.daily("TXF")
    assert last_request(server[1]) == ("/v1.0/futopt/historical/daily/TXF", {})


@pytest.mark.parametrize("kwarg", ["from_date", "to_date"])
def test_daily_rejects_the_old_date_range(historical, server, kwarg):
    with pytest.raises(TypeError, match="date="):
        historical.daily("TXF", **{kwarg: "2026-09-01"})
    assert server[1] == []


@pytest.mark.asyncio
async def test_daily_async_rejects_the_old_date_range(historical, server):
    with pytest.raises(TypeError, match="date="):
        await historical.daily_async("TXF", from_date="2026-09-01")
    assert server[1] == []


def test_candles_sends_contract_month_fields_sort_and_session(historical, server):
    historical.candles(
        "TXF",
        from_date="2026-09-01",
        to_date="2026-09-15",
        timeframe="D",
        after_hours=True,
        contract_month="1!",
        fields="open,close",
        sort="desc",
    )
    assert last_request(server[1]) == (
        "/v1.0/futopt/historical/candles/TXF",
        {
            "from": "2026-09-01",
            "to": "2026-09-15",
            "contractMonth": "1!",
            "fields": "open,close",
            "timeframe": "D",
            "sort": "desc",
            "session": "afterhours",
        },
    )


@pytest.mark.asyncio
async def test_candles_async_matches_sync(historical, server):
    await historical.candles_async("TXF", contract_month="202609")
    assert last_request(server[1]) == (
        "/v1.0/futopt/historical/candles/TXF",
        {"contractMonth": "202609"},
    )
