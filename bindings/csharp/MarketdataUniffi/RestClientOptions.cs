// Options class for configuring FugleMarketData.RestClient
using System;

namespace FugleMarketData
{
    /// <summary>
    /// Configuration options for constructing a RestClient.
    /// Exactly one authentication method must be provided.
    /// </summary>
    public class RestClientOptions
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
        /// Custom base URL for API endpoints (optional).
        /// If not provided, uses the default Fugle MarketData API URL.
        /// </summary>
        /// <remarks>
        /// Pass the host and path prefix only — a version segment such as
        /// "/v1.0" is rejected.
        /// </remarks>
        public string? BaseUrl { get; set; }
    }
}
