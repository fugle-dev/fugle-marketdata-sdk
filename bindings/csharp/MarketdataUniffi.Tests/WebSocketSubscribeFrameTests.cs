using Microsoft.VisualStudio.TestTools.UnitTesting;
using System;
using System.Threading.Tasks;

namespace MarketdataUniffi.Tests;

/// <summary>
/// Subscribe and unsubscribe follow the client's endpoint (#123): the FutOpt
/// endpoint takes FutOpt channels and the after-hours session; the Stock
/// endpoint rejects after-hours with 1005.
/// </summary>
[TestClass]
public class WebSocketSubscribeFrameTests
{
    [TestMethod]
    public async Task FutOptEndpoint_SendsAfterHours()
    {
        using var server = new WebSocketLoopbackServer();
        var options = new FugleMarketData.WebSocketClientOptions
        {
            ApiKey = "the-key",
            BaseUrl = server.Url,
            Endpoint = FugleMarketData.WebSocketEndpoint.FutOpt,
        };
        using var client = new FugleMarketData.WebSocketClient(options, new TestWebSocketListener());
        await client.ConnectAsync().WaitAsync(TimeSpan.FromSeconds(10));

        await AssertInvalidParameter(() => client.SubscribeAsync("indices", "TXFE6"));
        await client.SubscribeAsync("books", "TXFE6", afterHours: true).WaitAsync(TimeSpan.FromSeconds(10));
        await client.SubscribeAsync("trades", "TXFE6").WaitAsync(TimeSpan.FromSeconds(10));
        await client.UnsubscribeAsync("books", "TXFE6", afterHours: true).WaitAsync(TimeSpan.FromSeconds(10));

        var expected = new[]
        {
            "{\"event\":\"subscribe\",\"data\":{\"channel\":\"books\",\"symbol\":\"TXFE6\",\"afterHours\":true}}",
            "{\"event\":\"subscribe\",\"data\":{\"channel\":\"trades\",\"symbol\":\"TXFE6\"}}",
            "{\"event\":\"unsubscribe\",\"data\":{\"id\":\"books:TXFE6:afterhours\"}}",
        };
        var deadline = DateTime.UtcNow.AddSeconds(10);
        while (server.OtherFrames.Count < expected.Length && DateTime.UtcNow < deadline)
        {
            await Task.Delay(20);
        }
        await client.DisconnectAsync().WaitAsync(TimeSpan.FromSeconds(10));

        CollectionAssert.AreEqual(expected, server.OtherFrames.ToArray());
    }

    [TestMethod]
    [DataRow(true)]
    [DataRow(false)]
    public async Task StockEndpoint_RejectsAfterHours(bool afterHours)
    {
        using var client = new FugleMarketData.WebSocketClient("the-key", new TestWebSocketListener());

        await AssertInvalidParameter(() => client.SubscribeAsync("trades", "2330", afterHours));
        await AssertInvalidParameter(() => client.UnsubscribeAsync("trades", "2330", afterHours));
    }

    private static async Task AssertInvalidParameter(Func<Task> call)
    {
        var ex = await Assert.ThrowsExceptionAsync<uniffi.marketdata_uniffi.MarketDataException.ApiException>(call);
        Assert.AreEqual(1005, FugleMarketData.MarketDataExceptionExtensions.GetInfo(ex).code);
    }
}
