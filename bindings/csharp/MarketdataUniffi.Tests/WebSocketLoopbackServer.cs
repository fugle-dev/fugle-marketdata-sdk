using System;
using System.Collections.Concurrent;
using System.Linq;
using System.Net;
using System.Net.WebSockets;
using System.Text;
using System.Text.Json;
using System.Threading;
using System.Threading.Tasks;

namespace MarketdataUniffi.Tests;

/// <summary>
/// A loopback WebSocket server that records the <c>data</c> of every
/// <c>auth</c> frame (as compact JSON) and acks it, and records every other
/// text frame as is. <see cref="SendToAll"/> pushes a frame to the clients;
/// <see cref="DropConnections"/> cuts the open connections without a Close frame.
/// </summary>
internal sealed class WebSocketLoopbackServer : IDisposable
{
    private readonly HttpListener _listener = new();
    private readonly CancellationTokenSource _cts = new();
    private readonly ConcurrentDictionary<HttpListenerContext, WebSocket> _connections = new();

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

    /// <summary>Text frames other than auth, in arrival order.</summary>
    public ConcurrentQueue<string> OtherFrames { get; } = new();

    /// <summary>
    /// Answer each <c>subscribe</c> frame with a <c>subscribed</c> ack whose id
    /// is <c>id-&lt;channel&gt;-&lt;symbol&gt;[-ah]</c> (single-symbol frames) or
    /// <c>id-&lt;channel&gt;-&lt;symbol1,symbol2,...&gt;</c> (multi-symbol frames).
    /// </summary>
    public bool AckSubscribes { get; init; }

    /// <summary>
    /// Answer the <c>auth</c> frame with the server's credentials-rejected
    /// frame (<c>error</c>, code 1000) instead of <c>authenticated</c>.
    /// </summary>
    public bool RejectAuth { get; init; }

    /// <summary>Send a text frame to every open connection.</summary>
    public async Task SendToAll(string text)
    {
        var bytes = Encoding.UTF8.GetBytes(text);
        foreach (var socket in _connections.Values)
        {
            await socket.SendAsync(bytes, WebSocketMessageType.Text, true, CancellationToken.None).ConfigureAwait(false);
        }
    }

    /// <summary>Cut every open connection at the transport, as a network failure would.</summary>
    public void DropConnections()
    {
        foreach (var (ctx, socket) in _connections)
        {
            _connections.TryRemove(ctx, out _);
            socket.Abort();
            ctx.Response.Abort();
        }
    }

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
            _connections[ctx] = socket;
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

                var text = Encoding.UTF8.GetString(buffer, 0, received);
                using var frame = JsonDocument.Parse(text);
                var eventName = frame.RootElement.GetProperty("event").GetString();
                if (eventName != "auth")
                {
                    OtherFrames.Enqueue(text);
                    if (AckSubscribes && eventName == "subscribe")
                    {
                        var ack = Encoding.UTF8.GetBytes(SubscribedAck(frame.RootElement.GetProperty("data")));
                        await socket.SendAsync(ack, WebSocketMessageType.Text, true, CancellationToken.None).ConfigureAwait(false);
                    }
                }
                else
                {
                    AuthData.Enqueue(frame.RootElement.GetProperty("data").GetRawText());
                    var ack = Encoding.UTF8.GetBytes(RejectAuth
                        ? "{\"event\":\"error\",\"code\":1000,\"data\":{\"message\":\"Invalid token\"}}"
                        : "{\"event\":\"authenticated\",\"data\":{\"message\":\"Authenticated successfully\"}}");
                    await socket.SendAsync(ack, WebSocketMessageType.Text, true, CancellationToken.None).ConfigureAwait(false);
                }
            }
        }
        catch
        {
            // The client hung up or the server is shutting down.
        }
    }

    private static string SubscribedAck(JsonElement data)
    {
        var symbolPart = data.TryGetProperty("symbol", out var symbol)
            ? symbol.GetString()
            : string.Join(",", data.GetProperty("symbols").EnumerateArray().Select(e => e.GetString()));
        var id = $"id-{data.GetProperty("channel").GetString()}-{symbolPart}";
        if (data.TryGetProperty("afterHours", out var afterHours) && afterHours.ValueKind == JsonValueKind.True)
        {
            id += "-ah";
        }
        var raw = data.GetRawText();
        return $"{{\"event\":\"subscribed\",\"data\":{{\"id\":\"{id}\",{raw.Substring(1)}}}";
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
