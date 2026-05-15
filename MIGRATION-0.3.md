# Migration Guide: Rust 0.2 → 0.3

The Rust `fugle-marketdata` crate at 0.3.0 makes the `WebSocketClient` sync
by default and moves the tokio-backed async client behind a `tokio-comp`
feature flag. This mirrors the [redis-rs] `tokio-comp` pattern.

[redis-rs]: https://github.com/redis-rs/redis-rs

## TL;DR

- **Default**: pull `fugle-marketdata = "0.3"`. `WebSocketClient` is sync.
  No tokio in your dependency tree.
- **Async**: pull `fugle-marketdata = { version = "0.3", features = ["tokio-comp"] }`.
  Use `fugle_marketdata::aio::WebSocketClient` exactly like the old client.

## Side-by-side

```rust
// 0.2 (async, tokio always pulled)
use fugle_marketdata::{WebSocketClient, ConnectionConfig, AuthRequest, Channel};
use fugle_marketdata::websocket::StockSubscription;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = ConnectionConfig::fugle_stock(AuthRequest::with_api_key("..."));
    let client = WebSocketClient::new(cfg);
    client.connect().await?;
    client.subscribe(StockSubscription::new(Channel::Trades, "2330")).await?;
    let rx = client.messages();
    // recv loop ...
    Ok(())
}
```

```rust
// 0.3 sync (default, no tokio)
use fugle_marketdata::{WebSocketClient, ConnectionConfig, AuthRequest, Channel};
use fugle_marketdata::websocket::StockSubscription;

fn main() -> anyhow::Result<()> {
    let cfg = ConnectionConfig::fugle_stock(AuthRequest::with_api_key("..."));
    let client = WebSocketClient::new(cfg);
    client.connect()?;
    client.subscribe(StockSubscription::new(Channel::Trades, "2330"))?;
    let rx = client.messages();
    while let Ok(msg) = rx.receive() { /* ... */ }
    Ok(())
}
```

```rust
// 0.3 async (features = ["tokio-comp"])
use fugle_marketdata::aio::WebSocketClient;
use fugle_marketdata::{ConnectionConfig, AuthRequest, Channel};
use fugle_marketdata::websocket::StockSubscription;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = ConnectionConfig::fugle_stock(AuthRequest::with_api_key("..."));
    let client = WebSocketClient::new(cfg);
    client.connect().await?;
    client.subscribe(StockSubscription::new(Channel::Trades, "2330")).await?;
    let mut stream = client.message_stream();
    while let Some(msg) = stream.recv().await { /* ... */ }
    Ok(())
}
```

The async API in 0.3 is **identical** to 0.2 except for the import path
(`fugle_marketdata::aio::WebSocketClient`) and enabling the feature flag.

## API change reference

| 0.2 | 0.3 sync (default) | 0.3 async (`aio::`) | Notes |
|---|---|---|---|
| `WebSocketClient::new` | unchanged | unchanged | |
| `with_reconnection_config` | unchanged | unchanged | |
| `with_health_check_config` | unchanged | unchanged | |
| `with_full_config` | unchanged | unchanged | |
| `state()` | unchanged | unchanged | |
| `async fn state_async()` | **removed** | **removed** | Use `state()` |
| `async fn is_closed()` | renamed to `is_closed()` (sync) | renamed to `is_closed()` (sync) | The old `is_closed_sync()` is also gone — only `is_closed()` remains |
| `is_closed_sync()` | renamed `is_closed()` | renamed `is_closed()` | |
| `events()` / `state_events()` | unchanged | unchanged | Still returns std mpsc receiver |
| `messages()` | unchanged | unchanged | Returns `Arc<MessageReceiver>` |
| `message_stream()` | **not available** | unchanged | Returns `tokio::sync::mpsc::Receiver` — async-only |
| `async fn connect()` | `fn connect()` | unchanged | |
| `async fn disconnect()` | `fn disconnect()` | unchanged | |
| `async fn force_close()` | `fn force_close()` | unchanged | |
| `async fn is_connected()` | `fn is_connected()` | unchanged | |
| `async fn subscribe(...)` | `fn subscribe(...)` | unchanged | Same `StockSubscription` |
| `async fn subscribe_futopt(...)` | `fn subscribe_futopt(...)` | unchanged | Same `FutOptSubscription` |
| `async fn unsubscribe(ids)` | `fn unsubscribe(ids)` | unchanged | Same `IntoIterator<Item = impl Into<String>>` |
| `subscriptions()` | unchanged | unchanged | |
| `async fn reconnect()` | `fn reconnect()` | unchanged | |
| `async fn send(...)` | `fn send(...)` | unchanged | |
| `subscription_keys()` | unchanged | unchanged | |

## Common migration recipes

### "I want the smallest dependency footprint"

Use the default sync client. You drop ~80 transitive crates (tokio, mio,
futures-*, tokio-tungstenite, tokio-rustls) and ~600-900KB of binary.

```toml
fugle-marketdata = "0.3"
```

### "I'm in a tokio app — keep things as they were"

Enable `tokio-comp` and import the async client by its new path:

```toml
fugle-marketdata = { version = "0.3", features = ["tokio-comp"] }
```

```rust
use fugle_marketdata::aio::WebSocketClient;
```

The rest of your code stays identical — same constructors, same methods,
same return types, same `.await` ergonomics.

### "I use `event_handle.blocking_lock()` from a non-tokio thread"

`events()` / `state_events()` still return `Arc<std::sync::Mutex<Receiver<ConnectionEvent>>>`.
Both 0.2 and 0.3 use `std::sync::Mutex`, so any binding code that called
`tokio::sync::Mutex::blocking_lock()` was actually wrong against the
real type — switch to `std::sync::Mutex::lock()`. (This was a latent bug
made visible by the rename in 0.3.)

```rust
// works in both 0.2 and 0.3
let rx = events.lock().expect("event lock poisoned");
let event = rx.recv_timeout(Duration::from_millis(200))?;
```

### "I subscribed off the event loop and didn't await"

In 0.3 sync, `subscribe()` returns `Result<(), MarketDataError>`
directly — no `.await`. Replace:

```rust
client.subscribe(sub).await?;
// becomes
client.subscribe(sub)?;
```

### "I called `state_async()` because the sync version didn't work mid-await"

The 0.2 sync `state()` blocked the runtime via `Handle::try_current()
.block_on(...)`. In 0.3 it just reads a `std::sync::RwLock` — no runtime
involvement. Delete the `state_async()` call and use `state()`.

### "I held the receiver from `message_stream()`"

Only available on `aio::WebSocketClient` (still returns
`tokio::sync::mpsc::Receiver`). If you're on the sync default, switch to
`messages()` which returns a blocking `MessageReceiver`:

```rust
let rx = client.messages();
while let Ok(msg) = rx.receive() {
    // ...
}
```

If you need a non-blocking pull, `rx.try_receive()` or
`rx.receive_timeout(Duration::from_millis(50))` are available.

## Internal architecture (for contributors)

```
core/src/websocket/
├── channels/             unchanged: pure parsing
├── config.rs             unchanged: runtime-agnostic
├── connection_event.rs   NEW: ConnectionState, ConnectionEvent, emit_event
├── health_check.rs       unchanged: config only
├── message.rs            MessageReceiver only (tokio-free)
├── protocol.rs           NEW: framing/parsing wrappers (pure functions)
├── reconnection.rs       unchanged: pure policy
├── subscription.rs       unchanged: std::sync::RwLock
├── sync/                 NEW: default sync client (tungstenite + std::thread)
│   ├── client.rs
│   └── owner_thread.rs   single-thread owner+supervisor
└── aio/                  #[cfg(feature = "tokio-comp")]
    ├── client.rs         tokio client (moved from connection.rs)
    ├── dispatch.rs       moved from message.rs
    ├── reconnect.rs      moved from connection.rs
    ├── runtime.rs        moved from runtime.rs
    └── writer.rs         moved from connection.rs
```

Shared by both clients: `connection_event`, `protocol`, `config`,
`subscription`, `reconnection`, `health_check`, `channels`, `message`.

The sync client uses a single OS thread per connection with a bounded
outbound queue (`std::sync::mpsc::sync_channel(64)`) and a 200ms read
poll interval, falling back to `Instant`-based heartbeat liveness
matching the async path's `tokio::time::timeout` semantics. Reconnect
is folded into the same thread (no separate supervisor) and replays
subscriptions identically to the async `try_reconnect`.

TLS is unified: both clients build an `Arc<rustls::ClientConfig>` via
`crate::tls::build_rustls_config` (existing helper) and pass it to either
`tungstenite::Connector::Rustls` (sync) or
`tokio_tungstenite::Connector::Rustls` (async). The same
`rustls 0.23` + `rustls-native-certs 0.8` versions are pinned to avoid
duplicate crates.

## Verifying the upgrade

```bash
# Compile-time check: sync-only builds pull zero tokio
cargo tree -p fugle-marketdata --no-default-features --edges=no-dev \
  | grep -E 'tokio|mio|tungstenite'
# Expected output: only `tungstenite 0.29.0`.

# Both feature combinations compile + test
cargo build --no-default-features
cargo build --features tokio-comp
cargo test  --no-default-features
cargo test  --features tokio-comp
```
