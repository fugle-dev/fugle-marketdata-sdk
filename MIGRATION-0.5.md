# Migration Guide: Rust 0.4 → 0.5

The Rust `fugle-marketdata` / `fugle-marketdata-core` crates at 0.5.0
adopt three patterns observed in databento-rs that improve compile-time
safety and reduce builder boilerplate:

1. **`SymbolSpec` → `Symbols`** rename, with a normalization +
   deduplication contract.
2. **Typestate `WebSocketFactory`** that compile-checks `auth(...)` is
   set before `stock()` / `futopt()`.
3. **`bon::Builder` derive** for `RetryPolicy`, `ReconnectionConfig`,
   and `SubscribeRequest` — chainable builders with `maybe_*` setters
   for `Option<T>` fields.

`ConnectionConfig` intentionally retains its hand-rolled
`ConnectionConfigBuilder` because the existing zero-capacity-buffer
validation (`message_buffer(0)` / `event_buffer(0)` panic) is not
expressible through `bon`'s derived setters.

## TL;DR

- **Rust callers**: bump `fugle-marketdata = "0.5"`. Two mechanical edits:
  rename every `SymbolSpec` to `Symbols`; convert
  `WebSocketFactory::new(auth)` to `WebSocketFactory::new().auth(auth)`.
  Both produce clear compile errors after the upgrade.
- **Subscription dispatch behavior change**: duplicate symbols passed to
  `with_symbols(...)` or `*Subscription::new(...)` now collapse to one
  subscription instead of producing two server ACKs. Whitespace
  differences are squashed. This was always the intended behavior.
- **Bindings (Python / Node / UniFFI / Go / Java / C#)**: zero impact.
  Bindings never name `SymbolSpec` and construct `ConnectionConfig` at
  the FFI boundary rather than using the factory.
- **Workspace consumers needing fallible `with_*` setters on
  `ReconnectionConfig`**: those methods were removed; use the validating
  positional `ReconnectionConfig::new(...)` instead.

## Breaking changes

### 1. `SymbolSpec` is now `Symbols`

The flexible symbol-input enum moved out of
`core/src/models/subscription.rs` into a dedicated
`core/src/models/symbols.rs` module and was renamed to `Symbols` for
brevity and parity with databento-rs's top-level `Symbols` type.

**Migration** (mechanical):

```bash
# In your crate's Rust sources:
sed -i '' 's/SymbolSpec/Symbols/g' src/**/*.rs
```

All existing `From` impls (`&str`, `String`, `&String`, `Vec<String>`,
`Vec<&str>`, `[&str; N]`, `[String; N]`, `&[&str]`, `&[String]`)
retarget the renamed type, so call sites that relied on
`impl Into<SymbolSpec>` continue to compile after the rename.

The type is re-exported at the crate root (`fugle_marketdata_core::Symbols`)
and the module path (`fugle_marketdata_core::models::symbols::Symbols`).

### 2. `Symbols::normalized` is invoked on subscription dispatch

`SubscribeRequest::with_symbols`, `StockSubscription::new`, and
`FutOptSubscription::new` now run their `impl Into<Symbols>` input
through `Symbols::normalized()` before producing the request:

- Each symbol is trimmed for leading/trailing whitespace.
- Empty entries (post-trim) are dropped.
- Duplicates are removed preserving first-seen insertion order.
- `Many` of length 1 collapses to `Single` for canonical form.

**Observable behavior change**:

```rust
// Before 0.5.0
let req = SubscribeRequest::with_symbols(Channel::Trades, vec!["2330", "2330"]);
// → req.symbols == Some(["2330", "2330"]) — server returns two ACKs,
//   SubscriptionManager records two entries with the same key.

// After 0.5.0
let req = SubscribeRequest::with_symbols(Channel::Trades, vec!["2330", "2330"]);
// → req.symbol == Some("2330"), req.symbols == None — single subscription.
```

If your application depended on the duplicate-ACK behavior (almost
certainly not the intent), file an issue describing the use case.

### 3. `WebSocketFactory` is typestate-enforced

`WebSocketFactory::new(auth)` becomes `WebSocketFactory::new().auth(auth)`.
The Rust type system now prevents `.stock()` / `.futopt()` from being
called before `auth(...)` has set the credential.

**Migration**:

```rust
// Before 0.4.0
let cfg = WebSocketFactory::new(AuthRequest::with_api_key("k"))
    .stock()
    .build();

// After 0.5.0
let cfg = WebSocketFactory::new()
    .auth(AuthRequest::with_api_key("k"))
    .stock()
    .build();
```

`base_url(...)` remains chainable in any state (`Unset` or `WithAuth`),
so the two orderings produce the same result:

```rust
WebSocketFactory::new()
    .auth(auth)
    .base_url("wss://staging.example.com");

WebSocketFactory::new()
    .base_url("wss://staging.example.com")
    .auth(auth);
```

Attempting to call `.stock()` / `.futopt()` on a factory without
`.auth(...)` is now a compile-time error pointing at the missing
`WithAuth` state (verified by `compile_fail` doctests in
`core/src/websocket/factory.rs`).

### 4. `ReconnectionConfig::with_*` chainable validators removed

The fallible setters `with_max_attempts`, `with_initial_delay`, and
`with_max_delay` were removed. They duplicated the validation logic in
`ReconnectionConfig::new(...)` and are replaced by the unvalidated bon
builder (`ReconnectionConfig::builder()`).

**Migration**:

```rust
// Before 0.4.0 — fallible chainable validators
let cfg = ReconnectionConfig::default()
    .with_max_attempts(10)?
    .with_initial_delay(Duration::from_secs(2))?
    .with_max_delay(Duration::from_secs(120))?;

// After 0.5.0 — pick one of:

// (a) Unvalidated bon builder (defaults match `Default::default()`):
let cfg = ReconnectionConfig::builder()
    .max_attempts(10)
    .initial_delay(Duration::from_secs(2))
    .max_delay(Duration::from_secs(120))
    .build();

// (b) Validating positional constructor (raises ConfigError on invalid input):
let cfg = ReconnectionConfig::new(
    10,
    Duration::from_secs(2),
    Duration::from_secs(120),
)?;
```

## Additive: opt-in `bon` builders

These are new and don't require any migration — they're additional APIs.

### `RetryPolicy::builder()`

```rust
let policy = RetryPolicy::builder()
    .max_attempts(4)
    .initial_backoff(Duration::from_millis(50))
    .max_backoff(Duration::from_secs(5))
    .build();
```

The existing `RetryPolicy::new(...)`, `conservative()`, `aggressive()`
constructors remain.

### `SubscribeRequest::builder()`

```rust
let req = SubscribeRequest::builder()
    .channel("trades".to_string())
    .symbol("2330".to_string())
    .maybe_intraday_odd_lot(Some(true))
    .build();
```

`bon` generates `maybe_*` setters for every `Option<T>` field. The
existing `SubscribeRequest::new(channel, symbol)` and
`SubscribeRequest::with_symbols(channel, symbols)` constructors remain
— and `with_symbols` is still the only entry point that enforces the
dedup contract from §2 above.

## New helpers

### `Symbols::normalized() / .len() / .is_empty() / .iter() / .chunked(n)`

```rust
let s: Symbols = vec!["2330", "  2454  ", "2330"].into();
let normalized = s.normalized();
assert_eq!(normalized.len(), 2);
for sym in normalized.iter() {
    println!("{sym}");
}

let chunks = Symbols::from(vec!["A", "B", "C", "D", "E"]).chunked(2);
// → [Many(["A", "B"]), Many(["C", "D"]), Single("E")]
```

### `SUBSCRIPTION_BATCH_LIMIT`

```rust
use marketdata_core::models::symbols::SUBSCRIPTION_BATCH_LIMIT;
assert_eq!(SUBSCRIPTION_BATCH_LIMIT, None);
```

Currently `None` — the SDK does not enforce a per-frame symbol limit
and the dispatch path emits a single subscription frame regardless of
symbol count. Reserved so downstream code can branch on a future
server-documented limit without an additional version bump.

## Verification checklist

After upgrading, run:

```bash
cargo build -p fugle-marketdata-core --all-features
cargo test  -p fugle-marketdata-core --all-features
cargo clippy -p fugle-marketdata-core --all-features -- -D warnings
```

If you maintain FFI bindings:

```bash
cargo build -p marketdata-py -p marketdata-js -p marketdata-uniffi
```

All four targets should pass without source changes in the binding
crates themselves; the SDK changes are entirely on the Rust side of the
FFI boundary.
