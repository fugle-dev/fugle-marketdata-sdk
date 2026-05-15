# FugleMarketData Java

Java bindings for Fugle Market Data API. Built with UniFFI for native integration.

## Installation

### From Source

```bash
# 1. Build the native library
cd uniffi
cargo build --release

# 2. Build Java bindings with Gradle
cd bindings/java
./gradlew build

# 3. Set library path when running
export LD_LIBRARY_PATH=../../target/release:$LD_LIBRARY_PATH  # Linux/macOS
# OR for JNA
java -Djna.library.path=../../target/release YourApp
```

### Requirements

- Java 21 or later
- Gradle (for building)
- JNA (Java Native Access) library
- Rust toolchain (for building native library)

## Quick Start

### REST API

```java
import tw.com.fugle.marketdata.FugleRestClient;
import tw.com.fugle.marketdata.generated.*;

// Create client with API key
FugleRestClient client = FugleRestClient.builder()
    .apiKey("your-api-key")
    .build();

// Get stock quote
Quote quote = client.stock().intraday().getQuote("2330");
System.out.printf("TSMC Price: %.2f%n", quote.closePrice());
System.out.printf("Change: %.2f%n", quote.change());
System.out.printf("Volume: %d%n", quote.total().tradeVolume());

// Get stock ticker info
Ticker ticker = client.stock().intraday().getTicker("2330");
System.out.printf("Name: %s%n", ticker.name());

// Get intraday candles (5-minute)
CandlesResponse candles = client.stock().intraday().getCandles("2330", "5");
for (Candle candle : candles.data().subList(0, 3)) {
    System.out.printf("  %s: O=%.2f H=%.2f L=%.2f C=%.2f%n",
        candle.time(), candle.open(), candle.high(), candle.low(), candle.close());
}

// Get recent trades
TradesResponse trades = client.stock().intraday().getTrades("2330");
for (Trade trade : trades.data().subList(0, 5)) {
    System.out.printf("  Price: %.2f, Size: %d%n", trade.price(), trade.size());
}

// FutOpt (futures/options) data
Quote futoptQuote = client.futopt().intraday().getQuote("TXFC4");
System.out.printf("Futures Price: %.2f%n", futoptQuote.closePrice());
```

### WebSocket Streaming

```java
import tw.com.fugle.marketdata.FugleWebSocketClient;
import tw.com.fugle.marketdata.generated.StreamMessage;
import java.util.concurrent.TimeUnit;

// Create WebSocket client (pull mode with message queue)
FugleWebSocketClient ws = FugleWebSocketClient.builder()
    .apiKey("your-api-key")
    .stock()              // Stock market (default)
    .queueCapacity(100)   // Message queue capacity
    .build();

// Connect
ws.connect().join();
System.out.println("Connected!");

// Subscribe to channels
ws.subscribe("trades", "2330").join();
ws.subscribe("books", "2330").join();

// Poll messages (blocking with timeout)
while (true) {
    StreamMessage msg = ws.poll(1, TimeUnit.SECONDS);
    if (msg != null) {
        if (msg.event().equals("data")) {
            System.out.printf("[%s] %s%n", msg.channel(), msg.symbol());
            // Parse msg.dataJson() as needed
        }
    }

    // Check for errors
    if (ws.hasErrors()) {
        String error = ws.pollError();
        if (error != null) {
            System.err.println("Error: " + error);
        }
    }
}

// Disconnect (with timeout to avoid blocking)
ws.disconnect()
    .orTimeout(3, TimeUnit.SECONDS)
    .join();
```

## Authentication

Three authentication methods are supported:

```java
import tw.com.fugle.marketdata.FugleRestClient;

// 1. API Key (most common)
FugleRestClient client = FugleRestClient.builder()
    .apiKey("your-api-key")
    .build();

// 2. Bearer Token
FugleRestClient client = FugleRestClient.builder()
    .bearerToken("your-bearer-token")
    .build();

// 3. SDK Token
FugleRestClient client = FugleRestClient.builder()
    .sdkToken("your-sdk-token")
    .build();
```

## Advanced: Custom TLS / self-signed servers

For connecting to servers with a private CA (enterprise deployments) or
self-signed certs (dev / staging), the underlying UniFFI bindings expose
TLS-aware factory functions. `FugleRestClient.builder()` will gain
builder-style support in a future release; for now use the raw
`com.fugle.marketdata.uniffi` factories:

```java
import com.fugle.marketdata.uniffi.*;

import java.nio.file.Files;
import java.nio.file.Paths;

// Pin a custom CA (production-safe when your server cert is properly
// issued by this CA and has matching SANs).
byte[] caPem = Files.readAllBytes(Paths.get("/path/to/ca.crt"));
TlsConfigRecord tls = new TlsConfigRecord(caPem, false);
RestClient client = Marketdata.newRestClientWithApiKeyAndTls(
    "your-api-key", null /* baseUrl */, tls);

// Disable ALL TLS verification — dev / testing only. Exposes MITM risk.
TlsConfigRecord insecure = new TlsConfigRecord(null, true);
RestClient devClient = Marketdata.newRestClientWithApiKeyAndTls(
    "your-api-key", "wss://192.0.2.1/v1.0", insecure);
```

For WebSocket use `Marketdata.newWebsocketClientWithFullConfig(...)` —
same pattern, accepts `Option<TlsConfigRecord>` plus reconnect/health
check configs.

## API Reference

### FugleRestClient

#### Stock Intraday Methods

```java
// Real-time quote
Quote getQuote(String symbol)
CompletableFuture<Quote> getQuoteAsync(String symbol)

// Symbol information
Ticker getTicker(String symbol)
CompletableFuture<Ticker> getTickerAsync(String symbol)

// OHLCV candles (timeframe: "1", "5", "10", "15", "30", "60")
CandlesResponse getCandles(String symbol, String timeframe)
CompletableFuture<CandlesResponse> getCandlesAsync(String symbol, String timeframe)

// Trade history
TradesResponse getTrades(String symbol)
CompletableFuture<TradesResponse> getTradesAsync(String symbol)

// Volume by price
VolumesResponse getVolumes(String symbol)
CompletableFuture<VolumesResponse> getVolumesAsync(String symbol)
```

#### FutOpt Intraday Methods

```java
// Real-time quote
Quote getQuote(String symbol)
CompletableFuture<Quote> getQuoteAsync(String symbol)

// Contract information
Ticker getTicker(String symbol)
CompletableFuture<Ticker> getTickerAsync(String symbol)

// OHLCV candles
CandlesResponse getCandles(String symbol, String timeframe)
CompletableFuture<CandlesResponse> getCandlesAsync(String symbol, String timeframe)

// Trade history
TradesResponse getTrades(String symbol)
CompletableFuture<TradesResponse> getTradesAsync(String symbol)

// Volume by price
VolumesResponse getVolumes(String symbol)
CompletableFuture<VolumesResponse> getVolumesAsync(String symbol)

// Product listing (type: "F" for futures, "O" for options)
ProductsResponse getProducts(String type)
CompletableFuture<ProductsResponse> getProductsAsync(String type)
```

### FugleWebSocketClient

#### Builder Configuration

```java
FugleWebSocketClient.builder()
    .apiKey(String apiKey)           // Authentication
    .bearerToken(String token)       // Alternative auth
    .sdkToken(String token)          // Alternative auth
    .stock()                         // Use stock market
    .futopt()                        // Use futures/options market
    .queueCapacity(int capacity)     // Message queue size (default: 100)
    .build()
```

#### Methods

```java
// Connection management
CompletableFuture<Void> connect()             // Connect to server
CompletableFuture<Void> disconnect()          // Disconnect from server
boolean isConnected()                         // Check connection status
boolean isClosed()                            // Check if client is closed

// Subscription management
CompletableFuture<Void> subscribe(String channel, String symbol)  // Subscribe
CompletableFuture<Void> unsubscribe(String subscriptionId)        // Unsubscribe
List<Subscription> getSubscriptions()                              // List subscriptions

// Message polling (pull mode)
StreamMessage poll(long timeout, TimeUnit unit)  // Blocking poll with timeout
StreamMessage tryPoll()                          // Non-blocking poll (may return null)

// Error handling
boolean hasErrors()                           // Check if errors exist
String pollError()                            // Get next error message
```

#### StreamMessage Properties

```java
public class StreamMessage {
    String event()         // Event type: "subscribed", "snapshot", "data", "heartbeat"
    String channel()       // Channel name (may be null)
    String symbol()        // Symbol code (may be null)
    String dataJson()      // JSON data payload (may be null)
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

All API errors throw `FugleException`:

```java
import tw.com.fugle.marketdata.FugleRestClient;
import tw.com.fugle.marketdata.FugleException;

try {
    FugleRestClient client = FugleRestClient.builder()
        .apiKey("invalid-key")
        .build();
    Quote quote = client.stock().intraday().getQuote("2330");
} catch (FugleException e) {
    System.err.println("Error: " + e.getMessage());
    // Message format: "[2002] Authentication failed"
    e.printStackTrace();
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

```java
package tw.com.fugle.marketdata.examples;

import tw.com.fugle.marketdata.FugleRestClient;
import tw.com.fugle.marketdata.FugleException;
import tw.com.fugle.marketdata.generated.*;

public class RestExample {
    public static void main(String[] args) {
        String apiKey = System.getenv("FUGLE_API_KEY");
        if (apiKey == null || apiKey.isEmpty()) {
            System.out.println("Set FUGLE_API_KEY environment variable");
            System.exit(1);
        }

        try {
            // Create client
            FugleRestClient client = FugleRestClient.builder()
                .apiKey(apiKey)
                .build();

            // Stock data
            System.out.println("=== Stock Market Data ===");
            Quote quote = client.stock().intraday().getQuote("2330");
            System.out.printf("TSMC Quote: %.2f%n", quote.lastPrice());

            Ticker ticker = client.stock().intraday().getTicker("2330");
            System.out.printf("Ticker: %s%n", ticker.name());

            CandlesResponse candles = client.stock().intraday().getCandles("2330", "5");
            System.out.printf("Candles: %d entries%n", candles.data().size());

            // FutOpt data
            System.out.println("\n=== FutOpt Market Data ===");
            ProductsResponse products = client.futopt().intraday().getProducts("F");
            System.out.printf("Futures products: %d%n", products.data().size());

            // Async example
            System.out.println("\n=== Async Example ===");
            client.stock().intraday().getQuoteAsync("2317")
                .thenAccept(q -> System.out.printf("Async quote: %.2f%n", q.lastPrice()))
                .exceptionally(e -> {
                    System.err.println("Async error: " + e.getMessage());
                    return null;
                })
                .join();

        } catch (FugleException e) {
            System.err.println("Error: " + e.getMessage());
            e.printStackTrace();
        }
    }
}
```

### Full WebSocket Example

```java
package tw.com.fugle.marketdata.examples;

import tw.com.fugle.marketdata.FugleWebSocketClient;
import tw.com.fugle.marketdata.FugleException;
import tw.com.fugle.marketdata.generated.StreamMessage;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicBoolean;

public class WebSocketExample {
    private static final AtomicBoolean running = new AtomicBoolean(true);

    public static void main(String[] args) {
        String apiKey = System.getenv("FUGLE_API_KEY");
        if (apiKey == null || apiKey.isEmpty()) {
            System.out.println("Set FUGLE_API_KEY environment variable");
            System.exit(1);
        }

        // Setup shutdown hook
        Runtime.getRuntime().addShutdownHook(new Thread(() -> {
            System.out.println("\nShutdown requested...");
            running.set(false);
        }));

        FugleWebSocketClient client = null;
        int messageCount = 0;

        try {
            // Create client
            client = FugleWebSocketClient.builder()
                .apiKey(apiKey)
                .stock()
                .queueCapacity(100)
                .build();

            // Connect
            System.out.println("Connecting...");
            client.connect().join();
            System.out.println("Connected!");

            // Subscribe
            System.out.println("Subscribing to 2330 trades...");
            client.subscribe("trades", "2330").join();

            // Receive messages
            System.out.println("Waiting for messages (Ctrl+C to stop)...\n");
            long startTime = System.currentTimeMillis();
            long timeoutMs = 30_000;

            while (running.get()) {
                if (System.currentTimeMillis() - startTime > timeoutMs) {
                    System.out.println("\n30 seconds elapsed, stopping...");
                    break;
                }

                StreamMessage msg = client.poll(1, TimeUnit.SECONDS);
                if (msg != null) {
                    messageCount++;
                    System.out.printf("[%d] %s: %s - %s%n",
                        messageCount, msg.event(), msg.channel(), msg.symbol());
                }

                if (client.hasErrors()) {
                    String error = client.pollError();
                    if (error != null) {
                        System.err.println("Error: " + error);
                    }
                }
            }

        } catch (FugleException e) {
            System.err.println("Fugle API error: " + e.getMessage());
            e.printStackTrace();
        } catch (Exception e) {
            System.err.println("Error: " + e.getMessage());
            e.printStackTrace();
        } finally {
            System.out.printf("\nReceived %d messages%n", messageCount);
            System.out.println("Disconnecting...");

            if (client != null) {
                try {
                    client.disconnect()
                        .orTimeout(3, TimeUnit.SECONDS)
                        .handle((v, e) -> {
                            if (e != null) {
                                System.out.println("Disconnect timeout, forcing exit");
                            }
                            return null;
                        })
                        .join();
                } catch (Exception e) {
                    // ignore
                }
            }
            System.out.println("Done!");
        }
    }
}
```

## License

MIT
