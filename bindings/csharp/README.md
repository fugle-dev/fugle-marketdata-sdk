# FugleMarketData.NET

C# bindings for Fugle Market Data API. Built with UniFFI for native integration.

## Installation

### From NuGet

```bash
# Prerelease builds require --prerelease
dotnet add package Fugle.MarketData --prerelease
```

The package ships native libraries for `linux-x64`, `osx-arm64`, `osx-x64`
and `win-x64`. The high-level client lives in the `FugleMarketData` namespace;
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

// Get intraday candles (5-minute)
var candles = JsonDocument.Parse(
    await client.Stock.Intraday.GetCandlesAsync("2330", "5")).RootElement;
foreach (var candle in candles.GetProperty("data").EnumerateArray().Take(3))
{
    Console.WriteLine($"  {candle.GetProperty("date").GetString()}: "
        + $"O={candle.GetProperty("open").GetDouble()} "
        + $"C={candle.GetProperty("close").GetDouble()}");
}

// Get recent trades
var trades = JsonDocument.Parse(
    await client.Stock.Intraday.GetTradesAsync("2330")).RootElement;
foreach (var trade in trades.GetProperty("data").EnumerateArray().Take(5))
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

    public void OnError(string errorMessage)
    {
        Console.WriteLine($"Error: {errorMessage}");
    }

    public void OnReconnecting(uint attempt) { }

    public void OnReconnectFailed(uint attempts) { }
}

// Create WebSocket client
var listener = new MyListener();
using var ws = new WebSocketClient("your-api-key", listener);

// Connect and subscribe
await ws.ConnectAsync();
await ws.SubscribeAsync("trades", "2330");
await ws.SubscribeAsync("books", "2330");

// Keep running for 10 seconds
await Task.Delay(TimeSpan.FromSeconds(10));

// Disconnect
await ws.DisconnectAsync();
```

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

#### Stock Intraday Methods

```csharp
// Real-time quote
Task<Quote> GetQuoteAsync(string symbol)

// Symbol information
Task<Ticker> GetTickerAsync(string symbol)

// OHLCV candles (timeframe: "1", "5", "10", "15", "30", "60")
Task<CandlesResponse> GetCandlesAsync(string symbol, string timeframe = "1")

// Trade history
Task<TradesResponse> GetTradesAsync(string symbol)

// Volume by price
Task<VolumesResponse> GetVolumesAsync(string symbol)
```

#### FutOpt Intraday Methods

```csharp
// Real-time quote
Task<Quote> GetQuoteAsync(string symbol)

// Contract information
Task<Ticker> GetTickerAsync(string symbol)

// OHLCV candles
Task<CandlesResponse> GetCandlesAsync(string symbol, string timeframe = "1")

// Trade history
Task<TradesResponse> GetTradesAsync(string symbol)

// Volume by price
Task<VolumesResponse> GetVolumesAsync(string symbol)

// Product listing (type: "F" for futures, "O" for options)
Task<ProductsResponse> GetProductsAsync(string type)
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

Task SubscribeAsync(string channel, string symbol)  // Subscribe to channel
Task UnsubscribeAsync(string subscriptionId)        // Unsubscribe by ID
List<Subscription> GetSubscriptions()               // List active subscriptions
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
    void OnError(string errorMessage);
    void OnReconnecting(uint attempt);
    void OnReconnectFailed(uint attempts);        // terminal
}
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

All API errors throw `FugleException`:

```csharp
using FugleMarketData;

try
{
    using var client = new RestClient("invalid-key");
    var quote = await client.Stock.Intraday.GetQuoteAsync("2330");
}
catch (FugleException ex)
{
    Console.WriteLine($"Error: {ex.Message}");
    // Exception message includes error code, e.g., "[2002] Authentication failed"
}
```

### Error Codes

| Code | Error Type | Description |
|------|------------|-------------|
| 1001 | InvalidSymbol | Invalid or unsupported symbol |
| 1002 | DeserializationError | JSON parsing failed |
| 1003 | RuntimeError | Internal runtime error |
| 1004 | ConfigError | Configuration error |
| 2001 | ConnectionError | Network connection failed |
| 2002 | AuthError | Authentication failed |
| 2003 | ApiError | API returned error response |
| 2010 | ClientClosed | Client has been closed |
| 3001 | TimeoutError | Operation timed out |
| 3002 | WebSocketError | WebSocket protocol error |
| 9999 | Other | Unexpected error |

## Examples

### Full REST Example

```csharp
using System;
using System.Threading.Tasks;
using FugleMarketData;

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
                await client.Stock.Intraday.GetCandlesAsync("2330", "5")).RootElement;
            Console.WriteLine($"Candles: {candles.GetProperty("data").GetArrayLength()} entries");

            // FutOpt data
            Console.WriteLine("\n=== FutOpt Market Data ===");
            var products = await client.FutOpt.Intraday.GetProductsAsync("F");
            Console.WriteLine($"Futures products: "
                + JsonDocument.Parse(products).RootElement.GetProperty("data").GetArrayLength());
        }
        catch (FugleException ex)
        {
            Console.WriteLine($"Error: {ex.Message}");
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

    public void OnError(string errorMessage)
    {
        Console.WriteLine($"Error: {errorMessage}");
    }

    public void OnReconnecting(uint attempt) { }

    public void OnReconnectFailed(uint attempts) { }
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
        catch (FugleException ex)
        {
            Console.WriteLine($"Error: {ex.Message}");
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
