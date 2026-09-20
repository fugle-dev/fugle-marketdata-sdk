// Public wrapper providing FubonNeo-compatible API over UniFFI-generated bindings
// Types are exposed via uniffi.marketdata_uniffi namespace
using System;
using System.Threading.Tasks;

namespace FugleMarketData
{
    /// <summary>
    /// REST API client for Fugle MarketData.
    /// Provides async methods for stock and FutOpt market data.
    /// </summary>
    /// <remarks>
    /// This is a public wrapper over UniFFI-generated bindings that provides:
    /// - FubonNeo-compatible method naming (GetQuoteAsync, GetTradesAsync)
    /// - Task&lt;T&gt; async pattern for .NET idiomatic usage
    /// - IDisposable for resource cleanup
    ///
    /// Model types (Quote, Ticker, etc.) and the params records (e.g.
    /// <c>StockTradesParams</c>) are in the uniffi.marketdata_uniffi namespace.
    /// </remarks>
    public sealed class RestClient : IDisposable
    {
        private readonly uniffi.marketdata_uniffi.RestClient _inner;
        private bool _disposed;

        /// <summary>
        /// Create a new REST client with API key authentication.
        /// </summary>
        /// <param name="apiKey">Fugle API key</param>
        /// <exception cref="uniffi.marketdata_uniffi.MarketDataException">Code 1004 if apiKey is null, empty or whitespace</exception>
        public RestClient(string apiKey)
        {
            uniffi.marketdata_uniffi.MarketdataUniffiMethods.ValidateCredentials(apiKey, null, null);

            _inner = uniffi.marketdata_uniffi.MarketdataUniffiMethods.NewRestClientWithApiKey(apiKey);
        }

        /// <summary>
        /// Create a new REST client with configuration options.
        /// Exactly one non-empty authentication method must be provided in the
        /// options; an empty or whitespace-only value counts as not provided.
        /// </summary>
        /// <param name="options">Configuration options including authentication</param>
        /// <exception cref="ArgumentNullException">If options is null</exception>
        /// <exception cref="uniffi.marketdata_uniffi.MarketDataException">Code 1004 if zero or multiple non-empty authentication methods are provided</exception>
        public RestClient(RestClientOptions options)
        {
            if (options == null)
                throw new ArgumentNullException(nameof(options));

            // Core requires exactly one non-blank credential (ConfigError,
            // code 1004) and reports which one to use.
            var kind = uniffi.marketdata_uniffi.MarketdataUniffiMethods.ValidateCredentials(
                options.ApiKey, options.BearerToken, options.SdkToken);

            // Dispatch to correct UniFFI constructor based on which auth is set.
            //
            // The *AndTls factories are the only ones that take a base URL, so
            // an explicit BaseUrl routes through them with a default TLS
            // config. Previously BaseUrl was accepted and silently ignored.
            var tls = new uniffi.marketdata_uniffi.TlsConfigRecord(null, false);
            var baseUrl = string.IsNullOrEmpty(options.BaseUrl) ? null : options.BaseUrl;

            try
            {
                if (kind == uniffi.marketdata_uniffi.CredentialKind.ApiKey)
                {
                    _inner = baseUrl is null
                        ? uniffi.marketdata_uniffi.MarketdataUniffiMethods.NewRestClientWithApiKey(options.ApiKey!)
                        : uniffi.marketdata_uniffi.MarketdataUniffiMethods.NewRestClientWithApiKeyAndTls(options.ApiKey!, baseUrl, tls);
                }
                else if (kind == uniffi.marketdata_uniffi.CredentialKind.BearerToken)
                {
                    _inner = baseUrl is null
                        ? uniffi.marketdata_uniffi.MarketdataUniffiMethods.NewRestClientWithBearerToken(options.BearerToken!)
                        : uniffi.marketdata_uniffi.MarketdataUniffiMethods.NewRestClientWithBearerTokenAndTls(options.BearerToken!, baseUrl, tls);
                }
                else // SdkToken
                {
                    _inner = baseUrl is null
                        ? uniffi.marketdata_uniffi.MarketdataUniffiMethods.NewRestClientWithSdkToken(options.SdkToken!)
                        : uniffi.marketdata_uniffi.MarketdataUniffiMethods.NewRestClientWithSdkTokenAndTls(options.SdkToken!, baseUrl, tls);
                }
            }
            catch (uniffi.marketdata_uniffi.MarketDataException ex)
            {
                // Wrap UniFFI exceptions in a general .NET exception
                throw new InvalidOperationException($"Failed to create REST client: {ex.Message}", ex);
            }
        }

        // Private constructor for factory methods
        private RestClient(uniffi.marketdata_uniffi.RestClient inner)
        {
            _inner = inner;
        }

        /// <summary>
        /// The base URL this client resolved to, including the API version
        /// segment it appends. Reflects any <see cref="RestClientOptions.BaseUrl"/>
        /// override.
        /// </summary>
        public string BaseUrl => _inner.BaseUrl();

        /// <summary>
        /// Create a new REST client with SDK token authentication.
        /// </summary>
        /// <param name="sdkToken">Fugle SDK token</param>
        /// <returns>A new RestClient instance</returns>
        /// <exception cref="uniffi.marketdata_uniffi.MarketDataException">Code 1004 if sdkToken is null, empty or whitespace</exception>
        public static RestClient WithSdkToken(string sdkToken)
        {
            uniffi.marketdata_uniffi.MarketdataUniffiMethods.ValidateCredentials(null, null, sdkToken);

            var inner = uniffi.marketdata_uniffi.MarketdataUniffiMethods.NewRestClientWithSdkToken(sdkToken);
            return new RestClient(inner);
        }

        /// <summary>
        /// Create a new REST client with bearer token authentication.
        /// </summary>
        /// <param name="bearerToken">OAuth bearer token</param>
        /// <returns>A new RestClient instance</returns>
        /// <exception cref="uniffi.marketdata_uniffi.MarketDataException">Code 1004 if bearerToken is null, empty or whitespace</exception>
        public static RestClient WithBearerToken(string bearerToken)
        {
            uniffi.marketdata_uniffi.MarketdataUniffiMethods.ValidateCredentials(null, bearerToken, null);

            var inner = uniffi.marketdata_uniffi.MarketdataUniffiMethods.NewRestClientWithBearerToken(bearerToken);
            return new RestClient(inner);
        }

        /// <summary>
        /// Access stock market data endpoints.
        /// </summary>
        public StockClient Stock => new StockClient(_inner.Stock());

        /// <summary>
        /// Access futures/options market data endpoints.
        /// </summary>
        public FutOptClient FutOpt => new FutOptClient(_inner.Futopt());

        private void ThrowIfDisposed()
        {
            if (_disposed)
                throw new ObjectDisposedException(nameof(RestClient));
        }

        /// <inheritdoc/>
        public void Dispose()
        {
            if (_disposed) return;
            _inner?.Dispose();
            _disposed = true;
        }
    }

    /// <summary>
    /// Stock market data client providing access to all stock data categories.
    /// </summary>
    public sealed class StockClient
    {
        private readonly uniffi.marketdata_uniffi.StockClient _inner;

        internal StockClient(uniffi.marketdata_uniffi.StockClient inner)
        {
            _inner = inner;
        }

        /// <summary>
        /// Access intraday (real-time) stock data endpoints.
        /// </summary>
        public StockIntradayClient Intraday => new StockIntradayClient(_inner.Intraday());

        /// <summary>
        /// Access historical stock data endpoints.
        /// </summary>
        public StockHistoricalClient Historical => new StockHistoricalClient(_inner.Historical());

        /// <summary>
        /// Access market-wide snapshot endpoints (quotes, movers, actives).
        /// </summary>
        public StockSnapshotClient Snapshot => new StockSnapshotClient(_inner.Snapshot());

        /// <summary>
        /// Access technical indicator endpoints (SMA, RSI, KDJ, MACD, BB).
        /// </summary>
        public StockTechnicalClient Technical => new StockTechnicalClient(_inner.Technical());

        /// <summary>
        /// Access corporate actions endpoints (capital changes, dividends, IPO).
        /// </summary>
        public StockCorporateActionsClient CorporateActions => new StockCorporateActionsClient(_inner.CorporateActions());

        /// <summary>
        /// Access ownership endpoints (ETF holdings, institutional trades,
        /// director holdings, TDCC distribution).
        /// </summary>
        public StockOwnershipClient Ownership => new StockOwnershipClient(_inner.Ownership());
    }

    /// <summary>
    /// Stock intraday endpoints providing real-time market data.
    /// All methods have async and sync variants.
    /// </summary>
    public sealed class StockIntradayClient
    {
        private readonly uniffi.marketdata_uniffi.StockIntradayClient _inner;

        internal StockIntradayClient(uniffi.marketdata_uniffi.StockIntradayClient inner)
        {
            _inner = inner;
        }

        // ========== Async Methods (Primary - FubonNeo compatible naming) ==========

        /// <summary>
        /// Get real-time quote for a stock symbol (async).
        /// </summary>
        /// <param name="symbol">Stock symbol (e.g., "2330" for TSMC)</param>
        /// <param name="params">Optional filters, e.g. <c>oddLot</c></param>
        /// <returns>Quote with price, volume, and order book data</returns>
        public Task<string> GetQuoteAsync(string symbol, uniffi.marketdata_uniffi.OddLotParams? @params = null)
            => _inner.GetQuote(symbol, @params);

        /// <summary>
        /// Get ticker information for a stock symbol (async).
        /// </summary>
        /// <param name="symbol">Stock symbol (e.g., "2330")</param>
        /// <param name="params">Optional filters, e.g. <c>oddLot</c></param>
        /// <returns>Ticker with stock metadata and trading rules</returns>
        public Task<string> GetTickerAsync(string symbol, uniffi.marketdata_uniffi.OddLotParams? @params = null)
            => _inner.GetTicker(symbol, @params);

        /// <summary>
        /// Get trade history for a stock symbol (async).
        /// </summary>
        /// <param name="symbol">Stock symbol (e.g., "2330")</param>
        /// <param name="params">Optional filters: oddLot, offset, limit, sort, isTrial</param>
        /// <returns>TradesResponse with list of executed trades</returns>
        public Task<string> GetTradesAsync(string symbol, uniffi.marketdata_uniffi.StockTradesParams? @params = null)
            => _inner.GetTrades(symbol, @params);

        /// <summary>
        /// Get candlestick data for a stock symbol (async).
        /// </summary>
        /// <param name="symbol">Stock symbol (e.g., "2330")</param>
        /// <param name="params">Optional filters: timeframe, oddLot, sort. Unset timeframe takes the server default.</param>
        /// <returns>IntradayCandlesResponse with OHLCV data</returns>
        public Task<string> GetCandlesAsync(string symbol, uniffi.marketdata_uniffi.StockCandlesParams? @params = null)
            => _inner.GetCandles(symbol, @params);

        /// <summary>
        /// Get volume breakdown by price for a stock symbol (async).
        /// </summary>
        /// <param name="symbol">Stock symbol (e.g., "2330")</param>
        /// <param name="params">Optional filters, e.g. <c>oddLot</c></param>
        /// <returns>VolumesResponse with volume at each price level</returns>
        public Task<string> GetVolumesAsync(string symbol, uniffi.marketdata_uniffi.OddLotParams? @params = null)
            => _inner.GetVolumes(symbol, @params);

        /// <summary>
        /// Get batch tickers for a security type (async).
        /// </summary>
        /// <param name="type">Security type (e.g., "EQUITY", "INDEX", "ETF")</param>
        /// <param name="params">Optional filters: exchange, market, industry, isNormal, isAttention, isDisposition, isHalted, symbol</param>
        /// <returns>List of tickers matching the type filter</returns>
        public Task<string> GetTickersAsync(string type, uniffi.marketdata_uniffi.StockTickersParams? @params = null)
            => _inner.GetTickers(type, @params);

        // ========== Sync Methods (Blocking) ==========

        /// <summary>
        /// Get real-time quote for a stock symbol (blocking).
        /// </summary>
        /// <param name="symbol">Stock symbol (e.g., "2330")</param>
        /// <param name="params">Optional filters, e.g. <c>oddLot</c></param>
        /// <returns>Quote with price, volume, and order book data</returns>
        public string GetQuote(string symbol, uniffi.marketdata_uniffi.OddLotParams? @params = null)
            => _inner.QuoteSync(symbol, @params);

        /// <summary>
        /// Get ticker information for a stock symbol (blocking).
        /// </summary>
        /// <param name="symbol">Stock symbol (e.g., "2330")</param>
        /// <param name="params">Optional filters, e.g. <c>oddLot</c></param>
        /// <returns>Ticker with stock metadata and trading rules</returns>
        public string GetTicker(string symbol, uniffi.marketdata_uniffi.OddLotParams? @params = null)
            => _inner.TickerSync(symbol, @params);

        /// <summary>
        /// Get trade history for a stock symbol (blocking).
        /// </summary>
        /// <param name="symbol">Stock symbol (e.g., "2330")</param>
        /// <param name="params">Optional filters: oddLot, offset, limit, sort, isTrial</param>
        /// <returns>TradesResponse with list of executed trades</returns>
        public string GetTrades(string symbol, uniffi.marketdata_uniffi.StockTradesParams? @params = null)
            => _inner.TradesSync(symbol, @params);

        /// <summary>
        /// Get candlestick data for a stock symbol (blocking).
        /// </summary>
        /// <param name="symbol">Stock symbol (e.g., "2330")</param>
        /// <param name="params">Optional filters: timeframe, oddLot, sort. Unset timeframe takes the server default.</param>
        /// <returns>IntradayCandlesResponse with OHLCV data</returns>
        public string GetCandles(string symbol, uniffi.marketdata_uniffi.StockCandlesParams? @params = null)
            => _inner.CandlesSync(symbol, @params);

        /// <summary>
        /// Get volume breakdown by price for a stock symbol (blocking).
        /// </summary>
        /// <param name="symbol">Stock symbol (e.g., "2330")</param>
        /// <param name="params">Optional filters, e.g. <c>oddLot</c></param>
        /// <returns>VolumesResponse with volume at each price level</returns>
        public string GetVolumes(string symbol, uniffi.marketdata_uniffi.OddLotParams? @params = null)
            => _inner.VolumesSync(symbol, @params);

        /// <summary>
        /// Get batch tickers for a security type (blocking).
        /// </summary>
        /// <param name="type">Security type (e.g., "EQUITY", "INDEX", "ETF")</param>
        /// <param name="params">Optional filters: exchange, market, industry, isNormal, isAttention, isDisposition, isHalted, symbol</param>
        public string GetTickers(string type, uniffi.marketdata_uniffi.StockTickersParams? @params = null)
            => _inner.TickersSync(type, @params);
    }

    /// <summary>
    /// Stock historical data endpoints.
    /// </summary>
    public sealed class StockHistoricalClient
    {
        private readonly uniffi.marketdata_uniffi.StockHistoricalClient _inner;

        internal StockHistoricalClient(uniffi.marketdata_uniffi.StockHistoricalClient inner)
        {
            _inner = inner;
        }

        // ========== Async Methods ==========

        /// <summary>
        /// Get historical candles for a stock symbol (async).
        /// </summary>
        /// <param name="symbol">Stock symbol (e.g., "2330")</param>
        /// <param name="params">Optional filters: from, to, timeframe, fields, sort, adjusted</param>
        public Task<string> GetCandlesAsync(string symbol, uniffi.marketdata_uniffi.StockHistoricalCandlesParams? @params = null)
            => _inner.GetCandles(symbol, @params);

        /// <summary>
        /// Get historical stats for a stock symbol (async).
        /// </summary>
        /// <param name="symbol">Stock symbol (e.g., "2330")</param>
        public Task<string> GetStatsAsync(string symbol)
            => _inner.GetStats(symbol);

        // ========== Sync Methods ==========

        /// <summary>
        /// Get historical candles for a stock symbol (blocking).
        /// </summary>
        public string GetCandles(string symbol, uniffi.marketdata_uniffi.StockHistoricalCandlesParams? @params = null)
            => _inner.CandlesSync(symbol, @params);

        /// <summary>
        /// Get historical stats for a stock symbol (blocking).
        /// </summary>
        public string GetStats(string symbol)
            => _inner.StatsSync(symbol);
    }

    /// <summary>
    /// Stock snapshot endpoints for market-wide data.
    /// </summary>
    public sealed class StockSnapshotClient
    {
        private readonly uniffi.marketdata_uniffi.StockSnapshotClient _inner;

        internal StockSnapshotClient(uniffi.marketdata_uniffi.StockSnapshotClient inner)
        {
            _inner = inner;
        }

        // ========== Async Methods ==========

        /// <summary>
        /// Get market-wide snapshot quotes (async).
        /// </summary>
        /// <param name="market">Market code: TSE, OTC, ESB, TIB, PSB</param>
        /// <param name="params">Optional filter: typeFilter (ALL, ALLBUT0999, COMMONSTOCK)</param>
        public Task<string> GetQuotesAsync(string market, uniffi.marketdata_uniffi.SnapshotParams? @params = null)
            => _inner.GetQuotes(market, @params);

        /// <summary>
        /// Get top movers (gainers/losers) in a market (async).
        /// </summary>
        /// <param name="market">Market code: TSE, OTC</param>
        /// <param name="direction">"up" for gainers, "down" for losers</param>
        /// <param name="change">"percent" or "value"</param>
        /// <param name="params">Optional filters: typeFilter, gt, gte, lt, lte, eq</param>
        public Task<string> GetMoversAsync(
            string market, string direction, string change, uniffi.marketdata_uniffi.MoversParams? @params = null)
            => _inner.GetMovers(market, direction, change, @params);

        /// <summary>
        /// Get most actively traded stocks (async).
        /// </summary>
        /// <param name="market">Market code: TSE, OTC</param>
        /// <param name="trade">"volume" or "value"</param>
        /// <param name="params">Optional filter: typeFilter</param>
        public Task<string> GetActivesAsync(
            string market, string trade, uniffi.marketdata_uniffi.SnapshotParams? @params = null)
            => _inner.GetActives(market, trade, @params);

        // ========== Sync Methods ==========

        /// <summary>
        /// Get market-wide snapshot quotes (blocking).
        /// </summary>
        public string GetQuotes(string market, uniffi.marketdata_uniffi.SnapshotParams? @params = null)
            => _inner.QuotesSync(market, @params);

        /// <summary>
        /// Get top movers (blocking).
        /// </summary>
        public string GetMovers(
            string market, string direction, string change, uniffi.marketdata_uniffi.MoversParams? @params = null)
            => _inner.MoversSync(market, direction, change, @params);

        /// <summary>
        /// Get most actively traded stocks (blocking).
        /// </summary>
        public string GetActives(
            string market, string trade, uniffi.marketdata_uniffi.SnapshotParams? @params = null)
            => _inner.ActivesSync(market, trade, @params);
    }

    /// <summary>
    /// Stock technical indicator endpoints.
    /// </summary>
    public sealed class StockTechnicalClient
    {
        private readonly uniffi.marketdata_uniffi.StockTechnicalClient _inner;

        internal StockTechnicalClient(uniffi.marketdata_uniffi.StockTechnicalClient inner)
        {
            _inner = inner;
        }

        // ========== Async Methods ==========

        /// <summary>
        /// Get Simple Moving Average (async).
        /// </summary>
        /// <param name="symbol">Stock symbol</param>
        /// <param name="period">Moving average period</param>
        /// <param name="params">Optional filters: from, to, timeframe</param>
        public Task<string> GetSmaAsync(
            string symbol, uint period, uniffi.marketdata_uniffi.TechnicalParams? @params = null)
            => _inner.GetSma(symbol, period, @params);

        /// <summary>
        /// Get Relative Strength Index (async).
        /// </summary>
        /// <param name="symbol">Stock symbol</param>
        /// <param name="period">RSI period</param>
        /// <param name="params">Optional filters: from, to, timeframe</param>
        public Task<string> GetRsiAsync(
            string symbol, uint period, uniffi.marketdata_uniffi.TechnicalParams? @params = null)
            => _inner.GetRsi(symbol, period, @params);

        /// <summary>
        /// Get KDJ Stochastic Oscillator (async).
        /// </summary>
        /// <param name="symbol">Stock symbol</param>
        /// <param name="rPeriod">%R period</param>
        /// <param name="kPeriod">%K period</param>
        /// <param name="dPeriod">%D period</param>
        /// <param name="params">Optional filters: from, to, timeframe</param>
        public Task<string> GetKdjAsync(
            string symbol, uint rPeriod, uint kPeriod, uint dPeriod,
            uniffi.marketdata_uniffi.TechnicalParams? @params = null)
            => _inner.GetKdj(symbol, rPeriod, kPeriod, dPeriod, @params);

        /// <summary>
        /// Get MACD indicator (async).
        /// </summary>
        /// <param name="symbol">Stock symbol</param>
        /// <param name="fast">Fast EMA period</param>
        /// <param name="slow">Slow EMA period</param>
        /// <param name="signal">Signal line period</param>
        /// <param name="params">Optional filters: from, to, timeframe</param>
        public Task<string> GetMacdAsync(
            string symbol, uint fast, uint slow, uint signal,
            uniffi.marketdata_uniffi.TechnicalParams? @params = null)
            => _inner.GetMacd(symbol, fast, slow, signal, @params);

        /// <summary>
        /// Get Bollinger Bands (async).
        /// </summary>
        /// <param name="symbol">Stock symbol</param>
        /// <param name="period">Moving average period</param>
        /// <param name="params">Optional filters: from, to, timeframe</param>
        public Task<string> GetBbAsync(
            string symbol, uint period, uniffi.marketdata_uniffi.TechnicalParams? @params = null)
            => _inner.GetBb(symbol, period, @params);

        // ========== Sync Methods ==========

        /// <summary>
        /// Get Simple Moving Average (blocking).
        /// </summary>
        public string GetSma(
            string symbol, uint period, uniffi.marketdata_uniffi.TechnicalParams? @params = null)
            => _inner.SmaSync(symbol, period, @params);

        /// <summary>
        /// Get Relative Strength Index (blocking).
        /// </summary>
        public string GetRsi(
            string symbol, uint period, uniffi.marketdata_uniffi.TechnicalParams? @params = null)
            => _inner.RsiSync(symbol, period, @params);

        /// <summary>
        /// Get KDJ Stochastic Oscillator (blocking).
        /// </summary>
        public string GetKdj(
            string symbol, uint rPeriod, uint kPeriod, uint dPeriod,
            uniffi.marketdata_uniffi.TechnicalParams? @params = null)
            => _inner.KdjSync(symbol, rPeriod, kPeriod, dPeriod, @params);

        /// <summary>
        /// Get MACD indicator (blocking).
        /// </summary>
        public string GetMacd(
            string symbol, uint fast, uint slow, uint signal,
            uniffi.marketdata_uniffi.TechnicalParams? @params = null)
            => _inner.MacdSync(symbol, fast, slow, signal, @params);

        /// <summary>
        /// Get Bollinger Bands (blocking).
        /// </summary>
        public string GetBb(
            string symbol, uint period, uniffi.marketdata_uniffi.TechnicalParams? @params = null)
            => _inner.BbSync(symbol, period, @params);
    }

    /// <summary>
    /// Stock corporate actions endpoints.
    /// </summary>
    public sealed class StockCorporateActionsClient
    {
        private readonly uniffi.marketdata_uniffi.StockCorporateActionsClient _inner;

        internal StockCorporateActionsClient(uniffi.marketdata_uniffi.StockCorporateActionsClient inner)
        {
            _inner = inner;
        }

        // ========== Async Methods ==========

        /// <summary>
        /// Get capital structure changes (async).
        /// </summary>
        /// <param name="params">Optional filters: startDate, endDate, sort. <c>exchange</c> is not
        /// accepted by this endpoint and throws <see cref="uniffi.marketdata_uniffi.MarketDataException"/> (code 1005) if set.</param>
        public Task<string> GetCapitalChangesAsync(uniffi.marketdata_uniffi.CorporateActionsParams? @params = null)
            => _inner.GetCapitalChanges(@params);

        /// <summary>
        /// Get dividend announcements (async).
        /// </summary>
        /// <param name="params">Optional filters: startDate, endDate, exchange, sort</param>
        public Task<string> GetDividendsAsync(uniffi.marketdata_uniffi.CorporateActionsParams? @params = null)
            => _inner.GetDividends(@params);

        /// <summary>
        /// Get IPO listing applicants (async).
        /// </summary>
        /// <param name="params">Optional filters: startDate, endDate, exchange, sort</param>
        public Task<string> GetListingApplicantsAsync(uniffi.marketdata_uniffi.CorporateActionsParams? @params = null)
            => _inner.GetListingApplicants(@params);

        // ========== Sync Methods ==========

        /// <summary>
        /// Get capital structure changes (blocking).
        /// </summary>
        public string GetCapitalChanges(uniffi.marketdata_uniffi.CorporateActionsParams? @params = null)
            => _inner.CapitalChangesSync(@params);

        /// <summary>
        /// Get dividend announcements (blocking).
        /// </summary>
        public string GetDividends(uniffi.marketdata_uniffi.CorporateActionsParams? @params = null)
            => _inner.DividendsSync(@params);

        /// <summary>
        /// Get IPO listing applicants (blocking).
        /// </summary>
        public string GetListingApplicants(uniffi.marketdata_uniffi.CorporateActionsParams? @params = null)
            => _inner.ListingApplicantsSync(@params);
    }

    /// <summary>
    /// Stock ownership endpoints. Every method takes the same optional
    /// filters via <see cref="uniffi.marketdata_uniffi.OwnershipParams"/>:
    /// <c>from</c> / <c>to</c> in YYYY-MM-DD and <c>sort</c> of "asc" or "desc".
    /// </summary>
    public sealed class StockOwnershipClient
    {
        private readonly uniffi.marketdata_uniffi.StockOwnershipClient _inner;

        internal StockOwnershipClient(uniffi.marketdata_uniffi.StockOwnershipClient inner)
        {
            _inner = inner;
        }

        // ========== Async Methods ==========

        /// <summary>
        /// Get the constituents an ETF held over a date range (async).
        /// </summary>
        /// <param name="symbol">ETF symbol (e.g. "0050")</param>
        /// <param name="params">Optional filters: from, to, sort</param>
        public Task<string> GetEtfHoldingsAsync(
            string symbol, uniffi.marketdata_uniffi.OwnershipParams? @params = null)
            => _inner.GetEtfHoldings(symbol, @params);

        /// <summary>
        /// Get daily trading by the three major institutional investors (foreign, investment trust, dealer) (async).
        /// </summary>
        /// <param name="symbol">Stock symbol (e.g. "2330")</param>
        /// <param name="params">Optional filters: from, to, sort</param>
        public Task<string> GetInstitutionalTradesAsync(
            string symbol, uniffi.marketdata_uniffi.OwnershipParams? @params = null)
            => _inner.GetInstitutionalTrades(symbol, @params);

        /// <summary>
        /// Get monthly holdings and pledges disclosed by directors and supervisors (async).
        /// </summary>
        /// <param name="symbol">Stock symbol (e.g. "2330")</param>
        /// <param name="params">Optional filters: from, to, sort</param>
        public Task<string> GetDirectorHoldingsAsync(
            string symbol, uniffi.marketdata_uniffi.OwnershipParams? @params = null)
            => _inner.GetDirectorHoldings(symbol, @params);

        /// <summary>
        /// Get the weekly TDCC shareholder distribution by holding-size bracket (async).
        /// </summary>
        /// <param name="symbol">Stock symbol (e.g. "2330")</param>
        /// <param name="params">Optional filters: from, to, sort</param>
        public Task<string> GetTdccDistributionAsync(
            string symbol, uniffi.marketdata_uniffi.OwnershipParams? @params = null)
            => _inner.GetTdccDistribution(symbol, @params);

        // ========== Sync Methods ==========

        /// <summary>
        /// Get the constituents an ETF held over a date range (blocking).
        /// </summary>
        public string GetEtfHoldings(
            string symbol, uniffi.marketdata_uniffi.OwnershipParams? @params = null)
            => _inner.EtfHoldingsSync(symbol, @params);

        /// <summary>
        /// Get daily trading by the three major institutional investors (foreign, investment trust, dealer) (blocking).
        /// </summary>
        public string GetInstitutionalTrades(
            string symbol, uniffi.marketdata_uniffi.OwnershipParams? @params = null)
            => _inner.InstitutionalTradesSync(symbol, @params);

        /// <summary>
        /// Get monthly holdings and pledges disclosed by directors and supervisors (blocking).
        /// </summary>
        public string GetDirectorHoldings(
            string symbol, uniffi.marketdata_uniffi.OwnershipParams? @params = null)
            => _inner.DirectorHoldingsSync(symbol, @params);

        /// <summary>
        /// Get the weekly TDCC shareholder distribution by holding-size bracket (blocking).
        /// </summary>
        public string GetTdccDistribution(
            string symbol, uniffi.marketdata_uniffi.OwnershipParams? @params = null)
            => _inner.TdccDistributionSync(symbol, @params);
    }

    /// <summary>
    /// Futures and options market data client.
    /// </summary>
    public sealed class FutOptClient
    {
        private readonly uniffi.marketdata_uniffi.FutOptClient _inner;

        internal FutOptClient(uniffi.marketdata_uniffi.FutOptClient inner)
        {
            _inner = inner;
        }

        /// <summary>
        /// Access intraday (real-time) FutOpt data endpoints.
        /// </summary>
        public FutOptIntradayClient Intraday => new FutOptIntradayClient(_inner.Intraday());

        /// <summary>
        /// Access historical FutOpt data endpoints.
        /// </summary>
        public FutOptHistoricalClient Historical => new FutOptHistoricalClient(_inner.Historical());
    }

    /// <summary>
    /// FutOpt intraday endpoints providing real-time market data.
    /// </summary>
    public sealed class FutOptIntradayClient
    {
        private readonly uniffi.marketdata_uniffi.FutOptIntradayClient _inner;

        internal FutOptIntradayClient(uniffi.marketdata_uniffi.FutOptIntradayClient inner)
        {
            _inner = inner;
        }

        // ========== Async Methods (Primary) ==========

        /// <summary>
        /// Get real-time quote for a futures/options contract (async).
        /// </summary>
        /// <param name="symbol">Contract symbol</param>
        /// <param name="params">Optional filter: afterHours (true asks for the after-hours session)</param>
        /// <returns>FutOptQuote with price and trading data</returns>
        public Task<string> GetQuoteAsync(string symbol, uniffi.marketdata_uniffi.AfterHoursParams? @params = null)
            => _inner.GetQuote(symbol, @params);

        /// <summary>
        /// Get ticker information for a contract (async).
        /// </summary>
        /// <param name="symbol">Contract symbol</param>
        /// <param name="params">Optional filter: afterHours (true asks for the after-hours session)</param>
        /// <returns>FutOptTicker with contract metadata</returns>
        public Task<string> GetTickerAsync(string symbol, uniffi.marketdata_uniffi.AfterHoursParams? @params = null)
            => _inner.GetTicker(symbol, @params);

        /// <summary>
        /// Get available products list (async).
        /// </summary>
        /// <param name="type">Product type: "F" for futures, "O" for options</param>
        /// <param name="params">Optional filters: exchange, afterHours, contractType, status</param>
        /// <returns>ProductsResponse with available contracts</returns>
        public Task<string> GetProductsAsync(string type, uniffi.marketdata_uniffi.FutOptProductsParams? @params = null)
            => _inner.GetProducts(type, @params);

        /// <summary>
        /// Get candlestick data for a futures/options contract (async).
        /// </summary>
        /// <param name="symbol">Contract symbol</param>
        /// <param name="params">Optional filters: afterHours, timeframe</param>
        public Task<string> GetCandlesAsync(string symbol, uniffi.marketdata_uniffi.FutOptCandlesParams? @params = null)
            => _inner.GetCandles(symbol, @params);

        /// <summary>
        /// Get trade history for a futures/options contract (async).
        /// </summary>
        /// <param name="symbol">Contract symbol</param>
        /// <param name="params">Optional filters: afterHours, offset, limit, isTrial</param>
        public Task<string> GetTradesAsync(string symbol, uniffi.marketdata_uniffi.FutOptTradesParams? @params = null)
            => _inner.GetTrades(symbol, @params);

        /// <summary>
        /// Get volume breakdown by price for a futures/options contract (async).
        /// </summary>
        /// <param name="symbol">Contract symbol</param>
        /// <param name="params">Optional filter: afterHours (true asks for the after-hours session)</param>
        public Task<string> GetVolumesAsync(string symbol, uniffi.marketdata_uniffi.AfterHoursParams? @params = null)
            => _inner.GetVolumes(symbol, @params);

        /// <summary>
        /// Get batch tickers for futures/options (async).
        /// </summary>
        /// <param name="type">Product type: "F" for futures, "O" for options</param>
        /// <param name="params">Optional filters: exchange, afterHours, product, contractType, isSpread</param>
        public Task<string> GetTickersAsync(string type, uniffi.marketdata_uniffi.FutOptTickersParams? @params = null)
            => _inner.GetTickers(type, @params);

        // ========== Sync Methods (Blocking) ==========

        /// <summary>
        /// Get real-time quote for a futures/options contract (blocking).
        /// </summary>
        /// <param name="symbol">Contract symbol</param>
        /// <param name="params">Optional filter: afterHours (true asks for the after-hours session)</param>
        /// <returns>FutOptQuote with price and trading data</returns>
        public string GetQuote(string symbol, uniffi.marketdata_uniffi.AfterHoursParams? @params = null)
            => _inner.QuoteSync(symbol, @params);

        /// <summary>
        /// Get ticker information for a contract (blocking).
        /// </summary>
        /// <param name="symbol">Contract symbol</param>
        /// <param name="params">Optional filter: afterHours (true asks for the after-hours session)</param>
        /// <returns>FutOptTicker with contract metadata</returns>
        public string GetTicker(string symbol, uniffi.marketdata_uniffi.AfterHoursParams? @params = null)
            => _inner.TickerSync(symbol, @params);

        /// <summary>
        /// Get available products list (blocking).
        /// </summary>
        /// <param name="type">Product type: "F" for futures, "O" for options</param>
        /// <param name="params">Optional filters: exchange, afterHours, contractType, status</param>
        /// <returns>ProductsResponse with available contracts</returns>
        public string GetProducts(string type, uniffi.marketdata_uniffi.FutOptProductsParams? @params = null)
            => _inner.ProductsSync(type, @params);

        /// <summary>
        /// Get candlestick data for a futures/options contract (blocking).
        /// </summary>
        public string GetCandles(string symbol, uniffi.marketdata_uniffi.FutOptCandlesParams? @params = null)
            => _inner.CandlesSync(symbol, @params);

        /// <summary>
        /// Get trade history for a futures/options contract (blocking).
        /// </summary>
        public string GetTrades(string symbol, uniffi.marketdata_uniffi.FutOptTradesParams? @params = null)
            => _inner.TradesSync(symbol, @params);

        /// <summary>
        /// Get volume breakdown by price for a futures/options contract (blocking).
        /// </summary>
        public string GetVolumes(string symbol, uniffi.marketdata_uniffi.AfterHoursParams? @params = null)
            => _inner.VolumesSync(symbol, @params);

        /// <summary>
        /// Get batch tickers for futures/options (blocking).
        /// </summary>
        /// <param name="type">Product type: "F" for futures, "O" for options</param>
        /// <param name="params">Optional filters: exchange, afterHours, product, contractType, isSpread</param>
        public string GetTickers(string type, uniffi.marketdata_uniffi.FutOptTickersParams? @params = null)
            => _inner.TickersSync(type, @params);
    }

    /// <summary>
    /// FutOpt historical data endpoints.
    /// </summary>
    public sealed class FutOptHistoricalClient
    {
        private readonly uniffi.marketdata_uniffi.FutOptHistoricalClient _inner;

        internal FutOptHistoricalClient(uniffi.marketdata_uniffi.FutOptHistoricalClient inner)
        {
            _inner = inner;
        }

        // ========== Async Methods ==========

        /// <summary>
        /// Get historical candles for a futures/options product (async).
        /// </summary>
        /// <param name="symbol">Product code, e.g. "TXF" (a contract code such as "TXFC4" returns 404)</param>
        /// <param name="params">Optional filters: from, to, contractMonth, fields, timeframe, sort, strikePrice, callPut, afterHours</param>
        public Task<string> GetCandlesAsync(
            string symbol, uniffi.marketdata_uniffi.FutOptHistoricalCandlesParams? @params = null)
            => _inner.GetCandles(symbol, @params);

        /// <summary>
        /// Get one trading day's daily quotes for every contract month of a futures/options product (async).
        /// </summary>
        /// <param name="symbol">Product code, e.g. "TXF" (a contract code such as "TXFC4" returns 404)</param>
        /// <param name="params">Optional filters: date, afterHours. Unset date defaults to today.</param>
        public Task<string> GetDailyAsync(string symbol, uniffi.marketdata_uniffi.FutOptDailyParams? @params = null)
            => _inner.GetDaily(symbol, @params);

        // ========== Sync Methods ==========

        /// <summary>
        /// Get historical candles for a futures/options product (blocking).
        /// </summary>
        public string GetCandles(
            string symbol, uniffi.marketdata_uniffi.FutOptHistoricalCandlesParams? @params = null)
            => _inner.CandlesSync(symbol, @params);

        /// <summary>
        /// Get one trading day's daily quotes for every contract month of a futures/options product (blocking).
        /// </summary>
        public string GetDaily(string symbol, uniffi.marketdata_uniffi.FutOptDailyParams? @params = null)
            => _inner.DailySync(symbol, @params);
    }
}
