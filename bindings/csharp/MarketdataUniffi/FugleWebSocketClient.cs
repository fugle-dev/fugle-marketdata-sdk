// Public wrapper providing FubonNeo-compatible WebSocket API over UniFFI-generated bindings
using System;
using System.Threading.Tasks;

namespace FugleMarketData
{
    /// <summary>
    /// WebSocket endpoint types for market data streaming.
    /// </summary>
    public enum WebSocketEndpoint
    {
        /// <summary>
        /// Stock market data stream
        /// </summary>
        Stock,

        /// <summary>
        /// Futures and options market data stream
        /// </summary>
        FutOpt
    }

    /// <summary>
    /// Interface for receiving WebSocket events.
    /// Implement this interface to handle streaming market data.
    /// </summary>
    public interface IWebSocketListener
    {
        /// <summary>
        /// Called when the transport is established, before the server has answered
        /// the auth frame. Fires again on every successful reconnect; wait for
        /// <see cref="OnAuthenticated"/> before treating the connection as usable.
        /// </summary>
        void OnConnected();

        /// <summary>
        /// Called when the server accepts the credentials.
        /// </summary>
        /// <param name="dataJson">The <c>data</c> member of the server's frame as JSON, or null when absent</param>
        void OnAuthenticated(string? dataJson);

        /// <summary>
        /// Called when the server rejects the credentials. <c>ConnectAsync</c> also fails;
        /// no <see cref="OnError"/> is raised for the rejection.
        /// </summary>
        /// <param name="dataJson">The <c>data</c> member of the server's frame as JSON (message under <c>message</c>), or null when absent</param>
        void OnUnauthenticated(string? dataJson);

        /// <summary>
        /// Called when the connection is closed, at most once per connection.
        /// </summary>
        /// <param name="willReconnect">True when the client will try to reconnect
        /// (<see cref="OnReconnecting"/> follows); false when the connection's lifecycle has ended</param>
        void OnDisconnected(bool willReconnect);

        /// <summary>
        /// Called when a market data message is received.
        /// </summary>
        /// <param name="message">Streaming message with event, channel, symbol, and data</param>
        void OnMessage(uniffi.marketdata_uniffi.StreamMessage message);

        /// <summary>
        /// Called when an error occurs.
        /// </summary>
        /// <param name="errorMessage">Error description</param>
        void OnError(string errorMessage);

        /// <summary>
        /// Called when a reconnection attempt starts.
        /// </summary>
        /// <param name="attempt">Current attempt number (1-based)</param>
        void OnReconnecting(uint attempt);

        /// <summary>
        /// Called when all reconnection attempts are exhausted. No further lifecycle events follow.
        /// </summary>
        /// <param name="attempts">Total number of attempts made</param>
        void OnReconnectFailed(uint attempts);
    }

    /// <summary>
    /// Internal adapter to convert IWebSocketListener to UniFFI WebSocketListener interface.
    /// </summary>
    internal class WebSocketListenerAdapter : uniffi.marketdata_uniffi.WebSocketListener
    {
        private readonly IWebSocketListener _listener;

        public WebSocketListenerAdapter(IWebSocketListener listener)
        {
            _listener = listener ?? throw new ArgumentNullException(nameof(listener));
        }

        public void OnConnected() => _listener.OnConnected();
        public void OnAuthenticated(string? dataJson) => _listener.OnAuthenticated(dataJson);
        public void OnUnauthenticated(string? dataJson) => _listener.OnUnauthenticated(dataJson);
        public void OnDisconnected(bool willReconnect) => _listener.OnDisconnected(willReconnect);
        public void OnMessage(uniffi.marketdata_uniffi.StreamMessage message) => _listener.OnMessage(message);
        public void OnError(string errorMessage) => _listener.OnError(errorMessage);
        public void OnReconnecting(uint attempt) => _listener.OnReconnecting(attempt);
        public void OnReconnectFailed(uint attempts) => _listener.OnReconnectFailed(attempts);
    }

    /// <summary>
    /// WebSocket client for streaming market data.
    /// Provides real-time data via callback-based interface.
    /// </summary>
    /// <example>
    /// <code>
    /// class MyListener : IWebSocketListener
    /// {
    ///     public void OnConnected() => Console.WriteLine("Connected!");
    ///     public void OnAuthenticated(string? dataJson) => Console.WriteLine("Authenticated");
    ///     public void OnUnauthenticated(string? dataJson) => Console.WriteLine($"Rejected: {dataJson}");
    ///     public void OnDisconnected(bool willReconnect) => Console.WriteLine($"Disconnected (will reconnect: {willReconnect})");
    ///     public void OnMessage(StreamMessage msg) => Console.WriteLine($"{msg.Channel}: {msg.Symbol}");
    ///     public void OnError(string error) => Console.WriteLine($"Error: {error}");
    /// }
    ///
    /// var listener = new MyListener();
    /// using var client = new WebSocketClient("your-api-key", listener);
    /// await client.ConnectAsync();
    /// await client.SubscribeAsync("trades", "2330");
    /// // Messages will arrive via OnMessage callback
    /// </code>
    /// </example>
    public sealed class WebSocketClient : IDisposable
    {
        private readonly uniffi.marketdata_uniffi.WebSocketClient _inner;
        private bool _disposed;
        private readonly ReconnectOptions? _reconnectOptions;
        private readonly HealthCheckOptions? _healthCheckOptions;

        /// <summary>
        /// Create a WebSocket client for stock market data streaming.
        /// </summary>
        /// <param name="apiKey">Fugle API key</param>
        /// <param name="listener">Listener to receive WebSocket events</param>
        /// <exception cref="ArgumentNullException">If apiKey or listener is null</exception>
        public WebSocketClient(string apiKey, IWebSocketListener listener)
        {
            if (string.IsNullOrEmpty(apiKey))
                throw new ArgumentNullException(nameof(apiKey));
            if (listener == null)
                throw new ArgumentNullException(nameof(listener));

            var adapter = new WebSocketListenerAdapter(listener);
            _inner = new uniffi.marketdata_uniffi.WebSocketClient(apiKey, adapter);
            _reconnectOptions = null;
            _healthCheckOptions = null;
        }

        /// <summary>
        /// Create a WebSocket client for a specific endpoint (stock or futopt).
        /// </summary>
        /// <param name="apiKey">Fugle API key</param>
        /// <param name="listener">Listener to receive WebSocket events</param>
        /// <param name="endpoint">Endpoint type: Stock or FutOpt</param>
        /// <exception cref="ArgumentNullException">If apiKey or listener is null</exception>
        public WebSocketClient(string apiKey, IWebSocketListener listener, WebSocketEndpoint endpoint)
        {
            if (string.IsNullOrEmpty(apiKey))
                throw new ArgumentNullException(nameof(apiKey));
            if (listener == null)
                throw new ArgumentNullException(nameof(listener));

            var adapter = new WebSocketListenerAdapter(listener);
            var uniffiEndpoint = endpoint switch
            {
                WebSocketEndpoint.Stock => uniffi.marketdata_uniffi.WebSocketEndpoint.Stock,
                WebSocketEndpoint.FutOpt => uniffi.marketdata_uniffi.WebSocketEndpoint.FutOpt,
                _ => throw new ArgumentOutOfRangeException(nameof(endpoint))
            };

            _inner = uniffi.marketdata_uniffi.WebSocketClient.NewWithEndpoint(apiKey, adapter, uniffiEndpoint);
            _reconnectOptions = null;
            _healthCheckOptions = null;
        }

        /// <summary>
        /// Create a WebSocket client with configuration options.
        /// Exactly one authentication method must be provided in the options.
        /// </summary>
        /// <param name="options">Configuration options including authentication and connection settings</param>
        /// <param name="listener">Listener to receive WebSocket events</param>
        /// <exception cref="ArgumentNullException">If options or listener is null</exception>
        /// <exception cref="ArgumentException">If zero or multiple authentication methods are provided</exception>
        public WebSocketClient(WebSocketClientOptions options, IWebSocketListener listener)
        {
            if (options == null)
                throw new ArgumentNullException(nameof(options));
            if (listener == null)
                throw new ArgumentNullException(nameof(listener));

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

            // Create adapter
            var adapter = new WebSocketListenerAdapter(listener);

            // Convert endpoint
            var uniffiEndpoint = options.Endpoint switch
            {
                WebSocketEndpoint.Stock => uniffi.marketdata_uniffi.WebSocketEndpoint.Stock,
                WebSocketEndpoint.FutOpt => uniffi.marketdata_uniffi.WebSocketEndpoint.FutOpt,
                _ => throw new ArgumentOutOfRangeException(nameof(options.Endpoint))
            };

            // TODO: Current UniFFI WebSocketClient constructors only support API key authentication
            // BearerToken and SdkToken support will be added when UniFFI layer is updated
            try
            {
                if (!string.IsNullOrEmpty(options.ApiKey))
                {
                    // Convert config options to UniFFI record types
                    uniffi.marketdata_uniffi.ReconnectConfigRecord? reconnectRecord = null;
                    if (options.Reconnect != null)
                    {
                        reconnectRecord = new uniffi.marketdata_uniffi.ReconnectConfigRecord(
                            maxAttempts: options.Reconnect.MaxAttempts ?? 0,
                            initialDelayMs: options.Reconnect.InitialDelayMs ?? 0,
                            maxDelayMs: options.Reconnect.MaxDelayMs ?? 0
                        );
                    }

                    uniffi.marketdata_uniffi.HealthCheckConfigRecord? healthCheckRecord = null;
                    if (options.HealthCheck != null)
                    {
                        healthCheckRecord = new uniffi.marketdata_uniffi.HealthCheckConfigRecord(
                            enabled: options.HealthCheck.Enabled ?? true,
                            heartbeatTimeoutMs: options.HealthCheck.HeartbeatTimeoutMs ?? 0
                        );
                    }

                    if (!string.IsNullOrEmpty(options.BaseUrl))
                    {
                        _inner = uniffi.marketdata_uniffi.WebSocketClient.NewWithUrl(
                            options.ApiKey,
                            adapter,
                            uniffiEndpoint,
                            options.BaseUrl,
                            reconnectRecord,
                            healthCheckRecord
                        );
                    }
                    else
                    {
                        _inner = uniffi.marketdata_uniffi.WebSocketClient.NewWithConfig(
                            options.ApiKey,
                            adapter,
                            uniffiEndpoint,
                            reconnectRecord,
                            healthCheckRecord
                        );
                    }
                }
                else
                {
                    // For now, only ApiKey is supported in UniFFI WebSocketClient constructors
                    // BearerToken and SdkToken will require UniFFI layer updates
                    throw new NotSupportedException(
                        "WebSocketClient currently only supports ApiKey authentication. " +
                        "BearerToken and SdkToken support will be added in a future update."
                    );
                }

                _reconnectOptions = options.Reconnect;
                _healthCheckOptions = options.HealthCheck;
            }
            catch (uniffi.marketdata_uniffi.MarketDataException ex)
            {
                throw new InvalidOperationException($"Failed to create WebSocket client: {ex.Message}", ex);
            }
        }

        /// <summary>
        /// Connect to the WebSocket server.
        /// </summary>
        /// <returns>Task that completes when connection is established</returns>
        public Task ConnectAsync() => _inner.Connect();

        /// <summary>
        /// Disconnect from the WebSocket server.
        /// </summary>
        /// <returns>Task that completes when disconnected</returns>
        public Task DisconnectAsync() => _inner.Disconnect();

        /// <summary>
        /// Subscribe to a market data channel for a symbol.
        /// </summary>
        /// <param name="channel">Channel name: "trades", "candles", "books", "meta"</param>
        /// <param name="symbol">Symbol to subscribe (e.g., "2330" for TSMC)</param>
        /// <returns>Task that completes when subscription is confirmed</returns>
        public Task SubscribeAsync(string channel, string symbol) => _inner.Subscribe(channel, symbol);

        /// <summary>
        /// Unsubscribe from a market data channel for a symbol.
        /// </summary>
        /// <param name="channel">Channel name</param>
        /// <param name="symbol">Symbol to unsubscribe</param>
        /// <returns>Task that completes when unsubscription is confirmed</returns>
        public Task UnsubscribeAsync(string channel, string symbol) => _inner.Unsubscribe(channel, symbol);

        /// <summary>
        /// Whether the client is currently connected to the server.
        /// </summary>
        public bool IsConnected => _inner.IsConnected();

        /// <summary>
        /// Whether the client has been shut down.
        /// </summary>
        public bool IsClosed => _inner.IsClosed();

        /// <summary>
        /// Send a ping message to the server.
        /// </summary>
        /// <param name="state">Optional state string echoed back in the pong response</param>
        /// <returns>Task that completes when the ping is sent</returns>
        public Task PingAsync(string? state = null) => _inner.Ping(state);

        /// <summary>
        /// Query the server for current subscriptions.
        /// The response arrives via the OnMessage callback.
        /// </summary>
        /// <returns>Task that completes when the query is sent</returns>
        public Task QuerySubscriptionsAsync() => _inner.QuerySubscriptions();

        private void ThrowIfDisposed()
        {
            if (_disposed)
                throw new ObjectDisposedException(nameof(WebSocketClient));
        }

        /// <inheritdoc/>
        public void Dispose()
        {
            if (_disposed) return;
            _inner?.Dispose();
            _disposed = true;
        }
    }
}
