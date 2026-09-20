namespace FugleMarketData.QueryModels.FuOpt;

/// <summary>The positional <c>type</c> of the futopt list endpoints.</summary>
public enum FutOptType
{
    Future,
    Option
}

/// <summary>Sent upper-cased: <c>TAIFEX</c>.</summary>
public enum FutOptExchangeType
{
    TaiFex
}

/// <summary>
/// Session of the list endpoints (<c>products</c>, <c>tickers</c>).
/// <see cref="AfterHours"/> is sent as <c>session=AFTERHOURS</c>;
/// <see cref="Regular"/> is the server default and is not sent (FubonNeo
/// sent <c>session=REGULAR</c>, which the server treats the same).
/// </summary>
public enum SessionType
{
    Regular,
    AfterHours
}

public enum ContractType
{
    I,
    R,
    B,
    C,
    S,
    E
}

/// <summary>
/// Session of the single-contract and historical endpoints; sent as
/// <c>session=afterhours</c>.
/// </summary>
public enum TradeSession
{
    AfterHours
}

internal static class SessionValue
{
    internal static bool? Flag(SessionType? value) =>
        value == SessionType.AfterHours ? true : (bool?)null;

    internal static bool? Flag(TradeSession? value) =>
        value.HasValue ? true : (bool?)null;
}
