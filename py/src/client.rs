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
/// client = RestClient("your-api-key")
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
    /// Provide exactly one authentication method:
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
    ///     ValueError: If zero or multiple auth methods provided
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
        // Validate exactly one auth method (fail fast)
        let auth_count = [&api_key, &bearer_token, &sdk_token]
            .iter()
            .filter(|opt| opt.is_some())
            .count();

        if auth_count != 1 {
            return Err(pyo3::exceptions::PyTypeError::new_err(
                "Provide exactly one of: api_key, bearer_token, sdk_token"
            ));
        }

        // Build Auth enum after validation
        let auth = if let Some(key) = api_key {
            marketdata_core::Auth::ApiKey(key)
        } else if let Some(token) = bearer_token {
            marketdata_core::Auth::BearerToken(token)
        } else {
            marketdata_core::Auth::SdkToken(sdk_token.unwrap())
        };

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
            inner = inner.base_url(&url);
        }

        Ok(Self { inner })
    }

    /// Create a REST client with bearer token authentication
    ///
    /// Args:
    ///     token: Bearer token for authentication
    ///
    /// Returns:
    ///     A new RestClient instance
    #[staticmethod]
    pub fn with_bearer_token(token: String) -> Self {
        let inner = marketdata_core::RestClient::new(marketdata_core::Auth::BearerToken(token));
        Self { inner }
    }

    /// Create a REST client with SDK token authentication
    ///
    /// Args:
    ///     sdk_token: SDK token for authentication
    ///
    /// Returns:
    ///     A new RestClient instance
    #[staticmethod]
    pub fn with_sdk_token(sdk_token: String) -> Self {
        let inner = marketdata_core::RestClient::new(marketdata_core::Auth::SdkToken(sdk_token));
        Self { inner }
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
    ///     quote = await client.stock.intraday.quote("2330")
    ///     print(f"Last price: {quote['lastPrice']}")
    ///     print(f"Change: {quote['change']}")
    ///     ```
    #[pyo3(signature = (symbol, odd_lot=false, **_extra))]
    pub fn quote_async<'py>(&self, py: Python<'py>, symbol: String, odd_lot: bool, _extra: Option<Bound<'_, pyo3::types::PyDict>>) -> PyResult<Bound<'py, PyAny>> {
        warn_unknown_kwargs(py, "stock.intraday.quote", &_extra);
        let client = self.inner.clone();
        future_into_py(py, async move {
            let result = tokio::task::spawn_blocking(move || {
                let stock = client.stock();
                let intraday = stock.intraday();
                let mut builder = intraday.quote().symbol(&symbol);
                if odd_lot {
                    builder = builder.odd_lot(true);
                }
                builder.send()
            }).await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Task join error: {}", e)))?;

            match result {
                Ok(quote) => Python::attach(|py| types::quote_to_dict(py, &quote)),
                Err(e) => Err(errors::to_py_err(e)),
            }
        })
    }

    /// Get intraday quote for a stock symbol (synchronous, blocking).
    ///
    /// Sync sibling of `quote()` for callers migrating from the legacy
    /// fugle-marketdata Python SDK. Releases the GIL during the network call.
    #[pyo3(signature = (symbol, odd_lot=false, **_extra))]
    pub fn quote(&self, py: Python<'_>, symbol: String, odd_lot: bool, _extra: Option<Bound<'_, pyo3::types::PyDict>>) -> PyResult<Py<pyo3::types::PyDict>> {
        warn_unknown_kwargs(py, "stock.intraday.quote", &_extra);
        let inner = self.inner.clone();
        let result = py.detach(|| {
            let stock = inner.stock();
            let intraday = stock.intraday();
            let mut builder = intraday.quote().symbol(&symbol);
            if odd_lot {
                builder = builder.odd_lot(true);
            }
            builder.send()
        });
        match result {
            Ok(quote) => types::quote_to_dict(py, &quote),
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
    ///     ticker = await client.stock.intraday.ticker("2330")
    ///     ```
    #[pyo3(signature = (symbol, **_extra))]
    pub fn ticker_async<'py>(&self, py: Python<'py>, symbol: String, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Bound<'py, PyAny>> {
        warn_unknown_kwargs(py, "stock.intraday.ticker", &_extra);
        let client = self.inner.clone();
        future_into_py(py, async move {
            let result = tokio::task::spawn_blocking(move || {
                let stock = client.stock();
                let intraday = stock.intraday();
                intraday.ticker().symbol(&symbol).send()
            }).await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Task join error: {}", e)))?;

            match result {
                Ok(ticker) => Python::attach(|py| types::ticker_to_dict(py, &ticker)),
                Err(e) => Err(errors::to_py_err(e)),
            }
        })
    }

    /// Sync sibling of `ticker()` for legacy fugle-marketdata callers.
    #[pyo3(signature = (symbol, **_extra))]
    pub fn ticker(&self, py: Python<'_>, symbol: String, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Py<pyo3::types::PyDict>> {
        warn_unknown_kwargs(py, "stock.intraday.ticker", &_extra);
        let inner = self.inner.clone();
        let result = py.detach(|| {
            let stock = inner.stock();
            let intraday = stock.intraday();
            intraday.ticker().symbol(&symbol).send()
        });
        match result {
            Ok(ticker) => types::ticker_to_dict(py, &ticker),
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
    ///     candles = await client.stock.intraday.candles("2330", "5")
    ///     ```
    #[pyo3(signature = (symbol, timeframe="1".to_string(), **_extra))]
    pub fn candles_async<'py>(&self, py: Python<'py>, symbol: String, timeframe: String, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Bound<'py, PyAny>> {
        warn_unknown_kwargs(py, "stock.intraday.candles", &_extra);
        let client = self.inner.clone();
        future_into_py(py, async move {
            let result = tokio::task::spawn_blocking(move || {
                let stock = client.stock();
                let intraday = stock.intraday();
                intraday.candles().symbol(&symbol).timeframe(&timeframe).send()
            }).await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Task join error: {}", e)))?;

            match result {
                Ok(candles) => Python::attach(|py| types::candles_to_dict(py, &candles)),
                Err(e) => Err(errors::to_py_err(e)),
            }
        })
    }

    /// Sync sibling of `candles()` for legacy fugle-marketdata callers.
    #[pyo3(signature = (symbol, timeframe="1".to_string(), **_extra))]
    pub fn candles(&self, py: Python<'_>, symbol: String, timeframe: String, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Py<pyo3::types::PyDict>> {
        warn_unknown_kwargs(py, "stock.intraday.candles", &_extra);
        let inner = self.inner.clone();
        let result = py.detach(|| {
            let stock = inner.stock();
            let intraday = stock.intraday();
            intraday.candles().symbol(&symbol).timeframe(&timeframe).send()
        });
        match result {
            Ok(candles) => types::candles_to_dict(py, &candles),
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
    ///     trades = await client.stock.intraday.trades("2330")
    ///     ```
    #[pyo3(signature = (symbol, **_extra))]
    pub fn trades_async<'py>(&self, py: Python<'py>, symbol: String, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Bound<'py, PyAny>> {
        warn_unknown_kwargs(py, "stock.intraday.trades", &_extra);
        let client = self.inner.clone();
        future_into_py(py, async move {
            let result = tokio::task::spawn_blocking(move || {
                let stock = client.stock();
                let intraday = stock.intraday();
                intraday.trades().symbol(&symbol).send()
            }).await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Task join error: {}", e)))?;

            match result {
                Ok(trades) => Python::attach(|py| types::trades_to_dict(py, &trades)),
                Err(e) => Err(errors::to_py_err(e)),
            }
        })
    }

    /// Sync sibling of `trades()` for legacy fugle-marketdata callers.
    #[pyo3(signature = (symbol, **_extra))]
    pub fn trades(&self, py: Python<'_>, symbol: String, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Py<pyo3::types::PyDict>> {
        warn_unknown_kwargs(py, "stock.intraday.trades", &_extra);
        let inner = self.inner.clone();
        let result = py.detach(|| {
            let stock = inner.stock();
            let intraday = stock.intraday();
            intraday.trades().symbol(&symbol).send()
        });
        match result {
            Ok(trades) => types::trades_to_dict(py, &trades),
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
    ///     volumes = await client.stock.intraday.volumes("2330")
    ///     ```
    #[pyo3(signature = (symbol, **_extra))]
    pub fn volumes_async<'py>(&self, py: Python<'py>, symbol: String, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Bound<'py, PyAny>> {
        warn_unknown_kwargs(py, "stock.intraday.volumes", &_extra);
        let client = self.inner.clone();
        future_into_py(py, async move {
            let result = tokio::task::spawn_blocking(move || {
                let stock = client.stock();
                let intraday = stock.intraday();
                intraday.volumes().symbol(&symbol).send()
            }).await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Task join error: {}", e)))?;

            match result {
                Ok(volumes) => Python::attach(|py| types::volumes_to_dict(py, &volumes)),
                Err(e) => Err(errors::to_py_err(e)),
            }
        })
    }

    /// Sync sibling of `volumes()` for legacy fugle-marketdata callers.
    #[pyo3(signature = (symbol, **_extra))]
    pub fn volumes(&self, py: Python<'_>, symbol: String, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Py<pyo3::types::PyDict>> {
        warn_unknown_kwargs(py, "stock.intraday.volumes", &_extra);
        let inner = self.inner.clone();
        let result = py.detach(|| {
            let stock = inner.stock();
            let intraday = stock.intraday();
            intraday.volumes().symbol(&symbol).send()
        });
        match result {
            Ok(volumes) => types::volumes_to_dict(py, &volumes),
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
    ///     tickers = await client.stock.intraday.tickers(type="EQUITY")
    ///     ```
    #[pyo3(signature = (r#type, exchange=None, market=None, industry=None, is_normal=None, **_extra))]
    pub fn tickers_async<'py>(
        &self,
        py: Python<'py>,
        r#type: String,
        exchange: Option<String>,
        market: Option<String>,
        industry: Option<String>,
        is_normal: Option<bool>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Bound<'py, PyAny>> {
        warn_unknown_kwargs(py, "stock.intraday.tickers", &_extra);
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
    #[pyo3(signature = (r#type, exchange=None, market=None, industry=None, is_normal=None, **_extra))]
    pub fn tickers(
        &self,
        py: Python<'_>,
        r#type: String,
        exchange: Option<String>,
        market: Option<String>,
        industry: Option<String>,
        is_normal: Option<bool>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Py<PyAny>> {
        warn_unknown_kwargs(py, "stock.intraday.tickers", &_extra);
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
    ///     candles = await client.stock.historical.candles(
    ///         "2330",
    ///         from_date="2024-01-01",
    ///         to_date="2024-01-31",
    ///         timeframe="D"
    ///     )
    ///     ```
    #[pyo3(signature = (symbol, from_date=None, to_date=None, timeframe=None, fields=None, sort=None, adjusted=None, **_extra))]
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
        warn_unknown_kwargs(py, "stock.historical.candles", &_extra);
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
                Ok(candles) => Python::attach(|py| types::historical_candles_to_dict(py, &candles)),
                Err(e) => Err(errors::to_py_err(e)),
            }
        })
    }

    /// Sync sibling of `candles()` for legacy fugle-marketdata callers.
    #[pyo3(signature = (symbol, from_date=None, to_date=None, timeframe=None, fields=None, sort=None, adjusted=None, **_extra))]
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
        warn_unknown_kwargs(py, "stock.historical.candles", &_extra);
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
            Ok(candles) => types::historical_candles_to_dict(py, &candles),
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
    ///     stats = await client.stock.historical.stats("2330")
    ///     print(f"52-week high: {stats['week52High']}")
    ///     ```
    #[pyo3(signature = (symbol, **_extra))]
    pub fn stats_async<'py>(&self, py: Python<'py>, symbol: String, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Bound<'py, PyAny>> {
        warn_unknown_kwargs(py, "stock.historical.stats", &_extra);
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
                Ok(stats) => Python::attach(|py| types::stats_to_dict(py, &stats)),
                Err(e) => Err(errors::to_py_err(e)),
            }
        })
    }

    /// Sync sibling of `stats()` for legacy fugle-marketdata callers.
    #[pyo3(signature = (symbol, **_extra))]
    pub fn stats(&self, py: Python<'_>, symbol: String, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Py<pyo3::types::PyDict>> {
        warn_unknown_kwargs(py, "stock.historical.stats", &_extra);
        let inner = self.inner.clone();
        let result = py.detach(|| {
            let stock = inner.stock();
            let historical = stock.historical();
            historical.stats().symbol(&symbol).send()
        });
        match result {
            Ok(stats) => types::stats_to_dict(py, &stats),
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
    ///     quotes = await client.stock.snapshot.quotes("TSE", type_filter="COMMONSTOCK")
    ///     ```
    #[pyo3(signature = (market, type_filter=None, **_extra))]
    pub fn quotes_async<'py>(
        &self,
        py: Python<'py>,
        market: String,
        type_filter: Option<String>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Bound<'py, PyAny>> {
        warn_unknown_kwargs(py, "stock.snapshot.quotes", &_extra);
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
                Ok(quotes) => Python::attach(|py| types::snapshot_quotes_to_dict(py, &quotes)),
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
    ///     movers = await client.stock.snapshot.movers("TSE", direction="up", change="percent")
    ///     ```
    #[pyo3(signature = (market, direction=None, change=None, **_extra))]
    pub fn movers_async<'py>(
        &self,
        py: Python<'py>,
        market: String,
        direction: Option<String>,
        change: Option<String>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Bound<'py, PyAny>> {
        warn_unknown_kwargs(py, "stock.snapshot.movers", &_extra);
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
                builder.send()
            })
            .await
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Task join error: {}", e)))?;

            match result {
                Ok(movers) => Python::attach(|py| types::movers_to_dict(py, &movers)),
                Err(e) => Err(errors::to_py_err(e)),
            }
        })
    }

    /// Get most active stocks for a market
    ///
    /// Args:
    ///     market: Market code ("TSE", "OTC", "ESB", "TIB", "PSB")
    ///     trade: Trade type ("volume" or "value")
    ///
    /// Returns:
    ///     Awaitable[dict]: Most active stocks data
    ///
    /// Example:
    ///     ```python
    ///     actives = await client.stock.snapshot.actives("TSE", trade="volume")
    ///     ```
    #[pyo3(signature = (market, trade=None, **_extra))]
    pub fn actives_async<'py>(
        &self,
        py: Python<'py>,
        market: String,
        trade: Option<String>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Bound<'py, PyAny>> {
        warn_unknown_kwargs(py, "stock.snapshot.actives", &_extra);
        let client = self.inner.clone();
        future_into_py(py, async move {
            let result = tokio::task::spawn_blocking(move || {
                let stock = client.stock();
                let snapshot = stock.snapshot();
                let mut builder = snapshot.actives().market(&market);
                if let Some(t) = trade {
                    builder = builder.trade(&t);
                }
                builder.send()
            })
            .await
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Task join error: {}", e)))?;

            match result {
                Ok(actives) => Python::attach(|py| types::actives_to_dict(py, &actives)),
                Err(e) => Err(errors::to_py_err(e)),
            }
        })
    }

    /// Sync sibling of `quotes()` for legacy fugle-marketdata callers.
    #[pyo3(signature = (market, type_filter=None, **_extra))]
    pub fn quotes(
        &self,
        py: Python<'_>,
        market: String,
        type_filter: Option<String>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Py<pyo3::types::PyDict>> {
        warn_unknown_kwargs(py, "stock.snapshot.quotes", &_extra);
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
            Ok(quotes) => types::snapshot_quotes_to_dict(py, &quotes),
            Err(e) => Err(errors::to_py_err(e)),
        }
    }

    /// Sync sibling of `movers()` for legacy fugle-marketdata callers.
    #[pyo3(signature = (market, direction=None, change=None, **_extra))]
    pub fn movers(
        &self,
        py: Python<'_>,
        market: String,
        direction: Option<String>,
        change: Option<String>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Py<pyo3::types::PyDict>> {
        warn_unknown_kwargs(py, "stock.snapshot.movers", &_extra);
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
            builder.send()
        });
        match result {
            Ok(movers) => types::movers_to_dict(py, &movers),
            Err(e) => Err(errors::to_py_err(e)),
        }
    }

    /// Sync sibling of `actives()` for legacy fugle-marketdata callers.
    #[pyo3(signature = (market, trade=None, **_extra))]
    pub fn actives(
        &self,
        py: Python<'_>,
        market: String,
        trade: Option<String>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Py<pyo3::types::PyDict>> {
        warn_unknown_kwargs(py, "stock.snapshot.actives", &_extra);
        let inner = self.inner.clone();
        let result = py.detach(|| {
            let stock = inner.stock();
            let snapshot = stock.snapshot();
            let mut builder = snapshot.actives().market(&market);
            if let Some(t) = trade {
                builder = builder.trade(&t);
            }
            builder.send()
        });
        match result {
            Ok(actives) => types::actives_to_dict(py, &actives),
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
    ///     from_date: Start date (YYYY-MM-DD)
    ///     to_date: End date (YYYY-MM-DD)
    ///     timeframe: Timeframe ("D", "W", "M", "1", "5", etc.)
    ///     period: Moving average period
    ///
    /// Returns:
    ///     Awaitable[dict]: SMA indicator data
    #[pyo3(signature = (symbol, from_date=None, to_date=None, timeframe=None, period=None, **_extra))]
    pub fn sma_async<'py>(
        &self,
        py: Python<'py>,
        symbol: String,
        from_date: Option<String>,
        to_date: Option<String>,
        timeframe: Option<String>,
        period: Option<u32>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Bound<'py, PyAny>> {
        warn_unknown_kwargs(py, "stock.technical.sma", &_extra);
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
                Ok(sma) => Python::attach(|py| types::technical_to_dict(py, &sma)),
                Err(e) => Err(errors::to_py_err(e)),
            }
        })
    }

    /// Get Relative Strength Index (RSI) data
    ///
    /// Args:
    ///     symbol: Stock symbol (e.g., "2330" for TSMC)
    ///     from_date: Start date (YYYY-MM-DD)
    ///     to_date: End date (YYYY-MM-DD)
    ///     timeframe: Timeframe ("D", "W", "M", "1", "5", etc.)
    ///     period: RSI period (default 14)
    ///
    /// Returns:
    ///     Awaitable[dict]: RSI indicator data
    #[pyo3(signature = (symbol, from_date=None, to_date=None, timeframe=None, period=None, **_extra))]
    pub fn rsi_async<'py>(
        &self,
        py: Python<'py>,
        symbol: String,
        from_date: Option<String>,
        to_date: Option<String>,
        timeframe: Option<String>,
        period: Option<u32>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Bound<'py, PyAny>> {
        warn_unknown_kwargs(py, "stock.technical.rsi", &_extra);
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
                Ok(rsi) => Python::attach(|py| types::technical_to_dict(py, &rsi)),
                Err(e) => Err(errors::to_py_err(e)),
            }
        })
    }

    /// Get KDJ (Stochastic Oscillator) data
    ///
    /// Args:
    ///     symbol: Stock symbol (e.g., "2330" for TSMC)
    ///     from_date: Start date (YYYY-MM-DD)
    ///     to_date: End date (YYYY-MM-DD)
    ///     timeframe: Timeframe ("D", "W", "M", "1", "5", etc.)
    ///     period: KDJ period
    ///
    /// Returns:
    ///     Awaitable[dict]: KDJ indicator data with K, D, J values
    #[pyo3(signature = (symbol, from_date=None, to_date=None, timeframe=None, period=None, **_extra))]
    pub fn kdj_async<'py>(
        &self,
        py: Python<'py>,
        symbol: String,
        from_date: Option<String>,
        to_date: Option<String>,
        timeframe: Option<String>,
        period: Option<u32>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Bound<'py, PyAny>> {
        warn_unknown_kwargs(py, "stock.technical.kdj", &_extra);
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
                if let Some(p) = period {
                    builder = builder.period(p);
                }
                builder.send()
            })
            .await
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Task join error: {}", e)))?;

            match result {
                Ok(kdj) => Python::attach(|py| types::technical_to_dict(py, &kdj)),
                Err(e) => Err(errors::to_py_err(e)),
            }
        })
    }

    /// Get MACD (Moving Average Convergence Divergence) data
    ///
    /// Args:
    ///     symbol: Stock symbol (e.g., "2330" for TSMC)
    ///     from_date: Start date (YYYY-MM-DD)
    ///     to_date: End date (YYYY-MM-DD)
    ///     timeframe: Timeframe ("D", "W", "M", "1", "5", etc.)
    ///     fast: Fast EMA period (default 12)
    ///     slow: Slow EMA period (default 26)
    ///     signal: Signal line period (default 9)
    ///
    /// Returns:
    ///     Awaitable[dict]: MACD indicator data with MACD, signal, histogram
    #[pyo3(signature = (symbol, from_date=None, to_date=None, timeframe=None, fast=None, slow=None, signal=None, **_extra))]
    pub fn macd_async<'py>(
        &self,
        py: Python<'py>,
        symbol: String,
        from_date: Option<String>,
        to_date: Option<String>,
        timeframe: Option<String>,
        fast: Option<u32>,
        slow: Option<u32>,
        signal: Option<u32>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Bound<'py, PyAny>> {
        warn_unknown_kwargs(py, "stock.technical.macd", &_extra);
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
                Ok(macd) => Python::attach(|py| types::technical_to_dict(py, &macd)),
                Err(e) => Err(errors::to_py_err(e)),
            }
        })
    }

    /// Get Bollinger Bands (BB) data
    ///
    /// Args:
    ///     symbol: Stock symbol (e.g., "2330" for TSMC)
    ///     from_date: Start date (YYYY-MM-DD)
    ///     to_date: End date (YYYY-MM-DD)
    ///     timeframe: Timeframe ("D", "W", "M", "1", "5", etc.)
    ///     period: Moving average period (default 20)
    ///     stddev: Standard deviation multiplier (default 2.0)
    ///
    /// Returns:
    ///     Awaitable[dict]: Bollinger Bands data with upper, middle, lower bands
    #[pyo3(signature = (symbol, from_date=None, to_date=None, timeframe=None, period=None, stddev=None, **_extra))]
    pub fn bb_async<'py>(
        &self,
        py: Python<'py>,
        symbol: String,
        from_date: Option<String>,
        to_date: Option<String>,
        timeframe: Option<String>,
        period: Option<u32>,
        stddev: Option<f64>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Bound<'py, PyAny>> {
        warn_unknown_kwargs(py, "stock.technical.bb", &_extra);
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
                if let Some(sd) = stddev {
                    builder = builder.stddev(sd);
                }
                builder.send()
            })
            .await
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Task join error: {}", e)))?;

            match result {
                Ok(bb) => Python::attach(|py| types::technical_to_dict(py, &bb)),
                Err(e) => Err(errors::to_py_err(e)),
            }
        })
    }

    /// Sync sibling of `sma()` for legacy fugle-marketdata callers.
    #[pyo3(signature = (symbol, from_date=None, to_date=None, timeframe=None, period=None, **_extra))]
    pub fn sma(
        &self,
        py: Python<'_>,
        symbol: String,
        from_date: Option<String>,
        to_date: Option<String>,
        timeframe: Option<String>,
        period: Option<u32>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Py<pyo3::types::PyDict>> {
        warn_unknown_kwargs(py, "stock.technical.sma", &_extra);
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
            Ok(sma) => types::technical_to_dict(py, &sma),
            Err(e) => Err(errors::to_py_err(e)),
        }
    }

    /// Sync sibling of `rsi()` for legacy fugle-marketdata callers.
    #[pyo3(signature = (symbol, from_date=None, to_date=None, timeframe=None, period=None, **_extra))]
    pub fn rsi(
        &self,
        py: Python<'_>,
        symbol: String,
        from_date: Option<String>,
        to_date: Option<String>,
        timeframe: Option<String>,
        period: Option<u32>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Py<pyo3::types::PyDict>> {
        warn_unknown_kwargs(py, "stock.technical.rsi", &_extra);
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
            Ok(rsi) => types::technical_to_dict(py, &rsi),
            Err(e) => Err(errors::to_py_err(e)),
        }
    }

    /// Sync sibling of `kdj()` for legacy fugle-marketdata callers.
    #[pyo3(signature = (symbol, from_date=None, to_date=None, timeframe=None, period=None, **_extra))]
    pub fn kdj(
        &self,
        py: Python<'_>,
        symbol: String,
        from_date: Option<String>,
        to_date: Option<String>,
        timeframe: Option<String>,
        period: Option<u32>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Py<pyo3::types::PyDict>> {
        warn_unknown_kwargs(py, "stock.technical.kdj", &_extra);
        let inner = self.inner.clone();
        let result = py.detach(|| {
            let stock = inner.stock();
            let technical = stock.technical();
            let mut builder = technical.kdj().symbol(&symbol);
            if let Some(f) = from_date { builder = builder.from(&f); }
            if let Some(t) = to_date { builder = builder.to(&t); }
            if let Some(tf) = timeframe { builder = builder.timeframe(&tf); }
            if let Some(p) = period { builder = builder.period(p); }
            builder.send()
        });
        match result {
            Ok(kdj) => types::technical_to_dict(py, &kdj),
            Err(e) => Err(errors::to_py_err(e)),
        }
    }

    /// Sync sibling of `macd()` for legacy fugle-marketdata callers.
    #[pyo3(signature = (symbol, from_date=None, to_date=None, timeframe=None, fast=None, slow=None, signal=None, **_extra))]
    pub fn macd(
        &self,
        py: Python<'_>,
        symbol: String,
        from_date: Option<String>,
        to_date: Option<String>,
        timeframe: Option<String>,
        fast: Option<u32>,
        slow: Option<u32>,
        signal: Option<u32>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Py<pyo3::types::PyDict>> {
        warn_unknown_kwargs(py, "stock.technical.macd", &_extra);
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
            Ok(macd) => types::technical_to_dict(py, &macd),
            Err(e) => Err(errors::to_py_err(e)),
        }
    }

    /// Sync sibling of `bb()` for legacy fugle-marketdata callers.
    #[pyo3(signature = (symbol, from_date=None, to_date=None, timeframe=None, period=None, stddev=None, **_extra))]
    pub fn bb(
        &self,
        py: Python<'_>,
        symbol: String,
        from_date: Option<String>,
        to_date: Option<String>,
        timeframe: Option<String>,
        period: Option<u32>,
        stddev: Option<f64>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Py<pyo3::types::PyDict>> {
        warn_unknown_kwargs(py, "stock.technical.bb", &_extra);
        let inner = self.inner.clone();
        let result = py.detach(|| {
            let stock = inner.stock();
            let technical = stock.technical();
            let mut builder = technical.bb().symbol(&symbol);
            if let Some(f) = from_date { builder = builder.from(&f); }
            if let Some(t) = to_date { builder = builder.to(&t); }
            if let Some(tf) = timeframe { builder = builder.timeframe(&tf); }
            if let Some(p) = period { builder = builder.period(p); }
            if let Some(sd) = stddev { builder = builder.stddev(sd); }
            builder.send()
        });
        match result {
            Ok(bb) => types::technical_to_dict(py, &bb),
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
    ///     date: Specific date (YYYY-MM-DD)
    ///     start_date: Start date for range query (YYYY-MM-DD)
    ///     end_date: End date for range query (YYYY-MM-DD)
    ///
    /// Returns:
    ///     Awaitable[dict]: Capital changes data
    ///
    /// Example:
    ///     ```python
    ///     changes = await client.stock.corporate_actions.capital_changes(
    ///         start_date="2024-01-01",
    ///         end_date="2024-01-31"
    ///     )
    ///     ```
    #[pyo3(signature = (date=None, start_date=None, end_date=None, **_extra))]
    pub fn capital_changes_async<'py>(
        &self,
        py: Python<'py>,
        date: Option<String>,
        start_date: Option<String>,
        end_date: Option<String>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Bound<'py, PyAny>> {
        warn_unknown_kwargs(py, "stock.corporate_actions.capital_changes", &_extra);
        let client = self.inner.clone();
        future_into_py(py, async move {
            let result = tokio::task::spawn_blocking(move || {
                let stock = client.stock();
                let corp = stock.corporate_actions();
                let mut builder = corp.capital_changes();
                if let Some(d) = date {
                    builder = builder.date(&d);
                }
                if let Some(sd) = start_date {
                    builder = builder.start_date(&sd);
                }
                if let Some(ed) = end_date {
                    builder = builder.end_date(&ed);
                }
                builder.send()
            })
            .await
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Task join error: {}", e)))?;

            match result {
                Ok(changes) => Python::attach(|py| types::corporate_action_to_dict(py, &changes)),
                Err(e) => Err(errors::to_py_err(e)),
            }
        })
    }

    /// Get dividend announcements
    ///
    /// Args:
    ///     date: Specific date (YYYY-MM-DD)
    ///     start_date: Start date for range query (YYYY-MM-DD)
    ///     end_date: End date for range query (YYYY-MM-DD)
    ///
    /// Returns:
    ///     Awaitable[dict]: Dividend data
    ///
    /// Example:
    ///     ```python
    ///     dividends = await client.stock.corporate_actions.dividends(
    ///         start_date="2024-01-01",
    ///         end_date="2024-12-31"
    ///     )
    ///     ```
    #[pyo3(signature = (date=None, start_date=None, end_date=None, **_extra))]
    pub fn dividends_async<'py>(
        &self,
        py: Python<'py>,
        date: Option<String>,
        start_date: Option<String>,
        end_date: Option<String>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Bound<'py, PyAny>> {
        warn_unknown_kwargs(py, "stock.corporate_actions.dividends", &_extra);
        let client = self.inner.clone();
        future_into_py(py, async move {
            let result = tokio::task::spawn_blocking(move || {
                let stock = client.stock();
                let corp = stock.corporate_actions();
                let mut builder = corp.dividends();
                if let Some(d) = date {
                    builder = builder.date(&d);
                }
                if let Some(sd) = start_date {
                    builder = builder.start_date(&sd);
                }
                if let Some(ed) = end_date {
                    builder = builder.end_date(&ed);
                }
                builder.send()
            })
            .await
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Task join error: {}", e)))?;

            match result {
                Ok(dividends) => Python::attach(|py| types::corporate_action_to_dict(py, &dividends)),
                Err(e) => Err(errors::to_py_err(e)),
            }
        })
    }

    /// Get IPO listing applicants
    ///
    /// Args:
    ///     date: Specific date (YYYY-MM-DD)
    ///     start_date: Start date for range query (YYYY-MM-DD)
    ///     end_date: End date for range query (YYYY-MM-DD)
    ///
    /// Returns:
    ///     Awaitable[dict]: Listing applicants data
    ///
    /// Example:
    ///     ```python
    ///     applicants = await client.stock.corporate_actions.listing_applicants()
    ///     ```
    #[pyo3(signature = (date=None, start_date=None, end_date=None, **_extra))]
    pub fn listing_applicants_async<'py>(
        &self,
        py: Python<'py>,
        date: Option<String>,
        start_date: Option<String>,
        end_date: Option<String>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Bound<'py, PyAny>> {
        warn_unknown_kwargs(py, "stock.corporate_actions.listing_applicants", &_extra);
        let client = self.inner.clone();
        future_into_py(py, async move {
            let result = tokio::task::spawn_blocking(move || {
                let stock = client.stock();
                let corp = stock.corporate_actions();
                let mut builder = corp.listing_applicants();
                if let Some(d) = date {
                    builder = builder.date(&d);
                }
                if let Some(sd) = start_date {
                    builder = builder.start_date(&sd);
                }
                if let Some(ed) = end_date {
                    builder = builder.end_date(&ed);
                }
                builder.send()
            })
            .await
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Task join error: {}", e)))?;

            match result {
                Ok(applicants) => Python::attach(|py| types::corporate_action_to_dict(py, &applicants)),
                Err(e) => Err(errors::to_py_err(e)),
            }
        })
    }

    /// Sync sibling of `capital_changes()` for legacy fugle-marketdata callers.
    #[pyo3(signature = (date=None, start_date=None, end_date=None, **_extra))]
    pub fn capital_changes(
        &self,
        py: Python<'_>,
        date: Option<String>,
        start_date: Option<String>,
        end_date: Option<String>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Py<pyo3::types::PyDict>> {
        warn_unknown_kwargs(py, "stock.corporate_actions.capital_changes", &_extra);
        let inner = self.inner.clone();
        let result = py.detach(|| {
            let stock = inner.stock();
            let corp = stock.corporate_actions();
            let mut builder = corp.capital_changes();
            if let Some(d) = date { builder = builder.date(&d); }
            if let Some(sd) = start_date { builder = builder.start_date(&sd); }
            if let Some(ed) = end_date { builder = builder.end_date(&ed); }
            builder.send()
        });
        match result {
            Ok(changes) => types::corporate_action_to_dict(py, &changes),
            Err(e) => Err(errors::to_py_err(e)),
        }
    }

    /// Sync sibling of `dividends()` for legacy fugle-marketdata callers.
    #[pyo3(signature = (date=None, start_date=None, end_date=None, **_extra))]
    pub fn dividends(
        &self,
        py: Python<'_>,
        date: Option<String>,
        start_date: Option<String>,
        end_date: Option<String>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Py<pyo3::types::PyDict>> {
        warn_unknown_kwargs(py, "stock.corporate_actions.dividends", &_extra);
        let inner = self.inner.clone();
        let result = py.detach(|| {
            let stock = inner.stock();
            let corp = stock.corporate_actions();
            let mut builder = corp.dividends();
            if let Some(d) = date { builder = builder.date(&d); }
            if let Some(sd) = start_date { builder = builder.start_date(&sd); }
            if let Some(ed) = end_date { builder = builder.end_date(&ed); }
            builder.send()
        });
        match result {
            Ok(dividends) => types::corporate_action_to_dict(py, &dividends),
            Err(e) => Err(errors::to_py_err(e)),
        }
    }

    /// Sync sibling of `listing_applicants()` for legacy fugle-marketdata callers.
    #[pyo3(signature = (date=None, start_date=None, end_date=None, **_extra))]
    pub fn listing_applicants(
        &self,
        py: Python<'_>,
        date: Option<String>,
        start_date: Option<String>,
        end_date: Option<String>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Py<pyo3::types::PyDict>> {
        warn_unknown_kwargs(py, "stock.corporate_actions.listing_applicants", &_extra);
        let inner = self.inner.clone();
        let result = py.detach(|| {
            let stock = inner.stock();
            let corp = stock.corporate_actions();
            let mut builder = corp.listing_applicants();
            if let Some(d) = date { builder = builder.date(&d); }
            if let Some(sd) = start_date { builder = builder.start_date(&sd); }
            if let Some(ed) = end_date { builder = builder.end_date(&ed); }
            builder.send()
        });
        match result {
            Ok(applicants) => types::corporate_action_to_dict(py, &applicants),
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
    ///     quote = await client.futopt.intraday.quote("TXFC4")
    ///
    ///     # After-hours session
    ///     ah_quote = await client.futopt.intraday.quote("TXFC4", after_hours=True)
    ///     ```
    #[pyo3(signature = (symbol, after_hours=false, **_extra))]
    pub fn quote_async<'py>(&self, py: Python<'py>, symbol: String, after_hours: bool, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Bound<'py, PyAny>> {
        warn_unknown_kwargs(py, "futopt.intraday.quote", &_extra);
        let client = self.inner.clone();
        future_into_py(py, async move {
            let result = tokio::task::spawn_blocking(move || {
                let futopt = client.futopt();
                let intraday = futopt.intraday();
                let mut builder = intraday.quote().symbol(&symbol);
                if after_hours {
                    builder = builder.after_hours();
                }
                builder.send()
            }).await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Task join error: {}", e)))?;

            match result {
                Ok(quote) => Python::attach(|py| types::futopt_quote_to_dict(py, &quote)),
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
    ///     tickers = await client.futopt.intraday.tickers(type="FUTURE")
    ///     ```
    #[pyo3(signature = (r#type, exchange=None, after_hours=false, contract_type=None, **_extra))]
    pub fn tickers_async<'py>(
        &self,
        py: Python<'py>,
        r#type: String,
        exchange: Option<String>,
        after_hours: bool,
        contract_type: Option<String>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Bound<'py, PyAny>> {
        warn_unknown_kwargs(py, "futopt.intraday.tickers", &_extra);
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
                if after_hours {
                    builder = builder.after_hours();
                }
                if let Some(c) = ct {
                    builder = builder.contract_type(c);
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
    ///
    /// Returns:
    ///     Awaitable[list[dict]]: List of product info dicts
    ///
    /// Example:
    ///     ```python
    ///     products = await client.futopt.intraday.products(type="FUTURE")
    ///     ```
    #[pyo3(signature = (r#type, contract_type=None, **_extra))]
    pub fn products_async<'py>(
        &self,
        py: Python<'py>,
        r#type: String,
        contract_type: Option<String>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Bound<'py, PyAny>> {
        warn_unknown_kwargs(py, "futopt.intraday.products", &_extra);
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
    #[pyo3(signature = (symbol, after_hours=false, **_extra))]
    pub fn quote(&self, py: Python<'_>, symbol: String, after_hours: bool, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Py<pyo3::types::PyDict>> {
        warn_unknown_kwargs(py, "futopt.intraday.quote", &_extra);
        let inner = self.inner.clone();
        let result = py.detach(|| {
            let futopt = inner.futopt();
            let intraday = futopt.intraday();
            let mut builder = intraday.quote().symbol(&symbol);
            if after_hours {
                builder = builder.after_hours();
            }
            builder.send()
        });
        match result {
            Ok(quote) => types::futopt_quote_to_dict(py, &quote),
            Err(e) => Err(errors::to_py_err(e)),
        }
    }

    /// Sync sibling of `tickers()` for legacy fugle-marketdata callers.
    #[pyo3(signature = (r#type, exchange=None, after_hours=false, contract_type=None, **_extra))]
    pub fn tickers(
        &self,
        py: Python<'_>,
        r#type: String,
        exchange: Option<String>,
        after_hours: bool,
        contract_type: Option<String>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Py<PyAny>> {
        warn_unknown_kwargs(py, "futopt.intraday.tickers", &_extra);
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
            if after_hours {
                builder = builder.after_hours();
            }
            if let Some(c) = ct {
                builder = builder.contract_type(c);
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
    #[pyo3(signature = (r#type, contract_type=None, **_extra))]
    pub fn products(
        &self,
        py: Python<'_>,
        r#type: String,
        contract_type: Option<String>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Py<PyAny>> {
        warn_unknown_kwargs(py, "futopt.intraday.products", &_extra);
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
    #[pyo3(signature = (symbol, after_hours=false, **_extra))]
    pub fn ticker_async<'py>(
        &self,
        py: Python<'py>,
        symbol: String,
        after_hours: bool, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Bound<'py, PyAny>> {
        warn_unknown_kwargs(py, "futopt.intraday.ticker", &_extra);
        let client = self.inner.clone();
        future_into_py(py, async move {
            let result = tokio::task::spawn_blocking(move || {
                let futopt = client.futopt();
                let intraday = futopt.intraday();
                let mut builder = intraday.ticker().symbol(&symbol);
                if after_hours {
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
    #[pyo3(signature = (symbol, after_hours=false, **_extra))]
    pub fn ticker(
        &self,
        py: Python<'_>,
        symbol: String,
        after_hours: bool, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Py<PyAny>> {
        warn_unknown_kwargs(py, "futopt.intraday.ticker", &_extra);
        let inner = self.inner.clone();
        let result = py.detach(|| {
            let futopt = inner.futopt();
            let intraday = futopt.intraday();
            let mut builder = intraday.ticker().symbol(&symbol);
            if after_hours {
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
    #[pyo3(signature = (symbol, timeframe="1".to_string(), after_hours=false, **_extra))]
    pub fn candles_async<'py>(
        &self,
        py: Python<'py>,
        symbol: String,
        timeframe: String,
        after_hours: bool, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Bound<'py, PyAny>> {
        warn_unknown_kwargs(py, "futopt.intraday.candles", &_extra);
        let client = self.inner.clone();
        future_into_py(py, async move {
            let result = tokio::task::spawn_blocking(move || {
                let futopt = client.futopt();
                let intraday = futopt.intraday();
                let mut builder = intraday
                    .candles()
                    .symbol(&symbol)
                    .timeframe(&timeframe);
                if after_hours {
                    builder = builder.after_hours();
                }
                builder.send()
            })
            .await
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Task join error: {}", e)))?;

            match result {
                Ok(candles) => Python::attach(|py| types::candles_to_dict(py, &candles)),
                Err(e) => Err(errors::to_py_err(e)),
            }
        })
    }

    /// Sync sibling of `candles()` for legacy fugle-marketdata callers.
    #[pyo3(signature = (symbol, timeframe="1".to_string(), after_hours=false, **_extra))]
    pub fn candles(
        &self,
        py: Python<'_>,
        symbol: String,
        timeframe: String,
        after_hours: bool, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Py<pyo3::types::PyDict>> {
        warn_unknown_kwargs(py, "futopt.intraday.candles", &_extra);
        let inner = self.inner.clone();
        let result = py.detach(|| {
            let futopt = inner.futopt();
            let intraday = futopt.intraday();
            let mut builder = intraday
                .candles()
                .symbol(&symbol)
                .timeframe(&timeframe);
            if after_hours {
                builder = builder.after_hours();
            }
            builder.send()
        });
        match result {
            Ok(candles) => types::candles_to_dict(py, &candles),
            Err(e) => Err(errors::to_py_err(e)),
        }
    }

    /// Get intraday trades for a FutOpt contract
    #[pyo3(signature = (symbol, after_hours=false, offset=None, limit=None, is_trial=None, **_extra))]
    pub fn trades_async<'py>(
        &self,
        py: Python<'py>,
        symbol: String,
        after_hours: bool,
        offset: Option<i32>,
        limit: Option<i32>,
        is_trial: Option<bool>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Bound<'py, PyAny>> {
        warn_unknown_kwargs(py, "futopt.intraday.trades", &_extra);
        let client = self.inner.clone();
        future_into_py(py, async move {
            let result = tokio::task::spawn_blocking(move || {
                let futopt = client.futopt();
                let intraday = futopt.intraday();
                let mut builder = intraday.trades().symbol(&symbol);
                if after_hours {
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
                Ok(trades) => Python::attach(|py| types::trades_to_dict(py, &trades)),
                Err(e) => Err(errors::to_py_err(e)),
            }
        })
    }

    /// Sync sibling of `trades()` for legacy fugle-marketdata callers.
    #[pyo3(signature = (symbol, after_hours=false, offset=None, limit=None, is_trial=None, **_extra))]
    pub fn trades(
        &self,
        py: Python<'_>,
        symbol: String,
        after_hours: bool,
        offset: Option<i32>,
        limit: Option<i32>,
        is_trial: Option<bool>, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Py<pyo3::types::PyDict>> {
        warn_unknown_kwargs(py, "futopt.intraday.trades", &_extra);
        let inner = self.inner.clone();
        let result = py.detach(|| {
            let futopt = inner.futopt();
            let intraday = futopt.intraday();
            let mut builder = intraday.trades().symbol(&symbol);
            if after_hours {
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
            Ok(trades) => types::trades_to_dict(py, &trades),
            Err(e) => Err(errors::to_py_err(e)),
        }
    }

    /// Get intraday volumes for a FutOpt contract
    #[pyo3(signature = (symbol, after_hours=false, **_extra))]
    pub fn volumes_async<'py>(
        &self,
        py: Python<'py>,
        symbol: String,
        after_hours: bool, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Bound<'py, PyAny>> {
        warn_unknown_kwargs(py, "futopt.intraday.volumes", &_extra);
        let client = self.inner.clone();
        future_into_py(py, async move {
            let result = tokio::task::spawn_blocking(move || {
                let futopt = client.futopt();
                let intraday = futopt.intraday();
                let mut builder = intraday.volumes().symbol(&symbol);
                if after_hours {
                    builder = builder.after_hours();
                }
                builder.send()
            })
            .await
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Task join error: {}", e)))?;

            match result {
                Ok(volumes) => Python::attach(|py| types::volumes_to_dict(py, &volumes)),
                Err(e) => Err(errors::to_py_err(e)),
            }
        })
    }

    /// Sync sibling of `volumes()` for legacy fugle-marketdata callers.
    #[pyo3(signature = (symbol, after_hours=false, **_extra))]
    pub fn volumes(
        &self,
        py: Python<'_>,
        symbol: String,
        after_hours: bool, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Py<pyo3::types::PyDict>> {
        warn_unknown_kwargs(py, "futopt.intraday.volumes", &_extra);
        let inner = self.inner.clone();
        let result = py.detach(|| {
            let futopt = inner.futopt();
            let intraday = futopt.intraday();
            let mut builder = intraday.volumes().symbol(&symbol);
            if after_hours {
                builder = builder.after_hours();
            }
            builder.send()
        });
        match result {
            Ok(volumes) => types::volumes_to_dict(py, &volumes),
            Err(e) => Err(errors::to_py_err(e)),
        }
    }
}

/// If the caller passed unknown keyword arguments via `**kwargs`, emit a
/// `DeprecationWarning` (non-fatal) and drop them. Matches the 2.4.1 SDK's
/// `**params` interface well enough for drop-in use: known kwargs go through
/// the typed builder; unknown ones are surfaced but don't block.
///
/// A stricter future version could forward unknown kwargs as raw query params
/// via a new core-side extension point.
fn warn_unknown_kwargs(
    py: Python<'_>,
    method: &str,
    extra: &Option<Bound<'_, pyo3::types::PyDict>>,
) {
    let Some(extra) = extra else { return };
    if extra.len() == 0 {
        return;
    }
    let keys: Vec<String> = extra
        .keys()
        .iter()
        .filter_map(|k| k.extract::<String>().ok())
        .collect();
    if keys.is_empty() {
        return;
    }
    let msg = format!(
        "{}(): unrecognized keyword argument(s) {:?} were ignored. Known kwargs are typed; please pass them as named positionals/kwargs.",
        method, keys
    );
    let _ = py
        .import("warnings")
        .and_then(|w| w.call_method1("warn", (msg, py.get_type::<pyo3::exceptions::PyUserWarning>())));
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
    /// Get historical candles for a FutOpt contract
    ///
    /// Args:
    ///     symbol: Contract symbol (e.g., "TXFC4" for TAIEX futures)
    ///     from_date: Start date (YYYY-MM-DD)
    ///     to_date: End date (YYYY-MM-DD)
    ///     timeframe: Timeframe ("D", "W", "M", "1", "5", "10", "15", "30", "60")
    ///     after_hours: Whether to include after-hours session data (default: False)
    ///
    /// Returns:
    ///     Awaitable[dict]: Historical candles data
    ///
    /// Example:
    ///     ```python
    ///     candles = await client.futopt.historical.candles(
    ///         "TXFC4",
    ///         from_date="2024-01-01",
    ///         to_date="2024-01-31",
    ///         timeframe="D"
    ///     )
    ///     ```
    #[pyo3(signature = (symbol, from_date=None, to_date=None, timeframe=None, after_hours=false, **_extra))]
    pub fn candles_async<'py>(
        &self,
        py: Python<'py>,
        symbol: String,
        from_date: Option<String>,
        to_date: Option<String>,
        timeframe: Option<String>,
        after_hours: bool, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Bound<'py, PyAny>> {
        warn_unknown_kwargs(py, "futopt.historical.candles", &_extra);
        let client = self.inner.clone();
        future_into_py(py, async move {
            let result = tokio::task::spawn_blocking(move || {
                let futopt = client.futopt();
                let historical = futopt.historical();
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
                if after_hours {
                    builder = builder.after_hours(true);
                }
                builder.send()
            })
            .await
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Task join error: {}", e)))?;

            match result {
                Ok(candles) => Python::attach(|py| types::futopt_historical_candles_to_dict(py, &candles)),
                Err(e) => Err(errors::to_py_err(e)),
            }
        })
    }

    /// Sync sibling of `candles()` for legacy fugle-marketdata callers.
    #[pyo3(signature = (symbol, from_date=None, to_date=None, timeframe=None, after_hours=false, **_extra))]
    pub fn candles(
        &self,
        py: Python<'_>,
        symbol: String,
        from_date: Option<String>,
        to_date: Option<String>,
        timeframe: Option<String>,
        after_hours: bool, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Py<pyo3::types::PyDict>> {
        warn_unknown_kwargs(py, "futopt.historical.candles", &_extra);
        let inner = self.inner.clone();
        let result = py.detach(|| {
            let futopt = inner.futopt();
            let historical = futopt.historical();
            let mut builder = historical.candles().symbol(&symbol);
            if let Some(f) = from_date { builder = builder.from(&f); }
            if let Some(t) = to_date { builder = builder.to(&t); }
            if let Some(tf) = timeframe { builder = builder.timeframe(&tf); }
            if after_hours { builder = builder.after_hours(true); }
            builder.send()
        });
        match result {
            Ok(candles) => types::futopt_historical_candles_to_dict(py, &candles),
            Err(e) => Err(errors::to_py_err(e)),
        }
    }

    /// Get daily historical data for a FutOpt contract
    ///
    /// Args:
    ///     symbol: Contract symbol (e.g., "TXFC4" for TAIEX futures)
    ///     from_date: Start date (YYYY-MM-DD)
    ///     to_date: End date (YYYY-MM-DD)
    ///     after_hours: Whether to include after-hours session data (default: False)
    ///
    /// Returns:
    ///     Awaitable[dict]: Daily historical data with settlement prices
    ///
    /// Example:
    ///     ```python
    ///     daily = await client.futopt.historical.daily(
    ///         "TXFC4",
    ///         from_date="2024-01-01",
    ///         to_date="2024-01-31"
    ///     )
    ///     ```
    #[pyo3(signature = (symbol, from_date=None, to_date=None, after_hours=false, **_extra))]
    pub fn daily_async<'py>(
        &self,
        py: Python<'py>,
        symbol: String,
        from_date: Option<String>,
        to_date: Option<String>,
        after_hours: bool, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Bound<'py, PyAny>> {
        warn_unknown_kwargs(py, "futopt.historical.daily", &_extra);
        let client = self.inner.clone();
        future_into_py(py, async move {
            let result = tokio::task::spawn_blocking(move || {
                let futopt = client.futopt();
                let historical = futopt.historical();
                let mut builder = historical.daily().symbol(&symbol);
                if let Some(f) = from_date {
                    builder = builder.from(&f);
                }
                if let Some(t) = to_date {
                    builder = builder.to(&t);
                }
                if after_hours {
                    builder = builder.after_hours(true);
                }
                builder.send()
            })
            .await
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Task join error: {}", e)))?;

            match result {
                Ok(daily) => Python::attach(|py| types::futopt_daily_to_dict(py, &daily)),
                Err(e) => Err(errors::to_py_err(e)),
            }
        })
    }

    /// Sync sibling of `daily()` for legacy fugle-marketdata callers.
    #[pyo3(signature = (symbol, from_date=None, to_date=None, after_hours=false, **_extra))]
    pub fn daily(
        &self,
        py: Python<'_>,
        symbol: String,
        from_date: Option<String>,
        to_date: Option<String>,
        after_hours: bool, _extra: Option<Bound<'_, pyo3::types::PyDict>>
    ) -> PyResult<Py<pyo3::types::PyDict>> {
        warn_unknown_kwargs(py, "futopt.historical.daily", &_extra);
        let inner = self.inner.clone();
        let result = py.detach(|| {
            let futopt = inner.futopt();
            let historical = futopt.historical();
            let mut builder = historical.daily().symbol(&symbol);
            if let Some(f) = from_date { builder = builder.from(&f); }
            if let Some(t) = to_date { builder = builder.to(&t); }
            if after_hours { builder = builder.after_hours(true); }
            builder.send()
        });
        match result {
            Ok(daily) => types::futopt_daily_to_dict(py, &daily),
            Err(e) => Err(errors::to_py_err(e)),
        }
    }
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
