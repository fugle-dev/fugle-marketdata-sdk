# @fugle/marketdata

Fugle market data REST and WebSocket client for Node.js and TypeScript,
powered by a Rust core through NAPI-RS.

## Installation

```bash
npm install @fugle/marketdata@next   # 3.x pre-release
```

Prebuilt native addons are published for macOS (x64, arm64), Linux glibc
(x64, arm64) and Windows x64. npm installs the matching one automatically.

### From source

```bash
# Requires a Rust toolchain
cd js
npm install
npm run build
```

## Quick Start

### REST API

```javascript
const { RestClient } = require('@fugle/marketdata');

// Create client with API key
const client = new RestClient({ apiKey: 'your-api-key' });

// REST methods return Promises
const quote = await client.stock.intraday.quote({ symbol: '2330' });
console.log('TSMC Price:', quote.closePrice);

// Get futures quote
const futoptQuote = await client.futopt.intraday.quote('TXFC4');
console.log('TXF Price:', futoptQuote.closePrice);
```

### WebSocket Streaming

```javascript
const { WebSocketClient } = require('@fugle/marketdata');

// Create client
const ws = new WebSocketClient({ apiKey: 'your-api-key' });

// Register handlers
ws.stock.on('message', (data) => {
  const msg = JSON.parse(data);
  console.log('Message:', msg);
});

ws.stock.on('connect', () => {
  console.log('Socket open, authenticating...');
});

ws.stock.on('authenticated', (data) => {
  console.log('Authenticated:', data.message);
});

ws.stock.on('disconnect', ({ code, reason }) => {
  console.log('Disconnected:', code, reason);
});

ws.stock.on('error', (err) => {
  console.error('Error:', err.code, err.message);
});

// A listener that throws (or whose Promise rejects) does not crash the
// process or stop later events: it is reported through `error` with code
// 3004, `err.event` (the listener's event), `err.count` and `err.cause`
// (what was thrown) — the first at once, then at most once per second.
// Without an `error` listener it is printed with console.error.

// Connect: resolves with the server's authenticated data, or rejects with
// the server's data object when the credentials are rejected
await ws.stock.connect();
ws.stock.subscribe({ channel: 'trades', symbol: '2330' });

// Disconnect after 30 seconds
setTimeout(() => {
  ws.stock.disconnect();
}, 30000);
```

### TypeScript

```typescript
import { RestClient, WebSocketClient } from '@fugle/marketdata';

const client = new RestClient({ apiKey: 'your-api-key' });
const quote = await client.stock.intraday.quote({ symbol: '2330' });
// quote is typed as QuoteResponse
```

## Authentication

Three authentication methods are supported:

```javascript
const { RestClient } = require('@fugle/marketdata');

// 1. API Key (most common)
const client = new RestClient({ apiKey: 'your-api-key' });

// 2. Bearer Token
const client = new RestClient({ bearerToken: 'your-bearer-token' });

// 3. SDK Token
const client = new RestClient({ sdkToken: 'your-sdk-token' });
```

## Configuration

### Reconnection Options

Auto-reconnect is on by default: after an unexpected drop the client
reconnects with exponential backoff, without an attempt limit (waits capped
at `maxDelayMs`), and subscribes again. Pass `reconnect` to tune it:

```javascript
const { WebSocketClient } = require('@fugle/marketdata');

const ws = new WebSocketClient({
  apiKey: 'your-key',
  reconnect: {
    maxAttempts: 10,
    initialDelayMs: 2000,
    maxDelayMs: 120000
  }
});
```

To turn it off: `new WebSocketClient({ apiKey: 'your-key', reconnect: { enabled: false } })`.

**ReconnectOptions:**

- `enabled` (boolean): Whether auto-reconnect is enabled (default: true)
- `maxAttempts` (number): Maximum reconnection attempts; 0 means unlimited
  (default: 0). With a limit, the `error` event reports code 3005 once the
  last attempt fails
- `initialDelayMs` (number): Initial delay for exponential backoff (default: 1000, min: 100)
- `maxDelayMs` (number): Maximum delay cap (default: 60000)

### Health Check Options

Liveness detection is on by default: when no inbound frame (data, heartbeat or
pong) arrives within `heartbeatTimeoutMs`, the connection is declared dead and
auto-reconnect takes over. The server sends a heartbeat every 30 seconds.

With `probeEnabled`, a silent connection is asked before it is declared dead:
after `idleProbeAfterMs` of silence one ping is sent, and only if nothing
arrives within `probeTimeoutMs` is the connection declared dead. The defaults
keep detection at 35 seconds and send no ping while the server's heartbeat is
on time, so turning on `probeEnabled` alone only removes false disconnects
caused by a late heartbeat.

```javascript
const { WebSocketClient } = require('@fugle/marketdata');

const ws = new WebSocketClient({
  apiKey: 'your-key',
  healthCheck: { heartbeatTimeoutMs: 60000 }, // or { enabled: false } to turn it off
});

// Confirm before disconnecting; know within 10 seconds
const fast = new WebSocketClient({
  apiKey: 'your-key',
  healthCheck: { probeEnabled: true, idleProbeAfterMs: 5000, probeTimeoutMs: 5000 },
});

// Round trip on demand, in milliseconds (default timeout 5000)
const rtt = await fast.stock.measureLatency();
```

**HealthCheckOptions:**

- `enabled` (boolean): Whether health check is enabled (default: true)
- `heartbeatTimeoutMs` (number): Maximum gap between inbound frames before the
  connection is declared dead (default: 35000, min: 5000). **Does not apply
  when `probeEnabled` is true.**
- `probeEnabled` (boolean): Confirm with a ping before declaring the connection
  dead (default: false). Detection is `idleProbeAfterMs + probeTimeoutMs`.
- `idleProbeAfterMs` (number): Silence before the ping (default: 30000, the
  server's heartbeat period; min: 5000). Below 30000 a ping is sent in every
  gap between heartbeats while no data flows.
- `probeTimeoutMs` (number): Wait for any inbound frame after the ping
  (default: 5000, min: 1000)

Probing does not detect a half-open connection (the server still sends, our
writes no longer arrive). See [docs/configuration.md](../docs/configuration.md#healthcheckconfig--healthcheckoptions)
for the trade-offs and the server cost of short probe intervals.

### Message Queue

```javascript
const ws = new WebSocketClient({
  apiKey: 'your-key',
  messageOverflow: 'dropNewest', // default; or 'unbounded'
  messageBuffer: 4096,           // unread messages held (default 4096)
});

ws.stock.on('messagesDropped', ({ dropped, total }) => {
  console.warn(`dropped ${dropped} (${total} on this connection)`);
});
ws.stock.messagesDroppedTotal; // this connection's drops; still readable after disconnect()
```

A `message` listener that falls behind holds up delivery: once
`messageBuffer` frames are waiting for it, the SDK stops handing over more and
lets them queue, up to another `messageBuffer`. With `'dropNewest'`, frames
beyond that are dropped, counted and reported through `messagesDropped` (at
most once per second, and before `disconnect`). Events queued behind those
frames wait too, since the SDK keeps messages and events in order.
`'unbounded'` never drops; memory grows for as long as listeners lag.

While a listener keeps delivery held up, events can be lost too: the SDK holds
up to 1024 unread events (connection events, errors and `messagesDropped`
reports) separately from messages, and drops any beyond that. This takes a
listener that stays blocked for a long time, since `messagesDropped` is
reported at most once per second.

### Auth Timeout

```javascript
const ws = new WebSocketClient({
  apiKey: 'your-key',
  authTimeoutMs: 15000, // auth handshake limit (default 10000)
});
```

Once the WebSocket is open the client sends its auth frame and waits for the
server's verdict for `authTimeoutMs` (default 10 s). It applies to the first
`connect()` and to every reconnect; elapsing it fails the attempt with a
`TimeoutError` (code 3001). Raise it on a slow route to the server; the
server itself allows 60 s. It must be greater than 0 (a config error, code
1004, otherwise).

### Combined Configuration

```javascript
const { WebSocketClient } = require('@fugle/marketdata');

const ws = new WebSocketClient({
  apiKey: 'your-key',
  reconnect: { maxAttempts: 10, initialDelayMs: 2000 },
  healthCheck: { probeEnabled: true, idleProbeAfterMs: 10000 },
  authTimeoutMs: 15000
});
```

## API Reference

### RestClient

```typescript
class RestClient {
  constructor(options: RestClientOptions);

  stock: {
    intraday: {
      quote(symbol: string | { symbol: string; oddLot?: boolean }): Promise<QuoteResponse>;
      ticker(symbol: string): Promise<TickerResponse>;
      candles(symbol: string, timeframe?: string): Promise<CandlesResponse>;
      trades(symbol: string): Promise<TradesResponse>;
      volumes(symbol: string): Promise<VolumesResponse>;
    }
    // also: historical, snapshot, technical, corporateActions, ownership
  };

  futopt: {
    intraday: {
      quote(symbol: string): Promise<QuoteResponse>;
      ticker(symbol: string): Promise<TickerResponse>;
      candles(symbol: string, timeframe: string): Promise<CandlesResponse>;
      trades(symbol: string): Promise<TradesResponse>;
      volumes(symbol: string): Promise<VolumesResponse>;
      products(type: 'FUTURE' | 'OPTION', contractType?: string): Promise<ProductsResponse>;
    }
    // also: historical
  };

  // See index.d.ts for the complete, generated type definitions.
}
```

**Object form and query parameters.** Every REST method also takes a single
object, the call shape of the 1.x SDK: the path param (`symbol`, or `market`
for `stock.snapshot.*`) goes into the path and every other key is a query
parameter, under the API's own name (`isTrial`, `contractMonth`,
`type: 'oddlot'`). The keys are checked against the endpoint's parameter
list (core's `rest::params`, built from the server's request definitions)
before the request is sent:

```javascript
await client.stock.intraday.trades({ symbol: '2330', limit: 5, isTrial: true });
await client.stock.intraday.ticker({ symbol: '2330', type: 'oddlot' });     // or oddLot: true
await client.futopt.intraday.tickers({ type: 'FUTURE', product: 'TXF' });

await client.stock.intraday.trades({ symbol: '2330', istrial: true });
// rejects: Invalid parameter 'istrial': `stock.intraday.trades` does not accept `istrial`;
//          did you mean `isTrial`? accepted keys: symbol, type, offset, limit, sort, isTrial
//          (err.code === 1005, err.sourceKind === 'client'; the suggestion appears when
//          the key differs only in case or underscores)
```

The `Rest*Params` types list exactly the accepted keys — there is no
`[key: string]: unknown` — so in TypeScript a typo is a compile error too.
The snake_case spellings (`is_trial`, `odd_lot`) are accepted at runtime as
aliases. Values are sent as given; the server reports a bad value.

**RestClientOptions:**

```typescript
interface RestClientOptions {
  apiKey?: string;        // API key for authentication
  bearerToken?: string;   // Bearer token for authentication
  sdkToken?: string;      // SDK token for authentication
  baseUrl?: string;       // Override base URL (optional)
}
```

Exactly one of `apiKey`, `bearerToken`, or `sdkToken` must be provided. An empty or whitespace-only value counts as not provided; otherwise the constructor throws an error with `code` 1004.

### WebSocketClient

```typescript
class WebSocketClient {
  constructor(options: WebSocketClientOptions);

  stock: StockWebSocketClient;
  futopt: FutOptWebSocketClient;
}

class StockWebSocketClient {
  on<E extends WebSocketEvent>(event: E, callback: WebSocketEventMap[E]): void;
  connect(): Promise<WebSocketAuthData | undefined>;
  ping(params?: string | { state?: unknown }): void;   // fire and forget; pong via 'message'
  measureLatency(timeoutMs?: number): Promise<number>; // round trip in ms
  subscribe(options: { channel: string; symbol: string; oddLot?: boolean }): void;
  // A server id, { id } / { ids }, or the subscribe() options (FutOpt: afterHours)
  unsubscribe(options: string | { id?: string; ids?: string[] } | { channel: string; symbol?: string; symbols?: string[]; intradayOddLot?: boolean }): void;
  disconnect(): void;
  get isConnected(): boolean;
  get isClosed(): boolean;
}

// FutOptWebSocketClient has the same API
```

**WebSocketClientOptions:**

```typescript
interface WebSocketClientOptions {
  apiKey?: string;                     // API key for authentication
  bearerToken?: string;                // Bearer token for authentication
  sdkToken?: string;                   // SDK token for authentication
  baseUrl?: string;                    // Override base URL (optional)
  reconnect?: ReconnectOptions;        // Reconnection configuration (optional)
  healthCheck?: HealthCheckOptions;    // Health check configuration (optional)
  messageOverflow?: 'dropNewest' | 'unbounded'; // While messageBuffer are unread (default 'dropNewest')
  messageBuffer?: number;              // Unread messages held (default 4096)
  authTimeoutMs?: number;              // Auth handshake limit in ms (default 10000; > 0)
}
```

## Error Handling

```javascript
const { RestClient } = require('@fugle/marketdata');

const client = new RestClient({ apiKey: 'your-api-key' });

try {
  const quote = await client.stock.intraday.quote('2330');
} catch (e) {
  if (e.code === 2002) {
    console.log('Authentication failed', e.status, e.body);
  } else if (e.sourceKind === 'rate_limit') {
    console.log('Throttled; retry after', e.headers['retry-after']);
  } else {
    console.error('Error:', e.code, e.message);
  }
}
```

Errors thrown by constructors, rejected by REST methods and `connect()`, and
passed to the WebSocket `error` event carry the same fields
(TypeScript: `MarketDataError`):

| Property | Type | |
|---|---|---|
| `code` | `number` | Error code (table below) |
| `sourceKind` | `'network' \| 'protocol' \| 'auth' \| 'rate_limit' \| 'client'` | Category of the failure |
| `message` | `string` | Human-readable message, without a `[code]` prefix |
| `status` | `number \| null` | HTTP status |
| `body` | `string \| null` | Raw HTTP response body (REST) |
| `requestId` | `string \| null` | `x-request-id` response header |
| `headers` | `Record<string, string>` | HTTP response headers, lowercase names (REST) |

`connect()` rejected because the server refused the credentials rejects with
the server's data instead. The
[error reference](https://github.com/fugle-dev/fugle-marketdata-sdk/blob/main/docs/errors.md) has the same table for every language.

## Custom TLS / self-signed servers

For connecting to servers with a private CA (enterprise deployments) or
self-signed certs (dev / staging), both `RestClient` and
`WebSocketClient` accept optional TLS fields:

```javascript
const { readFileSync } = require('fs');

// Pin a custom CA (production-safe when your server cert has proper
// SANs and is issued by this CA).
const client = new RestClient({
  apiKey: 'your-api-key',
  tlsRootCertPem: readFileSync('/path/to/ca.crt'),
});

// Disable ALL TLS verification — dev / testing only. Exposes MITM risk.
const insecureClient = new RestClient({
  apiKey: 'your-api-key',
  baseUrl: 'wss://192.0.2.1/v1.0',
  tlsAcceptInvalidCerts: true,
});
```

Same fields work on `WebSocketClient`. When neither field is set the
client uses the OS trust store (rustls loads it via
`rustls-native-certs`).

## Error Codes

| Code | Error | Description |
|------|-------|-------------|
| 1001 | InvalidSymbol | Invalid symbol format |
| 1002 | DeserializationError | Failed to parse a response or WebSocket frame |
| 1003 | RuntimeError | Internal runtime error |
| 1004 | ConfigError | Invalid configuration |
| 1005 | InvalidParameter | Invalid or missing parameter (an unknown WebSocket channel, or a key the REST endpoint does not take in the object form) |
| 2001 | ConnectionError | Network connection failed |
| 2002 | AuthError | Authentication failed |
| 2003 | ApiError | API returned an error |
| 2010 | ClientClosed | Client already closed |
| 2011 | AlreadyConnected | WebSocket `connect()` called while connected or connecting |
| 3001 | TimeoutError | Operation timed out |
| 3002 | WebSocketError | WebSocket connect, read or write failed |
| 3003 | HeartbeatTimeout | No inbound WebSocket frame within the heartbeat window |
| 3004 | CallbackFailed | A WebSocket listener threw, or its Promise rejected (`error` event) |
| 3005 | ReconnectFailed | Reconnection failed after the last attempt (`error` event) |
| 9999 | Other | Unexpected error |
| -1 | ThreadPanic | A WebSocket worker thread panicked |

## License

MIT
