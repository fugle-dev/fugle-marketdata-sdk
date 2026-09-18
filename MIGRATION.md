# Migration Guide

There are two distinct migrations covered in this document:

1. **[Migrating from the legacy `fugle-marketdata` SDKs](#migrating-from-the-legacy-fugle-marketdata-sdks)** —
   you used the original `fugle-marketdata` (PyPI) or `@fugle/marketdata` (npm)
   packages and want to switch to this Rust-core based SDK.
2. **[Migrating from v0.2.x to v0.3.0](#migrating-from-v02x-to-v030)** — you
   already use this SDK and want to upgrade to the v0.3.0 options-object
   constructor API.

If you are coming from the old SDK, read section 1. If you already use this
package, jump to section 2.

## Per-release migration guides (Rust crates)

| Release | Guide | Headline |
|---|---|---|
| 0.9.0 | [MIGRATION-0.9.md](MIGRATION-0.9.md) | responses are passed through verbatim; typed response models leave the return path (**all languages**) |
| 0.8.0 | [MIGRATION-0.8.md](MIGRATION-0.8.md) | `base_url` reverses 0.6.0 (version segment now rejected); futopt streaming defaults to v1.1 with trial frames |
| 0.7.0 | [MIGRATION-0.7.md](MIGRATION-0.7.md) | dual-host REST/WebSocket endpoints |
| 0.6.0 | [MIGRATION-0.6.md](MIGRATION-0.6.md) | `base_url` required the version segment — **superseded by 0.8.0** |
| 0.5.0 | [MIGRATION-0.5.md](MIGRATION-0.5.md) | `SymbolSpec` → `Symbols`, typestate factory |
| 0.4.0 | [MIGRATION-0.4.md](MIGRATION-0.4.md) | reconnect default flip, graceful shutdown |
| 0.3.0 | [MIGRATION-0.3.md](MIGRATION-0.3.md) | sync-default + `tokio-comp` feature |

---

## Migrating from the legacy `fugle-marketdata` SDKs

This SDK aims to be a near drop-in replacement for the original Fugle market
data SDKs:

- **`fugle-marketdata`** (PyPI, currently 2.4.1) — pure-Python implementation
- **`@fugle/marketdata`** (npm, currently 1.4.2) — pure-JS implementation

The biggest change is that this SDK is built on a shared **Rust core** with
PyO3 + napi-rs bindings, so you get a single behaviour across languages and
significantly better runtime performance. The public API has been kept as
close as practical to the legacy SDKs — most call sites compile/run without
modification.

### Drop-in compatible (no changes needed)

The following old-SDK shapes were intentionally restored so you do **not** have
to rewrite call sites:

| Capability | Legacy shape (still works) |
|---|---|
| Top-level imports | `RestClient`, `WebSocketClient`, `HealthCheckConfig` |
| Constructor (Py) | `RestClient(api_key=...)`, `WebSocketClient(api_key=...)` |
| Constructor (Node) | `new RestClient({ apiKey })`, `new WebSocketClient({ apiKey })` |
| Auth methods | `apiKey` / `bearerToken` / `sdkToken` (exactly one required) |
| REST namespaces | `client.stock.{intraday,historical,snapshot,technical,corporateActions}`, `client.futopt.{intraday,historical}` |
| `stock.intraday.tickers(type=...)` | ✅ restored — was missing in early Rust SDK |
| `futopt.intraday.tickers(type=...)` | ✅ restored |
| `futopt.intraday.products(type=...)` | ✅ restored on Python (Node already had it) |
| Node REST object params, e.g. `quote({ symbol: '2330', type: 'oddlot' })` | ✅ every REST method; keys other than `symbol` / `market` are sent verbatim as query params |
| `quote(..., oddLot=true)` (Node REST) | ✅ second positional arg `quote(symbol, oddLot?)` |
| WebSocket `subscribe({ channel, symbol })` | ✅ |
| WebSocket `subscribe({ channel, symbols: [...] })` | ✅ batch supported |
| WebSocket `unsubscribe({ id })` / `unsubscribe({ ids: [...] })` | ✅ |
| WebSocket `on('authenticated', cb)` / `on('unauthenticated', cb)` | ✅ restored, with the server's `data` object as the argument (Node) |
| Node WebSocket listener arguments: `connect()`, `disconnect({ code, reason })` | ✅ plain arguments/objects, no JSON strings to parse — see [§12](#12-node-websocket-events-match-1x) |
| Node `connect()` resolves with the server's `data`; rejected credentials fire `unauthenticated(data)` and reject with that `data` | ✅ |
| WebSocket `ping({ state })` (Node) / `ping(state?)` | ✅ Node sends the object as the frame's `data`; a string still works |
| WebSocket `subscriptions()` (server query) | ✅ — sends `{event:"subscriptions"}`; reply arrives via `message` callback |
| Python `except FugleAPIError:` | ✅ aliased to `MarketDataError` so legacy try/except blocks keep working |
| `HealthCheckConfig.ping_interval` (Py) / `pingInterval` (JS) | ✅ kept the old field name |

### Breaking changes you need to adapt

These are the differences this SDK does **not** paper over. They are limited
on purpose — either because the new behaviour is materially better, or because
hiding them would mask real bugs in legacy code.

#### 1. Python REST methods: sync by default, `_async` siblings available

The legacy `fugle-marketdata` Python SDK is fully synchronous (`requests.get`
under the hood). This SDK matches that default — bare `quote()` calls
return a dict directly, just like the legacy SDK — and additionally exposes
an `_async` sibling for every REST method so asyncio-based applications can
avoid blocking the event loop.

```python
# Legacy fugle-marketdata (still works verbatim)
quote = client.stock.intraday.quote(symbol="2330")

# This SDK — sync default (drop-in replacement)
quote = client.stock.intraday.quote("2330")

# This SDK — async sibling for asyncio apps
quote = await client.stock.intraday.quote_async("2330")
```

The async sibling exists for every REST method:
`quote_async`, `ticker_async`, `candles_async`, `trades_async`,
`volumes_async`, `tickers_async`, `stats_async`, `quotes_async`,
`movers_async`, `actives_async`, `sma_async`, `rsi_async`, `kdj_async`,
`macd_async`, `bb_async`, `capital_changes_async`, `dividends_async`,
`listing_applicants_async`, `products_async`, `daily_async`.

#### 2. Python REST: positional symbol + explicit named params

Legacy Python passes everything as kwargs and forwards extra `**params` to
the query string. This SDK expects the path symbol as the first positional
argument and names every supported query parameter explicitly.

```python
# Legacy
client.stock.historical.candles(symbol="2330", from_="2024-01-01", to="2024-02-01")

# This SDK
await client.stock.historical.candles("2330", from_date="2024-01-01", to_date="2024-02-01")
```

Note that `from_` / `to` were renamed to `from_date` / `to_date`. The legacy
`**params` opaque pass-through is gone — if the API gains a new query
parameter, the binding has to be updated.

#### 2a. Python WebSocket `subscribe` / `unsubscribe` accept dict OR positional

Both call shapes work — pass a dict (legacy SDK style) or use positional /
kwarg arguments (this SDK's original style).

```python
# Legacy SDK style — dict (works verbatim)
ws.stock.subscribe({"channel": "trades", "symbol": "2330"})
ws.stock.subscribe({"channel": "trades", "symbols": ["2330", "2317"]})
ws.stock.subscribe({"channel": "candles", "symbol": "2330", "oddLot": True})

# Positional / kwargs style — also supported
ws.stock.subscribe("trades", "2330")
ws.stock.subscribe("trades", symbols=["2330", "2317"])
ws.stock.subscribe("candles", "2330", odd_lot=True)

# Same dual shape for unsubscribe
ws.stock.unsubscribe({"id": "abc123"})
ws.stock.unsubscribe({"ids": ["abc123", "def456"]})
ws.stock.unsubscribe("abc123")
```

When a dict is supplied, kwargs are ignored — the dict is the single source
of truth. Both `oddLot` (camelCase) and `odd_lot` keys are accepted in dict
form, as are `afterHours` / `after_hours` for futopt.

#### 3. Python WebSocket `message` event delivers a parsed dict

In the legacy Python SDK, the `message` event hands you the raw JSON bytes
and you call `json.loads` yourself. This SDK delivers the **already-parsed
dict** directly.

```python
# Legacy
def on_message(msg):
    payload = json.loads(msg)
    print(payload["event"], payload.get("data"))

# This SDK
def on_message(msg):  # msg is already a dict
    print(msg["event"], msg.get("data"))
```

This is intentionally asymmetric with the JS binding (which still emits raw
strings) — the JS side preserves the legacy `JSON.parse(msg)` pattern, while
the Python side leans into native dict ergonomics. If you have a shared
test/lint that asserts both behave identically, you will need to special-case
the language.

#### 4. Node REST methods also take positional args

The legacy `@fugle/marketdata` Node SDK takes a single object param for every
REST method, and that shape still works unchanged: the path param (`symbol`,
or `market` for `stock.snapshot.*`) goes into the path and every other key is
forwarded verbatim as a query param, so any param in the API docs is
reachable. This SDK additionally accepts positional arguments.

```javascript
// Legacy — still works
const quote = await rest.stock.intraday.quote({ symbol: '2330', type: 'oddlot' });
const trades = await rest.stock.intraday.trades({ symbol: '2330', limit: 5 });

// Positional
const quote = await rest.stock.intraday.quote('2330', true);  // odd lot
const candles = await rest.stock.intraday.candles('2330', '5');
```

The positional form only covers the most common params (for example,
`trades` has no positional `limit`); use the object form for the rest.

Two small differences from the legacy object form, neither of which the API
distinguishes in practice:

- A key set to `null` is dropped, like `undefined`. The legacy SDK sent it as
  a bare key with no value (`?offset`).
- Query params keep the order they appear in the object. The legacy SDK sorted
  them alphabetically.

#### 5. Auto-reconnect is on by default

The legacy SDKs do not have any auto-reconnect — when the WebSocket drops you
get a `disconnect` event and that is it. This SDK **reconnects by default**:
after an unexpected drop it retries with exponential backoff (1 s doubling up
to 60 s) **without an attempt limit**, and subscribes again once it is back.
The server closing with code 1000 or a 4xxx code (e.g. an auth failure) never
triggers a reconnect.

If your code reconnects on its own from a `disconnect` handler, remove that
code or turn auto-reconnect off; otherwise both try to reconnect. Each
attempt emits a `reconnect` event with its number, and `connect()` while one
is in progress fails with code 2011.

```python
# Turn auto-reconnect off (legacy behaviour)
ws = WebSocketClient(api_key="...", reconnect=ReconnectConfig.disabled())

# Or keep it and give up after 10 attempts (error code 3005 once the last fails)
ws = WebSocketClient(api_key="...", reconnect=ReconnectConfig(max_attempts=10))
```

```javascript
// Turn auto-reconnect off (legacy behaviour)
const ws = new WebSocketClient({ apiKey: '...', reconnect: { enabled: false } });

// Or keep it and give up after 10 attempts (error code 3005 once the last fails)
const ws = new WebSocketClient({ apiKey: '...', reconnect: { maxAttempts: 10 } });
```

#### 6. Python exception hierarchy is finer-grained

The legacy SDK only raises `FugleAPIError`. This SDK has a base class
`MarketDataError` plus specific subclasses (`ApiError`, `RateLimitError`,
`AuthError`, `ConnectionError`, `TimeoutError`, `WebSocketError`).

`FugleAPIError` is aliased to `MarketDataError` so your existing
`except FugleAPIError:` blocks keep catching everything. New code can opt
into the more specific subclasses for cleaner handling. Every exception has
`code`, `source_kind`, `message`, `status`, `body`, `request_id` and
`headers` ([error reference](docs/errors.md)); the legacy `status_code` and
`response_text` are kept as aliases of `status` and `body` and are now filled
in for HTTP errors:

```python
try:
    quote = await client.stock.intraday.quote("INVALID")
except RateLimitError as e:
    backoff(e.headers.get("retry-after"))
except ApiError as e:
    log.error("%s %s %s", e.code, e.status, e.body)
except MarketDataError as e:
    raise
```

#### 7. REST has a default request timeout

The legacy Python SDK calls `requests.get` with **no** timeout, so a stalled
connection hangs forever. This SDK has a default timeout enforced by the
Rust core. If you actually relied on the no-timeout behaviour you will see
new `TimeoutError` exceptions — the fix is to retry at the application
layer.

#### 8. Node REST rejects on HTTP errors instead of resolving the error body

The legacy Node SDK resolves whatever JSON the server returns, including a
4xx/5xx error body such as
`{ "statusCode": 404, "message": "Resource Not Found" }`. This SDK
**rejects** instead, with an `Error` that carries the error code, the HTTP
status and the server's body as properties:

- `err.code === 2003`, `err.status === 404`,
  `err.message === 'API error (status 404): {"message":"Resource Not Found",...}'`
- `err.code === 2002`, `err.message === 'Authentication error: {...}'` for 401 / 403

`err.body` is the body as sent and `err.headers` the response headers; see
[the error reference](docs/errors.md) for every field.

Code that checks `statusCode` inside `.then()` must move that handling into
`.catch()` / `try`, otherwise the rejection goes unhandled.

```javascript
// Legacy
const res = await rest.stock.intraday.quote({ symbol: 'NOPE' });
if (res.statusCode) { /* handle error */ }

// This SDK
try {
  const quote = await rest.stock.intraday.quote({ symbol: 'NOPE' });
} catch (err) {
  // err.code: 2003, err.status: 404, err.body: '{"statusCode":404,...}'
}
```

#### 9. Python `connect()` raises on auth failure (vs legacy's `unauthenticated` event)

Both this SDK and the legacy SDK block in `connect()` until the server has
either accepted or rejected authentication. The difference is **how a
rejection is reported**:

- **Legacy**: `connect()` returns normally; you find out about rejection by
  listening for the `unauthenticated` event.
- **This SDK**: `connect()` **raises an exception** (`AuthError` or
  `MarketDataError`) when the server rejects credentials. The
  `unauthenticated` event still fires for callers that want to listen for
  it, but you should also wrap the `connect()` call in `try/except`.

```python
# Recommended in this SDK
try:
    ws.stock.connect()
except AuthError as e:
    log.error("auth failed: %s", e)
    return
ws.stock.subscribe(channel="trades", symbol="2330")
```

#### 10. FutOpt historical: `session` replaces `afterhours`, and the path is a product

`futopt/historical/daily` and `futopt/historical/candles` changed on the
server side (fugle-realtime #727), and this SDK follows the new contract:

- The path takes a **product** code (`TXF`), not a contract (`TXFC4` is a
  404). For `candles`, pick the contract with `contractMonth`: `YYYYMM`, or
  `1!` / `2!` / `3!` for the front / next / third month (the server defaults
  to `1!`).
- The after-hours session is `session: 'afterhours'`. The legacy
  `afterhours: true` is no longer honoured by the server.
- `daily` returns one trading day (`date`, default today) with a row per
  contract month.

```javascript
// Legacy
await rest.futopt.historical.daily({ symbol: 'TXF', date: '2026-09-15', afterhours: true });

// This SDK — object form (`product` or `symbol`)
await rest.futopt.historical.daily({ product: 'TXF', date: '2026-09-15', session: 'afterhours' });
// Positional
await rest.futopt.historical.daily('TXF', '2026-09-15', true);
await rest.futopt.historical.candles('TXF', '2026-09-01', '2026-09-15', 'D', false, '202609');
```

```python
client.futopt.historical.daily("TXF", date="2026-09-15", after_hours=True)
client.futopt.historical.candles("TXF", contract_month="1!", timeframe="D")
```

#### 11. Node WebSocket `connect()` rejects while already connected

The legacy Node SDK let you call `connect()` on a connected client: it opened
another socket, left the old one open, and registered its listeners again.
This SDK rejects that call with an error whose `code` is `2011`
(`Already connected; call disconnect() first`) and keeps the
existing connection. To reconnect, `disconnect()` first — calling `connect()`
straight after `disconnect()`, or from a `disconnect` handler when
auto-reconnect is off (`reconnect: { enabled: false }`; it is on by default,
see §5), is fine.

```javascript
// with reconnect: { enabled: false }
ws.stock.on('disconnect', () => {
  ws.stock.connect().catch(console.error);
});
```

#### 12. Node WebSocket events match 1.x

Listener arguments and `connect()` settlement follow `@fugle/marketdata` 1.x,
so 1.x listeners work unchanged:

| Event / call | Arguments |
|---|---|
| `connect` | none — fires when the socket opens, before authentication |
| `authenticated` | the server's `data` object |
| `unauthenticated` | the server's `data` object (fires after `connect`) |
| `connect()` | resolves with the `authenticated` `data`; rejects with the `unauthenticated` `data` object |
| `disconnect` | `{ code, reason }` (`code` is `null` when the connection ended without one) |
| `ping(params)` | `params` sent as the frame's `data` |

Two differences remain from 1.x's `error` event:

- **The `error` argument is an `Error` with a numeric `code`.** Its `message`
  is the plain description, without a `[code]` prefix — read the code from
  `err.code` (3005 for "Reconnection failed after N attempts"). 1.x passed
  the socket's native error. A `connect()` that fails for a reason other than
  rejected credentials rejects with an `Error` carrying the same fields
  (`err.code`, no `[code]` prefix).
- **No `error` listener means errors are ignored.** 1.x's EventEmitter threw
  an unhandled `'error'` event and could crash the process; this SDK never
  does.
- **A listener that throws does not crash the process.** 1.x's EventEmitter
  let the exception propagate as an uncaught exception. This SDK reports it
  through `error` with code 3004 (`err.event`, `err.count`, `err.cause`), or
  prints it with `console.error` when there is no `error` listener, and keeps
  delivering later events. To stop on a listener failure, do so from the
  `error` listener, e.g. `if (err.code === 3004) process.exit(1)`.

This SDK also has a `reconnect` event (1.x had no auto-reconnect), receiving
`{ attempt }`.

```javascript
ws.stock.on('authenticated', (data) => console.log(data.message));
ws.stock.on('disconnect', ({ code, reason }) => console.log(code, reason));
ws.stock.on('error', (err) => console.error(err.code, err.message));

try {
  const data = await ws.stock.connect();
} catch (e) {
  // rejected credentials: e is the server's data, e.g. { message: '...' }
}
```

#### 13. Python WebSocket callbacks: errors and async callbacks

Three differences in how Python callbacks are handled:

- **"Reconnection failed after N attempts" has code 3005.** The `error`
  callback's `WebSocketError` for it had code -1, the code of a panicked
  worker thread. It now has `e.code == 3005` (`RECONNECT_FAILED`,
  `e.source_kind == "network"`); code that checked for -1 should check 3005.
- **`on()` refuses `async def` callbacks.** Callbacks run on the SDK's thread,
  where nothing would await a coroutine, so registering an `async def`
  function raises `TypeError`. In asyncio code, consume messages with
  `async for msg in ws.stock.messages()` instead.
- **A callback that returns a coroutine is reported, not left pending.** For
  example `ws.stock.on("message", lambda msg: handle(msg))` with an
  `async def handle`: the coroutine is closed (no "coroutine was never
  awaited" warning) and reported through `error` with code 3004
  (`CALLBACK_FAILED`), `e.event`, `e.count` and a `TypeError` as
  `e.__cause__` — the same report as a callback that raises. Without an
  `error` callback it goes to `sys.unraisablehook`.

```python
# Legacy style that no longer registers
async def on_message(msg): ...
ws.stock.on("message", on_message)        # TypeError

# This SDK
def on_error(err):
    if err.code == 3005:
        log.error("gave up reconnecting: %s", err.message)
    elif err.code == 3004:
        log.error("%s callback failed %dx: %r", err.event, err.count, err.__cause__)

ws.stock.on("error", on_error)
```

#### 14. WebSocket `subscribe()` throws for an unknown channel

Node's `subscribe()` checks the channel name when called, whether or not the
client is connected. A name that is not a channel of that product (`trades`,
`candles`, `books`, `aggregates`, plus `indices` for stock), such as `'trade'`,
throws an `Error` with `code` `1005` (`INVALID_PARAMETER`) and nothing is
sent. Earlier releases of this SDK sent nothing and reported nothing. Names
are matched ignoring case.

```javascript
try {
  ws.stock.subscribe({ channel: 'trade', symbol: '2330' });
} catch (err) {
  // err.code === 1005
  // err.message === "Invalid parameter 'channel': unknown channel 'trade'. Valid channels: trades, candles, books, aggregates, indices"
}
```

Python raises `MarketDataError` with `code` `1005` and the same message,
whether or not the client is connected; `subscribe_async()` raises it when
awaited.

```python
try:
    ws.stock.subscribe({"channel": "trade", "symbol": "2330"})
except MarketDataError as e:
    # e.code == 1005
    ...
```

### New things the legacy SDKs did not have

These are additive and do not break anything; you can ignore them if you
just want a drop-in replacement.

- **`indices` channel** on stock WebSocket — receive index ticks alongside
  trades / books / candles / aggregates.
- **`with_full_config`** core constructor — fully tunable reconnect +
  health-check config from a single options object.
- **Per-binding async runtime integration** — Python uses
  `pyo3-async-runtimes`, JS uses napi-rs Promises and the tokio runtime.

### Per-language quickstart

#### Python

```python
import asyncio
from fugle_marketdata import RestClient, WebSocketClient

async def main():
    client = RestClient(api_key="your-api-key")
    quote = await client.stock.intraday.quote("2330")
    print(quote["lastPrice"])

asyncio.run(main())
```

```python
from fugle_marketdata import WebSocketClient

ws = WebSocketClient(api_key="your-api-key")

ws.stock.on("authenticated", lambda: print("auth ok"))
ws.stock.on("message", lambda msg: print(msg["event"], msg.get("data")))

ws.stock.connect()
ws.stock.subscribe(channel="trades", symbols=["2330", "2317"])
```

#### Node.js

```javascript
const { RestClient, WebSocketClient } = require('marketdata-js');

const rest = new RestClient({ apiKey: 'your-api-key' });
const quote = await rest.stock.intraday.quote('2330');
console.log(quote.lastPrice);

const ws = new WebSocketClient({ apiKey: 'your-api-key' });
ws.stock.on('authenticated', () => console.log('auth ok'));
ws.stock.on('message', (raw) => {
  const msg = JSON.parse(raw);
  console.log(msg.event, msg.data);
});
ws.stock.connect();
ws.stock.subscribe({ channel: 'trades', symbols: ['2330', '2317'] });
```

---

## Migrating from v0.2.x to v0.3.0

This guide helps you upgrade from v0.2.x to v0.3.0. The major change is that constructors now use options objects instead of positional string arguments, and WebSocket configuration options are now exposed.

For a complete list of changes, see [CHANGELOG.md](CHANGELOG.md).

**Backward Compatibility Note:**

- **Python and Node.js**: String constructors still work but emit deprecation warnings. They will be removed in v0.4.0.
- **Java, Go, C#**: Constructors changed immediately (no deprecated path).

## Breaking Changes Summary

| Change | Languages | Impact |
|--------|-----------|--------|
| Constructor API | All | Options object/kwargs instead of string |
| Health check default | All | `false` (was `true`) |
| Auth validation | All | Exactly one auth method required |

## Language-Specific Migration Guides

### Python

**Before (v0.2.x):**

```python
from fugle_marketdata import RestClient, WebSocketClient

client = RestClient("your-api-key")
client2 = RestClient.with_bearer_token("your-token")
ws = WebSocketClient("your-api-key")
```

**After (v0.3.0):**

```python
from fugle_marketdata import RestClient, WebSocketClient, ReconnectConfig, HealthCheckConfig

client = RestClient(api_key="your-api-key")
client2 = RestClient(bearer_token="your-token")
ws = WebSocketClient(
    api_key="your-api-key",
    reconnect=ReconnectConfig(max_attempts=10),
    health_check=HealthCheckConfig(enabled=True),
)
```

**Migration Steps:**

1. Replace `RestClient("key")` with `RestClient(api_key="key")`
2. Replace `RestClient.with_bearer_token("token")` with `RestClient(bearer_token="token")`
3. Replace `RestClient.with_sdk_token("token")` with `RestClient(sdk_token="token")`
4. Replace `WebSocketClient("key")` with `WebSocketClient(api_key="key")`
5. (Optional) Add `reconnect` and `health_check` config if needed

**Automated Migration:**

```bash
# Transform positional arguments to keyword arguments
python migration/migrate-python.py --path src/

# Preview changes without modifying files
python migration/migrate-python.py --path src/ --dry-run
```

---

### Node.js / TypeScript

**Before (v0.2.x):**

```javascript
const { RestClient, WebSocketClient } = require('@fugle/marketdata');

const client = new RestClient('your-api-key');
const ws = new WebSocketClient('your-api-key');
```

**After (v0.3.0):**

```typescript
import { RestClient, WebSocketClient } from '@fugle/marketdata';

const client = new RestClient({ apiKey: 'your-api-key' });
const ws = new WebSocketClient({
  apiKey: 'your-api-key',
  reconnect: { maxAttempts: 10, initialDelayMs: 2000 },
  healthCheck: { enabled: true, pingInterval: 15000 },
});
```

**Migration Steps:**

1. Replace `new RestClient('key')` with `new RestClient({ apiKey: 'key' })`
2. Replace `new WebSocketClient('key')` with `new WebSocketClient({ apiKey: 'key' })`
3. (Optional) Add `reconnect` and `healthCheck` config objects

**Automated Migration:**

```bash
# Transform string constructors to object constructors
npx jscodeshift -t migration/migrate-javascript.js src/

# Preview changes without modifying files
npx jscodeshift -t migration/migrate-javascript.js src/ --dry
```

---

### Java

**Before (v0.2.x):**

```java
import tw.com.fugle.marketdata.*;

RestClient client = new RestClient("your-api-key");
```

**After (v0.3.0):**

```java
import tw.com.fugle.marketdata.*;

FugleRestClient client = FugleRestClient.builder()
    .apiKey("your-api-key")
    .reconnectOptions(new ReconnectOptions.Builder()
        .maxAttempts(10)
        .initialDelayMs(2000L)
        .build())
    .build();
```

**Migration Steps:**

1. Replace `new RestClient(...)` with `FugleRestClient.builder()...build()`
2. Use `.apiKey()`, `.bearerToken()`, or `.sdkToken()` methods
3. (Optional) Add `.reconnectOptions()` and `.healthCheckOptions()`

---

### Go

**Before (v0.2.x):**

```go
import marketdata "github.com/fugle-dev/fugle-marketdata-go"

client, err := marketdata.NewRestClientWithApiKey("your-api-key")
```

**After (v0.3.0):**

```go
import marketdata "github.com/fugle-dev/fugle-marketdata-go"

client, err := marketdata.NewFugleRestClient(
    marketdata.WithApiKey("your-api-key"),
    marketdata.WithReconnectOptions(marketdata.ReconnectOptions{
        MaxAttempts:    10,
        InitialDelayMs: 2000,
    }),
)
```

**Migration Steps:**

1. Replace `NewRestClientWithApiKey(key)` with `NewFugleRestClient(WithApiKey(key))`
2. Replace `NewRestClientWithBearerToken(token)` with `NewFugleRestClient(WithBearerToken(token))`
3. (Optional) Add `WithReconnectOptions()` and `WithHealthCheckOptions()` functional options

---

### C\#

**Before (v0.2.x):**

```csharp
using MarketdataUniffi;

var client = new RestClient("your-api-key");
```

**After (v0.3.0):**

```csharp
using MarketdataUniffi;

var client = new RestClient(new RestClientOptions {
    ApiKey = "your-api-key"
});

// String constructor still supported for now (deprecated)
var client2 = new RestClient("your-api-key");
```

**Migration Steps:**

1. Replace `new RestClient("key")` with `new RestClient(new RestClientOptions { ApiKey = "key" })`
2. (Optional) Add `ReconnectOptions` and `HealthCheckOptions` properties

---

## Common Issues

### "ValueError: Provide exactly one of: apiKey, bearerToken, sdkToken"

**Cause:** You provided zero or multiple authentication methods.

**Solution:** Pass exactly one of `api_key`, `bearer_token`, or `sdk_token`:

```python
# ✓ Correct
client = RestClient(api_key="key")

# ✗ Wrong - no auth provided
client = RestClient()

# ✗ Wrong - multiple auth provided
client = RestClient(api_key="key", bearer_token="token")
```

---

### "ConfigError: initial_delay must be >= 100ms"

**Cause:** Invalid configuration value provided.

**Solution:** Check configuration constraints in [docs/configuration.md](docs/configuration.md). For `ReconnectConfig`:

- `max_attempts`: any value; 0 means unlimited (the default)
- `initial_delay_ms`: Must be >= 100ms
- `max_delay_ms`: Must be >= `initial_delay_ms`

---

### Health Check Not Running

**Cause:** Default changed from `enabled: true` to `enabled: false` in v0.3.0.

**Solution:** Explicitly enable health checks if needed:

```python
ws = WebSocketClient(
    api_key="key",
    health_check=HealthCheckConfig(enabled=True)
)
```

---

## Automated Migration Tools

### Python Migration Script

```bash
# Transform all Python files in directory
python migration/migrate-python.py --path src/

# Dry run (show changes without writing)
python migration/migrate-python.py --path src/ --dry-run

# Process single file
python migration/migrate-python.py --path src/client.py
```

### JavaScript Migration Script

```bash
# Transform all JavaScript/TypeScript files
npx jscodeshift -t migration/migrate-javascript.js src/

# Dry run (show changes without writing)
npx jscodeshift -t migration/migrate-javascript.js src/ --dry

# Process specific extensions only
npx jscodeshift -t migration/migrate-javascript.js src/ --extensions=ts,tsx
```

### Post-Migration Validation

```bash
# Validate migration completed successfully
./migration/validate-migration.sh
```

Both tools support `--dry-run` for previewing changes before applying them.

---

## Getting Help

If you encounter migration issues:

1. Check [docs/configuration.md](docs/configuration.md) for configuration reference
2. Review code examples in the `examples/` directory
3. File an issue on GitHub with your migration error
