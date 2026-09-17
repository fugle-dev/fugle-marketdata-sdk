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

Control WebSocket automatic reconnection behavior:

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

**ReconnectOptions:**

- `maxAttempts` (number): Maximum reconnection attempts (default: 5, min: 1)
- `initialDelayMs` (number): Initial delay for exponential backoff (default: 1000, min: 100)
- `maxDelayMs` (number): Maximum delay cap (default: 60000)

### Health Check Options

Control WebSocket health check (ping-pong) behavior:

```javascript
const { WebSocketClient } = require('@fugle/marketdata');

const ws = new WebSocketClient({
  apiKey: 'your-key',
  healthCheck: {
    enabled: true,
    pingInterval: 15000,
    maxMissedPongs: 3
  }
});
```

**HealthCheckOptions:**

- `enabled` (boolean): Whether health check is enabled (default: false)
- `pingInterval` (number): Ping interval in milliseconds (default: 30000, min: 5000)
- `maxMissedPongs` (number): Maximum missed pongs before considering connection stale (default: 2, min: 1)

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

### Combined Configuration

```javascript
const { WebSocketClient } = require('@fugle/marketdata');

const ws = new WebSocketClient({
  apiKey: 'your-key',
  reconnect: { maxAttempts: 10, initialDelayMs: 2000 },
  healthCheck: { enabled: true, pingInterval: 15000 }
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
  ping(params?: string | { state?: unknown }): void;
  subscribe(options: { channel: string; symbol: string; oddLot?: boolean }): void;
  unsubscribe(subscriptionId: string): void;
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
| 1005 | InvalidParameter | Invalid or missing parameter (including an unknown WebSocket channel) |
| 2001 | ConnectionError | Network connection failed |
| 2002 | AuthError | Authentication failed |
| 2003 | ApiError | API returned an error |
| 2010 | ClientClosed | Client already closed |
| 2011 | AlreadyConnected | WebSocket `connect()` called while connected or connecting (Node only) |
| 3001 | TimeoutError | Operation timed out |
| 3002 | WebSocketError | WebSocket connect, read or write failed |
| 3003 | HeartbeatTimeout | No inbound WebSocket frame within the heartbeat window |
| 3004 | CallbackFailed | A WebSocket listener threw, or its Promise rejected (`error` event) |
| 3005 | ReconnectFailed | Reconnection failed after the last attempt (`error` event) |
| 9999 | Other | Unexpected error |
| -1 | ThreadPanic | A WebSocket worker thread panicked |

## License

MIT
