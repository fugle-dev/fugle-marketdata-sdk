using FugleMarketData.QueryModels.Stock.History;
using uniffi.marketdata_uniffi;

namespace FugleMarketData.QueryModels.Stock.Intraday;

/// <summary>Filters for <c>stock/intraday/trades/{symbol}</c>.</summary>
public sealed class TradeRequest : BaseTickerRequest
{
    public TradeRequest(TickerType? tickerType = default, int? offset = default, int? limit = default) : base(tickerType)
    {
        Offset = offset;
        Limit = limit;
    }
    /// <summary>Must not be negative (<see cref="System.ArgumentOutOfRangeException"/>).</summary>
    public int? Offset { get; init; }
    /// <summary>Must not be negative (<see cref="System.ArgumentOutOfRangeException"/>).</summary>
    public int? Limit { get; init; }
    /// <summary>Trade order. FubonNeo has no such property.</summary>
    public SortType? Sort { get; init; } = default;
    /// <summary>Include trial-match trades. FubonNeo has no such property.</summary>
    public bool? IsTrial { get; init; } = default;

    internal StockTradesParams ToParams() => new StockTradesParams(
        oddLot: OddLot,
        offset: QueryValue.Count(Offset, nameof(Offset)),
        limit: QueryValue.Count(Limit, nameof(Limit)),
        sort: QueryValue.Lower(Sort),
        isTrial: IsTrial);
}
