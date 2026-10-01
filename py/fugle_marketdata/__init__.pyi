"""Type stubs for fugle_marketdata

Fugle Market Data SDK - Python bindings with full type annotations.
"""
from typing import Any, Awaitable, Callable, Generic, Literal, Mapping, Optional, List, overload

# TypeVar with a default (PEP 696); type checkers resolve this import in a
# stub without typing_extensions being installed.
from typing_extensions import Self, TypeVar

# A message frame, as the server sent it.
Message = dict[str, Any]

# What a MessageIterator yields: Message, or str from messages(raw=True).
# A bare `MessageIterator` annotation means `MessageIterator[Message]`.
# `Message` is spelled out: mypy 1.14, the last to target Python 3.8, rejects
# the alias as the default of a constrained TypeVar.
_Yielded = TypeVar("_Yielded", dict[str, Any], str, default=dict[str, Any])

__version__: str

# Exception hierarchy
class MarketDataError(Exception):
    """Base exception for all market data errors.

    All SDK exceptions inherit from this class, making it easy to catch
    any SDK-related error with a single except clause.

    Every instance carries the unified error fields (see docs/errors.md).
    """
    code: int
    """Numeric error code (also ``args[1]``)."""
    source_kind: Literal["network", "protocol", "auth", "rate_limit", "client"]
    """Category of the failure."""
    message: str
    """Human-readable message (also ``args[0]`` and ``str(e)``)."""
    status: Optional[int]
    """HTTP status, when the error came from an HTTP response."""
    body: Optional[str]
    """Raw HTTP response body (REST only)."""
    request_id: Optional[str]
    """Server-assigned request id (``x-request-id``), when present."""
    headers: dict[str, str]
    """HTTP response headers with lowercase names (REST only; empty otherwise)."""
    status_code: Optional[int]
    """Alias of ``status``, kept from the 2.4.1 SDK's ``FugleAPIError``."""
    response_text: Optional[str]
    """Alias of ``body``, kept from the 2.4.1 SDK's ``FugleAPIError``."""
    url: None
    """Always None; kept from the 2.4.1 SDK's ``FugleAPIError``."""
    params: None
    """Always None; kept from the 2.4.1 SDK's ``FugleAPIError``."""

    def __str__(self) -> str:
        """``message``. An instance raised without one (``MarketDataError("...")``)
        prints like any exception."""
        ...

class ApiError(MarketDataError):
    """API returned an error response.

    Raised when the Fugle API returns an error status code or error message.
    Contains details about the specific API error.
    """
    ...

class AuthError(MarketDataError):
    """Authentication failed.

    Raised when the API key is invalid, expired, or missing.
    """
    ...

class ConfigError(MarketDataError):
    """Invalid configuration (code 1004, ``source_kind == "client"``).

    Raised by ``ReconnectConfig`` and ``HealthCheckConfig`` for a value
    below its floor, and by the client constructors when not exactly one
    non-empty credential is given. Not a ``ValueError``: catch
    ``ConfigError`` (or ``MarketDataError``), as the other languages'
    ``ConfigError`` (#171).
    """
    ...

class RateLimitError(ApiError):
    """Rate limit exceeded.

    Raised when too many requests have been made in a short period.
    Inherits from ApiError as it's a specific API error type.
    """
    ...

class ConnectionError(MarketDataError):
    """Network connection failed (code 2001, ``source_kind == "network"``).

    Raised when a REST request cannot reach the server (DNS resolution,
    connection refused, TLS), when a WebSocket command is sent while the
    connection is down, or when the WebSocket auth handshake fails for a
    reason other than rejected credentials. Not a ``WebSocketError`` (#219);
    a WebSocket *connect* that cannot reach the server is ``WebSocketError``
    code 3002.
    """
    ...

class TimeoutError(MarketDataError):
    """Operation timed out.

    Raised when an API call or WebSocket operation exceeds the timeout.
    """
    ...

class WebSocketError(MarketDataError):
    """WebSocket protocol error.

    Raised for WebSocket-specific errors like connection drops,
    protocol violations, or message parsing failures.
    """
    ...

# Backward-compat alias for the old `fugle-marketdata` SDK.
# Aliased to MarketDataError so `except FugleAPIError:` keeps catching every
# variant raised by this binding.
FugleAPIError = MarketDataError

class FugleHealthCheckWarning(UserWarning):
    """Issued when ``HealthCheckConfig`` is given the 2.x fields
    ``ping_interval`` / ``max_missed_pongs``, which 3.0 ignores.

    Issued on every such call; Python's warning filters decide what is shown
    (by default once per calling line).
    """
    code: str
    """``"FUGLE_HEALTH_CHECK_LEGACY_OPTIONS"``, as on Node's warning."""

# REST Client
class RestClient:
    """REST client for Fugle market data API.

    Provides access to stock and futures/options market data through
    REST endpoints. All data methods are async and return coroutines.

    Example:
        ```python
        import asyncio
        from fugle_marketdata import RestClient

        async def main():
            client = RestClient(api_key="your-api-key")
            quote = await client.stock.intraday.quote_async("2330")
            print(f"Last price: {quote['lastPrice']}")

        asyncio.run(main())
        ```
    """

    def __init__(
        self,
        *,
        api_key: str | None = None,
        bearer_token: str | None = None,
        sdk_token: str | None = None,
        base_url: str | None = None,
        tls_ca_file: str | None = None,
        tls_root_cert_pem: bytes | None = None,
        tls_accept_invalid_certs: bool = False,
    ) -> None:
        """Create a new REST client with authentication.

        Args:
            api_key: Your Fugle API key (exactly one non-empty auth method required)
            bearer_token: Bearer token for authentication (exactly one non-empty auth method required)
            sdk_token: SDK token for authentication (exactly one non-empty auth method required)
            base_url: Optional custom base URL — host and path prefix ONLY.
                The SDK appends the version segment; a base_url that already
                ends in one (e.g. ".../v1.0") raises TypeError. Changed in
                0.8.0, which reversed the 0.6-0.7 rule.
            tls_ca_file: Path to a PEM-encoded root CA to trust (in addition to
                the system trust store). Mutually exclusive with tls_root_cert_pem.
            tls_root_cert_pem: Raw PEM bytes of a root CA to trust. Mutually
                exclusive with tls_ca_file.
            tls_accept_invalid_certs: DANGER — disable ALL TLS verification
                (chain + hostname). Equivalent to ``curl -k``. Prefer
                tls_ca_file for production. Emits UserWarning when set.

        Raises:
            ConfigError: code 1004 if zero or multiple non-empty auth
                methods are given (empty or whitespace-only values count as
                not given)
            TypeError: both TLS cert options set, or a base_url carrying a
                version segment
            OSError: tls_ca_file path not readable
            ValueError: tls_root_cert_pem contents not a valid PEM certificate

        Example:
            ```python
            # API key auth
            client = RestClient(api_key="your-key")

            # Custom base URL — no version segment; the SDK appends it
            client = RestClient(api_key="key", base_url="https://custom.api")
            assert client.base_url == "https://custom.api/v1.0"

            # Self-signed deployment — pin the CA
            client = RestClient(api_key="key",
                                base_url="https://custom.api",
                                tls_ca_file="/path/to/ca.pem")

            # Local testing only — skip all TLS verification
            client = RestClient(api_key="key",
                                base_url="https://192.168.1.1/v1",
                                tls_accept_invalid_certs=True)
            ```
        """
        ...

    @property
    def base_url(self) -> str:
        """The prefix every request from this client is built on, fully
        resolved — host, path prefix and version segment.

        The version segment is chosen by the SDK rather than written by the
        caller, so this is the only way to see what a client resolved to.
        """
        ...

    @staticmethod
    def with_bearer_token(token: str) -> "RestClient":
        """Create a REST client with bearer token authentication.

        Args:
            token: Bearer token for authentication

        Returns:
            A new RestClient instance

        Raises:
            ConfigError: code 1004 if the token is empty or whitespace

        Note:
            This is a convenience method for backwards compatibility.
            Prefer using RestClient(bearer_token="token").
        """
        ...

    @staticmethod
    def with_sdk_token(sdk_token: str) -> "RestClient":
        """Create a REST client with SDK token authentication.

        Args:
            sdk_token: SDK token for authentication

        Returns:
            A new RestClient instance

        Raises:
            ConfigError: code 1004 if the token is empty or whitespace

        Note:
            This is a convenience method for backwards compatibility.
            Prefer using RestClient(sdk_token="token").
        """
        ...

    @property
    def stock(self) -> "StockClient":
        """Access stock market data endpoints.

        Returns:
            StockClient for accessing stock endpoints
        """
        ...

    @property
    def futopt(self) -> "FutOptClient":
        """Access futures and options market data endpoints.

        Returns:
            FutOptClient for accessing FutOpt endpoints
        """
        ...


class StockClient:
    """Stock market data client.

    Access via `client.stock`. Provides access to intraday, historical,
    snapshot, technical, and corporate actions stock data.
    """

    @property
    def intraday(self) -> "StockIntradayClient":
        """Access intraday (real-time) stock endpoints.

        Returns:
            StockIntradayClient for accessing intraday endpoints
        """
        ...

    @property
    def historical(self) -> "StockHistoricalClient":
        """Access historical stock data endpoints.

        Returns:
            StockHistoricalClient for accessing historical endpoints
        """
        ...

    @property
    def snapshot(self) -> "StockSnapshotClient":
        """Access snapshot endpoints for market-wide data.

        Returns:
            StockSnapshotClient for accessing snapshot endpoints
        """
        ...

    @property
    def ownership(self) -> "StockOwnershipClient":
        """Access ownership endpoints (ETF holdings).

        Returns:
            StockOwnershipClient for accessing ownership endpoints
        """
        ...

    @property
    def base_url(self) -> str:
        """The fully resolved request prefix for this product client."""
        ...

    @property
    def technical(self) -> "StockTechnicalClient":
        """Access technical indicator endpoints.

        Returns:
            StockTechnicalClient for accessing technical endpoints
        """
        ...

    @property
    def corporate_actions(self) -> "StockCorporateActionsClient":
        """Access corporate actions endpoints.

        Returns:
            StockCorporateActionsClient for accessing corporate actions endpoints
        """
        ...


class StockIntradayClient:
    """Stock intraday (real-time) endpoints client.

    Access via `client.stock.intraday`. Each async method has a `_sync`
    sibling (e.g. `quote_sync`) that blocks the calling thread and returns
    the dict directly — provided for callers migrating from the legacy
    fugle-marketdata SDK.
    """

    async def quote_async(self, symbol: str, *, odd_lot: Optional[bool] = None) -> dict[str, Any]:
        """Get intraday quote for a stock symbol.

        Args:
            symbol: Stock symbol (e.g., "2330" for TSMC)
            odd_lot: Whether to query odd lot data (default: False)

        Returns:
            Quote data including prices, order book, and trading info

        Raises:
            MarketDataError: If the request fails

        Example:
            ```python
            quote = await client.stock.intraday.quote_async("2330")
            print(f"Last price: {quote['lastPrice']}")
            print(f"Change: {quote['change']}")
            ```
        """
        ...

    async def ticker_async(self, symbol: str, *, odd_lot: Optional[bool] = None) -> dict[str, Any]:
        """Get ticker information for a stock symbol.

        Args:
            symbol: Stock symbol (e.g., "2330" for TSMC)
            odd_lot: Query odd-lot (盤中零股) data instead of board-lot

        Returns:
            Ticker data including name, industry, and basic info

        Raises:
            MarketDataError: If the request fails
        """
        ...

    async def candles_async(
        self,
        symbol: str,
        *,
        timeframe: Optional[str] = None,
        odd_lot: Optional[bool] = None,
        sort: Optional[str] = None,
    ) -> dict[str, Any]:
        """Get candlestick chart data.

        Args:
            symbol: Stock symbol (e.g., "2330" for TSMC)
            timeframe: Timeframe in minutes (default: "1")
            odd_lot: Query odd-lot (盤中零股) data instead of board-lot
            sort: Sort order, "asc" or "desc"

        Returns:
            Candlestick data with OHLCV values

        Raises:
            MarketDataError: If the request fails
        """
        ...

    async def trades_async(
        self,
        symbol: str,
        *,
        odd_lot: Optional[bool] = None,
        offset: Optional[int] = None,
        limit: Optional[int] = None,
        sort: Optional[str] = None,
        is_trial: Optional[bool] = None,
    ) -> dict[str, Any]:
        """Get trade ticks data.

        Args:
            symbol: Stock symbol (e.g., "2330" for TSMC)
            odd_lot: Query odd-lot (盤中零股) data instead of board-lot
            offset: Number of trades to skip
            limit: Maximum number of trades to return
            sort: Sort order, "asc" (oldest first) or "desc" (newest first, the server default)
            is_trial: Only trial-matching (試撮合) trades

        Returns:
            Trade ticks data with price, volume, and time

        Raises:
            MarketDataError: If the request fails
        """
        ...

    async def volumes_async(self, symbol: str, *, odd_lot: Optional[bool] = None) -> dict[str, Any]:
        """Get volume data.

        Args:
            symbol: Stock symbol (e.g., "2330" for TSMC)
            odd_lot: Query odd-lot (盤中零股) data instead of board-lot

        Returns:
            Volume data by price level

        Raises:
            MarketDataError: If the request fails
        """
        ...

    async def tickers_async(
        self,
        type: str,
        *,
        exchange: str | None = None,
        market: str | None = None,
        industry: str | None = None,
        is_normal: bool | None = None,
        is_attention: bool | None = None,
        is_disposition: bool | None = None,
        is_halted: bool | None = None,
        symbol: str | None = None,
    ) -> dict[str, Any]:
        """Get batch ticker list for a security type.

        Args:
            type: Security type (e.g., "EQUITY", "INDEX", "ETF")
            exchange: Exchange filter (e.g., "TWSE", "TPEx")
            market: Market filter (e.g., "TSE", "OTC")
            industry: Industry code filter
            is_normal: Filter to normal-status tickers only
            is_attention: Only attention stocks (注意股)
            is_disposition: Only disposition stocks (處置股)
            is_halted: Only halted stocks
            symbol: Comma-separated symbols to restrict the list to

        Returns:
            Response envelope: the query fields (``date``, ``type``, ``exchange``, ...) and ``data``, the list of ticker info dicts

        Raises:
            MarketDataError: If the request fails
        """
        ...

    # ----- Sync siblings (legacy fugle-marketdata compatibility) -----

    def quote(self, symbol: str, *, odd_lot: Optional[bool] = None) -> dict[str, Any]:
        """Blocking version of `quote()`."""
        ...

    def ticker(self, symbol: str, *, odd_lot: Optional[bool] = None) -> dict[str, Any]:
        """Blocking version of `ticker()`."""
        ...

    def candles(
        self,
        symbol: str,
        *,
        timeframe: Optional[str] = None,
        odd_lot: Optional[bool] = None,
        sort: Optional[str] = None,
    ) -> dict[str, Any]:
        """Blocking version of `candles()`."""
        ...

    def trades(
        self,
        symbol: str,
        *,
        odd_lot: Optional[bool] = None,
        offset: Optional[int] = None,
        limit: Optional[int] = None,
        sort: Optional[str] = None,
        is_trial: Optional[bool] = None,
    ) -> dict[str, Any]:
        """Blocking version of `trades()`."""
        ...

    def volumes(self, symbol: str, *, odd_lot: Optional[bool] = None) -> dict[str, Any]:
        """Blocking version of `volumes()`."""
        ...

    def tickers(
        self,
        type: str,
        *,
        exchange: str | None = None,
        market: str | None = None,
        industry: str | None = None,
        is_normal: bool | None = None,
        is_attention: bool | None = None,
        is_disposition: bool | None = None,
        is_halted: bool | None = None,
        symbol: str | None = None,
    ) -> dict[str, Any]:
        """Blocking version of `tickers()`."""
        ...


class StockHistoricalClient:
    """Stock historical data endpoints client.

    Access via `client.stock.historical`. All methods are async and
    return coroutines that resolve to dict objects.
    """

    async def candles_async(
        self,
        symbol: str,
        *,
        from_date: Optional[str] = None,
        to_date: Optional[str] = None,
        timeframe: Optional[str] = None,
        fields: Optional[str] = None,
        sort: Optional[str] = None,
        adjusted: Optional[bool] = None,
    ) -> dict[str, Any]:
        """Get historical candles for a stock symbol.

        Args:
            symbol: Stock symbol (e.g., "2330" for TSMC)
            from_date: Start date (YYYY-MM-DD)
            to_date: End date (YYYY-MM-DD)
            timeframe: Timeframe ("D", "W", "M", "1", "5", "10", "15", "30", "60")
            fields: Optional field selection
            sort: Sort order ("asc" or "desc")
            adjusted: Whether to adjust for splits/dividends

        Returns:
            Historical candles data

        Raises:
            MarketDataError: If the request fails

        Example:
            ```python
            candles = await client.stock.historical.candles_async(
                "2330",
                from_date="2024-01-01",
                to_date="2024-01-31",
                timeframe="D"
            )
            ```
        """
        ...

    async def stats_async(self, symbol: str) -> dict[str, Any]:
        """Get historical stats for a stock symbol.

        Args:
            symbol: Stock symbol (e.g., "2330" for TSMC)

        Returns:
            Historical stats data including 52-week high/low

        Raises:
            MarketDataError: If the request fails

        Example:
            ```python
            stats = await client.stock.historical.stats_async("2330")
            print(f"52-week high: {stats['week52High']}")
            ```
        """
        ...

    # ----- Sync siblings (legacy fugle-marketdata compatibility) -----

    def candles(
        self,
        symbol: str,
        *,
        from_date: Optional[str] = None,
        to_date: Optional[str] = None,
        timeframe: Optional[str] = None,
        fields: Optional[str] = None,
        sort: Optional[str] = None,
        adjusted: Optional[bool] = None,
    ) -> dict[str, Any]:
        """Blocking version of `candles()`."""
        ...

    def stats(self, symbol: str) -> dict[str, Any]:
        """Blocking version of `stats()`."""
        ...


class StockSnapshotClient:
    """Stock snapshot endpoints client.

    Access via `client.stock.snapshot`. All methods are async and
    return coroutines that resolve to dict objects.
    """

    async def quotes_async(
        self,
        market: str,
        *,
        type_filter: Optional[str] = None,
    ) -> dict[str, Any]:
        """Get snapshot quotes for a market.

        Args:
            market: Market code ("TSE", "OTC", "ESB", "TIB", "PSB")
            type_filter: Type filter ("ALL", "ALLBUT0999", "COMMONSTOCK")

        Returns:
            Market-wide quotes snapshot

        Raises:
            MarketDataError: If the request fails

        Example:
            ```python
            quotes = await client.stock.snapshot.quotes_async("TSE", type_filter="COMMONSTOCK")
            ```
        """
        ...

    async def movers_async(
        self,
        market: str,
        direction: Optional[str] = None,
        change: Optional[str] = None,
        *,
        type_filter: Optional[str] = None,
        gt: Optional[float] = None,
        gte: Optional[float] = None,
        lt: Optional[float] = None,
        lte: Optional[float] = None,
        eq: Optional[float] = None,
    ) -> dict[str, Any]:
        """Get top movers for a market.

        Args:
            market: Market code ("TSE", "OTC", "ESB", "TIB", "PSB")
            direction: Direction filter ("up" for gainers, "down" for losers)
            change: Change type ("percent" or "value")
            type_filter: Stock type filter, "ALLBUT0999" or "COMMONSTOCK" (sent as `type`)
            gt: Only changes greater than this
            gte: Only changes greater than or equal to this
            lt: Only changes less than this
            lte: Only changes less than or equal to this
            eq: Only changes equal to this

        Returns:
            Top movers data

        Raises:
            MarketDataError: If the request fails

        Example:
            ```python
            movers = await client.stock.snapshot.movers_async("TSE", direction="up", change="percent")
            ```
        """
        ...

    async def actives_async(
        self,
        market: str,
        trade: Optional[str] = None,
        *,
        type_filter: Optional[str] = None,
    ) -> dict[str, Any]:
        """Get most active stocks for a market.

        Args:
            market: Market code ("TSE", "OTC", "ESB", "TIB", "PSB")
            trade: Trade type ("volume" or "value")
            type_filter: Stock type filter, "ALLBUT0999" or "COMMONSTOCK" (sent as `type`)

        Returns:
            Most active stocks data

        Raises:
            MarketDataError: If the request fails

        Example:
            ```python
            actives = await client.stock.snapshot.actives_async("TSE", trade="volume")
            ```
        """
        ...

    # ----- Sync siblings (legacy fugle-marketdata compatibility) -----

    def quotes(self, market: str, *, type_filter: Optional[str] = None) -> dict[str, Any]:
        """Blocking version of `quotes()`."""
        ...

    def movers(
        self,
        market: str,
        direction: Optional[str] = None,
        change: Optional[str] = None,
        *,
        type_filter: Optional[str] = None,
        gt: Optional[float] = None,
        gte: Optional[float] = None,
        lt: Optional[float] = None,
        lte: Optional[float] = None,
        eq: Optional[float] = None,
    ) -> dict[str, Any]:
        """Blocking version of `movers()`."""
        ...

    def actives(
        self, market: str, trade: Optional[str] = None, *, type_filter: Optional[str] = None
    ) -> dict[str, Any]:
        """Blocking version of `actives()`."""
        ...


class StockTechnicalClient:
    """Stock technical indicator endpoints client.

    Access via `client.stock.technical`. All methods are async and
    return coroutines that resolve to dict objects.
    """

    async def sma_async(
        self,
        symbol: str,
        period: Optional[int] = None,
        *,
        from_date: Optional[str] = None,
        to_date: Optional[str] = None,
        timeframe: Optional[str] = None,
    ) -> dict[str, Any]:
        """Get Simple Moving Average (SMA) data.

        Args:
            symbol: Stock symbol (e.g., "2330" for TSMC)
            period: Moving average period
            from_date: Start date (YYYY-MM-DD)
            to_date: End date (YYYY-MM-DD)
            timeframe: Timeframe ("D", "W", "M", "1", "5", etc.)

        Returns:
            SMA indicator data

        Raises:
            MarketDataError: If the request fails
        """
        ...

    async def rsi_async(
        self,
        symbol: str,
        period: Optional[int] = None,
        *,
        from_date: Optional[str] = None,
        to_date: Optional[str] = None,
        timeframe: Optional[str] = None,
    ) -> dict[str, Any]:
        """Get Relative Strength Index (RSI) data.

        Args:
            symbol: Stock symbol (e.g., "2330" for TSMC)
            period: RSI period (default 14)
            from_date: Start date (YYYY-MM-DD)
            to_date: End date (YYYY-MM-DD)
            timeframe: Timeframe ("D", "W", "M", "1", "5", etc.)

        Returns:
            RSI indicator data

        Raises:
            MarketDataError: If the request fails
        """
        ...

    async def kdj_async(
        self,
        symbol: str,
        r_period: Optional[int] = None,
        k_period: Optional[int] = None,
        d_period: Optional[int] = None,
        *,
        from_date: Optional[str] = None,
        to_date: Optional[str] = None,
        timeframe: Optional[str] = None,
    ) -> dict[str, Any]:
        """Get KDJ (Stochastic Oscillator) data.

        Args:
            symbol: Stock symbol (e.g., "2330" for TSMC)
            r_period: RSV period (e.g., 9)
            k_period: K smoothing period (e.g., 3)
            d_period: D smoothing period (e.g., 3)
            from_date: Start date (YYYY-MM-DD)
            to_date: End date (YYYY-MM-DD)
            timeframe: Timeframe ("D", "W", "M", "1", "5", etc.)

        Returns:
            KDJ indicator data with K, D, J values

        Raises:
            MarketDataError: If the request fails
        """
        ...

    async def macd_async(
        self,
        symbol: str,
        fast: Optional[int] = None,
        slow: Optional[int] = None,
        signal: Optional[int] = None,
        *,
        from_date: Optional[str] = None,
        to_date: Optional[str] = None,
        timeframe: Optional[str] = None,
    ) -> dict[str, Any]:
        """Get MACD (Moving Average Convergence Divergence) data.

        Args:
            symbol: Stock symbol (e.g., "2330" for TSMC)
            fast: Fast EMA period (default 12)
            slow: Slow EMA period (default 26)
            signal: Signal line period (default 9)
            from_date: Start date (YYYY-MM-DD)
            to_date: End date (YYYY-MM-DD)
            timeframe: Timeframe ("D", "W", "M", "1", "5", etc.)

        Returns:
            MACD indicator data with MACD, signal, histogram

        Raises:
            MarketDataError: If the request fails
        """
        ...

    async def bb_async(
        self,
        symbol: str,
        period: Optional[int] = None,
        *,
        from_date: Optional[str] = None,
        to_date: Optional[str] = None,
        timeframe: Optional[str] = None,
    ) -> dict[str, Any]:
        """Get Bollinger Bands (BB) data.

        Args:
            symbol: Stock symbol (e.g., "2330" for TSMC)
            period: Moving average period (default 20)
            from_date: Start date (YYYY-MM-DD)
            to_date: End date (YYYY-MM-DD)
            timeframe: Timeframe ("D", "W", "M", "1", "5", etc.)

        Returns:
            Bollinger Bands data with upper, middle, lower bands

        Raises:
            MarketDataError: If the request fails
        """
        ...

    # ----- Sync siblings (legacy fugle-marketdata compatibility) -----

    def sma(
        self,
        symbol: str,
        period: Optional[int] = None,
        *,
        from_date: Optional[str] = None,
        to_date: Optional[str] = None,
        timeframe: Optional[str] = None,
    ) -> dict[str, Any]:
        """Blocking version of `sma()`."""
        ...

    def rsi(
        self,
        symbol: str,
        period: Optional[int] = None,
        *,
        from_date: Optional[str] = None,
        to_date: Optional[str] = None,
        timeframe: Optional[str] = None,
    ) -> dict[str, Any]:
        """Blocking version of `rsi()`."""
        ...

    def kdj(
        self,
        symbol: str,
        r_period: Optional[int] = None,
        k_period: Optional[int] = None,
        d_period: Optional[int] = None,
        *,
        from_date: Optional[str] = None,
        to_date: Optional[str] = None,
        timeframe: Optional[str] = None,
    ) -> dict[str, Any]:
        """Blocking version of `kdj()`."""
        ...

    def macd(
        self,
        symbol: str,
        fast: Optional[int] = None,
        slow: Optional[int] = None,
        signal: Optional[int] = None,
        *,
        from_date: Optional[str] = None,
        to_date: Optional[str] = None,
        timeframe: Optional[str] = None,
    ) -> dict[str, Any]:
        """Blocking version of `macd()`."""
        ...

    def bb(
        self,
        symbol: str,
        period: Optional[int] = None,
        *,
        from_date: Optional[str] = None,
        to_date: Optional[str] = None,
        timeframe: Optional[str] = None,
    ) -> dict[str, Any]:
        """Blocking version of `bb()`."""
        ...


class StockOwnershipClient:
    """Stock ownership endpoints client.

    Access via `client.stock.ownership`.

    Every method takes the same range arguments. `from_date` / `to_date` are
    the canonical names; the official SDK's `from_=` and `to=` spellings are
    accepted as aliases, so existing fugle-marketdata code keeps working.
    Numeric fields in the institutional-trades, director-holdings and
    tdcc-distribution payloads may be `None`.
    """

    async def etf_holdings_async(
        self,
        symbol: str,
        *,
        from_date: Optional[str] = None,
        to_date: Optional[str] = None,
        sort: Optional[str] = None,
        from_: Optional[str] = None,
        to: Optional[str] = None,
    ) -> dict[str, Any]:
        """Get the constituents an ETF held over a date range.

        Args:
            symbol: ETF symbol (e.g. "0050")
            from_date: Start of the date range (YYYY-MM-DD); alias `from_`
            to_date: End of the date range (YYYY-MM-DD); alias `to`
            sort: "asc" (oldest first) or "desc" (newest first)

        Raises:
            TypeError: a date is passed under both its name and its alias

        Example:
            ```python
            data = await client.stock.ownership.etf_holdings_async(symbol="0050")
            for entry in data["data"]:
                print(entry["date"], len(entry["components"]))
            ```
        """
        ...

    def etf_holdings(
        self,
        symbol: str,
        *,
        from_date: Optional[str] = None,
        to_date: Optional[str] = None,
        sort: Optional[str] = None,
        from_: Optional[str] = None,
        to: Optional[str] = None,
    ) -> dict[str, Any]:
        """Blocking version of `etf_holdings_async()`."""
        ...

    async def institutional_trades_async(
        self,
        symbol: str,
        *,
        from_date: Optional[str] = None,
        to_date: Optional[str] = None,
        sort: Optional[str] = None,
        from_: Optional[str] = None,
        to: Optional[str] = None,
    ) -> dict[str, Any]:
        """Get daily trading by the three major institutional investors.

        Args:
            symbol: Stock symbol (e.g. "2330")
            from_date: Start of the date range (YYYY-MM-DD); alias `from_`
            to_date: End of the date range (YYYY-MM-DD); alias `to`
            sort: "asc" (oldest first) or "desc" (newest first)

        Raises:
            TypeError: a date is passed under both its name and its alias

        Example:
            ```python
            data = await client.stock.ownership.institutional_trades_async(symbol="2330")
            for entry in data["data"]:
                print(entry["date"], entry["foreign"]["net"], entry["total"])
            ```
        """
        ...

    def institutional_trades(
        self,
        symbol: str,
        *,
        from_date: Optional[str] = None,
        to_date: Optional[str] = None,
        sort: Optional[str] = None,
        from_: Optional[str] = None,
        to: Optional[str] = None,
    ) -> dict[str, Any]:
        """Blocking version of `institutional_trades_async()`."""
        ...

    async def director_holdings_async(
        self,
        symbol: str,
        *,
        from_date: Optional[str] = None,
        to_date: Optional[str] = None,
        sort: Optional[str] = None,
        from_: Optional[str] = None,
        to: Optional[str] = None,
    ) -> dict[str, Any]:
        """Get monthly holdings and pledges disclosed by directors and supervisors.

        Args:
            symbol: Stock symbol (e.g. "2330")
            from_date: Start of the date range (YYYY-MM-DD); alias `from_`
            to_date: End of the date range (YYYY-MM-DD); alias `to`
            sort: "asc" (oldest first) or "desc" (newest first)

        Raises:
            TypeError: a date is passed under both its name and its alias

        Example:
            ```python
            data = await client.stock.ownership.director_holdings_async(symbol="2330")
            for entry in data["data"]:  # entry["date"] is YYYY-MM
                for d in entry["directors"]:
                    print(d["title"], d["name"], d["heldShares"], d["pledgeRatio"])
            ```
        """
        ...

    def director_holdings(
        self,
        symbol: str,
        *,
        from_date: Optional[str] = None,
        to_date: Optional[str] = None,
        sort: Optional[str] = None,
        from_: Optional[str] = None,
        to: Optional[str] = None,
    ) -> dict[str, Any]:
        """Blocking version of `director_holdings_async()`."""
        ...

    async def tdcc_distribution_async(
        self,
        symbol: str,
        *,
        from_date: Optional[str] = None,
        to_date: Optional[str] = None,
        sort: Optional[str] = None,
        from_: Optional[str] = None,
        to: Optional[str] = None,
    ) -> dict[str, Any]:
        """Get the weekly TDCC shareholder distribution by holding-size bracket.

        Args:
            symbol: Stock symbol (e.g. "2330")
            from_date: Start of the date range (YYYY-MM-DD); alias `from_`
            to_date: End of the date range (YYYY-MM-DD); alias `to`
            sort: "asc" (oldest first) or "desc" (newest first)

        Raises:
            TypeError: a date is passed under both its name and its alias

        Example:
            ```python
            data = await client.stock.ownership.tdcc_distribution_async(symbol="2330")
            for entry in data["data"]:
                for level in entry["distributions"]:
                    print(level["range"], level["holders"], level["proportion"])
            ```
        """
        ...

    def tdcc_distribution(
        self,
        symbol: str,
        *,
        from_date: Optional[str] = None,
        to_date: Optional[str] = None,
        sort: Optional[str] = None,
        from_: Optional[str] = None,
        to: Optional[str] = None,
    ) -> dict[str, Any]:
        """Blocking version of `tdcc_distribution_async()`."""
        ...


class StockCorporateActionsClient:
    """Stock corporate actions endpoints client.

    Access via `client.stock.corporate_actions`. All methods are async and
    return coroutines that resolve to dict objects.
    """

    async def capital_changes_async(
        self,
        *,
        start_date: Optional[str] = None,
        end_date: Optional[str] = None,
        sort: Optional[str] = None,
    ) -> dict[str, Any]:
        """Get capital changes (stock splits, rights issues, etc.)

        Args:
            start_date: Start date for range query (YYYY-MM-DD)
            end_date: End date for range query (YYYY-MM-DD)
            sort: Sort order, "asc" or "desc"

        Returns:
            Capital changes data

        Raises:
            MarketDataError: If the request fails

        Example:
            ```python
            changes = await client.stock.corporate_actions.capital_changes_async(
                start_date="2024-01-01",
                end_date="2024-01-31"
            )
            ```
        """
        ...

    async def dividends_async(
        self,
        *,
        start_date: Optional[str] = None,
        end_date: Optional[str] = None,
        exchange: Optional[str] = None,
        sort: Optional[str] = None,
    ) -> dict[str, Any]:
        """Get dividend announcements.

        Args:
            start_date: Start date for range query (YYYY-MM-DD)
            end_date: End date for range query (YYYY-MM-DD)
            exchange: Exchange filter, "TWSE" or "TPEx"
            sort: Sort order, "asc" or "desc"

        Returns:
            Dividend data

        Raises:
            MarketDataError: If the request fails

        Example:
            ```python
            dividends = await client.stock.corporate_actions.dividends_async(
                start_date="2024-01-01",
                end_date="2024-12-31"
            )
            ```
        """
        ...

    async def listing_applicants_async(
        self,
        *,
        start_date: Optional[str] = None,
        end_date: Optional[str] = None,
        exchange: Optional[str] = None,
        sort: Optional[str] = None,
    ) -> dict[str, Any]:
        """Get IPO listing applicants.

        Args:
            start_date: Start date for range query (YYYY-MM-DD)
            end_date: End date for range query (YYYY-MM-DD)
            exchange: Exchange filter, "TWSE" or "TPEx"
            sort: Sort order, "asc" or "desc"

        Returns:
            Listing applicants data

        Raises:
            MarketDataError: If the request fails

        Example:
            ```python
            applicants = await client.stock.corporate_actions.listing_applicants_async()
            ```
        """
        ...

    # ----- Sync siblings (legacy fugle-marketdata compatibility) -----

    def capital_changes(
        self,
        *,
        start_date: Optional[str] = None,
        end_date: Optional[str] = None,
        sort: Optional[str] = None,
    ) -> dict[str, Any]:
        """Blocking version of `capital_changes()`."""
        ...

    def dividends(
        self,
        *,
        start_date: Optional[str] = None,
        end_date: Optional[str] = None,
        exchange: Optional[str] = None,
        sort: Optional[str] = None,
    ) -> dict[str, Any]:
        """Blocking version of `dividends()`."""
        ...

    def listing_applicants(
        self,
        *,
        start_date: Optional[str] = None,
        end_date: Optional[str] = None,
        exchange: Optional[str] = None,
        sort: Optional[str] = None,
    ) -> dict[str, Any]:
        """Blocking version of `listing_applicants()`."""
        ...


class FutOptClient:
    """Futures and options market data client.

    Access via `client.futopt`. Provides access to intraday and historical
    futures and options data.
    """

    @property
    def intraday(self) -> "FutOptIntradayClient":
        """Access intraday (real-time) FutOpt endpoints.

        Returns:
            FutOptIntradayClient for accessing intraday endpoints
        """
        ...

    @property
    def historical(self) -> "FutOptHistoricalClient":
        """Access historical FutOpt data endpoints.

        Returns:
            FutOptHistoricalClient for accessing historical endpoints
        """
        ...


class FutOptIntradayClient:
    """FutOpt intraday (real-time) endpoints client.

    Access via `client.futopt.intraday`. All methods are async and
    return coroutines that resolve to dict objects.
    """

    async def quote_async(self, symbol: str, *, after_hours: Optional[bool] = None) -> dict[str, Any]:
        """Get intraday quote for a futures/options contract.

        Args:
            symbol: Contract symbol (e.g., "TXFC4" for TAIEX futures)
            after_hours: Whether to query after-hours session data

        Returns:
            Quote data including prices, order book, and trading info

        Raises:
            MarketDataError: If the request fails

        Example:
            ```python
            # Regular session
            quote = await client.futopt.intraday.quote_async("TXFC4")

            # After-hours session
            ah_quote = await client.futopt.intraday.quote_async("TXFC4", after_hours=True)
            ```
        """
        ...

    async def ticker_async(self, symbol: str, *, after_hours: Optional[bool] = None) -> dict[str, Any]:
        """Get ticker information for a futures/options contract.

        Args:
            symbol: Contract symbol (e.g., "TXFC4")
            after_hours: Whether to query after-hours session data

        Returns:
            Contract information

        Raises:
            MarketDataError: If the request fails
        """
        ...

    async def candles_async(
        self,
        symbol: str,
        *,
        timeframe: Optional[str] = None,
        after_hours: Optional[bool] = None,
    ) -> dict[str, Any]:
        """Get candlestick chart data for a futures/options contract.

        Args:
            symbol: Contract symbol (e.g., "TXFC4")
            timeframe: Timeframe in minutes (default: "1")
            after_hours: Whether to query after-hours session data

        Returns:
            Candlestick data with OHLCV values

        Raises:
            MarketDataError: If the request fails
        """
        ...

    async def trades_async(
        self,
        symbol: str,
        *,
        after_hours: Optional[bool] = None,
        offset: Optional[int] = None,
        limit: Optional[int] = None,
        is_trial: Optional[bool] = None,
    ) -> dict[str, Any]:
        """Get trade ticks for a futures/options contract.

        Args:
            symbol: Contract symbol (e.g., "TXFC4")
            after_hours: Whether to query after-hours session data
            offset: Number of trades to skip
            limit: Maximum number of trades to return
            is_trial: Only trial-matching (試撮合) trades

        Returns:
            Trade ticks data with price, volume, and time

        Raises:
            MarketDataError: If the request fails
        """
        ...

    async def volumes_async(self, symbol: str, *, after_hours: Optional[bool] = None) -> dict[str, Any]:
        """Get volume by price level for a futures/options contract.

        Args:
            symbol: Contract symbol (e.g., "TXFC4")
            after_hours: Whether to query after-hours session data

        Returns:
            Volume data by price level

        Raises:
            MarketDataError: If the request fails
        """
        ...

    async def tickers_async(
        self,
        type: str,
        *,
        exchange: str | None = None,
        after_hours: Optional[bool] = None,
        contract_type: str | None = None,
        is_spread: bool | None = None,
        product: str | None = None,
    ) -> dict[str, Any]:
        """Get batch ticker list for a FutOpt contract type.

        Args:
            type: Contract type ("FUTURE" or "OPTION")
            exchange: Exchange filter (e.g., "TAIFEX")
            after_hours: Query after-hours session data
            contract_type: Contract type code ("I", "R", "B", "C", "S", "E")
            is_spread: Only spread (價差) contracts, or only non-spread ones
            product: Only contracts of this product (e.g., "TXF")

        Returns:
            Response envelope: the query fields (``date``, ``type``, ``exchange``, ``session``, ...) and ``data``, the list of FutOpt ticker info dicts

        Raises:
            MarketDataError: If the request fails
        """
        ...

    async def products_async(
        self,
        type: str,
        *,
        contract_type: str | None = None,
        exchange: str | None = None,
        after_hours: Optional[bool] = None,
        status: str | None = None,
    ) -> dict[str, Any]:
        """Get available FutOpt products list.

        Args:
            type: Contract type ("FUTURE" or "OPTION")
            contract_type: Contract type code ("I", "R", "B", "C", "S", "E")
            exchange: Exchange filter (e.g., "TAIFEX")
            after_hours: Query after-hours session data
            status: Product status, "N", "P" or "U"

        Returns:
            Response envelope: the query fields (``date``, ``type``, ``exchange``, ``session``, ...) and ``data``, the list of product info dicts

        Raises:
            MarketDataError: If the request fails
        """
        ...

    # ----- Sync siblings (legacy fugle-marketdata compatibility) -----

    def quote(self, symbol: str, *, after_hours: Optional[bool] = None) -> dict[str, Any]:
        """Blocking version of `quote()`."""
        ...

    def ticker(self, symbol: str, *, after_hours: Optional[bool] = None) -> dict[str, Any]:
        """Blocking version of `ticker()`."""
        ...

    def candles(
        self,
        symbol: str,
        *,
        timeframe: Optional[str] = None,
        after_hours: Optional[bool] = None,
    ) -> dict[str, Any]:
        """Blocking version of `candles()`."""
        ...

    def trades(
        self,
        symbol: str,
        *,
        after_hours: Optional[bool] = None,
        offset: Optional[int] = None,
        limit: Optional[int] = None,
        is_trial: Optional[bool] = None,
    ) -> dict[str, Any]:
        """Blocking version of `trades()`."""
        ...

    def volumes(self, symbol: str, *, after_hours: Optional[bool] = None) -> dict[str, Any]:
        """Blocking version of `volumes()`."""
        ...

    def tickers(
        self,
        type: str,
        *,
        exchange: str | None = None,
        after_hours: Optional[bool] = None,
        contract_type: str | None = None,
        is_spread: bool | None = None,
        product: str | None = None,
    ) -> dict[str, Any]:
        """Blocking version of `tickers()`."""
        ...

    def products(
        self,
        type: str,
        *,
        contract_type: str | None = None,
        exchange: str | None = None,
        after_hours: Optional[bool] = None,
        status: str | None = None,
    ) -> dict[str, Any]:
        """Blocking version of `products()`."""
        ...


class FutOptHistoricalClient:
    """FutOpt historical data endpoints client.

    Access via `client.futopt.historical`. Both endpoints take a **product**
    code (e.g. "TXF"); a contract code such as "TXFC4" returns 404.
    """

    async def candles_async(
        self,
        symbol: str,
        *,
        from_date: Optional[str] = None,
        to_date: Optional[str] = None,
        timeframe: Optional[str] = None,
        after_hours: Optional[bool] = None,
        contract_month: Optional[str] = None,
        fields: Optional[str] = None,
        sort: Optional[str] = None,
        strike_price: Optional[float] = None,
        call_put: Optional[str] = None,
    ) -> dict[str, Any]:
        """Get historical candles for a FutOpt product.

        Args:
            symbol: Product code (e.g., "TXF")
            from_date: Start date (YYYY-MM-DD)
            to_date: End date (YYYY-MM-DD)
            timeframe: Timeframe ("D", "W", "M", "1", "5", "10", "15", "30", "60")
            after_hours: Query the after-hours session
            contract_month: "YYYYMM", or a continuous contract: "1!" (server default), "2!", "3!"
            fields: Comma-separated fields, e.g. "open,high,low,close,volume"
            sort: "asc" or "desc"
            strike_price: Strike price (options only, with call_put)
            call_put: "CALL" or "PUT" (options only, with strike_price)

        Returns:
            Historical candles data

        Raises:
            MarketDataError: If the request fails

        Example:
            ```python
            candles = await client.futopt.historical.candles_async(
                "TXF",
                contract_month="202609",
                from_date="2026-09-01",
                to_date="2026-09-15",
                timeframe="D"
            )
            ```
        """
        ...

    async def daily_async(
        self,
        symbol: str,
        *,
        date: Optional[str] = None,
        after_hours: Optional[bool] = None,
    ) -> dict[str, Any]:
        """Get one trading day's daily quotes for every contract month of a FutOpt product.

        Args:
            symbol: Product code (e.g., "TXF")
            date: Trading date (YYYY-MM-DD); the server defaults to today
            after_hours: Query the after-hours session

        Returns:
            Daily quotes, one row per contract month

        Raises:
            TypeError: If `from_date` / `to_date` are passed
            MarketDataError: If the request fails

        Example:
            ```python
            daily = await client.futopt.historical.daily_async("TXF", date="2026-09-15")
            ```
        """
        ...

    # ----- Sync siblings (legacy fugle-marketdata compatibility) -----

    def candles(
        self,
        symbol: str,
        *,
        from_date: Optional[str] = None,
        to_date: Optional[str] = None,
        timeframe: Optional[str] = None,
        after_hours: Optional[bool] = None,
        contract_month: Optional[str] = None,
        fields: Optional[str] = None,
        sort: Optional[str] = None,
        strike_price: Optional[float] = None,
        call_put: Optional[str] = None,
    ) -> dict[str, Any]:
        """Blocking version of `candles_async()`."""
        ...

    def daily(
        self,
        symbol: str,
        *,
        date: Optional[str] = None,
        after_hours: Optional[bool] = None,
    ) -> dict[str, Any]:
        """Blocking version of `daily_async()`."""
        ...


# WebSocket Client
class HealthCheckConfig:
    """Liveness detection configuration.

    The connection is declared dead when no inbound frame (data, heartbeat or
    pong) arrives within `heartbeat_timeout_ms`; the reconnect manager then
    takes over. The server sends a heartbeat every 30 seconds. Enabled by
    default; a client created without a health check config uses the defaults.

    Example:
        ```python
        from fugle_marketdata import HealthCheckConfig, WebSocketClient

        # Longer timeout
        config = HealthCheckConfig(heartbeat_timeout_ms=60000)
        ws = WebSocketClient(api_key="key", health_check=config)

        # Opt out of liveness detection
        ws = WebSocketClient(api_key="key", health_check=HealthCheckConfig(enabled=False))

        # Confirm with a ping before disconnecting; know within 10 s
        config = HealthCheckConfig(probe_enabled=True,
                                   idle_probe_after_ms=5000,
                                   probe_timeout_ms=5000)
        ```

    With ``probe_enabled``, a silent connection is asked before it is declared
    dead: after ``idle_probe_after_ms`` of silence one ping is sent, and only
    if nothing arrives within ``probe_timeout_ms`` is the connection declared
    dead. The defaults keep detection at 35 s and send no ping while the
    server's heartbeat is on time. Probing does not detect a half-open
    connection (the server still sends, our writes no longer arrive).
    """

    enabled: bool
    """Whether liveness detection is active (default: True)."""

    heartbeat_timeout_ms: int
    """Maximum gap between inbound frames in milliseconds before the connection is declared dead.

    Does not apply when ``probe_enabled`` is True."""

    probe_enabled: bool
    """Confirm a silent connection with a ping before declaring it dead (default: False)."""

    idle_probe_after_ms: int
    """Silence in milliseconds before the probe (default: 30000, the server's heartbeat period)."""

    probe_timeout_ms: int
    """Wait in milliseconds for any inbound frame after the probe (default: 5000)."""

    def __init__(
        self,
        enabled: bool = True,
        ping_interval: Optional[int] = None,
        max_missed_pongs: Optional[int] = None,
        *,
        heartbeat_timeout_ms: int = 35000,
        probe_enabled: bool = False,
        idle_probe_after_ms: int = 30000,
        probe_timeout_ms: int = 5000,
    ) -> None:
        """Create a new health check configuration.

        Args:
            enabled: Whether liveness detection is active (default: True)
            heartbeat_timeout_ms: Maximum gap between inbound frames before the
                connection is declared dead (default: 35000ms = the server's 30s
                heartbeat + 5s buffer, min: 5000ms). Does not apply when
                ``probe_enabled`` is True.
            probe_enabled: Confirm a silent connection with a ping before
                declaring it dead (default: False). Detection is
                ``idle_probe_after_ms + probe_timeout_ms``.
            idle_probe_after_ms: Silence before the ping (default: 30000ms, the
                server's heartbeat period, min: 5000ms). Below 30000 a ping is
                sent in every gap between heartbeats while no data flows.
            probe_timeout_ms: Wait for any inbound frame after the ping
                (default: 5000ms, min: 1000ms)
            ping_interval, max_missed_pongs: The 2.x fields, positional as in
                2.x. 3.0 does not have them: a value other than None is ignored
                with a ``FugleHealthCheckWarning``.

        Raises:
            ConfigError: code 1004 if heartbeat_timeout_ms < 5000,
                idle_probe_after_ms < 5000 or probe_timeout_ms < 1000
        """
        ...


class ReconnectConfig:
    """Auto-reconnect configuration.

    Controls automatic reconnection behavior when WebSocket connection is lost.
    Uses exponential backoff with configurable parameters.

    A normal closure by the server (close code 1000) ends the connection for
    good and is not reconnected, and neither are rejected credentials; every
    other drop is.

    Example:
        ```python
        from fugle_marketdata import ReconnectConfig, WebSocketClient

        config = ReconnectConfig(
            enabled=True,
            max_attempts=10,
            initial_delay_ms=1000,
            max_delay_ms=60000
        )
        ws = WebSocketClient(api_key="key", reconnect=config)
        ```
    """

    enabled: bool
    """Whether auto-reconnect is enabled."""

    max_attempts: int
    """Maximum number of reconnection attempts; 0 means unlimited."""

    initial_delay_ms: int
    """Initial delay in milliseconds for exponential backoff."""

    max_delay_ms: int
    """Maximum delay in milliseconds (caps exponential backoff)."""

    def __init__(
        self,
        *,
        enabled: bool = True,
        max_attempts: int = 0,
        initial_delay_ms: int = 1000,
        max_delay_ms: int = 60000,
    ) -> None:
        """Create a new reconnect configuration.

        Args:
            enabled: Whether auto-reconnect is enabled (default: True)
            max_attempts: Maximum reconnection attempts; 0 means unlimited
                (default: 0, so the client keeps retrying at most max_delay_ms apart)
            initial_delay_ms: Initial delay for exponential backoff (default: 1000ms, min: 100ms)
            max_delay_ms: Maximum delay cap (default: 60000ms = 60s)

        Raises:
            ConfigError: code 1004 if initial_delay_ms < 100 or
                max_delay_ms < initial_delay_ms
        """
        ...

    @staticmethod
    def default_config() -> "ReconnectConfig":
        """Create a default reconnect configuration (enabled, unlimited attempts)."""
        ...

    @staticmethod
    def disabled() -> "ReconnectConfig":
        """Create a disabled reconnect configuration."""
        ...


class DisconnectInfo:
    """The last disconnect of a stream client: who closed the connection and
    whether a reconnect follows. Read it from ``ws.stock.last_disconnect`` /
    ``ws.futopt.last_disconnect``; not constructed by user code.

    Example::

        def on_disconnect(code, reason):
            info = ws.stock.last_disconnect
            if not info.will_reconnect:
                print("connection over:", info.intent, code, reason)

        ws.stock.on("disconnect", on_disconnect)

    Compares equal, and hashes alike, when all four attributes are equal.
    """

    @property
    def code(self) -> Optional[int]:
        """WebSocket close code, or None when the connection ended without one
        (transport error, EOF, heartbeat timeout, or a server Close frame
        without a code)."""
        ...

    @property
    def reason(self) -> str:
        """Close reason (may be empty)."""
        ...

    @property
    def intent(self) -> Literal["client", "server", "network"]:
        """Who closed the connection: "client" = your disconnect(); "server" =
        the server's Close frame (any code); "network" = transport error, EOF
        without a Close frame, or heartbeat timeout."""
        ...

    @property
    def will_reconnect(self) -> bool:
        """True if a "reconnect" event follows (unless disconnect() is called
        first); False if this connection is over."""
        ...


class WebSocketClient:
    """WebSocket client for Fugle market data streaming.

    Provides real-time streaming access to stock and futures/options
    market data through WebSocket connections.

    Example:
        ```python
        from fugle_marketdata import WebSocketClient, ReconnectConfig, HealthCheckConfig

        # Basic usage
        ws = WebSocketClient(api_key="your-key")

        # With custom reconnect config
        rc = ReconnectConfig(max_attempts=10, initial_delay_ms=2000)
        ws = WebSocketClient(api_key="key", reconnect=rc)

        # With a longer health check timeout
        hc = HealthCheckConfig(heartbeat_timeout_ms=60000)
        ws = WebSocketClient(api_key="key", health_check=hc)

        # Callback mode
        def on_message(msg):
            print(f"Received: {msg}")

        ws.stock.on("message", on_message)
        ws.stock.connect()
        ws.stock.subscribe("trades", "2330")

        # Or async iterator mode
        async with ws.stock as client:
            await client.subscribe_async("trades", "2330")
            async for msg in client.messages():
                print(msg)
        ```
    """

    def __init__(
        self,
        *,
        api_key: str | None = None,
        bearer_token: str | None = None,
        sdk_token: str | None = None,
        base_url: str | None = None,
        version: dict[str, str] | None = None,
        reconnect: ReconnectConfig | None = None,
        health_check: HealthCheckConfig | None = None,
        tls_ca_file: str | None = None,
        tls_root_cert_pem: bytes | None = None,
        tls_accept_invalid_certs: bool = False,
        message_overflow: Literal["drop_newest", "unbounded"] | None = None,
        message_buffer: int | None = None,
        auth_timeout_ms: int | None = None,
    ) -> None:
        """Create a new WebSocket client with authentication and configuration.

        Args:
            api_key: Your Fugle API key (exactly one non-empty auth method required)
            bearer_token: Bearer token for authentication (exactly one non-empty auth method required)
            sdk_token: SDK token for authentication (exactly one non-empty auth method required)
            base_url: Optional custom base URL — host and path prefix ONLY.
                The SDK appends the version segment; a base_url that already
                ends in one (e.g. ".../v1.0") raises TypeError. Changed in
                0.8.0, which reversed the 0.6-0.7 rule.
            version: Per-product streaming version, e.g. {"futopt": "v1.0"}.
                Omitted products get their latest: stock v1.0, futopt v1.1.
                futopt v1.1 delivers trial-matching (試撮) frames on trades /
                books — branch on the frame's isTrial before acting on a price.
                Asking for a version a product does not serve raises TypeError
                rather than silently falling back.
            reconnect: Optional reconnect configuration (default: enabled, unlimited
                attempts; pass ReconnectConfig.disabled() to turn it off)
            health_check: Optional health check configuration (default: enabled,
                35000ms timeout)
            tls_ca_file: Path to a PEM-encoded root CA to trust (in addition to
                the system trust store). Mutually exclusive with tls_root_cert_pem.
            tls_root_cert_pem: Raw PEM bytes of a root CA to trust. Mutually
                exclusive with tls_ca_file.
            tls_accept_invalid_certs: DANGER — disable ALL TLS verification
                (chain + hostname). Equivalent to ``wscat --no-check``. Prefer
                tls_ca_file for production. Emits UserWarning when set.
            message_overflow: What happens while message_buffer messages are
                unread. "drop_newest" (default) drops new ones and reports
                them through the "messages_dropped" callback; "unbounded"
                never drops, and memory grows while you fall behind.
            message_buffer: Unread messages held before message_overflow
                applies (default 4096; must be positive).
            auth_timeout_ms: How long the auth handshake may take once the
                WebSocket is open, in milliseconds: from the auth frame being
                sent until the server's verdict (default 10000). Applies to
                the first connect and to every reconnect; elapsing it fails
                the attempt with TimeoutError (code 3001). Must be greater
                than 0. The server itself allows 60 seconds.

        Raises:
            ConfigError: code 1004 if zero or multiple non-empty auth
                methods are given (empty or whitespace-only values count as
                not given), or if auth_timeout_ms is not greater than 0
            TypeError: both TLS cert options set, or a base_url carrying a
                version segment
            OSError: tls_ca_file path not readable
            ValueError: tls_root_cert_pem contents not a valid PEM certificate,
                an unknown message_overflow, or a message_buffer below 1

        Example:
            ```python
            # API key auth
            ws = WebSocketClient(api_key="your-key")

            # Self-signed deployment — pin the CA
            ws = WebSocketClient(api_key="key",
                                 base_url="wss://192.168.1.1/v1",
                                 tls_ca_file="/path/to/ca.pem")
            ```
        """
        ...

    @property
    def stock(self) -> "StockWebSocketClient":
        """Access stock market data WebSocket streaming.

        Returns:
            StockWebSocketClient for stock streaming
        """
        ...

    @property
    def futopt(self) -> "FutOptWebSocketClient":
        """Access futures and options WebSocket streaming.

        Returns:
            FutOptWebSocketClient for FutOpt streaming
        """
        ...


class StockWebSocketClient:
    """Stock market WebSocket client.

    Access via `ws.stock`. Supports both callback-based and async iterator-based
    message consumption. Can be used as an async context manager.

    Example (callback mode):
        ```python
        def on_message(msg):
            print(msg)

        ws.stock.on("message", on_message)
        ws.stock.connect()
        ws.stock.subscribe("trades", "2330")
        ```

    Example (async iterator mode):
        ```python
        async with ws.stock as client:
            await client.subscribe_async("trades", "2330")
            async for msg in client.messages():
                print(msg)
        ```
    """

    def connect(self) -> None:
        """Connect to WebSocket server (blocking).

        If message callbacks are registered before connect(), a background
        thread will automatically dispatch incoming messages to the callbacks.

        During an automatic reconnect it opens no connection of its own: it
        waits for that reconnect and returns once the connection is back and the
        subscriptions are re-sent, so a subscribe() afterwards follows them.
        Called from a callback, it holds up the callbacks until the reconnect
        ends.

        Raises:
            MarketDataError: If connection fails
            WebSocketError: Code 2011 if already connected or another connect is
                in progress. While waiting on a reconnect: code 2010 if
                disconnect() is called, code 3005 if the reconnect runs out of
                attempts
            AuthError: While waiting on a reconnect, if its credentials are
                rejected
        """
        ...

    async def connect_async(self) -> None:
        """Connect to WebSocket server (async).

        Returns an awaitable that completes when connection is established.
        Releases GIL during connection, enabling concurrent Python tasks.

        During an automatic reconnect it opens no connection of its own: it
        waits for that reconnect and returns once the connection is back and the
        subscriptions are re-sent, so a subscribe() afterwards follows them.
        Called from a callback, it holds up the callbacks until the reconnect
        ends.

        Raises:
            MarketDataError: If connection fails
            WebSocketError: Code 2011 if already connected or another connect is
                in progress. While waiting on a reconnect: code 2010 if
                disconnect() is called, code 3005 if the reconnect runs out of
                attempts
            AuthError: While waiting on a reconnect, if its credentials are
                rejected
        """
        ...

    def disconnect(self) -> None:
        """Disconnect from WebSocket server (blocking).

        Returns once the stream readers of the connection it closes, of a
        connect it aborts, and of earlier connections left for it have ended,
        so their "disconnect" callbacks have fired. A connection that another
        thread's connect() opens meanwhile is not waited for.

        Called from a callback of any client, this one or another, it waits
        for no stream reader: no "disconnect" callback is guaranteed to have
        fired when it returns, and those readers are left to the next
        disconnect() or disconnect_async() called outside a callback.

        In a callback, do not block on a disconnect that runs on another
        thread or event loop, such as
        asyncio.run_coroutine_threadsafe(...).result(): it still hangs, since
        that disconnect waits for the callback's stream reader.
        """
        ...

    async def disconnect_async(self) -> None:
        """Disconnect from WebSocket server (async).

        Returns an awaitable that completes when disconnection finishes.

        Waits for the same stream readers as disconnect(). Created in a
        callback of any client, this one or another — to wait for it with
        asyncio.run(), say — it waits for no stream reader, as disconnect()
        called there: no "disconnect" callback is guaranteed to have fired
        when it returns.

        In a callback, do not block on a disconnect_async() created on
        another thread or event loop, such as
        asyncio.run_coroutine_threadsafe(...).result(): it still hangs, since
        it waits for the callback's stream reader.
        """
        ...

    def is_connected(self) -> bool:
        """Check if currently connected.

        Returns:
            True if connected, False otherwise
        """
        ...

    @property
    def url(self) -> str:
        """The endpoint this client connects to.

        base_url (host and path prefix), the version segment picked by
        version, then the product path, e.g.
        wss://api.fugle.tw/marketdata/v1.0/stock/streaming. Readable before
        connect().
        """
        ...

    def messages_dropped_total(self) -> int:
        """Messages dropped because message_buffer were unread.

        Counted from the start of the current connection (every connect() or
        reconnect restarts it); after disconnect() it still reads the last
        connection's count. 0 before the first connect().
        """
        ...

    @property
    def last_disconnect(self) -> Optional[DisconnectInfo]:
        """The last disconnect: who closed the connection and whether a
        reconnect follows. None before the first one.

        Written before the "disconnect" callbacks run, so a callback reads the
        disconnect it is handling. Never cleared: connect(), a reconnect and
        disconnect() returning keep it, so it is a record of the last
        disconnect, not the connection state; ask is_connected() for that. A
        reconnect given up (error 3005) leaves it at the drop that started the
        reconnect. After a messages() iterator ends, this is where to find why.
        Should two connections overlap (a disconnect() from a callback, then
        connect() from another thread before that callback returns), it is the
        disconnect handed to the callbacks last.
        """
        ...

    def is_closed(self) -> bool:
        """Check if client has been closed.

        Returns True once disconnect() has closed the connection, or once the
        connection has ended without an automatic reconnect (for example a
        normal closure by the server, code 1000). A later connect() on the
        same client opens a new connection and this returns False again, so
        the client can be reused after disconnect(). A client that was never
        connected is not closed.

        Returns:
            True if closed, False otherwise
        """
        ...

    def subscribe(
        self,
        channel: Mapping[str, Any] | str,
        symbol: str | None = None,
        *,
        symbols: list[str] | None = None,
        odd_lot: bool | None = None,
    ) -> None:
        """Subscribe to a channel for one or more symbols (blocking).

        Two call shapes are supported (legacy fugle-marketdata parity):

        **Dict shape** (matches the legacy SDK README)::

            ws.stock.subscribe({"channel": "trades", "symbol": "2330"})
            ws.stock.subscribe({"channel": "trades", "symbols": ["2330", "2317"]})
            ws.stock.subscribe({"channel": "candles", "symbol": "2330", "oddLot": True})

        **Positional / kwargs shape**::

            ws.stock.subscribe("trades", "2330")
            ws.stock.subscribe("trades", symbols=["2330", "2317"])
            ws.stock.subscribe("candles", "2330", odd_lot=True)

        The dict is the whole call, as in the legacy SDK's
        ``def subscribe(self, params)``: passing ``symbol`` / ``symbols`` /
        ``odd_lot`` next to it raises ``TypeError``, as does a key the dict
        does not take (``afterHours`` is FutOpt only) or a non-boolean flag.
        The flag may be spelled ``oddLot``, ``odd_lot`` or ``intradayOddLot``
        (the server's and Node's name); give one.
        """
        ...

    async def subscribe_async(
        self,
        channel: Mapping[str, Any] | str,
        symbol: str | None = None,
        *,
        symbols: list[str] | None = None,
        odd_lot: bool | None = None,
    ) -> None:
        """Subscribe to a channel for one or more symbols (async).

        Accepts the same dual-shape input as :meth:`subscribe`. See its
        docstring for details.
        """
        ...

    async def measure_latency_async(self, timeout_ms: int | None = None) -> float:
        """Async version of :meth:`measure_latency`."""
        ...

    def unsubscribe(
        self,
        subscription_id: Mapping[str, Any] | str | None = None,
        *,
        ids: list[str] | None = None,
        channel: str | None = None,
        symbol: str | None = None,
        symbols: list[str] | None = None,
        odd_lot: bool | None = None,
    ) -> None:
        """Unsubscribe from a channel.

        By the server id from the ``subscribed`` message::

            ws.stock.unsubscribe({"id": "abc123"})
            ws.stock.unsubscribe({"ids": ["abc123", "def456"]})
            ws.stock.unsubscribe("abc123")
            ws.stock.unsubscribe(ids=["abc123", "def456"])

        Or by the arguments given to ``subscribe()``; ``channel`` is
        keyword-only::

            ws.stock.unsubscribe({"channel": "trades", "symbol": "2330"})
            ws.stock.unsubscribe({"channel": "candles", "symbols": ["2330"], "oddLot": True})
            ws.stock.unsubscribe(channel="trades", symbol="2330")
            ws.stock.unsubscribe(channel="candles", symbols=["2330"], odd_lot=True)

        Naming a channel together with an id raises ``MarketDataError`` 1005.
        """
        ...

    def local_subscriptions(self) -> List[str]:
        """Return the locally cached list of active subscription keys.

        This is the in-process cache maintained by core's SubscriptionManager.
        Use ``subscriptions()`` to query the server for the authoritative list.
        """
        ...

    def subscriptions(self) -> None:
        """Ask the server for its current subscription list.

        Sends ``{"event": "subscriptions"}`` to the server. The reply is delivered
        asynchronously via the ``message`` callback, matching the old
        ``fugle-marketdata`` SDK semantics.

        Raises:
            RuntimeError: If not connected
        """
        ...

    def ping(self, state: str | None = None) -> None:
        """Send a ``ping`` frame to the server.

        Fire and forget: the server's pong reply is delivered to the message
        handlers. To wait for it and get the round trip, use
        :meth:`measure_latency`.

        Args:
            state: Optional state string echoed back in the server's pong reply

        Raises:
            RuntimeError: If not connected
        """
        ...

    def measure_latency(self, timeout_ms: int | None = None) -> float:
        """Measure the round trip to the server, in milliseconds.

        Sends a ping and waits for its pong. Works whether or not
        ``probe_enabled`` is set and sends nothing in the background; its pong
        is not delivered to the message handlers. Blocks with the GIL released.

        Args:
            timeout_ms: How long to wait for the pong (default: 5000)

        Raises:
            WebSocketError: Code 2010 if not connected
            ConnectionError: Code 2001 if the connection closes before the pong
            TimeoutError: Code 3001 if no pong arrives within ``timeout_ms``
            MarketDataError: Code 1005 for a ``timeout_ms`` of 0
        """
        ...

    def on(self, event: str, callback: Callable[..., None]) -> None:
        """Register a callback for an event type.

        Supported events:
          - "message" / "data": Called with message dict when data received
          - "raw_message": Called with each message as the str the server sent,
            not turned into a dict. json.loads() of it equals the dict "message" gets. With
            only "raw_message" callbacks the SDK builds no dict. When both are
            registered each message goes to "raw_message" first. Like
            "message", it takes the messages from messages() iterators.
          - "connect" / "connected": Called (no args) when the WebSocket opens, before authentication
          - "authenticated": Called with the server's ``authenticated`` frame when it accepts
            credentials, a dict as in 2.x: ``{"event": "authenticated", "data": {...}}``
          - "unauthenticated": Called with the server's rejection frame when it refuses credentials,
            a dict as in 2.x: ``{"event": "error", "code": 1000, "data": {"message": ...}}``. During an auto-reconnect this is terminal: an "error" with
            code 3005 follows and the client stays closed. Any other auth-phase server error (1011
            auth service unavailable, 1004 no auth request received) is an "error" with code 2001
            instead, and the reconnect goes on.
          - "disconnect" / "disconnected" / "close": Called with (code, reason) when connection closed;
            who closed it and whether a reconnect follows are in ``last_disconnect``
          - "reconnect" / "reconnecting": Called with the attempt number when reconnecting
          - "error": Called with a WebSocketError instance when an error occurs
          - "messages_dropped": Called with (dropped, total) when messages were
            dropped because you fell behind: dropped since the previous call,
            and on the connection so far. At most once per second, and once
            more before "disconnect".

        A callback ``==`` to one already registered for the event is not
        added again, as in 2.x: it runs once per event.

        Args:
            event: Event type string
            callback: Python callable to invoke
        """
        ...

    def off(self, event: str, listener: Optional[Callable[..., Any]] = None) -> None:
        """Remove callbacks for an event type.

        Args:
            event: Event type string
            listener: The callback to remove (compared with ``==``); one not
                registered is ignored. Omitted or None removes every callback
                for ``event``. 2.x documented ``off(event, listener)`` but it
                raised ``AttributeError``.
        """
        ...

    @overload
    def messages(self, timeout_ms: Optional[int] = None, *, raw: Literal[False] = False) -> "MessageIterator[Message]":
        """Get message iterator for consuming streaming data.

        Args:
            timeout_ms: Deprecated and ignored; passing it emits a
                DeprecationWarning.
            raw: Yield each message as the str the server sent instead of a
                dict; the SDK builds no dict from it. Keyword-only. Each
                iterator chooses for itself.

        Returns:
            MessageIterator for iterating over messages. Iteration yields
            messages only and stops once the connection is gone.

        Raises:
            RuntimeError: If not connected; call connect() first
        """
        ...
    @overload
    def messages(self, timeout_ms: Optional[int] = None, *, raw: Literal[True]) -> "MessageIterator[str]": ...
    @overload
    def messages(self, timeout_ms: Optional[int] = None, *, raw: bool) -> "MessageIterator[Any]": ...

    async def __aenter__(self) -> "StockWebSocketClient":
        """Async context manager entry - connects to WebSocket server and
        returns this client."""
        ...

    async def __aexit__(
        self,
        exc_type: Any,
        exc_val: Any,
        exc_tb: Any,
    ) -> None:
        """Async context manager exit - disconnects from WebSocket server."""
        ...


class FutOptWebSocketClient:
    """FutOpt (futures and options) WebSocket client.

    Access via `ws.futopt`. Similar to StockWebSocketClient but for
    futures and options market data. Can be used as an async context manager.

    Example (async iterator mode):
        ```python
        async with ws.futopt as client:
            await client.subscribe_async("trades", "TXFC4")
            async for msg in client.messages():
                print(msg)
        ```
    """

    def connect(self) -> None:
        """Connect to WebSocket server (blocking).

        During an automatic reconnect it opens no connection of its own: it
        waits for that reconnect and returns once the connection is back and the
        subscriptions are re-sent, so a subscribe() afterwards follows them.
        Called from a callback, it holds up the callbacks until the reconnect
        ends.

        Raises:
            MarketDataError: If connection fails
            WebSocketError: Code 2011 if already connected or another connect is
                in progress. While waiting on a reconnect: code 2010 if
                disconnect() is called, code 3005 if the reconnect runs out of
                attempts
            AuthError: While waiting on a reconnect, if its credentials are
                rejected
        """
        ...

    async def connect_async(self) -> None:
        """Connect to WebSocket server (async).

        Returns an awaitable that completes when connection is established.
        Releases GIL during connection, enabling concurrent Python tasks.

        During an automatic reconnect it opens no connection of its own: it
        waits for that reconnect and returns once the connection is back and the
        subscriptions are re-sent, so a subscribe() afterwards follows them.
        Called from a callback, it holds up the callbacks until the reconnect
        ends.

        Raises:
            MarketDataError: If connection fails
            WebSocketError: Code 2011 if already connected or another connect is
                in progress. While waiting on a reconnect: code 2010 if
                disconnect() is called, code 3005 if the reconnect runs out of
                attempts
            AuthError: While waiting on a reconnect, if its credentials are
                rejected
        """
        ...

    def disconnect(self) -> None:
        """Disconnect from WebSocket server (blocking).

        Returns once the stream readers of the connection it closes, of a
        connect it aborts, and of earlier connections left for it have ended,
        so their "disconnect" callbacks have fired. A connection that another
        thread's connect() opens meanwhile is not waited for.

        Called from a callback of any client, this one or another, it waits
        for no stream reader: no "disconnect" callback is guaranteed to have
        fired when it returns, and those readers are left to the next
        disconnect() or disconnect_async() called outside a callback.

        In a callback, do not block on a disconnect that runs on another
        thread or event loop, such as
        asyncio.run_coroutine_threadsafe(...).result(): it still hangs, since
        that disconnect waits for the callback's stream reader.
        """
        ...

    async def disconnect_async(self) -> None:
        """Disconnect from WebSocket server (async).

        Returns an awaitable that completes when disconnection finishes.

        Waits for the same stream readers as disconnect(). Created in a
        callback of any client, this one or another — to wait for it with
        asyncio.run(), say — it waits for no stream reader, as disconnect()
        called there: no "disconnect" callback is guaranteed to have fired
        when it returns.

        In a callback, do not block on a disconnect_async() created on
        another thread or event loop, such as
        asyncio.run_coroutine_threadsafe(...).result(): it still hangs, since
        it waits for the callback's stream reader.
        """
        ...

    def is_connected(self) -> bool:
        """Check if currently connected.

        Returns:
            True if connected, False otherwise
        """
        ...

    @property
    def url(self) -> str:
        """The endpoint this client connects to.

        base_url (host and path prefix), the version segment picked by
        version, then the product path, e.g.
        wss://api.fugle.tw/marketdata/v1.1/futopt/streaming. Readable before
        connect().
        """
        ...

    def messages_dropped_total(self) -> int:
        """Messages dropped because message_buffer were unread.

        Counted from the start of the current connection (every connect() or
        reconnect restarts it); after disconnect() it still reads the last
        connection's count. 0 before the first connect().
        """
        ...

    @property
    def last_disconnect(self) -> Optional[DisconnectInfo]:
        """The last disconnect: who closed the connection and whether a
        reconnect follows. None before the first one.

        Written before the "disconnect" callbacks run, so a callback reads the
        disconnect it is handling. Never cleared: connect(), a reconnect and
        disconnect() returning keep it, so it is a record of the last
        disconnect, not the connection state; ask is_connected() for that. A
        reconnect given up (error 3005) leaves it at the drop that started the
        reconnect. After a messages() iterator ends, this is where to find why.
        Should two connections overlap (a disconnect() from a callback, then
        connect() from another thread before that callback returns), it is the
        disconnect handed to the callbacks last.
        """
        ...

    def is_closed(self) -> bool:
        """Check if client has been closed.

        Returns True once disconnect() has closed the connection, or once the
        connection has ended without an automatic reconnect (for example a
        normal closure by the server, code 1000). A later connect() on the
        same client opens a new connection and this returns False again, so
        the client can be reused after disconnect(). A client that was never
        connected is not closed.

        Returns:
            True if closed, False otherwise
        """
        ...

    def subscribe(
        self,
        channel: Mapping[str, Any] | str,
        symbol: str | None = None,
        *,
        symbols: list[str] | None = None,
        after_hours: bool | None = None,
    ) -> None:
        """Subscribe to a channel for one or more FutOpt symbols (blocking).

        Two call shapes are supported (legacy fugle-marketdata parity)::

            ws.futopt.subscribe({"channel": "trades", "symbol": "TXFC4"})
            ws.futopt.subscribe({"channel": "books", "symbol": "MXFB4", "afterHours": True})
            ws.futopt.subscribe("trades", "TXFC4")
            ws.futopt.subscribe("books", "MXFB4", after_hours=True)

        Both ``afterHours`` (camelCase) and ``after_hours`` keys are accepted
        in dict form.
        """
        ...

    async def subscribe_async(
        self,
        channel: Mapping[str, Any] | str,
        symbol: str | None = None,
        *,
        symbols: list[str] | None = None,
        after_hours: bool | None = None,
    ) -> None:
        """Subscribe to a channel for one or more FutOpt symbols (async).

        Accepts the same dual-shape input as :meth:`subscribe`. See its
        docstring for details.
        """
        ...

    def unsubscribe(
        self,
        subscription_id: Mapping[str, Any] | str | None = None,
        *,
        ids: list[str] | None = None,
        channel: str | None = None,
        symbol: str | None = None,
        symbols: list[str] | None = None,
        after_hours: bool | None = None,
    ) -> None:
        """Unsubscribe from a channel.

        Accepts the same shapes as the stock client, with ``afterHours`` /
        ``after_hours`` as the modifier::

            ws.futopt.unsubscribe({"id": "abc123"})
            ws.futopt.unsubscribe({"ids": ["abc123", "def456"]})
            ws.futopt.unsubscribe("abc123")
            ws.futopt.unsubscribe(ids=["abc123", "def456"])
            ws.futopt.unsubscribe({"channel": "books", "symbol": "TXFC4", "afterHours": True})
            ws.futopt.unsubscribe(channel="books", symbol="TXFC4", after_hours=True)

        Naming a channel together with an id raises ``MarketDataError`` 1005.
        """
        ...

    def local_subscriptions(self) -> List[str]:
        """Return the locally cached list of active subscription keys.

        This is the in-process cache maintained by core's SubscriptionManager.
        Use ``subscriptions()`` to query the server for the authoritative list.
        """
        ...

    def subscriptions(self) -> None:
        """Ask the server for its current subscription list.

        Sends ``{"event": "subscriptions"}`` to the server. The reply is delivered
        asynchronously via the ``message`` callback, matching the old
        ``fugle-marketdata`` SDK semantics.

        Raises:
            RuntimeError: If not connected
        """
        ...

    def ping(self, state: str | None = None) -> None:
        """Send a ``ping`` frame to the server.

        Fire and forget: the server's pong reply is delivered to the message
        handlers. To wait for it and get the round trip, use
        :meth:`measure_latency`.

        Args:
            state: Optional state string echoed back in the server's pong reply

        Raises:
            RuntimeError: If not connected
        """
        ...

    def measure_latency(self, timeout_ms: int | None = None) -> float:
        """Measure the round trip to the server, in milliseconds.

        Sends a ping and waits for its pong. Works whether or not
        ``probe_enabled`` is set and sends nothing in the background; its pong
        is not delivered to the message handlers. Blocks with the GIL released.

        Args:
            timeout_ms: How long to wait for the pong (default: 5000)

        Raises:
            WebSocketError: Code 2010 if not connected
            ConnectionError: Code 2001 if the connection closes before the pong
            TimeoutError: Code 3001 if no pong arrives within ``timeout_ms``
            MarketDataError: Code 1005 for a ``timeout_ms`` of 0
        """
        ...

    async def measure_latency_async(self, timeout_ms: int | None = None) -> float:
        """Async version of :meth:`measure_latency`."""
        ...

    def on(self, event: str, callback: Callable[..., None]) -> None:
        """Register a callback for an event type.

        Same events as StockWebSocketClient.on(), "raw_message" included.

        A callback ``==`` to one already registered for the event is not
        added again, as in 2.x: it runs once per event.

        Args:
            event: Event type string
            callback: Python callable to invoke
        """
        ...

    def off(self, event: str, listener: Optional[Callable[..., Any]] = None) -> None:
        """Remove callbacks for an event type.

        Args:
            event: Event type string
            listener: The callback to remove (compared with ``==``); one not
                registered is ignored. Omitted or None removes every callback
                for ``event``. 2.x documented ``off(event, listener)`` but it
                raised ``AttributeError``.
        """
        ...

    @overload
    def messages(self, timeout_ms: Optional[int] = None, *, raw: Literal[False] = False) -> "MessageIterator[Message]":
        """Get message iterator for consuming streaming data.

        Args:
            timeout_ms: Deprecated and ignored; passing it emits a
                DeprecationWarning.
            raw: Yield each message as the str the server sent instead of a
                dict; the SDK builds no dict from it. Keyword-only. Each
                iterator chooses for itself.

        Returns:
            MessageIterator for iterating over messages. Iteration yields
            messages only and stops once the connection is gone.

        Raises:
            RuntimeError: If not connected; call connect() first
        """
        ...
    @overload
    def messages(self, timeout_ms: Optional[int] = None, *, raw: Literal[True]) -> "MessageIterator[str]": ...
    @overload
    def messages(self, timeout_ms: Optional[int] = None, *, raw: bool) -> "MessageIterator[Any]": ...

    async def __aenter__(self) -> "FutOptWebSocketClient":
        """Async context manager entry - connects to WebSocket server and
        returns this client."""
        ...

    async def __aexit__(
        self,
        exc_type: Any,
        exc_val: Any,
        exc_tb: Any,
    ) -> None:
        """Async context manager exit - disconnects from WebSocket server."""
        ...


class MessageIterator(Generic[_Yielded]):
    """Iterator for WebSocket messages.

    Yields message dicts, or from `messages(raw=True)` each message as the
    str the server sent.

    Supports both synchronous iteration (`for msg in iter`) and
    asynchronous iteration (`async for msg in iter`).

    Example (sync):
        ```python
        for msg in ws.stock.messages():
            print(msg)
        ```

    Example (async):
        ```python
        async for msg in ws.stock.messages():
            print(msg)
        ```

    Iteration yields messages only: it waits while none arrive and stops once
    the connection is gone.
    """

    def __iter__(self) -> Self:
        """Return self for iteration."""
        ...

    def __next__(self) -> _Yielded:
        """Get next message, waiting until one arrives (blocking).

        Never returns None. Wakes every 100 ms to let Python handle signals,
        so Ctrl+C interrupts the wait.

        Returns:
            Message dict (str from a `messages(raw=True)` iterator)

        Raises:
            StopIteration: When the connection is gone and every message was read
        """
        ...

    def __aiter__(self) -> Self:
        """Return self for async iteration."""
        ...

    def __anext__(self) -> Awaitable[_Yielded]:
        """Get next message, waiting until one arrives (async).

        Returns an awaitable, not a coroutine: `await` it, or wrap it with
        `asyncio.ensure_future`; `asyncio.create_task` rejects it. The
        awaited value is never None.

        While messages are queued the awaitable is returned already done,
        holding the next message, so `async for` reads a backlog without
        waiting on the event loop. One delivery in every 32 in a row is left
        to the loop instead (that awaitable is pending), so other tasks get
        to run.

        `async for` and a plain `await` of the awaitable lose no message.
        Code that holds the awaitable itself must not discard one that is
        done:

        - `asyncio.ensure_future(it.__anext__()).cancel()` returns False for
          a done awaitable, and the message is its `result()`.
        - After `asyncio.wait({step, stop})`, look at `step.done()` before
          anything else; cancelling "the rest" because `stop` finished drops
          the message a done `step` holds.
        - `asyncio.gather(it.__anext__(), other)` that is cancelled drops
          the message its done step holds. Read the step on its own.

        Before cancelling, check `done()`, or read `result()`.

        `asyncio.wait_for(it.__anext__(), timeout)`:

        - loses no message when `timeout` is longer than one turn of the
          event loop, in practice about 1 ms or more. `timeout=0` returns
          the message when the awaitable is done, and times out otherwise.
        - with a shorter `timeout`, can lose a queued message whose delivery
          was left to the loop (one in 32): the awaitable is resolved one
          turn before the waiting task resumes, and a timeout that expires
          within that turn cancels the task with the message already in the
          awaitable. Seen on Python 3.12 with `timeout=1e-6`.
        - up to Python 3.11, spends a turn of the loop even on a done
          awaitable. A task cancelled in that turn gets the message back
          instead of `CancelledError` (3.8.6 and later; run on 3.8.10 only),
          so `task.cancel()` on a `while True: await wait_for(...)` loop may
          only take effect at a step with nothing queued. Python 3.8.0 to
          3.8.5 raise `CancelledError` there and drop that message.

        Cancelling a pending awaitable (`asyncio.wait_for` timing out, a
        cancelled task) takes no message: the next read gets it. An awaitable
        still pending when its event loop closes is dropped silently.

        A single `async for` gets the messages in order. Awaitables of one
        iterator held at the same time may not get them in the order they
        were created.

        Returns:
            Message dict (str from a `messages(raw=True)` iterator)

        Raises:
            StopAsyncIteration: When the connection is gone and every message was read
        """
        ...

    def try_recv(self) -> Optional[_Yielded]:
        """Try to receive a message without blocking.

        Returns:
            Message dict if available (str from a `messages(raw=True)`
            iterator), None otherwise
        """
        ...

    def recv_timeout(self, timeout_ms: int) -> Optional[_Yielded]:
        """Receive a message, waiting up to `timeout_ms` (blocking).

        A plain method, not a coroutine: do not `await` it. Blocks the
        calling thread with the GIL released. Like `__next__`, wakes every
        100 ms to let Python handle signals, so Ctrl+C interrupts the wait;
        an interrupted call has taken no message.

        Args:
            timeout_ms: How long to wait, in milliseconds. A non-negative
                int below 2**64; a float raises TypeError, a negative int
                or one from 2**64 up OverflowError. A value too large for
                the clock to add waits until a message arrives.

        Returns:
            Message dict if received within timeout (str from a
            `messages(raw=True)` iterator), None on timeout

        Raises:
            ConnectionError: Code 2001 if the connection is gone and every
                message was read. This is the package's ConnectionError, a
                MarketDataError subclass, not the builtin.
        """
        ...
