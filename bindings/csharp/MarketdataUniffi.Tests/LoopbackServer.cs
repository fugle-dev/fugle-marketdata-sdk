using Microsoft.VisualStudio.TestTools.UnitTesting;
using System;
using System.Net;
using System.Text;
using System.Threading;
using System.Threading.Tasks;

namespace MarketdataUniffi.Tests;

/// <summary>
/// A loopback HTTP server that answers every request identically.
///
/// The SDK issues HTTP from Rust, so no .NET-level mock can intercept it. A
/// real server on 127.0.0.1 is the only way to exercise the request path
/// without talking to production.
/// </summary>
internal sealed class LoopbackServer : IDisposable
{
    private readonly HttpListener _listener = new();
    private readonly CancellationTokenSource _cts = new();

    public LoopbackServer(string body, int statusCode = 200, string contentType = "application/json")
    {
        Prefix = $"http://127.0.0.1:{FreePort()}";
        _listener.Prefixes.Add(Prefix + "/");
        _listener.Start();

        var payload = Encoding.UTF8.GetBytes(body);
        _ = Task.Run(async () =>
        {
            while (!_cts.IsCancellationRequested)
            {
                HttpListenerContext ctx;
                try { ctx = await _listener.GetContextAsync().ConfigureAwait(false); }
                catch { return; }

                try
                {
                    ctx.Response.StatusCode = statusCode;
                    ctx.Response.ContentType = contentType;
                    ctx.Response.ContentLength64 = payload.Length;
                    await ctx.Response.OutputStream.WriteAsync(payload, 0, payload.Length).ConfigureAwait(false);
                    ctx.Response.Close();
                }
                catch
                {
                    // The client may have hung up; nothing useful to do here.
                }
            }
        });
    }

    /// <summary>Base URL to hand to the SDK — host only, no version segment.</summary>
    public string Prefix { get; }

    /// <summary>A client pointed at this server.</summary>
    public FugleMarketData.RestClient NewClient(string apiKey = "test-key") =>
        new(new FugleMarketData.RestClientOptions { ApiKey = apiKey, BaseUrl = Prefix });

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

internal static class AssertEx
{
    /// <summary>
    /// Assert that <paramref name="action"/> throws something.
    ///
    /// <c>Assert.ThrowsExceptionAsync&lt;Exception&gt;</c> matches the exact
    /// type, so it fails on the SDK's own <c>AuthException</c> /
    /// <c>ApiException</c> subclasses — the opposite of what callers of this
    /// helper mean.
    /// </summary>
    public static async Task<Exception> ThrowsAnyAsync(Func<Task> action)
    {
        try
        {
            await action().ConfigureAwait(false);
        }
        catch (Exception ex)
        {
            return ex;
        }

        Assert.Fail("Expected an exception, but the call succeeded.");
        throw new InvalidOperationException("unreachable");
    }
}
