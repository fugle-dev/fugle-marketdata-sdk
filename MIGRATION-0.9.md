# Migrating to 0.9.0 (bindings 3.0.0-rc.8, uniffi 0.2.0-rc.6)

Responses now reach you exactly as the server sent them. Previously every
language decoded the JSON into a struct maintained by hand and re-encoded it
on the way out, which lost data in both directions.

Everything in this guide follows from that one change.

## Why

A stock quote used to come back missing `referencePrice` and the top-level
`serial`, while carrying a dozen `false` flags the server never sent. The
struct did not declare the first two, so they were dropped; it did declare the
flags, so they were filled in with defaults.

The structs lived in four separately maintained places — core's serde models,
the UniFFI mirror, the TypeScript declarations, and a hand-written Python dict
builder — and each drifted on its own. An audit found 22 fields missing from
the UniFFI mirror (11 on `FutOptQuote` alone, which made futures quotes
largely unusable) and 41 discrepancies in the TypeScript declarations,
including four field names that were simply wrong.

Handing the body over untouched removes the whole class of bug, and means a
field the API adds tomorrow reaches you without an SDK release.

## 1. REST methods return the response body

### Rust

`send()` returns `serde_json::Value`.

```rust
// Before
let quote = client.stock().intraday().quote().symbol("2330").send()?;
println!("{}", quote.last_price.unwrap_or(0.0));

// After
let quote = client.stock().intraday().quote().symbol("2330").send()?;
println!("{}", quote["lastPrice"].as_f64().unwrap_or(0.0));
```

The typed models are still public, so you can opt back into them:

```rust
use marketdata_core::models::Quote;

let raw = client.stock().intraday().quote().symbol("2330").send()?;
let quote: Quote = serde_json::from_value(raw)?;
```

Note that the models use Rust field names (`last_price`), while the raw JSON
uses the wire names (`lastPrice`).

### Node and Python

Nothing to change in how you read a response — you already got a plain object
or dict. What changes is that it now contains every field the server sent and
only those.

### C#, Go, C++, Java

These return a JSON string. The mirrored response records are gone; decode
with whatever JSON library you already use.

```csharp
// Before
var quote = await client.Stock.Intraday.GetQuoteAsync("2330");
Console.WriteLine(quote.lastPrice);

// After
var json = await client.Stock.Intraday.GetQuoteAsync("2330");
using var doc = JsonDocument.Parse(json);
Console.WriteLine(doc.RootElement.GetProperty("lastPrice").GetDouble());
```

```go
// Before
quote, err := client.Stock().Intraday().GetQuote("2330")
fmt.Println(quote.Symbol)

// After — declare only the fields you use
var quote struct {
    Symbol    string   `json:"symbol"`
    LastPrice *float64 `json:"lastPrice"`
}
body, err := client.Stock().Intraday().GetQuote("2330", nil)
json.Unmarshal([]byte(body), &quote)
fmt.Println(quote.Symbol)
```

## 2. Omitted fields are absent, not `false`

The server sends the trading-session and limit-price flags only when they
apply. They used to be filled in with `false`, so you could not tell "the
server said false" from "the server said nothing".

```javascript
// Before: always a boolean
if (quote.isOpen) { … }

// After: check for presence when the difference matters
if (quote.isOpen === true) { … }
```

Optional numeric fields behave the same way: `quote.previousClose` may be
absent outside trading hours rather than `null`.

In TypeScript these are now declared `field?: T`, so `strictNullChecks` will
point at the places that need attention.

## 3. `tickers()` keeps its envelope

`tickers()` used to unwrap the response and return just the array, throwing
away the surrounding `date` / `type` / `exchange` / `market` metadata. That
also diverged from the official SDK. The envelope is now returned intact.

```javascript
// Before
const tickers = await client.stock.intraday.tickers('EQUITY');
tickers.forEach(…);

// After
const res = await client.stock.intraday.tickers('EQUITY');
res.data.forEach(…);
console.log(res.exchange);  // now reachable
```

## 4. WebSocket messages are the frame itself

Node and Python hand you the frame exactly as it arrived. Previously the SDK
rebuilt it from the five fields it routes on, so anything else the server sent
was discarded and the absent ones came through as `null`.

C#, Go, C++ and Java gain `StreamMessage.raw`, the frame verbatim. The
existing `event` / `channel` / `symbol` / `id` / `dataJson` fields are
unchanged and remain the convenient way to dispatch — they are a parsed subset,
not the source of truth. `dataJson` is the `data` part of `raw` as sent,
not a re-serialization (§22).

```go
// Full frame
var frame map[string]interface{}
json.Unmarshal([]byte(msg.Raw), &frame)

// Still fine for dispatching
switch msg.Event {
case "data": …
}
```

## 5. `serial` keeps the server's type

The typed models normalise `serial` to a string, because futopt sends a
zero-padded one (`"00379320"`) that would lose its leading zeros as a number.
Raw passthrough does no such thing: stock `serial` arrives as a number and
futopt's as a string, matching the API and the official SDK.

If you need one consistent type, coerce at your boundary — or decode through
`marketdata_core::models`, which still normalises.

## 6. Key order follows the server

Object keys used to be reordered alphabetically (`amplitude`, `asks`,
`avgPrice`, …). They now arrive in the server's own order. This matters only
if you iterate keys or compare serialized output.

## 7. C#: `BaseUrl` now works

`RestClientOptions.BaseUrl` was accepted and silently ignored, so a client
pointed at a staging or test server still talked to production. It is applied
now. Pass the host and path prefix only — a version segment such as `/v1.0` is
rejected.

If you were setting `BaseUrl` and relying on the requests going to production
anyway, remove it.

## 8. Rust: connection events

`ConnectionEvent` variants carry more data, so exhaustive matches and
constructions need updating:

```rust,ignore
// Before
ConnectionEvent::Authenticated => {}
ConnectionEvent::Unauthenticated { message } => {}
ConnectionEvent::Disconnected { code, reason, intent } => {}

// After
ConnectionEvent::Authenticated { data } => {}          // server frame's `data`, or Null
ConnectionEvent::Unauthenticated { message, data } => {}
ConnectionEvent::Disconnected { code, reason, intent, will_reconnect } => {}
```

Patterns that already end in `..` keep compiling.

Behaviour you may have worked around:

- After `HeartbeatTimeout` a `Disconnected { intent: Network }` now follows.
  If you treated `HeartbeatTimeout` as the disconnect, react to
  `Disconnected` instead or you will handle the close twice.
- `ReconnectFailed` no longer fires with `attempts: 0` when the reconnect
  policy does not retry a close. Use `Disconnected { will_reconnect: false }`
  to learn that the client has stopped, instead of re-running
  `ReconnectionManager::should_reconnect` yourself.

## 9. Python: connection callbacks

`authenticated` and `unauthenticated` receive the server frame's `data`
(`dict`, or `None` when the frame has none):

```python
# Before
ws.stock.on("authenticated", lambda msg: ...)      # {"event": "authenticated"}
ws.stock.on("unauthenticated", lambda message: ...)  # "Invalid authentication credentials"

# After
ws.stock.on("authenticated", lambda data: ...)     # {"message": "Authenticated successfully"}
ws.stock.on("unauthenticated", lambda data: data["message"])
```

`connect` fires when the WebSocket opens, before authentication, so it also
fires for a rejected key; wait for `authenticated` if you need an
authenticated connection.

## 10. C#, Go, C++, Java: WebSocket listener

The listener now forwards core's connection events one-to-one, so it gains
two methods and `on_disconnected` gains a parameter. Every listener
implementation has to be updated to compile.

```csharp
// Before
public void OnConnected() { }
public void OnDisconnected() { }

// After
public void OnConnected() { }                              // transport up, not yet authenticated
public void OnAuthenticated(string? dataJson) { }          // server accepted the credentials
public void OnUnauthenticated(string? dataJson) { }        // server rejected them
public void OnDisconnected(bool willReconnect) { }
```

| | C# | Go | Java | C++ |
|---|---|---|---|---|
| authenticated | `OnAuthenticated(string? dataJson)` | `OnAuthenticated(dataJson *string)` | `onAuthenticated(String dataJson)` | `on_authenticated(std::optional<std::string>)` |
| unauthenticated | `OnUnauthenticated(string? dataJson)` | `OnUnauthenticated(dataJson *string)` | `onUnauthenticated(String dataJson)` | `on_unauthenticated(std::optional<std::string>)` |
| disconnected | `OnDisconnected(bool willReconnect)` | `OnDisconnected(willReconnect bool)` | `onDisconnected(Boolean willReconnect)` | `on_disconnected(bool)` |

`dataJson` is the `data` member of the server's frame, still encoded as JSON,
and null (`nil`, `std::nullopt`) when the frame has none. For a rejection the
server's message is under `message`.

Behaviour you may have relied on:

- `on_connected` used to mean "connected and authenticated" and fired once.
  It now fires as soon as the transport is up and again after each successful
  reconnect. Move "ready" logic to `on_authenticated`.
- A credential rejection used to arrive as `on_error("Unauthenticated: ...")`.
  It now arrives only as `on_unauthenticated`; `connect()` still fails.
- `on_disconnected` fires once per connection. With reconnect enabled it also
  fires for a connection that is about to be re-established
  (`willReconnect == true`); only `false` — or `on_reconnect_failed` — means
  the client has stopped.
- Go `StreamingClient` keeps `Messages()` / `Errors()` open across a
  reconnect. Java's pull mode and Go's `Errors()` report a rejection as
  `Unauthenticated: <dataJson>`.

## 11. Rust: one stream for messages and events

A client reports its messages and its connection events on one stream, in
the order they happened: every message of a connection comes after its
`Authenticated` and before its `Disconnected`. `messages()`,
`message_stream()`, `events()` and `state_events()` are gone:

```rust,ignore
// Before
let messages = client.messages();                 // blocking
let mut stream = client.message_stream();         // async (aio)
let events = client.events().lock().unwrap();     // lifecycle

// After
use marketdata_core::{StreamItem, websocket::ConnectionEvent};

let items = client.stream_receiver();             // blocking, both clients
while let Ok(item) = items.receive() {
    match item {
        StreamItem::Message(msg) => { /* was messages() */ }
        StreamItem::Event(ConnectionEvent::Disconnected { .. }) => break,
        StreamItem::Event(_) => { /* was events() */ }
        _ => {}
    }
}

let mut stream = client.stream();                 // async (aio): futures::Stream
while let Some(item) = stream.recv().await { /* same items */ }
```

`stream()` and `stream_receiver()` take the same stream, so use one of them.
Reading messages and events from two places no longer works; dispatch on the
item instead. `ConnectionStream::try_recv()` returns
`Err(std::sync::mpsc::TryRecvError)`, and `ConnectionStream` implements
`futures::Stream` itself, so drop any `tokio_stream` wrapper.

`ConnectionEvent` and `StreamItem` are `#[non_exhaustive]`; add a `_` arm to
exhaustive matches. The new `MessagesDropped { dropped, total }` reports
messages dropped because your consumer fell behind.

`ConnectionConfig` gains `message_overflow`. Code that builds the struct with
a literal needs the field (`MessageOverflow::DropNewest` keeps the default);
code using `ConnectionConfig::new` or the builder is unaffected.

`messages_dropped_total()` now counts from the start of the current
connection (it restarts at every `connect()` or reconnect attempt) rather than
from client construction; sum the `MessagesDropped` events if you need a
lifetime total.

Behaviour you may notice:

- Messages are capped at `message_buffer` (4096). The former `messages()`
  queued without limit, so a slow consumer never lost messages but used more
  and more memory. To keep every message, opt in with
  `.message_overflow(MessageOverflow::Unbounded)`. Events have their own
  `event_buffer` allowance, so a full message queue never costs an event.
- The server's `authenticated` frame now follows the `Authenticated` event;
  it used to be queued on the message channel before the event was emitted.
- Frames that arrive after a connection's `Disconnected` (only possible after
  `force_close()` or a `disconnect()` that timed out) are discarded.

## 12. C#, Go, C++, Java: dropped messages

`WebSocketListener` gains `on_messages_dropped(count)`, called when messages
were dropped because they were not consumed fast enough (#46). `count` is the
number dropped since the previous call; the client's
`messages_dropped_total()` has the running total for the current connection.
Every listener implementation has to add it to compile. C#'s
`IWebSocketListener` declares it without a default implementation, so this
applies there too.

| C# | Go | Java | C++ |
|---|---|---|---|
| `OnMessagesDropped(ulong count)` | `OnMessagesDropped(count uint64)` | `onMessagesDropped(Long count)` | `on_messages_dropped(uint64_t)` |

```csharp
public void OnMessagesDropped(ulong count)
{
    Console.WriteLine($"dropped {count} message(s)");
}
```

Up to 4096 unread messages are kept; after that new ones are dropped. To
change the limit or never drop, pass `MessageOverflow` / `MessageBuffer`
(C# `WebSocketClientOptions`, Go `WithMessageOverflow` / `WithMessageBuffer`,
Java builder `messageOverflow` / `messageBuffer`, C++
`new_with_options(..., MessageQueueConfigRecord)`).

Behaviour you may notice:

- Go `StreamingClient` and Java pull mode have no listener of their own:
  drops arrive on `Errors()` as `messages dropped: <count>`, and on Java's
  error queue as `Dropped <count> message(s): listener fell behind`. Go
  skips a drop report when `Errors()` is full, so read `Errors()` alongside
  `Messages()`; other errors still wait for room there.
- Java pull mode used to discard messages silently once its `queueCapacity`
  queue was full. It now waits for `poll()` to make room, so the drop happens
  in the SDK's queue instead, where it is counted and reported. A client that
  never polls holds up delivery until `disconnect()` or `close()`.

## 13. Errors: one set of fields in every language

Every error now carries the same fields — `code`, `source_kind`, `message`,
`status`, `body`, `request_id`, `headers` — defined once in core
(`ErrorInfo`). [docs/errors.md](docs/errors.md) lists the names per language
and every error code. Error code values are unchanged.

### Node

- The `[code]` prefix is gone from `err.message` of REST rejections,
  constructor errors and `connect()` rejections, matching the WebSocket
  `error` event. Branch on `err.code` (now a number; REST errors used to have
  the string `"GenericFailure"` there) instead of matching the message:

  ```javascript
  // Before
  if (e.message.includes('[2002]')) { /* auth failed */ }
  // After
  if (e.code === 2002) { /* auth failed */ }
  ```

- New properties: `sourceKind`, `status`, `body`, `requestId`, `headers`
  (TypeScript: `MarketDataError`).

### Python

New attributes `code`, `source_kind`, `status`, `body`, `request_id`,
`headers`. `args` is still `(message, code)`; `status_code` and
`response_text` are aliases of `status` and `body`, and `response_text` is no
longer always `None`.

`ReconnectConfig` and `HealthCheckConfig` raise the new `ConfigError`
(`code == 1004`, `source_kind == "client"`) for a value below its floor,
where 3.0.0-rc.5 and earlier raised the built-in `ValueError` with no
`code` (#171). `ConfigError` is a `MarketDataError`, not a `ValueError`, so
an `except ValueError` written against an earlier release candidate no
longer catches it:

```python
# Before (3.0.0-rc.5 and earlier)
try:
    reconnect = ReconnectConfig(initial_delay_ms=50)
except ValueError: ...
# After
try:
    reconnect = ReconnectConfig(initial_delay_ms=50)
except ConfigError as e:
    assert e.code == 1004
```

The credential check of the client constructors (§14) raises `ConfigError`
as well; it was already a `MarketDataError` with code `1004`, so
`except MarketDataError` keeps working. 2.x had neither config class.

### Rust

- `MarketDataError::ApiError` and `MarketDataError::AuthError` gain
  `http: Option<Box<HttpErrorContext>>` (status, raw body, headers). Add `..`
  to patterns, and `http: None` where you construct them.
- A REST 401 / 403 is still `AuthError`, now with `http` set.
- `ConnectionEvent::Error { message, code }` is now
  `ConnectionEvent::Error(ErrorInfo)`:

  ```rust,ignore
  // Before
  ConnectionEvent::Error { message, code } => eprintln!("[{code}] {message}"),
  // After
  ConnectionEvent::Error(info) => eprintln!("[{}] {} ({})", info.code, info.message, info.source_kind),
  ```

- `MarketDataError::info()` returns the `ErrorInfo`; `error_code` has a
  constant per code.

### C#, Go, C++, Java

- Every `MarketDataError` / `MarketDataException` variant gains an `info`
  field (`ErrorInfo` record); `ClientClosed` now has it too. The `msg`
  fields keep their text.
- `on_error` receives an `ErrorInfo` instead of a string; update every
  listener implementation.

| | C# | Go | Java | C++ |
|---|---|---|---|---|
| listener | `OnError(ErrorInfo error)` | `OnError(error ErrorInfo)` | `onError(ErrorInfo error)` | `on_error(const ErrorInfo&)` |
| read from an exception | `ex.GetInfo()` | `ErrorInfoOf(err)` | `e.getCode()`, … `e.getInfo()` | `e.info` |

- Go `StreamingClient.Errors()` delivers a `*StreamError` (its `Error()` is
  the message) for a WebSocket error event, instead of an error built from
  the message string.
- Java's `FugleException` gains `getCode()`, `getSourceKind()`,
  `getStatus()`, `getBody()`, `getRequestId()`, `getHeaders()` and
  `getInfo()`. Pull mode's `pollError()` still returns the message.
- C# `IWebSocketListener` (netstandard2.0, no default interface methods) and
  the wrappers' listener interfaces change the same way.

### WebSocket error events report the matching code

Three WebSocket `error` events used codes from the wrong row of the table.
They now report:

| Event | Before | After |
|---|---|---|
| A frame could not be parsed | `2003` (API error) | `1002` (deserialization) |
| Read failed (connection lost) | `2001` | `3002` (WebSocket), `source_kind` from the failure |
| Write failed, sync client (Rust) | `2002` (auth error) | `3002` (WebSocket), as the async client already did |

If you matched `2001` to detect a lost connection, react to `Disconnected`
(or match `3002` together with `source_kind == network`).

## 14. Credentials: checked once, in core

Every client constructor now applies the same rule, from core (#69): exactly
one of API key, bearer token and SDK token, and a value that is empty or only
whitespace counts as not provided. Otherwise it throws a configuration error,
code `1004`, `source_kind` `client`, with the message `Provide exactly one
non-empty credential: API key, bearer token, or SDK token`. Branch on the
code rather than the message.

What changes for you:

| Language | Empty / whitespace credential | Zero or several credentials |
|---|---|---|
| Python | Was accepted; now `ConfigError` (a `MarketDataError`, `e.code == 1004`) | `TypeError` → `ConfigError` (`e.code == 1004`) |
| Node | Was accepted; now an `Error` with `err.code === 1004` | Same `Error`, now with `err.code === 1004`; message changed |
| Java | Was accepted; now `FugleException` with `getCode() == 1004` | `FugleException` without `getInfo()` → with it, `getCode() == 1004`; message changed |
| Go | `WithApiKey("")` etc. returned `"... cannot be empty"` from the option; now the constructor returns a `*MarketDataError` | `errors.New("provide exactly one of ...")` → `*MarketDataError`; read it with `ErrorInfoOf(err)` |
| C# | `ArgumentNullException` / `ArgumentException` → `MarketDataException` (`ex.GetInfo().code == 1004`) | `ArgumentException` → `MarketDataException` |

- **Python**: `RestClient.with_bearer_token()` and `with_sdk_token()` also
  reject a blank token. The error is `ConfigError` (#171), a
  `MarketDataError`, so either name works in `except`.

  ```python
  # Before
  try:
      client = RestClient(api_key=key)
  except TypeError: ...
  # After
  try:
      client = RestClient(api_key=key)
  except ConfigError: ...          # or: except MarketDataError as e: if e.code == 1004
  ```

- **Node**: `new RestClient({ apiKey: '' })` used to succeed and fail on the
  first request; it now throws.
- **C#**: `new RestClient(string)`, `RestClient.WithBearerToken`,
  `RestClient.WithSdkToken` and the `WebSocketClient(string apiKey, ...)`
  constructors throw `MarketDataException` for a null, empty or whitespace
  credential (was `ArgumentNullException`). A null `options` or `listener` is
  still `ArgumentNullException`.
- A blank credential next to a real one is ignored:
  `RestClient(api_key="", sdk_token="t")` builds an SDK-token client instead
  of rejecting two credentials.
- **Rust**: `Auth::from_credentials(api_key, bearer_token, sdk_token)` applies
  the rule; `Auth::validate()` and `AuthRequest::validate()` check a
  credential you built yourself. `RestClient::new` stays infallible and
  returns the `ConfigError` from the first request; `connect()` on either
  WebSocket client returns it before connecting. `Auth::from_env()` also
  treats a whitespace-only variable as unset.
- **C#, Go, Java, C++ (UniFFI)**: the `new_rest_client_with_*` factories
  return the `ConfigError` for a blank credential, and the new
  `validate_credentials(api_key, bearer_token, sdk_token)` returns which
  `CredentialKind` was given. The WebSocket constructors that take an API
  key still cannot fail; a blank key is reported by `connect()`.
- **WebSocket tokens** (#91): the WebSocket clients now authenticate with a
  bearer token or an SDK token. Python and Node used to send either one as
  an API key, which the server rejects; C#, Go and Java refused them. The
  option wrappers (`WebSocketClientOptions`, `NewFugleWebSocketClient`,
  `FugleWebSocketClient.builder()`) need no change. Raw UniFFI callers use
  `WebSocketClient.new_with_credentials(CredentialsRecord, ...)`, which
  returns the `ConfigError` unless exactly one credential is given.

## 15. WebSocket: unknown channel, code 1005 everywhere

`subscribe()` checks the channel name with core's parser (#114), as Node does
since #113. A name that is not a channel of that product (`trades`,
`candles`, `books`, `aggregates`, plus `indices` for stock) fails with code
`1005` (`INVALID_PARAMETER`), `source_kind` `client`, and the message
`Invalid parameter 'channel': unknown channel 'trade'. Valid channels: trades,
candles, books, aggregates, indices`. Names are matched ignoring case. The
name is checked before the connection, so an unconnected client reports this
error rather than "Not connected".

| Language | Before | After |
|---|---|---|
| Python | `ValueError` | `MarketDataError` (`e.code == 1005`) |
| C#, Go, Java, C++ | `ConfigError` variant, code `1004`; `"Trades"` rejected | `ApiError` variant, code `1005`; `"Trades"` accepted |

- **Python**: `MarketDataError` is not a `ValueError`; update `except`
  clauses. `subscribe_async()` still raises when awaited, not when called.

  ```python
  # Before
  try:
      ws.stock.subscribe("trade", "2330")
  except ValueError: ...
  # After
  try:
      ws.stock.subscribe("trade", "2330")
  except MarketDataError as e:
      if e.code == 1005: ...
  ```

- **C#, Go, Java, C++**: the error is the `ApiError` variant (core's
  `InvalidParameter` maps to it) instead of `ConfigError`. Branch on the code
  (`ex.GetInfo().code`, `ErrorInfoOf(err).Code`, `e.getCode()`, `e.info.code`)
  rather than the variant.

## 16. WebSocket: `connect()` while connected, code 2011

`connect()` on a client that is connected or still connecting fails with
code `2011` (`ALREADY_CONNECTED`), `source_kind` `client`, message
`Already connected; call disconnect() first` (#119), as Node already did.
The live connection is not touched. Call `disconnect()` first to open a new
one, or `reconnect()` (Rust) to replace it. During an automatic reconnect,
`connect()` waits for the reconnect instead of failing with 2011 (#230; the
Rust sync client still fails) — see
[`docs/errors.md`](docs/errors.md) for how the wait ends.

| Language | Before | After |
|---|---|---|
| Rust | `Ok(())`, nothing done | `Err(MarketDataError::AlreadyConnected)` |
| C#, Go, Java, C++ | A second connection replaced the first one's event delivery | `WebSocketError` variant, code `2011` |
| Python | Same (`connect()` and `connect_async()`) | `WebSocketError`, `e.code == 2011` |

- **Rust**: `MarketDataError` is now `#[non_exhaustive]`, so a `match` on it
  needs a `_` arm; the new variant is `AlreadyConnected`.

  ```rust,ignore
  match err {
      MarketDataError::AuthError { msg, .. } => eprintln!("auth: {msg}"),
      MarketDataError::AlreadyConnected => {}
      other => eprintln!("{other}"),
  }
  ```

- **C#, Go, Java, C++**: branch on the code (`ex.GetInfo().code`,
  `ErrorInfoOf(err).Code`, `e.getCode()`, `e.info.code`) rather than the
  variant.
- **Python**: catch `WebSocketError` (or `MarketDataError`) and check
  `e.code == 2011`.

## 17. REST query parameters: checked against the server's table

Every REST endpoint's query parameters are now listed once, in core, from the
server's own request definitions (#164). Node's object form and Python's
keywords are checked against that list before the request is sent, and every
parameter the server takes is reachable from every binding's typed API.

Before, a key or keyword the binding did not know went one of two ways, and
both were silent: Node forwarded it and the server ignored it; Python warned
once (`UserWarning`, which Python prints a single time per call site) and
dropped it. Either way the call succeeded and returned the wrong data:
`trades("2330", limit=5, sort="asc")` came back with 50 trades,
`ticker("2330", type="oddlot")` with board-lot data, and
`futopt.intraday.tickers(type="FUTURE", product="TXF")` with 1793 contracts
instead of 6.

Two of the 28 endpoints are different: `stock/corporate-actions/capital-changes`
and `listing-applicants` are served by a backend that already answered 400 to
any unknown key, so for those two only the error moves from the server to the
call site. For the other 26 this is a real change of behaviour: a typo that
used to return unfiltered data now fails.

Values are not checked. The server's rules for values are not consistent
(`type=oddlot` is case-sensitive, the single-contract `session` is not, the
list endpoints want `REGULAR` / `AFTERHOURS` in upper case), so a bad value
still gets the server's own error.

### Python

- **A keyword the endpoint does not take raises `TypeError`.** The message
  names the method, suggests the nearest accepted spelling when one differs
  only in case or underscores, and lists every accepted keyword:

  ```text
  TypeError: stock.intraday.trades() got an unexpected keyword argument 'istrial'.
  Did you mean 'isTrial'? Accepted: odd_lot, type, oddLot, offset, limit, sort, is_trial, isTrial
  ```

- **The spellings the 2.x SDK used work again.** The 2.x `fugle-marketdata`
  forwarded `**params` verbatim, so its callers wrote the API's own names —
  `isTrial`, `isNormal`, `isSpread`, `contractType`, `rPeriod`, `contractMonth`,
  `from` / `to`, `type="oddlot"`, `session="afterhours"` — plus `from_` for the
  reserved word. 3.0.0-rc.1 to rc.4 dropped all of these with a warning. They
  are accepted now, alongside the 3.x snake_case keywords, which are unchanged.

- **One parameter under two spellings is a `TypeError`**, never a silent
  choice: `from_date` with `from`, `is_trial` with `isTrial`, `odd_lot=True`
  (or `False`) with `type="oddlot"`. The message is the one the ownership
  methods have used since rc.1: `got multiple values for from_date (also
  passed as 'from')`.

- **Every parameter has a keyword now** (#165). `odd_lot` on `ticker` /
  `candles` / `trades` / `volumes`; `offset`, `limit`, `sort`, `is_trial` on
  `trades`; `is_attention`, `is_disposition`, `is_halted`, `symbol` on
  `tickers`; `type_filter`, `gt`, `gte`, `lt`, `lte`, `eq` on `movers`;
  `type_filter` on `actives`; `exchange`, `sort` on the corporate-actions
  methods; `exchange`, `after_hours`, `status` on `products`; `product` on
  the futopt `tickers`; `strike_price`, `call_put` on the futopt historical
  `candles`. New keywords come after the existing ones, so positional calls
  keep their meaning.

- `odd_lot` and `after_hours` default to `None` instead of `False`, so that
  an explicit `False` counts as given in the conflict check. `True` and
  `False` mean what they did.

- **Type checkers only know the snake_case keywords.** The `.pyi` stubs list
  the typed keywords and no `**kwargs`, so mypy / pyright flag `isTrial=` or
  `from_=` even though they work at runtime. Write `is_trial=` / `from_date=`
  in code you type-check; the other spellings are a runtime compatibility
  layer for 2.x call sites.

The table below is the one from #164: every call from the developer.fugle.tw
examples that rc.4 silently mishandled, and what it does now.

| Call (2.x / documentation spelling) | rc.4 | Now |
|---|---|---|
| `stock.technical.sma(**{"symbol": "2330", "from": "2026-08-01", "to": "2026-09-10", "timeframe": "D", "period": 5})` | `from` / `to` dropped: default range (24 rows instead of 29) | sends `from` / `to` |
| `stock.intraday.trades(symbol="2330", limit=5, sort="asc")` | 50 trades | sends `limit=5&sort=asc` |
| `futopt.intraday.tickers(type="FUTURE", exchange="TAIFEX", session="REGULAR", product="TXF")` | 1793 contracts | sends `product=TXF`; `REGULAR` is the server default and is expressed by sending no `session` |
| `stock.intraday.ticker(symbol="2330", type="oddlot")` | board-lot data | sends `type=oddlot` |
| `stock.historical.candles(**{"symbol": "0050", "from": ..., "to": ..., "fields": ...})` | `from` / `to` dropped | sends them |
| `stock.technical.rsi` / `macd` / `bb` with `from` / `to` | dropped | sends them |
| `stock.intraday.tickers(type="EQUITY", isNormal=True)` | dropped | sends `isNormal=true` |
| `stock.intraday.quote(symbol="2330", type="oddlot")` | dropped | sends `type=oddlot` |
| `futopt.intraday.products(type="FUTURE", exchange="TAIFEX", session="AFTERHOURS", contractType="I")` | all three dropped | sends all three |
| `futopt.intraday.tickers(type="FUTURE", isSpread=True)` | dropped | sends `isSpread=true` |
| `futopt.intraday.quote(symbol="TXFD6", session="afterhours")` | dropped | sends `session=afterhours` |
| `stock.technical.sma("2330", from_="2026-08-01", to="2026-09-10", ...)` | `from_` / `to` dropped | sends `from` / `to` |
| `stock.intraday.quote(symbol="2330", totally_bogus=1)` | warning, request sent | `TypeError`, no request |

Each row is a test (`py/tests/test_rest_kwargs_strict.py::TestIssue164Cases`).

### Node

- **An unknown key in the object form rejects** with `code: 1005`
  (`sourceKind: 'client'`) and a message that names the endpoint, the
  suggestion, and the accepted keys:

  ```text
  Invalid parameter 'Product': `futopt.intraday.tickers` does not accept `Product`;
  did you mean `product`? accepted keys: type, exchange, session, product, contractType, isSpread
  ```

  The suggestion appears when the key differs only in case or underscores.
  For `capitalChanges` / `listingApplicants` an unknown key rejected already,
  with the server's error (`code: 2003`, `status: 400`, `property xxx should
  not exist`); it is now the client's (`code: 1005`, `status: null`).

- **TypeScript flags the typo at compile time.** The `Rest*Params` types no
  longer have an `[key: string]: unknown` index signature, and they list every
  key the endpoint takes. A project that relied on the index signature to pass
  an undeclared key no longer compiles; if the key is one the server takes, it
  is in the type now.

- The snake_case spellings (`is_trial`, `contract_month`, `odd_lot`,
  `after_hours`) are accepted at runtime as aliases of the API names, and
  `oddLot: true` works on `ticker` / `candles` / `trades` / `volumes` as it
  did on `quote`. One parameter under two spellings (`type: 'oddlot'` with
  `oddLot: true`, `symbol` with `product`) rejects.

- A missing path param and a nested object value, which threw plain `Error`s,
  now carry the same fields (`code: 1005`).

### Rust

Additive: the builders gain `sort` (intraday candles), `is_attention` /
`is_disposition` / `is_halted` / `symbol` (tickers), `type_filter` and the
`gt` / `gte` / `lt` / `lte` / `eq` thresholds (movers), `type_filter`
(actives), `sort` and `exchange` (corporate actions), `product` (futopt
tickers), `status` (futopt products), `strike_price` / `call_put` (futopt
historical candles). `core::rest::params` holds the table, hidden from the
docs like `RestClient::get_json`.

### C#, Go, Java, C++

Every endpoint's full parameter set is reachable now (#202),
through one params record per method; see [§20](#20-c-go-java-c-one-params-record-per-rest-method).

## 18. Parameters the server never read

Measured against the server (#166, #168): three parameters every language
offered had no effect, or broke the call. They are removed. Nothing you get
back changes; calls that passed them stop compiling (or raise, in Python).

| Parameter | What it did | Replace with |
|---|---|---|
| `stddev` on Bollinger Bands (`bb`) | Sent and ignored; every result used the server's own multiplier | Drop it |
| `period()` on the Rust KDJ builder | A lone `period` got HTTP 400 | `r_period` / `k_period` / `d_period` |
| `date` on `capital_changes` / `dividends` / `listing_applicants` | `capital-changes` and `listing-applicants` answered 400 `property date should not exist`; `dividends` ignored it and returned the default range | `start_date` / `end_date` |

- **Rust**: `BbRequestBuilder::stddev`, `KdjRequestBuilder::period` and the
  three `date()` builders are gone.
- **Python**: `bb()` / `bb_async()` lose `stddev`; the corporate-actions
  methods lose `date`.
- **Node**: `bb()` loses its trailing `stddev` argument. `startDate` moves
  into the corporate-actions methods' first slot — `dividends(startDate?,
  endDate?)`. The old three-argument call `dividends(undefined, start, end)`
  and the old two-argument `dividends(undefined, start)` would run with a
  shifted range, so both reject with a message that says how to rewrite the
  call; use `dividends({ end_date })` for an end date alone. The object form
  is unchanged.
- **C#, Go, Java, C++**: `GetBb` / `BbSync` / `bb_sync` lose the trailing
  `stddev`; `GetCapitalChanges` / `CapitalChangesSync` /
  `capital_changes_sync` and the dividends / listing-applicants
  counterparts lose the leading `date`.

## 19. `sort`: one shape on every builder

`sort` had three shapes in core for one server contract (`asc|desc`): a
string on the candles and corporate-actions builders, `sort_asc()` /
`sort_desc()` on intraday trades, and a `HoldingsSort` enum on the four
ownership builders (#179). It is `sort(&str)` everywhere now, following the
rule from §17: keys are checked, values are sent as given. A sort value the
server adds later needs no SDK release.

- **Rust**: `TradesRequestBuilder::sort_asc()` / `sort_desc()` are replaced
  by `sort("asc")` / `sort("desc")`. `HoldingsSort` is gone; the ownership
  builders' `sort()` takes `"asc"` / `"desc"`.

  ```rust
  // Before
  client.stock().intraday().trades().symbol("2330").sort_desc().send()?;
  client.stock().ownership().etf_holdings().symbol("0050").sort(HoldingsSort::Desc).send()?;
  // After
  client.stock().intraday().trades().symbol("2330").sort("desc").send()?;
  client.stock().ownership().etf_holdings().symbol("0050").sort("desc").send()?;
  ```

- **Python, Node, C#, Go, Java, C++**: no signature change. `sort` was the
  last free-form string parameter the bindings still checked the *value* of —
  `stock.intraday.trades(sort=...)` in Python and the four `stock.ownership.*`
  methods everywhere raised (Python `ValueError`), rejected (Node `Error`)
  or failed with a config error (C#, Go, Java, C++) on anything but `"asc"`
  / `"desc"`. They now send the value and return the server's error, like
  every other parameter.

## 20. C#, Go, Java, C++: one params record per REST method

The uniffi bindings exposed a fraction of the table from §17 (#202): the
single-symbol stock intraday methods had no `oddLot`, `trades` had no
`offset` / `limit` / `sort` / `isTrial`, `tickers` had no filters,
`historical.candles` had no `fields` / `sort` / `adjusted`, `movers` had no
thresholds, the corporate-actions methods had no `exchange` / `sort`, and
the futopt list methods had no filters. Intraday `candles` required a
`timeframe` and always sent it (the Python bug from #196, in another form).
The WebSocket `subscribe` took one symbol and had no `intradayOddLot`,
though core supports both.

Every REST method now takes the endpoint's **required** parameters
positionally — `type` on the list endpoints, `direction` and `change` on
`movers`, `trade` on `actives`, the technical periods — and its **optional**
ones as one record, which may be omitted (C#, Go, Java) or passed as
`std::nullopt` (C++). Every record field is optional and unset means "not
sent"; a default record (`new StockTradesParams()`, `nil`, `null`,
`StockTradesParams{}`) sends nothing, so the server's own defaults apply.
The flag fields (`oddLot`, `afterHours`) send the table's literal when
`true` (`type=oddlot`; `session=afterhours` on the single-contract futopt
endpoints, `session=AFTERHOURS` on `products` / `tickers`) and nothing when
`false` or unset. A field an endpoint does not take — `exchange` on
`capital-changes`, the one record shared by the three corporate-actions
methods — is code 1005, before any request is sent, as in Node and Python.
Values are sent as given (§17): the price thresholds and `strikePrice` are
doubles and `0` is a value like any other, so those fields are unset by
`null`, not by `0`.

The old shapes are gone, not kept as overloads; these bindings have never
been published, so there is nothing to stay compatible with. In summary
(the generated methods; the C#, Go and Java wrappers follow):

| Endpoint | Before | Now |
|---|---|---|
| `stock.intraday.tickers` | `(type)` | `(type, StockTickersParams?)` |
| `stock.intraday.ticker` / `quote` / `volumes` | `(symbol)` | `(symbol, OddLotParams?)` |
| `stock.intraday.candles` | `(symbol, timeframe)` | `(symbol, StockCandlesParams?)` — `timeframe` is a field |
| `stock.intraday.trades` | `(symbol)` | `(symbol, StockTradesParams?)` |
| `stock.historical.candles` | `(symbol, from?, to?, timeframe?)` | `(symbol, StockHistoricalCandlesParams?)` |
| `stock.snapshot.quotes` | `(market, type?)` | `(market, SnapshotParams?)` |
| `stock.snapshot.movers` | `(market, direction?, change?)` | `(market, direction, change, MoversParams?)` |
| `stock.snapshot.actives` | `(market, trade?)` | `(market, trade, SnapshotParams?)` |
| `stock.technical.sma` / `rsi` / `bb` | `(symbol, from?, to?, timeframe?, period?)` | `(symbol, period, TechnicalParams?)` |
| `stock.technical.kdj` | `(symbol, from?, to?, timeframe?, rPeriod?, kPeriod?, dPeriod?)` | `(symbol, rPeriod, kPeriod, dPeriod, TechnicalParams?)` |
| `stock.technical.macd` | `(symbol, from?, to?, timeframe?, fast?, slow?, signal?)` | `(symbol, fast, slow, signal, TechnicalParams?)` |
| `stock.corporateActions.*` | `(startDate?, endDate?)` | `(CorporateActionsParams?)` |
| `stock.ownership.*` | `(symbol, from?, to?, sort?)` | `(symbol, OwnershipParams?)` |
| `futopt.intraday.products` | `(type)` | `(type, FutOptProductsParams?)` |
| `futopt.intraday.tickers` | `(type, isSpread?)` | `(type, FutOptTickersParams?)` |
| `futopt.intraday.ticker` / `quote` / `volumes` | `(symbol, afterHours)` or `(symbol)` | `(symbol, AfterHoursParams?)` |
| `futopt.intraday.candles` | `(symbol, timeframe)` | `(symbol, FutOptCandlesParams?)` |
| `futopt.intraday.trades` | `(symbol)` | `(symbol, FutOptTradesParams?)` |
| `futopt.historical.candles` | `(symbol, from?, to?, timeframe?, afterHours, contractMonth?, fields?, sort?)` | `(symbol, FutOptHistoricalCandlesParams?)` |
| `futopt.historical.daily` | `(symbol, date?, afterHours)` | `(symbol, FutOptDailyParams?)` |
| WebSocket `subscribe` / `unsubscribe` | `(channel, symbol, afterHours?)` | `(channel, symbols, SubscribeOptions?)` |

`stock.historical.stats(symbol)` and `unsubscribeIds(ids)` are unchanged.
The `movers` thresholds and the technical periods used to be optional and
the server answered 400 without them; they are required now, which is what
they were.

- **C#** (`FugleMarketData`): the wrapper methods take the FubonNeo request
  models (`FugleMarketData.QueryModels.*`, #203) as an optional trailing
  argument; the generated records are what the raw
  `uniffi.marketdata_uniffi` clients take.

  ```csharp
  using FugleMarketData.QueryModels;
  using FugleMarketData.QueryModels.Stock.Intraday;
  using FugleMarketData.QueryModels.FuOpt;
  using FuOptIntraday = FugleMarketData.QueryModels.FuOpt.Intraday;
  // Before
  var candles = await client.Stock.Intraday.GetCandlesAsync("2330", "1");
  var trades = await client.Stock.Intraday.GetTradesAsync("2330");
  var quote = await client.FutOpt.Intraday.GetQuoteAsync("TXFE6", afterHours: true);
  await ws.SubscribeAsync("trades", "2330");
  // After
  var candles = await client.Stock.Intraday.GetCandlesAsync("2330");          // server default timeframe
  var trades = await client.Stock.Intraday.GetTradesAsync("2330",
      new TradeRequest(TickerType.OddLot, limit: 5));
  var quote = await client.FutOpt.Intraday.GetQuoteAsync("TXFE6",
      new FuOptIntraday.TickerVolumeRequest(TradeSession.AfterHours));
  await ws.SubscribeAsync("trades", "2330");                                 // kept
  await ws.SubscribeAsync("trades", new[] { "2330", "2317" });               // one frame
  await ws.SubscribeAsync("trades", new[] { "2330" }, new SubscribeOptions { IntradayOddLot = true });
  ```

  `WebSocketClientOptions` gains `Versions` (`WebsocketVersionOptions { Stock, FutOpt }`),
  which used to be fixed at the latest version.

- **Go**: the generated methods are called directly; `nil` is the empty
  record. Four helpers take the address of a literal: `Bool`, `String`,
  `Uint32`, `Float64`. `StreamingClient` keeps `Subscribe(channel, symbol,
  opts...)` and gains `SubscribeMany` / `UnsubscribeMany` and
  `WithIntradayOddLot`.

  ```go
  // Before
  candles, err := client.Stock().Intraday().GetCandles("2330", "1")
  trades, err := client.Stock().Intraday().GetTrades("2330")
  // After
  candles, err := client.Stock().Intraday().GetCandles("2330", nil)
  trades, err := client.Stock().Intraday().GetTrades("2330",
      &marketdata.StockTradesParams{OddLot: marketdata.Bool(true), Limit: marketdata.Uint32(5)})
  err = stream.SubscribeMany("trades", []string{"2330", "2317"})
  err = stream.Subscribe("trades", "2330", marketdata.WithIntradayOddLot(true))
  ```

- **Java**: the wrapper methods take the generated records; `null` is the
  empty record, and the one-argument overloads (`getQuote(symbol)` and the
  like) stay. `FugleWebSocketClient` keeps `subscribe(channel, symbol[,
  afterHours])` and gains `subscribe(channel, List<String> symbols[,
  SubscribeOptions])`.

  ```java
  // Before
  String candles = client.stock().intraday().getCandles("2330", "1");
  // After
  String candles = client.stock().intraday().getCandles("2330");
  String trades = client.stock().intraday().getTrades("2330",
      new StockTradesParams(true, null, 5, null, null));
  ws.subscribe("trades", List.of("2330", "2317"));
  ws.subscribe("trades", List.of("2330"), new SubscribeOptions(null, true));
  ```

- **C++**: the records are aggregates with every field `std::optional`, so
  designated initialisers work; pass `std::nullopt` for no record.

  ```cpp
  // Before
  intraday->candles_sync("2330", "1");
  ws->subscribe_sync("trades", "2330");
  // After
  intraday->candles_sync("2330", std::nullopt);
  intraday->trades_sync("2330", StockTradesParams{.odd_lot = true, .limit = 5});
  ws->subscribe_sync("trades", {"2330", "2317"}, std::nullopt);
  ws->subscribe_sync("trades", {"2330"}, SubscribeOptions{.intraday_odd_lot = true});
  ```

- **WebSocket, all four**: `subscribe` / `unsubscribe` take a list of
  symbols. One symbol is sent as `symbol`, as before; several as `symbols`
  in one frame, and each becomes its own subscription (`unsubscribe` with
  the same list removes them all; `unsubscribeIds` still works per id). An
  empty list is 1005. `SubscribeOptions.afterHours` is FutOpt only and
  `intradayOddLot` is Stock only; either on the other endpoint, to any
  value, is 1005 — `afterHours` on Stock was already.

## 21. Python: optional parameters are keyword-only

The REST methods' `#[pyo3(signature)]` accepted every parameter
positionally, while `__init__.pyi` declared 46 of them keyword-only after
`symbol` — and the sync siblings partly the other way round (#217). So
`candles("2330", "5")` ran but mypy / pyright rejected it, and
`historical.candles("2330", "2024-01-01", "D")` sent `"D"` as `to` without
an error: seven optional strings in a row, and a slot off is a wrong date
range, not a rejected request.

Every REST method now follows the rule the uniffi bindings got in §20: the
path parameter and the parameters core's `rest::params` table marks
required are positional, everything else is keyword-only. Positional beyond
that is a `TypeError` naming the limit (`candles() takes 1 positional
arguments but 2 were given`), raised before any request.

| Methods | Positional | Keyword-only |
|---|---|---|
| `stock.intraday` `quote` / `ticker` / `candles` / `trades` / `volumes`, `futopt.intraday` same five | `symbol` | `odd_lot`, `timeframe`, `sort`, `offset`, `limit`, `is_trial`, `after_hours` |
| `stock.intraday.tickers`, `futopt.intraday.products` / `tickers` | `type` | every filter |
| `stock.historical.candles`, `futopt.historical.candles` / `daily`, the four `stock.ownership` methods | `symbol` | `from_date`, `to_date`, `timeframe`, `fields`, `sort`, `adjusted`, `date`, … |
| `stock.snapshot.quotes` | `market` | `type_filter` |
| `stock.snapshot.movers` | `market`, `direction`, `change` | `type_filter`, `gt`, `gte`, `lt`, `lte`, `eq` |
| `stock.snapshot.actives` | `market`, `trade` | `type_filter` |
| `stock.technical.sma` / `rsi` / `bb` | `symbol`, `period` | `from_date`, `to_date`, `timeframe` |
| `stock.technical.kdj` | `symbol`, `r_period`, `k_period`, `d_period` | same |
| `stock.technical.macd` | `symbol`, `fast`, `slow`, `signal` | same |
| `stock.corporate_actions` (three) | — | `start_date`, `end_date`, `exchange`, `sort` |

The required query parameters keep their `None` default, as before: the
server, not the SDK, reports a missing one. On the technical methods they
move ahead of the date range in the signature, so `sma("2330", 5)` is the
period and `sma("2330", "2024-01-01")` is a `TypeError` (`period` is an
`int`), not a silent date-as-period.

```python
# Before — ran, but the stub disagreed and the slot order was the contract
candles = client.stock.intraday.candles("2330", "5")
hist = client.stock.historical.candles("2330", "2024-01-01", "2024-01-31", "D")
sma = client.stock.technical.sma("2330", "2024-01-01", "2024-01-31", "D", 5)
movers = client.stock.snapshot.movers("TSE", "up", "percent", "COMMONSTOCK")

# After — TypeError for each of the above; write
candles = client.stock.intraday.candles("2330", timeframe="5")
hist = client.stock.historical.candles("2330", from_date="2024-01-01", to_date="2024-01-31", timeframe="D")
sma = client.stock.technical.sma("2330", 5, from_date="2024-01-01", to_date="2024-01-31", timeframe="D")
movers = client.stock.snapshot.movers("TSE", "up", "percent", type_filter="COMMONSTOCK")
```

Unchanged: `symbol=` as a keyword, every keyword call, the API's own
spellings and `from_` through `**_extra` (§17), and the async siblings,
which have the same signatures. Coming from 2.x, where every method was
`candles(**params)`, nothing was positional to begin with.

## 22. Rust: `WebSocketMessage.data` is a method

Reading a frame used to build its `data` payload into a `serde_json::Value`
tree — a map per object, in key order — and the reader thread then spent
about as long dropping it (#214). Nothing on the market-data path read it:
Node, Python and the uniffi bindings hand you `raw`, and core looks inside
`data` only for control frames. `WebSocketMessage` now records where `data`
sits in `raw` and parses it the first time you ask (#236).

```rust,ignore
// Before
if let Some(data) = msg.data.as_ref() { /* … */ }
let msg = WebSocketMessage { event: "data".into(), data: None, /* … */ raw };

// After
if let Some(data) = msg.data() { /* … */ }      // Option<&Value>, parsed once, cached
let text = msg.data_json();                     // Option<Cow<str>>: the payload as sent, not parsed
let msg = WebSocketMessage::parse(&frame)?;     // replaces struct literals
```

- `data` is private, so struct literals no longer compile; build a message
  with `WebSocketMessage::parse`, which also fills `raw`.
- Calling `data()` on every market-data message is about 20% slower than
  rc.6, since the frame is scanned once to find `data` and again to parse
  it. On a hot path, read `data_json()` or `raw` instead.
- `serde_json::from_str` / `from_value` still work. A message built that
  way parses `data` up front and has an empty `raw`, as before;
  `data_json()` then returns the value re-serialized.
- `raw` stays public. If you overwrite it, `data()` and `data_json()` of a
  parsed message read from the new string: expect `None`, never a panic.
- `Serialize` writes the same JSON as before. `Debug` prints `data` as the
  text of the frame until `data()` has parsed it.

## Fields you could not reach before

Worth checking whether these change anything for you:

| Field | Where | Note |
|---|---|---|
| `referencePrice` | stock quote | The basis for `change`, `changePercent` and the limit prices — not `previousClose` |
| `serial` | stock quote | The quote's own sequence number |
| `lastTrade`, `lastTrial`, `tradingHalt` | stock quote, Python | Dropped entirely by the old dict builder |
| `total.tradeVolumeAtBid` / `AtAsk` / `time` | stock quote, Python | Same |
| `isOpen`, `isClose`, `isContinuous`, `tradingHalt`, `priceLimits`, `lastTrial`, `serial`, `market` | futopt quote, C#/Go/C++/Java | 11 fields the mirror never copied |
| `total.tradeValue` | futopt quote, C#/Go/C++/Java | The mirror copied 3 of 8 fields |
