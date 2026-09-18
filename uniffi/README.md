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

Bindings are generated in library mode from the compiled crate. The
generators are third-party tools and must match the UniFFI version in the
workspace (`0.29.4`):

```bash
cargo install uniffi-bindgen-cs   --git https://github.com/NordSecurity/uniffi-bindgen-cs   --tag v0.10.0+v0.29.4
cargo install uniffi-bindgen-go   --git https://github.com/NordSecurity/uniffi-bindgen-go   --tag v0.5.0+v0.29.5
cargo install uniffi-bindgen-cpp  --git https://github.com/NordSecurity/uniffi-bindgen-cpp  --tag v0.9.0+v0.29.4
cargo install uniffi-bindgen-java --git https://github.com/IronCoreLabs/uniffi-bindgen-java --tag 0.2.1 --locked

# C# output is formatted with CSharpier 1.x when it is on PATH; CI pins 1.3.0
dotnet tool install -g csharpier --version 1.3.0

make gen-csharp gen-go   # committed; CI fails if these drift
make gen-cpp gen-java
```

## Adding UniFFI Functions

Keep the number of by-value `String`, `Vec` and record arguments small on
exported functions and constructors; group related parameters into a
`uniffi::Record` instead. Each of those arguments crosses the FFI as a
`RustBuffer` struct passed by value, and JNA (Java) does not always marshal a
long list of them correctly. A constructor taking ten `RustBuffer` arguments
panicked with `RustBuffer length exceeds capacity` from Java on macOS arm64
while C# and Go worked (#91); packing the three credentials into one record
fixed it.

CI runs the Java tests with the native library loaded on Linux x86_64 and
macOS arm64 (`-PrequireNative`, so a library that fails to load fails the
tests instead of skipping them). Run them locally before adding a signature
like this:

```bash
cargo build -p marketdata-uniffi --release
cd bindings/java && ./gradlew test -PexcludeTags=integration -PrequireNative
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
    .probeEnabled(true)
    .idleProbeAfterMs(15000L)
    .build();

FugleWebSocketClient ws = FugleWebSocketClient.builder()
    .apiKey("your-key")
    .reconnect(reconnect)
    .healthCheck(healthCheck)
    .build();
```

**Configuration Options:**

`ReconnectOptions.builder()` (auto-reconnect is on without it):

- `enabled(Boolean)` - Whether auto-reconnect is enabled (default: true; `false` turns it off)
- `maxAttempts(Integer)` - Maximum reconnection attempts; 0 means unlimited (default: 0)
- `initialDelayMs(Long)` - Initial delay for exponential backoff (default: 1000ms, min: 100ms)
- `maxDelayMs(Long)` - Maximum delay cap (default: 60000ms)

`HealthCheckOptions.builder()`:

- `enabled(Boolean)` - Whether health check is enabled (default: true)
- `heartbeatTimeoutMs(Long)` - Maximum gap between inbound frames before the connection is declared dead (default: 35000ms, min: 5000ms); does not apply with `probeEnabled`
- `probeEnabled(Boolean)` - Confirm with a ping before declaring the connection dead (default: false)
- `idleProbeAfterMs(Long)` - Silence before the ping (default: 30000ms, min: 5000ms)
- `probeTimeoutMs(Long)` - Wait for any inbound frame after the ping (default: 5000ms, min: 1000ms)

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
reconnect := marketdata.ReconnectConfig{
    MaxAttempts:      10,
    InitialDelayMs:   2000,
    MaxDelayMs:       120000,
}

healthCheck := marketdata.HealthCheckConfig{
    ProbeEnabled:     true,
    IdleProbeAfterMs: 15000,
}

ws, err := marketdata.NewFugleWebSocketClient(
    marketdata.WithApiKey("your-key"),
    marketdata.WithReconnect(reconnect),
    marketdata.WithHealthCheck(healthCheck),
)
```

**Configuration Options:**

`ReconnectConfig` struct (auto-reconnect is on without it; pass
`marketdata.WithoutReconnect()` instead of `WithReconnect` to turn it off):

- `MaxAttempts uint32` - Maximum reconnection attempts (zero = use default: unlimited)
- `InitialDelayMs uint64` - Initial delay for exponential backoff (zero = use default 1000ms)
- `MaxDelayMs uint64` - Maximum delay cap (zero = use default 60000ms)

`HealthCheckConfig` struct (detection is on without it; pass
`marketdata.WithoutHealthCheck()` instead of `WithHealthCheck` to turn it off):

- `HeartbeatTimeoutMs uint64` - Maximum gap between inbound frames (zero = use default 35000ms); does not apply with `ProbeEnabled`
- `ProbeEnabled bool` - Confirm with a ping before declaring the connection dead
- `IdleProbeAfterMs uint64` - Silence before the ping (zero = use default 30000ms)
- `ProbeTimeoutMs uint64` - Wait for any inbound frame after the ping (zero = use default 5000ms)

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
    ProbeEnabled = true,
    IdleProbeAfterMs = 15000
};

var ws = new WebSocketClient(new WebSocketClientOptions
{
    ApiKey = "your-key",
    ReconnectOptions = reconnect,
    HealthCheck = healthCheck
});
```

**Configuration Options:**

`ReconnectOptions` class (auto-reconnect is on without it):

- `Enabled bool?` - Whether auto-reconnect is enabled (null = use default true; `false` turns it off)
- `MaxAttempts uint?` - Maximum reconnection attempts; 0 means unlimited (null = use default: unlimited)
- `InitialDelayMs ulong?` - Initial delay for exponential backoff (null = use default 1000ms)
- `MaxDelayMs ulong?` - Maximum delay cap (null = use default 60000ms)

`HealthCheckOptions` class:

- `Enabled bool?` - Whether health check is enabled (null = use default true)
- `HeartbeatTimeoutMs ulong?` - Maximum gap between inbound frames (null = use default 35000ms); does not apply with `ProbeEnabled`
- `ProbeEnabled bool?` - Confirm with a ping before declaring the connection dead (null = false)
- `IdleProbeAfterMs ulong?` - Silence before the ping (null = use default 30000ms)
- `ProbeTimeoutMs ulong?` - Wait for any inbound frame after the ping (null = use default 5000ms)

The probe options and their trade-offs are described in
[docs/configuration.md](../docs/configuration.md#healthcheckconfig--healthcheckoptions).

## API Reference

### REST API Methods

All methods return JSON strings that can be parsed with your language's JSON library:

**Stock Market Data:**

```text
getStockQuote(symbol)           # Get real-time quote
getStockTicker(symbol)          # Get symbol information
getStockCandles(symbol, timeframe)  # Get OHLCV candles
getStockTrades(symbol)          # Get trade history
getStockVolumes(symbol)         # Get volume by price
```

**Futures and Options (FutOpt) Data:**

```text
getFutOptQuote(symbol, afterHours)   # Get real-time quote
getFutOptTicker(symbol)              # Get contract information
getFutOptCandles(symbol, timeframe)  # Get OHLCV candles
getFutOptTrades(symbol)              # Get trade history
getFutOptVolumes(symbol)             # Get volume by price
getFutOptProducts(type)              # Get product listing ("F" or "O")
```

### WebSocket Methods

```text
connect()                       # Connect to WebSocket server
disconnect()                    # Disconnect from server
subscribe(channel, symbol)      # Subscribe to channel
unsubscribe(channel, symbol)    # Unsubscribe by channel and symbol
unsubscribe_ids(ids)            # Unsubscribe by server ids from `subscribed`
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

Every exception variant carries an `info` field (`ErrorInfo`: `code`,
`sourceKind`, `message`, `status`, `body`, `requestId`, `headers`), and
`WebSocketListener.on_error` receives the same record. See
[docs/errors.md](../docs/errors.md) for the field names per language and all
error codes.

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

- UniFFI 0.29.4
- marketdata-core (internal)

## License

MIT
