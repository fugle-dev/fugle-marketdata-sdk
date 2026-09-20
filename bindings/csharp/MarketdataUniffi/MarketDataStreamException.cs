// The exception the event-style WebSocket clients hand to OnException:
// core's ErrorInfo wrapped in a .NET exception.
using System;
using System.Collections.Generic;

namespace FugleMarketData
{
    /// <summary>
    /// A streaming error reported through
    /// <see cref="WebsocketClient.FugleWebsocketClient.OnException"/>.
    /// Wraps the unified <see cref="uniffi.marketdata_uniffi.ErrorInfo"/>
    /// (code, source kind, message); <see cref="Exception.Message"/> is
    /// <c>Info.message</c>.
    /// </summary>
    public sealed class MarketDataStreamException : Exception
    {
        /// <summary>The unified error info (code, source kind, message).</summary>
        public uniffi.marketdata_uniffi.ErrorInfo Info { get; }

        /// <summary>Wrap an error info.</summary>
        public MarketDataStreamException(uniffi.marketdata_uniffi.ErrorInfo info)
            : base((info ?? throw new ArgumentNullException(nameof(info))).message)
        {
            Info = info;
        }

        /// <summary>
        /// The server rejected the credentials: code 2002 (AUTH), message
        /// "Authenticate Failed!" as FubonNeo phrased it.
        /// </summary>
        internal static MarketDataStreamException AuthenticateFailed() =>
            new MarketDataStreamException(new uniffi.marketdata_uniffi.ErrorInfo(
                code: 2002,
                sourceKind: uniffi.marketdata_uniffi.ErrorSourceKind.Auth,
                message: "Authenticate Failed!",
                status: null,
                body: null,
                requestId: null,
                headers: new Dictionary<string, string>()
            ));
    }
}
