using Microsoft.VisualStudio.TestTools.UnitTesting;
using System;
using System.Collections.Generic;
using System.Linq;
using System.Net;
using System.Text;
using System.Text.Json;
using System.Threading;
using System.Threading.Tasks;

namespace MarketdataUniffi.Tests;

/// <summary>
/// The server's JSON reaches the caller untouched.
///
/// This replaces ResponseCompatibilityTests, which used reflection to assert
/// that the mirrored response records had particular properties. That is
/// tautological — the records' own definitions guaranteed it — and it never
/// looked at a real response, so it passed happily while the mirrors were
/// missing 22 fields (11 on FutOptQuote alone). The mirrors are gone; REST
/// methods now return the server's JSON verbatim.
///
/// A real loopback server is used rather than a mock, because the SDK issues
/// HTTP from Rust and no .NET-level interception can reach it.
/// </summary>
[TestClass]
public class PassthroughTests
{
    /// <summary>A real GET /stock/intraday/quote/2330 body, captured 2026-09-16.</summary>
    private const string Quote2330 = """
    {
      "date": "2026-09-16",
      "type": "EQUITY",
      "exchange": "TWSE",
      "market": "TSE",
      "symbol": "2330",
      "name": "台積電",
      "referencePrice": 2380,
      "previousClose": 2385,
      "openPrice": 2375,
      "highPrice": 2385,
      "lowPrice": 2375,
      "closePrice": 2385,
      "avgPrice": 2378.51,
      "change": 5,
      "changePercent": 0.21,
      "amplitude": 0.42,
      "lastPrice": 2385,
      "lastSize": 1,
      "bids": [{"price": 2380, "size": 185}, {"price": 2375, "size": 1407}],
      "asks": [{"price": 2385, "size": 199}, {"price": 2390, "size": 556}],
      "total": {
        "tradeValue": 14566025000,
        "tradeVolume": 6124,
        "tradeVolumeAtBid": 1962,
        "tradeVolumeAtAsk": 2686,
        "transaction": 1922,
        "time": 1789525853959140
      },
      "lastTrade": {
        "bid": 2380, "ask": 2385, "price": 2385, "size": 1,
        "time": 1789525853959140, "serial": 6132837
      },
      "isContinuous": true,
      "serial": 6152257,
      "lastUpdated": 1789525882301247
    }
    """;

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

    /// <summary>Re-serialize without whitespace so only content is compared.</summary>
    private static string Canonical(JsonElement element) => JsonSerializer.Serialize(element);

    private void SkipIfNativeLibraryUnavailable()
    {
        if (!_nativeLibraryAvailable)
        {
            Assert.Inconclusive("Native library not available. Build with: cargo build -p marketdata-uniffi --release");
        }
    }

    /// <summary>Fetch a quote from a loopback server serving <paramref name="body"/>.</summary>
    private async Task<JsonElement> FetchQuoteAsync(string body)
    {
        using var server = new LoopbackServer(body);
        using var client = server.NewClient();

        var json = await client.Stock.Intraday.GetQuoteAsync("2330").ConfigureAwait(false);
        return JsonDocument.Parse(json).RootElement.Clone();
    }

    [TestMethod]
    public async Task ResponseMatchesWhatTheServerSent()
    {
        SkipIfNativeLibraryUnavailable();

        var got = await FetchQuoteAsync(Quote2330);
        var want = JsonDocument.Parse(Quote2330).RootElement;

        var gotNames = got.EnumerateObject().Select(p => p.Name).ToList();
        var wantNames = want.EnumerateObject().Select(p => p.Name).ToList();

        CollectionAssert.AreEquivalent(wantNames, gotNames, "field set differs from the server's");

        foreach (var prop in want.EnumerateObject())
        {
            // Compare canonical JSON so the fixture's own indentation does
            // not count as a difference.
            Assert.AreEqual(
                Canonical(prop.Value),
                Canonical(got.GetProperty(prop.Name)),
                $"field '{prop.Name}' changed");
        }
    }

    [TestMethod]
    public async Task KeyOrderMatchesTheServer()
    {
        SkipIfNativeLibraryUnavailable();

        var got = await FetchQuoteAsync(Quote2330);
        var want = JsonDocument.Parse(Quote2330).RootElement;

        // Previously the JSON went through a sorted map, so callers saw
        // amplitude, asks, avgPrice, bids, ... instead of the server's order.
        CollectionAssert.AreEqual(
            want.EnumerateObject().Select(p => p.Name).ToList(),
            got.EnumerateObject().Select(p => p.Name).ToList());
    }

    [TestMethod]
    public async Task ReferencePriceSurvivesAndIsTheBasisForChange()
    {
        SkipIfNativeLibraryUnavailable();

        var q = await FetchQuoteAsync(Quote2330);

        var reference = q.GetProperty("referencePrice").GetDouble();
        var last = q.GetProperty("lastPrice").GetDouble();
        var change = q.GetProperty("change").GetDouble();
        var previousClose = q.GetProperty("previousClose").GetDouble();

        Assert.AreEqual(2380d, reference);
        Assert.AreEqual(change, last - reference);
        // Deriving it from previousClose would have given the wrong answer.
        Assert.AreNotEqual(change, last - previousClose);
    }

    [TestMethod]
    public async Task OmittedFieldsStayAbsent()
    {
        SkipIfNativeLibraryUnavailable();

        var q = await FetchQuoteAsync(Quote2330);

        // The server sent only isContinuous. The others used to be materialised
        // as false, which callers could not tell apart from a real false.
        Assert.IsTrue(q.GetProperty("isContinuous").GetBoolean());

        foreach (var absent in new[] { "isOpen", "isClose", "isTrial", "isLimitUpPrice", "tradingHalt" })
        {
            Assert.IsFalse(
                q.TryGetProperty(absent, out _),
                $"'{absent}' should be absent — the server did not send it");
        }
    }

    [TestMethod]
    public async Task UnknownFieldStillReachesTheCaller()
    {
        SkipIfNativeLibraryUnavailable();

        var q = await FetchQuoteAsync("""{"symbol":"2330","someFieldAddedLater":{"nested":[1,2]}}""");

        Assert.IsTrue(q.TryGetProperty("someFieldAddedLater", out var added));
        Assert.AreEqual("""{"nested":[1,2]}""", Canonical(added));
    }

    [TestMethod]
    public async Task TickersKeepsTheEnvelope()
    {
        SkipIfNativeLibraryUnavailable();

        const string envelope = """
        {"date":"2026-09-16","type":"EQUITY","exchange":"TWSE","market":"TSE",
         "data":[{"symbol":"2330","name":"台積電"}]}
        """;

        using var server = new LoopbackServer(envelope);
        using var client = server.NewClient();

        var json = await client.Stock.Intraday.GetTickersAsync("EQUITY").ConfigureAwait(false);
        var root = JsonDocument.Parse(json).RootElement;

        // Earlier releases returned just `data`, losing the sibling metadata.
        Assert.AreEqual(JsonValueKind.Object, root.ValueKind);
        Assert.AreEqual("TWSE", root.GetProperty("exchange").GetString());
        Assert.AreEqual(1, root.GetProperty("data").GetArrayLength());
    }

    [TestMethod]
    public void BaseUrlIsActuallyApplied()
    {
        SkipIfNativeLibraryUnavailable();

        // BaseUrl used to be accepted and silently discarded, so a client
        // pointed at a test server still talked to production.
        using var client = new FugleMarketData.RestClient(new FugleMarketData.RestClientOptions
        {
            ApiKey = "test-key",
            BaseUrl = "http://127.0.0.1:65123",
        });

        StringAssert.Contains(client.BaseUrl, "127.0.0.1:65123");
    }
}
