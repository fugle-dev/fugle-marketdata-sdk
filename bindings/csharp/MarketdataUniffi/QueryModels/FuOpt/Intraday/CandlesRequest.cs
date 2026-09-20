using uniffi.marketdata_uniffi;

namespace FugleMarketData.QueryModels.FuOpt.Intraday;

/// <summary>Filters for <c>futopt/intraday/candles/{symbol}</c>.</summary>
public class CandlesRequest : BaseRequest
{
    public CandlesRequest() { }
    public CandlesRequest(TradeSession? session = default, CandlesTimeFrame? timeFrame = default)
    {
        Session = session;
        TimeFrame = timeFrame;
    }
    public TradeSession? Session { get; init; } = default;
    public CandlesTimeFrame? TimeFrame { get; init; } = default;

    internal FutOptCandlesParams ToParams() => new FutOptCandlesParams(
        afterHours: SessionValue.Flag(Session),
        timeframe: QueryValue.Minutes(TimeFrame));
}

public enum CandlesTimeFrame
{
    OneMin = 1,
    FiveMin = 5,
    TenMin = 10,
    FifteenMin = 15,
    ThirtyMin = 30,
    SixtyMin = 60
}
