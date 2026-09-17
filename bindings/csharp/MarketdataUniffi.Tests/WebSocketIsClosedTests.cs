using Microsoft.VisualStudio.TestTools.UnitTesting;
using System;
using System.Diagnostics;
using System.Threading.Tasks;

namespace MarketdataUniffi.Tests;

/// <summary>
/// <c>IsClosed</c> reads core's connection state (#95): it used to read a flag
/// only <c>DisconnectAsync</c> set, so it stayed false after the server closed
/// the connection with no reconnect to follow.
/// </summary>
[TestClass]
public class WebSocketIsClosedTests
{
    [TestMethod]
    public async Task IsClosed_AfterServerCloseWithoutReconnect_IsTrue()
    {
        using var server = new WebSocketLoopbackServer();
        using var client = new FugleMarketData.WebSocketClient(
            new FugleMarketData.WebSocketClientOptions { ApiKey = "the-key", BaseUrl = server.Url },
            new TestWebSocketListener());

        await client.ConnectAsync().WaitAsync(TimeSpan.FromSeconds(10));
        Assert.IsFalse(client.IsClosed);

        server.DropConnections();
        await WaitUntil(() => client.IsClosed, "IsClosed never became true");
        Assert.IsFalse(client.IsConnected);
    }

    [TestMethod]
    public async Task IsClosed_WhileReconnecting_IsFalse_AndAfterDisconnect_IsTrue()
    {
        using var server = new WebSocketLoopbackServer();
        using var client = new FugleMarketData.WebSocketClient(
            new FugleMarketData.WebSocketClientOptions
            {
                ApiKey = "the-key",
                BaseUrl = server.Url,
                Reconnect = new FugleMarketData.ReconnectOptions { MaxAttempts = 3, InitialDelayMs = 2000, MaxDelayMs = 2000 },
            },
            new TestWebSocketListener());

        await client.ConnectAsync().WaitAsync(TimeSpan.FromSeconds(10));
        server.DropConnections();
        await WaitUntil(() => !client.IsConnected, "IsConnected never became false");
        Assert.IsFalse(client.IsClosed, "false while reconnecting");

        await client.DisconnectAsync().WaitAsync(TimeSpan.FromSeconds(10));
        Assert.IsTrue(client.IsClosed, "true once DisconnectAsync completes");
    }

    private static async Task WaitUntil(Func<bool> condition, string message)
    {
        var watch = Stopwatch.StartNew();
        while (!condition())
        {
            if (watch.Elapsed > TimeSpan.FromSeconds(5)) Assert.Fail(message);
            await Task.Delay(10);
        }
    }
}
