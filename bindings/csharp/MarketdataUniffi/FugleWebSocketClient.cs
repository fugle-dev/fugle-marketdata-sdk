// Public wrapper providing FubonNeo-compatible WebSocket API over UniFFI-generated bindings
using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading.Tasks;

namespace FugleMarketData
{
    /// <summary>
    /// Per-subscription options for <c>WebSocketClient.SubscribeAsync</c>
    /// and <c>WebSocketClient.UnsubscribeAsync</c>. Maps to
    /// <see cref="uniffi.marketdata_uniffi.SubscribeOptions"/>.
    /// </summary>
    public class SubscribeOptions
    {
        /// <summary>
        /// FutOpt endpoint only: true subscribes to the after-hours session.
        /// Any non-null value on the Stock endpoint is error 1005.
        /// </summary>
        public bool? AfterHours { get; set; }

        /// <summary>
        /// Stock endpoint only: true subscribes to the intraday odd-lot session.
        /// Any non-null value on the FutOpt endpoint is error 1005.
        /// </summary>
        public bool? IntradayOddLot { get; set; }
    }

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
        /// The reconnect record for core, or null to keep the core defaults.
        /// An unset <see cref="ReconnectOptions.Enabled"/> stays unset so core
        /// applies its default (on); unset numbers are 0, meaning "use default".
        /// </summary>
        internal static uniffi.marketdata_uniffi.ReconnectConfigRecord? ToReconnectRecord(ReconnectOptions? options)
        {
            if (options == null)
                return null;
            return new uniffi.marketdata_uniffi.ReconnectConfigRecord(
                enabled: options.Enabled,
                maxAttempts: options.MaxAttempts ?? 0,
                initialDelayMs: options.InitialDelayMs ?? 0,
                maxDelayMs: options.MaxDelayMs ?? 0
            );
        }

        /// <summary>
        /// The health check record for core, or null to keep the core defaults.
        /// An unset <see cref="HealthCheckOptions.Enabled"/> stays unset so
        /// core applies its default (on); unset numbers are 0, meaning "use
        /// default".
        /// </summary>
        internal static uniffi.marketdata_uniffi.HealthCheckConfigRecord? ToHealthCheckRecord(HealthCheckOptions? options)
        {
            if (options == null)
                return null;
            return new uniffi.marketdata_uniffi.HealthCheckConfigRecord(
                enabled: options.Enabled,
                heartbeatTimeoutMs: options.HeartbeatTimeoutMs ?? 0,
                probeEnabled: options.ProbeEnabled ?? false,
                idleProbeAfterMs: options.IdleProbeAfterMs ?? 0,
                probeTimeoutMs: options.ProbeTimeoutMs ?? 0
            );
        }

        /// <summary>
        /// The streaming version record for core, or null to let the server
        /// pick the latest version for every endpoint.
        /// </summary>
        internal static uniffi.marketdata_uniffi.StreamingVersionRecord? ToStreamingVersionRecord(WebsocketClient.WebsocketVersionOptions? options)
        {
            if (options == null)
                return null;
            return new uniffi.marketdata_uniffi.StreamingVersionRecord(
                stock: options.Stock,
                futopt: options.FutOpt
            );
        }

        /// <summary>
        /// The connection record for core, or null to keep the core defaults.
        /// An unset <see cref="WebSocketClientOptions.AuthTimeoutMs"/> leaves
        /// the whole record out; core's zero means "use default" (10 s).
        /// </summary>
        internal static uniffi.marketdata_uniffi.ConnectionConfigRecord? ToConnectionRecord(ulong? authTimeoutMs)
        {
            if (authTimeoutMs == null)
                return null;
            return new uniffi.marketdata_uniffi.ConnectionConfigRecord(
                authTimeoutMs: authTimeoutMs.Value
            );
        }

        /// <summary>
        /// Create a WebSocket client with configuration options.
        /// Exactly one non-empty authentication method must be provided in the
        /// options; an empty or whitespace-only value counts as not provided.
        /// </summary>
        /// <param name="options">Configuration options including authentication, connection
        /// (<see cref="WebSocketClientOptions.AuthTimeoutMs"/>), and
        /// message queue (<see cref="WebSocketClientOptions.MessageOverflow"/>, <see cref="WebSocketClientOptions.MessageBuffer"/>) settings</param>
        /// <param name="listener">Listener to receive WebSocket events</param>
        /// <exception cref="ArgumentNullException">If options or listener is null</exception>
        /// <exception cref="uniffi.marketdata_uniffi.MarketDataException">Code 1004 if zero or multiple non-empty authentication methods are provided</exception>
        /// <exception cref="ArgumentOutOfRangeException">If <see cref="WebSocketClientOptions.MessageBuffer"/> or <see cref="WebSocketClientOptions.AuthTimeoutMs"/> is set to a value that is not greater than 0</exception>
        public WebSocketClient(WebSocketClientOptions options, IWebSocketListener listener)
        {
            if (options == null)
                throw new ArgumentNullException(nameof(options));
            if (listener == null)
                throw new ArgumentNullException(nameof(listener));

            if (options.MessageBuffer.HasValue && options.MessageBuffer.Value <= 0)
                throw new ArgumentOutOfRangeException(nameof(options.MessageBuffer), options.MessageBuffer, "MessageBuffer must be greater than 0 when set");
            // 0 would read as "use default" across the FFI boundary, so it is
            // refused here rather than silently ignored.
            if (options.AuthTimeoutMs.HasValue && options.AuthTimeoutMs.Value == 0)
                throw new ArgumentOutOfRangeException(nameof(options.AuthTimeoutMs), options.AuthTimeoutMs, "AuthTimeoutMs must be greater than 0 when set");

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
            var reconnectRecord = ToReconnectRecord(options.Reconnect);
            var healthCheckRecord = ToHealthCheckRecord(options.HealthCheck);
            var versionRecord = ToStreamingVersionRecord(options.Versions);

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

            var connectionRecord = ToConnectionRecord(options.AuthTimeoutMs);

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
                version: versionRecord,
                messageQueue: messageQueueRecord,
                connection: connectionRecord
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
        /// <returns>
        /// Task that completes once the listener has handled the connection's remaining
        /// events, <see cref="IWebSocketListener.OnDisconnected"/> included. There is no
        /// timeout on that wait: a listener method that blocks keeps it waiting for as
        /// long as it does. Called from a
        /// listener method, it completes without that wait: those events are delivered on
        /// the thread running the method, after it returns.
        /// </returns>
        public Task DisconnectAsync() => _inner.Disconnect();

        /// <summary>
        /// Map the wrapper's <see cref="SubscribeOptions"/> to the generated
        /// record, or null when there is nothing to send.
        /// </summary>
        internal static uniffi.marketdata_uniffi.SubscribeOptions? ToInnerOptions(SubscribeOptions? options)
        {
            if (options == null)
                return null;
            return new uniffi.marketdata_uniffi.SubscribeOptions(
                afterHours: options.AfterHours,
                intradayOddLot: options.IntradayOddLot
            );
        }

        /// <summary>
        /// Subscribe to a market data channel for a symbol.
        /// </summary>
        /// <param name="channel">Channel name: "trades", "candles", "books", "aggregates", plus "indices" on the Stock endpoint</param>
        /// <param name="symbol">Symbol to subscribe (e.g., "2330" for TSMC)</param>
        /// <param name="afterHours">After-hours (盤後) session. FutOpt endpoint only: on the Stock endpoint any non-null value is error 1005</param>
        /// <returns>Task that completes when subscription is confirmed</returns>
        public Task SubscribeAsync(string channel, string symbol, bool? afterHours = null) =>
            SubscribeAsync(channel, new[] { symbol }, afterHours.HasValue ? new SubscribeOptions { AfterHours = afterHours } : null);

        /// <summary>
        /// Subscribe to a market data channel for one symbol, with explicit options.
        /// </summary>
        /// <param name="channel">Channel name: "trades", "candles", "books", "aggregates", plus "indices" on the Stock endpoint</param>
        /// <param name="symbol">Symbol to subscribe (e.g., "2330" for TSMC)</param>
        /// <param name="options">Endpoint-specific options: <see cref="SubscribeOptions.AfterHours"/> (FutOpt only) or <see cref="SubscribeOptions.IntradayOddLot"/> (Stock only)</param>
        /// <returns>Task that completes when subscription is confirmed</returns>
        public Task SubscribeAsync(string channel, string symbol, SubscribeOptions options) =>
            SubscribeAsync(channel, new[] { symbol }, options);

        /// <summary>
        /// Subscribe to a market data channel for one or more symbols.
        /// </summary>
        /// <param name="channel">Channel name: "trades", "candles", "books", "aggregates", plus "indices" on the Stock endpoint</param>
        /// <param name="symbols">Symbols to subscribe; at least one is required</param>
        /// <param name="options">Endpoint-specific options: <see cref="SubscribeOptions.AfterHours"/> (FutOpt only) or <see cref="SubscribeOptions.IntradayOddLot"/> (Stock only)</param>
        /// <returns>Task that completes when subscription is confirmed</returns>
        public Task SubscribeAsync(string channel, IEnumerable<string> symbols, SubscribeOptions? options = null)
        {
            if (symbols == null)
                throw new ArgumentNullException(nameof(symbols));
            return _inner.Subscribe(channel, symbols.ToArray(), ToInnerOptions(options));
        }

        /// <summary>
        /// Unsubscribe from a market data channel for a symbol.
        /// </summary>
        /// <param name="channel">Channel name</param>
        /// <param name="symbol">Symbol to unsubscribe</param>
        /// <param name="afterHours">The same value as the <c>SubscribeAsync</c> call: an after-hours subscription is separate from the regular one</param>
        /// <returns>Task that completes when unsubscription is confirmed</returns>
        public Task UnsubscribeAsync(string channel, string symbol, bool? afterHours = null) =>
            UnsubscribeAsync(channel, new[] { symbol }, afterHours.HasValue ? new SubscribeOptions { AfterHours = afterHours } : null);

        /// <summary>
        /// Unsubscribe from a market data channel for one symbol, with explicit options.
        /// </summary>
        /// <param name="channel">Channel name</param>
        /// <param name="symbol">Symbol to unsubscribe</param>
        /// <param name="options">The same options as the matching <c>SubscribeAsync</c> call</param>
        /// <returns>Task that completes when unsubscription is confirmed</returns>
        public Task UnsubscribeAsync(string channel, string symbol, SubscribeOptions options) =>
            UnsubscribeAsync(channel, new[] { symbol }, options);

        /// <summary>
        /// Unsubscribe from a market data channel for one or more symbols.
        /// </summary>
        /// <param name="channel">Channel name</param>
        /// <param name="symbols">Symbols to unsubscribe; at least one is required</param>
        /// <param name="options">The same options as the matching <c>SubscribeAsync</c> call</param>
        /// <returns>Task that completes when unsubscription is confirmed</returns>
        public Task UnsubscribeAsync(string channel, IEnumerable<string> symbols, SubscribeOptions? options = null)
        {
            if (symbols == null)
                throw new ArgumentNullException(nameof(symbols));
            return _inner.Unsubscribe(channel, symbols.ToArray(), ToInnerOptions(options));
        }

        /// <summary>
        /// Unsubscribe by the ids the server issued in its <c>subscribed</c> messages.
        /// A reconnect does not restore these subscriptions.
        /// </summary>
        /// <param name="ids">Server subscription ids; an empty list is error 1005</param>
        /// <returns>Task that completes when the unsubscribe is sent</returns>
        public Task UnsubscribeAsync(IEnumerable<string> ids)
        {
            if (ids == null)
                throw new ArgumentNullException(nameof(ids));
            return _inner.UnsubscribeIds(ids.ToArray());
        }

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
        /// Send a ping message to the server. Fire-and-forget: the task
        /// completes once the ping is sent, and the pong (if any) arrives
        /// later via the message callback. See <see cref="MeasureLatencyAsync"/>
        /// for an awaitable round-trip measurement.
        /// </summary>
        /// <param name="state">Optional state string echoed back in the pong response</param>
        /// <returns>Task that completes when the ping is sent</returns>
        public Task PingAsync(string? state = null) => _inner.Ping(state);

        /// <summary>
        /// Send a ping and await the matching pong, returning the round-trip
        /// time in milliseconds.
        /// </summary>
        /// <param name="timeoutMs">Timeout in milliseconds (default: 5000)</param>
        /// <returns>Task that completes with the round-trip time in milliseconds</returns>
        public Task<double> MeasureLatencyAsync(ulong? timeoutMs = null) => _inner.MeasureLatency(timeoutMs);

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
