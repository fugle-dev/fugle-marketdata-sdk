# marketdata-py

Python bindings for Fugle market data streaming. Built with PyO3 for high-performance native integration.

## Installation

### Development Build

```bash
# Create virtual environment
python -m venv .venv
source .venv/bin/activate

# Install maturin
pip install maturin

# Build and install in development mode
cd marketdata-py
maturin develop
```

### Production Build

```bash
maturin build --release
pip install target/wheels/fugle_marketdata-*.whl
```

## Quick Start

### REST API

```python
from fugle_marketdata import RestClient, MarketDataError

# Create client with API key
client = RestClient(api_key="your-api-key")

# Get stock quote
quote = client.stock.intraday.quote("2330")
print(f"TSMC Price: {quote['closePrice']}")
print(f"Change: {quote['change']}")
print(f"Volume: {quote['total']['tradeVolume']}")

# Get stock ticker info
ticker = client.stock.intraday.ticker("2330")
print(f"Name: {ticker['name']}")

# Get intraday candles (5-minute)
candles = client.stock.intraday.candles("2330", timeframe="5")
for candle in candles['data'][:3]:
    print(f"  {candle['time']}: O={candle['open']} H={candle['high']} L={candle['low']} C={candle['close']}")

# Get recent trades
trades = client.stock.intraday.trades("2330")
for trade in trades['data'][:5]:
    print(f"  Price: {trade['price']}, Size: {trade['size']}")

# FutOpt (futures/options) data
futopt_quote = client.futopt.intraday.quote("TXFC4")
print(f"Futures Price: {futopt_quote['closePrice']}")
```

### WebSocket Streaming

```python
from fugle_marketdata import WebSocketClient
import time

# Create WebSocket client
ws = WebSocketClient(api_key="your-api-key")

# --- Callback Mode ---

def on_message(msg):
    """Handle incoming messages"""
    if msg.get('event') == 'data':
        channel = msg.get('channel')
        symbol = msg.get('symbol')
        data = msg.get('data', {})
        print(f"[{channel}] {symbol}: {data}")

def on_connect():
    print("Connected!")

def on_disconnect(code, reason):
    print(f"Disconnected: {code} - {reason}")

def on_error(message, code):
    print(f"Error [{code}]: {message}")

# Register callbacks
stock = ws.stock
stock.on("message", on_message)
stock.on("connect", on_connect)
stock.on("disconnect", on_disconnect)
stock.on("error", on_error)

# Connect and subscribe
stock.connect()
stock.subscribe("trades", "2330")
stock.subscribe("books", "2330")

# Keep running for 10 seconds
time.sleep(10)

# Disconnect
stock.disconnect()

# --- Iterator Mode ---

ws2 = WebSocketClient(api_key="your-api-key")
stock2 = ws2.stock
stock2.connect()
stock2.subscribe("trades", "2330")

# Iterate over messages
for msg in stock2.messages():
    print(msg)
    # Add break condition as needed
```

## Authentication

Three authentication methods are supported:

```python
from fugle_marketdata import RestClient

# 1. API Key (most common)
client = RestClient(api_key="your-api-key")

# 2. Bearer Token
client = RestClient(bearer_token="your-bearer-token")

# 3. SDK Token
client = RestClient(sdk_token="your-sdk-token")
```

## Configuration

### Reconnection Config

Control WebSocket automatic reconnection behavior:

```python
from fugle_marketdata import WebSocketClient, ReconnectConfig

# Create custom reconnect configuration
reconnect = ReconnectConfig(
    enabled=True,
    max_attempts=10,
    initial_delay_ms=2000,
    max_delay_ms=120000
)

ws = WebSocketClient(api_key="your-key", reconnect=reconnect)
```

**ReconnectConfig Options:**
- `enabled` (bool): Whether auto-reconnect is enabled (default: True)
- `max_attempts` (int): Maximum reconnection attempts (default: 5, min: 1)
- `initial_delay_ms` (int): Initial delay for exponential backoff (default: 1000ms, min: 100ms)
- `max_delay_ms` (int): Maximum delay cap (default: 60000ms)

### Health Check Config

Control WebSocket health check (ping-pong) behavior:

```python
from fugle_marketdata import WebSocketClient, HealthCheckConfig

# Create custom health check configuration
health_check = HealthCheckConfig(
    enabled=True,
    interval_ms=15000,
    max_missed_pongs=3
)

ws = WebSocketClient(api_key="your-key", health_check=health_check)
```

**HealthCheckConfig Options:**
- `enabled` (bool): Whether health check is enabled (default: False)
- `interval_ms` (int): Ping interval in milliseconds (default: 30000ms, min: 5000ms)
- `max_missed_pongs` (int): Maximum missed pongs before considering connection stale (default: 2, min: 1)

### Combined Configuration

```python
from fugle_marketdata import WebSocketClient, ReconnectConfig, HealthCheckConfig

reconnect = ReconnectConfig(max_attempts=10, initial_delay_ms=2000)
health_check = HealthCheckConfig(enabled=True, interval_ms=15000)

ws = WebSocketClient(
    api_key="your-key",
    reconnect=reconnect,
    health_check=health_check
)
```

## API Reference

### RestClient

#### Stock Intraday Methods

```python
client.stock.intraday.quote(symbol)        # Real-time quote
client.stock.intraday.ticker(symbol)       # Symbol information
client.stock.intraday.candles(symbol, timeframe="1")  # OHLCV candles
client.stock.intraday.trades(symbol)       # Trade history
client.stock.intraday.volumes(symbol)      # Volume by price
```

#### FutOpt Intraday Methods

```python
client.futopt.intraday.quote(symbol)       # Real-time quote
client.futopt.intraday.ticker(symbol)      # Contract information
client.futopt.intraday.candles(symbol, timeframe="1")  # OHLCV candles
client.futopt.intraday.trades(symbol)      # Trade history
client.futopt.intraday.volumes(symbol)     # Volume by price
client.futopt.intraday.products(type)      # Product listing ("F" or "O")
```

### WebSocketClient

#### Properties

```python
ws.stock    # Access StockWebSocketClient
ws.futopt   # Access FutOptWebSocketClient
```

#### StockWebSocketClient / FutOptWebSocketClient Methods

```python
client.connect()                           # Connect to server
client.disconnect()                        # Disconnect from server
client.is_connected()                      # Check connection status
client.is_closed()                         # Check if client is closed

client.subscribe(channel, symbol)          # Subscribe to channel
client.unsubscribe(subscription_id)        # Unsubscribe by ID
client.subscriptions()                     # List active subscriptions

client.on(event, callback)                 # Register event callback
client.off(event)                          # Unregister callback

client.messages()                          # Get message iterator
```

#### Event Types

| Event | Callback Signature | Description |
|-------|-------------------|-------------|
| `message` | `fn(msg: dict)` | Incoming data message |
| `connect` | `fn()` | Connection established |
| `disconnect` | `fn(code: int, reason: str)` | Connection closed |
| `error` | `fn(message: str, code: int)` | Error occurred |

#### Channels

| Channel | Description |
|---------|-------------|
| `trades` | Real-time trade executions |
| `candles` | Candlestick updates |
| `books` | Order book (5 levels) |
| `aggregates` | Aggregated market data |
| `indices` | Index values (stock only) |

### MessageIterator

```python
# Get iterator from connected client
messages = stock.messages()

# Iterate (blocking)
for msg in messages:
    print(msg)

# Manual iteration
msg = next(messages)          # Blocking
msg = messages.try_recv()     # Non-blocking, returns None if no message
msg = messages.recv_timeout(5.0)  # Timeout in seconds
```

## Error Handling

All API errors raise `MarketDataError`:

```python
from fugle_marketdata import RestClient, MarketDataError

client = RestClient(api_key="invalid-key")

try:
    quote = client.stock.intraday.quote("2330")
except MarketDataError as e:
    message = e.args[0]
    error_code = e.args[1]
    print(f"Error [{error_code}]: {message}")
```

### Error Codes

| Code | Error Type | Description |
|------|------------|-------------|
| 1001 | InvalidSymbol | Invalid or unsupported symbol |
| 1002 | DeserializationError | JSON parsing failed |
| 1003 | RuntimeError | Internal runtime error |
| 1004 | ConfigError | Configuration error |
| 2001 | ConnectionError | Network connection failed |
| 2002 | AuthError | Authentication failed |
| 2003 | ApiError | API returned error response |
| 2010 | ClientClosed | Client has been closed |
| 3001 | TimeoutError | Operation timed out |
| 3002 | WebSocketError | WebSocket protocol error |
| 9999 | Other | Unexpected error |

## Examples

### Full REST Example

```python
from fugle_marketdata import RestClient, MarketDataError
import os

def main():
    api_key = os.environ.get("FUGLE_API_KEY")
    if not api_key:
        print("Set FUGLE_API_KEY environment variable")
        return

    client = RestClient(api_key=api_key)

    try:
        # Stock data
        print("=== Stock Market Data ===")
        quote = client.stock.intraday.quote("2330")
        print(f"TSMC Quote: {quote['closePrice']}")

        ticker = client.stock.intraday.ticker("2330")
        print(f"Ticker: {ticker['name']}")

        candles = client.stock.intraday.candles("2330", timeframe="5")
        print(f"Candles: {len(candles['data'])} entries")

        # FutOpt data
        print("\n=== FutOpt Market Data ===")
        products = client.futopt.intraday.products("F")
        print(f"Futures products: {len(products['data'])}")

    except MarketDataError as e:
        print(f"Error [{e.args[1]}]: {e.args[0]}")

if __name__ == "__main__":
    main()
```

### Full WebSocket Example

```python
from fugle_marketdata import WebSocketClient, MarketDataError
import os
import time

def main():
    api_key = os.environ.get("FUGLE_API_KEY")
    if not api_key:
        print("Set FUGLE_API_KEY environment variable")
        return

    ws = WebSocketClient(api_key=api_key)
    stock = ws.stock

    message_count = 0

    def on_message(msg):
        nonlocal message_count
        message_count += 1
        if msg.get('event') == 'data':
            print(f"[{message_count}] {msg.get('channel')}: {msg.get('symbol')}")

    def on_connect():
        print("Connected!")

    def on_error(message, code):
        print(f"Error [{code}]: {message}")

    stock.on("message", on_message)
    stock.on("connect", on_connect)
    stock.on("error", on_error)

    try:
        stock.connect()
        stock.subscribe("trades", "2330")
        stock.subscribe("books", "2330")

        print("Listening for 10 seconds...")
        time.sleep(10)

        print(f"\nReceived {message_count} messages")
        print(f"Subscriptions: {stock.subscriptions()}")

    except MarketDataError as e:
        print(f"Error [{e.args[1]}]: {e.args[0]}")
    finally:
        if stock.is_connected():
            stock.disconnect()
        print("Done")

if __name__ == "__main__":
    main()
```

## Requirements

- Python 3.8+
- Rust toolchain (for building from source)
- maturin (for development builds)

## License

MIT
