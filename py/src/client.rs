//! Python REST client wrapper
//!
//! Provides Python-friendly interface to marketdata-core RestClient.
//! Mirrors the official SDK's API: `client.stock.intraday.quote()` pattern.

use pyo3::prelude::*;
use pyo3_async_runtimes::tokio::future_into_py;

use crate::errors;
use crate::types;

/// Python REST client for Fugle market data API
///
/// # Example (Python)
///
/// ```python
/// from fugle_marketdata import RestClient
///
/// # Create client with API key
/// client = RestClient(api_key="your-api-key")
///
/// # Get stock quote
/// quote = client.stock.intraday.quote("2330")
/// print(quote["lastPrice"])
///
/// # Get futures quote
/// futopt_quote = client.futopt.intraday.quote("TXFC4")
/// ```
#[pyclass]
pub struct RestClient {
    inner: marketdata_core::RestClient,
}

#[pymethods]
impl RestClient {
    /// Create a new REST client with authentication
    ///
    /// Provide exactly one authentication method (empty or whitespace-only
    /// values count as not provided):
    ///   - api_key: Your Fugle API key
    ///   - bearer_token: Bearer token for authentication
    ///   - sdk_token: SDK token for authentication
    ///
    /// Optional:
    ///   - base_url: Custom base URL for API endpoint
    ///
    /// Returns:
    ///     A new RestClient instance
    ///
    /// Raises:
    ///     MarketDataError: code 1004 if zero or multiple auth methods provided
    ///
    /// Example:
    ///     ```python
    ///     # API key auth
    ///     client = RestClient(api_key="your-api-key")
    ///
    ///     # Bearer token auth
    ///     client = RestClient(bearer_token="your-token")
    ///
    ///     # With custom base URL
    ///     client = RestClient(api_key="key", base_url="https://custom.api")
    ///     ```
    #[new]
    #[pyo3(signature = (*, api_key=None, bearer_token=None, sdk_token=None, base_url=None, tls_ca_file=None, tls_root_cert_pem=None, tls_accept_invalid_certs=false))]
    pub fn new(
        py: Python<'_>,
        api_key: Option<String>,
        bearer_token: Option<String>,
        sdk_token: Option<String>,
        base_url: Option<String>,
        tls_ca_file: Option<String>,
        tls_root_cert_pem: Option<Vec<u8>>,
        tls_accept_invalid_certs: bool,
    ) -> PyResult<Self> {
        // Core requires exactly one non-blank credential (ConfigError, 1004).
        let auth = marketdata_core::Auth::from_credentials(api_key, bearer_token, sdk_token)
            .map_err(errors::to_py_err)?;

        // Parse TLS kwargs; emits UserWarning if verification is disabled.
        let tls = crate::tls_kwargs::parse_tls_kwargs(
            py,
            tls_ca_file,
            tls_root_cert_pem,
            tls_accept_invalid_certs,
        )?;

        // Create client. with_tls returns Err only on malformed PEM —
        // surface that as a Python ValueError.
        let mut inner = marketdata_core::RestClient::with_tls(auth, tls).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("{e}"))
        })?;
        if let Some(url) = base_url {
            // `try_base_url` rather than `base_url`: a Python caller expects a
            // bad kwarg to raise at construction, matching the official SDK's
            // TypeError, not to surface later from an unrelated request.
            inner = inner.try_base_url(&url).map_err(|e| {
                pyo3::exceptions::PyTypeError::new_err(format!("{e}"))
            })?;
        }

        Ok(Self { inner })
    }

    /// The prefix every request from this client is built on, fully resolved —
    /// host, path prefix and version segment. Endpoints are appended to it.
    ///
    /// The version segment is chosen by the SDK rather than written by the
    /// caller, so this is the only way to see what a client resolved to.
    #[getter]
    pub fn base_url(&self) -> &str {
        self.inner.resolved_base_url()
    }

    /// Create a REST client with bearer token authentication
    ///
    /// Args:
    ///     token: Bearer token for authentication
    ///
    /// Returns:
    ///     A new RestClient instance
    ///
    /// Raises:
    ///     MarketDataError: code 1004 if the credential is empty or whitespace
    #[staticmethod]
    pub fn with_bearer_token(token: String) -> PyResult<Self> {
        let auth = marketdata_core::Auth::BearerToken(token);
        auth.validate().map_err(errors::to_py_err)?;
        Ok(Self { inner: marketdata_core::RestClient::new(auth) })
    }

    /// Create a REST client with SDK token authentication
    ///
    /// Args:
    ///     sdk_token: SDK token for authentication
    ///
    /// Returns:
    ///     A new RestClient instance
    ///
    /// Raises:
    ///     MarketDataError: code 1004 if the credential is empty or whitespace
    #[staticmethod]
    pub fn with_sdk_token(sdk_token: String) -> PyResult<Self> {
        let auth = marketdata_core::Auth::SdkToken(sdk_token);
        auth.validate().map_err(errors::to_py_err)?;
        Ok(Self { inner: marketdata_core::RestClient::new(auth) })
    }

    /// Access stock market data endpoints
    ///
    /// Returns:
    ///     StockClient for accessing stock endpoints
    #[getter]
    pub fn stock(&self) -> StockClient {
        StockClient {
            inner: self.inner.clone(),
        }
    }

    /// Access futures and options market data endpoints
    ///
    /// Returns:
    ///     FutOptClient for accessing FutOpt endpoints
    #[getter]
    pub fn futopt(&self) -> FutOptClient {
        FutOptClient {
            inner: self.inner.clone(),
        }
    }
}

/// Stock market data client
///
/// Access via `client.stock`
#[pyclass]
pub struct StockClient {
    inner: marketdata_core::RestClient,
}

#[pymethods]
impl StockClient {
    /// Access intraday (real-time) stock endpoints
    ///
    /// Returns:
    ///     StockIntradayClient for accessing intraday endpoints
    #[getter]
    pub fn intraday(&self) -> StockIntradayClient {
        StockIntradayClient {
            inner: self.inner.clone(),
        }
    }

    /// Access historical stock data endpoints
    ///
    /// Returns:
    ///     StockHistoricalClient for accessing historical endpoints
    #[getter]
    pub fn historical(&self) -> StockHistoricalClient {
        StockHistoricalClient {
            inner: self.inner.clone(),
        }
    }

    /// Access snapshot endpoints for market-wide data
    ///
    /// Returns:
    ///     StockSnapshotClient for accessing snapshot endpoints
    #[getter]
    pub fn snapshot(&self) -> StockSnapshotClient {
        StockSnapshotClient {
            inner: self.inner.clone(),
        }
    }

    /// Access technical indicator endpoints
    ///
    /// Returns:
    ///     StockTechnicalClient for accessing technical endpoints
    #[getter]
    pub fn technical(&self) -> StockTechnicalClient {
        StockTechnicalClient {
            inner: self.inner.clone(),
        }
    }

    /// Access corporate actions endpoints
    ///
    /// Returns:
    ///     StockCorporateActionsClient for accessing corporate actions endpoints
    #[getter]
    pub fn corporate_actions(&self) -> StockCorporateActionsClient {
        StockCorporateActionsClient {
            inner: self.inner.clone(),
        }
    }

    /// Access ownership endpoints (ETF holdings, institutional trades,
    /// director holdings, TDCC distribution)
    ///
    /// Returns:
    ///     StockOwnershipClient for accessing ownership endpoints
    #[getter]
    pub fn ownership(&self) -> StockOwnershipClient {
        StockOwnershipClient {
            inner: self.inner.clone(),
        }
    }

    /// The prefix every request from this product client is built on, fully
    /// resolved — host, path prefix and version segment.
    #[getter]
    pub fn base_url(&self) -> &str {
        self.inner.resolved_base_url()
    }
}

/// Stock ownership endpoints client
///
/// Access via `client.stock.ownership`
#[pyclass]
pub struct StockOwnershipClient {
    inner: marketdata_core::RestClient,
}

#[pymethods]
impl StockOwnershipClient {
    /// Get the constituents an ETF held over a date range
    ///
    /// Args:
    ///     symbol: ETF symbol (e.g. "0050") — required
    ///     from_date: Start of the date range (YYYY-MM-DD). The official SDK's
    ///         `from_=` / `**{"from": ...}` spellings are accepted too.
    ///     to_date: End of the date range (YYYY-MM-DD). `to=` is accepted too.
    ///     sort: "asc" (oldest first) or "desc" (newest first)
    ///
    /// Returns:
    ///     Awaitable[dict]: ETF holdings data
    ///
    /// Example:
    ///     ```python
    ///     data = await client.stock.ownership.etf_holdings_async(symbol="0050")
    ///     ```
    #[pyo3(signature = (symbol, *, from_date=None, to_date=None, sort=None, **_extra))]
    pub fn etf_holdings_async<'py>(
        &self,
        py: Python<'py>,
        symbol: String,
        from_date: Option<String>,
        to_date: Option<String>,
        sort: Option<String>,
        _extra: Option<Bound<'_, pyo3::types::PyDict>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let query = OwnershipQuery::resolve("stock.ownership.etf_holdings", symbol, from_date, to_date, sort, &_extra)?;
        ownership_async(py, self.inner.clone(), query, send_etf_holdings)
    }

    /// Sync sibling of `etf_holdings_async()`, matching the legacy fugle-marketdata call shape.
    #[pyo3(signature = (symbol, *, from_date=None, to_date=None, sort=None, **_extra))]
    pub fn etf_holdings(
        &self,
        py: Python<'_>,
        symbol: String,
        from_date: Option<String>,
        to_date: Option<String>,
        sort: Option<String>,
        _extra: Option<Bound<'_, pyo3::types::PyDict>>,
    ) -> PyResult<Py<pyo3::types::PyDict>> {
        let query = OwnershipQuery::resolve("stock.ownership.etf_holdings", symbol, from_date, to_date, sort, &_extra)?;
        ownership_sync(py, &self.inner, query, send_etf_holdings)
    }

    /// Get daily trading by the three major institutional investors (foreign, investment trust, dealer)
    ///
    /// Args:
    ///     symbol: Stock symbol (e.g. "2330") — required
    ///     from_date: Start of the date range (YYYY-MM-DD). The official SDK's
    ///         `from_=` / `**{"from": ...}` spellings are accepted too.
    ///     to_date: End of the date range (YYYY-MM-DD). `to=` is accepted too.
    ///     sort: "asc" (oldest first) or "desc" (newest first)
    ///
    /// Returns:
    ///     Awaitable[dict]: institutional trades data
    ///
    /// Example:
    ///     ```python
    ///     data = await client.stock.ownership.institutional_trades_async(symbol="2330")
    ///     ```
    #[pyo3(signature = (symbol, *, from_date=None, to_date=None, sort=None, **_extra))]
    pub fn institutional_trades_async<'py>(
        &self,
        py: Python<'py>,
        symbol: String,
        from_date: Option<String>,
        to_date: Option<String>,
        sort: Option<String>,
        _extra: Option<Bound<'_, pyo3::types::PyDict>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let query = OwnershipQuery::resolve("stock.ownership.institutional_trades", symbol, from_date, to_date, sort, &_extra)?;
        ownership_async(py, self.inner.clone(), query, send_institutional_trades)
    }

    /// Sync sibling of `institutional_trades_async()`, matching the legacy fugle-marketdata call shape.
    #[pyo3(signature = (symbol, *, from_date=None, to_date=None, sort=None, **_extra))]
    pub fn institutional_trades(
        &self,
        py: Python<'_>,
        symbol: String,
        from_date: Option<String>,
        to_date: Option<String>,
        sort: Option<String>,
        _extra: Option<Bound<'_, pyo3::types::PyDict>>,
    ) -> PyResult<Py<pyo3::types::PyDict>> {
        let query = OwnershipQuery::resolve("stock.ownership.institutional_trades", symbol, from_date, to_date, sort, &_extra)?;
        ownership_sync(py, &self.inner, query, send_institutional_trades)
    }

    /// Get monthly holdings and pledges disclosed by directors and supervisors
    ///
    /// Args:
    ///     symbol: Stock symbol (e.g. "2330") — required
    ///     from_date: Start of the date range (YYYY-MM-DD). The official SDK's
    ///         `from_=` / `**{"from": ...}` spellings are accepted too.
    ///     to_date: End of the date range (YYYY-MM-DD). `to=` is accepted too.
    ///     sort: "asc" (oldest first) or "desc" (newest first)
    ///
    /// Returns:
    ///     Awaitable[dict]: director holdings data
    ///
    /// Example:
    ///     ```python
    ///     data = await client.stock.ownership.director_holdings_async(symbol="2330")
    ///     ```
    #[pyo3(signature = (symbol, *, from_date=None, to_date=None, sort=None, **_extra))]
    pub fn director_holdings_async<'py>(
        &self,
        py: Python<'py>,
        symbol: String,
        from_date: Option<String>,
        to_date: Option<String>,
        sort: Option<String>,
        _extra: Option<Bound<'_, pyo3::types::PyDict>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let query = OwnershipQuery::resolve("stock.ownership.director_holdings", symbol, from_date, to_date, sort, &_extra)?;
        ownership_async(py, self.inner.clone(), query, send_director_holdings)
    }

    /// Sync sibling of `director_holdings_async()`, matching the legacy fugle-marketdata call shape.
    #[pyo3(signature = (symbol, *, from_date=None, to_date=None, sort=None, **_extra))]
    pub fn director_holdings(
        &self,
        py: Python<'_>,
        symbol: String,
        from_date: Option<String>,
        to_date: Option<String>,
        sort: Option<String>,
        _extra: Option<Bound<'_, pyo3::types::PyDict>>,
    ) -> PyResult<Py<pyo3::types::PyDict>> {
        let query = OwnershipQuery::resolve("stock.ownership.director_holdings", symbol, from_date, to_date, sort, &_extra)?;
        ownership_sync(py, &self.inner, query, send_director_holdings)
    }

    /// Get the weekly TDCC shareholder distribution by holding-size bracket
    ///
    /// Args:
    ///     symbol: Stock symbol (e.g. "2330") — required
    ///     from_date: Start of the date range (YYYY-MM-DD). The official SDK's
    ///         `from_=` / `**{"from": ...}` spellings are accepted too.
    ///     to_date: End of the date range (YYYY-MM-DD). `to=` is accepted too.
    ///     sort: "asc" (oldest first) or "desc" (newest first)
    ///
    /// Returns:
    ///     Awaitable[dict]: TDCC distribution data
    ///
    /// Example:
    ///     ```python
    ///     data = await client.stock.ownership.tdcc_distribution_async(symbol="2330")
    ///     ```
    #[pyo3(signature = (symbol, *, from_date=None, to_date=None, sort=None, **_extra))]
    pub fn tdcc_distribution_async<'py>(
        &self,
        py: Python<'py>,
        symbol: String,
        from_date: Option<String>,
        to_date: Option<String>,
        sort: Option<String>,
        _extra: Option<Bound<'_, pyo3::types::PyDict>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let query = OwnershipQuery::resolve("stock.ownership.tdcc_distribution", symbol, from_date, to_date, sort, &_extra)?;
        ownership_async(py, self.inner.clone(), query, send_tdcc_distribution)
    }

    /// Sync sibling of `tdcc_distribution_async()`, matching the legacy fugle-marketdata call shape.
    #[pyo3(signature = (symbol, *, from_date=None, to_date=None, sort=None, **_extra))]
    pub fn tdcc_distribution(
        &self,
        py: Python<'_>,
        symbol: String,
        from_date: Option<String>,
        to_date: Option<String>,
        sort: Option<String>,
        _extra: Option<Bound<'_, pyo3::types::PyDict>>,
    ) -> PyResult<Py<pyo3::types::PyDict>> {
        let query = OwnershipQuery::resolve("stock.ownership.tdcc_distribution", symbol, from_date, to_date, sort, &_extra)?;
        ownership_sync(py, &self.inner, query, send_tdcc_distribution)
    }
}

/// Parsed arguments shared by every `stock.ownership.*` method.
struct OwnershipQuery {
    symbol: String,
    from: Option<String>,
    to: Option<String>,
    sort: Option<String>,
}

impl OwnershipQuery {
    /// Normalise kwargs into a query.
    ///
    /// The official fugle-marketdata forwards `**params` verbatim as query
    /// parameters, so callers write `from_=` (2.6.0+ alias for the reserved
    /// word), `**{"from": ...}` or `to=`. Those spellings land in `**_extra`
    /// and are merged through the same table as every other REST method.
    fn resolve(
        method: &'static str,
        symbol: String,
        from_date: Option<String>,
        to_date: Option<String>,
        sort: Option<String>,
        extra: &Option<Bound<'_, pyo3::types::PyDict>>,
    ) -> PyResult<Self> {
        let mut kw = crate::kwargs::Kwargs::parse(method, extra)?;
        let from = kw.take_string("from", from_date)?;
        let to = kw.take_string("to", to_date)?;
        let sort = kw.take_string("sort", sort)?;
        kw.finish()?;
        Ok(Self { symbol, from, to, sort })
    }
}

macro_rules! ownership_sender {
    ($fn_name:ident, $method:ident) => {
        fn $fn_name(
            client: &marketdata_core::RestClient,
            q: OwnershipQuery,
        ) -> Result<serde_json::Value, marketdata_core::MarketDataError> {
            let stock = client.stock();
            let ownership = stock.ownership();
            let mut builder = ownership.$method().symbol(&q.symbol);
            if let Some(f) = q.from.as_deref() {
                builder = builder.from(f);
            }
            if let Some(t) = q.to.as_deref() {
                builder = builder.to(t);
            }
            if let Some(s) = q.sort.as_deref() {
                builder = builder.sort(s);
            }
            builder.send()
        }
    };
}

ownership_sender!(send_etf_holdings, etf_holdings);
ownership_sender!(send_institutional_trades, institutional_trades);
ownership_sender!(send_director_holdings, director_holdings);
ownership_sender!(send_tdcc_distribution, tdcc_distribution);

type OwnershipSend = fn(
    &marketdata_core::RestClient,
    OwnershipQuery,
) -> Result<serde_json::Value, marketdata_core::MarketDataError>;

fn ownership_async<'py>(
    py: Python<'py>,
    client: marketdata_core::RestClient,
    query: OwnershipQuery,
    send: OwnershipSend,
) -> PyResult<Bound<'py, PyAny>> {
    future_into_py(py, async move {
        let result = tokio::task::spawn_blocking(move || send(&client, query))
            .await
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Task join error: {}", e)))?;
        match result {
            Ok(data) => Python::attach(|py| types::value_to_dict(py, &data)),
            Err(e) => Err(errors::to_py_err(e)),
        }
    })
}

fn ownership_sync(
    py: Python<'_>,
    client: &marketdata_core::RestClient,
    query: OwnershipQuery,
    send: OwnershipSend,
) -> PyResult<Py<pyo3::types::PyDict>> {
    let client = client.clone();
    match py.detach(move || send(&client, query)) {
        Ok(data) => types::value_to_dict(py, &data),
        Err(e) => Err(errors::to_py_err(e)),
    }
}

/// Stock intraday (real-time) endpoints client
///
/// Access via `client.stock.intraday`
#[pyclass]
pub struct StockIntradayClient {
    inner: marketdata_core::RestClient,
}

#[pymethods]
impl StockIntradayClient {
    /// Get intraday quote for a stock symbol
    ///
    /// Args:
    ///     symbol: Stock symbol (e.g., "2330" for TSMC)
    ///     odd_lot: Whether to query odd lot data (default: False)
    ///
    /// Returns:
    ///     Awaitable[dict]: Quote data including prices, order book, and trading info
    ///
    /// Raises:
    ///     MarketDataError: If the request fails
    ///
    /// Example:
    ///     ```python
    ///     quote = await client.stock.intraday.quote_async("2330")
    ///     print(f"Last price: {quote['lastPrice']}")
    ///     print(f"Change: {quote['change']}")
    ///     ```
    #[pyo3(signature = (symbol, *, odd_lot=None, **_extra))]
    pub fn quote_async<'py>(&self, py: Python<'py>, symbol: String, odd_lot: Option<bool>, _extra: Option<Bound<'_, pyo3::types::PyDict>>) -> PyResult<Bound<'py, PyAny>> {
        let mut kw = crate::kwargs::Kwargs::parse("stock.intraday.quote", &_extra)?;
        let odd_lot = kw.take_flag("odd_lot", odd_lot)?;
        kw.finish()?;
        let client = self.inner.clone();
        future_into_py(py, async move {
            let result = tokio::task::spawn_blocking(move || {
                let stock = client.stock();
                let intraday = stock.intraday();
                let mut builder = intraday.quote().symbol(&symbol);
                if odd_lot == Some(true) {
                    builder = builder.odd_lot(true);
                }
                builder.send()
            }).await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Task join error: {}", e)))?;

            match result {
                Ok(quote) => Python::attach(|py| types::value_to_dict(py, &quote)),
                Err(e) => Err(errors::to_py_err(e)),
            }
        })
    }

    /// Get intraday quote for a stock symbol (synchronous, blocking).
    ///
    /// Sync sibling of `quote()` for callers migrating from the legacy
    /// fugle-marketdata Python SDK. Releases the GIL during the network call.
    #[pyo3(signature = (symbol, *, odd_lot=None, **_extra))]
    pub fn quote(&self, py: Python<'_>, symbol: String, odd_lot: Option<bool>, _extra: Option<Bound<'_, pyo3::types::PyDict>>) -> PyResult<Py<pyo3::types::PyDict>> {
        let mut kw = crate::kwargs::Kwargs::parse("stock.intraday.quote", &_extra)?;
        let odd_lot = kw.take_flag("odd_lot", odd_lot)?;
        kw.finish()?;
        let inner = self.inner.clone();
        let result = py.detach(|| {
            let stock = inner.stock();
            let intraday = stock.intraday();
            let mut builder = intraday.quote().symbol(&symbol);
            if odd_lot == Some(true) {
                builder = builder.odd_lot(true);
            }
            builder.send()
        });
        match result {
            Ok(quote) => types::value_to_dict(py, &quote),
            Err(e) => Err(errors::to_py_err(e)),
        }
    }

    /// Get ticker information for a stock symbol
    ///
    /// Args:
    ///     symbol: Stock symbol (e.g., "2330" for TSMC)
    ///
    /// Returns:
    ///     Awaitable[dict]: Ticker data
    ///
    /// Raises:
    ///     MarketDataError: If the request fails
    ///
    /// Example:
    ///     ```python
    ///     ticker = await client.stock.intraday.ticker_async("2330")
    ///     ```
    #[pyo3(signature = (symbol, *, odd_lot=None, **_extra))]
    pub fn ticker_async<'py>(&self, py: Python<'py>, symbol: String, odd_lot: Option<bool>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Bound<'py, PyAny>> {
        let mut kw = crate::kwargs::Kwargs::parse("stock.intraday.ticker", &_extra)?;
        let odd_lot = kw.take_flag("odd_lot", odd_lot)?;
        kw.finish()?;
        let client = self.inner.clone();
        future_into_py(py, async move {
            let result = tokio::task::spawn_blocking(move || {
                let stock = client.stock();
                let intraday = stock.intraday();
                let mut builder = intraday.ticker().symbol(&symbol);
                if odd_lot == Some(true) {
                    builder = builder.odd_lot(true);
                }
                builder.send()
            }).await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Task join error: {}", e)))?;

            match result {
                Ok(ticker) => Python::attach(|py| types::value_to_dict(py, &ticker)),
                Err(e) => Err(errors::to_py_err(e)),
            }
        })
    }

    /// Sync sibling of `ticker()` for legacy fugle-marketdata callers.
    #[pyo3(signature = (symbol, *, odd_lot=None, **_extra))]
    pub fn ticker(&self, py: Python<'_>, symbol: String, odd_lot: Option<bool>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Py<pyo3::types::PyDict>> {
        let mut kw = crate::kwargs::Kwargs::parse("stock.intraday.ticker", &_extra)?;
        let odd_lot = kw.take_flag("odd_lot", odd_lot)?;
        kw.finish()?;
        let inner = self.inner.clone();
        let result = py.detach(|| {
            let stock = inner.stock();
            let intraday = stock.intraday();
            let mut builder = intraday.ticker().symbol(&symbol);
            if odd_lot == Some(true) {
                builder = builder.odd_lot(true);
            }
            builder.send()
        });
        match result {
            Ok(ticker) => types::value_to_dict(py, &ticker),
            Err(e) => Err(errors::to_py_err(e)),
        }
    }

    /// Get candlestick chart data
    ///
    /// Args:
    ///     symbol: Stock symbol (e.g., "2330" for TSMC)
    ///     timeframe: Timeframe in minutes (default: "1")
    ///
    /// Returns:
    ///     Awaitable[dict]: Candlestick data
    ///
    /// Raises:
    ///     MarketDataError: If the request fails
    ///
    /// Example:
    ///     ```python
    ///     candles = await client.stock.intraday.candles_async("2330", timeframe="5")
    ///     ```
    #[pyo3(signature = (symbol, *, timeframe=None, odd_lot=None, sort=None, **_extra))]
    pub fn candles_async<'py>(&self, py: Python<'py>, symbol: String, timeframe: Option<String>, odd_lot: Option<bool>, sort: Option<String>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Bound<'py, PyAny>> {
        let mut kw = crate::kwargs::Kwargs::parse("stock.intraday.candles", &_extra)?;
        let odd_lot = kw.take_flag("odd_lot", odd_lot)?;
        let sort = kw.take_string("sort", sort)?;
        kw.finish()?;
        let client = self.inner.clone();
        future_into_py(py, async move {
            let result = tokio::task::spawn_blocking(move || {
                let stock = client.stock();
                let intraday = stock.intraday();
                let mut builder = intraday.candles().symbol(&symbol);
                if let Some(tf) = &timeframe {
                    builder = builder.timeframe(tf);
                }
                if odd_lot == Some(true) {
                    builder = builder.odd_lot(true);
                }
                if let Some(v) = &sort {
                    builder = builder.sort(v);
                }
                builder.send()
            }).await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Task join error: {}", e)))?;

            match result {
                Ok(candles) => Python::attach(|py| types::value_to_dict(py, &candles)),
                Err(e) => Err(errors::to_py_err(e)),
            }
        })
    }

    /// Sync sibling of `candles()` for legacy fugle-marketdata callers.
    #[pyo3(signature = (symbol, *, timeframe=None, odd_lot=None, sort=None, **_extra))]
    pub fn candles(&self, py: Python<'_>, symbol: String, timeframe: Option<String>, odd_lot: Option<bool>, sort: Option<String>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Py<pyo3::types::PyDict>> {
        let mut kw = crate::kwargs::Kwargs::parse("stock.intraday.candles", &_extra)?;
        let odd_lot = kw.take_flag("odd_lot", odd_lot)?;
        let sort = kw.take_string("sort", sort)?;
        kw.finish()?;
        let inner = self.inner.clone();
        let result = py.detach(|| {
            let stock = inner.stock();
            let intraday = stock.intraday();
            let mut builder = intraday.candles().symbol(&symbol);
            if let Some(tf) = &timeframe {
                builder = builder.timeframe(tf);
            }
            if odd_lot == Some(true) {
                builder = builder.odd_lot(true);
            }
            if let Some(v) = &sort {
                builder = builder.sort(v);
            }
            builder.send()
        });
        match result {
            Ok(candles) => types::value_to_dict(py, &candles),
            Err(e) => Err(errors::to_py_err(e)),
        }
    }

    /// Get trade ticks data
    ///
    /// Args:
    ///     symbol: Stock symbol (e.g., "2330" for TSMC)
    ///
    /// Returns:
    ///     Awaitable[dict]: Trade ticks data
    ///
    /// Raises:
    ///     MarketDataError: If the request fails
    ///
    /// Example:
    ///     ```python
    ///     trades = await client.stock.intraday.trades_async("2330")
    ///     ```
    #[pyo3(signature = (symbol, *, odd_lot=None, offset=None, limit=None, sort=None, is_trial=None, **_extra))]
    #[allow(clippy::too_many_arguments, reason = "mirrors the Python keyword signature")]
    pub fn trades_async<'py>(&self, py: Python<'py>, symbol: String, odd_lot: Option<bool>, offset: Option<u32>, limit: Option<u32>, sort: Option<String>, is_trial: Option<bool>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Bound<'py, PyAny>> {
        let mut kw = crate::kwargs::Kwargs::parse("stock.intraday.trades", &_extra)?;
        let odd_lot = kw.take_flag("odd_lot", odd_lot)?;
        let offset = kw.take("offset", offset)?;
        let limit = kw.take("limit", limit)?;
        let sort = kw.take_string("sort", sort)?;
        let is_trial = kw.take("is_trial", is_trial)?;
        kw.finish()?;
        let client = self.inner.clone();
        future_into_py(py, async move {
            let result = tokio::task::spawn_blocking(move || {
                let stock = client.stock();
                let intraday = stock.intraday();
                let mut builder = intraday.trades().symbol(&symbol);
                if odd_lot == Some(true) {
                    builder = builder.odd_lot(true);
                }
                if let Some(v) = offset {
                    builder = builder.offset(v);
                }
                if let Some(v) = limit {
                    builder = builder.limit(v);
                }
                if let Some(v) = sort.as_deref() {
                    builder = builder.sort(v);
                }
                if let Some(v) = is_trial {
                    builder = builder.is_trial(v);
                }
                builder.send()
            }).await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Task join error: {}", e)))?;

            match result {
                Ok(trades) => Python::attach(|py| types::value_to_dict(py, &trades)),
                Err(e) => Err(errors::to_py_err(e)),
            }
        })
    }

    /// Sync sibling of `trades()` for legacy fugle-marketdata callers.
    #[pyo3(signature = (symbol, *, odd_lot=None, offset=None, limit=None, sort=None, is_trial=None, **_extra))]
    #[allow(clippy::too_many_arguments, reason = "mirrors the Python keyword signature")]
    pub fn trades(&self, py: Python<'_>, symbol: String, odd_lot: Option<bool>, offset: Option<u32>, limit: Option<u32>, sort: Option<String>, is_trial: Option<bool>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Py<pyo3::types::PyDict>> {
        let mut kw = crate::kwargs::Kwargs::parse("stock.intraday.trades", &_extra)?;
        let odd_lot = kw.take_flag("odd_lot", odd_lot)?;
        let offset = kw.take("offset", offset)?;
        let limit = kw.take("limit", limit)?;
        let sort = kw.take_string("sort", sort)?;
        let is_trial = kw.take("is_trial", is_trial)?;
        kw.finish()?;
        let inner = self.inner.clone();
        let result = py.detach(|| {
            let stock = inner.stock();
            let intraday = stock.intraday();
            let mut builder = intraday.trades().symbol(&symbol);
            if odd_lot == Some(true) {
                builder = builder.odd_lot(true);
            }
            if let Some(v) = offset {
                builder = builder.offset(v);
            }
            if let Some(v) = limit {
                builder = builder.limit(v);
            }
            if let Some(v) = sort.as_deref() {
                builder = builder.sort(v);
            }
            if let Some(v) = is_trial {
                builder = builder.is_trial(v);
            }
            builder.send()
        });
        match result {
            Ok(trades) => types::value_to_dict(py, &trades),
            Err(e) => Err(errors::to_py_err(e)),
        }
    }

    /// Get volume data
    ///
    /// Args:
    ///     symbol: Stock symbol (e.g., "2330" for TSMC)
    ///
    /// Returns:
    ///     Awaitable[dict]: Volume data
    ///
    /// Raises:
    ///     MarketDataError: If the request fails
    ///
    /// Example:
    ///     ```python
    ///     volumes = await client.stock.intraday.volumes_async("2330")
    ///     ```
    #[pyo3(signature = (symbol, *, odd_lot=None, **_extra))]
    pub fn volumes_async<'py>(&self, py: Python<'py>, symbol: String, odd_lot: Option<bool>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Bound<'py, PyAny>> {
        let mut kw = crate::kwargs::Kwargs::parse("stock.intraday.volumes", &_extra)?;
        let odd_lot = kw.take_flag("odd_lot", odd_lot)?;
        kw.finish()?;
        let client = self.inner.clone();
        future_into_py(py, async move {
            let result = tokio::task::spawn_blocking(move || {
                let stock = client.stock();
                let intraday = stock.intraday();
                let mut builder = intraday.volumes().symbol(&symbol);
                if odd_lot == Some(true) {
                    builder = builder.odd_lot(true);
                }
                builder.send()
            }).await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Task join error: {}", e)))?;

            match result {
                Ok(volumes) => Python::attach(|py| types::value_to_dict(py, &volumes)),
                Err(e) => Err(errors::to_py_err(e)),
            }
        })
    }

    /// Sync sibling of `volumes()` for legacy fugle-marketdata callers.
    #[pyo3(signature = (symbol, *, odd_lot=None, **_extra))]
    pub fn volumes(&self, py: Python<'_>, symbol: String, odd_lot: Option<bool>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Py<pyo3::types::PyDict>> {
        let mut kw = crate::kwargs::Kwargs::parse("stock.intraday.volumes", &_extra)?;
        let odd_lot = kw.take_flag("odd_lot", odd_lot)?;
        kw.finish()?;
        let inner = self.inner.clone();
        let result = py.detach(|| {
            let stock = inner.stock();
            let intraday = stock.intraday();
            let mut builder = intraday.volumes().symbol(&symbol);
            if odd_lot == Some(true) {
                builder = builder.odd_lot(true);
            }
            builder.send()
        });
        match result {
            Ok(volumes) => types::value_to_dict(py, &volumes),
            Err(e) => Err(errors::to_py_err(e)),
        }
    }

    /// Get batch ticker list for a security type
    ///
    /// Args:
    ///     type: Security type (e.g., "EQUITY", "INDEX", "ETF")
    ///     exchange: Exchange filter (e.g., "TWSE", "TPEx")
    ///     market: Market filter (e.g., "TSE", "OTC")
    ///     industry: Industry code filter
    ///     is_normal: Filter to normal-status tickers only
    ///
    /// Returns:
    ///     Awaitable[list[dict]]: List of ticker info dicts
    ///
    /// Example:
    ///     ```python
    ///     tickers = await client.stock.intraday.tickers_async(type="EQUITY")
    ///     ```
    #[pyo3(signature = (r#type, *, exchange=None, market=None, industry=None, is_normal=None, is_attention=None, is_disposition=None, is_halted=None, symbol=None, **_extra))]
    #[allow(clippy::too_many_arguments, reason = "mirrors the Python keyword signature")]
    pub fn tickers_async<'py>(
        &self,
        py: Python<'py>,
        r#type: String,
        exchange: Option<String>,
        market: Option<String>,
        industry: Option<String>,
        is_normal: Option<bool>,is_attention: Option<bool>, is_disposition: Option<bool>, is_halted: Option<bool>, symbol: Option<String>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Bound<'py, PyAny>> {
        let mut kw = crate::kwargs::Kwargs::parse("stock.intraday.tickers", &_extra)?;
        let exchange = kw.take_string("exchange", exchange)?;
        let industry = kw.take_string("industry", industry)?;
        let is_normal = kw.take("is_normal", is_normal)?;
        let is_attention = kw.take("is_attention", is_attention)?;
        let is_disposition = kw.take("is_disposition", is_disposition)?;
        let is_halted = kw.take("is_halted", is_halted)?;
        let symbol = kw.take_string("symbol", symbol)?;
        kw.finish()?;
        let client = self.inner.clone();
        future_into_py(py, async move {
            let result = tokio::task::spawn_blocking(move || {
                let stock = client.stock();
                let intraday = stock.intraday();
                let mut builder = intraday.tickers().typ(&r#type);
                if let Some(e) = &exchange {
                    builder = builder.exchange(e);
                }
                if let Some(m) = &market {
                    builder = builder.market(m);
                }
                if let Some(i) = &industry {
                    builder = builder.industry(i);
                }
                if let Some(n) = is_normal {
                    builder = builder.is_normal(n);
                }
                if let Some(v) = is_attention {
                    builder = builder.is_attention(v);
                }
                if let Some(v) = is_disposition {
                    builder = builder.is_disposition(v);
                }
                if let Some(v) = is_halted {
                    builder = builder.is_halted(v);
                }
                if let Some(v) = &symbol {
                    builder = builder.symbol(v);
                }
                builder.send()
            }).await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Task join error: {}", e)))?;

            match result {
                Ok(tickers) => Python::attach(|py| {
                    let json_val = serde_json::to_value(&tickers)
                        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Serialization error: {}", e)))?;
                    types::json_value_to_py(py, &json_val)
                }),
                Err(e) => Err(errors::to_py_err(e)),
            }
        })
    }

    /// Sync sibling of `tickers()` for legacy fugle-marketdata callers.
    #[pyo3(signature = (r#type, *, exchange=None, market=None, industry=None, is_normal=None, is_attention=None, is_disposition=None, is_halted=None, symbol=None, **_extra))]
    #[allow(clippy::too_many_arguments, reason = "mirrors the Python keyword signature")]
    pub fn tickers(
        &self,
        py: Python<'_>,
        r#type: String,
        exchange: Option<String>,
        market: Option<String>,
        industry: Option<String>,
        is_normal: Option<bool>,is_attention: Option<bool>, is_disposition: Option<bool>, is_halted: Option<bool>, symbol: Option<String>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Py<PyAny>> {
        let mut kw = crate::kwargs::Kwargs::parse("stock.intraday.tickers", &_extra)?;
        let exchange = kw.take_string("exchange", exchange)?;
        let industry = kw.take_string("industry", industry)?;
        let is_normal = kw.take("is_normal", is_normal)?;
        let is_attention = kw.take("is_attention", is_attention)?;
        let is_disposition = kw.take("is_disposition", is_disposition)?;
        let is_halted = kw.take("is_halted", is_halted)?;
        let symbol = kw.take_string("symbol", symbol)?;
        kw.finish()?;
        let inner = self.inner.clone();
        let result = py.detach(|| {
            let stock = inner.stock();
            let intraday = stock.intraday();
            let mut builder = intraday.tickers().typ(&r#type);
            if let Some(e) = &exchange {
                builder = builder.exchange(e);
            }
            if let Some(m) = &market {
                builder = builder.market(m);
            }
            if let Some(i) = &industry {
                builder = builder.industry(i);
            }
            if let Some(n) = is_normal {
                builder = builder.is_normal(n);
            }
            if let Some(v) = is_attention {
                builder = builder.is_attention(v);
            }
            if let Some(v) = is_disposition {
                builder = builder.is_disposition(v);
            }
            if let Some(v) = is_halted {
                builder = builder.is_halted(v);
            }
            if let Some(v) = &symbol {
                builder = builder.symbol(v);
            }
            builder.send()
        });
        match result {
            Ok(tickers) => {
                let json_val = serde_json::to_value(&tickers)
                    .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Serialization error: {}", e)))?;
                types::json_value_to_py(py, &json_val)
            }
            Err(e) => Err(errors::to_py_err(e)),
        }
    }
}

/// Stock historical data endpoints client
///
/// Access via `client.stock.historical`
#[pyclass]
pub struct StockHistoricalClient {
    inner: marketdata_core::RestClient,
}

#[pymethods]
impl StockHistoricalClient {
    /// Get historical candles for a stock symbol
    ///
    /// Args:
    ///     symbol: Stock symbol (e.g., "2330" for TSMC)
    ///     from_date: Start date (YYYY-MM-DD)
    ///     to_date: End date (YYYY-MM-DD)
    ///     timeframe: Timeframe ("D", "W", "M", "1", "5", "10", "15", "30", "60")
    ///     fields: Optional field selection
    ///     sort: Sort order ("asc" or "desc")
    ///     adjusted: Whether to adjust for splits/dividends
    ///
    /// Returns:
    ///     Awaitable[dict]: Historical candles data
    ///
    /// Example:
    ///     ```python
    ///     candles = await client.stock.historical.candles_async(
    ///         "2330",
    ///         from_date="2024-01-01",
    ///         to_date="2024-01-31",
    ///         timeframe="D"
    ///     )
    ///     ```
    #[pyo3(signature = (symbol, *, from_date=None, to_date=None, timeframe=None, fields=None, sort=None, adjusted=None, **_extra))]
    pub fn candles_async<'py>(
        &self,
        py: Python<'py>,
        symbol: String,
        from_date: Option<String>,
        to_date: Option<String>,
        timeframe: Option<String>,
        fields: Option<String>,
        sort: Option<String>,
        adjusted: Option<bool>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Bound<'py, PyAny>> {
        let mut kw = crate::kwargs::Kwargs::parse("stock.historical.candles", &_extra)?;
        let from_date = kw.take_string("from", from_date)?;
        let to_date = kw.take_string("to", to_date)?;
        let timeframe = kw.take_string("timeframe", timeframe)?;
        let fields = kw.take_string("fields", fields)?;
        let sort = kw.take_string("sort", sort)?;
        let adjusted = kw.take("adjusted", adjusted)?;
        kw.finish()?;
        let client = self.inner.clone();
        future_into_py(py, async move {
            let result = tokio::task::spawn_blocking(move || {
                let stock = client.stock();
                let historical = stock.historical();
                let mut builder = historical.candles().symbol(&symbol);
                if let Some(f) = from_date {
                    builder = builder.from(&f);
                }
                if let Some(t) = to_date {
                    builder = builder.to(&t);
                }
                if let Some(tf) = timeframe {
                    builder = builder.timeframe(&tf);
                }
                if let Some(fld) = fields {
                    builder = builder.fields(&fld);
                }
                if let Some(s) = sort {
                    builder = builder.sort(&s);
                }
                if let Some(adj) = adjusted {
                    builder = builder.adjusted(adj);
                }
                builder.send()
            })
            .await
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Task join error: {}", e)))?;

            match result {
                Ok(candles) => Python::attach(|py| types::value_to_dict(py, &candles)),
                Err(e) => Err(errors::to_py_err(e)),
            }
        })
    }

    /// Sync sibling of `candles()` for legacy fugle-marketdata callers.
    #[pyo3(signature = (symbol, *, from_date=None, to_date=None, timeframe=None, fields=None, sort=None, adjusted=None, **_extra))]
    pub fn candles(
        &self,
        py: Python<'_>,
        symbol: String,
        from_date: Option<String>,
        to_date: Option<String>,
        timeframe: Option<String>,
        fields: Option<String>,
        sort: Option<String>,
        adjusted: Option<bool>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Py<pyo3::types::PyDict>> {
        let mut kw = crate::kwargs::Kwargs::parse("stock.historical.candles", &_extra)?;
        let from_date = kw.take_string("from", from_date)?;
        let to_date = kw.take_string("to", to_date)?;
        let timeframe = kw.take_string("timeframe", timeframe)?;
        let fields = kw.take_string("fields", fields)?;
        let sort = kw.take_string("sort", sort)?;
        let adjusted = kw.take("adjusted", adjusted)?;
        kw.finish()?;
        let inner = self.inner.clone();
        let result = py.detach(|| {
            let stock = inner.stock();
            let historical = stock.historical();
            let mut builder = historical.candles().symbol(&symbol);
            if let Some(f) = from_date {
                builder = builder.from(&f);
            }
            if let Some(t) = to_date {
                builder = builder.to(&t);
            }
            if let Some(tf) = timeframe {
                builder = builder.timeframe(&tf);
            }
            if let Some(fld) = fields {
                builder = builder.fields(&fld);
            }
            if let Some(s) = sort {
                builder = builder.sort(&s);
            }
            if let Some(adj) = adjusted {
                builder = builder.adjusted(adj);
            }
            builder.send()
        });
        match result {
            Ok(candles) => types::value_to_dict(py, &candles),
            Err(e) => Err(errors::to_py_err(e)),
        }
    }

    /// Get historical stats for a stock symbol
    ///
    /// Args:
    ///     symbol: Stock symbol (e.g., "2330" for TSMC)
    ///
    /// Returns:
    ///     Awaitable[dict]: Historical stats data including 52-week high/low
    ///
    /// Example:
    ///     ```python
    ///     stats = await client.stock.historical.stats_async("2330")
    ///     print(f"52-week high: {stats['week52High']}")
    ///     ```
    #[pyo3(signature = (symbol, **_extra))]
    pub fn stats_async<'py>(&self, py: Python<'py>, symbol: String, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Bound<'py, PyAny>> {
        let kw = crate::kwargs::Kwargs::parse("stock.historical.stats", &_extra)?;
        kw.finish()?;
        let client = self.inner.clone();
        future_into_py(py, async move {
            let result = tokio::task::spawn_blocking(move || {
                let stock = client.stock();
                let historical = stock.historical();
                historical.stats().symbol(&symbol).send()
            })
            .await
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Task join error: {}", e)))?;

            match result {
                Ok(stats) => Python::attach(|py| types::value_to_dict(py, &stats)),
                Err(e) => Err(errors::to_py_err(e)),
            }
        })
    }

    /// Sync sibling of `stats()` for legacy fugle-marketdata callers.
    #[pyo3(signature = (symbol, **_extra))]
    pub fn stats(&self, py: Python<'_>, symbol: String, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Py<pyo3::types::PyDict>> {
        let kw = crate::kwargs::Kwargs::parse("stock.historical.stats", &_extra)?;
        kw.finish()?;
        let inner = self.inner.clone();
        let result = py.detach(|| {
            let stock = inner.stock();
            let historical = stock.historical();
            historical.stats().symbol(&symbol).send()
        });
        match result {
            Ok(stats) => types::value_to_dict(py, &stats),
            Err(e) => Err(errors::to_py_err(e)),
        }
    }
}

/// Stock snapshot endpoints client
///
/// Access via `client.stock.snapshot`
#[pyclass]
pub struct StockSnapshotClient {
    inner: marketdata_core::RestClient,
}

#[pymethods]
impl StockSnapshotClient {
    /// Get snapshot quotes for a market
    ///
    /// Args:
    ///     market: Market code ("TSE", "OTC", "ESB", "TIB", "PSB")
    ///     type_filter: Type filter ("ALL", "ALLBUT0999", "COMMONSTOCK")
    ///
    /// Returns:
    ///     Awaitable[dict]: Market-wide quotes snapshot
    ///
    /// Example:
    ///     ```python
    ///     quotes = await client.stock.snapshot.quotes_async("TSE", type_filter="COMMONSTOCK")
    ///     ```
    #[pyo3(signature = (market, *, type_filter=None, **_extra))]
    pub fn quotes_async<'py>(
        &self,
        py: Python<'py>,
        market: String,
        type_filter: Option<String>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Bound<'py, PyAny>> {
        let mut kw = crate::kwargs::Kwargs::parse("stock.snapshot.quotes", &_extra)?;
        let type_filter = kw.take_string("type", type_filter)?;
        kw.finish()?;
        let client = self.inner.clone();
        future_into_py(py, async move {
            let result = tokio::task::spawn_blocking(move || {
                let stock = client.stock();
                let snapshot = stock.snapshot();
                let mut builder = snapshot.quotes().market(&market);
                if let Some(tf) = type_filter {
                    builder = builder.type_filter(&tf);
                }
                builder.send()
            })
            .await
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Task join error: {}", e)))?;

            match result {
                Ok(quotes) => Python::attach(|py| types::value_to_dict(py, &quotes)),
                Err(e) => Err(errors::to_py_err(e)),
            }
        })
    }

    /// Get top movers for a market
    ///
    /// Args:
    ///     market: Market code ("TSE", "OTC", "ESB", "TIB", "PSB")
    ///     direction: Direction filter ("up" for gainers, "down" for losers)
    ///     change: Change type ("percent" or "value")
    ///
    /// Returns:
    ///     Awaitable[dict]: Top movers data
    ///
    /// Example:
    ///     ```python
    ///     movers = await client.stock.snapshot.movers_async("TSE", direction="up", change="percent")
    ///     ```
    #[pyo3(signature = (market, direction=None, change=None, *, type_filter=None, gt=None, gte=None, lt=None, lte=None, eq=None, **_extra))]
    #[allow(clippy::too_many_arguments, reason = "mirrors the Python keyword signature")]
    pub fn movers_async<'py>(
        &self,
        py: Python<'py>,
        market: String,
        direction: Option<String>,
        change: Option<String>,type_filter: Option<String>, gt: Option<f64>, gte: Option<f64>, lt: Option<f64>, lte: Option<f64>, eq: Option<f64>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Bound<'py, PyAny>> {
        let mut kw = crate::kwargs::Kwargs::parse("stock.snapshot.movers", &_extra)?;
        let direction = kw.take_string("direction", direction)?;
        let change = kw.take_string("change", change)?;
        let type_filter = kw.take_string("type", type_filter)?;
        let gt = kw.take("gt", gt)?;
        let gte = kw.take("gte", gte)?;
        let lt = kw.take("lt", lt)?;
        let lte = kw.take("lte", lte)?;
        let eq = kw.take("eq", eq)?;
        kw.finish()?;
        let client = self.inner.clone();
        future_into_py(py, async move {
            let result = tokio::task::spawn_blocking(move || {
                let stock = client.stock();
                let snapshot = stock.snapshot();
                let mut builder = snapshot.movers().market(&market);
                if let Some(d) = direction {
                    builder = builder.direction(&d);
                }
                if let Some(c) = change {
                    builder = builder.change(&c);
                }
                if let Some(v) = &type_filter {
                    builder = builder.type_filter(v);
                }
                if let Some(v) = gt {
                    builder = builder.gt(v);
                }
                if let Some(v) = gte {
                    builder = builder.gte(v);
                }
                if let Some(v) = lt {
                    builder = builder.lt(v);
                }
                if let Some(v) = lte {
                    builder = builder.lte(v);
                }
                if let Some(v) = eq {
                    builder = builder.eq(v);
                }
                builder.send()
            })
            .await
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Task join error: {}", e)))?;

            match result {
                Ok(movers) => Python::attach(|py| types::value_to_dict(py, &movers)),
                Err(e) => Err(errors::to_py_err(e)),
            }
        })
    }

    /// Get most active stocks for a market
    ///
    /// Args:
    ///     market: Market code ("TSE", "OTC", "ESB", "TIB", "PSB")
    ///     trade: Trade type ("volume" or "value")
    ///     type_filter: Stock type filter, "ALLBUT0999" or "COMMONSTOCK" (sent as `type`)
    ///     gt: Only changes greater than this
    ///     gte: Only changes greater than or equal to this
    ///     lt: Only changes less than this
    ///     lte: Only changes less than or equal to this
    ///     eq: Only changes equal to this
    ///
    /// Returns:
    ///     Awaitable[dict]: Most active stocks data
    ///
    /// Example:
    ///     ```python
    ///     actives = await client.stock.snapshot.actives_async("TSE", trade="volume")
    ///     ```
    #[pyo3(signature = (market, trade=None, *, type_filter=None, **_extra))]
    pub fn actives_async<'py>(
        &self,
        py: Python<'py>,
        market: String,
        trade: Option<String>,type_filter: Option<String>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Bound<'py, PyAny>> {
        let mut kw = crate::kwargs::Kwargs::parse("stock.snapshot.actives", &_extra)?;
        let trade = kw.take_string("trade", trade)?;
        let type_filter = kw.take_string("type", type_filter)?;
        kw.finish()?;
        let client = self.inner.clone();
        future_into_py(py, async move {
            let result = tokio::task::spawn_blocking(move || {
                let stock = client.stock();
                let snapshot = stock.snapshot();
                let mut builder = snapshot.actives().market(&market);
                if let Some(t) = trade {
                    builder = builder.trade(&t);
                }
                if let Some(v) = &type_filter {
                    builder = builder.type_filter(v);
                }
                builder.send()
            })
            .await
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Task join error: {}", e)))?;

            match result {
                Ok(actives) => Python::attach(|py| types::value_to_dict(py, &actives)),
                Err(e) => Err(errors::to_py_err(e)),
            }
        })
    }

    /// Sync sibling of `quotes()` for legacy fugle-marketdata callers.
    #[pyo3(signature = (market, *, type_filter=None, **_extra))]
    pub fn quotes(
        &self,
        py: Python<'_>,
        market: String,
        type_filter: Option<String>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Py<pyo3::types::PyDict>> {
        let mut kw = crate::kwargs::Kwargs::parse("stock.snapshot.quotes", &_extra)?;
        let type_filter = kw.take_string("type", type_filter)?;
        kw.finish()?;
        let inner = self.inner.clone();
        let result = py.detach(|| {
            let stock = inner.stock();
            let snapshot = stock.snapshot();
            let mut builder = snapshot.quotes().market(&market);
            if let Some(tf) = type_filter {
                builder = builder.type_filter(&tf);
            }
            builder.send()
        });
        match result {
            Ok(quotes) => types::value_to_dict(py, &quotes),
            Err(e) => Err(errors::to_py_err(e)),
        }
    }

    /// Sync sibling of `movers()` for legacy fugle-marketdata callers.
    #[pyo3(signature = (market, direction=None, change=None, *, type_filter=None, gt=None, gte=None, lt=None, lte=None, eq=None, **_extra))]
    #[allow(clippy::too_many_arguments, reason = "mirrors the Python keyword signature")]
    pub fn movers(
        &self,
        py: Python<'_>,
        market: String,
        direction: Option<String>,
        change: Option<String>,type_filter: Option<String>, gt: Option<f64>, gte: Option<f64>, lt: Option<f64>, lte: Option<f64>, eq: Option<f64>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Py<pyo3::types::PyDict>> {
        let mut kw = crate::kwargs::Kwargs::parse("stock.snapshot.movers", &_extra)?;
        let direction = kw.take_string("direction", direction)?;
        let change = kw.take_string("change", change)?;
        let type_filter = kw.take_string("type", type_filter)?;
        let gt = kw.take("gt", gt)?;
        let gte = kw.take("gte", gte)?;
        let lt = kw.take("lt", lt)?;
        let lte = kw.take("lte", lte)?;
        let eq = kw.take("eq", eq)?;
        kw.finish()?;
        let inner = self.inner.clone();
        let result = py.detach(|| {
            let stock = inner.stock();
            let snapshot = stock.snapshot();
            let mut builder = snapshot.movers().market(&market);
            if let Some(d) = direction {
                builder = builder.direction(&d);
            }
            if let Some(c) = change {
                builder = builder.change(&c);
            }
            if let Some(v) = &type_filter {
                builder = builder.type_filter(v);
            }
            if let Some(v) = gt {
                builder = builder.gt(v);
            }
            if let Some(v) = gte {
                builder = builder.gte(v);
            }
            if let Some(v) = lt {
                builder = builder.lt(v);
            }
            if let Some(v) = lte {
                builder = builder.lte(v);
            }
            if let Some(v) = eq {
                builder = builder.eq(v);
            }
            builder.send()
        });
        match result {
            Ok(movers) => types::value_to_dict(py, &movers),
            Err(e) => Err(errors::to_py_err(e)),
        }
    }

    /// Sync sibling of `actives()` for legacy fugle-marketdata callers.
    #[pyo3(signature = (market, trade=None, *, type_filter=None, **_extra))]
    pub fn actives(
        &self,
        py: Python<'_>,
        market: String,
        trade: Option<String>,type_filter: Option<String>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Py<pyo3::types::PyDict>> {
        let mut kw = crate::kwargs::Kwargs::parse("stock.snapshot.actives", &_extra)?;
        let trade = kw.take_string("trade", trade)?;
        let type_filter = kw.take_string("type", type_filter)?;
        kw.finish()?;
        let inner = self.inner.clone();
        let result = py.detach(|| {
            let stock = inner.stock();
            let snapshot = stock.snapshot();
            let mut builder = snapshot.actives().market(&market);
            if let Some(t) = trade {
                builder = builder.trade(&t);
            }
            if let Some(v) = &type_filter {
                builder = builder.type_filter(v);
            }
            builder.send()
        });
        match result {
            Ok(actives) => types::value_to_dict(py, &actives),
            Err(e) => Err(errors::to_py_err(e)),
        }
    }
}

/// Stock technical indicator endpoints client
///
/// Access via `client.stock.technical`
#[pyclass]
pub struct StockTechnicalClient {
    inner: marketdata_core::RestClient,
}

#[pymethods]
impl StockTechnicalClient {
    /// Get Simple Moving Average (SMA) data
    ///
    /// Args:
    ///     symbol: Stock symbol (e.g., "2330" for TSMC)
    ///     period: Moving average period
    ///     from_date: Start date (YYYY-MM-DD)
    ///     to_date: End date (YYYY-MM-DD)
    ///     timeframe: Timeframe ("D", "W", "M", "1", "5", etc.)
    ///
    /// Returns:
    ///     Awaitable[dict]: SMA indicator data
    #[pyo3(signature = (symbol, period=None, *, from_date=None, to_date=None, timeframe=None, **_extra))]
    pub fn sma_async<'py>(
        &self,
        py: Python<'py>,
        symbol: String,
        period: Option<u32>,
        from_date: Option<String>,
        to_date: Option<String>,
        timeframe: Option<String>,
        _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Bound<'py, PyAny>> {
        let mut kw = crate::kwargs::Kwargs::parse("stock.technical.sma", &_extra)?;
        let from_date = kw.take_string("from", from_date)?;
        let to_date = kw.take_string("to", to_date)?;
        let timeframe = kw.take_string("timeframe", timeframe)?;
        let period = kw.take("period", period)?;
        kw.finish()?;
        let client = self.inner.clone();
        future_into_py(py, async move {
            let result = tokio::task::spawn_blocking(move || {
                let stock = client.stock();
                let technical = stock.technical();
                let mut builder = technical.sma().symbol(&symbol);
                if let Some(f) = from_date {
                    builder = builder.from(&f);
                }
                if let Some(t) = to_date {
                    builder = builder.to(&t);
                }
                if let Some(tf) = timeframe {
                    builder = builder.timeframe(&tf);
                }
                if let Some(p) = period {
                    builder = builder.period(p);
                }
                builder.send()
            })
            .await
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Task join error: {}", e)))?;

            match result {
                Ok(sma) => Python::attach(|py| types::value_to_dict(py, &sma)),
                Err(e) => Err(errors::to_py_err(e)),
            }
        })
    }

    /// Get Relative Strength Index (RSI) data
    ///
    /// Args:
    ///     symbol: Stock symbol (e.g., "2330" for TSMC)
    ///     period: RSI period (default 14)
    ///     from_date: Start date (YYYY-MM-DD)
    ///     to_date: End date (YYYY-MM-DD)
    ///     timeframe: Timeframe ("D", "W", "M", "1", "5", etc.)
    ///
    /// Returns:
    ///     Awaitable[dict]: RSI indicator data
    #[pyo3(signature = (symbol, period=None, *, from_date=None, to_date=None, timeframe=None, **_extra))]
    pub fn rsi_async<'py>(
        &self,
        py: Python<'py>,
        symbol: String,
        period: Option<u32>,
        from_date: Option<String>,
        to_date: Option<String>,
        timeframe: Option<String>,
        _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Bound<'py, PyAny>> {
        let mut kw = crate::kwargs::Kwargs::parse("stock.technical.rsi", &_extra)?;
        let from_date = kw.take_string("from", from_date)?;
        let to_date = kw.take_string("to", to_date)?;
        let timeframe = kw.take_string("timeframe", timeframe)?;
        let period = kw.take("period", period)?;
        kw.finish()?;
        let client = self.inner.clone();
        future_into_py(py, async move {
            let result = tokio::task::spawn_blocking(move || {
                let stock = client.stock();
                let technical = stock.technical();
                let mut builder = technical.rsi().symbol(&symbol);
                if let Some(f) = from_date {
                    builder = builder.from(&f);
                }
                if let Some(t) = to_date {
                    builder = builder.to(&t);
                }
                if let Some(tf) = timeframe {
                    builder = builder.timeframe(&tf);
                }
                if let Some(p) = period {
                    builder = builder.period(p);
                }
                builder.send()
            })
            .await
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Task join error: {}", e)))?;

            match result {
                Ok(rsi) => Python::attach(|py| types::value_to_dict(py, &rsi)),
                Err(e) => Err(errors::to_py_err(e)),
            }
        })
    }

    /// Get KDJ (Stochastic Oscillator) data
    ///
    /// Args:
    ///     symbol: Stock symbol (e.g., "2330" for TSMC)
    ///     r_period: RSV period (e.g., 9)
    ///     k_period: K smoothing period (e.g., 3)
    ///     d_period: D smoothing period (e.g., 3)
    ///     from_date: Start date (YYYY-MM-DD)
    ///     to_date: End date (YYYY-MM-DD)
    ///     timeframe: Timeframe ("D", "W", "M", "1", "5", etc.)
    ///
    /// Returns:
    ///     Awaitable[dict]: KDJ indicator data with K, D, J values
    #[pyo3(signature = (symbol, r_period=None, k_period=None, d_period=None, *, from_date=None, to_date=None, timeframe=None, **_extra))]
    pub fn kdj_async<'py>(
        &self,
        py: Python<'py>,
        symbol: String,
        r_period: Option<u32>,
        k_period: Option<u32>,
        d_period: Option<u32>,
        from_date: Option<String>,
        to_date: Option<String>,
        timeframe: Option<String>,
        _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Bound<'py, PyAny>> {
        let mut kw = crate::kwargs::Kwargs::parse("stock.technical.kdj", &_extra)?;
        let from_date = kw.take_string("from", from_date)?;
        let to_date = kw.take_string("to", to_date)?;
        let timeframe = kw.take_string("timeframe", timeframe)?;
        let r_period = kw.take("r_period", r_period)?;
        let k_period = kw.take("k_period", k_period)?;
        let d_period = kw.take("d_period", d_period)?;
        kw.finish()?;
        let client = self.inner.clone();
        future_into_py(py, async move {
            let result = tokio::task::spawn_blocking(move || {
                let stock = client.stock();
                let technical = stock.technical();
                let mut builder = technical.kdj().symbol(&symbol);
                if let Some(f) = from_date {
                    builder = builder.from(&f);
                }
                if let Some(t) = to_date {
                    builder = builder.to(&t);
                }
                if let Some(tf) = timeframe {
                    builder = builder.timeframe(&tf);
                }
                if let Some(p) = r_period {
                    builder = builder.r_period(p);
                }
                if let Some(p) = k_period {
                    builder = builder.k_period(p);
                }
                if let Some(p) = d_period {
                    builder = builder.d_period(p);
                }
                builder.send()
            })
            .await
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Task join error: {}", e)))?;

            match result {
                Ok(kdj) => Python::attach(|py| types::value_to_dict(py, &kdj)),
                Err(e) => Err(errors::to_py_err(e)),
            }
        })
    }

    /// Get MACD (Moving Average Convergence Divergence) data
    ///
    /// Args:
    ///     symbol: Stock symbol (e.g., "2330" for TSMC)
    ///     fast: Fast EMA period (default 12)
    ///     slow: Slow EMA period (default 26)
    ///     signal: Signal line period (default 9)
    ///     from_date: Start date (YYYY-MM-DD)
    ///     to_date: End date (YYYY-MM-DD)
    ///     timeframe: Timeframe ("D", "W", "M", "1", "5", etc.)
    ///
    /// Returns:
    ///     Awaitable[dict]: MACD indicator data with MACD, signal, histogram
    #[pyo3(signature = (symbol, fast=None, slow=None, signal=None, *, from_date=None, to_date=None, timeframe=None, **_extra))]
    pub fn macd_async<'py>(
        &self,
        py: Python<'py>,
        symbol: String,
        fast: Option<u32>,
        slow: Option<u32>,
        signal: Option<u32>,
        from_date: Option<String>,
        to_date: Option<String>,
        timeframe: Option<String>,
        _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Bound<'py, PyAny>> {
        let mut kw = crate::kwargs::Kwargs::parse("stock.technical.macd", &_extra)?;
        let from_date = kw.take_string("from", from_date)?;
        let to_date = kw.take_string("to", to_date)?;
        let timeframe = kw.take_string("timeframe", timeframe)?;
        let fast = kw.take("fast", fast)?;
        let slow = kw.take("slow", slow)?;
        let signal = kw.take("signal", signal)?;
        kw.finish()?;
        let client = self.inner.clone();
        future_into_py(py, async move {
            let result = tokio::task::spawn_blocking(move || {
                let stock = client.stock();
                let technical = stock.technical();
                let mut builder = technical.macd().symbol(&symbol);
                if let Some(f) = from_date {
                    builder = builder.from(&f);
                }
                if let Some(t) = to_date {
                    builder = builder.to(&t);
                }
                if let Some(tf) = timeframe {
                    builder = builder.timeframe(&tf);
                }
                if let Some(fst) = fast {
                    builder = builder.fast(fst);
                }
                if let Some(slw) = slow {
                    builder = builder.slow(slw);
                }
                if let Some(sig) = signal {
                    builder = builder.signal(sig);
                }
                builder.send()
            })
            .await
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Task join error: {}", e)))?;

            match result {
                Ok(macd) => Python::attach(|py| types::value_to_dict(py, &macd)),
                Err(e) => Err(errors::to_py_err(e)),
            }
        })
    }

    /// Get Bollinger Bands (BB) data
    ///
    /// Args:
    ///     symbol: Stock symbol (e.g., "2330" for TSMC)
    ///     period: Moving average period (default 20)
    ///     from_date: Start date (YYYY-MM-DD)
    ///     to_date: End date (YYYY-MM-DD)
    ///     timeframe: Timeframe ("D", "W", "M", "1", "5", etc.)
    ///
    /// Returns:
    ///     Awaitable[dict]: Bollinger Bands data with upper, middle, lower bands
    #[pyo3(signature = (symbol, period=None, *, from_date=None, to_date=None, timeframe=None, **_extra))]
    pub fn bb_async<'py>(
        &self,
        py: Python<'py>,
        symbol: String,
        period: Option<u32>,
        from_date: Option<String>,
        to_date: Option<String>,
        timeframe: Option<String>,
        _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Bound<'py, PyAny>> {
        let mut kw = crate::kwargs::Kwargs::parse("stock.technical.bb", &_extra)?;
        let from_date = kw.take_string("from", from_date)?;
        let to_date = kw.take_string("to", to_date)?;
        let timeframe = kw.take_string("timeframe", timeframe)?;
        let period = kw.take("period", period)?;
        kw.finish()?;
        let client = self.inner.clone();
        future_into_py(py, async move {
            let result = tokio::task::spawn_blocking(move || {
                let stock = client.stock();
                let technical = stock.technical();
                let mut builder = technical.bb().symbol(&symbol);
                if let Some(f) = from_date {
                    builder = builder.from(&f);
                }
                if let Some(t) = to_date {
                    builder = builder.to(&t);
                }
                if let Some(tf) = timeframe {
                    builder = builder.timeframe(&tf);
                }
                if let Some(p) = period {
                    builder = builder.period(p);
                }
                builder.send()
            })
            .await
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Task join error: {}", e)))?;

            match result {
                Ok(bb) => Python::attach(|py| types::value_to_dict(py, &bb)),
                Err(e) => Err(errors::to_py_err(e)),
            }
        })
    }

    /// Sync sibling of `sma()` for legacy fugle-marketdata callers.
    #[pyo3(signature = (symbol, period=None, *, from_date=None, to_date=None, timeframe=None, **_extra))]
    pub fn sma(
        &self,
        py: Python<'_>,
        symbol: String,
        period: Option<u32>,
        from_date: Option<String>,
        to_date: Option<String>,
        timeframe: Option<String>,
        _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Py<pyo3::types::PyDict>> {
        let mut kw = crate::kwargs::Kwargs::parse("stock.technical.sma", &_extra)?;
        let from_date = kw.take_string("from", from_date)?;
        let to_date = kw.take_string("to", to_date)?;
        let timeframe = kw.take_string("timeframe", timeframe)?;
        let period = kw.take("period", period)?;
        kw.finish()?;
        let inner = self.inner.clone();
        let result = py.detach(|| {
            let stock = inner.stock();
            let technical = stock.technical();
            let mut builder = technical.sma().symbol(&symbol);
            if let Some(f) = from_date { builder = builder.from(&f); }
            if let Some(t) = to_date { builder = builder.to(&t); }
            if let Some(tf) = timeframe { builder = builder.timeframe(&tf); }
            if let Some(p) = period { builder = builder.period(p); }
            builder.send()
        });
        match result {
            Ok(sma) => types::value_to_dict(py, &sma),
            Err(e) => Err(errors::to_py_err(e)),
        }
    }

    /// Sync sibling of `rsi()` for legacy fugle-marketdata callers.
    #[pyo3(signature = (symbol, period=None, *, from_date=None, to_date=None, timeframe=None, **_extra))]
    pub fn rsi(
        &self,
        py: Python<'_>,
        symbol: String,
        period: Option<u32>,
        from_date: Option<String>,
        to_date: Option<String>,
        timeframe: Option<String>,
        _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Py<pyo3::types::PyDict>> {
        let mut kw = crate::kwargs::Kwargs::parse("stock.technical.rsi", &_extra)?;
        let from_date = kw.take_string("from", from_date)?;
        let to_date = kw.take_string("to", to_date)?;
        let timeframe = kw.take_string("timeframe", timeframe)?;
        let period = kw.take("period", period)?;
        kw.finish()?;
        let inner = self.inner.clone();
        let result = py.detach(|| {
            let stock = inner.stock();
            let technical = stock.technical();
            let mut builder = technical.rsi().symbol(&symbol);
            if let Some(f) = from_date { builder = builder.from(&f); }
            if let Some(t) = to_date { builder = builder.to(&t); }
            if let Some(tf) = timeframe { builder = builder.timeframe(&tf); }
            if let Some(p) = period { builder = builder.period(p); }
            builder.send()
        });
        match result {
            Ok(rsi) => types::value_to_dict(py, &rsi),
            Err(e) => Err(errors::to_py_err(e)),
        }
    }

    /// Sync sibling of `kdj()` for legacy fugle-marketdata callers.
    #[pyo3(signature = (symbol, r_period=None, k_period=None, d_period=None, *, from_date=None, to_date=None, timeframe=None, **_extra))]
    pub fn kdj(
        &self,
        py: Python<'_>,
        symbol: String,
        r_period: Option<u32>,
        k_period: Option<u32>,
        d_period: Option<u32>,
        from_date: Option<String>,
        to_date: Option<String>,
        timeframe: Option<String>,
        _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Py<pyo3::types::PyDict>> {
        let mut kw = crate::kwargs::Kwargs::parse("stock.technical.kdj", &_extra)?;
        let from_date = kw.take_string("from", from_date)?;
        let to_date = kw.take_string("to", to_date)?;
        let timeframe = kw.take_string("timeframe", timeframe)?;
        let r_period = kw.take("r_period", r_period)?;
        let k_period = kw.take("k_period", k_period)?;
        let d_period = kw.take("d_period", d_period)?;
        kw.finish()?;
        let inner = self.inner.clone();
        let result = py.detach(|| {
            let stock = inner.stock();
            let technical = stock.technical();
            let mut builder = technical.kdj().symbol(&symbol);
            if let Some(f) = from_date { builder = builder.from(&f); }
            if let Some(t) = to_date { builder = builder.to(&t); }
            if let Some(tf) = timeframe { builder = builder.timeframe(&tf); }
            if let Some(p) = r_period { builder = builder.r_period(p); }
            if let Some(p) = k_period { builder = builder.k_period(p); }
            if let Some(p) = d_period { builder = builder.d_period(p); }
            builder.send()
        });
        match result {
            Ok(kdj) => types::value_to_dict(py, &kdj),
            Err(e) => Err(errors::to_py_err(e)),
        }
    }

    /// Sync sibling of `macd()` for legacy fugle-marketdata callers.
    #[pyo3(signature = (symbol, fast=None, slow=None, signal=None, *, from_date=None, to_date=None, timeframe=None, **_extra))]
    pub fn macd(
        &self,
        py: Python<'_>,
        symbol: String,
        fast: Option<u32>,
        slow: Option<u32>,
        signal: Option<u32>,
        from_date: Option<String>,
        to_date: Option<String>,
        timeframe: Option<String>,
        _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Py<pyo3::types::PyDict>> {
        let mut kw = crate::kwargs::Kwargs::parse("stock.technical.macd", &_extra)?;
        let from_date = kw.take_string("from", from_date)?;
        let to_date = kw.take_string("to", to_date)?;
        let timeframe = kw.take_string("timeframe", timeframe)?;
        let fast = kw.take("fast", fast)?;
        let slow = kw.take("slow", slow)?;
        let signal = kw.take("signal", signal)?;
        kw.finish()?;
        let inner = self.inner.clone();
        let result = py.detach(|| {
            let stock = inner.stock();
            let technical = stock.technical();
            let mut builder = technical.macd().symbol(&symbol);
            if let Some(f) = from_date { builder = builder.from(&f); }
            if let Some(t) = to_date { builder = builder.to(&t); }
            if let Some(tf) = timeframe { builder = builder.timeframe(&tf); }
            if let Some(fst) = fast { builder = builder.fast(fst); }
            if let Some(slw) = slow { builder = builder.slow(slw); }
            if let Some(sig) = signal { builder = builder.signal(sig); }
            builder.send()
        });
        match result {
            Ok(macd) => types::value_to_dict(py, &macd),
            Err(e) => Err(errors::to_py_err(e)),
        }
    }

    /// Sync sibling of `bb()` for legacy fugle-marketdata callers.
    #[pyo3(signature = (symbol, period=None, *, from_date=None, to_date=None, timeframe=None, **_extra))]
    pub fn bb(
        &self,
        py: Python<'_>,
        symbol: String,
        period: Option<u32>,
        from_date: Option<String>,
        to_date: Option<String>,
        timeframe: Option<String>,
        _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Py<pyo3::types::PyDict>> {
        let mut kw = crate::kwargs::Kwargs::parse("stock.technical.bb", &_extra)?;
        let from_date = kw.take_string("from", from_date)?;
        let to_date = kw.take_string("to", to_date)?;
        let timeframe = kw.take_string("timeframe", timeframe)?;
        let period = kw.take("period", period)?;
        kw.finish()?;
        let inner = self.inner.clone();
        let result = py.detach(|| {
            let stock = inner.stock();
            let technical = stock.technical();
            let mut builder = technical.bb().symbol(&symbol);
            if let Some(f) = from_date { builder = builder.from(&f); }
            if let Some(t) = to_date { builder = builder.to(&t); }
            if let Some(tf) = timeframe { builder = builder.timeframe(&tf); }
            if let Some(p) = period { builder = builder.period(p); }
            builder.send()
        });
        match result {
            Ok(bb) => types::value_to_dict(py, &bb),
            Err(e) => Err(errors::to_py_err(e)),
        }
    }
}

/// Stock corporate actions endpoints client
///
/// Access via `client.stock.corporate_actions`
#[pyclass]
pub struct StockCorporateActionsClient {
    inner: marketdata_core::RestClient,
}

#[pymethods]
impl StockCorporateActionsClient {
    /// Get capital changes (stock splits, rights issues, etc.)
    ///
    /// Args:
    ///     start_date: Start date for range query (YYYY-MM-DD)
    ///     end_date: End date for range query (YYYY-MM-DD)
    ///
    /// Returns:
    ///     Awaitable[dict]: Capital changes data
    ///
    /// Example:
    ///     ```python
    ///     changes = await client.stock.corporate_actions.capital_changes_async(
    ///         start_date="2024-01-01",
    ///         end_date="2024-01-31"
    ///     )
    ///     ```
    #[pyo3(signature = (*, start_date=None, end_date=None, sort=None, **_extra))]
    pub fn capital_changes_async<'py>(
        &self,
        py: Python<'py>,
        start_date: Option<String>,
        end_date: Option<String>,sort: Option<String>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Bound<'py, PyAny>> {
        let mut kw = crate::kwargs::Kwargs::parse("stock.corporate_actions.capital_changes", &_extra)?;
        let start_date = kw.take_string("start_date", start_date)?;
        let end_date = kw.take_string("end_date", end_date)?;
        let sort = kw.take_string("sort", sort)?;
        kw.finish()?;
        let client = self.inner.clone();
        future_into_py(py, async move {
            let result = tokio::task::spawn_blocking(move || {
                let stock = client.stock();
                let corp = stock.corporate_actions();
                let mut builder = corp.capital_changes();
                if let Some(sd) = start_date {
                    builder = builder.start_date(&sd);
                }
                if let Some(ed) = end_date {
                    builder = builder.end_date(&ed);
                }
                if let Some(v) = &sort {
                    builder = builder.sort(v);
                }
                builder.send()
            })
            .await
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Task join error: {}", e)))?;

            match result {
                Ok(changes) => Python::attach(|py| types::value_to_dict(py, &changes)),
                Err(e) => Err(errors::to_py_err(e)),
            }
        })
    }

    /// Get dividend announcements
    ///
    /// Args:
    ///     start_date: Start date for range query (YYYY-MM-DD)
    ///     end_date: End date for range query (YYYY-MM-DD)
    ///     sort: Sort order, "asc" or "desc"
    ///
    /// Returns:
    ///     Awaitable[dict]: Dividend data
    ///
    /// Example:
    ///     ```python
    ///     dividends = await client.stock.corporate_actions.dividends_async(
    ///         start_date="2024-01-01",
    ///         end_date="2024-12-31"
    ///     )
    ///     ```
    #[pyo3(signature = (*, start_date=None, end_date=None, exchange=None, sort=None, **_extra))]
    #[allow(clippy::too_many_arguments, reason = "mirrors the Python keyword signature")]
    pub fn dividends_async<'py>(
        &self,
        py: Python<'py>,
        start_date: Option<String>,
        end_date: Option<String>,exchange: Option<String>, sort: Option<String>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Bound<'py, PyAny>> {
        let mut kw = crate::kwargs::Kwargs::parse("stock.corporate_actions.dividends", &_extra)?;
        let start_date = kw.take_string("start_date", start_date)?;
        let end_date = kw.take_string("end_date", end_date)?;
        let exchange = kw.take_string("exchange", exchange)?;
        let sort = kw.take_string("sort", sort)?;
        kw.finish()?;
        let client = self.inner.clone();
        future_into_py(py, async move {
            let result = tokio::task::spawn_blocking(move || {
                let stock = client.stock();
                let corp = stock.corporate_actions();
                let mut builder = corp.dividends();
                if let Some(sd) = start_date {
                    builder = builder.start_date(&sd);
                }
                if let Some(ed) = end_date {
                    builder = builder.end_date(&ed);
                }
                if let Some(v) = &exchange {
                    builder = builder.exchange(v);
                }
                if let Some(v) = &sort {
                    builder = builder.sort(v);
                }
                builder.send()
            })
            .await
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Task join error: {}", e)))?;

            match result {
                Ok(dividends) => Python::attach(|py| types::value_to_dict(py, &dividends)),
                Err(e) => Err(errors::to_py_err(e)),
            }
        })
    }

    /// Get IPO listing applicants
    ///
    /// Args:
    ///     start_date: Start date for range query (YYYY-MM-DD)
    ///     end_date: End date for range query (YYYY-MM-DD)
    ///     exchange: Exchange filter, "TWSE" or "TPEx"
    ///     sort: Sort order, "asc" or "desc"
    ///
    /// Returns:
    ///     Awaitable[dict]: Listing applicants data
    ///
    /// Example:
    ///     ```python
    ///     applicants = await client.stock.corporate_actions.listing_applicants_async()
    ///     ```
    #[pyo3(signature = (*, start_date=None, end_date=None, exchange=None, sort=None, **_extra))]
    #[allow(clippy::too_many_arguments, reason = "mirrors the Python keyword signature")]
    pub fn listing_applicants_async<'py>(
        &self,
        py: Python<'py>,
        start_date: Option<String>,
        end_date: Option<String>,exchange: Option<String>, sort: Option<String>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Bound<'py, PyAny>> {
        let mut kw = crate::kwargs::Kwargs::parse("stock.corporate_actions.listing_applicants", &_extra)?;
        let start_date = kw.take_string("start_date", start_date)?;
        let end_date = kw.take_string("end_date", end_date)?;
        let exchange = kw.take_string("exchange", exchange)?;
        let sort = kw.take_string("sort", sort)?;
        kw.finish()?;
        let client = self.inner.clone();
        future_into_py(py, async move {
            let result = tokio::task::spawn_blocking(move || {
                let stock = client.stock();
                let corp = stock.corporate_actions();
                let mut builder = corp.listing_applicants();
                if let Some(sd) = start_date {
                    builder = builder.start_date(&sd);
                }
                if let Some(ed) = end_date {
                    builder = builder.end_date(&ed);
                }
                if let Some(v) = &exchange {
                    builder = builder.exchange(v);
                }
                if let Some(v) = &sort {
                    builder = builder.sort(v);
                }
                builder.send()
            })
            .await
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Task join error: {}", e)))?;

            match result {
                Ok(applicants) => Python::attach(|py| types::value_to_dict(py, &applicants)),
                Err(e) => Err(errors::to_py_err(e)),
            }
        })
    }

    /// Sync sibling of `capital_changes()` for legacy fugle-marketdata callers.
    #[pyo3(signature = (*, start_date=None, end_date=None, sort=None, **_extra))]
    pub fn capital_changes(
        &self,
        py: Python<'_>,
        start_date: Option<String>,
        end_date: Option<String>,sort: Option<String>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Py<pyo3::types::PyDict>> {
        let mut kw = crate::kwargs::Kwargs::parse("stock.corporate_actions.capital_changes", &_extra)?;
        let start_date = kw.take_string("start_date", start_date)?;
        let end_date = kw.take_string("end_date", end_date)?;
        let sort = kw.take_string("sort", sort)?;
        kw.finish()?;
        let inner = self.inner.clone();
        let result = py.detach(|| {
            let stock = inner.stock();
            let corp = stock.corporate_actions();
            let mut builder = corp.capital_changes();
            if let Some(sd) = start_date { builder = builder.start_date(&sd); }
            if let Some(ed) = end_date { builder = builder.end_date(&ed); }
            if let Some(v) = &sort {
                builder = builder.sort(v);
            }
            builder.send()
        });
        match result {
            Ok(changes) => types::value_to_dict(py, &changes),
            Err(e) => Err(errors::to_py_err(e)),
        }
    }

    /// Sync sibling of `dividends()` for legacy fugle-marketdata callers.
    #[pyo3(signature = (*, start_date=None, end_date=None, exchange=None, sort=None, **_extra))]
    #[allow(clippy::too_many_arguments, reason = "mirrors the Python keyword signature")]
    pub fn dividends(
        &self,
        py: Python<'_>,
        start_date: Option<String>,
        end_date: Option<String>,exchange: Option<String>, sort: Option<String>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Py<pyo3::types::PyDict>> {
        let mut kw = crate::kwargs::Kwargs::parse("stock.corporate_actions.dividends", &_extra)?;
        let start_date = kw.take_string("start_date", start_date)?;
        let end_date = kw.take_string("end_date", end_date)?;
        let exchange = kw.take_string("exchange", exchange)?;
        let sort = kw.take_string("sort", sort)?;
        kw.finish()?;
        let inner = self.inner.clone();
        let result = py.detach(|| {
            let stock = inner.stock();
            let corp = stock.corporate_actions();
            let mut builder = corp.dividends();
            if let Some(sd) = start_date { builder = builder.start_date(&sd); }
            if let Some(ed) = end_date { builder = builder.end_date(&ed); }
            if let Some(v) = &exchange {
                builder = builder.exchange(v);
            }
            if let Some(v) = &sort {
                builder = builder.sort(v);
            }
            builder.send()
        });
        match result {
            Ok(dividends) => types::value_to_dict(py, &dividends),
            Err(e) => Err(errors::to_py_err(e)),
        }
    }

    /// Sync sibling of `listing_applicants()` for legacy fugle-marketdata callers.
    #[pyo3(signature = (*, start_date=None, end_date=None, exchange=None, sort=None, **_extra))]
    #[allow(clippy::too_many_arguments, reason = "mirrors the Python keyword signature")]
    pub fn listing_applicants(
        &self,
        py: Python<'_>,
        start_date: Option<String>,
        end_date: Option<String>,exchange: Option<String>, sort: Option<String>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Py<pyo3::types::PyDict>> {
        let mut kw = crate::kwargs::Kwargs::parse("stock.corporate_actions.listing_applicants", &_extra)?;
        let start_date = kw.take_string("start_date", start_date)?;
        let end_date = kw.take_string("end_date", end_date)?;
        let exchange = kw.take_string("exchange", exchange)?;
        let sort = kw.take_string("sort", sort)?;
        kw.finish()?;
        let inner = self.inner.clone();
        let result = py.detach(|| {
            let stock = inner.stock();
            let corp = stock.corporate_actions();
            let mut builder = corp.listing_applicants();
            if let Some(sd) = start_date { builder = builder.start_date(&sd); }
            if let Some(ed) = end_date { builder = builder.end_date(&ed); }
            if let Some(v) = &exchange {
                builder = builder.exchange(v);
            }
            if let Some(v) = &sort {
                builder = builder.sort(v);
            }
            builder.send()
        });
        match result {
            Ok(applicants) => types::value_to_dict(py, &applicants),
            Err(e) => Err(errors::to_py_err(e)),
        }
    }
}

/// Futures and options market data client
///
/// Access via `client.futopt`
#[pyclass]
pub struct FutOptClient {
    inner: marketdata_core::RestClient,
}

#[pymethods]
impl FutOptClient {
    /// Access intraday (real-time) FutOpt endpoints
    ///
    /// Returns:
    ///     FutOptIntradayClient for accessing intraday endpoints
    #[getter]
    pub fn intraday(&self) -> FutOptIntradayClient {
        FutOptIntradayClient {
            inner: self.inner.clone(),
        }
    }

    /// Access historical FutOpt data endpoints
    ///
    /// Returns:
    ///     FutOptHistoricalClient for accessing historical endpoints
    #[getter]
    pub fn historical(&self) -> FutOptHistoricalClient {
        FutOptHistoricalClient {
            inner: self.inner.clone(),
        }
    }
}

/// FutOpt intraday (real-time) endpoints client
///
/// Access via `client.futopt.intraday`
#[pyclass]
pub struct FutOptIntradayClient {
    inner: marketdata_core::RestClient,
}

#[pymethods]
impl FutOptIntradayClient {
    /// Get intraday quote for a futures/options contract
    ///
    /// Args:
    ///     symbol: Contract symbol (e.g., "TXFC4" for TAIEX futures)
    ///     after_hours: Whether to query after-hours session data (default: False)
    ///
    /// Returns:
    ///     Awaitable[dict]: Quote data including prices, order book, and trading info
    ///
    /// Raises:
    ///     MarketDataError: If the request fails
    ///
    /// Example:
    ///     ```python
    ///     # Regular session
    ///     quote = await client.futopt.intraday.quote_async("TXFC4")
    ///
    ///     # After-hours session
    ///     ah_quote = await client.futopt.intraday.quote_async("TXFC4", after_hours=True)
    ///     ```
    #[pyo3(signature = (symbol, *, after_hours=None, **_extra))]
    pub fn quote_async<'py>(&self, py: Python<'py>, symbol: String, after_hours: Option<bool>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Bound<'py, PyAny>> {
        let mut kw = crate::kwargs::Kwargs::parse("futopt.intraday.quote", &_extra)?;
        let after_hours = kw.take_flag("after_hours", after_hours)?;
        kw.finish()?;
        let client = self.inner.clone();
        future_into_py(py, async move {
            let result = tokio::task::spawn_blocking(move || {
                let futopt = client.futopt();
                let intraday = futopt.intraday();
                let mut builder = intraday.quote().symbol(&symbol);
                if after_hours == Some(true) {
                    builder = builder.after_hours();
                }
                builder.send()
            }).await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Task join error: {}", e)))?;

            match result {
                Ok(quote) => Python::attach(|py| types::value_to_dict(py, &quote)),
                Err(e) => Err(errors::to_py_err(e)),
            }
        })
    }

    /// Get batch ticker list for a FutOpt contract type
    ///
    /// Args:
    ///     type: Contract type ("FUTURE" or "OPTION")
    ///     exchange: Exchange filter (e.g., "TAIFEX")
    ///     after_hours: Query after-hours session data
    ///     contract_type: Contract type code ("I", "R", "B", "C", "S", "E")
    ///
    /// Returns:
    ///     Awaitable[list[dict]]: List of FutOpt ticker info dicts
    ///
    /// Example:
    ///     ```python
    ///     tickers = await client.futopt.intraday.tickers_async(type="FUTURE")
    ///     ```
    #[pyo3(signature = (r#type, *, exchange=None, after_hours=None, contract_type=None, is_spread=None, product=None, **_extra))]
    pub fn tickers_async<'py>(
        &self,
        py: Python<'py>,
        r#type: String,
        exchange: Option<String>,
        after_hours: Option<bool>,
        contract_type: Option<String>,
        is_spread: Option<bool>,product: Option<String>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Bound<'py, PyAny>> {
        let mut kw = crate::kwargs::Kwargs::parse("futopt.intraday.tickers", &_extra)?;
        let exchange = kw.take_string("exchange", exchange)?;
        let after_hours = kw.take_flag("after_hours", after_hours)?;
        let contract_type = kw.take_string("contract_type", contract_type)?;
        let is_spread = kw.take("is_spread", is_spread)?;
        let product = kw.take_string("product", product)?;
        kw.finish()?;
        let client = self.inner.clone();
        future_into_py(py, async move {
            let typ = parse_futopt_type(&r#type)?;
            let ct = match contract_type.as_deref() {
                Some(s) => Some(parse_contract_type(s)?),
                None => None,
            };
            let result = tokio::task::spawn_blocking(move || {
                let futopt = client.futopt();
                let intraday = futopt.intraday();
                let mut builder = intraday.tickers().typ(typ);
                if let Some(e) = &exchange {
                    builder = builder.exchange(e);
                }
                if after_hours == Some(true) {
                    builder = builder.after_hours();
                }
                if let Some(c) = ct {
                    builder = builder.contract_type(c);
                }
                if let Some(sp) = is_spread {
                    builder = builder.is_spread(sp);
                }
                if let Some(v) = &product {
                    builder = builder.product(v);
                }
                builder.send()
            }).await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Task join error: {}", e)))?;

            match result {
                Ok(tickers) => Python::attach(|py| {
                    let json_val = serde_json::to_value(&tickers)
                        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Serialization error: {}", e)))?;
                    types::json_value_to_py(py, &json_val)
                }),
                Err(e) => Err(errors::to_py_err(e)),
            }
        })
    }

    /// Get available FutOpt products list
    ///
    /// Args:
    ///     type: Contract type ("FUTURE" or "OPTION")
    ///     contract_type: Contract type code ("I", "R", "B", "C", "S", "E")
    ///     product: Only contracts of this product (e.g. "TXF")
    ///
    /// Returns:
    ///     Awaitable[list[dict]]: List of product info dicts
    ///
    /// Example:
    ///     ```python
    ///     products = await client.futopt.intraday.products_async(type="FUTURE")
    ///     ```
    #[pyo3(signature = (r#type, *, contract_type=None, exchange=None, after_hours=None, status=None, **_extra))]
    #[allow(clippy::too_many_arguments, reason = "mirrors the Python keyword signature")]
    pub fn products_async<'py>(
        &self,
        py: Python<'py>,
        r#type: String,
        contract_type: Option<String>,exchange: Option<String>, after_hours: Option<bool>, status: Option<String>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Bound<'py, PyAny>> {
        let mut kw = crate::kwargs::Kwargs::parse("futopt.intraday.products", &_extra)?;
        let contract_type = kw.take_string("contract_type", contract_type)?;
        let exchange = kw.take_string("exchange", exchange)?;
        let after_hours = kw.take_flag("after_hours", after_hours)?;
        let status = kw.take_string("status", status)?;
        kw.finish()?;
        let client = self.inner.clone();
        future_into_py(py, async move {
            let typ = parse_futopt_type(&r#type)?;
            let ct = match contract_type.as_deref() {
                Some(s) => Some(parse_contract_type(s)?),
                None => None,
            };
            let result = tokio::task::spawn_blocking(move || {
                let futopt = client.futopt();
                let intraday = futopt.intraday();
                let mut builder = intraday.products().typ(typ);
                if let Some(c) = ct {
                    builder = builder.contract_type(c);
                }
                if let Some(v) = &exchange {
                    builder = builder.exchange(v);
                }
                if after_hours == Some(true) {
                    builder = builder.after_hours();
                }
                if let Some(v) = &status {
                    builder = builder.status(v);
                }
                builder.send()
            }).await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Task join error: {}", e)))?;

            match result {
                Ok(products) => Python::attach(|py| {
                    let json_val = serde_json::to_value(&products)
                        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Serialization error: {}", e)))?;
                    types::json_value_to_py(py, &json_val)
                }),
                Err(e) => Err(errors::to_py_err(e)),
            }
        })
    }

    /// Sync sibling of `quote()` for legacy fugle-marketdata callers.
    #[pyo3(signature = (symbol, *, after_hours=None, **_extra))]
    pub fn quote(&self, py: Python<'_>, symbol: String, after_hours: Option<bool>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Py<pyo3::types::PyDict>> {
        let mut kw = crate::kwargs::Kwargs::parse("futopt.intraday.quote", &_extra)?;
        let after_hours = kw.take_flag("after_hours", after_hours)?;
        kw.finish()?;
        let inner = self.inner.clone();
        let result = py.detach(|| {
            let futopt = inner.futopt();
            let intraday = futopt.intraday();
            let mut builder = intraday.quote().symbol(&symbol);
            if after_hours == Some(true) {
                builder = builder.after_hours();
            }
            builder.send()
        });
        match result {
            Ok(quote) => types::value_to_dict(py, &quote),
            Err(e) => Err(errors::to_py_err(e)),
        }
    }

    /// Sync sibling of `tickers()` for legacy fugle-marketdata callers.
    #[pyo3(signature = (r#type, *, exchange=None, after_hours=None, contract_type=None, is_spread=None, product=None, **_extra))]
    pub fn tickers(
        &self,
        py: Python<'_>,
        r#type: String,
        exchange: Option<String>,
        after_hours: Option<bool>,
        contract_type: Option<String>,
        is_spread: Option<bool>,product: Option<String>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Py<PyAny>> {
        let mut kw = crate::kwargs::Kwargs::parse("futopt.intraday.tickers", &_extra)?;
        let exchange = kw.take_string("exchange", exchange)?;
        let after_hours = kw.take_flag("after_hours", after_hours)?;
        let contract_type = kw.take_string("contract_type", contract_type)?;
        let is_spread = kw.take("is_spread", is_spread)?;
        let product = kw.take_string("product", product)?;
        kw.finish()?;
        let typ = parse_futopt_type(&r#type)?;
        let ct = match contract_type.as_deref() {
            Some(s) => Some(parse_contract_type(s)?),
            None => None,
        };
        let inner = self.inner.clone();
        let result = py.detach(|| {
            let futopt = inner.futopt();
            let intraday = futopt.intraday();
            let mut builder = intraday.tickers().typ(typ);
            if let Some(e) = &exchange {
                builder = builder.exchange(e);
            }
            if after_hours == Some(true) {
                builder = builder.after_hours();
            }
            if let Some(c) = ct {
                builder = builder.contract_type(c);
            }
            if let Some(sp) = is_spread {
                builder = builder.is_spread(sp);
            }
            if let Some(v) = &product {
                builder = builder.product(v);
            }
            builder.send()
        });
        match result {
            Ok(tickers) => {
                let json_val = serde_json::to_value(&tickers)
                    .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Serialization error: {}", e)))?;
                types::json_value_to_py(py, &json_val)
            }
            Err(e) => Err(errors::to_py_err(e)),
        }
    }

    /// Sync sibling of `products()` for legacy fugle-marketdata callers.
    #[pyo3(signature = (r#type, *, contract_type=None, exchange=None, after_hours=None, status=None, **_extra))]
    #[allow(clippy::too_many_arguments, reason = "mirrors the Python keyword signature")]
    pub fn products(
        &self,
        py: Python<'_>,
        r#type: String,
        contract_type: Option<String>,exchange: Option<String>, after_hours: Option<bool>, status: Option<String>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Py<PyAny>> {
        let mut kw = crate::kwargs::Kwargs::parse("futopt.intraday.products", &_extra)?;
        let contract_type = kw.take_string("contract_type", contract_type)?;
        let exchange = kw.take_string("exchange", exchange)?;
        let after_hours = kw.take_flag("after_hours", after_hours)?;
        let status = kw.take_string("status", status)?;
        kw.finish()?;
        let typ = parse_futopt_type(&r#type)?;
        let ct = match contract_type.as_deref() {
            Some(s) => Some(parse_contract_type(s)?),
            None => None,
        };
        let inner = self.inner.clone();
        let result = py.detach(|| {
            let futopt = inner.futopt();
            let intraday = futopt.intraday();
            let mut builder = intraday.products().typ(typ);
            if let Some(c) = ct {
                builder = builder.contract_type(c);
            }
            if let Some(v) = &exchange {
                builder = builder.exchange(v);
            }
            if after_hours == Some(true) {
                builder = builder.after_hours();
            }
            if let Some(v) = &status {
                builder = builder.status(v);
            }
            builder.send()
        });
        match result {
            Ok(products) => {
                let json_val = serde_json::to_value(&products)
                    .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Serialization error: {}", e)))?;
                types::json_value_to_py(py, &json_val)
            }
            Err(e) => Err(errors::to_py_err(e)),
        }
    }

    // ============================================================
    // Methods B4 — drop-in parity with fugle-marketdata 2.4.1
    //
    // Official 2.4.1 exposes: futopt.intraday.{ticker, candles, trades, volumes}
    // via **params. We expose typed sync + async pairs; the **kwargs
    // forwarding layer (B2) will sit on top in a later commit.
    // ============================================================

    /// Get intraday ticker for a FutOpt contract
    #[pyo3(signature = (symbol, *, after_hours=None, **_extra))]
    pub fn ticker_async<'py>(
        &self,
        py: Python<'py>,
        symbol: String,
        after_hours: Option<bool>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Bound<'py, PyAny>> {
        let mut kw = crate::kwargs::Kwargs::parse("futopt.intraday.ticker", &_extra)?;
        let after_hours = kw.take_flag("after_hours", after_hours)?;
        kw.finish()?;
        let client = self.inner.clone();
        future_into_py(py, async move {
            let result = tokio::task::spawn_blocking(move || {
                let futopt = client.futopt();
                let intraday = futopt.intraday();
                let mut builder = intraday.ticker().symbol(&symbol);
                if after_hours == Some(true) {
                    builder = builder.after_hours();
                }
                builder.send()
            })
            .await
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Task join error: {}", e)))?;

            match result {
                Ok(ticker) => Python::attach(|py| {
                    let json_val = serde_json::to_value(&ticker).map_err(|e| {
                        pyo3::exceptions::PyValueError::new_err(format!("Serialization error: {}", e))
                    })?;
                    types::json_value_to_py(py, &json_val)
                }),
                Err(e) => Err(errors::to_py_err(e)),
            }
        })
    }

    /// Sync sibling of `ticker()` for legacy fugle-marketdata callers.
    #[pyo3(signature = (symbol, *, after_hours=None, **_extra))]
    pub fn ticker(
        &self,
        py: Python<'_>,
        symbol: String,
        after_hours: Option<bool>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Py<PyAny>> {
        let mut kw = crate::kwargs::Kwargs::parse("futopt.intraday.ticker", &_extra)?;
        let after_hours = kw.take_flag("after_hours", after_hours)?;
        kw.finish()?;
        let inner = self.inner.clone();
        let result = py.detach(|| {
            let futopt = inner.futopt();
            let intraday = futopt.intraday();
            let mut builder = intraday.ticker().symbol(&symbol);
            if after_hours == Some(true) {
                builder = builder.after_hours();
            }
            builder.send()
        });
        match result {
            Ok(ticker) => {
                let json_val = serde_json::to_value(&ticker).map_err(|e| {
                    pyo3::exceptions::PyValueError::new_err(format!("Serialization error: {}", e))
                })?;
                types::json_value_to_py(py, &json_val)
            }
            Err(e) => Err(errors::to_py_err(e)),
        }
    }

    /// Get intraday candles for a FutOpt contract
    #[pyo3(signature = (symbol, *, timeframe=None, after_hours=None, **_extra))]
    pub fn candles_async<'py>(
        &self,
        py: Python<'py>,
        symbol: String,
        timeframe: Option<String>,
        after_hours: Option<bool>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Bound<'py, PyAny>> {
        let mut kw = crate::kwargs::Kwargs::parse("futopt.intraday.candles", &_extra)?;
        let after_hours = kw.take_flag("after_hours", after_hours)?;
        kw.finish()?;
        let client = self.inner.clone();
        future_into_py(py, async move {
            let result = tokio::task::spawn_blocking(move || {
                let futopt = client.futopt();
                let intraday = futopt.intraday();
                let mut builder = intraday.candles().symbol(&symbol);
                if let Some(tf) = &timeframe {
                    builder = builder.timeframe(tf);
                }
                if after_hours == Some(true) {
                    builder = builder.after_hours();
                }
                builder.send()
            })
            .await
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Task join error: {}", e)))?;

            match result {
                Ok(candles) => Python::attach(|py| types::value_to_dict(py, &candles)),
                Err(e) => Err(errors::to_py_err(e)),
            }
        })
    }

    /// Sync sibling of `candles()` for legacy fugle-marketdata callers.
    #[pyo3(signature = (symbol, *, timeframe=None, after_hours=None, **_extra))]
    pub fn candles(
        &self,
        py: Python<'_>,
        symbol: String,
        timeframe: Option<String>,
        after_hours: Option<bool>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Py<pyo3::types::PyDict>> {
        let mut kw = crate::kwargs::Kwargs::parse("futopt.intraday.candles", &_extra)?;
        let after_hours = kw.take_flag("after_hours", after_hours)?;
        kw.finish()?;
        let inner = self.inner.clone();
        let result = py.detach(|| {
            let futopt = inner.futopt();
            let intraday = futopt.intraday();
            let mut builder = intraday.candles().symbol(&symbol);
            if let Some(tf) = &timeframe {
                builder = builder.timeframe(tf);
            }
            if after_hours == Some(true) {
                builder = builder.after_hours();
            }
            builder.send()
        });
        match result {
            Ok(candles) => types::value_to_dict(py, &candles),
            Err(e) => Err(errors::to_py_err(e)),
        }
    }

    /// Get intraday trades for a FutOpt contract
    #[pyo3(signature = (symbol, *, after_hours=None, offset=None, limit=None, is_trial=None, **_extra))]
    pub fn trades_async<'py>(
        &self,
        py: Python<'py>,
        symbol: String,
        after_hours: Option<bool>,
        offset: Option<i32>,
        limit: Option<i32>,
        is_trial: Option<bool>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Bound<'py, PyAny>> {
        let mut kw = crate::kwargs::Kwargs::parse("futopt.intraday.trades", &_extra)?;
        let after_hours = kw.take_flag("after_hours", after_hours)?;
        let offset = kw.take("offset", offset)?;
        let limit = kw.take("limit", limit)?;
        let is_trial = kw.take("is_trial", is_trial)?;
        kw.finish()?;
        let client = self.inner.clone();
        future_into_py(py, async move {
            let result = tokio::task::spawn_blocking(move || {
                let futopt = client.futopt();
                let intraday = futopt.intraday();
                let mut builder = intraday.trades().symbol(&symbol);
                if after_hours == Some(true) {
                    builder = builder.after_hours();
                }
                if let Some(o) = offset {
                    builder = builder.offset(o);
                }
                if let Some(l) = limit {
                    builder = builder.limit(l);
                }
                if let Some(t) = is_trial {
                    builder = builder.is_trial(t);
                }
                builder.send()
            })
            .await
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Task join error: {}", e)))?;

            match result {
                Ok(trades) => Python::attach(|py| types::value_to_dict(py, &trades)),
                Err(e) => Err(errors::to_py_err(e)),
            }
        })
    }

    /// Sync sibling of `trades()` for legacy fugle-marketdata callers.
    #[pyo3(signature = (symbol, *, after_hours=None, offset=None, limit=None, is_trial=None, **_extra))]
    pub fn trades(
        &self,
        py: Python<'_>,
        symbol: String,
        after_hours: Option<bool>,
        offset: Option<i32>,
        limit: Option<i32>,
        is_trial: Option<bool>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Py<pyo3::types::PyDict>> {
        let mut kw = crate::kwargs::Kwargs::parse("futopt.intraday.trades", &_extra)?;
        let after_hours = kw.take_flag("after_hours", after_hours)?;
        let offset = kw.take("offset", offset)?;
        let limit = kw.take("limit", limit)?;
        let is_trial = kw.take("is_trial", is_trial)?;
        kw.finish()?;
        let inner = self.inner.clone();
        let result = py.detach(|| {
            let futopt = inner.futopt();
            let intraday = futopt.intraday();
            let mut builder = intraday.trades().symbol(&symbol);
            if after_hours == Some(true) {
                builder = builder.after_hours();
            }
            if let Some(o) = offset {
                builder = builder.offset(o);
            }
            if let Some(l) = limit {
                builder = builder.limit(l);
            }
            if let Some(t) = is_trial {
                builder = builder.is_trial(t);
            }
            builder.send()
        });
        match result {
            Ok(trades) => types::value_to_dict(py, &trades),
            Err(e) => Err(errors::to_py_err(e)),
        }
    }

    /// Get intraday volumes for a FutOpt contract
    #[pyo3(signature = (symbol, *, after_hours=None, **_extra))]
    pub fn volumes_async<'py>(
        &self,
        py: Python<'py>,
        symbol: String,
        after_hours: Option<bool>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Bound<'py, PyAny>> {
        let mut kw = crate::kwargs::Kwargs::parse("futopt.intraday.volumes", &_extra)?;
        let after_hours = kw.take_flag("after_hours", after_hours)?;
        kw.finish()?;
        let client = self.inner.clone();
        future_into_py(py, async move {
            let result = tokio::task::spawn_blocking(move || {
                let futopt = client.futopt();
                let intraday = futopt.intraday();
                let mut builder = intraday.volumes().symbol(&symbol);
                if after_hours == Some(true) {
                    builder = builder.after_hours();
                }
                builder.send()
            })
            .await
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Task join error: {}", e)))?;

            match result {
                Ok(volumes) => Python::attach(|py| types::value_to_dict(py, &volumes)),
                Err(e) => Err(errors::to_py_err(e)),
            }
        })
    }

    /// Sync sibling of `volumes()` for legacy fugle-marketdata callers.
    #[pyo3(signature = (symbol, *, after_hours=None, **_extra))]
    pub fn volumes(
        &self,
        py: Python<'_>,
        symbol: String,
        after_hours: Option<bool>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Py<pyo3::types::PyDict>> {
        let mut kw = crate::kwargs::Kwargs::parse("futopt.intraday.volumes", &_extra)?;
        let after_hours = kw.take_flag("after_hours", after_hours)?;
        kw.finish()?;
        let inner = self.inner.clone();
        let result = py.detach(|| {
            let futopt = inner.futopt();
            let intraday = futopt.intraday();
            let mut builder = intraday.volumes().symbol(&symbol);
            if after_hours == Some(true) {
                builder = builder.after_hours();
            }
            builder.send()
        });
        match result {
            Ok(volumes) => types::value_to_dict(py, &volumes),
            Err(e) => Err(errors::to_py_err(e)),
        }
    }
}


fn parse_futopt_type(s: &str) -> PyResult<marketdata_core::models::futopt::FutOptType> {
    use marketdata_core::models::futopt::FutOptType;
    match s.to_ascii_uppercase().as_str() {
        "FUTURE" | "FUTURES" => Ok(FutOptType::Future),
        "OPTION" | "OPTIONS" => Ok(FutOptType::Option),
        other => Err(pyo3::exceptions::PyValueError::new_err(format!(
            "type must be 'FUTURE' or 'OPTION', got '{}'",
            other
        ))),
    }
}

fn parse_contract_type(s: &str) -> PyResult<marketdata_core::models::futopt::ContractType> {
    use marketdata_core::models::futopt::ContractType;
    match s.to_ascii_uppercase().as_str() {
        "I" | "INDEX" => Ok(ContractType::Index),
        "R" | "RATE" => Ok(ContractType::Rate),
        "B" | "BOND" => Ok(ContractType::Bond),
        "C" | "CURRENCY" => Ok(ContractType::Currency),
        "S" | "STOCK" => Ok(ContractType::Stock),
        "E" | "ETF" => Ok(ContractType::Etf),
        other => Err(pyo3::exceptions::PyValueError::new_err(format!(
            "contract_type must be one of I/R/B/C/S/E, got '{}'",
            other
        ))),
    }
}

/// FutOpt historical data endpoints client
///
/// Access via `client.futopt.historical`
#[pyclass]
pub struct FutOptHistoricalClient {
    inner: marketdata_core::RestClient,
}

#[pymethods]
impl FutOptHistoricalClient {
    /// Get historical candles for a FutOpt product
    ///
    /// Args:
    ///     symbol: Product code (e.g., "TXF"); a contract code such as "TXFC4" returns 404
    ///     from_date: Start date (YYYY-MM-DD)
    ///     to_date: End date (YYYY-MM-DD)
    ///     timeframe: Timeframe ("D", "W", "M", "1", "5", "10", "15", "30", "60")
    ///     after_hours: Query the after-hours session (default: False)
    ///     contract_month: "YYYYMM", or a continuous contract: "1!" (server default), "2!", "3!"
    ///     fields: Comma-separated fields, e.g. "open,high,low,close,volume"
    ///     sort: "asc" or "desc"
    ///
    /// Returns:
    ///     Awaitable[dict]: Historical candles data
    ///
    /// Example:
    ///     ```python
    ///     candles = await client.futopt.historical.candles_async(
    ///         "TXF",
    ///         contract_month="202609",
    ///         from_date="2026-09-01",
    ///         to_date="2026-09-15",
    ///         timeframe="D"
    ///     )
    ///     ```
    #[pyo3(signature = (symbol, *, from_date=None, to_date=None, timeframe=None, after_hours=None, contract_month=None, fields=None, sort=None, strike_price=None, call_put=None, **_extra))]
    #[allow(clippy::too_many_arguments, reason = "mirrors the Python keyword signature")]
    pub fn candles_async<'py>(
        &self,
        py: Python<'py>,
        symbol: String,
        from_date: Option<String>,
        to_date: Option<String>,
        timeframe: Option<String>,
        after_hours: Option<bool>,
        contract_month: Option<String>,
        fields: Option<String>,
        sort: Option<String>,
        strike_price: Option<f64>,
        call_put: Option<String>,
        _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Bound<'py, PyAny>> {
        let mut kw = crate::kwargs::Kwargs::parse("futopt.historical.candles", &_extra)?;
        let from_date = kw.take_string("from", from_date)?;
        let to_date = kw.take_string("to", to_date)?;
        let timeframe = kw.take_string("timeframe", timeframe)?;
        let after_hours = kw.take_flag("after_hours", after_hours)?;
        let contract_month = kw.take_string("contract_month", contract_month)?;
        let fields = kw.take_string("fields", fields)?;
        let sort = kw.take_string("sort", sort)?;
        let strike_price = kw.take("strike_price", strike_price)?;
        let call_put = kw.take_string("call_put", call_put)?;
        kw.finish()?;
        let client = self.inner.clone();
        future_into_py(py, async move {
            let result = tokio::task::spawn_blocking(move || {
                let query = FutOptCandlesQuery { symbol, from_date, to_date, timeframe, after_hours, contract_month, fields, sort, strike_price, call_put };
                query.send(&client)
            })
            .await
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Task join error: {}", e)))?;

            match result {
                Ok(candles) => Python::attach(|py| types::value_to_dict(py, &candles)),
                Err(e) => Err(errors::to_py_err(e)),
            }
        })
    }

    /// Sync sibling of `candles()` for legacy fugle-marketdata callers.
    #[pyo3(signature = (symbol, *, from_date=None, to_date=None, timeframe=None, after_hours=None, contract_month=None, fields=None, sort=None, strike_price=None, call_put=None, **_extra))]
    #[allow(clippy::too_many_arguments, reason = "mirrors the Python keyword signature")]
    pub fn candles(
        &self,
        py: Python<'_>,
        symbol: String,
        from_date: Option<String>,
        to_date: Option<String>,
        timeframe: Option<String>,
        after_hours: Option<bool>,
        contract_month: Option<String>,
        fields: Option<String>,
        sort: Option<String>,
        strike_price: Option<f64>,
        call_put: Option<String>,
        _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Py<pyo3::types::PyDict>> {
        let mut kw = crate::kwargs::Kwargs::parse("futopt.historical.candles", &_extra)?;
        let from_date = kw.take_string("from", from_date)?;
        let to_date = kw.take_string("to", to_date)?;
        let timeframe = kw.take_string("timeframe", timeframe)?;
        let after_hours = kw.take_flag("after_hours", after_hours)?;
        let contract_month = kw.take_string("contract_month", contract_month)?;
        let fields = kw.take_string("fields", fields)?;
        let sort = kw.take_string("sort", sort)?;
        let strike_price = kw.take("strike_price", strike_price)?;
        let call_put = kw.take_string("call_put", call_put)?;
        kw.finish()?;
        let inner = self.inner.clone();
        let query = FutOptCandlesQuery { symbol, from_date, to_date, timeframe, after_hours, contract_month, fields, sort, strike_price, call_put };
        let result = py.detach(|| query.send(&inner));
        match result {
            Ok(candles) => types::value_to_dict(py, &candles),
            Err(e) => Err(errors::to_py_err(e)),
        }
    }

    /// Get one trading day's daily quotes for every contract month of a FutOpt product
    ///
    /// Args:
    ///     symbol: Product code (e.g., "TXF"); a contract code such as "TXFC4" returns 404
    ///     date: Trading date (YYYY-MM-DD); the server defaults to today
    ///     after_hours: Query the after-hours session (default: False)
    ///
    /// Returns:
    ///     Awaitable[dict]: Daily quotes, one row per contract month
    ///
    /// Raises:
    ///     TypeError: If `from_date` / `to_date` are passed — the endpoint takes a single `date`
    ///
    /// Example:
    ///     ```python
    ///     daily = await client.futopt.historical.daily_async("TXF", date="2026-09-15")
    ///     ```
    #[pyo3(signature = (symbol, *, date=None, after_hours=None, **_extra))]
    pub fn daily_async<'py>(
        &self,
        py: Python<'py>,
        symbol: String,
        date: Option<String>,
        after_hours: Option<bool>,
        _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Bound<'py, PyAny>> {
        reject_daily_range_kwargs(&_extra)?;
        let mut kw = crate::kwargs::Kwargs::parse("futopt.historical.daily", &_extra)?;
        let date = kw.take_string("date", date)?;
        let after_hours = kw.take_flag("after_hours", after_hours)?;
        kw.finish()?;
        let client = self.inner.clone();
        future_into_py(py, async move {
            let result = tokio::task::spawn_blocking(move || send_futopt_daily(&client, &symbol, date.as_deref(), after_hours))
                .await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Task join error: {}", e)))?;

            match result {
                Ok(daily) => Python::attach(|py| types::value_to_dict(py, &daily)),
                Err(e) => Err(errors::to_py_err(e)),
            }
        })
    }

    /// Sync sibling of `daily()` for legacy fugle-marketdata callers.
    #[pyo3(signature = (symbol, *, date=None, after_hours=None, **_extra))]
    pub fn daily(
        &self,
        py: Python<'_>,
        symbol: String,
        date: Option<String>,
        after_hours: Option<bool>,
        _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Py<pyo3::types::PyDict>> {
        reject_daily_range_kwargs(&_extra)?;
        let mut kw = crate::kwargs::Kwargs::parse("futopt.historical.daily", &_extra)?;
        let date = kw.take_string("date", date)?;
        let after_hours = kw.take_flag("after_hours", after_hours)?;
        kw.finish()?;
        let inner = self.inner.clone();
        let result = py.detach(|| send_futopt_daily(&inner, &symbol, date.as_deref(), after_hours));
        match result {
            Ok(daily) => types::value_to_dict(py, &daily),
            Err(e) => Err(errors::to_py_err(e)),
        }
    }
}

/// Arguments shared by `futopt.historical.candles` and its `_async` sibling.
struct FutOptCandlesQuery {
    symbol: String,
    from_date: Option<String>,
    to_date: Option<String>,
    timeframe: Option<String>,
    after_hours: Option<bool>,
    contract_month: Option<String>,
    fields: Option<String>,
    sort: Option<String>,
    strike_price: Option<f64>,
    call_put: Option<String>,
}

impl FutOptCandlesQuery {
    fn send(self, client: &marketdata_core::RestClient) -> Result<serde_json::Value, marketdata_core::MarketDataError> {
        let futopt = client.futopt();
        let historical = futopt.historical();
        let mut builder = historical.candles().symbol(&self.symbol);
        if let Some(f) = &self.from_date { builder = builder.from(f); }
        if let Some(t) = &self.to_date { builder = builder.to(t); }
        if let Some(tf) = &self.timeframe { builder = builder.timeframe(tf); }
        if self.after_hours == Some(true) { builder = builder.after_hours(true); }
        if let Some(cm) = &self.contract_month { builder = builder.contract_month(cm); }
        if let Some(f) = &self.fields { builder = builder.fields(f); }
        if let Some(s) = &self.sort { builder = builder.sort(s); }
        if let Some(sp) = self.strike_price { builder = builder.strike_price(sp); }
        if let Some(cp) = &self.call_put { builder = builder.call_put(cp); }
        builder.send()
    }
}

fn send_futopt_daily(
    client: &marketdata_core::RestClient,
    symbol: &str,
    date: Option<&str>,
    after_hours: Option<bool>,
) -> Result<serde_json::Value, marketdata_core::MarketDataError> {
    let futopt = client.futopt();
    let historical = futopt.historical();
    let mut builder = historical.daily().symbol(symbol);
    if let Some(d) = date { builder = builder.date(d); }
    if after_hours == Some(true) { builder = builder.after_hours(true); }
    builder.send()
}

/// `futopt.historical.daily` used to take a date range. The endpoint returns a
/// single trading day, so a range cannot be translated; the generic
/// unknown-keyword error would only say the key is unknown, so this one says
/// what to pass instead. Runs before the generic check.
fn reject_daily_range_kwargs(extra: &Option<Bound<'_, pyo3::types::PyDict>>) -> PyResult<()> {
    let Some(extra) = extra else { return Ok(()) };
    for key in ["from_date", "to_date", "from_", "from", "to"] {
        if extra.contains(key)? {
            return Err(pyo3::exceptions::PyTypeError::new_err(format!(
                "futopt.historical.daily() no longer accepts `{key}`: the endpoint returns a single \
                 trading day. Pass `date=\"YYYY-MM-DD\"` instead."
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rest_client_creation_with_api_key() {
        Python::attach(|py| {
            let _client = RestClient::new(
                py,
                Some("test-key".to_string()),
                None,  // bearer_token
                None,  // sdk_token
                None,  // base_url
                None,  // tls_ca_file
                None,  // tls_root_cert_pem
                false, // tls_accept_invalid_certs
            ).unwrap();
        });
    }

    #[test]
    fn test_rest_client_with_bearer_token() {
        let _client = RestClient::with_bearer_token("test-token".to_string());
        // Client should be created without error
    }

    #[test]
    fn test_rest_client_with_sdk_token() {
        let _client = RestClient::with_sdk_token("test-sdk-token".to_string());
        // Client should be created without error
    }

    #[test]
    fn test_rest_client_no_auth_fails() {
        Python::attach(|py| {
            let result = RestClient::new(py, None, None, None, None, None, None, false);
            assert!(result.is_err());
        });
    }

    #[test]
    fn test_rest_client_multiple_auth_fails() {
        Python::attach(|py| {
            let result = RestClient::new(
                py,
                Some("key".to_string()),
                Some("token".to_string()),
                None,  // sdk_token
                None,  // base_url
                None,  // tls_ca_file
                None,  // tls_root_cert_pem
                false, // tls_accept_invalid_certs
            );
            assert!(result.is_err());
        });
    }
}
