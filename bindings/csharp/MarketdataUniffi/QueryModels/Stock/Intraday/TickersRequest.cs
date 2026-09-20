using uniffi.marketdata_uniffi;

namespace FugleMarketData.QueryModels.Stock.Intraday;

/// <summary>Filters for <c>stock/intraday/tickers</c>.</summary>
public sealed class TickersRequest : BaseRequest
{
    public TickersRequest() { }
    public TickersRequest(MarketType marketType, ExchangeType exchangeType, string industry, bool isNormal, bool isAttention, bool isDisposition, bool isHalted)
    {
        Market = marketType;
        Exchange = exchangeType;
        Industry = industry;
        IsNormal = isNormal;
        IsAttention = isAttention;
        IsDisposition = isDisposition;
        IsHalted = isHalted;
    }
    public MarketType? Market { get; set; } = default;
    public ExchangeType? Exchange { get; set; } = default;
    /// <summary>Industry code; empty means not sent.</summary>
    public string Industry { get; set; } = "";
    public bool? IsNormal { get; set; } = default;
    public bool? IsAttention { get; set; } = default;
    public bool? IsDisposition { get; set; } = default;
    public bool? IsHalted { get; set; } = default;
    /// <summary>Restrict to one symbol. FubonNeo has no such property.</summary>
    public string? Symbol { get; set; } = default;

    internal StockTickersParams ToParams()
    {
        // FubonNeo sends isAttention=false and isDisposition=false whenever
        // isNormal is true (it also overwrote the properties; this does not).
        var normal = IsNormal == true;
        return new StockTickersParams(
            exchange: QueryValue.Name(Exchange),
            market: QueryValue.Name(Market),
            industry: QueryValue.NonEmpty(Industry),
            isNormal: IsNormal,
            isAttention: normal ? false : IsAttention,
            isDisposition: normal ? false : IsDisposition,
            isHalted: IsHalted,
            symbol: QueryValue.NonEmpty(Symbol));
    }
}

/// <summary>Sent as-is: <c>TWSE</c> / <c>TPEx</c>.</summary>
public enum ExchangeType
{
    TWSE,
    TPEx
}

/// <summary>The positional <c>type</c> of <c>stock/intraday/tickers</c>.</summary>
public enum TickersType
{
    Equity,
    Index,
    Warrant,
    OddLot
}
