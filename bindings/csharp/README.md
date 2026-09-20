# FugleMarketData.NET

C# bindings for Fugle Market Data API. Built with UniFFI for native integration.

## Installation

### From NuGet

```bash
# Prerelease builds require --prerelease
dotnet add package Fugle.MarketData --prerelease
```

The package ships native libraries for `linux-x64`, `linux-arm64`, `osx-arm64`,
`osx-x64` and `win-x64`. The high-level client lives in the `FugleMarketData` namespace;
the raw UniFFI bindings are in `MarketdataUniffi`.

### From Source

```bash
# 1. Build the native library
cd uniffi
cargo build --release

# 2. Copy native library to bindings directory
cp ../target/release/libmarketdata_uniffi.dylib bindings/csharp/  # macOS
# OR
cp ../target/release/libmarketdata_uniffi.so bindings/csharp/    # Linux
# OR
cp ../target/release/marketdata_uniffi.dll bindings/csharp/      # Windows

# 3. Build C# binding
cd bindings/csharp
dotnet build
```

### Requirements

- .NET 8.0 or later
- Rust toolchain (for building native library)

## Quick Start

### REST API

```csharp
using System.Text.Json;
using FugleMarketData;
using StockCandlesParams = uniffi.marketdata_uniffi.StockCandlesParams;
using StockTradesParams = uniffi.marketdata_uniffi.StockTradesParams;

// Create client with API key
using var client = new RestClient("your-api-key");

// Methods return the server's JSON body as a string. Parse it with
// System.Text.Json, or deserialize into your own model types.
var quote = JsonDocument.Parse(
    await client.Stock.Intraday.GetQuoteAsync("2330")).RootElement;
Console.WriteLine($"TSMC Price: {quote.GetProperty("closePrice").GetDouble()}");
Console.WriteLine($"Change: {quote.GetProperty("change").GetDouble()}");
Console.WriteLine($"Volume: {quote.GetProperty("total").GetProperty("tradeVolume").GetInt64()}");

// Fields the server omits are absent, not null — check before reading.
if (quote.TryGetProperty("referencePrice", out var reference))
{
    Console.WriteLine($"Reference: {reference.GetDouble()}");
}

// Get stock ticker info
var ticker = JsonDocument.Parse(
    await client.Stock.Intraday.GetTickerAsync("2330")).RootElement;
Console.WriteLine($"Name: {ticker.GetProperty("name").GetString()}");

// Get intraday candles (5-minute); unset timeframe takes the server default
var candles = JsonDocument.Parse(
    await client.Stock.Intraday.GetCandlesAsync(
        "2330", new StockCandlesParams(timeframe: "5"))).RootElement;
foreach (var candle in candles.GetProperty("data").EnumerateArray().Take(3))
{
    Console.WriteLine($"  {candle.GetProperty("date").GetString()}: "
        + $"O={candle.GetProperty("open").GetDouble()} "
        + $"C={candle.GetProperty("close").GetDouble()}");
}

// Get recent trades, limited to 5 (params record: oddLot, offset, limit, sort, isTrial)
var trades = JsonDocument.Parse(
    await client.Stock.Intraday.GetTradesAsync(
        "2330", new StockTradesParams(limit: 5))).RootElement;
foreach (var trade in trades.GetProperty("data").EnumerateArray())
{
    Console.WriteLine($"  Price: {trade.GetProperty("price").GetDouble()}, "
        + $"Size: {trade.GetProperty("size").GetInt64()}");
}

// FutOpt (futures/options) data
var futoptQuote = JsonDocument.Parse(
    await client.FutOpt.Intraday.GetQuoteAsync("TXFC4")).RootElement;
Console.WriteLine($"Futures Price: {futoptQuote.GetProperty("closePrice").GetDouble()}");
```

### WebSocket Streaming

```csharp
using FugleMarketData;
using uniffi.marketdata_uniffi;

// Create listener
class MyListener : IWebSocketListener
{
    public void OnConnected()
    {
        Console.WriteLine("Connected!");
    }

    public void OnAuthenticated(string? dataJson)
    {
        Console.WriteLine("Authenticated");
    }

    public void OnUnauthenticated(string? dataJson)
    {
        Console.WriteLine($"Rejected: {dataJson}");
    }

    public void OnDisconnected(bool willReconnect)
    {
        Console.WriteLine($"Disconnected (will reconnect: {willReconnect})");
    }

    public void OnMessage(StreamMessage message)
    {
        if (message.@event == "data")
        {
            Console.WriteLine($"[{message.channel}] {message.symbol}");
            // Parse message.dataJson as needed
        }
    }

    public void OnError(ErrorInfo error)
    {
        Console.WriteLine($"Error [{error.code}]: {error.message}");
    }

    public void OnReconnecting(uint attempt) { }

    public void OnReconnectFailed(uint attempts) { }

    public void OnMessagesDropped(ulong count) { }
}

// Create WebSocket client
var listener = new MyListener();
using var ws = new WebSocketClient("your-api-key", listener);

// Connect and subscribe
await ws.ConnectAsync();
await ws.SubscribeAsync("trades", "2330");
await ws.SubscribeAsync("books", "2330");

// Multiple symbols in one subscription; endpoint-specific options
// (Stock: IntradayOddLot, FutOpt: AfterHours)
await ws.SubscribeAsync("trades", new[] { "2330", "2317" });
await ws.SubscribeAsync("trades", new[] { "2330" }, new SubscribeOptions { IntradayOddLot = true });

// Keep running for 10 seconds
await Task.Delay(TimeSpan.FromSeconds(10));

// Disconnect
await ws.DisconnectAsync();
```

### WebSocket Streaming, event style (FubonNeo shape)

The same stream through `Action<string>` events instead of a listener class —
the shape of FubonNeo's `FugleMarketData` client, so code written against it
moves over with its `using` lines and event handlers intact:

```csharp
using FugleMarketData.WebsocketClient;
using FugleMarketData.WebsocketModels;

using var factory = FugleWebsocketClientFactory.Create(sdkToken);   // or CreateWithApiKey(apiKey)
var stock = factory.Stock;                                          // factory.FutureOption for FutOpt

stock.OnConnected = msg => Console.WriteLine(msg);                  // "Connected"
stock.OnMessage += raw => Console.WriteLine(raw);                   // every frame, verbatim
stock.OnError += raw => Console.WriteLine($"server error: {raw}");  // error frames
stock.OnException += ex => Console.WriteLine($"sdk error: {ex.Message}");
stock.OnDisconnected += msg => Console.WriteLine($"disconnected: {msg}");

await stock.Connect();
await stock.Subscribe(StockChannel.Trades, "2330");
await stock.Subscribe(StockChannel.Trades, "2330", "2317");         // one frame, symbols: [...]
await stock.Subscribe(StockChannel.Trades, new StockSubscribeParams { Symbol = "2330", IntradayOddLot = true });
await stock.Unsubscribe("id-from-subscribed-message");

await stock.Disconnect("bye");                                      // OnDisconnected("bye")
```

Every event is a settable property with a no-op default, so `=` and `+=`
both work (set them before `Connect()`; `+=` is not atomic). Handlers run on
the SDK's callback thread; one that throws does not stop the stream — the
failure comes back through `OnException` (code 3004).
See [Event-style client](#event-style-client-fubonneo-shape) for the full
surface and the differences from FubonNeo.

## Authentication

Three authentication methods are supported:

```csharp
using FugleMarketData;

// 1. API Key (most common)
using var client = new RestClient("your-api-key");

// 2. Bearer Token
using var client = RestClient.WithBearerToken("your-bearer-token");

// 3. SDK Token
using var client = RestClient.WithSdkToken("your-sdk-token");
```

The WebSocket client takes the same three credentials through
`WebSocketClientOptions` (`ApiKey`, `BearerToken` or `SdkToken`). Do not log
the raw `uniffi.marketdata_uniffi.CredentialsRecord` the credentials are passed
in: a C# record's `ToString()` prints every field, secrets included.

## Advanced: Custom TLS / self-signed servers

For connecting to servers with a private CA (enterprise deployments) or
self-signed certs (dev / staging), the underlying UniFFI bindings expose
TLS-aware factory functions. `RestClient` will gain direct support in a
future release; for now use the raw `MarketdataUniffi` namespace:

```csharp
using MarketdataUniffi;

using System.IO;

// Pin a custom CA (production-safe when your server cert is properly
// issued by this CA and has matching SANs).
byte[] caPem = File.ReadAllBytes("/path/to/ca.crt");
var tls = new TlsConfigRecord(caPem, false);
var client = MarketdataUniffiMethods.NewRestClientWithApiKeyAndTls(
    "your-api-key", baseUrl: null, tls: tls);

// Disable ALL TLS verification — dev / testing only. Exposes MITM risk.
var insecure = new TlsConfigRecord(null, true);
var devClient = MarketdataUniffiMethods.NewRestClientWithApiKeyAndTls(
    "your-api-key", baseUrl: "wss://192.0.2.1/v1.0", tls: insecure);
```

For WebSocket use `NewWebsocketClientWithFullConfig(...)` — same pattern,
accepts optional `TlsConfigRecord` plus reconnect/health check configs.

## API Reference

### RestClient

Every method returns the server's JSON body as a `Task<string>`; parse it with
`System.Text.Json` or your own model types. Optional filters go through one
params record per endpoint (`uniffi.marketdata_uniffi` namespace); an unset
field is not sent, so `new StockTradesParams()` (or the `params: null`
default) sends nothing extra. Required parameters (`type`, `direction`,
`change`, `trade`, technical periods) are always positional.

#### Stock Intraday Methods

```csharp
// Real-time quote (params: OddLotParams — oddLot)
Task<string> GetQuoteAsync(string symbol, OddLotParams? @params = null)

// Symbol information (params: OddLotParams — oddLot)
Task<string> GetTickerAsync(string symbol, OddLotParams? @params = null)

// OHLCV candles (params: StockCandlesParams — timeframe, oddLot, sort;
// unset timeframe takes the server default)
Task<string> GetCandlesAsync(string symbol, StockCandlesParams? @params = null)

// Trade history (params: StockTradesParams — oddLot, offset, limit, sort, isTrial)
Task<string> GetTradesAsync(string symbol, StockTradesParams? @params = null)

// Volume by price (params: OddLotParams — oddLot)
Task<string> GetVolumesAsync(string symbol, OddLotParams? @params = null)

// Batch tickers (params: StockTickersParams — exchange, market, industry,
// isNormal, isAttention, isDisposition, isHalted, symbol)
Task<string> GetTickersAsync(string type, StockTickersParams? @params = null)
```

#### FutOpt Intraday Methods

```csharp
// Real-time quote (params: AfterHoursParams — afterHours)
Task<string> GetQuoteAsync(string symbol, AfterHoursParams? @params = null)

// Contract information (params: AfterHoursParams — afterHours)
Task<string> GetTickerAsync(string symbol, AfterHoursParams? @params = null)

// OHLCV candles (params: FutOptCandlesParams — afterHours, timeframe)
Task<string> GetCandlesAsync(string symbol, FutOptCandlesParams? @params = null)

// Trade history (params: FutOptTradesParams — afterHours, offset, limit, isTrial)
Task<string> GetTradesAsync(string symbol, FutOptTradesParams? @params = null)

// Volume by price (params: AfterHoursParams — afterHours)
Task<string> GetVolumesAsync(string symbol, AfterHoursParams? @params = null)

// Product listing (type: "F" for futures, "O" for options;
// params: FutOptProductsParams — exchange, afterHours, contractType, status)
Task<string> GetProductsAsync(string type, FutOptProductsParams? @params = null)
```

### WebSocketClient

#### Properties

```csharp
ws.Stock    // Access stock market streaming
ws.FutOpt   // Access futures/options streaming
```

#### Methods

```csharp
Task ConnectAsync()                           // Connect to server
Task DisconnectAsync()                        // Disconnect from server
bool IsConnected                              // Check connection status
bool IsClosed                                 // Check if client is closed
ulong MessagesDroppedTotal                    // Messages dropped this connection (see below)

// afterHours: FutOpt after-hours session (FutOpt endpoint only; 1005 on Stock)
Task SubscribeAsync(string channel, string symbol, bool? afterHours = null)    // Subscribe to channel
Task UnsubscribeAsync(string channel, string symbol, bool? afterHours = null)  // Unsubscribe (same afterHours as subscribe)

// SubscribeOptions { AfterHours (FutOpt only), IntradayOddLot (Stock only) };
// one symbol with options is new[] { "2330" }
Task SubscribeAsync(string channel, IEnumerable<string> symbols, SubscribeOptions? options = null)  // multiple symbols in one frame
Task UnsubscribeAsync(string channel, IEnumerable<string> symbols, SubscribeOptions? options = null)

Task UnsubscribeAsync(IEnumerable<string> ids)                                // Unsubscribe by server ids (empty: 1005)
List<Subscription> GetSubscriptions()               // List active subscriptions

Task PingAsync(string? state = null)                // Fire-and-forget ping; pong (if any) arrives via OnMessage
Task<double> MeasureLatencyAsync(ulong? timeoutMs = null)  // Ping and await the pong; returns round-trip time in ms (see Health Check below)
```

#### IWebSocketListener Interface

```csharp
public interface IWebSocketListener
{
    void OnConnected();                           // transport up, before auth
    void OnAuthenticated(string? dataJson);       // server accepted the credentials
    void OnUnauthenticated(string? dataJson);     // server rejected the credentials
    void OnDisconnected(bool willReconnect);      // at most once per connection
    void OnMessage(StreamMessage message);
    void OnError(ErrorInfo error);                // code, sourceKind, message, ...
    void OnReconnecting(uint attempt);
    void OnReconnectFailed(uint attempts);        // terminal
    void OnMessagesDropped(ulong count);          // messages dropped since the last call (DropNewest overflow)
}
```

An exception thrown by a listener method does not crash the process or stop
later events. It is reported to `OnError` with code 3004 (`CALLBACK_FAILED`,
`sourceKind` `Client`) and a message naming the method, the exception type and
message, and the number of failures, e.g.
`Listener OnMessage threw System.InvalidOperationException: boom (1 in the last 1s)`.
The first failure is reported at once, later ones at most once per second,
counting the failures since the previous report; failures after the last
report are not reported on their own. An exception thrown by `OnError` itself
is written to `Console.Error` and not re-reported.

#### Event-style client (FubonNeo shape)

`FugleMarketData.WebsocketClient` / `FugleMarketData.WebsocketModels` hold a
second surface over the same `WebSocketClient`: FubonNeo's class names, enums
and `Action<string>` events. The listener interface and `WebSocketClient`
are unchanged; each event client owns one `WebSocketClient` (`Inner`) and an
internal listener that maps its callbacks onto the events.

```csharp
// FugleMarketData.WebsocketClient
FugleWebsocketClientFactory.Create(string sdkToken, WebsocketVersionOptions? versions = null, string? baseUrl = null)
FugleWebsocketClientFactory.CreateWithApiKey(string apiKey, WebsocketVersionOptions? versions = null, string? baseUrl = null)
factory.Stock          // FugleWebsocketStockClient, built on first access
factory.FutureOption   // FugleWebsocketFutOptClient, built on first access
factory.Dispose()      // disposes the clients it built

// Own options (reconnect, health check, message queue, …): Endpoint must match the class
new FugleWebsocketStockClient(WebSocketClientOptions options)     // Endpoint = Stock (the default)
new FugleWebsocketFutOptClient(WebSocketClientOptions options)    // Endpoint = FutOpt

// abstract FugleWebsocketClient : IDisposable
Action<string>    OnMessage, OnError, OnConnected, OnDisconnected, OnClose
Action<Exception> OnException                       // MarketDataStreamException { ErrorInfo Info }
Action<uint>      OnReconnecting, OnReconnectFailed // not in FubonNeo
Action<ulong>     OnMessagesDropped                 // not in FubonNeo
Task Connect()
Task Disconnect(string msg = "Disconnect")
Task Ping(string pingMsg = "ping")
Task Unsubscribe(string channelId)
Task Unsubscribe(params string[] channelIds)
Task Unsubscribe(UnsubscribeParams param)           // ChannelId + ChannelIds in one frame
bool IsConnected
WebSocketClient Inner                               // the underlying client

// FugleWebsocketStockClient — StockChannel { Trades, Candles, Books, Aggregates, Indices }
Task Subscribe(StockChannel channel, string symbol)
Task Subscribe(StockChannel channel, params string[] symbols)
Task Subscribe(StockChannel channel, StockSubscribeParams param)    // { Symbol, Symbols, IntradayOddLot }

// FugleWebsocketFutOptClient — FutureOptionChannel { Trades, Books, Candles, Aggregates }
Task Subscribe(FutureOptionChannel channel, string symbol)
Task Subscribe(FutureOptionChannel channel, params string[] symbols)
Task Subscribe(FutureOptionChannel channel, FutureOptionParams param) // { Symbol, Symbols, AfterHours }
```

The channel string is the enum name lower-cased. A params object's `Symbol`
(when non-empty) and `Symbols` go into one frame; none at all is error 1005.
`FutureOptionParams` cannot be passed to the stock client (and vice versa):
that is a compile error, not a runtime one.

Events, from the listener callbacks:

| Listener callback | Event |
|---|---|
| `OnConnected()` | `OnConnected("Connected")` — again after every reconnect |
| `OnAuthenticated(json)` | nothing |
| `OnUnauthenticated(json)` | `OnError(json ?? "{}")`, then `OnException(MarketDataStreamException "Authenticate Failed!")` (code 2002) |
| `OnMessage(msg)` | `OnMessage(msg.raw)`; an `error` frame raises `OnError(msg.raw)` first |
| `OnError(info)` | `OnException(new MarketDataStreamException(info))` |
| `OnDisconnected(_)` after `Disconnect(msg)` | `OnDisconnected(msg)` |
| `OnDisconnected(_)` otherwise | `OnClose("Received close message")`, then `OnDisconnected("Server Disconnected")` — with reconnect on, `OnReconnecting` follows |
| `OnReconnecting` / `OnReconnectFailed` / `OnMessagesDropped` | same name |

Moving from FubonNeo's `FugleMarketData` client:

| FubonNeo | Here |
|---|---|
| `FugleWebsocketClientFactory.Create(token, Mode.Speed, versions, baseUrl)` | `Create(sdkToken, versions, baseUrl)` — no `Mode`: Speed/Normal is a Fubon endpoint concept |
| `Connect(timeoutMs, enablePingPong)` | `Connect()` — liveness is the SDK's [health check](#health-check) (on by default); auth timeout is `WebSocketClientOptions.AuthTimeoutMs` |
| `ValueTask` returns | `Task` — `await` as before |
| events default to `Console.WriteLine` | default to no-op |
| `WebSocketState` | `IsConnected`, or `Inner.IsClosed` |
| `Subscribe(channel, new[] { "2330" })` (one-element array) sends `symbols: ["2330"]` | sends `symbol: "2330"` — the server treats both alike |
| `Unsubscribe(new UnsubscribeParams { ChannelId = "a" })` sent no id at all (the condition was inverted) | sends `id: "a"` |
| `Disconnect(msg)` always raised `OnDisconnected(msg)`, even with nothing to close | raised by core's disconnect only: no event when never connected, already disconnected, or reconnecting |
| reconnect: none | on by default (`OnClose` → `OnDisconnected` → `OnReconnecting` → `OnConnected`); off via `new FugleWebsocketStockClient(new WebSocketClientOptions { …, Reconnect = new ReconnectOptions { Enabled = false } })` |
| an exception in a handler propagates | is caught and reported through `OnException` (code 3004), the stream continues |

#### Reconnection

After an unexpected drop the client reconnects on its own with exponential
backoff (1 s doubling up to 60 s), without an attempt limit, and subscribes
again once it is back. The server closing with 1000 or a 4xxx code (e.g. an
auth failure) never triggers a reconnect. Configure it with `WebSocketClientOptions.Reconnect`:

```csharp
// Stop after 10 attempts; OnReconnectFailed fires once the last one fails
Reconnect = new ReconnectOptions { MaxAttempts = 10 },

// Turn auto-reconnect off
Reconnect = new ReconnectOptions { Enabled = false },
```

`MaxAttempts` 0 means unlimited (the default); `InitialDelayMs` (default 1000,
min 100) and `MaxDelayMs` (default 60000) tune the backoff.

#### Health Check

Liveness detection is on by default: when the connection stays silent too
long it is declared dead and auto-reconnect takes over. Configure it with
`WebSocketClientOptions.HealthCheck`:

```csharp
// Default: HeartbeatTimeoutMs 35000
HealthCheck = new HealthCheckOptions { HeartbeatTimeoutMs = 60000 },

// Probe mode: confirm a silent connection with a ping before declaring it
// dead, instead of guessing off a timeout
HealthCheck = new HealthCheckOptions { ProbeEnabled = true, IdleProbeAfterMs = 10000 },

// Turn it off
HealthCheck = new HealthCheckOptions { Enabled = false },
```

`HeartbeatTimeoutMs` (default 35000, min 5000) does not apply when
`ProbeEnabled` is true. With `ProbeEnabled`, `IdleProbeAfterMs` (default
30000, min 5000) and `ProbeTimeoutMs` (default 5000, min 1000) control when a
ping is sent and how long to wait for a reply. With the defaults, detection
stays at 35 s and no ping is sent while the server's heartbeat is on time;
lowering `IdleProbeAfterMs` detects faster at the cost of pinging the server
more often. Probing does not detect a half-open connection (the server still
sends, but our writes no longer reach it). See
[HealthCheckConfig / HealthCheckOptions](../../docs/configuration.md#healthcheckconfig--healthcheckoptions)
for the full trade-offs and estimated server cost.

`PingAsync` is fire-and-forget, as in the old SDK, with the pong delivered to
`OnMessage`. `MeasureLatencyAsync` sends one ping, awaits its pong and returns
the round-trip time in milliseconds; it works regardless of `ProbeEnabled` and
sends nothing in the background otherwise.

#### Message queue overflow

By default the client buffers up to 4096 unread messages and drops the
newest ones once `OnMessage` falls behind, reporting the drop count via
`OnMessagesDropped`. Configure this through `WebSocketClientOptions`:

```csharp
var options = new WebSocketClientOptions
{
    ApiKey = "your-api-key",
    MessageOverflow = MessageOverflow.DropNewest,  // or MessageOverflow.Unbounded
    MessageBuffer = 8192,                          // null = default (4096); must be > 0
};
using var ws = new WebSocketClient(options, listener);

// Running total of dropped messages for the current connection
Console.WriteLine(ws.MessagesDroppedTotal);
```

`MessageOverflow.Unbounded` never drops messages; the queue keeps growing
while `OnMessage` lags, so only use it when the listener is guaranteed to
keep up.

#### Streaming protocol version

By default the server picks the latest streaming version for both endpoints.
Pin a specific version with `WebSocketClientOptions.Versions`:

```csharp
using FugleMarketData.WebsocketClient;

var options = new WebSocketClientOptions
{
    ApiKey = "your-api-key",
    Versions = new WebsocketVersionOptions
    {
        Stock = "v1.0",   // only "v1.0" is served
        FutOpt = "v1.0",  // "v1.0" or "v1.1" (default: latest, v1.1)
    },
};
using var ws = new WebSocketClient(options, listener);
```

FutOpt v1.1 adds trial-matching (試撮) frames on `trades` / `books` — check
the frame's `isTrial` before acting on a price.

#### Auth timeout

Once the WebSocket is open the client sends its auth frame and waits for the
server's verdict for `AuthTimeoutMs` (default 10000). It applies to the
first connect and to every reconnect; elapsing it fails the attempt with a
timeout error (code 3001). Raise it on a slow route to the server; the
server itself allows 60 seconds.

```csharp
var options = new WebSocketClientOptions
{
    ApiKey = "your-api-key",
    AuthTimeoutMs = 15000,  // null = default (10000); must be > 0
};
using var ws = new WebSocketClient(options, listener);
```

#### StreamMessage Properties

| Property | Type | Description |
|----------|------|-------------|
| `event` | `string` | Event type: "subscribed", "snapshot", "data", "heartbeat" |
| `channel` | `string?` | Channel name |
| `symbol` | `string?` | Symbol code |
| `dataJson` | `string?` | JSON data payload |

#### Channels

| Channel | Description |
|---------|-------------|
| `trades` | Real-time trade executions |
| `candles` | Candlestick updates |
| `books` | Order book (5 levels) |
| `aggregates` | Aggregated market data |
| `indices` | Index values (stock only) |

## Error Handling

API errors throw `MarketDataException` (one subclass per error type).
`GetInfo()` returns the fields every language exposes:

```csharp
using FugleMarketData;
using uniffi.marketdata_uniffi;

try
{
    using var client = new RestClient("invalid-key");
    var quote = await client.Stock.Intraday.GetQuoteAsync("2330");
}
catch (MarketDataException ex)
{
    var info = ex.GetInfo();
    Console.WriteLine($"Error [{info.code}] {info.sourceKind}: {info.message}");
    if (info.status is not null)
        Console.WriteLine($"HTTP {info.status}: {info.body}");
}
```

| `ErrorInfo` property | Type | |
|---|---|---|
| `code` | `int` | Error code (table below) |
| `sourceKind` | `ErrorSourceKind` | `Network`, `Protocol`, `Auth`, `RateLimit` or `Client` |
| `message` | `string` | Human-readable message |
| `status` | `ushort?` | HTTP status |
| `body` | `string?` | Raw HTTP response body (REST) |
| `requestId` | `string?` | `x-request-id` response header |
| `headers` | `Dictionary<string, string>` | HTTP response headers, lowercase names (REST) |

`IWebSocketListener.OnError` receives the same `ErrorInfo`. See the
[error reference](https://github.com/fugle-dev/fugle-marketdata-sdk/blob/main/docs/errors.md) for all languages.

### Error Codes

| Code | Error Type | Description |
|------|------------|-------------|
| 1001 | InvalidSymbol | Invalid or unsupported symbol |
| 1002 | DeserializationError | JSON parsing failed |
| 1003 | RuntimeError | Internal runtime error |
| 1004 | ConfigError | Configuration error |
| 1005 | InvalidParameter | Invalid or missing parameter (including an unknown WebSocket channel) |
| 2001 | ConnectionError | Network connection failed |
| 2002 | AuthError | Authentication failed |
| 2003 | ApiError | API returned error response |
| 2010 | ClientClosed | Client has been closed |
| 3001 | TimeoutError | Operation timed out |
| 3002 | WebSocketError | WebSocket connect, read or write failed |
| 3003 | HeartbeatTimeout | No inbound WebSocket frame within the heartbeat window |
| 3004 | CallbackFailed | A listener method threw (`OnError` only) |
| 9999 | Other | Unexpected error |

## Examples

### Full REST Example

```csharp
using System;
using System.Text.Json;
using System.Threading.Tasks;
using FugleMarketData;
using StockCandlesParams = uniffi.marketdata_uniffi.StockCandlesParams;

class Program
{
    static async Task Main(string[] args)
    {
        var apiKey = Environment.GetEnvironmentVariable("FUGLE_API_KEY");
        if (string.IsNullOrEmpty(apiKey))
        {
            Console.WriteLine("Set FUGLE_API_KEY environment variable");
            return;
        }

        using var client = new RestClient(apiKey);

        try
        {
            // Stock data
            Console.WriteLine("=== Stock Market Data ===");
            var quote = await client.Stock.Intraday.GetQuoteAsync("2330");
            Console.WriteLine($"TSMC Quote: {quote}");  // raw JSON

            var ticker = await client.Stock.Intraday.GetTickerAsync("2330");
            Console.WriteLine($"Ticker: {ticker}");

            var candles = JsonDocument.Parse(
                await client.Stock.Intraday.GetCandlesAsync(
                    "2330", new StockCandlesParams(timeframe: "5"))).RootElement;
            Console.WriteLine($"Candles: {candles.GetProperty("data").GetArrayLength()} entries");

            // FutOpt data
            Console.WriteLine("\n=== FutOpt Market Data ===");
            var products = await client.FutOpt.Intraday.GetProductsAsync("F");
            Console.WriteLine($"Futures products: "
                + JsonDocument.Parse(products).RootElement.GetProperty("data").GetArrayLength());
        }
        catch (MarketDataException ex)
        {
            Console.WriteLine($"Error [{ex.GetInfo().code}]: {ex.Message}");
        }
    }
}
```

### Full WebSocket Example

```csharp
using System;
using System.Threading.Tasks;
using FugleMarketData;
using uniffi.marketdata_uniffi;

class MyListener : IWebSocketListener
{
    public int MessageCount { get; private set; }

    public void OnConnected()
    {
        Console.WriteLine("Connected!");
    }

    public void OnAuthenticated(string? dataJson)
    {
        Console.WriteLine("Authenticated");
    }

    public void OnUnauthenticated(string? dataJson)
    {
        Console.WriteLine($"Rejected: {dataJson}");
    }

    public void OnDisconnected(bool willReconnect)
    {
        Console.WriteLine($"Disconnected (will reconnect: {willReconnect})");
    }

    public void OnMessage(StreamMessage message)
    {
        MessageCount++;
        if (message.@event == "data")
        {
            Console.WriteLine($"[{MessageCount}] {message.channel}: {message.symbol}");
        }
    }

    public void OnError(ErrorInfo error)
    {
        Console.WriteLine($"Error [{error.code}]: {error.message}");
    }

    public void OnReconnecting(uint attempt) { }

    public void OnReconnectFailed(uint attempts) { }

    public void OnMessagesDropped(ulong count) { }
}

class Program
{
    static async Task Main(string[] args)
    {
        var apiKey = Environment.GetEnvironmentVariable("FUGLE_API_KEY");
        if (string.IsNullOrEmpty(apiKey))
        {
            Console.WriteLine("Set FUGLE_API_KEY environment variable");
            return;
        }

        var listener = new MyListener();
        using var ws = new WebSocketClient(apiKey, listener);

        try
        {
            await ws.ConnectAsync();
            await ws.SubscribeAsync("trades", "2330");
            await ws.SubscribeAsync("books", "2330");

            Console.WriteLine("Listening for 10 seconds...");
            await Task.Delay(TimeSpan.FromSeconds(10));

            Console.WriteLine($"\nReceived {listener.MessageCount} messages");
            Console.WriteLine($"Subscriptions: {ws.GetSubscriptions().Count}");
        }
        catch (MarketDataException ex)
        {
            Console.WriteLine($"Error [{ex.GetInfo().code}]: {ex.Message}");
        }
        finally
        {
            if (ws.IsConnected)
            {
                await ws.DisconnectAsync();
            }
            Console.WriteLine("Done");
        }
    }
}
```

## License

MIT
