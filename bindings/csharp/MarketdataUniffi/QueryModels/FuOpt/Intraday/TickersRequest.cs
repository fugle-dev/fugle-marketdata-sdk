using uniffi.marketdata_uniffi;

namespace FugleMarketData.QueryModels.FuOpt.Intraday;

/// <summary>Filters for <c>futopt/intraday/tickers</c>.</summary>
public class TickersRequest : BaseRequest
{
    public TickersRequest()
    {
    }
    public TickersRequest(FutOptExchangeType? exchange, SessionType? session, string? product, ContractType? contractType)
    {
        Exchange = exchange;
        Session = session;
        ContractType = contractType;
        Product = product;
    }
    public FutOptExchangeType? Exchange { get; init; } = default;
    public SessionType? Session { get; init; } = default;
    public ContractType? ContractType { get; init; } = default;
    /// <summary>Product code, e.g. <c>TXF</c>; empty means not sent.</summary>
    public string? Product { get; init; } = default;
    public bool? IsSpread { get; init; } = default;

    internal FutOptTickersParams ToParams() => new FutOptTickersParams(
        exchange: QueryValue.Upper(Exchange),
        afterHours: SessionValue.Flag(Session),
        product: QueryValue.NonEmpty(Product),
        contractType: QueryValue.Upper(ContractType),
        isSpread: IsSpread);
}
