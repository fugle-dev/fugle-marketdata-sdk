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
