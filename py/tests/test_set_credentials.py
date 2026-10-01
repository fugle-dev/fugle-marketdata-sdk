"""``set_credentials()`` changes what later connection attempts and requests
send (#322).

The loopback server accepts only the new credential once the test asks it
to. A client connected with the old one, given the new one, authenticates its
automatic reconnect with it, in the field of its kind; a live connection gets
no new auth frame. ``ws.set_credentials()`` reaches both product clients,
built or not; ``ws.stock.set_credentials()`` only the stock client. After a
rejection — of ``connect()`` or of a reconnect — setting the new credential
and calling ``connect()`` again succeeds. A REST client sends the new
credential from its next request on, through product clients taken before.
Anything but exactly one non-blank credential is ``ConfigError`` (1004) and
leaves the current one in place.
"""
import asyncio
import time

import pytest

from fugle_marketdata import AuthError, ConfigError, ReconnectConfig, RestClient, WebSocketClient
from tests.rest_loopback import RestLoopbackServer
from tests.ws_loopback import (
    TIMEOUT_S,
    InProcessLoopbackServer,
    Recorder,
    disconnect_quietly,
)

PRODUCTS = [pytest.param("stock", id="stock"), pytest.param("futopt", id="futopt")]
OLD = "old-key"
NEW = "new-token"

hard_timeout = pytest.mark.timeout(30, method="thread")


@pytest.fixture
def server():
    with InProcessLoopbackServer() as srv:
        yield srv


def client(url, max_attempts=2):
    return WebSocketClient(
        api_key=OLD,
        base_url=url,
        reconnect=ReconnectConfig(
            enabled=True, max_attempts=max_attempts, initial_delay_ms=100, max_delay_ms=100
        ),
    )


def wait_until(predicate, what):
    deadline = time.monotonic() + TIMEOUT_S
    while not predicate():
        assert time.monotonic() < deadline, f"no {what} within {TIMEOUT_S}s"
        time.sleep(0.01)


def reauthenticated(recorder):
    recorder.wait_until(
        lambda calls: [name for name, _ in calls].count("authenticated") == 2,
        TIMEOUT_S,
        "second authenticated",
    )


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
@pytest.mark.parametrize("on_parent", [False, True], ids=["product", "parent"])
def test_reconnect_sends_the_new_credential_of_another_kind(server, product, on_parent):
    ws_client = client(server.url)
    ws = getattr(ws_client, product)
    recorder = Recorder(ws)
    try:
        ws.connect()
        server.require_credential(NEW)
        target = ws_client if on_parent else ws
        target.set_credentials(sdk_token=NEW)

        server.drop_connections()
        reauthenticated(recorder)

        assert server.auth_data == [{"apikey": OLD}, {"sdkToken": NEW}]
        assert "unauthenticated" not in recorder.names()
    finally:
        disconnect_quietly(ws)


@hard_timeout
def test_live_connection_gets_no_new_auth_frame(server):
    ws = client(server.url).stock
    try:
        ws.connect()
        ws.set_credentials(bearer_token=NEW)
        time.sleep(0.2)
        assert server.auth_data == [{"apikey": OLD}]
    finally:
        disconnect_quietly(ws)


@hard_timeout
def test_parent_reaches_both_products_and_one_not_built_yet(server):
    ws_client = client(server.url)
    stock = ws_client.stock
    ws_client.set_credentials(bearer_token=NEW)
    futopt = ws_client.futopt
    try:
        stock.connect()
        futopt.connect()
        assert server.auth_data == [{"token": NEW}, {"token": NEW}]
    finally:
        disconnect_quietly(stock)
        disconnect_quietly(futopt)


@hard_timeout
def test_product_changes_only_its_own(server):
    ws_client = client(server.url)
    stock, futopt = ws_client.stock, ws_client.futopt
    stock.set_credentials(sdk_token=NEW)
    try:
        stock.connect()
        futopt.connect()
        assert server.auth_data == [{"sdkToken": NEW}, {"apikey": OLD}]
    finally:
        disconnect_quietly(stock)
        disconnect_quietly(futopt)


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_rejected_connect_succeeds_after_the_credential_is_set(server, product):
    server.require_credential(NEW)
    ws = getattr(client(server.url), product)
    try:
        with pytest.raises(AuthError):
            ws.connect()
        ws.set_credentials(sdk_token=NEW)
        ws.connect()
        assert server.auth_data == [{"apikey": OLD}, {"sdkToken": NEW}]
    finally:
        disconnect_quietly(ws)


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_connect_after_a_rejected_reconnect_sends_the_new_credential(server, product):
    ws = getattr(client(server.url), product)
    recorder = Recorder(ws)
    try:
        ws.connect()
        server.require_credential(NEW)
        server.drop_connections()
        recorder.wait_for("unauthenticated", TIMEOUT_S)
        # The rejection ends the reconnect and closes the client.
        wait_until(ws.is_closed, "the reconnect to end")

        ws.set_credentials(sdk_token=NEW)
        ws.connect()
        assert server.auth_data[-1] == {"sdkToken": NEW}
    finally:
        disconnect_quietly(ws)


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_connect_async_sends_the_new_credential(server, product):
    async def run(ws):
        try:
            await ws.connect_async()
        finally:
            await ws.disconnect_async()

    ws = getattr(client(server.url), product)
    ws.set_credentials(api_key=NEW)
    asyncio.run(run(ws))
    assert server.auth_data == [{"apikey": NEW}]


BAD = [
    pytest.param({}, id="none"),
    pytest.param({"api_key": "a", "sdk_token": "b"}, id="two"),
    pytest.param({"bearer_token": "   "}, id="blank"),
]


@hard_timeout
@pytest.mark.parametrize("credentials", BAD)
def test_invalid_credentials_are_config_error_and_keep_the_current_one(server, credentials):
    ws_client = client(server.url)
    ws = ws_client.stock
    for target in (ws_client, ws):
        with pytest.raises(ConfigError) as err:
            target.set_credentials(**credentials)
        assert err.value.code == 1004
    try:
        ws.connect()
        assert server.auth_data == [{"apikey": OLD}]
    finally:
        disconnect_quietly(ws)


def test_set_credentials_takes_keywords_only():
    ws = WebSocketClient(api_key=OLD)
    with pytest.raises(TypeError):
        ws.set_credentials(NEW)
    with pytest.raises(TypeError):
        RestClient(api_key=OLD).set_credentials(NEW)


def test_rest_sends_the_new_credential_through_product_clients_taken_before():
    with RestLoopbackServer() as srv:
        rest = RestClient(api_key=OLD, base_url=srv.url)
        intraday = rest.stock.intraday
        rest.set_credentials(sdk_token=NEW)
        for call in (lambda: intraday.quote(symbol="2330"), lambda: rest.stock.intraday.quote(symbol="2330")):
            with pytest.raises(Exception):
                call()  # the server answers 401; only the header matters
        assert [h.get("x-sdk-token") for h in srv.headers] == [NEW, NEW]
        assert all("x-api-key" not in h for h in srv.headers)


@pytest.mark.parametrize("credentials", BAD + [pytest.param({"api_key": "bad\nkey"}, id="header")])
def test_rest_invalid_credentials_are_config_error_and_keep_the_current_one(credentials):
    with RestLoopbackServer() as srv:
        rest = RestClient(bearer_token=OLD, base_url=srv.url)
        with pytest.raises(ConfigError) as err:
            rest.set_credentials(**credentials)
        assert err.value.code == 1004
        with pytest.raises(Exception):
            rest.stock.intraday.quote(symbol="2330")
        assert srv.headers[0]["authorization"] == f"Bearer {OLD}"
