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
cargo install cargo-public-api --locked  # one-time
cargo public-api -p fugle-marketdata-core --simplified > core/PUBLIC-API.txt
```

Add an entry to the **Acknowledged changes** section below referencing the
PR number and listing the new/changed/removed symbols.

## Acknowledged changes

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
