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
        /// <exception cref="ArgumentNullException">If apiKey is null or empty</exception>
        public RestClient(string apiKey)
        {
            if (string.IsNullOrEmpty(apiKey))
                throw new ArgumentNullException(nameof(apiKey));

            _inner = uniffi.marketdata_uniffi.MarketdataUniffiMethods.NewRestClientWithApiKey(apiKey);
        }

        /// <summary>
        /// Create a new REST client with configuration options.
        /// Exactly one authentication method must be provided in the options.
        /// </summary>
        /// <param name="options">Configuration options including authentication</param>
        /// <exception cref="ArgumentNullException">If options is null</exception>
        /// <exception cref="ArgumentException">If zero or multiple authentication methods are provided</exception>
        public RestClient(RestClientOptions options)
        {
            if (options == null)
                throw new ArgumentNullException(nameof(options));

            // Count non-null/non-empty auth properties
            int authCount = 0;
            if (!string.IsNullOrEmpty(options.ApiKey)) authCount++;
            if (!string.IsNullOrEmpty(options.BearerToken)) authCount++;
            if (!string.IsNullOrEmpty(options.SdkToken)) authCount++;

            // Validate exactly-one-auth
            if (authCount == 0)
                throw new ArgumentException("Provide exactly one of: ApiKey, BearerToken, SdkToken", nameof(options));
            if (authCount > 1)
                throw new ArgumentException("Provide exactly one of: ApiKey, BearerToken, SdkToken", nameof(options));

            // Dispatch to correct UniFFI constructor based on which auth is set
            try
            {
                if (!string.IsNullOrEmpty(options.ApiKey))
                {
                    _inner = uniffi.marketdata_uniffi.MarketdataUniffiMethods.NewRestClientWithApiKey(options.ApiKey);
                }
                else if (!string.IsNullOrEmpty(options.BearerToken))
                {
                    _inner = uniffi.marketdata_uniffi.MarketdataUniffiMethods.NewRestClientWithBearerToken(options.BearerToken);
                }
                else // SdkToken
                {
                    _inner = uniffi.marketdata_uniffi.MarketdataUniffiMethods.NewRestClientWithSdkToken(options.SdkToken!);
                }

                // TODO: Apply BaseUrl when UniFFI RestClient exposes base_url() setter
                // Currently storing for future use but not applied
                if (!string.IsNullOrEmpty(options.BaseUrl))
                {
                    // BaseUrl configuration will be implemented when core library supports it
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
        /// Create a new REST client with SDK token authentication.
        /// </summary>
        /// <param name="sdkToken">Fugle SDK token</param>
        /// <returns>A new RestClient instance</returns>
        /// <exception cref="ArgumentNullException">If sdkToken is null or empty</exception>
        public static RestClient WithSdkToken(string sdkToken)
        {
            if (string.IsNullOrEmpty(sdkToken))
                throw new ArgumentNullException(nameof(sdkToken));

            var inner = uniffi.marketdata_uniffi.MarketdataUniffiMethods.NewRestClientWithSdkToken(sdkToken);
            return new RestClient(inner);
        }

        /// <summary>
        /// Create a new REST client with bearer token authentication.
        /// </summary>
        /// <param name="bearerToken">OAuth bearer token</param>
        /// <returns>A new RestClient instance</returns>
        /// <exception cref="ArgumentNullException">If bearerToken is null or empty</exception>
        public static RestClient WithBearerToken(string bearerToken)
        {
            if (string.IsNullOrEmpty(bearerToken))
                throw new ArgumentNullException(nameof(bearerToken));

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
        public Task<uniffi.marketdata_uniffi.Quote> GetQuoteAsync(string symbol)
            => _inner.GetQuote(symbol);

        /// <summary>
        /// Get ticker information for a stock symbol (async).
        /// </summary>
        /// <param name="symbol">Stock symbol (e.g., "2330")</param>
        /// <returns>Ticker with stock metadata and trading rules</returns>
        public Task<uniffi.marketdata_uniffi.Ticker> GetTickerAsync(string symbol)
            => _inner.GetTicker(symbol);

        /// <summary>
        /// Get trade history for a stock symbol (async).
        /// </summary>
        /// <param name="symbol">Stock symbol (e.g., "2330")</param>
        /// <returns>TradesResponse with list of executed trades</returns>
        public Task<uniffi.marketdata_uniffi.TradesResponse> GetTradesAsync(string symbol)
            => _inner.GetTrades(symbol);

        /// <summary>
        /// Get candlestick data for a stock symbol (async).
        /// </summary>
        /// <param name="symbol">Stock symbol (e.g., "2330")</param>
        /// <param name="timeframe">Candle timeframe: "1", "5", "10", "15", "30", "60" (minutes)</param>
        /// <returns>IntradayCandlesResponse with OHLCV data</returns>
        public Task<uniffi.marketdata_uniffi.IntradayCandlesResponse> GetCandlesAsync(string symbol, string timeframe)
            => _inner.GetCandles(symbol, timeframe);

        /// <summary>
        /// Get volume breakdown by price for a stock symbol (async).
        /// </summary>
        /// <param name="symbol">Stock symbol (e.g., "2330")</param>
        /// <returns>VolumesResponse with volume at each price level</returns>
        public Task<uniffi.marketdata_uniffi.VolumesResponse> GetVolumesAsync(string symbol)
            => _inner.GetVolumes(symbol);

        /// <summary>
        /// Get batch tickers for a security type (async).
        /// </summary>
        /// <param name="type">Security type (e.g., "EQUITY", "INDEX", "ETF")</param>
        /// <returns>List of tickers matching the type filter</returns>
        public Task<System.Collections.Generic.List<uniffi.marketdata_uniffi.Ticker>> GetTickersAsync(string type)
            => _inner.GetTickers(type);

        // ========== Sync Methods (Blocking) ==========

        /// <summary>
        /// Get real-time quote for a stock symbol (blocking).
        /// </summary>
        /// <param name="symbol">Stock symbol (e.g., "2330")</param>
        /// <returns>Quote with price, volume, and order book data</returns>
        public uniffi.marketdata_uniffi.Quote GetQuote(string symbol)
            => _inner.QuoteSync(symbol);

        /// <summary>
        /// Get ticker information for a stock symbol (blocking).
        /// </summary>
        /// <param name="symbol">Stock symbol (e.g., "2330")</param>
        /// <returns>Ticker with stock metadata and trading rules</returns>
        public uniffi.marketdata_uniffi.Ticker GetTicker(string symbol)
            => _inner.TickerSync(symbol);

        /// <summary>
        /// Get trade history for a stock symbol (blocking).
        /// </summary>
        /// <param name="symbol">Stock symbol (e.g., "2330")</param>
        /// <returns>TradesResponse with list of executed trades</returns>
        public uniffi.marketdata_uniffi.TradesResponse GetTrades(string symbol)
            => _inner.TradesSync(symbol);

        /// <summary>
        /// Get candlestick data for a stock symbol (blocking).
        /// </summary>
        /// <param name="symbol">Stock symbol (e.g., "2330")</param>
        /// <param name="timeframe">Candle timeframe: "1", "5", "10", "15", "30", "60" (minutes)</param>
        /// <returns>IntradayCandlesResponse with OHLCV data</returns>
        public uniffi.marketdata_uniffi.IntradayCandlesResponse GetCandles(string symbol, string timeframe)
            => _inner.CandlesSync(symbol, timeframe);

        /// <summary>
        /// Get volume breakdown by price for a stock symbol (blocking).
        /// </summary>
        /// <param name="symbol">Stock symbol (e.g., "2330")</param>
        /// <returns>VolumesResponse with volume at each price level</returns>
        public uniffi.marketdata_uniffi.VolumesResponse GetVolumes(string symbol)
            => _inner.VolumesSync(symbol);

        /// <summary>
        /// Get batch tickers for a security type (blocking).
        /// </summary>
        public System.Collections.Generic.List<uniffi.marketdata_uniffi.Ticker> GetTickers(string type)
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
        public Task<uniffi.marketdata_uniffi.HistoricalCandlesResponse> GetCandlesAsync(
            string symbol, string? from = null, string? to = null, string? timeframe = null)
            => _inner.GetCandles(symbol, from, to, timeframe);

        /// <summary>
        /// Get historical stats for a stock symbol (async).
        /// </summary>
        /// <param name="symbol">Stock symbol (e.g., "2330")</param>
        public Task<uniffi.marketdata_uniffi.StatsResponse> GetStatsAsync(string symbol)
            => _inner.GetStats(symbol);

        // ========== Sync Methods ==========

        /// <summary>
        /// Get historical candles for a stock symbol (blocking).
        /// </summary>
        public uniffi.marketdata_uniffi.HistoricalCandlesResponse GetCandles(
            string symbol, string? from = null, string? to = null, string? timeframe = null)
            => _inner.CandlesSync(symbol, from, to, timeframe);

        /// <summary>
        /// Get historical stats for a stock symbol (blocking).
        /// </summary>
        public uniffi.marketdata_uniffi.StatsResponse GetStats(string symbol)
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
        public Task<uniffi.marketdata_uniffi.SnapshotQuotesResponse> GetQuotesAsync(
            string market, string? typeFilter = null)
            => _inner.GetQuotes(market, typeFilter);

        /// <summary>
        /// Get top movers (gainers/losers) in a market (async).
        /// </summary>
        /// <param name="market">Market code: TSE, OTC</param>
        /// <param name="direction">"up" for gainers, "down" for losers (optional)</param>
        /// <param name="change">"percent" or "value" (optional)</param>
        public Task<uniffi.marketdata_uniffi.MoversResponse> GetMoversAsync(
            string market, string? direction = null, string? change = null)
            => _inner.GetMovers(market, direction, change);

        /// <summary>
        /// Get most actively traded stocks (async).
        /// </summary>
        /// <param name="market">Market code: TSE, OTC</param>
        /// <param name="trade">"volume" or "value" (optional)</param>
        public Task<uniffi.marketdata_uniffi.ActivesResponse> GetActivesAsync(
            string market, string? trade = null)
            => _inner.GetActives(market, trade);

        // ========== Sync Methods ==========

        /// <summary>
        /// Get market-wide snapshot quotes (blocking).
        /// </summary>
        public uniffi.marketdata_uniffi.SnapshotQuotesResponse GetQuotes(
            string market, string? typeFilter = null)
            => _inner.QuotesSync(market, typeFilter);

        /// <summary>
        /// Get top movers (blocking).
        /// </summary>
        public uniffi.marketdata_uniffi.MoversResponse GetMovers(
            string market, string? direction = null, string? change = null)
            => _inner.MoversSync(market, direction, change);

        /// <summary>
        /// Get most actively traded stocks (blocking).
        /// </summary>
        public uniffi.marketdata_uniffi.ActivesResponse GetActives(
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
        public Task<uniffi.marketdata_uniffi.SmaResponse> GetSmaAsync(
            string symbol, string? from = null, string? to = null, string? timeframe = null, uint? period = null)
            => _inner.GetSma(symbol, from, to, timeframe, period);

        /// <summary>
        /// Get Relative Strength Index (async).
        /// </summary>
        public Task<uniffi.marketdata_uniffi.RsiResponse> GetRsiAsync(
            string symbol, string? from = null, string? to = null, string? timeframe = null, uint? period = null)
            => _inner.GetRsi(symbol, from, to, timeframe, period);

        /// <summary>
        /// Get KDJ Stochastic Oscillator (async).
        /// </summary>
        public Task<uniffi.marketdata_uniffi.KdjResponse> GetKdjAsync(
            string symbol, string? from = null, string? to = null, string? timeframe = null, uint? period = null)
            => _inner.GetKdj(symbol, from, to, timeframe, period);

        /// <summary>
        /// Get MACD indicator (async).
        /// </summary>
        public Task<uniffi.marketdata_uniffi.MacdResponse> GetMacdAsync(
            string symbol, string? from = null, string? to = null, string? timeframe = null,
            uint? fast = null, uint? slow = null, uint? signal = null)
            => _inner.GetMacd(symbol, from, to, timeframe, fast, slow, signal);

        /// <summary>
        /// Get Bollinger Bands (async).
        /// </summary>
        public Task<uniffi.marketdata_uniffi.BbResponse> GetBbAsync(
            string symbol, string? from = null, string? to = null, string? timeframe = null,
            uint? period = null, double? stddev = null)
            => _inner.GetBb(symbol, from, to, timeframe, period, stddev);

        // ========== Sync Methods ==========

        /// <summary>
        /// Get Simple Moving Average (blocking).
        /// </summary>
        public uniffi.marketdata_uniffi.SmaResponse GetSma(
            string symbol, string? from = null, string? to = null, string? timeframe = null, uint? period = null)
            => _inner.SmaSync(symbol, from, to, timeframe, period);

        /// <summary>
        /// Get Relative Strength Index (blocking).
        /// </summary>
        public uniffi.marketdata_uniffi.RsiResponse GetRsi(
            string symbol, string? from = null, string? to = null, string? timeframe = null, uint? period = null)
            => _inner.RsiSync(symbol, from, to, timeframe, period);

        /// <summary>
        /// Get KDJ Stochastic Oscillator (blocking).
        /// </summary>
        public uniffi.marketdata_uniffi.KdjResponse GetKdj(
            string symbol, string? from = null, string? to = null, string? timeframe = null, uint? period = null)
            => _inner.KdjSync(symbol, from, to, timeframe, period);

        /// <summary>
        /// Get MACD indicator (blocking).
        /// </summary>
        public uniffi.marketdata_uniffi.MacdResponse GetMacd(
            string symbol, string? from = null, string? to = null, string? timeframe = null,
            uint? fast = null, uint? slow = null, uint? signal = null)
            => _inner.MacdSync(symbol, from, to, timeframe, fast, slow, signal);

        /// <summary>
        /// Get Bollinger Bands (blocking).
        /// </summary>
        public uniffi.marketdata_uniffi.BbResponse GetBb(
            string symbol, string? from = null, string? to = null, string? timeframe = null,
            uint? period = null, double? stddev = null)
            => _inner.BbSync(symbol, from, to, timeframe, period, stddev);
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
        /// <param name="date">Specific date in YYYY-MM-DD (optional)</param>
        /// <param name="startDate">Range start date (optional)</param>
        /// <param name="endDate">Range end date (optional)</param>
        public Task<uniffi.marketdata_uniffi.CapitalChangesResponse> GetCapitalChangesAsync(
            string? date = null, string? startDate = null, string? endDate = null)
            => _inner.GetCapitalChanges(date, startDate, endDate);

        /// <summary>
        /// Get dividend announcements (async).
        /// </summary>
        public Task<uniffi.marketdata_uniffi.DividendsResponse> GetDividendsAsync(
            string? date = null, string? startDate = null, string? endDate = null)
            => _inner.GetDividends(date, startDate, endDate);

        /// <summary>
        /// Get IPO listing applicants (async).
        /// </summary>
        public Task<uniffi.marketdata_uniffi.ListingApplicantsResponse> GetListingApplicantsAsync(
            string? date = null, string? startDate = null, string? endDate = null)
            => _inner.GetListingApplicants(date, startDate, endDate);

        // ========== Sync Methods ==========

        /// <summary>
        /// Get capital structure changes (blocking).
        /// </summary>
        public uniffi.marketdata_uniffi.CapitalChangesResponse GetCapitalChanges(
            string? date = null, string? startDate = null, string? endDate = null)
            => _inner.CapitalChangesSync(date, startDate, endDate);

        /// <summary>
        /// Get dividend announcements (blocking).
        /// </summary>
        public uniffi.marketdata_uniffi.DividendsResponse GetDividends(
            string? date = null, string? startDate = null, string? endDate = null)
            => _inner.DividendsSync(date, startDate, endDate);

        /// <summary>
        /// Get IPO listing applicants (blocking).
        /// </summary>
        public uniffi.marketdata_uniffi.ListingApplicantsResponse GetListingApplicants(
            string? date = null, string? startDate = null, string? endDate = null)
            => _inner.ListingApplicantsSync(date, startDate, endDate);
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
        public Task<uniffi.marketdata_uniffi.FutOptQuote> GetQuoteAsync(string symbol, bool afterHours = false)
            => _inner.GetQuote(symbol, afterHours);

        /// <summary>
        /// Get ticker information for a contract (async).
        /// </summary>
        /// <param name="symbol">Contract symbol</param>
        /// <param name="afterHours">True for after-hours session</param>
        /// <returns>FutOptTicker with contract metadata</returns>
        public Task<uniffi.marketdata_uniffi.FutOptTicker> GetTickerAsync(string symbol, bool afterHours = false)
            => _inner.GetTicker(symbol, afterHours);

        /// <summary>
        /// Get available products list (async).
        /// </summary>
        /// <param name="type">Product type: "F" for futures, "O" for options</param>
        /// <returns>ProductsResponse with available contracts</returns>
        public Task<uniffi.marketdata_uniffi.ProductsResponse> GetProductsAsync(string type)
            => _inner.GetProducts(type);

        /// <summary>
        /// Get candlestick data for a futures/options contract (async).
        /// </summary>
        /// <param name="symbol">Contract symbol</param>
        /// <param name="timeframe">Candle timeframe: "1", "5", "10", "15", "30", "60" (minutes)</param>
        public Task<uniffi.marketdata_uniffi.IntradayCandlesResponse> GetCandlesAsync(string symbol, string timeframe)
            => _inner.GetCandles(symbol, timeframe);

        /// <summary>
        /// Get trade history for a futures/options contract (async).
        /// </summary>
        /// <param name="symbol">Contract symbol</param>
        public Task<uniffi.marketdata_uniffi.TradesResponse> GetTradesAsync(string symbol)
            => _inner.GetTrades(symbol);

        /// <summary>
        /// Get volume breakdown by price for a futures/options contract (async).
        /// </summary>
        /// <param name="symbol">Contract symbol</param>
        public Task<uniffi.marketdata_uniffi.VolumesResponse> GetVolumesAsync(string symbol)
            => _inner.GetVolumes(symbol);

        /// <summary>
        /// Get batch tickers for futures/options (async).
        /// </summary>
        /// <param name="type">Product type: "F" for futures, "O" for options</param>
        public Task<System.Collections.Generic.List<uniffi.marketdata_uniffi.FutOptTicker>> GetTickersAsync(string type)
            => _inner.GetTickers(type);

        // ========== Sync Methods (Blocking) ==========

        /// <summary>
        /// Get real-time quote for a futures/options contract (blocking).
        /// </summary>
        /// <param name="symbol">Contract symbol</param>
        /// <param name="afterHours">True for after-hours session</param>
        /// <returns>FutOptQuote with price and trading data</returns>
        public uniffi.marketdata_uniffi.FutOptQuote GetQuote(string symbol, bool afterHours = false)
            => _inner.QuoteSync(symbol, afterHours);

        /// <summary>
        /// Get ticker information for a contract (blocking).
        /// </summary>
        /// <param name="symbol">Contract symbol</param>
        /// <param name="afterHours">True for after-hours session</param>
        /// <returns>FutOptTicker with contract metadata</returns>
        public uniffi.marketdata_uniffi.FutOptTicker GetTicker(string symbol, bool afterHours = false)
            => _inner.TickerSync(symbol, afterHours);

        /// <summary>
        /// Get available products list (blocking).
        /// </summary>
        /// <param name="type">Product type: "F" for futures, "O" for options</param>
        /// <returns>ProductsResponse with available contracts</returns>
        public uniffi.marketdata_uniffi.ProductsResponse GetProducts(string type)
            => _inner.ProductsSync(type);

        /// <summary>
        /// Get candlestick data for a futures/options contract (blocking).
        /// </summary>
        public uniffi.marketdata_uniffi.IntradayCandlesResponse GetCandles(string symbol, string timeframe)
            => _inner.CandlesSync(symbol, timeframe);

        /// <summary>
        /// Get trade history for a futures/options contract (blocking).
        /// </summary>
        public uniffi.marketdata_uniffi.TradesResponse GetTrades(string symbol)
            => _inner.TradesSync(symbol);

        /// <summary>
        /// Get volume breakdown by price for a futures/options contract (blocking).
        /// </summary>
        public uniffi.marketdata_uniffi.VolumesResponse GetVolumes(string symbol)
            => _inner.VolumesSync(symbol);

        /// <summary>
        /// Get batch tickers for futures/options (blocking).
        /// </summary>
        public System.Collections.Generic.List<uniffi.marketdata_uniffi.FutOptTicker> GetTickers(string type)
            => _inner.TickersSync(type);
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
        /// Get historical candles for a futures/options contract (async).
        /// </summary>
        /// <param name="symbol">Contract symbol</param>
        /// <param name="from">Start date in YYYY-MM-DD format (optional)</param>
        /// <param name="to">End date in YYYY-MM-DD format (optional)</param>
        /// <param name="timeframe">Timeframe (optional)</param>
        /// <param name="afterHours">True for after-hours session</param>
        public Task<uniffi.marketdata_uniffi.FutOptHistoricalCandlesResponse> GetCandlesAsync(
            string symbol, string? from = null, string? to = null, string? timeframe = null, bool afterHours = false)
            => _inner.GetCandles(symbol, from, to, timeframe, afterHours);

        /// <summary>
        /// Get daily historical data for a futures/options contract (async).
        /// </summary>
        /// <param name="symbol">Contract symbol</param>
        /// <param name="from">Start date in YYYY-MM-DD format (optional)</param>
        /// <param name="to">End date in YYYY-MM-DD format (optional)</param>
        /// <param name="afterHours">True for after-hours session</param>
        public Task<uniffi.marketdata_uniffi.FutOptDailyResponse> GetDailyAsync(
            string symbol, string? from = null, string? to = null, bool afterHours = false)
            => _inner.GetDaily(symbol, from, to, afterHours);

        // ========== Sync Methods ==========

        /// <summary>
        /// Get historical candles for a futures/options contract (blocking).
        /// </summary>
        public uniffi.marketdata_uniffi.FutOptHistoricalCandlesResponse GetCandles(
            string symbol, string? from = null, string? to = null, string? timeframe = null, bool afterHours = false)
            => _inner.CandlesSync(symbol, from, to, timeframe, afterHours);

        /// <summary>
        /// Get daily historical data for a futures/options contract (blocking).
        /// </summary>
        public uniffi.marketdata_uniffi.FutOptDailyResponse GetDaily(
            string symbol, string? from = null, string? to = null, bool afterHours = false)
            => _inner.DailySync(symbol, from, to, afterHours);
    }
}
