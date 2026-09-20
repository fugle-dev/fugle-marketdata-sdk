// Conversions from the FubonNeo request shapes (enums, DateTime, int) to the
// string / bool / uint values the uniffi params records carry. Every rule is
// the one FubonNeo's `SetQuery()` applied, so the query the server sees is the
// same; the differences are listed in the README's migration section.
using System;
using System.Collections.Generic;
using System.Globalization;

namespace FugleMarketData.QueryModels;

internal static class QueryValue
{
    /// <summary><c>yyyy-MM-dd</c>, invariant culture.</summary>
    internal static string? Date(DateTime? value) =>
        value?.ToString("yyyy-MM-dd", CultureInfo.InvariantCulture);

    /// <summary>Enum member name as-is (<c>TSE</c>, <c>TPEx</c>, <c>N</c>).</summary>
    internal static string? Name<TEnum>(TEnum? value) where TEnum : struct, Enum =>
        value?.ToString();

    /// <summary>Enum member name lower-cased (<c>asc</c>, <c>up</c>, <c>percent</c>).</summary>
    internal static string? Lower<TEnum>(TEnum? value) where TEnum : struct, Enum =>
        value?.ToString().ToLowerInvariant();

    /// <summary>Enum member name upper-cased (<c>TAIFEX</c>, <c>FUTURE</c>, <c>EQUITY</c>).</summary>
    internal static string? Upper<TEnum>(TEnum? value) where TEnum : struct, Enum =>
        value?.ToString().ToUpperInvariant();

    /// <summary>Minute time frames: the enum's numeric value in decimal.</summary>
    internal static string? Minutes<TEnum>(TEnum? value) where TEnum : struct, Enum =>
        value.HasValue ? Convert.ToInt32(value.Value, CultureInfo.InvariantCulture).ToString(CultureInfo.InvariantCulture) : null;

    /// <summary>
    /// A <c>[Flags]</c> enum as the comma-joined, lower-cased names of the
    /// members it contains, in numeric order and without the <paramref name="all"/>
    /// aggregate: <c>open,high,low,close</c>.
    /// </summary>
    internal static string? Flags<TEnum>(TEnum? value, TEnum all) where TEnum : struct, Enum
    {
        if (!value.HasValue)
        {
            return null;
        }
        var names = new List<string>();
        foreach (TEnum member in Enum.GetValues(typeof(TEnum)))
        {
            if (!member.Equals(all) && value.Value.HasFlag(member))
            {
                names.Add(member.ToString().ToLowerInvariant());
            }
        }
        return string.Join(",", names);
    }

    /// <summary>FubonNeo treats an empty string as "not set" for free-text filters.</summary>
    internal static string? NonEmpty(string? value) =>
        string.IsNullOrEmpty(value) ? null : value;

    /// <summary>
    /// A count (offset, limit) for a <c>uint?</c> record field. The server
    /// answers 400 to a negative value; the SDK's type cannot carry one.
    /// </summary>
    internal static uint? Count(int? value, string name)
    {
        if (!value.HasValue)
        {
            return null;
        }
        return Count(value.Value, name);
    }

    /// <summary>A required count (technical periods) for a <c>uint</c> argument.</summary>
    internal static uint Count(int value, string name)
    {
        if (value < 0)
        {
            throw new ArgumentOutOfRangeException(name, value, "must not be negative");
        }
        return (uint)value;
    }

    /// <summary>Only <c>true</c> is a flag; <c>false</c> and unset are both "not sent".</summary>
    internal static bool? Flag(bool? value) =>
        value == true ? true : (bool?)null;
}
