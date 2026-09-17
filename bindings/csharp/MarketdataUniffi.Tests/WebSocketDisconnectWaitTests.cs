using Microsoft.VisualStudio.TestTools.UnitTesting;
using System;
using System.Threading;
using System.Threading.Tasks;

namespace MarketdataUniffi.Tests;

/// <summary>
/// <c>DisconnectAsync</c> completes once the listener has handled the
/// connection's remaining events (#126), and a listener method that calls it
/// does not wait for itself.
/// </summary>
[TestClass]
public class WebSocketDisconnectWaitTests
{
    [TestMethod]
    public async Task DisconnectAsync_CompletesAfterOnDisconnected()
    {
        using var server = new WebSocketLoopbackServer();
        var authenticated = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var disconnected = 0;
        var listener = new HookListener
        {
            Authenticated = () => authenticated.TrySetResult(),
            Disconnected = () =>
            {
                Thread.Sleep(300);
                Interlocked.Exchange(ref disconnected, 1);
            },
        };
        using var client = new FugleMarketData.WebSocketClient(
            new FugleMarketData.WebSocketClientOptions { ApiKey = "the-key", BaseUrl = server.Url },
            listener);

        await client.ConnectAsync().WaitAsync(TimeSpan.FromSeconds(10));
        await authenticated.Task.WaitAsync(TimeSpan.FromSeconds(5));

        await client.DisconnectAsync().WaitAsync(TimeSpan.FromSeconds(10));
        Assert.AreEqual(1, Volatile.Read(ref disconnected), "OnDisconnected had not run when DisconnectAsync completed");
    }

    [TestMethod]
    public async Task DisconnectAsync_FromListenerMethod_DoesNotWaitForItself()
    {
        using var server = new WebSocketLoopbackServer();
        FugleMarketData.WebSocketClient? self = null;
        var fromCallback = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var disconnected = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var listener = new HookListener
        {
            Authenticated = () =>
            {
                // Blocks the listener thread until the disconnect completes.
                if (self!.DisconnectAsync().Wait(TimeSpan.FromSeconds(5)))
                    fromCallback.TrySetResult();
                else
                    fromCallback.TrySetException(new TimeoutException("DisconnectAsync in OnAuthenticated"));
            },
            Disconnected = () => disconnected.TrySetResult(),
        };
        using var client = new FugleMarketData.WebSocketClient(
            new FugleMarketData.WebSocketClientOptions { ApiKey = "the-key", BaseUrl = server.Url },
            listener);
        self = client;

        await client.ConnectAsync().WaitAsync(TimeSpan.FromSeconds(10));
        await fromCallback.Task.WaitAsync(TimeSpan.FromSeconds(10));
        await disconnected.Task.WaitAsync(TimeSpan.FromSeconds(5));
        await client.DisconnectAsync().WaitAsync(TimeSpan.FromSeconds(10));
    }

    /// <summary>Runs <see cref="Authenticated"/> and <see cref="Disconnected"/>, if set, and ignores every other event.</summary>
    private sealed class HookListener : FugleMarketData.IWebSocketListener
    {
        public Action? Authenticated { get; init; }
        public Action? Disconnected { get; init; }

        public void OnConnected() { }
        public void OnAuthenticated(string? dataJson) => Authenticated?.Invoke();
        public void OnUnauthenticated(string? dataJson) { }
        public void OnDisconnected(bool willReconnect) => Disconnected?.Invoke();
        public void OnMessage(uniffi.marketdata_uniffi.StreamMessage message) { }
        public void OnError(uniffi.marketdata_uniffi.ErrorInfo error) { }
        public void OnReconnecting(uint attempt) { }
        public void OnReconnectFailed(uint attempts) { }
        public void OnMessagesDropped(ulong count) { }
    }
}
