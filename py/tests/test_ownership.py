"""Stock ownership endpoints — URL shape, legacy kwarg aliases, decoding.

Runs against a local HTTP server, so no API key or network is needed.
"""
import http.server
import json
import threading
import warnings

import pytest

from fugle_marketdata import RestClient

ENDPOINTS = {
    "etf_holdings": "etf-holdings",
    "institutional_trades": "institutional-trades",
    "director_holdings": "director-holdings",
    "tdcc_distribution": "tdcc-distribution",
}

BODIES = {
    "institutional-trades": {
        "symbol": "2330",
        "data": [{
            "date": "2026-07-31",
            "foreign": {"buy": 30000000, "sell": 25000000, "net": 5000000},
            "trust": {"buy": 1200000, "sell": 800000, "net": 400000},
            "dealer": {"buy": None, "sell": 900000, "net": -400000},
            "total": 5000000,
        }],
    },
    "director-holdings": {
        "symbol": "2330",
        "data": [{"date": "2026-05", "directors": [{
            "order": 1, "title": "董事長", "name": "某某", "electedShares": 1000,
            "heldShares": 1200, "pledgedShares": 0, "pledgeRatio": None,
            "relatedHeldShares": 50, "relatedPledgedShares": 0, "relatedPledgeRatio": 0,
        }]}],
    },
    "tdcc-distribution": {
        "symbol": "2330",
        "data": [{"date": "2026-07-03", "distributions": [
            {"range": "1-999", "holders": 1500000, "shares": 250000000, "proportion": 0.96},
        ]}],
    },
    "etf-holdings": {
        "symbol": "0050",
        "data": [{"date": "2026-07-31", "components": [
            {"symbol": "2330", "name": "台積電", "quantity": 1.0, "weight": 57.12},
        ]}],
    },
}


@pytest.fixture
def server():
    requests = []

    class Handler(http.server.BaseHTTPRequestHandler):
        def do_GET(self):  # noqa: N802
            requests.append(self.path)
            endpoint = self.path.split("/stock/ownership/")[1].split("/")[0]
            payload = json.dumps(BODIES[endpoint]).encode()
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


@pytest.fixture
def client(server):
    base, _ = server
    return RestClient(api_key="test-key", base_url=base)


def test_ownership_exposes_all_endpoints(client):
    for name in ENDPOINTS:
        assert callable(getattr(client.stock.ownership, name))
        assert callable(getattr(client.stock.ownership, f"{name}_async"))


@pytest.mark.parametrize("name,path", ENDPOINTS.items())
def test_request_path_without_query(client, server, name, path):
    _, requests = server
    getattr(client.stock.ownership, name)(symbol="2330")
    assert requests[-1] == f"/v1.0/stock/ownership/{path}/2330"


@pytest.mark.parametrize("name,path", ENDPOINTS.items())
def test_official_from_underscore_alias(client, server, name, path):
    """fugle-marketdata 2.6.0 call shape: from_= / to= must not be dropped."""
    _, requests = server
    with warnings.catch_warnings():
        warnings.simplefilter("error")  # an "ignored kwarg" warning would fail here
        getattr(client.stock.ownership, name)(
            symbol="2330", from_="2026-07-01", to="2026-07-31", sort="desc"
        )
    assert requests[-1] == (
        f"/v1.0/stock/ownership/{path}/2330?from=2026-07-01&to=2026-07-31&sort=desc"
    )


def test_literal_from_kwarg_alias(client, server):
    _, requests = server
    client.stock.ownership.institutional_trades(**{"symbol": "2330", "from": "2026-07-01"})
    assert requests[-1].endswith("/institutional-trades/2330?from=2026-07-01")


def test_canonical_names_still_work(client, server):
    _, requests = server
    client.stock.ownership.tdcc_distribution(symbol="2330", from_date="2026-06-01", to_date="2026-07-03")
    assert requests[-1].endswith("/tdcc-distribution/2330?from=2026-06-01&to=2026-07-03")


def test_duplicate_date_argument_is_rejected(client):
    with pytest.raises(TypeError, match="multiple values"):
        client.stock.ownership.director_holdings(symbol="2330", from_date="2026-01-01", from_="2026-02-01")


@pytest.mark.parametrize("name,path", ENDPOINTS.items())
def test_sort_asc_and_desc_are_sent(client, server, name, path):
    _, requests = server
    for sort in ("asc", "desc"):
        getattr(client.stock.ownership, name)(symbol="2330", sort=sort)
        assert requests[-1] == f"/v1.0/stock/ownership/{path}/2330?sort={sort}"


def test_sort_is_sent_as_given(client, server):
    # Keys are checked, values are not (#164): a sort the server does not
    # know gets the server's own error, like every other endpoint (#179).
    _, requests = server
    client.stock.ownership.institutional_trades(symbol="2330", sort="newest")
    assert requests[-1].endswith("/institutional-trades/2330?sort=newest")


def test_institutional_trades_decoding(client):
    data = client.stock.ownership.institutional_trades(symbol="2330")
    entry = data["data"][0]
    assert entry["foreign"]["net"] == 5000000
    assert entry["dealer"]["buy"] is None
    assert entry["total"] == 5000000


def test_director_holdings_decoding(client):
    data = client.stock.ownership.director_holdings(symbol="2330")
    director = data["data"][0]["directors"][0]
    assert data["data"][0]["date"] == "2026-05"
    assert director["heldShares"] == 1200
    assert director["pledgeRatio"] is None


def test_tdcc_distribution_decoding(client):
    data = client.stock.ownership.tdcc_distribution(symbol="2330")
    level = data["data"][0]["distributions"][0]
    assert level["range"] == "1-999"
    assert level["holders"] == 1500000


@pytest.mark.asyncio
async def test_async_variant(client, server):
    _, requests = server
    data = await client.stock.ownership.director_holdings_async(symbol="2330", from_="2026-01-01")
    assert data["symbol"] == "2330"
    assert requests[-1].endswith("/director-holdings/2330?from=2026-01-01")
