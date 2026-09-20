using System;
using FugleMarketData.QueryModels.Stock.History;

namespace FugleMarketData.QueryModels.Stock.Technical;

/// <summary>Relative strength index: <c>stock/technical/rsi/{symbol}</c>.</summary>
public sealed class RsiRequest : TechnicalRequest
{
    public RsiRequest() { }
    public RsiRequest(int period, DateTime? from = default, DateTime? to = default, HistoryTimeFrame? timeFrame = default)
    {
        Period = period;
        From = from;
        To = to;
        TimeFrame = timeFrame;
    }
    /// <summary>Sent as given, including the default 0 (the server answers 400 to it); negative throws.</summary>
    public int Period { get; init; }

    internal uint PeriodArg() => QueryValue.Count(Period, nameof(Period));
}
