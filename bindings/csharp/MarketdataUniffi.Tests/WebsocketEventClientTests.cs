using Microsoft.VisualStudio.TestTools.UnitTesting;
using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.Diagnostics;
using System.Linq;
using System.Threading.Tasks;
using FugleMarketData;
using FugleMarketData.WebsocketClient;
using FugleMarketData.WebsocketModels;

namespace MarketdataUniffi.Tests;

/// <summary>
/// The FubonNeo-shaped event clients (#204): channel enums and params
/// objects map onto the same subscribe frames as <c>WebSocketClient</c>, and
/// the listener callbacks map onto string events in the documented order.
/// </summary>
[TestClass]
public class WebsocketEventClientTests
{
    private static readonly TimeSpan Timeout = TimeSpan.FromSeconds(10);

    // ========== Frames (W1, W2, W6) ==========

    [TestMethod]
    public async Task Stock_Subscribe_SymbolsArray_And_OddLotParams()
    {
        using var server = new WebSocketLoopbackServer { AckSubscribes = true };
        using var client = new FugleWebsocketStockClient(Options(server));
        await client.Connect().WaitAsync(Timeout);

        await client.Subscribe(StockChannel.Trades, "2330", "2317").WaitAsync(Timeout);
        await client.Subscribe(StockChannel.Trades, new StockSubscribeParams { Symbol = "2330", IntradayOddLot = true }).WaitAsync(Timeout);
        await client.Subscribe(StockChannel.Books, "2330").WaitAsync(Timeout);
        await client.Subscribe(StockChannel.Indices, new StockSubscribeParams { Symbol = "IX0001", Symbols = new[] { "IX0002" } }).WaitAsync(Timeout);

        await AssertFrames(server, new[]
        {
            "{\"event\":\"subscribe\",\"data\":{\"channel\":\"trades\",\"symbols\":[\"2330\",\"2317\"]}}",
            "{\"event\":\"subscribe\",\"data\":{\"channel\":\"trades\",\"symbol\":\"2330\",\"intradayOddLot\":true}}",
            "{\"event\":\"subscribe\",\"data\":{\"channel\":\"books\",\"symbol\":\"2330\"}}",
            "{\"event\":\"subscribe\",\"data\":{\"channel\":\"indices\",\"symbols\":[\"IX0001\",\"IX0002\"]}}",
        });
        await client.Disconnect().WaitAsync(Timeout);
    }

    [TestMethod]
    public async Task FutOpt_Subscribe_AfterHoursParams()
    {
        using var server = new WebSocketLoopbackServer { AckSubscribes = true };
        using var client = new FugleWebsocketFutOptClient(Options(server, WebSocketEndpoint.FutOpt));
        await client.Connect().WaitAsync(Timeout);

        await client.Subscribe(FutureOptionChannel.Books, new FutureOptionParams { Symbols = new[] { "TXFE6", "TXFF6" }, AfterHours = true }).WaitAsync(Timeout);
        await client.Subscribe(FutureOptionChannel.Trades, "TXFE6").WaitAsync(Timeout);

        await AssertFrames(server, new[]
        {
            "{\"event\":\"subscribe\",\"data\":{\"channel\":\"books\",\"symbols\":[\"TXFE6\",\"TXFF6\"],\"afterHours\":true}}",
            "{\"event\":\"subscribe\",\"data\":{\"channel\":\"trades\",\"symbol\":\"TXFE6\"}}",
        });
        await client.Disconnect().WaitAsync(Timeout);
    }

    [TestMethod]
    public async Task Subscribe_NoSymbols_IsInvalidParameter()
    {
        using var client = new FugleWebsocketStockClient(new WebSocketClientOptions { ApiKey = "the-key" });
        var ex = await Assert.ThrowsExceptionAsync<uniffi.marketdata_uniffi.MarketDataException.ApiException>(
            () => client.Subscribe(StockChannel.Trades, new StockSubscribeParams()));
        Assert.AreEqual(1005, ex.GetInfo().code);
    }

    [TestMethod]
    public async Task Unsubscribe_Params_MergesIdAndIds()
    {
        using var server = new WebSocketLoopbackServer();
        using var client = new FugleWebsocketStockClient(Options(server));
        await client.Connect().WaitAsync(Timeout);

        await client.Unsubscribe(new UnsubscribeParams { ChannelId = "a", ChannelIds = new[] { "b" } }).WaitAsync(Timeout);
        await client.Unsubscribe("c").WaitAsync(Timeout);
        await client.Unsubscribe("d", "e").WaitAsync(Timeout);

        await AssertFrames(server, new[]
        {
            "{\"event\":\"unsubscribe\",\"data\":{\"ids\":[\"a\",\"b\"]}}",
            "{\"event\":\"unsubscribe\",\"data\":{\"id\":\"c\"}}",
            "{\"event\":\"unsubscribe\",\"data\":{\"ids\":[\"d\",\"e\"]}}",
        });
        await client.Disconnect().WaitAsync(Timeout);
    }

    [TestMethod]
    public void Endpoint_MustMatchClient()
    {
        Assert.ThrowsException<ArgumentException>(() =>
            new FugleWebsocketFutOptClient(new WebSocketClientOptions { ApiKey = "the-key" }));
        Assert.ThrowsException<ArgumentException>(() =>
            new FugleWebsocketStockClient(new WebSocketClientOptions { ApiKey = "the-key", Endpoint = WebSocketEndpoint.FutOpt }));
    }

    // ========== Events (W3, W4, W5) ==========

    [TestMethod]
    public async Task Events_Connect_ServerDrop_Disconnect()
    {
        using var server = new WebSocketLoopbackServer();
        var options = Options(server);
        options.Reconnect = new ReconnectOptions { Enabled = false };
        using var client = new FugleWebsocketStockClient(options);
        var events = Record(client);

        await client.Connect().WaitAsync(Timeout);
        await WaitFor(events, 1);
        CollectionAssert.AreEqual(new[] { "connected:Connected" }, events.ToArray());

        // Core reports the transport failure (OnError → OnException) before
        // the close, as FubonNeo's receive loop did.
        server.DropConnections();
        await WaitFor(events, 4);
        var actual = events.ToArray();
        StringAssert.StartsWith(actual[1], "exception:", string.Join(" | ", actual));
        CollectionAssert.AreEqual(
            new[] { "connected:Connected", "close:Received close message", "disconnected:Server Disconnected" },
            actual.Where(e => !e.StartsWith("exception:")).ToArray(), string.Join(" | ", actual));
    }

    [TestMethod]
    public async Task Events_Disconnect_CarriesMessage_NoClose()
    {
        using var server = new WebSocketLoopbackServer();
        using var client = new FugleWebsocketStockClient(Options(server));
        var events = Record(client);

        await client.Connect().WaitAsync(Timeout);
        await client.Disconnect("bye").WaitAsync(Timeout);

        CollectionAssert.AreEqual(new[] { "connected:Connected", "disconnected:bye" }, events.ToArray());
    }

    [TestMethod]
    public async Task Events_Message_Raw_And_ErrorFrame()
    {
        using var server = new WebSocketLoopbackServer();
        using var client = new FugleWebsocketStockClient(Options(server));
        var events = Record(client);
        await client.Connect().WaitAsync(Timeout);

        const string data = "{\"event\":\"data\",\"data\":{\"symbol\":\"2330\",\"price\":1000}}";
        const string error = "{\"event\":\"error\",\"data\":{\"message\":\"Unknown channel\"}}";
        await server.SendToAll(data);
        await server.SendToAll(error);
        await WaitFor(events, 4);

        CollectionAssert.AreEqual(
            new[] { "connected:Connected", "message:" + data, "error:" + error, "message:" + error },
            events.ToArray());
        await client.Disconnect().WaitAsync(Timeout);
    }

    [TestMethod]
    public async Task Events_ThrowingHandler_DoesNotStopStream()
    {
        using var server = new WebSocketLoopbackServer();
        using var client = new FugleWebsocketStockClient(Options(server));
        var messages = new ConcurrentQueue<string>();
        var exceptions = new ConcurrentQueue<Exception>();
        var first = true;
        client.OnMessage = raw =>
        {
            if (raw.Contains("\"event\":\"authenticated\""))
                return;
            messages.Enqueue(raw);
            if (first)
            {
                first = false;
                throw new InvalidOperationException("boom");
            }
        };
        client.OnException = exceptions.Enqueue;
        await client.Connect().WaitAsync(Timeout);

        await server.SendToAll("{\"event\":\"data\",\"data\":{\"n\":1}}");
        await server.SendToAll("{\"event\":\"data\",\"data\":{\"n\":2}}");
        await WaitFor(messages, 2);

        Assert.AreEqual(2, messages.Count);
        await WaitFor(exceptions, 1);
        var ex = exceptions.Single() as MarketDataStreamException;
        Assert.IsNotNull(ex);
        Assert.AreEqual(WebSocketListenerAdapter.CallbackFailedCode, ex!.Info.code);
        StringAssert.Contains(ex.Message, "boom");
        await client.Disconnect().WaitAsync(Timeout);
    }

    [TestMethod]
    public async Task Events_RejectedAuth_ErrorThenException()
    {
        using var server = new WebSocketLoopbackServer { RejectAuth = true };
        using var client = new FugleWebsocketStockClient(Options(server));
        var events = Record(client);
        var exceptions = new ConcurrentQueue<Exception>();
        client.OnException += exceptions.Enqueue;

        await Assert.ThrowsExceptionAsync<uniffi.marketdata_uniffi.MarketDataException.AuthException>(
            () => client.Connect().WaitAsync(Timeout));
        await WaitFor(events, 3);

        CollectionAssert.AreEqual(
            new[] { "connected:Connected", "error:{\"message\":\"Invalid token\"}", "exception:Authenticate Failed!" },
            events.ToArray(), string.Join(" | ", events));
        var ex = exceptions.Single() as MarketDataStreamException;
        Assert.IsNotNull(ex);
        Assert.AreEqual(2002, ex!.Info.code);
        Assert.AreEqual(uniffi.marketdata_uniffi.ErrorSourceKind.Auth, ex.Info.sourceKind);
    }

    [TestMethod]
    public async Task Events_ThrowingOnClose_StillRaisesDisconnected()
    {
        using var server = new WebSocketLoopbackServer();
        var options = Options(server);
        options.Reconnect = new ReconnectOptions { Enabled = false };
        using var client = new FugleWebsocketStockClient(options);
        var events = Record(client);
        client.OnClose = _ => throw new InvalidOperationException("close boom");
        await client.Connect().WaitAsync(Timeout);

        server.DropConnections();
        await WaitFor(events, 4);

        var actual = events.ToArray();
        Assert.IsTrue(actual.Contains("disconnected:Server Disconnected"), string.Join(" | ", actual));
        Assert.IsTrue(actual.Any(e => e.StartsWith("exception:") && e.Contains("close boom")), string.Join(" | ", actual));
    }

    [TestMethod]
    public void Events_PlusEquals_AddsToDefault()
    {
        using var client = new FugleWebsocketStockClient(new WebSocketClientOptions { ApiKey = "the-key" });
        var seen = new List<string>();
        client.OnMessage += seen.Add;
        client.OnMessage += raw => seen.Add(raw + "!");
        client.OnMessage("x");
        CollectionAssert.AreEqual(new[] { "x", "x!" }, seen);
    }

    // ========== Factory ==========

    [TestMethod]
    public void Factory_LazyClients_And_BlankTokenIsConfigError()
    {
        using var factory = FugleWebsocketClientFactory.Create("tok");
        Assert.AreSame(factory.Stock, factory.Stock);
        Assert.AreSame(factory.FutureOption, factory.FutureOption);
        Assert.IsFalse(factory.Stock.IsConnected);

        var ex = Assert.ThrowsException<uniffi.marketdata_uniffi.MarketDataException.ConfigException>(
            () => FugleWebsocketClientFactory.Create(" "));
        Assert.AreEqual(1004, ex.GetInfo().code);

        factory.Dispose();
        Assert.ThrowsException<ObjectDisposedException>(() => factory.FutureOption);
    }

    [TestMethod]
    public async Task Factory_SendsSdkToken_ApiKey_AndVersion()
    {
        using var server = new WebSocketLoopbackServer();
        using var sdk = FugleWebsocketClientFactory.Create("tok", null, server.Url);
        await sdk.Stock.Connect().WaitAsync(Timeout);
        await sdk.Stock.Disconnect().WaitAsync(Timeout);

        using var key = FugleWebsocketClientFactory.CreateWithApiKey("key", new WebsocketVersionOptions { FutOpt = "v1.0" }, server.Url);
        await key.FutureOption.Connect().WaitAsync(Timeout);
        await key.FutureOption.Disconnect().WaitAsync(Timeout);

        CollectionAssert.AreEqual(
            new[] { "{\"sdkToken\":\"tok\"}", "{\"apikey\":\"key\"}" },
            server.AuthData.ToArray());
    }

    // ========== Helpers ==========

    private static WebSocketClientOptions Options(WebSocketLoopbackServer server, WebSocketEndpoint endpoint = WebSocketEndpoint.Stock) =>
        new WebSocketClientOptions { ApiKey = "the-key", BaseUrl = server.Url, Endpoint = endpoint };

    private static ConcurrentQueue<string> Record(FugleWebsocketClient client)
    {
        var events = new ConcurrentQueue<string>();
        client.OnConnected = m => events.Enqueue("connected:" + m);
        client.OnDisconnected = m => events.Enqueue("disconnected:" + m);
        client.OnClose = m => events.Enqueue("close:" + m);
        // The server's authenticated ack reaches OnMessage too; it is not what
        // these tests are about.
        client.OnMessage = m => { if (!m.Contains("\"event\":\"authenticated\"")) events.Enqueue("message:" + m); };
        client.OnError = m => events.Enqueue("error:" + m);
        client.OnException = e => events.Enqueue("exception:" + e.Message);
        return events;
    }

    private static async Task WaitFor<T>(ConcurrentQueue<T> queue, int count)
    {
        var watch = Stopwatch.StartNew();
        while (queue.Count < count)
        {
            if (watch.Elapsed > Timeout) Assert.Fail($"expected {count} items, got {queue.Count}: {string.Join(" | ", queue)}");
            await Task.Delay(10);
        }
    }

    private static async Task AssertFrames(WebSocketLoopbackServer server, string[] expected)
    {
        var deadline = DateTime.UtcNow + Timeout;
        while (server.OtherFrames.Count < expected.Length && DateTime.UtcNow < deadline)
        {
            await Task.Delay(20);
        }
        CollectionAssert.AreEqual(expected, server.OtherFrames.ToArray());
    }
}
