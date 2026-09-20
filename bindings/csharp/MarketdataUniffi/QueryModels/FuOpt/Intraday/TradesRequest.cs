using uniffi.marketdata_uniffi;

namespace FugleMarketData.QueryModels.FuOpt.Intraday;

/// <summary>Filters for <c>futopt/intraday/trades/{symbol}</c>.</summary>
public class TradesRequest : BaseRequest
{
    public TradesRequest() { }
    public TradesRequest(TradeSession? session = default, int? offset = default, int? limit = default)
    {
        Session = session;
        Offset = offset;
        Limit = limit;
    }
    public TradeSession? Session { get; init; } = default;
    /// <summary>Must not be negative (<see cref="System.ArgumentOutOfRangeException"/>).</summary>
    public int? Offset { get; init; } = default;
    /// <summary>Must not be negative (<see cref="System.ArgumentOutOfRangeException"/>).</summary>
    public int? Limit { get; init; } = default;
    public bool? IsTrial { get; init; } = default;

    internal FutOptTradesParams ToParams() => new FutOptTradesParams(
        afterHours: SessionValue.Flag(Session),
        offset: QueryValue.Count(Offset, nameof(Offset)),
        limit: QueryValue.Count(Limit, nameof(Limit)),
        isTrial: IsTrial);
}
