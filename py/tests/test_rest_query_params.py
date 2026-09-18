"""The keyword arguments added in #164 (PR-4) reach the query string.

Each REST method used to expose only part of what core could send, so a
caller had no way to ask for odd-lot data, a trade window, a snapshot filter
and so on: the keyword landed in ``**_extra`` and was dropped with a warning.
Every parameter in core's ``rest::params`` table now has a keyword here, and
each one is asserted against the request a local HTTP server receives. No API
key or network is needed.

``test_ticker_odd_lot_sends_type_oddlot`` guards #165: ``ticker(odd_lot=True)``
returned board-lot data because the keyword did not exist.
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
            payload = b"{}"
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
    base_url, _ = server
    return RestClient(api_key="test-key", base_url=base_url)


def last_request(requests):
    url = urlsplit(requests[-1])
    return url.path, {k: v[0] for k, v in parse_qs(url.query).items()}


# ----- stock.intraday: odd_lot on the four methods that lacked it -----


def test_ticker_odd_lot_sends_type_oddlot(client, server):
    """#165: ``ticker(odd_lot=True)`` silently returned board-lot data.

    ``quote()`` had the keyword and ``ticker()`` did not, so the same call
    shape gave odd-lot data for one and board-lot data for the other.
    """
    client.stock.intraday.ticker("2330", odd_lot=True)
    assert last_request(server[1]) == ("/v1.0/stock/intraday/ticker/2330", {"type": "oddlot"})


@pytest.mark.asyncio
async def test_ticker_async_odd_lot_sends_type_oddlot(client, server):
    await client.stock.intraday.ticker_async("2330", odd_lot=True)
    assert last_request(server[1]) == ("/v1.0/stock/intraday/ticker/2330", {"type": "oddlot"})


def test_ticker_without_odd_lot_sends_nothing(client, server):
    client.stock.intraday.ticker("2330")
    assert last_request(server[1]) == ("/v1.0/stock/intraday/ticker/2330", {})


def test_volumes_odd_lot(client, server):
    client.stock.intraday.volumes("2330", odd_lot=True)
    assert last_request(server[1]) == ("/v1.0/stock/intraday/volumes/2330", {"type": "oddlot"})


# ----- stock.intraday.quotes: the batch quote, no path param (#176) -----


def test_quotes_sends_symbols_as_a_query_key(client, server):
    client.stock.intraday.quotes("2330,2317")
    assert last_request(server[1]) == ("/v1.0/stock/intraday/quotes", {"symbol": "2330,2317"})


def test_quotes_odd_lot_sends_type_oddlot(client, server):
    client.stock.intraday.quotes("2330,2317", odd_lot=True)
    assert last_request(server[1]) == (
        "/v1.0/stock/intraday/quotes",
        {"symbol": "2330,2317", "type": "oddlot"},
    )


@pytest.mark.asyncio
async def test_quotes_async_matches_sync(client, server):
    await client.stock.intraday.quotes_async("2330", odd_lot=True)
    assert last_request(server[1]) == ("/v1.0/stock/intraday/quotes", {"symbol": "2330", "type": "oddlot"})


def test_quotes_returns_the_list_the_server_sends():
    """The endpoint answers a JSON array, one quote per symbol, not an object."""
    payload = b'[{"symbol": "2330", "lastPrice": 2460}, {"symbol": "2317", "lastPrice": 250}]'

    class Handler(http.server.BaseHTTPRequestHandler):
        def do_GET(self):  # noqa: N802
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(payload)))
            self.end_headers()
            self.wfile.write(payload)

        def log_message(self, *args):
            pass

    httpd = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    threading.Thread(target=httpd.serve_forever, daemon=True).start()
    try:
        client = RestClient(api_key="test-key", base_url=f"http://127.0.0.1:{httpd.server_address[1]}")
        quotes = client.stock.intraday.quotes("2330,2317")
    finally:
        httpd.shutdown()
        httpd.server_close()
    assert quotes == [{"symbol": "2330", "lastPrice": 2460}, {"symbol": "2317", "lastPrice": 250}]


def test_candles_odd_lot_and_sort(client, server):
    client.stock.intraday.candles("2330", timeframe="5", odd_lot=True, sort="desc")
    assert last_request(server[1]) == (
        "/v1.0/stock/intraday/candles/2330",
        {"timeframe": "5", "type": "oddlot", "sort": "desc"},
    )


def test_trades_window_sort_and_trial(client, server):
    client.stock.intraday.trades("2330", odd_lot=True, offset=10, limit=5, sort="asc", is_trial=True)
    assert last_request(server[1]) == (
        "/v1.0/stock/intraday/trades/2330",
        {"type": "oddlot", "offset": "10", "limit": "5", "sort": "asc", "isTrial": "true"},
    )


def test_trades_sort_desc(client, server):
    client.stock.intraday.trades("2330", sort="desc")
    assert last_request(server[1]) == ("/v1.0/stock/intraday/trades/2330", {"sort": "desc"})


def test_trades_sort_is_sent_as_given(client, server):
    # Keys are checked, values are not (#164): a sort the server does not
    # know gets the server's own error, like every other endpoint (#179).
    client.stock.intraday.trades("2330", sort="newest")
    assert last_request(server[1]) == ("/v1.0/stock/intraday/trades/2330", {"sort": "newest"})


@pytest.mark.asyncio
async def test_trades_async_sort_asc_and_desc(client, server):
    for sort in ("asc", "desc"):
        await client.stock.intraday.trades_async("2330", sort=sort)
        assert last_request(server[1]) == ("/v1.0/stock/intraday/trades/2330", {"sort": sort})


def test_positional_order_is_unchanged(client, server):
    # New keywords go after the existing ones, so a positional call keeps
    # its meaning: `symbol` first, then `timeframe` for candles.
    client.stock.intraday.candles("2330", "10")
    assert last_request(server[1]) == ("/v1.0/stock/intraday/candles/2330", {"timeframe": "10"})
    client.stock.intraday.quote("2330", True)
    assert last_request(server[1]) == ("/v1.0/stock/intraday/quote/2330", {"type": "oddlot"})


# ----- stock.intraday.tickers: status filters and symbol -----


def test_tickers_status_filters_and_symbol(client, server):
    client.stock.intraday.tickers(
        "EQUITY", is_attention=True, is_disposition=False, is_halted=True, symbol="2330,2317"
    )
    assert last_request(server[1]) == (
        "/v1.0/stock/intraday/tickers",
        {
            "type": "EQUITY",
            "isAttention": "true",
            "isDisposition": "false",
            "isHalted": "true",
            "symbol": "2330,2317",
        },
    )


# ----- stock.snapshot: type filter and movers thresholds -----


def test_movers_type_filter_and_thresholds(client, server):
    client.stock.snapshot.movers(
        "TSE", direction="up", change="percent", type_filter="COMMONSTOCK",
        gt=1.5, gte=2, lt=9.5, lte=10, eq=-3.25,
    )
    assert last_request(server[1]) == (
        "/v1.0/stock/snapshot/movers/TSE",
        {
            "direction": "up",
            "change": "percent",
            "type": "COMMONSTOCK",
            "gt": "1.5",
            "gte": "2",
            "lt": "9.5",
            "lte": "10",
            "eq": "-3.25",
        },
    )


def test_actives_type_filter(client, server):
    client.stock.snapshot.actives("OTC", trade="value", type_filter="ALLBUT0999")
    assert last_request(server[1]) == (
        "/v1.0/stock/snapshot/actives/OTC",
        {"trade": "value", "type": "ALLBUT0999"},
    )


# ----- stock.snapshot.heatmap: an index code in the path (#176) -----


def test_heatmap_sends_time_and_period(client, server):
    client.stock.snapshot.heatmap("IX0001", time="100000", period="1m")
    assert last_request(server[1]) == (
        "/v1.0/stock/snapshot/heatmap/IX0001",
        {"time": "100000", "period": "1m"},
    )


def test_heatmap_without_query(client, server):
    client.stock.snapshot.heatmap("IX0027")
    assert last_request(server[1]) == ("/v1.0/stock/snapshot/heatmap/IX0027", {})


@pytest.mark.asyncio
async def test_heatmap_async_matches_sync(client, server):
    await client.stock.snapshot.heatmap_async("IX0001", period="ytd")
    assert last_request(server[1]) == ("/v1.0/stock/snapshot/heatmap/IX0001", {"period": "ytd"})


# ----- stock.corporate_actions: exchange and sort -----


@pytest.mark.parametrize(
    ("method", "path", "exchange"),
    [
        ("dividends", "dividends", "TPEx"),
        ("listing_applicants", "listing-applicants", "TWSE"),
    ],
)
def test_corporate_actions_exchange_and_sort(client, server, method, path, exchange):
    getattr(client.stock.corporate_actions, method)(
        start_date="2026-01-01", end_date="2026-06-30", exchange=exchange, sort="asc"
    )
    assert last_request(server[1]) == (
        f"/v1.0/stock/corporate-actions/{path}",
        {"start_date": "2026-01-01", "end_date": "2026-06-30", "exchange": exchange, "sort": "asc"},
    )


def test_capital_changes_sort(client, server):
    # capital-changes takes no `exchange`; its backend answers 400 to it (#168).
    client.stock.corporate_actions.capital_changes(start_date="2026-01-01", sort="desc")
    assert last_request(server[1]) == (
        "/v1.0/stock/corporate-actions/capital-changes",
        {"start_date": "2026-01-01", "sort": "desc"},
    )


# ----- futopt.intraday: products and tickers -----


def test_products_exchange_session_and_status(client, server):
    client.futopt.intraday.products("OPTION", exchange="TAIFEX", after_hours=True, status="N")
    assert last_request(server[1]) == (
        "/v1.0/futopt/intraday/products",
        {"type": "OPTION", "exchange": "TAIFEX", "session": "AFTERHOURS", "status": "N"},
    )


def test_products_positional_contract_type_is_unchanged(client, server):
    client.futopt.intraday.products("FUTURE", "I")
    assert last_request(server[1]) == (
        "/v1.0/futopt/intraday/products",
        {"type": "FUTURE", "contractType": "I"},
    )


def test_tickers_product(client, server):
    client.futopt.intraday.tickers("FUTURE", product="TXF")
    assert last_request(server[1]) == (
        "/v1.0/futopt/intraday/tickers",
        {"type": "FUTURE", "product": "TXF"},
    )


# ----- futopt.historical.candles: options -----


def test_historical_candles_strike_price_and_call_put(client, server):
    client.futopt.historical.candles("TXO", contract_month="202610", strike_price=23000, call_put="CALL")
    assert last_request(server[1]) == (
        "/v1.0/futopt/historical/candles/TXO",
        {"contractMonth": "202610", "strikePrice": "23000", "callPut": "CALL"},
    )


@pytest.mark.asyncio
async def test_historical_candles_async_strike_price_and_call_put(client, server):
    await client.futopt.historical.candles_async("TXO", strike_price=22950.5, call_put="PUT")
    assert last_request(server[1]) == (
        "/v1.0/futopt/historical/candles/TXO",
        {"strikePrice": "22950.5", "callPut": "PUT"},
    )
