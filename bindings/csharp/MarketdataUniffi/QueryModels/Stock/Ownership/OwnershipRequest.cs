using System;
using FugleMarketData.QueryModels.Stock.History;
using uniffi.marketdata_uniffi;

namespace FugleMarketData.QueryModels.Stock.Ownership;

/// <summary>
/// Filters for the four <c>stock/ownership/*</c> endpoints. FubonNeo only
/// had <see cref="EtfHoldingsRequest"/>, which derives from this.
/// </summary>
public class OwnershipRequest : BaseRequest
{
    public OwnershipRequest() { }
    public OwnershipRequest(DateTime? from = default, DateTime? to = default, SortType? sort = default)
    {
        From = from?.Date;
        To = to?.Date;
        Sort = sort;
    }
    public DateTime? From { get; set; } = default;
    public DateTime? To { get; set; } = default;
    public SortType? Sort { get; init; } = default;

    internal OwnershipParams ToParams() => new OwnershipParams(
        from: QueryValue.Date(From),
        to: QueryValue.Date(To),
        sort: QueryValue.Lower(Sort));
}
