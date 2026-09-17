// Options classes for configuring FugleMarketData.WebSocketClient
using System;

namespace FugleMarketData
{
    /// <summary>
    /// Reconnection configuration for WebSocket clients.
    /// Controls automatic reconnection behavior on connection loss.
    /// </summary>
    public class ReconnectOptions
    {
        /// <summary>
        /// Maximum reconnection attempts (default: 5, min: 1).
        /// After reaching this limit, the client stops attempting to reconnect.
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
        /// connection is declared dead (default: 35000, min: 5000).
        /// </summary>
        public ulong? HeartbeatTimeoutMs { get; set; }
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
        /// If not provided, uses default reconnection settings (max 5 attempts, 1s initial delay).
        /// </summary>
        public ReconnectOptions? Reconnect { get; set; }

        /// <summary>
        /// Health check configuration (optional).
        /// If not provided, health checks are disabled by default.
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
    }
}
