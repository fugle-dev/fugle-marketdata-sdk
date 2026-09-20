// FubonNeo-compatible request models (#203). The class, enum and property
// names below are those of FubonNeo 2.3.0's built-in `FugleMarketData`
// client so that code written against it compiles against this SDK with its
// `using` lines unchanged. Each request converts itself into the uniffi params
// record the endpoint takes (`ToParams()`); the wire keys and flag literals
// come from core's parameter table, never from here.
using System;

namespace FugleMarketData.QueryModels;

/// <summary>
/// Base of every request model, as in FubonNeo. It carries no behaviour
/// here: FubonNeo's <c>ToQueryString()</c> is not reproduced — the query is
/// built by the core from the uniffi record each request converts into.
/// </summary>
public abstract class BaseRequest
{
}

/// <summary>
/// Base of the single-symbol stock intraday requests: an optional
/// <see cref="TickerType.OddLot"/> that asks for the odd-lot session
/// (<c>type=oddlot</c>).
/// </summary>
public abstract class BaseTickerRequest : BaseRequest
{
    protected BaseTickerRequest(TickerType? tickerType = default)
    {
        Type = tickerType;
    }

    /// <summary>Set to <see cref="TickerType.OddLot"/> for the odd-lot session.</summary>
    public TickerType? Type { get; set; }

    /// <summary><c>true</c> when the odd-lot session is asked for, else unset.</summary>
    internal bool? OddLot => Type.HasValue ? true : (bool?)null;
}

public enum TickerType
{
    OddLot
}

public enum TradeType
{
    Volume,
    Value
}

public enum DirectionType
{
    Up,
    Down
}

public enum ChangeType
{
    Percent,
    Value
}

public enum MarketType
{
    TSE,
    OTC,
    ESB,
    TIB,
    PSB
}

[Flags]
public enum FieldsType
{
    Open = 1,
    High = 2,
    Low = 4,
    Close = 8,
    Volume = 16,
    Turnover = 32,
    Change = 64,
    // average is only returned for intraday timeframes; the server ignores it elsewhere.
    Average = 128,
    All = Open | High | Low | Close | Volume | Turnover | Change | Average
}
