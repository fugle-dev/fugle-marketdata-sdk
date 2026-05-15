# Migration Guide: Rust 0.6.x → 0.7

**Zero breaking changes.** Every addition is opt-in. Existing 0.6.0 code
compiles unchanged against 0.7.0; this guide covers the new opt-in
patterns and the documentation improvements.

## What is NOT changing

- No `WebSocketErrorKind` variants added or removed.
- `MarketDataError::source_kind()` and `is_retryable()` semantics unchanged.
- No `base_url(...)` semantic shift (last shifted in 0.6.0; see
  `MIGRATION-0.6.md` if upgrading from 0.5.x).
- No MSRV bump — still Rust 1.82.
- No new required dependencies; `metrics` is optional.

## 1. Opt-in `metrics` feature

The SDK now registers two `metrics` crate counters per `WebSocketClient`
when the feature is enabled:

| Counter                                       | Increments on                                            |
|-----------------------------------------------|----------------------------------------------------------|
| `fugle_marketdata_ws_messages_dropped_total`  | inbound `messages()` channel back-pressure (drop-newest) |
| `fugle_marketdata_ws_events_dropped_total`    | `connection_events()` broadcast back-pressure            |

Both carry labels `endpoint` (URL host) and `client_id`
(low-cardinality identifier from `ConnectionConfig::client_id(...)`,
empty when unset).

The polling getters `messages_dropped_total()` / `events_dropped_total()`
remain authoritative for in-process callers; the `metrics` integration
is mirroring, not replacing.

### Enabling

`Cargo.toml`:

```toml
[dependencies]
fugle-marketdata-core = { version = "0.7", features = ["tokio-comp", "metrics"] }
metrics-exporter-prometheus = "0.16"
```

### Wiring a Prometheus exporter

```rust,ignore
use marketdata_core::aio::WebSocketClient;
use marketdata_core::websocket::ConnectionConfig;
use marketdata_core::AuthRequest;
use metrics_exporter_prometheus::PrometheusBuilder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Install the recorder ONCE per process.
    PrometheusBuilder::new()
        .with_http_listener(([0, 0, 0, 0], 9090))
        .install()?;

    let auth = AuthRequest::with_api_key(std::env::var("FUGLE_API_KEY")?);
    let config = ConnectionConfig::builder(
        "wss://api.fugle.tw/marketdata/v1.0/stock/streaming",
        auth,
    )
    .client_id("monitor-stock-probe")  // low-cardinality label
    .build();

    let client = WebSocketClient::new(config);
    client.connect().await?;
    // ...
    Ok(())
}
```

The `client_id` setter accepts up to 64 bytes; longer values are
truncated at a UTF-8 char boundary and logged via `tracing::warn!`
(when the `tracing` feature is enabled). **Use deployment / instance /
service identifiers — never per-request UUIDs**, which explode
Prometheus storage.

## 2. Multi-client `MockWsServer`

`core::testing::MockWsServer` now supports N clients per instance.
Build a multi-client mock with `start_with_capacity(n)` or use the
convenience pair `aio_pair_n(n)`:

```rust,ignore
use marketdata_core::testing::aio_pair_n;
use marketdata_core::models::streaming::StreamMessage;

#[tokio::test]
async fn dual_probe_topology() {
    let (server, clients) = aio_pair_n(2).await;
    for c in &clients {
        c.connect().await.unwrap();
    }

    // Targeted injection per client.
    server.inject_frame_for(0, StreamMessage::Pong { state: Some("a".into()) }).await;
    server.inject_frame_for(1, StreamMessage::Pong { state: Some("b".into()) }).await;
}
```

The single-client `start()` / `aio_pair()` constructors are unchanged.
Calling the bare `inject_frame(...)` / `next_subscribe_id(...)` /
`close(...)` on a multi-client mock panics with an explicit message
naming the `_for(idx, ...)` alternative.

## 3. Transport-drop intent injection

`MockWsServer::drop_transport()` (and `drop_transport_for(client_idx)`)
closes the underlying TCP socket without sending a WebSocket Close
frame, forcing the client into `DisconnectIntent::Network`:

```rust,ignore
use marketdata_core::testing::aio_pair;
use marketdata_core::websocket::{ConnectionEvent, DisconnectIntent};

#[tokio::test]
async fn network_failure_classified_correctly() {
    let (server, client) = aio_pair().await;
    client.connect().await.unwrap();

    server.drop_transport().await;
    // …assert the next ConnectionEvent::Disconnected carries
    // intent: DisconnectIntent::Network.
}
```

Combined with the existing `close(code, reason)` (which produces
`DisconnectIntent::Server`) and the SDK's own `client.disconnect()`
(which produces `DisconnectIntent::Client`), the mock now covers every
disconnect intent the SDK can emit.

## 4. REST and WebSocket can target different hosts

`RestClient::base_url(...)` and `WebSocketFactory::base_url(...)` are
fully independent. You can keep REST on `api.fugle.tw` while routing
WebSocket through a separate host — useful for staging, edge proxies,
or private deployments. Both setters expect the full URL including the
`/v1.0` segment per the 0.6.0 convention; the WebSocket factory
appends only `/{stock|futopt}/streaming`, the REST client appends
nothing.

```rust,ignore
use marketdata_core::{Auth, AuthRequest, RestClient};
use marketdata_core::websocket::WebSocketFactory;

let auth = AuthRequest::with_api_key("YOUR_KEY");

// REST: production API
let rest = RestClient::new(Auth::SdkToken("YOUR_KEY".into()))
    .base_url("https://api.fugle.tw/marketdata/v1.0");

// WebSocket: separate host (substitute your own URL — never reference
// internal-only hosts in published code or examples).
let stock_ws_config = WebSocketFactory::new()
    .auth(auth.clone())
    .base_url("wss://your-ws-host.example.com/marketdata/v1.0")
    .stock()
    .build();
```

This pattern was already supported in 0.6.0; 0.7.0 makes it
discoverable in the migration guide and in `core/README.md`'s
"Which constructor should I use?" section. No code change required.

## 5. Documentation hygiene improvements

- `WebSocketErrorKind::Http(u16)`'s rustdoc now contains the full
  status-code → `ErrorKind` / `is_retryable()` mapping table. A
  `#[cfg(test)]` consistency assertion in `core/src/errors.rs` pins the
  doc-vs-impl contract — drift fails CI.
- `core/src/tracing_compat.rs` has a module-level note explaining the
  internal `__tracing_noop` macro and reaffirming it is not part of
  the public API.
- A `cargo public-api` snapshot at `core/PUBLIC-API.txt` plus a CI job
  in `.github/workflows/public-api.yml` track the public surface for
  drift on PRs that touch `core/src/lib.rs`, `core/src/tracing_compat.rs`,
  or `core/Cargo.toml`. Acknowledge intentional changes in
  `core/PUBLIC-API.md`.

## Rollback

All four opt-ins are reversible without data migrations:

- `metrics`: drop the `metrics` feature flag from your `Cargo.toml`;
  the integration disappears at compile time.
- Multi-client mock: replace `start_with_capacity(N)` /
  `inject_frame_for(idx, …)` calls with `start()` / `inject_frame(…)`.
- Transport drop: replace `drop_transport()` calls with `close(…)` if
  the test only needs `DisconnectIntent::Server`.
- Dual-host: revert to a single shared base URL.

No persistent state changes, no schema migrations, no on-the-wire
protocol changes.
