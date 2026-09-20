using System;
using uniffi.marketdata_uniffi;

namespace FugleMarketData.QueryModels.FuOpt.Historical;

/// <summary>Filters for <c>futopt/historical/candles/{product}</c>.</summary>
public sealed class HistoricalCandlesRequest : BaseRequest
{
    public HistoricalCandlesRequest() { }
    public HistoricalCandlesRequest(DateTime? from = default, DateTime? to = default, HistoricalTimeFrame? timeFrame = default, HistoricalFieldsType? fields = default, string? contractMonth = default, HistoricalSortType? sort = default, TradeSession? session = default)
    {
        From = from?.Date;
        To = to?.Date;
        TimeFrame = timeFrame;
        Fields = fields;
        ContractMonth = contractMonth;
        Sort = sort;
        Session = session;
    }
    public DateTime? From { get; set; } = default;
    public DateTime? To { get; set; } = default;
    public HistoricalTimeFrame? TimeFrame { get; init; } = default;
    public HistoricalFieldsType? Fields { get; init; } = default;
    /// <summary><c>YYYYMM</c> or a continuous contract (<c>1!</c>, <c>2!</c>, <c>3!</c>); empty means not sent.</summary>
    public string? ContractMonth { get; init; } = default;
    public HistoricalSortType? Sort { get; init; } = default;
    public TradeSession? Session { get; init; } = default;
    /// <summary>Option strike price. FubonNeo has no such property.</summary>
    public decimal? StrikePrice { get; init; } = default;
    /// <summary><c>CALL</c> or <c>PUT</c>. FubonNeo has no such property.</summary>
    public string? CallPut { get; init; } = default;

    internal FutOptHistoricalCandlesParams ToParams() => new FutOptHistoricalCandlesParams(
        from: QueryValue.Date(From),
        to: QueryValue.Date(To),
        contractMonth: QueryValue.NonEmpty(ContractMonth),
        fields: QueryValue.Flags(Fields, HistoricalFieldsType.All),
        timeframe: TimeFrameValue(TimeFrame),
        sort: QueryValue.Lower(Sort),
        strikePrice: StrikePrice.HasValue ? (double)StrikePrice.Value : (double?)null,
        callPut: QueryValue.NonEmpty(CallPut),
        afterHours: SessionValue.Flag(Session));

    private static string? TimeFrameValue(HistoricalTimeFrame? value)
    {
        if (!value.HasValue)
        {
            return null;
        }
        switch (value.Value)
        {
            case HistoricalTimeFrame.OneMin: return "1";
            case HistoricalTimeFrame.FiveMin: return "5";
            case HistoricalTimeFrame.TenMin: return "10";
            case HistoricalTimeFrame.FifteenMin: return "15";
            case HistoricalTimeFrame.ThirtyMin: return "30";
            case HistoricalTimeFrame.SixtyMin: return "60";
            case HistoricalTimeFrame.Day: return "D";
            case HistoricalTimeFrame.Week: return "W";
            case HistoricalTimeFrame.Month: return "M";
            default: throw new ArgumentOutOfRangeException(nameof(TimeFrame));
        }
    }
}

public enum HistoricalTimeFrame
{
    Day = 0,
    Week = 1,
    Month = 2,
    OneMin = 3,
    FiveMin = 4,
    TenMin = 5,
    FifteenMin = 6,
    ThirtyMin = 7,
    SixtyMin = 8
}

public enum HistoricalSortType
{
    Asc,
    Desc
}

[Flags]
public enum HistoricalFieldsType
{
    Open = 1,
    High = 2,
    Low = 4,
    Close = 8,
    Volume = 16,
    Average = 32,
    Transaction = 64,
    Change = 128,
    All = Open | High | Low | Close | Volume | Average | Transaction | Change
}
