# fugle-marketdata

Rust SDK for [Fugle](https://developer.fugle.tw) market data. Provides REST API and WebSocket clients for Taiwan stock and futures/options market data.

## Features

- **REST Client**: Synchronous HTTP client for market data queries
  - Stock intraday data (quote, ticker, candles, trades, volumes)
  - FutOpt (futures/options) intraday data
- **WebSocket Client**: real-time streaming, **sync by default**, optional
  tokio variant under `aio::` behind the `tokio-comp` feature
  - Stock channels: trades, candles, books, aggregates, indices
  - FutOpt channels: trades, candles, books, aggregates
  - Automatic reconnection with exponential backoff
  - Health check monitoring
- **Authentication**: API key, bearer token, or SDK token
- **Optional `tokio-comp` feature** — opt in for the async client. Sync
  consumers pay zero tokio in their `Cargo.lock`.

## Installation

```toml
# sync (no tokio in the dependency tree)
fugle-marketdata = "0.7"

# async (tokio + tokio-tungstenite)
fugle-marketdata = { version = "0.7", features = ["tokio-comp"] }

# opt in to structured tracing (no-op when off; zero binary cost)
fugle-marketdata = { version = "0.7", features = ["tokio-comp", "tracing"] }

# opt in to the `metrics` crate integration for Prometheus / OTLP / statsd
# exporters. Registers `fugle_marketdata_ws_messages_dropped_total` +
# `fugle_marketdata_ws_events_dropped_total` counters automatically.
fugle-marketdata = { version = "0.7", features = ["tokio-comp", "metrics"] }

# in-process WebSocket mock server (`testing::MockWsServer`) for tests
fugle-marketdata = { version = "0.7", features = ["test-utils"] }
```

Upgrading from 0.6? See [MIGRATION-0.7.md](../MIGRATION-0.7.md). Zero
breaking changes; four opt-in additions (`metrics` feature, multi-client
mock, transport-drop intent injection, REST/WebSocket dual-host docs)
plus doc/hygiene improvements (`WebSocketErrorKind::Http` mapping table,
`tracing_compat` internal-macro doc, `cargo public-api` snapshot).

Upgrading from 0.3? See [MIGRATION-0.4.md](../MIGRATION-0.4.md). The
notable defaults that changed: `ReconnectionConfig::default().enabled` is
now `true`, `disconnect()` is now a graceful 5 s drain (not
fire-and-forget), `message_buffer` default is 4096, `Auth` /
`ConnectionConfig` `Debug` redact secrets, and `SubscribeRequest::trades`
/ `::candles` / `::books` / `::aggregates` were removed in favour of
`SubscribeRequest::new(Channel::*, symbol)`.

## Quick Start

### REST API

```rust,no_run
use fugle_marketdata::{RestClient, Auth};

# fn main() -> Result<(), fugle_marketdata::MarketDataError> {
let client = RestClient::new(Auth::ApiKey(
    std::env::var("FUGLE_API_KEY").expect("FUGLE_API_KEY not set")
));

// Stock quote
let quote = client.stock().intraday().quote().symbol("2330").send()?;
println!("TSMC close price: {:?}", quote.close_price);

// Intraday candles (5-minute)
let candles = client.stock().intraday().candles()
    .symbol("2330")
    .timeframe("5")
    .send()?;
println!("Candles: {} entries", candles.data.len());

// FutOpt quote
let futopt_quote = client.futopt().intraday().quote()
    .symbol("TXFC4")
    .send()?;
println!("Futures close price: {:?}", futopt_quote.close_price);
# Ok(())
# }
```

### WebSocket Streaming (sync, default)

```rust,no_run
use fugle_marketdata::{
    AuthRequest, Channel, WebSocketClient,
    websocket::{ConnectionConfig, StockSubscription},
};
use std::time::Duration;

# fn run() -> Result<(), fugle_marketdata::MarketDataError> {
let config = ConnectionConfig::fugle_stock(
    AuthRequest::with_api_key(
        std::env::var("FUGLE_API_KEY").expect("FUGLE_API_KEY not set")
    )
);
let client = WebSocketClient::new(config);
client.connect()?;

// Single symbol
client.subscribe(StockSubscription::new(Channel::Trades, "2330"))?;

// Batch — one frame subscribes N symbols
client.subscribe(StockSubscription::new(
    Channel::Aggregates,
    vec!["2330", "0050", "2603"],
))?;

// Odd-lot session: builder modifier applies to every symbol
client.subscribe(
    StockSubscription::new(Channel::Trades, "2330").with_odd_lot(true)
)?;

let messages = client.messages();
for _ in 0..10 {
    if let Ok(Some(msg)) = messages.receive_timeout(Duration::from_secs(5)) {
        if msg.is_data() {
            println!("Data: {:?} - {:?}", msg.channel, msg.symbol);
        }
    }
}

// Unsubscribe by server id(s) — accepts a single id or a batch.
client.unsubscribe(["server-id-1", "server-id-2"])?;

client.disconnect()?;
# Ok(())
# }
```

### WebSocket Streaming (async, `features = ["tokio-comp"]`)

Same API surface — replace `WebSocketClient` with `aio::WebSocketClient`
and add `.await`:

```rust,ignore
use fugle_marketdata::aio::WebSocketClient;
use fugle_marketdata::{AuthRequest, Channel, websocket::{ConnectionConfig, StockSubscription}};

# async fn run() -> Result<(), fugle_marketdata::MarketDataError> {
let config = ConnectionConfig::fugle_stock(AuthRequest::with_api_key("..."));
let client = WebSocketClient::new(config);
client.connect().await?;
client.subscribe(StockSubscription::new(Channel::Trades, "2330")).await?;

let mut stream = client.message_stream();
while let Some(msg) = stream.recv().await {
    if msg.is_data() { /* ... */ }
}
client.disconnect().await?;
# Ok(())
# }
```

The async client lives under `fugle_marketdata::aio::` and is only available
when the `tokio-comp` feature is enabled. Upgrading from 0.2: see
[MIGRATION-0.3.md](../MIGRATION-0.3.md).

## Authentication

```rust,no_run
use fugle_marketdata::{Auth, AuthRequest};

// REST authentication
let _ = Auth::ApiKey("your-api-key".to_string());
let _ = Auth::BearerToken("your-bearer-token".to_string());
let _ = Auth::SdkToken("your-sdk-token".to_string());

// WebSocket authentication
let _ = AuthRequest::with_api_key("your-api-key");
let _ = AuthRequest::with_token("your-bearer-token");
let _ = AuthRequest::with_sdk_token("your-sdk-token");
```

## Configuration

### Reconnection

`ReconnectionConfig::default().enabled` is **`true`** as of 0.4. Rust
callers on the `WebSocketClient::new(config)` happy path get
auto-reconnect with no opt-in; bindings (Python / Node / UniFFI / Go /
Java / C++ / C#) call `ReconnectionConfig::disabled()` at the FFI
boundary so end-user behaviour is preserved.

```rust,no_run
use fugle_marketdata::websocket::ReconnectionConfig;
use std::time::Duration;

# fn main() -> Result<(), Box<dyn std::error::Error>> {
// Default — auto-reconnect enabled (5 attempts, 1 s → 60 s exponential).
let reconnect = ReconnectionConfig::default();

// Custom — explicit `new()` also enables auto-reconnect.
let reconnect = ReconnectionConfig::new(
    10,                              // max_attempts
    Duration::from_millis(2_000),    // initial_delay (min 100ms)
    Duration::from_millis(120_000),  // max_delay
)?;

// Opt out — matches old `fugle-marketdata-{python,node}` semantics.
let reconnect = ReconnectionConfig::disabled();
# drop(reconnect);
# Ok(())
# }
```

### Health Check

The SDK uses passive activity detection at the WebSocket read site — no
background task, no protocol-level pings. The dispatch loop wraps each
`ws_read.next()` in `tokio::time::timeout(heartbeat_timeout, ...)` and emits
`ConnectionEvent::HeartbeatTimeout` when the timer fires, which then triggers
the auto-reconnect path.

```rust,no_run
use fugle_marketdata::websocket::HealthCheckConfig;
use std::time::Duration;

# fn main() -> Result<(), Box<dyn std::error::Error>> {
// Default: enabled=true, heartbeat_timeout=35s (server's 30s + 5s buffer)
let health = HealthCheckConfig::default();

// Custom timeout (the floor is 5s, but values below the live server's 30s
// heartbeat period will cause repeated false disconnects)
let health = HealthCheckConfig::with_timeout(Duration::from_secs(45))?;

// Opt out (discouraged — stalled connections won't surface until the OS
// times out the underlying TCP, typically hours later)
let health = HealthCheckConfig::disabled();
# drop(health);
# Ok(())
# }
```

### Full Configuration

For complete control over connection, reconnection, and health-check
parameters, use `WebSocketClient::with_full_config`:

```rust,no_run
use fugle_marketdata::{
    AuthRequest, WebSocketClient,
    websocket::{ConnectionConfig, HealthCheckConfig, ReconnectionConfig},
};
use std::time::Duration;

# fn main() -> Result<(), Box<dyn std::error::Error>> {
let auth = AuthRequest::with_api_key("your-api-key");
let connection = ConnectionConfig::fugle_stock(auth);
let reconnect = ReconnectionConfig::new(
    10,
    Duration::from_secs(2),
    Duration::from_secs(120),
)?;
let health = HealthCheckConfig::with_timeout(Duration::from_secs(45))?;
let client = WebSocketClient::with_full_config(connection, reconnect, health);
# drop(client);
# Ok(())
# }
```

### Custom endpoints (staging / proxy / mock server)

Both transports accept a custom base URL. It carries the **host and path
prefix only** — the SDK appends the version segment. A base URL that already
ends in a version segment is rejected.

> **Changed in 0.8.0.** 0.6.0–0.7.x required the opposite (the version segment
> had to be included). See [MIGRATION-0.8.md](../MIGRATION-0.8.md).

REST — chainable setter:

```rust,no_run
use fugle_marketdata::{Auth, RestClient};

let client = RestClient::new(Auth::SdkToken("t".into()))
    .base_url("https://staging.fugle.tw/marketdata");

assert_eq!(
    client.resolved_base_url(),
    "https://staging.fugle.tw/marketdata/v1.0"
);
```

WebSocket — `WebSocketFactory` mirrors the JS / Python SDK shape:
supply one base URL, get both stock and futopt endpoints.

```rust,no_run
use fugle_marketdata::{AuthRequest, WebSocketClient, WebSocketFactory};

# fn main() -> Result<(), fugle_marketdata::MarketDataError> {
let factory = WebSocketFactory::new()
    .base_url("wss://staging.fugle.tw/marketdata")
    .auth(AuthRequest::with_api_key("k"));

let stock_client  = WebSocketClient::new(factory.stock()?.build());
let futopt_client = WebSocketClient::new(factory.futopt()?.build());
# drop((stock_client, futopt_client));
# Ok(())
# }
```

Streaming versions are per-product options, not something written into the
base URL. futopt defaults to `v1.1`, which delivers trial-matching (試撮)
frames — branch on `is_trial` before acting on a price, or pin `V1_0`:

```rust,no_run
use fugle_marketdata::{AuthRequest, WebSocketFactory};
use fugle_marketdata::websocket::FutOptVersion;

# fn main() -> Result<(), fugle_marketdata::MarketDataError> {
let cfg = WebSocketFactory::new()
    .futopt_version(FutOptVersion::V1_0)
    .auth(AuthRequest::with_api_key("k"))
    .futopt()?
    .build();
# drop(cfg);
# Ok(())
# }
```

For full control, `ConnectionConfig::new(url, auth)` accepts the
fully-qualified WebSocket URL. The `urls` module exposes the canonical
constants and roots:

```rust,no_run
use fugle_marketdata::urls;

let _ = urls::STOCK_WS;        // wss://api.fugle.tw/marketdata/v1.0/stock/streaming
let _ = urls::WS_BASE_ROOT;    // wss://api.fugle.tw/marketdata
let _ = urls::API_VERSION;     // v1.0
```

### REST retry

```rust,no_run
use fugle_marketdata::{Auth, RestClient, RetryPolicy};

let client = RestClient::new(Auth::SdkToken("t".into()))
    .with_retry(RetryPolicy::conservative());  // 3 attempts, 100 ms initial, 2 s ceiling

// or build your own
use std::time::Duration;
let client = RestClient::new(Auth::SdkToken("t".into()))
    .with_retry(RetryPolicy::new(5, Duration::from_millis(250), Duration::from_secs(10)));
# drop(client);
```

Off by default — observability use cases need real failures visible.
Retries only errors classified by `MarketDataError::is_retryable()`
(HTTP 429, HTTP 5xx, transport timeouts, connection errors). Exhausted
retries return the last error verbatim.

### Graceful shutdown

`WebSocketClient::disconnect()` (both sync and async) defaults to a
**5 second drain timeout**: it signals the dispatch loop to stop, drops
the writer-side sender so any queued frames flush, sends the WebSocket
Close frame, awaits the peer's Close acknowledgement, and only then
forcibly aborts the background tasks. Pass an explicit budget for
SIGTERM-style scenarios:

```rust,no_run
use fugle_marketdata::WebSocketClient;
use std::time::Duration;

# fn run(client: WebSocketClient) -> Result<(), fugle_marketdata::MarketDataError> {
client.shutdown_with_timeout(Duration::from_millis(100))?;  // tight budget
# Ok(())
# }
```

`Duration::ZERO` is valid — same fire-and-forget shape as 0.3.

### Tracing

Opt in via the `tracing` feature. Hot-path `debug!` for received frames,
lifecycle `info!` / `warn!` for connect / auth / reconnect /
heartbeat / saturation, `error!` for runtime-init / close-frame
failures. `#[tracing::instrument]` spans on
`ws.connect` / `ws.subscribe` / `ws.unsubscribe` / `ws.disconnect`
(cold path only — zero overhead per frame).

```rust,ignore
fn main() {
    tracing_subscriber::fmt::init();
    // ... existing client code unchanged
}
```

### Backpressure & introspection

```rust,no_run
use fugle_marketdata::{Channel, WebSocketClient};

# fn run(client: WebSocketClient) {
let dropped: u64 = client.messages_dropped_total();
let count: usize = client.subscription_count();
let on:    bool  = client.is_subscribed(&Channel::Trades, "2330");
# drop((dropped, count, on));
# }
```

Default message-channel cap is 4096 (drop-newest backpressure). Tune
with `ConnectionConfig::builder(...).message_buffer(N)`.

## Error Handling

All operations return `Result<T, MarketDataError>`:

```rust,no_run
use fugle_marketdata::{MarketDataError, RestClient, Auth};

# fn main() {
# let client = RestClient::new(Auth::ApiKey("key".into()));
match client.stock().intraday().quote().symbol("2330").send() {
    Ok(quote) => println!("Price: {:?}", quote.close_price),
    Err(MarketDataError::AuthError { msg }) => eprintln!("Auth failed: {}", msg),
    Err(MarketDataError::ApiError { status, message }) => eprintln!("API {}: {}", status, message),
    Err(MarketDataError::TimeoutError { operation }) => eprintln!("Timeout: {}", operation),
    Err(e) => eprintln!("Error: {}", e),
}
# }
```

Error codes (for FFI consumers):

| Code | Variant |
|------|---------|
| 1001 | `InvalidSymbol` |
| 1002 | `DeserializationError` |
| 1003 | `RuntimeError` |
| 1004 | `ConfigError` |
| 2001 | `ConnectionError` |
| 2002 | `AuthError` |
| 2003 | `ApiError` |
| 2010 | `ClientClosed` |
| 3001 | `TimeoutError` |
| 3002 | `WebSocketError` |
| 9999 | `Other` |

## API Reference

See the [API documentation on docs.rs](https://docs.rs/fugle-marketdata) for the full type catalogue.

### REST Endpoints

**Stock intraday** — `client.stock().intraday()`:
- `.quote()` — real-time quote
- `.ticker()` — symbol information
- `.candles()` — OHLCV candles
- `.trades()` — trade history
- `.volumes()` — volume by price

**FutOpt intraday** — `client.futopt().intraday()`:
- `.quote()`, `.ticker()`, `.tickers()`, `.candles()`, `.trades()`, `.volumes()`, `.products()`

### WebSocket Channels

| Channel | Description |
|---------|-------------|
| `Trades` | Real-time trade executions |
| `Candles` | Real-time candlestick updates |
| `Books` | Order book (5 levels bid/ask) |
| `Aggregates` | Aggregated market data |
| `Indices` | Index values (stock only) |

## Architecture

`fugle-marketdata` is a thin facade over the [`fugle-marketdata-core`](https://docs.rs/fugle-marketdata-core) kernel crate. The kernel is the same library used by the Python, Node.js, and other language bindings of the official Fugle SDK. End users should depend on `fugle-marketdata`; advanced users who need direct kernel access can depend on `fugle-marketdata-core`.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
