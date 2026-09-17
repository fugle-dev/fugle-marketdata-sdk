using System;
using System.Collections.Concurrent;
using System.Net;
using System.Net.WebSockets;
using System.Text;
using System.Text.Json;
using System.Threading;
using System.Threading.Tasks;

namespace MarketdataUniffi.Tests;

/// <summary>
/// A loopback WebSocket server that records the <c>data</c> of every
/// <c>auth</c> frame (as compact JSON) and acks it.
/// </summary>
internal sealed class WebSocketLoopbackServer : IDisposable
{
    private readonly HttpListener _listener = new();
    private readonly CancellationTokenSource _cts = new();

    public WebSocketLoopbackServer()
    {
        var port = FreePort();
        Url = $"ws://127.0.0.1:{port}";
        _listener.Prefixes.Add($"http://127.0.0.1:{port}/");
        _listener.Start();
        _ = Task.Run(AcceptLoop);
    }

    /// <summary>Base URL to hand to the SDK.</summary>
    public string Url { get; }

    public ConcurrentQueue<string> AuthData { get; } = new();

    private async Task AcceptLoop()
    {
        while (!_cts.IsCancellationRequested)
        {
            HttpListenerContext ctx;
            try { ctx = await _listener.GetContextAsync().ConfigureAwait(false); }
            catch { return; }
            _ = Task.Run(() => Serve(ctx));
        }
    }

    private async Task Serve(HttpListenerContext ctx)
    {
        try
        {
            var socket = (await ctx.AcceptWebSocketAsync(null).ConfigureAwait(false)).WebSocket;
            var buffer = new byte[64 * 1024];
            while (!_cts.IsCancellationRequested)
            {
                var received = 0;
                WebSocketReceiveResult result;
                do
                {
                    result = await socket.ReceiveAsync(new ArraySegment<byte>(buffer, received, buffer.Length - received), _cts.Token).ConfigureAwait(false);
                    received += result.Count;
                } while (!result.EndOfMessage);

                if (result.MessageType == WebSocketMessageType.Close)
                {
                    await socket.CloseOutputAsync(WebSocketCloseStatus.NormalClosure, null, CancellationToken.None).ConfigureAwait(false);
                    return;
                }

                using var frame = JsonDocument.Parse(Encoding.UTF8.GetString(buffer, 0, received));
                if (frame.RootElement.GetProperty("event").GetString() == "auth")
                {
                    AuthData.Enqueue(frame.RootElement.GetProperty("data").GetRawText());
                    var ack = Encoding.UTF8.GetBytes("{\"event\":\"authenticated\",\"data\":{\"message\":\"Authenticated successfully\"}}");
                    await socket.SendAsync(ack, WebSocketMessageType.Text, true, CancellationToken.None).ConfigureAwait(false);
                }
            }
        }
        catch
        {
            // The client hung up or the server is shutting down.
        }
    }

    private static int FreePort()
    {
        var probe = new System.Net.Sockets.TcpListener(IPAddress.Loopback, 0);
        probe.Start();
        var port = ((IPEndPoint)probe.LocalEndpoint).Port;
        probe.Stop();
        return port;
    }

    public void Dispose()
    {
        _cts.Cancel();
        _listener.Close();
        _cts.Dispose();
    }
}
