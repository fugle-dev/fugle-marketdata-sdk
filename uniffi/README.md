# marketdata-uniffi

UniFFI bindings for Fugle marketdata-core library providing multi-language support.

## Overview

This crate provides FFI bindings for multiple programming languages using [UniFFI](https://mozilla.github.io/uniffi-rs/). Supported languages include:

- **Java** - Builder pattern with fluent API
- **Go** - Functional options pattern
- **C#** - Options classes pattern (via csbindgen)
- C++ - Direct FFI bindings
- Swift (macOS/iOS) - Native Swift API
- Kotlin (Android) - Kotlin-friendly bindings

## Architecture

All REST API methods return **JSON strings** for maximum language compatibility. This design choice:

- Avoids complex type mapping across different languages
- Allows parsing with native JSON libraries in each language
- Maintains flexibility for future API changes

WebSocket clients use native callback patterns in each language for optimal developer experience.

## Building

```bash
# Build the library
cargo build -p marketdata-uniffi --release

# The library will be located at:
# target/release/libmarketdata_uniffi.dylib (macOS)
# target/release/libmarketdata_uniffi.so (Linux)
# target/release/marketdata_uniffi.dll (Windows)
```

## Generating Bindings

Use the `uniffi-bindgen` command to generate bindings for your target language:

```bash
# Install uniffi-bindgen-go for Go bindings
cargo install uniffi-bindgen-go --git https://github.com/AbelLykworking/uniffi-bindgen-go --tag v0.2.2+v0.28.3

# Generate Go bindings
uniffi-bindgen-go marketdata-uniffi/src/marketdata.udl -o ./bindings/go/

# For C# bindings, use uniffi-bindgen-cs
cargo install uniffi-bindgen-cs
uniffi-bindgen-cs marketdata-uniffi/src/marketdata.udl -o ./bindings/csharp/

# For C++ bindings
cargo run --bin uniffi-bindgen -- generate \
  marketdata-uniffi/src/marketdata.udl \
  --language cpp \
  --out-dir ./bindings/cpp/
```

## Language-Specific Usage

### Java (Builder Pattern)

The Java binding uses the builder pattern for flexible configuration:

```java
import tw.com.fugle.marketdata.*;

// Create client with API key
FugleRestClient client = FugleRestClient.builder()
    .apiKey("your-api-key")
    .build();

// Bearer token authentication
FugleRestClient client = FugleRestClient.builder()
    .bearerToken("your-bearer-token")
    .build();

// SDK token authentication
FugleRestClient client = FugleRestClient.builder()
    .sdkToken("your-sdk-token")
    .build();

// With custom base URL
FugleRestClient client = FugleRestClient.builder()
    .apiKey("your-key")
    .baseUrl("https://custom.api")
    .build();

// Get stock quote (returns JSON string)
String quoteJson = client.getStockQuote("2330");

// WebSocket with configuration
ReconnectOptions reconnect = ReconnectOptions.builder()
    .maxAttempts(10)
    .initialDelayMs(2000L)
    .maxDelayMs(120000L)
    .build();

HealthCheckOptions healthCheck = HealthCheckOptions.builder()
    .enabled(true)
    .intervalMs(15000L)
    .maxMissedPongs(3)
    .build();

FugleWebSocketClient ws = FugleWebSocketClient.builder()
    .apiKey("your-key")
    .reconnect(reconnect)
    .healthCheck(healthCheck)
    .build();
```

**Configuration Options:**

`ReconnectOptions.builder()`:
- `maxAttempts(Integer)` - Maximum reconnection attempts (default: 5, min: 1)
- `initialDelayMs(Long)` - Initial delay for exponential backoff (default: 1000ms, min: 100ms)
- `maxDelayMs(Long)` - Maximum delay cap (default: 60000ms)

`HealthCheckOptions.builder()`:
- `enabled(Boolean)` - Whether health check is enabled (default: false)
- `intervalMs(Long)` - Ping interval in milliseconds (default: 30000ms, min: 5000ms)
- `maxMissedPongs(Integer)` - Maximum missed pongs (default: 2, min: 1)

### Go (Functional Options)

The Go binding uses functional options for idiomatic Go configuration:

```go
import marketdata "github.com/user/fugle-marketdata-sdk/bindings/go/marketdata"

// Create client with API key
client, err := marketdata.NewFugleRestClient(
    marketdata.WithApiKey("your-api-key"),
)

// Bearer token authentication
client, err := marketdata.NewFugleRestClient(
    marketdata.WithBearerToken("your-bearer-token"),
)

// SDK token authentication
client, err := marketdata.NewFugleRestClient(
    marketdata.WithSdkToken("your-sdk-token"),
)

// With custom base URL
client, err := marketdata.NewFugleRestClient(
    marketdata.WithApiKey("your-key"),
    marketdata.WithBaseUrl("https://custom.api"),
)

// Get stock quote (returns JSON string)
quoteJson, err := client.GetStockQuote("2330")

// WebSocket with configuration
reconnect := &marketdata.ReconnectOptions{
    MaxAttempts:      10,
    InitialDelayMs:   2000,
    MaxDelayMs:       120000,
}

healthCheck := &marketdata.HealthCheckOptions{
    Enabled:         true,
    IntervalMs:      15000,
    MaxMissedPongs:  3,
}

ws, err := marketdata.NewFugleWebSocketClient(
    marketdata.WithApiKey("your-key"),
    marketdata.WithReconnect(reconnect),
    marketdata.WithHealthCheck(healthCheck),
)
```

**Configuration Options:**

`ReconnectOptions` struct:
- `MaxAttempts int` - Maximum reconnection attempts (zero = use default 5)
- `InitialDelayMs uint64` - Initial delay for exponential backoff (zero = use default 1000ms)
- `MaxDelayMs uint64` - Maximum delay cap (zero = use default 60000ms)

`HealthCheckOptions` struct:
- `Enabled bool` - Whether health check is enabled
- `IntervalMs uint64` - Ping interval in milliseconds (zero = use default 30000ms)
- `MaxMissedPongs int` - Maximum missed pongs (zero = use default 2)

### C# (Options Pattern)

The C# binding uses .NET options pattern with nullable properties:

```csharp
using FugleMarketData;

// Create client with API key
var client = new RestClient(new RestClientOptions
{
    ApiKey = "your-api-key"
});

// Bearer token authentication
var client = new RestClient(new RestClientOptions
{
    BearerToken = "your-bearer-token"
});

// SDK token authentication
var client = new RestClient(new RestClientOptions
{
    SdkToken = "your-sdk-token"
});

// With custom base URL
var client = new RestClient(new RestClientOptions
{
    ApiKey = "your-key",
    BaseUrl = "https://custom.api"
});

// Get stock quote (returns JSON string)
string quoteJson = client.GetStockQuote("2330");

// WebSocket with configuration
var reconnect = new ReconnectOptions
{
    MaxAttempts = 10,
    InitialDelayMs = 2000,
    MaxDelayMs = 120000
};

var healthCheck = new HealthCheckOptions
{
    Enabled = true,
    IntervalMs = 15000,
    MaxMissedPongs = 3
};

var ws = new WebSocketClient(new WebSocketClientOptions
{
    ApiKey = "your-key",
    ReconnectOptions = reconnect,
    HealthCheckOptions = healthCheck
});
```

**Configuration Options:**

`ReconnectOptions` class:
- `MaxAttempts int?` - Maximum reconnection attempts (null = use default 5)
- `InitialDelayMs ulong?` - Initial delay for exponential backoff (null = use default 1000ms)
- `MaxDelayMs ulong?` - Maximum delay cap (null = use default 60000ms)

`HealthCheckOptions` class:
- `Enabled bool?` - Whether health check is enabled (null = use default false)
- `IntervalMs ulong?` - Ping interval in milliseconds (null = use default 30000ms)
- `MaxMissedPongs int?` - Maximum missed pongs (null = use default 2)

## API Reference

### REST API Methods

All methods return JSON strings that can be parsed with your language's JSON library:

**Stock Market Data:**
```
getStockQuote(symbol)           # Get real-time quote
getStockTicker(symbol)          # Get symbol information
getStockCandles(symbol, timeframe)  # Get OHLCV candles
getStockTrades(symbol)          # Get trade history
getStockVolumes(symbol)         # Get volume by price
```

**Futures and Options (FutOpt) Data:**
```
getFutOptQuote(symbol, afterHours)   # Get real-time quote
getFutOptTicker(symbol)              # Get contract information
getFutOptCandles(symbol, timeframe)  # Get OHLCV candles
getFutOptTrades(symbol)              # Get trade history
getFutOptVolumes(symbol)             # Get volume by price
getFutOptProducts(type)              # Get product listing ("F" or "O")
```

### WebSocket Methods

```
connect()                       # Connect to WebSocket server
disconnect()                    # Disconnect from server
subscribe(channel, symbol)      # Subscribe to channel
unsubscribe(subscriptionId)     # Unsubscribe by ID
isConnected()                   # Check connection status
isClosed()                      # Check if client is closed
```

**WebSocket Channels:**
- `trades` - Real-time trade executions
- `candles` - Real-time candlestick updates
- `books` - Order book (5 levels bid/ask)
- `aggregates` - Aggregated market data
- `indices` - Index values (stock only)

### Error Handling

Errors are thrown as exceptions in target languages:

**Java:**
```java
try {
    String quote = client.getStockQuote("INVALID");
} catch (MarketDataException.InvalidSymbol e) {
    System.out.println("Invalid symbol: " + e.getMessage());
} catch (MarketDataException.AuthError e) {
    System.out.println("Authentication failed: " + e.getMessage());
}
```

**Go:**
```go
quote, err := client.GetStockQuote("INVALID")
if err != nil {
    log.Printf("Error: %v", err)
}
```

**C#:**
```csharp
try
{
    var quote = client.GetStockQuote("INVALID");
}
catch (InvalidSymbolException ex)
{
    Console.WriteLine($"Invalid symbol: {ex.Message}");
}
catch (AuthErrorException ex)
{
    Console.WriteLine($"Authentication failed: {ex.Message}");
}
```

### Error Types

| Error Type | Description |
|-----------|-------------|
| `InvalidSymbol` | Invalid or unsupported symbol |
| `DeserializationError` | JSON parsing failed |
| `RuntimeError` | Internal runtime error |
| `ConfigError` | Configuration error |
| `ConnectionError` | Network connection failed |
| `AuthError` | Authentication failed |
| `ApiError` | API returned error response |
| `TimeoutError` | Operation timed out |
| `WebSocketError` | WebSocket protocol error |
| `Other` | Other unexpected errors |

## Response Format

All REST methods return JSON strings. Example response:

```json
{
  "symbol": "2330",
  "date": "2026-01-30",
  "time": "13:30:00",
  "open": 650.0,
  "high": 655.0,
  "low": 648.0,
  "close": 652.0,
  "volume": 12345678
}
```

Parse with your language's JSON library (e.g., `Jackson` for Java, `encoding/json` for Go, `System.Text.Json` for C#).

## Dependencies

- UniFFI 0.28
- marketdata-core (internal)

## License

MIT
