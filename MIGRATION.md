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
| `quote(..., oddLot=true)` (Node REST) | ✅ second positional arg `quote(symbol, oddLot?)` |
| WebSocket `subscribe({ channel, symbol })` | ✅ |
| WebSocket `subscribe({ channel, symbols: [...] })` | ✅ batch supported |
| WebSocket `unsubscribe({ id })` / `unsubscribe({ ids: [...] })` | ✅ |
| WebSocket `on('authenticated', cb)` / `on('unauthenticated', cb)` | ✅ restored — old listeners would silently fail without this |
| WebSocket `ping(state?)` | ✅ public method |
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

#### 4. Node REST methods take positional args (not single object param)

The legacy `@fugle/marketdata` Node SDK takes a single object param for every
REST method. This SDK uses positional arguments, matching the napi-rs idiom.

```javascript
// Legacy
const quote = await rest.stock.intraday.quote({ symbol: '2330', type: 'oddlot' });
const candles = await rest.stock.intraday.candles({ symbol: '2330', timeframe: 5 });

// This SDK
const quote = await rest.stock.intraday.quote('2330', true);  // oddLot=true
const candles = await rest.stock.intraday.candles('2330', '5');
```

The `type: 'oddlot'` flag becomes the second positional `oddLot` boolean.

#### 5. Auto-reconnect is opt-in (matches the legacy SDKs)

The legacy SDKs do not have any auto-reconnect — when the WebSocket drops you
get a `disconnect` event and that is it. This SDK ships an auto-reconnect
machinery but **defaults to disabled** so behaviour matches the legacy SDKs.
Enable it explicitly if you want it:

```python
ws = WebSocketClient(api_key="...", reconnect=ReconnectConfig(max_attempts=5))
```
```javascript
const ws = new WebSocketClient({
  apiKey: '...',
  reconnect: { maxAttempts: 5, initialDelayMs: 1000, maxDelayMs: 60000 },
});
```

When `reconnect` is omitted the client behaves exactly like the legacy SDKs:
on `disconnect` you call `connect()` again yourself.

#### 6. Python exception hierarchy is finer-grained

The legacy SDK only raises `FugleAPIError`. This SDK has a base class
`MarketDataError` plus specific subclasses (`ApiError`, `RateLimitError`,
`AuthError`, `ConnectionError`, `TimeoutError`, `WebSocketError`).

`FugleAPIError` is aliased to `MarketDataError` so your existing
`except FugleAPIError:` blocks keep catching everything. New code can opt
into the more specific subclasses for cleaner handling:

```python
try:
    quote = await client.stock.intraday.quote("INVALID")
except RateLimitError as e:
    backoff(e.args[1])         # error code is in args[1]
except ApiError as e:
    log.error(e)
except MarketDataError as e:
    raise
```

#### 7. REST has a default request timeout

The legacy Python SDK calls `requests.get` with **no** timeout, so a stalled
connection hangs forever. This SDK has a default timeout enforced by the
Rust core. If you actually relied on the no-timeout behaviour you will see
new `TimeoutError` exceptions — the fix is to retry at the application
layer.

#### 8. Python `connect()` raises on auth failure (vs legacy's `unauthenticated` event)

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
const { RestClient, WebSocketClient } = require('@fubon/marketdata-js');

const client = new RestClient('your-api-key');
const ws = new WebSocketClient('your-api-key');
```

**After (v0.3.0):**
```typescript
import { RestClient, WebSocketClient } from '@fubon/marketdata-js';

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
import "github.com/fugle/marketdata-sdk-go/marketdata"

client, err := marketdata.NewRestClientWithApiKey("your-api-key")
```

**After (v0.3.0):**
```go
import "github.com/fugle/marketdata-sdk-go/marketdata"

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

### C#

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

### "ConfigError: max_attempts must be >= 1"
**Cause:** Invalid configuration value provided.

**Solution:** Check configuration constraints in [docs/configuration.md](docs/configuration.md). For `ReconnectConfig`:
- `max_attempts`: Must be >= 1
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
