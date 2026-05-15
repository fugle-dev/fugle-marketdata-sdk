# fugle-marketdata-go

Go bindings for Fugle Market Data API. Built with UniFFI for native integration.

## Installation

### From Source

```bash
# 1. Build the native library
cd uniffi
cargo build --release

# 2. Install Go bindings
go get github.com/fugle-dev/fugle-marketdata-go

# 3. Set library path when running
export LD_LIBRARY_PATH=/path/to/target/release:$LD_LIBRARY_PATH  # Linux
export DYLD_LIBRARY_PATH=/path/to/target/release:$DYLD_LIBRARY_PATH  # macOS
```

### Requirements

- Go 1.18 or later
- CGO enabled
- Rust toolchain (for building native library)

## Quick Start

### REST API

```go
package main

import (
    "fmt"
    "log"

    mkt "github.com/fugle-dev/fugle-marketdata-go"
)

func main() {
    // Create client with API key
    client, err := mkt.NewRestClientWithApiKey("your-api-key")
    if err != nil {
        log.Fatal(err)
    }
    defer client.Destroy()

    // Get stock quote
    quote, err := client.Stock().Intraday().GetQuote("2330")
    if err != nil {
        log.Fatal(err)
    }
    fmt.Printf("TSMC Price: %.2f\n", *quote.ClosePrice)
    fmt.Printf("Change: %.2f\n", *quote.Change)
    fmt.Printf("Volume: %d\n", quote.Total.TradeVolume)

    // Get stock ticker info
    ticker, err := client.Stock().Intraday().GetTicker("2330")
    if err != nil {
        log.Fatal(err)
    }
    fmt.Printf("Name: %s\n", *ticker.Name)

    // Get intraday candles (5-minute)
    candles, err := client.Stock().Intraday().GetCandles("2330", "5")
    if err != nil {
        log.Fatal(err)
    }
    for i, candle := range candles.Data[:3] {
        fmt.Printf("  [%d] %s: O=%.2f H=%.2f L=%.2f C=%.2f\n",
            i+1, candle.Time, candle.Open, candle.High, candle.Low, candle.Close)
    }

    // Get recent trades
    trades, err := client.Stock().Intraday().GetTrades("2330")
    if err != nil {
        log.Fatal(err)
    }
    for i, trade := range trades.Data[:5] {
        fmt.Printf("  [%d] Price: %.2f, Size: %d\n", i+1, trade.Price, trade.Size)
    }

    // FutOpt (futures/options) data
    futoptQuote, err := client.FutOpt().Intraday().GetQuote("TXFC4")
    if err != nil {
        log.Fatal(err)
    }
    fmt.Printf("Futures Price: %.2f\n", *futoptQuote.ClosePrice)
}
```

### WebSocket Streaming

```go
package main

import (
    "fmt"
    "log"
    "os"
    "os/signal"
    "syscall"
    "time"

    mkt "github.com/fugle-dev/fugle-marketdata-go"
)

func main() {
    // Create streaming client (channel-based)
    client, err := mkt.NewStreamingClient("your-api-key", 100)
    if err != nil {
        log.Fatal(err)
    }

    // Connect
    if err := client.Connect(); err != nil {
        log.Fatal(err)
    }
    fmt.Println("Connected!")

    // Subscribe to channels
    if err := client.Subscribe("trades", "2330"); err != nil {
        log.Fatal(err)
    }
    if err := client.Subscribe("books", "2330"); err != nil {
        log.Fatal(err)
    }

    // Setup signal handler
    sigChan := make(chan os.Signal, 1)
    signal.Notify(sigChan, syscall.SIGINT, syscall.SIGTERM)

    // Receive messages (channel-based Go idiom)
    messageCount := 0
    timeout := time.After(10 * time.Second)

    for {
        select {
        case msg, ok := <-client.Messages():
            if !ok {
                fmt.Println("Channel closed")
                return
            }
            messageCount++
            if msg.Event == "data" {
                fmt.Printf("[%d] %s: %s\n", messageCount, *msg.Channel, *msg.Symbol)
            }

        case err := <-client.Errors():
            fmt.Printf("Error: %v\n", err)

        case <-sigChan:
            fmt.Println("\nShutdown requested...")
            goto cleanup

        case <-timeout:
            fmt.Println("\nTimeout reached...")
            goto cleanup
        }
    }

cleanup:
    fmt.Printf("\nReceived %d messages\n", messageCount)

    // Close with timeout
    done := make(chan struct{})
    go func() {
        client.Close()
        close(done)
    }()

    select {
    case <-done:
        fmt.Println("Done!")
    case <-time.After(3 * time.Second):
        fmt.Println("Close timeout, forcing exit")
    }
}
```

## Authentication

Three authentication methods are supported:

```go
import mkt "github.com/fugle-dev/fugle-marketdata-go"

// 1. API Key (most common)
client, err := mkt.NewRestClientWithApiKey("your-api-key")

// 2. Bearer Token
client, err := mkt.NewRestClientWithBearerToken("your-bearer-token")

// 3. SDK Token
client, err := mkt.NewRestClientWithSdkToken("your-sdk-token")
```

## Advanced: Custom TLS / self-signed servers

For connecting to servers with a private CA (enterprise deployments) or
self-signed certs (dev / staging), the `NewRestClient*AndTls` variants
and the WebSocket full-config factory accept an optional
`TlsConfigRecord`:

```go
import (
    "os"
    mkt "github.com/fugle-dev/fugle-marketdata-go"
)

// Pin a custom CA (production-safe when your server cert is properly
// issued by this CA and has matching SANs).
caPem, err := os.ReadFile("/path/to/ca.crt")
if err != nil { /* ... */ }

tls := mkt.TlsConfigRecord{
    RootCertPem:        &caPem,
    AcceptInvalidCerts: false,
}
client, err := mkt.NewRestClientWithApiKeyAndTls(
    "your-api-key", nil /* baseUrl */, tls)

// Disable ALL TLS verification — dev / testing only. Exposes MITM risk.
baseUrl := "wss://192.0.2.1/v1.0"
insecure := mkt.TlsConfigRecord{
    RootCertPem:        nil,
    AcceptInvalidCerts: true,
}
devClient, err := mkt.NewRestClientWithApiKeyAndTls(
    "your-api-key", &baseUrl, insecure)
```

For WebSocket use `NewWebsocketClientWithFullConfig(...)` — same pattern,
accepts an optional `TlsConfigRecord` plus reconnect/health check
configs.

## API Reference

### RestClient

#### Stock Intraday Methods

```go
// Real-time quote
GetQuote(symbol string) (*Quote, error)

// Symbol information
GetTicker(symbol string) (*Ticker, error)

// OHLCV candles (timeframe: "1", "5", "10", "15", "30", "60")
GetCandles(symbol string, timeframe string) (*CandlesResponse, error)

// Trade history
GetTrades(symbol string) (*TradesResponse, error)

// Volume by price
GetVolumes(symbol string) (*VolumesResponse, error)
```

#### FutOpt Intraday Methods

```go
// Real-time quote
GetQuote(symbol string) (*Quote, error)

// Contract information
GetTicker(symbol string) (*Ticker, error)

// OHLCV candles
GetCandles(symbol string, timeframe string) (*CandlesResponse, error)

// Trade history
GetTrades(symbol string) (*TradesResponse, error)

// Volume by price
GetVolumes(symbol string) (*VolumesResponse, error)

// Product listing (type: "F" for futures, "O" for options)
GetProducts(productType string) (*ProductsResponse, error)
```

#### Resource Management

```go
// Always call Destroy() when done
defer client.Destroy()
```

### StreamingClient

#### Creation

```go
// Create client with buffer size
NewStreamingClient(apiKey string, bufferSize int) (*StreamingClient, error)

// Alternative authentication
NewStreamingClientWithBearerToken(token string, bufferSize int) (*StreamingClient, error)
NewStreamingClientWithSdkToken(token string, bufferSize int) (*StreamingClient, error)
```

#### Methods

```go
// Connection management
Connect() error                    // Connect to server
Close()                            // Close connection (blocks until complete)
IsConnected() bool                 // Check connection status
IsClosed() bool                    // Check if client is closed

// Subscription management
Subscribe(channel string, symbol string) error       // Subscribe to channel
Unsubscribe(subscriptionId string) error            // Unsubscribe by ID
GetSubscriptions() []Subscription                    // List active subscriptions

// Message channels (Go idiom)
Messages() <-chan StreamMessage    // Receive messages
Errors() <-chan error              // Receive errors
```

#### StreamMessage Type

```go
type StreamMessage struct {
    Event    string   // Event type: "subscribed", "snapshot", "data", "heartbeat"
    Channel  *string  // Channel name (may be nil)
    Symbol   *string  // Symbol code (may be nil)
    DataJson *string  // JSON data payload (may be nil)
}
```

#### Channels

| Channel | Description |
|---------|-------------|
| `trades` | Real-time trade executions |
| `candles` | Candlestick updates |
| `books` | Order book (5 levels) |
| `aggregates` | Aggregated market data |
| `indices` | Index values (stock only) |

## Error Handling

All API methods return Go-style errors:

```go
import mkt "github.com/fugle-dev/fugle-marketdata-go"

client, err := mkt.NewRestClientWithApiKey("invalid-key")
if err != nil {
    log.Fatal(err)
}
defer client.Destroy()

quote, err := client.Stock().Intraday().GetQuote("2330")
if err != nil {
    // Error message includes code, e.g., "[2002] Authentication failed"
    log.Printf("Error: %v", err)
    return
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

```go
package main

import (
    "fmt"
    "log"
    "os"

    mkt "github.com/fugle-dev/fugle-marketdata-go"
)

func main() {
    apiKey := os.Getenv("FUGLE_API_KEY")
    if apiKey == "" {
        fmt.Println("Set FUGLE_API_KEY environment variable")
        os.Exit(1)
    }

    // Create client
    client, err := mkt.NewRestClientWithApiKey(apiKey)
    if err != nil {
        log.Fatalf("Failed to create client: %v", err)
    }
    defer client.Destroy()

    // Stock data
    fmt.Println("=== Stock Market Data ===")
    quote, err := client.Stock().Intraday().GetQuote("2330")
    if err != nil {
        log.Fatalf("Failed to get quote: %v", err)
    }
    fmt.Printf("TSMC Quote: %.2f\n", *quote.LastPrice)

    ticker, err := client.Stock().Intraday().GetTicker("2330")
    if err != nil {
        log.Fatalf("Failed to get ticker: %v", err)
    }
    fmt.Printf("Ticker: %s\n", *ticker.Name)

    candles, err := client.Stock().Intraday().GetCandles("2330", "5")
    if err != nil {
        log.Fatalf("Failed to get candles: %v", err)
    }
    fmt.Printf("Candles: %d entries\n", len(candles.Data))

    // FutOpt data
    fmt.Println("\n=== FutOpt Market Data ===")
    products, err := client.FutOpt().Intraday().GetProducts("F")
    if err != nil {
        log.Fatalf("Failed to get products: %v", err)
    }
    fmt.Printf("Futures products: %d\n", len(products.Data))

    fmt.Println("\nDone!")
}
```

### Full WebSocket Example

```go
package main

import (
    "encoding/json"
    "fmt"
    "log"
    "os"
    "os/signal"
    "syscall"
    "time"

    mkt "github.com/fugle-dev/fugle-marketdata-go"
)

func main() {
    apiKey := os.Getenv("FUGLE_API_KEY")
    if apiKey == "" {
        fmt.Println("Set FUGLE_API_KEY environment variable")
        os.Exit(1)
    }

    // Create streaming client
    fmt.Println("Creating streaming client...")
    client, err := mkt.NewStreamingClient(apiKey, 100)
    if err != nil {
        log.Fatalf("Failed to create client: %v", err)
    }

    // Connect
    fmt.Println("Connecting...")
    if err := client.Connect(); err != nil {
        log.Fatalf("Failed to connect: %v", err)
    }
    fmt.Println("Connected!")

    // Subscribe
    fmt.Println("Subscribing to 2330 trades...")
    if err := client.Subscribe("trades", "2330"); err != nil {
        log.Fatalf("Failed to subscribe: %v", err)
    }

    // Setup signal handler
    sigChan := make(chan os.Signal, 1)
    signal.Notify(sigChan, syscall.SIGINT, syscall.SIGTERM)

    // Receive messages
    fmt.Println("Waiting for messages (Ctrl+C to stop)...\n")
    messageCount := 0
    timeout := time.After(30 * time.Second)

    for {
        select {
        case msg, ok := <-client.Messages():
            if !ok {
                fmt.Println("Channel closed")
                goto cleanup
            }
            messageCount++
            printMessage(msg, messageCount)

        case err := <-client.Errors():
            fmt.Printf("Error: %v\n", err)

        case <-sigChan:
            fmt.Println("\nShutdown requested...")
            goto cleanup

        case <-timeout:
            fmt.Println("\n30 seconds elapsed, stopping...")
            goto cleanup
        }
    }

cleanup:
    fmt.Printf("\nReceived %d messages\n", messageCount)
    fmt.Println("Closing connection...")

    // Close with timeout
    done := make(chan struct{})
    go func() {
        client.Close()
        close(done)
    }()

    select {
    case <-done:
        fmt.Println("Done!")
    case <-time.After(3 * time.Second):
        fmt.Println("Close timeout, forcing exit")
        os.Exit(0)
    }
}

func printMessage(msg mkt.StreamMessage, count int) {
    event := msg.Event
    channel := ""
    symbol := ""

    if msg.Channel != nil {
        channel = *msg.Channel
    }
    if msg.Symbol != nil {
        symbol = *msg.Symbol
    }

    switch event {
    case "subscribed":
        fmt.Printf("  [Subscribed] channel=%s, symbol=%s\n", channel, symbol)

    case "snapshot", "data":
        prefix := "[Snapshot]"
        if event == "data" {
            prefix = "[Data]"
        }
        fmt.Printf("  %s %s:%s", prefix, channel, symbol)

        if msg.DataJson != nil {
            var data map[string]interface{}
            if err := json.Unmarshal([]byte(*msg.DataJson), &data); err == nil {
                if price, ok := data["price"].(float64); ok {
                    fmt.Printf(" price=%.2f", price)
                }
                if vol, ok := data["volume"].(float64); ok {
                    fmt.Printf(" vol=%.0f", vol)
                }
            }
        }
        fmt.Println()

    case "heartbeat":
        fmt.Println("  [Heartbeat]")

    default:
        fmt.Printf("  [%s] channel=%s symbol=%s\n", event, channel, symbol)
        if msg.DataJson != nil {
            fmt.Printf("    data: %s\n", *msg.DataJson)
        }
    }
}
```

## License

MIT
