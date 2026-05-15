using Microsoft.VisualStudio.TestTools.UnitTesting;
using System;
using System.Threading.Tasks;
using System.Reflection;

namespace MarketdataUniffi.Tests;

/// <summary>
/// Response Compatibility Tests
///
/// Validates that response objects have expected fields and types,
/// ensuring compatibility with official Fugle SDK response structures.
/// </summary>
[TestClass]
public class ResponseCompatibilityTests
{
    private static bool _nativeLibraryAvailable;

    [ClassInitialize]
    public static void ClassInit(TestContext context)
    {
        try
        {
            using var client = new FugleMarketData.RestClient("test-key");
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

    // ========== Quote Response Structure ==========

    [TestMethod]
    public void QuoteResponse_HasSymbolProperty()
    {
        // Use reflection to check response type has expected properties
        var assembly = typeof(FugleMarketData.RestClient).Assembly;
        var quoteType = assembly.GetType("FugleMarketData.Quote")
                     ?? assembly.GetType("uniffi.marketdata_uniffi.Quote");

        Assert.IsNotNull(quoteType, "Quote type should exist");

        var symbolProp = quoteType.GetProperty("symbol", BindingFlags.Public | BindingFlags.Instance | BindingFlags.IgnoreCase)
                      ?? quoteType.GetProperty("Symbol", BindingFlags.Public | BindingFlags.Instance);

        Assert.IsNotNull(symbolProp, "Quote should have symbol property");
        Assert.AreEqual(typeof(string), symbolProp.PropertyType, "Symbol should be string type");
    }

    [TestMethod]
    public void QuoteResponse_HasDateProperty()
    {
        var assembly = typeof(FugleMarketData.RestClient).Assembly;
        var quoteType = assembly.GetType("FugleMarketData.Quote")
                     ?? assembly.GetType("uniffi.marketdata_uniffi.Quote");

        Assert.IsNotNull(quoteType, "Quote type should exist");

        var dateProp = quoteType.GetProperty("date", BindingFlags.Public | BindingFlags.Instance | BindingFlags.IgnoreCase)
                    ?? quoteType.GetProperty("Date", BindingFlags.Public | BindingFlags.Instance);

        Assert.IsNotNull(dateProp, "Quote should have date property");
        Assert.AreEqual(typeof(string), dateProp.PropertyType, "Date should be string type");
    }

    [TestMethod]
    public void QuoteResponse_HasExpectedFields()
    {
        var assembly = typeof(FugleMarketData.RestClient).Assembly;
        var quoteType = assembly.GetType("FugleMarketData.Quote")
                     ?? assembly.GetType("uniffi.marketdata_uniffi.Quote");

        Assert.IsNotNull(quoteType, "Quote type should exist");

        // Required fields
        var requiredFields = new[] { "symbol", "date" };
        foreach (var fieldName in requiredFields)
        {
            var prop = quoteType.GetProperty(fieldName, BindingFlags.Public | BindingFlags.Instance | BindingFlags.IgnoreCase)
                    ?? quoteType.GetProperty(char.ToUpper(fieldName[0]) + fieldName.Substring(1), BindingFlags.Public | BindingFlags.Instance);
            Assert.IsNotNull(prop, $"Quote should have {fieldName} property");
        }

        // Optional fields (should exist in type)
        var optionalFields = new[] { "name", "exchange", "market" };
        foreach (var fieldName in optionalFields)
        {
            var prop = quoteType.GetProperty(fieldName, BindingFlags.Public | BindingFlags.Instance | BindingFlags.IgnoreCase)
                    ?? quoteType.GetProperty(char.ToUpper(fieldName[0]) + fieldName.Substring(1), BindingFlags.Public | BindingFlags.Instance);
            Assert.IsNotNull(prop, $"Quote should have {fieldName} property");
        }
    }

    // ========== Ticker Response Structure ==========

    [TestMethod]
    public void TickerResponse_HasSymbolField()
    {
        var assembly = typeof(FugleMarketData.RestClient).Assembly;
        var tickerType = assembly.GetType("FugleMarketData.Ticker")
                      ?? assembly.GetType("uniffi.marketdata_uniffi.Ticker");

        Assert.IsNotNull(tickerType, "Ticker type should exist");

        var symbolProp = tickerType.GetProperty("symbol", BindingFlags.Public | BindingFlags.Instance | BindingFlags.IgnoreCase)
                      ?? tickerType.GetProperty("Symbol", BindingFlags.Public | BindingFlags.Instance);

        Assert.IsNotNull(symbolProp, "Ticker should have symbol property");
        Assert.AreEqual(typeof(string), symbolProp.PropertyType);
    }

    [TestMethod]
    public void TickerResponse_HasExpectedFields()
    {
        var assembly = typeof(FugleMarketData.RestClient).Assembly;
        var tickerType = assembly.GetType("FugleMarketData.Ticker")
                      ?? assembly.GetType("uniffi.marketdata_uniffi.Ticker");

        Assert.IsNotNull(tickerType, "Ticker type should exist");

        // Required fields
        var requiredFields = new[] { "symbol", "date", "name" };
        foreach (var fieldName in requiredFields)
        {
            var prop = tickerType.GetProperty(fieldName, BindingFlags.Public | BindingFlags.Instance | BindingFlags.IgnoreCase)
                    ?? tickerType.GetProperty(char.ToUpper(fieldName[0]) + fieldName.Substring(1), BindingFlags.Public | BindingFlags.Instance);
            Assert.IsNotNull(prop, $"Ticker should have {fieldName} property");
        }
    }

    // ========== Trades Response Structure ==========

    [TestMethod]
    public void TradesResponse_HasExpectedFields()
    {
        var assembly = typeof(FugleMarketData.RestClient).Assembly;
        var tradesType = assembly.GetType("FugleMarketData.TradesResponse")
                      ?? assembly.GetType("uniffi.marketdata_uniffi.TradesResponse");

        Assert.IsNotNull(tradesType, "TradesResponse type should exist");

        // Required fields
        var requiredFields = new[] { "symbol", "date", "data" };
        foreach (var fieldName in requiredFields)
        {
            var prop = tradesType.GetProperty(fieldName, BindingFlags.Public | BindingFlags.Instance | BindingFlags.IgnoreCase)
                    ?? tradesType.GetProperty(char.ToUpper(fieldName[0]) + fieldName.Substring(1), BindingFlags.Public | BindingFlags.Instance);
            Assert.IsNotNull(prop, $"TradesResponse should have {fieldName} property");
        }
    }

    // ========== Historical Response Structure (Phase 7) ==========

    [TestMethod]
    [TestCategory("Structural")]
    public void HistoricalCandlesResponse_HasExpectedFields()
    {
        var assembly = typeof(FugleMarketData.RestClient).Assembly;
        var responseType = assembly.GetType("uniffi.marketdata_uniffi.HistoricalCandlesResponse");

        Assert.IsNotNull(responseType, "HistoricalCandlesResponse type should exist");

        var requiredFields = new[] { "symbol", "data" };
        foreach (var fieldName in requiredFields)
        {
            var prop = responseType.GetProperty(fieldName, BindingFlags.Public | BindingFlags.Instance | BindingFlags.IgnoreCase)
                    ?? responseType.GetProperty(char.ToUpper(fieldName[0]) + fieldName.Substring(1), BindingFlags.Public | BindingFlags.Instance);
            Assert.IsNotNull(prop, $"HistoricalCandlesResponse should have {fieldName} property");
        }
    }

    [TestMethod]
    [TestCategory("Structural")]
    public void StatsResponse_HasExpectedFields()
    {
        var assembly = typeof(FugleMarketData.RestClient).Assembly;
        var responseType = assembly.GetType("uniffi.marketdata_uniffi.StatsResponse");

        Assert.IsNotNull(responseType, "StatsResponse type should exist");

        var requiredFields = new[] { "symbol", "date", "name" };
        foreach (var fieldName in requiredFields)
        {
            var prop = responseType.GetProperty(fieldName, BindingFlags.Public | BindingFlags.Instance | BindingFlags.IgnoreCase)
                    ?? responseType.GetProperty(char.ToUpper(fieldName[0]) + fieldName.Substring(1), BindingFlags.Public | BindingFlags.Instance);
            Assert.IsNotNull(prop, $"StatsResponse should have {fieldName} property");
        }
    }

    // ========== Snapshot Response Structure (Phase 7) ==========

    [TestMethod]
    [TestCategory("Structural")]
    public void SnapshotQuotesResponse_HasExpectedFields()
    {
        var assembly = typeof(FugleMarketData.RestClient).Assembly;
        var responseType = assembly.GetType("uniffi.marketdata_uniffi.SnapshotQuotesResponse");

        Assert.IsNotNull(responseType, "SnapshotQuotesResponse type should exist");

        var requiredFields = new[] { "date", "time", "market", "data" };
        foreach (var fieldName in requiredFields)
        {
            var prop = responseType.GetProperty(fieldName, BindingFlags.Public | BindingFlags.Instance | BindingFlags.IgnoreCase)
                    ?? responseType.GetProperty(char.ToUpper(fieldName[0]) + fieldName.Substring(1), BindingFlags.Public | BindingFlags.Instance);
            Assert.IsNotNull(prop, $"SnapshotQuotesResponse should have {fieldName} property");
        }
    }

    [TestMethod]
    [TestCategory("Structural")]
    public void MoversResponse_HasExpectedFields()
    {
        var assembly = typeof(FugleMarketData.RestClient).Assembly;
        var responseType = assembly.GetType("uniffi.marketdata_uniffi.MoversResponse");

        Assert.IsNotNull(responseType, "MoversResponse type should exist");

        var requiredFields = new[] { "date", "time", "market", "data" };
        foreach (var fieldName in requiredFields)
        {
            var prop = responseType.GetProperty(fieldName, BindingFlags.Public | BindingFlags.Instance | BindingFlags.IgnoreCase)
                    ?? responseType.GetProperty(char.ToUpper(fieldName[0]) + fieldName.Substring(1), BindingFlags.Public | BindingFlags.Instance);
            Assert.IsNotNull(prop, $"MoversResponse should have {fieldName} property");
        }
    }

    [TestMethod]
    [TestCategory("Structural")]
    public void ActivesResponse_HasExpectedFields()
    {
        var assembly = typeof(FugleMarketData.RestClient).Assembly;
        var responseType = assembly.GetType("uniffi.marketdata_uniffi.ActivesResponse");

        Assert.IsNotNull(responseType, "ActivesResponse type should exist");

        var requiredFields = new[] { "date", "time", "market", "data" };
        foreach (var fieldName in requiredFields)
        {
            var prop = responseType.GetProperty(fieldName, BindingFlags.Public | BindingFlags.Instance | BindingFlags.IgnoreCase)
                    ?? responseType.GetProperty(char.ToUpper(fieldName[0]) + fieldName.Substring(1), BindingFlags.Public | BindingFlags.Instance);
            Assert.IsNotNull(prop, $"ActivesResponse should have {fieldName} property");
        }
    }

    // ========== Technical Indicator Response Structure (Phase 7) ==========

    [TestMethod]
    [TestCategory("Structural")]
    public void SmaResponse_HasExpectedFields()
    {
        var assembly = typeof(FugleMarketData.RestClient).Assembly;
        var responseType = assembly.GetType("uniffi.marketdata_uniffi.SmaResponse");

        Assert.IsNotNull(responseType, "SmaResponse type should exist");

        var requiredFields = new[] { "symbol", "data" };
        foreach (var fieldName in requiredFields)
        {
            var prop = responseType.GetProperty(fieldName, BindingFlags.Public | BindingFlags.Instance | BindingFlags.IgnoreCase)
                    ?? responseType.GetProperty(char.ToUpper(fieldName[0]) + fieldName.Substring(1), BindingFlags.Public | BindingFlags.Instance);
            Assert.IsNotNull(prop, $"SmaResponse should have {fieldName} property");
        }
    }

    [TestMethod]
    [TestCategory("Structural")]
    public void RsiResponse_HasExpectedFields()
    {
        var assembly = typeof(FugleMarketData.RestClient).Assembly;
        var responseType = assembly.GetType("uniffi.marketdata_uniffi.RsiResponse");

        Assert.IsNotNull(responseType, "RsiResponse type should exist");
    }

    [TestMethod]
    [TestCategory("Structural")]
    public void KdjResponse_HasExpectedFields()
    {
        var assembly = typeof(FugleMarketData.RestClient).Assembly;
        var responseType = assembly.GetType("uniffi.marketdata_uniffi.KdjResponse");

        Assert.IsNotNull(responseType, "KdjResponse type should exist");
    }

    [TestMethod]
    [TestCategory("Structural")]
    public void MacdResponse_HasExpectedFields()
    {
        var assembly = typeof(FugleMarketData.RestClient).Assembly;
        var responseType = assembly.GetType("uniffi.marketdata_uniffi.MacdResponse");

        Assert.IsNotNull(responseType, "MacdResponse type should exist");
    }

    [TestMethod]
    [TestCategory("Structural")]
    public void BbResponse_HasExpectedFields()
    {
        var assembly = typeof(FugleMarketData.RestClient).Assembly;
        var responseType = assembly.GetType("uniffi.marketdata_uniffi.BbResponse");

        Assert.IsNotNull(responseType, "BbResponse type should exist");
    }

    // ========== Corporate Actions Response Structure (Phase 7) ==========

    [TestMethod]
    [TestCategory("Structural")]
    public void CapitalChangesResponse_HasExpectedFields()
    {
        var assembly = typeof(FugleMarketData.RestClient).Assembly;
        var responseType = assembly.GetType("uniffi.marketdata_uniffi.CapitalChangesResponse");

        Assert.IsNotNull(responseType, "CapitalChangesResponse type should exist");

        var requiredFields = new[] { "data" };
        foreach (var fieldName in requiredFields)
        {
            var prop = responseType.GetProperty(fieldName, BindingFlags.Public | BindingFlags.Instance | BindingFlags.IgnoreCase)
                    ?? responseType.GetProperty(char.ToUpper(fieldName[0]) + fieldName.Substring(1), BindingFlags.Public | BindingFlags.Instance);
            Assert.IsNotNull(prop, $"CapitalChangesResponse should have {fieldName} property");
        }
    }

    [TestMethod]
    [TestCategory("Structural")]
    public void DividendsResponse_HasExpectedFields()
    {
        var assembly = typeof(FugleMarketData.RestClient).Assembly;
        var responseType = assembly.GetType("uniffi.marketdata_uniffi.DividendsResponse");

        Assert.IsNotNull(responseType, "DividendsResponse type should exist");
    }

    [TestMethod]
    [TestCategory("Structural")]
    public void ListingApplicantsResponse_HasExpectedFields()
    {
        var assembly = typeof(FugleMarketData.RestClient).Assembly;
        var responseType = assembly.GetType("uniffi.marketdata_uniffi.ListingApplicantsResponse");

        Assert.IsNotNull(responseType, "ListingApplicantsResponse type should exist");
    }

    // ========== FutOpt Historical Response Structure (Phase 7) ==========

    [TestMethod]
    [TestCategory("Structural")]
    public void FutOptHistoricalCandlesResponse_HasExpectedFields()
    {
        var assembly = typeof(FugleMarketData.RestClient).Assembly;
        var responseType = assembly.GetType("uniffi.marketdata_uniffi.FutOptHistoricalCandlesResponse");

        Assert.IsNotNull(responseType, "FutOptHistoricalCandlesResponse type should exist");

        // FutOptHistoricalCandlesResponse has 'candles' field instead of 'data'
        var requiredFields = new[] { "symbol", "candles" };
        foreach (var fieldName in requiredFields)
        {
            var prop = responseType.GetProperty(fieldName, BindingFlags.Public | BindingFlags.Instance | BindingFlags.IgnoreCase)
                    ?? responseType.GetProperty(char.ToUpper(fieldName[0]) + fieldName.Substring(1), BindingFlags.Public | BindingFlags.Instance);
            Assert.IsNotNull(prop, $"FutOptHistoricalCandlesResponse should have {fieldName} property");
        }
    }

    [TestMethod]
    [TestCategory("Structural")]
    public void FutOptDailyResponse_HasExpectedFields()
    {
        var assembly = typeof(FugleMarketData.RestClient).Assembly;
        var responseType = assembly.GetType("uniffi.marketdata_uniffi.FutOptDailyResponse");

        Assert.IsNotNull(responseType, "FutOptDailyResponse type should exist");

        var requiredFields = new[] { "symbol", "data" };
        foreach (var fieldName in requiredFields)
        {
            var prop = responseType.GetProperty(fieldName, BindingFlags.Public | BindingFlags.Instance | BindingFlags.IgnoreCase)
                    ?? responseType.GetProperty(char.ToUpper(fieldName[0]) + fieldName.Substring(1), BindingFlags.Public | BindingFlags.Instance);
            Assert.IsNotNull(prop, $"FutOptDailyResponse should have {fieldName} property");
        }
    }

    // ========== Integration Response Tests ==========

    [TestMethod]
    [TestCategory("Integration")]
    public async Task QuoteResponse_LiveData_HasRequiredFields()
    {
        SkipIfNativeLibraryUnavailable();

        var apiKey = Environment.GetEnvironmentVariable("FUGLE_API_KEY");
        if (string.IsNullOrEmpty(apiKey))
        {
            Assert.Inconclusive("FUGLE_API_KEY not set");
            return;
        }

        using var client = new FugleMarketData.RestClient(apiKey);
        var quote = await client.Stock.Intraday.GetQuoteAsync("2330");

        Assert.IsNotNull(quote);
        Assert.IsNotNull(quote.symbol, "Quote must have symbol");
        Assert.AreEqual("2330", quote.symbol);
        Assert.IsNotNull(quote.date, "Quote must have date");
        Assert.IsFalse(string.IsNullOrEmpty(quote.date), "Quote date should not be empty");
    }

    [TestMethod]
    [TestCategory("Integration")]
    public async Task TickerResponse_LiveData_HasRequiredFields()
    {
        SkipIfNativeLibraryUnavailable();

        var apiKey = Environment.GetEnvironmentVariable("FUGLE_API_KEY");
        if (string.IsNullOrEmpty(apiKey))
        {
            Assert.Inconclusive("FUGLE_API_KEY not set");
            return;
        }

        using var client = new FugleMarketData.RestClient(apiKey);
        var ticker = await client.Stock.Intraday.GetTickerAsync("2330");

        Assert.IsNotNull(ticker);
        Assert.IsNotNull(ticker.symbol, "Ticker must have symbol");
        Assert.AreEqual("2330", ticker.symbol);
        Assert.IsNotNull(ticker.date, "Ticker must have date");
        Assert.IsNotNull(ticker.name, "Ticker must have name");
        Assert.IsFalse(string.IsNullOrEmpty(ticker.name), "Ticker name should not be empty");
    }

    [TestMethod]
    [TestCategory("Integration")]
    public async Task TradesResponse_LiveData_HasRequiredFields()
    {
        SkipIfNativeLibraryUnavailable();

        var apiKey = Environment.GetEnvironmentVariable("FUGLE_API_KEY");
        if (string.IsNullOrEmpty(apiKey))
        {
            Assert.Inconclusive("FUGLE_API_KEY not set");
            return;
        }

        using var client = new FugleMarketData.RestClient(apiKey);
        var trades = await client.Stock.Intraday.GetTradesAsync("2330");

        Assert.IsNotNull(trades);
        Assert.IsNotNull(trades.symbol, "Trades must have symbol");
        Assert.AreEqual("2330", trades.symbol);
        Assert.IsNotNull(trades.date, "Trades must have date");
        Assert.IsNotNull(trades.data, "Trades must have data array");
    }
}
