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


def test_rest_product_base_url_includes_product():
    rest = RestClient(api_key="test-key")
    assert rest.base_url == "https://api.fugle.tw/marketdata/v1.0"
    assert rest.stock.base_url == "https://api.fugle.tw/marketdata/v1.0/stock"
    assert rest.futopt.base_url == "https://api.fugle.tw/marketdata/v1.0/futopt"


def test_rest_product_base_url_follows_custom_base_url():
    rest = RestClient(api_key="test-key", base_url="https://custom.api/prefix")
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


@pytest.mark.parametrize("method", ["candles", "daily"])
def test_symbol_and_product_conflict(historical, method):
    with pytest.raises(TypeError, match="multiple values for symbol"):
        getattr(historical, method)("TXF", product="TXF")


@pytest.mark.parametrize("method", ["candles", "daily"])
def test_without_symbol_or_product(historical, method):
    with pytest.raises(TypeError, match="missing required argument: 'symbol' or 'product'"):
        getattr(historical, method)()


@pytest.mark.parametrize("value", [1, None])
def test_product_must_be_a_string(historical, value):
    with pytest.raises(TypeError, match="argument 'product'"):
        historical.candles(product=value)


def test_unknown_key_lists_product(historical):
    with pytest.raises(TypeError, match="Accepted: product, "):
        historical.candles(prodcut="TXF")
