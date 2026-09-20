using uniffi.marketdata_uniffi;

namespace FugleMarketData.QueryModels.FuOpt.Intraday;

/// <summary>Filters for <c>futopt/intraday/products</c>.</summary>
public class ProductsRequest : BaseRequest
{
    public ProductsRequest()
    {
    }
    public ProductsRequest(FutOptExchangeType? exchange, SessionType? session, ContractType? contractType, ProductStatus? productStatus)
    {
        Exchange = exchange;
        Session = session;
        ContractType = contractType;
        ProductStatus = productStatus;
    }
    public FutOptExchangeType? Exchange { get; init; } = default;
    public SessionType? Session { get; init; } = default;
    public ContractType? ContractType { get; init; } = default;
    public ProductStatus? ProductStatus { get; init; } = default;

    internal FutOptProductsParams ToParams() => new FutOptProductsParams(
        exchange: QueryValue.Upper(Exchange),
        afterHours: SessionValue.Flag(Session),
        contractType: QueryValue.Upper(ContractType),
        status: QueryValue.Name(ProductStatus));
}

public enum ProductStatus
{
    N,
    P,
    U,
}
