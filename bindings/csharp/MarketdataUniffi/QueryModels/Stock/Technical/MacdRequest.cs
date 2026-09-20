using System;
using FugleMarketData.QueryModels.Stock.History;

namespace FugleMarketData.QueryModels.Stock.Technical;

/// <summary>MACD: <c>stock/technical/macd/{symbol}</c>.</summary>
public sealed class MacdRequest : TechnicalRequest
{
    public MacdRequest() { }
    public MacdRequest(int fast, int slow, int signal, DateTime? from = default, DateTime? to = default, HistoryTimeFrame? timeFrame = default)
    {
        Fast = fast;
        Slow = slow;
        Signal = signal;
        From = from;
        To = to;
        TimeFrame = timeFrame;
    }
    /// <summary>Sent as given, including the default 0 (the server answers 400 to it); negative throws.</summary>
    public int Fast { get; init; }
    /// <inheritdoc cref="Fast"/>
    public int Slow { get; init; }
    /// <inheritdoc cref="Fast"/>
    public int Signal { get; init; }

    internal uint FastArg() => QueryValue.Count(Fast, nameof(Fast));
    internal uint SlowArg() => QueryValue.Count(Slow, nameof(Slow));
    internal uint SignalArg() => QueryValue.Count(Signal, nameof(Signal));
}
