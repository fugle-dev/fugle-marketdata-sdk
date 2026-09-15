"""
Fugle Market Data SDK - Python bindings

Drop-in replacement for fugle-marketdata-python with Rust performance.

Usage:
    from fugle_marketdata import RestClient, WebSocketClient

    # REST API (async)
    async def main():
        client = RestClient("your-api-key")
        quote = await client.stock.intraday.quote("2330")
        print(quote)

    # WebSocket (async iterator)
    async def stream():
        ws = WebSocketClient("your-api-key")
        await ws.stock.connect()
        await ws.stock.subscribe("trades", "2330")
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
    RateLimitError,
    ConnectionError,
    TimeoutError,
    WebSocketError,
    # Backward-compat alias for the legacy fugle-marketdata-python single
    # exception class. Resolves to MarketDataError so `except FugleAPIError:`
    # keeps catching every error variant.
    FugleAPIError,
    # Config
    ReconnectConfig,
    HealthCheckConfig,
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
    "RateLimitError",
    "ConnectionError",
    "TimeoutError",
    "WebSocketError",
    "FugleAPIError",
    "ReconnectConfig",
    "HealthCheckConfig",
    "__version__",
]
