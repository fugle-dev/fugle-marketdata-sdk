# Public API surface tracking — `fugle-marketdata-core`

This file documents how the public-API regression check works and lists
acknowledged additions/changes per release.

## How it works

1. The full public surface is captured in `core/PUBLIC-API.txt` (text dump
   from `cargo public-api`).
2. `core/tests/public_api_snapshot.rs` is `#[ignore]`d by default; CI runs
   it explicitly via `cargo test -p fugle-marketdata-core --all-features
   --test public_api_snapshot -- --ignored --include-ignored`.
3. The CI workflow `.github/workflows/public-api.yml` runs `cargo
   public-api` on PRs that touch `core/src/lib.rs`, `core/src/tracing_compat.rs`,
   or `core/Cargo.toml`. A non-empty diff fails the job unless this file
   has a matching acknowledgement entry.

## Regenerating the snapshot

After landing an intentional public-surface change:

```bash
# Versions pinned in .github/workflows/public-api.yml; other nightlies render
# some items differently and the snapshot would not match CI.
rustup toolchain install nightly-2026-08-05 --profile minimal  # one-time
cargo install cargo-public-api --version 0.52.0 --locked        # one-time
RUSTC_BOOTSTRAP=1 cargo +nightly-2026-08-05 public-api -p fugle-marketdata-core --simplified --all-features > core/PUBLIC-API.txt
```

Use `--all-features`: CI and `core/tests/public_api_snapshot.rs` both do, so a
snapshot generated without it will never match.

Add an entry to the **Acknowledged changes** section below referencing the
PR number and listing the new/changed/removed symbols.

## Acknowledged changes

### Unreleased — top-level `message` on error frames (#209)

- `+` `models::WebSocketMessage::message: Option<String>` — the server's
  code-less error shape `{"event":"error","message":"…"}` carries its
  message at the top level, not under `data`. `error_message()` reads
  `data.message` first and falls back to this field. Additive; struct
  literals of `WebSocketMessage` gain one field, as with `code` in #201.

### Unreleased — reconnect policy follows the server; the last `error` code (#201)

- `~` `websocket::ReconnectionManager::should_reconnect(&self, Option<u16>)`
  → `should_reconnect(&self, close_code: Option<u16>, last_error_code: Option<i32>)`.
  The server rejects credentials with `error{1000}` and then a Close without
  a code, so the close code alone cannot tell a rejection from a dropped
  connection. Not reconnecting is the enumerated set (disabled, close `1000`,
  last error `1000`); 4xxx now reconnects.
- `+` `models::WebSocketMessage::code: Option<i32>` — the server's error
  code, sent at the top level of the frame next to `event`.
- `+` `models::WebSocketMessage::error_code(&self) -> Option<i32>` — `code`
  when the frame is an `error` event.

### Unreleased — WebSocket auth timeout as configuration (#199)

The auth handshake limit was a hardcoded 10 s; it is a `ConnectionConfig`
field now, next to `connect_timeout`. The default is unchanged.

- `+` `websocket::config::ConnectionConfig::auth_timeout: Duration` (also
  visible through the `websocket::ConnectionConfig` and `ConnectionConfig`
  re-exports).
- `+` `websocket::config::ConnectionConfigBuilder::auth_timeout(Duration)`
  — panics on zero, like `message_buffer` / `event_buffer`.
- `+` `websocket::config::DEFAULT_AUTH_TIMEOUT` (re-exported from
  `websocket`) — 10 s.
- `+` `websocket::config::auth_timeout_from_millis(u64) -> Result<Duration,
  MarketDataError>` (re-exported from `websocket`) — the bindings' shared
  validation: zero is a `ConfigError`.
- `+` `testing::MockWsServer::set_answer_auth(bool)` (`test-utils`) — leave
  the auth frame unanswered so the timeout can be exercised.

### Unreleased — one shape for `sort`: `sort(&str)` everywhere (#179)

`sort` had three shapes in core (`sort(&str)`, `sort_asc()` / `sort_desc()`,
`sort(HoldingsSort)`) for one server contract (`asc|desc`). It is `sort(&str)`
on every builder now, in line with #164: keys are checked, values are sent as
given and rejected by the server. A sort value the server adds later needs no
SDK change.

- `-` `stock::intraday::TradesRequestBuilder::sort_asc`, `::sort_desc` —
  replaced by `+` `TradesRequestBuilder::sort(&str)`.
- `-` `stock::ownership::HoldingsSort` (the enum, its variants and derived
  impls).
- `~` `stock::ownership::{EtfHoldingsRequestBuilder,
  InstitutionalTradesRequestBuilder, DirectorHoldingsRequestBuilder,
  TdccDistributionRequestBuilder}::sort` — takes `&str` instead of
  `HoldingsSort`.

### Unreleased — REST query params checked against the server (#164, #166, #168)

The server's DTOs (fugle-realtime `apps/api-gateway`, `apps/service-stock`)
are the source of truth, not developer.fugle.tw. Builders only set keys; values
are left for the server to reject.

- `+` `stock::intraday::CandlesRequestBuilder::sort`
- `+` `stock::intraday::TickersRequestBuilder::is_attention`, `is_disposition`,
  `is_halted`, `symbol`
- `+` `stock::snapshot::MoversRequestBuilder::type_filter` (sent as `type`),
  `gt`, `gte`, `lt`, `lte`, `eq`
- `+` `stock::snapshot::ActivesRequestBuilder::type_filter` (sent as `type`)
- `+` `stock::corporate_actions::CapitalChangesRequestBuilder::sort`
- `+` `stock::corporate_actions::ListingApplicantsRequestBuilder::exchange`,
  `sort`
- `+` `stock::corporate_actions::DividendsRequestBuilder::exchange`, `sort`
- `+` `futopt::intraday::TickersRequestBuilder::product`
- `+` `futopt::intraday::ProductsRequestBuilder::status`
- `+` `futopt::historical::FutOptHistoricalCandlesRequestBuilder::strike_price`
  (sent as `strikePrice`), `call_put` (sent as `callPut`)
- `-` `stock::technical::BbRequestBuilder::stddev` — the server does not read
  `stddev`; setting it changed nothing (#166).
- `-` `stock::technical::KdjRequestBuilder::period` — the server takes
  `rPeriod`/`kPeriod`/`dPeriod` and computes the window itself; a lone
  `period` got HTTP 400 in prod (#166).
- `-` `stock::corporate_actions::CapitalChangesRequestBuilder::date`,
  `DividendsRequestBuilder::date`, `ListingApplicantsRequestBuilder::date` —
  prod answers `?date=` with 400 `property date should not exist` on
  capital-changes and listing-applicants, and ignores it on dividends (#168).

### Unreleased — health check probe and `measure_latency()` (#150)

- `~` `HealthCheckConfig` gains `probe_enabled: bool`,
  `idle_probe_after: Option<Duration>` and `probe_timeout: Option<Duration>`.
  A struct literal must now name them or end in `..HealthCheckConfig::default()`.
- `+` `HealthCheckConfig::from_parts` (all settings, validated — what the
  bindings convert into), `HealthCheckConfig::with_probe`,
  `idle_probe_after_or_default`, `probe_timeout_or_default`.
- `+` `DEFAULT_IDLE_PROBE_AFTER_MS`, `MIN_IDLE_PROBE_AFTER_MS`,
  `DEFAULT_PROBE_TIMEOUT_MS`, `MIN_PROBE_TIMEOUT_MS` (also at the crate root)
  and `DEFAULT_LATENCY_TIMEOUT_MS`.
- `+` `aio::WebSocketClient::measure_latency` and
  `WebSocketClient::measure_latency` (sync).
- `+` `testing::MockWsServer::set_answer_pings` and `pings_received`; the mock
  now answers `ping` with `pong` echoing `state`, like the server.

### Unreleased — reconnect defaults: on, unlimited attempts (#149)

- `~` `ReconnectionManager::attempts_remaining` returns `Option<u32>` instead
  of `u32`; `None` when attempts are unlimited (`max_attempts == 0`), which
  used to read as `0`, i.e. "exhausted".
- Not visible in the snapshot (values, not signatures): `DEFAULT_MAX_ATTEMPTS`
  is `0` (unlimited) instead of `5`, so `ReconnectionConfig::default()` and
  the builder default never give up; `ReconnectionConfig::new` accepts
  `max_attempts == 0` instead of returning `ConfigError`.

### Unreleased — `connect()` aborted by a concurrent `disconnect()` (#121)

- `+` `MarketDataError::ConnectionAborted` (code 2010, the same code as
  `ClientClosed`) — returned by `aio::WebSocketClient::connect()` when
  `disconnect()` / `force_close()` aborts the handshake. `MarketDataError` is
  already `#[non_exhaustive]` (#119), so this is an addition, not a break.

### Unreleased — credentials checked in core (#69)

- `+` `Auth::from_credentials` — the one-credential, non-blank rule every
  binding applies; the variant keeps which kind was given.
- `+` `Auth::validate` and `AuthRequest::validate` — the same rule for a
  credential built directly; used by `RestClient` and `connect()`.
- `+` `From<Auth> for AuthRequest` — sends the credential in the field of its
  kind, for the WebSocket bindings (#91).

### Unreleased — unified error spec (#81)

- `+` `errors::ErrorInfo` (`#[non_exhaustive]`; `code`, `source_kind`,
  `message`, `status`, `body`, `request_id`, `headers`; `new`) and
  `MarketDataError::info`.
- `+` `errors::HttpErrorContext` (`#[non_exhaustive]`; `status`, `body`,
  `headers`; `new`, `header`, `request_id`).
- `+` `errors::error_code` constants; `ErrorKind::as_str` and
  `Display for ErrorKind`. All re-exported at the crate root.
- `~` `MarketDataError::ApiError` and `MarketDataError::AuthError` gain
  `http: Option<Box<HttpErrorContext>>`.
- `~` `ConnectionEvent::Error { message, code }` becomes
  `ConnectionEvent::Error(ErrorInfo)`.

### Unreleased — `connect()` while connected (#119)

- `~` `MarketDataError` becomes `#[non_exhaustive]`.
- `+` `MarketDataError::AlreadyConnected` (code 2011).
- `+` `ConnectionStateHandle::is_active`.
- `~` `aio::WebSocketClient` is no longer `Freeze` (it holds an atomic
  directly); `Send` / `Sync` are unchanged.
- The regenerated snapshot also picks up earlier unrecorded additions:
  `error_code::CALLBACK_FAILED` / `RECONNECT_FAILED` (#83),
  `websocket::report_throttle` (#83) and `FromStr` for `Channel` /
  `FutOptChannel` (#113).

### Unreleased — connection state that outlives the client (#67)

- `+` `aio::WebSocketClient::state_handle`.
- `+` `websocket::connection_event::ConnectionStateHandle` (`Clone`, `Debug`,
  `state()`, `is_connected()`, `is_closed()`), re-exported at the crate root
  and from `websocket`.

### Unreleased — dropped-message count that outlives the client (#46)

- `+` `aio::WebSocketClient::messages_dropped_handle`,
  `WebSocketClient::messages_dropped_handle`.
- `+` `websocket::stream::MessagesDroppedHandle` (`Clone`, `Debug`, `total()`),
  re-exported at the crate root and from `websocket`.

### Unreleased — one ordered stream of messages and events (#68)

- `-` `aio::WebSocketClient::{messages, message_stream, events, state_events}`
  and `WebSocketClient::{messages, events, state_events}`.
- `+` `aio::WebSocketClient::{stream, stream_receiver}`,
  `WebSocketClient::stream_receiver`.
- `-` `websocket::message` (`MessageReceiver`, `MessageStream`).
- `+` `websocket::stream`: `StreamItem` (`#[non_exhaustive]`),
  `StreamReceiver`, `ConnectionStream` (`futures::Stream`); re-exported at the
  crate root and from `websocket`.

### Unreleased — inbound message queue and `MessagesDropped` (#46)

- `~` `aio::WebSocketClient::message_stream` — returns `MessageStream`
  instead of `tokio::sync::mpsc::Receiver<WebSocketMessage>`.
- `+` `MessageStream` (`recv`, `try_recv`, `poll_recv`, `futures::Stream`),
  re-exported at the crate root and from `websocket`.
- `-` `MessageReceiver::new` — receivers are only created by the clients.
  `+` `Freeze` for `MessageReceiver`.
- `+` `MessageOverflow { DropNewest, Unbounded }` (`#[non_exhaustive]`),
  `ConnectionConfig::message_overflow`,
  `ConnectionConfigBuilder::message_overflow`.
- `~` `ConnectionEvent` — `#[non_exhaustive]`; `+` `MessagesDropped { dropped, total }`.

### Unreleased — connection events carry data and reconnect intent (#55)

- `~` `ConnectionEvent::Authenticated` — unit variant becomes
  `Authenticated { data: serde_json::Value }`.
- `~` `ConnectionEvent::Unauthenticated` — `+` `data: serde_json::Value`.
- `~` `ConnectionEvent::Disconnected` — `+` `will_reconnect: bool`.
- `+` `testing::MockWsServer::set_auth_response` — lets tests serve a custom
  auth reply (with or without `data`, or a rejection).

### Unreleased — futopt historical follows fugle-realtime #727 (#21)

Both endpoints are keyed by product (`TXF`) and take `session` instead of
`afterHours`; see `CHANGELOG.md` for the release-order constraint.

- `+` `FutOptHistoricalCandlesRequestBuilder::{contract_month, fields, sort}`
  — query params the endpoint documents; `contractMonth` selects the contract.
- `+` `FutOptDailyRequestBuilder::date` / `-` `from`, `to` — the endpoint
  returns a single trading day; a range never reached the server.
- `~` `FutOptHistoricalCandlesResponse` — `-` `symbol`, `data_type`;
  `+` `product`, `contract_month`, `session`, `sort`.
- `~` `FutOptHistoricalCandle` — `open` / `high` / `low` / `close` become
  `Option<f64>` (the `fields` param selects them); `+` `contract_month`,
  `average`, `transaction`; `-` `open_interest`, `change_percent`. `body()` /
  `range()` return `Option<f64>`.
- `~` `FutOptDailyResponse` — `-` `symbol`, `data_type`, `highest_high()`,
  `lowest_low()`; `+` `date`, `product`, `session`.
- `~` `FutOptDailyData` — one row per contract month: `-` `date`, `open`,
  `high`, `low`, `close`; `+` `contract_month`, `open_price`, `high_price`,
  `low_price`, `close_price`, `change`, `change_percent`, `volume_spread`,
  `call_put`, `strike_price`, `exchange`; `volume` becomes `Option<u64>`.
  `range()` returns `Option<f64>`. Checked against the #727 gateway on
  api-dev (futures, spread and options rows).
- `FutOptHistoricalClient::daily` is no longer `#[deprecated]`.

### Unreleased — raw GET for verbatim query params (#16)

- No surface change. `RestClient::get_json` is `pub` so the Node binding can
  forward the legacy `{ symbol, ...query }` object verbatim, but it is
  `#[doc(hidden)]`: it bypasses every typed check and is not a supported Rust
  API, so it stays out of the docs and out of `PUBLIC-API.txt`. It may change
  or disappear without a changelog entry.

### 0.9.0-rc.1 — REST responses pass through verbatim (#10)

The typed response models leave the return path. They were on it as a
lossy intermediary: a field they did not declare was silently dropped
(`Quote` was missing `referencePrice` and `serial`), and a field they did
declare but the server omitted was materialised with a default. See
`MIGRATION-0.9.md`.

- `~` every `*RequestBuilder::send` in `rest::**` (32 of them) — now returns
  `Result<serde_json::Value, MarketDataError>` instead of its own response
  model. The server's body reaches the caller untouched.
- `~` `rest::stock::intraday::TickersRequestBuilder::send` and its futopt
  counterpart — previously `Vec<Ticker>` / `Vec<FutOptTicker>`, unwrapped
  from the response envelope. The envelope is now returned whole; the array
  is under `data`. This also restores parity with the official SDK, which
  never unwrapped it.
- `+` `models::Quote::reference_price` — the basis the exchange computes
  `change`, `change_percent` and the limit prices from. Not `previous_close`,
  which differs whenever the reference is adjusted.
- `+` `models::Quote::serial` — the quote's own sequence number, distinct
  from `last_trade.serial`.
- `+` `models::WebSocketMessage::raw` — the frame exactly as received.
  Bindings hand this to the caller instead of re-serializing the routing
  struct, which dropped unknown fields and emitted `null` for absent ones.

`models` stays public and is unchanged apart from the two added `Quote`
fields: Rust callers who want static types can `serde_json::from_value`.
`core/tests/model_fidelity.rs` now decodes a recorded response into a model
and asserts every field survives the round trip, so the gap that motivated
this change cannot reopen silently.

### 0.8.0-rc.1 (cont.) — ureq 3, HTTP client removed from the public API

- `-` `impl From<ureq::Error> for MarketDataError` — exposed the HTTP client's
  error type. Transport failures still surface as `ConnectionError` /
  `TimeoutError`, and error statuses as `AuthError` / `ApiError`.
- `-` `Auth::apply_to_request(&self, ureq::Request) -> ureq::Request` — took and
  returned the HTTP client's request type. Credentials are now applied inside
  the client; there is no replacement because callers never needed to build
  requests themselves.
- `~` `impl !Freeze for RestClient` — the ureq 3 agent holds interior
  mutability directly. `Freeze` only affects const evaluation; `Send` and
  `Sync` are unchanged.

With no HTTP client types left in the surface, replacing the client later is
not a breaking change.

### 0.8.0-rc.1 (cont.) — official 1.6.0 / 2.6.0 ownership endpoints

This is also the first real snapshot. Until now `core/PUBLIC-API.txt` held
only placeholder comments, so the regenerated file lists the whole surface,
including items already acknowledged below and the `websocket::aio`
`WebSocketClient` methods that were never captured.

- `+` `rest::client::OwnershipClient::{institutional_trades, director_holdings,
  tdcc_distribution}`.
- `+` `rest::stock::ownership::{InstitutionalTradesRequestBuilder,
  DirectorHoldingsRequestBuilder, TdccDistributionRequestBuilder}`.
- `+` `models::{InstitutionalInvestorTrade, InstitutionalTradesEntry,
  InstitutionalTradesResponse, DirectorHolding, DirectorHoldingsEntry,
  DirectorHoldingsResponse, TdccDistributionLevel, TdccDistributionEntry,
  TdccDistributionResponse}`. Numeric fields are `Option`: these models follow
  the official TypeScript interfaces and have not been checked against live
  payloads yet.
- `=` `rest::stock::ownership::HoldingsSort` moved to a private `range`
  module and is re-exported from the same public path. No caller-visible
  change.

### 0.8.0-rc.1 — official 1.5.0 / 2.5.0 parity

Breaking and additive changes; see `MIGRATION-0.8.md` for the caller-facing
story.

- `~` `WebSocketFactory::stock` / `::futopt` — now return
  `Result<ConnectionConfigBuilder, MarketDataError>`. A `base_url` carrying a
  version segment is rejected, and this is the earliest point that can report
  it.
- `~` `RestClient::base_url` — semantics reversed (host + prefix only, SDK
  appends the version). Signature unchanged; gained `#[must_use]`.
- `~` `urls::FUTOPT_WS` — value moved from `/v1.0/` to `/v1.1/` so it cannot
  drift from the new futopt default.
- `+` `urls::with_version` — shared base-URL + version join and rejection.
- `+` `websocket::version` module: `StockVersion`, `FutOptVersion`
  (`#[non_exhaustive]`, re-exported from `websocket`).
- `+` `WebSocketFactory::stock_version` / `::futopt_version`.
- `+` `RestClient::try_base_url`, `RestClient::resolved_base_url`.
- `+` `rest::client::OwnershipClient`, `StockClient::ownership`.
- `+` `rest::stock::ownership` module: `EtfHoldingsRequestBuilder`,
  `HoldingsSort`.
- `+` `models::{EtfHoldingComponent, EtfHoldingsEntry, EtfHoldingsResponse}`.
- `+` `models::futopt::{FutOptPriceLimits, FutOptTradingHalt}` and new fields
  on `FutOptQuote`, `FutOptTotalStats`, `FutOptLastTrade`, `FutOptTicker`.
- `+` `models::TradeInfo::serial` (`Option<String>`).
- `~` `FutOptLastTrade::serial` is `Option<String>`, not a numeric type — the
  server sends a zero-padded string on futopt and a number on stock, and both
  normalise to `String`. Verified against live payloads; the official
  TypeScript interface declares `number` for both and is wrong for futopt.
- `+` New fields on `models::streaming::{TradesData, BooksData, StreamTrade,
  AggregatesData}`.
- `+` `TickersRequestBuilder::is_spread`.

Note: `urls::API_VERSION` is retained and still means the REST version.
WebSocket versions are now per-product and live in `websocket::version`.

### 0.7.0 (baseline)

Initial baseline captured at the 0.7.0 release. Contents of
`core/PUBLIC-API.txt` reflect the full public surface as of this release;
no per-symbol acknowledgements are needed because the full set is the
baseline.

Surface highlights established in this release:

- `pub mod testing` adds `MockWsServer::start_with_capacity`,
  `inject_frame_for`, `next_subscribe_id_for`, `close_for`, `drop_transport`,
  `drop_transport_for`, and the convenience `aio_pair_n` (gated behind
  `feature = "test-utils"`).
- `ConnectionConfig::client_id(...)` / `maybe_client_id(...)` builder fields
  and `client_id() -> Option<&str>` accessor.
- No new symbols leak from `tracing_compat` (the `__tracing_noop` macro
  remains `#[doc(hidden)]`).

### Workflow for future releases

For each PR that intentionally changes the public surface:

```markdown
### <semver-bump> (PR #<n>)

- `+` `pub fn ...` — rationale.
- `~` `<symbol>` — signature change reason.
- `-` `<symbol>` — removal reason + migration note.
```
