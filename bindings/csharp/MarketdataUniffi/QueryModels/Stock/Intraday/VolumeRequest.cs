using uniffi.marketdata_uniffi;

namespace FugleMarketData.QueryModels.Stock.Intraday;

/// <summary>Filters for <c>stock/intraday/volumes/{symbol}</c>.</summary>
public sealed class VolumeRequest : BaseTickerRequest
{
    public VolumeRequest(TickerType? tickerType = default) : base(tickerType) { }

    internal OddLotParams ToParams() => new OddLotParams(oddLot: OddLot);
}
