using uniffi.marketdata_uniffi;

namespace FugleMarketData.QueryModels.Stock.Snapshot;

/// <summary>
/// Filters for <c>stock/snapshot/quotes/{market}</c> and
/// <c>stock/snapshot/actives/{market}</c>. FubonNeo has no request type for
/// these endpoints; this one carries the <c>type</c> filter they take.
/// </summary>
public sealed class SnapshotRequest : BaseRequest
{
    public SnapshotRequest() { }
    public SnapshotRequest(string? type)
    {
        Type = type;
    }
    /// <summary><c>ALL</c>, <c>ALLBUT0999</c> or <c>COMMONSTOCK</c>.</summary>
    public string? Type { get; init; } = default;

    internal SnapshotParams ToParams() => new SnapshotParams(typeFilter: Type);
}
