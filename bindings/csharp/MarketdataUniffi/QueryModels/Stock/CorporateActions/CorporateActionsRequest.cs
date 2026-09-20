using System;
using uniffi.marketdata_uniffi;

namespace FugleMarketData.QueryModels.Stock.CorporateActions;

public enum SortType
{
    Asc,
    Desc
}

/// <summary>
/// Filters for the three <c>stock/corporate-actions/*</c> endpoints.
/// </summary>
public class CorporateActionsRequest : BaseRequest
{
    public DateTime? StartDate { get; set; }
    public DateTime? EndDate { get; set; }
    public SortType? Sort { get; set; }
    /// <summary>
    /// <c>TWSE</c> or <c>TPEx</c>; <c>dividends</c> and <c>listing-applicants</c>
    /// only — <c>capital-changes</c> rejects it (code 1005) before any request.
    /// FubonNeo has no such property.
    /// </summary>
    public string? Exchange { get; set; }

    internal CorporateActionsParams ToParams() => new CorporateActionsParams(
        startDate: QueryValue.Date(StartDate),
        endDate: QueryValue.Date(EndDate),
        exchange: QueryValue.NonEmpty(Exchange),
        sort: QueryValue.Lower(Sort));
}
