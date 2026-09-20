// Streaming protocol version selection for FugleMarketData.WebSocketClient
namespace FugleMarketData.WebsocketClient
{
    /// <summary>
    /// Streaming protocol version to request per endpoint. Maps to
    /// <see cref="uniffi.marketdata_uniffi.StreamingVersionRecord"/>; unset
    /// fields let the server pick the latest version.
    /// </summary>
    public class WebsocketVersionOptions
    {
        /// <summary>
        /// Stock streaming version. Only "v1.0" is served. Unset means latest.
        /// </summary>
        public string? Stock { get; set; }

        /// <summary>
        /// FutOpt streaming version: "v1.0" or "v1.1". Unset means latest (v1.1).
        ///
        /// v1.1 adds trial-matching (試撮) frames on trades / books — check the
        /// frame's <c>isTrial</c> before acting on a price.
        /// </summary>
        public string? FutOpt { get; set; }
    }
}
