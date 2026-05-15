//! Sync WebSocket client public surface.
//!
//! Mirrors the async `aio::WebSocketClient` API minus `.await` and
//! `message_stream()` (which returns a tokio receiver).

use crate::models::{SubscribeRequest, WebSocketMessage, WebSocketRequest};
use crate::websocket::connection_event::emit_event;
use crate::websocket::protocol::{
    frame_request, frame_subscribe, frame_subscribe_futopt, frame_unsubscribe,
};
use crate::websocket::sync::owner_thread::{
    do_auth_handshake, do_blocking_connect, run_supervisor, OwnerShared, WRITE_QUEUE_CAPACITY,
};
use crate::websocket::{
    ConnectionConfig, ConnectionEvent, ConnectionState, HealthCheckConfig, MessageReceiver,
    ReconnectionConfig, ReconnectionManager, SubscriptionManager,
};
use crate::MarketDataError;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex, RwLock};
use std::thread;

/// Synchronous WebSocket client.
///
/// All operations block the caller. No tokio runtime required. Internally
/// owns one OS thread per active connection (the supervisor/owner thread).
pub struct WebSocketClient {
    shared: Arc<OwnerShared>,
    /// Event receiver wrapped for shared access (mirrors async client API).
    event_rx: Arc<Mutex<mpsc::Receiver<ConnectionEvent>>>,
    /// Holds the inbound-message receiver until `messages()` consumes it.
    message_rx_slot: Mutex<Option<mpsc::Receiver<WebSocketMessage>>>,
    /// Cached `MessageReceiver` returned by `messages()`.
    message_receiver: Mutex<Option<Arc<MessageReceiver>>>,
    /// Supervisor thread JoinHandle (Some once connected, None after disconnect).
    supervisor_handle: Mutex<Option<thread::JoinHandle<()>>>,
}

impl WebSocketClient {
    /// Create a new WebSocket client with default reconnection + health check config.
    pub fn new(config: ConnectionConfig) -> Self {
        Self::with_full_config(config, ReconnectionConfig::default(), HealthCheckConfig::default())
    }

    /// Create a new WebSocket client with custom reconnection config.
    pub fn with_reconnection_config(
        config: ConnectionConfig,
        reconnection_config: ReconnectionConfig,
    ) -> Self {
        Self::with_full_config(config, reconnection_config, HealthCheckConfig::default())
    }

    /// Create a new WebSocket client with custom health check config.
    pub fn with_health_check_config(
        config: ConnectionConfig,
        health_check_config: HealthCheckConfig,
    ) -> Self {
        Self::with_full_config(config, ReconnectionConfig::default(), health_check_config)
    }

    /// Create a new WebSocket client with full custom config.
    pub fn with_full_config(
        config: ConnectionConfig,
        reconnection_config: ReconnectionConfig,
        health_check_config: HealthCheckConfig,
    ) -> Self {
        let (event_tx, event_rx) = mpsc::sync_channel::<ConnectionEvent>(1024);
        let (message_tx, message_rx) = mpsc::sync_channel::<WebSocketMessage>(1024);

        // Eagerly build the rustls config so connect() reuses an Arc-shared instance
        // and reconnects don't pay the native-certs load cost (~10-50ms).
        let tls_config = crate::tls::build_rustls_config(&config.tls)
            .unwrap_or_else(|e| panic!("Failed to build TLS config: {e}"));

        let shared = Arc::new(OwnerShared {
            config,
            tls_config,
            health: health_check_config,
            reconnection: Mutex::new(ReconnectionManager::new(reconnection_config)),
            state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            subscriptions: Arc::new(SubscriptionManager::new()),
            event_tx,
            message_tx,
            write_tx_slot: Mutex::new(None),
            should_stop: Arc::new(AtomicBool::new(false)),
        });

        Self {
            shared,
            event_rx: Arc::new(Mutex::new(event_rx)),
            message_rx_slot: Mutex::new(Some(message_rx)),
            message_receiver: Mutex::new(None),
            supervisor_handle: Mutex::new(None),
        }
    }

    /// Current connection state (snapshot).
    pub fn state(&self) -> ConnectionState {
        self.shared.state.read().expect("state lock poisoned").clone()
    }

    /// Returns true once the client has been disconnected and cannot be reused.
    pub fn is_closed(&self) -> bool {
        matches!(*self.shared.state.read().expect("state lock poisoned"), ConnectionState::Closed { .. })
    }

    /// True iff the supervisor reports a `Connected` state.
    pub fn is_connected(&self) -> bool {
        matches!(*self.shared.state.read().expect("state lock poisoned"), ConnectionState::Connected)
    }

    /// Reference to the event receiver. Lifecycle events arrive here (bounded
    /// channel, drop-newest on saturation).
    pub fn events(&self) -> &Arc<Mutex<mpsc::Receiver<ConnectionEvent>>> {
        &self.event_rx
    }

    /// Semantic alias for [`events`](Self::events).
    pub fn state_events(&self) -> &Arc<Mutex<mpsc::Receiver<ConnectionEvent>>> {
        &self.event_rx
    }

    /// Get the blocking inbound-message receiver. Idempotent; subsequent calls
    /// return the same `Arc<MessageReceiver>`.
    pub fn messages(&self) -> Arc<MessageReceiver> {
        let mut slot = self.message_receiver.lock().expect("message_receiver lock poisoned");
        if let Some(rx) = slot.as_ref() {
            return Arc::clone(rx);
        }
        let std_rx = self
            .message_rx_slot
            .lock()
            .expect("message_rx_slot lock poisoned")
            .take()
            .expect("message receiver already taken");
        let receiver = Arc::new(MessageReceiver::new(std_rx));
        *slot = Some(Arc::clone(&receiver));
        receiver
    }

    /// Connect to the WebSocket server and authenticate. Blocks until either
    /// authentication succeeds or fails.
    pub fn connect(&self) -> Result<(), MarketDataError> {
        if self.is_closed() {
            return Err(MarketDataError::ClientClosed);
        }
        if self.supervisor_handle.lock().expect("supervisor handle lock poisoned").is_some() {
            // Already connected (or supervisor still alive). No-op rather than error.
            return Ok(());
        }

        self.set_state(ConnectionState::Connecting);
        emit_event(&self.shared.event_tx, ConnectionEvent::Connecting);

        let mut ws = match do_blocking_connect(
            &self.shared.config,
            Arc::clone(&self.shared.tls_config),
        ) {
            Ok(ws) => ws,
            Err(e) => {
                self.set_state(ConnectionState::Disconnected);
                emit_event(&self.shared.event_tx, ConnectionEvent::Error {
                    message: e.to_string(),
                    code: e.to_error_code(),
                });
                return Err(e);
            }
        };
        emit_event(&self.shared.event_tx, ConnectionEvent::Connected);

        self.set_state(ConnectionState::Authenticating);
        if let Err(e) = do_auth_handshake(&mut ws, &self.shared.config, &self.shared.message_tx) {
            self.set_state(ConnectionState::Disconnected);
            if let MarketDataError::AuthError { msg } = &e {
                emit_event(&self.shared.event_tx, ConnectionEvent::Unauthenticated {
                    message: msg.clone(),
                });
            } else {
                emit_event(&self.shared.event_tx, ConnectionEvent::Error {
                    message: e.to_string(),
                    code: e.to_error_code(),
                });
            }
            return Err(e);
        }

        // Build the outbound queue + install sender into the shared slot
        let (write_tx, write_rx) = mpsc::sync_channel::<String>(WRITE_QUEUE_CAPACITY);
        *self.shared.write_tx_slot.lock().expect("write_tx_slot lock poisoned") = Some(write_tx);

        self.set_state(ConnectionState::Connected);
        emit_event(&self.shared.event_tx, ConnectionEvent::Authenticated);

        // Spawn supervisor thread
        let shared = Arc::clone(&self.shared);
        let handle = thread::Builder::new()
            .name("fugle-ws-supervisor".to_string())
            .spawn(move || run_supervisor(ws, write_rx, shared))
            .map_err(|e| MarketDataError::ConnectionError {
                msg: format!("Failed to spawn supervisor thread: {e}"),
            })?;
        *self.supervisor_handle.lock().expect("supervisor handle lock poisoned") = Some(handle);

        Ok(())
    }

    /// Disconnect gracefully. Signals the supervisor thread to stop, drops the
    /// outbound sender (which wakes the owner loop if blocked on writes), and
    /// joins the supervisor thread with a generous timeout.
    pub fn disconnect(&self) -> Result<(), MarketDataError> {
        self.shared.should_stop.store(true, Ordering::SeqCst);

        // Drop the writer sender so the owner loop sees Disconnected on try_recv.
        *self.shared.write_tx_slot.lock().expect("write_tx_slot lock poisoned") = None;

        // Join supervisor thread
        if let Some(handle) = self.supervisor_handle.lock().expect("supervisor handle lock poisoned").take() {
            // `JoinHandle::join` does not support timeout in std. We accept a
            // brief wait — the owner loop polls `should_stop` every
            // READ_POLL_INTERVAL (200ms) so worst case the join completes
            // within a few hundred ms.
            let _ = handle.join();
        }

        self.set_state(ConnectionState::Closed {
            code: Some(1000),
            reason: "Normal closure".to_string(),
        });
        emit_event(&self.shared.event_tx, ConnectionEvent::Disconnected {
            code: Some(1000),
            reason: "Normal closure".to_string(),
        });

        Ok(())
    }

    /// Force-close without waiting for the supervisor.
    pub fn force_close(&self) -> Result<(), MarketDataError> {
        self.shared.should_stop.store(true, Ordering::SeqCst);
        *self.shared.write_tx_slot.lock().expect("write_tx_slot lock poisoned") = None;
        // Drop the join handle without joining — supervisor will exit on its own.
        let _ = self.supervisor_handle.lock().expect("supervisor handle lock poisoned").take();

        self.set_state(ConnectionState::Closed {
            code: Some(1006),
            reason: "Force closed".to_string(),
        });
        emit_event(&self.shared.event_tx, ConnectionEvent::Disconnected {
            code: Some(1006),
            reason: "Force closed".to_string(),
        });

        Ok(())
    }

    /// Subscribe to a stock-domain stream.
    pub fn subscribe(
        &self,
        sub: crate::websocket::channels::StockSubscription,
    ) -> Result<(), MarketDataError> {
        if self.is_closed() {
            return Err(MarketDataError::ClientClosed);
        }

        let (json, expanded) = frame_subscribe(sub)?;
        for entry in expanded {
            self.shared.subscriptions.subscribe(entry);
        }

        if self.is_connected() {
            self.enqueue_write(json)?;
        }
        Ok(())
    }

    /// Subscribe to a FutOpt-domain stream.
    pub fn subscribe_futopt(
        &self,
        sub: crate::websocket::channels::FutOptSubscription,
    ) -> Result<(), MarketDataError> {
        if self.is_closed() {
            return Err(MarketDataError::ClientClosed);
        }

        let (json, expanded) = frame_subscribe_futopt(sub)?;
        for entry in expanded {
            self.shared.subscriptions.subscribe(entry);
        }

        if self.is_connected() {
            self.enqueue_write(json)?;
        }
        Ok(())
    }

    /// Unsubscribe by server id or local key.
    pub fn unsubscribe(
        &self,
        ids: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<(), MarketDataError> {
        if self.is_closed() {
            return Err(MarketDataError::ClientClosed);
        }

        let keys: Vec<String> = ids.into_iter().map(Into::into).collect();
        if keys.is_empty() {
            return Ok(());
        }

        let mut wire_ids = Vec::with_capacity(keys.len());
        for key in &keys {
            let id = self
                .shared
                .subscriptions
                .take_server_id(key)
                .unwrap_or_else(|| key.clone());
            self.shared.subscriptions.unsubscribe(key);
            wire_ids.push(id);
        }

        if !self.is_connected() {
            return Ok(());
        }

        let json = frame_unsubscribe(wire_ids)?;
        self.enqueue_write(json)
    }

    /// Get all active subscriptions.
    pub fn subscriptions(&self) -> Vec<SubscribeRequest> {
        self.shared.subscriptions.get_all()
    }

    /// Get list of active subscription keys.
    pub fn subscription_keys(&self) -> Vec<String> {
        self.shared.subscriptions.keys()
    }

    /// Manually reconnect. Calls disconnect() then connect() — simpler and
    /// safer than poking the supervisor.
    pub fn reconnect(&self) -> Result<(), MarketDataError> {
        if self.is_closed() {
            return Err(MarketDataError::ClientClosed);
        }
        // Stop supervisor, drain queue, etc.
        self.shared.should_stop.store(true, Ordering::SeqCst);
        *self.shared.write_tx_slot.lock().expect("write_tx_slot lock poisoned") = None;
        if let Some(handle) = self.supervisor_handle.lock().expect("supervisor handle lock poisoned").take() {
            let _ = handle.join();
        }
        // Reset stop flag and reconnection counter for a fresh attempt.
        self.shared.should_stop.store(false, Ordering::SeqCst);
        {
            let mut mgr = self.shared.reconnection.lock().expect("reconnection lock poisoned");
            mgr.reset();
        }

        self.connect()
    }

    /// Send an arbitrary WebSocket request frame.
    pub fn send(&self, request: WebSocketRequest) -> Result<(), MarketDataError> {
        if self.is_closed() {
            return Err(MarketDataError::ClientClosed);
        }
        let json = frame_request(&request)?;
        self.enqueue_write(json)
    }

    fn enqueue_write(&self, json: String) -> Result<(), MarketDataError> {
        let sender_clone = {
            let guard = self.shared.write_tx_slot.lock().expect("write_tx_slot lock poisoned");
            guard.clone()
        };
        match sender_clone {
            Some(tx) => tx.send(json).map_err(|_| MarketDataError::ConnectionError {
                msg: "Writer queue closed (supervisor exited)".to_string(),
            }),
            None => Err(MarketDataError::ConnectionError {
                msg: "Not connected".to_string(),
            }),
        }
    }

    fn set_state(&self, new_state: ConnectionState) {
        let mut st = self.shared.state.write().expect("state lock poisoned");
        *st = new_state;
    }
}

impl Drop for WebSocketClient {
    fn drop(&mut self) {
        // Best-effort shutdown. We don't join indefinitely — the supervisor
        // polls `should_stop` every READ_POLL_INTERVAL (200ms) so cleanup
        // is bounded.
        self.shared.should_stop.store(true, Ordering::SeqCst);
        *self.shared.write_tx_slot.lock().expect("write_tx_slot lock poisoned") = None;
        if let Some(handle) = self.supervisor_handle.lock().expect("supervisor handle lock poisoned").take() {
            let _ = handle.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AuthRequest;

    #[test]
    fn test_new_starts_disconnected() {
        let config = ConnectionConfig::fugle_stock(AuthRequest::with_api_key("test"));
        let client = WebSocketClient::new(config);
        assert_eq!(client.state(), ConnectionState::Disconnected);
        assert!(!client.is_closed());
        assert!(!client.is_connected());
    }

    #[test]
    fn test_subscribe_before_connect_records_subscription() {
        use crate::models::Channel;
        use crate::websocket::channels::StockSubscription;
        let config = ConnectionConfig::fugle_stock(AuthRequest::with_api_key("test"));
        let client = WebSocketClient::new(config);

        let sub = StockSubscription::new(Channel::Trades, "2330");
        client.subscribe(sub).unwrap();

        assert_eq!(client.subscription_keys().len(), 1);
    }

    #[test]
    fn test_unsubscribe_when_disconnected_removes_state() {
        use crate::models::Channel;
        use crate::websocket::channels::StockSubscription;
        let config = ConnectionConfig::fugle_stock(AuthRequest::with_api_key("test"));
        let client = WebSocketClient::new(config);

        let sub = StockSubscription::new(Channel::Trades, "2330");
        client.subscribe(sub).unwrap();
        assert_eq!(client.subscription_keys().len(), 1);

        client.unsubscribe(["trades:2330"]).unwrap();
        assert_eq!(client.subscription_keys().len(), 0);
    }
}
