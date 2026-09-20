using System;
using FugleMarketData.QueryModels.Stock.History;
using uniffi.marketdata_uniffi;

namespace FugleMarketData.QueryModels.Stock.Technical;

/// <summary>The date range and time frame every technical indicator takes.</summary>
public abstract class TechnicalRequest : BaseRequest
{
    public DateTime? From { get; set; } = default;
    public DateTime? To { get; set; } = default;
    public HistoryTimeFrame? TimeFrame { get; set; } = default;

    internal TechnicalParams ToParams() => new TechnicalParams(
        from: QueryValue.Date(From),
        to: QueryValue.Date(To),
        timeframe: HistoryTimeFrameValue.Of(TimeFrame));
}
