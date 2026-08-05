# Migrating to 0.8.0

0.8.0 aligns the Rust SDK with the official Node / Python SDKs' 1.5.0 / 2.5.0
release. Two changes need your attention; everything else is additive.

| | Change | Who is affected |
|---|---|---|
| 1 | `base_url` no longer takes the version segment — **this reverses 0.6.0** | anyone passing a custom `base_url` |
| 2 | futopt streaming defaults to `v1.1`, which delivers trial-matching (試撮) frames | anyone consuming futopt `trades` / `books` |

---

## 1. `base_url` reverses direction

**If you do not pass a custom `base_url`, nothing changes.** Skip to §2.

### What happened

0.6.0 changed `base_url` to require the version segment, following the
OpenAI / Stripe / AWS convention. The official SDKs have now gone the other
way: `baseUrl` carries the host and path prefix only, and the SDK owns the
version segment. 0.8.0 follows the official SDKs.

The official rationale, from `@fugle/marketdata`'s `src/base-url.ts`:

> A version written into `baseUrl` is rejected rather than swapped or appended
> on top of, because letting two options decide the same path segment is what
> forced the old precedence rules — and those rules meant anyone who only
> wanted to change host was made to manage the version by hand.

That reasoning applies with more force now that streaming versions are
per-product (§2): with `futopt` on `v1.1` and `stock` on `v1.0`, a single
version segment baked into `base_url` cannot be right for both.

Reversing a decision two minor versions after making it is not free, and it is
worth saying plainly: the 0.6.0 convention was defensible in isolation, but
matching the official SDKs is what this SDK is for.

### Migrating from 0.6.x / 0.7.x

Delete the version segment from your `base_url`.

```rust
// 0.6.0 - 0.7.x
let factory = WebSocketFactory::new()
    .base_url("wss://staging.fugle.tw/marketdata/v1.0")
    .auth(auth);
let cfg = factory.stock().build();

// 0.8.0
let factory = WebSocketFactory::new()
    .base_url("wss://staging.fugle.tw/marketdata")   // <- no /v1.0
    .auth(auth);
let cfg = factory.stock()?.build();                  // <- now returns Result
```

```rust
// 0.6.0 - 0.7.x
let client = RestClient::new(auth).base_url("https://staging.fugle.tw/marketdata/v1.0");

// 0.8.0
let client = RestClient::new(auth).base_url("https://staging.fugle.tw/marketdata");
```

**This failure is loud, not silent.** Unlike the 0.6.0 change — which compiled
fine and produced a 404 at runtime — a leftover version segment is rejected
with a message naming the exact prefix to use:

```text
base_url must not include a version segment (found '/v1.0'). Pass the host and
path prefix only: 'wss://staging.fugle.tw/marketdata'. The version comes from
the streaming version options, e.g. .futopt_version(FutOptVersion::V1_1).
```

### Migrating from 0.5.x or earlier

You are in luck: 0.8.0 restores the 0.5.x shape. If you never adopted 0.6.0,
your `base_url` strings already work. The only change you need is the new
`Result` on `stock()` / `futopt()`.

### Where the error surfaces

`base_url` stays an infallible builder setter on both clients, so the chain
still reads the same. The rejection appears at the first point that can carry
it:

| Client | Rejection surfaces at | Want it earlier? |
|---|---|---|
| `WebSocketFactory` | `.stock()` / `.futopt()`, now `-> Result<_, MarketDataError>` | already immediate |
| `RestClient` | the first request's `send()` | `try_base_url()` returns `Result` at set time |

```rust
// Report a bad prefix at construction rather than at first request.
let client = RestClient::new(auth)
    .try_base_url("https://staging.fugle.tw/marketdata")?;
```

### Seeing what a client resolved to

Because the SDK now owns the version segment, there is a getter for the
resolved prefix — the only way to see where requests actually go:

```rust
let client = RestClient::new(auth).base_url("https://staging.fugle.tw/marketdata");
assert_eq!(client.resolved_base_url(), "https://staging.fugle.tw/marketdata/v1.0");

let cfg = WebSocketFactory::new().auth(auth).futopt()?.build();
assert_eq!(cfg.url, "wss://api.fugle.tw/marketdata/v1.1/futopt/streaming");
```

---

## 2. futopt streaming defaults to v1.1 (trial frames)

### What changed

Futures/options streaming now serves two versions:

| Version | `trades` / `books` |
|---|---|
| `v1.0` | real matches only |
| `v1.1` **(new default)** | real matches **plus** trial-matching (試撮, TAIFEX I022/I082) frames |

Stock has no `v1.1` — its trials have always been streamed, so there was no
compatibility break to gate.

Because the default moved, **futopt connections that worked unchanged in 0.7.x
now receive extra frames**. A trial frame is a *simulated* match, not a trade.
Treating one as a real trade will corrupt any price-derived state you keep.

### What you must do

Branch on `is_trial` before acting on a price:

```rust
match parse_channel_data("trades", &data, true)? {
    ChannelData::Trades(t) if t.is_trial => {
        // 試撮 — simulated. Do not treat as a fill.
    }
    ChannelData::Trades(t) => {
        // real match
    }
    _ => {}
}
```

The server omits `isTrial` entirely rather than sending `false`, so the field
defaults to `false` and stock frames are unaffected.

### Opting out

```rust
use marketdata_core::websocket::{FutOptVersion, WebSocketFactory};

let cfg = WebSocketFactory::new()
    .futopt_version(FutOptVersion::V1_0)   // back to 0.7.x behaviour
    .auth(auth)
    .futopt()?
    .build();
```

### ⚠️ `aggregates` is not version-gated

Pinning `V1_0` does **not** stop trial data reaching you on the `aggregates`
channel. That channel carries trial data on every version: during a trial
session its `last_price` / `last_size` *are* the trial values, and the
top-level `is_trial` flag is the only thing distinguishing them from a real
trade. The same is true of the REST `futopt/intraday/quote` response.

If you consume `aggregates` or that REST endpoint, you must check `is_trial`
regardless of which streaming version you pin.

### Why enums instead of a version map

The official SDKs take a per-product map (`{ futopt: 'v1.1' }`) and validate it
at runtime, because a bare version string would be ambiguous about which
product it applies to. Here each product has its own enum, so an unsupported
pairing does not compile:

```rust
// Does not compile — FutOptVersion is not a StockVersion.
WebSocketFactory::new().auth(auth).stock_version(FutOptVersion::V1_1);
```

---

## 3. Additive changes (no action required)

### New endpoint: `stock.ownership.etf_holdings`

```rust
let holdings = client
    .stock()
    .ownership()
    .etf_holdings()
    .symbol("0050")
    .from("2026-01-01")
    .sort(HoldingsSort::Desc)
    .send()?;
```

### Spread contracts now work

Symbols are percent-encoded into the URL path. Spread contract symbols carry a
`/` (e.g. `TXFC4/TXFD4`); before 0.8.0 that slash was interpolated raw and
became a path separator, silently sending the request to a different endpoint.
Filter for them with `.is_spread(true)` on `futopt().intraday().tickers()`.

### Expanded futopt quote

`FutOptQuote` gains `market`, `price_limits`, `last_trial`, `trading_halt`,
`is_trial`, `is_delayed_open`, `is_delayed_close`, `is_continuous`, `is_open`,
`is_close` and `serial`. `FutOptTotalStats` goes from 3 fields to 8;
`FutOptLastTrade` gains `bid`, `ask` and `serial`.

`name` and `previous_close` were removed from the official response in 1.5.0
but are **kept here** as `Option`, so payloads still carrying them keep
decoding.

### Expanded streaming frames

`TradesData` and `BooksData` gain `is_trial`; `BooksData` gains `derived_bid`
/ `derived_ask` (the extended 6th book level) plus `data_type` / `exchange`;
`StreamTrade` gains `time`, `serial` and `is_replaced`; `AggregatesData` gains
`is_trial` and `last_trial`.

### Health check: nothing to do

The official SDKs reworked their ping/pong counting into a freshness check and
added a disconnect reason in 1.5.0 — converging on what this SDK has done
since 0.3.0. There is no `max_missed_pongs` here and never was; see the
mapping table in `websocket::health_check`'s module docs if you are porting
config across from the Node or Python SDK.
