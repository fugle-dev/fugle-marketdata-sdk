// Public wrapper over the UniFFI-generated REST bindings.
//
// The surface is the one of FubonNeo 2.3.0's built-in `FugleMarketData` client
// (#203): the same client tree, the same method names, and the request models
// under `FugleMarketData.QueryModels.*`, so code written against it compiles
// here with its `using` lines unchanged. What differs: every method returns
// the server's JSON as `Task<string>` (FubonNeo: `Task<HttpResponseMessage>`)
// and a failed request throws `MarketDataException` instead of coming back as
// a non-success status. Each endpoint has three names — the FubonNeo one
// (`Trades`), `GetTradesAsync` (same `Task<string>`) and the blocking
// `GetTrades` — and one parameter shape.
using System;
using System.Threading.Tasks;
using FugleMarketData.QueryModels;
using FugleMarketData.QueryModels.FuOpt;
using FugleMarketData.QueryModels.Stock.CorporateActions;
using FugleMarketData.QueryModels.Stock.History;
using FugleMarketData.QueryModels.Stock.Intraday;
using FugleMarketData.QueryModels.Stock.Ownership;
using FugleMarketData.QueryModels.Stock.Snapshot;
using FugleMarketData.QueryModels.Stock.Technical;
using FuOptHistorical = FugleMarketData.QueryModels.FuOpt.Historical;
using FuOptIntraday = FugleMarketData.QueryModels.FuOpt.Intraday;

namespace FugleMarketData
{
    /// <summary>
    /// REST API client for Fugle MarketData.
    /// Provides async methods for stock and FutOpt market data.
    /// </summary>
    /// <remarks>
    /// This is a public wrapper over UniFFI-generated bindings with the surface
    /// of FubonNeo 2.3.0's built-in <c>FugleMarketData</c> client: the same
    /// client tree, method names and request models
    /// (<c>FugleMarketData.QueryModels.*</c>), so code written against it
    /// compiles here with its <c>using</c> lines unchanged. What differs:
    /// every method returns the server's JSON as <c>Task&lt;string&gt;</c>
    /// (FubonNeo: <c>Task&lt;HttpResponseMessage&gt;</c>), and a failed request
    /// throws <see cref="uniffi.marketdata_uniffi.MarketDataException"/> instead
    /// of coming back as a non-success status. Each endpoint has three names —
    /// the FubonNeo one (<c>Trades</c>), <c>GetTradesAsync</c> (same
    /// <c>Task&lt;string&gt;</c>) and the blocking <c>GetTrades</c> — and one
    /// parameter shape. The client is <see cref="IDisposable"/>.
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

        /// <summary>
        /// FubonNeo name for <see cref="FutOpt"/>.
        /// </summary>
        public FutOptClient FutureOption => FutOpt;

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
        /// FubonNeo name for <see cref="Historical"/>.
        /// </summary>
        public StockHistoricalClient History => Historical;

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
    /// Every endpoint has a FubonNeo-named method and a <c>Get*Async</c>
    /// method (both <c>Task&lt;string&gt;</c>) plus a blocking <c>Get*</c>.
    /// </summary>
    public sealed class StockIntradayClient
    {
        private readonly uniffi.marketdata_uniffi.StockIntradayClient _inner;

        internal StockIntradayClient(uniffi.marketdata_uniffi.StockIntradayClient inner)
        {
            _inner = inner;
        }

        // ========== Async Methods ==========

        /// <summary>
        /// List stocks or indices of one security type (async).
        /// </summary>
        /// <param name="type">Security type; sent upper-cased (<c>EQUITY</c>, <c>INDEX</c>, <c>WARRANT</c>, <c>ODDLOT</c>)</param>
        /// <param name="request">Optional filters: market, exchange, industry, isNormal, isAttention, isDisposition, isHalted, symbol</param>
        public Task<string> Tickers(TickersType type = TickersType.Equity, TickersRequest? request = null)
            => _inner.GetTickers(TypeArg(type), request?.ToParams());

        /// <inheritdoc cref="Tickers"/>
        public Task<string> GetTickersAsync(TickersType type = TickersType.Equity, TickersRequest? request = null)
            => Tickers(type, request);

        /// <summary>
        /// Get ticker information for a stock symbol (async).
        /// </summary>
        /// <param name="symbol">Stock symbol (e.g., "2330")</param>
        /// <param name="request">Optional filter: <see cref="TickerType.OddLot"/></param>
        /// <returns>Ticker with stock metadata and trading rules</returns>
        public Task<string> Ticker(string symbol, TickerRequest? request = null)
            => _inner.GetTicker(symbol, request?.ToParams());

        /// <inheritdoc cref="Ticker"/>
        public Task<string> GetTickerAsync(string symbol, TickerRequest? request = null)
            => Ticker(symbol, request);

        /// <summary>
        /// Get real-time quote for a stock symbol (async).
        /// </summary>
        /// <param name="symbol">Stock symbol (e.g., "2330" for TSMC)</param>
        /// <param name="request">Optional filter: <see cref="TickerType.OddLot"/></param>
        /// <returns>Quote with price, volume, and order book data</returns>
        public Task<string> Quote(string symbol, QuoteRequest? request = null)
            => _inner.GetQuote(symbol, request?.ToParams());

        /// <inheritdoc cref="Quote"/>
        public Task<string> GetQuoteAsync(string symbol, QuoteRequest? request = null)
            => Quote(symbol, request);

        /// <summary>
        /// Get candlestick data for a stock symbol (async).
        /// </summary>
        /// <param name="symbol">Stock symbol (e.g., "2330")</param>
        /// <param name="request">Optional filters: odd lot, timeframe (unset takes the server default), sort</param>
        /// <returns>IntradayCandlesResponse with OHLCV data</returns>
        public Task<string> Candles(string symbol, IntradayCandlesRequest? request = null)
            => _inner.GetCandles(symbol, request?.ToParams());

        /// <inheritdoc cref="Candles"/>
        public Task<string> GetCandlesAsync(string symbol, IntradayCandlesRequest? request = null)
            => Candles(symbol, request);

        /// <summary>
        /// Get trade history for a stock symbol (async).
        /// </summary>
        /// <param name="symbol">Stock symbol (e.g., "2330")</param>
        /// <param name="request">Optional filters: odd lot, offset, limit, sort, isTrial</param>
        /// <returns>TradesResponse with list of executed trades</returns>
        /// <exception cref="ArgumentOutOfRangeException">If offset or limit is negative</exception>
        public Task<string> Trades(string symbol, TradeRequest? request = null)
            => _inner.GetTrades(symbol, request?.ToParams());

        /// <inheritdoc cref="Trades"/>
        public Task<string> GetTradesAsync(string symbol, TradeRequest? request = null)
            => Trades(symbol, request);

        /// <summary>
        /// Get volume breakdown by price for a stock symbol (async).
        /// </summary>
        /// <param name="symbol">Stock symbol (e.g., "2330")</param>
        /// <param name="request">Optional filter: <see cref="TickerType.OddLot"/></param>
        /// <returns>VolumesResponse with volume at each price level</returns>
        public Task<string> Volume(string symbol, VolumeRequest? request = null)
            => _inner.GetVolumes(symbol, request?.ToParams());

        /// <inheritdoc cref="Volume"/>
        public Task<string> GetVolumesAsync(string symbol, VolumeRequest? request = null)
            => Volume(symbol, request);

        // ========== Sync Methods (Blocking) ==========

        /// <summary>
        /// List stocks or indices of one security type (blocking).
        /// </summary>
        public string GetTickers(TickersType type = TickersType.Equity, TickersRequest? request = null)
            => _inner.TickersSync(TypeArg(type), request?.ToParams());

        /// <summary>
        /// Get ticker information for a stock symbol (blocking).
        /// </summary>
        public string GetTicker(string symbol, TickerRequest? request = null)
            => _inner.TickerSync(symbol, request?.ToParams());

        /// <summary>
        /// Get real-time quote for a stock symbol (blocking).
        /// </summary>
        public string GetQuote(string symbol, QuoteRequest? request = null)
            => _inner.QuoteSync(symbol, request?.ToParams());

        /// <summary>
        /// Get candlestick data for a stock symbol (blocking).
        /// </summary>
        public string GetCandles(string symbol, IntradayCandlesRequest? request = null)
            => _inner.CandlesSync(symbol, request?.ToParams());

        /// <summary>
        /// Get trade history for a stock symbol (blocking).
        /// </summary>
        public string GetTrades(string symbol, TradeRequest? request = null)
            => _inner.TradesSync(symbol, request?.ToParams());

        /// <summary>
        /// Get volume breakdown by price for a stock symbol (blocking).
        /// </summary>
        public string GetVolumes(string symbol, VolumeRequest? request = null)
            => _inner.VolumesSync(symbol, request?.ToParams());

        private static string TypeArg(TickersType type) => type.ToString().ToUpperInvariant();
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
        /// <param name="request">Optional filters: from, to, timeframe, fields, adjusted, sort</param>
        public Task<string> Candles(string symbol, HistoryCandlesRequest? request = null)
            => _inner.GetCandles(symbol, request?.ToParams());

        /// <inheritdoc cref="Candles"/>
        public Task<string> GetCandlesAsync(string symbol, HistoryCandlesRequest? request = null)
            => Candles(symbol, request);

        /// <summary>
        /// Get 52-week stats for a stock symbol (async).
        /// </summary>
        /// <param name="symbol">Stock symbol (e.g., "2330")</param>
        public Task<string> Stats(string symbol)
            => _inner.GetStats(symbol);

        /// <inheritdoc cref="Stats"/>
        public Task<string> GetStatsAsync(string symbol)
            => Stats(symbol);

        // ========== Sync Methods ==========

        /// <summary>
        /// Get historical candles for a stock symbol (blocking).
        /// </summary>
        public string GetCandles(string symbol, HistoryCandlesRequest? request = null)
            => _inner.CandlesSync(symbol, request?.ToParams());

        /// <summary>
        /// Get 52-week stats for a stock symbol (blocking).
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
        /// <param name="market">Market: TSE, OTC, ESB, TIB, PSB</param>
        /// <param name="request">Optional filter: type (ALL, ALLBUT0999, COMMONSTOCK)</param>
        public Task<string> Quotes(MarketType market = MarketType.TSE, SnapshotRequest? request = null)
            => _inner.GetQuotes(market.ToString(), request?.ToParams());

        /// <inheritdoc cref="Quotes"/>
        public Task<string> GetQuotesAsync(MarketType market = MarketType.TSE, SnapshotRequest? request = null)
            => Quotes(market, request);

        /// <summary>
        /// Get top movers (gainers/losers) in a market (async).
        /// </summary>
        /// <param name="market">Market: TSE, OTC</param>
        /// <param name="direction">Gainers (<see cref="DirectionType.Up"/>) or losers</param>
        /// <param name="change">By percent or by value</param>
        /// <param name="request">Optional filters: a price threshold (operation + price), type</param>
        public Task<string> Movers(
            MarketType market = MarketType.TSE, DirectionType direction = DirectionType.Up,
            ChangeType change = ChangeType.Percent, MoverRequest? request = null)
            => _inner.GetMovers(market.ToString(), Lower(direction), Lower(change), request?.ToParams());

        /// <inheritdoc cref="Movers"/>
        public Task<string> GetMoversAsync(
            MarketType market = MarketType.TSE, DirectionType direction = DirectionType.Up,
            ChangeType change = ChangeType.Percent, MoverRequest? request = null)
            => Movers(market, direction, change, request);

        /// <summary>
        /// Get most actively traded stocks (async).
        /// </summary>
        /// <param name="market">Market: TSE, OTC</param>
        /// <param name="trade">By volume or by value</param>
        /// <param name="request">Optional filter: type</param>
        public Task<string> Actives(
            MarketType market = MarketType.TSE, TradeType trade = TradeType.Volume, SnapshotRequest? request = null)
            => _inner.GetActives(market.ToString(), Lower(trade), request?.ToParams());

        /// <inheritdoc cref="Actives"/>
        public Task<string> GetActivesAsync(
            MarketType market = MarketType.TSE, TradeType trade = TradeType.Volume, SnapshotRequest? request = null)
            => Actives(market, trade, request);

        // ========== Sync Methods ==========

        /// <summary>
        /// Get market-wide snapshot quotes (blocking).
        /// </summary>
        public string GetQuotes(MarketType market = MarketType.TSE, SnapshotRequest? request = null)
            => _inner.QuotesSync(market.ToString(), request?.ToParams());

        /// <summary>
        /// Get top movers (blocking).
        /// </summary>
        public string GetMovers(
            MarketType market = MarketType.TSE, DirectionType direction = DirectionType.Up,
            ChangeType change = ChangeType.Percent, MoverRequest? request = null)
            => _inner.MoversSync(market.ToString(), Lower(direction), Lower(change), request?.ToParams());

        /// <summary>
        /// Get most actively traded stocks (blocking).
        /// </summary>
        public string GetActives(
            MarketType market = MarketType.TSE, TradeType trade = TradeType.Volume, SnapshotRequest? request = null)
            => _inner.ActivesSync(market.ToString(), Lower(trade), request?.ToParams());

        private static string Lower<TEnum>(TEnum value) where TEnum : struct, Enum
            => value.ToString().ToLowerInvariant();
    }

    /// <summary>
    /// Stock technical indicator endpoints. The periods are carried by the
    /// request and sent as given — a request left at its default sends
    /// <c>period=0</c>, which the server rejects with 400 (FubonNeo did the
    /// same); a negative period throws <see cref="ArgumentOutOfRangeException"/>.
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
        /// <param name="request">Period (required by the server), from, to, timeframe</param>
        public Task<string> Sma(string symbol, SmaRequest? request = null)
        {
            request ??= new SmaRequest();
            return _inner.GetSma(symbol, request.PeriodArg(), request.ToParams());
        }

        /// <inheritdoc cref="Sma"/>
        public Task<string> GetSmaAsync(string symbol, SmaRequest? request = null)
            => Sma(symbol, request);

        /// <summary>
        /// Get Relative Strength Index (async).
        /// </summary>
        /// <param name="symbol">Stock symbol</param>
        /// <param name="request">Period (required by the server), from, to, timeframe</param>
        public Task<string> Rsi(string symbol, RsiRequest? request = null)
        {
            request ??= new RsiRequest();
            return _inner.GetRsi(symbol, request.PeriodArg(), request.ToParams());
        }

        /// <inheritdoc cref="Rsi"/>
        public Task<string> GetRsiAsync(string symbol, RsiRequest? request = null)
            => Rsi(symbol, request);

        /// <summary>
        /// Get KDJ Stochastic Oscillator (async).
        /// </summary>
        /// <param name="symbol">Stock symbol</param>
        /// <param name="request">R/K/D periods (required by the server), from, to, timeframe</param>
        public Task<string> Kdj(string symbol, KdjRequest? request = null)
        {
            request ??= new KdjRequest();
            return _inner.GetKdj(symbol, request.RPeriodArg(), request.KPeriodArg(), request.DPeriodArg(), request.ToParams());
        }

        /// <inheritdoc cref="Kdj"/>
        public Task<string> GetKdjAsync(string symbol, KdjRequest? request = null)
            => Kdj(symbol, request);

        /// <summary>
        /// Get MACD indicator (async).
        /// </summary>
        /// <param name="symbol">Stock symbol</param>
        /// <param name="request">Fast/slow/signal periods (required by the server), from, to, timeframe</param>
        public Task<string> Macd(string symbol, MacdRequest? request = null)
        {
            request ??= new MacdRequest();
            return _inner.GetMacd(symbol, request.FastArg(), request.SlowArg(), request.SignalArg(), request.ToParams());
        }

        /// <inheritdoc cref="Macd"/>
        public Task<string> GetMacdAsync(string symbol, MacdRequest? request = null)
            => Macd(symbol, request);

        /// <summary>
        /// Get Bollinger Bands (async).
        /// </summary>
        /// <param name="symbol">Stock symbol</param>
        /// <param name="request">Period (required by the server), from, to, timeframe</param>
        public Task<string> Bb(string symbol, BbRequest? request = null)
        {
            request ??= new BbRequest();
            return _inner.GetBb(symbol, request.PeriodArg(), request.ToParams());
        }

        /// <inheritdoc cref="Bb"/>
        public Task<string> GetBbAsync(string symbol, BbRequest? request = null)
            => Bb(symbol, request);

        // ========== Sync Methods ==========

        /// <summary>
        /// Get Simple Moving Average (blocking).
        /// </summary>
        public string GetSma(string symbol, SmaRequest? request = null)
        {
            request ??= new SmaRequest();
            return _inner.SmaSync(symbol, request.PeriodArg(), request.ToParams());
        }

        /// <summary>
        /// Get Relative Strength Index (blocking).
        /// </summary>
        public string GetRsi(string symbol, RsiRequest? request = null)
        {
            request ??= new RsiRequest();
            return _inner.RsiSync(symbol, request.PeriodArg(), request.ToParams());
        }

        /// <summary>
        /// Get KDJ Stochastic Oscillator (blocking).
        /// </summary>
        public string GetKdj(string symbol, KdjRequest? request = null)
        {
            request ??= new KdjRequest();
            return _inner.KdjSync(symbol, request.RPeriodArg(), request.KPeriodArg(), request.DPeriodArg(), request.ToParams());
        }

        /// <summary>
        /// Get MACD indicator (blocking).
        /// </summary>
        public string GetMacd(string symbol, MacdRequest? request = null)
        {
            request ??= new MacdRequest();
            return _inner.MacdSync(symbol, request.FastArg(), request.SlowArg(), request.SignalArg(), request.ToParams());
        }

        /// <summary>
        /// Get Bollinger Bands (blocking).
        /// </summary>
        public string GetBb(string symbol, BbRequest? request = null)
        {
            request ??= new BbRequest();
            return _inner.BbSync(symbol, request.PeriodArg(), request.ToParams());
        }
    }

    /// <summary>
    /// Stock corporate actions endpoints. All three take the same
    /// <see cref="CorporateActionsRequest"/>; <c>capital-changes</c> does not
    /// accept its <c>Exchange</c> and throws
    /// <see cref="uniffi.marketdata_uniffi.MarketDataException"/> (code 1005)
    /// before any request if it is set.
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
        /// <param name="request">Optional filters: startDate, endDate, sort</param>
        public Task<string> CapitalChanges(CorporateActionsRequest? request = null)
            => _inner.GetCapitalChanges(request?.ToParams());

        /// <inheritdoc cref="CapitalChanges"/>
        public Task<string> GetCapitalChangesAsync(CorporateActionsRequest? request = null)
            => CapitalChanges(request);

        /// <summary>
        /// Get dividend announcements (async).
        /// </summary>
        /// <param name="request">Optional filters: startDate, endDate, exchange, sort</param>
        public Task<string> Dividends(CorporateActionsRequest? request = null)
            => _inner.GetDividends(request?.ToParams());

        /// <inheritdoc cref="Dividends"/>
        public Task<string> GetDividendsAsync(CorporateActionsRequest? request = null)
            => Dividends(request);

        /// <summary>
        /// Get IPO listing applicants (async).
        /// </summary>
        /// <param name="request">Optional filters: startDate, endDate, exchange, sort</param>
        public Task<string> ListingApplicants(CorporateActionsRequest? request = null)
            => _inner.GetListingApplicants(request?.ToParams());

        /// <inheritdoc cref="ListingApplicants"/>
        public Task<string> GetListingApplicantsAsync(CorporateActionsRequest? request = null)
            => ListingApplicants(request);

        // ========== Sync Methods ==========

        /// <summary>
        /// Get capital structure changes (blocking).
        /// </summary>
        public string GetCapitalChanges(CorporateActionsRequest? request = null)
            => _inner.CapitalChangesSync(request?.ToParams());

        /// <summary>
        /// Get dividend announcements (blocking).
        /// </summary>
        public string GetDividends(CorporateActionsRequest? request = null)
            => _inner.DividendsSync(request?.ToParams());

        /// <summary>
        /// Get IPO listing applicants (blocking).
        /// </summary>
        public string GetListingApplicants(CorporateActionsRequest? request = null)
            => _inner.ListingApplicantsSync(request?.ToParams());
    }

    /// <summary>
    /// Stock ownership endpoints. Every method takes the same optional
    /// filters via <see cref="OwnershipRequest"/> (<c>From</c>, <c>To</c>,
    /// <c>Sort</c>); <see cref="EtfHoldingsRequest"/> is the FubonNeo name
    /// of the same shape.
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
        /// <param name="request">Optional filters: from, to, sort</param>
        public Task<string> EtfHoldings(string symbol, OwnershipRequest? request = null)
            => _inner.GetEtfHoldings(symbol, request?.ToParams());

        /// <inheritdoc cref="EtfHoldings"/>
        public Task<string> GetEtfHoldingsAsync(string symbol, OwnershipRequest? request = null)
            => EtfHoldings(symbol, request);

        /// <summary>
        /// Get daily trading by the three major institutional investors (foreign, investment trust, dealer) (async).
        /// </summary>
        /// <param name="symbol">Stock symbol (e.g. "2330")</param>
        /// <param name="request">Optional filters: from, to, sort</param>
        public Task<string> InstitutionalTrades(string symbol, OwnershipRequest? request = null)
            => _inner.GetInstitutionalTrades(symbol, request?.ToParams());

        /// <inheritdoc cref="InstitutionalTrades"/>
        public Task<string> GetInstitutionalTradesAsync(string symbol, OwnershipRequest? request = null)
            => InstitutionalTrades(symbol, request);

        /// <summary>
        /// Get monthly holdings and pledges disclosed by directors and supervisors (async).
        /// </summary>
        /// <param name="symbol">Stock symbol (e.g. "2330")</param>
        /// <param name="request">Optional filters: from, to, sort</param>
        public Task<string> DirectorHoldings(string symbol, OwnershipRequest? request = null)
            => _inner.GetDirectorHoldings(symbol, request?.ToParams());

        /// <inheritdoc cref="DirectorHoldings"/>
        public Task<string> GetDirectorHoldingsAsync(string symbol, OwnershipRequest? request = null)
            => DirectorHoldings(symbol, request);

        /// <summary>
        /// Get the weekly TDCC shareholder distribution by holding-size bracket (async).
        /// </summary>
        /// <param name="symbol">Stock symbol (e.g. "2330")</param>
        /// <param name="request">Optional filters: from, to, sort</param>
        public Task<string> TdccDistribution(string symbol, OwnershipRequest? request = null)
            => _inner.GetTdccDistribution(symbol, request?.ToParams());

        /// <inheritdoc cref="TdccDistribution"/>
        public Task<string> GetTdccDistributionAsync(string symbol, OwnershipRequest? request = null)
            => TdccDistribution(symbol, request);

        // ========== Sync Methods ==========

        /// <summary>
        /// Get the constituents an ETF held over a date range (blocking).
        /// </summary>
        public string GetEtfHoldings(string symbol, OwnershipRequest? request = null)
            => _inner.EtfHoldingsSync(symbol, request?.ToParams());

        /// <summary>
        /// Get daily trading by the three major institutional investors (foreign, investment trust, dealer) (blocking).
        /// </summary>
        public string GetInstitutionalTrades(string symbol, OwnershipRequest? request = null)
            => _inner.InstitutionalTradesSync(symbol, request?.ToParams());

        /// <summary>
        /// Get monthly holdings and pledges disclosed by directors and supervisors (blocking).
        /// </summary>
        public string GetDirectorHoldings(string symbol, OwnershipRequest? request = null)
            => _inner.DirectorHoldingsSync(symbol, request?.ToParams());

        /// <summary>
        /// Get the weekly TDCC shareholder distribution by holding-size bracket (blocking).
        /// </summary>
        public string GetTdccDistribution(string symbol, OwnershipRequest? request = null)
            => _inner.TdccDistributionSync(symbol, request?.ToParams());
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

        // ========== Async Methods ==========

        /// <summary>
        /// Get available products list (async).
        /// </summary>
        /// <param name="type">Futures or options; sent upper-cased (<c>FUTURE</c> / <c>OPTION</c>)</param>
        /// <param name="request">Optional filters: exchange, session, contractType, productStatus</param>
        /// <returns>ProductsResponse with available contracts</returns>
        public Task<string> Products(FutOptType type = FutOptType.Future, FuOptIntraday.ProductsRequest? request = null)
            => _inner.GetProducts(TypeArg(type), request?.ToParams());

        /// <inheritdoc cref="Products"/>
        public Task<string> GetProductsAsync(FutOptType type = FutOptType.Future, FuOptIntraday.ProductsRequest? request = null)
            => Products(type, request);

        /// <summary>
        /// Get batch tickers for futures/options (async).
        /// </summary>
        /// <param name="type">Futures or options; sent upper-cased (<c>FUTURE</c> / <c>OPTION</c>)</param>
        /// <param name="request">Optional filters: exchange, session, product, contractType, isSpread</param>
        public Task<string> Tickers(FutOptType type = FutOptType.Future, FuOptIntraday.TickersRequest? request = null)
            => _inner.GetTickers(TypeArg(type), request?.ToParams());

        /// <inheritdoc cref="Tickers"/>
        public Task<string> GetTickersAsync(FutOptType type = FutOptType.Future, FuOptIntraday.TickersRequest? request = null)
            => Tickers(type, request);

        /// <summary>
        /// Get ticker information for a contract (async).
        /// </summary>
        /// <param name="symbol">Contract symbol</param>
        /// <param name="request">Optional filter: session (<see cref="TradeSession.AfterHours"/>)</param>
        /// <returns>FutOptTicker with contract metadata</returns>
        public Task<string> Ticker(string symbol, FuOptIntraday.TickerVolumeRequest? request = null)
            => _inner.GetTicker(symbol, request?.ToParams());

        /// <inheritdoc cref="Ticker"/>
        public Task<string> GetTickerAsync(string symbol, FuOptIntraday.TickerVolumeRequest? request = null)
            => Ticker(symbol, request);

        /// <summary>
        /// Get real-time quote for a futures/options contract (async).
        /// </summary>
        /// <param name="symbol">Contract symbol</param>
        /// <param name="request">Optional filter: session (<see cref="TradeSession.AfterHours"/>)</param>
        /// <returns>FutOptQuote with price and trading data</returns>
        public Task<string> Quote(string symbol, FuOptIntraday.TickerVolumeRequest? request = null)
            => _inner.GetQuote(symbol, request?.ToParams());

        /// <inheritdoc cref="Quote"/>
        public Task<string> GetQuoteAsync(string symbol, FuOptIntraday.TickerVolumeRequest? request = null)
            => Quote(symbol, request);

        /// <summary>
        /// Get candlestick data for a futures/options contract (async).
        /// </summary>
        /// <param name="symbol">Contract symbol</param>
        /// <param name="request">Optional filters: session, timeframe</param>
        public Task<string> Candles(string symbol, FuOptIntraday.CandlesRequest? request = null)
            => _inner.GetCandles(symbol, request?.ToParams());

        /// <inheritdoc cref="Candles"/>
        public Task<string> GetCandlesAsync(string symbol, FuOptIntraday.CandlesRequest? request = null)
            => Candles(symbol, request);

        /// <summary>
        /// Get trade history for a futures/options contract (async).
        /// </summary>
        /// <param name="symbol">Contract symbol</param>
        /// <param name="request">Optional filters: session, offset, limit, isTrial</param>
        /// <exception cref="ArgumentOutOfRangeException">If offset or limit is negative</exception>
        public Task<string> Trades(string symbol, FuOptIntraday.TradesRequest? request = null)
            => _inner.GetTrades(symbol, request?.ToParams());

        /// <inheritdoc cref="Trades"/>
        public Task<string> GetTradesAsync(string symbol, FuOptIntraday.TradesRequest? request = null)
            => Trades(symbol, request);

        /// <summary>
        /// Get volume breakdown by price for a futures/options contract (async).
        /// </summary>
        /// <param name="symbol">Contract symbol</param>
        /// <param name="request">Optional filter: session (<see cref="TradeSession.AfterHours"/>)</param>
        public Task<string> Volumes(string symbol, FuOptIntraday.TickerVolumeRequest? request = null)
            => _inner.GetVolumes(symbol, request?.ToParams());

        /// <inheritdoc cref="Volumes"/>
        public Task<string> GetVolumesAsync(string symbol, FuOptIntraday.TickerVolumeRequest? request = null)
            => Volumes(symbol, request);

        // ========== Sync Methods (Blocking) ==========

        /// <summary>
        /// Get available products list (blocking).
        /// </summary>
        public string GetProducts(FutOptType type = FutOptType.Future, FuOptIntraday.ProductsRequest? request = null)
            => _inner.ProductsSync(TypeArg(type), request?.ToParams());

        /// <summary>
        /// Get batch tickers for futures/options (blocking).
        /// </summary>
        public string GetTickers(FutOptType type = FutOptType.Future, FuOptIntraday.TickersRequest? request = null)
            => _inner.TickersSync(TypeArg(type), request?.ToParams());

        /// <summary>
        /// Get ticker information for a contract (blocking).
        /// </summary>
        public string GetTicker(string symbol, FuOptIntraday.TickerVolumeRequest? request = null)
            => _inner.TickerSync(symbol, request?.ToParams());

        /// <summary>
        /// Get real-time quote for a futures/options contract (blocking).
        /// </summary>
        public string GetQuote(string symbol, FuOptIntraday.TickerVolumeRequest? request = null)
            => _inner.QuoteSync(symbol, request?.ToParams());

        /// <summary>
        /// Get candlestick data for a futures/options contract (blocking).
        /// </summary>
        public string GetCandles(string symbol, FuOptIntraday.CandlesRequest? request = null)
            => _inner.CandlesSync(symbol, request?.ToParams());

        /// <summary>
        /// Get trade history for a futures/options contract (blocking).
        /// </summary>
        public string GetTrades(string symbol, FuOptIntraday.TradesRequest? request = null)
            => _inner.TradesSync(symbol, request?.ToParams());

        /// <summary>
        /// Get volume breakdown by price for a futures/options contract (blocking).
        /// </summary>
        public string GetVolumes(string symbol, FuOptIntraday.TickerVolumeRequest? request = null)
            => _inner.VolumesSync(symbol, request?.ToParams());

        private static string TypeArg(FutOptType type) => type.ToString().ToUpperInvariant();
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
        /// Get one trading day's daily quotes for every contract month of a futures/options product (async).
        /// </summary>
        /// <param name="symbol">Product code, e.g. "TXF" (a contract code such as "TXFC4" returns 404)</param>
        /// <param name="request">Optional filters: date (unset defaults to today), afterHours</param>
        public Task<string> Daily(string symbol, FuOptHistorical.DailyRequest? request = null)
            => _inner.GetDaily(symbol, request?.ToParams());

        /// <inheritdoc cref="Daily"/>
        public Task<string> GetDailyAsync(string symbol, FuOptHistorical.DailyRequest? request = null)
            => Daily(symbol, request);

        /// <summary>
        /// Get historical candles for a futures/options product (async).
        /// </summary>
        /// <param name="product">Product code, e.g. "TXF" or "TXO" (a contract code such as "TXFC4" returns 404)</param>
        /// <param name="request">Optional filters: from, to, timeframe, fields, contractMonth, sort, session, strikePrice, callPut</param>
        public Task<string> Candles(string product, FuOptHistorical.HistoricalCandlesRequest? request = null)
            => _inner.GetCandles(product, request?.ToParams());

        /// <inheritdoc cref="Candles"/>
        public Task<string> GetCandlesAsync(string product, FuOptHistorical.HistoricalCandlesRequest? request = null)
            => Candles(product, request);

        // ========== Sync Methods ==========

        /// <summary>
        /// Get one trading day's daily quotes for every contract month of a futures/options product (blocking).
        /// </summary>
        public string GetDaily(string symbol, FuOptHistorical.DailyRequest? request = null)
            => _inner.DailySync(symbol, request?.ToParams());

        /// <summary>
        /// Get historical candles for a futures/options product (blocking).
        /// </summary>
        public string GetCandles(string product, FuOptHistorical.HistoricalCandlesRequest? request = null)
            => _inner.CandlesSync(product, request?.ToParams());
    }
}
