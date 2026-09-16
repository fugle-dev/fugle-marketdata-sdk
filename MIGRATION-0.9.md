# Migrating to 0.9.0 (bindings 3.0.0-rc.2, uniffi 0.2.0-rc.1)

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
body, err := client.Stock().Intraday().GetQuote("2330")
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
not the source of truth.

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
