// Subscribe / unsubscribe parameter objects, in the FubonNeo shape.
using System.Collections.Generic;
using System.Linq;

namespace FugleMarketData.WebsocketModels
{
    /// <summary>
    /// Symbols of one subscribe call. <see cref="Symbol"/> and
    /// <see cref="Symbols"/> are merged into one list (Symbol first); at
    /// least one non-empty symbol is required (core error 1005).
    /// </summary>
    public abstract class BaseParams
    {
        /// <summary>One symbol, or empty when only <see cref="Symbols"/> is set.</summary>
        public string Symbol { get; set; } = "";

        /// <summary>Symbols to subscribe in one frame.</summary>
        public IEnumerable<string> Symbols { get; set; } = Enumerable.Empty<string>();

        /// <summary>
        /// <see cref="Symbol"/> (when non-empty) followed by <see cref="Symbols"/>.
        /// </summary>
        internal List<string> AllSymbols()
        {
            var list = new List<string>();
            if (!string.IsNullOrEmpty(Symbol))
                list.Add(Symbol);
            if (Symbols != null)
                list.AddRange(Symbols);
            return list;
        }
    }

    /// <summary>
    /// Stock subscribe parameters.
    /// </summary>
    public class StockSubscribeParams : BaseParams
    {
        /// <summary>True subscribes to the intraday odd-lot (盤中零股) session.</summary>
        public bool IntradayOddLot { get; set; }
    }

    /// <summary>
    /// FutOpt subscribe parameters.
    /// </summary>
    public class FutureOptionParams : BaseParams
    {
        /// <summary>True subscribes to the after-hours (盤後) session.</summary>
        public bool AfterHours { get; set; }
    }

    /// <summary>
    /// Server subscription ids to unsubscribe. <see cref="ChannelId"/> and
    /// <see cref="ChannelIds"/> are merged into one list (ChannelId first);
    /// at least one is required (core error 1005).
    /// </summary>
    public class UnsubscribeParams
    {
        /// <summary>One subscription id, or empty when only <see cref="ChannelIds"/> is set.</summary>
        public string ChannelId { get; set; } = "";

        /// <summary>Subscription ids to unsubscribe in one frame.</summary>
        public IEnumerable<string> ChannelIds { get; set; } = Enumerable.Empty<string>();

        /// <summary>
        /// <see cref="ChannelId"/> (when non-empty) followed by <see cref="ChannelIds"/>.
        /// </summary>
        internal List<string> AllIds()
        {
            var list = new List<string>();
            if (!string.IsNullOrEmpty(ChannelId))
                list.Add(ChannelId);
            if (ChannelIds != null)
                list.AddRange(ChannelIds);
            return list;
        }
    }
}
