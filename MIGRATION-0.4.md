# Migration Guide: Rust 0.3 → 0.4

The Rust `fugle-marketdata` crate at 0.4.0 ships production-readiness fixes
that the downstream `monitor` integration drove: opt-in `tracing`,
secret-redacting `Debug`, sane reconnect default, REST retry policy,
multi-connection event labels, graceful shutdown drain, configurable
backpressure buffers, a JS-style `WebSocketFactory`, and the removal of a
small set of legacy constructors.

## TL;DR

- **Most callers**: bump `fugle-marketdata = "0.4"`. Recompile. If you
  pattern-match on `ConnectionEvent` variants, adjust two patterns
  (instructions below). Everything else is additive.
- **Bindings (Python / Node / UniFFI / Go / Java / C++ / C#)**: end users
  observe no behavior change, *provided* the binding wrapper explicitly
  calls `ReconnectionConfig::disabled()` at the FFI boundary (the
  workspace test in `core/tests/reconnect_default.rs` is the CI gate).
- **Production observability**: enable the new `tracing` feature
  (`features = ["tracing"]`) and install a `tracing_subscriber`. No-op
  when feature off; zero binary cost.

## Breaking changes

There are five public-API breaks. All emit a clear compile-time error
after the upgrade — runtime behavior is preserved (or improved) for code
that compiles.

### 1. `ConnectionEvent::Disconnected` gains `intent`

`ConnectionEvent::Disconnected` now carries
`intent: DisconnectIntent { Client, Server, Network }`.
`ConnectionState::Closed` mirrors the same `intent` field. All other
`ConnectionEvent` variants are unchanged.

```rust
// 0.3
match event {
    ConnectionEvent::Disconnected { code, reason } => { /* ... */ }
    ConnectionEvent::Connected => { /* ... */ }
    _ => {}
}

// 0.4 — destructure the new field
match event {
    ConnectionEvent::Disconnected { code, reason, intent } => {
        match intent {
            DisconnectIntent::Client  => log::info!("local-initiated"),
            DisconnectIntent::Server  => log::warn!("server kicked us"),
            DisconnectIntent::Network => log::error!("transport failed"),
        }
    }
    ConnectionEvent::Connected => { /* ... */ }
    _ => {}
}

// 0.4 — or `..` to ignore intent if you don't need it
match event {
    ConnectionEvent::Disconnected { code, reason, .. } => { /* ... */ }
    _ => {}
}
```

For multi-connection attribution (e.g. one client per channel), wrap the
per-client `events()` Receiver with a label adapter or use
`tokio::select!` arms — the SDK does not stuff a `connection_id` onto
every event itself.

### 2. `ReconnectionConfig::default().enabled` flipped `false` → `true`

Rust callers on the `WebSocketClient::new(config)` happy path now get
auto-reconnect by default, aligning with `reqwest` / `redis-rs` /
`tokio-tungstenite` ergonomics.

```rust
// 0.4 — default is auto-reconnect on
let client = WebSocketClient::new(config);
// equivalent to:
let client = WebSocketClient::with_reconnection_config(
    config,
    ReconnectionConfig::default(),  // enabled = true
);

// 0.4 — opt out (matches old fugle-marketdata-{python,node} semantics)
let client = WebSocketClient::with_reconnection_config(
    config,
    ReconnectionConfig::disabled(),
);
```

**Bindings** (`fugle-marketdata-python`, `fugle-marketdata-node`,
`uniffi/`, `bindings/{go,java,cpp,csharp}/`) MUST call
`ReconnectionConfig::disabled()` at their FFI boundary so end users see no
behavior change — `core/tests/reconnect_default.rs` enforces this at CI
time.

### 3. `Auth` and `ConnectionConfig` `Debug` output redacted

The `Debug` impls now print `Auth::ApiKey(***)` instead of the raw token,
and `ConnectionConfig::url`'s query parameters whose names match
`token` / `key` / `apikey` / `api_key` / `secret` / `password`
(case-insensitive) are masked as `***`.

```rust
// 0.3
println!("{:?}", Auth::ApiKey("super-secret".into()));
// → Auth::ApiKey("super-secret")     ← LEAK

// 0.4
println!("{:?}", Auth::ApiKey("super-secret".into()));
// → Auth::ApiKey(***)
```

Anyone parsing the formatted Debug output (e.g. `format!("{:?}", config)`
into a regex) will need to update. Logs, panic messages, and `tracing`
output all benefit silently.

### 4. `SubscribeRequest::{trades, candles, books, aggregates}` removed

These per-channel constructors duplicated `SubscribeRequest::new(channel,
symbol)` and were removed:

```rust
// 0.3
let req = SubscribeRequest::trades("2330");

// 0.4
let req = SubscribeRequest::new(Channel::Trades, "2330");
```

A grep across this workspace found zero non-test callers; the migration
should be mechanical for any external code that did use them.

### 5. `disconnect()` is now a graceful drain, not fire-and-forget

`WebSocketClient::disconnect()` (both sync and async) now defaults to a
**5 second drain timeout**: it signals the dispatch loop to stop, drops
the writer-side sender so any in-flight queued frames flush, sends a
WebSocket Close frame, awaits the peer's Close acknowledgement, and only
then forcibly aborts the background tasks. The emitted
`Disconnected { intent: DisconnectIntent::Client }` arrives once the
sequence completes (or the timeout elapses).

```rust
// 0.3 — returned almost immediately, often dropped close frame
client.disconnect().await?;

// 0.4 — same call, now blocks up to 5 s for graceful shutdown
client.disconnect().await?;

// 0.4 — caller-supplied drain timeout
client.shutdown_with_timeout(Duration::from_millis(100)).await?;

// 0.4 — fire-and-forget (rare; same as old behavior)
client.shutdown_with_timeout(Duration::ZERO).await?;
```

Callers that relied on `disconnect()` returning instantly (k8s SIGTERM
handlers, test teardown) should pass an explicit short timeout via
`shutdown_with_timeout`.

## Additive features

### Opt-in `tracing` integration

```toml
[dependencies]
fugle-marketdata = { version = "0.4", features = ["tracing"] }
```

```rust
fn main() {
    tracing_subscriber::fmt::init();
    // ... existing client code unchanged
}
```

Emits `info!` on connect/auth, `warn!` on reconnect-attempt /
heartbeat-timeout / channel-saturation, `error!` on close-frame send
failure, `debug!` on every received frame (`bytes`, `kind`), and
`#[tracing::instrument]` spans named `ws.connect` / `ws.subscribe` /
`ws.unsubscribe` / `ws.disconnect`. Off by default — zero binary cost.

### `RestClient::with_retry(RetryPolicy)`

```rust
use fugle_marketdata::{RestClient, RetryPolicy, Auth};

let client = RestClient::new(Auth::SdkToken("t".into()))
    .with_retry(RetryPolicy::conservative());  // 3 attempts, 100 ms initial, 2 s ceiling
```

Exponential backoff plus uniform jitter. Retries only the errors
classified by `MarketDataError::is_retryable()` (HTTP 429, HTTP 5xx,
transport timeouts, connection errors). Default is no retry — observability
use cases need real failures visible.

### `Auth::from_env()`

```rust
let auth = Auth::from_env()?;
// Probes FUGLE_API_KEY → FUGLE_BEARER_TOKEN → FUGLE_SDK_TOKEN.
// Empty string treated as unset.
```

### `WebSocketFactory` (mirrors JS / Python SDK shape)

```rust
use fugle_marketdata::{WebSocketFactory, WebSocketClient, AuthRequest};

// Production default endpoints:
let factory = WebSocketFactory::new(AuthRequest::with_api_key("k"));
let stock = WebSocketClient::new(factory.stock().build());
let futopt = WebSocketClient::new(factory.futopt().build());

// Custom base (staging / proxy / mock server) — both endpoints share it:
let factory = WebSocketFactory::new(AuthRequest::with_api_key("k"))
    .base_url("wss://staging.fugle.tw/marketdata");
let stock = WebSocketClient::new(factory.stock().build());
```

### Configurable channel buffers

```rust
let cfg = ConnectionConfig::builder(urls::STOCK_WS, auth)
    .message_buffer(8192)  // default 4096 in 0.4 (was 1024 in 0.3)
    .event_buffer(1024)    // default 1024 (unchanged)
    .build();
```

Default `message_buffer` bumped 1024 → 4096 to give multi-symbol
consumers ~2 s of headroom at TWSE 9:00 open burst (~2000 msg/s) before
drop-newest backpressure begins. Bounded mpsc channels are lazily
allocated, so a higher cap costs nothing at idle.

### `messages_dropped_total` counter

```rust
let dropped: u64 = client.messages_dropped_total();
```

Monotonic atomic counter on every client. Increments whenever the inbound
message channel saturates and a frame is dropped (drop-newest). Pair with
`tracing` to see per-saturation `warn!` events with the running drop
count.

### Subscription introspection

```rust
let n: usize = client.subscription_count();
let on: bool = client.is_subscribed(&Channel::Trades, "2330");
```

### `urls` module — endpoint constants

```rust
use fugle_marketdata::urls;

// Full canonical endpoints:
urls::STOCK_WS;        // wss://api.fugle.tw/marketdata/v1.0/stock/streaming
urls::FUTOPT_WS;       // wss://api.fugle.tw/marketdata/v1.0/futopt/streaming
urls::REST_BASE;       // https://api.fugle.tw/marketdata/v1.0

// Roots + version (for composing custom URLs):
urls::WS_BASE_ROOT;    // wss://api.fugle.tw/marketdata
urls::REST_BASE_ROOT;  // https://api.fugle.tw/marketdata
urls::API_VERSION;     // v1.0
```

## Rollback

`0.4.0` is a minor bump; downstream pins to `fugle-marketdata = "0.3"`
continue to work. There is no upgrade pressure beyond the consumer
deciding they want the new ergonomics.

## Acknowledgements

This release was driven by review feedback during the `monitor`
integration design. See
`openspec/changes/sdk-04-improvements/{proposal,design}.md` for the full
decision log.
