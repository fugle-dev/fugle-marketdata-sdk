using Microsoft.VisualStudio.TestTools.UnitTesting;
using System;
using System.Threading.Tasks;
using FugleMarketData;

namespace MarketdataUniffi.Tests;

/// <summary>
/// Tests for <see cref="FugleMarketData.MarketDataExceptionExtensions.GetInfo"/>
/// (#81, unified error spec).
///
/// Uses <see cref="LoopbackServer"/> for the API/auth cases (no network
/// required) and an unreachable loopback port for the network case.
/// </summary>
[TestClass]
public class MarketDataExceptionExtensionsTests
{
    private static bool _nativeLibraryAvailable;

    [ClassInitialize]
    public static void ClassInit(TestContext context)
    {
        try
        {
            using var client = new FugleMarketData.RestClient("test-api-key");
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

    [TestMethod]
    public async Task ApiException_GetInfo_CarriesStatusAndCode()
    {
        SkipIfNativeLibraryUnavailable();

        using var server = new LoopbackServer(
            """{"message":"Resource not found","statusCode":404}""", statusCode: 404);
        using var client = server.NewClient();

        var ex = await AssertEx.ThrowsAnyAsync(() =>
            client.Stock.Intraday.GetQuoteAsync("2330"));

        var mde = ex as uniffi.marketdata_uniffi.MarketDataException;
        Assert.IsNotNull(mde, $"expected a MarketDataException, got {ex.GetType()}");

        var info = mde!.GetInfo();
        Assert.AreEqual((ushort)404, info.status);
        Assert.AreEqual(2003, info.code); // marketdata_core::error_code::API
    }

    [TestMethod]
    public async Task AuthException_GetInfo_CarriesStatusAndAuthSourceKind()
    {
        SkipIfNativeLibraryUnavailable();

        using var server = new LoopbackServer(
            """{"message":"Invalid credentials","statusCode":401}""", statusCode: 401);
        using var client = server.NewClient();

        var ex = await AssertEx.ThrowsAnyAsync(() =>
            client.Stock.Intraday.GetQuoteAsync("2330"));

        var mde = ex as uniffi.marketdata_uniffi.MarketDataException;
        Assert.IsNotNull(mde, $"expected a MarketDataException, got {ex.GetType()}");

        var info = mde!.GetInfo();
        Assert.AreEqual((ushort)401, info.status);
        Assert.AreEqual(2002, info.code); // marketdata_core::error_code::AUTH
        Assert.AreEqual(uniffi.marketdata_uniffi.ErrorSourceKind.Auth, info.sourceKind);
    }

    [TestMethod]
    public async Task ConnectionException_GetInfo_HasConnectionCode()
    {
        SkipIfNativeLibraryUnavailable();

        // Nothing listens on this loopback port: connection refused, no
        // network access required.
        using var client = new FugleMarketData.RestClient(new FugleMarketData.RestClientOptions
        {
            ApiKey = "test-key",
            BaseUrl = "http://127.0.0.1:1",
        });

        var ex = await AssertEx.ThrowsAnyAsync(() =>
            client.Stock.Intraday.GetQuoteAsync("2330"));

        var mde = ex as uniffi.marketdata_uniffi.MarketDataException;
        Assert.IsNotNull(mde, $"expected a MarketDataException, got {ex.GetType()}");

        var info = mde!.GetInfo();
        Assert.AreEqual(2001, info.code); // marketdata_core::error_code::CONNECTION
        Assert.AreEqual(uniffi.marketdata_uniffi.ErrorSourceKind.Network, info.sourceKind);
    }
}
