# FFI Binding Runtime Decision (Rust 0.3 follow-up)

Status: **decided** — 2026-05-15

After Rust 0.3.0 introduced the redis-rs-style sync default and gated the
async client behind `tokio-comp`, the [plan][plan] left open whether each
FFI binding should switch to the sync core to drop their tokio dependency.

This document is the per-binding evaluation that closes that question.

[plan]: ../.claude/plans/rust-redis-rs-effervescent-wren.md

## Outcome

**All four bindings (py / js / uniffi / tauri) keep the `tokio-comp`
feature enabled and continue using `marketdata_core::aio::WebSocketClient`.**

No binding-side rewrite ships in 0.3.x. The decision is revisitable when
upstream binding frameworks change shape (e.g. UniFFI sync-async parity
matures, or napi-rs grows first-class blocking workers).

## Per-binding rationale

### `py/` — PyO3 + pyo3-async-runtimes

- **User-visible API is async.** All client methods are exposed as
  awaitables that return Python coroutines (`await client.connect()`,
  `await client.subscribe(...)`). Switching to the sync core would
  require redesigning the Python API to be blocking, breaking the
  documented `asyncio` contract and the parity with the official
  PyPI [`fugle-marketdata`][pypi] package the user maintains.
- **Cost of conversion:** rewrite all 3 `future_into_py` sites (around
  py/src/websocket.rs:1249, 1337, 1395) to `py.allow_threads(...)`
  + drop `pyo3-async-runtimes` dep. Mechanically small.
- **Cost of API break:** non-trivial. Every downstream notebook /
  script / framework integration that uses `await` breaks.
- **Verdict:** **keep tokio-comp.** The dep tree cost (tokio + a small
  pyo3-async-runtimes) is acceptable relative to keeping the user-
  visible async contract.

[pypi]: https://pypi.org/project/fugle-marketdata/

### `js/` — napi-rs

- **User-visible API is Promise-based.** Node.js consumers do
  `await wsClient.connect()`. napi-rs's idiomatic mapping for that is
  `tokio::spawn` + a oneshot to resolve the JS Promise.
- The auth handshake completion signal (`js/src/websocket.rs:482-552`)
  and the `threadsafe_function` callback fan-out depend on running
  inside a tokio runtime that napi-rs already spins up via `tokio_rt`.
- **Cost of conversion:** large. Promise resolution would have to be
  rewired through napi's worker pool with `spawn_blocking`, and the
  event-callback path would need a new dispatch primitive.
- **Verdict:** **keep tokio-comp.**

### `uniffi/` — Cross-language (Kotlin, Swift, Go, …)

- Marked `#[uniffi::export(async_runtime = "tokio")]` across 30
  call sites. Kotlin and Swift consumers expect `suspend fun` / async
  semantics that UniFFI's tokio integration maps cleanly to.
- Go consumers already get sync mappings (the existing `cpp` feature
  strips async for `uniffi-bindgen-cpp`). A future Go-only optimization
  could pivot to sync core, but that's separate from the 0.3 refactor.
- **Cost of conversion:** the highest of the four. Would need a per-
  language eval of async-vs-sync surface mismatches.
- **Verdict:** **keep tokio-comp.** Revisit if/when UniFFI sync-async
  parity reaches feature complete for all target languages.

### `gui-web/src-tauri/` — Tauri 2.x desktop

- Runs inside Tauri 2's tokio runtime. `tokio::sync::Mutex<BridgeInner>`,
  `tokio::spawn`, and `tokio::task::spawn_blocking` are already woven
  throughout `bridge.rs`.
- Calling the sync `WebSocketClient` from inside a tokio task is the
  opposite of an optimization — it would force `spawn_blocking` around
  every WS operation just to keep the async ergonomics the rest of the
  bridge already gives for free.
- **Verdict:** **keep tokio-comp.** Sync core only makes sense for
  CLI/embedded callers, neither of which describes this app.

## Cost / benefit summary

| Binding | Drop tokio benefit | Conversion cost | Decision |
|---|---|---|---|
| py     | small (drops `pyo3-async-runtimes`, ~5 transitive crates) | API break — every `await` site rewritten | **keep** |
| js     | minimal (napi-rs already pulls tokio for other reasons) | large (Promise plumbing rewrite) | **keep** |
| uniffi | small (one async_runtime arg) | large (per-language surface re-eval) | **keep** |
| tauri  | none (tauri brings tokio anyway) | large (anti-pattern) | **keep** |

The sync core's real audience is **third-party Rust applications that
don't want a runtime imposed on them**. That goal is satisfied by the
default `fugle-marketdata = "0.3"` config. Bindings exist to expose the
SDK to async-native runtimes; making them sync would be a regression for
the consumers they serve.

## When to revisit

Concrete triggers that would justify reopening this decision:

1. **UniFFI sync-async parity** for Kotlin coroutines + Swift async/await
   reaches "first-class" status equivalent to tokio. (Tracked upstream;
   not on a near-term timeline as of 2026-05.)
2. **napi-rs blocking worker primitive** lands. Currently `tokio_rt` is
   the only reasonable Promise driver. If napi-rs adds first-class
   blocking workers with Promise integration, the js binding becomes a
   candidate.
3. **PyPI Python SDK direction change.** If the official sync-only
   `fugle-marketdata` Python release strategy changes (e.g. moving fully
   to a sync API), the py binding can mirror that shift.
4. **A separate Go-only release lane.** If UniFFI Go bindings warrant a
   sync-only feature flag, that would justify carving the uniffi crate
   into a `tokio-comp` / sync-comp split similar to the core's redis-rs
   pattern.

Until then: bindings track `tokio-comp`, and `fugle-marketdata` (no
features) remains the canonical zero-runtime entry point.
