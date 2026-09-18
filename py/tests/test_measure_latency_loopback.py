"""``measure_latency()`` and the health check probe settings (#150), against
a loopback server that answers ``ping`` like the Fugle server does.

``measure_latency()`` waits for its pong and returns the round trip in
milliseconds; the old fire-and-forget ``ping()`` still delivers its pong to
the ``message`` callback.
"""
import json

import pytest

from fugle_marketdata import HealthCheckConfig, WebSocketError
from tests.ws_loopback import (
    TIMEOUT_S,
    LoopbackServer,
    Recorder,
    disconnect_quietly,
    product_ws,
)

PRODUCTS = [pytest.param("stock", id="stock"), pytest.param("futopt", id="futopt")]

hard_timeout = pytest.mark.timeout(20, method="thread")


@pytest.fixture
def server():
    with LoopbackServer() as srv:
        yield srv


def _pongs(recorder):
    def event(message):
        return (json.loads(message) if isinstance(message, str) else message).get("event")

    return [args for args in recorder.args_of("message") if event(args[0]) == "pong"]


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_measure_latency_returns_milliseconds(server, product):
    ws = product_ws(server.url, product)
    try:
        ws.connect()
        recorder = Recorder(ws, messages=True)
        latency = ws.measure_latency()
        assert isinstance(latency, float)
        assert 0 <= latency < TIMEOUT_S * 1000
        # The SDK's own pong is not the caller's message.
        ws.ping("mine")
        recorder.wait_until(lambda calls: _pongs(recorder), TIMEOUT_S, "the ping() pong")
        assert len(_pongs(recorder)) == 1
    finally:
        disconnect_quietly(ws)


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_measure_latency_needs_a_connection(server, product):
    ws = product_ws(server.url, product)
    with pytest.raises(WebSocketError) as exc_info:
        ws.measure_latency()
    assert exc_info.value.code == 2010


@hard_timeout
def test_measure_latency_rejects_a_zero_timeout(server):
    ws = product_ws(server.url, "stock")
    try:
        ws.connect()
        with pytest.raises(Exception) as exc_info:
            ws.measure_latency(timeout_ms=0)
        assert exc_info.value.code == 1005
    finally:
        disconnect_quietly(ws)


@hard_timeout
async def test_measure_latency_async(server):
    ws = product_ws(server.url, "stock")
    try:
        await ws.connect_async()
        latency = await ws.measure_latency_async(timeout_ms=2000)
        assert 0 <= latency < 2000
    finally:
        await ws.disconnect_async()


@hard_timeout
@pytest.mark.parametrize("product", PRODUCTS)
def test_probe_mode_connects(server, product):
    health = HealthCheckConfig(probe_enabled=True, idle_probe_after_ms=5000, probe_timeout_ms=1000)
    ws = product_ws(server.url, product, health_check=health)
    try:
        ws.connect()
        assert ws.is_connected()
    finally:
        disconnect_quietly(ws)
