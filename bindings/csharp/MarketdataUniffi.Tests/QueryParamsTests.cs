using Microsoft.VisualStudio.TestTools.UnitTesting;
using System;
using System.Linq;
using System.Threading.Tasks;

namespace MarketdataUniffi.Tests;

/// <summary>
/// The wrapper sends the query pairs a params record maps to (#202): required
/// parameters go on the path or as fixed query keys, and each set field of a
/// params record resolves through the endpoint's table into its wire key.
///
/// A real loopback server is used (see <see cref="LoopbackServer"/>) because
/// the SDK issues HTTP from Rust; only the request's raw URL is inspected
/// here, and pairs are compared as a set — the wire order is an
/// implementation detail of the table walk, not part of the contract.
/// </summary>
[TestClass]
public class QueryParamsTests
{
    private static bool _nativeLibraryAvailable;

    [ClassInitialize]
    public static void ClassInit(TestContext context)
    {
        try
        {
            using var probe = new FugleMarketData.RestClient("test-key");
            _nativeLibraryAvailable = true;
        }
        catch (DllNotFoundException)
        {
            _nativeLibraryAvailable = false;
        }
        catch (TypeInitializationException ex) when (ex.InnerException is DllNotFoundException)
        {
            _nativeLibraryAvailable = false;
        }
        catch
        {
            _nativeLibraryAvailable = true;
        }
    }

    private void SkipIfNativeLibraryUnavailable()
    {
        if (!_nativeLibraryAvailable)
        {
            Assert.Inconclusive("Native library not available. Build with: cargo build -p marketdata-uniffi --release");
        }
    }

    /// <summary>Split a raw URL (path + query string) into its path and query pairs.</summary>
    private static (string Path, string[] Pairs) Parse(string rawUrl)
    {
        var idx = rawUrl.IndexOf('?');
        if (idx < 0)
        {
            return (rawUrl, Array.Empty<string>());
        }
        var path = rawUrl.Substring(0, idx);
        var query = rawUrl.Substring(idx + 1);
        var pairs = query.Split('&', StringSplitOptions.RemoveEmptyEntries);
        return (path, pairs);
    }

    [TestMethod]
    public async Task Trades_OddLotAndLimit_ResolveToWireKeys()
    {
        SkipIfNativeLibraryUnavailable();

        using var server = new LoopbackServer("{}");
        using var client = server.NewClient();

        await client.Stock.Intraday.GetTradesAsync(
            "2330", new uniffi.marketdata_uniffi.StockTradesParams(oddLot: true, limit: 5)).ConfigureAwait(false);

        Assert.IsTrue(server.Requests.TryDequeue(out var rawUrl));
        var (path, pairs) = Parse(rawUrl!);

        StringAssert.EndsWith(path, "/stock/intraday/trades/2330");
        CollectionAssert.AreEquivalent(new[] { "type=oddlot", "limit=5" }, pairs);
    }

    [TestMethod]
    public async Task Movers_DirectionAndChange_ArePositionalQueryKeys()
    {
        SkipIfNativeLibraryUnavailable();

        using var server = new LoopbackServer("{}");
        using var client = server.NewClient();

        await client.Stock.Snapshot.GetMoversAsync("TSE", "up", "percent").ConfigureAwait(false);

        Assert.IsTrue(server.Requests.TryDequeue(out var rawUrl));
        var (path, pairs) = Parse(rawUrl!);

        StringAssert.EndsWith(path, "/stock/snapshot/movers/TSE");
        CollectionAssert.AreEquivalent(new[] { "direction=up", "change=percent" }, pairs);
    }

    [TestMethod]
    public async Task CapitalChanges_RejectsExchange_WithInvalidParameter()
    {
        SkipIfNativeLibraryUnavailable();

        using var server = new LoopbackServer("{}");
        using var client = server.NewClient();

        var ex = await Assert.ThrowsExceptionAsync<uniffi.marketdata_uniffi.MarketDataException.ApiException>(
            () => client.Stock.CorporateActions.GetCapitalChangesAsync(
                new uniffi.marketdata_uniffi.CorporateActionsParams(exchange: "TWSE")));

        Assert.AreEqual(1005, FugleMarketData.MarketDataExceptionExtensions.GetInfo(ex).code);
        // Rejected before any HTTP request is issued.
        Assert.IsFalse(server.Requests.Any());
    }

    [TestMethod]
    public async Task NoParams_SendsEmptyQuery()
    {
        SkipIfNativeLibraryUnavailable();

        using var server = new LoopbackServer("{}");
        using var client = server.NewClient();

        await client.Stock.Intraday.GetQuoteAsync("2330").ConfigureAwait(false);

        Assert.IsTrue(server.Requests.TryDequeue(out var rawUrl));
        var (_, pairs) = Parse(rawUrl!);

        CollectionAssert.AreEqual(Array.Empty<string>(), pairs);
    }

    [TestMethod]
    public async Task FutOptProducts_AfterHours_SendsUppercaseSessionFlag()
    {
        SkipIfNativeLibraryUnavailable();

        using var server = new LoopbackServer("{}");
        using var client = server.NewClient();

        await client.FutOpt.Intraday.GetProductsAsync(
            "F", new uniffi.marketdata_uniffi.FutOptProductsParams(afterHours: true)).ConfigureAwait(false);

        Assert.IsTrue(server.Requests.TryDequeue(out var rawUrl));
        var (path, pairs) = Parse(rawUrl!);

        StringAssert.EndsWith(path, "/futopt/intraday/products");
        CollectionAssert.AreEquivalent(new[] { "type=FUTURE", "session=AFTERHOURS" }, pairs);
    }
}
