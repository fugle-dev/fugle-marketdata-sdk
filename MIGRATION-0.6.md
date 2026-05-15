# Migration Guide: Rust 0.5.x → 0.6

Three changes in 0.6.0. **One is a silent breaking semantic shift** —
read §1 carefully before upgrading production code.

## 1. SILENT BREAKING: `WebSocketFactory::base_url(...)` semantic

The factory now aligns with the OpenAI / Stripe / AWS / Anthropic SDK
convention. **`base_url` MUST include the API version segment.**

| Call | 0.5.x result | 0.6.0 result |
|---|---|---|
| `.base_url("wss://api.fugle.tw/marketdata")` then `.stock().build()` | `…/marketdata/v1.0/stock/streaming` ✓ | `…/marketdata/stock/streaming` ❌ (404 from gateway) |
| `.base_url("wss://api.fugle.tw/marketdata/v1.0")` then `.stock().build()` | `…/marketdata/v1.0/v1.0/stock/streaming` ❌ | `…/marketdata/v1.0/stock/streaming` ✓ |
| `WebSocketFactory::new().auth(a).stock().build()` (no override) | canonical | canonical (unchanged) |

The shift is **silent**: code compiles in both versions but produces
different URLs at runtime. The 0.5.x → 0.6.0 transition needs a code
update everywhere `.base_url(...)` is called with a custom string.

### Migration recipe

```bash
# Mechanical: append /v1.0 to factory base_url strings.
# Audit your codebase for `.base_url(` then add /v1.0 to each string.
grep -rn '\.base_url("wss' src/
```

Or pin to `crate::urls::API_VERSION`:

```rust
use marketdata_core::urls::{WS_BASE_ROOT, API_VERSION};
use marketdata_core::websocket::WebSocketFactory;

let factory = WebSocketFactory::new()
    .base_url(format!("{}/{}", WS_BASE_ROOT, API_VERSION))
    .auth(auth);
```

`RestClient::base_url(&str)` is unchanged — it already followed the
OpenAI convention.

## 2. BREAKING: `MarketDataError::WebSocketError` gains structured `kind`

`WebSocketError { msg: String }` → `WebSocketError { kind: WebSocketErrorKind, msg: String }`.

Pattern matches need `kind` (or `..`):

```rust
// Before 0.5.x
match err {
    MarketDataError::WebSocketError { msg } => eprintln!("ws: {msg}"),
    _ => {}
}

// After 0.6.0
match err {
    MarketDataError::WebSocketError { kind, msg } => eprintln!("ws ({kind:?}): {msg}"),
    _ => {}
}
```

`WebSocketErrorKind` is `#[non_exhaustive]` with variants `Protocol`,
`Capacity`, `Utf8`, `Tls`, `Io`, `Http(u16)`, `Other`.

### Retry verdict diff

`is_retryable()` is refined to honour the kind. **Protocol violations
are no longer retryable** (they were in 0.5.x):

| Kind | 0.5.x retryable | 0.6.0 retryable |
|---|---|---|
| `Protocol`, `Capacity`, `Utf8` | yes | **no** |
| `Tls` | (was AuthError; not retryable) | no |
| `Io` | (was ConnectionError; yes) | yes |
| `Http(401\|403)` | (was AuthError; no) | no |
| `Http(429)`, `Http(5xx)` | (was ConnectionError; yes) | yes |
| `Http(other)` | (was ConnectionError; yes) | **no** |
| `Other` | yes | yes |

Programmatic classification: `err.source_kind() -> ErrorKind` now
distinguishes Protocol from Network correctly without 0.5.1's coarse
fallback. `WebSocketErrorKind::Tls` → `ErrorKind::Auth`; `Io` →
`Network`; `Http(429)` → `RateLimit`; etc.

### Tungstenite mapping shift

The `From<tungstenite::Error>` impl no longer collapses variants into
`ConnectionError` / `AuthError`. Every `tungstenite::Error` now produces
a `MarketDataError::WebSocketError` with the appropriate kind. Code that
pattern-matched on `MarketDataError::ConnectionError` arising from
WebSocket transport must move to matching `WebSocketError { kind: Io, .. }`.

## 3. ADDITIVE: `core::testing::MockWsServer` for downstream tests

Behind `features = ["test-utils"]`. Off by default. Use as a
dev-dependency:

```toml
[dev-dependencies]
fugle-marketdata-core = { version = "0.6", features = ["test-utils"] }
```

```rust
use marketdata_core::testing::{MockWsServer, aio_pair};
use marketdata_core::models::streaming::StreamMessage;

#[tokio::test]
async fn my_test() {
    let (server, client) = aio_pair().await;
    client.connect().await.unwrap();
    server.inject_frame(StreamMessage::Authenticated).await;
    // ...assert on client state...
    server.close(1001, "going away").await;
}
```

Replaces the hand-rolled echo servers in monitor / py / js / uniffi
test suites. The mock implements the same subscribe-ACK protocol as
production `protocol.rs`; drift is caught by
`core/tests/mock_server_smoke.rs`.

## Verification checklist

```bash
cargo build  -p fugle-marketdata-core
cargo test   -p fugle-marketdata-core --all-features
cargo build  -p fugle-marketdata-core --features test-utils  # mock available
cargo clippy -p fugle-marketdata-core --all-features -- -D warnings
```

If you maintain bindings:

```bash
cargo build -p marketdata-py -p marketdata-js -p marketdata-uniffi
```

The `WebSocketError` reshape is the only cross-FFI change worth
verifying — `uniffi/src/errors.rs` already stringifies `kind` into `msg`
at the boundary so binding consumers see no shape change. Direct Rust
consumers are the population that needs the §2 migration.
