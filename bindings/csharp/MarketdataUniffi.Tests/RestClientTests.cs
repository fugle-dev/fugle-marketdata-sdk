using Microsoft.VisualStudio.TestTools.UnitTesting;
using System;
using System.Text.Json;
using System.Threading.Tasks;

namespace MarketdataUniffi.Tests;

/// <summary>
/// Tests for FugleMarketData.RestClient wrapper over UniFFI bindings.
/// Structural tests verify type existence and API shape.
/// Integration tests (marked with TestCategory) require FUGLE_API_KEY.
/// </summary>
[TestClass]
public class RestClientTests
{
    private static bool _nativeLibraryAvailable;

    [ClassInitialize]
    public static void ClassInit(TestContext context)
    {
        // Check if native library is available by attempting to load it
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
            // Other exceptions mean the library loaded but failed validation
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

    private static void AssertCredentialsRejected(Action create)
    {
        var ex = Assert.ThrowsException<uniffi.marketdata_uniffi.MarketDataException.ConfigException>(create);
        var info = FugleMarketData.MarketDataExceptionExtensions.GetInfo(ex);
        Assert.AreEqual(1004, info.code); // marketdata_core::error_code::CONFIG
        StringAssert.Contains(info.message, "exactly one non-empty credential");
    }

    // ========== Structural Tests (Type Existence) ==========

    [TestMethod]
    public void RestClient_TypeExists()
    {
        var type = typeof(FugleMarketData.RestClient);
        Assert.IsNotNull(type);
        Assert.IsTrue(typeof(IDisposable).IsAssignableFrom(type));
    }

    [TestMethod]
    public void StockClient_TypeExists()
    {
        var type = typeof(FugleMarketData.StockClient);
        Assert.IsNotNull(type);
    }

    [TestMethod]
    public void StockIntradayClient_TypeExists()
    {
        var type = typeof(FugleMarketData.StockIntradayClient);
        Assert.IsNotNull(type);
    }

    [TestMethod]
    public void FutOptClient_TypeExists()
    {
        var type = typeof(FugleMarketData.FutOptClient);
        Assert.IsNotNull(type);
    }

    [TestMethod]
    public void FutOptIntradayClient_TypeExists()
    {
        var type = typeof(FugleMarketData.FutOptIntradayClient);
        Assert.IsNotNull(type);
    }

    // ========== API Shape Tests ==========

    [TestMethod]
    public void StockIntradayClient_HasExpectedMethods()
    {
        var type = typeof(FugleMarketData.StockIntradayClient);

        // Async methods
        Assert.IsNotNull(type.GetMethod("GetQuoteAsync"));
        Assert.IsNotNull(type.GetMethod("GetTickerAsync"));
        Assert.IsNotNull(type.GetMethod("GetTradesAsync"));
        Assert.IsNotNull(type.GetMethod("GetCandlesAsync"));
        Assert.IsNotNull(type.GetMethod("GetVolumesAsync"));

        // Sync methods
        Assert.IsNotNull(type.GetMethod("GetQuote"));
        Assert.IsNotNull(type.GetMethod("GetTicker"));
        Assert.IsNotNull(type.GetMethod("GetTrades"));
        Assert.IsNotNull(type.GetMethod("GetCandles"));
        Assert.IsNotNull(type.GetMethod("GetVolumes"));
    }

    [TestMethod]
    public void FutOptIntradayClient_HasExpectedMethods()
    {
        var type = typeof(FugleMarketData.FutOptIntradayClient);

        // Async methods
        Assert.IsNotNull(type.GetMethod("GetQuoteAsync"));
        Assert.IsNotNull(type.GetMethod("GetTickerAsync"));
        Assert.IsNotNull(type.GetMethod("GetProductsAsync"));

        // Sync methods
        Assert.IsNotNull(type.GetMethod("GetQuote"));
        Assert.IsNotNull(type.GetMethod("GetTicker"));
        Assert.IsNotNull(type.GetMethod("GetProducts"));
    }

    // ========== Constructor Tests (require native library) ==========

    [TestMethod]
    public void CreateRestClient_WithApiKey_Succeeds()
    {
        SkipIfNativeLibraryUnavailable();

        using var client = new FugleMarketData.RestClient("test-api-key");
        Assert.IsNotNull(client);
    }

    [TestMethod]
    public void CreateRestClient_WithEmptyApiKey_ThrowsConfigError()
    {
        SkipIfNativeLibraryUnavailable();

        AssertCredentialsRejected(() => new FugleMarketData.RestClient(""));
        AssertCredentialsRejected(() => new FugleMarketData.RestClient("   "));
        AssertCredentialsRejected(() => FugleMarketData.RestClient.WithBearerToken(""));
        AssertCredentialsRejected(() => FugleMarketData.RestClient.WithSdkToken(" "));
    }

    [TestMethod]
    public void CreateRestClient_WithNullApiKey_ThrowsConfigError()
    {
        SkipIfNativeLibraryUnavailable();

        AssertCredentialsRejected(() => new FugleMarketData.RestClient((string)null!));
    }

    [TestMethod]
    public void RestClient_Stock_ReturnsStockClient()
    {
        SkipIfNativeLibraryUnavailable();

        using var client = new FugleMarketData.RestClient("test-api-key");
        var stock = client.Stock;
        Assert.IsNotNull(stock);
    }

    [TestMethod]
    public void RestClient_FutOpt_ReturnsFutOptClient()
    {
        SkipIfNativeLibraryUnavailable();

        using var client = new FugleMarketData.RestClient("test-api-key");
        var futopt = client.FutOpt;
        Assert.IsNotNull(futopt);
    }

    [TestMethod]
    public void StockClient_Intraday_ReturnsIntradayClient()
    {
        SkipIfNativeLibraryUnavailable();

        using var client = new FugleMarketData.RestClient("test-api-key");
        var intraday = client.Stock.Intraday;
        Assert.IsNotNull(intraday);
    }

    // ========== Factory Method Tests ==========

    [TestMethod]
    public void WithSdkToken_ReturnsRestClient()
    {
        SkipIfNativeLibraryUnavailable();

        using var client = FugleMarketData.RestClient.WithSdkToken("test-sdk-token");
        Assert.IsNotNull(client);
    }

    [TestMethod]
    public void WithBearerToken_ReturnsRestClient()
    {
        SkipIfNativeLibraryUnavailable();

        using var client = FugleMarketData.RestClient.WithBearerToken("test-bearer-token");
        Assert.IsNotNull(client);
    }

    // ========== Integration Tests (require FUGLE_API_KEY) ==========

    [TestMethod]
    [TestCategory("Integration")]
    public async Task GetQuoteAsync_WithValidKey_ReturnsQuote()
    {
        SkipIfNativeLibraryUnavailable();

        var apiKey = Environment.GetEnvironmentVariable("FUGLE_API_KEY");
        if (string.IsNullOrEmpty(apiKey))
        {
            Assert.Inconclusive("FUGLE_API_KEY environment variable not set");
            return;
        }

        using var client = new FugleMarketData.RestClient(apiKey);
        var quote = await client.Stock.Intraday.GetQuoteAsync("2330");

        Assert.IsNotNull(quote);
        // REST methods hand back the server's JSON verbatim.
        Assert.AreEqual("2330", JsonDocument.Parse(quote).RootElement.GetProperty("symbol").GetString());
    }

    [TestMethod]
    [TestCategory("Integration")]
    public async Task GetTickerAsync_WithValidKey_ReturnsTicker()
    {
        SkipIfNativeLibraryUnavailable();

        var apiKey = Environment.GetEnvironmentVariable("FUGLE_API_KEY");
        if (string.IsNullOrEmpty(apiKey))
        {
            Assert.Inconclusive("FUGLE_API_KEY environment variable not set");
            return;
        }

        using var client = new FugleMarketData.RestClient(apiKey);
        var ticker = await client.Stock.Intraday.GetTickerAsync("2330");

        Assert.IsNotNull(ticker);
        Assert.AreEqual("2330", JsonDocument.Parse(ticker).RootElement.GetProperty("symbol").GetString());
    }
}
