using Microsoft.VisualStudio.TestTools.UnitTesting;
using System;
using System.Globalization;
using System.Linq;
using System.Reflection;
using System.Threading.Tasks;
using FugleMarketData;
using FugleMarketData.QueryModels;
using FugleMarketData.QueryModels.FuOpt;
using FugleMarketData.QueryModels.FuOpt.Historical;
using FugleMarketData.QueryModels.Stock.History;
using FugleMarketData.QueryModels.Stock.Intraday;
using FugleMarketData.QueryModels.Stock.Ownership;
using FugleMarketData.QueryModels.Stock.Snapshot;
using FugleMarketData.QueryModels.Stock.Technical;
using CorporateActions = FugleMarketData.QueryModels.Stock.CorporateActions;
using FuOptIntraday = FugleMarketData.QueryModels.FuOpt.Intraday;

namespace MarketdataUniffi.Tests;

/// <summary>
/// The FubonNeo-compatible REST surface (#203): the request models convert
/// to the uniffi params records exactly as FubonNeo's <c>SetQuery()</c>
/// built its query string (Q1, pure), the wrapper sends those pairs (Q2,
/// loopback), every endpoint has its three names with the right return
/// types (Q3, reflection), and the two places the SDK is stricter than
/// FubonNeo — negative counts, <c>exchange</c> on capital-changes — fail
/// before any request (Q4).
/// </summary>
[TestClass]
public class FubonNeoCompatTests
{
    private static bool _nativeLibraryAvailable;

    [ClassInitialize]
    public static void ClassInit(TestContext context)
    {
        try
        {
            using var probe = new RestClient("test-key");
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

    private static void SkipIfNativeLibraryUnavailable()
    {
        if (!_nativeLibraryAvailable)
        {
            Assert.Inconclusive("Native library not available. Build with: cargo build -p marketdata-uniffi --release");
        }
    }

    /// <summary>
    /// Split a raw URL into its path and its decoded <c>key=value</c> pairs.
    /// The Rust side percent-encodes values (<c>open,close</c> travels as
    /// <c>open%2Cclose</c>); the contract is the decoded pair set.
    /// </summary>
    private static (string Path, string[] Pairs) Parse(string rawUrl)
    {
        var idx = rawUrl.IndexOf('?');
        if (idx < 0)
        {
            return (rawUrl, Array.Empty<string>());
        }
        var pairs = rawUrl.Substring(idx + 1)
            .Split('&', StringSplitOptions.RemoveEmptyEntries)
            .Select(Uri.UnescapeDataString)
            .ToArray();
        return (rawUrl.Substring(0, idx), pairs);
    }

    private static void AssertRequest(LoopbackServer server, string pathSuffix, params string[] pairs)
    {
        Assert.IsTrue(server.Requests.TryDequeue(out var rawUrl), "no request reached the server");
        var (path, got) = Parse(rawUrl!);
        StringAssert.EndsWith(path, pathSuffix);
        CollectionAssert.AreEquivalent(pairs, got, $"query of {rawUrl}");
    }

    // ========== Q1: ToParams() value mapping (no native library) ==========

    [TestMethod]
    public void EnumValues_AreFubonNeos()
    {
        // Code moved from FubonNeo may cast or serialize these; the mapping
        // tests below go by name and would not notice a renumbering.
        Assert.AreEqual(-1, (int)HistoryTimeFrame.Day);
        Assert.AreEqual(-2, (int)HistoryTimeFrame.Week);
        Assert.AreEqual(-3, (int)HistoryTimeFrame.Month);
        Assert.AreEqual(3, (int)HistoryTimeFrame.ThreeMin);
        Assert.AreEqual(60, (int)HistoryTimeFrame.SixtyMin);

        Assert.AreEqual(0, (int)HistoricalTimeFrame.Day);
        Assert.AreEqual(1, (int)HistoricalTimeFrame.Week);
        Assert.AreEqual(2, (int)HistoricalTimeFrame.Month);
        Assert.AreEqual(3, (int)HistoricalTimeFrame.OneMin);
        Assert.AreEqual(8, (int)HistoricalTimeFrame.SixtyMin);

        Assert.AreEqual(5, (int)IntradayTimeFrame.FiveMin);
        Assert.AreEqual(30, (int)FuOptIntraday.CandlesTimeFrame.ThirtyMin);

        Assert.AreEqual(1, (int)FieldsType.Open);
        Assert.AreEqual(64, (int)FieldsType.Change);
        Assert.AreEqual(128, (int)FieldsType.Average);
        Assert.AreEqual(255, (int)FieldsType.All);
        Assert.AreEqual(32, (int)HistoricalFieldsType.Average);
        Assert.AreEqual(64, (int)HistoricalFieldsType.Transaction);
        Assert.AreEqual(128, (int)HistoricalFieldsType.Change);
        Assert.AreEqual(255, (int)HistoricalFieldsType.All);
    }

    [TestMethod]
    public void EveryRequest_IsABaseRequest()
    {
        var requests = typeof(BaseRequest).Assembly.GetTypes()
            .Where(t => t.IsClass && t.Namespace!.StartsWith("FugleMarketData.QueryModels", StringComparison.Ordinal)
                        && t.Name.EndsWith("Request", StringComparison.Ordinal))
            .ToArray();
        Assert.IsTrue(requests.Length >= 25, $"found {requests.Length}");
        foreach (var t in requests)
        {
            Assert.IsTrue(typeof(BaseRequest).IsAssignableFrom(t), t.FullName);
        }
    }

    [TestMethod]
    public void AddedStringProperties_EmptyMeansNotSent()
    {
        Assert.IsNull(new TickersRequest { Symbol = "" }.ToParams().symbol);
        Assert.IsNull(new CorporateActions.CorporateActionsRequest { Exchange = "" }.ToParams().exchange);
        Assert.IsNull(new HistoricalCandlesRequest { CallPut = "" }.ToParams().callPut);
    }

    [TestMethod]
    public void HistoryTimeFrame_MinutesAreNumbers_OthersAreInitials()
    {
        string? Of(HistoryTimeFrame tf) =>
            new HistoryCandlesRequest { TimeFrame = tf }.ToParams().timeframe;

        Assert.AreEqual("1", Of(HistoryTimeFrame.OneMin));
        Assert.AreEqual("3", Of(HistoryTimeFrame.ThreeMin));
        Assert.AreEqual("60", Of(HistoryTimeFrame.SixtyMin));
        Assert.AreEqual("D", Of(HistoryTimeFrame.Day));
        Assert.AreEqual("W", Of(HistoryTimeFrame.Week));
        Assert.AreEqual("M", Of(HistoryTimeFrame.Month));
        Assert.IsNull(new HistoryCandlesRequest().ToParams().timeframe);

        // The technical requests share the enum and the rule.
        Assert.AreEqual("D", new SmaRequest(5, timeFrame: HistoryTimeFrame.Day).ToParams().timeframe);
    }

    [TestMethod]
    public void HistoricalTimeFrame_FutOpt_MapsBySwitch()
    {
        string? Of(HistoricalTimeFrame tf) =>
            new HistoricalCandlesRequest { TimeFrame = tf }.ToParams().timeframe;

        // The enum values are 0..8, not the minute counts, so this is not `(int)`.
        Assert.AreEqual("D", Of(HistoricalTimeFrame.Day));
        Assert.AreEqual("W", Of(HistoricalTimeFrame.Week));
        Assert.AreEqual("M", Of(HistoricalTimeFrame.Month));
        Assert.AreEqual("1", Of(HistoricalTimeFrame.OneMin));
        Assert.AreEqual("5", Of(HistoricalTimeFrame.FiveMin));
        Assert.AreEqual("10", Of(HistoricalTimeFrame.TenMin));
        Assert.AreEqual("15", Of(HistoricalTimeFrame.FifteenMin));
        Assert.AreEqual("30", Of(HistoricalTimeFrame.ThirtyMin));
        Assert.AreEqual("60", Of(HistoricalTimeFrame.SixtyMin));
    }

    [TestMethod]
    public void IntradayTimeFrames_AreTheMinuteCount()
    {
        Assert.AreEqual("5", new IntradayCandlesRequest(timeFrame: IntradayTimeFrame.FiveMin).ToParams().timeframe);
        Assert.AreEqual("60", new FuOptIntraday.CandlesRequest(timeFrame: FuOptIntraday.CandlesTimeFrame.SixtyMin).ToParams().timeframe);
        Assert.IsNull(new IntradayCandlesRequest().ToParams().timeframe);
    }

    [TestMethod]
    public void Fields_AreLowerCaseNamesInNumericOrder_WithoutAll()
    {
        Assert.AreEqual("open,close",
            new HistoryCandlesRequest { Fields = FieldsType.Open | FieldsType.Close }.ToParams().fields);
        Assert.AreEqual("open,high,low,close,volume,turnover,change,average",
            new HistoryCandlesRequest { Fields = FieldsType.All }.ToParams().fields);
        Assert.IsNull(new HistoryCandlesRequest().ToParams().fields);

        Assert.AreEqual("open,volume",
            new HistoricalCandlesRequest { Fields = HistoricalFieldsType.Volume | HistoricalFieldsType.Open }.ToParams().fields);
        Assert.AreEqual("open,high,low,close,volume,average,transaction,change",
            new HistoricalCandlesRequest { Fields = HistoricalFieldsType.All }.ToParams().fields);
    }

    [TestMethod]
    public void Sort_IsLowerCase_InEveryNamespace()
    {
        Assert.AreEqual("desc", new HistoryCandlesRequest { Sort = SortType.Desc }.ToParams().sort);
        Assert.AreEqual("asc", new CorporateActions.CorporateActionsRequest { Sort = CorporateActions.SortType.Asc }.ToParams().sort);
        Assert.AreEqual("desc", new HistoricalCandlesRequest { Sort = HistoricalSortType.Desc }.ToParams().sort);
        Assert.AreEqual("asc", new OwnershipRequest(sort: SortType.Asc).ToParams().sort);
        Assert.AreEqual("desc", new TradeRequest { Sort = SortType.Desc }.ToParams().sort);
        Assert.AreEqual("asc", new IntradayCandlesRequest { Sort = SortType.Asc }.ToParams().sort);
    }

    [TestMethod]
    public void Dates_AreIsoDayInvariant_TimeDropped()
    {
        var noon = new DateTime(2024, 1, 2, 13, 30, 0);

        var history = new HistoryCandlesRequest(noon, new DateTime(2024, 1, 31)).ToParams();
        Assert.AreEqual("2024-01-02", history.from);
        Assert.AreEqual("2024-01-31", history.to);

        var actions = new CorporateActions.CorporateActionsRequest { StartDate = noon, EndDate = noon }.ToParams();
        Assert.AreEqual("2024-01-02", actions.startDate);
        Assert.AreEqual("2024-01-02", actions.endDate);

        Assert.AreEqual("2024-01-02", new DailyRequest(noon).ToParams().date);
        Assert.AreEqual("2024-01-02", new EtfHoldingsRequest(from: noon).ToParams().from);
        Assert.AreEqual("2024-01-02", new RsiRequest(14, from: noon).ToParams().from);
        Assert.IsNull(new RsiRequest(14).ToParams().from);
    }

    [TestMethod]
    public void OddLot_TickerType_MapsToOddLotFlag()
    {
        Assert.IsTrue(new TickerRequest(TickerType.OddLot).ToParams().oddLot);
        Assert.IsNull(new TickerRequest().ToParams().oddLot);
        Assert.IsTrue(new QuoteRequest(TickerType.OddLot).ToParams().oddLot);
        Assert.IsTrue(new VolumeRequest(TickerType.OddLot).ToParams().oddLot);
        Assert.IsTrue(new IntradayCandlesRequest(TickerType.OddLot).ToParams().oddLot);
        Assert.IsTrue(new TradeRequest(TickerType.OddLot).ToParams().oddLot);
    }

    [TestMethod]
    public void Tickers_EnumsAsIs_EmptyIndustryNotSent()
    {
        var p = new TickersRequest { Market = MarketType.OTC, Exchange = ExchangeType.TPEx }.ToParams();
        Assert.AreEqual("OTC", p.market);
        Assert.AreEqual("TPEx", p.exchange); // mixed case, as FubonNeo sent it
        Assert.IsNull(p.industry);           // "" is the default and means not sent
        Assert.IsNull(p.isNormal);
        Assert.IsNull(p.symbol);

        Assert.AreEqual("24", new TickersRequest { Industry = "24" }.ToParams().industry);
        Assert.AreEqual("2330", new TickersRequest { Symbol = "2330" }.ToParams().symbol);
    }

    [TestMethod]
    public void Tickers_IsNormalTrue_ForcesAttentionAndDispositionFalse_WithoutMutating()
    {
        var request = new TickersRequest { IsNormal = true, IsAttention = true, IsDisposition = true, IsHalted = true };
        var p = request.ToParams();

        Assert.IsTrue(p.isNormal);
        Assert.IsFalse(p.isAttention);
        Assert.IsFalse(p.isDisposition);
        Assert.IsTrue(p.isHalted);
        // FubonNeo overwrote the caller's properties; the object is left alone here.
        Assert.IsTrue(request.IsAttention);
        Assert.IsTrue(request.IsDisposition);

        var notNormal = new TickersRequest { IsNormal = false, IsAttention = true }.ToParams();
        Assert.IsFalse(notNormal.isNormal);
        Assert.IsTrue(notNormal.isAttention);
        Assert.IsNull(notNormal.isDisposition);
    }

    [TestMethod]
    public void Movers_OperationAndPrice_BothRequired_PriceInvariant()
    {
        Assert.AreEqual(2380.5, new MoverRequest(OperationType.GreaterThan, 2380.5m).ToParams().gt);
        Assert.AreEqual(2380.5, new MoverRequest(OperationType.GreaterThanOrEqual, 2380.5m).ToParams().gte);
        Assert.AreEqual(1.0, new MoverRequest(OperationType.LessThan, 1m).ToParams().lt);
        Assert.AreEqual(1.0, new MoverRequest(OperationType.LessThanOrEqual, 1m).ToParams().lte);
        Assert.AreEqual(0.0, new MoverRequest(OperationType.Equal, 0m).ToParams().eq);

        var onlyOperation = new MoverRequest { Operation = OperationType.GreaterThan }.ToParams();
        Assert.IsNull(onlyOperation.gt);
        var onlyPrice = new MoverRequest { Price = 1m }.ToParams();
        Assert.IsNull(onlyPrice.gt);
        Assert.IsNull(onlyPrice.gte);

        Assert.AreEqual("COMMONSTOCK", new MoverRequest { Type = "COMMONSTOCK" }.ToParams().typeFilter);
        Assert.AreEqual("ALLBUT0999", new SnapshotRequest("ALLBUT0999").ToParams().typeFilter);

        // The price is a double on the record; the culture cannot reach the wire
        // (FubonNeo's `decimal.ToString()` sent `2380,5` under de-DE).
        var culture = CultureInfo.CurrentCulture;
        try
        {
            CultureInfo.CurrentCulture = new CultureInfo("de-DE");
            Assert.AreEqual(2380.5, new MoverRequest(OperationType.GreaterThanOrEqual, 2380.5m).ToParams().gte);
        }
        finally
        {
            CultureInfo.CurrentCulture = culture;
        }
    }

    [TestMethod]
    public void FutOpt_ListSession_AfterHoursOnly_RegularNotSent()
    {
        Assert.IsTrue(new FuOptIntraday.ProductsRequest { Session = SessionType.AfterHours }.ToParams().afterHours);
        Assert.IsNull(new FuOptIntraday.ProductsRequest { Session = SessionType.Regular }.ToParams().afterHours);
        Assert.IsNull(new FuOptIntraday.ProductsRequest().ToParams().afterHours);
        Assert.IsTrue(new FuOptIntraday.TickersRequest { Session = SessionType.AfterHours }.ToParams().afterHours);
        Assert.IsNull(new FuOptIntraday.TickersRequest { Session = SessionType.Regular }.ToParams().afterHours);
    }

    [TestMethod]
    public void FutOpt_TradeSession_AndEnums()
    {
        Assert.IsTrue(new FuOptIntraday.TickerVolumeRequest(TradeSession.AfterHours).ToParams().afterHours);
        Assert.IsNull(new FuOptIntraday.TickerVolumeRequest(null).ToParams().afterHours);
        Assert.IsTrue(new FuOptIntraday.CandlesRequest(TradeSession.AfterHours).ToParams().afterHours);
        Assert.IsTrue(new FuOptIntraday.TradesRequest(TradeSession.AfterHours).ToParams().afterHours);
        Assert.IsTrue(new HistoricalCandlesRequest(session: TradeSession.AfterHours).ToParams().afterHours);

        var products = new FuOptIntraday.ProductsRequest(FutOptExchangeType.TaiFex, null, ContractType.I, FuOptIntraday.ProductStatus.N).ToParams();
        Assert.AreEqual("TAIFEX", products.exchange);
        Assert.AreEqual("I", products.contractType);
        Assert.AreEqual("N", products.status);

        var tickers = new FuOptIntraday.TickersRequest(FutOptExchangeType.TaiFex, null, "", ContractType.R) { IsSpread = false }.ToParams();
        Assert.AreEqual("TAIFEX", tickers.exchange);
        Assert.IsNull(tickers.product); // "" is not sent
        Assert.AreEqual("R", tickers.contractType);
        Assert.IsFalse(tickers.isSpread);
        Assert.AreEqual("TXF", new FuOptIntraday.TickersRequest { Product = "TXF" }.ToParams().product);
    }

    [TestMethod]
    public void Daily_AfterHours_TrueIsFlag_FalseNotSent()
    {
        Assert.IsTrue(new DailyRequest(afterHours: true).ToParams().afterHours);
        Assert.IsNull(new DailyRequest(afterHours: false).ToParams().afterHours);
        Assert.IsNull(new DailyRequest().ToParams().afterHours);
    }

    [TestMethod]
    public void HistoricalCandles_AddedProperties_AndEmptyContractMonth()
    {
        var p = new HistoricalCandlesRequest(contractMonth: "202403") { StrikePrice = 18000.5m, CallPut = "CALL" }.ToParams();
        Assert.AreEqual("202403", p.contractMonth);
        Assert.AreEqual(18000.5, p.strikePrice);
        Assert.AreEqual("CALL", p.callPut);

        var empty = new HistoricalCandlesRequest(contractMonth: "").ToParams();
        Assert.IsNull(empty.contractMonth);
        Assert.IsNull(empty.strikePrice);
    }

    [TestMethod]
    public void Trades_Counts_AndAddedProperties()
    {
        var stock = new TradeRequest(TickerType.OddLot, 10, 5) { IsTrial = true }.ToParams();
        Assert.AreEqual(10u, stock.offset);
        Assert.AreEqual(5u, stock.limit);
        Assert.IsTrue(stock.isTrial);
        Assert.IsNull(new TradeRequest().ToParams().offset);

        var futopt = new FuOptIntraday.TradesRequest(null, 0, 100) { IsTrial = false }.ToParams();
        Assert.AreEqual(0u, futopt.offset);
        Assert.AreEqual(100u, futopt.limit);
        Assert.IsFalse(futopt.isTrial);
    }

    [TestMethod]
    public void Technical_PeriodsSentAsGiven_DefaultIsZero()
    {
        Assert.AreEqual(20u, new SmaRequest(20).PeriodArg());
        Assert.AreEqual(0u, new SmaRequest().PeriodArg()); // FubonNeo sent period=0 too
        Assert.AreEqual(14u, new RsiRequest(14).PeriodArg());
        Assert.AreEqual(20u, new BbRequest(20).PeriodArg());

        var kdj = new KdjRequest(9, 3, 3);
        Assert.AreEqual(9u, kdj.RPeriodArg());
        Assert.AreEqual(3u, kdj.KPeriodArg());
        Assert.AreEqual(3u, kdj.DPeriodArg());

        var macd = new MacdRequest(12, 26, 9);
        Assert.AreEqual(12u, macd.FastArg());
        Assert.AreEqual(26u, macd.SlowArg());
        Assert.AreEqual(9u, macd.SignalArg());
    }

    [TestMethod]
    public void Ownership_EtfHoldingsRequest_IsAnOwnershipRequest()
    {
        OwnershipRequest request = new EtfHoldingsRequest(new DateTime(2024, 1, 1), new DateTime(2024, 1, 31), SortType.Desc);
        var p = request.ToParams();
        Assert.AreEqual("2024-01-01", p.from);
        Assert.AreEqual("2024-01-31", p.to);
        Assert.AreEqual("desc", p.sort);
    }

    // ========== Q4 (pure part): negative counts ==========

    [TestMethod]
    public void NegativeCounts_ThrowArgumentOutOfRange_BeforeAnyRequest()
    {
        Assert.ThrowsException<ArgumentOutOfRangeException>(() => new TradeRequest(offset: -1).ToParams());
        Assert.ThrowsException<ArgumentOutOfRangeException>(() => new TradeRequest(limit: -5).ToParams());
        Assert.ThrowsException<ArgumentOutOfRangeException>(() => new FuOptIntraday.TradesRequest(offset: -1).ToParams());
        Assert.ThrowsException<ArgumentOutOfRangeException>(() => new SmaRequest(-1).PeriodArg());
        Assert.ThrowsException<ArgumentOutOfRangeException>(() => new KdjRequest(9, -3, 3).KPeriodArg());
        Assert.ThrowsException<ArgumentOutOfRangeException>(() => new MacdRequest(12, 26, -9).SignalArg());
    }

    // ========== Q3: three names per endpoint, one parameter shape ==========

    private static readonly (Type Client, string FubonNeo, string Async, string Sync)[] Endpoints =
    {
        (typeof(StockIntradayClient), "Tickers", "GetTickersAsync", "GetTickers"),
        (typeof(StockIntradayClient), "Ticker", "GetTickerAsync", "GetTicker"),
        (typeof(StockIntradayClient), "Quote", "GetQuoteAsync", "GetQuote"),
        (typeof(StockIntradayClient), "Candles", "GetCandlesAsync", "GetCandles"),
        (typeof(StockIntradayClient), "Trades", "GetTradesAsync", "GetTrades"),
        (typeof(StockIntradayClient), "Volume", "GetVolumesAsync", "GetVolumes"),
        (typeof(StockHistoricalClient), "Candles", "GetCandlesAsync", "GetCandles"),
        (typeof(StockHistoricalClient), "Stats", "GetStatsAsync", "GetStats"),
        (typeof(StockSnapshotClient), "Quotes", "GetQuotesAsync", "GetQuotes"),
        (typeof(StockSnapshotClient), "Movers", "GetMoversAsync", "GetMovers"),
        (typeof(StockSnapshotClient), "Actives", "GetActivesAsync", "GetActives"),
        (typeof(StockTechnicalClient), "Sma", "GetSmaAsync", "GetSma"),
        (typeof(StockTechnicalClient), "Rsi", "GetRsiAsync", "GetRsi"),
        (typeof(StockTechnicalClient), "Kdj", "GetKdjAsync", "GetKdj"),
        (typeof(StockTechnicalClient), "Macd", "GetMacdAsync", "GetMacd"),
        (typeof(StockTechnicalClient), "Bb", "GetBbAsync", "GetBb"),
        (typeof(StockCorporateActionsClient), "CapitalChanges", "GetCapitalChangesAsync", "GetCapitalChanges"),
        (typeof(StockCorporateActionsClient), "Dividends", "GetDividendsAsync", "GetDividends"),
        (typeof(StockCorporateActionsClient), "ListingApplicants", "GetListingApplicantsAsync", "GetListingApplicants"),
        (typeof(StockOwnershipClient), "EtfHoldings", "GetEtfHoldingsAsync", "GetEtfHoldings"),
        (typeof(StockOwnershipClient), "InstitutionalTrades", "GetInstitutionalTradesAsync", "GetInstitutionalTrades"),
        (typeof(StockOwnershipClient), "DirectorHoldings", "GetDirectorHoldingsAsync", "GetDirectorHoldings"),
        (typeof(StockOwnershipClient), "TdccDistribution", "GetTdccDistributionAsync", "GetTdccDistribution"),
        (typeof(FutOptIntradayClient), "Products", "GetProductsAsync", "GetProducts"),
        (typeof(FutOptIntradayClient), "Tickers", "GetTickersAsync", "GetTickers"),
        (typeof(FutOptIntradayClient), "Ticker", "GetTickerAsync", "GetTicker"),
        (typeof(FutOptIntradayClient), "Quote", "GetQuoteAsync", "GetQuote"),
        (typeof(FutOptIntradayClient), "Candles", "GetCandlesAsync", "GetCandles"),
        (typeof(FutOptIntradayClient), "Trades", "GetTradesAsync", "GetTrades"),
        (typeof(FutOptIntradayClient), "Volumes", "GetVolumesAsync", "GetVolumes"),
        (typeof(FutOptHistoricalClient), "Daily", "GetDailyAsync", "GetDaily"),
        (typeof(FutOptHistoricalClient), "Candles", "GetCandlesAsync", "GetCandles"),
    };

    [TestMethod]
    public void EveryEndpoint_HasThreeNames_WithOneParameterShape()
    {
        foreach (var (client, fubonNeo, async, sync) in Endpoints)
        {
            // GetMethod throws AmbiguousMatchException on overloads, which is
            // the point: one parameter shape per name.
            var a = client.GetMethod(fubonNeo);
            var b = client.GetMethod(async);
            var c = client.GetMethod(sync);
            Assert.IsNotNull(a, $"{client.Name}.{fubonNeo}");
            Assert.IsNotNull(b, $"{client.Name}.{async}");
            Assert.IsNotNull(c, $"{client.Name}.{sync}");

            Assert.AreEqual(typeof(Task<string>), a!.ReturnType, $"{client.Name}.{fubonNeo}");
            Assert.AreEqual(typeof(Task<string>), b!.ReturnType, $"{client.Name}.{async}");
            Assert.AreEqual(typeof(string), c!.ReturnType, $"{client.Name}.{sync}");

            var shape = a.GetParameters().Select(p => p.ParameterType).ToArray();
            CollectionAssert.AreEqual(shape, b.GetParameters().Select(p => p.ParameterType).ToArray(), $"{client.Name}.{async}");
            CollectionAssert.AreEqual(shape, c.GetParameters().Select(p => p.ParameterType).ToArray(), $"{client.Name}.{sync}");

            // No generated params record on the public surface.
            foreach (var p in a.GetParameters())
            {
                Assert.AreNotEqual("uniffi.marketdata_uniffi", p.ParameterType.Namespace, $"{client.Name}.{fubonNeo}({p.Name})");
            }
        }
    }

    [TestMethod]
    public void RequestParameters_AreOptional()
    {
        foreach (var (client, fubonNeo, _, _) in Endpoints)
        {
            var last = client.GetMethod(fubonNeo)!.GetParameters().LastOrDefault();
            if (last is null || !last.Name!.Equals("request", StringComparison.Ordinal))
            {
                continue; // Stats(symbol)
            }
            Assert.IsTrue(last.HasDefaultValue && last.DefaultValue is null, $"{client.Name}.{fubonNeo}(request)");
        }
    }

    [TestMethod]
    public void FubonNeoClientTree_Aliases()
    {
        Assert.IsNotNull(typeof(RestClient).GetProperty("FutureOption", BindingFlags.Public | BindingFlags.Instance));
        Assert.IsNotNull(typeof(StockClient).GetProperty("History", BindingFlags.Public | BindingFlags.Instance));
        Assert.IsNotNull(typeof(StockClient).GetProperty("CorporateActions"));
        Assert.IsNotNull(typeof(StockClient).GetProperty("Ownership"));
    }

    // ========== Q2: the wire (loopback) ==========

    [TestMethod]
    public async Task Tickers_OddLot_Full()
    {
        SkipIfNativeLibraryUnavailable();
        using var server = new LoopbackServer("{}");
        using var client = server.NewClient();

        await client.Stock.Intraday.Tickers(TickersType.OddLot,
            new TickersRequest(MarketType.OTC, ExchangeType.TPEx, "24", isNormal: true, isAttention: true, isDisposition: true, isHalted: false)
            { Symbol = "6488" }).ConfigureAwait(false);

        AssertRequest(server, "/stock/intraday/tickers",
            "type=ODDLOT", "exchange=TPEx", "market=OTC", "industry=24",
            "isNormal=true", "isAttention=false", "isDisposition=false", "isHalted=false", "symbol=6488");
    }

    [TestMethod]
    public async Task Tickers_Default_IsEquityOnly()
    {
        SkipIfNativeLibraryUnavailable();
        using var server = new LoopbackServer("{}");
        using var client = server.NewClient();

        await client.Stock.Intraday.Tickers().ConfigureAwait(false);

        AssertRequest(server, "/stock/intraday/tickers", "type=EQUITY");
    }

    [TestMethod]
    public async Task Trades_OddLot_Offset_Limit()
    {
        SkipIfNativeLibraryUnavailable();
        using var server = new LoopbackServer("{}");
        using var client = server.NewClient();

        await client.Stock.Intraday.Trades("2330", new TradeRequest(TickerType.OddLot, 10, 5) { Sort = SortType.Desc, IsTrial = true }).ConfigureAwait(false);

        AssertRequest(server, "/stock/intraday/trades/2330",
            "type=oddlot", "offset=10", "limit=5", "sort=desc", "isTrial=true");
    }

    [TestMethod]
    public async Task SingleSymbolStockIntraday_OddLotFlag()
    {
        SkipIfNativeLibraryUnavailable();
        using var server = new LoopbackServer("{}");
        using var client = server.NewClient();

        await client.Stock.Intraday.Quote("2330", new QuoteRequest(TickerType.OddLot)).ConfigureAwait(false);
        AssertRequest(server, "/stock/intraday/quote/2330", "type=oddlot");

        await client.Stock.Intraday.Ticker("2330").ConfigureAwait(false);
        AssertRequest(server, "/stock/intraday/ticker/2330");

        await client.Stock.Intraday.Volume("2330", new VolumeRequest(TickerType.OddLot)).ConfigureAwait(false);
        AssertRequest(server, "/stock/intraday/volumes/2330", "type=oddlot");

        await client.Stock.Intraday.Candles("2330", new IntradayCandlesRequest(TickerType.OddLot, IntradayTimeFrame.FiveMin) { Sort = SortType.Asc }).ConfigureAwait(false);
        AssertRequest(server, "/stock/intraday/candles/2330", "type=oddlot", "timeframe=5", "sort=asc");
    }

    [TestMethod]
    public async Task HistoryCandles_Full()
    {
        SkipIfNativeLibraryUnavailable();
        using var server = new LoopbackServer("{}");
        using var client = server.NewClient();

        await client.Stock.History.Candles("2330", new HistoryCandlesRequest(
            new DateTime(2024, 1, 2, 13, 30, 0), new DateTime(2024, 1, 31),
            HistoryTimeFrame.Day, FieldsType.Open | FieldsType.Close, adjusted: true, SortType.Asc)).ConfigureAwait(false);

        AssertRequest(server, "/stock/historical/candles/2330",
            "from=2024-01-02", "to=2024-01-31", "timeframe=D", "fields=open,close", "adjusted=true", "sort=asc");
    }

    [TestMethod]
    public async Task Stats_NoQuery()
    {
        SkipIfNativeLibraryUnavailable();
        using var server = new LoopbackServer("{}");
        using var client = server.NewClient();

        await client.Stock.Historical.Stats("2330").ConfigureAwait(false);

        AssertRequest(server, "/stock/historical/stats/2330");
    }

    [TestMethod]
    public async Task Movers_OTC_Down_Value_Gte_UnderGermanCulture()
    {
        SkipIfNativeLibraryUnavailable();
        using var server = new LoopbackServer("{}");
        using var client = server.NewClient();

        var culture = CultureInfo.CurrentCulture;
        try
        {
            CultureInfo.CurrentCulture = new CultureInfo("de-DE");
            await client.Stock.Snapshot.Movers(MarketType.OTC, DirectionType.Down, ChangeType.Value,
                new MoverRequest(OperationType.GreaterThanOrEqual, 2380.5m) { Type = "COMMONSTOCK" }).ConfigureAwait(false);
        }
        finally
        {
            CultureInfo.CurrentCulture = culture;
        }

        AssertRequest(server, "/stock/snapshot/movers/OTC",
            "direction=down", "change=value", "type=COMMONSTOCK", "gte=2380.5");
    }

    [TestMethod]
    public async Task Quotes_And_Actives()
    {
        SkipIfNativeLibraryUnavailable();
        using var server = new LoopbackServer("{}");
        using var client = server.NewClient();

        await client.Stock.Snapshot.Quotes().ConfigureAwait(false);
        AssertRequest(server, "/stock/snapshot/quotes/TSE");

        await client.Stock.Snapshot.Quotes(MarketType.OTC, new SnapshotRequest("ALLBUT0999")).ConfigureAwait(false);
        AssertRequest(server, "/stock/snapshot/quotes/OTC", "type=ALLBUT0999");

        await client.Stock.Snapshot.Actives(MarketType.TSE, TradeType.Value).ConfigureAwait(false);
        AssertRequest(server, "/stock/snapshot/actives/TSE", "trade=value");

        // FubonNeo's defaults
        await client.Stock.Snapshot.Actives().ConfigureAwait(false);
        AssertRequest(server, "/stock/snapshot/actives/TSE", "trade=volume");

        await client.Stock.Snapshot.Movers().ConfigureAwait(false);
        AssertRequest(server, "/stock/snapshot/movers/TSE", "direction=up", "change=percent");
    }

    [TestMethod]
    public async Task RemainingEndpoints_FullRequest()
    {
        SkipIfNativeLibraryUnavailable();
        using var server = new LoopbackServer("{}");
        using var client = server.NewClient();

        var from = new DateTime(2024, 1, 1);
        var to = new DateTime(2024, 1, 31);
        var range = new[] { "from=2024-01-01", "to=2024-01-31" };

        await client.Stock.Technical.Rsi("2330", new RsiRequest(14, from, to, HistoryTimeFrame.Month)).ConfigureAwait(false);
        AssertRequest(server, "/stock/technical/rsi/2330", range.Concat(new[] { "period=14", "timeframe=M" }).ToArray());

        await client.Stock.Technical.Bb("2330", new BbRequest(20, from, to, HistoryTimeFrame.OneMin)).ConfigureAwait(false);
        AssertRequest(server, "/stock/technical/bb/2330", range.Concat(new[] { "period=20", "timeframe=1" }).ToArray());

        await client.Stock.CorporateActions.ListingApplicants(new CorporateActions.CorporateActionsRequest
        { StartDate = from, EndDate = to, Sort = CorporateActions.SortType.Asc, Exchange = "TPEx" }).ConfigureAwait(false);
        AssertRequest(server, "/stock/corporate-actions/listing-applicants", "start_date=2024-01-01", "end_date=2024-01-31", "sort=asc", "exchange=TPEx");

        var ownership = new OwnershipRequest(from, to, SortType.Asc);
        var ownershipPairs = range.Concat(new[] { "sort=asc" }).ToArray();
        await client.Stock.Ownership.InstitutionalTrades("2330", ownership).ConfigureAwait(false);
        AssertRequest(server, "/stock/ownership/institutional-trades/2330", ownershipPairs);
        await client.Stock.Ownership.DirectorHoldings("2330", ownership).ConfigureAwait(false);
        AssertRequest(server, "/stock/ownership/director-holdings/2330", ownershipPairs);
        await client.Stock.Ownership.TdccDistribution("2330", ownership).ConfigureAwait(false);
        AssertRequest(server, "/stock/ownership/tdcc-distribution/2330", ownershipPairs);
    }

    [TestMethod]
    public async Task Kdj_Full()
    {
        SkipIfNativeLibraryUnavailable();
        using var server = new LoopbackServer("{}");
        using var client = server.NewClient();

        await client.Stock.Technical.Kdj("2330",
            new KdjRequest(9, 3, 3, new DateTime(2024, 1, 1), new DateTime(2024, 3, 1), HistoryTimeFrame.Week)).ConfigureAwait(false);

        AssertRequest(server, "/stock/technical/kdj/2330",
            "rPeriod=9", "kPeriod=3", "dPeriod=3", "from=2024-01-01", "to=2024-03-01", "timeframe=W");
    }

    [TestMethod]
    public async Task Sma_DefaultRequest_SendsPeriodZero()
    {
        SkipIfNativeLibraryUnavailable();
        using var server = new LoopbackServer("{}");
        using var client = server.NewClient();

        // FubonNeo did the same; the server answers 400, the loopback does not care.
        await client.Stock.Technical.Sma("2330").ConfigureAwait(false);
        AssertRequest(server, "/stock/technical/sma/2330", "period=0");

        await client.Stock.Technical.Macd("2330", new MacdRequest(12, 26, 9)).ConfigureAwait(false);
        AssertRequest(server, "/stock/technical/macd/2330", "fast=12", "slow=26", "signal=9");
    }

    [TestMethod]
    public async Task Dividends_Full()
    {
        SkipIfNativeLibraryUnavailable();
        using var server = new LoopbackServer("{}");
        using var client = server.NewClient();

        await client.Stock.CorporateActions.Dividends(new CorporateActions.CorporateActionsRequest
        {
            StartDate = new DateTime(2024, 1, 1),
            EndDate = new DateTime(2024, 12, 31),
            Sort = CorporateActions.SortType.Desc,
            Exchange = "TWSE",
        }).ConfigureAwait(false);

        AssertRequest(server, "/stock/corporate-actions/dividends",
            "start_date=2024-01-01", "end_date=2024-12-31", "sort=desc", "exchange=TWSE");
    }

    [TestMethod]
    public async Task Ownership_EtfHoldings()
    {
        SkipIfNativeLibraryUnavailable();
        using var server = new LoopbackServer("{}");
        using var client = server.NewClient();

        await client.Stock.Ownership.EtfHoldings("0050",
            new EtfHoldingsRequest(new DateTime(2024, 1, 1), new DateTime(2024, 1, 31), SortType.Desc)).ConfigureAwait(false);

        AssertRequest(server, "/stock/ownership/etf-holdings/0050", "from=2024-01-01", "to=2024-01-31", "sort=desc");
    }

    [TestMethod]
    public async Task Products_Option_Full()
    {
        SkipIfNativeLibraryUnavailable();
        using var server = new LoopbackServer("{}");
        using var client = server.NewClient();

        await client.FutureOption.Intraday.Products(FutOptType.Option,
            new FuOptIntraday.ProductsRequest(FutOptExchangeType.TaiFex, SessionType.AfterHours, ContractType.I, FuOptIntraday.ProductStatus.N)).ConfigureAwait(false);

        AssertRequest(server, "/futopt/intraday/products",
            "type=OPTION", "exchange=TAIFEX", "session=AFTERHOURS", "contractType=I", "status=N");
    }

    [TestMethod]
    public async Task Products_Regular_SendsNoSession()
    {
        SkipIfNativeLibraryUnavailable();
        using var server = new LoopbackServer("{}");
        using var client = server.NewClient();

        await client.FutOpt.Intraday.Products(request: new FuOptIntraday.ProductsRequest { Session = SessionType.Regular }).ConfigureAwait(false);

        AssertRequest(server, "/futopt/intraday/products", "type=FUTURE");
    }

    [TestMethod]
    public async Task FutOptTickers_Full()
    {
        SkipIfNativeLibraryUnavailable();
        using var server = new LoopbackServer("{}");
        using var client = server.NewClient();

        await client.FutOpt.Intraday.Tickers(FutOptType.Future,
            new FuOptIntraday.TickersRequest(FutOptExchangeType.TaiFex, SessionType.AfterHours, "TXF", ContractType.I) { IsSpread = false }).ConfigureAwait(false);

        AssertRequest(server, "/futopt/intraday/tickers",
            "type=FUTURE", "exchange=TAIFEX", "session=AFTERHOURS", "product=TXF", "contractType=I", "isSpread=false");
    }

    [TestMethod]
    public async Task FutOptSingleContract_LowerCaseSessionFlag()
    {
        SkipIfNativeLibraryUnavailable();
        using var server = new LoopbackServer("{}");
        using var client = server.NewClient();

        await client.FutOpt.Intraday.Quote("TXFE6", new FuOptIntraday.TickerVolumeRequest(TradeSession.AfterHours)).ConfigureAwait(false);
        AssertRequest(server, "/futopt/intraday/quote/TXFE6", "session=afterhours");

        await client.FutOpt.Intraday.Ticker("TXFE6").ConfigureAwait(false);
        AssertRequest(server, "/futopt/intraday/ticker/TXFE6");

        await client.FutOpt.Intraday.Volumes("TXFE6", new FuOptIntraday.TickerVolumeRequest(TradeSession.AfterHours)).ConfigureAwait(false);
        AssertRequest(server, "/futopt/intraday/volumes/TXFE6", "session=afterhours");

        await client.FutOpt.Intraday.Candles("TXFE6", new FuOptIntraday.CandlesRequest(TradeSession.AfterHours, FuOptIntraday.CandlesTimeFrame.OneMin)).ConfigureAwait(false);
        AssertRequest(server, "/futopt/intraday/candles/TXFE6", "session=afterhours", "timeframe=1");

        await client.FutOpt.Intraday.Trades("TXFE6", new FuOptIntraday.TradesRequest(TradeSession.AfterHours, 0, 50) { IsTrial = true }).ConfigureAwait(false);
        AssertRequest(server, "/futopt/intraday/trades/TXFE6", "session=afterhours", "offset=0", "limit=50", "isTrial=true");
    }

    [TestMethod]
    public async Task HistoricalCandles_Full()
    {
        SkipIfNativeLibraryUnavailable();
        using var server = new LoopbackServer("{}");
        using var client = server.NewClient();

        await client.FutOpt.Historical.Candles("TXO", new HistoricalCandlesRequest(
            new DateTime(2024, 3, 1), new DateTime(2024, 3, 20), HistoricalTimeFrame.Day,
            HistoricalFieldsType.Open | HistoricalFieldsType.Volume, "202403", HistoricalSortType.Desc, TradeSession.AfterHours)
        { StrikePrice = 18000m, CallPut = "CALL" }).ConfigureAwait(false);

        AssertRequest(server, "/futopt/historical/candles/TXO",
            "from=2024-03-01", "to=2024-03-20", "timeframe=D", "fields=open,volume", "contractMonth=202403",
            "sort=desc", "session=afterhours", "strikePrice=18000", "callPut=CALL");
    }

    [TestMethod]
    public async Task Daily_Date_AfterHours()
    {
        SkipIfNativeLibraryUnavailable();
        using var server = new LoopbackServer("{}");
        using var client = server.NewClient();

        await client.FutOpt.Historical.Daily("TXF", new DailyRequest(new DateTime(2024, 1, 2), true)).ConfigureAwait(false);
        AssertRequest(server, "/futopt/historical/daily/TXF", "date=2024-01-02", "session=afterhours");

        // FubonNeo sent afterhours=false, a key the server never read.
        await client.FutOpt.Historical.Daily("TXF", new DailyRequest(new DateTime(2024, 1, 2), false)).ConfigureAwait(false);
        AssertRequest(server, "/futopt/historical/daily/TXF", "date=2024-01-02");
    }

    [TestMethod]
    public void BlockingNames_SendTheSameQuery()
    {
        SkipIfNativeLibraryUnavailable();
        using var server = new LoopbackServer("{}");
        using var client = server.NewClient();

        client.Stock.Intraday.GetTrades("2330", new TradeRequest(TickerType.OddLot, limit: 5));
        AssertRequest(server, "/stock/intraday/trades/2330", "type=oddlot", "limit=5");

        client.Stock.Snapshot.GetMovers(MarketType.TSE, DirectionType.Up, ChangeType.Percent);
        AssertRequest(server, "/stock/snapshot/movers/TSE", "direction=up", "change=percent");

        client.FutOpt.Historical.GetDaily("TXF");
        AssertRequest(server, "/futopt/historical/daily/TXF");
    }

    // ========== Q4 (client part): stricter than FubonNeo, before any request ==========

    [TestMethod]
    public async Task CapitalChanges_WithExchange_IsInvalidParameter1005()
    {
        SkipIfNativeLibraryUnavailable();
        using var server = new LoopbackServer("{}");
        using var client = server.NewClient();

        var ex = await Assert.ThrowsExceptionAsync<uniffi.marketdata_uniffi.MarketDataException.ApiException>(
            () => client.Stock.CorporateActions.CapitalChanges(new CorporateActions.CorporateActionsRequest { Exchange = "TWSE" }));

        Assert.AreEqual(1005, ex.GetInfo().code);
        Assert.IsFalse(server.Requests.Any());

        // Without Exchange the same request type is fine.
        await client.Stock.CorporateActions.CapitalChanges(new CorporateActions.CorporateActionsRequest { StartDate = new DateTime(2024, 1, 1) }).ConfigureAwait(false);
        AssertRequest(server, "/stock/corporate-actions/capital-changes", "start_date=2024-01-01");
    }

    [TestMethod]
    public void NegativeCounts_ThroughTheClient_ThrowBeforeAnyRequest()
    {
        SkipIfNativeLibraryUnavailable();
        using var server = new LoopbackServer("{}");
        using var client = server.NewClient();

        Assert.ThrowsException<ArgumentOutOfRangeException>(() => client.Stock.Intraday.Trades("2330", new TradeRequest(offset: -1)));
        Assert.ThrowsException<ArgumentOutOfRangeException>(() => client.Stock.Technical.GetSma("2330", new SmaRequest(-1)));
        Assert.ThrowsException<ArgumentOutOfRangeException>(() => client.FutOpt.Intraday.GetTrades("TXFE6", new FuOptIntraday.TradesRequest(limit: -1)));
        Assert.IsFalse(server.Requests.Any());
    }
}
