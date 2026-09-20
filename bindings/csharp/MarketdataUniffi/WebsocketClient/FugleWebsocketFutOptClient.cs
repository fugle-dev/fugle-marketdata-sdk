// FutOpt endpoint event-style client (#204).
using System;
using System.Threading.Tasks;
using FugleMarketData.WebsocketModels;

namespace FugleMarketData.WebsocketClient
{
    /// <summary>
    /// Event-style client of the FutOpt streaming endpoint. Get one from
    /// <see cref="FugleWebsocketClientFactory.FutureOption"/>, or build it
    /// over your own <see cref="WebSocketClientOptions"/> with
    /// <see cref="WebSocketClientOptions.Endpoint"/> set to
    /// <see cref="WebSocketEndpoint.FutOpt"/>.
    /// </summary>
    public sealed class FugleWebsocketFutOptClient : FugleWebsocketClient
    {
        /// <inheritdoc cref="FugleWebsocketClient(WebSocketClientOptions, WebSocketEndpoint)"/>
        public FugleWebsocketFutOptClient(WebSocketClientOptions options)
            : base(options, WebSocketEndpoint.FutOpt)
        {
        }

        /// <summary>Subscribe one symbol.</summary>
        public Task Subscribe(FutureOptionChannel channel, string symbol) =>
            SubscribeCore(ChannelName(channel), new[] { symbol });

        /// <summary>Subscribe symbols in one frame.</summary>
        public Task Subscribe(FutureOptionChannel channel, params string[] symbols) =>
            SubscribeCore(ChannelName(channel), symbols);

        /// <summary>
        /// Subscribe <see cref="BaseParams.Symbol"/> and <see cref="BaseParams.Symbols"/>
        /// in one frame; <see cref="FutureOptionParams.AfterHours"/> selects the after-hours session.
        /// </summary>
        public Task Subscribe(FutureOptionChannel channel, FutureOptionParams param)
        {
            if (param == null)
                throw new ArgumentNullException(nameof(param));
            var options = param.AfterHours ? new SubscribeOptions { AfterHours = true } : null;
            return SubscribeCore(ChannelName(channel), param.AllSymbols(), options);
        }
    }
}
