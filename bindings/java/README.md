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

// Methods return the server's JSON body as a String. Decode it with
// whichever JSON library you already use — Jackson shown here.
ObjectMapper mapper = new ObjectMapper();

// Get stock quote
JsonNode quote = mapper.readTree(client.stock().intraday().getQuote("2330"));
System.out.printf("TSMC Price: %.2f%n", quote.get("closePrice").asDouble());
System.out.printf("Change: %.2f%n", quote.get("change").asDouble());
System.out.printf("Volume: %d%n", quote.get("total").get("tradeVolume").asLong());

// Fields the server omits are absent, not null — check before reading.
if (quote.has("referencePrice")) {
    System.out.printf("Reference: %.2f%n", quote.get("referencePrice").asDouble());
}

// Get stock ticker info
JsonNode ticker = mapper.readTree(client.stock().intraday().getTicker("2330"));
System.out.printf("Name: %s%n", ticker.get("name").asText());

// Get intraday candles. With no params (or null) the server default
// timeframe (1 minute) applies; pass a StockCandlesParams for another
// timeframe (minutes: "1", "5", "10", "15", "30", "60").
JsonNode oneMinute = mapper.readTree(client.stock().intraday().getCandles("2330"));
JsonNode candles = mapper.readTree(client.stock().intraday().getCandles("2330",
    new StockCandlesParams("5", null, null)));
for (JsonNode candle : candles.get("data")) {
    System.out.printf("  %s: O=%.2f C=%.2f%n",
        candle.get("date").asText(), candle.get("open").asDouble(),
        candle.get("close").asDouble());
}

// Get recent trades, filtered to odd-lot with a limit
JsonNode trades = mapper.readTree(client.stock().intraday().getTrades("2330",
    new StockTradesParams(true, null, 20, null, null)));
for (JsonNode trade : trades.get("data")) {
    System.out.printf("  Price: %.2f, Size: %d%n",
        trade.get("price").asDouble(), trade.get("size").asLong());
}

// Market movers - direction and change are required positional arguments
JsonNode movers = mapper.readTree(
    client.stock().snapshot().moversSync("TSE", "up", "percent", null));

// FutOpt (futures/options) data
JsonNode futoptQuote = mapper.readTree(client.futopt().intraday().getQuote("TXFC4"));
System.out.printf("Futures Price: %.2f%n", futoptQuote.get("closePrice").asDouble());

// After-hours session for a FutOpt contract
JsonNode futoptAfterHours = mapper.readTree(client.futopt().intraday().getQuote("TXFC4",
    new AfterHoursParams(true)));
```

Every REST method's optional filters live in one generated `*Params` record per
endpoint (`StockTradesParams`, `MoversParams`, `CorporateActionsParams`, ...);
pass `null` for none of them. See the generated Javadoc for each record's
fields — they mirror the server's query parameters 1:1.

### WebSocket Streaming

```java
import tw.com.fugle.marketdata.FugleWebSocketClient;
import tw.com.fugle.marketdata.generated.StreamMessage;
import tw.com.fugle.marketdata.generated.SubscribeOptions;
import java.util.List;
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

// Subscribe multiple symbols in a single frame
ws.subscribe("trades", List.of("2330", "2317")).join();

// Stock endpoint only: intraday odd-lot subscription
ws.subscribe("trades", List.of("2330"), new SubscribeOptions(null, true)).join();

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

`FugleWebSocketClient.builder()` takes the same three credentials
(`apiKey`, `bearerToken` or `sdkToken`). Do not log them, nor the generated
`tw.com.fugle.marketdata.generated.CredentialsRecord` they reach the native client in: its fields are
the secrets themselves.

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

Every method has a single-argument convenience overload (equivalent to
passing `null` params) alongside the params overload shown below.

```java
// Real-time quote (params: OddLotParams, or null)
String getQuote(String symbol, OddLotParams params)
CompletableFuture<String> getQuoteAsync(String symbol, OddLotParams params)

// Symbol information (params: OddLotParams, or null)
String getTicker(String symbol, OddLotParams params)
CompletableFuture<String> getTickerAsync(String symbol, OddLotParams params)

// OHLCV candles (params: StockCandlesParams - timeframe "1"/"5"/"10"/"15"/"30"/"60"
// minutes, oddLot, sort; null/no timeframe takes the server default, 1 minute)
String getCandles(String symbol, StockCandlesParams params)
CompletableFuture<String> getCandlesAsync(String symbol, StockCandlesParams params)

// Trade history (params: StockTradesParams - oddLot, offset, limit, sort, isTrial)
String getTrades(String symbol, StockTradesParams params)
CompletableFuture<String> getTradesAsync(String symbol, StockTradesParams params)

// Volume by price (params: OddLotParams, or null)
String getVolumes(String symbol, OddLotParams params)
CompletableFuture<String> getVolumesAsync(String symbol, OddLotParams params)
```

Stock historical/snapshot/technical/corporate-actions/ownership methods are
exposed directly from the generated clients (`client.stock().historical()`,
`.snapshot()`, `.technical()`, `.corporateActions()`), plus a wrapped
`.ownership()` with the same single-argument convenience overloads (e.g.
`getEtfHoldings(String symbol, OwnershipParams params)`). See the generated
Javadoc for their `*Params` record and exact signature (e.g.
`getMovers(String market, String direction, String change, MoversParams params)`,
`getSma(String symbol, Integer period, TechnicalParams params)`).

#### FutOpt Intraday Methods

```java
// Real-time quote (params: AfterHoursParams, or null for regular hours)
String getQuote(String symbol, AfterHoursParams params)
CompletableFuture<String> getQuoteAsync(String symbol, AfterHoursParams params)

// Contract information (params: AfterHoursParams, or null for regular hours)
String getTicker(String symbol, AfterHoursParams params)
CompletableFuture<String> getTickerAsync(String symbol, AfterHoursParams params)

// Product listing (type: "F" for futures, "O" for options;
// params: FutOptProductsParams, or null)
String getProducts(String type, FutOptProductsParams params)
CompletableFuture<String> getProductsAsync(String type, FutOptProductsParams params)
```

`FutOptIntradayClientWrapper` currently wraps only the three methods above;
`getCandles`, `getTrades`, `getVolumes` and the tickers listing exist on the
generated `FutOptIntradayClient` (e.g.
`candlesSync(String symbol, FutOptCandlesParams params)`) but are not yet
exposed through `client.futopt().intraday()` — a pre-existing gap, not
introduced by the params-record change in this release.

### FugleWebSocketClient

#### Builder Configuration

```java
FugleWebSocketClient.builder()
    .apiKey(String apiKey)           // Authentication
    .bearerToken(String token)       // Alternative auth
    .sdkToken(String token)          // Alternative auth
    .stock()                         // Use stock market
    .futopt()                        // Use futures/options market
    .queueCapacity(int capacity)     // Pull-mode BlockingQueue size (default: 10000)
    .messageOverflow(MessageOverflow overflow)  // DROP_NEWEST (default) or UNBOUNDED
    .messageBuffer(int buffer)       // Unread messages before overflow applies (default: 4096)
    .reconnect(ReconnectOptions options)  // Auto-reconnect tuning (default: on, unlimited attempts)
    .healthCheck(HealthCheckOptions options)  // Liveness detection tuning (default: on, 35s timeout)
    .build()
```

After an unexpected drop the client reconnects on its own with exponential
backoff (1 s doubling up to 60 s), without an attempt limit, and subscribes
again once it is back. The server closing with 1000 or a 4xxx code (e.g. an
auth failure) never triggers a reconnect. Configure it with `reconnect(...)`:

```java
// Stop after 10 attempts; onReconnectFailed fires once the last one fails
.reconnect(ReconnectOptions.builder().maxAttempts(10).build())

// Turn auto-reconnect off
.reconnect(ReconnectOptions.builder().enabled(false).build())
```

`maxAttempts` 0 means unlimited (the default); `initialDelayMs` (default 1000,
min 100) and `maxDelayMs` (default 60000) tune the backoff.

Liveness detection is on by default: when the connection stays silent too
long it is declared dead and auto-reconnect takes over. Configure it with
`healthCheck(...)`:

```java
// Default: heartbeatTimeoutMs 35000
.healthCheck(HealthCheckOptions.builder().heartbeatTimeoutMs(60000L).build())

// Probe mode: confirm a silent connection with a ping before declaring it
// dead, instead of guessing off a timeout
.healthCheck(HealthCheckOptions.builder().probeEnabled(true).idleProbeAfterMs(10000L).build())

// Turn it off
.healthCheck(HealthCheckOptions.builder().enabled(false).build())
```

`heartbeatTimeoutMs` (default 35000, min 5000) does not apply when
`probeEnabled` is true. With `probeEnabled`, `idleProbeAfterMs` (default
30000, min 5000) and `probeTimeoutMs` (default 5000, min 1000) control when a
ping is sent and how long to wait for a reply. With the defaults, detection
stays at 35 s and no ping is sent while the server's heartbeat is on time;
lowering `idleProbeAfterMs` detects faster at the cost of pinging the server
more often. Probing does not detect a half-open connection (the server still
sends, but our writes no longer reach it). See
[HealthCheckConfig / HealthCheckOptions](../../docs/configuration.md#healthcheckconfig--healthcheckoptions)
for the full trade-offs and estimated server cost.

`ping(state)` is fire-and-forget, as in the old SDK, with the pong delivered
to `onMessage`/the pull queue. `measureLatency()` sends one ping, awaits its
pong and returns the round-trip time in milliseconds; it works regardless of
`probeEnabled` and sends nothing in the background otherwise.

`messageOverflow`/`messageBuffer` configure the client's internal message
queue (shared by both callback and pull mode): with the default
`DROP_NEWEST`, new messages are dropped once `messageBuffer` messages are
unread, and `WebSocketListener.onMessagesDropped(count)` reports how many.
`UNBOUNDED` never drops, at the cost of unbounded memory growth if the
listener falls behind.

In callback mode, an exception thrown by a listener method does not stop
later events. It is reported to `onError` with code 3004 (`CALLBACK_FAILED`,
source kind `CLIENT`) and a message naming the method, the exception type and
message, and the number of failures, e.g.
`Listener onMessage threw java.lang.RuntimeException: boom (1 in the last 1s)`.
The first failure is reported at once, later ones at most once per second,
counting the failures since the previous report; failures after the last
report are not reported on their own. An exception thrown by `onError` itself
is logged through `java.util.logging` (WARNING, logger
`tw.com.fugle.marketdata.FugleWebSocketClient`) and not re-reported.

In pull mode a full `queueCapacity` queue makes the client wait for `poll()`
rather than discard messages, so what you do not keep up with is dropped
there instead: counted in `messagesDroppedTotal()` and reported through
`pollError()` as `Dropped <count> message(s): listener fell behind`.

#### Methods

```java
// Connection management
CompletableFuture<Void> connect()             // Connect to server
CompletableFuture<Void> disconnect()          // Disconnect from server
boolean isConnected()                         // Check connection status
boolean isClosed()                            // Check if client is closed
long messagesDroppedTotal()                   // Messages dropped this connection (DROP_NEWEST only)

// Subscription management
CompletableFuture<Void> subscribe(String channel, String symbol)                                  // Subscribe one symbol
CompletableFuture<Void> subscribe(String channel, String symbol, boolean afterHours)              // FutOpt after-hours (FutOpt endpoint only; 1005 on Stock)
CompletableFuture<Void> subscribe(String channel, List<String> symbols)                           // Subscribe multiple symbols in one frame
CompletableFuture<Void> subscribe(String channel, List<String> symbols, SubscribeOptions opts)    // + afterHours (FutOpt only) / intradayOddLot (Stock only); empty symbols: 1005
CompletableFuture<Void> unsubscribe(String channel, String symbol)                                // Unsubscribe one symbol
CompletableFuture<Void> unsubscribe(String channel, String symbol, boolean afterHours)            // Same afterHours as subscribe
CompletableFuture<Void> unsubscribe(String channel, List<String> symbols)                         // Unsubscribe multiple symbols
CompletableFuture<Void> unsubscribe(String channel, List<String> symbols, SubscribeOptions opts)  // Same opts as subscribe
CompletableFuture<Void> unsubscribe(List<String> ids)                                             // Unsubscribe by server ids (empty: 1005)
List<Subscription> getSubscriptions()                              // List subscriptions

// Message polling (pull mode)
StreamMessage poll(long timeout, TimeUnit unit)  // Blocking poll with timeout
StreamMessage tryPoll()                          // Non-blocking poll (may return null)

// Error handling
boolean hasErrors()                           // Check if errors exist
String pollError()                            // Get next error message

// Ping is fire-and-forget; the pong (if any) arrives via onMessage/the pull queue
CompletableFuture<Void> ping(String state)
// measureLatency sends a ping and awaits the matching pong; returns the round-trip time in ms (see Health Check above)
CompletableFuture<Double> measureLatency(Long timeoutMs)
CompletableFuture<Double> measureLatency()    // Same, with the default 5000ms timeout
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
    String quote = client.stock().intraday().getQuote("2330");
} catch (FugleException e) {
    System.err.println("Error [" + e.getCode() + "] " + e.getSourceKind() + ": " + e.getMessage());
    if (e.getStatus() != null) {
        System.err.println("HTTP " + e.getStatus() + ": " + e.getBody());
    }
}
```

| Getter | Type | |
|---|---|---|
| `getCode()` | `Integer` | Error code (table below) |
| `getSourceKind()` | `ErrorSourceKind` | `NETWORK`, `PROTOCOL`, `AUTH`, `RATE_LIMIT` or `CLIENT` |
| `getMessage()` | `String` | Human-readable message |
| `getStatus()` | `Integer` | HTTP status, or null |
| `getBody()` | `String` | Raw HTTP response body (REST), or null |
| `getRequestId()` | `String` | `x-request-id` response header, or null |
| `getHeaders()` | `Map<String, String>` | HTTP response headers, lowercase names (REST; else empty) |
| `getInfo()` | `ErrorInfo` | All of the above as one record |

The getters return null (`getHeaders()` an empty map) for a `FugleException`
not raised from an SDK error. `WebSocketListener.onError` receives the same
`ErrorInfo`. See the [error reference](https://github.com/fugle-dev/fugle-marketdata-sdk/blob/main/docs/errors.md) for all languages.

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
| 3004 | CallbackFailed | A listener method threw (`onError` only) |
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
            String quote = client.stock().intraday().getQuote("2330");
            System.out.printf("TSMC Quote: %s%n", quote);  // raw JSON

            String ticker = client.stock().intraday().getTicker("2330");
            System.out.printf("Ticker: %s%n", ticker);

            String candles = client.stock().intraday().getCandles("2330",
                new StockCandlesParams("5", null, null));
            System.out.printf("Candles: %d entries%n",
                new ObjectMapper().readTree(candles).get("data").size());

            // FutOpt data
            System.out.println("\n=== FutOpt Market Data ===");
            ProductsResponse products = client.futopt().intraday().getProducts("F");
            System.out.printf("Futures products: %d%n",
                new ObjectMapper().readTree(products).get("data").size());

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
