# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **All languages**: one set of error fields everywhere, defined in core
  (#81): `code`, `source_kind`, `message`, `status`, `body`, `request_id`,
  `headers`. REST errors now keep the HTTP status, the raw response body and
  the response headers (401 / 403 included). Names per language and every
  error code are in [docs/errors.md](docs/errors.md).
  - **Rust**: `ErrorInfo` via `MarketDataError::info()`, `HttpErrorContext`,
    `error_code` constants, `ErrorKind::as_str()`.
  - **Node**: errors thrown or rejected by the SDK and the WebSocket `error`
    event carry `code`, `sourceKind`, `status`, `body`, `requestId`,
    `headers` (TypeScript `MarketDataError`).
  - **Python**: exceptions gain `code`, `source_kind`, `status`, `body`,
    `request_id`, `headers`; `status_code` / `response_text` stay as aliases
    and `response_text` is no longer always `None`.
  - **C#, Go, Java, C++**: `ErrorInfo` / `ErrorSourceKind` records; C#
    `MarketDataException.GetInfo()`, Go `ErrorInfoOf(err)`, Java
    `FugleException` getters.
- **Rust**: `aio::WebSocketClient::state_handle()` returns a
  `ConnectionStateHandle` that reads the client's `ConnectionState` and stays
  readable after the client is dropped, like `messages_dropped_handle()` (#67).
- **Node**: every REST method accepts the legacy `@fugle/marketdata` 1.x
  object param, e.g. `stock.intraday.trades({ symbol: '2330', limit: 5 })`.
  The path param (`symbol` / `market`) goes into the path and every other key
  is forwarded verbatim as a query param, so params the positional form lacks
  (`type=oddlot`, `limit`, `sort`, `session=afterhours`, `product`, movers'
  `gt`/`lt`, …) are reachable. Previously only `quote` and `ownership.*`
  accepted an object; every other method threw `Failed to convert JavaScript
  value`. `Rest*Params` types are exported for each method.
- **Node**: `futopt.historical.*` object params accept the product code as
  `product` (the API's own name) as well as `symbol`.
- **C#, Go, Java**: `stock.ownership.*` gain the blocking `*_sync` variants
  (`etf_holdings_sync`, `institutional_trades_sync`, `director_holdings_sync`,
  `tdcc_distribution_sync`) that only the C++ binding had, so ownership offers
  both async and sync like every other REST client (#32).
- **Java**: `client.stock().ownership()` returns a wrapper with
  `getEtfHoldings` / `getInstitutionalTrades` / `getDirectorHoldings` /
  `getTdccDistribution` (blocking) and their `*Async` counterparts, throwing
  `FugleException` like the other wrappers. Previously Java callers had to use
  the generated `StockOwnershipClient` directly (#37).
- **Core**: `ConnectionConfigBuilder::message_overflow(MessageOverflow)`
  chooses what happens while the inbound message queue is full:
  `DropNewest` (default) keeps at most `message_buffer` messages and drops
  new ones; `Unbounded` never drops and lets the queue grow. Both the sync
  and async clients honour it (#46).
- **Core**: `ConnectionEvent::MessagesDropped { dropped, total }` reports
  dropped messages: the first drop on a connection at once, then at most once
  per second, and any remainder right before that connection's
  `Disconnected`. `total` counts from the start of the connection. Previously
  drops were only counted (#46).
- **Core**: `messages_dropped_handle()` on both clients returns a
  `MessagesDroppedHandle` that reads `messages_dropped_total()` and stays
  readable after the client is dropped (#46).
- **Python**: `WebSocketClient(message_overflow="drop_newest"|"unbounded",
  message_buffer=...)`, the `messages_dropped` callback `(dropped, total)` and
  `messages_dropped_total()` on the stock / futopt clients (#46).
- **Node**: `messageOverflow: 'dropNewest' | 'unbounded'` and `messageBuffer`
  options, the `messagesDropped` event `{ dropped, total }` and the
  `messagesDroppedTotal` getter (#46). Frames waiting for a `message` listener
  count against `messageBuffer`: once that many are pending, the SDK stops
  handing over more, so a slow listener leads to drops (per
  `messageOverflow`) instead of an ever-growing queue in Node. Events behind
  those frames wait with them, keeping their order; a listener blocked long
  enough for more than 1024 events to back up loses events as well.
- **C#, Go, Java, C++**: `WebSocketClient::new_with_options(...)` takes a
  `MessageQueueConfigRecord { overflow, buffer }`; `WebSocketListener` gains
  `on_messages_dropped(count)` and the client `messages_dropped_total()`. The
  C#, Go and Java wrappers expose them as options (#46). Go's
  `StreamingClient` reports drops on `Errors()` (skipping a report when
  `Errors()` is full rather than holding up `Messages()`), Java's pull mode
  on the error queue.

### Changed

- **Rust**: when a connection is lost and no reconnect follows,
  `ConnectionState::Closed` carries the `code`, `reason` and `intent` of the
  `Disconnected` it came with, e.g. `Closed { code: Some(4001), reason: "bye",
  intent: Server }` for a server close. It used to be `reason: "Non-retriable
  error"` with `intent: Network` whatever the cause. While a reconnect is
  about to start, the state is `Disconnected` until `Reconnecting` (#86).

- **C#**: the blocking `Stock.Ownership.Get*` methods call the native `*Sync`
  exports directly instead of running the async call via `Task.Run` (#37).
- **Node**: `stock.intraday.candles` and `futopt.intraday.candles` no longer
  require `timeframe`; the server defaults to `1`, as in 1.x.
- **Core**: `aio::WebSocketClient::stream_receiver()` (formerly `messages()`)
  needs no tokio runtime context. It can be called from any thread, before or
  after `connect()`, instead of panicking with `there is no reactor running`
  (#26). The receiver reads the client's stream directly; no bridge task runs
  (#46).
- **Core**: messages read through the blocking receiver are capped at
  `message_buffer` (default 4096) like every other consumer. The bridge behind
  the former `messages()` queued without limit, so a slow consumer (a Node,
  Python or UniFFI callback) grew memory without bound instead of dropping.
  Use `MessageOverflow::Unbounded` to keep the old behaviour (#46).
- **All languages**: messages and connection events arrive in one order.
  Every message of a connection, including the server's `authenticated`
  frame, comes after that connection's `authenticated` event and before its
  `disconnect`; frames that arrive after the connection was reported closed
  are discarded (not counted as dropped). Messages and events used to travel
  on separate channels, so e.g. the last messages could follow `disconnect`
  (#68).
- **Node, Python, C#, Go, C++, Java**: frames read while authenticating a
  connection that is then rejected (the server's `error` frame) no longer
  reach the `message` callback, including on a reconnect, where they used to
  (#68).
- **Python**: whether a `message` callback is registered is checked for each
  message. A callback registered after `connect()` now receives messages; one
  registered before `connect()` used to be the only way to get them through
  callbacks. Without one, messages wait for a `messages()` iterator: at most
  `message_buffer` of them, and while that many are unread, lifecycle
  callbacks (`disconnect`, `reconnect`, …) that follow them wait until the
  iterator reads or `disconnect()` is called. The iterator releases the GIL
  while it waits. Events and messages share one background thread, so a panic
  on it (#25) now stops lifecycle callbacks as well as messages (#68).
- **Node, Python**: messages that arrive while `disconnect()` closes the
  connection, before its `disconnect` event, still reach the `message`
  listeners. They used to be dropped from the moment `disconnect()` was called
  (#68). C#, Go, C++ and Java still stop `OnMessage` at `disconnect()`.
- **Core**: `messages_dropped_total()` counts the current connection's drops:
  it restarts from zero when `connect()` or a reconnect attempt opens a new
  connection, and still reads the last connection's count after
  `disconnect()`. It used to count from client construction. The `metrics`
  counter is unchanged and keeps counting across connections (#46).
- **Core**: while the message queue is full, the auth handshake of a
  reconnect drops the server's `authenticated` frame from the stream instead
  of waiting for room, which could stall the reconnect until its 10 s
  auth timeout (#46).
- **All languages**: a WebSocket connection emits `Disconnected` (the
  `disconnect` callback in the bindings) at most once. A server Close or
  transport error that raced `disconnect()` could emit it twice, once with
  `Server` / `Network` intent and once with `Client`. As a consequence,
  `disconnect()` / `force_close()` on a connection already reported lost, or
  a second `disconnect()`, no longer emits another `Disconnected`; the state
  still becomes `Closed`. A successful reconnect starts a new connection that
  reports its own close (#41).
- **Core**: `aio::WebSocketClient::connect()` while the client is connected
  (or auto-reconnecting) is a no-op returning `Ok(())`, and `reconnect()`
  stops the running connection before opening a new one, as the sync client
  already did. Previously both left the old dispatch task running, whose
  later close was reported as the new connection's `Disconnected` (#41).
- **Core**: a heartbeat timeout emits `HeartbeatTimeout` followed by exactly
  one `Disconnected { intent: Network }` for the connection. Previously it
  emitted no `Disconnected`, so the Node, Python and UniFFI bindings
  synthesized a `disconnect` of their own, and a later `disconnect()`
  reported the same connection closed a second time (#47).
- **Core**: `ReconnectFailed` is emitted only after at least one reconnect
  attempt. A close the reconnect policy does not retry (reconnect disabled,
  code 1000 or 4xxx) is reported solely as
  `Disconnected { will_reconnect: false }`; previously it also emitted
  `ReconnectFailed { attempts: 0 }`, which Node and Python surfaced as an
  `error` "Reconnection failed after 0 attempts" (#55).
- **Core**: `aio::WebSocketClient::connect()` emits `Error` and returns the
  client to `Disconnected` when sending the auth frame fails; previously it
  returned the error silently and left the state at `Authenticating`. The sync
  client emits `Disconnected { intent: Network }` after a failed write, as it
  already did after a failed read (#55).
- **Core**: `disconnect()` while the client is waiting to reconnect stops the
  reconnect: no further `Connecting` / `Connected` / `Authenticated` is
  emitted and the state stays `Closed { intent: Client }`. Previously the
  async client finished the pending attempt (reconnecting and resubscribing)
  until its drain timeout aborted it, and both clients could report an attempt
  already in progress (#55).
- **Python**: WebSocket `connect` fires from core's `Connected` event, once
  the transport is open and before authentication, so it also fires when the
  server then rejects the credentials. Previously the binding called it itself
  after `connect()` had authenticated (#56).
- **Python**: `disconnect()` / `disconnect_async()` return only after the
  connection-event callbacks for that connection (`disconnect`, a final
  `error`, …) have run, and a failed `connect()` / `connect_async()` raises
  only after `unauthenticated` / `error` have run. Previously they could fire
  after the call returned. `disconnect()` therefore also waits for calls still
  in flight on the same client from other threads (`subscribe()`, `ping()`, …)
  to return (#54).

### Breaking

- **Node**: error messages no longer start with `[code]` (REST rejections,
  constructor errors, `connect()` rejections), matching the WebSocket `error`
  event; `err.code` is a number on every SDK error (REST errors used to carry
  the string `"GenericFailure"`). Check `err.code` instead of the message
  (#81). See [MIGRATION-0.9.md](MIGRATION-0.9.md#13-errors-one-set-of-fields-in-every-language).
- **Rust**: `MarketDataError::ApiError` and `MarketDataError::AuthError` gain
  `http: Option<Box<HttpErrorContext>>`; `ConnectionEvent::Error { message,
  code }` becomes `ConnectionEvent::Error(ErrorInfo)` (#81).
- **C#, Go, Java, C++**: every `MarketDataError` variant gains
  `info: ErrorInfo` (`ClientClosed` included) and `WebSocketListener::on_error`
  receives an `ErrorInfo` instead of a string (#81).
- **All languages**: WebSocket `error` events report the code matching the
  failure (#81): an unparsable frame `1002` (was `2003`), a failed read
  `3002` (was `2001`), a failed write on the Rust blocking client `3002`
  (was `2002`).
- **Python**: `messages()` iteration (`for` and `async for`) yields messages
  only and stops only once the connection is gone, raising `StopIteration` /
  `StopAsyncIteration` (#68). It no longer yields `None`, and no longer ends
  when no message arrives in time: with `timeout_ms`, the sync iterator used
  to end the loop at the first timeout and the async one yielded `None`.
  `messages(timeout_ms=...)` is deprecated and ignored, with a
  `DeprecationWarning`. A waiting sync iterator wakes every 100 ms to let
  Python handle signals, so Ctrl+C interrupts it. For periodic work while no
  data arrives, use `message` callbacks or `async for` alongside other tasks.
- **Rust**: one ordered stream of messages and connection events (#46, #68).
  See [MIGRATION-0.9.md](MIGRATION-0.9.md#11-rust-one-stream-for-messages-and-events).
  - `messages()`, `message_stream()`, `events()` and `state_events()` are
    replaced by `stream_receiver()` (blocking `StreamReceiver`, both clients)
    and `stream()` (async `ConnectionStream`, `aio` client), which yield
    `StreamItem::Message` / `StreamItem::Event`. `MessageReceiver` and the
    `websocket::message` module are removed.
  - `ConnectionStream` has `recv()`, `try_recv()` and `poll_recv()`, and
    implements `futures::Stream`.
  - `ConnectionConfig` gains the public field `message_overflow`.
  - `ConnectionEvent` and `StreamItem` are `#[non_exhaustive]`;
    `ConnectionEvent` gains `MessagesDropped`. Matches need a `_` arm.
- **Rust**: `ConnectionEvent` carries what bindings previously had to
  re-derive (#55). See [MIGRATION-0.9.md](MIGRATION-0.9.md#8-rust-connection-events).
  - `Authenticated` becomes `Authenticated { data }`, the server frame's
    `data` (`Value::Null` when absent).
  - `Unauthenticated { message }` gains `data`.
  - `Disconnected` gains `will_reconnect: bool`, `true` only when a
    `Reconnecting` follows.

  The event channel's delivery guarantees are documented on the
  `websocket::connection_event` module.
- **Node**: WebSocket listener arguments, `connect()` settlement and `ping()`
  match `@fugle/marketdata` 1.x (#23). Every event is now forwarded from
  core's connection events. See
  [MIGRATION.md](MIGRATION.md#12-node-websocket-events-match-1x).
  - `connect` fires with no arguments when the socket opens, before
    authentication (was `"connected"`, after authentication).
  - `authenticated` / `unauthenticated` receive the server's `data` object
    (was the string `"authenticated"` / the error message); a failed
    authentication fires `connect` then `unauthenticated`, no longer `error`.
  - `connect()` resolves with the server's `authenticated` `data`, and on
    rejected credentials rejects with the server's `data` object instead of
    `Error("[2002] ...")`. Other failures reject with an `Error` carrying
    the numeric `code` (no `[code]` prefix in the message).
  - `disconnect` receives `{ code, reason }` (`code` is `null` without a close
    code) instead of a JSON string; `reconnect` receives `{ attempt }`.
  - `error` receives an `Error` whose `message` has no `[code]` prefix, with
    the numeric code as `err.code` (absent for "Reconnection failed after N
    attempts"). Without an `error` listener errors are ignored.
  - `ping()` accepts an object sent verbatim as the frame's `data`, e.g.
    `ping({ state: 'x' })`; a string is still sent as `{ state }`.
- **Python**: WebSocket `authenticated` and `unauthenticated` callbacks
  receive the server frame's `data` — a `dict`, or `None` when the frame has
  none — instead of `{"event": "authenticated"}` and the rejection message
  string (#56). See
  [MIGRATION-0.9.md](MIGRATION-0.9.md#9-python-connection-callbacks).
- **C#, Go, C++, Java**: the WebSocket listener mirrors core's connection
  events (#57). Listener implementations must add the new methods; see
  [MIGRATION-0.9.md](MIGRATION-0.9.md#10-c-go-c-java-websocket-listener).
  - New `on_authenticated(data_json)` and `on_unauthenticated(data_json)`,
    where `data_json` is the server frame's `data` as a JSON string, or
    null / `None` when the frame has none. A credential rejection no longer
    reaches `on_error` as `"Unauthenticated: ..."`.
  - `on_disconnected()` becomes `on_disconnected(will_reconnect)`.
  - `on_connected` fires when the transport is up, before authentication,
    and again after every successful reconnect. Previously it fired once,
    after `connect()` had authenticated. Wait for `on_authenticated` before
    treating the connection as usable.
- **C#, Go, C++, Java**: `WebSocketListener` gains
  `on_messages_dropped(count)` (#46). Every listener implementation must add
  it, including C# `IWebSocketListener.OnMessagesDropped(ulong count)`, which
  has no default implementation; see
  [MIGRATION-0.9.md](MIGRATION-0.9.md#12-c-go-c-java-dropped-messages).
- **Java**: pull mode no longer drops messages silently when the
  `queueCapacity` queue is full. The client waits for `poll()` to make room
  (until `disconnect()` or `close()`), so messages you do not keep up with are
  dropped by the SDK per `messageOverflow`, counted in
  `messagesDroppedTotal()` and reported on the error queue (#46).

> **Release order:** the `futopt/historical` changes below follow
> fugle-realtime #727. Publish this release only after #727 is live in
> production; against the old server, `session=afterhours` is ignored and
> after-hours queries silently return the regular session.

- **All languages**: `futopt.historical.daily()` takes a single `date`
  (Python `date=`) in place of `from` / `to`. The endpoint returns one trading
  day with a row per contract month; the old range params were never honoured
  by the server. Python raises `TypeError` if `from_date` / `to_date` are
  still passed. Core no longer marks `daily()` deprecated: the "always 404"
  was caused by passing a contract code instead of a product code.
- **All languages**: `futopt.historical.candles()` gains `contractMonth` /
  `fields` / `sort` (Python `contract_month` / `fields` / `sort`). Appended
  to the JS positional signature and the Python kwargs; in the C# / Go / Java
  generated bindings the extra positional parameters change the signature.
- **All languages**: `futopt.historical.*` after-hours now sends
  `session=afterhours` instead of `afterHours=true`, which the server ignored.
- **Rust**: `FutOptHistoricalCandlesResponse`, `FutOptHistoricalCandle`,
  `FutOptDailyResponse` and `FutOptDailyData` follow the #727 response shape:
  `product` replaces `symbol`, `session` / `contractMonth` are added, candle
  prices are optional (the `fields` param selects them), and daily rows are
  per contract month with `openPrice` / `highPrice` / … / `volumeSpread`.
  `FutOptDailyResponse::highest_high()` and `lowest_low()` are removed: with
  one row per contract month they no longer describe a single series.
  The Node `.d.ts` types change the same way.
- **All languages**: `stock.technical.kdj()` takes `rPeriod` / `kPeriod` /
  `dPeriod` (Python `r_period` / `k_period` / `d_period`) in place of
  `period`. The API rejects `period` with HTTP 400, so the method could not
  succeed from any binding before.
- **Node**: the unused `SymbolParams` interface is no longer exported. No
  method took it; each method has its own `Rest*Params` type (#32).

### Fixed

- **All languages**: a `disconnect` listener or callback that reads the
  connection state already sees the close it reports: not connected, and
  closed when no reconnect follows (Node `isConnected` / `isClosed`, Python
  `is_connected()` / `is_closed()`, UniFFI `is_connected()`, Rust `state()`).
  Core queued `Disconnected` before recording the new state, so a listener
  could still read "connected". Both the async and the sync client now record
  the state first (#86).
- **Rust**: the blocking `WebSocketClient::reconnect()` works on a connected
  client. Stopping the old connection marked the client closed, so the
  `connect()` that followed always failed with `ClientClosed`. After
  reconnect attempts run out the client stays closed and `reconnect()` returns
  `ClientClosed`, as on the async client. `reconnect()` now also re-sends the
  stored subscriptions, which it used to skip (#82).
- **All languages**: a subscription that cannot be re-sent after a reconnect
  (automatic, or the async and blocking `reconnect()`) emits an `Error` event
  whose message names the subscription key; the other subscriptions are still
  sent. Failures used to be ignored, so the subscription looked restored but
  received nothing. `reconnect()` returns the first failure (#82).
- **Rust**: the blocking client's automatic reconnect no longer hangs with
  more than 64 stored subscriptions. Replaying them filled the write queue
  before anything drained it (#82).

- **Rust**: the blocking `WebSocketClient::force_close()` aborts the
  connection like the async one: it sends no Close frame and discards queued
  writes, and the socket is closed within about 200 ms. It used to run the
  graceful `disconnect()` sequence in the background, so the socket stayed
  open for up to about 2 seconds waiting for the server's Close ack (#79).
- **Python**: `async for msg in ws.stock.messages()` ends once the connection
  is gone. `__anext__` returned `None` for a closed channel instead of raising
  `StopAsyncIteration`, so the loop spun on `None` forever after
  `disconnect()` (#68).
- **All languages**: under a fast, continuous stream the async client no
  longer stalls message delivery. The network loop never yielded while the
  socket had data, so a consumer on the same tokio runtime (every binding's
  message bridge, or a `message_stream()` reader) got to run only every
  ~4096 messages, and most messages in between were silently dropped. Against
  a loopback server sending about 200,000 messages per second, Rust and Node
  received about 4,500 of them per second and Python about 8,700 (#46).
- **Node**: WebSocket listeners run in the order core emits the connection
  events (`connect` → `authenticated`, `disconnect` → `reconnect` → …), and
  `connect()` settles after the listener of the event that settles it, even
  when that event has no listener. Each listener had its own threadsafe
  function, and Node-API does not order calls across them; all events now go
  through one per connection. A listener is looked up when its event runs, so
  one registered after the event was queued (e.g. by an earlier listener, or
  while the JS thread was busy) receives it, and one replaced by `on()` while
  events are queued receives none of them (#62). `message` frames that arrive
  while no `message` listener is registered are dropped rather than queued, and
  are not delivered to a listener registered later. `message` keeps its place
  among the other events since #68.
- **Node, Python**: a panic on a WebSocket background thread no longer leaves
  the connection silently dead (#25). Node's worker and event threads report
  it as an `error` (`Error` with `code` -1, "WebSocket <thread> thread
  panicked: ..."), followed by `disconnect({ code: null, reason })` if the
  client was connected; `isConnected` turns false, a `connect()` whose
  authentication was not yet reported rejects with an `Error` with code -1, and the
  connection is closed so the process can exit. Python's event and message threads report it to the `error`
  callbacks as `WebSocketError(message, -1)`.
- **Core**: `aio::WebSocketClient::state()` and `is_closed_sync()` no longer
  panic with `Cannot start a runtime from within a runtime` when called on a
  tokio runtime thread, and no longer report a fake `Disconnected` /
  not-closed when called outside a runtime. They now read the state directly
  from any thread (#33).
- **All languages**: REST query values are form-urlencoded, in the typed
  builders and in the Node object form alike. A value was interpolated into
  the URL as-is, so one carrying `&`, `=`, `#`, `+` or a space split into
  extra params, was truncated, or made the URL invalid — e.g.
  `industry = "24&type=ETF"` overrode `type`. Ordinary values are
  unaffected; on the wire a comma is now `%2C`, a space `+` and `!` `%21`
  (`contractMonth=1%21`), all of which the server decodes (checked against
  api-dev).
- **All languages**: `stock.snapshot.{quotes,movers,actives}` percent-encode
  the `market` path segment, as every other path param already was.
- **Node**: calling WebSocket `connect()` again on a client that was already
  connected or still connecting started a second connection sharing
  `isConnected` and the listeners, so events fired twice and `disconnect()`
  stopped only one of them. It now rejects with an `Error` with code 2011
  (`Already connected; call disconnect() first`).
  Reconnecting after `disconnect()`, from a `disconnect` handler once no
  auto-reconnect follows, or after a failed auth still works, and `isClosed`
  turns back to false on the new connection. A `connect()` that is still
  authenticating when `disconnect()` is called now rejects with
  an `Error` with code 2010 (`Connection aborted: ...`) and fires no
  `authenticated` event, instead of resolving and connecting anyway (#44).

- Intraday `quote` / `ticker` / `candles` / `trades` / `volumes` sent
  `oddLot=true`, which the server ignores, so odd-lot requests silently
  returned board-lot data. They now send `type=oddlot`.
- Corporate actions `dividends` / `capital-changes` / `listing-applicants`
  sent `startDate` / `endDate` instead of `start_date` / `end_date`.
  `dividends` silently ignored the range; the other two failed with HTTP 400.
- **Node**: the WebSocket worker thread panicked right after `connect()`
  resolved (`there is no reactor running`), so no `message` ever arrived and a
  later `subscribe()` threw `Failed to send subscribe command`. Stock and
  futopt were both affected (#13).
- **Python**: the synchronous `connect()` on stock and futopt panicked the same
  way before connecting. `connect_async()` was unaffected.
- **C#, Go, C++, Java**: `on_disconnected` fired twice for one close — once
  from the binding itself and once from core's event — both on `disconnect()`
  and when the connection was lost. It now fires once per connection. A rejected `connect()` delivers its events: the binding only
  started forwarding them after a successful connect (#57).
- **C#, Go, C++, Java**: `is_connected()` could stay `true` after a connection
  that dropped right as `connect()` returned, until the next lifecycle event.
  It now reads core's connection state, so it is `false` as soon as the
  connection drops or starts reconnecting (#64).
- **Node**: `isConnected` stayed `true` while an auto-reconnect was in
  progress. `isConnected` / `isClosed` now read core's connection state
  instead of flags the binding kept, so `isConnected` is `false` from the
  moment a reconnect starts until it authenticates again (#67).
- **Go**: `StreamingClient` closed `Messages()` / `Errors()` on the first
  disconnect even when the client was about to reconnect, ending a
  `range` loop mid-session. The channels now close on the final disconnect or
  after reconnection gives up. Closing them while a callback was sending no
  longer panics with `send on closed channel` (#57).
- **Node**: a single `disconnect()` fired the `disconnect` event three times
  (`{"code":0,"reason":"Server initiated close"}`, `"disconnected"`,
  `{"code":1000,"reason":"Normal closure"}`). It now fires once, with the
  last payload; a server-initiated close likewise fires once (#22).
- **All languages**: a caller-initiated disconnect no longer emits an error
  (`[2001] WebSocket error: ...`) when the peer tears the transport down
  without a clean close (e.g. no TLS close_notify), nor a second
  `Disconnected { intent: Server }` for the peer's Close ack (#22).
- **Node**: a WebSocket client kept the process alive forever once any
  listener was registered with `on()`, even without connecting and after
  `disconnect()`, so scripts and Jest never exited. Listeners no longer hold
  the event loop; only an open connection does, as with 1.x. The process can
  exit after `disconnect()`, a failed `connect()`, or a server close / network
  loss with no reconnect left — in that last case `isConnected` now also turns
  `false` (#30).
- **Python**: the synchronous WebSocket methods (`connect`, `disconnect`,
  `subscribe`, `unsubscribe`, `subscriptions`, `ping`, `is_connected`,
  `is_closed`) on stock and futopt release the GIL while they wait.
  `disconnect()` could deadlock forever when called while `message` callbacks
  were being delivered, and `disconnect_async()` likewise; a server running on
  a Python thread of the same process could never answer `connect()`. These
  methods are no longer serialized by the GIL, so calls on one client from
  several threads can now interleave (#39).
- **Python**: after `stock.connect_async()` no connection-event callback
  except `connect` ever fired — `authenticated`, `disconnect`, `reconnect`
  and `error` were silently dropped. A rejected API key never fired
  `unauthenticated` on either `connect()` or `connect_async()` (#56).
- Long JSON decimals could decode to the neighbouring double, so a value
  such as `51.708947112827516` arrived as `51.70894711282752` — not the number
  `JSON.parse` gives for the same body. serde_json now uses its
  correctly-rounded float parser (`float_roundtrip`) in every binding.
- **C++**: every REST client except `stock.ownership` had no methods — the
  `cpp` feature stripped the async methods together with the sync ones sharing
  their `impl` block, so `stock.intraday`, `historical`, `snapshot`,
  `technical`, `corporateActions` and `futopt.intraday` / `historical` were
  unreachable. Their `*_sync` methods are now exported (#32).

## [Bindings 3.0.0-rc.2 / core 0.9.0-rc.1 / uniffi 0.2.0-rc.1] - 2026-09-16

Responses are now handed to the caller exactly as the server sent them.

Every language previously decoded the JSON into a hand-maintained struct and
re-encoded it on the way out. That cost data in both directions: a field the
struct did not declare was silently dropped, and a field the struct declared
but the server omitted was materialised with a default. A stock quote came
back missing `referencePrice` and `serial` while carrying a dozen invented
`false` flags.

The structs were maintained separately in four places — core's serde models,
the UniFFI mirror, the TypeScript declarations, and a hand-written Python dict
builder — so each one drifted on its own. An audit found 22 fields missing in
the UniFFI mirror and 41 discrepancies in the TypeScript declarations. Two of
those layers are now gone.

### Breaking

- **All languages**: REST methods return the raw response body. Rust and Node
  get `serde_json::Value` / a plain object, Python a `dict`, and C#, Go, C++
  and Java a JSON string to decode with their own library. Rust callers who
  want the old static types can still `serde_json::from_value::<Quote>(v)` —
  `marketdata_core::models` remains public.
- **All languages**: WebSocket messages carry the frame verbatim. Node and
  Python deliver the untouched frame; C#, Go, C++ and Java gain
  `StreamMessage.raw` alongside the existing routing fields.
- **All languages**: `tickers()` no longer unwraps the response envelope. It
  used to return just the array, discarding the sibling `date` / `type` /
  `exchange` / `market` metadata and diverging from the official SDK. The
  array is now at `.data`.
- **All languages**: fields the server omits are absent rather than `false`
  or `null`. Code that relied on, say, `quote.isOpen` always being a boolean
  must now treat it as optional.
- **C#, Go, C++, Java**: the mirrored response records are gone. Decode the
  returned JSON with your platform's usual library.
- **Rust**: `send()` returns `serde_json::Value`.

### Fixed

- `Quote` was missing `referencePrice` and the top-level `serial`.
  `referencePrice` is the basis the exchange computes `change`,
  `changePercent` and the limit prices from — not `previousClose`, which
  differs whenever the reference is adjusted. Callers could not reach it at
  all.
- JSON object keys were being reordered alphabetically, so responses arrived
  as `amplitude, asks, avgPrice, …` instead of the server's own ordering.
- Python's `stock.intraday.quote()` dropped `lastTrade`, `lastTrial`,
  `tradingHalt`, `isContinuous`, the delayed-open/close flags, several
  limit-price flags, and `total`'s `tradeVolumeAtBid` / `tradeVolumeAtAsk` /
  `time`.
- The UniFFI mirror was missing 22 fields, 11 of them on `FutOptQuote`
  (`isOpen`, `isClose`, `isContinuous`, `tradingHalt`, `priceLimits`,
  `lastTrial`, `serial`, `market` and more), which left futures quotes
  largely unusable from C#, Go, C++ and Java. `FutOptTotalStats` mirrored 3
  of 8 fields, losing `tradeValue` entirely.
- TypeScript: four field names were simply wrong, so correct-looking code got
  `undefined` — `FutOptHistoricalCandlesResponse.candles` (really `data`),
  `MacdDataPoint.macd`/`.signal` (really `macdLine`/`signalLine`),
  `KdjResponse.period` (really `rPeriod`/`kPeriod`/`dPeriod`) and
  `IntradayCandle.time: number` (really `date: string`). Futures `quote()`
  and `ticker()` were declared with the stock response types, and
  `FutOptTickerResponse` and `UnsubscribeOptions` were referenced but never
  defined. A stray Rust raw identifier (`r#type`) also made the whole
  declaration file unparseable by `tsc`.
- C#: `RestClientOptions.BaseUrl` was accepted and silently discarded, so a
  client aimed at a test server still talked to production.

### Testing

- The "response compatibility" suites in JavaScript, Python, Go, C# and Java
  asserted against fixture files and reflection rather than against anything
  the SDK produced, so they could not fail. The JavaScript one was actively
  vouching for the three fields the SDK was dropping, and the Python
  cassettes recorded a v0.3-era API whose schema this SDK never parsed. All
  are replaced with tests that run a real loopback HTTP server and compare
  the SDK's output against the bytes the server sent.
- `core/tests/model_fidelity.rs` decodes a recorded response into the
  optional typed models and asserts every field survives the round trip.
- `js/scripts/check-dts-drift.mjs` compares the TypeScript declarations
  against the serde wire names in `core/src/models`, and CI now also runs
  `tsc` over them.
- C# tests skipped themselves when the native library was not on the loader
  path — 36 of 55 never ran in CI. CI now sets it, and the suite passes with
  no skips.

## [Bindings 3.0.0-rc.1 / uniffi 0.1.0-rc.1] - 2026-09-16

First release of the Python, Node and UniFFI bindings, aligned with core
0.8.0-rc.1 and therefore with official `@fugle/marketdata` 1.6.0 /
`fugle-marketdata` 2.6.0 from day one.

Because these bindings have never shipped, **none of core's 0.8.0 breaking
changes are breaking for them** — a binding user has never seen the 0.6-era
`base_url` rule. The reversal described below affects only the Rust crates on
crates.io.

### Version tracks

| Artifact | Registry | Version | Why |
|---|---|---|---|
| `fugle-marketdata` | PyPI | `3.0.0rc1` | must exceed the official package's 2.6.0 |
| `@fugle/marketdata` | npm (`next` tag) | `3.0.0-rc.1` | must exceed the official package's 1.6.0 |
| `Fugle.MarketData` (C#) | NuGet | `0.1.0-rc.1` | never published, no namespace to supersede |
| `github.com/fugle-dev/fugle-marketdata-go` | Go modules | `v0.1.0-rc.1` | same |
| C++ | GitHub Release tarballs | `0.1.0-rc.1` | same |
| Java | not published | `0.1.0-rc.1` | JNA callbacks benchmark far below the other bindings |

### Security

Dependencies with published advisories are updated, so the shipped wheels,
npm addons, NuGet package and Go static libraries no longer contain them:

- `rustls` 0.23.45 (RUSTSEC-2026-0285, TLS 1.3 handshake messages accepted
  across encryption levels), `bytes` 1.12 (RUSTSEC-2026-0007).
- Python: PyO3 0.27 → 0.29 and pyo3-async-runtimes 0.27 → 0.29
  (RUSTSEC-2026-0176, RUSTSEC-2026-0177).
- The unmaintained `rustls-pemfile` is replaced by rustls' built-in PEM parser.
- UniFFI 0.29.4 drops the unmaintained `bincode` and `paste` crates from the
  build.

CI now runs `cargo audit` on every pull request.

### Performance

Every binding sends REST requests through core, so all of them get core's
faster JSON decoding and connection reuse for concurrent calls. See the Rust
0.8.0-rc.1 section.

### Distribution

- Registries come online one at a time. This release candidate is published
  to npm first; PyPI, NuGet and the Go module follow under the same versions
  once their publishing credentials are in place.

- Pre-releases are published to the real registries on channels that are
  never selected by default: pip needs `--pre`, npm uses the `next` dist-tag
  (the official 1.x keeps `latest`), NuGet needs `--prerelease`, Go needs an
  explicit version. See [docs/INSTALL.md](docs/INSTALL.md).
- The NuGet package ID is `Fugle.MarketData`. C# namespaces are unchanged.
- Go is published as a standalone module that links prebuilt static
  libraries for `darwin/arm64`, `darwin/amd64`, `linux/amd64` and
  `windows/amd64` (MinGW), so no `LD_LIBRARY_PATH` is needed. `linux/arm64`
  is not supported yet.
- npm ships one optional-dependency package per platform
  (`@fugle/marketdata-<platform>`); PyPI wheels carry full project metadata.
- Every release is verified after publishing by installing each package from
  its registry on Linux, macOS and Windows. See
  [docs/RELEASING.md](docs/RELEASING.md).

### Added — all bindings

- The three `stock.ownership` endpoints added by official 1.6.0 / 2.6.0,
  with the same `from` / `to` / `sort` contract as ETF holdings:
  institutional investors' trades (`institutional_trades` /
  `institutionalTrades`), director and supervisor holdings
  (`director_holdings` / `directorHoldings`) and TDCC shareholding
  distribution (`tdcc_distribution` / `tdccDistribution`). C# and Go get them
  through UniFFI.

### Changed — Python

- **Minimum Python is 3.8** (wheels are tagged `cp38-abi3`). PyO3 0.29 no
  longer supports 3.7, which reached end of life in June 2023. pip on
  Python 3.7 keeps resolving to the 2.x series through `requires-python`.

### Changed — Node

- `engines.node` is `>= 18`, the oldest version CI tests. napi-rs 3.12.

### Added — Python

- `RestClient(base_url=...)` takes host + path prefix only; a version segment
  raises `TypeError` at construction, matching the official SDK.
- `RestClient.base_url` and `client.stock.base_url` expose the resolved prefix.
- `WebSocketClient(version={"futopt": "v1.0"})`. Omitted products get their
  latest (stock v1.0, futopt v1.1). An unsupported pairing raises `TypeError`
  with the official SDK's wording.
- `client.stock.ownership.etf_holdings(...)` (async + sync). `sort` accepts
  only `"asc"` / `"desc"`; anything else raises `ValueError` rather than being
  dropped, since a typo would otherwise return the opposite series.
- `futopt.intraday.tickers(is_spread=...)`.
- Ownership methods accept the official 2.x spellings `from_=`, `to=` and
  `**{"from": ...}`. Previously these were dropped with a warning and the
  date range silently did not apply.
- `cargo test -p marketdata-py --no-default-features` now links and runs.
  `extension-module` became an optional (default-on) feature; previously the
  crate's Rust tests could not build at all.

### Added — Node

- Same surface as Python: `baseUrl` semantics, `RestClient.baseUrl` /
  `StockClient.baseUrl` getters, `version` option (typed as
  `StreamingVersionOptions`, so TypeScript rejects an unknown product at
  compile time), `stock.ownership.etfHoldings(...)`, `isSpread`.
- `types.d.ts` gains `EtfHoldingComponent` / `EtfHoldingsEntry` /
  `EtfHoldingsResponse` — `etfHoldings` referenced `EtfHoldingsResponse` in its
  return type without defining it.

### Changed — UniFFI (C# / Go / Java / C++)

- UniFFI 0.29.4 (from 0.28.3), the highest version every shipped generator
  supports: C# v0.10.0, Go v0.5.0, C++ v0.9.0, Java 0.2.1.
- C#: the package targets `netstandard2.0`, `net8.0` and `net10.0`. `net6.0`
  reached end of support in November 2024 and is dropped. Generated records
  expose sequences as arrays (`T[]`) instead of `List<T>`; the wrapper's
  `GetTickers` / `GetTickersAsync` still return `List<T>`.
- Go: generated constructors return `error` (untyped nil on success) instead
  of `*MarketDataError`; use `errors.As` to inspect the concrete error.
  `NewFugleRestClient` is unaffected.
- Java: CI installs the generator from `IronCoreLabs/uniffi-bindgen-java`; the
  previously referenced repository does not exist. Java is still not
  published.

### Added — UniFFI (C# / Go / Java / C++)

- `StreamingVersionRecord` for per-product version selection.
- `stock.ownership.etf_holdings` (async + `cpp`-feature sync variant) and the
  three ETF holdings records.
- `RestClient.base_url` / `StockClient.base_url`, `is_spread` on futopt
  tickers.
- C#: `RestClient.Stock.Ownership`. The C# wrapper previously exposed no
  ownership endpoints at all, not even ETF holdings. FutOpt `GetTickers`
  gains `isSpread`.

### Fixed — UniFFI

- **The crate did not compile at all**, on `main`, for an unknown span: 29
  type errors where the mirror records had drifted from core after the
  0.7.2/0.7.3 decode fixes loosened fields to `Option`. Mirrors now match core
  rather than papering over absence with `unwrap_or_default()`.
- `KdjResponse` exposed a single `period`; the endpoint has taken
  `r_period` / `k_period` / `d_period` since 0.7.2.
- **The committed C# and Go generated bindings predated 0.8.0**, so loading
  them failed with a UniFFI checksum mismatch. They are regenerated, and CI
  now fails if they drift from the Rust interface again.
- Go: `NewFugleRestClient` returned an error on every call (a typed-nil
  `*MarketDataError` wrapped in `error`), and `WithBaseUrl` was stored but
  never applied. Both are fixed.
- C# / Go health-check options use `HeartbeatTimeoutMs`. The removed
  `IntervalMs` / `MaxMissedPongs` never had a counterpart in core and were
  silently ignored. C# `HealthCheckOptions.Enabled` now defaults to true,
  matching core.

### Fixed — Tauri GUI

- `StreamTrade` construction and an `Option<u64>` cast; the latter had been
  broken on `main` since core loosened the futopt candle volume field.

### Fixed — Python test suite

- 102 of 138 tests were failing on `main`. They constructed clients
  positionally (removed in 0.4.0), asserted the 2.x `HealthCheckConfig` shape
  (`interval_ms` / `max_missed_pongs` — this SDK has neither), and expected
  `ValueError` where both this SDK and the official one raise `TypeError`.
  Now 142 passed.

  Note: run `maturin develop` before `pytest` — a stale gitignored `.so` under
  `py/fugle_marketdata/` shadows the installed wheel.

## [Rust 0.8.0-rc.1] - 2026-09-16

Aligns with the official `@fugle/marketdata` 1.6.0 and `fugle-marketdata`
2.6.0.

See [MIGRATION-0.8.md](MIGRATION-0.8.md).

### ⚠️ Breaking

- **`base_url` no longer accepts a version segment — this reverses 0.6.0.**
  A base URL carries the host and path prefix only; the SDK appends the
  version. Passing a 0.6-era base URL (one ending in `/v1.0`) is now
  rejected with a `ConfigError` naming the prefix to use instead.

  This follows the official SDKs, whose rationale is that letting two
  options decide the same path segment forces precedence rules — and those
  rules make anyone who only wants to change host manage the version by
  hand. That matters more now that streaming versions are per-product: with
  `futopt` on `v1.1` and `stock` on `v1.0`, one baked-in segment cannot be
  right for both.

  Unlike the 0.6.0 change, this failure is loud rather than silent.

- **MSRV is 1.88** (`rust-version`), up from a declared 1.82 that no longer
  built. The current releases of `bon` and `napi` need 1.88, and a toolchain
  without the MSRV-aware resolver would pick them up anyway, so a lower
  declaration was nominal. CI builds the declared MSRV against an
  MSRV-aware lockfile.

- **The HTTP client is no longer part of the public API.** `impl
  From<ureq::Error> for MarketDataError` and `Auth::apply_to_request` are
  removed. Neither was needed to call the API: transport failures still
  surface as `ConnectionError` / `TimeoutError` and error statuses as
  `AuthError` / `ApiError`. Replacing the HTTP client in the future is
  therefore not a breaking change.

- **ureq 2.12 → 3.4.** ureq 2 has had no release since December 2024 and
  re-exported pre-1.0 crates such as rustls, so a rustls major bump forced
  its own breaking changes. Observable REST behaviour is unchanged and pinned
  by new end-to-end tests: OS trust store plus optional extra root CA,
  `accept_invalid_certs`, error bodies on 4xx/5xx, retries, timeouts, gzip,
  and proxy environment variables staying ignored (ureq 3 would read them by
  default). Two small differences: idle pooled connections now close after
  15 seconds, and a credential containing characters that are invalid in an
  HTTP header is reported as `ConfigError` instead of a connection error.
  Small requests cost about 1.5 µs more in ureq 3 itself; large responses are
  unaffected.

- **`tungstenite` 0.29 → 0.30.** `MarketDataError` implements
  `From<tungstenite::Error>`, so the error type in that impl changed. Client
  behaviour is unchanged.

- **`WebSocketFactory::stock()` / `::futopt()` now return `Result`**, so a
  rejected `base_url` surfaces at the earliest honest point. `RestClient`
  keeps an infallible `base_url()` and surfaces the rejection from the first
  request; `try_base_url()` reports it immediately instead.

### ⚠️ Behaviour change

- **futopt streaming defaults to `v1.1`**, which delivers trial-matching
  (試撮, TAIFEX I022/I082) frames on `trades` / `books`. A trial frame is a
  simulated match, not a trade — branch on `is_trial` before acting on a
  price. Pin `FutOptVersion::V1_0` to opt out.

  `urls::FUTOPT_WS` and `ConnectionConfig::fugle_futopt` moved to `v1.1`
  in step, so they cannot drift from the factory.

  Note that `aggregates` is **not** version-gated: it carries trial data on
  every version, and pinning `V1_0` does not opt out of it.

### Added

- `stock().ownership().etf_holdings()` — `GET
  /stock/ownership/etf-holdings/{symbol}` with `from` / `to` / `sort`.
- `stock().ownership().{institutional_trades, director_holdings,
  tdcc_distribution}()` with their request builders and response models,
  matching official 1.6.0 / 2.6.0. Numeric fields are `Option` because they
  have not been checked against live payloads yet.
- `StockVersion` / `FutOptVersion` enums and
  `WebSocketFactory::{stock_version, futopt_version}`. One enum per product
  makes an unsupported pairing unrepresentable, so unlike the official SDKs'
  runtime-validated version map, a bad combination does not compile.
- `RestClient::resolved_base_url()` — the fully resolved request prefix.
  Since the SDK owns the version segment, this is the only way to see what a
  client actually resolved to.
- `RestClient::try_base_url()`.
- `futopt().intraday().tickers().is_spread(bool)` filter, and `is_spread` on
  `FutOptTicker`.
- `FutOptQuote`: `market`, `price_limits`, `last_trial`, `trading_halt`,
  `is_trial`, `is_delayed_open`, `is_delayed_close`, `is_continuous`,
  `is_open`, `is_close`, `serial`. `FutOptTotalStats` goes from 3 fields to
  8; `FutOptLastTrade` gains `bid` / `ask` / `serial`. New
  `FutOptPriceLimits` and `FutOptTradingHalt`.
- Streaming frames: `is_trial` on `TradesData` / `BooksData` /
  `AggregatesData`; `derived_bid` / `derived_ask` / `data_type` / `exchange`
  on `BooksData`; `time` / `serial` / `is_replaced` on `StreamTrade`;
  `last_trial` on `AggregatesData`.
- `TradeInfo` gains `serial: Option<String>` — stock's `lastTrade` /
  `lastTrial` carry one and it was previously discarded.
- Optional boolean flags (`is_trial`, `is_replaced`) now tolerate an explicit
  JSON `null` as well as an absent key. `#[serde(default)]` alone only covers
  the absent case; a literal `null` failed the whole decode. The API
  demonstrably uses explicit nulls for unset fields on dormant contracts.
  Precautionary — no `isTrial: null` has been observed in the wild.
- `prod_smoke` probes for etf-holdings, the three new ownership endpoints,
  the `isSpread` filter, and spread contracts (discovered dynamically).

### Performance

- REST responses are buffered before JSON decoding instead of being parsed
  byte by byte from the socket. Measured with `benches/rest_decode.rs`: a
  stock quote takes 18% less time, 2000 historical candles 59% less, and the
  same response gzip-encoded 82% less.
- The HTTP agent keeps up to 16 idle connections per host instead of ureq's
  default of one. Concurrent requests on a shared client previously opened a
  new TCP and TLS connection each time and could exhaust local ports.

### Fixed

- **`lastTrade.serial` / `lastTrial.serial` decode correctly on futopt.**
  The server sends a zero-padded **string** (`"00379320"`) for futopt and a
  **number** (`17738549`) for stock — the same field, different JSON types
  per product. The official TypeScript interface declares `serial: number`
  for both, which is wrong for futopt; typing it that way made
  `futopt/intraday/quote` fail to decode outright, and would have broken
  futopt's streaming `aggregates` frame too, since it carries the same
  object. Both spellings now normalise to `String` — a serial is an opaque
  identifier, never an operand, and futopt's padding is significant.

  Found by running the sweep against a live environment. Same class of bug
  as 0.7.2/0.7.3: the published type did not match the payload.

- **Symbol path segments are percent-encoded.** Spread contract symbols
  carry a `/` (e.g. `BRFJ6/F7`). Applied to all 19 endpoints that put a
  symbol in the path. The encoder reproduces `encodeURIComponent`'s reserved
  set exactly, so a symbol encodes identically here and in the Node SDK.

  Measured caveat: the live gateway currently *tolerates* an unencoded
  slash — encoded and unencoded requests return identical responses. So
  this is correctness-by-spec and protection against any symbol containing
  reserved characters, not the repair of an observed outage.

### Notes

- The official SDKs' 1.5.0 health-check rework (freshness-based detection
  plus a disconnect reason, and the `maxMissedPongs >= 1` clamp) needs no
  counterpart: this SDK has used a single async-native timeout window since
  0.3.0, and has no missed-pong counter to clamp. `health_check`'s module
  docs now carry a mapping table for anyone porting config from Node or
  Python.
- `core/PUBLIC-API.txt` is now a real `cargo public-api` snapshot; it was a
  placeholder, so the public-api CI check could never pass.
- `name` / `previous_close` were dropped from the official futopt quote
  response in 1.5.0 but are retained here as `Option`, so payloads still
  carrying them keep decoding.

## [Rust 0.7.3] - 2026-05-16

Follow-up to 0.7.2: a deeper prod sweep showed the futopt
symbol-dependent endpoints were never decode-tested because the
after-hours session value was wrong.

### Fixed

- **`futopt/intraday/tickers` & `futopt/intraday/products`**:
  `after_hours()` emitted `session=afterhours`, but these two endpoints
  require the **uppercase** `session=AFTERHOURS` — lowercase is silently
  accepted and returns **zero rows**. So `.after_hours()` on tickers/
  products appeared to "work" while always yielding an empty list. Now
  emit `AFTERHOURS`. (quote/ticker/candles/trades/volumes correctly keep
  lowercase `afterhours` — the server is genuinely inconsistent across
  endpoints; verified against prod.)

### Changed

- `core/examples/prod_smoke`: futopt contract discovery now queries the
  (populated) after-hours tickers list and prefers a `TXF*` contract,
  falling back to the current near-month `TXFF6` (was the long-expired
  `TXFE5`, which 404'd the entire futopt REST+WS sweep).

## [Rust 0.7.2] - 2026-05-16

Decode-correctness patch. A full prod-environment smoke sweep (every REST
endpoint + every WS channel, new `core/examples/prod_smoke` harness)
surfaced response models written against the API spec rather than real
payloads. Several endpoints were **completely unusable** before this fix.

### Fixed

- **`stock/intraday/tickers` & `futopt/intraday/tickers`**: `send()`
  deserialised the body straight into `Vec<Ticker>` / `Vec<FutOptTicker>`,
  but prod wraps the list in an envelope object
  (`{date,type,exchange,data:[…]}`). Every call failed with
  `invalid type: map, expected a sequence`. Now decodes the envelope and
  returns `.data`. **Both endpoints were 100% broken.**
- **`futopt/intraday/products`**: `Product.end_session` was `Option<i32>`
  but prod sends it as the string `"1"`. New `de_opt_i32_flexible`
  deserializer accepts a JSON int, a numeric string, `null`, or `""`.
- **`stock/historical/stats`**: `StatsResponse.change_percent` was a
  required `f64`; prod never sends `changePercent`. Now `Option<f64>`.
- **`stock/technical/{sma,rsi,kdj,macd,bb}`**: responses required
  `type`/`exchange`/`market`/`timeframe`; prod returns none of them. All
  four are now `Option`.
- **`stock/technical/macd`**: data points expected `macd`/`signal`/
  `histogram`; prod sends `macdLine`/`signalLine` and no histogram.
  Renamed via serde; `histogram` is now `Option<f64>`.
- **`stock/technical/kdj`**: `KdjResponse` expected a single `period`;
  prod returns `rPeriod`/`kPeriod`/`dPeriod`. Response fields corrected.

### Added

- `KdjRequestBuilder::r_period` / `k_period` / `d_period` setters. The
  endpoint requires `rPeriod`/`kPeriod`/`dPeriod`; previously only
  `period` could be set, so the endpoint was unreachable (HTTP 400) via
  the SDK.
- `core/examples/prod_smoke` — re-runnable REST+WS decode sweep that
  classifies each probe by `MarketDataError` variant (Schema vs HTTP vs
  param) and emits one JSON record per endpoint.

## [Rust 0.7.1] - 2026-05-16

Refactor-only patch on top of 0.7.0. **Zero public API or behaviour
changes.** Internal cleanup driven by a code-reuse / quality / efficiency
review of the 0.7.0 diff.

### Changed

- Extracted shared `await_auth_response` helper in `websocket::aio::reconnect`.
  Both `WebSocketClient::connect` and the internal `try_connect` reconnect
  path now share a single 22-line auth-frame read loop, removing a copy-paste
  risk where the WebSocket auth protocol could drift between fresh-connect
  and reconnect.
- `metrics_compat::build_drop_counters` now passes `client_id` as
  `&str` (`.as_deref().unwrap_or("")`) instead of a freshly allocated
  `String`, saving one allocation per `WebSocketClient::new`.
- Inlined the `delay_ms` binding into the `tracing_compat::warn!` macro
  call in `try_reconnect` so the `Duration::as_millis()` cast is dropped
  along with the rest of the macro tokens when the `tracing` feature is
  disabled. Also clears a stale `unused_variable` warning under that
  feature combo.

## [Rust 0.7.0] - 2026-05-16

Monitor-readiness followups bundle. **Zero breaking changes.** Five
additive improvements driven by SDK user feedback after 0.6.0
integration: macro hygiene, `WebSocketErrorKind::Http` doc relocation,
multi-client mock, transport-drop intent injection, and an opt-in
`metrics` crate integration. See `MIGRATION-0.7.md` for the opt-in
patterns; existing 0.6.0 code compiles unchanged.

### Added

- **Optional `metrics` feature** (`features = ["metrics"]`). When
  enabled, `WebSocketClient::new` registers
  `fugle_marketdata_ws_messages_dropped_total` and
  `fugle_marketdata_ws_events_dropped_total` counters on the active
  `metrics::Recorder`, both labelled with `endpoint` (URL host) and
  `client_id`. Polling getters remain authoritative; the integration
  mirrors. Off by default — no transitive cost without the feature.
- **`ConnectionConfig::client_id(...)` builder field** —
  caller-supplied low-cardinality identifier used as a metric label.
  64-byte cap with `tracing::warn!` on truncation. New `client_id() ->
  Option<&str>` accessor.
- **`MockWsServer::start_with_capacity(n)`** + per-client targeting via
  `inject_frame_for(idx, …)`, `next_subscribe_id_for(idx, …)`,
  `close_for(idx, code, reason)`. New convenience `aio_pair_n(n)`. The
  bare `inject_frame` / `next_subscribe_id` / `close` panic on
  multi-client mocks with a message naming the `_for` alternative.
- **`MockWsServer::drop_transport(...)`** + `drop_transport_for(idx)` —
  closes the underlying TCP socket without sending a Close frame,
  forcing `DisconnectIntent::Network` on the client side. Idempotent.
- **`metrics_compat::DropCounter`** internal wrapper around
  `Arc<AtomicU64>` + optional `metrics::Counter`. Single bump path
  guarantees the polling-getter atomic and the `metrics` recorder stay
  in lock-step.
- **`core/PUBLIC-API.txt` snapshot** + `core/tests/public_api_snapshot.rs`
  (ignored by default; CI runs explicitly) + new
  `.github/workflows/public-api.yml` job filtered on `core/src/lib.rs`,
  `core/src/tracing_compat.rs`, `core/Cargo.toml`. Acknowledge
  intentional surface changes in `core/PUBLIC-API.md`.

### Changed

- **`WebSocketErrorKind::Http(u16)` doc-comment** now contains the full
  status-code → `ErrorKind` / `is_retryable()` mapping table.
  `MarketDataError::source_kind`'s rustdoc cross-references the
  variant. A `#[cfg(test)]` consistency assertion in `core/src/errors.rs`
  exercises representative status codes against both methods so
  doc-vs-impl drift fails CI.
- **`tracing_compat` module-level rustdoc** explains why
  `__tracing_noop` is exported at crate root via `#[macro_export]` and
  reaffirms it is internal-only (carries `#[doc(hidden)]`).

### Documentation

- **`MIGRATION-0.7.md`** — opt-in patterns for the four feature
  additions plus the REST + WebSocket dual-host pattern (already
  supported in 0.6.0; 0.7.0 makes it discoverable). Uses
  `your-ws-host.example.com` as the placeholder host name; never
  references internal-only hostnames.
- **`core/README.md`** — new "Feature flags" table covering
  `tokio-comp` / `tracing` / `test-utils` / `metrics`. New "Independent
  endpoints" paragraph in "Which constructor should I use?" cross-
  referencing `MIGRATION-0.7.md`.

### Origin

Five-point feedback list from 0.6.0 SDK consumer reviews
(2026-05-15…16):

1. `tracing_compat` macro hygiene clarification.
2. `WebSocketErrorKind::Http` mapping table relocation.
3. `MockWsServer` multi-client support for monitor's dual-probe topology.
4. `MockWsServer` `DisconnectIntent::Network` injection for incident-
   classifier testing.
5. `metrics` ecosystem integration so consumers don't hand-roll
   `gauge.set(client.messages_dropped_total())` boilerplate.

## [Rust 0.6.0] - 2026-05-15

Minor-version pass: `WebSocketError` structured-kind split, `MockWsServer`
test utility, and an OpenAI-aligned `WebSocketFactory::base_url` semantic
shift. See `MIGRATION-0.6.md` for the full migration recipe.

### Breaking — SILENT (no compile error; behaviour changes)

- **`WebSocketFactory::base_url(...)` now expects the full URL prefix
  including the API version segment.** Pre-0.6.0 the factory silently
  injected `/v1.0`; 0.6.0 does not. Code that worked in 0.5.x compiles
  against 0.6.0 but produces a 404 on first connect because the URL
  lacks `/v1.0`. Aligns with OpenAI / Stripe / AWS / Anthropic SDK
  conventions. **READ `MIGRATION-0.6.md` §1 BEFORE UPGRADING PROD.**

### Breaking — type-level (compile error)

- **`MarketDataError::WebSocketError` reshape**:
  `{ msg: String }` → `{ kind: WebSocketErrorKind, msg: String }`. Pattern
  matches need the new `kind` field (or use `..`).
- **`is_retryable()` retry verdict refined.**
  Protocol violations (`Protocol`, `Capacity`, `Utf8`) and TLS failures
  are now **non-retryable** — they were retryable in 0.5.x. Migration
  table in `MIGRATION-0.6.md` §2.
- **`From<tungstenite::Error>` no longer routes to `ConnectionError` /
  `AuthError` for WebSocket transport failures.** Every upstream variant
  produces a `MarketDataError::WebSocketError { kind, msg }`.

### Added

- **`WebSocketErrorKind` enum** (`#[non_exhaustive]`): `Protocol`,
  `Capacity`, `Utf8`, `Tls`, `Io`, `Http(u16)`, `Other`. Re-exported at
  crate root.
- **`MarketDataError::source_kind()` mapping refined** to honour
  `WebSocketErrorKind` — `Io` → `Network`, `Tls` → `Auth`,
  `Http(429)` → `RateLimit`, etc.
- **`core::testing::MockWsServer`** behind `features = ["test-utils"]`.
  In-process WebSocket server with subscribe-ACK echo, frame injection,
  and server-initiated close. `aio_pair()` convenience constructor pairs
  it with a pre-configured async client. Replaces hand-rolled echo
  servers in monitor / py / js / uniffi test suites.
- **`test-utils` cargo feature** (off by default; pulls `tokio-comp`
  transitively).
- **`tests/mock_server_smoke.rs`** — 5-scenario smoke test catching
  drift between the mock's subscribe protocol and production
  `protocol.rs`.

### Internal

- `core/src/websocket/factory.rs`: `endpoint_for(kind)` now reads from
  `urls::STOCK_WS` / `urls::FUTOPT_WS` directly on the no-override path;
  appends only `/{kind}/streaming` on the override path.
- `uniffi/src/errors.rs`: shadow `WebSocketError { msg }` retained for
  FFI ABI stability; conversion at boundary stringifies `kind` into `msg`.

## [Rust 0.5.1] - 2026-05-15

Polish pass between 0.5.0 (databento patterns) and 0.6.0 (WebSocketError
split + mock server). All changes are additive or doc-only; no API
removals.

### Added

- **`MarketDataError::source_kind() -> ErrorKind`** — coarse-grained
  classification helper returning one of `Network`, `Protocol`, `Auth`,
  `RateLimit`, or `Client`. Lets monitor / downstream code branch on
  failure category without pattern-matching every variant.
- **`ErrorKind` enum** — `#[non_exhaustive]`, re-exported at the crate
  root. Includes a dedicated `RateLimit` variant for HTTP 429 so the
  incident-response action ("reduce request volume") doesn't get mixed
  with `Network` failures ("retry with backoff").
- **`events_dropped_total() -> u64`** on both sync and async
  `WebSocketClient`. Mirrors `messages_dropped_total()` for the
  lifecycle event channel; increments when the bounded channel drops
  events under the drop-newest backpressure policy.

### Documentation

- **`Symbols::normalized` case-sensitivity policy** is now explicit in
  the module rustdoc: dedup is **byte-for-byte case-sensitive**.
  `"TXFB6"` and `"txfb6"` are distinct subscriptions, matching the
  TWSE / Fugle wire contract.
- **"Which constructor should I use?"** section in `core/README.md`
  pinning the idiomatic choice for the four construction paths
  (`bon` builder, positional `new(...)`, typestate factory,
  convenience constructors).
- **Stability promise** sections on
  `ReconnectionConfig::disabled()`,
  `RetryPolicy::conservative()`, and `RetryPolicy::aggressive()`.
  These functions are FFI-load-bearing and will be preserved across
  every `0.x` release.

### Internal

- `emit_event` now takes a `&Arc<AtomicU64>` drop counter; 56 call
  sites updated. `dispatch_messages`, `try_reconnect`, `try_connect`,
  and `run_writer_task` carry the counter through to the saturation
  point.

## [Rust 0.5.0] - 2026-05-15

Minor-version pass adopting three patterns observed in databento-rs:
`Symbols` rename + dedup contract, typestate `WebSocketFactory`, and
`bon::Builder` derives on `RetryPolicy`, `ReconnectionConfig`, and
`SubscribeRequest`. See `MIGRATION-0.5.md` for the full migration
guide.

### Breaking

- **`SymbolSpec` renamed to `Symbols`** and moved out of
  `models/subscription.rs` into a dedicated `models/symbols.rs`. All
  nine existing `From` impls retarget the renamed type. Mechanical
  migration: `sed -i '' 's/SymbolSpec/Symbols/g'` over downstream
  sources.
- **Subscription dispatch deduplicates symbols.**
  `SubscribeRequest::with_symbols`, `StockSubscription::new`, and
  `FutOptSubscription::new` now run their input through
  `Symbols::normalized()` (trim whitespace, drop empty, dedup
  preserving insertion order, collapse `Many` of length 1 to `Single`)
  before producing the request. Duplicate symbols that previously
  produced two server ACKs now collapse to one subscription.
- **`WebSocketFactory` is typestate-enforced.**
  `WebSocketFactory::new(auth)` becomes
  `WebSocketFactory::new().auth(auth)`. Calling `.stock()` / `.futopt()`
  before `.auth(...)` is now a compile-time error
  (`compile_fail` doctests guard the contract).
- **`ReconnectionConfig::with_max_attempts` / `with_initial_delay` /
  `with_max_delay` removed.** These fallible chainable validators are
  superseded by the unvalidated `ReconnectionConfig::builder()` (bon)
  and the existing validating `ReconnectionConfig::new(...)`
  positional constructor.

### Added

- **`Symbols::normalized()`, `len()`, `is_empty()`, `iter()`,
  `chunked(n)`** helpers on the renamed enum.
- **`SUBSCRIPTION_BATCH_LIMIT: Option<usize>`** const (currently `None`)
  in `models::symbols`, reserved for a future server-documented
  per-frame limit. Downstream code can branch on the constant without
  another version bump.
- **`bon::Builder` derives** on `RetryPolicy`, `ReconnectionConfig`,
  and `SubscribeRequest`. `bon` adds `maybe_*` setters for `Option<T>`
  fields. Existing constructors (`new`, `with_symbols`, presets) are
  preserved.

### Internal

- Adopted `bon = "3"` as a runtime dependency for builder generation.
- `ConnectionConfig` intentionally retains its hand-rolled builder so
  the `assert!`-based zero-capacity-buffer validation contract from
  the `websocket-config` spec is preserved.

## [Rust 0.4.1] - 2026-05-15

Documentation-policy and publish-readiness pass. No runtime changes; no
breaking API changes.

### Added

- **Declared MSRV: `rust-version = "1.82"`** in `core/Cargo.toml`. New
  `rust-core-msrv` CI job builds with Rust 1.82 to guard the contract.
- **Strict documentation lints** at crate root: `#![deny(missing_docs,
  rustdoc::broken_intra_doc_links, clippy::missing_errors_doc)]`.
  Backfilled doc comments and `# Errors` sections across the public API.
- **README is now the crate-level rustdoc** via
  `#![doc = include_str!("../README.md")]`. README code blocks tagged
  `rust,ignore` so doctests stay green.
- **`docs.rs` renders all features with feature badges.**
  `[package.metadata.docs.rs]` switched to `all-features = true` plus
  `--cfg docsrs`; `aio` and other `tokio-comp`/`tracing`-gated items
  carry `#[cfg_attr(docsrs, doc(cfg(...)))]`.
- **`check-cfg` declaration** for the `python` and `js` feature flags
  used by the FFI binding crates, silencing the `unexpected_cfgs` warnings
  that previously polluted `cargo build`/`cargo doc` output.

### Internal

- Auto-fixed 21 `elided_named_lifetimes` warnings via `cargo fix`.
- Tagged `aio::WebSocketClient::send_text` as `#[allow(dead_code)]` with
  a `reason` (kept for future direct-frame test harness).

## [Rust 0.4.0] - TBD

Production-readiness pass driven by the `monitor` integration: opt-in
`tracing`, secret-redacting `Debug`, sane reconnect default, REST retry
policy, multi-connection event labels, graceful shutdown drain, JS-style
WebSocket factory, and the removal of a small set of legacy constructors.
See `MIGRATION-0.4.md` for the full migration guide.

### Breaking

- **`ConnectionEvent::Disconnected` gains `intent: DisconnectIntent { Client, Server, Network }`.**
  `ConnectionState::Closed` mirrors the same `intent` field. Other
  `ConnectionEvent` variants are unchanged. Pattern matches on
  `Disconnected` need `..` or explicit `intent` destructuring.
- **`ReconnectionConfig::default().enabled` flipped `false` → `true`.**
  Rust callers on the `WebSocketClient::new(config)` happy path get
  auto-reconnect by default. Bindings explicitly call
  `ReconnectionConfig::disabled()` at the FFI boundary so end-user
  behavior is preserved (workspace-level CI gate in
  `core/tests/reconnect_default.rs`).
- **`Auth` and `ConnectionConfig` `Debug` redacted.** `Auth::ApiKey(***)`
  etc. instead of the raw token. `ConnectionConfig::url`'s sensitive
  query parameters (`token`, `key`, `apikey`, `api_key`, `secret`,
  `password` — case insensitive) are masked. Logs and `tracing` output
  now safe by default.
- **`SubscribeRequest::{trades, candles, books, aggregates}` removed.**
  Use `SubscribeRequest::new(Channel::*, symbol)`. Zero non-test callers
  in the workspace.
- **`disconnect()` is now a graceful drain, not fire-and-forget.**
  Default 5 s drain timeout sends Close, awaits peer Close ack, then
  force-closes on timeout. Use
  `WebSocketClient::shutdown_with_timeout(Duration)` for a custom
  budget; `Duration::ZERO` matches the old fire-and-forget behavior.

### Added

- **Opt-in `tracing` feature** (`features = ["tracing"]`). Hot-path
  `debug!` for received frames, lifecycle `info!`/`warn!` for
  connect/auth/reconnect/heartbeat, `error!` for runtime-init / close-
  frame failures. `#[tracing::instrument]` spans named
  `ws.connect` / `ws.subscribe` / `ws.unsubscribe` / `ws.disconnect`
  on cold path only — zero overhead on the per-frame dispatch loop.
  Replaces 3 of 5 `eprintln!` sites; the 2 panic-boundary sites stay as
  `eprintln!` so they survive subscriber teardown.
- **`RestClient::with_retry(RetryPolicy)`** — opt-in exponential backoff
  with uniform jitter. `RetryPolicy::conservative()` (3 attempts, 100 ms
  initial, 2 s ceiling) and `RetryPolicy::aggressive()` (5/250 ms/10 s)
  presets. Retries only errors classified by
  `MarketDataError::is_retryable()`.
- **`Auth::from_env()`** — probes `FUGLE_API_KEY` →
  `FUGLE_BEARER_TOKEN` → `FUGLE_SDK_TOKEN`, treats empty string as
  unset.
- **`WebSocketFactory`** — JS / Python SDK-equivalent factory taking one
  auth + optional shared base URL. `.stock()` / `.futopt()` return
  `ConnectionConfigBuilder` for further chaining. Mirrors
  `fugle-marketdata-node/src/websocket/factory.ts` shape.
- **`pub mod urls`** — centralized endpoint constants. Full canonical
  endpoints (`STOCK_WS`, `FUTOPT_WS`, `REST_BASE`) plus host roots and
  version (`WS_BASE_ROOT`, `REST_BASE_ROOT`, `API_VERSION`) for
  composing custom URLs.
- **Configurable channel buffers** — `ConnectionConfig::builder()`
  exposes `message_buffer(usize)` and `event_buffer(usize)`. Default
  `message_buffer` bumped 1024 → 4096 to give multi-symbol consumers
  ~2 s of headroom at TWSE 9:00 open burst (~2000 msg/s);
  `event_buffer` stays at 1024.
- **`messages_dropped_total()` counter** — monotonic `AtomicU64` on each
  client, incremented when the inbound message channel saturates and a
  frame is dropped (drop-newest). Paired with `tracing::warn!` per
  drop.
- **`is_subscribed(&Channel, &str)` + `subscription_count()`** on both
  sync and async clients.
- **`shutdown_with_timeout(Duration)`** + `DEFAULT_SHUTDOWN_TIMEOUT`
  const on both clients (5 s default).

### Internal

- `ConnectionEvent` saturation drop signal moved from `eprintln!` to
  `tracing::warn!` (gated, no-op when feature off).
- Sync `owner_thread` shutdown path now drains write queue → sends
  Close → awaits peer Close ack within `CLOSE_ACK_DEADLINE` (2 s).
  Supervisor exit signaled via mpsc one-shot so `shutdown_with_timeout`
  can bound its wait without `JoinHandle::join_timeout` (which std
  lacks).
- Async dispatch task short-circuits its reconnect loop via a new
  `shutdown_requested: AtomicBool` flag so `disconnect()` cannot race
  the auto-reconnect path.

### Migration

See `MIGRATION-0.4.md` at the repo root.

## [Rust 0.3.0] - TBD

Third Rust crate release — **sync-default `WebSocketClient` with optional
tokio runtime**, following the redis-rs `tokio-comp` pattern. REST already
ran on `ureq` (sync); WebSocket joins it as the default surface. Consumers
that need the async client opt in via a feature flag.

### Breaking — default `WebSocketClient` is now sync

```rust
// 0.2
let client = WebSocketClient::new(config);
client.connect().await?;
client.subscribe(StockSubscription::new(Channel::Trades, "2330")).await?;

// 0.3 (default, no tokio)
let client = WebSocketClient::new(config);
client.connect()?;
client.subscribe(StockSubscription::new(Channel::Trades, "2330"))?;

// 0.3 (async, requires `features = ["tokio-comp"]`)
use fugle_marketdata::aio::WebSocketClient;
let client = WebSocketClient::new(config);
client.connect().await?;
client.subscribe(StockSubscription::new(Channel::Trades, "2330")).await?;
```

`.await` on `connect()`/`subscribe()`/etc. is a compile error after the
upgrade — that's the migration signal. Names and arguments are identical
between the two clients (redis-rs convention).

### New — `tokio-comp` feature

```toml
[features]
default = []
tokio-comp = ["dep:tokio", "dep:tokio-tungstenite", "dep:futures-util"]
```

- `fugle-marketdata` and `fugle-marketdata-core` both expose `tokio-comp`.
- Sync consumers compile with **zero tokio** in `Cargo.lock` (~80 fewer
  transitive crates, ~30-40s faster cold build, ~600-900KB lighter binary).
- Async consumers see no change in dep graph: `tokio-tungstenite 0.29`
  already depends on the same `tungstenite 0.29` that the sync path uses.

### Moved — async API under `aio::`

| 0.2 path | 0.3 path |
|---|---|
| `marketdata_core::WebSocketClient` (async) | `marketdata_core::aio::WebSocketClient` |
| `marketdata_core::AsyncRuntime` | `marketdata_core::aio::AsyncRuntime` (gated) |
| `fugle_marketdata::WebSocketClient` (async) | `fugle_marketdata::aio::WebSocketClient` |

### Removed — redundant async wrappers

- `state_async()` — drop; the sync `state()` reads the same `RwLock`.
- `is_closed()` (async) — folded into the sync `is_closed()`. The 0.2
  `is_closed_sync()` rename intermediate is gone; just use `is_closed()`.
- `message_stream()` — only available on `aio::WebSocketClient` (returns
  `tokio::sync::mpsc::Receiver`). Sync callers use `messages()`.

### Internal refactors (no behavior change)

- New `core/src/websocket/protocol.rs`: framing/parsing helpers shared
  between sync + async paths (wraps existing `WebSocketRequest::{auth,
  subscribe, unsubscribe}` model constructors).
- New `core/src/websocket/connection_event.rs`: runtime-free
  `ConnectionState`, `ConnectionEvent`, `emit_event`.
- New `core/src/websocket/sync/`: blocking client backed by `tungstenite`
  and `std::thread`. Single owner thread per connection with a bounded
  outbound queue (`sync_channel(64)`) and `set_read_timeout`-based
  polling. Supervisor handles automatic reconnect with exponential
  backoff matching the async path.
- `core/src/runtime.rs` moved to `core/src/websocket/aio/runtime.rs`
  (only consumed by FFI bindings; gated behind `tokio-comp`).

### FFI bindings

Python / Node.js / UniFFI / Tauri all enable `tokio-comp` on their
`marketdata-core` workspace dep and import
`marketdata_core::aio::WebSocketClient` explicitly. No FFI surface
change.

Per-binding sync-vs-async evaluation completed in
[docs/FFI-BINDING-RUNTIME-DECISION.md](docs/FFI-BINDING-RUNTIME-DECISION.md):
**all four bindings keep `tokio-comp`** because each maps its target
language's idiomatic async surface (Python `await`, Node `Promise`,
UniFFI `suspend fun` / Swift async, Tauri's tokio runtime) onto the
async client. Sync core remains the canonical entry point for
third-party Rust applications that don't want a runtime imposed.

## [Rust 0.2.0] - TBD

Second Rust crate release — clean-slate subscribe/unsubscribe API and
async-friendly channel surface.

### Breaking — `WebSocketClient` subscribe/unsubscribe API

Seven older methods are removed without a deprecation cycle (0.1.0 was
published 2026-05-15 with zero downstream usage):

- `subscribe(req: SubscribeRequest)`
- `subscribe_channel(sub: StockSubscription)`
- `subscribe_symbols(channel, &[&str], odd_lot)`
- `subscribe_futopt_channel(sub: FutOptSubscription)`
- `unsubscribe(key: &str)`
- `unsubscribe_channel(sub: &StockSubscription)`
- `unsubscribe_futopt_channel(sub: &FutOptSubscription)`
- `unsubscribe_by_id(id: &str)`

Replaced by three methods:

- `subscribe(StockSubscription)` — stock channels
- `subscribe_futopt(FutOptSubscription)` — FutOpt channels
- `unsubscribe(impl IntoIterator<Item = impl Into<String>>)` — single id or batch

`StockSubscription` / `FutOptSubscription` schema changes from `symbol: String`
to `symbols: SymbolSpec`. `StockSubscription::new(channel, symbols)` accepts
`&str`, `String`, `Vec<String>`, array literals (`["A", "B"]`), and slices via
`impl Into<SymbolSpec>`.

`SubscribeRequest` is no longer re-exported from `marketdata_core` — it's an
internal wire type. User code should not construct it directly.

### Added — true batch subscribe / unsubscribe

`StockSubscription::new(Channel::Trades, vec!["A", "B", "C"])` sends one frame
with `{"symbols": ["A","B","C"]}`, gets one ACK array back, and registers N
internal rows in `SubscriptionManager` (one local key per symbol). Previously
each symbol was a separate frame — the Fugle server gateway natively handles
both wire shapes (`stock.gateway.ts:13` and `futopt.gateway.ts:58`) so the
batch path is a real 1-frame-in / 1-ACK-out round-trip, not an N-frame loop.

### Added — async-friendly receive APIs

- `WebSocketClient::message_stream() -> tokio::sync::mpsc::Receiver<WebSocketMessage>`
  for pure-async Rust consumers. Avoids the std-mpsc bridge hop that `messages()`
  incurs.

Internal `message_tx` switches to `tokio::sync::mpsc::channel(1024)`. The
existing `messages()` API stays backward-compatible — first call lazily spawns
a bridge task that drains the tokio channel into a std mpsc for FFI bindings.

`messages()` and `message_stream()` are **mutually exclusive** — each takes
the receiver; calling the other afterwards panics.

### Changed — event channel bounded with drop semantics

`event_tx` switches from `std::sync::mpsc::channel()` (unbounded) to
`std::sync::mpsc::sync_channel(1024)`. All event emission goes through the new
internal `emit_event` helper which uses `try_send` and logs a stderr warning
on saturation. Saturation drops the **new** event (drop-newest) — drop-oldest
would require receiver-side access from the sender, which the
`Arc<Mutex<Receiver>>` public API does not expose without breaking callers.

### Binding changes

- **py / js**: external API unchanged (`subscribe({symbol|symbols})` and
  `unsubscribe({id|ids})` dicts still accepted). Internally the per-symbol
  loop is replaced by a single batch call to core, so the wire now sends
  one frame per `subscribe([...])` call instead of N.
- **UniFFI** (Java / Go / C#): unchanged in this release — UDL cannot express
  the generic `IntoIterator` signature. A dedicated batch surface
  (`subscribe_single` / `subscribe_many`) is planned for 0.3.0.

## [Rust 0.1.0] - 2026-05-15

Initial public release of the Rust SDK on crates.io. Two crates ship together:

- `fugle-marketdata-core` — internal kernel (also used by Python / Node.js /
  Java / Go / C# bindings via FFI)
- `fugle-marketdata` — user-facing facade; depend on this from your
  `Cargo.toml`

The Rust crate publishes on an independent 0.x track so the Rust API can
stabilize without being yoked to the unified 3.x release cadence for the
language-binding family. Once the public surface is judged stable, the crate
will graduate to 1.0.

All behavioral changes listed under [3.0.0] (especially the WebSocket
read-site liveness rework) apply equally to this release; the version split
is purely about release-cadence independence, not feature delta.

## [3.0.0] - TBD

Major release for the binding ecosystem — Python, Node.js, Java, Go, and C#
bindings bump together. The Rust crate publishes separately at 0.1.0 (see
above), sharing the same underlying core kernel.

### WebSocket connection liveness — read-site timeout (BREAKING)

The background activity-timer task is replaced with a `tokio::time::timeout`
wrapped at the WebSocket read site inside `dispatch_messages`. No more polling
task, no atomic timestamps, no `pause`/`resume` choreography during reconnect.
Detection latency improves from up to 90s (3 × 30s heartbeats missed) to the
configured `heartbeat_timeout` (default 35s).

#### Breaking

- `HealthCheckConfig` collapsed `interval` + `max_missed_pongs` into a single
  `heartbeat_timeout: Duration` field. Use
  `HealthCheckConfig::with_timeout(Duration::from_secs(35))?` to construct,
  or `HealthCheckConfig::default()` for the new 35s default.
- `HealthCheckConfig::enabled` default changed from `false` to `true`. Restore
  previous opt-out behaviour with `HealthCheckConfig::disabled()`.
- Removed the `HealthCheck` runtime struct (was `pub` but only used internally).
  All `touch` / `pause` / `resume` / `stop` / `spawn_check_task` / `ping`
  methods are gone — the read-site timeout doesn't need them.
- Removed constants `DEFAULT_HEALTH_CHECK_INTERVAL_MS`,
  `DEFAULT_HEALTH_CHECK_MAX_MISSED_PONGS`, `MIN_HEALTH_CHECK_INTERVAL_MS`.
  Replaced by `DEFAULT_HEARTBEAT_TIMEOUT_MS = 35_000` and
  `MIN_HEARTBEAT_TIMEOUT_MS = 5_000`.
- Binding-layer field renames (PyO3 / napi / UniFFI):
  - PyO3: `HealthCheckConfig.ping_interval` + `max_missed_pongs` →
    `heartbeat_timeout_ms`
  - napi: `HealthCheckOptions.ping_interval` + `max_missed_pongs` →
    `heartbeat_timeout_ms`
  - UniFFI: `HealthCheckConfigRecord.interval_ms` + `max_missed_pongs` →
    `heartbeat_timeout_ms`

#### Added

- `MarketDataError::HeartbeatTimeout { elapsed: Duration }` — first-class
  error variant for liveness timeout (error code 3003). PyO3 binding routes
  to the existing `TimeoutError` Python exception; UniFFI binding routes to
  the existing UniFFI `TimeoutError` variant.
- `ConnectionEvent::HeartbeatTimeout { elapsed: Duration }` — distinguishes
  "we stopped hearing from the server" from a server-initiated `Disconnected`
  close frame. Bindings reuse the existing disconnect callback path with a
  synthesized reason string for now; a dedicated `on_heartbeat_timeout`
  callback can be added in a follow-up if user code needs to discriminate.
- `AuthRequest.heartbeat_interval_ms` — wire-only optional field
  (`heartbeatIntervalMs` in JSON) for future client-requested heartbeat
  interval negotiation. Not exposed via builder method until server-side
  honoring lands; see `WEBSOCKET-SERVER-RECOMMENDATIONS.md`.

#### Changed

- WebSocket dispatch loop now uses `tokio::time::timeout(heartbeat_timeout,
  ws_read.next())` at the read site, replacing the background polling task.
- `WebSocketClient` storage shifts from `Arc<HealthCheck>` to
  `HealthCheckConfig` (plain owned value).

### Python (fugle-marketdata on PyPI)

Drop-in successor to the pure-Python `fugle-marketdata` 2.4.1 maintained at
[fugle-dev/fugle-marketdata-python](https://github.com/fugle-dev/fugle-marketdata-python).
`pip install -U fugle-marketdata` brings you to this Rust-based rewrite.

#### Changed (BREAKING)

- Import path renamed from `marketdata_py` to `fugle_marketdata`, matching
  the 2.4.1 convention. A `marketdata_py` shim emits `DeprecationWarning`
  and re-exports for one release; it will be removed in 3.1.0.
- Exceptions now anchored at `fugle_marketdata.*` (previously
  `marketdata_py.*`). Affects traceback display and pickling.

#### Added

- Version aligned with official 2.x series — this is the 3.0 major.

### Node.js / Java / C# / Go

All bindings bump from 0.3.x to 3.0.0 to share a unified SDK version across
the workspace. No API changes in this version beyond the Python-specific
rename above.

## [0.3.0] - 2026-02-16

### Added

- Options object constructor for all language bindings (Python kwargs-only, Node.js options object, Java builder, Go functional options, C# options pattern)
- ReconnectConfig/ReconnectionConfig exposure for WebSocket auto-reconnect control (max_attempts, initial_delay_ms, max_delay_ms)
- HealthCheckConfig/HealthCheckOptions exposure for WebSocket health check control (enabled, interval_ms, max_missed_pongs)
- Exactly-one-auth validation at construction time (Python ValueError, Node.js Error, Java FugleException, Go error, C# ArgumentException)
- Configuration validation at construction time with descriptive error messages
- Java builder pattern for client and config classes
- Go functional options pattern (WithApiKey, WithBearerToken, WithSdkToken)
- C# options pattern with nullable properties
- Configuration constants exported from core (DEFAULT_*, MIN_* constants for binding layers)

### Changed

- **BREAKING**: Python constructors now require kwargs-only parameters (`RestClient(api_key=)`, not `RestClient("key")`)
- **BREAKING**: Node.js constructors now require options object (`new RestClient({ apiKey })`, not `new RestClient('key')`)
- **BREAKING**: Java constructors now require builder pattern (`FugleRestClient.builder().apiKey().build()`)
- **BREAKING**: Go constructors now require functional options (`NewFugleRestClient(WithApiKey("key"))`)
- **BREAKING**: C# constructors now require options classes (`new RestClient(new RestClientOptions { ApiKey = "key" })`)
- Health check default changed from `true` to `false` (aligned with official SDKs)
- ReconnectConfig field rename: `max_retries` → `max_attempts`, `base_delay_ms` → `initial_delay_ms`

### Deprecated

- Python: Positional string constructors (`RestClient("key")`, removed in v0.4.0)
- Python: Static methods `.with_bearer_token()` and `.with_sdk_token()` (removed in v0.4.0)
- Node.js: String constructors (`new RestClient('key')`, removed in v0.4.0)

## [0.2.0] - 2026-01-31

### Added

- Multi-language SDK support (Python, Node.js, C#, Java, Go)
- Complete REST API coverage (26+ endpoints across stock and futures/options)
  - Stock intraday: quote, ticker, candles, trades, volumes
  - Stock historical: candles, stats
  - Stock snapshot: quotes, movers, actives
  - Stock technical: SMA, RSI, KDJ, MACD, Bollinger Bands
  - Stock corporate actions: capital changes, dividends, listing applicants
  - FutOpt intraday: quote, ticker, candles, trades, volumes, products
  - FutOpt historical: candles, daily
- WebSocket streaming with automatic reconnection and exponential backoff
- WebSocket health check monitoring (ping-pong)
- Async support for all language bindings
  - Python: async/await with asyncio
  - Node.js: Promise-based API
  - C#: Task-based async
  - Java: CompletableFuture
  - Go: goroutines and channels
- Type definitions
  - TypeScript: Full .d.ts definitions for Node.js
  - Python: PEP 484 type stubs (.pyi files)
- Error handling with consistent error codes across all languages
- Three authentication methods: API key, bearer token, SDK token
- FFI bindings via PyO3 (Python), napi-rs (Node.js), UniFFI (Java/Go/C#)

[unreleased]: https://github.com/yourusername/fugle-marketdata-sdk/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/yourusername/fugle-marketdata-sdk/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/yourusername/fugle-marketdata-sdk/releases/tag/v0.2.0
