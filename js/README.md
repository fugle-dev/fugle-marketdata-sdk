# marketdata-js

JavaScript/TypeScript bindings for Fugle Market Data API, built with NAPI-RS.

## Installation

```bash
# From source (requires Rust toolchain)
cd marketdata-js
npm install
npm run build

# The native module is built to marketdata-js.*.node
```

## Quick Start

### REST API

```javascript
const { RestClient } = require('@fubon/marketdata-js');

// Create client with API key
const client = new RestClient({ apiKey: 'your-api-key' });

// Get stock quote
const quote = client.stock.intraday.quote('2330');
console.log('TSMC Price:', quote.closePrice);

// Get futures quote
const futoptQuote = client.futopt.intraday.quote('TXFC4');
console.log('TXF Price:', futoptQuote.closePrice);
```

### WebSocket Streaming

```javascript
const { WebSocketClient } = require('@fubon/marketdata-js');

// Create client
const ws = new WebSocketClient({ apiKey: 'your-api-key' });

// Register handlers
ws.stock.on('message', (data) => {
  const msg = JSON.parse(data);
  console.log('Message:', msg);
});

ws.stock.on('connect', () => {
  console.log('Connected!');
  ws.stock.subscribe({ channel: 'trades', symbol: '2330' });
});

ws.stock.on('disconnect', (reason) => {
  console.log('Disconnected:', reason);
});

ws.stock.on('error', (err) => {
  console.error('Error:', err);
});

// Connect
ws.stock.connect();

// Disconnect after 30 seconds
setTimeout(() => {
  ws.stock.disconnect();
}, 30000);
```

### TypeScript

```typescript
import { RestClient, WebSocketClient } from '@fubon/marketdata-js';

const client = new RestClient({ apiKey: 'your-api-key' });
const quote = client.stock.intraday.quote('2330');
// quote is typed as Record<string, any>
```

## Authentication

Three authentication methods are supported:

```javascript
const { RestClient } = require('@fubon/marketdata-js');

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
const { WebSocketClient } = require('@fubon/marketdata-js');

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
const { WebSocketClient } = require('@fubon/marketdata-js');

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

### Combined Configuration

```javascript
const { WebSocketClient } = require('@fubon/marketdata-js');

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
      quote(symbol: string): Record<string, any>;
      ticker(symbol: string): Record<string, any>;
      candles(symbol: string): Record<string, any>;
      trades(symbol: string): Record<string, any>;
      volumes(symbol: string): Record<string, any>;
    }
  };

  futopt: {
    intraday: {
      quote(symbol: string): Record<string, any>;
      ticker(symbol: string): Record<string, any>;
      candles(symbol: string): Record<string, any>;
      trades(symbol: string): Record<string, any>;
      volumes(symbol: string): Record<string, any>;
      products(type: 'futures' | 'options'): Record<string, any>;
    }
  };
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

Exactly one of `apiKey`, `bearerToken`, or `sdkToken` must be provided.

### WebSocketClient

```typescript
class WebSocketClient {
  constructor(options: WebSocketClientOptions);

  stock: StockWebSocketClient;
  futopt: FutOptWebSocketClient;
}

class StockWebSocketClient {
  on(event: 'message' | 'connect' | 'disconnect' | 'error', handler: Function): void;
  connect(): void;
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
}
```

## Error Handling

```javascript
const { RestClient } = require('@fubon/marketdata-js');

const client = new RestClient({ apiKey: 'your-api-key' });

try {
  const quote = client.stock.intraday.quote('INVALID');
} catch (e) {
  if (e.message.includes('[2010]')) {
    console.log('Client already closed');
  } else if (e.message.includes('[2002]')) {
    console.log('Authentication failed');
  } else {
    console.error('Error:', e.message);
  }
}
```

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
| 1002 | DeserializationError | Failed to parse response |
| 2001 | ConnectionError | Network connection failed |
| 2002 | AuthError | Authentication failed |
| 2003 | ApiError | API returned an error |
| 2010 | ClientClosed | Client already closed |
| 3001 | TimeoutError | Operation timed out |
| 3002 | WebSocketError | WebSocket protocol error |

## License

MIT
