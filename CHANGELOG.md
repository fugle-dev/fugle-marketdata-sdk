# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [Rust 0.5.0] - 2026-05-15

Minor-version pass adopting three patterns observed in databento-rs:
`Symbols` rename + dedup contract, typestate `WebSocketFactory`, and
`bon::Builder` derives on `RetryPolicy`, `ReconnectionConfig`, and
`SubscribeRequest`. See `MIGRATION-0.5.md` for the full migration
guide.

### Breaking

- **`SymbolSpec` renamed to `Symbols`** and moved out of
  `models/subscription.rs` into a dedicated `models/symbols.rs`. All
  nine existing `From` impls retarget the renamed type. Mechanical
  migration: `sed -i '' 's/SymbolSpec/Symbols/g'` over downstream
  sources.
- **Subscription dispatch deduplicates symbols.**
  `SubscribeRequest::with_symbols`, `StockSubscription::new`, and
  `FutOptSubscription::new` now run their input through
  `Symbols::normalized()` (trim whitespace, drop empty, dedup
  preserving insertion order, collapse `Many` of length 1 to `Single`)
  before producing the request. Duplicate symbols that previously
  produced two server ACKs now collapse to one subscription.
- **`WebSocketFactory` is typestate-enforced.**
  `WebSocketFactory::new(auth)` becomes
  `WebSocketFactory::new().auth(auth)`. Calling `.stock()` / `.futopt()`
  before `.auth(...)` is now a compile-time error
  (`compile_fail` doctests guard the contract).
- **`ReconnectionConfig::with_max_attempts` / `with_initial_delay` /
  `with_max_delay` removed.** These fallible chainable validators are
  superseded by the unvalidated `ReconnectionConfig::builder()` (bon)
  and the existing validating `ReconnectionConfig::new(...)`
  positional constructor.

### Added

- **`Symbols::normalized()`, `len()`, `is_empty()`, `iter()`,
  `chunked(n)`** helpers on the renamed enum.
- **`SUBSCRIPTION_BATCH_LIMIT: Option<usize>`** const (currently `None`)
  in `models::symbols`, reserved for a future server-documented
  per-frame limit. Downstream code can branch on the constant without
  another version bump.
- **`bon::Builder` derives** on `RetryPolicy`, `ReconnectionConfig`,
  and `SubscribeRequest`. `bon` adds `maybe_*` setters for `Option<T>`
  fields. Existing constructors (`new`, `with_symbols`, presets) are
  preserved.

### Internal

- Adopted `bon = "3"` as a runtime dependency for builder generation.
- `ConnectionConfig` intentionally retains its hand-rolled builder so
  the `assert!`-based zero-capacity-buffer validation contract from
  the `websocket-config` spec is preserved.

## [Rust 0.4.1] - 2026-05-15

Documentation-policy and publish-readiness pass. No runtime changes; no
breaking API changes.

### Added

- **Declared MSRV: `rust-version = "1.82"`** in `core/Cargo.toml`. New
  `rust-core-msrv` CI job builds with Rust 1.82 to guard the contract.
- **Strict documentation lints** at crate root: `#![deny(missing_docs,
  rustdoc::broken_intra_doc_links, clippy::missing_errors_doc)]`.
  Backfilled doc comments and `# Errors` sections across the public API.
- **README is now the crate-level rustdoc** via
  `#![doc = include_str!("../README.md")]`. README code blocks tagged
  `rust,ignore` so doctests stay green.
- **`docs.rs` renders all features with feature badges.**
  `[package.metadata.docs.rs]` switched to `all-features = true` plus
  `--cfg docsrs`; `aio` and other `tokio-comp`/`tracing`-gated items
  carry `#[cfg_attr(docsrs, doc(cfg(...)))]`.
- **`check-cfg` declaration** for the `python` and `js` feature flags
  used by the FFI binding crates, silencing the `unexpected_cfgs` warnings
  that previously polluted `cargo build`/`cargo doc` output.

### Internal

- Auto-fixed 21 `elided_named_lifetimes` warnings via `cargo fix`.
- Tagged `aio::WebSocketClient::send_text` as `#[allow(dead_code)]` with
  a `reason` (kept for future direct-frame test harness).

## [Rust 0.4.0] - TBD

Production-readiness pass driven by the `monitor` integration: opt-in
`tracing`, secret-redacting `Debug`, sane reconnect default, REST retry
policy, multi-connection event labels, graceful shutdown drain, JS-style
WebSocket factory, and the removal of a small set of legacy constructors.
See `MIGRATION-0.4.md` for the full migration guide.

### Breaking

- **`ConnectionEvent::Disconnected` gains `intent: DisconnectIntent { Client, Server, Network }`.**
  `ConnectionState::Closed` mirrors the same `intent` field. Other
  `ConnectionEvent` variants are unchanged. Pattern matches on
  `Disconnected` need `..` or explicit `intent` destructuring.
- **`ReconnectionConfig::default().enabled` flipped `false` → `true`.**
  Rust callers on the `WebSocketClient::new(config)` happy path get
  auto-reconnect by default. Bindings explicitly call
  `ReconnectionConfig::disabled()` at the FFI boundary so end-user
  behavior is preserved (workspace-level CI gate in
  `core/tests/reconnect_default.rs`).
- **`Auth` and `ConnectionConfig` `Debug` redacted.** `Auth::ApiKey(***)`
  etc. instead of the raw token. `ConnectionConfig::url`'s sensitive
  query parameters (`token`, `key`, `apikey`, `api_key`, `secret`,
  `password` — case insensitive) are masked. Logs and `tracing` output
  now safe by default.
- **`SubscribeRequest::{trades, candles, books, aggregates}` removed.**
  Use `SubscribeRequest::new(Channel::*, symbol)`. Zero non-test callers
  in the workspace.
- **`disconnect()` is now a graceful drain, not fire-and-forget.**
  Default 5 s drain timeout sends Close, awaits peer Close ack, then
  force-closes on timeout. Use
  `WebSocketClient::shutdown_with_timeout(Duration)` for a custom
  budget; `Duration::ZERO` matches the old fire-and-forget behavior.

### Added

- **Opt-in `tracing` feature** (`features = ["tracing"]`). Hot-path
  `debug!` for received frames, lifecycle `info!`/`warn!` for
  connect/auth/reconnect/heartbeat, `error!` for runtime-init / close-
  frame failures. `#[tracing::instrument]` spans named
  `ws.connect` / `ws.subscribe` / `ws.unsubscribe` / `ws.disconnect`
  on cold path only — zero overhead on the per-frame dispatch loop.
  Replaces 3 of 5 `eprintln!` sites; the 2 panic-boundary sites stay as
  `eprintln!` so they survive subscriber teardown.
- **`RestClient::with_retry(RetryPolicy)`** — opt-in exponential backoff
  with uniform jitter. `RetryPolicy::conservative()` (3 attempts, 100 ms
  initial, 2 s ceiling) and `RetryPolicy::aggressive()` (5/250 ms/10 s)
  presets. Retries only errors classified by
  `MarketDataError::is_retryable()`.
- **`Auth::from_env()`** — probes `FUGLE_API_KEY` →
  `FUGLE_BEARER_TOKEN` → `FUGLE_SDK_TOKEN`, treats empty string as
  unset.
- **`WebSocketFactory`** — JS / Python SDK-equivalent factory taking one
  auth + optional shared base URL. `.stock()` / `.futopt()` return
  `ConnectionConfigBuilder` for further chaining. Mirrors
  `fugle-marketdata-node/src/websocket/factory.ts` shape.
- **`pub mod urls`** — centralized endpoint constants. Full canonical
  endpoints (`STOCK_WS`, `FUTOPT_WS`, `REST_BASE`) plus host roots and
  version (`WS_BASE_ROOT`, `REST_BASE_ROOT`, `API_VERSION`) for
  composing custom URLs.
- **Configurable channel buffers** — `ConnectionConfig::builder()`
  exposes `message_buffer(usize)` and `event_buffer(usize)`. Default
  `message_buffer` bumped 1024 → 4096 to give multi-symbol consumers
  ~2 s of headroom at TWSE 9:00 open burst (~2000 msg/s);
  `event_buffer` stays at 1024.
- **`messages_dropped_total()` counter** — monotonic `AtomicU64` on each
  client, incremented when the inbound message channel saturates and a
  frame is dropped (drop-newest). Paired with `tracing::warn!` per
  drop.
- **`is_subscribed(&Channel, &str)` + `subscription_count()`** on both
  sync and async clients.
- **`shutdown_with_timeout(Duration)`** + `DEFAULT_SHUTDOWN_TIMEOUT`
  const on both clients (5 s default).

### Internal

- `ConnectionEvent` saturation drop signal moved from `eprintln!` to
  `tracing::warn!` (gated, no-op when feature off).
- Sync `owner_thread` shutdown path now drains write queue → sends
  Close → awaits peer Close ack within `CLOSE_ACK_DEADLINE` (2 s).
  Supervisor exit signaled via mpsc one-shot so `shutdown_with_timeout`
  can bound its wait without `JoinHandle::join_timeout` (which std
  lacks).
- Async dispatch task short-circuits its reconnect loop via a new
  `shutdown_requested: AtomicBool` flag so `disconnect()` cannot race
  the auto-reconnect path.

### Migration

See `MIGRATION-0.4.md` at the repo root.

## [Rust 0.3.0] - TBD

Third Rust crate release — **sync-default `WebSocketClient` with optional
tokio runtime**, following the redis-rs `tokio-comp` pattern. REST already
ran on `ureq` (sync); WebSocket joins it as the default surface. Consumers
that need the async client opt in via a feature flag.

### Breaking — default `WebSocketClient` is now sync

```rust
// 0.2
let client = WebSocketClient::new(config);
client.connect().await?;
client.subscribe(StockSubscription::new(Channel::Trades, "2330")).await?;

// 0.3 (default, no tokio)
let client = WebSocketClient::new(config);
client.connect()?;
client.subscribe(StockSubscription::new(Channel::Trades, "2330"))?;

// 0.3 (async, requires `features = ["tokio-comp"]`)
use fugle_marketdata::aio::WebSocketClient;
let client = WebSocketClient::new(config);
client.connect().await?;
client.subscribe(StockSubscription::new(Channel::Trades, "2330")).await?;
```

`.await` on `connect()`/`subscribe()`/etc. is a compile error after the
upgrade — that's the migration signal. Names and arguments are identical
between the two clients (redis-rs convention).

### New — `tokio-comp` feature

```toml
[features]
default = []
tokio-comp = ["dep:tokio", "dep:tokio-tungstenite", "dep:futures-util"]
```

- `fugle-marketdata` and `fugle-marketdata-core` both expose `tokio-comp`.
- Sync consumers compile with **zero tokio** in `Cargo.lock` (~80 fewer
  transitive crates, ~30-40s faster cold build, ~600-900KB lighter binary).
- Async consumers see no change in dep graph: `tokio-tungstenite 0.29`
  already depends on the same `tungstenite 0.29` that the sync path uses.

### Moved — async API under `aio::`

| 0.2 path | 0.3 path |
|---|---|
| `marketdata_core::WebSocketClient` (async) | `marketdata_core::aio::WebSocketClient` |
| `marketdata_core::AsyncRuntime` | `marketdata_core::aio::AsyncRuntime` (gated) |
| `fugle_marketdata::WebSocketClient` (async) | `fugle_marketdata::aio::WebSocketClient` |

### Removed — redundant async wrappers

- `state_async()` — drop; the sync `state()` reads the same `RwLock`.
- `is_closed()` (async) — folded into the sync `is_closed()`. The 0.2
  `is_closed_sync()` rename intermediate is gone; just use `is_closed()`.
- `message_stream()` — only available on `aio::WebSocketClient` (returns
  `tokio::sync::mpsc::Receiver`). Sync callers use `messages()`.

### Internal refactors (no behavior change)

- New `core/src/websocket/protocol.rs`: framing/parsing helpers shared
  between sync + async paths (wraps existing `WebSocketRequest::{auth,
  subscribe, unsubscribe}` model constructors).
- New `core/src/websocket/connection_event.rs`: runtime-free
  `ConnectionState`, `ConnectionEvent`, `emit_event`.
- New `core/src/websocket/sync/`: blocking client backed by `tungstenite`
  + `std::thread`. Single owner thread per connection with a bounded
  outbound queue (`sync_channel(64)`) and `set_read_timeout`-based
  polling. Supervisor handles automatic reconnect with exponential
  backoff matching the async path.
- `core/src/runtime.rs` moved to `core/src/websocket/aio/runtime.rs`
  (only consumed by FFI bindings; gated behind `tokio-comp`).

### FFI bindings

Python / Node.js / UniFFI / Tauri all enable `tokio-comp` on their
`marketdata-core` workspace dep and import
`marketdata_core::aio::WebSocketClient` explicitly. No FFI surface
change.

Per-binding sync-vs-async evaluation completed in
[docs/FFI-BINDING-RUNTIME-DECISION.md](docs/FFI-BINDING-RUNTIME-DECISION.md):
**all four bindings keep `tokio-comp`** because each maps its target
language's idiomatic async surface (Python `await`, Node `Promise`,
UniFFI `suspend fun` / Swift async, Tauri's tokio runtime) onto the
async client. Sync core remains the canonical entry point for
third-party Rust applications that don't want a runtime imposed.

## [Rust 0.2.0] - TBD

Second Rust crate release — clean-slate subscribe/unsubscribe API and
async-friendly channel surface.

### Breaking — `WebSocketClient` subscribe/unsubscribe API

Seven older methods are removed without a deprecation cycle (0.1.0 was
published 2026-05-15 with zero downstream usage):

- `subscribe(req: SubscribeRequest)`
- `subscribe_channel(sub: StockSubscription)`
- `subscribe_symbols(channel, &[&str], odd_lot)`
- `subscribe_futopt_channel(sub: FutOptSubscription)`
- `unsubscribe(key: &str)`
- `unsubscribe_channel(sub: &StockSubscription)`
- `unsubscribe_futopt_channel(sub: &FutOptSubscription)`
- `unsubscribe_by_id(id: &str)`

Replaced by three methods:

- `subscribe(StockSubscription)` — stock channels
- `subscribe_futopt(FutOptSubscription)` — FutOpt channels
- `unsubscribe(impl IntoIterator<Item = impl Into<String>>)` — single id or batch

`StockSubscription` / `FutOptSubscription` schema changes from `symbol: String`
to `symbols: SymbolSpec`. `StockSubscription::new(channel, symbols)` accepts
`&str`, `String`, `Vec<String>`, array literals (`["A", "B"]`), and slices via
`impl Into<SymbolSpec>`.

`SubscribeRequest` is no longer re-exported from `marketdata_core` — it's an
internal wire type. User code should not construct it directly.

### Added — true batch subscribe / unsubscribe

`StockSubscription::new(Channel::Trades, vec!["A", "B", "C"])` sends one frame
with `{"symbols": ["A","B","C"]}`, gets one ACK array back, and registers N
internal rows in `SubscriptionManager` (one local key per symbol). Previously
each symbol was a separate frame — the Fugle server gateway natively handles
both wire shapes (`stock.gateway.ts:13` and `futopt.gateway.ts:58`) so the
batch path is a real 1-frame-in / 1-ACK-out round-trip, not an N-frame loop.

### Added — async-friendly receive APIs

- `WebSocketClient::message_stream() -> tokio::sync::mpsc::Receiver<WebSocketMessage>`
  for pure-async Rust consumers. Avoids the std-mpsc bridge hop that `messages()`
  incurs.

Internal `message_tx` switches to `tokio::sync::mpsc::channel(1024)`. The
existing `messages()` API stays backward-compatible — first call lazily spawns
a bridge task that drains the tokio channel into a std mpsc for FFI bindings.

`messages()` and `message_stream()` are **mutually exclusive** — each takes
the receiver; calling the other afterwards panics.

### Changed — event channel bounded with drop semantics

`event_tx` switches from `std::sync::mpsc::channel()` (unbounded) to
`std::sync::mpsc::sync_channel(1024)`. All event emission goes through the new
internal `emit_event` helper which uses `try_send` and logs a stderr warning
on saturation. Saturation drops the **new** event (drop-newest) — drop-oldest
would require receiver-side access from the sender, which the
`Arc<Mutex<Receiver>>` public API does not expose without breaking callers.

### Binding changes

- **py / js**: external API unchanged (`subscribe({symbol|symbols})` and
  `unsubscribe({id|ids})` dicts still accepted). Internally the per-symbol
  loop is replaced by a single batch call to core, so the wire now sends
  one frame per `subscribe([...])` call instead of N.
- **UniFFI** (Java / Go / C#): unchanged in this release — UDL cannot express
  the generic `IntoIterator` signature. A dedicated batch surface
  (`subscribe_single` / `subscribe_many`) is planned for 0.3.0.

## [Rust 0.1.0] - 2026-05-15

Initial public release of the Rust SDK on crates.io. Two crates ship together:

- `fugle-marketdata-core` — internal kernel (also used by Python / Node.js /
  Java / Go / C# bindings via FFI)
- `fugle-marketdata` — user-facing facade; depend on this from your
  `Cargo.toml`

The Rust crate publishes on an independent 0.x track so the Rust API can
stabilize without being yoked to the unified 3.x release cadence for the
language-binding family. Once the public surface is judged stable, the crate
will graduate to 1.0.

All behavioral changes listed under [3.0.0] (especially the WebSocket
read-site liveness rework) apply equally to this release; the version split
is purely about release-cadence independence, not feature delta.

## [3.0.0] - TBD

Major release for the binding ecosystem — Python, Node.js, Java, Go, and C#
bindings bump together. The Rust crate publishes separately at 0.1.0 (see
above), sharing the same underlying core kernel.

### WebSocket connection liveness — read-site timeout (BREAKING)

The background activity-timer task is replaced with a `tokio::time::timeout`
wrapped at the WebSocket read site inside `dispatch_messages`. No more polling
task, no atomic timestamps, no `pause`/`resume` choreography during reconnect.
Detection latency improves from up to 90s (3 × 30s heartbeats missed) to the
configured `heartbeat_timeout` (default 35s).

#### Breaking
- `HealthCheckConfig` collapsed `interval` + `max_missed_pongs` into a single
  `heartbeat_timeout: Duration` field. Use
  `HealthCheckConfig::with_timeout(Duration::from_secs(35))?` to construct,
  or `HealthCheckConfig::default()` for the new 35s default.
- `HealthCheckConfig::enabled` default changed from `false` to `true`. Restore
  previous opt-out behaviour with `HealthCheckConfig::disabled()`.
- Removed the `HealthCheck` runtime struct (was `pub` but only used internally).
  All `touch` / `pause` / `resume` / `stop` / `spawn_check_task` / `ping`
  methods are gone — the read-site timeout doesn't need them.
- Removed constants `DEFAULT_HEALTH_CHECK_INTERVAL_MS`,
  `DEFAULT_HEALTH_CHECK_MAX_MISSED_PONGS`, `MIN_HEALTH_CHECK_INTERVAL_MS`.
  Replaced by `DEFAULT_HEARTBEAT_TIMEOUT_MS = 35_000` and
  `MIN_HEARTBEAT_TIMEOUT_MS = 5_000`.
- Binding-layer field renames (PyO3 / napi / UniFFI):
  - PyO3: `HealthCheckConfig.ping_interval` + `max_missed_pongs` →
    `heartbeat_timeout_ms`
  - napi: `HealthCheckOptions.ping_interval` + `max_missed_pongs` →
    `heartbeat_timeout_ms`
  - UniFFI: `HealthCheckConfigRecord.interval_ms` + `max_missed_pongs` →
    `heartbeat_timeout_ms`

#### Added
- `MarketDataError::HeartbeatTimeout { elapsed: Duration }` — first-class
  error variant for liveness timeout (error code 3003). PyO3 binding routes
  to the existing `TimeoutError` Python exception; UniFFI binding routes to
  the existing UniFFI `TimeoutError` variant.
- `ConnectionEvent::HeartbeatTimeout { elapsed: Duration }` — distinguishes
  "we stopped hearing from the server" from a server-initiated `Disconnected`
  close frame. Bindings reuse the existing disconnect callback path with a
  synthesized reason string for now; a dedicated `on_heartbeat_timeout`
  callback can be added in a follow-up if user code needs to discriminate.
- `AuthRequest.heartbeat_interval_ms` — wire-only optional field
  (`heartbeatIntervalMs` in JSON) for future client-requested heartbeat
  interval negotiation. Not exposed via builder method until server-side
  honoring lands; see `WEBSOCKET-SERVER-RECOMMENDATIONS.md`.

#### Changed
- WebSocket dispatch loop now uses `tokio::time::timeout(heartbeat_timeout,
  ws_read.next())` at the read site, replacing the background polling task.
- `WebSocketClient` storage shifts from `Arc<HealthCheck>` to
  `HealthCheckConfig` (plain owned value).

### Python (fugle-marketdata on PyPI)

Drop-in successor to the pure-Python `fugle-marketdata` 2.4.1 maintained at
[fugle-dev/fugle-marketdata-python](https://github.com/fugle-dev/fugle-marketdata-python).
`pip install -U fugle-marketdata` brings you to this Rust-based rewrite.

#### Changed (BREAKING)
- Import path renamed from `marketdata_py` to `fugle_marketdata`, matching
  the 2.4.1 convention. A `marketdata_py` shim emits `DeprecationWarning`
  and re-exports for one release; it will be removed in 3.1.0.
- Exceptions now anchored at `fugle_marketdata.*` (previously
  `marketdata_py.*`). Affects traceback display and pickling.

#### Added
- Version aligned with official 2.x series — this is the 3.0 major.

### Node.js / Java / C# / Go

All bindings bump from 0.3.x to 3.0.0 to share a unified SDK version across
the workspace. No API changes in this version beyond the Python-specific
rename above.

## [0.3.0] - 2026-02-16

### Added
- Options object constructor for all language bindings (Python kwargs-only, Node.js options object, Java builder, Go functional options, C# options pattern)
- ReconnectConfig/ReconnectionConfig exposure for WebSocket auto-reconnect control (max_attempts, initial_delay_ms, max_delay_ms)
- HealthCheckConfig/HealthCheckOptions exposure for WebSocket health check control (enabled, interval_ms, max_missed_pongs)
- Exactly-one-auth validation at construction time (Python ValueError, Node.js Error, Java FugleException, Go error, C# ArgumentException)
- Configuration validation at construction time with descriptive error messages
- Java builder pattern for client and config classes
- Go functional options pattern (WithApiKey, WithBearerToken, WithSdkToken)
- C# options pattern with nullable properties
- Configuration constants exported from core (DEFAULT_*, MIN_* constants for binding layers)

### Changed
- **BREAKING**: Python constructors now require kwargs-only parameters (`RestClient(api_key=)`, not `RestClient("key")`)
- **BREAKING**: Node.js constructors now require options object (`new RestClient({ apiKey })`, not `new RestClient('key')`)
- **BREAKING**: Java constructors now require builder pattern (`FugleRestClient.builder().apiKey().build()`)
- **BREAKING**: Go constructors now require functional options (`NewFugleRestClient(WithApiKey("key"))`)
- **BREAKING**: C# constructors now require options classes (`new RestClient(new RestClientOptions { ApiKey = "key" })`)
- Health check default changed from `true` to `false` (aligned with official SDKs)
- ReconnectConfig field rename: `max_retries` → `max_attempts`, `base_delay_ms` → `initial_delay_ms`

### Deprecated
- Python: Positional string constructors (`RestClient("key")`, removed in v0.4.0)
- Python: Static methods `.with_bearer_token()` and `.with_sdk_token()` (removed in v0.4.0)
- Node.js: String constructors (`new RestClient('key')`, removed in v0.4.0)

## [0.2.0] - 2026-01-31

### Added
- Multi-language SDK support (Python, Node.js, C#, Java, Go)
- Complete REST API coverage (26+ endpoints across stock and futures/options)
  - Stock intraday: quote, ticker, candles, trades, volumes
  - Stock historical: candles, stats
  - Stock snapshot: quotes, movers, actives
  - Stock technical: SMA, RSI, KDJ, MACD, Bollinger Bands
  - Stock corporate actions: capital changes, dividends, listing applicants
  - FutOpt intraday: quote, ticker, candles, trades, volumes, products
  - FutOpt historical: candles, daily
- WebSocket streaming with automatic reconnection and exponential backoff
- WebSocket health check monitoring (ping-pong)
- Async support for all language bindings
  - Python: async/await with asyncio
  - Node.js: Promise-based API
  - C#: Task-based async
  - Java: CompletableFuture
  - Go: goroutines and channels
- Type definitions
  - TypeScript: Full .d.ts definitions for Node.js
  - Python: PEP 484 type stubs (.pyi files)
- Error handling with consistent error codes across all languages
- Three authentication methods: API key, bearer token, SDK token
- FFI bindings via PyO3 (Python), napi-rs (Node.js), UniFFI (Java/Go/C#)

[unreleased]: https://github.com/yourusername/fugle-marketdata-sdk/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/yourusername/fugle-marketdata-sdk/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/yourusername/fugle-marketdata-sdk/releases/tag/v0.2.0
