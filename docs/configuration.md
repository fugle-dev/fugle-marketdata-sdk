# Configuration Reference

This document provides comprehensive reference for all WebSocket configuration options available in the Fugle Market Data SDK across all supported languages.

**Key Principles:**

- All configuration validation happens at construction time (fail-fast)
- Invalid configurations throw errors immediately, not at connection time
- All config options have sensible defaults

---

## ReconnectConfig / ReconnectOptions

Controls automatic reconnection behavior after WebSocket connection drops.

**Auto-reconnect is on by default in every language** (3.0.0, #149): a client
created without a reconnect config reconnects after an unexpected drop,
retrying without an attempt limit, and subscribes again once it is back. Pass
a reconnect config only to tune it or to turn it off.

### Options Reference

| Option | Type | Default | Min | Max | Description |
|--------|------|---------|-----|-----|-------------|
| `enabled` | bool | true | - | - | Whether auto-reconnect is active (Go: use `WithoutReconnect()`) |
| `max_attempts` | u32/int/number | 0 (unlimited) | 0 | - | Maximum reconnection attempts before giving up; 0 means never give up |
| `initial_delay_ms` | u64/int/number | 1000 | 100 | - | Initial backoff delay in milliseconds |
| `max_delay_ms` | u64/int/number | 60000 | >= initial_delay_ms | - | Maximum backoff delay cap in milliseconds |

**Constraints:**

- `initial_delay_ms` must be >= 100ms (prevent connection storms)
- `max_delay_ms` must be >= `initial_delay_ms` (logical constraint)

**Backoff Strategy:** Exponential backoff with jitter (0-15%). Delay doubles on
each attempt until hitting the `max_delay_ms` cap. With the defaults the waits
are about 1s, 2s, 4s, 9s, 18s, 33s, then about once a minute until the
connection is back.

**Giving up:** only with a non-zero `max_attempts`. After that many failed
attempts the client emits `ReconnectFailed` (Node/Python: `error` code 3005;
C#/Go/Java/C++: `OnReconnectFailed`) and stays closed. The server closing with
1000 (normal) or a 4xxx code (e.g. auth failure) never triggers a reconnect.

### Language-Specific Examples

#### Python

```python
from fugle_marketdata import WebSocketClient, ReconnectConfig

# Default: auto-reconnect on, unlimited attempts
ws = WebSocketClient(api_key="your-api-key")

# Custom reconnect configuration
reconnect = ReconnectConfig(
    max_attempts=10,
    initial_delay_ms=2000,
    max_delay_ms=120000
)
ws = WebSocketClient(api_key="your-api-key", reconnect=reconnect)

# Turn auto-reconnect off
ws = WebSocketClient(api_key="your-api-key", reconnect=ReconnectConfig.disabled())
```

#### JavaScript/TypeScript

```typescript
import { WebSocketClient } from '@fugle/marketdata';

// Default: auto-reconnect on, unlimited attempts
const ws = new WebSocketClient({ apiKey: 'your-api-key' });

// Custom reconnect configuration
const ws = new WebSocketClient({
  apiKey: 'your-api-key',
  reconnect: {
    maxAttempts: 10,
    initialDelayMs: 2000,
    maxDelayMs: 120000,
  },
});

// Turn auto-reconnect off
const ws = new WebSocketClient({ apiKey: 'your-api-key', reconnect: { enabled: false } });
```

#### Java

```java
import tw.com.fugle.marketdata.*;

// Default: auto-reconnect on, unlimited attempts
FugleWebSocketClient client = FugleWebSocketClient.builder()
    .apiKey("your-api-key")
    .stock()
    .build();

// Custom reconnect configuration
FugleWebSocketClient client = FugleWebSocketClient.builder()
    .apiKey("your-api-key")
    .stock()
    .reconnect(ReconnectOptions.builder()
        .maxAttempts(10)
        .initialDelayMs(2000L)
        .maxDelayMs(120000L)
        .build())
    .build();

// Turn auto-reconnect off
FugleWebSocketClient client = FugleWebSocketClient.builder()
    .apiKey("your-api-key")
    .stock()
    .reconnect(ReconnectOptions.builder().enabled(false).build())
    .build();
```

#### Go

```go
import mkt "github.com/fugle-dev/fugle-marketdata-go"

// Default: auto-reconnect on, unlimited attempts
client, err := mkt.NewFugleWebSocketClient(listener,
    mkt.WithApiKey("your-api-key"),
)

// Custom reconnect configuration
client, err := mkt.NewFugleWebSocketClient(listener,
    mkt.WithApiKey("your-api-key"),
    mkt.WithReconnect(mkt.ReconnectConfig{
        MaxAttempts:    10,
        InitialDelayMs: 2000,
        MaxDelayMs:     120000,
    }),
)

// Turn auto-reconnect off
client, err := mkt.NewFugleWebSocketClient(listener,
    mkt.WithApiKey("your-api-key"),
    mkt.WithoutReconnect(),
)
```

`ReconnectConfig` has no on/off field: its zero values mean "use default", so
`WithReconnect(ReconnectConfig{MaxAttempts: 10})` keeps auto-reconnect on.
Between `WithReconnect` and `WithoutReconnect`, the last option given wins.

#### C\#

```csharp
using FugleMarketData;

// Default: auto-reconnect on, unlimited attempts
using var client = new WebSocketClient(
    new WebSocketClientOptions { ApiKey = "your-api-key" }, listener);

// Custom reconnect configuration
using var client = new WebSocketClient(new WebSocketClientOptions
{
    ApiKey = "your-api-key",
    Reconnect = new ReconnectOptions
    {
        MaxAttempts = 10,
        InitialDelayMs = 2000,
        MaxDelayMs = 120000,
    },
}, listener);

// Turn auto-reconnect off
using var client = new WebSocketClient(new WebSocketClientOptions
{
    ApiKey = "your-api-key",
    Reconnect = new ReconnectOptions { Enabled = false },
}, listener);
```

#### C++

The generated `ReconnectConfigRecord` is passed as-is: pass `std::nullopt` or
`ReconnectConfigRecord{}` for the defaults, or a record with `.enabled = false`
to turn auto-reconnect off. `enabled` is a `std::optional<bool>` that is unset
by default, meaning "use default" (on), and zero numeric fields mean "use
default" too.

---

## HealthCheckConfig / HealthCheckOptions

Controls WebSocket liveness detection. **Enabled by default in every language**
(3.0): when the connection stays silent too long it is declared dead
(`HeartbeatTimeout`, then `Disconnected` with intent `Network`) and
auto-reconnect takes over (see [ReconnectConfig](#reconnectconfig--reconnectoptions)).
The server sends a heartbeat every 30 seconds. *Any* inbound frame — data,
heartbeat or pong — counts as a sign of life.

### Options Reference

| Option | Type | Default | Min | Applies | Description |
|--------|------|---------|-----|---------|-------------|
| `enabled` | bool | true | - | both modes | Whether liveness detection is active (Go: use `WithoutHealthCheck()`) |
| `heartbeat_timeout_ms` | u64/int/number | 35000 | 5000 | **`probe_enabled: false` only** | Maximum gap between inbound frames before the connection is declared dead |
| `probe_enabled` | bool | false | - | - | Confirm a silent connection with a ping instead of declaring it dead on a timeout |
| `idle_probe_after_ms` | u64/int/number | 30000 | 5000 | `probe_enabled: true` only | Silence before the probe is sent |
| `probe_timeout_ms` | u64/int/number | 5000 | 1000 | `probe_enabled: true` only | Wait for any inbound frame after the probe |

JavaScript spells them `heartbeatTimeoutMs`, `probeEnabled`, `idleProbeAfterMs`
and `probeTimeoutMs`; Java, Go and C# use the same names in their own casing.

**Constraints:**

- A value below its minimum is a configuration error (code 1004), whether or
  not its mode is in use.
- In passive mode, `heartbeat_timeout_ms` values below the server's 30 s
  heartbeat period cause repeated false disconnects.

### Passive and probe mode

| `probe_enabled` | What happens | Detection time | Verdict |
|---|---|---|---|
| `false` (default) | Silent for `heartbeat_timeout_ms` → dead | `heartbeat_timeout_ms` (35 s) | a guess |
| `true` | Silent for `idle_probe_after_ms` → one `{"event":"ping"}` is sent; nothing arrives within `probe_timeout_ms` → dead | `idle_probe_after_ms + probe_timeout_ms` (35 s) | confirmed |

**With `probe_enabled: true`, `heartbeat_timeout_ms` does not apply.**

In passive mode a server heartbeat that is more than 5 s late is
indistinguishable from a dead connection, so the SDK disconnects a healthy
connection. Probe mode asks first. Its defaults are chosen so that **turning on
`probe_enabled` and nothing else costs nothing**:

- `idle_probe_after_ms` defaults to 30000, the server's heartbeat period. While
  heartbeats arrive on time the silence never reaches it, and **no ping is
  sent at all**.
- Only when a heartbeat is late — exactly the case that causes false
  disconnects in passive mode — is one ping sent. A live server answers and the
  connection stays up; a dead one does not, and the connection is declared
  dead.
- Detection stays at 30 s + 5 s = 35 s, the same as passive mode.

For faster detection, lower `idle_probe_after_ms` (and, if you like,
`probe_timeout_ms`). While market data flows the silence never builds up and no
ping is sent, so an active subscription gets fast detection for free. Detecting
faster than the server's own heartbeat is impossible without traffic, though:
below 30000, **a ping is sent in every gap between heartbeats** whenever no data
flows (after hours, quiet symbols).

A probe that cannot even be written within `probe_timeout_ms` (the write path
is stuck) counts as unanswered: the connection is declared dead on time.

**Not covered: a half-open connection** where the server still sends but our
writes no longer reach it. The server's heartbeat is a broadcast and keeps
arriving, resetting the silence, so the probe never fires.

### Server cost

Every probe is a request the server handles and records. Estimated load from
**10 000 concurrent connections** during a quiet period:

| `idle_probe_after_ms` | Pings per 30 s gap, per connection | 10 000 connections |
|---|---|---|
| 30000 (default) | about 0 (only when a heartbeat is late) | close to 0 |
| 15000 | about 1 | about 330 / s |
| 10000 | about 2 | about 670 / s |
| 5000 | about 5 | about 1700 / s |

Until the server stops recording each pong as an action (fugle-realtime #731),
5000 is **not recommended** for deployments with tens of thousands of
connections.

### Measuring latency

Every client also has `measure_latency` (`measureLatency` in JavaScript,
`MeasureLatencyAsync` in C#, `MeasureLatency` in Go, `measureLatency` in Java):
it sends one ping, waits for its pong and returns the round trip —
`Duration` in Rust, milliseconds elsewhere. It works whether or not
`probe_enabled` is set and sends nothing in the background; call it when you
want to know (before placing an order, when your own watchdog fires). The
timeout defaults to 5000 ms. It fails with `ClientClosed` (2010) when not
connected, `ConnectionError` (2001) when the connection closes before the pong,
and `TimeoutError` (3001) when no pong arrives in time.

The existing `ping()` is unchanged: fire and forget, as in the old SDK, with the
pong delivered to your message handler. The pongs of the SDK's own pings (the
probe and `measure_latency`) are not delivered.

### Language-Specific Examples

#### Python

```python
from fugle_marketdata import WebSocketClient, HealthCheckConfig

# Default: enabled, 35s timeout
ws = WebSocketClient(api_key="your-api-key")

# Longer timeout
ws = WebSocketClient(api_key="your-api-key",
                     health_check=HealthCheckConfig(heartbeat_timeout_ms=60000))

# Confirm before disconnecting: still 35s, no pings while heartbeats are on time
ws = WebSocketClient(api_key="your-api-key",
                     health_check=HealthCheckConfig(probe_enabled=True))

# Know within 10s
ws = WebSocketClient(api_key="your-api-key",
                     health_check=HealthCheckConfig(probe_enabled=True,
                                                    idle_probe_after_ms=5000,
                                                    probe_timeout_ms=5000))

# Turn it off
ws = WebSocketClient(api_key="your-api-key", health_check=HealthCheckConfig(enabled=False))
```

#### JavaScript/TypeScript

```typescript
import { WebSocketClient } from '@fugle/marketdata';

// Default: enabled, 35s timeout
const ws = new WebSocketClient({ apiKey: 'your-api-key' });

// Longer timeout
const ws = new WebSocketClient({ apiKey: 'your-api-key', healthCheck: { heartbeatTimeoutMs: 60000 } });

// Confirm before disconnecting: still 35s, no pings while heartbeats are on time
const ws = new WebSocketClient({ apiKey: 'your-api-key', healthCheck: { probeEnabled: true } });

// Know within 10s
const ws = new WebSocketClient({
  apiKey: 'your-api-key',
  healthCheck: { probeEnabled: true, idleProbeAfterMs: 5000, probeTimeoutMs: 5000 },
});

// Turn it off
const ws = new WebSocketClient({ apiKey: 'your-api-key', healthCheck: { enabled: false } });
```

#### Java

```java
// Longer timeout (default: enabled, 35s)
FugleWebSocketClient client = FugleWebSocketClient.builder()
    .apiKey("your-api-key")
    .stock()
    .healthCheck(HealthCheckOptions.builder().heartbeatTimeoutMs(60000L).build())
    .build();

// Turn it off: HealthCheckOptions.builder().enabled(false).build()
// Probe mode: HealthCheckOptions.builder().probeEnabled(true).idleProbeAfterMs(10000L).build()
```

#### Go

```go
// Longer timeout (default: enabled, 35s)
client, err := mkt.NewFugleWebSocketClient(listener,
    mkt.WithApiKey("your-api-key"),
    mkt.WithHealthCheck(mkt.HealthCheckConfig{HeartbeatTimeoutMs: 60000}),
)

// Turn it off: mkt.WithoutHealthCheck()
// Probe mode: mkt.HealthCheckConfig{ProbeEnabled: true, IdleProbeAfterMs: 10000}
```

#### C\#

```csharp
// Longer timeout (default: enabled, 35s)
using var client = new WebSocketClient(new WebSocketClientOptions
{
    ApiKey = "your-api-key",
    HealthCheck = new HealthCheckOptions { HeartbeatTimeoutMs = 60000 },
}, listener);

// Turn it off: HealthCheck = new HealthCheckOptions { Enabled = false }
// Probe mode: HealthCheck = new HealthCheckOptions { ProbeEnabled = true, IdleProbeAfterMs = 10000 }
```

---

## Authentication Options

All clients require exactly one authentication method. An empty or whitespace-only value counts as not provided. Providing zero or multiple authentication methods results in a configuration error (code 1004, see [errors.md](errors.md)) at construction time.

### Options Reference

| Option | Type | Description |
|--------|------|-------------|
| `api_key` / `apiKey` / `ApiKey` | string | Fugle API key |
| `bearer_token` / `bearerToken` / `BearerToken` | string | Bearer token for OAuth authentication |
| `sdk_token` / `sdkToken` / `SdkToken` | string | SDK token for partner integrations |
| `base_url` / `baseUrl` / `BaseUrl` | string (optional) | Custom API base URL (for testing or private deployments) |

**Constraint:** Exactly one non-empty `api_key`, `bearer_token`, or `sdk_token` must be provided.

### Language-Specific Examples

#### Python

```python
from marketdata_py import RestClient

# API key authentication
client = RestClient(api_key="your-api-key")

# Bearer token authentication
client = RestClient(bearer_token="your-bearer-token")

# SDK token authentication
client = RestClient(sdk_token="your-sdk-token")

# Custom base URL (optional)
client = RestClient(api_key="your-api-key", base_url="https://custom.api.url")
```

#### JavaScript/TypeScript

```typescript
import { RestClient } from '@fugle/marketdata';

// API key authentication
const client = new RestClient({ apiKey: 'your-api-key' });

// Bearer token authentication
const client = new RestClient({ bearerToken: 'your-bearer-token' });

// SDK token authentication
const client = new RestClient({ sdkToken: 'your-sdk-token' });

// Custom base URL (optional)
const client = new RestClient({
  apiKey: 'your-api-key',
  baseUrl: 'https://custom.api.url',
});
```

#### Java

```java
import tw.com.fugle.marketdata.*;

// API key authentication
FugleRestClient client = FugleRestClient.builder()
    .apiKey("your-api-key")
    .build();

// Bearer token authentication
FugleRestClient client = FugleRestClient.builder()
    .bearerToken("your-bearer-token")
    .build();

// SDK token authentication
FugleRestClient client = FugleRestClient.builder()
    .sdkToken("your-sdk-token")
    .build();
```

#### Go

```go
import marketdata "github.com/fugle-dev/fugle-marketdata-go"

// API key authentication
client, err := marketdata.NewFugleRestClient(
    marketdata.WithApiKey("your-api-key"),
)

// Bearer token authentication
client, err := marketdata.NewFugleRestClient(
    marketdata.WithBearerToken("your-bearer-token"),
)

// SDK token authentication
client, err := marketdata.NewFugleRestClient(
    marketdata.WithSdkToken("your-sdk-token"),
)
```

#### C\#

```csharp
using MarketdataUniffi;

// API key authentication
var client = new RestClient(new RestClientOptions {
    ApiKey = "your-api-key"
});

// Bearer token authentication
var client = new RestClient(new RestClientOptions {
    BearerToken = "your-bearer-token"
});

// SDK token authentication
var client = new RestClient(new RestClientOptions {
    SdkToken = "your-sdk-token"
});
```

---

## Validation Error Messages

When configuration validation fails, you'll see one of these error messages:

### Authentication Errors

**"Configuration error: Provide exactly one non-empty credential: API key, bearer token, or SDK token"**

- **Cause:** Zero or multiple authentication methods provided, or the only one is empty or whitespace
- **Error:** code 1004 (`client`) — Python `MarketDataError`, Node.js `Error`, C# `MarketDataException`, Go `*MarketDataError`, Java `FugleException`
- **Solution:** Pass exactly one non-empty auth method

**Example (Python):**

```python
# ✗ Wrong - no auth
client = RestClient()
# MarketDataError: Configuration error: Provide exactly one non-empty credential: ...

# ✗ Wrong - multiple auth
client = RestClient(api_key="key", bearer_token="token")
# MarketDataError: Configuration error: Provide exactly one non-empty credential: ...

# ✗ Wrong - empty key
client = RestClient(api_key="")
# MarketDataError: Configuration error: Provide exactly one non-empty credential: ...

# ✓ Correct - exactly one auth
client = RestClient(api_key="key")
```

### ReconnectConfig Errors

**"initial_delay_ms must be >= 100ms (got {value}ms)"**

- **Cause:** `initial_delay_ms` less than minimum 100ms
- **Solution:** Use at least 100ms delay

**"max_delay_ms ({value}ms) must be >= initial_delay_ms ({value}ms)"**

- **Cause:** `max_delay_ms` less than `initial_delay_ms`
- **Solution:** Ensure `max_delay_ms` >= `initial_delay_ms`

**Example (JavaScript):**

```typescript
// ✗ Wrong - initial delay too small
const ws = new WebSocketClient({
  apiKey: 'key',
  reconnect: { initialDelayMs: 50 }  // Error: must be >= 100ms
});

// ✗ Wrong - max_delay less than initial_delay
const ws = new WebSocketClient({
  apiKey: 'key',
  reconnect: { initialDelayMs: 5000, maxDelayMs: 2000 }
  // Error: max_delay_ms must be >= initial_delay_ms
});
```

### HealthCheckConfig Errors

**"heartbeat_timeout must be >= 5000ms (got {value})"**

- **Cause:** `heartbeat_timeout_ms` less than the 5000ms floor
- **Solution:** Use at least 5000ms; below 30000ms the server's heartbeat period causes false disconnects

**"idle_probe_after must be >= 5000ms (got {value})"** /
**"probe_timeout must be >= 1000ms (got {value})"**

- **Cause:** a probe setting below its floor (checked even with `probe_enabled` off)
- **Solution:** Use at least 5000ms / 1000ms

**Example (Python):**

```python
# ✗ Wrong - timeout below the floor
health_check = HealthCheckConfig(heartbeat_timeout_ms=2000)
# ValueError: Configuration error: heartbeat_timeout must be >= 5000ms (got 2s)
```

---

## Defaults Summary

Quick reference of all default values:

| Configuration | Option | Default Value | Notes |
|---------------|--------|---------------|-------|
| **Reconnect** | `enabled` | true | On in every language (3.0.0) |
| | `max_attempts` | 0 | Unlimited |
| | `initial_delay_ms` | 1000 | 1 second |
| | `max_delay_ms` | 60000 | 1 minute |
| **Health Check** | `enabled` | true | On in every language (3.0) |
| | `heartbeat_timeout_ms` | 35000 | Server heartbeat (30 s) + 5 s; passive mode only |
| | `probe_enabled` | false | Opt-in |
| | `idle_probe_after_ms` | 30000 | Server heartbeat (30 s); probe mode only |
| | `probe_timeout_ms` | 5000 | Probe mode only |

**Default values sourced from:**

- `core/src/websocket/reconnection.rs` constants
- `core/src/websocket/health_check.rs` constants

---

## Additional Resources

- [MIGRATION.md](../MIGRATION.md) - Migration guide from v0.2.x
- [CHANGELOG.md](../CHANGELOG.md) - Full changelog with version history
- `examples/` directory - Working code examples for all languages
