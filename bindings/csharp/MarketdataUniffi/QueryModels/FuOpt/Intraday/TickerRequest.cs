using uniffi.marketdata_uniffi;

namespace FugleMarketData.QueryModels.FuOpt.Intraday;

/// <summary>
/// Filters for <c>futopt/intraday/ticker/{symbol}</c>, <c>quote/{symbol}</c>
/// and <c>volumes/{symbol}</c>.
/// </summary>
public class TickerVolumeRequest : BaseRequest
{
    public TickerVolumeRequest() { }
    public TickerVolumeRequest(TradeSession? session)
    {
        Session = session;
    }
    public TradeSession? Session { get; init; } = default;

    internal AfterHoursParams ToParams() => new AfterHoursParams(afterHours: SessionValue.Flag(Session));
}
