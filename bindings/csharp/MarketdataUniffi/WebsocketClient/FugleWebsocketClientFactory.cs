// Entry point of the event-style WebSocket clients, in the FubonNeo shape (#204).
using System;

namespace FugleMarketData.WebsocketClient
{
    /// <summary>
    /// Holds one lazily built <see cref="FugleWebsocketStockClient"/> and
    /// one <see cref="FugleWebsocketFutOptClient"/> over the same
    /// credentials. Dispose it to dispose the clients it built.
    /// </summary>
    /// <example>
    /// <code>
    /// using var factory = FugleWebsocketClientFactory.Create(sdkToken);
    /// var stock = factory.Stock;
    /// stock.OnMessage += raw => Console.WriteLine(raw);
    /// await stock.Connect();
    /// await stock.Subscribe(StockChannel.Trades, "2330", "2317");
    /// </code>
    /// </example>
    public sealed class FugleWebsocketClientFactory : IDisposable
    {
        private readonly Lazy<FugleWebsocketStockClient> _stock;
        private readonly Lazy<FugleWebsocketFutOptClient> _futOpt;
        private bool _disposed;
        /// <summary>
        /// Guards <see cref="_template"/> and the clients built from it, so a
        /// <see cref="SetCredentials"/> reaches a client being built.
        /// </summary>
        private readonly object _lock = new();
        private WebSocketClientOptions _template;
        private FugleWebsocketStockClient? _builtStock;
        private FugleWebsocketFutOptClient? _builtFutOpt;

        /// <summary>The Stock client, built on first access.</summary>
        /// <exception cref="ObjectDisposedException">After <see cref="Dispose"/></exception>
        public FugleWebsocketStockClient Stock
        {
            get { ThrowIfDisposed(); return _stock.Value; }
        }

        /// <summary>The FutOpt client, built on first access.</summary>
        /// <exception cref="ObjectDisposedException">After <see cref="Dispose"/></exception>
        public FugleWebsocketFutOptClient FutureOption
        {
            get { ThrowIfDisposed(); return _futOpt.Value; }
        }

        private void ThrowIfDisposed()
        {
            if (_disposed)
                throw new ObjectDisposedException(nameof(FugleWebsocketClientFactory));
        }

        /// <summary>
        /// A factory authenticating with an SDK token.
        /// </summary>
        /// <param name="sdkToken">SDK token, sent as <c>sdkToken</c> in the auth frame</param>
        /// <param name="versions">Streaming version per endpoint; null lets the server pick the latest</param>
        /// <param name="baseUrl">Custom base URL; null uses the default Fugle MarketData WebSocket URL</param>
        /// <exception cref="uniffi.marketdata_uniffi.MarketDataException">Code 1004 if sdkToken is null, empty or whitespace</exception>
        public static FugleWebsocketClientFactory Create(string sdkToken, WebsocketVersionOptions? versions = null, string? baseUrl = null)
        {
            uniffi.marketdata_uniffi.MarketdataUniffiMethods.ValidateCredentials(null, null, sdkToken);
            return new FugleWebsocketClientFactory(new WebSocketClientOptions { SdkToken = sdkToken, Versions = versions, BaseUrl = baseUrl });
        }

        /// <summary>
        /// A factory authenticating with an API key.
        /// </summary>
        /// <param name="apiKey">Fugle API key</param>
        /// <param name="versions">Streaming version per endpoint; null lets the server pick the latest</param>
        /// <param name="baseUrl">Custom base URL; null uses the default Fugle MarketData WebSocket URL</param>
        /// <exception cref="uniffi.marketdata_uniffi.MarketDataException">Code 1004 if apiKey is null, empty or whitespace</exception>
        public static FugleWebsocketClientFactory CreateWithApiKey(string apiKey, WebsocketVersionOptions? versions = null, string? baseUrl = null)
        {
            uniffi.marketdata_uniffi.MarketdataUniffiMethods.ValidateCredentials(apiKey, null, null);
            return new FugleWebsocketClientFactory(new WebSocketClientOptions { ApiKey = apiKey, Versions = versions, BaseUrl = baseUrl });
        }

        private FugleWebsocketClientFactory(WebSocketClientOptions template)
        {
            _template = template;
            _stock = new Lazy<FugleWebsocketStockClient>(() =>
            {
                lock (_lock)
                    return _builtStock = new FugleWebsocketStockClient(WithEndpoint(_template, WebSocketEndpoint.Stock));
            });
            _futOpt = new Lazy<FugleWebsocketFutOptClient>(() =>
            {
                lock (_lock)
                    return _builtFutOpt = new FugleWebsocketFutOptClient(WithEndpoint(_template, WebSocketEndpoint.FutOpt));
            });
        }

        /// <summary>
        /// Replace the credential of both clients, built or not, from their
        /// next connection attempt on: the next <c>Connect</c> or automatic
        /// reconnect. Any of the three kinds may replace any other. Call it
        /// before a token expires (an SDK token is valid for two days): a live
        /// connection is not authenticated again. Rejected credentials still end
        /// automatic reconnection; set a new credential, then call <c>Connect</c>
        /// again. <c>Stock.SetCredentials</c> changes the Stock client's alone.
        /// </summary>
        /// <param name="apiKey">Fugle API key</param>
        /// <param name="bearerToken">OAuth bearer token</param>
        /// <param name="sdkToken">Fugle SDK token</param>
        /// <exception cref="uniffi.marketdata_uniffi.MarketDataException">Code 1004 unless exactly one non-empty credential is given; the current credential is then kept</exception>
        /// <exception cref="ObjectDisposedException">After <see cref="Dispose"/></exception>
        public void SetCredentials(string? apiKey = null, string? bearerToken = null, string? sdkToken = null)
        {
            ThrowIfDisposed();
            uniffi.marketdata_uniffi.MarketdataUniffiMethods.ValidateCredentials(apiKey, bearerToken, sdkToken);
            lock (_lock)
            {
                ThrowIfDisposed();
                _template = new WebSocketClientOptions
                {
                    ApiKey = apiKey,
                    BearerToken = bearerToken,
                    SdkToken = sdkToken,
                    BaseUrl = _template.BaseUrl,
                    Versions = _template.Versions,
                };
                _builtStock?.SetCredentials(apiKey, bearerToken, sdkToken);
                _builtFutOpt?.SetCredentials(apiKey, bearerToken, sdkToken);
            }
        }

        private static WebSocketClientOptions WithEndpoint(WebSocketClientOptions template, WebSocketEndpoint endpoint) =>
            new WebSocketClientOptions
            {
                ApiKey = template.ApiKey,
                BearerToken = template.BearerToken,
                SdkToken = template.SdkToken,
                BaseUrl = template.BaseUrl,
                Versions = template.Versions,
                Endpoint = endpoint,
            };

        /// <summary>Dispose the clients built so far.</summary>
        public void Dispose()
        {
            // Under the lock, so a SetCredentials never reaches a disposed client.
            lock (_lock)
            {
                if (_disposed) return;
                _disposed = true;
                if (_stock.IsValueCreated) _stock.Value.Dispose();
                if (_futOpt.IsValueCreated) _futOpt.Value.Dispose();
            }
        }
    }
}
