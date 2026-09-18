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

The generated `ReconnectConfigRecord` is passed as-is: pass `std::nullopt` for
the defaults, or a record with `enabled = false` to turn auto-reconnect off.
Zero numeric fields mean "use default".

---

## HealthCheckConfig / HealthCheckOptions

Controls WebSocket health monitoring via ping/pong messages.

### Options Reference

| Option | Type | Default | Min | Max | Description |
|--------|------|---------|-----|-----|-------------|
| `enabled` | bool | false | - | - | Enable WebSocket health monitoring |
| `interval_ms` | u64/int/number | 30000 | 5000 | - | Ping interval in milliseconds |
| `max_missed_pongs` | u64/int/number | 2 | 1 | - | Missed pongs before reconnect |

**Constraints:**

- `interval_ms` must be >= 5000ms (5 seconds) to prevent excessive overhead
- `max_missed_pongs` must be >= 1 (at least one missed pong required to trigger reconnect)
- Implicit constraint: timeout must be less than interval

**Default Behavior:**

- Health check is **disabled by default** (aligned with official Fugle SDKs)
- Must explicitly enable if monitoring is needed
- When enabled, sends ping every `interval_ms` milliseconds
- Triggers reconnect if `max_missed_pongs` consecutive pongs are missed

### Language-Specific Examples

#### Python

```python
from marketdata_py import WebSocketClient, HealthCheckConfig

# Health check disabled by default
ws = WebSocketClient(api_key="your-api-key")

# Enable health check with defaults
health_check = HealthCheckConfig(enabled=True)
ws = WebSocketClient(api_key="your-api-key", health_check=health_check)

# Custom health check configuration
health_check = HealthCheckConfig(
    enabled=True,
    interval_ms=15000,
    max_missed_pongs=3
)
ws = WebSocketClient(api_key="your-api-key", health_check=health_check)
```

#### JavaScript/TypeScript

```typescript
import { WebSocketClient } from '@fugle/marketdata';

// Health check disabled by default
const ws = new WebSocketClient({ apiKey: 'your-api-key' });

// Enable health check with custom config
const ws = new WebSocketClient({
  apiKey: 'your-api-key',
  healthCheck: {
    enabled: true,
    intervalMs: 15000,
    maxMissedPongs: 3,
  },
});
```

#### Java

```java
import tw.com.fugle.marketdata.*;

// Health check disabled by default
FugleRestClient client = FugleRestClient.builder()
    .apiKey("your-api-key")
    .build();

// Enable health check with custom config
HealthCheckOptions healthCheck = new HealthCheckOptions.Builder()
    .enabled(true)
    .intervalMs(15000L)
    .maxMissedPongs(3)
    .build();

FugleRestClient client = FugleRestClient.builder()
    .apiKey("your-api-key")
    .healthCheckOptions(healthCheck)
    .build();
```

#### Go

```go
import marketdata "github.com/fugle-dev/fugle-marketdata-go"

// Health check disabled by default
client, err := marketdata.NewFugleRestClient(
    marketdata.WithApiKey("your-api-key"),
)

// Enable health check with custom config
client, err := marketdata.NewFugleRestClient(
    marketdata.WithApiKey("your-api-key"),
    marketdata.WithHealthCheckOptions(marketdata.HealthCheckOptions{
        Enabled:         true,
        IntervalMs:      15000,
        MaxMissedPongs:  3,
    }),
)
```

#### C\#

```csharp
using MarketdataUniffi;

// Health check disabled by default
var client = new RestClient(new RestClientOptions {
    ApiKey = "your-api-key"
});

// Enable health check with custom config
var healthCheck = new HealthCheckOptions {
    Enabled = true,
    IntervalMs = 15000,
    MaxMissedPongs = 3,
};

var client = new RestClient(new RestClientOptions {
    ApiKey = "your-api-key",
    HealthCheckOptions = healthCheck
});
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

**"health_check interval must be >= 5000ms (got {value}ms)"**

- **Cause:** `interval_ms` less than minimum 5000ms (5 seconds)
- **Solution:** Use at least 5000ms interval

**"max_missed_pongs must be >= 1"**

- **Cause:** `max_missed_pongs` set to 0
- **Solution:** Use at least 1 missed pong

**Example (Python):**

```python
# ✗ Wrong - interval too small
health_check = HealthCheckConfig(enabled=True, interval_ms=2000)
# ConfigError: health_check interval must be >= 5000ms (got 2000ms)

# ✗ Wrong - max_missed_pongs is 0
health_check = HealthCheckConfig(enabled=True, max_missed_pongs=0)
# ConfigError: max_missed_pongs must be >= 1
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
| **Health Check** | `enabled` | false | Must explicitly enable |
| | `interval_ms` | 30000 | 30 seconds |
| | `max_missed_pongs` | 2 | |

**Default values sourced from:**

- `core/src/websocket/reconnection.rs` constants
- `core/src/websocket/health_check.rs` constants

---

## Additional Resources

- [MIGRATION.md](../MIGRATION.md) - Migration guide from v0.2.x
- [CHANGELOG.md](../CHANGELOG.md) - Full changelog with version history
- `examples/` directory - Working code examples for all languages
