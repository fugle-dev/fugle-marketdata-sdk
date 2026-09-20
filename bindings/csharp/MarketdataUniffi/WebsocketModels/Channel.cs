// Streaming channels, in the FubonNeo shape: the enum name lower-cased is the
// channel string the wire takes.
namespace FugleMarketData.WebsocketModels
{
    /// <summary>
    /// Channels of the Stock streaming endpoint.
    /// </summary>
    public enum StockChannel
    {
        Trades,
        Candles,
        Books,
        Aggregates,
        Indices
    }

    /// <summary>
    /// Channels of the FutOpt streaming endpoint.
    /// </summary>
    public enum FutureOptionChannel
    {
        Trades,
        Books,
        Candles,
        Aggregates,
    }
}
