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
    /// Model types (Quote, Ticker, etc.) are in the uniffi.marketdata_uniffi namespace.
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
        /// <returns>Quote with price, volume, and order book data</returns>
        public Task<string> GetQuoteAsync(string symbol)
            => _inner.GetQuote(symbol);

        /// <summary>
        /// Get ticker information for a stock symbol (async).
        /// </summary>
        /// <param name="symbol">Stock symbol (e.g., "2330")</param>
        /// <returns>Ticker with stock metadata and trading rules</returns>
        public Task<string> GetTickerAsync(string symbol)
            => _inner.GetTicker(symbol);

        /// <summary>
        /// Get trade history for a stock symbol (async).
        /// </summary>
        /// <param name="symbol">Stock symbol (e.g., "2330")</param>
        /// <returns>TradesResponse with list of executed trades</returns>
        public Task<string> GetTradesAsync(string symbol)
            => _inner.GetTrades(symbol);

        /// <summary>
        /// Get candlestick data for a stock symbol (async).
        /// </summary>
        /// <param name="symbol">Stock symbol (e.g., "2330")</param>
        /// <param name="timeframe">Candle timeframe: "1", "5", "10", "15", "30", "60" (minutes)</param>
        /// <returns>IntradayCandlesResponse with OHLCV data</returns>
        public Task<string> GetCandlesAsync(string symbol, string timeframe)
            => _inner.GetCandles(symbol, timeframe);

        /// <summary>
        /// Get volume breakdown by price for a stock symbol (async).
        /// </summary>
        /// <param name="symbol">Stock symbol (e.g., "2330")</param>
        /// <returns>VolumesResponse with volume at each price level</returns>
        public Task<string> GetVolumesAsync(string symbol)
            => _inner.GetVolumes(symbol);

        /// <summary>
        /// Get batch tickers for a security type (async).
        /// </summary>
        /// <param name="type">Security type (e.g., "EQUITY", "INDEX", "ETF")</param>
        /// <returns>List of tickers matching the type filter</returns>
        public Task<string> GetTickersAsync(string type)
            => _inner.GetTickers(type);

        // ========== Sync Methods (Blocking) ==========

        /// <summary>
        /// Get real-time quote for a stock symbol (blocking).
        /// </summary>
        /// <param name="symbol">Stock symbol (e.g., "2330")</param>
        /// <returns>Quote with price, volume, and order book data</returns>
        public string GetQuote(string symbol)
            => _inner.QuoteSync(symbol);

        /// <summary>
        /// Get ticker information for a stock symbol (blocking).
        /// </summary>
        /// <param name="symbol">Stock symbol (e.g., "2330")</param>
        /// <returns>Ticker with stock metadata and trading rules</returns>
        public string GetTicker(string symbol)
            => _inner.TickerSync(symbol);

        /// <summary>
        /// Get trade history for a stock symbol (blocking).
        /// </summary>
        /// <param name="symbol">Stock symbol (e.g., "2330")</param>
        /// <returns>TradesResponse with list of executed trades</returns>
        public string GetTrades(string symbol)
            => _inner.TradesSync(symbol);

        /// <summary>
        /// Get candlestick data for a stock symbol (blocking).
        /// </summary>
        /// <param name="symbol">Stock symbol (e.g., "2330")</param>
        /// <param name="timeframe">Candle timeframe: "1", "5", "10", "15", "30", "60" (minutes)</param>
        /// <returns>IntradayCandlesResponse with OHLCV data</returns>
        public string GetCandles(string symbol, string timeframe)
            => _inner.CandlesSync(symbol, timeframe);

        /// <summary>
        /// Get volume breakdown by price for a stock symbol (blocking).
        /// </summary>
        /// <param name="symbol">Stock symbol (e.g., "2330")</param>
        /// <returns>VolumesResponse with volume at each price level</returns>
        public string GetVolumes(string symbol)
            => _inner.VolumesSync(symbol);

        /// <summary>
        /// Get batch tickers for a security type (blocking).
        /// </summary>
        public string GetTickers(string type)
            => _inner.TickersSync(type);
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
        /// <param name="from">Start date in YYYY-MM-DD format (optional)</param>
        /// <param name="to">End date in YYYY-MM-DD format (optional)</param>
        /// <param name="timeframe">Timeframe: "D" (day), "W" (week), "M" (month), or "1","5","10","15","30","60" (optional)</param>
        public Task<string> GetCandlesAsync(
            string symbol, string? from = null, string? to = null, string? timeframe = null)
            => _inner.GetCandles(symbol, from, to, timeframe);

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
        public string GetCandles(
            string symbol, string? from = null, string? to = null, string? timeframe = null)
            => _inner.CandlesSync(symbol, from, to, timeframe);

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
        /// <param name="typeFilter">Optional filter: ALL, ALLBUT0999, COMMONSTOCK</param>
        public Task<string> GetQuotesAsync(
            string market, string? typeFilter = null)
            => _inner.GetQuotes(market, typeFilter);

        /// <summary>
        /// Get top movers (gainers/losers) in a market (async).
        /// </summary>
        /// <param name="market">Market code: TSE, OTC</param>
        /// <param name="direction">"up" for gainers, "down" for losers (optional)</param>
        /// <param name="change">"percent" or "value" (optional)</param>
        public Task<string> GetMoversAsync(
            string market, string? direction = null, string? change = null)
            => _inner.GetMovers(market, direction, change);

        /// <summary>
        /// Get most actively traded stocks (async).
        /// </summary>
        /// <param name="market">Market code: TSE, OTC</param>
        /// <param name="trade">"volume" or "value" (optional)</param>
        public Task<string> GetActivesAsync(
            string market, string? trade = null)
            => _inner.GetActives(market, trade);

        // ========== Sync Methods ==========

        /// <summary>
        /// Get market-wide snapshot quotes (blocking).
        /// </summary>
        public string GetQuotes(
            string market, string? typeFilter = null)
            => _inner.QuotesSync(market, typeFilter);

        /// <summary>
        /// Get top movers (blocking).
        /// </summary>
        public string GetMovers(
            string market, string? direction = null, string? change = null)
            => _inner.MoversSync(market, direction, change);

        /// <summary>
        /// Get most actively traded stocks (blocking).
        /// </summary>
        public string GetActives(
            string market, string? trade = null)
            => _inner.ActivesSync(market, trade);
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
        public Task<string> GetSmaAsync(
            string symbol, string? from = null, string? to = null, string? timeframe = null, uint? period = null)
            => _inner.GetSma(symbol, from, to, timeframe, period);

        /// <summary>
        /// Get Relative Strength Index (async).
        /// </summary>
        public Task<string> GetRsiAsync(
            string symbol, string? from = null, string? to = null, string? timeframe = null, uint? period = null)
            => _inner.GetRsi(symbol, from, to, timeframe, period);

        /// <summary>
        /// Get KDJ Stochastic Oscillator (async).
        /// </summary>
        public Task<string> GetKdjAsync(
            string symbol, string? from = null, string? to = null, string? timeframe = null,
            uint? rPeriod = null, uint? kPeriod = null, uint? dPeriod = null)
            => _inner.GetKdj(symbol, from, to, timeframe, rPeriod, kPeriod, dPeriod);

        /// <summary>
        /// Get MACD indicator (async).
        /// </summary>
        public Task<string> GetMacdAsync(
            string symbol, string? from = null, string? to = null, string? timeframe = null,
            uint? fast = null, uint? slow = null, uint? signal = null)
            => _inner.GetMacd(symbol, from, to, timeframe, fast, slow, signal);

        /// <summary>
        /// Get Bollinger Bands (async).
        /// </summary>
        public Task<string> GetBbAsync(
            string symbol, string? from = null, string? to = null, string? timeframe = null,
            uint? period = null)
            => _inner.GetBb(symbol, from, to, timeframe, period);

        // ========== Sync Methods ==========

        /// <summary>
        /// Get Simple Moving Average (blocking).
        /// </summary>
        public string GetSma(
            string symbol, string? from = null, string? to = null, string? timeframe = null, uint? period = null)
            => _inner.SmaSync(symbol, from, to, timeframe, period);

        /// <summary>
        /// Get Relative Strength Index (blocking).
        /// </summary>
        public string GetRsi(
            string symbol, string? from = null, string? to = null, string? timeframe = null, uint? period = null)
            => _inner.RsiSync(symbol, from, to, timeframe, period);

        /// <summary>
        /// Get KDJ Stochastic Oscillator (blocking).
        /// </summary>
        public string GetKdj(
            string symbol, string? from = null, string? to = null, string? timeframe = null,
            uint? rPeriod = null, uint? kPeriod = null, uint? dPeriod = null)
            => _inner.KdjSync(symbol, from, to, timeframe, rPeriod, kPeriod, dPeriod);

        /// <summary>
        /// Get MACD indicator (blocking).
        /// </summary>
        public string GetMacd(
            string symbol, string? from = null, string? to = null, string? timeframe = null,
            uint? fast = null, uint? slow = null, uint? signal = null)
            => _inner.MacdSync(symbol, from, to, timeframe, fast, slow, signal);

        /// <summary>
        /// Get Bollinger Bands (blocking).
        /// </summary>
        public string GetBb(
            string symbol, string? from = null, string? to = null, string? timeframe = null,
            uint? period = null)
            => _inner.BbSync(symbol, from, to, timeframe, period);
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
        /// <param name="startDate">Range start date (optional)</param>
        /// <param name="endDate">Range end date (optional)</param>
        public Task<string> GetCapitalChangesAsync(
            string? startDate = null, string? endDate = null)
            => _inner.GetCapitalChanges(startDate, endDate);

        /// <summary>
        /// Get dividend announcements (async).
        /// </summary>
        public Task<string> GetDividendsAsync(
            string? startDate = null, string? endDate = null)
            => _inner.GetDividends(startDate, endDate);

        /// <summary>
        /// Get IPO listing applicants (async).
        /// </summary>
        public Task<string> GetListingApplicantsAsync(
            string? startDate = null, string? endDate = null)
            => _inner.GetListingApplicants(startDate, endDate);

        // ========== Sync Methods ==========

        /// <summary>
        /// Get capital structure changes (blocking).
        /// </summary>
        public string GetCapitalChanges(
            string? startDate = null, string? endDate = null)
            => _inner.CapitalChangesSync(startDate, endDate);

        /// <summary>
        /// Get dividend announcements (blocking).
        /// </summary>
        public string GetDividends(
            string? startDate = null, string? endDate = null)
            => _inner.DividendsSync(startDate, endDate);

        /// <summary>
        /// Get IPO listing applicants (blocking).
        /// </summary>
        public string GetListingApplicants(
            string? startDate = null, string? endDate = null)
            => _inner.ListingApplicantsSync(startDate, endDate);
    }

    /// <summary>
    /// Stock ownership endpoints. Every method takes the same range arguments:
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
        /// <param name="from">Range start date (optional)</param>
        /// <param name="to">Range end date (optional)</param>
        /// <param name="sort">"asc" (oldest first) or "desc" (newest first); anything else throws</param>
        public Task<string> GetEtfHoldingsAsync(
            string symbol, string? from = null, string? to = null, string? sort = null)
            => _inner.GetEtfHoldings(symbol, from, to, sort);

        /// <summary>
        /// Get daily trading by the three major institutional investors (foreign, investment trust, dealer) (async).
        /// </summary>
        /// <param name="symbol">Stock symbol (e.g. "2330")</param>
        /// <param name="from">Range start date (optional)</param>
        /// <param name="to">Range end date (optional)</param>
        /// <param name="sort">"asc" (oldest first) or "desc" (newest first); anything else throws</param>
        public Task<string> GetInstitutionalTradesAsync(
            string symbol, string? from = null, string? to = null, string? sort = null)
            => _inner.GetInstitutionalTrades(symbol, from, to, sort);

        /// <summary>
        /// Get monthly holdings and pledges disclosed by directors and supervisors (async).
        /// </summary>
        /// <param name="symbol">Stock symbol (e.g. "2330")</param>
        /// <param name="from">Range start date (optional)</param>
        /// <param name="to">Range end date (optional)</param>
        /// <param name="sort">"asc" (oldest first) or "desc" (newest first); anything else throws</param>
        public Task<string> GetDirectorHoldingsAsync(
            string symbol, string? from = null, string? to = null, string? sort = null)
            => _inner.GetDirectorHoldings(symbol, from, to, sort);

        /// <summary>
        /// Get the weekly TDCC shareholder distribution by holding-size bracket (async).
        /// </summary>
        /// <param name="symbol">Stock symbol (e.g. "2330")</param>
        /// <param name="from">Range start date (optional)</param>
        /// <param name="to">Range end date (optional)</param>
        /// <param name="sort">"asc" (oldest first) or "desc" (newest first); anything else throws</param>
        public Task<string> GetTdccDistributionAsync(
            string symbol, string? from = null, string? to = null, string? sort = null)
            => _inner.GetTdccDistribution(symbol, from, to, sort);

        // ========== Sync Methods ==========

        /// <summary>
        /// Get the constituents an ETF held over a date range (blocking).
        /// </summary>
        public string GetEtfHoldings(
            string symbol, string? from = null, string? to = null, string? sort = null)
            => _inner.EtfHoldingsSync(symbol, from, to, sort);

        /// <summary>
        /// Get daily trading by the three major institutional investors (foreign, investment trust, dealer) (blocking).
        /// </summary>
        public string GetInstitutionalTrades(
            string symbol, string? from = null, string? to = null, string? sort = null)
            => _inner.InstitutionalTradesSync(symbol, from, to, sort);

        /// <summary>
        /// Get monthly holdings and pledges disclosed by directors and supervisors (blocking).
        /// </summary>
        public string GetDirectorHoldings(
            string symbol, string? from = null, string? to = null, string? sort = null)
            => _inner.DirectorHoldingsSync(symbol, from, to, sort);

        /// <summary>
        /// Get the weekly TDCC shareholder distribution by holding-size bracket (blocking).
        /// </summary>
        public string GetTdccDistribution(
            string symbol, string? from = null, string? to = null, string? sort = null)
            => _inner.TdccDistributionSync(symbol, from, to, sort);
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
        /// <param name="afterHours">True for after-hours session</param>
        /// <returns>FutOptQuote with price and trading data</returns>
        public Task<string> GetQuoteAsync(string symbol, bool afterHours = false)
            => _inner.GetQuote(symbol, afterHours);

        /// <summary>
        /// Get ticker information for a contract (async).
        /// </summary>
        /// <param name="symbol">Contract symbol</param>
        /// <param name="afterHours">True for after-hours session</param>
        /// <returns>FutOptTicker with contract metadata</returns>
        public Task<string> GetTickerAsync(string symbol, bool afterHours = false)
            => _inner.GetTicker(symbol, afterHours);

        /// <summary>
        /// Get available products list (async).
        /// </summary>
        /// <param name="type">Product type: "F" for futures, "O" for options</param>
        /// <returns>ProductsResponse with available contracts</returns>
        public Task<string> GetProductsAsync(string type)
            => _inner.GetProducts(type);

        /// <summary>
        /// Get candlestick data for a futures/options contract (async).
        /// </summary>
        /// <param name="symbol">Contract symbol</param>
        /// <param name="timeframe">Candle timeframe: "1", "5", "10", "15", "30", "60" (minutes)</param>
        public Task<string> GetCandlesAsync(string symbol, string timeframe)
            => _inner.GetCandles(symbol, timeframe);

        /// <summary>
        /// Get trade history for a futures/options contract (async).
        /// </summary>
        /// <param name="symbol">Contract symbol</param>
        public Task<string> GetTradesAsync(string symbol)
            => _inner.GetTrades(symbol);

        /// <summary>
        /// Get volume breakdown by price for a futures/options contract (async).
        /// </summary>
        /// <param name="symbol">Contract symbol</param>
        public Task<string> GetVolumesAsync(string symbol)
            => _inner.GetVolumes(symbol);

        /// <summary>
        /// Get batch tickers for futures/options (async).
        /// </summary>
        /// <param name="type">Product type: "F" for futures, "O" for options</param>
        /// <param name="isSpread">Filter to spread (true) or non-spread (false) contracts; null returns both</param>
        public Task<string> GetTickersAsync(string type, bool? isSpread = null)
            => _inner.GetTickers(type, isSpread);

        // ========== Sync Methods (Blocking) ==========

        /// <summary>
        /// Get real-time quote for a futures/options contract (blocking).
        /// </summary>
        /// <param name="symbol">Contract symbol</param>
        /// <param name="afterHours">True for after-hours session</param>
        /// <returns>FutOptQuote with price and trading data</returns>
        public string GetQuote(string symbol, bool afterHours = false)
            => _inner.QuoteSync(symbol, afterHours);

        /// <summary>
        /// Get ticker information for a contract (blocking).
        /// </summary>
        /// <param name="symbol">Contract symbol</param>
        /// <param name="afterHours">True for after-hours session</param>
        /// <returns>FutOptTicker with contract metadata</returns>
        public string GetTicker(string symbol, bool afterHours = false)
            => _inner.TickerSync(symbol, afterHours);

        /// <summary>
        /// Get available products list (blocking).
        /// </summary>
        /// <param name="type">Product type: "F" for futures, "O" for options</param>
        /// <returns>ProductsResponse with available contracts</returns>
        public string GetProducts(string type)
            => _inner.ProductsSync(type);

        /// <summary>
        /// Get candlestick data for a futures/options contract (blocking).
        /// </summary>
        public string GetCandles(string symbol, string timeframe)
            => _inner.CandlesSync(symbol, timeframe);

        /// <summary>
        /// Get trade history for a futures/options contract (blocking).
        /// </summary>
        public string GetTrades(string symbol)
            => _inner.TradesSync(symbol);

        /// <summary>
        /// Get volume breakdown by price for a futures/options contract (blocking).
        /// </summary>
        public string GetVolumes(string symbol)
            => _inner.VolumesSync(symbol);

        /// <summary>
        /// Get batch tickers for futures/options (blocking).
        /// </summary>
        /// <param name="type">Product type: "F" for futures, "O" for options</param>
        /// <param name="isSpread">Filter to spread (true) or non-spread (false) contracts; null returns both</param>
        public string GetTickers(string type, bool? isSpread = null)
            => _inner.TickersSync(type, isSpread);
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
        /// <param name="from">Start date in YYYY-MM-DD format (optional)</param>
        /// <param name="to">End date in YYYY-MM-DD format (optional)</param>
        /// <param name="timeframe">Timeframe (optional)</param>
        /// <param name="afterHours">True for after-hours session</param>
        /// <param name="contractMonth">"YYYYMM", or a continuous contract: "1!" (server default), "2!", "3!"</param>
        /// <param name="fields">Comma-separated fields, e.g. "open,high,low,close,volume" (optional)</param>
        /// <param name="sort">"asc" or "desc" (optional)</param>
        public Task<string> GetCandlesAsync(
            string symbol, string? from = null, string? to = null, string? timeframe = null, bool afterHours = false,
            string? contractMonth = null, string? fields = null, string? sort = null)
            => _inner.GetCandles(symbol, from, to, timeframe, afterHours, contractMonth, fields, sort);

        /// <summary>
        /// Get one trading day's daily quotes for every contract month of a futures/options product (async).
        /// </summary>
        /// <param name="symbol">Product code, e.g. "TXF" (a contract code such as "TXFC4" returns 404)</param>
        /// <param name="date">Trading date in YYYY-MM-DD format; the server defaults to today (optional)</param>
        /// <param name="afterHours">True for after-hours session</param>
        public Task<string> GetDailyAsync(string symbol, string? date = null, bool afterHours = false)
            => _inner.GetDaily(symbol, date, afterHours);

        // ========== Sync Methods ==========

        /// <summary>
        /// Get historical candles for a futures/options product (blocking).
        /// </summary>
        public string GetCandles(
            string symbol, string? from = null, string? to = null, string? timeframe = null, bool afterHours = false,
            string? contractMonth = null, string? fields = null, string? sort = null)
            => _inner.CandlesSync(symbol, from, to, timeframe, afterHours, contractMonth, fields, sort);

        /// <summary>
        /// Get one trading day's daily quotes for every contract month of a futures/options product (blocking).
        /// </summary>
        public string GetDaily(string symbol, string? date = null, bool afterHours = false)
            => _inner.DailySync(symbol, date, afterHours);
    }
}
