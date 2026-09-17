using Microsoft.VisualStudio.TestTools.UnitTesting;
using System;
using System.Collections.Generic;
using System.Threading.Tasks;

namespace MarketdataUniffi.Tests;

/// <summary>
/// The auth frame carries each credential in its own field (#91): the server
/// reads <c>apikey</c>, <c>token</c> or <c>sdkToken</c> and rejects a frame
/// with more than one. Bearer and SDK tokens used to be refused outright.
/// </summary>
[TestClass]
public class WebSocketAuthFrameTests
{
    public static IEnumerable<object[]> Credentials => new[]
    {
        new object[] { new FugleMarketData.WebSocketClientOptions { ApiKey = "the-key" }, "{\"apikey\":\"the-key\"}" },
        new object[] { new FugleMarketData.WebSocketClientOptions { BearerToken = "the-token" }, "{\"token\":\"the-token\"}" },
        new object[] { new FugleMarketData.WebSocketClientOptions { SdkToken = "the-sdk-token" }, "{\"sdkToken\":\"the-sdk-token\"}" },
    };

    [TestMethod]
    [DynamicData(nameof(Credentials))]
    public async Task Connect_SendsCredentialInItsField(FugleMarketData.WebSocketClientOptions options, string expected)
    {
        using var server = new WebSocketLoopbackServer();
        options.BaseUrl = server.Url;

        using var client = new FugleMarketData.WebSocketClient(options, new TestWebSocketListener());
        await client.ConnectAsync().WaitAsync(TimeSpan.FromSeconds(10));
        await client.DisconnectAsync().WaitAsync(TimeSpan.FromSeconds(10));

        CollectionAssert.AreEqual(new[] { expected }, server.AuthData.ToArray());
    }

    [TestMethod]
    public void Options_WithoutCredential_ThrowsUnwrappedConfigError()
    {
        // The options constructor used to wrap core errors in an
        // InvalidOperationException; the 1004 ConfigError must reach the caller
        // as is, also with every other option set.
        var options = new FugleMarketData.WebSocketClientOptions
        {
            BearerToken = "  ",
            BaseUrl = "ws://127.0.0.1:1",
            Reconnect = new FugleMarketData.ReconnectOptions { MaxAttempts = 1 },
            HealthCheck = new FugleMarketData.HealthCheckOptions { Enabled = false },
            MessageBuffer = 16,
        };

        var ex = Assert.ThrowsException<uniffi.marketdata_uniffi.MarketDataException.ConfigException>(
            () => new FugleMarketData.WebSocketClient(options, new TestWebSocketListener()));
        Assert.AreEqual(1004, FugleMarketData.MarketDataExceptionExtensions.GetInfo(ex).code);
    }
}
