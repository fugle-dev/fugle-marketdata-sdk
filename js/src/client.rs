//! REST client wrapper for JavaScript
//!
//! This module provides the JavaScript-facing RestClient that wraps
//! marketdata-core::RestClient for NAPI-RS bindings.

use crate::errors::to_napi_error;
use crate::websocket::RestClientOptions;
use napi::Either;
use napi_derive::napi;
use serde_json::Value;

// ---------------------------------------------------------------------------
// Legacy fugle-marketdata-node compatibility helpers
//
// The legacy `@fugle/marketdata` SDK calls REST methods with a single object
// argument (e.g. `stock.intraday.quote({ symbol: '2330' })`). Our binding
// originally only accepted positional strings; the helper structs and
// `Either<String, _>` parameter types let both shapes coexist without
// breaking existing positional callers.
// ---------------------------------------------------------------------------

/// Stock intraday quote params (object form)
#[napi(object)]
pub struct StockIntradayQuoteParams {
    pub symbol: String,
    pub odd_lot: Option<bool>,
}

/// Plain `{ symbol }` params reused by methods that take only a symbol.
#[napi(object)]
pub struct SymbolParams {
    pub symbol: String,
}

/// REST client for Fugle market data API (JavaScript wrapper)
///
/// # JavaScript Usage
///
/// ```javascript
/// const { RestClient } = require('@fubon/marketdata-js');
///
/// // Create client with API key
/// const client = new RestClient('your-api-key');
///
/// // Access stock market data
/// const quote = client.stock.intraday.quote('2330');
/// console.log(quote.lastPrice, quote.symbol);
///
/// // Access futures/options market data
/// const futoptQuote = client.futopt.intraday.quote('TXFC4');
/// console.log(futoptQuote.lastPrice, futoptQuote.symbol);
/// ```
#[napi]
pub struct RestClient {
    inner: marketdata_core::RestClient,
}

#[napi]
impl RestClient {
    /// Create a new REST client with options
    ///
    /// @param options - Client configuration options
    /// @throws {Error} If validation fails (zero or multiple auth methods)
    ///
    /// @example
    /// ```javascript
    /// const { RestClient } = require('@fugle/marketdata');
    ///
    /// // API key auth
    /// const client = new RestClient({ apiKey: 'your-key' });
    ///
    /// // Bearer token auth with custom base URL
    /// const client = new RestClient({
    ///   bearerToken: 'token',
    ///   baseUrl: 'https://custom.api'
    /// });
    /// ```
    #[napi(constructor)]
    pub fn new(options: RestClientOptions) -> napi::Result<Self> {
        // Validate exactly one auth method (fail fast per CONTEXT.md)
        let auth_count = [
            options.api_key.is_some(),
            options.bearer_token.is_some(),
            options.sdk_token.is_some(),
        ]
        .iter()
        .filter(|&&x| x)
        .count();

        if auth_count == 0 {
            return Err(napi::Error::from_reason(
                "Provide exactly one of: apiKey, bearerToken, sdkToken"
            ));
        }

        if auth_count > 1 {
            return Err(napi::Error::from_reason(
                "Provide exactly one of: apiKey, bearerToken, sdkToken"
            ));
        }

        // Build auth (safe to unwrap after validation)
        let auth = if let Some(key) = options.api_key {
            marketdata_core::rest::Auth::ApiKey(key)
        } else if let Some(token) = options.bearer_token {
            marketdata_core::rest::Auth::BearerToken(token)
        } else {
            marketdata_core::rest::Auth::SdkToken(options.sdk_token.unwrap())
        };

        // Build TLS config from optional kwargs. When both are default we
        // use `with_tls(TlsConfig::default())` — same path `new(auth)` takes
        // internally, so behaviour is preserved for consumers not touching TLS.
        let tls = marketdata_core::TlsConfig {
            root_cert_pem: options.tls_root_cert_pem.map(|arr| arr.to_vec()),
            accept_invalid_certs: options.tls_accept_invalid_certs.unwrap_or(false),
        };

        let mut inner = marketdata_core::RestClient::with_tls(auth, tls).map_err(to_napi_error)?;
        if let Some(url) = options.base_url {
            inner = inner.base_url(&url);
        }

        Ok(Self { inner })
    }

    /// Get the stock client for accessing stock market data
    #[napi(getter)]
    pub fn stock(&self) -> StockClient {
        StockClient {
            inner: self.inner.clone(),
        }
    }

    /// Get the FutOpt client for accessing futures/options market data
    #[napi(getter)]
    pub fn futopt(&self) -> FutOptClient {
        FutOptClient {
            inner: self.inner.clone(),
        }
    }
}

/// Stock market data client
#[napi]
pub struct StockClient {
    inner: marketdata_core::RestClient,
}

#[napi]
impl StockClient {
    /// Get intraday client for real-time stock data
    #[napi(getter)]
    pub fn intraday(&self) -> StockIntradayClient {
        StockIntradayClient {
            inner: self.inner.clone(),
        }
    }

    /// Get historical client for historical stock data
    #[napi(getter)]
    pub fn historical(&self) -> StockHistoricalClient {
        StockHistoricalClient {
            inner: self.inner.clone(),
        }
    }

    /// Get snapshot client for market-wide data
    #[napi(getter)]
    pub fn snapshot(&self) -> StockSnapshotClient {
        StockSnapshotClient {
            inner: self.inner.clone(),
        }
    }

    /// Get technical indicators client
    #[napi(getter)]
    pub fn technical(&self) -> StockTechnicalClient {
        StockTechnicalClient {
            inner: self.inner.clone(),
        }
    }

    /// Get corporate actions client
    #[napi(getter)]
    pub fn corporate_actions(&self) -> StockCorporateActionsClient {
        StockCorporateActionsClient {
            inner: self.inner.clone(),
        }
    }
}

/// Stock intraday data client
#[napi]
pub struct StockIntradayClient {
    inner: marketdata_core::RestClient,
}

#[napi]
impl StockIntradayClient {
    /// Get intraday quote for a stock symbol.
    ///
    /// Two call shapes are supported (legacy fugle-marketdata-node parity):
    ///
    /// ```javascript
    /// // Object shape (matches legacy SDK README)
    /// await client.stock.intraday.quote({ symbol: '2330' });
    /// await client.stock.intraday.quote({ symbol: '2330', oddLot: true });
    ///
    /// // Positional shape
    /// await client.stock.intraday.quote('2330');
    /// await client.stock.intraday.quote('2330', true);
    /// ```
    #[napi(ts_return_type = "Promise<QuoteResponse>")]
    pub async fn quote(
        &self,
        symbol: Either<String, StockIntradayQuoteParams>,
        odd_lot: Option<bool>,
    ) -> napi::Result<Value> {
        let (symbol, effective_odd_lot) = match symbol {
            Either::A(s) => (s, odd_lot),
            Either::B(p) => (p.symbol, p.odd_lot.or(odd_lot)),
        };

        let inner = self.inner.clone();

        // Use spawn_blocking since core uses synchronous HTTP (ureq)
        let result = tokio::task::spawn_blocking(move || {
            let stock = inner.stock();
            let intraday = stock.intraday();
            let mut builder = intraday.quote().symbol(&symbol);
            if let Some(ol) = effective_odd_lot {
                builder = builder.odd_lot(ol);
            }
            builder.send()
        })
        .await
        .map_err(|e| napi::Error::from_reason(format!("Task error: {}", e)))?;

        match result {
            Ok(quote) => serde_json::to_value(&quote)
                .map_err(|e| napi::Error::from_reason(e.to_string())),
            Err(e) => Err(to_napi_error(e)),
        }
    }

    /// Get intraday ticker for a stock symbol
    ///
    /// @param symbol - Stock symbol (e.g., "2330" for TSMC)
    /// @returns Promise resolving to Ticker object with last trade info
    #[napi(ts_return_type = "Promise<TickerResponse>")]
    pub async fn ticker(&self, symbol: String) -> napi::Result<Value> {
        let inner = self.inner.clone();

        let result = tokio::task::spawn_blocking(move || {
            inner.stock().intraday().ticker().symbol(&symbol).send()
        })
        .await
        .map_err(|e| napi::Error::from_reason(format!("Task error: {}", e)))?;

        match result {
            Ok(ticker) => serde_json::to_value(&ticker)
                .map_err(|e| napi::Error::from_reason(e.to_string())),
            Err(e) => Err(to_napi_error(e)),
        }
    }

    /// Get intraday candles for a stock symbol
    ///
    /// @param symbol - Stock symbol (e.g., "2330" for TSMC)
    /// @param timeframe - Candle timeframe: "1", "5", "10", "15", "30", "60" (minutes)
    /// @returns Promise resolving to Candles response with OHLCV data
    #[napi(ts_return_type = "Promise<CandlesResponse>")]
    pub async fn candles(&self, symbol: String, timeframe: String) -> napi::Result<Value> {
        let inner = self.inner.clone();

        let result = tokio::task::spawn_blocking(move || {
            inner
                .stock()
                .intraday()
                .candles()
                .symbol(&symbol)
                .timeframe(&timeframe)
                .send()
        })
        .await
        .map_err(|e| napi::Error::from_reason(format!("Task error: {}", e)))?;

        match result {
            Ok(candles) => serde_json::to_value(&candles)
                .map_err(|e| napi::Error::from_reason(e.to_string())),
            Err(e) => Err(to_napi_error(e)),
        }
    }

    /// Get intraday trades for a stock symbol
    ///
    /// @param symbol - Stock symbol (e.g., "2330" for TSMC)
    /// @returns Promise resolving to Trades response with recent trade history
    #[napi(ts_return_type = "Promise<TradesResponse>")]
    pub async fn trades(&self, symbol: String) -> napi::Result<Value> {
        let inner = self.inner.clone();

        let result = tokio::task::spawn_blocking(move || {
            inner.stock().intraday().trades().symbol(&symbol).send()
        })
        .await
        .map_err(|e| napi::Error::from_reason(format!("Task error: {}", e)))?;

        match result {
            Ok(trades) => serde_json::to_value(&trades)
                .map_err(|e| napi::Error::from_reason(e.to_string())),
            Err(e) => Err(to_napi_error(e)),
        }
    }

    /// Get intraday volumes for a stock symbol
    ///
    /// @param symbol - Stock symbol (e.g., "2330" for TSMC)
    /// @returns Promise resolving to Volumes response with volume at each price level
    #[napi(ts_return_type = "Promise<VolumesResponse>")]
    pub async fn volumes(&self, symbol: String) -> napi::Result<Value> {
        let inner = self.inner.clone();

        let result = tokio::task::spawn_blocking(move || {
            inner.stock().intraday().volumes().symbol(&symbol).send()
        })
        .await
        .map_err(|e| napi::Error::from_reason(format!("Task error: {}", e)))?;

        match result {
            Ok(volumes) => serde_json::to_value(&volumes)
                .map_err(|e| napi::Error::from_reason(e.to_string())),
            Err(e) => Err(to_napi_error(e)),
        }
    }

    /// Get batch ticker list for a security type
    ///
    /// @param type - Security type ("EQUITY", "INDEX", "ETF", ...)
    /// @param exchange - Optional exchange filter (e.g., "TWSE", "TPEx")
    /// @param market - Optional market filter (e.g., "TSE", "OTC")
    /// @param industry - Optional industry code filter
    /// @param isNormal - Filter to normal-status tickers only
    /// @returns Promise resolving to an array of ticker info objects
    #[napi(ts_return_type = "Promise<TickerResponse[]>")]
    pub async fn tickers(
        &self,
        #[napi(ts_arg_type = "string")] r#type: String,
        exchange: Option<String>,
        market: Option<String>,
        industry: Option<String>,
        is_normal: Option<bool>,
    ) -> napi::Result<Value> {
        let inner = self.inner.clone();

        let result = tokio::task::spawn_blocking(move || {
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
        })
        .await
        .map_err(|e| napi::Error::from_reason(format!("Task error: {}", e)))?;

        match result {
            Ok(tickers) => serde_json::to_value(&tickers)
                .map_err(|e| napi::Error::from_reason(e.to_string())),
            Err(e) => Err(to_napi_error(e)),
        }
    }
}

/// Stock historical data client
#[napi]
pub struct StockHistoricalClient {
    inner: marketdata_core::RestClient,
}

#[napi]
impl StockHistoricalClient {
    /// Get historical candles for a stock symbol
    ///
    /// @param symbol - Stock symbol (e.g., "2330")
    /// @param from - Start date (YYYY-MM-DD)
    /// @param to - End date (YYYY-MM-DD)
    /// @param timeframe - Timeframe ("D", "W", "M", "1", "5", etc.)
    /// @returns Promise resolving to historical candles data
    #[napi(ts_return_type = "Promise<HistoricalCandlesResponse>")]
    pub async fn candles(
        &self,
        symbol: String,
        from: Option<String>,
        to: Option<String>,
        timeframe: Option<String>,
    ) -> napi::Result<Value> {
        let inner = self.inner.clone();

        let result = tokio::task::spawn_blocking(move || {
            let stock = inner.stock();
            let hist = stock.historical();
            let mut builder = hist.candles().symbol(&symbol);
            if let Some(f) = from {
                builder = builder.from(&f);
            }
            if let Some(t) = to {
                builder = builder.to(&t);
            }
            if let Some(tf) = timeframe {
                builder = builder.timeframe(&tf);
            }
            builder.send()
        })
        .await
        .map_err(|e| napi::Error::from_reason(format!("Task error: {}", e)))?;

        match result {
            Ok(data) => {
                serde_json::to_value(&data).map_err(|e| napi::Error::from_reason(e.to_string()))
            }
            Err(e) => Err(to_napi_error(e)),
        }
    }

    /// Get historical stats for a stock symbol
    ///
    /// @param symbol - Stock symbol (e.g., "2330")
    /// @returns Promise resolving to historical stats data
    #[napi(ts_return_type = "Promise<StatsResponse>")]
    pub async fn stats(&self, symbol: String) -> napi::Result<Value> {
        let inner = self.inner.clone();

        let result = tokio::task::spawn_blocking(move || {
            inner.stock().historical().stats().symbol(&symbol).send()
        })
        .await
        .map_err(|e| napi::Error::from_reason(format!("Task error: {}", e)))?;

        match result {
            Ok(data) => {
                serde_json::to_value(&data).map_err(|e| napi::Error::from_reason(e.to_string()))
            }
            Err(e) => Err(to_napi_error(e)),
        }
    }
}

/// Stock snapshot data client
#[napi]
pub struct StockSnapshotClient {
    inner: marketdata_core::RestClient,
}

#[napi]
impl StockSnapshotClient {
    /// Get snapshot quotes for a market
    ///
    /// @param market - Market code (e.g., "TSE", "OTC")
    /// @param typeFilter - Optional type filter (e.g., "ALL", "COMMONSTOCK")
    /// @returns Promise resolving to snapshot quotes data
    #[napi(ts_return_type = "Promise<SnapshotQuotesResponse>")]
    pub async fn quotes(&self, market: String, type_filter: Option<String>) -> napi::Result<Value> {
        let inner = self.inner.clone();

        let result = tokio::task::spawn_blocking(move || {
            let stock = inner.stock();
            let snap = stock.snapshot();
            let mut builder = snap.quotes().market(&market);
            if let Some(tf) = type_filter {
                builder = builder.type_filter(&tf);
            }
            builder.send()
        })
        .await
        .map_err(|e| napi::Error::from_reason(format!("Task error: {}", e)))?;

        match result {
            Ok(data) => {
                serde_json::to_value(&data).map_err(|e| napi::Error::from_reason(e.to_string()))
            }
            Err(e) => Err(to_napi_error(e)),
        }
    }

    /// Get movers (top gainers/losers) for a market
    ///
    /// @param market - Market code (e.g., "TSE", "OTC")
    /// @param direction - Direction filter ("up" or "down")
    /// @param change - Change type ("percent" or "value")
    /// @returns Promise resolving to movers data
    #[napi(ts_return_type = "Promise<MoversResponse>")]
    pub async fn movers(
        &self,
        market: String,
        direction: Option<String>,
        change: Option<String>,
    ) -> napi::Result<Value> {
        let inner = self.inner.clone();

        let result = tokio::task::spawn_blocking(move || {
            let stock = inner.stock();
            let snap = stock.snapshot();
            let mut builder = snap.movers().market(&market);
            if let Some(d) = direction {
                builder = builder.direction(&d);
            }
            if let Some(c) = change {
                builder = builder.change(&c);
            }
            builder.send()
        })
        .await
        .map_err(|e| napi::Error::from_reason(format!("Task error: {}", e)))?;

        match result {
            Ok(data) => {
                serde_json::to_value(&data).map_err(|e| napi::Error::from_reason(e.to_string()))
            }
            Err(e) => Err(to_napi_error(e)),
        }
    }

    /// Get most actively traded stocks for a market
    ///
    /// @param market - Market code (e.g., "TSE", "OTC")
    /// @param trade - Trade type filter ("volume" or "value")
    /// @returns Promise resolving to actives data
    #[napi(ts_return_type = "Promise<ActivesResponse>")]
    pub async fn actives(&self, market: String, trade: Option<String>) -> napi::Result<Value> {
        let inner = self.inner.clone();

        let result = tokio::task::spawn_blocking(move || {
            let stock = inner.stock();
            let snap = stock.snapshot();
            let mut builder = snap.actives().market(&market);
            if let Some(t) = trade {
                builder = builder.trade(&t);
            }
            builder.send()
        })
        .await
        .map_err(|e| napi::Error::from_reason(format!("Task error: {}", e)))?;

        match result {
            Ok(data) => {
                serde_json::to_value(&data).map_err(|e| napi::Error::from_reason(e.to_string()))
            }
            Err(e) => Err(to_napi_error(e)),
        }
    }
}

/// Stock technical indicators client
#[napi]
pub struct StockTechnicalClient {
    inner: marketdata_core::RestClient,
}

#[napi]
impl StockTechnicalClient {
    /// Get SMA (Simple Moving Average) for a stock
    ///
    /// @param symbol - Stock symbol (e.g., "2330")
    /// @param from - Start date (YYYY-MM-DD)
    /// @param to - End date (YYYY-MM-DD)
    /// @param timeframe - Timeframe ("D", "W", "M")
    /// @param period - SMA period (e.g., 20)
    /// @returns Promise resolving to SMA data
    #[napi(ts_return_type = "Promise<SmaResponse>")]
    pub async fn sma(
        &self,
        symbol: String,
        from: Option<String>,
        to: Option<String>,
        timeframe: Option<String>,
        period: Option<u32>,
    ) -> napi::Result<Value> {
        let inner = self.inner.clone();

        let result = tokio::task::spawn_blocking(move || {
            let stock = inner.stock();
            let tech = stock.technical();
            let mut builder = tech.sma().symbol(&symbol);
            if let Some(f) = from {
                builder = builder.from(&f);
            }
            if let Some(t) = to {
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
        .map_err(|e| napi::Error::from_reason(format!("Task error: {}", e)))?;

        match result {
            Ok(data) => {
                serde_json::to_value(&data).map_err(|e| napi::Error::from_reason(e.to_string()))
            }
            Err(e) => Err(to_napi_error(e)),
        }
    }

    /// Get RSI (Relative Strength Index) for a stock
    ///
    /// @param symbol - Stock symbol (e.g., "2330")
    /// @param from - Start date (YYYY-MM-DD)
    /// @param to - End date (YYYY-MM-DD)
    /// @param timeframe - Timeframe ("D", "W", "M")
    /// @param period - RSI period (e.g., 14)
    /// @returns Promise resolving to RSI data
    #[napi(ts_return_type = "Promise<RsiResponse>")]
    pub async fn rsi(
        &self,
        symbol: String,
        from: Option<String>,
        to: Option<String>,
        timeframe: Option<String>,
        period: Option<u32>,
    ) -> napi::Result<Value> {
        let inner = self.inner.clone();

        let result = tokio::task::spawn_blocking(move || {
            let stock = inner.stock();
            let tech = stock.technical();
            let mut builder = tech.rsi().symbol(&symbol);
            if let Some(f) = from {
                builder = builder.from(&f);
            }
            if let Some(t) = to {
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
        .map_err(|e| napi::Error::from_reason(format!("Task error: {}", e)))?;

        match result {
            Ok(data) => {
                serde_json::to_value(&data).map_err(|e| napi::Error::from_reason(e.to_string()))
            }
            Err(e) => Err(to_napi_error(e)),
        }
    }

    /// Get KDJ (Stochastic Oscillator) for a stock
    ///
    /// @param symbol - Stock symbol (e.g., "2330")
    /// @param from - Start date (YYYY-MM-DD)
    /// @param to - End date (YYYY-MM-DD)
    /// @param timeframe - Timeframe ("D", "W", "M")
    /// @param period - KDJ period (e.g., 9)
    /// @returns Promise resolving to KDJ data
    #[napi(ts_return_type = "Promise<KdjResponse>")]
    pub async fn kdj(
        &self,
        symbol: String,
        from: Option<String>,
        to: Option<String>,
        timeframe: Option<String>,
        period: Option<u32>,
    ) -> napi::Result<Value> {
        let inner = self.inner.clone();

        let result = tokio::task::spawn_blocking(move || {
            let stock = inner.stock();
            let tech = stock.technical();
            let mut builder = tech.kdj().symbol(&symbol);
            if let Some(f) = from {
                builder = builder.from(&f);
            }
            if let Some(t) = to {
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
        .map_err(|e| napi::Error::from_reason(format!("Task error: {}", e)))?;

        match result {
            Ok(data) => {
                serde_json::to_value(&data).map_err(|e| napi::Error::from_reason(e.to_string()))
            }
            Err(e) => Err(to_napi_error(e)),
        }
    }

    /// Get MACD (Moving Average Convergence Divergence) for a stock
    ///
    /// @param symbol - Stock symbol (e.g., "2330")
    /// @param from - Start date (YYYY-MM-DD)
    /// @param to - End date (YYYY-MM-DD)
    /// @param timeframe - Timeframe ("D", "W", "M")
    /// @param fast - Fast EMA period (default: 12)
    /// @param slow - Slow EMA period (default: 26)
    /// @param signal - Signal line period (default: 9)
    /// @returns Promise resolving to MACD data
    #[napi(ts_return_type = "Promise<MacdResponse>")]
    pub async fn macd(
        &self,
        symbol: String,
        from: Option<String>,
        to: Option<String>,
        timeframe: Option<String>,
        fast: Option<u32>,
        slow: Option<u32>,
        signal: Option<u32>,
    ) -> napi::Result<Value> {
        let inner = self.inner.clone();

        let result = tokio::task::spawn_blocking(move || {
            let stock = inner.stock();
            let tech = stock.technical();
            let mut builder = tech.macd().symbol(&symbol);
            if let Some(f) = from {
                builder = builder.from(&f);
            }
            if let Some(t) = to {
                builder = builder.to(&t);
            }
            if let Some(tf) = timeframe {
                builder = builder.timeframe(&tf);
            }
            if let Some(f) = fast {
                builder = builder.fast(f);
            }
            if let Some(s) = slow {
                builder = builder.slow(s);
            }
            if let Some(s) = signal {
                builder = builder.signal(s);
            }
            builder.send()
        })
        .await
        .map_err(|e| napi::Error::from_reason(format!("Task error: {}", e)))?;

        match result {
            Ok(data) => {
                serde_json::to_value(&data).map_err(|e| napi::Error::from_reason(e.to_string()))
            }
            Err(e) => Err(to_napi_error(e)),
        }
    }

    /// Get Bollinger Bands for a stock
    ///
    /// @param symbol - Stock symbol (e.g., "2330")
    /// @param from - Start date (YYYY-MM-DD)
    /// @param to - End date (YYYY-MM-DD)
    /// @param timeframe - Timeframe ("D", "W", "M")
    /// @param period - SMA period (default: 20)
    /// @param stddev - Standard deviation multiplier (default: 2.0)
    /// @returns Promise resolving to Bollinger Bands data
    #[napi(ts_return_type = "Promise<BbResponse>")]
    pub async fn bb(
        &self,
        symbol: String,
        from: Option<String>,
        to: Option<String>,
        timeframe: Option<String>,
        period: Option<u32>,
        stddev: Option<f64>,
    ) -> napi::Result<Value> {
        let inner = self.inner.clone();

        let result = tokio::task::spawn_blocking(move || {
            let stock = inner.stock();
            let tech = stock.technical();
            let mut builder = tech.bb().symbol(&symbol);
            if let Some(f) = from {
                builder = builder.from(&f);
            }
            if let Some(t) = to {
                builder = builder.to(&t);
            }
            if let Some(tf) = timeframe {
                builder = builder.timeframe(&tf);
            }
            if let Some(p) = period {
                builder = builder.period(p);
            }
            if let Some(s) = stddev {
                builder = builder.stddev(s);
            }
            builder.send()
        })
        .await
        .map_err(|e| napi::Error::from_reason(format!("Task error: {}", e)))?;

        match result {
            Ok(data) => {
                serde_json::to_value(&data).map_err(|e| napi::Error::from_reason(e.to_string()))
            }
            Err(e) => Err(to_napi_error(e)),
        }
    }
}

/// Stock corporate actions client
#[napi]
pub struct StockCorporateActionsClient {
    inner: marketdata_core::RestClient,
}

#[napi]
impl StockCorporateActionsClient {
    /// Get capital changes (capital structure changes)
    ///
    /// @param date - Specific date (YYYY-MM-DD)
    /// @param startDate - Start date for range query (YYYY-MM-DD)
    /// @param endDate - End date for range query (YYYY-MM-DD)
    /// @returns Promise resolving to capital changes data
    #[napi(ts_return_type = "Promise<CapitalChangesResponse>")]
    pub async fn capital_changes(
        &self,
        date: Option<String>,
        start_date: Option<String>,
        end_date: Option<String>,
    ) -> napi::Result<Value> {
        let inner = self.inner.clone();

        let result = tokio::task::spawn_blocking(move || {
            let stock = inner.stock();
            let ca = stock.corporate_actions();
            let mut builder = ca.capital_changes();
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
        .map_err(|e| napi::Error::from_reason(format!("Task error: {}", e)))?;

        match result {
            Ok(data) => {
                serde_json::to_value(&data).map_err(|e| napi::Error::from_reason(e.to_string()))
            }
            Err(e) => Err(to_napi_error(e)),
        }
    }

    /// Get dividend announcements
    ///
    /// @param date - Specific date (YYYY-MM-DD)
    /// @param startDate - Start date for range query (YYYY-MM-DD)
    /// @param endDate - End date for range query (YYYY-MM-DD)
    /// @returns Promise resolving to dividends data
    #[napi(ts_return_type = "Promise<DividendsResponse>")]
    pub async fn dividends(
        &self,
        date: Option<String>,
        start_date: Option<String>,
        end_date: Option<String>,
    ) -> napi::Result<Value> {
        let inner = self.inner.clone();

        let result = tokio::task::spawn_blocking(move || {
            let stock = inner.stock();
            let ca = stock.corporate_actions();
            let mut builder = ca.dividends();
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
        .map_err(|e| napi::Error::from_reason(format!("Task error: {}", e)))?;

        match result {
            Ok(data) => {
                serde_json::to_value(&data).map_err(|e| napi::Error::from_reason(e.to_string()))
            }
            Err(e) => Err(to_napi_error(e)),
        }
    }

    /// Get IPO listing applicants
    ///
    /// @param date - Specific date (YYYY-MM-DD)
    /// @param startDate - Start date for range query (YYYY-MM-DD)
    /// @param endDate - End date for range query (YYYY-MM-DD)
    /// @returns Promise resolving to listing applicants data
    #[napi(ts_return_type = "Promise<ListingApplicantsResponse>")]
    pub async fn listing_applicants(
        &self,
        date: Option<String>,
        start_date: Option<String>,
        end_date: Option<String>,
    ) -> napi::Result<Value> {
        let inner = self.inner.clone();

        let result = tokio::task::spawn_blocking(move || {
            let stock = inner.stock();
            let ca = stock.corporate_actions();
            let mut builder = ca.listing_applicants();
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
        .map_err(|e| napi::Error::from_reason(format!("Task error: {}", e)))?;

        match result {
            Ok(data) => {
                serde_json::to_value(&data).map_err(|e| napi::Error::from_reason(e.to_string()))
            }
            Err(e) => Err(to_napi_error(e)),
        }
    }
}

/// Futures and Options market data client
#[napi]
pub struct FutOptClient {
    inner: marketdata_core::RestClient,
}

#[napi]
impl FutOptClient {
    /// Get intraday client for real-time futures/options data
    #[napi(getter)]
    pub fn intraday(&self) -> FutOptIntradayClient {
        FutOptIntradayClient {
            inner: self.inner.clone(),
        }
    }

    /// Get historical client for historical futures/options data
    #[napi(getter)]
    pub fn historical(&self) -> FutOptHistoricalClient {
        FutOptHistoricalClient {
            inner: self.inner.clone(),
        }
    }
}

/// FutOpt intraday data client
#[napi]
pub struct FutOptIntradayClient {
    inner: marketdata_core::RestClient,
}

#[napi]
impl FutOptIntradayClient {
    /// Get intraday quote for a futures/options contract
    ///
    /// @param symbol - Contract symbol (e.g., "TXFC4" for TX futures, "TXO18000C4" for options)
    /// @returns Promise resolving to Quote object with current price and volume data
    ///
    /// @example
    /// ```javascript
    /// const client = new RestClient('your-api-key');
    /// const quote = await client.futopt.intraday.quote('TXFC4');
    /// console.log(quote.lastPrice);  // 17550.0
    /// console.log(quote.symbol);     // "TXFC4"
    /// ```
    #[napi(ts_return_type = "Promise<QuoteResponse>")]
    pub async fn quote(&self, symbol: String) -> napi::Result<Value> {
        let inner = self.inner.clone();

        let result = tokio::task::spawn_blocking(move || {
            inner.futopt().intraday().quote().symbol(&symbol).send()
        })
        .await
        .map_err(|e| napi::Error::from_reason(format!("Task error: {}", e)))?;

        match result {
            Ok(quote) => serde_json::to_value(&quote)
                .map_err(|e| napi::Error::from_reason(e.to_string())),
            Err(e) => Err(to_napi_error(e)),
        }
    }

    /// Get intraday ticker for a futures/options contract
    ///
    /// @param symbol - Contract symbol (e.g., "TXFC4")
    /// @returns Promise resolving to Ticker object with last trade info
    #[napi(ts_return_type = "Promise<TickerResponse>")]
    pub async fn ticker(&self, symbol: String) -> napi::Result<Value> {
        let inner = self.inner.clone();

        let result = tokio::task::spawn_blocking(move || {
            inner.futopt().intraday().ticker().symbol(&symbol).send()
        })
        .await
        .map_err(|e| napi::Error::from_reason(format!("Task error: {}", e)))?;

        match result {
            Ok(ticker) => serde_json::to_value(&ticker)
                .map_err(|e| napi::Error::from_reason(e.to_string())),
            Err(e) => Err(to_napi_error(e)),
        }
    }

    /// Get intraday candles for a futures/options contract
    ///
    /// @param symbol - Contract symbol (e.g., "TXFC4")
    /// @param timeframe - Candle timeframe: "1", "5", "10", "15", "30", "60" (minutes)
    /// @returns Promise resolving to Candles response with OHLCV data
    #[napi(ts_return_type = "Promise<CandlesResponse>")]
    pub async fn candles(&self, symbol: String, timeframe: String) -> napi::Result<Value> {
        let inner = self.inner.clone();

        let result = tokio::task::spawn_blocking(move || {
            inner
                .futopt()
                .intraday()
                .candles()
                .symbol(&symbol)
                .timeframe(&timeframe)
                .send()
        })
        .await
        .map_err(|e| napi::Error::from_reason(format!("Task error: {}", e)))?;

        match result {
            Ok(candles) => serde_json::to_value(&candles)
                .map_err(|e| napi::Error::from_reason(e.to_string())),
            Err(e) => Err(to_napi_error(e)),
        }
    }

    /// Get intraday trades for a futures/options contract
    ///
    /// @param symbol - Contract symbol (e.g., "TXFC4")
    /// @returns Promise resolving to Trades response with recent trade history
    #[napi(ts_return_type = "Promise<TradesResponse>")]
    pub async fn trades(&self, symbol: String) -> napi::Result<Value> {
        let inner = self.inner.clone();

        let result = tokio::task::spawn_blocking(move || {
            inner.futopt().intraday().trades().symbol(&symbol).send()
        })
        .await
        .map_err(|e| napi::Error::from_reason(format!("Task error: {}", e)))?;

        match result {
            Ok(trades) => serde_json::to_value(&trades)
                .map_err(|e| napi::Error::from_reason(e.to_string())),
            Err(e) => Err(to_napi_error(e)),
        }
    }

    /// Get intraday volumes for a futures/options contract
    ///
    /// @param symbol - Contract symbol (e.g., "TXFC4")
    /// @returns Promise resolving to Volumes response with volume at each price level
    #[napi(ts_return_type = "Promise<VolumesResponse>")]
    pub async fn volumes(&self, symbol: String) -> napi::Result<Value> {
        let inner = self.inner.clone();

        let result = tokio::task::spawn_blocking(move || {
            inner.futopt().intraday().volumes().symbol(&symbol).send()
        })
        .await
        .map_err(|e| napi::Error::from_reason(format!("Task error: {}", e)))?;

        match result {
            Ok(volumes) => serde_json::to_value(&volumes)
                .map_err(|e| napi::Error::from_reason(e.to_string())),
            Err(e) => Err(to_napi_error(e)),
        }
    }

    /// Get batch ticker list for a FutOpt contract type
    ///
    /// @param type - Contract type: "FUTURE" or "OPTION"
    /// @param exchange - Optional exchange filter (e.g., "TAIFEX")
    /// @param afterHours - Query after-hours session data
    /// @param contractType - Optional contract type code: "I" / "R" / "B" / "C" / "S" / "E"
    /// @returns Promise resolving to an array of FutOpt ticker info objects
    #[napi(ts_return_type = "Promise<FutOptTickerResponse[]>", ts_args_type = "type: FutOptType, exchange?: string, afterHours?: boolean, contractType?: ContractType")]
    pub async fn tickers(
        &self,
        typ: String,
        exchange: Option<String>,
        after_hours: Option<bool>,
        contract_type: Option<String>,
    ) -> napi::Result<Value> {
        use marketdata_core::models::futopt::{ContractType, FutOptType};

        let fut_opt_type = match typ.to_uppercase().as_str() {
            "FUTURE" => FutOptType::Future,
            "OPTION" => FutOptType::Option,
            _ => {
                return Err(napi::Error::from_reason(format!(
                    "Invalid type '{}': must be 'FUTURE' or 'OPTION'",
                    typ
                )))
            }
        };

        let ct_enum = if let Some(ct) = contract_type {
            Some(match ct.to_uppercase().as_str() {
                "I" | "INDEX" => ContractType::Index,
                "R" | "RATE" => ContractType::Rate,
                "B" | "BOND" => ContractType::Bond,
                "C" | "CURRENCY" => ContractType::Currency,
                "S" | "STOCK" => ContractType::Stock,
                "E" | "ETF" => ContractType::Etf,
                _ => {
                    return Err(napi::Error::from_reason(format!(
                        "Invalid contractType '{}': must be I/R/B/C/S/E",
                        ct
                    )))
                }
            })
        } else {
            None
        };

        let inner = self.inner.clone();

        let result = tokio::task::spawn_blocking(move || {
            let futopt = inner.futopt();
            let intraday = futopt.intraday();
            let mut builder = intraday.tickers().typ(fut_opt_type);
            if let Some(e) = &exchange {
                builder = builder.exchange(e);
            }
            if after_hours.unwrap_or(false) {
                builder = builder.after_hours();
            }
            if let Some(ct) = ct_enum {
                builder = builder.contract_type(ct);
            }
            builder.send()
        })
        .await
        .map_err(|e| napi::Error::from_reason(format!("Task error: {}", e)))?;

        match result {
            Ok(tickers) => serde_json::to_value(&tickers)
                .map_err(|e| napi::Error::from_reason(e.to_string())),
            Err(e) => Err(to_napi_error(e)),
        }
    }

    /// Get product list for futures/options
    ///
    /// @param typ - Type: "FUTURE" or "OPTION" (required)
    /// @param contractType - Contract type filter (optional): "I" (index), "R" (rate), "B" (bond), "C" (currency), "S" (stock), "E" (ETF)
    /// @returns Promise resolving to Products response with available contracts
    #[napi(ts_return_type = "Promise<ProductsResponse>", ts_args_type = "type: FutOptType, contractType?: ContractType")]
    pub async fn products(&self, typ: String, contract_type: Option<String>) -> napi::Result<Value> {
        use marketdata_core::models::futopt::{ContractType, FutOptType};

        // Parse typ parameter before spawn_blocking
        let fut_opt_type = match typ.to_uppercase().as_str() {
            "FUTURE" => FutOptType::Future,
            "OPTION" => FutOptType::Option,
            _ => {
                return Err(napi::Error::from_reason(format!(
                    "Invalid type '{}': must be 'FUTURE' or 'OPTION'",
                    typ
                )))
            }
        };

        // Parse optional contract type
        let ct_enum = if let Some(ct) = contract_type {
            Some(match ct.to_uppercase().as_str() {
                "I" | "INDEX" => ContractType::Index,
                "R" | "RATE" => ContractType::Rate,
                "B" | "BOND" => ContractType::Bond,
                "C" | "CURRENCY" => ContractType::Currency,
                "S" | "STOCK" => ContractType::Stock,
                "E" | "ETF" => ContractType::Etf,
                _ => {
                    return Err(napi::Error::from_reason(format!(
                        "Invalid contractType '{}': must be I/R/B/C/S/E",
                        ct
                    )))
                }
            })
        } else {
            None
        };

        let inner = self.inner.clone();

        let result = tokio::task::spawn_blocking(move || {
            let mut builder = inner.futopt().intraday().products().typ(fut_opt_type);
            if let Some(ct) = ct_enum {
                builder = builder.contract_type(ct);
            }
            builder.send()
        })
        .await
        .map_err(|e| napi::Error::from_reason(format!("Task error: {}", e)))?;

        match result {
            Ok(products) => serde_json::to_value(&products)
                .map_err(|e| napi::Error::from_reason(e.to_string())),
            Err(e) => Err(to_napi_error(e)),
        }
    }
}

/// FutOpt historical data client
#[napi]
pub struct FutOptHistoricalClient {
    inner: marketdata_core::RestClient,
}

#[napi]
impl FutOptHistoricalClient {
    /// Get historical candles for a futures/options contract
    ///
    /// @param symbol - Contract symbol (e.g., "TXFC4")
    /// @param from - Start date (YYYY-MM-DD)
    /// @param to - End date (YYYY-MM-DD)
    /// @param timeframe - Timeframe ("D", "W", "M", "1", "5", etc.)
    /// @param afterHours - Include after-hours data
    /// @returns Promise resolving to historical candles data
    #[napi(ts_return_type = "Promise<FutOptHistoricalCandlesResponse>")]
    pub async fn candles(
        &self,
        symbol: String,
        from: Option<String>,
        to: Option<String>,
        timeframe: Option<String>,
        after_hours: Option<bool>,
    ) -> napi::Result<Value> {
        let inner = self.inner.clone();

        let result = tokio::task::spawn_blocking(move || {
            let futopt = inner.futopt();
            let hist = futopt.historical();
            let mut builder = hist.candles().symbol(&symbol);
            if let Some(f) = from {
                builder = builder.from(&f);
            }
            if let Some(t) = to {
                builder = builder.to(&t);
            }
            if let Some(tf) = timeframe {
                builder = builder.timeframe(&tf);
            }
            if let Some(ah) = after_hours {
                builder = builder.after_hours(ah);
            }
            builder.send()
        })
        .await
        .map_err(|e| napi::Error::from_reason(format!("Task error: {}", e)))?;

        match result {
            Ok(data) => {
                serde_json::to_value(&data).map_err(|e| napi::Error::from_reason(e.to_string()))
            }
            Err(e) => Err(to_napi_error(e)),
        }
    }

    /// Get daily historical data for a futures/options contract
    ///
    /// @param symbol - Contract symbol (e.g., "TXFC4")
    /// @param from - Start date (YYYY-MM-DD)
    /// @param to - End date (YYYY-MM-DD)
    /// @param afterHours - Include after-hours data
    /// @returns Promise resolving to daily historical data
    #[napi(ts_return_type = "Promise<FutOptDailyResponse>")]
    pub async fn daily(
        &self,
        symbol: String,
        from: Option<String>,
        to: Option<String>,
        after_hours: Option<bool>,
    ) -> napi::Result<Value> {
        let inner = self.inner.clone();

        let result = tokio::task::spawn_blocking(move || {
            let futopt = inner.futopt();
            let hist = futopt.historical();
            let mut builder = hist.daily().symbol(&symbol);
            if let Some(f) = from {
                builder = builder.from(&f);
            }
            if let Some(t) = to {
                builder = builder.to(&t);
            }
            if let Some(ah) = after_hours {
                builder = builder.after_hours(ah);
            }
            builder.send()
        })
        .await
        .map_err(|e| napi::Error::from_reason(format!("Task error: {}", e)))?;

        match result {
            Ok(data) => {
                serde_json::to_value(&data).map_err(|e| napi::Error::from_reason(e.to_string()))
            }
            Err(e) => Err(to_napi_error(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_options(api_key: &str) -> RestClientOptions {
        RestClientOptions {
            api_key: Some(api_key.to_string()),
            bearer_token: None,
            sdk_token: None,
            base_url: None,
            tls_root_cert_pem: None,
            tls_accept_invalid_certs: None,
        }
    }

    #[test]
    fn test_rest_client_creation_with_api_key() {
        let client = RestClient::new(make_options("test-api-key")).unwrap();
        // Verify client was created (compilation success is the test)
        let _ = client.stock();
        let _ = client.futopt();
    }

    #[test]
    fn test_rest_client_creation_with_bearer_token() {
        let options = RestClientOptions {
            api_key: None,
            bearer_token: Some("test-token".to_string()),
            sdk_token: None,
            base_url: None,
            tls_root_cert_pem: None,
            tls_accept_invalid_certs: None,
        };
        let client = RestClient::new(options).unwrap();
        let _ = client.stock();
    }

    #[test]
    fn test_rest_client_creation_with_base_url() {
        let options = RestClientOptions {
            api_key: Some("test-key".to_string()),
            bearer_token: None,
            sdk_token: None,
            base_url: Some("https://custom.api".to_string()),
            tls_root_cert_pem: None,
            tls_accept_invalid_certs: None,
        };
        let client = RestClient::new(options).unwrap();
        let _ = client.stock();
    }

    #[test]
    fn test_rest_client_no_auth_fails() {
        let options = RestClientOptions {
            api_key: None,
            bearer_token: None,
            sdk_token: None,
            base_url: None,
            tls_root_cert_pem: None,
            tls_accept_invalid_certs: None,
        };
        let result = RestClient::new(options);
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(err.reason.contains("exactly one"));
        }
    }

    #[test]
    fn test_rest_client_multiple_auth_fails() {
        let options = RestClientOptions {
            api_key: Some("key".to_string()),
            bearer_token: Some("token".to_string()),
            sdk_token: None,
            base_url: None,
            tls_root_cert_pem: None,
            tls_accept_invalid_certs: None,
        };
        let result = RestClient::new(options);
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(err.reason.contains("exactly one"));
        }
    }

    #[test]
    fn test_stock_client_chain() {
        let client = RestClient::new(make_options("test-api-key")).unwrap();
        let stock = client.stock();
        let _intraday = stock.intraday();
        let _historical = stock.historical();
        let _snapshot = stock.snapshot();
        let _technical = stock.technical();
        let _corporate_actions = stock.corporate_actions();
    }

    #[test]
    fn test_futopt_client_chain() {
        let client = RestClient::new(make_options("test-api-key")).unwrap();
        let futopt = client.futopt();
        let _intraday = futopt.intraday();
        let _historical = futopt.historical();
    }
}
