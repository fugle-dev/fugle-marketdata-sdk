// WebSocket benchmark client -- new Rust-core SDK (C# / UniFFI binding)
//
// Connects to the mock server, subscribes, receives data messages, and
// reports throughput / latency / memory metrics as a single JSON line on stdout.
//
// Usage:
//   dotnet run -- --url ws://localhost:8765 --timeout 60000

using System.Diagnostics;
using System.Text.Json;
using FugleMarketData;
using uniffi.marketdata_uniffi;

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------
string Flag(string name, string fallback)
{
    for (int i = 0; i < args.Length - 1; i++)
        if (args[i] == $"--{name}") return args[i + 1];
    return fallback;
}

var baseUrl = Flag("url", "ws://localhost:8765");
var timeout = int.Parse(Flag("timeout", "60000"));

// ---------------------------------------------------------------------------
// Metrics state
// ---------------------------------------------------------------------------
int received = 0;
long t0 = 0;
bool t0Set = false;
var latencies = new double[500000];
int latIdx = 0;
int maxSerial = -1;
JsonElement? serverStats = null;
var done = new ManualResetEventSlim(false);

var proc = Process.GetCurrentProcess();
var startCpu = proc.UserProcessorTime;
var startMem = proc.WorkingSet64;

// Force GC before benchmark
GC.Collect();
GC.WaitForPendingFinalizers();
GC.Collect();

// ---------------------------------------------------------------------------
// Listener
// ---------------------------------------------------------------------------
var listener = new BenchListener(
    onMsg: (StreamMessage msg) =>
    {
        var ev = msg.@event;

        // Discard warmup
        if (ev == "warmup") return;

        // Sentinel
        if (ev == "bench_done")
        {
            if (msg.dataJson != null)
            {
                try { serverStats = JsonDocument.Parse(msg.dataJson).RootElement; } catch { }
            }
            done.Set();
            return;
        }

        if (ev == "data")
        {
            if (!t0Set) { t0 = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds(); t0Set = true; }
            received++;

            if (msg.dataJson != null)
            {
                try
                {
                    using var doc = JsonDocument.Parse(msg.dataJson);
                    var root = doc.RootElement;
                    if (root.TryGetProperty("server_ts", out var tsProp))
                    {
                        var serverTs = tsProp.GetInt64();
                        var now = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();
                        if (latIdx < latencies.Length)
                            latencies[latIdx++] = now - serverTs;
                    }
                    if (root.TryGetProperty("serial", out var serialProp))
                    {
                        var s = serialProp.GetInt32();
                        if (s > maxSerial) maxSerial = s;
                    }
                }
                catch { }
            }
        }
    },
    onErr: (string err) => Console.Error.WriteLine($"error: {err}")
);

// ---------------------------------------------------------------------------
// Connect & subscribe
// ---------------------------------------------------------------------------
using var client = new FugleMarketData.WebSocketClient(
    new WebSocketClientOptions
    {
        ApiKey = "bench-key",
        BaseUrl = baseUrl,
        Endpoint = FugleMarketData.WebSocketEndpoint.Stock,
    },
    listener
);

await client.ConnectAsync();
await client.SubscribeAsync("trades", "2330");

// Wait for bench_done or timeout
if (!done.Wait(timeout))
    Console.Error.WriteLine($"TIMEOUT: did not receive bench_done within {timeout} ms");

// ---------------------------------------------------------------------------
// Report
// ---------------------------------------------------------------------------
var elapsed = t0Set ? DateTimeOffset.UtcNow.ToUnixTimeMilliseconds() - t0 : 0L;
proc.Refresh();
var endCpu = proc.UserProcessorTime;
var endMem = proc.WorkingSet64;

// Sort latencies
var lats = new ArraySegment<double>(latencies, 0, latIdx).ToArray();
Array.Sort(lats);

double? Percentile(double[] sorted, int p)
{
    if (sorted.Length == 0) return null;
    int idx = Math.Min((int)Math.Ceiling(p / 100.0 * sorted.Length) - 1, sorted.Length - 1);
    return Math.Round(sorted[Math.Max(0, idx)], 2);
}

int? ssCount = null;
double? ssMps = null;
if (serverStats.HasValue)
{
    var ss = serverStats.Value;
    if (ss.TryGetProperty("count", out var c)) ssCount = c.GetInt32();
    if (ss.TryGetProperty("server_msgs_per_sec", out var m)) ssMps = m.GetDouble();
}

var result = new Dictionary<string, object?>
{
    ["sdk"] = "rust-core-cs",
    ["count"] = received,
    ["expected"] = ssCount,
    ["lost"] = ssCount.HasValue ? ssCount.Value - received : null,
    ["elapsed_ms"] = elapsed,
    ["msgs_per_sec"] = elapsed > 0 ? (int)(received / (double)elapsed * 1000) : 0,
    ["latency_p50_ms"] = Percentile(lats, 50),
    ["latency_p99_ms"] = Percentile(lats, 99),
    ["latency_min_ms"] = lats.Length > 0 ? (double?)lats[0] : null,
    ["latency_max_ms"] = lats.Length > 0 ? (double?)lats[^1] : null,
    ["mem_rss_delta_mb"] = Math.Round((endMem - startMem) / 1e6, 1),
    ["cpu_user_ms"] = Math.Round((endCpu - startCpu).TotalMilliseconds, 1),
    ["cpu_system_ms"] = 0.0,
    ["server_msgs_per_sec"] = ssMps,
};

Console.WriteLine(JsonSerializer.Serialize(result));

await client.DisconnectAsync();
await Task.Delay(200);
Environment.Exit(0);

// ---------------------------------------------------------------------------
// Listener class
// ---------------------------------------------------------------------------
class BenchListener : IWebSocketListener
{
    private readonly Action<StreamMessage> _onMsg;
    private readonly Action<string> _onErr;

    public BenchListener(Action<StreamMessage> onMsg, Action<string> onErr)
    {
        _onMsg = onMsg;
        _onErr = onErr;
    }

    public void OnConnected() { }
    public void OnDisconnected() { }
    public void OnMessage(StreamMessage message) => _onMsg(message);
    public void OnError(string errorMessage) => _onErr(errorMessage);
    public void OnReconnecting(uint attempt) { }
    public void OnReconnectFailed(uint attempts) { }
}
