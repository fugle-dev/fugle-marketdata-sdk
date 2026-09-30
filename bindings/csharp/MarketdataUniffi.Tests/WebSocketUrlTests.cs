using Microsoft.VisualStudio.TestTools.UnitTesting;

namespace MarketdataUniffi.Tests;

/// <summary>
/// <c>Url</c> is the endpoint core resolved from the base URL, the version
/// and the product, readable before connecting (#245).
/// </summary>
[TestClass]
public class WebSocketUrlTests
{
    private static FugleMarketData.WebSocketClient Client(FugleMarketData.WebSocketClientOptions options)
    {
        options.ApiKey = "the-key";
        return new FugleMarketData.WebSocketClient(options, new TestWebSocketListener());
    }

    [TestMethod]
    public void Url_DefaultsToTheProductionEndpointOfEachProduct()
    {
        using var stock = Client(new FugleMarketData.WebSocketClientOptions());
        Assert.AreEqual("wss://api.fugle.tw/marketdata/v1.0/stock/streaming", stock.Url);

        using var futopt = Client(new FugleMarketData.WebSocketClientOptions
        {
            Endpoint = FugleMarketData.WebSocketEndpoint.FutOpt,
        });
        Assert.AreEqual("wss://api.fugle.tw/marketdata/v1.1/futopt/streaming", futopt.Url);
    }

    [TestMethod]
    public void Url_ReflectsBaseUrlAndVersion()
    {
        using var client = Client(new FugleMarketData.WebSocketClientOptions
        {
            Endpoint = FugleMarketData.WebSocketEndpoint.FutOpt,
            BaseUrl = "wss://staging.fugle.tw/marketdata",
            Versions = new FugleMarketData.WebsocketClient.WebsocketVersionOptions { FutOpt = "v1.0" },
        });
        Assert.AreEqual("wss://staging.fugle.tw/marketdata/v1.0/futopt/streaming", client.Url);
    }

    [TestMethod]
    public void Url_WithVersionedBaseUrl_ThrowsConfigError()
    {
        using var client = Client(new FugleMarketData.WebSocketClientOptions
        {
            BaseUrl = "wss://staging.fugle.tw/marketdata/v1.0",
        });
        var ex = Assert.ThrowsException<uniffi.marketdata_uniffi.MarketDataException.ConfigException>(() => client.Url);
        Assert.AreEqual(1004, FugleMarketData.MarketDataExceptionExtensions.GetInfo(ex).code);
    }
}
