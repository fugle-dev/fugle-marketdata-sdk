// Event-style WebSocket client in the FubonNeo shape (#204): Action<string>
// events over a FugleMarketData.WebSocketClient. The listener interface and
// WebSocketClient are unchanged; this is composition on top of them.
using System;
using System.Collections.Generic;
using System.Runtime.ExceptionServices;
using System.Threading;
using System.Threading.Tasks;
using FugleMarketData.WebsocketModels;

namespace FugleMarketData.WebsocketClient
{
    /// <summary>
    /// Base of <see cref="FugleWebsocketStockClient"/> and
    /// <see cref="FugleWebsocketFutOptClient"/>: the FubonNeo
    /// <c>FugleWebsocketClient</c> surface (string events, <c>Connect</c> /
    /// <c>Disconnect</c> / <c>Ping</c> / <c>Unsubscribe</c>) over a
    /// <see cref="WebSocketClient"/>, which <see cref="Inner"/> exposes for
    /// anything the events do not cover.
    ///
    /// Every event is a settable property with a no-op default, so both
    /// <c>client.OnMessage = Handle;</c> and <c>client.OnMessage += Handle;</c>
    /// work (set them before <see cref="Connect"/>; <c>+=</c> is not atomic).
    /// Handlers run on the SDK's callback thread; one that throws does
    /// not stop the stream — the failure comes back through
    /// <see cref="OnException"/> as code 3004 (throttled, see
    /// <see cref="WebSocketListenerAdapter"/>).
    ///
    /// Event mapping from <see cref="IWebSocketListener"/>:
    /// <list type="bullet">
    /// <item><c>OnConnected()</c> → <see cref="OnConnected"/>("Connected"), again after every reconnect</item>
    /// <item><c>OnAuthenticated</c> → nothing</item>
    /// <item><c>OnMessage(msg)</c> → <see cref="OnMessage"/>(msg.raw); an <c>error</c> frame first raises <see cref="OnError"/>(msg.raw)</item>
    /// <item><c>OnError(info)</c> → <see cref="OnException"/>(<see cref="MarketDataStreamException"/>)</item>
    /// <item><c>OnUnauthenticated(json)</c> → <see cref="OnError"/>(json ?? "{}") then <see cref="OnException"/>("Authenticate Failed!")</item>
    /// <item><c>OnDisconnected</c> after <see cref="Disconnect"/>(msg) → <see cref="OnDisconnected"/>(msg);
    /// otherwise <see cref="OnClose"/>("Received close message") then <see cref="OnDisconnected"/>("Server Disconnected")</item>
    /// <item><c>OnReconnecting</c> / <c>OnReconnectFailed</c> / <c>OnMessagesDropped</c> → same name</item>
    /// </list>
    /// </summary>
    public abstract class FugleWebsocketClient : IDisposable
    {
        private string? _disconnectMessage;
        private bool _disposed;

        /// <summary>Every frame received, verbatim.</summary>
        public Action<string> OnMessage { get; set; } = _ => { };

        /// <summary>An SDK error, as a <see cref="MarketDataStreamException"/>.</summary>
        public Action<Exception> OnException { get; set; } = _ => { };

        /// <summary>An <c>error</c> frame from the server, verbatim (also delivered to <see cref="OnMessage"/>).</summary>
        public Action<string> OnError { get; set; } = _ => { };

        /// <summary>"Connected": transport up, before auth. Fires again after every reconnect.</summary>
        public Action<string> OnConnected { get; set; } = _ => { };

        /// <summary>The <see cref="Disconnect"/> message, or "Server Disconnected" when the server closed the connection.</summary>
        public Action<string> OnDisconnected { get; set; } = _ => { };

        /// <summary>"Received close message": the server closed the connection; <see cref="OnDisconnected"/> follows.</summary>
        public Action<string> OnClose { get; set; } = _ => { };

        /// <summary>A reconnection attempt starts (1-based attempt number).</summary>
        public Action<uint> OnReconnecting { get; set; } = _ => { };

        /// <summary>Reconnection gave up after this many attempts.</summary>
        public Action<uint> OnReconnectFailed { get; set; } = _ => { };

        /// <summary>Messages dropped because <see cref="OnMessage"/> fell behind (see <see cref="IWebSocketListener.OnMessagesDropped"/>).</summary>
        public Action<ulong> OnMessagesDropped { get; set; } = _ => { };

        /// <summary>The underlying client, for what the event surface does not cover.</summary>
        public WebSocketClient Inner { get; }

        /// <summary>Whether the client is currently connected to the server.</summary>
        public bool IsConnected => Inner.IsConnected;

        /// <summary>
        /// Build the client over <paramref name="options"/>, whose
        /// <see cref="WebSocketClientOptions.Endpoint"/> must be
        /// <paramref name="endpoint"/>.
        /// </summary>
        /// <exception cref="ArgumentNullException">If options is null</exception>
        /// <exception cref="ArgumentException">If options.Endpoint is not <paramref name="endpoint"/></exception>
        /// <exception cref="uniffi.marketdata_uniffi.MarketDataException">Code 1004 if the credentials are invalid</exception>
        protected FugleWebsocketClient(WebSocketClientOptions options, WebSocketEndpoint endpoint)
        {
            if (options == null)
                throw new ArgumentNullException(nameof(options));
            if (options.Endpoint != endpoint)
                throw new ArgumentException(
                    $"options.Endpoint must be WebSocketEndpoint.{endpoint} for {GetType().Name}, got {options.Endpoint}",
                    nameof(options));
            Inner = new WebSocketClient(options, new EventListener(this));
        }

        /// <summary>
        /// Connect and authenticate. Await a preceding <see cref="Disconnect"/>
        /// first: a message it left behind is discarded here, so a
        /// disconnect event still in flight would read as a server-side close.
        /// </summary>
        public Task Connect()
        {
            // A message left by a Disconnect that produced no event must not
            // label a later server-side close.
            Interlocked.Exchange(ref _disconnectMessage, null);
            return Inner.ConnectAsync();
        }

        /// <summary>
        /// Disconnect; the connection's <see cref="OnDisconnected"/> receives
        /// <paramref name="msg"/>. Unlike FubonNeo there is no event when
        /// there is no connection to close (never connected, already
        /// disconnected, or reconnecting): the event follows core's
        /// disconnect, it is not raised by this method.
        /// </summary>
        public Task Disconnect(string msg = "Disconnect")
        {
            Interlocked.Exchange(ref _disconnectMessage, msg ?? "Disconnect");
            return Inner.DisconnectAsync();
        }

        /// <summary>Send a ping; the pong arrives through <see cref="OnMessage"/>.</summary>
        public Task Ping(string pingMsg = "ping") => Inner.PingAsync(pingMsg);

        /// <summary>Unsubscribe by the id the server issued in its <c>subscribed</c> message.</summary>
        public Task Unsubscribe(string channelId) => Inner.UnsubscribeAsync(new[] { channelId });

        /// <summary>Unsubscribe by server subscription ids; an empty list is error 1005.</summary>
        public Task Unsubscribe(params string[] channelIds) => Inner.UnsubscribeAsync(channelIds);

        /// <summary>Unsubscribe <see cref="UnsubscribeParams.ChannelId"/> and <see cref="UnsubscribeParams.ChannelIds"/> in one frame; none is error 1005.</summary>
        public Task Unsubscribe(UnsubscribeParams param)
        {
            if (param == null)
                throw new ArgumentNullException(nameof(param));
            return Inner.UnsubscribeAsync(param.AllIds());
        }

        /// <summary>The wire name of a channel enum value: its name lower-cased.</summary>
        protected static string ChannelName<TEnum>(TEnum channel) where TEnum : struct, Enum =>
            channel.ToString().ToLowerInvariant();

        /// <summary>Subscribe <paramref name="symbols"/> on <paramref name="channel"/> in one frame.</summary>
        protected Task SubscribeCore(string channel, IEnumerable<string> symbols, SubscribeOptions? options = null) =>
            Inner.SubscribeAsync(channel, symbols, options);

        /// <inheritdoc/>
        public void Dispose()
        {
            if (_disposed) return;
            Inner.Dispose();
            _disposed = true;
        }

        private static void Raise<T>(Action<T>? handler, T arg) => handler?.Invoke(arg);

        /// <summary>
        /// Raise two events from one callback so that the first handler
        /// throwing does not swallow the second event; the first exception
        /// is rethrown afterwards for the adapter to report (code 3004).
        /// </summary>
        private static void RaiseBoth(Action first, Action second)
        {
            Exception? pending = null;
            try { first(); } catch (Exception ex) { pending = ex; }
            try { second(); } catch (Exception ex) { pending ??= ex; }
            if (pending != null)
                ExceptionDispatchInfo.Capture(pending).Throw();
        }

        /// <summary>
        /// Maps <see cref="IWebSocketListener"/> onto the owner's events. Sits
        /// behind <see cref="WebSocketListenerAdapter"/>, so a throwing
        /// handler is caught there and reported back through
        /// <see cref="OnError(uniffi.marketdata_uniffi.ErrorInfo)"/>.
        /// </summary>
        internal sealed class EventListener : IWebSocketListener
        {
            private readonly FugleWebsocketClient _owner;

            public EventListener(FugleWebsocketClient owner)
            {
                _owner = owner;
            }

            public void OnConnected() => Raise(_owner.OnConnected, "Connected");

            public void OnAuthenticated(string? dataJson) { }

            public void OnUnauthenticated(string? dataJson) => RaiseBoth(
                () => Raise(_owner.OnError, dataJson ?? "{}"),
                () => Raise(_owner.OnException, MarketDataStreamException.AuthenticateFailed()));

            public void OnDisconnected(bool willReconnect)
            {
                var message = Interlocked.Exchange(ref _owner._disconnectMessage, null);
                if (message != null)
                {
                    Raise(_owner.OnDisconnected, message);
                    return;
                }
                RaiseBoth(
                    () => Raise(_owner.OnClose, "Received close message"),
                    () => Raise(_owner.OnDisconnected, "Server Disconnected"));
            }

            public void OnMessage(uniffi.marketdata_uniffi.StreamMessage message)
            {
                if (message.@event != "error")
                {
                    Raise(_owner.OnMessage, message.raw);
                    return;
                }
                RaiseBoth(
                    () => Raise(_owner.OnError, message.raw),
                    () => Raise(_owner.OnMessage, message.raw));
            }

            public void OnError(uniffi.marketdata_uniffi.ErrorInfo error) =>
                Raise(_owner.OnException, new MarketDataStreamException(error));

            public void OnReconnecting(uint attempt) => Raise(_owner.OnReconnecting, attempt);

            public void OnReconnectFailed(uint attempts) => Raise(_owner.OnReconnectFailed, attempts);

            public void OnMessagesDropped(ulong count) => Raise(_owner.OnMessagesDropped, count);
        }
    }
}
