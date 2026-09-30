//! Python iterator for WebSocket messages
//!
//! Provides both sync (__iter__/__next__) and async (__aiter__/__anext__) iterator protocols.
//! Both yield messages only and stop only once the connection is gone (#68).
//! A `messages(raw=True)` iterator yields the text of each frame instead of a
//! dict (#246).

use pyo3::prelude::*;
use pyo3::types::{PyString, PyType};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

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

/// One `__anext__` call: the awaitable handed to Python and what resolves it.
///
/// A blocking-pool thread only waits for the queue to become readable; the
/// message is taken by `Deliver`, on the event loop's thread, after it has
/// seen that the awaitable is still pending. Cancelling happens on that
/// thread too, so a cancelled wait never takes a message (#260).
///
/// The queue wakes one reader per message. A wait woken for a message it
/// will not take (awaitable done, loop closed) hands the wake-up on with
/// `pass_wakeup`, or the reader that should get the message would only see
/// it at its own next wake-up.
struct AnextWait {
    handoff: Arc<Handoff>,
    raw: bool,
    event_loop: Py<PyAny>,
    future: Py<PyAny>,
    /// Set once the awaitable is done (resolved or cancelled): the waiting
    /// thread stops at its next wake-up.
    done: Arc<AtomicBool>,
}

impl AnextWait {
    /// Wait off the event loop until there is something to deliver, then
    /// hand over to `Deliver` on the loop.
    fn spawn(self: Arc<Self>) {
        pyo3_async_runtimes::tokio::get_runtime().spawn_blocking(move || {
            while !self.done.load(Ordering::SeqCst) {
                if self.handoff.wait_readable(WAKE_INTERVAL) == Ok(false) {
                    continue;
                }
                let handoff = Arc::clone(&self.handoff);
                let handed_over = Python::attach(|py| {
                    let done = self.future.bind(py).call_method0(pyo3::intern!(py, "done"));
                    if !matches!(done.and_then(|done| done.is_truthy()), Ok(false)) {
                        return false;
                    }
                    let event_loop = self.event_loop.clone_ref(py);
                    // Fails once the loop is closed. Nothing was taken from
                    // the queue and nobody awaits the result: stop quietly.
                    event_loop
                        .bind(py)
                        .call_method1(pyo3::intern!(py, "call_soon_threadsafe"), (Deliver(self),))
                        .is_ok()
                });
                if !handed_over {
                    handoff.pass_wakeup();
                }
                return;
            }
        });
    }
}

/// Done callback of an `__anext__` awaitable.
#[pyclass(frozen)]
struct MarkDone(Arc<AtomicBool>);

#[pymethods]
impl MarkDone {
    fn __call__(&self, _future: &Bound<'_, PyAny>) {
        self.0.store(true, Ordering::SeqCst);
    }
}

/// Runs on the event loop: resolves a still-pending `__anext__` awaitable
/// with the next message, or ends the iteration once the queue is closed.
#[pyclass(frozen)]
struct Deliver(Arc<AnextWait>);

#[pymethods]
impl Deliver {
    fn __call__(&self, py: Python<'_>) -> PyResult<()> {
        let wait = &self.0;
        let future = wait.future.bind(py);
        if future.call_method0(pyo3::intern!(py, "done"))?.is_truthy()? {
            // Cancelled after the wait was woken for this message.
            wait.handoff.pass_wakeup();
            return Ok(());
        }
        let result = match wait.handoff.try_receive() {
            Some(msg) => yielded(py, &msg, wait.raw),
            None if wait.handoff.is_finished() => Err(
                pyo3::exceptions::PyStopAsyncIteration::new_err("Message channel closed"),
            ),
            // Another reader took it first: wait for the next one.
            None => {
                Arc::clone(wait).spawn();
                return Ok(());
            }
        };
        match result {
            Ok(value) => future.call_method1(pyo3::intern!(py, "set_result"), (value,))?,
            Err(err) => future.call_method1(pyo3::intern!(py, "set_exception"), (err,))?,
        };
        Ok(())
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
    ///     timeout_ms: Timeout in milliseconds. A value too large for the
    ///         clock to add waits until a message arrives.
    ///
    /// Returns:
    ///     dict: Message data if received within timeout (str from a `messages(raw=True)` iterator)
    ///     None: If timeout elapsed with no message
    ///
    /// Raises:
    ///     MarketDataError: If channel is closed
    ///
    /// Note: Like `__next__`, the wait wakes every 100 ms to let Python
    /// handle signals, so Ctrl+C interrupts it. An interrupted call has taken
    /// no message.
    fn recv_timeout(&self, py: Python<'_>, timeout_ms: u64) -> PyResult<Option<Py<PyAny>>> {
        // A timeout too large for the clock means no deadline.
        let deadline = Instant::now().checked_add(Duration::from_millis(timeout_ms));
        loop {
            let remaining = deadline.map(|deadline| deadline.saturating_duration_since(Instant::now()));
            let slice = remaining.map_or(WAKE_INTERVAL, |remaining| remaining.min(WAKE_INTERVAL));
            let handoff = Arc::clone(&self.handoff);
            match py.detach(move || handoff.receive(Some(slice))) {
                Ok(Some(msg)) => return Ok(Some(yielded(py, &msg, self.raw)?)),
                Ok(None) if remaining.is_some_and(|remaining| remaining <= slice) => return Ok(None),
                // Nothing yet: run pending signal handlers, then wait again.
                Ok(None) => py.check_signals()?,
                Err(()) => {
                    return Err(crate::errors::to_py_err(marketdata_core::MarketDataError::ConnectionError {
                        msg: "Message channel closed".to_string(),
                    }))
                }
            }
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
    /// Note: The wait runs on tokio's blocking pool, off the event loop. A
    /// cancelled awaitable takes no message: the next read gets it. One
    /// still pending when its event loop closes is dropped silently.
    fn __anext__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let event_loop = pyo3_async_runtimes::tokio::get_current_locals(py)?.event_loop(py);
        let future = event_loop.call_method0(pyo3::intern!(py, "create_future"))?;
        let done = Arc::new(AtomicBool::new(false));
        future.call_method1(
            pyo3::intern!(py, "add_done_callback"),
            (MarkDone(Arc::clone(&done)),),
        )?;
        Arc::new(AnextWait {
            handoff: Arc::clone(&self.handoff),
            raw: self.raw,
            event_loop: event_loop.unbind(),
            future: future.clone().unbind(),
            done,
        })
        .spawn();
        Ok(future)
    }
}

// Note: Tests for MessageIterator require Python runtime and must be run via maturin develop + pytest
// See test_websocket.py for Python-side iterator tests
