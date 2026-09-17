using Microsoft.VisualStudio.TestTools.UnitTesting;
using System;
using System.Threading.Tasks;

namespace MarketdataUniffi.Tests;

/// <summary>
/// <c>ConnectAsync</c> while connected is refused with code 2011 and leaves the
/// connection up (#119); it used to open a second connection.
/// </summary>
[TestClass]
public class WebSocketAlreadyConnectedTests
{
    [TestMethod]
    public async Task ConnectAsync_WhileConnected_IsRefusedWith2011()
    {
        using var server = new WebSocketLoopbackServer();
        using var client = new FugleMarketData.WebSocketClient(
            new FugleMarketData.WebSocketClientOptions { ApiKey = "the-key", BaseUrl = server.Url },
            new TestWebSocketListener());

        await client.ConnectAsync().WaitAsync(TimeSpan.FromSeconds(10));

        var error = await Assert.ThrowsExceptionAsync<uniffi.marketdata_uniffi.MarketDataException.WebSocketException>(
            () => client.ConnectAsync().WaitAsync(TimeSpan.FromSeconds(10)));
        Assert.AreEqual(2011, FugleMarketData.MarketDataExceptionExtensions.GetInfo(error).code);
        Assert.IsTrue(client.IsConnected, "the first connection stays up");

        await client.DisconnectAsync().WaitAsync(TimeSpan.FromSeconds(10));
        await client.ConnectAsync().WaitAsync(TimeSpan.FromSeconds(10));
        Assert.IsTrue(client.IsConnected, "ConnectAsync after DisconnectAsync is allowed");
    }
}
