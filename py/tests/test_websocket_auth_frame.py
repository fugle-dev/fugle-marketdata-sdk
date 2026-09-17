"""The auth frame carries each credential in its own field (#91).

The server reads ``apikey``, ``token`` or ``sdkToken`` and rejects a frame
with more than one. Every kind used to go out as ``apikey``.
"""
import asyncio

import pytest

from fugle_marketdata import ReconnectConfig, WebSocketClient
from tests.ws_loopback import InProcessLoopbackServer

CREDENTIALS = [
    pytest.param({"api_key": "the-key"}, {"apikey": "the-key"}, id="api_key"),
    pytest.param({"bearer_token": "the-token"}, {"token": "the-token"}, id="bearer_token"),
    pytest.param({"sdk_token": "the-sdk-token"}, {"sdkToken": "the-sdk-token"}, id="sdk_token"),
]

hard_timeout = pytest.mark.timeout(20, method="thread")


def _client(url, credential):
    return WebSocketClient(base_url=url, reconnect=ReconnectConfig.disabled(), **credential)


@hard_timeout
@pytest.mark.parametrize("product", ["stock", "futopt"])
@pytest.mark.parametrize("credential, expected", CREDENTIALS)
def test_connect_sends_credential_in_its_field(product, credential, expected):
    with InProcessLoopbackServer() as srv:
        ws = getattr(_client(srv.url, credential), product)
        try:
            ws.connect()
        finally:
            ws.disconnect()
        assert srv.auth_data == [expected]


@hard_timeout
@pytest.mark.parametrize("credential, expected", CREDENTIALS)
def test_connect_async_sends_credential_in_its_field(credential, expected):
    async def run(url):
        ws = _client(url, credential).stock
        try:
            await ws.connect_async()
        finally:
            await ws.disconnect_async()

    with InProcessLoopbackServer() as srv:
        asyncio.run(run(srv.url))
        assert srv.auth_data == [expected]
