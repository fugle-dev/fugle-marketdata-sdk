"""A malformed ``tls_root_cert_pem`` makes ``connect()`` and ``connect_async()`` fail.

Core builds the TLS connector at the start of every connect, so a bad root
certificate is a ``ConfigError`` raised before any network I/O. The loopback
server has no TLS, so a good certificate could not be told from none here; a
bad one can: this checks that the config ``connect_async()`` actually uses
carries the TLS setting (once it ignored it and connected anyway).
"""
import asyncio

import pytest

from fugle_marketdata import ConfigError, MarketDataError, ReconnectConfig, WebSocketClient
from tests.ws_loopback import LoopbackServer, disconnect_quietly

PRODUCTS = [pytest.param("stock", id="stock"), pytest.param("futopt", id="futopt")]

BAD_PEM = b"-----BEGIN CERTIFICATE-----\nnot base64!!\n-----END CERTIFICATE-----\n"

hard_timeout = pytest.mark.timeout(20, method="thread")


def _assert_bad_root_cert(excinfo):
    assert isinstance(excinfo.value, ConfigError)
    assert isinstance(excinfo.value, MarketDataError)
    assert excinfo.value.code == 1004
    assert "invalid TLS root cert PEM" in excinfo.value.message


def _ws(url, product):
    client = WebSocketClient(
        api_key="test-key",
        base_url=url,
        reconnect=ReconnectConfig.disabled(),
        tls_root_cert_pem=BAD_PEM,
    )
    return getattr(client, product)


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_connect_rejects_a_malformed_root_cert(product):
    with LoopbackServer() as srv:
        ws = _ws(srv.url, product)
        try:
            with pytest.raises(MarketDataError) as excinfo:
                ws.connect()
            _assert_bad_root_cert(excinfo)
            assert not ws.is_connected()
        finally:
            disconnect_quietly(ws)


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_connect_async_rejects_a_malformed_root_cert(product):
    async def run(ws):
        with pytest.raises(MarketDataError) as excinfo:
            await ws.connect_async()
        _assert_bad_root_cert(excinfo)

    with LoopbackServer() as srv:
        ws = _ws(srv.url, product)
        try:
            asyncio.run(run(ws))
            assert not ws.is_connected()
        finally:
            disconnect_quietly(ws)
