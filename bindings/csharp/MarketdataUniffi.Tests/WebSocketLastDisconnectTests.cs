using Microsoft.VisualStudio.TestTools.UnitTesting;
using System;
using System.Diagnostics;
using System.Net.WebSockets;
using System.Threading.Tasks;
using uniffi.marketdata_uniffi;

namespace MarketdataUniffi.Tests;

/// <summary>
/// <c>LastDisconnect</c> says who closed the connection and whether a
/// reconnect follows (#293); <c>OnDisconnected(bool)</c> is unchanged.
/// </summary>
[TestClass]
public class WebSocketLastDisconnectTests
{
    private static FugleMarketData.WebSocketClient NewClient(WebSocketLoopbackServer server) =>
        new(
            new FugleMarketData.WebSocketClientOptions
            {
                ApiKey = "the-key",
                BaseUrl = server.Url,
                Reconnect = new FugleMarketData.ReconnectOptions { Enabled = false },
            },
            new TestWebSocketListener());

    [TestMethod]
    public async Task LastDisconnect_AfterDisconnectAsync_IsClient()
    {
        using var server = new WebSocketLoopbackServer();
        using var client = NewClient(server);
        Assert.IsNull(client.LastDisconnect, "null before connecting");

        await client.ConnectAsync().WaitAsync(TimeSpan.FromSeconds(10));
        await client.DisconnectAsync().WaitAsync(TimeSpan.FromSeconds(10));

        Assert.AreEqual(new DisconnectInfo(1000, "Normal closure", DisconnectIntent.Client, false), client.LastDisconnect);
    }

    [TestMethod]
    public async Task LastDisconnect_AfterServerClose_IsServer()
    {
        using var server = new WebSocketLoopbackServer();
        using var client = NewClient(server);

        await client.ConnectAsync().WaitAsync(TimeSpan.FromSeconds(10));
        await server.CloseConnections(WebSocketCloseStatus.EndpointUnavailable, "going away");
        await WaitUntil(() => client.LastDisconnect != null, "LastDisconnect never set");

        Assert.AreEqual(new DisconnectInfo(1001, "going away", DisconnectIntent.Server, false), client.LastDisconnect);
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
