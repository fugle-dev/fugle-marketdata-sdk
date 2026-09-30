//! Python iterator for WebSocket messages
//!
//! Provides both sync (__iter__/__next__) and async (__aiter__/__anext__) iterator protocols.
//! Both yield messages only and stop only once the connection is gone (#68).
//! A `messages(raw=True)` iterator yields the text of each frame instead of a
//! dict (#246).

use pyo3::prelude::*;
use pyo3::types::{PyString, PyType};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::handoff::Handoff;
use crate::websocket::message_to_dict;

/// How often a waiting iterator wakes up: to notice the connection closing
/// and, when iterating synchronously, to let Python handle signals (Ctrl+C).
const WAKE_INTERVAL: Duration = Duration::from_millis(100);

/// An `__anext__` wait checks whether its event loop was closed once per
/// this many idle wake-ups: about once a second.
const LOOP_CHECK_EVERY: u32 = 10;

/// While messages are queued, `__anext__` resolves its awaitable before
/// returning it, which never gives the event loop a turn. One delivery in
/// every this many goes through the loop instead, so other tasks run (#267).
///
/// Why 32, from measurements with a second task on the same loop waking
/// every 1 ms (PR #269, 50K-message burst, loop body of about 5 µs a
/// message): `async for` reads 6.3 times what it did with every delivery
/// through a thread, 83% of what never yielding reads, and the other task
/// is at most 0.5 ms late. A larger interval gains little (128: 15% more)
/// while the other task's delay grows in step with it: about the interval
/// times what the loop body takes per message. Never yielding kept the
/// other task waiting 156 ms.
const YIELD_EVERY: u32 = 32;

/// `__anext__` waits currently on the blocking pool, over all iterators.
static PENDING_WAITS: AtomicUsize = AtomicUsize::new(0);

/// Counts one wait in `PENDING_WAITS` for as long as it lives.
struct PendingWait;

impl PendingWait {
    fn new() -> Self {
        PENDING_WAITS.fetch_add(1, Ordering::SeqCst);
        Self
    }
}

impl Drop for PendingWait {
    fn drop(&mut self) {
        PENDING_WAITS.fetch_sub(1, Ordering::SeqCst);
    }
}

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
    /// Deliveries `__anext__` made in a row without the event loop.
    direct_streak: AtomicU32,
}

impl MessageIterator {
    /// Create a new message iterator
    pub(crate) fn new(handoff: Arc<Handoff>, raw: bool) -> Self {
        Self {
            handoff,
            raw,
            direct_streak: AtomicU32::new(0),
        }
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

/// One `__anext__` call that found nothing it could take at once: the
/// awaitable handed to Python and what resolves it.
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
///
/// An awaitable nobody cancelled stays pending once its event loop is
/// closed, so the waiting thread also asks the loop, about once a second
/// while no message arrives, and stops when it is closed (#267).
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
        let counted = PendingWait::new();
        pyo3_async_runtimes::tokio::get_runtime().spawn_blocking(move || {
            let _counted = counted;
            let mut idle_wakeups = 0u32;
            while !self.done.load(Ordering::SeqCst) {
                if self.handoff.wait_readable(WAKE_INTERVAL) == Ok(false) {
                    idle_wakeups += 1;
                    // Woken by the timeout, not for a message: nothing to pass on.
                    if idle_wakeups.is_multiple_of(LOOP_CHECK_EVERY)
                        && !self.done.load(Ordering::SeqCst)
                        && self.loop_closed()
                    {
                        return;
                    }
                    continue;
                }
                let handoff = Arc::clone(&self.handoff);
                // `None`: the interpreter is shutting down, nobody to deliver to.
                let handed_over = Python::try_attach(|py| {
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
                if handed_over != Some(true) {
                    handoff.pass_wakeup();
                }
                return;
            }
        });
    }

    /// Also true once the interpreter is shutting down.
    fn loop_closed(&self) -> bool {
        Python::try_attach(|py| {
            let closed = self.event_loop.bind(py).call_method0(pyo3::intern!(py, "is_closed"));
            matches!(closed.and_then(|closed| closed.is_truthy()), Ok(true))
        })
        .unwrap_or(true)
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
    /// Note: While messages are queued the awaitable comes back already
    /// done, with no turn of the event loop; one delivery in every 32 in a
    /// row goes through the loop so other tasks run. A done awaitable holds
    /// its message and cannot be cancelled: `cancel()` returns False and the
    /// message is its `result()`, so check `done()` before cancelling one
    /// (after `asyncio.wait`, around `asyncio.gather`). `async for` and a
    /// plain `await` lose no message; `asyncio.wait_for` loses none with a
    /// timeout longer than a turn of the event loop (see the stub for
    /// shorter ones). With nothing queued the wait runs on
    /// tokio's blocking pool, off the event loop, and a cancelled awaitable
    /// takes no message: the next read gets it. One still pending when its
    /// event loop closes is dropped silently, and its wait ends within
    /// about a second.
    fn __anext__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let event_loop = pyo3_async_runtimes::tokio::get_current_locals(py)?.event_loop(py);
        let future = event_loop.call_method0(pyo3::intern!(py, "create_future"))?;

        let streak = self.direct_streak.load(Ordering::Relaxed);
        let direct = streak + 1 < YIELD_EVERY;
        if direct {
            if let Some(msg) = self.handoff.try_receive() {
                self.direct_streak.store(streak + 1, Ordering::Relaxed);
                match yielded(py, &msg, self.raw) {
                    Ok(value) => future.call_method1(pyo3::intern!(py, "set_result"), (value,))?,
                    Err(err) => future.call_method1(pyo3::intern!(py, "set_exception"), (err,))?,
                };
                return Ok(future);
            }
        }
        // Either way the event loop gets a turn before the next delivery.
        self.direct_streak.store(0, Ordering::Relaxed);

        let done = Arc::new(AtomicBool::new(false));
        future.call_method1(
            pyo3::intern!(py, "add_done_callback"),
            (MarkDone(Arc::clone(&done)),),
        )?;
        let wait = Arc::new(AnextWait {
            handoff: Arc::clone(&self.handoff),
            raw: self.raw,
            event_loop: event_loop.clone().unbind(),
            future: future.clone().unbind(),
            done,
        });
        if !direct && self.handoff.wait_readable(Duration::ZERO) != Ok(false) {
            // Something to deliver and this delivery's turn to yield: let
            // the loop run `Deliver`, with no thread in between.
            event_loop.call_method1(pyo3::intern!(py, "call_soon"), (Deliver(wait),))?;
        } else {
            wait.spawn();
        }
        Ok(future)
    }

    /// `__anext__` waits currently on the blocking pool, over all iterators.
    /// For tests.
    #[staticmethod]
    fn _pending_waits() -> usize {
        PENDING_WAITS.load(Ordering::SeqCst)
    }
}

// Note: Tests for MessageIterator require Python runtime and must be run via maturin develop + pytest
// See test_websocket.py for Python-side iterator tests
