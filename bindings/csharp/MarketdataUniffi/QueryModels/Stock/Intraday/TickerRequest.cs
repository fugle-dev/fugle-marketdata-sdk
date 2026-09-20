using uniffi.marketdata_uniffi;

namespace FugleMarketData.QueryModels.Stock.Intraday;

/// <summary>Filters for <c>stock/intraday/ticker/{symbol}</c>.</summary>
public sealed class TickerRequest : BaseTickerRequest
{
    public TickerRequest(TickerType? tickerType = default) : base(tickerType) { }

    internal OddLotParams ToParams() => new OddLotParams(oddLot: OddLot);
}
