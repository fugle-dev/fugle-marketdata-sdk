using FugleMarketData.QueryModels.Stock.History;
using uniffi.marketdata_uniffi;

namespace FugleMarketData.QueryModels.Stock.Intraday;

/// <summary>Filters for <c>stock/intraday/candles/{symbol}</c>.</summary>
public sealed class IntradayCandlesRequest : BaseTickerRequest
{
    public IntradayCandlesRequest(TickerType? tickerType = default, IntradayTimeFrame? timeFrame = default) : base(tickerType)
    {
        TimeFrame = timeFrame;
    }

    /// <summary>Unset takes the server default.</summary>
    public IntradayTimeFrame? TimeFrame { get; init; } = default;
    /// <summary>Candle order. FubonNeo has no such property.</summary>
    public SortType? Sort { get; init; } = default;

    internal StockCandlesParams ToParams() => new StockCandlesParams(
        timeframe: QueryValue.Minutes(TimeFrame),
        oddLot: OddLot,
        sort: QueryValue.Lower(Sort));
}

public enum IntradayTimeFrame
{
    OneMin = 1,
    FiveMin = 5,
    TenMin = 10,
    FifteenMin = 15,
    ThirtyMin = 30,
    SixtyMin = 60,
}
