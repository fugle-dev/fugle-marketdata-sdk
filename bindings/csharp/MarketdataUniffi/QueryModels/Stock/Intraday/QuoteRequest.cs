using uniffi.marketdata_uniffi;

namespace FugleMarketData.QueryModels.Stock.Intraday;

/// <summary>Filters for <c>stock/intraday/quote/{symbol}</c>.</summary>
public sealed class QuoteRequest : BaseTickerRequest
{
    public QuoteRequest(TickerType? tickerType = default) : base(tickerType) { }

    internal OddLotParams ToParams() => new OddLotParams(oddLot: OddLot);
}
