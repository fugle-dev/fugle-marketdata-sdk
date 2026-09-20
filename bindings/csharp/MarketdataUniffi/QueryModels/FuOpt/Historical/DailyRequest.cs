using System;
using uniffi.marketdata_uniffi;

namespace FugleMarketData.QueryModels.FuOpt.Historical;

/// <summary>Filters for <c>futopt/historical/daily/{symbol}</c>.</summary>
public sealed class DailyRequest : BaseRequest
{
    public DailyRequest() { }
    public DailyRequest(DateTime? date = default, bool? afterHours = default)
    {
        Date = date?.Date;
        AfterHours = afterHours;
    }
    /// <summary>Unset defaults to today.</summary>
    public DateTime? Date { get; set; } = default;
    /// <summary>
    /// <c>true</c> is sent as <c>session=afterhours</c>; <c>false</c> is not
    /// sent. FubonNeo sent <c>afterhours=true|false</c>, a key the server
    /// does not read, so its value never took effect.
    /// </summary>
    public bool? AfterHours { get; init; } = default;

    internal FutOptDailyParams ToParams() => new FutOptDailyParams(
        date: QueryValue.Date(Date),
        afterHours: QueryValue.Flag(AfterHours));
}
