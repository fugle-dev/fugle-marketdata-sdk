//! Python callback registration mechanism for WebSocket events
//!
//! Provides thread-safe callback storage and invocation for Python event handlers.
//! Supports event types: message, connect, disconnect, reconnect, error.
//!
//! # Example (Python)
//!
//! ```python
//! def on_message(msg):
//!     print(f"Received: {msg}")
//!
//! ws.stock.on("message", on_message)
//! ```

use marketdata_core::websocket::ReportThrottle;
use marketdata_core::{error_code, ErrorInfo, ErrorKind};
use pyo3::exceptions::{PyException, PyTypeError};
use pyo3::prelude::*;
use pyo3::sync::PyOnceLock;
use pyo3::types::PyType;
use std::collections::HashMap;
use std::sync::{Mutex, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::time::Instant;

/// Callbacks by event type.
type CallbackMap = HashMap<EventType, Vec<Py<PyAny>>>;

/// Event types supported by WebSocket client
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventType {
    /// Data message received
    Message,
    /// Connection established
    Connect,
    /// Connection closed
    Disconnect,
    /// Reconnection attempt
    Reconnect,
    /// Error occurred
    Error,
    /// Authentication accepted by server
    Authenticated,
    /// Authentication rejected by server
    Unauthenticated,
    /// Messages dropped because the consumer fell behind
    MessagesDropped,
}

impl EventType {
    /// Parse event type from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "message" | "data" => Some(EventType::Message),
            "connect" | "connected" => Some(EventType::Connect),
            "disconnect" | "disconnected" | "close" | "closed" => Some(EventType::Disconnect),
            "reconnect" | "reconnecting" => Some(EventType::Reconnect),
            "error" => Some(EventType::Error),
            "authenticated" => Some(EventType::Authenticated),
            "unauthenticated" => Some(EventType::Unauthenticated),
            "messages_dropped" => Some(EventType::MessagesDropped),
            _ => None,
        }
    }

    /// The name `on()` registers this event under.
    pub fn name(self) -> &'static str {
        match self {
            EventType::Message => "message",
            EventType::Connect => "connect",
            EventType::Disconnect => "disconnect",
            EventType::Reconnect => "reconnect",
            EventType::Error => "error",
            EventType::Authenticated => "authenticated",
            EventType::Unauthenticated => "unauthenticated",
            EventType::MessagesDropped => "messages_dropped",
        }
    }
}

/// Thread-safe registry for Python callbacks
///
/// Stores callbacks as `Py<PyAny>` to enable cross-thread access.
/// Uses RwLock for concurrent read access during message dispatch.
pub struct CallbackRegistry {
    /// Maps event type to list of callbacks
    callbacks: RwLock<CallbackMap>,
    /// Throttles the reports of failed callbacks (#83).
    failures: Mutex<ReportThrottle>,
    /// Debug builds: panic once inside `unregister` while holding the write
    /// lock, poisoning it, for `FUGLE_MARKETDATA_TEST_PANIC=ws_callback_poison`
    /// (#25). Only a writer's panic poisons an `RwLock`.
    #[cfg(debug_assertions)]
    test_poison_lock: std::sync::atomic::AtomicBool,
}

impl CallbackRegistry {
    /// Create a new empty callback registry
    pub fn new() -> Self {
        Self {
            callbacks: RwLock::new(HashMap::new()),
            failures: Mutex::new(ReportThrottle::new()),
            #[cfg(debug_assertions)]
            test_poison_lock: std::sync::atomic::AtomicBool::new(
                std::env::var("FUGLE_MARKETDATA_TEST_PANIC").as_deref() == Ok("ws_callback_poison"),
            ),
        }
    }

    // The map is left consistent by every writer, so a lock poisoned by a
    // panic elsewhere (say, in a callback thread) is still safe to use — and
    // must be, or reporting that panic through `error` would panic too (#25).

    fn read(&self) -> RwLockReadGuard<'_, CallbackMap> {
        self.callbacks.read().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn write(&self) -> RwLockWriteGuard<'_, CallbackMap> {
        self.callbacks.write().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Register a callback for an event type
    ///
    /// # Arguments
    ///
    /// * `event` - Event type string (message, connect, disconnect, reconnect, error)
    /// * `callback` - Python callable to invoke when event occurs
    ///
    /// # Returns
    ///
    /// * `Ok(())` if callback registered successfully
    /// * `Err(PyErr)` if event type is invalid or callback is not callable
    pub fn register(&self, event: &str, callback: &Bound<'_, PyAny>) -> PyResult<()> {
        let event_type = EventType::from_str(event).ok_or_else(|| {
            pyo3::exceptions::PyValueError::new_err(format!(
                "Invalid event type: '{}'. Valid types: message, connect, disconnect, reconnect, error, authenticated, unauthenticated",
                event
            ))
        })?;

        // Verify callback is callable
        if !callback.is_callable() {
            return Err(pyo3::exceptions::PyTypeError::new_err(
                "Callback must be callable",
            ));
        }

        // Nothing would await its coroutine (#83).
        let py = callback.py();
        let is_async = py
            .import("inspect")?
            .call_method1("iscoroutinefunction", (callback,))?
            .is_truthy()?;
        if is_async {
            return Err(PyTypeError::new_err(
                "async def callbacks are not supported; use a regular function, \
                 or consume messages with `async for message in ws.messages()`",
            ));
        }

        // Store as Py<PyAny> for thread-safe access
        let py_callback: Py<PyAny> = callback.clone().unbind();

        let mut callbacks = self.write();
        callbacks
            .entry(event_type)
            .or_default()
            .push(py_callback);

        Ok(())
    }

    /// Unregister all callbacks for an event type
    pub fn unregister(&self, event: &str) -> PyResult<()> {
        let event_type = EventType::from_str(event).ok_or_else(|| {
            pyo3::exceptions::PyValueError::new_err(format!("Invalid event type: '{}'", event))
        })?;

        let mut callbacks = self.write();
        #[cfg(debug_assertions)]
        if self.test_poison_lock.swap(false, std::sync::atomic::Ordering::SeqCst) {
            panic!("injected test panic at ws_callback_poison");
        }
        callbacks.remove(&event_type);

        Ok(())
    }

    /// Clear all registered callbacks
    #[allow(dead_code)]
    pub fn clear(&self) {
        let mut callbacks = self.write();
        callbacks.clear();
    }

    /// Get number of callbacks registered for an event type
    #[allow(dead_code)]
    pub fn count(&self, event_type: EventType) -> usize {
        let callbacks = self.read();
        callbacks.get(&event_type).map(|v| v.len()).unwrap_or(0)
    }

    /// Invoke all callbacks for an event type with given arguments
    ///
    /// A callback that raises an `Exception` does not stop the others or
    /// later events; it is reported as described in [`Self::report_failure`].
    /// A `BaseException` that is not an `Exception` (`KeyboardInterrupt`,
    /// `SystemExit`, ...) cannot reach the caller from this thread: it is
    /// only printed, through `sys.unraisablehook`.
    ///
    /// # Returns
    ///
    /// Number of callbacks invoked successfully
    pub fn invoke(&self, py: Python<'_>, event_type: EventType, args: &Bound<'_, pyo3::types::PyTuple>) -> usize {
        // Released before calling, so a callback may register callbacks.
        let handlers: Vec<Py<PyAny>> = match self.read().get(&event_type) {
            Some(handlers) => handlers.iter().map(|callback| callback.clone_ref(py)).collect(),
            None => return 0,
        };

        let mut invoked = 0;
        for callback in &handlers {
            let callback = callback.bind(py);
            match callback.call1(args) {
                Ok(returned) => match close_coroutine(py, &returned) {
                    None => invoked += 1,
                    Some(err) => self.report_failure(py, event_type, callback, err),
                },
                Err(err) if err.is_instance_of::<PyException>(py) => {
                    self.report_failure(py, event_type, callback, err)
                }
                Err(err) => err.write_unraisable(py, Some(callback)),
            }
        }

        invoked
    }

    /// Let the user know `callback`, registered for `event_type`, raised `err`
    /// — without stopping delivery and without recursing (#83).
    ///
    /// A failing `error` callback is only printed. Any other failure goes to
    /// the `error` callbacks as a `WebSocketError` (code
    /// [`error_code::CALLBACK_FAILED`], `__cause__`, `event`, `count`),
    /// throttled like `messages_dropped`: the first at once, later ones at
    /// most once per second, counting the failures since the previous report.
    /// With no `error` callback, or one that raises in turn, it is printed
    /// through `sys.unraisablehook` instead.
    fn report_failure(&self, py: Python<'_>, event_type: EventType, callback: &Bound<'_, PyAny>, err: PyErr) {
        if event_type == EventType::Error {
            err.write_unraisable(py, Some(callback));
            return;
        }
        let count = self
            .failures
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .record(Instant::now());
        let Some(count) = count else { return };

        let event = event_type.name();
        let kind = err
            .get_type(py)
            .name()
            .map(|name| name.to_string())
            .unwrap_or_else(|_| "Exception".to_string());
        let message = format!("'{event}' callback raised {kind}: {}", err.value(py));
        let info = ErrorInfo::new(error_code::CALLBACK_FAILED, ErrorKind::Client, message);
        let report = crate::errors::websocket_error(py, &info);
        report.set_cause(py, Some(err));
        let value = report.value(py);
        let _ = value.setattr("event", event);
        let _ = value.setattr("count", count);

        let handlers: Vec<Py<PyAny>> = match self.read().get(&EventType::Error) {
            Some(handlers) if !handlers.is_empty() => {
                handlers.iter().map(|handler| handler.clone_ref(py)).collect()
            }
            _ => {
                report.write_unraisable(py, Some(callback));
                return;
            }
        };
        for handler in &handlers {
            let handler = handler.bind(py);
            let failure = match handler.call1((report.clone_ref(py).into_value(py),)) {
                Ok(returned) => close_coroutine(py, &returned),
                Err(failure) => Some(failure),
            };
            if let Some(failure) = failure {
                failure.set_context(py, Some(report.clone_ref(py)));
                failure.write_unraisable(py, Some(handler));
            }
        }
    }

    /// Invoke message callbacks with a WebSocket message dict
    #[allow(dead_code)]
    pub fn invoke_message(&self, py: Python<'_>, msg_dict: Py<pyo3::types::PyDict>) {
        let args = pyo3::types::PyTuple::new(py, [msg_dict.into_any()]).expect("Failed to create tuple");
        self.invoke(py, EventType::Message, &args);
    }

    /// Invoke connect callbacks
    pub fn invoke_connect(&self, py: Python<'_>) {
        let args = pyo3::types::PyTuple::empty(py);
        self.invoke(py, EventType::Connect, &args);
    }

    /// Invoke disconnect callbacks with optional code and reason
    pub fn invoke_disconnect(&self, py: Python<'_>, code: Option<u16>, reason: &str) {
        use pyo3::IntoPyObject;
        let code_obj: Py<PyAny> = code.into_pyobject(py).expect("Failed to convert code").into();
        let reason_obj: Py<PyAny> = reason.into_pyobject(py).expect("Failed to convert reason").unbind().into_any();
        let args = pyo3::types::PyTuple::new(py, [code_obj, reason_obj]).expect("Failed to create tuple");
        self.invoke(py, EventType::Disconnect, &args);
    }

    /// Invoke reconnect callbacks with attempt number
    pub fn invoke_reconnect(&self, py: Python<'_>, attempt: u32) {
        use pyo3::IntoPyObject;
        let attempt_obj: Py<PyAny> = attempt.into_pyobject(py).expect("Failed to convert attempt").unbind().into_any();
        let args = pyo3::types::PyTuple::new(py, [attempt_obj]).expect("Failed to create tuple");
        self.invoke(py, EventType::Reconnect, &args);
    }

    /// Invoke messages_dropped callbacks with `(dropped, total)`: messages
    /// dropped since the previous call, and on the connection so far.
    pub fn invoke_messages_dropped(&self, py: Python<'_>, dropped: u64, total: u64) {
        let args = pyo3::types::PyTuple::new(py, [dropped, total]).expect("Failed to create tuple");
        self.invoke(py, EventType::MessagesDropped, &args);
    }

    /// Invoke error callbacks with a single exception-like argument, matching
    /// the 2.4.1 SDK's `error(err)` callback arity: a `WebSocketError` whose
    /// `args` are `(message, code)`, carrying `info`'s unified fields, so user
    /// code can do `on('error', lambda err: print(err))` or read `err.code`.
    pub fn invoke_error(&self, py: Python<'_>, info: &ErrorInfo) {
        let err = crate::errors::websocket_error(py, info);
        let err_obj: Py<PyAny> = err.into_value(py).into_any();
        let args = pyo3::types::PyTuple::new(py, [err_obj]).expect("Failed to create tuple");
        self.invoke(py, EventType::Error, &args);
    }

    /// Issue core's reconnect-conflict report (code 3006) as a
    /// `RuntimeWarning` instead of an `error` callback (#226). Core reports
    /// it once per client. A warning filter that turns it into an exception
    /// gets it reported as unraisable: this runs on the SDK's thread.
    pub fn warn_reconnect_conflict(&self, py: Python<'_>, info: &ErrorInfo) {
        let category = py.get_type::<pyo3::exceptions::PyRuntimeWarning>();
        // `warn_explicit` with a fixed module: there is no Python frame on
        // this thread to attribute it to, and `module="fugle_marketdata"`
        // lets a filter select it.
        let warned = py.import("warnings").and_then(|warnings| {
            warnings.call_method1(
                "warn_explicit",
                (info.message.as_str(), category, "fugle_marketdata", 0, "fugle_marketdata"),
            )
        });
        if let Err(err) = warned {
            err.write_unraisable(py, None);
        }
    }

    /// Invoke authenticated callbacks with the `data` of the server's
    /// authenticated frame (`dict`, or `None` when the frame has none).
    pub fn invoke_authenticated(&self, py: Python<'_>, data: &serde_json::Value) {
        self.invoke_with_data(py, EventType::Authenticated, data);
    }

    /// Invoke unauthenticated callbacks with the `data` of the server's
    /// rejection frame (`dict`, or `None` when the frame has none).
    pub fn invoke_unauthenticated(&self, py: Python<'_>, data: &serde_json::Value) {
        self.invoke_with_data(py, EventType::Unauthenticated, data);
    }

    fn invoke_with_data(&self, py: Python<'_>, event_type: EventType, data: &serde_json::Value) {
        let data_obj = crate::types::json_value_to_py(py, data).unwrap_or_else(|_| py.None());
        let args = pyo3::types::PyTuple::new(py, [data_obj]).expect("Failed to create tuple");
        self.invoke(py, event_type, &args);
    }
}

/// If a callback returned a coroutine (say, a lambda around an `async def`),
/// close it — nothing would await it — and return the error to report.
///
/// Runs after every callback, messages included: the usual `None` returns
/// at once, and the coroutine type is looked up only once.
fn close_coroutine(py: Python<'_>, returned: &Bound<'_, PyAny>) -> Option<PyErr> {
    static COROUTINE_TYPE: PyOnceLock<Py<PyType>> = PyOnceLock::new();
    if returned.is_none() {
        return None;
    }
    let is_coroutine = COROUTINE_TYPE
        .import(py, "types", "CoroutineType")
        .and_then(|coroutine| returned.is_instance(coroutine))
        .unwrap_or(false);
    if !is_coroutine {
        return None;
    }
    let _ = returned.call_method0("close");
    Some(PyTypeError::new_err(
        "callback returned a coroutine; async callbacks are not supported \
         (use `async for message in ws.messages()`)",
    ))
}

impl Default for CallbackRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_type_from_str() {
        assert_eq!(EventType::from_str("message"), Some(EventType::Message));
        assert_eq!(EventType::from_str("MESSAGE"), Some(EventType::Message));
        assert_eq!(EventType::from_str("data"), Some(EventType::Message));
        assert_eq!(EventType::from_str("connect"), Some(EventType::Connect));
        assert_eq!(EventType::from_str("connected"), Some(EventType::Connect));
        assert_eq!(EventType::from_str("disconnect"), Some(EventType::Disconnect));
        assert_eq!(EventType::from_str("disconnected"), Some(EventType::Disconnect));
        assert_eq!(EventType::from_str("close"), Some(EventType::Disconnect));
        assert_eq!(EventType::from_str("reconnect"), Some(EventType::Reconnect));
        assert_eq!(EventType::from_str("error"), Some(EventType::Error));
        assert_eq!(EventType::from_str("invalid"), None);
    }

    #[test]
    fn test_callback_registry_new() {
        let registry = CallbackRegistry::new();
        assert_eq!(registry.count(EventType::Message), 0);
        assert_eq!(registry.count(EventType::Connect), 0);
    }

    #[test]
    fn test_callback_registry_clear() {
        let registry = CallbackRegistry::new();
        // Just verify clear doesn't panic on empty registry
        registry.clear();
    }
}
