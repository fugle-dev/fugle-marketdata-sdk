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
    /// Health check configuration for WebSocket connections.
    /// Monitors connection health via periodic ping/pong messages.
    /// </summary>
    public class HealthCheckOptions
    {
        /// <summary>
        /// Whether health check is enabled (default: false).
        /// When enabled, the client sends periodic ping messages to verify connection health.
        /// </summary>
        public bool? Enabled { get; set; }

        /// <summary>
        /// Interval between ping messages in milliseconds (default: 30000, min: 5000).
        /// Lower values provide faster detection of connection issues but increase overhead.
        /// </summary>
        public ulong? IntervalMs { get; set; }

        /// <summary>
        /// Maximum missed pongs before disconnect (default: 2, min: 1).
        /// If the server fails to respond to this many consecutive pings, the connection is closed.
        /// </summary>
        public ulong? MaxMissedPongs { get; set; }
    }

    /// <summary>
    /// Configuration options for constructing a WebSocketClient.
    /// Exactly one authentication method must be provided.
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
    }
}
