//! Python iterator for WebSocket messages
//!
//! Provides both sync (__iter__/__next__) and async (__aiter__/__anext__) iterator protocols.
//! Both yield messages only and stop only once the connection is gone (#68).
//! A `messages(raw=True)` iterator yields the text of each frame instead of a
//! dict (#246).

use pyo3::prelude::*;
use pyo3::types::{PyString, PyType};
use pyo3_async_runtimes::tokio::future_into_py;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::handoff::Handoff;
use crate::websocket::message_to_dict;

/// How often a waiting iterator wakes up: to notice the connection closing
/// and, when iterating synchronously, to let Python handle signals (Ctrl+C).
const WAKE_INTERVAL: Duration = Duration::from_millis(100);

/// Python iterator for WebSocket messages
///
/// Implements both sync (__iter__/__next__) and async (__aiter__/__anext__) iterator protocols.
/// Reads the messages the connection's stream reader hands over when no
/// `message` or `raw_message` callback is registered (#68).
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
/// ```
///
/// Iteration yields messages only: it waits while none arrive, never yields
/// `None`, and stops only once the connection is gone. For periodic work
/// while no data arrives, use `message` callbacks or `async for` with your
/// own tasks.
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
    /// Yield the frame's text instead of a dict. The queue holds unparsed
    /// frames, so each iterator chooses for itself.
    raw: bool,
}

impl MessageIterator {
    /// Create a new message iterator
    pub(crate) fn new(handoff: Arc<Handoff>, raw: bool) -> Self {
        Self { handoff, raw }
    }
}

/// What an iterator yields for `msg`: the frame's text when `raw`, with no
/// dict built from it, else the dict.
fn yielded(py: Python<'_>, msg: &marketdata_core::WebSocketMessage, raw: bool) -> PyResult<Py<PyAny>> {
    if raw {
        Ok(PyString::new(py, &msg.raw).unbind().into_any())
    } else {
        Ok(message_to_dict(py, msg)?.into_any())
    }
}

/// Sets its flag when dropped: an `__anext__` awaitable that was cancelled
/// or dropped stops its blocking wait at the next wake-up, before it takes a
/// message nobody will receive.
struct AbandonOnDrop(Arc<AtomicBool>);

impl Drop for AbandonOnDrop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

#[pymethods]
impl MessageIterator {
    /// `MessageIterator[str]` / `MessageIterator[Message]` in annotations:
    /// the stubs declare the class generic over what it yields. Returns
    /// `types.GenericAlias(cls, item)`, so `typing.get_args` sees the
    /// parameter; Python 3.8 has no `GenericAlias` and gets the class itself.
    #[classmethod]
    fn __class_getitem__<'py>(cls: &Bound<'py, PyType>, item: &Bound<'py, PyAny>) -> Bound<'py, PyAny> {
        let alias = cls
            .py()
            .import("types")
            .and_then(|types| types.getattr("GenericAlias"))
            .and_then(|generic_alias| generic_alias.call1((cls, item)));
        alias.unwrap_or_else(|_| cls.clone().into_any())
    }

    /// Return self as iterator (required for Python iteration protocol)
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    /// Get next message from stream
    ///
    /// Returns:
    ///     dict: Message data containing event, channel, symbol, data fields
    ///         (str: the frame as sent, from a `messages(raw=True)` iterator)
    ///
    /// Raises:
    ///     StopIteration: When the connection is gone and every message was read
    ///
    /// Note: This method blocks the current thread until a message arrives,
    /// with the GIL released. It wakes every 100 ms to let Python handle
    /// signals, so Ctrl+C interrupts it.
    fn __next__(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        loop {
            let handoff = Arc::clone(&self.handoff);
            match py.detach(move || handoff.receive(Some(WAKE_INTERVAL))) {
                Ok(Some(msg)) => return yielded(py, &msg, self.raw),
                // Nothing yet: run pending signal handlers, then wait again.
                Ok(None) => py.check_signals()?,
                Err(()) => {
                    return Err(pyo3::exceptions::PyStopIteration::new_err(
                        "Message channel closed",
                    ))
                }
            }
        }
    }

    /// Try to receive a message without blocking
    ///
    /// Returns:
    ///     dict: Message data if available (str from a `messages(raw=True)` iterator)
    ///     None: If no message available
    fn try_recv(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        match self.handoff.try_receive() {
            Some(msg) => Ok(Some(yielded(py, &msg, self.raw)?)),
            None => Ok(None),
        }
    }

    /// Receive a message with timeout
    ///
    /// Args:
    ///     timeout_ms: Timeout in milliseconds
    ///
    /// Returns:
    ///     dict: Message data if received within timeout (str from a `messages(raw=True)` iterator)
    ///     None: If timeout elapsed with no message
    ///
    /// Raises:
    ///     MarketDataError: If channel is closed
    fn recv_timeout(&self, py: Python<'_>, timeout_ms: u64) -> PyResult<Option<Py<PyAny>>> {
        let handoff = Arc::clone(&self.handoff);
        let timeout = Duration::from_millis(timeout_ms);
        match py.detach(move || handoff.receive(Some(timeout))) {
            Ok(Some(msg)) => Ok(Some(yielded(py, &msg, self.raw)?)),
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
    ///         (str: the frame as sent, from a `messages(raw=True)` iterator)
    ///
    /// Raises:
    ///     StopAsyncIteration: When the connection is gone and every message was read
    ///
    /// Note: This method releases the GIL while waiting for messages.
    /// The wait runs on tokio's blocking pool, enabling true async concurrency.
    fn __anext__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let handoff = Arc::clone(&self.handoff);
        let raw = self.raw;

        future_into_py(py, async move {
            let abandoned = Arc::new(AtomicBool::new(false));
            let _abandon = AbandonOnDrop(Arc::clone(&abandoned));
            let result = tokio::task::spawn_blocking(move || loop {
                if abandoned.load(Ordering::SeqCst) {
                    return Ok(None);
                }
                match handoff.receive(Some(WAKE_INTERVAL)) {
                    Ok(Some(msg)) => return Ok(Some(msg)),
                    Ok(None) => continue,
                    Err(()) => return Err(()),
                }
            })
            .await
            .map_err(|e| {
                pyo3::exceptions::PyRuntimeError::new_err(format!("Task join error: {}", e))
            })?;

            match result {
                Ok(Some(msg)) => Python::attach(|py| yielded(py, &msg, raw)),
                // Only when abandoned, and then nobody awaits this result.
                Ok(None) => Err(pyo3::exceptions::PyStopAsyncIteration::new_err(
                    "Iteration abandoned",
                )),
                // Channel closed: end `async for`.
                Err(()) => Err(pyo3::exceptions::PyStopAsyncIteration::new_err(
                    "Message channel closed",
                )),
            }
        })
    }
}

// Note: Tests for MessageIterator require Python runtime and must be run via maturin develop + pytest
// See test_websocket.py for Python-side iterator tests
