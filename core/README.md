# marketdata-core

Rust library for Fugle market data streaming. Provides REST API and WebSocket clients for Taiwan stock and futures/options market data.

## Features

- **REST Client**: Synchronous HTTP client for market data queries
  - Stock intraday data (quote, ticker, candles, trades, volumes)
  - FutOpt (futures/options) intraday data
- **WebSocket Client**: Async real-time streaming
  - Stock channels: trades, candles, books, aggregates, indices
  - FutOpt channels: trades, candles, books, aggregates
  - Automatic reconnection with exponential backoff
  - Health check monitoring
- **Authentication**: API key, bearer token, or SDK token
- **FFI-ready**: Error codes and types designed for Python/JavaScript bindings

## Quick Start

### Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
marketdata-core = { path = "../marketdata-core" }
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

### REST API

```rust,ignore
use marketdata_core::{RestClient, Auth};

fn main() -> Result<(), marketdata_core::MarketDataError> {
    // Create client with API key authentication
    let client = RestClient::new(Auth::ApiKey(
        std::env::var("FUGLE_API_KEY").expect("FUGLE_API_KEY not set")
    ));

    // Get stock quote
    let quote = client.stock().intraday().quote().symbol("2330").send()?;
    println!("TSMC Quote:");
    println!("  Price: {:?}", quote.close_price);
    println!("  Change: {:?}", quote.change);
    println!("  Volume: {:?}", quote.total.trade_volume);

    // Get stock ticker info
    let ticker = client.stock().intraday().ticker().symbol("2330").send()?;
    println!("\nTicker: {} - {}", ticker.symbol, ticker.name);

    // Get intraday candles (5-minute)
    let candles = client.stock().intraday().candles()
        .symbol("2330")
        .timeframe("5")
        .send()?;
    println!("\nCandles: {} entries", candles.data.len());

    // Get FutOpt quote
    let futopt_quote = client.futopt().intraday().quote()
        .symbol("TXF202502")
        .send()?;
    println!("\nFutures Quote: {:?}", futopt_quote.close_price);

    Ok(())
}
```

### WebSocket Streaming

```rust,ignore
use marketdata_core::{
    AuthRequest, Channel, WebSocketClient,
    websocket::{ConnectionConfig, ConnectionEvent},
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), marketdata_core::MarketDataError> {
    // Create WebSocket client
    let config = ConnectionConfig::fugle_stock(
        AuthRequest::with_api_key(
            std::env::var("FUGLE_API_KEY").expect("FUGLE_API_KEY not set")
        )
    );
    let client = WebSocketClient::new(config);

    // Connect and authenticate
    client.connect().await?;
    println!("Connected to WebSocket");

    // Subscribe to channels — single symbol
    use marketdata_core::websocket::channels::StockSubscription;
    client.subscribe(StockSubscription::new(Channel::Trades, "2330")).await?;
    client.subscribe(StockSubscription::new(Channel::Books, "2330")).await?;

    // Batch subscribe — one frame, N symbols
    client.subscribe(StockSubscription::new(
        Channel::Aggregates,
        vec!["2330", "0050", "2603"],
    )).await?;
    println!("Subscribed to 2330 trades + books and 3-symbol aggregates batch");

    // Get message receiver
    let messages = client.messages();

    // Process messages in a separate task
    let msg_handle = tokio::spawn(async move {
        for _ in 0..10 {
            // receive_timeout returns Result<Option<msg>, _>:
            //   Ok(Some(msg)) — message received
            //   Ok(None)      — timeout elapsed
            //   Err(_)        — channel closed
            match messages.receive_timeout(std::time::Duration::from_secs(5)) {
                Ok(Some(msg)) => {
                    if msg.is_data() {
                        println!("Data: {:?} - {:?}", msg.channel, msg.symbol);
                    }
                }
                Ok(None) => continue,
                Err(_) => break,
            }
        }
    });

    // Wait for messages
    msg_handle.await.ok();

    // Graceful disconnect
    client.disconnect().await?;
    println!("Disconnected");

    Ok(())
}
```

## Authentication

Three authentication methods are supported:

```rust,ignore
use marketdata_core::Auth;

// 1. API Key (most common)
let auth = Auth::ApiKey("your-api-key".to_string());

// 2. Bearer Token
let auth = Auth::BearerToken("your-bearer-token".to_string());

// 3. SDK Token
let auth = Auth::SdkToken("your-sdk-token".to_string());
```

For WebSocket:

```rust,ignore
use marketdata_core::AuthRequest;

let auth = AuthRequest::with_api_key("your-api-key");
let auth = AuthRequest::with_token("your-bearer-token");
let auth = AuthRequest::with_sdk_token("your-sdk-token");
```

## Configuration

### ReconnectionConfig

Control WebSocket automatic reconnection behavior with exponential backoff:

```rust,ignore
use marketdata_core::websocket::ReconnectionConfig;
use std::time::Duration;

let reconnect = ReconnectionConfig::new(
    10,                              // max_attempts (min: 1)
    Duration::from_millis(2_000),    // initial_delay (min: 100ms)
    Duration::from_millis(120_000),  // max_delay
)?;
```

**Parameters:**
- `max_attempts` (u32): Maximum reconnection attempts (default: 5, range: 1+)
- `initial_delay` (Duration): Initial delay for exponential backoff (default: 1000ms, min: 100ms)
- `max_delay` (Duration): Maximum delay cap (default: 60000ms)

**Validation:**
- `max_attempts` must be >= 1
- `initial_delay` must be >= 100ms (prevents connection storms)
- `max_delay` must be >= `initial_delay` (logical constraint)

### HealthCheckConfig

Control connection liveness detection. The SDK declares the connection dead and
triggers reconnect when no inbound frame arrives within `heartbeat_timeout`.

The SDK uses **passive activity detection at the WebSocket read site** — no
background task, no atomic timestamps, no protocol-level pings. The dispatch
loop wraps each `ws_read.next()` in `tokio::time::timeout(heartbeat_timeout, ...)`
and emits `ConnectionEvent::HeartbeatTimeout` when the timer fires.

```rust,ignore
use marketdata_core::websocket::HealthCheckConfig;
use std::time::Duration;

// Recommended: with_timeout validates against the 5s sanity floor.
let health = HealthCheckConfig::with_timeout(Duration::from_secs(35))?;

// Default: enabled=true, heartbeat_timeout=35s (Fugle server's 30s heartbeat
// + 5s buffer).
let health = HealthCheckConfig::default();

// Opt out (discouraged — stalled connections won't surface until OS times
// out the underlying TCP, typically hours later).
let health = HealthCheckConfig::disabled();
```

**Parameters:**
- `enabled` (bool): Whether liveness detection is active. Default `true` in 3.0
  (was `false` in 2.x).
- `heartbeat_timeout` (Duration): Maximum allowed gap between inbound frames.
  Default 35s. Floor 5s — but values below the live server's 30s heartbeat
  period will cause repeated false disconnects.

**Migration from 2.x:** `interval × max_missed_pongs` collapsed into a single
`heartbeat_timeout` field. If you used `HealthCheckConfig::new(enabled, interval,
max_missed_pongs)`, the equivalent is `HealthCheckConfig::with_timeout(interval *
max_missed_pongs)?`.

### Config Constants

All configuration constants are exported from `lib.rs` for use in binding layers:

```rust,ignore
// Reconnection defaults
pub const DEFAULT_MAX_ATTEMPTS: u32 = 5;
pub const DEFAULT_INITIAL_DELAY_MS: u64 = 1000;
pub const DEFAULT_MAX_DELAY_MS: u64 = 60000;
pub const MIN_INITIAL_DELAY_MS: u64 = 100;

// Health check defaults (3.0)
pub const DEFAULT_HEALTH_CHECK_ENABLED: bool = true;
pub const DEFAULT_HEARTBEAT_TIMEOUT_MS: u64 = 35000;
pub const MIN_HEARTBEAT_TIMEOUT_MS: u64 = 5000;
```

## Which constructor should I use?

The SDK exposes four distinct construction paths. They look similar; this section pins which to reach for in which situation.

**1. `bon`-derived builders** — default-filled construction, `maybe_*` setters for `Option<T>` fields, no validation. Use for:

- `SubscribeRequest::builder()`
- `RetryPolicy::builder()`
- `ReconnectionConfig::builder()`

```rust,ignore
use marketdata_core::{ReconnectionConfig, RetryPolicy};
use std::time::Duration;

let reconnect = ReconnectionConfig::builder()
    .max_attempts(10)
    .initial_delay(Duration::from_secs(2))
    .build();

let retry = RetryPolicy::builder()
    .max_attempts(5)
    .initial_backoff(Duration::from_millis(250))
    .max_backoff(Duration::from_secs(10))
    .build();
```

**2. Validating positional constructors** — return `Result<Self, MarketDataError>` and reject bad inputs at construction. Use when validation matters:

- `ReconnectionConfig::new(max_attempts, initial_delay, max_delay)?`

```rust,ignore
use marketdata_core::ReconnectionConfig;
use std::time::Duration;

let reconnect = ReconnectionConfig::new(
    5,
    Duration::from_secs(1),
    Duration::from_secs(60),
)?; // returns ConfigError on invalid inputs
```

**3. Typestate factory** — derives stock + futopt endpoint configurations from one auth credential. Use for full-application setup:

```rust,ignore
use marketdata_core::websocket::WebSocketFactory;
use marketdata_core::AuthRequest;

let cfg = WebSocketFactory::new()
    .auth(AuthRequest::with_api_key("k"))
    .stock()
    .build();
```

**4. Convenience constructors** — one-liner for common cases. Use in examples, scripts, and one-shot integration tests:

- `ConnectionConfig::fugle_stock(auth)`
- `ConnectionConfig::fugle_futopt(auth)`
- `RetryPolicy::conservative()` / `RetryPolicy::aggressive()`
- `ReconnectionConfig::default()` / `ReconnectionConfig::disabled()`

```rust,ignore
use marketdata_core::websocket::ConnectionConfig;
use marketdata_core::AuthRequest;

let cfg = ConnectionConfig::fugle_stock(AuthRequest::with_api_key("k"));
```

**Rule of thumb:** Reach for the `bon` builder by default. If your inputs come from external configuration (env vars, JSON, a CLI flag) and you want bounds-checking at the boundary, use the positional `new(...)` constructor. Use the typestate factory when one auth credential drives multiple endpoint configurations.

## Error Handling

All operations return `Result<T, MarketDataError>`:

```rust,ignore
use marketdata_core::MarketDataError;

match client.stock().intraday().quote().symbol("2330").send() {
    Ok(quote) => println!("Price: {:?}", quote.close_price),
    Err(MarketDataError::AuthError { msg }) => {
        eprintln!("Authentication failed: {}", msg);
    }
    Err(MarketDataError::ApiError { status, message }) => {
        eprintln!("API error {}: {}", status, message);
    }
    Err(MarketDataError::TimeoutError { operation }) => {
        eprintln!("Timeout during: {}", operation);
    }
    Err(e) => eprintln!("Error: {}", e),
}
```

Error codes for FFI consumers:

| Code | Error Type |
|------|------------|
| 1001 | InvalidSymbol |
| 1002 | DeserializationError |
| 1003 | RuntimeError |
| 1004 | ConfigError |
| 2001 | ConnectionError |
| 2002 | AuthError |
| 2003 | ApiError |
| 2010 | ClientClosed |
| 3001 | TimeoutError |
| 3002 | WebSocketError |
| 9999 | Other |

## API Reference

See the [API documentation](https://docs.rs/marketdata-core) for complete details.

### REST Endpoints

**Stock Intraday:**
- `client.stock().intraday().quote()` - Real-time quote
- `client.stock().intraday().ticker()` - Symbol information
- `client.stock().intraday().candles()` - OHLCV candles
- `client.stock().intraday().trades()` - Trade history
- `client.stock().intraday().volumes()` - Volume by price

**FutOpt Intraday:**
- `client.futopt().intraday().quote()` - Real-time quote
- `client.futopt().intraday().ticker()` - Contract information
- `client.futopt().intraday().tickers()` - Multiple contracts
- `client.futopt().intraday().candles()` - OHLCV candles
- `client.futopt().intraday().trades()` - Trade history
- `client.futopt().intraday().volumes()` - Volume by price
- `client.futopt().intraday().products()` - Product listing

### WebSocket Channels

| Channel | Description |
|---------|-------------|
| Trades | Real-time trade executions |
| Candles | Real-time candlestick updates |
| Books | Order book (5 levels bid/ask) |
| Aggregates | Aggregated market data |
| Indices | Index values (stock only) |

## Examples

See the [examples/](examples/) directory:

- `rest_basic.rs` - REST API usage
- `websocket_basic.rs` - WebSocket streaming

Run examples:

```bash
export FUGLE_API_KEY="your-api-key"
cargo run --example rest_basic
cargo run --example websocket_basic
```

## License

MIT
