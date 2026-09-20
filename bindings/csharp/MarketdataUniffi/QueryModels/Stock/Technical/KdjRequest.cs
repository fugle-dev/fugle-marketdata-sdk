using System;
using FugleMarketData.QueryModels.Stock.History;

namespace FugleMarketData.QueryModels.Stock.Technical;

/// <summary>Stochastic oscillator: <c>stock/technical/kdj/{symbol}</c>.</summary>
public sealed class KdjRequest : TechnicalRequest
{
    public KdjRequest() { }
    public KdjRequest(int rPeriod, int kPeriod, int dPeriod, DateTime? from = default, DateTime? to = default, HistoryTimeFrame? timeFrame = default)
    {
        RPeriod = rPeriod;
        KPeriod = kPeriod;
        DPeriod = dPeriod;
        From = from;
        To = to;
        TimeFrame = timeFrame;
    }
    /// <summary>Sent as given, including the default 0 (the server answers 400 to it); negative throws.</summary>
    public int RPeriod { get; init; }
    /// <inheritdoc cref="RPeriod"/>
    public int KPeriod { get; init; }
    /// <inheritdoc cref="RPeriod"/>
    public int DPeriod { get; init; }

    internal uint RPeriodArg() => QueryValue.Count(RPeriod, nameof(RPeriod));
    internal uint KPeriodArg() => QueryValue.Count(KPeriod, nameof(KPeriod));
    internal uint DPeriodArg() => QueryValue.Count(DPeriod, nameof(DPeriod));
}
