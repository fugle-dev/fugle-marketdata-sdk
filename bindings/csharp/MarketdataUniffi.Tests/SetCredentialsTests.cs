using Microsoft.VisualStudio.TestTools.UnitTesting;
using System;
using System.Diagnostics;
using System.Linq;
using System.Threading;
using System.Threading.Tasks;

namespace MarketdataUniffi.Tests;

/// <summary>
/// <c>SetCredentials</c> changes what later connection attempts and requests
/// send (#322). The loopback server accepts only the new credential once the
/// test asks it to: the automatic reconnect after a drop authenticates with
/// it, in the field of its kind, and the live connection gets no new auth
/// frame. After a rejected connect, setting it and connecting again succeeds.
/// A REST client sends it from its next request on, through clients taken
/// before. Anything but exactly one non-empty credential is 1004 and keeps
/// the current one.
/// </summary>
[TestClass]
public class SetCredentialsTests
{
    private const string Old = "old-key";
    private const string New = "new-token";

    private sealed class CountingListener : FugleMarketData.IWebSocketListener
    {
        private int _authenticated;
        public int Authenticated => Volatile.Read(ref _authenticated);
        public void OnConnected() { }
        public void OnAuthenticated(string? dataJson) => Interlocked.Increment(ref _authenticated);
        public void OnUnauthenticated(string? dataJson) { }
        public void OnDisconnected(bool willReconnect) { }
        public void OnMessage(uniffi.marketdata_uniffi.StreamMessage message) { }
        public void OnError(uniffi.marketdata_uniffi.ErrorInfo error) { }
        public void OnReconnecting(uint attempt) { }
        public void OnReconnectFailed(uint attempts) { }
        public void OnMessagesDropped(ulong count) { }
    }

    private static FugleMarketData.WebSocketClient NewClient(WebSocketLoopbackServer server, FugleMarketData.IWebSocketListener listener) =>
        new(new FugleMarketData.WebSocketClientOptions
        {
            ApiKey = Old,
            BaseUrl = server.Url,
            Reconnect = new FugleMarketData.ReconnectOptions { MaxAttempts = 2, InitialDelayMs = 100, MaxDelayMs = 100 },
        }, listener);

    private static async Task WaitUntil(Func<bool> condition, string message)
    {
        var watch = Stopwatch.StartNew();
        while (!condition())
        {
            if (watch.Elapsed > TimeSpan.FromSeconds(5)) Assert.Fail(message);
            await Task.Delay(10);
        }
    }

    private static void AssertConfigError(Action action)
    {
        var ex = Assert.ThrowsException<uniffi.marketdata_uniffi.MarketDataException.ConfigException>(action);
        Assert.AreEqual(1004, FugleMarketData.MarketDataExceptionExtensions.GetInfo(ex).code);
    }

    [TestMethod]
    public async Task Reconnect_SendsTheNewCredentialOfAnotherKind_AndTheLiveConnectionIsLeftAlone()
    {
        using var server = new WebSocketLoopbackServer();
        var listener = new CountingListener();
        using var client = NewClient(server, listener);
        await client.ConnectAsync().WaitAsync(TimeSpan.FromSeconds(10));

        server.RequiredCredential = New;
        client.SetCredentials(sdkToken: New);
        await Task.Delay(100);
        Assert.AreEqual(1, server.AuthData.Count, "no auth frame on the live connection");

        server.DropConnections();
        await WaitUntil(() => listener.Authenticated == 2, "the reconnect never authenticated");
        await client.DisconnectAsync().WaitAsync(TimeSpan.FromSeconds(10));

        CollectionAssert.AreEqual(
            new[] { "{\"apikey\":\"old-key\"}", "{\"sdkToken\":\"new-token\"}" },
            server.AuthData.ToArray());
    }

    [TestMethod]
    public async Task RejectedConnect_SucceedsAfterTheCredentialIsSet()
    {
        using var server = new WebSocketLoopbackServer { RequiredCredential = New };
        using var client = NewClient(server, new CountingListener());
        await AssertEx.ThrowsAnyAsync(() => client.ConnectAsync().WaitAsync(TimeSpan.FromSeconds(10)));

        client.SetCredentials(bearerToken: New);
        await client.ConnectAsync().WaitAsync(TimeSpan.FromSeconds(10));
        await client.DisconnectAsync().WaitAsync(TimeSpan.FromSeconds(10));

        CollectionAssert.AreEqual(
            new[] { "{\"apikey\":\"old-key\"}", "{\"token\":\"new-token\"}" },
            server.AuthData.ToArray());
    }

    [TestMethod]
    public async Task InvalidCredentials_AreConfigError_AndKeepTheCurrentOne()
    {
        using var server = new WebSocketLoopbackServer();
        using var client = NewClient(server, new CountingListener());

        AssertConfigError(() => client.SetCredentials());
        AssertConfigError(() => client.SetCredentials(apiKey: "a", sdkToken: "b"));
        AssertConfigError(() => client.SetCredentials(bearerToken: "   "));
        await client.ConnectAsync().WaitAsync(TimeSpan.FromSeconds(10));
        await client.DisconnectAsync().WaitAsync(TimeSpan.FromSeconds(10));

        CollectionAssert.AreEqual(new[] { "{\"apikey\":\"old-key\"}" }, server.AuthData.ToArray());
    }

    [TestMethod]
    public async Task Factory_ReachesBuiltClientsAndOnesNotBuiltYet()
    {
        using var server = new WebSocketLoopbackServer();
        using var factory = FugleMarketData.WebsocketClient.FugleWebsocketClientFactory.Create(Old, baseUrl: server.Url);
        var stock = factory.Stock;

        AssertConfigError(() => factory.SetCredentials(apiKey: "a", sdkToken: "b"));
        factory.SetCredentials(bearerToken: New);
        var futOpt = factory.FutureOption;
        await stock.Connect().WaitAsync(TimeSpan.FromSeconds(10));
        await futOpt.Connect().WaitAsync(TimeSpan.FromSeconds(10));
        await stock.Disconnect().WaitAsync(TimeSpan.FromSeconds(10));
        await futOpt.Disconnect().WaitAsync(TimeSpan.FromSeconds(10));

        CollectionAssert.AreEqual(
            new[] { "{\"token\":\"new-token\"}", "{\"token\":\"new-token\"}" },
            server.AuthData.ToArray());
    }

    [TestMethod]
    public async Task Rest_SendsTheNewCredential_ThroughClientsTakenBefore()
    {
        using var server = new LoopbackServer("{}");
        var client = server.NewClient(Old);
        var intraday = client.Stock.Intraday;

        client.SetCredentials(sdkToken: New);
        await intraday.Quote("2330");
        await client.Stock.Intraday.Quote("2330");

        var headers = server.Headers.ToArray();
        Assert.AreEqual(2, headers.Length);
        foreach (var h in headers)
        {
            Assert.AreEqual(New, h["X-SDK-TOKEN"]);
            Assert.IsNull(h["X-API-KEY"]);
        }
    }

    [TestMethod]
    public async Task Rest_InvalidCredentials_AreConfigError_AndKeepTheCurrentOne()
    {
        using var server = new LoopbackServer("{}");
        var client = server.NewClient(Old);

        AssertConfigError(() => client.SetCredentials());
        AssertConfigError(() => client.SetCredentials(apiKey: "a", bearerToken: "b"));
        AssertConfigError(() => client.SetCredentials(apiKey: "bad\nkey"));
        await client.Stock.Intraday.Quote("2330");

        Assert.AreEqual(Old, server.Headers.Single()["X-API-KEY"]);
    }
}
