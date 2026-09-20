//! REST client wrapper types for UniFFI bindings
//!
//! This module provides Arc-wrapped client types that can be safely passed
//! across FFI boundaries.
//!
//! Every REST method returns the server's JSON response verbatim, as a string.
//! UniFFI cannot carry a dynamic JSON value across the FFI boundary, and the
//! mirrored structs this module used to return were a hand-maintained copy of
//! `marketdata_core::models` that had drifted badly — `FutOptQuote` alone was
//! missing 11 fields. Handing the body over untouched removes that whole class
//! of bug; callers decode it with their platform's JSON library.
//!
//! Every method takes the endpoint's required parameters positionally and
//! the optional ones as one record from [`crate::params`] (#202); the
//! request goes through `RestClient::get_json`, the entry the core keeps for
//! bindings, with the query resolved against `core::rest::params`. Auth,
//! retry and status handling are the typed builders'.
//!
//! Sync variants are provided for simple use cases.

use std::sync::Arc;
use marketdata_core::rest::params::EndpointSpec;
use marketdata_core::{Auth, RestClient as CoreRestClient};
use crate::errors::MarketDataError;
use crate::params::{fields_of, to_query, Field, QueryParams, Value};

/// Serialise a decoded response body back to a JSON string for the FFI boundary.
fn to_json(value: &serde_json::Value) -> Result<String, MarketDataError> {
    serde_json::to_string(value)
        .map_err(|e| crate::errors::other_error(format!("Failed to serialize response: {}", e)))
}

/// One request: the endpoint's path, its path parameter and the fields to
/// send (the positional ones first, then the record's).
struct Request {
    path: &'static [&'static str],
    path_param: Option<String>,
    fields: Vec<Field>,
}

impl Request {
    fn new(path: &'static [&'static str], path_param: Option<String>) -> Self {
        Self { path, path_param, fields: Vec::new() }
    }

    fn with_path_param(path: &'static [&'static str], path_param: String) -> Self {
        Self::new(path, Some(path_param))
    }

    /// A required parameter, sent verbatim.
    fn positional(mut self, canonical: &'static str, value: impl ToString) -> Self {
        self.fields.push((canonical, Value::Str(value.to_string())));
        self
    }

    /// The optional record; `None` sends nothing.
    fn params<P: QueryParams>(mut self, params: Option<P>) -> Self {
        self.fields.extend(fields_of(params));
        self
    }

    /// Resolve the query against the table and send. An unknown field is
    /// 1005 `INVALID_PARAMETER`, before any I/O.
    fn send(self, client: &CoreRestClient) -> Result<String, MarketDataError> {
        let spec = EndpointSpec::for_path(self.path)
            .unwrap_or_else(|| panic!("{} has no entry in core::rest::params", self.path.join("/")));
        let query = to_query(spec, self.fields)?;
        let mut segments: Vec<&str> = self.path.to_vec();
        if let Some(param) = self.path_param.as_deref() {
            segments.push(param);
        }
        let body = client.get_json(&segments, &query)?;
        to_json(&body)
    }

    /// `send` off the async runtime.
    #[cfg(not(feature = "cpp"))]
    async fn send_async(self, client: &CoreRestClient) -> Result<String, MarketDataError> {
        let client = client.clone();
        tokio::task::spawn_blocking(move || self.send(&client))
            .await
            .map_err(|e| crate::errors::other_error(e.to_string()))?
    }
}

// ============================================================================
// RestClient - Main entry point
// ============================================================================

/// REST client for UniFFI bindings
///
/// Wraps the core RestClient and provides Arc-wrapped sub-clients for FFI safety.
#[derive(uniffi::Object)]
pub struct RestClient {
    inner: CoreRestClient,
}

impl RestClient {
    /// Create a new REST client with the given authentication
    pub fn new(auth: Auth) -> Self {
        Self {
            inner: CoreRestClient::new(auth),
        }
    }

    /// Create a new REST client with custom TLS configuration
    pub fn with_tls(
        auth: Auth,
        tls: marketdata_core::TlsConfig,
    ) -> Result<Self, MarketDataError> {
        Ok(Self {
            inner: CoreRestClient::with_tls(auth, tls)?,
        })
    }

    /// Override the base URL (consumes and returns a new instance).
    ///
    /// `url` carries the host and path prefix only — the SDK appends the
    /// version segment. A url that already ends in one is rejected here so
    /// the error surfaces from the factory function that built the client,
    /// not from an unrelated request much later.
    pub(crate) fn with_base_url(self, url: &str) -> Result<Self, MarketDataError> {
        Ok(Self {
            inner: self.inner.try_base_url(url)?,
        })
    }
}

#[uniffi::export]
impl RestClient {
    /// The prefix every request from this client is built on, fully resolved —
    /// host, path prefix and version segment.
    ///
    /// The version segment is chosen by the SDK rather than written by the
    /// caller, so this is the only way to see what a client resolved to.
    pub fn base_url(&self) -> String {
        self.inner.resolved_base_url().to_string()
    }

    /// Access stock-related endpoints
    pub fn stock(&self) -> Arc<StockClient> {
        Arc::new(StockClient::new(self.inner.clone()))
    }

    /// Access FutOpt (futures and options) endpoints
    pub fn futopt(&self) -> Arc<FutOptClient> {
        Arc::new(FutOptClient::new(self.inner.clone()))
    }
}

// ============================================================================
// Stock Client Hierarchy
// ============================================================================

/// Stock market data client
#[derive(uniffi::Object)]
pub struct StockClient {
    inner: CoreRestClient,
}

impl StockClient {
    pub fn new(client: CoreRestClient) -> Self {
        Self { inner: client }
    }
}

#[uniffi::export]
impl StockClient {
    /// Access intraday (real-time) endpoints
    pub fn intraday(&self) -> Arc<StockIntradayClient> {
        Arc::new(StockIntradayClient::new(self.inner.clone()))
    }

    /// Access historical data endpoints
    pub fn historical(&self) -> Arc<StockHistoricalClient> {
        Arc::new(StockHistoricalClient::new(self.inner.clone()))
    }

    /// Access snapshot (market-wide) endpoints
    pub fn snapshot(&self) -> Arc<StockSnapshotClient> {
        Arc::new(StockSnapshotClient::new(self.inner.clone()))
    }

    /// Access technical indicator endpoints
    pub fn technical(&self) -> Arc<StockTechnicalClient> {
        Arc::new(StockTechnicalClient::new(self.inner.clone()))
    }

    /// Access corporate actions endpoints
    pub fn corporate_actions(&self) -> Arc<StockCorporateActionsClient> {
        Arc::new(StockCorporateActionsClient::new(self.inner.clone()))
    }

    /// Access ownership endpoints (ETF holdings, institutional trades, director
    /// holdings, TDCC distribution)
    pub fn ownership(&self) -> Arc<StockOwnershipClient> {
        Arc::new(StockOwnershipClient::new(self.inner.clone()))
    }

    /// The fully resolved request prefix for this product client.
    pub fn base_url(&self) -> String {
        self.inner.resolved_base_url().to_string()
    }
}

// ----------------------------------------------------------------------------
// Stock Intraday
// ----------------------------------------------------------------------------

/// Stock intraday endpoints
///
/// All methods have both async (get_*) and sync (*_sync) variants:
/// - Async methods are preferred for best performance (non-blocking)
/// - Sync methods block the calling thread (simpler API for scripting)
#[derive(uniffi::Object)]
pub struct StockIntradayClient {
    inner: CoreRestClient,
}

impl StockIntradayClient {
    pub fn new(client: CoreRestClient) -> Self {
        Self { inner: client }
    }

    fn tickers(typ: String, params: Option<crate::params::StockTickersParams>) -> Request {
        Request::new(&["stock", "intraday", "tickers"], None)
            .positional("type", typ)
            .params(params)
    }

    fn ticker(symbol: String, params: Option<crate::params::OddLotParams>) -> Request {
        Request::with_path_param(&["stock", "intraday", "ticker"], symbol).params(params)
    }

    fn quote(symbol: String, params: Option<crate::params::OddLotParams>) -> Request {
        Request::with_path_param(&["stock", "intraday", "quote"], symbol).params(params)
    }

    fn candles(symbol: String, params: Option<crate::params::StockCandlesParams>) -> Request {
        Request::with_path_param(&["stock", "intraday", "candles"], symbol).params(params)
    }

    fn trades(symbol: String, params: Option<crate::params::StockTradesParams>) -> Request {
        Request::with_path_param(&["stock", "intraday", "trades"], symbol).params(params)
    }

    fn volumes(symbol: String, params: Option<crate::params::OddLotParams>) -> Request {
        Request::with_path_param(&["stock", "intraday", "volumes"], symbol).params(params)
    }
}

#[cfg(not(feature = "cpp"))]
#[uniffi::export(async_runtime = "tokio")]
impl StockIntradayClient {
    /// Get batch tickers for a security type (async)
    ///
    /// typ: Security type (e.g., "EQUITY", "INDEX", "ETF")
    #[uniffi::method(default(params = None))]
    pub async fn get_tickers(
        &self,
        typ: String,
        params: Option<crate::params::StockTickersParams>,
    ) -> Result<String, MarketDataError> {
        Self::tickers(typ, params).send_async(&self.inner).await
    }

    /// Get ticker info for a symbol (async)
    #[uniffi::method(default(params = None))]
    pub async fn get_ticker(
        &self,
        symbol: String,
        params: Option<crate::params::OddLotParams>,
    ) -> Result<String, MarketDataError> {
        Self::ticker(symbol, params).send_async(&self.inner).await
    }

    /// Get quote for a symbol (async)
    #[uniffi::method(default(params = None))]
    pub async fn get_quote(
        &self,
        symbol: String,
        params: Option<crate::params::OddLotParams>,
    ) -> Result<String, MarketDataError> {
        Self::quote(symbol, params).send_async(&self.inner).await
    }

    /// Get candlestick data for a symbol (async)
    ///
    /// `timeframe` is in the record: unset takes the server default.
    #[uniffi::method(default(params = None))]
    pub async fn get_candles(
        &self,
        symbol: String,
        params: Option<crate::params::StockCandlesParams>,
    ) -> Result<String, MarketDataError> {
        Self::candles(symbol, params).send_async(&self.inner).await
    }

    /// Get trade history for a symbol (async)
    #[uniffi::method(default(params = None))]
    pub async fn get_trades(
        &self,
        symbol: String,
        params: Option<crate::params::StockTradesParams>,
    ) -> Result<String, MarketDataError> {
        Self::trades(symbol, params).send_async(&self.inner).await
    }

    /// Get volume breakdown for a symbol (async)
    #[uniffi::method(default(params = None))]
    pub async fn get_volumes(
        &self,
        symbol: String,
        params: Option<crate::params::OddLotParams>,
    ) -> Result<String, MarketDataError> {
        Self::volumes(symbol, params).send_async(&self.inner).await
    }
}

#[uniffi::export]
impl StockIntradayClient {
    /// Get batch tickers for a security type (sync/blocking)
    ///
    /// typ: Security type (e.g., "EQUITY", "INDEX", "ETF")
    #[uniffi::method(default(params = None))]
    pub fn tickers_sync(
        &self,
        typ: String,
        params: Option<crate::params::StockTickersParams>,
    ) -> Result<String, MarketDataError> {
        Self::tickers(typ, params).send(&self.inner)
    }

    /// Get ticker info for a symbol (sync/blocking)
    #[uniffi::method(default(params = None))]
    pub fn ticker_sync(
        &self,
        symbol: String,
        params: Option<crate::params::OddLotParams>,
    ) -> Result<String, MarketDataError> {
        Self::ticker(symbol, params).send(&self.inner)
    }

    /// Get quote for a symbol (sync/blocking)
    #[uniffi::method(default(params = None))]
    pub fn quote_sync(
        &self,
        symbol: String,
        params: Option<crate::params::OddLotParams>,
    ) -> Result<String, MarketDataError> {
        Self::quote(symbol, params).send(&self.inner)
    }

    /// Get candlestick data for a symbol (sync/blocking)
    #[uniffi::method(default(params = None))]
    pub fn candles_sync(
        &self,
        symbol: String,
        params: Option<crate::params::StockCandlesParams>,
    ) -> Result<String, MarketDataError> {
        Self::candles(symbol, params).send(&self.inner)
    }

    /// Get trade history for a symbol (sync/blocking)
    #[uniffi::method(default(params = None))]
    pub fn trades_sync(
        &self,
        symbol: String,
        params: Option<crate::params::StockTradesParams>,
    ) -> Result<String, MarketDataError> {
        Self::trades(symbol, params).send(&self.inner)
    }

    /// Get volume breakdown for a symbol (sync/blocking)
    #[uniffi::method(default(params = None))]
    pub fn volumes_sync(
        &self,
        symbol: String,
        params: Option<crate::params::OddLotParams>,
    ) -> Result<String, MarketDataError> {
        Self::volumes(symbol, params).send(&self.inner)
    }
}

// ----------------------------------------------------------------------------
// Stock Historical
// ----------------------------------------------------------------------------

/// Stock historical endpoints
#[derive(uniffi::Object)]
pub struct StockHistoricalClient {
    inner: CoreRestClient,
}

impl StockHistoricalClient {
    pub fn new(client: CoreRestClient) -> Self {
        Self { inner: client }
    }

    fn candles(symbol: String, params: Option<crate::params::StockHistoricalCandlesParams>) -> Request {
        Request::with_path_param(&["stock", "historical", "candles"], symbol).params(params)
    }

    fn stats(symbol: String) -> Request {
        Request::with_path_param(&["stock", "historical", "stats"], symbol)
    }
}

#[cfg(not(feature = "cpp"))]
#[uniffi::export(async_runtime = "tokio")]
impl StockHistoricalClient {
    /// Get historical candles for a symbol (async)
    #[uniffi::method(default(params = None))]
    pub async fn get_candles(
        &self,
        symbol: String,
        params: Option<crate::params::StockHistoricalCandlesParams>,
    ) -> Result<String, MarketDataError> {
        Self::candles(symbol, params).send_async(&self.inner).await
    }

    /// Get historical stats for a symbol (async)
    ///
    /// Returns summary statistics including 52-week high/low
    pub async fn get_stats(&self, symbol: String) -> Result<String, MarketDataError> {
        Self::stats(symbol).send_async(&self.inner).await
    }
}

#[uniffi::export]
impl StockHistoricalClient {
    /// Get historical candles for a symbol (sync/blocking)
    #[uniffi::method(default(params = None))]
    pub fn candles_sync(
        &self,
        symbol: String,
        params: Option<crate::params::StockHistoricalCandlesParams>,
    ) -> Result<String, MarketDataError> {
        Self::candles(symbol, params).send(&self.inner)
    }

    /// Get historical stats for a symbol (sync/blocking)
    pub fn stats_sync(&self, symbol: String) -> Result<String, MarketDataError> {
        Self::stats(symbol).send(&self.inner)
    }
}

// ----------------------------------------------------------------------------
// Stock Snapshot
// ----------------------------------------------------------------------------

/// Stock snapshot endpoints for market-wide data
///
/// Provides access to quotes, movers (gainers/losers), and most active stocks
/// across entire markets.
#[derive(uniffi::Object)]
pub struct StockSnapshotClient {
    inner: CoreRestClient,
}

impl StockSnapshotClient {
    pub fn new(client: CoreRestClient) -> Self {
        Self { inner: client }
    }

    fn quotes(market: String, params: Option<crate::params::SnapshotParams>) -> Request {
        Request::with_path_param(&["stock", "snapshot", "quotes"], market).params(params)
    }

    fn movers(
        market: String,
        direction: String,
        change: String,
        params: Option<crate::params::MoversParams>,
    ) -> Request {
        Request::with_path_param(&["stock", "snapshot", "movers"], market)
            .positional("direction", direction)
            .positional("change", change)
            .params(params)
    }

    fn actives(market: String, trade: String, params: Option<crate::params::SnapshotParams>) -> Request {
        Request::with_path_param(&["stock", "snapshot", "actives"], market)
            .positional("trade", trade)
            .params(params)
    }
}

#[cfg(not(feature = "cpp"))]
#[uniffi::export(async_runtime = "tokio")]
impl StockSnapshotClient {
    /// Get market-wide snapshot quotes (async)
    ///
    /// market: TSE, OTC, ESB, TIB or PSB
    #[uniffi::method(default(params = None))]
    pub async fn get_quotes(
        &self,
        market: String,
        params: Option<crate::params::SnapshotParams>,
    ) -> Result<String, MarketDataError> {
        Self::quotes(market, params).send_async(&self.inner).await
    }

    /// Get top movers (gainers/losers) in a market (async)
    ///
    /// direction: "up" for gainers, "down" for losers;
    /// change: "percent" or "value"
    #[uniffi::method(default(params = None))]
    pub async fn get_movers(
        &self,
        market: String,
        direction: String,
        change: String,
        params: Option<crate::params::MoversParams>,
    ) -> Result<String, MarketDataError> {
        Self::movers(market, direction, change, params).send_async(&self.inner).await
    }

    /// Get most actively traded stocks (async)
    ///
    /// trade: "volume" or "value"
    #[uniffi::method(default(params = None))]
    pub async fn get_actives(
        &self,
        market: String,
        trade: String,
        params: Option<crate::params::SnapshotParams>,
    ) -> Result<String, MarketDataError> {
        Self::actives(market, trade, params).send_async(&self.inner).await
    }
}

#[uniffi::export]
impl StockSnapshotClient {
    /// Get market-wide snapshot quotes (sync/blocking)
    #[uniffi::method(default(params = None))]
    pub fn quotes_sync(
        &self,
        market: String,
        params: Option<crate::params::SnapshotParams>,
    ) -> Result<String, MarketDataError> {
        Self::quotes(market, params).send(&self.inner)
    }

    /// Get top movers (sync/blocking)
    #[uniffi::method(default(params = None))]
    pub fn movers_sync(
        &self,
        market: String,
        direction: String,
        change: String,
        params: Option<crate::params::MoversParams>,
    ) -> Result<String, MarketDataError> {
        Self::movers(market, direction, change, params).send(&self.inner)
    }

    /// Get most actively traded stocks (sync/blocking)
    #[uniffi::method(default(params = None))]
    pub fn actives_sync(
        &self,
        market: String,
        trade: String,
        params: Option<crate::params::SnapshotParams>,
    ) -> Result<String, MarketDataError> {
        Self::actives(market, trade, params).send(&self.inner)
    }
}

// ----------------------------------------------------------------------------
// Stock Technical
// ----------------------------------------------------------------------------

/// Stock technical indicator endpoints
///
/// Provides access to SMA, RSI, KDJ, MACD, and Bollinger Bands indicators.
/// The periods are required by the server and so are positional; the date
/// range is the record.
#[derive(uniffi::Object)]
pub struct StockTechnicalClient {
    inner: CoreRestClient,
}

impl StockTechnicalClient {
    pub fn new(client: CoreRestClient) -> Self {
        Self { inner: client }
    }

    fn sma(symbol: String, period: u32, params: Option<crate::params::TechnicalParams>) -> Request {
        Request::with_path_param(&["stock", "technical", "sma"], symbol)
            .positional("period", period)
            .params(params)
    }

    fn rsi(symbol: String, period: u32, params: Option<crate::params::TechnicalParams>) -> Request {
        Request::with_path_param(&["stock", "technical", "rsi"], symbol)
            .positional("period", period)
            .params(params)
    }

    fn kdj(
        symbol: String,
        r_period: u32,
        k_period: u32,
        d_period: u32,
        params: Option<crate::params::TechnicalParams>,
    ) -> Request {
        Request::with_path_param(&["stock", "technical", "kdj"], symbol)
            .positional("r_period", r_period)
            .positional("k_period", k_period)
            .positional("d_period", d_period)
            .params(params)
    }

    fn macd(
        symbol: String,
        fast: u32,
        slow: u32,
        signal: u32,
        params: Option<crate::params::TechnicalParams>,
    ) -> Request {
        Request::with_path_param(&["stock", "technical", "macd"], symbol)
            .positional("fast", fast)
            .positional("slow", slow)
            .positional("signal", signal)
            .params(params)
    }

    fn bb(symbol: String, period: u32, params: Option<crate::params::TechnicalParams>) -> Request {
        Request::with_path_param(&["stock", "technical", "bb"], symbol)
            .positional("period", period)
            .params(params)
    }
}

#[cfg(not(feature = "cpp"))]
#[uniffi::export(async_runtime = "tokio")]
impl StockTechnicalClient {
    /// Get Simple Moving Average (async)
    #[uniffi::method(default(params = None))]
    pub async fn get_sma(
        &self,
        symbol: String,
        period: u32,
        params: Option<crate::params::TechnicalParams>,
    ) -> Result<String, MarketDataError> {
        Self::sma(symbol, period, params).send_async(&self.inner).await
    }

    /// Get Relative Strength Index (async)
    #[uniffi::method(default(params = None))]
    pub async fn get_rsi(
        &self,
        symbol: String,
        period: u32,
        params: Option<crate::params::TechnicalParams>,
    ) -> Result<String, MarketDataError> {
        Self::rsi(symbol, period, params).send_async(&self.inner).await
    }

    /// Get KDJ (Stochastic Oscillator) (async)
    #[uniffi::method(default(params = None))]
    pub async fn get_kdj(
        &self,
        symbol: String,
        r_period: u32,
        k_period: u32,
        d_period: u32,
        params: Option<crate::params::TechnicalParams>,
    ) -> Result<String, MarketDataError> {
        Self::kdj(symbol, r_period, k_period, d_period, params).send_async(&self.inner).await
    }

    /// Get MACD indicator (async)
    #[uniffi::method(default(params = None))]
    pub async fn get_macd(
        &self,
        symbol: String,
        fast: u32,
        slow: u32,
        signal: u32,
        params: Option<crate::params::TechnicalParams>,
    ) -> Result<String, MarketDataError> {
        Self::macd(symbol, fast, slow, signal, params).send_async(&self.inner).await
    }

    /// Get Bollinger Bands (async)
    #[uniffi::method(default(params = None))]
    pub async fn get_bb(
        &self,
        symbol: String,
        period: u32,
        params: Option<crate::params::TechnicalParams>,
    ) -> Result<String, MarketDataError> {
        Self::bb(symbol, period, params).send_async(&self.inner).await
    }
}

#[uniffi::export]
impl StockTechnicalClient {
    /// Get Simple Moving Average (sync/blocking)
    #[uniffi::method(default(params = None))]
    pub fn sma_sync(
        &self,
        symbol: String,
        period: u32,
        params: Option<crate::params::TechnicalParams>,
    ) -> Result<String, MarketDataError> {
        Self::sma(symbol, period, params).send(&self.inner)
    }

    /// Get Relative Strength Index (sync/blocking)
    #[uniffi::method(default(params = None))]
    pub fn rsi_sync(
        &self,
        symbol: String,
        period: u32,
        params: Option<crate::params::TechnicalParams>,
    ) -> Result<String, MarketDataError> {
        Self::rsi(symbol, period, params).send(&self.inner)
    }

    /// Get KDJ (sync/blocking)
    #[uniffi::method(default(params = None))]
    pub fn kdj_sync(
        &self,
        symbol: String,
        r_period: u32,
        k_period: u32,
        d_period: u32,
        params: Option<crate::params::TechnicalParams>,
    ) -> Result<String, MarketDataError> {
        Self::kdj(symbol, r_period, k_period, d_period, params).send(&self.inner)
    }

    /// Get MACD (sync/blocking)
    #[uniffi::method(default(params = None))]
    pub fn macd_sync(
        &self,
        symbol: String,
        fast: u32,
        slow: u32,
        signal: u32,
        params: Option<crate::params::TechnicalParams>,
    ) -> Result<String, MarketDataError> {
        Self::macd(symbol, fast, slow, signal, params).send(&self.inner)
    }

    /// Get Bollinger Bands (sync/blocking)
    #[uniffi::method(default(params = None))]
    pub fn bb_sync(
        &self,
        symbol: String,
        period: u32,
        params: Option<crate::params::TechnicalParams>,
    ) -> Result<String, MarketDataError> {
        Self::bb(symbol, period, params).send(&self.inner)
    }
}

// ----------------------------------------------------------------------------
// Stock Corporate Actions
// ----------------------------------------------------------------------------

/// Stock corporate actions endpoints
///
/// Provides access to capital changes, dividends, and listing applicants (IPO).
/// One record serves all three; `capital-changes` has no `exchange`, so
/// setting it there is 1005 `INVALID_PARAMETER`.
#[derive(uniffi::Object)]
pub struct StockCorporateActionsClient {
    inner: CoreRestClient,
}

impl StockCorporateActionsClient {
    pub fn new(client: CoreRestClient) -> Self {
        Self { inner: client }
    }

    fn capital_changes(params: Option<crate::params::CorporateActionsParams>) -> Request {
        Request::new(&["stock", "corporate-actions", "capital-changes"], None).params(params)
    }

    fn dividends(params: Option<crate::params::CorporateActionsParams>) -> Request {
        Request::new(&["stock", "corporate-actions", "dividends"], None).params(params)
    }

    fn listing_applicants(params: Option<crate::params::CorporateActionsParams>) -> Request {
        Request::new(&["stock", "corporate-actions", "listing-applicants"], None).params(params)
    }
}

#[cfg(not(feature = "cpp"))]
#[uniffi::export(async_runtime = "tokio")]
impl StockCorporateActionsClient {
    /// Get capital structure changes (async)
    #[uniffi::method(default(params = None))]
    pub async fn get_capital_changes(
        &self,
        params: Option<crate::params::CorporateActionsParams>,
    ) -> Result<String, MarketDataError> {
        Self::capital_changes(params).send_async(&self.inner).await
    }

    /// Get dividend announcements (async)
    #[uniffi::method(default(params = None))]
    pub async fn get_dividends(
        &self,
        params: Option<crate::params::CorporateActionsParams>,
    ) -> Result<String, MarketDataError> {
        Self::dividends(params).send_async(&self.inner).await
    }

    /// Get IPO listing applicants (async)
    #[uniffi::method(default(params = None))]
    pub async fn get_listing_applicants(
        &self,
        params: Option<crate::params::CorporateActionsParams>,
    ) -> Result<String, MarketDataError> {
        Self::listing_applicants(params).send_async(&self.inner).await
    }
}

#[uniffi::export]
impl StockCorporateActionsClient {
    /// Get capital structure changes (sync/blocking)
    #[uniffi::method(default(params = None))]
    pub fn capital_changes_sync(
        &self,
        params: Option<crate::params::CorporateActionsParams>,
    ) -> Result<String, MarketDataError> {
        Self::capital_changes(params).send(&self.inner)
    }

    /// Get dividend announcements (sync/blocking)
    #[uniffi::method(default(params = None))]
    pub fn dividends_sync(
        &self,
        params: Option<crate::params::CorporateActionsParams>,
    ) -> Result<String, MarketDataError> {
        Self::dividends(params).send(&self.inner)
    }

    /// Get IPO listing applicants (sync/blocking)
    #[uniffi::method(default(params = None))]
    pub fn listing_applicants_sync(
        &self,
        params: Option<crate::params::CorporateActionsParams>,
    ) -> Result<String, MarketDataError> {
        Self::listing_applicants(params).send(&self.inner)
    }
}

// ----------------------------------------------------------------------------
// Stock Ownership
// ----------------------------------------------------------------------------

/// Stock ownership endpoints client
#[derive(uniffi::Object)]
pub struct StockOwnershipClient {
    inner: CoreRestClient,
}

impl StockOwnershipClient {
    pub fn new(client: CoreRestClient) -> Self {
        Self { inner: client }
    }

    fn etf_holdings(symbol: String, params: Option<crate::params::OwnershipParams>) -> Request {
        Request::with_path_param(&["stock", "ownership", "etf-holdings"], symbol).params(params)
    }

    fn institutional_trades(symbol: String, params: Option<crate::params::OwnershipParams>) -> Request {
        Request::with_path_param(&["stock", "ownership", "institutional-trades"], symbol).params(params)
    }

    fn director_holdings(symbol: String, params: Option<crate::params::OwnershipParams>) -> Request {
        Request::with_path_param(&["stock", "ownership", "director-holdings"], symbol).params(params)
    }

    fn tdcc_distribution(symbol: String, params: Option<crate::params::OwnershipParams>) -> Request {
        Request::with_path_param(&["stock", "ownership", "tdcc-distribution"], symbol).params(params)
    }
}

#[cfg(not(feature = "cpp"))]
#[uniffi::export(async_runtime = "tokio")]
impl StockOwnershipClient {
    /// Get the constituents an ETF held over a date range (async)
    #[uniffi::method(default(params = None))]
    pub async fn get_etf_holdings(
        &self,
        symbol: String,
        params: Option<crate::params::OwnershipParams>,
    ) -> Result<String, MarketDataError> {
        Self::etf_holdings(symbol, params).send_async(&self.inner).await
    }

    /// Get daily trading by the three major institutional investors (async)
    #[uniffi::method(default(params = None))]
    pub async fn get_institutional_trades(
        &self,
        symbol: String,
        params: Option<crate::params::OwnershipParams>,
    ) -> Result<String, MarketDataError> {
        Self::institutional_trades(symbol, params).send_async(&self.inner).await
    }

    /// Get monthly holdings and pledges disclosed by directors and supervisors (async)
    #[uniffi::method(default(params = None))]
    pub async fn get_director_holdings(
        &self,
        symbol: String,
        params: Option<crate::params::OwnershipParams>,
    ) -> Result<String, MarketDataError> {
        Self::director_holdings(symbol, params).send_async(&self.inner).await
    }

    /// Get the weekly TDCC shareholder distribution by holding-size bracket (async)
    #[uniffi::method(default(params = None))]
    pub async fn get_tdcc_distribution(
        &self,
        symbol: String,
        params: Option<crate::params::OwnershipParams>,
    ) -> Result<String, MarketDataError> {
        Self::tdcc_distribution(symbol, params).send_async(&self.inner).await
    }
}

#[uniffi::export]
impl StockOwnershipClient {
    /// Get the constituents an ETF held over a date range (sync/blocking)
    #[uniffi::method(default(params = None))]
    pub fn etf_holdings_sync(
        &self,
        symbol: String,
        params: Option<crate::params::OwnershipParams>,
    ) -> Result<String, MarketDataError> {
        Self::etf_holdings(symbol, params).send(&self.inner)
    }

    /// Get daily trading by the three major institutional investors (sync/blocking)
    #[uniffi::method(default(params = None))]
    pub fn institutional_trades_sync(
        &self,
        symbol: String,
        params: Option<crate::params::OwnershipParams>,
    ) -> Result<String, MarketDataError> {
        Self::institutional_trades(symbol, params).send(&self.inner)
    }

    /// Get monthly holdings and pledges disclosed by directors and supervisors (sync/blocking)
    #[uniffi::method(default(params = None))]
    pub fn director_holdings_sync(
        &self,
        symbol: String,
        params: Option<crate::params::OwnershipParams>,
    ) -> Result<String, MarketDataError> {
        Self::director_holdings(symbol, params).send(&self.inner)
    }

    /// Get the weekly TDCC shareholder distribution by holding-size bracket (sync/blocking)
    #[uniffi::method(default(params = None))]
    pub fn tdcc_distribution_sync(
        &self,
        symbol: String,
        params: Option<crate::params::OwnershipParams>,
    ) -> Result<String, MarketDataError> {
        Self::tdcc_distribution(symbol, params).send(&self.inner)
    }
}

// ============================================================================
// FutOpt Client Hierarchy
// ============================================================================

/// FutOpt market data client
#[derive(uniffi::Object)]
pub struct FutOptClient {
    inner: CoreRestClient,
}

impl FutOptClient {
    pub fn new(client: CoreRestClient) -> Self {
        Self { inner: client }
    }
}

#[uniffi::export]
impl FutOptClient {
    /// Access intraday (real-time) endpoints
    pub fn intraday(&self) -> Arc<FutOptIntradayClient> {
        Arc::new(FutOptIntradayClient::new(self.inner.clone()))
    }

    /// Access historical data endpoints
    pub fn historical(&self) -> Arc<FutOptHistoricalClient> {
        Arc::new(FutOptHistoricalClient::new(self.inner.clone()))
    }
}

// ----------------------------------------------------------------------------
// FutOpt Intraday
// ----------------------------------------------------------------------------

/// FutOpt intraday endpoints
#[derive(uniffi::Object)]
pub struct FutOptIntradayClient {
    inner: CoreRestClient,
}

impl FutOptIntradayClient {
    pub fn new(client: CoreRestClient) -> Self {
        Self { inner: client }
    }

    fn products(typ: &str, params: Option<crate::params::FutOptProductsParams>) -> Result<Request, MarketDataError> {
        let typ = parse_futopt_type(typ)?;
        Ok(Request::new(&["futopt", "intraday", "products"], None)
            .positional("type", typ)
            .params(params))
    }

    fn tickers(typ: &str, params: Option<crate::params::FutOptTickersParams>) -> Result<Request, MarketDataError> {
        let typ = parse_futopt_type(typ)?;
        Ok(Request::new(&["futopt", "intraday", "tickers"], None)
            .positional("type", typ)
            .params(params))
    }

    fn ticker(symbol: String, params: Option<crate::params::AfterHoursParams>) -> Request {
        Request::with_path_param(&["futopt", "intraday", "ticker"], symbol).params(params)
    }

    fn quote(symbol: String, params: Option<crate::params::AfterHoursParams>) -> Request {
        Request::with_path_param(&["futopt", "intraday", "quote"], symbol).params(params)
    }

    fn candles(symbol: String, params: Option<crate::params::FutOptCandlesParams>) -> Request {
        Request::with_path_param(&["futopt", "intraday", "candles"], symbol).params(params)
    }

    fn trades(symbol: String, params: Option<crate::params::FutOptTradesParams>) -> Request {
        Request::with_path_param(&["futopt", "intraday", "trades"], symbol).params(params)
    }

    fn volumes(symbol: String, params: Option<crate::params::AfterHoursParams>) -> Request {
        Request::with_path_param(&["futopt", "intraday", "volumes"], symbol).params(params)
    }
}

#[cfg(not(feature = "cpp"))]
#[uniffi::export(async_runtime = "tokio")]
impl FutOptIntradayClient {
    /// Get available products list (async)
    ///
    /// typ: "F" for futures, "O" for options
    #[uniffi::method(default(params = None))]
    pub async fn get_products(
        &self,
        typ: String,
        params: Option<crate::params::FutOptProductsParams>,
    ) -> Result<String, MarketDataError> {
        Self::products(&typ, params)?.send_async(&self.inner).await
    }

    /// Get batch tickers for futures/options (async)
    ///
    /// typ: "F" for futures, "O" for options
    #[uniffi::method(default(params = None))]
    pub async fn get_tickers(
        &self,
        typ: String,
        params: Option<crate::params::FutOptTickersParams>,
    ) -> Result<String, MarketDataError> {
        Self::tickers(&typ, params)?.send_async(&self.inner).await
    }

    /// Get ticker info for a contract (async)
    #[uniffi::method(default(params = None))]
    pub async fn get_ticker(
        &self,
        symbol: String,
        params: Option<crate::params::AfterHoursParams>,
    ) -> Result<String, MarketDataError> {
        Self::ticker(symbol, params).send_async(&self.inner).await
    }

    /// Get quote for a futures/options contract (async)
    #[uniffi::method(default(params = None))]
    pub async fn get_quote(
        &self,
        symbol: String,
        params: Option<crate::params::AfterHoursParams>,
    ) -> Result<String, MarketDataError> {
        Self::quote(symbol, params).send_async(&self.inner).await
    }

    /// Get candlestick data for a futures/options contract (async)
    #[uniffi::method(default(params = None))]
    pub async fn get_candles(
        &self,
        symbol: String,
        params: Option<crate::params::FutOptCandlesParams>,
    ) -> Result<String, MarketDataError> {
        Self::candles(symbol, params).send_async(&self.inner).await
    }

    /// Get trade history for a futures/options contract (async)
    #[uniffi::method(default(params = None))]
    pub async fn get_trades(
        &self,
        symbol: String,
        params: Option<crate::params::FutOptTradesParams>,
    ) -> Result<String, MarketDataError> {
        Self::trades(symbol, params).send_async(&self.inner).await
    }

    /// Get volume breakdown by price for a futures/options contract (async)
    #[uniffi::method(default(params = None))]
    pub async fn get_volumes(
        &self,
        symbol: String,
        params: Option<crate::params::AfterHoursParams>,
    ) -> Result<String, MarketDataError> {
        Self::volumes(symbol, params).send_async(&self.inner).await
    }
}

#[uniffi::export]
impl FutOptIntradayClient {
    /// Get available products list (sync/blocking)
    ///
    /// typ: "F" for futures, "O" for options
    #[uniffi::method(default(params = None))]
    pub fn products_sync(
        &self,
        typ: String,
        params: Option<crate::params::FutOptProductsParams>,
    ) -> Result<String, MarketDataError> {
        Self::products(&typ, params)?.send(&self.inner)
    }

    /// Get batch tickers for futures/options (sync/blocking)
    ///
    /// typ: "F" for futures, "O" for options
    #[uniffi::method(default(params = None))]
    pub fn tickers_sync(
        &self,
        typ: String,
        params: Option<crate::params::FutOptTickersParams>,
    ) -> Result<String, MarketDataError> {
        Self::tickers(&typ, params)?.send(&self.inner)
    }

    /// Get ticker info for a contract (sync/blocking)
    #[uniffi::method(default(params = None))]
    pub fn ticker_sync(
        &self,
        symbol: String,
        params: Option<crate::params::AfterHoursParams>,
    ) -> Result<String, MarketDataError> {
        Self::ticker(symbol, params).send(&self.inner)
    }

    /// Get quote for a futures/options contract (sync/blocking)
    #[uniffi::method(default(params = None))]
    pub fn quote_sync(
        &self,
        symbol: String,
        params: Option<crate::params::AfterHoursParams>,
    ) -> Result<String, MarketDataError> {
        Self::quote(symbol, params).send(&self.inner)
    }

    /// Get candlestick data for a contract (sync/blocking)
    #[uniffi::method(default(params = None))]
    pub fn candles_sync(
        &self,
        symbol: String,
        params: Option<crate::params::FutOptCandlesParams>,
    ) -> Result<String, MarketDataError> {
        Self::candles(symbol, params).send(&self.inner)
    }

    /// Get trade history for a contract (sync/blocking)
    #[uniffi::method(default(params = None))]
    pub fn trades_sync(
        &self,
        symbol: String,
        params: Option<crate::params::FutOptTradesParams>,
    ) -> Result<String, MarketDataError> {
        Self::trades(symbol, params).send(&self.inner)
    }

    /// Get volume breakdown by price for a contract (sync/blocking)
    #[uniffi::method(default(params = None))]
    pub fn volumes_sync(
        &self,
        symbol: String,
        params: Option<crate::params::AfterHoursParams>,
    ) -> Result<String, MarketDataError> {
        Self::volumes(symbol, params).send(&self.inner)
    }
}

// ----------------------------------------------------------------------------
// FutOpt Historical
// ----------------------------------------------------------------------------

/// FutOpt historical data endpoints
///
/// Provides access to historical candles and daily data for futures and options.
#[derive(uniffi::Object)]
pub struct FutOptHistoricalClient {
    inner: CoreRestClient,
}

impl FutOptHistoricalClient {
    pub fn new(client: CoreRestClient) -> Self {
        Self { inner: client }
    }

    fn candles(symbol: String, params: Option<crate::params::FutOptHistoricalCandlesParams>) -> Request {
        Request::with_path_param(&["futopt", "historical", "candles"], symbol).params(params)
    }

    fn daily(symbol: String, params: Option<crate::params::FutOptDailyParams>) -> Request {
        Request::with_path_param(&["futopt", "historical", "daily"], symbol).params(params)
    }
}

#[cfg(not(feature = "cpp"))]
#[uniffi::export(async_runtime = "tokio")]
impl FutOptHistoricalClient {
    /// Get historical candles for a product such as "TXF" (async)
    #[uniffi::method(default(params = None))]
    pub async fn get_candles(
        &self,
        symbol: String,
        params: Option<crate::params::FutOptHistoricalCandlesParams>,
    ) -> Result<String, MarketDataError> {
        Self::candles(symbol, params).send_async(&self.inner).await
    }

    /// Get one trading day's daily quotes for every contract month of a product such as "TXF" (async)
    #[uniffi::method(default(params = None))]
    pub async fn get_daily(
        &self,
        symbol: String,
        params: Option<crate::params::FutOptDailyParams>,
    ) -> Result<String, MarketDataError> {
        Self::daily(symbol, params).send_async(&self.inner).await
    }
}

#[uniffi::export]
impl FutOptHistoricalClient {
    /// Get historical candles for a product such as "TXF" (sync/blocking)
    #[uniffi::method(default(params = None))]
    pub fn candles_sync(
        &self,
        symbol: String,
        params: Option<crate::params::FutOptHistoricalCandlesParams>,
    ) -> Result<String, MarketDataError> {
        Self::candles(symbol, params).send(&self.inner)
    }

    /// Get one trading day's daily quotes for every contract month of a product such as "TXF" (sync/blocking)
    #[uniffi::method(default(params = None))]
    pub fn daily_sync(
        &self,
        symbol: String,
        params: Option<crate::params::FutOptDailyParams>,
    ) -> Result<String, MarketDataError> {
        Self::daily(symbol, params).send(&self.inner)
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Parse FutOpt type from string
/// "F" or "futures" -> FutOptType::Future
/// "O" or "options" -> FutOptType::Option
fn parse_futopt_type(typ: &str) -> Result<marketdata_core::FutOptType, MarketDataError> {
    use marketdata_core::FutOptType;
    match typ.to_uppercase().as_str() {
        "F" | "FUTURE" | "FUTURES" => Ok(FutOptType::Future),
        "O" | "OPTION" | "OPTIONS" => Ok(FutOptType::Option),
        _ => Err(crate::errors::config_error(format!(
            "Invalid FutOpt type: '{}'. Use 'F' for futures or 'O' for options.",
            typ
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::*;

    #[test]
    fn test_rest_client_creation() {
        let client = RestClient::new(Auth::SdkToken("test-token".to_string()));
        let _ = client.stock();
        let _ = client.futopt();
    }

    #[test]
    fn test_stock_client_chain() {
        let client = RestClient::new(Auth::SdkToken("test-token".to_string()));
        let stock = client.stock();
        let _intraday = stock.intraday();
    }

    #[test]
    fn test_futopt_client_chain() {
        let client = RestClient::new(Auth::SdkToken("test-token".to_string()));
        let futopt = client.futopt();
        let _intraday = futopt.intraday();
    }

    #[test]
    fn test_parse_futopt_type() {
        assert!(matches!(parse_futopt_type("F"), Ok(marketdata_core::FutOptType::Future)));
        assert!(matches!(parse_futopt_type("O"), Ok(marketdata_core::FutOptType::Option)));
        assert!(matches!(parse_futopt_type("future"), Ok(marketdata_core::FutOptType::Future)));
        assert!(matches!(parse_futopt_type("options"), Ok(marketdata_core::FutOptType::Option)));
        assert!(parse_futopt_type("invalid").is_err());
    }

    /// Every method's path is in core's table: `Request::send` panics on
    /// one that is not, and only a few paths are exercised by the other
    /// tests.
    #[test]
    fn every_request_path_is_in_the_table() {
        let requests = vec![
            StockIntradayClient::tickers("EQUITY".into(), None),
            StockIntradayClient::ticker("2330".into(), None),
            StockIntradayClient::quote("2330".into(), None),
            StockIntradayClient::candles("2330".into(), None),
            StockIntradayClient::trades("2330".into(), None),
            StockIntradayClient::volumes("2330".into(), None),
            StockHistoricalClient::candles("2330".into(), None),
            StockHistoricalClient::stats("2330".into()),
            StockSnapshotClient::quotes("TSE".into(), None),
            StockSnapshotClient::movers("TSE".into(), "up".into(), "percent".into(), None),
            StockSnapshotClient::actives("TSE".into(), "volume".into(), None),
            StockTechnicalClient::sma("2330".into(), 5, None),
            StockTechnicalClient::rsi("2330".into(), 14, None),
            StockTechnicalClient::kdj("2330".into(), 9, 3, 3, None),
            StockTechnicalClient::macd("2330".into(), 12, 26, 9, None),
            StockTechnicalClient::bb("2330".into(), 20, None),
            StockCorporateActionsClient::capital_changes(None),
            StockCorporateActionsClient::dividends(None),
            StockCorporateActionsClient::listing_applicants(None),
            StockOwnershipClient::etf_holdings("0050".into(), None),
            StockOwnershipClient::institutional_trades("2330".into(), None),
            StockOwnershipClient::director_holdings("2330".into(), None),
            StockOwnershipClient::tdcc_distribution("2330".into(), None),
            FutOptIntradayClient::products("F", None).unwrap(),
            FutOptIntradayClient::tickers("O", None).unwrap(),
            FutOptIntradayClient::ticker("TXFE6".into(), None),
            FutOptIntradayClient::quote("TXFE6".into(), None),
            FutOptIntradayClient::candles("TXFE6".into(), None),
            FutOptIntradayClient::trades("TXFE6".into(), None),
            FutOptIntradayClient::volumes("TXFE6".into(), None),
            FutOptHistoricalClient::candles("TXF".into(), None),
            FutOptHistoricalClient::daily("TXF".into(), None),
        ];
        assert_eq!(requests.len(), marketdata_core::rest::params::ENDPOINTS.len());
        for request in requests {
            let spec = EndpointSpec::for_path(request.path)
                .unwrap_or_else(|| panic!("{} is not in the table", request.path.join("/")));
            // The positional fields resolve, so a required key is never sent
            // under a name the table does not know.
            to_query(spec, request.fields).expect("positional fields resolve");
        }
    }

    fn query(request: Request) -> Vec<(&'static str, String)> {
        let spec = EndpointSpec::for_path(request.path).expect("in the table");
        to_query(spec, request.fields).expect("valid")
    }

    #[test]
    fn positional_parameters_come_first_and_resolve_through_the_table() {
        let request = StockTechnicalClient::kdj(
            "2330".into(),
            9,
            3,
            3,
            Some(TechnicalParams { timeframe: Some("D".into()), ..Default::default() }),
        );
        assert_eq!(request.path_param.as_deref(), Some("2330"));
        assert_eq!(
            query(request),
            vec![
                ("rPeriod", "9".to_string()),
                ("kPeriod", "3".to_string()),
                ("dPeriod", "3".to_string()),
                ("timeframe", "D".to_string()),
            ]
        );
        assert_eq!(
            query(StockSnapshotClient::movers("TSE".into(), "up".into(), "percent".into(), None)),
            vec![("direction", "up".to_string()), ("change", "percent".to_string())]
        );
        assert_eq!(
            query(StockIntradayClient::tickers("EQUITY".into(), None)),
            vec![("type", "EQUITY".to_string())]
        );
    }

    #[test]
    fn futopt_type_is_parsed_and_sent_in_full() {
        let request = FutOptIntradayClient::products("o", None).expect("valid type");
        assert_eq!(query(request), vec![("type", "OPTION".to_string())]);
        let request = FutOptIntradayClient::tickers(
            "F",
            Some(FutOptTickersParams { after_hours: Some(true), ..Default::default() }),
        )
        .expect("valid type");
        assert_eq!(
            query(request),
            vec![("type", "FUTURE".to_string()), ("session", "AFTERHOURS".to_string())]
        );
        assert!(FutOptIntradayClient::products("X", None).is_err());
    }

    /// The unknown-key check runs before any I/O: no server is listening.
    #[test]
    fn capital_changes_with_exchange_is_invalid_parameter_before_sending() {
        let client = RestClient::new(Auth::SdkToken("test-token".to_string()))
            .with_base_url("http://127.0.0.1:9")
            .expect("base url");
        let params = CorporateActionsParams { exchange: Some("TWSE".into()), ..Default::default() };
        match client.stock().corporate_actions().capital_changes_sync(Some(params)) {
            Err(MarketDataError::ApiError { info, .. }) => {
                assert_eq!(info.code, marketdata_core::error_code::INVALID_PARAMETER)
            }
            other => panic!("expected INVALID_PARAMETER, got {other:?}"),
        }
    }
}
