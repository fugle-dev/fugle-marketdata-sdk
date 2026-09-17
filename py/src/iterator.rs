//! Python iterator for WebSocket messages
//!
//! Provides both sync (__iter__/__next__) and async (__aiter__/__anext__) iterator protocols.
//! Sync iteration blocks with optional timeout. Async iteration releases GIL during receive.

use pyo3::prelude::*;
use pyo3_async_runtimes::tokio::future_into_py;
use std::sync::Arc;
use std::time::Duration;

use crate::handoff::Handoff;
use crate::websocket::message_to_dict;

/// Python iterator for WebSocket messages
///
/// Implements both sync (__iter__/__next__) and async (__aiter__/__anext__) iterator protocols.
/// Reads the messages the connection's stream reader hands over when no
/// `message` callback is registered (#68).
///
/// # Example (Python)
///
/// ```python
/// # Sync iteration (blocks waiting for messages)
/// for msg in ws.stock.messages():
///     print(msg)
///
/// # Async iteration (releases GIL, modern Python)
/// async for msg in ws.stock.messages():
///     print(msg)
///
/// # With timeout (returns None on timeout instead of blocking forever)
/// for msg in ws.stock.messages(timeout_ms=1000):
///     if msg is None:
///         print("Timeout, no message received")
///         continue
///     print(msg)
/// ```
///
/// # GIL Safety
///
/// Every wait releases the GIL: the connection's stream reader needs it to
/// run the lifecycle callbacks queued ahead of the next message.
///
/// # Backpressure
///
/// At most `message_buffer` unread messages are held for iterators. While
/// that many are, the reader stops taking items from the connection, so
/// lifecycle callbacks (`disconnect`, `reconnect`, ...) queued after those
/// messages wait until the iterator reads or `disconnect()` is called.
#[pyclass]
pub struct MessageIterator {
    handoff: Arc<Handoff>,
    timeout: Option<Duration>,
}

impl MessageIterator {
    /// Create a new message iterator
    pub(crate) fn new(handoff: Arc<Handoff>, timeout: Option<Duration>) -> Self {
        Self { handoff, timeout }
    }
}

#[pymethods]
impl MessageIterator {
    /// Return self as iterator (required for Python iteration protocol)
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    /// Get next message from stream
    ///
    /// Returns:
    ///     dict: Message data containing event, channel, symbol, data fields
    ///     None: If timeout specified and no message received within timeout
    ///
    /// Raises:
    ///     StopIteration: When channel is closed (connection disconnected)
    ///
    /// Note: This method blocks the current thread while waiting for
    /// messages, with the GIL released.
    fn __next__(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let handoff = Arc::clone(&self.handoff);
        let timeout = self.timeout;
        let result = py.detach(move || handoff.receive(timeout));

        match result {
            Ok(Some(msg)) => Ok(message_to_dict(py, &msg)?.into_any()),
            Ok(None) => {
                // Timeout: yield None and keep iterating. Returning a Rust
                // `None` here would end the iteration, as pyo3 maps an empty
                // `Option` from `__next__` to StopIteration.
                Ok(py.None())
            }
            Err(_) => {
                // Channel closed, stop iteration
                Err(pyo3::exceptions::PyStopIteration::new_err(
                    "Message channel closed",
                ))
            }
        }
    }

    /// Try to receive a message without blocking
    ///
    /// Returns:
    ///     dict: Message data if available
    ///     None: If no message available
    fn try_recv(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        match self.handoff.try_receive() {
            Some(msg) => {
                let dict = message_to_dict(py, &msg)?;
                Ok(Some(dict.into_any()))
            }
            None => Ok(None),
        }
    }

    /// Receive a message with timeout
    ///
    /// Args:
    ///     timeout_ms: Timeout in milliseconds
    ///
    /// Returns:
    ///     dict: Message data if received within timeout
    ///     None: If timeout elapsed with no message
    ///
    /// Raises:
    ///     MarketDataError: If channel is closed
    fn recv_timeout(&self, py: Python<'_>, timeout_ms: u64) -> PyResult<Option<Py<PyAny>>> {
        let handoff = Arc::clone(&self.handoff);
        let timeout = Duration::from_millis(timeout_ms);
        match py.detach(move || handoff.receive(Some(timeout))) {
            Ok(Some(msg)) => {
                let dict = message_to_dict(py, &msg)?;
                Ok(Some(dict.into_any()))
            }
            Ok(None) => Ok(None),
            Err(()) => Err(crate::errors::to_py_err(marketdata_core::MarketDataError::ConnectionError {
                msg: "Message channel closed".to_string(),
            })),
        }
    }

    /// Return self as async iterator (required for Python async iteration protocol)
    ///
    /// This enables `async for msg in iterator:` syntax in Python.
    fn __aiter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    /// Get next message from stream (async)
    ///
    /// Returns:
    ///     dict: Message data containing event, channel, symbol, data fields
    ///     None: If timeout specified and no message received within timeout
    ///
    /// Raises:
    ///     StopAsyncIteration: When channel is closed (connection disconnected)
    ///
    /// Note: This method releases the GIL while waiting for messages.
    /// The wait runs on tokio's blocking pool, enabling true async concurrency.
    fn __anext__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let handoff = Arc::clone(&self.handoff);
        let timeout = self.timeout;

        future_into_py(py, async move {
            let result = tokio::task::spawn_blocking(move || handoff.receive(timeout))
            .await
            .map_err(|e| {
                pyo3::exceptions::PyRuntimeError::new_err(format!("Task join error: {}", e))
            })?;

            match result {
                Ok(Some(msg)) => {
                    // Convert to Python dict with GIL
                    Python::attach(|py| {
                        let dict = message_to_dict(py, &msg)?;
                        Ok(Some(dict.into_any()))
                    })
                }
                Ok(None) => {
                    // Timeout - return None without stopping iteration
                    Ok(None)
                }
                Err(_) => {
                    // Channel closed: end `async for`.
                    Err(pyo3::exceptions::PyStopAsyncIteration::new_err(
                        "Message channel closed",
                    ))
                }
            }
        })
    }
}

// Note: Tests for MessageIterator require Python runtime and must be run via maturin develop + pytest
// See test_websocket.py for Python-side iterator tests
