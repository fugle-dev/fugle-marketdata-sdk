using System;
using FugleMarketData.QueryModels.Stock.History;

namespace FugleMarketData.QueryModels.Stock.Ownership;

/// <summary>Filters for <c>stock/ownership/etf-holdings/{symbol}</c>.</summary>
public sealed class EtfHoldingsRequest : OwnershipRequest
{
    public EtfHoldingsRequest() { }
    public EtfHoldingsRequest(DateTime? from = default, DateTime? to = default, SortType? sort = default)
        : base(from, to, sort) { }
}
