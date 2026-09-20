// Options classes for configuring FugleMarketData.WebSocketClient
using System;
using FugleMarketData.WebsocketClient;

namespace FugleMarketData
{
    /// <summary>
    /// Reconnection configuration for WebSocket clients.
    /// Controls automatic reconnection behavior on connection loss.
    /// </summary>
    public class ReconnectOptions
    {
        /// <summary>
        /// Whether auto-reconnect is enabled (default: true). Set false to turn it off.
        /// </summary>
        public bool? Enabled { get; set; }

        /// <summary>
        /// Maximum reconnection attempts; 0 means unlimited (default: 0).
        /// With a limit set, the client stops attempting to reconnect after reaching it.
        /// </summary>
        public uint? MaxAttempts { get; set; }

        /// <summary>
        /// Initial reconnection delay in milliseconds (default: 1000, min: 100).
        /// The delay increases exponentially with each retry up to MaxDelayMs.
        /// </summary>
        public ulong? InitialDelayMs { get; set; }

        /// <summary>
        /// Maximum reconnection delay in milliseconds (default: 60000).
        /// The exponential backoff will not exceed this value.
        /// </summary>
        public ulong? MaxDelayMs { get; set; }
    }

    /// <summary>
    /// Liveness detection for WebSocket connections.
    /// The connection is declared dead when no inbound frame (data, heartbeat
    /// or pong) arrives within <see cref="HeartbeatTimeoutMs"/>. The server
    /// sends a heartbeat every 30 seconds.
    /// </summary>
    public class HealthCheckOptions
    {
        /// <summary>
        /// Whether liveness detection is enabled (default: true).
        /// </summary>
        public bool? Enabled { get; set; }

        /// <summary>
        /// Maximum gap between inbound frames in milliseconds before the
        /// connection is declared dead (default: 35000, min: 5000). Does not
        /// apply when <see cref="ProbeEnabled"/> is true.
        /// </summary>
        public ulong? HeartbeatTimeoutMs { get; set; }

        /// <summary>
        /// Confirm a silent connection with a ping before declaring it dead
        /// (default: false). After <see cref="IdleProbeAfterMs"/> of silence
        /// one ping is sent; if nothing arrives within
        /// <see cref="ProbeTimeoutMs"/> the connection is declared dead.
        /// </summary>
        public bool? ProbeEnabled { get; set; }

        /// <summary>
        /// Silence before the probe, in milliseconds (default: 30000, the
        /// server's heartbeat period; min: 5000). Only used when
        /// <see cref="ProbeEnabled"/> is true.
        /// </summary>
        public ulong? IdleProbeAfterMs { get; set; }

        /// <summary>
        /// Wait for any inbound frame after the probe, in milliseconds
        /// (default: 5000, min: 1000). Only used when
        /// <see cref="ProbeEnabled"/> is true.
        /// </summary>
        public ulong? ProbeTimeoutMs { get; set; }
    }

    /// <summary>
    /// What the client does with an inbound market data message while
    /// <see cref="WebSocketClientOptions.MessageBuffer"/> unread messages are
    /// already queued (i.e. <see cref="IWebSocketListener.OnMessage"/> is
    /// falling behind).
    /// </summary>
    public enum MessageOverflow
    {
        /// <summary>
        /// Drop new messages and report the dropped count through
        /// <see cref="IWebSocketListener.OnMessagesDropped"/> (default).
        /// </summary>
        DropNewest,

        /// <summary>
        /// Never drop: the queue keeps growing while <see cref="IWebSocketListener.OnMessage"/> lags.
        /// </summary>
        Unbounded,
    }

    /// <summary>
    /// Configuration options for constructing a WebSocketClient.
    /// Exactly one non-empty authentication method must be provided; an empty
    /// or whitespace-only value counts as not provided.
    /// </summary>
    public class WebSocketClientOptions
    {
        /// <summary>
        /// API key authentication (optional).
        /// Provide exactly one of: ApiKey, BearerToken, or SdkToken.
        /// </summary>
        public string? ApiKey { get; set; }

        /// <summary>
        /// Bearer token authentication (optional).
        /// Provide exactly one of: ApiKey, BearerToken, or SdkToken.
        /// </summary>
        public string? BearerToken { get; set; }

        /// <summary>
        /// SDK token authentication (optional).
        /// Provide exactly one of: ApiKey, BearerToken, or SdkToken.
        /// </summary>
        public string? SdkToken { get; set; }

        /// <summary>
        /// Custom base URL for WebSocket endpoints (optional).
        /// If not provided, uses the default Fugle MarketData WebSocket URL.
        /// </summary>
        public string? BaseUrl { get; set; }

        /// <summary>
        /// Reconnection configuration (optional).
        /// If not provided, the client auto-reconnects with the defaults (unlimited
        /// attempts, 1s initial delay, 60s maximum delay). Set
        /// <see cref="ReconnectOptions.Enabled"/> to false to turn it off.
        /// </summary>
        public ReconnectOptions? Reconnect { get; set; }

        /// <summary>
        /// Health check configuration (optional).
        /// If not provided, health checks are enabled with the defaults.
        /// </summary>
        public HealthCheckOptions? HealthCheck { get; set; }

        /// <summary>
        /// WebSocket endpoint type (default: Stock).
        /// Determines which market data stream to connect to.
        /// </summary>
        public WebSocketEndpoint Endpoint { get; set; } = WebSocketEndpoint.Stock;

        /// <summary>
        /// What to do with inbound messages while <see cref="MessageBuffer"/>
        /// unread messages are already queued (optional, default: <see cref="MessageOverflow.DropNewest"/>).
        /// </summary>
        public MessageOverflow? MessageOverflow { get; set; }

        /// <summary>
        /// Number of unread messages the client holds before applying
        /// <see cref="MessageOverflow"/> (optional; null uses the default of 4096).
        /// Must be greater than 0 when set.
        /// </summary>
        public int? MessageBuffer { get; set; }

        /// <summary>
        /// Streaming protocol version per endpoint (optional). If not
        /// provided, the server picks the latest version for both Stock and
        /// FutOpt.
        /// </summary>
        public WebsocketVersionOptions? Versions { get; set; }

        /// <summary>
        /// How long the auth handshake may take once the WebSocket is open, in
        /// milliseconds: from the auth frame being sent until the server's
        /// verdict (optional; null uses the default of 10000). Applies to the
        /// first connect and to every reconnect; elapsing it fails the attempt
        /// with a timeout error (code 3001). Must be greater than 0 when set.
        /// The server itself allows 60 seconds.
        /// </summary>
        public ulong? AuthTimeoutMs { get; set; }
    }
}
