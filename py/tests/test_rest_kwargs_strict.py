"""Unknown REST keywords raise, and the API's own spellings work (#164, PR-5).

Before, a keyword a method did not list went into ``**kwargs``, produced one
``UserWarning`` and was dropped, so the call succeeded with the wrong data.
Now every extra keyword is resolved through core's ``rest::params`` table:

* the API's documented names (``isTrial``, ``from``/``to``, ``type="oddlot"``,
  ``session="afterhours"``, ``contractType`` ...) — what the 2.x SDK and the
  developer.fugle.tw examples use;
* ``from_``, the 2.x alias for the reserved word;
* the 3.x snake_case keywords, unchanged.

Anything else is a ``TypeError`` that lists the accepted keywords, and one
parameter given under two spellings is a ``TypeError`` too.

``TestIssue164Cases`` replays the 14 calls from the issue's table against a
local HTTP server: each must now send the parameter or raise, never drop it.
"""
import http.server
import threading
import warnings
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
        httpd.server_close()


@pytest.fixture
def client(server):
    base_url, _ = server
    return RestClient(api_key="test-key", base_url=base_url)


def last_request(requests):
    url = urlsplit(requests[-1])
    return url.path, {k: v[0] for k, v in parse_qs(url.query).items()}


@pytest.fixture(autouse=True)
def no_warnings():
    # The old path warned and carried on; nothing here may warn.
    with warnings.catch_warnings():
        warnings.simplefilter("error")
        yield


# ----- unknown keywords -----


def test_unknown_kwarg_raises_with_the_accepted_list(client, server):
    with pytest.raises(TypeError) as err:
        client.stock.intraday.quote("2330", totally_bogus=1)
    assert str(err.value) == (
        "stock.intraday.quote() got an unexpected keyword argument 'totally_bogus'. "
        "Accepted: odd_lot, type, oddLot"
    )
    assert server[1] == []


@pytest.mark.asyncio
async def test_unknown_kwarg_raises_on_the_async_method(client, server):
    with pytest.raises(TypeError, match="got an unexpected keyword argument 'totally_bogus'"):
        await client.stock.intraday.quote_async("2330", totally_bogus=1)
    assert server[1] == []


def test_unknown_kwarg_suggests_a_spelling_that_works_as_written(client, server):
    # `oddlot` is nearest to the flag `oddLot`/`odd_lot`, which takes a bool;
    # suggesting the wire name `type` would send the caller to `type=True`.
    with pytest.raises(TypeError, match="Did you mean 'oddLot'\\?") as err:
        client.stock.intraday.ticker("2330", oddlot=True)
    assert "'type'?" not in str(err.value)
    with pytest.raises(TypeError, match="Did you mean 'isTrial'\\?"):
        client.stock.intraday.trades("2330", istrial=True)
    with pytest.raises(TypeError, match="Did you mean 'after_hours'\\?"):
        client.futopt.intraday.quote("TXFD6", afterhours=True)


def test_a_key_valid_on_another_endpoint_is_still_unknown(client, server):
    with pytest.raises(TypeError, match="unexpected keyword argument 'timeframe'"):
        client.stock.intraday.quote("2330", timeframe="5")
    with pytest.raises(TypeError, match="unexpected keyword argument 'product'"):
        client.futopt.intraday.products("FUTURE", product="TXF")
    with pytest.raises(TypeError, match="unexpected keyword argument 'status'"):
        client.futopt.intraday.tickers("FUTURE", status="N")
    with pytest.raises(TypeError, match="unexpected keyword argument 'exchange'"):
        client.stock.corporate_actions.capital_changes(exchange="TWSE")
    assert server[1] == []


def test_the_accepted_list_names_every_spelling(client, server):
    with pytest.raises(TypeError) as err:
        client.stock.technical.sma("2330", bogus=1)
    assert str(err.value).endswith("Accepted: from_date, from, from_, to_date, to, timeframe, period")
    # `type` is the keyword itself on the list endpoints; `type_filter` only
    # exists on the snapshot ones.
    with pytest.raises(TypeError) as err:
        client.stock.intraday.tickers("EQUITY", bogus=1)
    assert str(err.value).endswith(
        "Accepted: type, exchange, market, industry, is_normal, isNormal, is_attention, isAttention, "
        "is_disposition, isDisposition, is_halted, isHalted, symbol"
    )
    with pytest.raises(TypeError) as err:
        client.stock.snapshot.quotes("TSE", bogus=1)
    assert str(err.value).endswith("Accepted: type_filter, type")


def test_a_near_miss_of_the_reserved_word_suggests_the_underscore_form(client, server):
    # `from` cannot be written as a bare keyword, so the hint is `from_`.
    with pytest.raises(TypeError, match="Did you mean 'from_'\\?"):
        client.stock.technical.sma("2330", From="2026-08-01")


# ----- aliases, one of each kind -----


def test_camel_case_api_names(client, server):
    client.stock.intraday.trades("2330", isTrial=True, limit=3)
    assert last_request(server[1]) == (
        "/v1.0/stock/intraday/trades/2330",
        {"isTrial": "true", "limit": "3"},
    )
    client.stock.technical.kdj("2330", rPeriod=9, kPeriod=3, dPeriod=3)
    assert last_request(server[1])[1] == {"rPeriod": "9", "kPeriod": "3", "dPeriod": "3"}
    client.futopt.historical.candles("TXO", contractMonth="202610", strikePrice=23000, callPut="CALL")
    assert last_request(server[1])[1] == {"contractMonth": "202610", "strikePrice": "23000", "callPut": "CALL"}


def test_type_oddlot(client, server):
    client.stock.intraday.ticker("2330", type="oddlot")
    assert last_request(server[1]) == ("/v1.0/stock/intraday/ticker/2330", {"type": "oddlot"})


def test_type_oddlot_is_case_sensitive_like_the_server(client, server):
    with pytest.raises(ValueError, match="must be 'oddlot' \\(got 'ODDLOT'\\); the typed keyword is odd_lot=True"):
        client.stock.intraday.ticker("2330", type="ODDLOT")
    assert server[1] == []


def test_snapshot_type_is_a_plain_filter(client, server):
    # On the snapshot endpoints `type` is a string, the 3.x keyword `type_filter`.
    client.stock.snapshot.movers("TSE", direction="up", change="percent", type="COMMONSTOCK")
    assert last_request(server[1])[1] == {"direction": "up", "change": "percent", "type": "COMMONSTOCK"}


def test_session_on_a_single_contract(client, server):
    client.futopt.intraday.quote("TXFD6", session="afterhours")
    assert last_request(server[1])[1] == {"session": "afterhours"}
    client.futopt.intraday.quote("TXFD6", session="AFTERHOURS")
    assert last_request(server[1])[1] == {"session": "afterhours"}


def test_session_on_a_list_endpoint(client, server):
    client.futopt.intraday.tickers("FUTURE", session="AFTERHOURS")
    assert last_request(server[1])[1] == {"type": "FUTURE", "session": "AFTERHOURS"}
    client.futopt.intraday.tickers("FUTURE", session="regular")
    assert last_request(server[1])[1] == {"type": "FUTURE"}


def test_session_rejects_another_value(client, server):
    with pytest.raises(ValueError, match="must be 'afterhours' \\(got 'night'\\)"):
        client.futopt.intraday.quote("TXFD6", session="night")


def test_from_and_to_as_the_api_spells_them(client, server):
    client.stock.historical.candles("2330", **{"from": "2026-08-01", "to": "2026-09-10"})
    assert last_request(server[1])[1] == {"from": "2026-08-01", "to": "2026-09-10"}


def test_from_underscore_alias(client, server):
    client.stock.technical.sma("2330", from_="2026-08-01", to="2026-09-10", timeframe="D", period=5)
    assert last_request(server[1])[1] == {"from": "2026-08-01", "to": "2026-09-10", "timeframe": "D", "period": "5"}


def test_trailing_underscore_only_covers_the_reserved_word(client, server):
    # `to_` was never a 2.x alias and `to` is not a keyword.
    with pytest.raises(TypeError, match="unexpected keyword argument 'to_'"):
        client.stock.technical.sma("2330", to_="2026-09-10")


def test_snake_case_keywords_are_unchanged(client, server):
    client.stock.technical.sma("2330", from_date="2026-08-01", to_date="2026-09-10", period=5)
    assert last_request(server[1])[1] == {"from": "2026-08-01", "to": "2026-09-10", "period": "5"}
    client.stock.intraday.quote("2330", odd_lot=True)
    assert last_request(server[1])[1] == {"type": "oddlot"}


def test_the_rc_odd_lot_alias(client, server):
    # `oddLot` is this SDK's own 3.0.0-rc spelling, listed in the table.
    client.stock.intraday.volumes("2330", oddLot=True)
    assert last_request(server[1])[1] == {"type": "oddlot"}


def test_values_are_not_checked(client, server):
    client.stock.intraday.trades("2330", limit=0, sort="asc")
    assert last_request(server[1])[1] == {"limit": "0", "sort": "asc"}
    client.stock.snapshot.movers("TSE", direction="sideways")
    assert last_request(server[1])[1] == {"direction": "sideways"}


def test_a_wrongly_typed_alias_value_fails_like_the_typed_keyword(client, server):
    # pyo3's own message, so the alias and the typed keyword reject the same way.
    with pytest.raises(TypeError, match="argument 'isTrial': 'str' object is not an instance of 'bool'"):
        client.stock.intraday.trades("2330", isTrial="yes")
    with pytest.raises(TypeError, match="argument 'rPeriod': 'str' object cannot be interpreted as an integer"):
        client.stock.technical.kdj("2330", rPeriod="9")
    with pytest.raises(TypeError, match="argument 'rPeriod': out of range"):
        client.stock.technical.kdj("2330", rPeriod=-1)
    assert server[1] == []


# ----- one parameter, two spellings -----


@pytest.mark.parametrize(
    ("kwargs", "message"),
    [
        ({"from_date": "2026-08-01", "from": "2026-08-02"}, "got multiple values for from_date (also passed as 'from')"),
        ({"from_date": "2026-08-01", "from_": "2026-08-02"}, "got multiple values for from_date (also passed as 'from_')"),
        ({"from": "2026-08-01", "from_": "2026-08-02"}, "got multiple values for from_date (passed as 'from' and 'from_')"),
    ],
)
def test_same_parameter_twice_raises(client, server, kwargs, message):
    with pytest.raises(TypeError) as err:
        client.stock.technical.sma("2330", **kwargs)
    assert str(err.value) == f"stock.technical.sma() {message}"
    assert server[1] == []


def test_odd_lot_true_with_type_oddlot_raises(client, server):
    with pytest.raises(TypeError, match="got multiple values for odd_lot \\(also passed as 'type'\\)"):
        client.stock.intraday.ticker("2330", odd_lot=True, type="oddlot")


def test_typed_and_camel_case_together_raises(client, server):
    with pytest.raises(TypeError, match="got multiple values for is_trial \\(also passed as 'isTrial'\\)"):
        client.stock.intraday.trades("2330", is_trial=True, isTrial=False)


# ----- the table from #164: 14 calls that used to drop a parameter -----


class TestIssue164Cases:
    """Every row of the issue's table now either sends the parameter or raises."""

    def test_1_sma_from_to(self, client, server):
        client.stock.technical.sma(**{"symbol": "2330", "from": "2026-08-01", "to": "2026-09-10", "timeframe": "D", "period": 5})
        assert last_request(server[1])[1] == {"from": "2026-08-01", "to": "2026-09-10", "timeframe": "D", "period": "5"}

    def test_2_trades_limit_sort(self, client, server):
        client.stock.intraday.trades(symbol="2330", limit=5, sort="asc")
        assert last_request(server[1])[1] == {"limit": "5", "sort": "asc"}

    def test_3_futopt_tickers_session_product(self, client, server):
        client.futopt.intraday.tickers(type="FUTURE", exchange="TAIFEX", session="REGULAR", product="TXF")
        # REGULAR is the server default, so it is expressed by sending no session.
        assert last_request(server[1])[1] == {"type": "FUTURE", "exchange": "TAIFEX", "product": "TXF"}

    def test_4_ticker_type_oddlot(self, client, server):
        client.stock.intraday.ticker(symbol="2330", type="oddlot")
        assert last_request(server[1])[1] == {"type": "oddlot"}

    def test_5_historical_candles_from_to(self, client, server):
        client.stock.historical.candles(**{"symbol": "0050", "from": "2026-08-01", "to": "2026-09-10", "fields": "open,close"})
        assert last_request(server[1])[1] == {"from": "2026-08-01", "to": "2026-09-10", "fields": "open,close"}

    @pytest.mark.parametrize("indicator", ["rsi", "macd", "bb"])
    def test_6_to_8_technical_from_to(self, client, server, indicator):
        getattr(client.stock.technical, indicator)(**{"symbol": "2330", "from": "2026-08-01", "to": "2026-09-10"})
        assert last_request(server[1])[1] == {"from": "2026-08-01", "to": "2026-09-10"}

    def test_9_tickers_is_normal(self, client, server):
        client.stock.intraday.tickers(type="EQUITY", isNormal=True)
        assert last_request(server[1])[1] == {"type": "EQUITY", "isNormal": "true"}

    def test_10_quote_type_oddlot(self, client, server):
        client.stock.intraday.quote(symbol="2330", type="oddlot")
        assert last_request(server[1])[1] == {"type": "oddlot"}

    def test_11_products_exchange_session_contract_type(self, client, server):
        client.futopt.intraday.products(type="FUTURE", exchange="TAIFEX", session="AFTERHOURS", contractType="I")
        assert last_request(server[1])[1] == {"type": "FUTURE", "exchange": "TAIFEX", "session": "AFTERHOURS", "contractType": "I"}

    def test_12_futopt_tickers_is_spread(self, client, server):
        client.futopt.intraday.tickers(type="FUTURE", isSpread=True)
        assert last_request(server[1])[1] == {"type": "FUTURE", "isSpread": "true"}

    def test_13_futopt_quote_session(self, client, server):
        client.futopt.intraday.quote(symbol="TXFD6", session="afterhours")
        assert last_request(server[1])[1] == {"session": "afterhours"}

    def test_14_sma_from_underscore(self, client, server):
        client.stock.technical.sma("2330", from_="2026-08-01", to="2026-09-10", timeframe="D", period=5)
        assert last_request(server[1])[1] == {"from": "2026-08-01", "to": "2026-09-10", "timeframe": "D", "period": "5"}

    def test_and_the_bogus_key_from_the_repro(self, client, server):
        with pytest.raises(TypeError, match="unexpected keyword argument 'totally_bogus'"):
            client.stock.intraday.quote(symbol="2330", totally_bogus=1)
        assert server[1] == []


# ----- every method resolves its own name in the table -----

# `Kwargs::parse` looks the method up by a name string on every call, even
# one without extra keywords, and panics when the name is not in the table.
# The names are typed by hand in 56 places; a call per method proves them.
ALL_METHODS = [
    ("stock.intraday", "quote", ("2330",)),
    ("stock.intraday", "ticker", ("2330",)),
    ("stock.intraday", "candles", ("2330",)),
    ("stock.intraday", "trades", ("2330",)),
    ("stock.intraday", "volumes", ("2330",)),
    ("stock.intraday", "tickers", ("EQUITY",)),
    ("stock.historical", "candles", ("2330",)),
    ("stock.historical", "stats", ("2330",)),
    ("stock.snapshot", "quotes", ("TSE",)),
    ("stock.snapshot", "movers", ("TSE",)),
    ("stock.snapshot", "actives", ("TSE",)),
    ("stock.technical", "sma", ("2330",)),
    ("stock.technical", "rsi", ("2330",)),
    ("stock.technical", "kdj", ("2330",)),
    ("stock.technical", "macd", ("2330",)),
    ("stock.technical", "bb", ("2330",)),
    ("stock.corporate_actions", "capital_changes", ()),
    ("stock.corporate_actions", "dividends", ()),
    ("stock.corporate_actions", "listing_applicants", ()),
    ("stock.ownership", "etf_holdings", ("2330",)),
    ("stock.ownership", "institutional_trades", ("2330",)),
    ("stock.ownership", "director_holdings", ("2330",)),
    ("stock.ownership", "tdcc_distribution", ("2330",)),
    ("futopt.intraday", "quote", ("TXFD6",)),
    ("futopt.intraday", "ticker", ("TXFD6",)),
    ("futopt.intraday", "candles", ("TXFD6",)),
    ("futopt.intraday", "trades", ("TXFD6",)),
    ("futopt.intraday", "volumes", ("TXFD6",)),
    ("futopt.intraday", "tickers", ("FUTURE",)),
    ("futopt.intraday", "products", ("FUTURE",)),
    ("futopt.historical", "candles", ("TXF",)),
    ("futopt.historical", "daily", ("TXF",)),
]


def _resolve(client, dotted):
    obj = client
    for part in dotted.split("."):
        obj = getattr(obj, part)
    return obj


@pytest.mark.parametrize(("group", "method", "args"), ALL_METHODS, ids=lambda v: v if isinstance(v, str) else "")
def test_every_sync_method_resolves_its_table_entry(client, server, group, method, args):
    getattr(_resolve(client, group), method)(*args)
    assert len(server[1]) == 1
    with pytest.raises(TypeError, match="unexpected keyword argument 'bogus'"):
        getattr(_resolve(client, group), method)(*args, bogus=1)


@pytest.mark.asyncio
@pytest.mark.parametrize(("group", "method", "args"), ALL_METHODS, ids=lambda v: v if isinstance(v, str) else "")
async def test_every_async_method_resolves_its_table_entry(client, server, group, method, args):
    await getattr(_resolve(client, group), f"{method}_async")(*args)
    assert len(server[1]) == 1
    with pytest.raises(TypeError, match="unexpected keyword argument 'bogus'"):
        await getattr(_resolve(client, group), f"{method}_async")(*args, bogus=1)
