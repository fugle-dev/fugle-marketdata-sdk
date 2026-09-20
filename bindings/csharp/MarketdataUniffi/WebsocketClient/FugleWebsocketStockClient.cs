// Stock endpoint event-style client (#204).
using System;
using System.Threading.Tasks;
using FugleMarketData.WebsocketModels;

namespace FugleMarketData.WebsocketClient
{
    /// <summary>
    /// Event-style client of the Stock streaming endpoint. Get one from
    /// <see cref="FugleWebsocketClientFactory.Stock"/>, or build it over
    /// your own <see cref="WebSocketClientOptions"/> (reconnect, health
    /// check, message queue, …) with <see cref="WebSocketClientOptions.Endpoint"/>
    /// left at <see cref="WebSocketEndpoint.Stock"/>.
    /// </summary>
    public sealed class FugleWebsocketStockClient : FugleWebsocketClient
    {
        /// <inheritdoc cref="FugleWebsocketClient(WebSocketClientOptions, WebSocketEndpoint)"/>
        public FugleWebsocketStockClient(WebSocketClientOptions options)
            : base(options, WebSocketEndpoint.Stock)
        {
        }

        /// <summary>Subscribe one symbol.</summary>
        public Task Subscribe(StockChannel channel, string symbol) =>
            SubscribeCore(ChannelName(channel), new[] { symbol });

        /// <summary>Subscribe symbols in one frame.</summary>
        public Task Subscribe(StockChannel channel, params string[] symbols) =>
            SubscribeCore(ChannelName(channel), symbols);

        /// <summary>
        /// Subscribe <see cref="BaseParams.Symbol"/> and <see cref="BaseParams.Symbols"/>
        /// in one frame; <see cref="StockSubscribeParams.IntradayOddLot"/> selects the odd-lot session.
        /// </summary>
        public Task Subscribe(StockChannel channel, StockSubscribeParams param)
        {
            if (param == null)
                throw new ArgumentNullException(nameof(param));
            var options = param.IntradayOddLot ? new SubscribeOptions { IntradayOddLot = true } : null;
            return SubscribeCore(ChannelName(channel), param.AllSymbols(), options);
        }
    }
}
