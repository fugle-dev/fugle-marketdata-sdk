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
        /// <param name="error">The unified error info (code, source kind, message, HTTP details)</param>
        void OnError(uniffi.marketdata_uniffi.ErrorInfo error);

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

        /// <summary>
        /// Called when messages were dropped because <see cref="OnMessage"/> fell
        /// behind while the client's message queue was full
        /// (<see cref="MessageOverflow.DropNewest"/>).
        ///
        /// Note: this library multi-targets netstandard2.0, which does not
        /// support default interface implementations, so this method has no
        /// default body. Existing <see cref="IWebSocketListener"/> implementations
        /// must add it when upgrading.
        /// </summary>
        /// <param name="count">Number of messages dropped since the previous call.
        /// The first drop on a connection is reported at once, later ones at most
        /// once per second, and any remainder before <see cref="OnDisconnected"/>.
        /// The connection's running total is available via <see cref="WebSocketClient.MessagesDroppedTotal"/>.</param>
        void OnMessagesDropped(ulong count);
    }

    /// <summary>
    /// Internal adapter to convert IWebSocketListener to UniFFI WebSocketListener interface.
    ///
    /// Each callback is wrapped so an exception thrown by the user's
    /// listener cannot cross the FFI boundary (#83): it is caught, reported
    /// to <see cref="IWebSocketListener.OnError"/> as a code 3004
    /// <c>CALLBACK_FAILED</c> error (throttled — see <see cref="ReportThrottle"/>),
    /// and the stream keeps running. An exception from
    /// <see cref="IWebSocketListener.OnError"/> itself — whether reporting a
    /// callback failure or a normal SDK error — is neither re-raised nor
    /// re-reported; it is printed to <see cref="Console.Error"/>.
    /// </summary>
    internal class WebSocketListenerAdapter : uniffi.marketdata_uniffi.WebSocketListener
    {
        /// <summary>Numeric code for a listener callback throwing (mirrors the Rust core's error_code table).</summary>
        internal const int CallbackFailedCode = 3004;

        private readonly IWebSocketListener _listener;
        private readonly ReportThrottle _throttle;

        public WebSocketListenerAdapter(IWebSocketListener listener)
            : this(listener, new ReportThrottle())
        {
        }

        /// <summary>Test-only constructor to inject the throttle's clock.</summary>
        internal WebSocketListenerAdapter(IWebSocketListener listener, ReportThrottle throttle)
        {
            _listener = listener ?? throw new ArgumentNullException(nameof(listener));
            _throttle = throttle ?? throw new ArgumentNullException(nameof(throttle));
        }

        public void OnConnected() => Invoke(nameof(OnConnected), () => _listener.OnConnected());
        public void OnAuthenticated(string? dataJson) => Invoke(nameof(OnAuthenticated), () => _listener.OnAuthenticated(dataJson));
        public void OnUnauthenticated(string? dataJson) => Invoke(nameof(OnUnauthenticated), () => _listener.OnUnauthenticated(dataJson));
        public void OnDisconnected(bool willReconnect) => Invoke(nameof(OnDisconnected), () => _listener.OnDisconnected(willReconnect));
        public void OnMessage(uniffi.marketdata_uniffi.StreamMessage message) => Invoke(nameof(OnMessage), () => _listener.OnMessage(message));
        public void OnError(uniffi.marketdata_uniffi.ErrorInfo error) => InvokeOnError(() => _listener.OnError(error));
        public void OnReconnecting(uint attempt) => Invoke(nameof(OnReconnecting), () => _listener.OnReconnecting(attempt));
        public void OnReconnectFailed(uint attempts) => Invoke(nameof(OnReconnectFailed), () => _listener.OnReconnectFailed(attempts));
        public void OnMessagesDropped(ulong count) => Invoke(nameof(OnMessagesDropped), () => _listener.OnMessagesDropped(count));

        /// <summary>Run a listener method; a thrown exception is reported via OnError instead of crossing the FFI boundary.</summary>
        private void Invoke(string methodName, Action call)
        {
            try
            {
                call();
            }
            catch (Exception ex)
            {
                ReportCallbackFailure(methodName, ex);
            }
        }

        /// <summary>Run OnError itself; a thrown exception is printed, never re-raised or re-reported.</summary>
        private void InvokeOnError(Action call)
        {
            try
            {
                call();
            }
            catch (Exception ex)
            {
                Console.Error.WriteLine(ex.ToString());
            }
        }

        private void ReportCallbackFailure(string methodName, Exception ex)
        {
            var count = _throttle.Record();
            if (count == null)
            {
                return;
            }

            var message = $"Listener {methodName} threw {ex.GetType().FullName}: {ex.Message} ({count} in the last 1s)";
            var error = new uniffi.marketdata_uniffi.ErrorInfo(
                code: CallbackFailedCode,
                sourceKind: uniffi.marketdata_uniffi.ErrorSourceKind.Client,
                message: message,
                status: null,
                body: null,
                requestId: null,
                headers: new System.Collections.Generic.Dictionary<string, string>()
            );

            InvokeOnError(() => _listener.OnError(error));
        }
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
    ///     public void OnError(uniffi.marketdata_uniffi.ErrorInfo error) => Console.WriteLine($"Error: {error.message}");
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
        /// <exception cref="ArgumentNullException">If listener is null</exception>
        /// <exception cref="uniffi.marketdata_uniffi.MarketDataException">Code 1004 if apiKey is null, empty or whitespace</exception>
        public WebSocketClient(string apiKey, IWebSocketListener listener)
        {
            if (listener == null)
                throw new ArgumentNullException(nameof(listener));
            uniffi.marketdata_uniffi.MarketdataUniffiMethods.ValidateCredentials(apiKey, null, null);

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
        /// <exception cref="ArgumentNullException">If listener is null</exception>
        /// <exception cref="uniffi.marketdata_uniffi.MarketDataException">Code 1004 if apiKey is null, empty or whitespace</exception>
        public WebSocketClient(string apiKey, IWebSocketListener listener, WebSocketEndpoint endpoint)
        {
            if (listener == null)
                throw new ArgumentNullException(nameof(listener));
            uniffi.marketdata_uniffi.MarketdataUniffiMethods.ValidateCredentials(apiKey, null, null);

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
        /// Exactly one non-empty authentication method must be provided in the
        /// options; an empty or whitespace-only value counts as not provided.
        /// </summary>
        /// <param name="options">Configuration options including authentication, connection, and
        /// message queue (<see cref="WebSocketClientOptions.MessageOverflow"/>, <see cref="WebSocketClientOptions.MessageBuffer"/>) settings</param>
        /// <param name="listener">Listener to receive WebSocket events</param>
        /// <exception cref="ArgumentNullException">If options or listener is null</exception>
        /// <exception cref="uniffi.marketdata_uniffi.MarketDataException">Code 1004 if zero or multiple non-empty authentication methods are provided</exception>
        /// <exception cref="ArgumentOutOfRangeException">If <see cref="WebSocketClientOptions.MessageBuffer"/> is set to a value that is not greater than 0</exception>
        public WebSocketClient(WebSocketClientOptions options, IWebSocketListener listener)
        {
            if (options == null)
                throw new ArgumentNullException(nameof(options));
            if (listener == null)
                throw new ArgumentNullException(nameof(listener));

            if (options.MessageBuffer.HasValue && options.MessageBuffer.Value <= 0)
                throw new ArgumentOutOfRangeException(nameof(options.MessageBuffer), options.MessageBuffer, "MessageBuffer must be greater than 0 when set");

            // Create adapter
            var adapter = new WebSocketListenerAdapter(listener);

            // Convert endpoint
            var uniffiEndpoint = options.Endpoint switch
            {
                WebSocketEndpoint.Stock => uniffi.marketdata_uniffi.WebSocketEndpoint.Stock,
                WebSocketEndpoint.FutOpt => uniffi.marketdata_uniffi.WebSocketEndpoint.FutOpt,
                _ => throw new ArgumentOutOfRangeException(nameof(options.Endpoint))
            };

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

            uniffi.marketdata_uniffi.MessageQueueConfigRecord? messageQueueRecord = null;
            if (options.MessageOverflow != null || options.MessageBuffer != null)
            {
                var overflowRecord = options.MessageOverflow switch
                {
                    MessageOverflow.DropNewest or null => uniffi.marketdata_uniffi.MessageOverflowRecord.DropNewest,
                    MessageOverflow.Unbounded => uniffi.marketdata_uniffi.MessageOverflowRecord.Unbounded,
                    _ => throw new ArgumentOutOfRangeException(nameof(options.MessageOverflow))
                };

                messageQueueRecord = new uniffi.marketdata_uniffi.MessageQueueConfigRecord(
                    overflow: overflowRecord,
                    buffer: (uint)(options.MessageBuffer ?? 0)
                );
            }

            // Core requires exactly one non-blank credential and throws its
            // ConfigError (code 1004) unwrapped; the auth frame carries the
            // credential as apikey, token or sdkToken to match its kind.
            _inner = uniffi.marketdata_uniffi.WebSocketClient.NewWithCredentials(
                new uniffi.marketdata_uniffi.CredentialsRecord(
                    apiKey: options.ApiKey,
                    bearerToken: options.BearerToken,
                    sdkToken: options.SdkToken
                ),
                adapter,
                uniffiEndpoint,
                options.BaseUrl,
                reconnectRecord,
                healthCheckRecord,
                tls: null,
                version: null,
                messageQueue: messageQueueRecord
            );

            _reconnectOptions = options.Reconnect;
            _healthCheckOptions = options.HealthCheck;
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
        /// Whether the connection has ended: after <see cref="DisconnectAsync"/>, or after the
        /// server closed it with no reconnect to follow. False while reconnecting.
        /// </summary>
        public bool IsClosed => _inner.IsClosed();

        /// <summary>
        /// Messages dropped because they arrived while the message queue was
        /// full (<see cref="MessageOverflow.DropNewest"/>).
        /// Counted from the start of the current connection (every connect or
        /// reconnect restarts it); after <see cref="DisconnectAsync"/> it still
        /// reads the last connection's count. 0 before the first connect.
        /// </summary>
        public ulong MessagesDroppedTotal => _inner.MessagesDroppedTotal();

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
