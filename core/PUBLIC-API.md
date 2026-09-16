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

- `+` `RestClient::get_json(&self, &[&str], &[(K, V)])` — sends a GET to an
  arbitrary path with an arbitrary query string and returns the body as-is.
  The typed builders only emit the params they declare; the Node binding
  uses this to forward the legacy `{ symbol, ...query }` object verbatim, as
  the official 1.x SDK does, so every documented query param is reachable.
  Path segments and query keys/values are percent-encoded; auth, retry and
  status handling match the typed builders.

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
