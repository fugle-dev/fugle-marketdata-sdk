"""The 2.x factory behaviour 3.0 keeps (#306).

- ``ws.stock`` / ``ws.futopt`` are built once and returned on every read, so
  the 2.x idiom ``ws.stock.on(...)``, ``ws.stock.connect()``,
  ``ws.stock.subscribe(...)`` drives one client.
- ``rest.stock.base_url`` / ``rest.futopt.base_url`` carry the product
  segment, as 2.x's product clients did.
- ``futopt.historical.candles(product=...)``, 2.x's spelling, is accepted.

Runs against loopback servers, so no API key or network is needed.
"""
import json
import threading

import pytest

from fugle_marketdata import ReconnectConfig, RestClient, WebSocketClient
from tests.rest_loopback import RestLoopbackServer
from tests.ws_loopback import TIMEOUT_S, LoopbackServer, Recorder, disconnect_quietly

PRODUCTS = [pytest.param("stock", id="stock"), pytest.param("futopt", id="futopt")]

hard_timeout = pytest.mark.timeout(20, method="thread")


@pytest.mark.parametrize("product", PRODUCTS)
def test_ws_product_client_is_shared(product):
    ws = WebSocketClient(api_key="test-key")
    assert getattr(ws, product) is getattr(ws, product)


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_ws_product_client_first_read_from_many_threads(product):
    """Threads that all make the first read at once get one client."""
    ws = WebSocketClient(api_key="test-key")
    threads_n = 8
    barrier = threading.Barrier(threads_n)
    seen = []

    def read():
        barrier.wait()
        seen.append(getattr(ws, product))

    threads = [threading.Thread(target=read) for _ in range(threads_n)]
    for t in threads:
        t.start()
    for t in threads:
        t.join()
    assert len(seen) == threads_n
    assert len({id(client) for client in seen}) == 1
    assert seen[0] is getattr(ws, product)


def test_ws_product_clients_are_distinct():
    ws = WebSocketClient(api_key="test-key")
    assert ws.stock is not ws.futopt


def test_ws_product_client_is_per_websocket_client():
    assert WebSocketClient(api_key="test-key").stock is not WebSocketClient(api_key="test-key").stock


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_ws_legacy_idiom_reaches_one_client(product):
    """The MIGRATION quickstart, written the 2.x way: every call reads
    ``ws.stock`` again."""
    with LoopbackServer() as server:
        ws = WebSocketClient(
            api_key="test-key", base_url=server.url, reconnect=ReconnectConfig.disabled()
        )
        try:
            recorder = Recorder(getattr(ws, product), messages=True)
            getattr(ws, product).connect()
            getattr(ws, product).subscribe({"channel": "trades", "symbol": "2330"})

            def is_data(call):
                name, args = call
                if name != "message":
                    return False
                message = args[0]
                return (json.loads(message) if isinstance(message, str) else message).get("event") == "data"

            recorder.wait_until(lambda calls: any(map(is_data, calls)), TIMEOUT_S, "data message")
            assert getattr(ws, product).is_connected()
        finally:
            disconnect_quietly(getattr(ws, product))


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_ws_last_disconnect_read_again_in_the_handler(product):
    """MIGRATION §5 / README: the handler reads ``ws.stock.last_disconnect``
    off the property, not off a variable."""
    with LoopbackServer() as server:
        ws = WebSocketClient(
            api_key="test-key", base_url=server.url, reconnect=ReconnectConfig.disabled()
        )
        seen = []
        getattr(ws, product).on(
            "disconnect", lambda code, reason: seen.append(getattr(ws, product).last_disconnect)
        )
        recorder = Recorder(getattr(ws, product))
        getattr(ws, product).connect()
        getattr(ws, product).disconnect()
        recorder.wait_for("disconnect", TIMEOUT_S)
        assert len(seen) == 1
        assert seen[0] is not None
        assert (seen[0].intent, seen[0].will_reconnect) == ("client", False)
        assert seen[0] == getattr(ws, product).last_disconnect


@hard_timeout
@pytest.mark.asyncio
@pytest.mark.parametrize("product", PRODUCTS)
async def test_ws_async_with_yields_the_shared_client(product):
    with LoopbackServer() as server:
        ws = WebSocketClient(
            api_key="test-key", base_url=server.url, reconnect=ReconnectConfig.disabled()
        )
        async with getattr(ws, product) as client:
            assert client is getattr(ws, product)
            assert client.is_connected()
        assert not getattr(ws, product).is_connected()


def test_rest_product_base_url_includes_product():
    rest = RestClient(api_key="test-key")
    assert rest.base_url == "https://api.fugle.tw/marketdata/v1.0"
    assert rest.stock.base_url == "https://api.fugle.tw/marketdata/v1.0/stock"
    assert rest.futopt.base_url == "https://api.fugle.tw/marketdata/v1.0/futopt"


@pytest.mark.parametrize(
    "base_url",
    ["https://custom.api/prefix", "https://custom.api/prefix/", "https://custom.api/prefix//"],
)
def test_rest_product_base_url_follows_custom_base_url(base_url):
    rest = RestClient(api_key="test-key", base_url=base_url)
    assert rest.stock.base_url == "https://custom.api/prefix/v1.0/stock"
    assert rest.futopt.base_url == "https://custom.api/prefix/v1.0/futopt"


@pytest.fixture
def futopt_server():
    with RestLoopbackServer(status=200, body={"product": "TXF", "data": []}) as server:
        yield server


@pytest.fixture
def historical(futopt_server):
    return RestClient(api_key="test-key", base_url=futopt_server.url).futopt.historical


FUTOPT_HISTORICAL = [
    pytest.param("candles", "/v1.0/futopt/historical/candles/TXF", id="candles"),
    pytest.param("daily", "/v1.0/futopt/historical/daily/TXF", id="daily"),
]


@pytest.mark.parametrize("method, path", FUTOPT_HISTORICAL)
def test_accepts_product(historical, futopt_server, method, path):
    getattr(historical, method)(product="TXF")
    assert futopt_server.requests[-1] == path


@pytest.mark.asyncio
@pytest.mark.parametrize("method, path", FUTOPT_HISTORICAL)
async def test_async_accepts_product(historical, futopt_server, method, path):
    await getattr(historical, f"{method}_async")(product="TXF")
    assert futopt_server.requests[-1] == path


def test_candles_product_with_params(historical, futopt_server):
    historical.candles(product="TXF", timeframe="D")
    assert futopt_server.requests[-1] == "/v1.0/futopt/historical/candles/TXF?timeframe=D"


def test_symbol_and_product_conflict(historical):
    with pytest.raises(TypeError, match="multiple values for symbol"):
        historical.candles("TXF", product="TXF")
    # daily's path param is `product`; `symbol` is the alias there.
    with pytest.raises(TypeError, match="multiple values for product"):
        historical.daily("TXF", symbol="TXF")


def test_without_symbol_or_product(historical):
    with pytest.raises(TypeError, match="missing required argument: 'symbol' or 'product'"):
        historical.candles()
    with pytest.raises(TypeError, match="missing required argument: 'product' or 'symbol'"):
        historical.daily()


def test_daily_accepts_symbol(historical, futopt_server):
    historical.daily(symbol="TXF")
    assert futopt_server.requests[-1] == "/v1.0/futopt/historical/daily/TXF"


@pytest.mark.parametrize("value", [1, None])
def test_product_must_be_a_string(historical, value):
    with pytest.raises(TypeError, match="argument 'product'"):
        historical.candles(product=value)


def test_unknown_key_lists_product(historical):
    with pytest.raises(TypeError, match="Accepted: product, "):
        historical.candles(prodcut="TXF")
