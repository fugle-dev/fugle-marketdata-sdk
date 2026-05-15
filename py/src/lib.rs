//! Python bindings for marketdata-core
//!
//! This crate provides Python FFI bindings for the Fugle market data library.
//! Built on PyO3 0.27 with async runtime support via pyo3-async-runtimes.
//!
//! # REST Client Example (Python)
//!
//! ```python
//! from fugle_marketdata import RestClient
//!
//! # Create client with API key
//! client = RestClient("your-api-key")
//!
//! # Get stock quote (blocking)
//! quote = client.stock.intraday.quote("2330")
//!
//! # Get futures quote
//! futopt_quote = client.futopt.intraday.quote("TXFC4")
//! ```
//!
//! # WebSocket Client Example (Python)
//!
//! ```python
//! from fugle_marketdata import WebSocketClient
//!
//! # Create WebSocket client
//! ws = WebSocketClient("your-api-key")
//!
//! # Callback mode (sync)
//! def on_message(msg):
//!     print(f"Received: {msg}")
//!
//! ws.stock.on("message", on_message)
//! ws.stock.connect()
//! ws.stock.subscribe("trades", "2330")
//!
//! # Or iterator mode (blocking)
//! for msg in ws.stock.messages():
//!     print(msg)
//! ```
//!
//! # Async Support (Future Enhancement)
//!
//! PyO3 0.27 with pyo3-async-runtimes enables native Python asyncio integration:
//!
//! ```python
//! # Future async API (Plan 02-02)
//! import asyncio
//! from fugle_marketdata import AsyncRestClient, AsyncWebSocketClient
//!
//! async def main():
//!     # Async REST client
//!     client = AsyncRestClient("your-api-key")
//!     quote = await client.stock.intraday.quote("2330")
//!
//!     # Async WebSocket client
//!     ws = AsyncWebSocketClient("your-api-key")
//!     await ws.stock.connect()
//!     await ws.stock.subscribe("trades", "2330")
//!
//!     async for msg in ws.stock:
//!         print(msg)
//!
//! asyncio.run(main())
//! ```
//!
//! # Exception Hierarchy
//!
//! ```python
//! from fugle_marketdata import (
//!     MarketDataError,  # Base exception
//!     ApiError,         # API request failed
//!     RateLimitError,   # Rate limit exceeded (extends ApiError)
//!     AuthError,        # Authentication failed
//!     ConnectionError,  # Connection failed
//!     TimeoutError,     # Operation timed out
//!     WebSocketError,   # WebSocket operation failed
//! )
//!
//! try:
//!     quote = client.stock.intraday.quote("INVALID")
//! except ApiError as e:
//!     print(f"API Error {e.args[1]}: {e.args[0]}")
//! except MarketDataError as e:
//!     print(f"Error {e.args[1]}: {e.args[0]}")
//! ```

use pyo3::prelude::*;

mod callback;
mod client;
mod errors;
pub mod iterator;
mod tls_kwargs;
mod types;
mod websocket;

/// Python module for marketdata-core bindings
#[pymodule]
fn fugle_marketdata(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Register the main RestClient class
    m.add_class::<client::RestClient>()?;

    // Register stock client classes
    m.add_class::<client::StockClient>()?;
    m.add_class::<client::StockIntradayClient>()?;
    m.add_class::<client::StockHistoricalClient>()?;
    m.add_class::<client::StockSnapshotClient>()?;
    m.add_class::<client::StockTechnicalClient>()?;
    m.add_class::<client::StockCorporateActionsClient>()?;

    // Register futopt client classes
    m.add_class::<client::FutOptClient>()?;
    m.add_class::<client::FutOptIntradayClient>()?;
    m.add_class::<client::FutOptHistoricalClient>()?;

    // Register WebSocket client classes
    m.add_class::<websocket::WebSocketClient>()?;
    m.add_class::<websocket::StockWebSocketClient>()?;
    m.add_class::<websocket::FutOptWebSocketClient>()?;
    m.add_class::<websocket::ReconnectConfig>()?;
    m.add_class::<websocket::HealthCheckConfig>()?;

    // Register iterator class
    m.add_class::<iterator::MessageIterator>()?;

    // Register exception hierarchy
    m.add("MarketDataError", m.py().get_type::<errors::MarketDataError>())?;
    m.add("ApiError", m.py().get_type::<errors::ApiError>())?;
    m.add("RateLimitError", m.py().get_type::<errors::RateLimitError>())?;
    m.add("AuthError", m.py().get_type::<errors::AuthError>())?;
    m.add("ConnectionError", m.py().get_type::<errors::ConnectionError>())?;
    m.add("TimeoutError", m.py().get_type::<errors::TimeoutError>())?;
    m.add("WebSocketError", m.py().get_type::<errors::WebSocketError>())?;

    // Backward-compat alias: old fugle-marketdata SDK exposed a single `FugleAPIError`.
    // Aliasing it to MarketDataError lets `except FugleAPIError:` catch every variant
    // raised by this binding so legacy try/except blocks keep working.
    m.add("FugleAPIError", m.py().get_type::<errors::MarketDataError>())?;

    Ok(())
}
