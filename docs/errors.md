# Error Reference

Every error the SDK reports — a failed REST call, a WebSocket `error` event, a
rejected `connect()` — carries the same fields in every language. They are
defined once in the Rust core (`marketdata_core::ErrorInfo`, built with
`MarketDataError::info()`) and each binding only renames them to its own
casing.

- [Fields](#fields)
- [Field names per language](#field-names-per-language)
- [`source_kind` values](#source_kind-values)
- [Error codes](#error-codes)
- [Callback failures](#callback-failures)
- [Examples](#examples)

---

## Fields

| Field | Type | Present | Meaning |
|---|---|---|---|
| `code` | integer | always | Numeric error code, see [Error codes](#error-codes). Values never change once released. |
| `source_kind` | string / enum | always | Category of the failure, see [`source_kind` values](#source_kind-values). |
| `message` | string | always | Human-readable message. Meant for logs, not for branching — branch on `code` / `source_kind`. |
| `status` | integer or null | HTTP errors | HTTP status of a REST response, or of a rejected WebSocket upgrade. |
| `body` | string or null | REST HTTP errors | Response body exactly as the server sent it; null if it could not be read as text. |
| `request_id` | string or null | REST HTTP errors | Value of the `x-request-id` response header. The Fugle API does not send one today, so this is null until it does; use `headers` for tracing. |
| `headers` | string map | REST HTTP errors (empty otherwise) | Response headers, names lowercased, except `set-cookie`; a repeated header is joined with `", "`. Useful ones include `retry-after` and `x-ratelimit-*`. |

`set-cookie` is never kept, but other headers and `body` come from the
server as-is: filter them before writing an error to logs.

The SDK does not tell you whether to retry. Decide from `source_kind`,
`status` and `headers` (for example `retry-after`).

One case is handled before you see it: when the server closes the
connection of a REST GET before a full response header arrives — typically
a reused keep-alive connection that hit the server's idle timeout — the SDK
sends the GET once more. If that fails too, you get its `ConnectionError`.
This resend is not configurable and is separate from the Rust
`RetryPolicy`; no other error is resent.

The SDK recognises that close by the I/O error it produces: unexpected EOF,
connection reset, connection aborted (how Windows reports it), broken pipe,
and invalid input. The last one is macOS: ureq sets the socket's write
timeout before sending the request and its read timeout before reading the
response (`maybe_update_timeout` in ureq's `transport/tcp.rs`), and macOS
returns `EINVAL` for that call on a socket the peer has already reset. The
request may or may not have been sent by then; resending is safe either way
because GET is idempotent and is resent only once.

For a REST HTTP error the message is `API error (status <status>): <body>`,
or `Authentication error: <body>` for 401 / 403.

## Field names per language

| Core (`ErrorInfo`) | Node.js | Python | C# | Go | Java |
|---|---|---|---|---|---|
| `code` | `err.code` | `e.code` | `ex.GetInfo().code` | `info.Code` | `e.getCode()` |
| `source_kind` | `err.sourceKind` | `e.source_kind` | `ex.GetInfo().sourceKind` | `info.SourceKind` | `e.getSourceKind()` |
| `message` | `err.message` | `e.message` | `ex.GetInfo().message` | `info.Message` | `e.getMessage()` |
| `status` | `err.status` | `e.status` | `ex.GetInfo().status` | `info.Status` | `e.getStatus()` |
| `body` | `err.body` | `e.body` | `ex.GetInfo().body` | `info.Body` | `e.getBody()` |
| `request_id` | `err.requestId` | `e.request_id` | `ex.GetInfo().requestId` | `info.RequestId` | `e.getRequestId()` |
| `headers` | `err.headers` | `e.headers` | `ex.GetInfo().headers` | `info.Headers` | `e.getHeaders()` |

Where the fields live:

- **Node.js** — properties of the `Error` thrown by constructors, rejected by
  REST methods and `connect()`, and passed to the WebSocket `error` event
  (TypeScript: `MarketDataError`). Absent values are `null`. The `error`
  event reporting a failed listener (code 3004) also has `event`, `count`
  and `cause`, see [Callback failures](#callback-failures).
- **Python** — attributes of every `MarketDataError` subclass. `args` stays
  `(message, code)`. Aliases kept from the 2.4.1 `FugleAPIError`:
  `status_code` (= `status`) and `response_text` (= `body`). The WebSocket
  `error` callback receives a `WebSocketError` whose `args` stay
  `(message, code)`; one reporting a failed callback (code 3004) also has
  `event`, `count` and `__cause__`.
- **C#** — `GetInfo()` (namespace `FugleMarketData`) on any
  `MarketDataException` returns the `ErrorInfo` record, whose properties keep
  the generated camelCase names; each exception variant also has it as
  `info`. `IWebSocketListener.OnError` receives an `ErrorInfo`.
- **Go** — `ErrorInfoOf(err)` returns `(ErrorInfo, bool)` for an error
  returned by the SDK or a `*StreamError` read from `StreamingClient.Errors()`.
  Optional fields are pointers (`Status *uint16`, `Body *string`,
  `RequestId *string`); the generated variants also have it as `Info`.
- **Java** — getters on `FugleException`; `getInfo()` returns the whole
  `ErrorInfo` record, `getStatus()` an `Integer`. The listener's `onError`
  receives an `ErrorInfo`; pull mode's error queue still holds the message
  string.
- **C++** — each exception variant has an `info` member
  (`marketdata_uniffi::ErrorInfo`); `on_error` receives a
  `const ErrorInfo&`.

## `source_kind` values

| Value | Meaning | Typical errors |
|---|---|---|
| `network` | Transport failure or server outage | connection refused/reset, timeouts, heartbeat timeout, HTTP 5xx, WebSocket I/O errors |
| `protocol` | Protocol violation or SDK bug | malformed WebSocket frames, unclassified WebSocket failures, a binding thread panic |
| `auth` | Credentials rejected | HTTP 401 / 403, WebSocket authentication failure, TLS certificate errors |
| `rate_limit` | Too many requests | HTTP 429 |
| `client` | The request or the caller is wrong | invalid symbol / parameter / configuration, other HTTP 4xx, client already closed, unparsable response |

Rust (`ErrorKind`) and C# / Go / Java (`ErrorSourceKind`) use enums with the
same five members.

## Error codes

Ranges: `1xxx` caller input and parsing, `2xxx` connection, authentication
and API, `3xxx` timeouts and WebSocket, `9xxx` unexpected. New codes are
added to this table within their range; existing values never change.
Rust constants live in `marketdata_core::error_code`.

| Code | Constant | Core variant | `source_kind` | Raised when |
|---|---|---|---|---|
| 1001 | `INVALID_SYMBOL` | `InvalidSymbol` | `client` | A symbol fails validation. |
| 1002 | `DESERIALIZATION` | `DeserializationError` | `client` | A REST response or WebSocket frame cannot be parsed (WebSocket: `error` event, connection stays up). |
| 1003 | `RUNTIME` | `RuntimeError` | `client` | An internal runtime operation fails. |
| 1004 | `CONFIG` | `ConfigError` | `client` | Invalid client configuration (for example a `baseUrl` that includes the version, not exactly one non-empty credential, or a reconnect / health check value below its floor). Python raises its `ConfigError` class, not `ValueError`. |
| 1005 | `INVALID_PARAMETER` | `InvalidParameter` | `client` | A request parameter is missing or invalid: an unknown WebSocket channel name in `subscribe()`, or (Node) a key the REST endpoint does not take in the object form. Python raises `TypeError` for the latter. |
| 2001 | `CONNECTION` | `ConnectionError` | `network` | A REST request or WebSocket connection cannot reach the server, or a WebSocket command is sent while not connected. |
| 2002 | `AUTH` | `AuthError` | `auth` | HTTP 401 / 403, or WebSocket authentication failed. |
| 2003 | `API` | `ApiError` | by status: 429 `rate_limit`, 5xx `network`, else `client` | The API answered with any other error status. |
| 2010 | `CLIENT_CLOSED` | `ClientClosed`, `ConnectionAborted` | `client` | `ClientClosed`: the client was already closed. `ConnectionAborted` (message `Connection aborted: …`): `connect()` given up because `disconnect()` was called before the connection was established (Rust async client, Node, Python, and the C#, Go, Java and C++ bindings, which report it as the `ClientClosed` variant). |
| 2011 | `ALREADY_CONNECTED` | `AlreadyConnected` | `client` | WebSocket `connect()` called while connected, connecting or auto-reconnecting. |
| 3001 | `TIMEOUT` | `TimeoutError` | `network` | A request or the WebSocket connect timed out. |
| 3002 | `WEBSOCKET` | `WebSocketError` | by kind: I/O `network`, TLS `auth`, upgrade HTTP status as for 2003 (401/403 `auth`), else `protocol` | A WebSocket connect, read or write fails. |
| 3003 | `HEARTBEAT_TIMEOUT` | `HeartbeatTimeout` | `network` | No inbound WebSocket frame within the heartbeat window. |
| 3004 | `CALLBACK_FAILED` | — | `client` | A WebSocket callback / listener raised an exception, or (Node) the Promise it returned rejected. See [Callback failures](#callback-failures). |
| 3005 | `RECONNECT_FAILED` | — | `network` | Automatic reconnection gave up after its last attempt (Node / Python `error`; C#, Go, Java and C++ have a dedicated reconnect-failed callback). |
| 9999 | `OTHER` | `Other` | `client` | Unexpected error. |
| -1 | `THREAD_PANIC` | — | `protocol` | Node / Python: a WebSocket worker thread panicked. |

## Callback failures

An exception raised by your WebSocket callback or listener never crashes the
process and never closes the connection: later events and messages are still
delivered. It is not silent either:

- It is reported as an error with code **3004** (`CALLBACK_FAILED`,
  `source_kind` `client`) to your error callback — `error` in Node and
  Python, `OnError` / `onError` / `on_error` elsewhere.
- Reports are throttled, like `messagesDropped`: the first failure is
  reported at once; later ones at most once per second, each report counting
  the failures since the previous one. Failures after the last report are
  not reported on their own when they stop.
- If there is no error callback, or the error callback raises too, the
  failure is printed instead and nothing is re-reported (no recursion).

| Language | Report | Original exception | Printed with |
|---|---|---|---|
| Node.js | `error` event: `err.event` (event name, e.g. `'message'`), `err.count` | `err.cause` | `console.error` |
| Python | `error` callback: `WebSocketError` with `e.event` (e.g. `"message"`), `e.count` | `e.__cause__` | `sys.unraisablehook` |
| C# | `OnError(ErrorInfo)` | type, message, method and count in `message` | `Console.Error` |
| Java | `onError(ErrorInfo)` | type, message, method and count in `getMessage()` | `java.util.logging` |
| C++ | `on_error(ErrorInfo)` | `what()`, method and count in `message` | stderr |

Go delivers events on channels, so there is no callback to fail.

Node: a listener that returns a Promise is not awaited; a rejection is
reported like a throw. Python: `async def` callbacks are rejected by `on()`
with `TypeError` — use `async for` over `messages()` instead — and a callback
returning a coroutine is reported as a failure. A `BaseException` that is not
an `Exception` (`KeyboardInterrupt`, `SystemExit`) raised in a callback runs
on the SDK's thread and cannot reach your code: it is only printed through
`sys.unraisablehook`.

## Examples

**Node.js**

```js
try {
  await client.stock.intraday.quote('2330');
} catch (err) {
  if (err.sourceKind === 'rate_limit') {
    console.warn('throttled; retry after', err.headers['retry-after']);
  } else if (err.status !== null) {
    console.error(err.code, err.status, err.body);
  } else {
    console.error(err.code, err.message);
  }
}
```

**Python**

```python
from fugle_marketdata import MarketDataError

try:
    client.stock.intraday.quote("2330")
except MarketDataError as e:
    print(e.code, e.source_kind, e.status, e.body, e.headers.get("retry-after"))
```

**C#**

```csharp
try
{
    var quote = await client.Stock.Intraday.GetQuoteAsync("2330");
}
catch (MarketDataException ex)
{
    var info = ex.GetInfo();
    Console.WriteLine($"{info.code} {info.sourceKind} {info.status} {info.body}");
}
```

**Go**

```go
quote, err := client.Stock().Intraday().GetQuote("2330")
if info, ok := marketdata.ErrorInfoOf(err); ok { // import marketdata "github.com/fugle-dev/fugle-marketdata-go"
    log.Println(info.Code, info.SourceKind, info.Status, info.Body)
}
```

**Java**

```java
try {
    client.stock().intraday().getQuote("2330");
} catch (FugleException e) {
    System.out.println(e.getCode() + " " + e.getSourceKind() + " " + e.getStatus() + " " + e.getBody());
}
```

**Rust**

```rust
if let Err(err) = client.stock().intraday().quote().symbol("2330").send() {
    let info = err.info();
    eprintln!("{} {} {:?} {:?}", info.code, info.source_kind, info.status, info.body);
}
```
