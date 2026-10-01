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

- **`fugle-marketdata`** (PyPI, last release 2.7.0) — pure-Python implementation
- **`@fugle/marketdata`** (npm, last release 1.7.0) — pure-JS implementation

The biggest change is that this SDK is built on a shared **Rust core** with
PyO3 + napi-rs bindings, so you get a single behaviour across languages and
significantly better runtime performance. The public API has been kept as
close as practical to the legacy SDKs — most call sites compile/run without
modification.

### The five changes most upgrades run into

| | Python 2.x → 3.0 | Node 1.x → 3.0 |
|---|---|---|
| 1. Auto-reconnect is on | Your own `disconnect()` + `connect()` recovery code loops with it; keep one ([§5](#5-auto-reconnect-is-on-by-default)) | Same; a `disconnect` handler that only calls `connect()` keeps working ([§5](#5-auto-reconnect-is-on-by-default), [§11](#11-node-websocket-connect-rejects-while-already-connected-and-waits-during-a-reconnect)) |
| 2. Errors look different | `str(e)` is just the message; a missing credential raises `ConfigError`, not `TypeError` ([§6](#6-python-exception-hierarchy-is-finer-grained), [§20](#20-python-smaller-differences-from-2x)) | REST rejects on HTTP 4xx/5xx instead of resolving the error body ([§8](#8-node-rest-rejects-on-http-errors-instead-of-resolving-the-error-body)) |
| 3. Messages | The `message` callback gets a `dict`; `raw_message` gets the `str` ([§3](#3-python-websocket-message-event-delivers-a-parsed-dict)) | Unchanged strings, but a slow listener now loses messages past 4096 unread ([defaults](#defaults-that-changed)) |
| 4. Slow callbacks drop data | Callbacks share one thread; one that blocks makes the queue overflow ([§15](#15-python-websocket-callbacks-run-one-at-a-time-on-one-thread)) | See row 3: `messageOverflow: 'unbounded'` restores 1.x's never-drop |
| 5. Health check | `HealthCheckConfig` keeps its name, not its fields; detection is on by default ([drop-in table](#drop-in-compatible-no-changes-needed)) | `healthCheck.pingInterval` / `maxMissedPongs` are ignored with a warning; detection is on by default |

Every other difference is below: the drop-in table lists what keeps working,
the numbered sections what does not, and
[Defaults that changed](#defaults-that-changed) what behaves differently
without any code change.

### Supported platforms

The legacy SDKs are pure Python / pure JS and install anywhere; this SDK ships
native code for Linux (glibc and musl) x86_64/aarch64, macOS x86_64/arm64 and
Windows x64 only, and requires Python 3.8+ / Node.js 18+. On other platforms
(armv7, Windows arm64, 32-bit Windows, Python 3.7) pip keeps installing 2.x,
while npm installs 3.x and fails at `require`. Pin `fugle-marketdata<3` or
`@fugle/marketdata@<3` there; see
[Unsupported platforms](docs/INSTALL.md#unsupported-platforms).

1.x also ran outside Node (it used `isomorphic-fetch` and `isomorphic-ws`).
3.x is a native Node module: it does not run in a browser, in a front-end
bundle (webpack, Vite), in an Electron renderer process, or in Deno or Bun
(not tested). Keep `@fugle/marketdata@<3` there.

2.x installed `requests`, `websocket-client`, `pyee` and `orjson` with it;
3.x depends on nothing. Code that imports one of them without declaring it
fails in a fresh environment: add it to your own requirements.

### Drop-in compatible (no changes needed)

The following old-SDK shapes were intentionally restored so you do **not** have
to rewrite call sites:

| Capability | Legacy shape (still works) |
|---|---|
| Top-level imports | `RestClient`, `WebSocketClient`, `HealthCheckConfig` |
| Constructor (Py) | `RestClient(api_key=...)`, `WebSocketClient(api_key=...)` |
| Constructor (Node) | `new RestClient({ apiKey })`, `new WebSocketClient({ apiKey })` |
| Auth methods | `apiKey` / `bearerToken` / `sdkToken` (exactly one required) |
| REST namespaces | `client.stock.{intraday,historical,snapshot,technical,ownership}` plus `corporate_actions` (Python) / `corporateActions` (Node), `client.futopt.{intraday,historical}` |
| `stock.intraday.tickers(type=...)` | ✅ restored — was missing in early Rust SDK |
| `futopt.intraday.tickers(type=...)` | ✅ restored |
| `futopt.intraday.products(type=...)` | ✅ restored on Python (Node already had it) |
| Node REST object params, e.g. `quote({ symbol: '2330', type: 'oddlot' })` | ✅ every REST method; keys other than `symbol` / `market` are checked against the endpoint's parameter list and sent under the API's names — an unknown key rejects (see [§4](#4-node-rest-methods-also-take-positional-args)) |
| Python REST kwargs in the API's spelling, e.g. `trades(symbol="2330", limit=5, isTrial=True)`, `ticker(symbol="2330", type="oddlot")`, `sma(..., from_="2024-01-01", to="2024-02-01")` | ✅ every REST method; an unknown keyword raises `TypeError` (see [§2](#2-python-rest-keywords-are-checked-and-the-legacy-spellings-work)) |
| `quote(..., oddLot=true)` (Node REST) | ✅ second positional arg `quote(symbol, oddLot?)` |
| WebSocket `subscribe({ channel, symbol })` | ✅ |
| WebSocket `subscribe({ channel, symbols: [...] })` | ✅ batch supported |
| WebSocket `unsubscribe({ id })` / `unsubscribe({ ids: [...] })` | ✅ |
| WebSocket `on('authenticated', cb)` / `on('unauthenticated', cb)` | ✅ restored, each with its legacy argument: the server's `data` object (Node, as 1.x); the whole frame as a dict, `{"event": "authenticated", "data": {...}}` / `{"event": "error", "code": 1000, "data": {...}}` (Python, as 2.x) |
| Node WebSocket listener arguments: `connect()`, `disconnect({ code, reason })` (3.0 adds `intent` and `willReconnect`) | ✅ plain arguments/objects, no JSON strings to parse — see [§12](#12-node-websocket-events-match-1x) |
| Node `connect()` resolves with the server's `data`; rejected credentials fire `unauthenticated(data)` and reject with that `data` | ✅ |
| WebSocket `ping({ state })` (Node) / `ping(state?)` | ✅ Node sends the object as the frame's `data`; a string still works. Python takes a `str` only: 2.x sent any JSON value, 3.0 raises `TypeError` for a dict or number |
| Node `client.stock.baseUrl` / `client.futopt.baseUrl` | ✅ the request prefix with the product segment, e.g. `https://api.fugle.tw/marketdata/v1.0/stock`, as in 1.x; `client.baseUrl` (new) is the prefix without it |
| Python `ws.stock` / `ws.futopt` read again for each call (`ws.stock.on(...)`, `ws.stock.connect()`, `ws.stock.subscribe(...)`) | ✅ each property returns the same client every time, as 2.x's factory did (see [§19](#19-the-legacy-clients-plumbing-is-gone-get_client-request-options)) |
| Python `rest.stock.base_url` / `rest.futopt.base_url` | ✅ includes the product segment, e.g. `https://api.fugle.tw/marketdata/v1.0/stock`, as in 2.x; `rest.base_url` is the value without it |
| WebSocket `ws.stock.url` / `ws.futopt.url` | ✅ the resolved endpoint, e.g. `wss://api.fugle.tw/marketdata/v1.1/futopt/streaming`; reflects `base_url` / `baseUrl` and `version`, readable before `connect()` |
| Node `on()` chaining: `ws.stock.on('message', cb).subscribe({ ... })` | ✅ `on()` returns the client it was called on |
| Node listeners as on the 1.x `EventEmitter`: several per event, `once`, `off` / `removeListener`, `removeAllListeners`, `listenerCount`, `addListener` | ✅ every listener of an event is called, in registration order, with `this` set to the client — see [§12](#12-node-websocket-events-match-1x) |
| Node `ws.stock.connect().then(...)` with no `.catch`, as in the 1.x README | ✅ with an `error` listener, a failed connection does not end the process — see [§12](#12-node-websocket-events-match-1x) |
| WebSocket `subscriptions()` (server query) | ✅ — sends `{event:"subscriptions"}`; reply arrives via `message` callback |
| Python `except FugleAPIError:` | ✅ aliased to `MarketDataError` so legacy try/except blocks keep working; `str(e)` is the message, in a different format from 2.x (see [§6](#6-python-exception-hierarchy-is-finer-grained)) |
| Python `ws.stock.off(event, listener)` | ✅ removes every registration `==` to `listener` (one not registered is ignored); `off(event)` removes every callback for the event. 2.x documented this call but it raised `AttributeError` |
| Python `ws.stock.on(event, f)` twice with the same `f` | ✅ registered once, as in 2.x: `f` runs once per event |
| Python WebSocket `error` callback `on_error(err)` | ✅ one argument as in 2.x, now a `WebSocketError` (2.x passed the websocket-client exception); `str(err)` is its message |
| `HealthCheckConfig` (Py) / `healthCheck` (JS) | ⚠️ the class/option is kept, the old fields are not: `ping_interval` / `pingInterval` and `max_missed_pongs` / `maxMissedPongs` do not exist (both ignore them and warn: Node with a process warning once per process, Python with a `FugleHealthCheckWarning` — a `UserWarning`, shown once per calling line under the default warning filters; both carry code `FUGLE_HEALTH_CHECK_LEGACY_OPTIONS`. Python also keeps 2.x's positional order, `HealthCheckConfig(enabled, ping_interval, max_missed_pongs)`). Detection is on by default (35 s); to have the SDK ping a silent connection, use `probe_enabled` + `idle_probe_after_ms` (Py) / `probeEnabled` + `idleProbeAfterMs` (JS) — see [configuration](docs/configuration.md#healthcheckconfig--healthcheckoptions) |

### Defaults that changed

Code that passes none of these options gets different behaviour:

| Setting | Legacy | 3.0 default | To get the legacy behaviour |
|---|---|---|---|
| Auto-reconnect | none | on, no attempt limit, 1 s doubling to 60 s ([§5](#5-auto-reconnect-is-on-by-default)) | `ReconnectConfig.disabled()` / `reconnect: { enabled: false }` |
| Health check | off (2.x / 1.x pinged only when enabled) | on: no inbound frame for 35 s closes the connection, then auto-reconnect | `HealthCheckConfig(enabled=False)` / `healthCheck: { enabled: false }` |
| Unread message queue | none, nothing dropped (2.x ran callbacks on the socket's reader thread; 1.x: plain `EventEmitter`) | `drop_newest` / `'dropNewest'`: past 4096 unread messages new ones are dropped and reported with `messages_dropped` / `messagesDropped` | `message_overflow="unbounded"` / `messageOverflow: 'unbounded'` |
| Auth timeout | Python 5 s (`Exception('authentication timeout')`); Node none (`connect()` could stay pending) | 10 s, then `TimeoutError` / an `Error` with code 3001 | Python `auth_timeout_ms=5000`; Node cannot turn it off, raise `authTimeoutMs` |
| REST request timeout | none | 30 s ([§7](#7-rest-has-a-default-request-timeout)) | — |

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

Every REST method `name` has an `name_async` sibling taking the same
arguments (`quote_async`, `etf_holdings_async`, `daily_async`, ...).

#### 2. Python REST: keywords are checked, and the legacy spellings work

Legacy Python passes everything as kwargs and forwards extra `**params` to
the query string verbatim. This SDK names every query parameter of every
endpoint (the list comes from the server's request definitions) and accepts
each one under three spellings: the API's own name as the legacy SDK and
developer.fugle.tw use it (`isTrial`, `contractType`, `from` / `to`,
`type="oddlot"`, `session="afterhours"`), the legacy `from_` alias for the
reserved word, and this SDK's snake_case keyword (`is_trial`, `from_date`,
`odd_lot=True`). The path symbol may be positional or `symbol=`.

```python
# Legacy — still works
client.stock.historical.candles(symbol="2330", from_="2024-01-01", to="2024-02-01")
client.stock.intraday.trades(symbol="2330", limit=5, isTrial=True)
client.stock.intraday.ticker(symbol="2330", type="oddlot")

# This SDK's spelling
client.stock.historical.candles("2330", from_date="2024-01-01", to_date="2024-02-01")
client.stock.intraday.trades("2330", limit=5, is_trial=True)
client.stock.intraday.ticker("2330", odd_lot=True)
```

Two differences from the legacy pass-through, both on purpose:

- A keyword the endpoint does not take raises `TypeError` naming the
  accepted ones, instead of being sent (and, on most endpoints, ignored by
  the server). A typo in a legacy call site that used to return unfiltered
  data now fails at the call.
- One parameter given under two spellings (`from_` and `from_date`) raises
  `TypeError` rather than one being picked.

Type checkers (mypy, pyright) only know the snake_case keywords; the other
spellings are a runtime compatibility layer.

Values are checked by type too, where 2.x sent whatever it got through
`urlencode`: `timeframe` must be a `str` (`timeframe=5` raises `TypeError`;
write `"5"`) and `limit` / `offset` an `int` (`limit="5"` raises).

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

The dict is the whole call, as in the legacy `subscribe(params)`: passing
`symbol` / `symbols` / `odd_lot` (futopt `after_hours`) next to it, or `ids=`
next to an unsubscribe dict, raises `TypeError` instead of being ignored.
The dict takes `channel`, `symbol` / `symbols` and the session flag, spelled
`oddLot`, `odd_lot` or `intradayOddLot` (the server's and Node's name) for
stock and `afterHours` / `after_hours` for futopt; give one spelling. Any
other key — the other product's flag included, such as `afterHours` on the
stock client — or a flag that is not a boolean raises `TypeError` naming the
accepted keys; legacy code that passed such a key got data it did not ask
for. A key set to `None` counts as not given. An unsubscribe dict without
`channel` takes only `id` / `ids`.

#### 3. Python WebSocket `message` event delivers a parsed dict

In the legacy Python SDK, the `message` event hands you the raw JSON `str`
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

To keep the legacy pattern, or to skip the dict altogether, register
`raw_message` instead: it delivers each message as the `str` the server
sent, and `json.loads` of it equals the dict `message` gets.

```python
def on_message(msg):  # legacy handler, unchanged
    payload = json.loads(msg)
    print(payload["event"], payload.get("data"))

ws.stock.on("raw_message", on_message)
```

With only `raw_message` callbacks the SDK builds no dict. For iteration,
`ws.stock.messages(raw=True)` yields the same strings (`for` and
`async for`). `message` itself is unchanged and keeps delivering dicts.

This is intentionally asymmetric with the JS binding (which still emits raw
strings) — the JS side preserves the legacy `JSON.parse(msg)` pattern, while
the Python side leans into native dict ergonomics. If you have a shared
test/lint that asserts both behave identically, you will need to special-case
the language.

#### 4. Node REST methods also take positional args

The legacy `@fugle/marketdata` Node SDK takes a single object param for every
REST method, and that shape still works: the path param (`symbol`, or
`market` for `stock.snapshot.*`) goes into the path and every other key is
sent as a query param under the API's name, so any param in the API docs is
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

Differences from the legacy object form:

- **A key the endpoint does not take rejects** (`code: 1005`) with the
  accepted keys in the message, instead of being forwarded. The legacy SDK
  forwarded everything; on most endpoints the server ignored an unknown key,
  so a typo returned unfiltered data. (On `capitalChanges` /
  `listingApplicants` the server already answered 400.) The `Rest*Params`
  TypeScript types list exactly the accepted keys, so a typo is also a
  compile error.
- The snake_case spellings (`is_trial`, `odd_lot`) are accepted as aliases of
  the API names; one parameter under two spellings rejects.
- A key set to `null` is dropped, like `undefined`. The legacy SDK sent it as
  a bare key with no value (`?offset`).
- Query params are not sent in any particular order (the legacy SDK sorted
  them alphabetically). The server does not depend on the order.

#### 5. Auto-reconnect is on by default

The legacy SDKs do not have any auto-reconnect — when the WebSocket drops you
get a `disconnect` event and that is it. This SDK **reconnects by default**:
after an unexpected drop it retries with exponential backoff (1 s doubling up
to 60 s, each wait minus 0–50% random jitter) **without an attempt limit**, and subscribes again once it is back.
Each attempt emits a `reconnect` event with its number. Only a normal closure
by the server (code 1000) and rejected credentials end the connection for
good; every other close reconnects (#201). See
[Giving up](docs/configuration.md#reconnectconfig--reconnectoptions) in the
configuration reference for the full list.

If your code reconnects on its own from a `disconnect` handler — calling
`connect()` and then subscribing again — it keeps working (#230). A
`connect()` made while an automatic reconnect is in progress opens no
connection of its own: it waits for that reconnect and resolves once the
connection is back and the stored subscriptions have been re-sent. It fails
if the reconnect does not come back: code 2010 when `disconnect()` is
called, code 3005 when the attempts run out, and, when the credentials are
rejected, the server's `data` object (Node) or `AuthError` (Python). The
`subscribe()` calls that follow repeat subscriptions the SDK has already
re-sent, which is harmless: the SDK and the server both keep one entry per
channel and symbol, so nothing is delivered twice and no extra subscription
quota is used. The one visible effect is when the quota is exactly full: the
server answers the repeated subscribe with an `error` message of code 1001,
and the subscription stays active. (1.x code that subscribes in an
`authenticated` handler already repeats them the same way on every
reconnect.)

**Pick one: your own reconnect code, or auto-reconnect — not both.** The
common 2.x recovery code *closes* the connection before opening a new one —
a `disconnect` handler that sets a flag, and a background thread that then
calls `disconnect()` and `connect()`. With auto-reconnect on, that turns
into an endless reconnect loop (#226):

1. The connection drops; the SDK emits `disconnect`, your flag is set, and
   the SDK reconnects within a second or two.
2. Your thread wakes up later and calls `disconnect()` on the connection
   the SDK has just restored. Closing it emits `disconnect` again (a close
   you ask for emits `disconnect`, as it did in 2.x), which sets your flag
   again, and `connect()` opens a new connection.
3. Repeat forever: each round is a full login — 18 in 40 seconds in the
   report on #226. Messages keep flowing, so nothing looks wrong on your
   side.

2.x never looped because by the time your thread ran, the connection was
already dead. Choose one of these:

- **Remove your reconnect code** and let the SDK reconnect. It subscribes
  again by itself; listen for `reconnect` / `authenticated` if you need to
  know.
- **Or turn auto-reconnect off** and keep your code as it is:
  `ReconnectConfig.disabled()` (Python), `reconnect: { enabled: false }`
  (Node), `enabled = false` in the reconnect config (C#, Go, Java, C++).

**Telling a drop the SDK recovers from apart from the end** (#293). Don't
work it out from the close code — core reconnects after most codes,
4xxx included (#201). Ask the SDK: Node's `disconnect` event carries
`willReconnect` (and `intent`: `'client'`, `'server'` or `'network'`);
in Python read `ws.stock.last_disconnect` in your handler; in C#, Java and
C++ it is the `will_reconnect` argument of `on_disconnected`, and
`last_disconnect()` has the rest; with Go's `StreamingClient`, `Messages()`
is closed when the connection is over and `LastDisconnect()` says why (read
it before `Close()`). When `willReconnect` is false the
connection is over; when it is true a `reconnect` follows. A reconnect that
gives up ends with `error` 3005, not another `disconnect`.

```javascript
ws.stock.on('disconnect', ({ code, reason, intent, willReconnect }) => {
  if (!willReconnect) console.log('connection over:', intent, code, reason);
});
```

```python
def on_disconnect(code, reason):
    info = ws.stock.last_disconnect  # the disconnect being handled
    if not info.will_reconnect:
        print("connection over:", info.intent, code, reason)

ws.stock.on("disconnect", on_disconnect)
```

The SDK warns once per client when it sees this pattern: `disconnect()`
closing a connection that auto-reconnect restored less than 30 seconds
earlier, then `connect()` on the same client less than 30 seconds after
that. The warning comes from that `connect()`, so the loop is caught in its
first round. A `disconnect()` with nothing after it — the end of a script or
a test — is not warned about, however soon after a reconnect it comes, and
neither is a `connect()` that joins a reconnect under way. It covers the
usual timing only — code that closes the connection while the reconnect is
still under way, or waits longer before connecting again, loops without
it — so no warning does not mean no loop. Python issues a `RuntimeWarning`,
Node a process warning (`FugleReconnectWarning`, code
`FUGLE_RECONNECT_CONFLICT`), and C#, Go, Java and C++ report code 3006
through their error callback. The warning changes nothing else: the
`connect()` goes ahead and so does the loop, so fix the code.

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
from fugle_marketdata import ApiError, MarketDataError, RateLimitError

try:
    quote = client.stock.intraday.quote("INVALID")
except RateLimitError as e:
    backoff(e.headers.get("retry-after"))
except ApiError as e:
    log.error("%s %s %s", e.code, e.status, e.body)
except MarketDataError as e:
    raise
```

`str(e)` is `e.message`, the same text as `args[0]`. It is not 2.x's format:
2.x's `FugleAPIError` printed several lines — `[Fugle API Error] <message>`,
then `URL: ...`, `Status: ...`, `Params: ...` and the first 200 characters
of the response — and `args` was that one string. 3.0 prints only the
message, e.g. `Authentication error: {"message":"Unauthorized","statusCode":401}`,
and `args` is `(message, code)`. The request URL and params are not in the
message nor anywhere else (`e.url` and `e.params` are always `None`); the
status and body are in `e.status` and `e.body`. Log monitors that match the
`[Fugle API Error]` prefix or the `URL:` line need updating.

Other fields that changed meaning:

- **`e.message`** was the `message` of the server's JSON (`Resource Not
  Found`). It is now the SDK's description, the same text as `str(e)`
  (`API error (status 404): {"statusCode":404,"message":"Resource Not
  Found"}`). The server's own text is `json.loads(e.body)["message"]`.
- **A response that is not valid JSON** raised `FugleAPIError("Failed to
  parse JSON response")` with `status_code` and `response_text`. It is now a
  `MarketDataError` with code 9999 whose message is the parser's, and
  `status` / `body` are `None`.
- **`FugleAPIError(...)` cannot be constructed with 2.x's keywords** (`url=`,
  `status_code=`, `response_text=`, `params=`): `MarketDataError() takes no
  keyword arguments`. Tests and mocks that build one need another exception
  or a stub.
- **A missing or extra credential** raised `TypeError` in 2.x
  (`One of the "apiKey", ... options must be specified`). It is now
  `ConfigError` (code 1004), which is not a `TypeError`; `except TypeError:`
  around the constructor needs `except ConfigError:`. A credential that is
  only whitespace counts as missing.

#### 7. REST has a default request timeout

The legacy SDKs set **no** timeout (Python's `requests.get`, Node's `fetch`),
so a stalled connection hangs forever. This SDK gives up on a request after
30 seconds, in every language. If you actually relied on the no-timeout
behaviour you will see new `TimeoutError` exceptions — the fix is to retry
at the application layer.

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

#### 9. Python `connect()` raises `AuthError` on auth failure

Both this SDK and the legacy SDK block in `connect()` until the server has
either accepted or rejected authentication, fire `unauthenticated` for a
rejection, and then raise. What is raised differs:

- **Legacy (2.x)**: a plain `Exception('Invalid authentication credentials')`
  (or `Exception('authentication timeout')`), so `str(e)` was that text.
- **This SDK**: `AuthError` (code 2002) for rejected credentials, a
  `MarketDataError` subclass for the other failures. `except Exception:`
  still catches it; `str(e)` is `Authentication error: Invalid authentication
  credentials`, so code that compares the text needs updating.

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
# 2.x's keyword, as before
client.futopt.historical.candles(product="TXF", contract_month="1!", timeframe="D")
```

#### 11. Node WebSocket `connect()` rejects while already connected, and waits during a reconnect

The legacy Node SDK let you call `connect()` on a connected client: it opened
another socket, left the old one open, and registered its listeners again.
This SDK rejects that call with an error whose `code` is `2011`
(`Already connected; connect() is not needed while the connection is open or
being opened`) and keeps the existing connection; so does a `connect()` made
while the first `connect()` is still in progress. The connection is already
there, so there is nothing to do; do not call `disconnect()` to make room for
it — with auto-reconnect on, that is the loop §5 describes. Calling
`connect()` after `disconnect()` for another reason is fine.

During an automatic reconnect (on by default, see §5), `connect()` does not
reject with 2011: it waits for the reconnect and resolves with its
`authenticated` data once the subscriptions have been re-sent (#230). So a
1.x `disconnect` handler that calls `connect()` keeps working with the
default settings. It rejects if the reconnect does not come back — code
2010 when `disconnect()` is called or the connection ends without
reconnecting, 3005 when the attempts run out, the server's `data` object
when the credentials are rejected — so add a `.catch`: without one, an
unhandled rejection ends the process unless the client has an `error`
listener and the rejection is not rejected credentials (§12). With auto-reconnect off
(`reconnect: { enabled: false }`), the handler opens a new connection, as in
1.x.

```javascript
// default settings: waits for the automatic reconnect
ws.stock.on('disconnect', () => {
  ws.stock.connect()
    .then(() => ws.stock.subscribe({ channel: 'trades', symbol: '2330' }))
    .catch(console.error);
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
| `connect()` | resolves with the `authenticated` `data` (when it waits on an automatic reconnect, that reconnect's `data`, §11); rejects with the `unauthenticated` `data` object |
| `disconnect` | `{ code, reason, intent, willReconnect }` (`code` is `null` when the connection ended without one; `intent` and `willReconnect` are new in 3.0, see §5 — 1.x listeners that destructure `{ code, reason }` are unaffected) |
| `ping(params)` | `params` sent as the frame's `data` |

Listeners are kept as the 1.x `EventEmitter` kept them (#307): each `on()`
(or `addListener()`) adds one, every listener of an event is called in
registration order with `this` set to the client, and `once`, `off` /
`removeListener`, `removeAllListeners(event?)` and `listenerCount(event)`
work as on an EventEmitter. Unlike 1.x, each access to `ws.stock` /
`ws.futopt` returns a new wrapper of the same client: listeners, connection
and methods are shared, so a listener registered through one access is called
however the client is reached later, and `this.subscribe(...)` works, but
`this` is not `===` to the `ws.stock` of another access (keep one in a
variable if you compare them). The client is not an `EventEmitter`, though:
`instanceof EventEmitter` is `false`; `emit`, `prependListener`,
`listeners`, `eventNames` and `setMaxListeners` do not exist; and
`on()` / `once()` throw for an event name that is not one of the events
above.

Registering a listener does not by itself keep the client from being
garbage-collected, but a listener holds what it captures: with
`const s = ws.stock; s.on('message', () => s.subscribe(...))` the listener
keeps `s`, and with it the client, alive until it is removed. Call
`removeAllListeners()` before dropping a client whose listeners capture it.

> **From an earlier 3.0 release candidate:** `on()` replaced the event's
> previous listener. It now adds one; to replace, call
> `removeAllListeners(event)` first.

A failed `connect()` rejects, but the 1.x README's
`ws.stock.connect().then(...)` with no `.catch` does not end the process
when the client has an `error` listener, registered before `connect()` or
before it fails (#307): in 1.x that Promise never settled and the failure
only reached `error`. This covers `connect()` alone and `.then(f)` chains
without a rejection handler; `await`, `.catch` and `.then(f, r)` still
receive the rejection. It does not cover rejected credentials (1.x rejected
those too), code 2011 (`connect()` on a connection already open or being
opened, §11 — a mistake in the calling code), a client without an `error`
listener, an error thrown by `f`, `.finally()`, `Promise.all()` and
`Promise.race()` around `connect()`, an async function that returns
`connect()`'s promise, or anything chained after those or after a `.catch`:
those still reject unhandled, so give them a `.catch`.

To do this, `connect()` returns a subclass of `Promise`: `console.log` shows
it as `ConnectPromise`, and `await ws.stock.connect()` takes two more
microtask ticks than awaiting a plain `Promise`. It is still a `Promise`
(`instanceof Promise` is `true`).

Three differences remain from 1.x's `error` event:

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
`{ attempt }`, and a `messagesDropped` event, receiving `{ dropped, total }`,
for messages dropped while 4096 were unread (see
[Defaults that changed](#defaults-that-changed)).

The `disconnect` argument is a plain object, not the socket's `CloseEvent`:
`wasClean`, `type` and `target` are gone (`intent` tells who closed it), and
`code` is `null` where 1.x's socket reported 1006, a connection that ended
without a Close frame. 1.x passed a second argument,
`{ reason: 'health-check-timeout' }`, when its health check closed the
connection; 3.0 passes none, and a heartbeat timeout is `intent:
'network'` with the reason `Heartbeat timeout after …`.

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

#### 15. Python WebSocket callbacks run one at a time on one thread

Each client (`ws.stock`, `ws.futopt`) delivers all of its callbacks —
`message`, `connect`, `authenticated`, `disconnect`, `reconnect`, `error`,
`messages_dropped` — on one SDK thread, in order. A callback that blocks
holds up every later event and message of that client until it returns.

While it blocks, messages from the server wait in a queue of
`message_buffer` messages (default 4096). With the default
`message_overflow="drop_newest"`, once that queue is full new messages are
**dropped**. 2.x was single-threaded too, but a blocked callback only
delayed data; here it loses data. The common 2.x pattern of sleeping and
retrying inside a `disconnect` callback is exactly this case: with
auto-reconnect on (§5) the SDK is back and receiving within seconds, and
everything past the queue's size is dropped while your callback sleeps.

To see drops, listen for `messages_dropped` (`fn(dropped, total)`, at most
once per second) or read `messages_dropped_total()`:

```python
ws.stock.on("messages_dropped", lambda dropped, total: log.warning("dropped %d (%d total)", dropped, total))
```

Keep callbacks short: hand slow work to your own thread or queue, consume
messages with `ws.stock.messages()` instead of a `message` callback, and
leave reconnecting to the SDK (§5). `message_overflow="unbounded"` never
drops, at the cost of memory that grows for as long as you lag.

#### 16. Python `subscribed` data can be a list

- **A `subscribed` message's `data` can be a list.** After an automatic
  reconnect (§5) the SDK subscribes again with one frame per channel,
  batching the symbols, so for a channel with more than one symbol the
  server answers with a `subscribed` whose `data` is a list of
  subscriptions, even if you subscribed one symbol at a time. (Subscribing with `symbols=[...]` yourself gets the same shape.)
  Code such as `msg["data"]["id"]` then fails; handle both shapes:

```python
def on_message(msg):
    if msg["event"] == "subscribed":
        data = msg["data"]
        for sub in data if isinstance(data, list) else [data]:
            ids[sub["channel"], sub["symbol"]] = sub["id"]
```

#### 17. From an earlier 3.0 release candidate only: a Python `messages().__anext__()` awaitable can already be done

This does not concern code coming from 2.x, which had no async iterator. It
affects code written against an earlier 3.0 release candidate that handles
the awaitable of `messages().__anext__()` itself (`asyncio.wait`,
`asyncio.gather`, `cancel()`), or passes `asyncio.wait_for` a timeout
shorter than about 1 ms. `async for msg in ws.stock.messages()` and a plain
`await messages.__anext__()` need no change and lose no message.

While messages are queued, `messages.__anext__()` returns an awaitable that
is **already done** and holds the next message, instead of one that is
always pending: reading a backlog no longer takes a thread hop per message.
One step in every 32 in a row is still pending and is delivered by the event
loop, so other tasks on it get to run; how long they wait is about 32 times
what your loop body takes per message. Code that holds the awaitable itself
must not discard a done one:

- **`cancel()` can fail.** `asyncio.ensure_future(messages.__anext__()).cancel()`
  returns False for a done awaitable; the message is its `result()`.
- **`asyncio.wait`, then cancelling the rest, drops a message** when the
  step was done as well:

```python
step = asyncio.ensure_future(messages.__anext__())
done, pending = await asyncio.wait({step, stop}, return_when=asyncio.FIRST_COMPLETED)

# Before: drops the message `step` holds when both are done
if stop in done:
    step.cancel()
    return

# After: look at the step first
if step.done():
    handle(step.result())
else:
    step.cancel()          # pending: takes no message
```

- **A cancelled `asyncio.gather(messages.__anext__(), other)` drops a
  message**: the step is done, `gather` is cancelled while it waits for
  `other`, and the result is discarded. Read the step on its own, or keep a
  reference to it and read `result()` when the `gather` is cancelled.
- **`asyncio.wait_for(messages.__anext__(), timeout)`** loses no message
  when `timeout` is longer than one turn of the event loop, in practice
  about 1 ms or more. `timeout=0` returns the message when one is queued; it
  always raised `TimeoutError` before.
- **`asyncio.wait_for` with a shorter timeout can lose a message.** A step
  left to the loop (one in 32 of a backlog) is resolved one turn before the
  waiting task resumes; a timeout that expires within that turn cancels the
  task with the message already in the awaitable. Seen on Python 3.12 with
  `timeout=1e-6`, which lost the 32nd, 64th and 96th of 100 queued
  messages; before, such a call only timed out. Use a timeout of 1 ms or
  more.
- **Up to Python 3.11, `task.cancel()` can be swallowed while messages are
  queued.** There `wait_for` spends a turn of the loop even on a done
  awaitable, and a task cancelled in that turn gets the message back instead
  of `CancelledError` (3.8.6 and later; run on 3.8.10 only). A
  `while True: await asyncio.wait_for(messages.__anext__(), t)` loop stopped
  with `task.cancel()` may therefore only stop at a step with nothing
  queued; check a flag of your own in the loop. Python 3.8.0 to 3.8.5 raise
  `CancelledError` there and drop that message.
- **Several awaitables of one iterator at once** may not get the messages
  in the order the awaitables were created. A single `async for` does.

Before cancelling a step, check `done()`, or read `result()`. A pending step
is as before: cancelling it takes no message.

#### 18. Node refuses options and arguments it would not use

The 1.x SDK passed unknown keys along and the server mostly ignored them;
an earlier 3.0 release candidate dropped them. Now each of these throws (or,
for a REST method, rejects with) a `TypeError` that names the key and what it
takes, so a typo fails at the call instead of returning default data:

- **`new WebSocketClient()` / `new RestClient()`**: an unknown key at any
  level (`reconnect: { maxRetries: 3 }`), a value of the wrong type, a
  nested option that is not a plain object, or a bare `version` string
  (`version: 'v1.0'` — write `version: { futopt: 'v1.0' }`, as 1.7.0 already
  required). An object made with `Object.create(defaults)` is not plain;
  spread it instead (`{ ...defaults, apiKey }`). `undefined` and `null` count as not given, except
  `version: null`. `RestClient` still accepts the WebSocket-only keys, so one
  options object builds both clients, and `healthCheck.pingInterval` /
  `maxMissedPongs` are still only warned about.
- **`subscribe()` / `unsubscribe()`**: a key other than `channel`,
  `symbol`, `symbols` and the product's flag (`intradayOddLot` for stock,
  `afterHours` for futopt), a flag that is not a boolean, a non-string
  symbol, or a second argument. `unsubscribe()` also takes `id` / `ids`, and
  without `channel` only those two. 1.x sent such keys to the server as they
  were; `{ channel, symbol, afterHours: true }` on the stock client quietly
  subscribed board-lot data.
- **REST methods**: an argument of the wrong type, an integer that is
  negative, fractional or not finite, or an argument the method does not
  take — after the params object, or past its positional parameters —
  rejects the promise; nothing is sent. 1.x methods took one params object,
  so 1.x call sites are not affected unless they passed something extra. The
  params object may still be any object (`Object.create(...)`, a class
  instance); only the client options have to be plain objects.

A getter or Proxy trap that throws while an argument is read throws its own
error, synchronously, as before.

#### 19. The legacy clients' plumbing is gone: `get_client()`, `request()`, `options`

The 2.x / 1.x factories and product clients exposed the pieces they were
built from. 3.0 builds them in the Rust core, so these members do not exist
and using one raises `AttributeError` (Python) or `TypeError: … is not a
function` / `undefined` (Node). In Node 1.7.0 they were not public API —
`getClient` is `private` and `options` / `request` are `protected` in its
TypeScript types — so only plain JavaScript could reach them at run time:

| Legacy member | Python 2.7.0 | Node 1.7.0 | In 3.0 |
|---|---|---|---|
| `get_client('stock' \| 'futopt')` / `getClient(...)` on `RestClient` and `WebSocketClient` | public | TS-private; JS run time only | the `.stock` / `.futopt` properties (see below) |
| Generic `request(path, **params)` / `request(endpoint, params)` | public, on each endpoint group: `stock.{intraday,historical,snapshot,technical,ownership,corporate_actions}`, `futopt.{intraday,historical}` | TS-protected, on the product clients `stock` / `futopt`; JS run time only | call the named method (`client.stock.intraday.quote(symbol="2330")`); see below for endpoints without one |
| `options` on `RestClient` / `WebSocketClient` | public | TS-protected; JS run time only | not exposed — keep the options you passed to the constructor |
| `config` (Py) / `options` (Node) on the product clients `stock` / `futopt` | public; also `config` on each endpoint group | TS-protected; JS run time only | not exposed |

`get_client()` returned one cached client per product. In 3.0 the
`.stock` / `.futopt` properties take its place: in Python the first read
builds the client and every later read returns the same object
(`ws.stock is ws.stock`, since
[#306](https://github.com/fugle-dev/fugle-marketdata-sdk/issues/306)); in
Node each read returns a new wrapper over the same callbacks and connection.
Either way, the legacy idiom of reading the property for each call works:

```python
ws.stock.on("message", handle)
ws.stock.connect()
ws.stock.subscribe({"channel": "trades", "symbol": "2330"})
```

Code written against a 3.0 release candidate that read `ws.stock` twice to
get two connections (`a = ws.stock; b = ws.stock`) now gets one client
twice. For two connections, create two `WebSocketClient`s.

Every endpoint the 2.7.0 / 1.7.0 clients had a method for has one in 3.0,
with the same name. `request()` was only needed for a path the SDK had no
method for. 3.0 has no generic escape hatch for that; call the REST API
directly with the same credentials header:

```python
# Legacy
client.stock.intraday.request("intraday/quote/2330", type="oddlot")

# This SDK
client.stock.intraday.quote(symbol="2330", type="oddlot")

# A path 3.0 has no method for: plain HTTP
import requests
requests.get(
    "https://api.fugle.tw/marketdata/v1.0/stock/<path>",
    headers={"X-API-KEY": api_key},  # or Authorization: Bearer / X-SDK-TOKEN
    params={...},
).json()
```

```js
// Legacy
await client.stock.request('intraday/quote/2330', { type: 'oddlot' });

// This SDK
await client.stock.intraday.quote({ symbol: '2330', type: 'oddlot' });
```

The legacy `request()` returned the parsed body; 2.x raised `FugleAPIError`
on HTTP ≥ 400, while 1.x resolved with the error body. The named methods
behave as described in [§6](#6-python-exception-hierarchy-is-finer-grained)
and [§8](#8-node-rest-rejects-on-http-errors-instead-of-resolving-the-error-body).

#### 20. Python: smaller differences from 2.x

Each of these breaks only code that relied on it:

- **`RestClient(**options)` refuses the WebSocket options.** 2.x ignored
  `health_check`, `version` and any other keyword, so one options dict could
  build both clients; 3.0 raises `TypeError: unexpected keyword argument`.
  Pass `RestClient` only `api_key` / `bearer_token` / `sdk_token`,
  `base_url` and the TLS options. `WebSocketClient` also refuses a keyword
  it does not take, where 2.x passed everything on.
- **Only the package itself can be imported.** `fugle_marketdata.rest`,
  `.websocket`, `.exceptions`, `.constants`, `.client_factory` and
  `.base_url` do not exist (`ModuleNotFoundError`); import every name from
  `fugle_marketdata` (`from fugle_marketdata import FugleAPIError`). The 2.x
  internals on the WebSocket client — `ee`, `auth_status`, `config`,
  `health_check`, `ping_timer`, `check_auth_status()` — are gone too.
- **`from fugle_marketdata import *` brings in `ConnectionError` and
  `TimeoutError`**, the SDK's, which hide Python's built-ins of the same
  name in that module: an `except TimeoutError:` there no longer catches
  `socket.timeout` or `asyncio.TimeoutError`. Import the names you use.
- **`on()` refuses an unknown event name** with `ValueError` listing the
  events; 2.x's `pyee` accepted any name and never called it.
- **A command while not connected raises `RuntimeError`** (`Not connected.
  Call connect() first.`) — `subscribe()`, `unsubscribe()`, `ping()`,
  `subscriptions()` before `connect()` or after `disconnect()`. 2.x raised
  websocket-client's `WebSocketConnectionClosedException`.
- **`connect()` on a connected client raises `WebSocketError` 2011**, as
  in Node (§11). 2.x returned at once without an error, keeping the one
  connection.
- **`disconnect` callbacks get two arguments, `(code, reason)`.** 2.x passed
  a third, `{"reason": "health-check-timeout"}`, when its health check
  closed the connection; read `ws.stock.last_disconnect.intent` instead
  (`"network"`, with the reason `Heartbeat timeout after …`).
- **`authenticated` fires before the auth frame reaches `message`**; 2.x
  delivered the frame to `message` first.

#### 21. Node: smaller differences from 1.x

- **Constructor errors are `Error`, not `TypeError`.** No credential, more
  than one, or a `baseUrl` with a version segment threw `TypeError` in 1.x
  (from the `stock` / `futopt` getter, for `baseUrl`). 3.0 throws an `Error`
  with `code` 1004 from the constructor, and its message starts with
  `Configuration error:`; `instanceof TypeError` checks miss it. A
  credential that is only whitespace counts as missing. (A misspelled or
  unknown option is a `TypeError`, §18.)
- **TypeScript type names changed, and `lib/**` is gone.** 1.x's root types
  `HealthCheckConfig`, `WebSocketProduct`, `WebSocketVersion`,
  `WebSocketVersionMap`, `WebSocketVersionOption` and the
  `WebSocketFutOpt*` message types do not exist; use `HealthCheckOptions`,
  `'stock' | 'futopt'`, `StreamingVersionOptions` and `WebSocketMessage`.
  Imports from `@fugle/marketdata/lib/...` fail: the package ships no `lib/`.
  The `Rest*Params` types are exported from the package root under their
  1.x names, except ownership's (`EtfHoldingsParams`,
  `InstitutionalTradesParams`, `DirectorHoldingsParams`,
  `TdccDistributionParams`). The response types are renamed after the
  method: `RestStockIntradayQuoteResponse` is `QuoteResponse`,
  `RestStockHistoricalCandlesResponse` is `HistoricalCandlesResponse`,
  `RestFutOptIntradayQuoteResponse` is `FutOptQuoteResponse`, and so on.
- **`client.stock` is a new object on each access**, on `RestClient` as on
  `WebSocketClient` (§12): `client.stock === client.stock` is `false`.
  Keep one in a variable to compare it, use it as a `WeakMap` key, or
  attach properties to it.

### New things the legacy SDKs did not have

These are additive and do not break anything; you can ignore them if you
just want a drop-in replacement.

- **`indices` channel** on stock WebSocket — receive index ticks alongside
  trades / books / candles / aggregates.
- **Connection state and latency** — `is_connected()` / `is_closed()` /
  `measure_latency()` (Python), `isConnected` / `isClosed` /
  `measureLatency()` (Node), and the dropped-message count
  `messages_dropped_total()` / `messagesDroppedTotal`.
- **`reconnect` and `messages_dropped` / `messagesDropped` events**, and
  Python's `raw_message` event and `messages()` iterator.
- **`with_full_config`** core constructor — fully tunable reconnect +
  health-check config from a single options object.
- **Per-binding async runtime integration** — Python uses
  `pyo3-async-runtimes`, JS uses napi-rs Promises and the tokio runtime.

### Per-language quickstart

#### Python

```python
from fugle_marketdata import RestClient

client = RestClient(api_key="your-api-key")
quote = client.stock.intraday.quote("2330")
print(quote["lastPrice"])
# in asyncio code: quote = await client.stock.intraday.quote_async("2330")
```

```python
import time
from fugle_marketdata import WebSocketClient

ws = WebSocketClient(api_key="your-api-key")

ws.stock.on("authenticated", lambda frame: print("auth ok"))
ws.stock.on("message", lambda msg: print(msg["event"], msg.get("data")))

ws.stock.connect()
ws.stock.subscribe(channel="trades", symbols=["2330", "2317"])
time.sleep(10)
ws.stock.disconnect()
```

#### Node.js

```javascript
const { RestClient, WebSocketClient } = require('@fugle/marketdata');

async function main() {
  const rest = new RestClient({ apiKey: 'your-api-key' });
  const quote = await rest.stock.intraday.quote('2330');
  console.log(quote.lastPrice);

  const ws = new WebSocketClient({ apiKey: 'your-api-key' });
  ws.stock.on('authenticated', () => console.log('auth ok'));
  ws.stock.on('message', (raw) => {
    const msg = JSON.parse(raw);
    console.log(msg.event, msg.data);
  });
  ws.stock.on('error', (err) => console.error(err.code, err.message));
  await ws.stock.connect();
  ws.stock.subscribe({ channel: 'trades', symbols: ['2330', '2317'] });
  setTimeout(() => ws.stock.disconnect(), 10000);
}

main().catch(console.error);
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
  healthCheck: { probeEnabled: true, idleProbeAfterMs: 15000 },
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

### "ConfigError: Configuration error: Provide exactly one non-empty credential: API key, bearer token, or SDK token"

**Cause:** You provided zero or multiple authentication methods, or one that
is only whitespace. Python raises `ConfigError` (code 1004; 2.x raised
`TypeError`), Node an `Error` with `code` 1004 (1.x threw `TypeError`).

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

**Cause:** v0.3.0 changed the default to `enabled: false`. 3.0 turned it
on again (passive, 35 s; see
[Defaults that changed](#defaults-that-changed)), so this only applies to
the 0.x releases.

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
