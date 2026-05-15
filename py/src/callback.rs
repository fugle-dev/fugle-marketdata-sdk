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

use pyo3::prelude::*;
use std::collections::HashMap;
use std::sync::RwLock;

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
            _ => None,
        }
    }
}

/// Thread-safe registry for Python callbacks
///
/// Stores callbacks as `Py<PyAny>` to enable cross-thread access.
/// Uses RwLock for concurrent read access during message dispatch.
pub struct CallbackRegistry {
    /// Maps event type to list of callbacks
    callbacks: RwLock<HashMap<EventType, Vec<Py<PyAny>>>>,
}

impl CallbackRegistry {
    /// Create a new empty callback registry
    pub fn new() -> Self {
        Self {
            callbacks: RwLock::new(HashMap::new()),
        }
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

        // Store as Py<PyAny> for thread-safe access
        let py_callback: Py<PyAny> = callback.clone().unbind();

        let mut callbacks = self.callbacks.write().unwrap();
        callbacks
            .entry(event_type)
            .or_insert_with(Vec::new)
            .push(py_callback);

        Ok(())
    }

    /// Unregister all callbacks for an event type
    pub fn unregister(&self, event: &str) -> PyResult<()> {
        let event_type = EventType::from_str(event).ok_or_else(|| {
            pyo3::exceptions::PyValueError::new_err(format!("Invalid event type: '{}'", event))
        })?;

        let mut callbacks = self.callbacks.write().unwrap();
        callbacks.remove(&event_type);

        Ok(())
    }

    /// Clear all registered callbacks
    #[allow(dead_code)]
    pub fn clear(&self) {
        let mut callbacks = self.callbacks.write().unwrap();
        callbacks.clear();
    }

    /// Get number of callbacks registered for an event type
    #[allow(dead_code)]
    pub fn count(&self, event_type: EventType) -> usize {
        let callbacks = self.callbacks.read().unwrap();
        callbacks.get(&event_type).map(|v| v.len()).unwrap_or(0)
    }

    /// Invoke all callbacks for an event type with given arguments
    ///
    /// # Arguments
    ///
    /// * `py` - Python GIL token
    /// * `event_type` - Event type to dispatch
    /// * `args` - Arguments tuple to pass to callbacks
    ///
    /// # Returns
    ///
    /// Number of callbacks invoked successfully
    pub fn invoke(&self, py: Python<'_>, event_type: EventType, args: &Bound<'_, pyo3::types::PyTuple>) -> usize {
        let callbacks = self.callbacks.read().unwrap();

        let Some(handlers) = callbacks.get(&event_type) else {
            return 0;
        };

        let mut invoked = 0;

        for callback in handlers {
            // Bind callback to current Python context
            let bound_callback = callback.bind(py);

            // Call with arguments, log errors but continue
            match bound_callback.call1(args) {
                Ok(_) => invoked += 1,
                Err(e) => {
                    // Log error to Python stderr but don't propagate
                    // Simply print the error - avoid complex Python runtime calls
                    eprintln!("Callback error: {}", e);
                }
            }
        }

        invoked
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

    /// Invoke error callbacks with message and code
    /// Invoke error callbacks with a single exception-like argument, matching
    /// the 2.4.1 SDK's `error(err)` callback arity. Constructs a
    /// `WebSocketError` instance with `(message, code)` so user code can do
    /// `on('error', lambda err: print(err))` or inspect `err.args`.
    pub fn invoke_error(&self, py: Python<'_>, message: &str, code: i32) {
        let err = crate::errors::WebSocketError::new_err((message.to_string(), code));
        let err_obj: Py<PyAny> = err.into_value(py).into_any();
        let args = pyo3::types::PyTuple::new(py, [err_obj]).expect("Failed to create tuple");
        self.invoke(py, EventType::Error, &args);
    }

    /// Invoke authenticated callbacks with the server's authenticated ack
    /// message, matching the 2.4.1 SDK's `authenticated(message)` arity.
    /// Core currently doesn't propagate the raw ack payload, so we pass a
    /// minimal `{"event": "authenticated"}` dict; full payload propagation
    /// can follow when core's `ConnectionEvent::Authenticated` carries data.
    pub fn invoke_authenticated(&self, py: Python<'_>) {
        let msg_dict = pyo3::types::PyDict::new(py);
        msg_dict
            .set_item("event", "authenticated")
            .expect("Failed to set event");
        let msg_obj: Py<PyAny> = msg_dict.unbind().into_any();
        let args = pyo3::types::PyTuple::new(py, [msg_obj]).expect("Failed to create tuple");
        self.invoke(py, EventType::Authenticated, &args);
    }

    /// Invoke unauthenticated callbacks with the rejection message
    pub fn invoke_unauthenticated(&self, py: Python<'_>, message: &str) {
        use pyo3::IntoPyObject;
        let msg_obj: Py<PyAny> = message.into_pyobject(py).expect("Failed to convert message").unbind().into_any();
        let args = pyo3::types::PyTuple::new(py, [msg_obj]).expect("Failed to create tuple");
        self.invoke(py, EventType::Unauthenticated, &args);
    }
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
