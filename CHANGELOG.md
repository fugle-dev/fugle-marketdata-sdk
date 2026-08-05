# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [Bindings 3.0.0-rc.1 / uniffi 0.1.0-rc.1] - 2026-08-05

First release of the Python, Node and UniFFI bindings, aligned with core
0.8.0-rc.1 and therefore with official `@fugle/marketdata` 1.5.0 /
`fugle-marketdata` 2.5.0 from day one.

Because these bindings have never shipped, **none of core's 0.8.0 breaking
changes are breaking for them** — a binding user has never seen the 0.6-era
`base_url` rule. The reversal described below affects only the Rust crates on
crates.io.

### Version tracks

| Artifact | Registry | Version | Why |
|---|---|---|---|
| `fugle-marketdata` | PyPI | `3.0.0rc1` | must exceed the official package's 2.5.0 |
| `@fugle/marketdata` | npm | `3.0.0-rc.1` | must exceed the official package's 1.5.0 |
| C# / Go / Java / C++ | — | `0.1.0-rc.1` | never published, no namespace to supersede |

### Added — Python

- `RestClient(base_url=...)` takes host + path prefix only; a version segment
  raises `TypeError` at construction, matching the official SDK.
- `RestClient.base_url` and `client.stock.base_url` expose the resolved prefix.
- `WebSocketClient(version={"futopt": "v1.0"})`. Omitted products get their
  latest (stock v1.0, futopt v1.1). An unsupported pairing raises `TypeError`
  with the official SDK's wording.
- `client.stock.ownership.etf_holdings(...)` (async + sync). `sort` accepts
  only `"asc"` / `"desc"`; anything else raises `ValueError` rather than being
  dropped, since a typo would otherwise return the opposite series.
- `futopt.intraday.tickers(is_spread=...)`.
- `cargo test -p marketdata-py --no-default-features` now links and runs.
  `extension-module` became an optional (default-on) feature; previously the
  crate's Rust tests could not build at all.

### Added — Node

- Same surface as Python: `baseUrl` semantics, `RestClient.baseUrl` /
  `StockClient.baseUrl` getters, `version` option (typed as
  `StreamingVersionOptions`, so TypeScript rejects an unknown product at
  compile time), `stock.ownership.etfHoldings(...)`, `isSpread`.
- `types.d.ts` gains `EtfHoldingComponent` / `EtfHoldingsEntry` /
  `EtfHoldingsResponse` — `etfHoldings` referenced `EtfHoldingsResponse` in its
  return type without defining it.

### Added — UniFFI (C# / Go / Java / C++)

- `StreamingVersionRecord` for per-product version selection.
- `stock.ownership.etf_holdings` (async + `cpp`-feature sync variant) and the
  three ETF holdings records.
- `RestClient.base_url` / `StockClient.base_url`, `is_spread` on futopt
  tickers.

### Fixed — UniFFI

- **The crate did not compile at all**, on `main`, for an unknown span: 29
  type errors where the mirror records had drifted from core after the
  0.7.2/0.7.3 decode fixes loosened fields to `Option`. Mirrors now match core
  rather than papering over absence with `unwrap_or_default()`.
- `KdjResponse` exposed a single `period`; the endpoint has taken
  `r_period` / `k_period` / `d_period` since 0.7.2.

### Fixed — Tauri GUI

- `StreamTrade` construction and an `Option<u64>` cast; the latter had been
  broken on `main` since core loosened the futopt candle volume field.

### Fixed — Python test suite

- 102 of 138 tests were failing on `main`. They constructed clients
  positionally (removed in 0.4.0), asserted the 2.x `HealthCheckConfig` shape
  (`interval_ms` / `max_missed_pongs` — this SDK has neither), and expected
  `ValueError` where both this SDK and the official one raise `TypeError`.
  Now 142 passed.

  Note: run `maturin develop` before `pytest` — a stale gitignored `.so` under
  `py/fugle_marketdata/` shadows the installed wheel.

## [Rust 0.8.0-rc.1] - 2026-08-05

Aligns with the official `@fugle/marketdata` 1.5.0-rc.5 and
`fugle-marketdata` 2.5.0rc5. Released as a pre-release while the official
SDKs are still in rc.

See [MIGRATION-0.8.md](MIGRATION-0.8.md).

### ⚠️ Breaking

- **`base_url` no longer accepts a version segment — this reverses 0.6.0.**
  A base URL carries the host and path prefix only; the SDK appends the
  version. Passing a 0.6-era base URL (one ending in `/v1.0`) is now
  rejected with a `ConfigError` naming the prefix to use instead.

  This follows the official SDKs, whose rationale is that letting two
  options decide the same path segment forces precedence rules — and those
  rules make anyone who only wants to change host manage the version by
  hand. That matters more now that streaming versions are per-product: with
  `futopt` on `v1.1` and `stock` on `v1.0`, one baked-in segment cannot be
  right for both.

  Unlike the 0.6.0 change, this failure is loud rather than silent.

- **`WebSocketFactory::stock()` / `::futopt()` now return `Result`**, so a
  rejected `base_url` surfaces at the earliest honest point. `RestClient`
  keeps an infallible `base_url()` and surfaces the rejection from the first
  request; `try_base_url()` reports it immediately instead.

### ⚠️ Behaviour change

- **futopt streaming defaults to `v1.1`**, which delivers trial-matching
  (試撮, TAIFEX I022/I082) frames on `trades` / `books`. A trial frame is a
  simulated match, not a trade — branch on `is_trial` before acting on a
  price. Pin `FutOptVersion::V1_0` to opt out.

  `urls::FUTOPT_WS` and `ConnectionConfig::fugle_futopt` moved to `v1.1`
  in step, so they cannot drift from the factory.

  Note that `aggregates` is **not** version-gated: it carries trial data on
  every version, and pinning `V1_0` does not opt out of it.

### Added

- `stock().ownership().etf_holdings()` — `GET
  /stock/ownership/etf-holdings/{symbol}` with `from` / `to` / `sort`.
- `StockVersion` / `FutOptVersion` enums and
  `WebSocketFactory::{stock_version, futopt_version}`. One enum per product
  makes an unsupported pairing unrepresentable, so unlike the official SDKs'
  runtime-validated version map, a bad combination does not compile.
- `RestClient::resolved_base_url()` — the fully resolved request prefix.
  Since the SDK owns the version segment, this is the only way to see what a
  client actually resolved to.
- `RestClient::try_base_url()`.
- `futopt().intraday().tickers().is_spread(bool)` filter, and `is_spread` on
  `FutOptTicker`.
- `FutOptQuote`: `market`, `price_limits`, `last_trial`, `trading_halt`,
  `is_trial`, `is_delayed_open`, `is_delayed_close`, `is_continuous`,
  `is_open`, `is_close`, `serial`. `FutOptTotalStats` goes from 3 fields to
  8; `FutOptLastTrade` gains `bid` / `ask` / `serial`. New
  `FutOptPriceLimits` and `FutOptTradingHalt`.
- Streaming frames: `is_trial` on `TradesData` / `BooksData` /
  `AggregatesData`; `derived_bid` / `derived_ask` / `data_type` / `exchange`
  on `BooksData`; `time` / `serial` / `is_replaced` on `StreamTrade`;
  `last_trial` on `AggregatesData`.
- `TradeInfo` gains `serial: Option<String>` — stock's `lastTrade` /
  `lastTrial` carry one and it was previously discarded.
- Optional boolean flags (`is_trial`, `is_replaced`) now tolerate an explicit
  JSON `null` as well as an absent key. `#[serde(default)]` alone only covers
  the absent case; a literal `null` failed the whole decode. The API
  demonstrably uses explicit nulls for unset fields on dormant contracts.
  Precautionary — no `isTrial: null` has been observed in the wild.
- `prod_smoke` probes for etf-holdings, the `isSpread` filter, and spread
  contracts (discovered dynamically).

### Fixed

- **`lastTrade.serial` / `lastTrial.serial` decode correctly on futopt.**
  The server sends a zero-padded **string** (`"00379320"`) for futopt and a
  **number** (`17738549`) for stock — the same field, different JSON types
  per product. The official TypeScript interface declares `serial: number`
  for both, which is wrong for futopt; typing it that way made
  `futopt/intraday/quote` fail to decode outright, and would have broken
  futopt's streaming `aggregates` frame too, since it carries the same
  object. Both spellings now normalise to `String` — a serial is an opaque
  identifier, never an operand, and futopt's padding is significant.

  Found by running the sweep against a live environment. Same class of bug
  as 0.7.2/0.7.3: the published type did not match the payload.

- **Symbol path segments are percent-encoded.** Spread contract symbols
  carry a `/` (e.g. `BRFJ6/F7`). Applied to all 19 endpoints that put a
  symbol in the path. The encoder reproduces `encodeURIComponent`'s reserved
  set exactly, so a symbol encodes identically here and in the Node SDK.

  Measured caveat: the live gateway currently *tolerates* an unencoded
  slash — encoded and unencoded requests return identical responses. So
  this is correctness-by-spec and protection against any symbol containing
  reserved characters, not the repair of an observed outage.

### Notes

- The official SDKs' 1.5.0 health-check rework (freshness-based detection
  plus a disconnect reason, and the `maxMissedPongs >= 1` clamp) needs no
  counterpart: this SDK has used a single async-native timeout window since
  0.3.0, and has no missed-pong counter to clamp. `health_check`'s module
  docs now carry a mapping table for anyone porting config from Node or
  Python.
- `name` / `previous_close` were dropped from the official futopt quote
  response in 1.5.0 but are retained here as `Option`, so payloads still
  carrying them keep decoding.

## [Rust 0.7.3] - 2026-05-16

Follow-up to 0.7.2: a deeper prod sweep showed the futopt
symbol-dependent endpoints were never decode-tested because the
after-hours session value was wrong.

### Fixed

- **`futopt/intraday/tickers` & `futopt/intraday/products`**:
  `after_hours()` emitted `session=afterhours`, but these two endpoints
  require the **uppercase** `session=AFTERHOURS` — lowercase is silently
  accepted and returns **zero rows**. So `.after_hours()` on tickers/
  products appeared to "work" while always yielding an empty list. Now
  emit `AFTERHOURS`. (quote/ticker/candles/trades/volumes correctly keep
  lowercase `afterhours` — the server is genuinely inconsistent across
  endpoints; verified against prod.)

### Changed

- `core/examples/prod_smoke`: futopt contract discovery now queries the
  (populated) after-hours tickers list and prefers a `TXF*` contract,
  falling back to the current near-month `TXFF6` (was the long-expired
  `TXFE5`, which 404'd the entire futopt REST+WS sweep).

## [Rust 0.7.2] - 2026-05-16

Decode-correctness patch. A full prod-environment smoke sweep (every REST
endpoint + every WS channel, new `core/examples/prod_smoke` harness)
surfaced response models written against the API spec rather than real
payloads. Several endpoints were **completely unusable** before this fix.

### Fixed

- **`stock/intraday/tickers` & `futopt/intraday/tickers`**: `send()`
  deserialised the body straight into `Vec<Ticker>` / `Vec<FutOptTicker>`,
  but prod wraps the list in an envelope object
  (`{date,type,exchange,data:[…]}`). Every call failed with
  `invalid type: map, expected a sequence`. Now decodes the envelope and
  returns `.data`. **Both endpoints were 100% broken.**
- **`futopt/intraday/products`**: `Product.end_session` was `Option<i32>`
  but prod sends it as the string `"1"`. New `de_opt_i32_flexible`
  deserializer accepts a JSON int, a numeric string, `null`, or `""`.
- **`stock/historical/stats`**: `StatsResponse.change_percent` was a
  required `f64`; prod never sends `changePercent`. Now `Option<f64>`.
- **`stock/technical/{sma,rsi,kdj,macd,bb}`**: responses required
  `type`/`exchange`/`market`/`timeframe`; prod returns none of them. All
  four are now `Option`.
- **`stock/technical/macd`**: data points expected `macd`/`signal`/
  `histogram`; prod sends `macdLine`/`signalLine` and no histogram.
  Renamed via serde; `histogram` is now `Option<f64>`.
- **`stock/technical/kdj`**: `KdjResponse` expected a single `period`;
  prod returns `rPeriod`/`kPeriod`/`dPeriod`. Response fields corrected.

### Added

- `KdjRequestBuilder::r_period` / `k_period` / `d_period` setters. The
  endpoint requires `rPeriod`/`kPeriod`/`dPeriod`; previously only
  `period` could be set, so the endpoint was unreachable (HTTP 400) via
  the SDK.
- `core/examples/prod_smoke` — re-runnable REST+WS decode sweep that
  classifies each probe by `MarketDataError` variant (Schema vs HTTP vs
  param) and emits one JSON record per endpoint.

## [Rust 0.7.1] - 2026-05-16

Refactor-only patch on top of 0.7.0. **Zero public API or behaviour
changes.** Internal cleanup driven by a code-reuse / quality / efficiency
review of the 0.7.0 diff.

### Changed

- Extracted shared `await_auth_response` helper in `websocket::aio::reconnect`.
  Both `WebSocketClient::connect` and the internal `try_connect` reconnect
  path now share a single 22-line auth-frame read loop, removing a copy-paste
  risk where the WebSocket auth protocol could drift between fresh-connect
  and reconnect.
- `metrics_compat::build_drop_counters` now passes `client_id` as
  `&str` (`.as_deref().unwrap_or("")`) instead of a freshly allocated
  `String`, saving one allocation per `WebSocketClient::new`.
- Inlined the `delay_ms` binding into the `tracing_compat::warn!` macro
  call in `try_reconnect` so the `Duration::as_millis()` cast is dropped
  along with the rest of the macro tokens when the `tracing` feature is
  disabled. Also clears a stale `unused_variable` warning under that
  feature combo.

## [Rust 0.7.0] - 2026-05-16

Monitor-readiness followups bundle. **Zero breaking changes.** Five
additive improvements driven by SDK user feedback after 0.6.0
integration: macro hygiene, `WebSocketErrorKind::Http` doc relocation,
multi-client mock, transport-drop intent injection, and an opt-in
`metrics` crate integration. See `MIGRATION-0.7.md` for the opt-in
patterns; existing 0.6.0 code compiles unchanged.

### Added

- **Optional `metrics` feature** (`features = ["metrics"]`). When
  enabled, `WebSocketClient::new` registers
  `fugle_marketdata_ws_messages_dropped_total` and
  `fugle_marketdata_ws_events_dropped_total` counters on the active
  `metrics::Recorder`, both labelled with `endpoint` (URL host) and
  `client_id`. Polling getters remain authoritative; the integration
  mirrors. Off by default — no transitive cost without the feature.
- **`ConnectionConfig::client_id(...)` builder field** —
  caller-supplied low-cardinality identifier used as a metric label.
  64-byte cap with `tracing::warn!` on truncation. New `client_id() ->
  Option<&str>` accessor.
- **`MockWsServer::start_with_capacity(n)`** + per-client targeting via
  `inject_frame_for(idx, …)`, `next_subscribe_id_for(idx, …)`,
  `close_for(idx, code, reason)`. New convenience `aio_pair_n(n)`. The
  bare `inject_frame` / `next_subscribe_id` / `close` panic on
  multi-client mocks with a message naming the `_for` alternative.
- **`MockWsServer::drop_transport(...)`** + `drop_transport_for(idx)` —
  closes the underlying TCP socket without sending a Close frame,
  forcing `DisconnectIntent::Network` on the client side. Idempotent.
- **`metrics_compat::DropCounter`** internal wrapper around
  `Arc<AtomicU64>` + optional `metrics::Counter`. Single bump path
  guarantees the polling-getter atomic and the `metrics` recorder stay
  in lock-step.
- **`core/PUBLIC-API.txt` snapshot** + `core/tests/public_api_snapshot.rs`
  (ignored by default; CI runs explicitly) + new
  `.github/workflows/public-api.yml` job filtered on `core/src/lib.rs`,
  `core/src/tracing_compat.rs`, `core/Cargo.toml`. Acknowledge
  intentional surface changes in `core/PUBLIC-API.md`.

### Changed

- **`WebSocketErrorKind::Http(u16)` doc-comment** now contains the full
  status-code → `ErrorKind` / `is_retryable()` mapping table.
  `MarketDataError::source_kind`'s rustdoc cross-references the
  variant. A `#[cfg(test)]` consistency assertion in `core/src/errors.rs`
  exercises representative status codes against both methods so
  doc-vs-impl drift fails CI.
- **`tracing_compat` module-level rustdoc** explains why
  `__tracing_noop` is exported at crate root via `#[macro_export]` and
  reaffirms it is internal-only (carries `#[doc(hidden)]`).

### Documentation

- **`MIGRATION-0.7.md`** — opt-in patterns for the four feature
  additions plus the REST + WebSocket dual-host pattern (already
  supported in 0.6.0; 0.7.0 makes it discoverable). Uses
  `your-ws-host.example.com` as the placeholder host name; never
  references internal-only hostnames.
- **`core/README.md`** — new "Feature flags" table covering
  `tokio-comp` / `tracing` / `test-utils` / `metrics`. New "Independent
  endpoints" paragraph in "Which constructor should I use?" cross-
  referencing `MIGRATION-0.7.md`.

### Origin

Five-point feedback list from 0.6.0 SDK consumer reviews
(2026-05-15…16):

1. `tracing_compat` macro hygiene clarification.
2. `WebSocketErrorKind::Http` mapping table relocation.
3. `MockWsServer` multi-client support for monitor's dual-probe topology.
4. `MockWsServer` `DisconnectIntent::Network` injection for incident-
   classifier testing.
5. `metrics` ecosystem integration so consumers don't hand-roll
   `gauge.set(client.messages_dropped_total())` boilerplate.

## [Rust 0.6.0] - 2026-05-15

Minor-version pass: `WebSocketError` structured-kind split, `MockWsServer`
test utility, and an OpenAI-aligned `WebSocketFactory::base_url` semantic
shift. See `MIGRATION-0.6.md` for the full migration recipe.

### Breaking — SILENT (no compile error; behaviour changes)

- **`WebSocketFactory::base_url(...)` now expects the full URL prefix
  including the API version segment.** Pre-0.6.0 the factory silently
  injected `/v1.0`; 0.6.0 does not. Code that worked in 0.5.x compiles
  against 0.6.0 but produces a 404 on first connect because the URL
  lacks `/v1.0`. Aligns with OpenAI / Stripe / AWS / Anthropic SDK
  conventions. **READ `MIGRATION-0.6.md` §1 BEFORE UPGRADING PROD.**

### Breaking — type-level (compile error)

- **`MarketDataError::WebSocketError` reshape**:
  `{ msg: String }` → `{ kind: WebSocketErrorKind, msg: String }`. Pattern
  matches need the new `kind` field (or use `..`).
- **`is_retryable()` retry verdict refined.**
  Protocol violations (`Protocol`, `Capacity`, `Utf8`) and TLS failures
  are now **non-retryable** — they were retryable in 0.5.x. Migration
  table in `MIGRATION-0.6.md` §2.
- **`From<tungstenite::Error>` no longer routes to `ConnectionError` /
  `AuthError` for WebSocket transport failures.** Every upstream variant
  produces a `MarketDataError::WebSocketError { kind, msg }`.

### Added

- **`WebSocketErrorKind` enum** (`#[non_exhaustive]`): `Protocol`,
  `Capacity`, `Utf8`, `Tls`, `Io`, `Http(u16)`, `Other`. Re-exported at
  crate root.
- **`MarketDataError::source_kind()` mapping refined** to honour
  `WebSocketErrorKind` — `Io` → `Network`, `Tls` → `Auth`,
  `Http(429)` → `RateLimit`, etc.
- **`core::testing::MockWsServer`** behind `features = ["test-utils"]`.
  In-process WebSocket server with subscribe-ACK echo, frame injection,
  and server-initiated close. `aio_pair()` convenience constructor pairs
  it with a pre-configured async client. Replaces hand-rolled echo
  servers in monitor / py / js / uniffi test suites.
- **`test-utils` cargo feature** (off by default; pulls `tokio-comp`
  transitively).
- **`tests/mock_server_smoke.rs`** — 5-scenario smoke test catching
  drift between the mock's subscribe protocol and production
  `protocol.rs`.

### Internal

- `core/src/websocket/factory.rs`: `endpoint_for(kind)` now reads from
  `urls::STOCK_WS` / `urls::FUTOPT_WS` directly on the no-override path;
  appends only `/{kind}/streaming` on the override path.
- `uniffi/src/errors.rs`: shadow `WebSocketError { msg }` retained for
  FFI ABI stability; conversion at boundary stringifies `kind` into `msg`.

## [Rust 0.5.1] - 2026-05-15

Polish pass between 0.5.0 (databento patterns) and 0.6.0 (WebSocketError
split + mock server). All changes are additive or doc-only; no API
removals.

### Added

- **`MarketDataError::source_kind() -> ErrorKind`** — coarse-grained
  classification helper returning one of `Network`, `Protocol`, `Auth`,
  `RateLimit`, or `Client`. Lets monitor / downstream code branch on
  failure category without pattern-matching every variant.
- **`ErrorKind` enum** — `#[non_exhaustive]`, re-exported at the crate
  root. Includes a dedicated `RateLimit` variant for HTTP 429 so the
  incident-response action ("reduce request volume") doesn't get mixed
  with `Network` failures ("retry with backoff").
- **`events_dropped_total() -> u64`** on both sync and async
  `WebSocketClient`. Mirrors `messages_dropped_total()` for the
  lifecycle event channel; increments when the bounded channel drops
  events under the drop-newest backpressure policy.

### Documentation

- **`Symbols::normalized` case-sensitivity policy** is now explicit in
  the module rustdoc: dedup is **byte-for-byte case-sensitive**.
  `"TXFB6"` and `"txfb6"` are distinct subscriptions, matching the
  TWSE / Fugle wire contract.
- **"Which constructor should I use?"** section in `core/README.md`
  pinning the idiomatic choice for the four construction paths
  (`bon` builder, positional `new(...)`, typestate factory,
  convenience constructors).
- **Stability promise** sections on
  `ReconnectionConfig::disabled()`,
  `RetryPolicy::conservative()`, and `RetryPolicy::aggressive()`.
  These functions are FFI-load-bearing and will be preserved across
  every `0.x` release.

### Internal

- `emit_event` now takes a `&Arc<AtomicU64>` drop counter; 56 call
  sites updated. `dispatch_messages`, `try_reconnect`, `try_connect`,
  and `run_writer_task` carry the counter through to the saturation
  point.

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
