using System;
using uniffi.marketdata_uniffi;

namespace FugleMarketData.QueryModels.Stock.History;

/// <summary>Filters for <c>stock/historical/candles/{symbol}</c>.</summary>
public sealed class HistoryCandlesRequest : BaseRequest
{
    public HistoryCandlesRequest() { }
    public HistoryCandlesRequest(DateTime from, DateTime to, HistoryTimeFrame? timeFrame = default, FieldsType? fields = default, bool? adjusted = default, SortType? sort = default)
    {
        From = from.Date;
        To = to.Date;
        TimeFrame = timeFrame;
        Fields = fields;
        Adjusted = adjusted;
        Sort = sort;
    }
    public DateTime? From { get; set; } = default;
    public DateTime? To { get; set; } = default;

    public HistoryTimeFrame? TimeFrame { get; init; } = default;

    public FieldsType? Fields { get; init; } = default;

    public bool? Adjusted { get; init; } = default;

    public SortType? Sort { get; init; } = default;

    internal StockHistoricalCandlesParams ToParams() => new StockHistoricalCandlesParams(
        from: QueryValue.Date(From),
        to: QueryValue.Date(To),
        timeframe: HistoryTimeFrameValue.Of(TimeFrame),
        fields: QueryValue.Flags(Fields, FieldsType.All),
        sort: QueryValue.Lower(Sort),
        adjusted: Adjusted);
}

public enum SortType
{
    Asc,
    Desc
}

/// <summary>
/// Minute frames are sent as their number; <see cref="Day"/>, <see cref="Week"/>
/// and <see cref="Month"/> as <c>D</c>, <c>W</c>, <c>M</c>.
/// </summary>
public enum HistoryTimeFrame
{
    OneMin = 1,
    ThreeMin = 3,
    FiveMin = 5,
    TenMin = 10,
    FifteenMin = 15,
    ThirtyMin = 30,
    SixtyMin = 60,
    Day = -1,
    Week = -2,
    Month = -3
}

internal static class HistoryTimeFrameValue
{
    /// <summary>FubonNeo: a positive value is the minute count, otherwise the name's first letter.</summary>
    internal static string? Of(HistoryTimeFrame? value)
    {
        if (!value.HasValue)
        {
            return null;
        }
        var minutes = (int)value.Value;
        return minutes > 0
            ? minutes.ToString(System.Globalization.CultureInfo.InvariantCulture)
            : value.Value.ToString().Substring(0, 1);
    }
}
