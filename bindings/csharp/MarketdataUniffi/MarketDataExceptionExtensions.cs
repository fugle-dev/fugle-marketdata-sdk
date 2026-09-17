// Extension methods bridging the flat generated MarketDataException hierarchy
// to the unified cross-language ErrorInfo (#81).
using System;

namespace FugleMarketData
{
    /// <summary>
    /// Extension methods for <see cref="uniffi.marketdata_uniffi.MarketDataException"/>.
    /// </summary>
    public static class MarketDataExceptionExtensions
    {
        /// <summary>
        /// The unified cross-language error info (code, source kind, message,
        /// HTTP details) carried by every generated
        /// <see cref="uniffi.marketdata_uniffi.MarketDataException"/> variant.
        /// </summary>
        /// <param name="ex">The exception to read the info from</param>
        /// <returns>The error info</returns>
        /// <exception cref="ArgumentException">
        /// If <paramref name="ex"/> is a variant this SDK version does not
        /// know about (should not happen: every generated variant carries
        /// an `info` field as of 0.2.0-rc.2)
        /// </exception>
        public static uniffi.marketdata_uniffi.ErrorInfo GetInfo(this uniffi.marketdata_uniffi.MarketDataException ex)
        {
            switch (ex)
            {
                case uniffi.marketdata_uniffi.MarketDataException.NetworkException e:
                    return e.info;
                case uniffi.marketdata_uniffi.MarketDataException.AuthException e:
                    return e.info;
                case uniffi.marketdata_uniffi.MarketDataException.RateLimitException e:
                    return e.info;
                case uniffi.marketdata_uniffi.MarketDataException.InvalidSymbol e:
                    return e.info;
                case uniffi.marketdata_uniffi.MarketDataException.ParseException e:
                    return e.info;
                case uniffi.marketdata_uniffi.MarketDataException.TimeoutException e:
                    return e.info;
                case uniffi.marketdata_uniffi.MarketDataException.WebSocketException e:
                    return e.info;
                case uniffi.marketdata_uniffi.MarketDataException.ClientClosed e:
                    return e.info;
                case uniffi.marketdata_uniffi.MarketDataException.ConfigException e:
                    return e.info;
                case uniffi.marketdata_uniffi.MarketDataException.ApiException e:
                    return e.info;
                case uniffi.marketdata_uniffi.MarketDataException.Other e:
                    return e.info;
                default:
                    throw new ArgumentException(
                        $"Unknown MarketDataException variant: {ex.GetType()}",
                        nameof(ex));
            }
        }
    }
}
