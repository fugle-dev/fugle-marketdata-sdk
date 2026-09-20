using System;
using uniffi.marketdata_uniffi;

namespace FugleMarketData.QueryModels.Stock.Snapshot;

/// <summary>Filters for <c>stock/snapshot/movers/{market}</c>.</summary>
public class MoverRequest : BaseRequest
{
    public MoverRequest() { }
    public MoverRequest(OperationType operation = default, decimal? value = default)
    {
        Operation = operation;
        Price = value;
    }
    public OperationType? Operation { get; init; } = default;
    /// <summary>
    /// Sent only together with <see cref="Operation"/>, formatted
    /// culture-invariantly (FubonNeo used the current culture).
    /// </summary>
    public decimal? Price { get; init; }
    /// <summary><c>ALL</c>, <c>ALLBUT0999</c> or <c>COMMONSTOCK</c>. FubonNeo has no such property.</summary>
    public string? Type { get; init; } = default;

    internal MoversParams ToParams()
    {
        double? gt = null, gte = null, lt = null, lte = null, eq = null;
        if (Operation.HasValue && Price.HasValue)
        {
            var price = (double)Price.Value;
            switch (Operation.Value)
            {
                case OperationType.GreaterThan: gt = price; break;
                case OperationType.GreaterThanOrEqual: gte = price; break;
                case OperationType.LessThan: lt = price; break;
                case OperationType.LessThanOrEqual: lte = price; break;
                case OperationType.Equal: eq = price; break;
                default: throw new NotImplementedException("Operation not implemented");
            }
        }
        return new MoversParams(typeFilter: Type, gt: gt, gte: gte, lt: lt, lte: lte, eq: eq);
    }
}

public enum OperationType
{
    GreaterThan,
    GreaterThanOrEqual,
    LessThan,
    LessThanOrEqual,
    Equal,
}
