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
//! Sync variants are provided for simple use cases.

use std::sync::Arc;
use marketdata_core::{Auth, RestClient as CoreRestClient};
use crate::errors::MarketDataError;

/// Serialise a decoded response body back to a JSON string for the FFI boundary.
fn to_json(value: &serde_json::Value) -> Result<String, MarketDataError> {
    serde_json::to_string(value).map_err(|e| MarketDataError::Other {
        msg: format!("Failed to serialize response: {}", e),
    })
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

/// Stock intraday endpoints with typed model returns
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
}

#[cfg(not(feature = "cpp"))]
#[uniffi::export(async_runtime = "tokio")]
impl StockIntradayClient {
    // ========== Async Methods (Primary) ==========

    /// Get quote for a symbol (async)
    ///
    /// Returns typed Quote model with all fields directly accessible.
    pub async fn get_quote(&self, symbol: String) -> Result<String, MarketDataError> {
        let inner = self.inner.clone();
        let result = tokio::task::spawn_blocking(move || {
            inner.stock().intraday().quote().symbol(&symbol).send()
        })
        .await
        .map_err(|e| MarketDataError::Other { msg: e.to_string() })??;

        to_json(&result)
    }

    /// Get ticker info for a symbol (async)
    ///
    /// Returns typed Ticker model with stock metadata.
    pub async fn get_ticker(&self, symbol: String) -> Result<String, MarketDataError> {
        let inner = self.inner.clone();
        let result = tokio::task::spawn_blocking(move || {
            inner.stock().intraday().ticker().symbol(&symbol).send()
        })
        .await
        .map_err(|e| MarketDataError::Other { msg: e.to_string() })??;

        to_json(&result)
    }

    /// Get trade history for a symbol (async)
    ///
    /// Returns typed TradesResponse with list of trades.
    pub async fn get_trades(&self, symbol: String) -> Result<String, MarketDataError> {
        let inner = self.inner.clone();
        let result = tokio::task::spawn_blocking(move || {
            inner.stock().intraday().trades().symbol(&symbol).send()
        })
        .await
        .map_err(|e| MarketDataError::Other { msg: e.to_string() })??;

        to_json(&result)
    }

    /// Get candlestick data for a symbol (async)
    ///
    /// timeframe: "1", "5", "10", "15", "30", "60" (minutes)
    /// Returns typed IntradayCandlesResponse with OHLCV data.
    pub async fn get_candles(&self, symbol: String, timeframe: String) -> Result<String, MarketDataError> {
        let inner = self.inner.clone();
        let result = tokio::task::spawn_blocking(move || {
            inner.stock().intraday().candles()
                .symbol(&symbol)
                .timeframe(&timeframe)
                .send()
        })
        .await
        .map_err(|e| MarketDataError::Other { msg: e.to_string() })??;

        to_json(&result)
    }

    /// Get volume breakdown for a symbol (async)
    ///
    /// Returns typed VolumesResponse with volume at price data.
    pub async fn get_volumes(&self, symbol: String) -> Result<String, MarketDataError> {
        let inner = self.inner.clone();
        let result = tokio::task::spawn_blocking(move || {
            inner.stock().intraday().volumes().symbol(&symbol).send()
        })
        .await
        .map_err(|e| MarketDataError::Other { msg: e.to_string() })??;

        to_json(&result)
    }

    /// Get batch tickers for a security type (async)
    ///
    /// typ: Security type (e.g., "EQUITY", "INDEX", "ETF")
    pub async fn get_tickers(&self, typ: String) -> Result<String, MarketDataError> {
        let inner = self.inner.clone();
        let result = tokio::task::spawn_blocking(move || {
            inner.stock().intraday().tickers()
                .typ(&typ)
                .send()
        })
        .await
        .map_err(|e| MarketDataError::Other { msg: e.to_string() })??;
        to_json(&result)
    }

    // ========== Sync Methods (Blocking) ==========

    /// Get quote for a symbol (sync/blocking)
    pub fn quote_sync(&self, symbol: String) -> Result<String, MarketDataError> {
        let result = self.inner.stock().intraday().quote().symbol(&symbol).send()?;
        to_json(&result)
    }

    /// Get ticker info for a symbol (sync/blocking)
    pub fn ticker_sync(&self, symbol: String) -> Result<String, MarketDataError> {
        let result = self.inner.stock().intraday().ticker().symbol(&symbol).send()?;
        to_json(&result)
    }

    /// Get trade history for a symbol (sync/blocking)
    pub fn trades_sync(&self, symbol: String) -> Result<String, MarketDataError> {
        let result = self.inner.stock().intraday().trades().symbol(&symbol).send()?;
        to_json(&result)
    }

    /// Get candlestick data for a symbol (sync/blocking)
    pub fn candles_sync(&self, symbol: String, timeframe: String) -> Result<String, MarketDataError> {
        let result = self.inner.stock().intraday().candles()
            .symbol(&symbol)
            .timeframe(&timeframe)
            .send()?;
        to_json(&result)
    }

    /// Get volume breakdown for a symbol (sync/blocking)
    pub fn volumes_sync(&self, symbol: String) -> Result<String, MarketDataError> {
        let result = self.inner.stock().intraday().volumes().symbol(&symbol).send()?;
        to_json(&result)
    }

    /// Get batch tickers for a security type (sync/blocking)
    ///
    /// typ: Security type (e.g., "EQUITY", "INDEX", "ETF")
    pub fn tickers_sync(&self, typ: String) -> Result<String, MarketDataError> {
        let result = self.inner.stock().intraday().tickers()
            .typ(&typ)
            .send()?;
        to_json(&result)
    }
}

// ============================================================================
// Stock Historical Client
// ============================================================================

/// Stock historical endpoints with typed model returns
///
/// All methods have both async (get_*) and sync (*_sync) variants:
/// - Async methods are preferred for best performance (non-blocking)
/// - Sync methods block the calling thread (simpler API for scripting)
#[derive(uniffi::Object)]
pub struct StockHistoricalClient {
    inner: CoreRestClient,
}

impl StockHistoricalClient {
    pub fn new(client: CoreRestClient) -> Self {
        Self { inner: client }
    }
}

#[cfg(not(feature = "cpp"))]
#[uniffi::export(async_runtime = "tokio")]
impl StockHistoricalClient {
    // ========== Async Methods (Primary) ==========

    /// Get historical candles for a symbol (async)
    ///
    /// Parameters:
    /// - symbol: Stock symbol (e.g., "2330")
    /// - from: Start date (YYYY-MM-DD, optional)
    /// - to: End date (YYYY-MM-DD, optional)
    /// - timeframe: "D" (day), "W" (week), "M" (month), or intraday "1", "5", "10", "15", "30", "60"
    pub async fn get_candles(
        &self,
        symbol: String,
        from: Option<String>,
        to: Option<String>,
        timeframe: Option<String>,
    ) -> Result<String, MarketDataError> {
        let inner = self.inner.clone();
        let result = tokio::task::spawn_blocking(move || {
            build_historical_candles_request(&inner, &symbol, from.as_deref(), to.as_deref(), timeframe.as_deref())
        })
        .await
        .map_err(|e| MarketDataError::Other { msg: e.to_string() })??;

        to_json(&result)
    }

    /// Get historical stats for a symbol (async)
    ///
    /// Returns summary statistics including 52-week high/low
    pub async fn get_stats(&self, symbol: String) -> Result<String, MarketDataError> {
        let inner = self.inner.clone();
        let result = tokio::task::spawn_blocking(move || {
            inner.stock().historical().stats().symbol(&symbol).send()
        })
        .await
        .map_err(|e| MarketDataError::Other { msg: e.to_string() })??;

        to_json(&result)
    }

    // ========== Sync Methods (Blocking) ==========

    /// Get historical candles for a symbol (sync/blocking)
    pub fn candles_sync(
        &self,
        symbol: String,
        from: Option<String>,
        to: Option<String>,
        timeframe: Option<String>,
    ) -> Result<String, MarketDataError> {
        let result = build_historical_candles_request(&self.inner, &symbol, from.as_deref(), to.as_deref(), timeframe.as_deref())?;
        to_json(&result)
    }

    /// Get historical stats for a symbol (sync/blocking)
    pub fn stats_sync(&self, symbol: String) -> Result<String, MarketDataError> {
        let result = self.inner.stock().historical().stats().symbol(&symbol).send()?;
        to_json(&result)
    }
}

// ============================================================================
// Stock Snapshot Client
// ============================================================================

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
}

#[cfg(not(feature = "cpp"))]
#[uniffi::export(async_runtime = "tokio")]
impl StockSnapshotClient {
    // ========== Async Methods (Primary) ==========

    /// Get market-wide snapshot quotes (async)
    ///
    /// Parameters:
    /// - market: Market code (TSE, OTC, ESB, TIB, PSB)
    /// - type_filter: Optional filter (ALL, ALLBUT0999, COMMONSTOCK)
    pub async fn get_quotes(
        &self,
        market: String,
        type_filter: Option<String>,
    ) -> Result<String, MarketDataError> {
        let inner = self.inner.clone();
        let result = tokio::task::spawn_blocking(move || {
            build_snapshot_quotes_request(&inner, &market, type_filter.as_deref())
        })
        .await
        .map_err(|e| MarketDataError::Other { msg: e.to_string() })??;

        to_json(&result)
    }

    /// Get top movers (gainers/losers) in a market (async)
    ///
    /// Parameters:
    /// - market: Market code (TSE, OTC)
    /// - direction: "up" for gainers, "down" for losers (optional)
    /// - change: "percent" or "value" (optional)
    pub async fn get_movers(
        &self,
        market: String,
        direction: Option<String>,
        change: Option<String>,
    ) -> Result<String, MarketDataError> {
        let inner = self.inner.clone();
        let result = tokio::task::spawn_blocking(move || {
            build_snapshot_movers_request(&inner, &market, direction.as_deref(), change.as_deref())
        })
        .await
        .map_err(|e| MarketDataError::Other { msg: e.to_string() })??;

        to_json(&result)
    }

    /// Get most actively traded stocks (async)
    ///
    /// Parameters:
    /// - market: Market code (TSE, OTC)
    /// - trade: "volume" or "value" (optional)
    pub async fn get_actives(
        &self,
        market: String,
        trade: Option<String>,
    ) -> Result<String, MarketDataError> {
        let inner = self.inner.clone();
        let result = tokio::task::spawn_blocking(move || {
            build_snapshot_actives_request(&inner, &market, trade.as_deref())
        })
        .await
        .map_err(|e| MarketDataError::Other { msg: e.to_string() })??;

        to_json(&result)
    }

    // ========== Sync Methods (Blocking) ==========

    /// Get market-wide snapshot quotes (sync/blocking)
    pub fn quotes_sync(
        &self,
        market: String,
        type_filter: Option<String>,
    ) -> Result<String, MarketDataError> {
        let result = build_snapshot_quotes_request(&self.inner, &market, type_filter.as_deref())?;
        to_json(&result)
    }

    /// Get top movers (sync/blocking)
    pub fn movers_sync(
        &self,
        market: String,
        direction: Option<String>,
        change: Option<String>,
    ) -> Result<String, MarketDataError> {
        let result = build_snapshot_movers_request(&self.inner, &market, direction.as_deref(), change.as_deref())?;
        to_json(&result)
    }

    /// Get most actively traded stocks (sync/blocking)
    pub fn actives_sync(
        &self,
        market: String,
        trade: Option<String>,
    ) -> Result<String, MarketDataError> {
        let result = build_snapshot_actives_request(&self.inner, &market, trade.as_deref())?;
        to_json(&result)
    }
}

// ============================================================================
// Stock Technical Client
// ============================================================================

/// Stock technical indicator endpoints
///
/// Provides access to SMA, RSI, KDJ, MACD, and Bollinger Bands indicators.
#[derive(uniffi::Object)]
pub struct StockTechnicalClient {
    inner: CoreRestClient,
}

impl StockTechnicalClient {
    pub fn new(client: CoreRestClient) -> Self {
        Self { inner: client }
    }
}

#[cfg(not(feature = "cpp"))]
#[uniffi::export(async_runtime = "tokio")]
impl StockTechnicalClient {
    // ========== Async Methods (Primary) ==========

    /// Get Simple Moving Average (async)
    pub async fn get_sma(
        &self,
        symbol: String,
        from: Option<String>,
        to: Option<String>,
        timeframe: Option<String>,
        period: Option<u32>,
    ) -> Result<String, MarketDataError> {
        let inner = self.inner.clone();
        let result = tokio::task::spawn_blocking(move || {
            build_sma_request(&inner, &symbol, from.as_deref(), to.as_deref(), timeframe.as_deref(), period)
        })
        .await
        .map_err(|e| MarketDataError::Other { msg: e.to_string() })??;
        to_json(&result)
    }

    /// Get Relative Strength Index (async)
    pub async fn get_rsi(
        &self,
        symbol: String,
        from: Option<String>,
        to: Option<String>,
        timeframe: Option<String>,
        period: Option<u32>,
    ) -> Result<String, MarketDataError> {
        let inner = self.inner.clone();
        let result = tokio::task::spawn_blocking(move || {
            build_rsi_request(&inner, &symbol, from.as_deref(), to.as_deref(), timeframe.as_deref(), period)
        })
        .await
        .map_err(|e| MarketDataError::Other { msg: e.to_string() })??;
        to_json(&result)
    }

    /// Get KDJ (Stochastic Oscillator) (async)
    pub async fn get_kdj(
        &self,
        symbol: String,
        from: Option<String>,
        to: Option<String>,
        timeframe: Option<String>,
        r_period: Option<u32>,
        k_period: Option<u32>,
        d_period: Option<u32>,
    ) -> Result<String, MarketDataError> {
        let inner = self.inner.clone();
        let result = tokio::task::spawn_blocking(move || {
            build_kdj_request(&inner, &symbol, from.as_deref(), to.as_deref(), timeframe.as_deref(), r_period, k_period, d_period)
        })
        .await
        .map_err(|e| MarketDataError::Other { msg: e.to_string() })??;
        to_json(&result)
    }

    /// Get MACD indicator (async)
    pub async fn get_macd(
        &self,
        symbol: String,
        from: Option<String>,
        to: Option<String>,
        timeframe: Option<String>,
        fast: Option<u32>,
        slow: Option<u32>,
        signal: Option<u32>,
    ) -> Result<String, MarketDataError> {
        let inner = self.inner.clone();
        let result = tokio::task::spawn_blocking(move || {
            build_macd_request(&inner, &symbol, from.as_deref(), to.as_deref(), timeframe.as_deref(), fast, slow, signal)
        })
        .await
        .map_err(|e| MarketDataError::Other { msg: e.to_string() })??;
        to_json(&result)
    }

    /// Get Bollinger Bands (async)
    pub async fn get_bb(
        &self,
        symbol: String,
        from: Option<String>,
        to: Option<String>,
        timeframe: Option<String>,
        period: Option<u32>,
        stddev: Option<f64>,
    ) -> Result<String, MarketDataError> {
        let inner = self.inner.clone();
        let result = tokio::task::spawn_blocking(move || {
            build_bb_request(&inner, &symbol, from.as_deref(), to.as_deref(), timeframe.as_deref(), period, stddev)
        })
        .await
        .map_err(|e| MarketDataError::Other { msg: e.to_string() })??;
        to_json(&result)
    }

    // ========== Sync Methods (Blocking) ==========

    /// Get Simple Moving Average (sync/blocking)
    pub fn sma_sync(
        &self,
        symbol: String,
        from: Option<String>,
        to: Option<String>,
        timeframe: Option<String>,
        period: Option<u32>,
    ) -> Result<String, MarketDataError> {
        let result = build_sma_request(&self.inner, &symbol, from.as_deref(), to.as_deref(), timeframe.as_deref(), period)?;
        to_json(&result)
    }

    /// Get Relative Strength Index (sync/blocking)
    pub fn rsi_sync(
        &self,
        symbol: String,
        from: Option<String>,
        to: Option<String>,
        timeframe: Option<String>,
        period: Option<u32>,
    ) -> Result<String, MarketDataError> {
        let result = build_rsi_request(&self.inner, &symbol, from.as_deref(), to.as_deref(), timeframe.as_deref(), period)?;
        to_json(&result)
    }

    /// Get KDJ (sync/blocking)
    pub fn kdj_sync(
        &self,
        symbol: String,
        from: Option<String>,
        to: Option<String>,
        timeframe: Option<String>,
        r_period: Option<u32>,
        k_period: Option<u32>,
        d_period: Option<u32>,
    ) -> Result<String, MarketDataError> {
        let result = build_kdj_request(&self.inner, &symbol, from.as_deref(), to.as_deref(), timeframe.as_deref(), r_period, k_period, d_period)?;
        to_json(&result)
    }

    /// Get MACD (sync/blocking)
    pub fn macd_sync(
        &self,
        symbol: String,
        from: Option<String>,
        to: Option<String>,
        timeframe: Option<String>,
        fast: Option<u32>,
        slow: Option<u32>,
        signal: Option<u32>,
    ) -> Result<String, MarketDataError> {
        let result = build_macd_request(&self.inner, &symbol, from.as_deref(), to.as_deref(), timeframe.as_deref(), fast, slow, signal)?;
        to_json(&result)
    }

    /// Get Bollinger Bands (sync/blocking)
    pub fn bb_sync(
        &self,
        symbol: String,
        from: Option<String>,
        to: Option<String>,
        timeframe: Option<String>,
        period: Option<u32>,
        stddev: Option<f64>,
    ) -> Result<String, MarketDataError> {
        let result = build_bb_request(&self.inner, &symbol, from.as_deref(), to.as_deref(), timeframe.as_deref(), period, stddev)?;
        to_json(&result)
    }
}

// ============================================================================
// Stock Corporate Actions Client
// ============================================================================

/// Stock corporate actions endpoints
///
/// Provides access to capital changes, dividends, and listing applicants (IPO).
#[derive(uniffi::Object)]
pub struct StockCorporateActionsClient {
    inner: CoreRestClient,
}

impl StockCorporateActionsClient {
    pub fn new(client: CoreRestClient) -> Self {
        Self { inner: client }
    }
}

#[cfg(not(feature = "cpp"))]
#[uniffi::export(async_runtime = "tokio")]
impl StockCorporateActionsClient {
    // ========== Async Methods (Primary) ==========

    /// Get capital structure changes (async)
    pub async fn get_capital_changes(
        &self,
        date: Option<String>,
        start_date: Option<String>,
        end_date: Option<String>,
    ) -> Result<String, MarketDataError> {
        let inner = self.inner.clone();
        let result = tokio::task::spawn_blocking(move || {
            build_capital_changes_request(&inner, date.as_deref(), start_date.as_deref(), end_date.as_deref())
        })
        .await
        .map_err(|e| MarketDataError::Other { msg: e.to_string() })??;
        to_json(&result)
    }

    /// Get dividend announcements (async)
    pub async fn get_dividends(
        &self,
        date: Option<String>,
        start_date: Option<String>,
        end_date: Option<String>,
    ) -> Result<String, MarketDataError> {
        let inner = self.inner.clone();
        let result = tokio::task::spawn_blocking(move || {
            build_dividends_request(&inner, date.as_deref(), start_date.as_deref(), end_date.as_deref())
        })
        .await
        .map_err(|e| MarketDataError::Other { msg: e.to_string() })??;
        to_json(&result)
    }

    /// Get IPO listing applicants (async)
    pub async fn get_listing_applicants(
        &self,
        date: Option<String>,
        start_date: Option<String>,
        end_date: Option<String>,
    ) -> Result<String, MarketDataError> {
        let inner = self.inner.clone();
        let result = tokio::task::spawn_blocking(move || {
            build_listing_applicants_request(&inner, date.as_deref(), start_date.as_deref(), end_date.as_deref())
        })
        .await
        .map_err(|e| MarketDataError::Other { msg: e.to_string() })??;
        to_json(&result)
    }

    // ========== Sync Methods (Blocking) ==========

    /// Get capital structure changes (sync/blocking)
    pub fn capital_changes_sync(
        &self,
        date: Option<String>,
        start_date: Option<String>,
        end_date: Option<String>,
    ) -> Result<String, MarketDataError> {
        let result = build_capital_changes_request(&self.inner, date.as_deref(), start_date.as_deref(), end_date.as_deref())?;
        to_json(&result)
    }

    /// Get dividend announcements (sync/blocking)
    pub fn dividends_sync(
        &self,
        date: Option<String>,
        start_date: Option<String>,
        end_date: Option<String>,
    ) -> Result<String, MarketDataError> {
        let result = build_dividends_request(&self.inner, date.as_deref(), start_date.as_deref(), end_date.as_deref())?;
        to_json(&result)
    }

    /// Get IPO listing applicants (sync/blocking)
    pub fn listing_applicants_sync(
        &self,
        date: Option<String>,
        start_date: Option<String>,
        end_date: Option<String>,
    ) -> Result<String, MarketDataError> {
        let result = build_listing_applicants_request(&self.inner, date.as_deref(), start_date.as_deref(), end_date.as_deref())?;
        to_json(&result)
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

/// FutOpt intraday endpoints with typed model returns
#[derive(uniffi::Object)]
pub struct FutOptIntradayClient {
    inner: CoreRestClient,
}

// Non-exported impl block for constructors
impl FutOptIntradayClient {
    pub fn new(client: CoreRestClient) -> Self {
        Self { inner: client }
    }
}

#[cfg(not(feature = "cpp"))]
#[uniffi::export(async_runtime = "tokio")]
impl FutOptIntradayClient {
    // ========== Async Methods (Primary) ==========

    /// Get quote for a futures/options contract (async)
    ///
    /// after_hours: true for after-hours session
    pub async fn get_quote(&self, symbol: String, after_hours: bool) -> Result<String, MarketDataError> {
        let inner = self.inner.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut builder = inner.futopt().intraday().quote().symbol(&symbol);
            if after_hours {
                builder = builder.after_hours();
            }
            builder.send()
        })
        .await
        .map_err(|e| MarketDataError::Other { msg: e.to_string() })??;

        to_json(&result)
    }

    /// Get ticker info for a contract (async)
    pub async fn get_ticker(&self, symbol: String, after_hours: bool) -> Result<String, MarketDataError> {
        let inner = self.inner.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut builder = inner.futopt().intraday().ticker().symbol(&symbol);
            if after_hours {
                builder = builder.after_hours();
            }
            builder.send()
        })
        .await
        .map_err(|e| MarketDataError::Other { msg: e.to_string() })??;

        to_json(&result)
    }

    /// Get available products list (async)
    ///
    /// typ: "F" for futures, "O" for options
    pub async fn get_products(&self, typ: String) -> Result<String, MarketDataError> {
        let futopt_type = parse_futopt_type(&typ)?;
        let inner = self.inner.clone();
        let result = tokio::task::spawn_blocking(move || {
            inner.futopt().intraday().products().typ(futopt_type).send()
        })
        .await
        .map_err(|e| MarketDataError::Other { msg: e.to_string() })??;

        to_json(&result)
    }

    /// Get candlestick data for a futures/options contract (async)
    pub async fn get_candles(&self, symbol: String, timeframe: String) -> Result<String, MarketDataError> {
        let inner = self.inner.clone();
        let result = tokio::task::spawn_blocking(move || {
            inner.futopt().intraday().candles()
                .symbol(&symbol)
                .timeframe(&timeframe)
                .send()
        })
        .await
        .map_err(|e| MarketDataError::Other { msg: e.to_string() })??;
        to_json(&result)
    }

    /// Get trade history for a futures/options contract (async)
    pub async fn get_trades(&self, symbol: String) -> Result<String, MarketDataError> {
        let inner = self.inner.clone();
        let result = tokio::task::spawn_blocking(move || {
            inner.futopt().intraday().trades()
                .symbol(&symbol)
                .send()
        })
        .await
        .map_err(|e| MarketDataError::Other { msg: e.to_string() })??;
        to_json(&result)
    }

    /// Get volume breakdown by price for a futures/options contract (async)
    pub async fn get_volumes(&self, symbol: String) -> Result<String, MarketDataError> {
        let inner = self.inner.clone();
        let result = tokio::task::spawn_blocking(move || {
            inner.futopt().intraday().volumes()
                .symbol(&symbol)
                .send()
        })
        .await
        .map_err(|e| MarketDataError::Other { msg: e.to_string() })??;
        to_json(&result)
    }

    /// Get batch tickers for futures/options (async)
    ///
    /// typ: "F" for futures, "O" for options
    pub async fn get_tickers(
        &self,
        typ: String,
        is_spread: Option<bool>,
    ) -> Result<String, MarketDataError> {
        let futopt_type = parse_futopt_type(&typ)?;
        let inner = self.inner.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut builder = inner.futopt().intraday().tickers().typ(futopt_type);
            if let Some(sp) = is_spread {
                builder = builder.is_spread(sp);
            }
            builder.send()
        })
        .await
        .map_err(|e| MarketDataError::Other { msg: e.to_string() })??;
        to_json(&result)
    }

    // ========== Sync Methods (Blocking) ==========

    /// Get quote for a futures/options contract (sync/blocking)
    pub fn quote_sync(&self, symbol: String, after_hours: bool) -> Result<String, MarketDataError> {
        let mut builder = self.inner.futopt().intraday().quote().symbol(&symbol);
        if after_hours {
            builder = builder.after_hours();
        }
        let result = builder.send()?;
        to_json(&result)
    }

    /// Get ticker info for a contract (sync/blocking)
    pub fn ticker_sync(&self, symbol: String, after_hours: bool) -> Result<String, MarketDataError> {
        let mut builder = self.inner.futopt().intraday().ticker().symbol(&symbol);
        if after_hours {
            builder = builder.after_hours();
        }
        let result = builder.send()?;
        to_json(&result)
    }

    /// Get available products list (sync/blocking)
    pub fn products_sync(&self, typ: String) -> Result<String, MarketDataError> {
        let futopt_type = parse_futopt_type(&typ)?;
        let result = self.inner.futopt().intraday().products().typ(futopt_type).send()?;
        to_json(&result)
    }

    /// Get candlestick data for a contract (sync/blocking)
    pub fn candles_sync(&self, symbol: String, timeframe: String) -> Result<String, MarketDataError> {
        let result = self.inner.futopt().intraday().candles()
            .symbol(&symbol)
            .timeframe(&timeframe)
            .send()?;
        to_json(&result)
    }

    /// Get trade history for a contract (sync/blocking)
    pub fn trades_sync(&self, symbol: String) -> Result<String, MarketDataError> {
        let result = self.inner.futopt().intraday().trades()
            .symbol(&symbol)
            .send()?;
        to_json(&result)
    }

    /// Get volume breakdown by price for a contract (sync/blocking)
    pub fn volumes_sync(&self, symbol: String) -> Result<String, MarketDataError> {
        let result = self.inner.futopt().intraday().volumes()
            .symbol(&symbol)
            .send()?;
        to_json(&result)
    }

    /// Get batch tickers for futures/options (sync/blocking)
    ///
    /// typ: "F" for futures, "O" for options
    pub fn tickers_sync(
        &self,
        typ: String,
        is_spread: Option<bool>,
    ) -> Result<String, MarketDataError> {
        let futopt_type = parse_futopt_type(&typ)?;
        let mut builder = self.inner.futopt().intraday().tickers().typ(futopt_type);
        if let Some(sp) = is_spread {
            builder = builder.is_spread(sp);
        }
        let result = builder.send()?;
        to_json(&result)
    }
}

// ============================================================================
// FutOpt Historical Client
// ============================================================================

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
}

#[cfg(not(feature = "cpp"))]
#[uniffi::export(async_runtime = "tokio")]
impl FutOptHistoricalClient {
    // ========== Async Methods (Primary) ==========

    /// Get historical candles for a product such as "TXF" (async)
    ///
    /// `contract_month` is "YYYYMM" or a continuous contract ("1!", the server
    /// default, "2!", "3!").
    #[allow(clippy::too_many_arguments, reason = "UniFFI exports positional parameters")]
    pub async fn get_candles(
        &self,
        symbol: String,
        from: Option<String>,
        to: Option<String>,
        timeframe: Option<String>,
        after_hours: bool,
        contract_month: Option<String>,
        fields: Option<String>,
        sort: Option<String>,
    ) -> Result<String, MarketDataError> {
        let inner = self.inner.clone();
        let result = tokio::task::spawn_blocking(move || {
            let query = FutOptCandlesQuery { from, to, timeframe, after_hours, contract_month, fields, sort };
            build_futopt_historical_candles_request(&inner, &symbol, &query)
        })
        .await
        .map_err(|e| MarketDataError::Other { msg: e.to_string() })??;
        to_json(&result)
    }

    /// Get one trading day's daily quotes for every contract month of a product such as "TXF" (async)
    pub async fn get_daily(
        &self,
        symbol: String,
        date: Option<String>,
        after_hours: bool,
    ) -> Result<String, MarketDataError> {
        let inner = self.inner.clone();
        let result = tokio::task::spawn_blocking(move || {
            build_futopt_daily_request(&inner, &symbol, date.as_deref(), after_hours)
        })
        .await
        .map_err(|e| MarketDataError::Other { msg: e.to_string() })??;
        to_json(&result)
    }

    // ========== Sync Methods (Blocking) ==========

    /// Get historical candles for a product such as "TXF" (sync/blocking)
    #[allow(clippy::too_many_arguments, reason = "UniFFI exports positional parameters")]
    pub fn candles_sync(
        &self,
        symbol: String,
        from: Option<String>,
        to: Option<String>,
        timeframe: Option<String>,
        after_hours: bool,
        contract_month: Option<String>,
        fields: Option<String>,
        sort: Option<String>,
    ) -> Result<String, MarketDataError> {
        let query = FutOptCandlesQuery { from, to, timeframe, after_hours, contract_month, fields, sort };
        let result = build_futopt_historical_candles_request(&self.inner, &symbol, &query)?;
        to_json(&result)
    }

    /// Get one trading day's daily quotes for every contract month of a product such as "TXF" (sync/blocking)
    pub fn daily_sync(
        &self,
        symbol: String,
        date: Option<String>,
        after_hours: bool,
    ) -> Result<String, MarketDataError> {
        let result = build_futopt_daily_request(&self.inner, &symbol, date.as_deref(), after_hours)?;
        to_json(&result)
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
        _ => Err(MarketDataError::ConfigError {
            msg: format!("Invalid FutOpt type: '{}'. Use 'F' for futures or 'O' for options.", typ)
        }),
    }
}

/// Build historical candles request (helper to avoid borrow checker issues)
fn build_historical_candles_request(
    client: &CoreRestClient,
    symbol: &str,
    from: Option<&str>,
    to: Option<&str>,
    timeframe: Option<&str>,
) -> Result<serde_json::Value, marketdata_core::MarketDataError> {
    let historical = client.stock().historical();
    let mut builder = historical.candles().symbol(symbol);
    if let Some(f) = from { builder = builder.from(f); }
    if let Some(t) = to { builder = builder.to(t); }
    if let Some(tf) = timeframe { builder = builder.timeframe(tf); }
    builder.send()
}

/// Build snapshot quotes request
fn build_snapshot_quotes_request(
    client: &CoreRestClient,
    market: &str,
    type_filter: Option<&str>,
) -> Result<serde_json::Value, marketdata_core::MarketDataError> {
    let snapshot = client.stock().snapshot();
    let mut builder = snapshot.quotes().market(market);
    if let Some(tf) = type_filter { builder = builder.type_filter(tf); }
    builder.send()
}

/// Build snapshot movers request
fn build_snapshot_movers_request(
    client: &CoreRestClient,
    market: &str,
    direction: Option<&str>,
    change: Option<&str>,
) -> Result<serde_json::Value, marketdata_core::MarketDataError> {
    let snapshot = client.stock().snapshot();
    let mut builder = snapshot.movers().market(market);
    if let Some(d) = direction { builder = builder.direction(d); }
    if let Some(c) = change { builder = builder.change(c); }
    builder.send()
}

/// Build snapshot actives request
fn build_snapshot_actives_request(
    client: &CoreRestClient,
    market: &str,
    trade: Option<&str>,
) -> Result<serde_json::Value, marketdata_core::MarketDataError> {
    let snapshot = client.stock().snapshot();
    let mut builder = snapshot.actives().market(market);
    if let Some(t) = trade { builder = builder.trade(t); }
    builder.send()
}

/// Build SMA request
fn build_sma_request(
    client: &CoreRestClient,
    symbol: &str,
    from: Option<&str>,
    to: Option<&str>,
    timeframe: Option<&str>,
    period: Option<u32>,
) -> Result<serde_json::Value, marketdata_core::MarketDataError> {
    let technical = client.stock().technical();
    let mut builder = technical.sma().symbol(symbol);
    if let Some(f) = from { builder = builder.from(f); }
    if let Some(t) = to { builder = builder.to(t); }
    if let Some(tf) = timeframe { builder = builder.timeframe(tf); }
    if let Some(p) = period { builder = builder.period(p); }
    builder.send()
}

/// Build RSI request
fn build_rsi_request(
    client: &CoreRestClient,
    symbol: &str,
    from: Option<&str>,
    to: Option<&str>,
    timeframe: Option<&str>,
    period: Option<u32>,
) -> Result<serde_json::Value, marketdata_core::MarketDataError> {
    let technical = client.stock().technical();
    let mut builder = technical.rsi().symbol(symbol);
    if let Some(f) = from { builder = builder.from(f); }
    if let Some(t) = to { builder = builder.to(t); }
    if let Some(tf) = timeframe { builder = builder.timeframe(tf); }
    if let Some(p) = period { builder = builder.period(p); }
    builder.send()
}

/// Build KDJ request
fn build_kdj_request(
    client: &CoreRestClient,
    symbol: &str,
    from: Option<&str>,
    to: Option<&str>,
    timeframe: Option<&str>,
    r_period: Option<u32>,
    k_period: Option<u32>,
    d_period: Option<u32>,
) -> Result<serde_json::Value, marketdata_core::MarketDataError> {
    let technical = client.stock().technical();
    let mut builder = technical.kdj().symbol(symbol);
    if let Some(f) = from { builder = builder.from(f); }
    if let Some(t) = to { builder = builder.to(t); }
    if let Some(tf) = timeframe { builder = builder.timeframe(tf); }
    if let Some(p) = r_period { builder = builder.r_period(p); }
    if let Some(p) = k_period { builder = builder.k_period(p); }
    if let Some(p) = d_period { builder = builder.d_period(p); }
    builder.send()
}

/// Build MACD request
fn build_macd_request(
    client: &CoreRestClient,
    symbol: &str,
    from: Option<&str>,
    to: Option<&str>,
    timeframe: Option<&str>,
    fast: Option<u32>,
    slow: Option<u32>,
    signal: Option<u32>,
) -> Result<serde_json::Value, marketdata_core::MarketDataError> {
    let technical = client.stock().technical();
    let mut builder = technical.macd().symbol(symbol);
    if let Some(f) = from { builder = builder.from(f); }
    if let Some(t) = to { builder = builder.to(t); }
    if let Some(tf) = timeframe { builder = builder.timeframe(tf); }
    if let Some(fa) = fast { builder = builder.fast(fa); }
    if let Some(s) = slow { builder = builder.slow(s); }
    if let Some(sig) = signal { builder = builder.signal(sig); }
    builder.send()
}

/// Build Bollinger Bands request
fn build_bb_request(
    client: &CoreRestClient,
    symbol: &str,
    from: Option<&str>,
    to: Option<&str>,
    timeframe: Option<&str>,
    period: Option<u32>,
    stddev: Option<f64>,
) -> Result<serde_json::Value, marketdata_core::MarketDataError> {
    let technical = client.stock().technical();
    let mut builder = technical.bb().symbol(symbol);
    if let Some(f) = from { builder = builder.from(f); }
    if let Some(t) = to { builder = builder.to(t); }
    if let Some(tf) = timeframe { builder = builder.timeframe(tf); }
    if let Some(p) = period { builder = builder.period(p); }
    if let Some(s) = stddev { builder = builder.stddev(s); }
    builder.send()
}

/// Build capital changes request
fn build_capital_changes_request(
    client: &CoreRestClient,
    date: Option<&str>,
    start_date: Option<&str>,
    end_date: Option<&str>,
) -> Result<serde_json::Value, marketdata_core::MarketDataError> {
    let corporate = client.stock().corporate_actions();
    let mut builder = corporate.capital_changes();
    if let Some(d) = date { builder = builder.date(d); }
    if let Some(sd) = start_date { builder = builder.start_date(sd); }
    if let Some(ed) = end_date { builder = builder.end_date(ed); }
    builder.send()
}

/// Build dividends request
fn build_dividends_request(
    client: &CoreRestClient,
    date: Option<&str>,
    start_date: Option<&str>,
    end_date: Option<&str>,
) -> Result<serde_json::Value, marketdata_core::MarketDataError> {
    let corporate = client.stock().corporate_actions();
    let mut builder = corporate.dividends();
    if let Some(d) = date { builder = builder.date(d); }
    if let Some(sd) = start_date { builder = builder.start_date(sd); }
    if let Some(ed) = end_date { builder = builder.end_date(ed); }
    builder.send()
}

/// Build listing applicants request
fn build_listing_applicants_request(
    client: &CoreRestClient,
    date: Option<&str>,
    start_date: Option<&str>,
    end_date: Option<&str>,
) -> Result<serde_json::Value, marketdata_core::MarketDataError> {
    let corporate = client.stock().corporate_actions();
    let mut builder = corporate.listing_applicants();
    if let Some(d) = date { builder = builder.date(d); }
    if let Some(sd) = start_date { builder = builder.start_date(sd); }
    if let Some(ed) = end_date { builder = builder.end_date(ed); }
    builder.send()
}

/// Optional query params for FutOpt historical candles
struct FutOptCandlesQuery {
    from: Option<String>,
    to: Option<String>,
    timeframe: Option<String>,
    after_hours: bool,
    contract_month: Option<String>,
    fields: Option<String>,
    sort: Option<String>,
}

/// Build FutOpt historical candles request
fn build_futopt_historical_candles_request(
    client: &CoreRestClient,
    symbol: &str,
    query: &FutOptCandlesQuery,
) -> Result<serde_json::Value, marketdata_core::MarketDataError> {
    let historical = client.futopt().historical();
    let mut builder = historical.candles().symbol(symbol);
    if let Some(f) = &query.from { builder = builder.from(f); }
    if let Some(t) = &query.to { builder = builder.to(t); }
    if let Some(tf) = &query.timeframe { builder = builder.timeframe(tf); }
    if query.after_hours { builder = builder.after_hours(true); }
    if let Some(cm) = &query.contract_month { builder = builder.contract_month(cm); }
    if let Some(f) = &query.fields { builder = builder.fields(f); }
    if let Some(s) = &query.sort { builder = builder.sort(s); }
    builder.send()
}

/// Build FutOpt daily request
fn build_futopt_daily_request(
    client: &CoreRestClient,
    symbol: &str,
    date: Option<&str>,
    after_hours: bool,
) -> Result<serde_json::Value, marketdata_core::MarketDataError> {
    let historical = client.futopt().historical();
    let mut builder = historical.daily().symbol(symbol);
    if let Some(d) = date { builder = builder.date(d); }
    if after_hours { builder = builder.after_hours(true); }
    builder.send()
}

#[cfg(test)]
mod tests {
    use super::*;

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
}

// ============================================================================
// Stock Ownership Client
// ============================================================================

/// Stock ownership endpoints client
#[derive(uniffi::Object)]
pub struct StockOwnershipClient {
    inner: CoreRestClient,
}

impl StockOwnershipClient {
    pub fn new(client: CoreRestClient) -> Self {
        Self { inner: client }
    }
}

/// Parse the `sort` argument shared by every ownership endpoint.
///
/// `sort` is validated rather than dropped: a typo would otherwise return the
/// opposite series without complaint.
fn parse_holdings_sort(
    sort: Option<&str>,
) -> Result<
    Option<marketdata_core::rest::stock::ownership::HoldingsSort>,
    marketdata_core::MarketDataError,
> {
    use marketdata_core::rest::stock::ownership::HoldingsSort;

    match sort {
        None => Ok(None),
        Some("asc") => Ok(Some(HoldingsSort::Asc)),
        Some("desc") => Ok(Some(HoldingsSort::Desc)),
        Some(other) => Err(marketdata_core::MarketDataError::ConfigError(format!(
            "sort must be 'asc' or 'desc' (got '{other}')"
        ))),
    }
}

/// Generate one blocking request function per ownership endpoint. The core
/// builders share a query contract but no trait, hence a macro.
macro_rules! ownership_request {
    ($fn_name:ident, $method:ident) => {
        fn $fn_name(
            client: &CoreRestClient,
            symbol: &str,
            from: Option<&str>,
            to: Option<&str>,
            sort: Option<&str>,
        ) -> Result<serde_json::Value, marketdata_core::MarketDataError> {
            let sort = parse_holdings_sort(sort)?;
            let stock = client.stock();
            let ownership = stock.ownership();
            let mut builder = ownership.$method().symbol(symbol);
            if let Some(f) = from {
                builder = builder.from(f);
            }
            if let Some(t) = to {
                builder = builder.to(t);
            }
            if let Some(s) = sort {
                builder = builder.sort(s);
            }
            builder.send()
        }
    };
}

ownership_request!(build_etf_holdings_request, etf_holdings);
ownership_request!(build_institutional_trades_request, institutional_trades);
ownership_request!(build_director_holdings_request, director_holdings);
ownership_request!(build_tdcc_distribution_request, tdcc_distribution);

#[cfg(not(feature = "cpp"))]
#[uniffi::export(async_runtime = "tokio")]
impl StockOwnershipClient {
    /// Get the constituents an ETF held over a date range (async)
    pub async fn get_etf_holdings(
        &self,
        symbol: String,
        from: Option<String>,
        to: Option<String>,
        sort: Option<String>,
    ) -> Result<String, MarketDataError> {
        let inner = self.inner.clone();
        let result = tokio::task::spawn_blocking(move || {
            build_etf_holdings_request(
                &inner,
                &symbol,
                from.as_deref(),
                to.as_deref(),
                sort.as_deref(),
            )
        })
        .await
        .map_err(|e| MarketDataError::Other { msg: e.to_string() })??;
        to_json(&result)
    }

    /// Get daily trading by the three major institutional investors (async)
    pub async fn get_institutional_trades(
        &self,
        symbol: String,
        from: Option<String>,
        to: Option<String>,
        sort: Option<String>,
    ) -> Result<String, MarketDataError> {
        let inner = self.inner.clone();
        let result = tokio::task::spawn_blocking(move || {
            build_institutional_trades_request(
                &inner,
                &symbol,
                from.as_deref(),
                to.as_deref(),
                sort.as_deref(),
            )
        })
        .await
        .map_err(|e| MarketDataError::Other { msg: e.to_string() })??;
        to_json(&result)
    }

    /// Get monthly holdings and pledges disclosed by directors and supervisors (async)
    pub async fn get_director_holdings(
        &self,
        symbol: String,
        from: Option<String>,
        to: Option<String>,
        sort: Option<String>,
    ) -> Result<String, MarketDataError> {
        let inner = self.inner.clone();
        let result = tokio::task::spawn_blocking(move || {
            build_director_holdings_request(
                &inner,
                &symbol,
                from.as_deref(),
                to.as_deref(),
                sort.as_deref(),
            )
        })
        .await
        .map_err(|e| MarketDataError::Other { msg: e.to_string() })??;
        to_json(&result)
    }

    /// Get the weekly TDCC shareholder distribution by holding-size bracket (async)
    pub async fn get_tdcc_distribution(
        &self,
        symbol: String,
        from: Option<String>,
        to: Option<String>,
        sort: Option<String>,
    ) -> Result<String, MarketDataError> {
        let inner = self.inner.clone();
        let result = tokio::task::spawn_blocking(move || {
            build_tdcc_distribution_request(
                &inner,
                &symbol,
                from.as_deref(),
                to.as_deref(),
                sort.as_deref(),
            )
        })
        .await
        .map_err(|e| MarketDataError::Other { msg: e.to_string() })??;
        to_json(&result)
    }
}

#[cfg(feature = "cpp")]
#[uniffi::export]
impl StockOwnershipClient {
    /// Get the constituents an ETF held over a date range (sync/blocking)
    pub fn etf_holdings_sync(
        &self,
        symbol: String,
        from: Option<String>,
        to: Option<String>,
        sort: Option<String>,
    ) -> Result<String, MarketDataError> {
        let result = build_etf_holdings_request(
            &self.inner,
            &symbol,
            from.as_deref(),
            to.as_deref(),
            sort.as_deref(),
        )?;
        to_json(&result)
    }

    /// Get daily trading by the three major institutional investors (sync/blocking)
    pub fn institutional_trades_sync(
        &self,
        symbol: String,
        from: Option<String>,
        to: Option<String>,
        sort: Option<String>,
    ) -> Result<String, MarketDataError> {
        let result = build_institutional_trades_request(
            &self.inner,
            &symbol,
            from.as_deref(),
            to.as_deref(),
            sort.as_deref(),
        )?;
        to_json(&result)
    }

    /// Get monthly holdings and pledges disclosed by directors and supervisors (sync/blocking)
    pub fn director_holdings_sync(
        &self,
        symbol: String,
        from: Option<String>,
        to: Option<String>,
        sort: Option<String>,
    ) -> Result<String, MarketDataError> {
        let result = build_director_holdings_request(
            &self.inner,
            &symbol,
            from.as_deref(),
            to.as_deref(),
            sort.as_deref(),
        )?;
        to_json(&result)
    }

    /// Get the weekly TDCC shareholder distribution by holding-size bracket (sync/blocking)
    pub fn tdcc_distribution_sync(
        &self,
        symbol: String,
        from: Option<String>,
        to: Option<String>,
        sort: Option<String>,
    ) -> Result<String, MarketDataError> {
        let result = build_tdcc_distribution_request(
            &self.inner,
            &symbol,
            from.as_deref(),
            to.as_deref(),
            sort.as_deref(),
        )?;
        to_json(&result)
    }
}
