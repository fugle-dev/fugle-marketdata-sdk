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
        /// Note: BaseUrl override is not yet fully implemented in the underlying library.
        /// This property is stored for future use.
        /// </remarks>
        public string? BaseUrl { get; set; }
    }
}
