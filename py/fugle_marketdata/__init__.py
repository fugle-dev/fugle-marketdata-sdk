"""
Fugle Market Data SDK - Python bindings

Drop-in replacement for fugle-marketdata-python with Rust performance.

Usage:
    from fugle_marketdata import RestClient, WebSocketClient

    # REST API: methods block and return a dict, as in 2.x
    client = RestClient(api_key="your-api-key")
    quote = client.stock.intraday.quote("2330")

    # ... and each has an _async sibling for asyncio code
    async def main():
        quote = await client.stock.intraday.quote_async("2330")

    # WebSocket (async iterator)
    async def stream():
        ws = WebSocketClient(api_key="your-api-key")
        await ws.stock.connect_async()
        await ws.stock.subscribe_async("trades", "2330")
        async for msg in ws.stock.messages():
            print(msg)
"""

from .fugle_marketdata import (
    # Clients
    RestClient,
    WebSocketClient,
    # Sub-clients (for type hints)
    StockClient,
    StockIntradayClient,
    FutOptClient,
    FutOptIntradayClient,
    StockWebSocketClient,
    FutOptWebSocketClient,
    # Iterators
    MessageIterator,
    # Exceptions
    MarketDataError,
    ApiError,
    AuthError,
    ConfigError,
    RateLimitError,
    ConnectionError,
    TimeoutError,
    WebSocketError,
    # Backward-compat alias for the legacy fugle-marketdata-python single
    # exception class. Resolves to MarketDataError so `except FugleAPIError:`
    # keeps catching every error variant.
    FugleAPIError,
    # Warning for the 2.x HealthCheckConfig fields 3.0 ignores
    FugleHealthCheckWarning,
    # Config
    ReconnectConfig,
    HealthCheckConfig,
    # Events
    DisconnectInfo,
)

from importlib.metadata import version as _pkg_version, PackageNotFoundError

try:
    __version__ = _pkg_version("fugle-marketdata")
except PackageNotFoundError:
    __version__ = "0.0.0+unknown"

del _pkg_version, PackageNotFoundError

__all__ = [
    "RestClient",
    "WebSocketClient",
    "StockClient",
    "StockIntradayClient",
    "FutOptClient",
    "FutOptIntradayClient",
    "StockWebSocketClient",
    "FutOptWebSocketClient",
    "MessageIterator",
    "MarketDataError",
    "ApiError",
    "AuthError",
    "ConfigError",
    "RateLimitError",
    "ConnectionError",
    "TimeoutError",
    "WebSocketError",
    "FugleAPIError",
    "FugleHealthCheckWarning",
    "ReconnectConfig",
    "HealthCheckConfig",
    "DisconnectInfo",
    "__version__",
]
