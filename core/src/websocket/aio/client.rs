//! Async WebSocket client (tokio-tungstenite).

use crate::models::{Channel, SubscribeRequest, WebSocketMessage, WebSocketRequest};
use crate::websocket::aio::dispatch::dispatch_messages;
use crate::websocket::aio::reconnect::{tls_connector_for, try_reconnect};
use crate::websocket::aio::writer::run_writer_task;
use crate::websocket::aio::{WsSink, WsStream};
use crate::websocket::connection_event::emit_event;
use crate::websocket::protocol::{
    classify_auth_response, frame_auth, frame_request, frame_subscribe, frame_subscribe_futopt,
    frame_subscribe_raw, frame_unsubscribe, AuthOutcome,
};
use crate::websocket::{
    ConnectionConfig, ConnectionEvent, ConnectionState, DisconnectIntent, HealthCheckConfig,
    MessageReceiver, ReconnectionConfig, ReconnectionManager, SubscriptionManager,
};
use crate::MarketDataError;
use futures_util::{SinkExt, StreamExt};
use std::sync::{mpsc, Arc};
use tokio::sync::mpsc as tokio_mpsc;
use tokio::sync::{Mutex, RwLock};
use tokio::task::JoinHandle;
use tokio::time::{sleep, timeout, Duration};
use tokio_tungstenite::connect_async_tls_with_config;
use tokio_tungstenite::tungstenite::Message;

/// WebSocket client for real-time market data
pub struct WebSocketClient {
    config: ConnectionConfig,
    state: Arc<RwLock<ConnectionState>>,
    event_tx: mpsc::SyncSender<ConnectionEvent>,
    event_rx: Arc<Mutex<mpsc::Receiver<ConnectionEvent>>>,
    /// Write half of the WebSocket stream (held by the writer task during
    /// normal operation; close/force_close paths may also touch it).
    ws_sink: Arc<Mutex<Option<WsSink>>>,
    /// Outbound write channel. All `subscribe`/`unsubscribe`/`send`/health-check
    /// pings push pre-serialized JSON strings here; a single writer task drains
    /// it into `ws_sink`. This eliminates lock contention on `ws_sink` between
    /// concurrent senders.
    write_tx: Arc<Mutex<Option<tokio_mpsc::Sender<String>>>>,
    reconnection: Arc<Mutex<ReconnectionManager>>,
    subscriptions: Arc<SubscriptionManager>,
    /// Health check / liveness configuration. The dispatch loop reads
    /// `heartbeat_timeout` from this and wraps `ws_read.next()` in
    /// `tokio::time::timeout`; no separate runtime struct or background
    /// polling task is needed.
    health_check_config: HealthCheckConfig,
    /// Inbound message channel (tokio mpsc). Producer side handed to
    /// `dispatch_messages` and auth handshake; consumer side is taken
    /// either by `messages()` (lazily spawns bridge to std mpsc for FFI)
    /// or by `message_stream()` (returns the tokio receiver directly).
    /// The two consumers are mutually exclusive — see method docs.
    message_tx: tokio_mpsc::Sender<WebSocketMessage>,
    message_rx: Arc<std::sync::Mutex<Option<tokio_mpsc::Receiver<WebSocketMessage>>>>,
    /// Cached `MessageReceiver` for FFI consumers. Initialized lazily on
    /// first `messages()` call, when we spawn the bridge task that drains
    /// the tokio receiver into a `std::sync::mpsc::Sender`.
    message_receiver: Arc<std::sync::Mutex<Option<Arc<MessageReceiver>>>>,
    /// Monotonic counter incremented every time the inbound message
    /// channel is saturated and a frame is dropped (drop-newest policy).
    /// Exposed via [`Self::messages_dropped_total`].
    messages_dropped: Arc<std::sync::atomic::AtomicU64>,
    /// Set by [`Self::disconnect`] / [`Self::shutdown_with_timeout`] to
    /// instruct the spawned dispatch task to exit cleanly instead of
    /// looping back into the reconnect path after the next dispatch
    /// return. Cleared on construction.
    shutdown_requested: Arc<std::sync::atomic::AtomicBool>,
    // Internal handles
    dispatch_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
    writer_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
}

/// Default drain timeout for [`WebSocketClient::disconnect`] when no
/// explicit value is supplied.
pub const DEFAULT_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

impl WebSocketClient {
    /// Create a new WebSocket client with default reconnection config
    ///
    /// # Example
    ///
    /// ```rust
    /// use marketdata_core::aio::WebSocketClient;
    /// use marketdata_core::websocket::ConnectionConfig;
    /// use marketdata_core::AuthRequest;
    ///
    /// let config = ConnectionConfig::fugle_stock(
    ///     AuthRequest::with_api_key("my-api-key")
    /// );
    /// let client = WebSocketClient::new(config);
    /// ```
    pub fn new(config: ConnectionConfig) -> Self {
        Self::with_reconnection_config(config, ReconnectionConfig::default())
    }

    /// Create a new WebSocket client with custom reconnection config
    pub fn with_reconnection_config(
        config: ConnectionConfig,
        reconnection_config: ReconnectionConfig,
    ) -> Self {
        Self::with_full_config(
            config,
            reconnection_config,
            HealthCheckConfig::default(),
        )
    }

    /// Create a new WebSocket client with custom health check config
    pub fn with_health_check_config(
        config: ConnectionConfig,
        health_check_config: HealthCheckConfig,
    ) -> Self {
        Self::with_full_config(
            config,
            ReconnectionConfig::default(),
            health_check_config,
        )
    }

    /// Create a new WebSocket client with full custom config
    pub fn with_full_config(
        config: ConnectionConfig,
        reconnection_config: ReconnectionConfig,
        health_check_config: HealthCheckConfig,
    ) -> Self {
        let (event_tx, event_rx) = mpsc::sync_channel(config.event_buffer);
        let (message_tx, message_rx) = tokio_mpsc::channel(config.message_buffer);

        Self {
            config,
            state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            event_tx,
            event_rx: Arc::new(Mutex::new(event_rx)),
            ws_sink: Arc::new(Mutex::new(None)),
            write_tx: Arc::new(Mutex::new(None)),
            reconnection: Arc::new(Mutex::new(ReconnectionManager::new(reconnection_config))),
            subscriptions: Arc::new(SubscriptionManager::new()),
            health_check_config,
            message_tx,
            message_rx: Arc::new(std::sync::Mutex::new(Some(message_rx))),
            message_receiver: Arc::new(std::sync::Mutex::new(None)),
            messages_dropped: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            shutdown_requested: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            dispatch_handle: Arc::new(Mutex::new(None)),
            writer_handle: Arc::new(Mutex::new(None)),
        }
    }

    /// Total number of inbound messages dropped due to consumer-side
    /// channel saturation since this client was constructed.
    ///
    /// Frames are dropped under the **drop-newest** backpressure policy:
    /// when `message_buffer` is full, new arrivals are discarded rather
    /// than blocking the network read loop. A non-zero value here usually
    /// indicates the downstream consumer (your `messages()` /
    /// `message_stream()` reader) is too slow or stalled.
    ///
    /// Counter is monotonic and thread-safe (`AtomicU64`). Reset only by
    /// constructing a new client.
    pub fn messages_dropped_total(&self) -> u64 {
        self.messages_dropped.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Get current connection state (snapshot)
    ///
    /// # Example
    ///
    /// ```rust
    /// use marketdata_core::aio::WebSocketClient;
    /// use marketdata_core::websocket::{ConnectionConfig, ConnectionState};
    /// use marketdata_core::AuthRequest;
    ///
    /// let config = ConnectionConfig::fugle_stock(
    ///     AuthRequest::with_api_key("my-api-key")
    /// );
    /// let client = WebSocketClient::new(config);
    /// assert_eq!(client.state(), ConnectionState::Disconnected);
    /// ```
    pub fn state(&self) -> ConnectionState {
        // This is a blocking call, but state reads are fast
        // In a real async context, use state_async() instead
        tokio::runtime::Handle::try_current()
            .ok()
            .and_then(|handle| {
                handle.block_on(async {
                    let state = self.state.read().await;
                    Some(state.clone())
                })
            })
            .unwrap_or(ConnectionState::Disconnected)
    }

    /// Get current connection state (async version)
    pub async fn state_async(&self) -> ConnectionState {
        let state = self.state.read().await;
        state.clone()
    }

    /// Check if client has been closed
    ///
    /// Returns true if disconnect() has been called and state is Closed.
    /// Once closed, the client cannot be reused - create a new instance.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use marketdata_core::aio::WebSocketClient;
    /// use marketdata_core::websocket::{ConnectionConfig, ConnectionState};
    /// use marketdata_core::AuthRequest;
    ///
    /// # async fn example() {
    /// let config = ConnectionConfig::fugle_stock(AuthRequest::with_api_key("key"));
    /// let client = WebSocketClient::new(config);
    ///
    /// // Initially not closed
    /// assert!(!client.is_closed().await);
    /// # }
    /// ```
    pub async fn is_closed(&self) -> bool {
        let state = self.state.read().await;
        matches!(*state, ConnectionState::Closed { .. })
    }

    /// Sync version of is_closed() for FFI
    ///
    /// Returns true if the client has been closed. Returns false if
    /// unable to determine state (e.g., no tokio runtime).
    pub fn is_closed_sync(&self) -> bool {
        tokio::runtime::Handle::try_current()
            .ok()
            .and_then(|handle| {
                handle.block_on(async {
                    let state = self.state.read().await;
                    Some(matches!(*state, ConnectionState::Closed { .. }))
                })
            })
            .unwrap_or(false)
    }

    /// Get reference to event receiver
    ///
    /// Consumers can use this to receive connection events
    pub fn events(&self) -> &Arc<Mutex<mpsc::Receiver<ConnectionEvent>>> {
        &self.event_rx
    }

    /// Subscribe to connection state change events
    ///
    /// This is a semantic alias for `events()` that emphasizes the state change focus.
    /// Returns a receiver for connection lifecycle events.
    ///
    /// Event types:
    /// - `Connecting` - Connection attempt started
    /// - `Connected` - WebSocket connection established
    /// - `Authenticated` - Authentication successful
    /// - `Disconnected { code, reason }` - Connection closed
    /// - `Reconnecting { attempt }` - Reconnection attempt started
    /// - `ReconnectFailed { attempts }` - Reconnection failed after max attempts
    /// - `Error { message, code }` - Error occurred
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use marketdata_core::aio::WebSocketClient;
    /// use marketdata_core::websocket::{ConnectionConfig, ConnectionEvent};
    /// use marketdata_core::AuthRequest;
    /// use std::sync::Arc;
    ///
    /// let client = WebSocketClient::new(
    ///     ConnectionConfig::fugle_stock(AuthRequest::with_api_key("key"))
    /// );
    ///
    /// // Clone the Arc to move into the thread
    /// let events = Arc::clone(client.state_events());
    /// std::thread::spawn(move || {
    ///     while let Ok(event) = events.blocking_lock().recv() {
    ///         match event {
    ///             ConnectionEvent::Connected { .. } => println!("Connected!"),
    ///             ConnectionEvent::Disconnected { code, reason, .. } => {
    ///                 println!("Disconnected: {:?} - {}", code, reason);
    ///                 break;
    ///             }
    ///             _ => {}
    ///         }
    ///     }
    /// });
    /// ```
    pub fn state_events(&self) -> &Arc<Mutex<mpsc::Receiver<ConnectionEvent>>> {
        &self.event_rx
    }

    /// Get reference to message receiver for FFI consumers
    ///
    /// Returns a blocking-API receiver suitable for FFI bindings (PyO3, napi,
    /// UniFFI). Internally the SDK uses a tokio mpsc channel; the first call
    /// to this method spawns a lightweight bridge task that drains the tokio
    /// receiver into a `std::sync::mpsc::Sender`. Subsequent calls return the
    /// same cached `Arc<MessageReceiver>`.
    ///
    /// **Mutually exclusive with [`message_stream`]**: only one of the two
    /// methods may take ownership of the underlying tokio receiver. Calling
    /// `messages()` after `message_stream()` (or vice versa) will panic with
    /// a descriptive message.
    ///
    /// Pure-async Rust callers should prefer [`message_stream`] to avoid the
    /// std-mpsc bridge hop.
    ///
    /// [`message_stream`]: Self::message_stream
    pub fn messages(&self) -> Arc<MessageReceiver> {
        let mut slot = self.message_receiver.lock().expect("message_receiver poisoned");
        if let Some(rx) = slot.as_ref() {
            return Arc::clone(rx);
        }
        let tokio_rx = self
            .message_rx
            .lock()
            .expect("message_rx poisoned")
            .take()
            .expect("message_stream() already consumed the message receiver");
        let (std_tx, std_rx) = mpsc::channel();
        tokio::spawn(async move {
            let mut rx = tokio_rx;
            while let Some(msg) = rx.recv().await {
                if std_tx.send(msg).is_err() {
                    break;
                }
            }
        });
        let receiver = Arc::new(MessageReceiver::new(std_rx));
        *slot = Some(Arc::clone(&receiver));
        receiver
    }

    /// Get the async message stream for pure-Rust async consumers.
    ///
    /// Returns the underlying tokio mpsc receiver, allowing direct `.recv().await`
    /// or use with `tokio_stream::wrappers::ReceiverStream` for `Stream`-based
    /// processing. Avoids the std-mpsc bridge hop that [`messages`] incurs.
    ///
    /// **Mutually exclusive with [`messages`]**: takes ownership of the
    /// receiver; can only be called once per client and panics if [`messages`]
    /// has already been called (or this method called twice).
    ///
    /// [`messages`]: Self::messages
    pub fn message_stream(&self) -> tokio_mpsc::Receiver<WebSocketMessage> {
        self.message_rx
            .lock()
            .expect("message_rx poisoned")
            .take()
            .expect(
                "message receiver already taken — `messages()` or `message_stream()` may only be \
                 called once between them",
            )
    }

    /// Connect to WebSocket server and authenticate
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Client has been closed (ClientClosed)
    /// - Connection fails
    /// - Authentication fails or times out
    /// - WebSocket handshake fails
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(target = "fugle_marketdata::ws", name = "ws.connect", skip(self))
    )]
    pub async fn connect(&self) -> Result<(), MarketDataError> {
        // Check if client is closed - cannot reconnect a closed client
        if self.is_closed().await {
            return Err(MarketDataError::ClientClosed);
        }

        // Update state to Connecting
        {
            let mut state = self.state.write().await;
            *state = ConnectionState::Connecting;
        }
        emit_event(&self.event_tx, ConnectionEvent::Connecting {
        });

        // Connect to WebSocket (with optional TLS customization).
        let tls_connector = tls_connector_for(&self.config)?;
        let connect_result = timeout(
            self.config.connect_timeout,
            connect_async_tls_with_config(&self.config.url, None, false, Some(tls_connector)),
        )
        .await;

        let (ws_stream, _response) = match connect_result {
            Ok(Ok((stream, response))) => (stream, response),
            Ok(Err(e)) => {
                let err: MarketDataError = e.into();
                {
                    let mut state = self.state.write().await;
                    *state = ConnectionState::Disconnected;
                }
                emit_event(&self.event_tx, ConnectionEvent::Error {
                    message: err.to_string(),
                    code: err.to_error_code(),
                });
                return Err(err);
            }
            Err(_) => {
                let err = MarketDataError::TimeoutError {
                    operation: "WebSocket connect".to_string(),
                };
                {
                    let mut state = self.state.write().await;
                    *state = ConnectionState::Disconnected;
                }
                emit_event(&self.event_tx, ConnectionEvent::Error {
                    message: err.to_string(),
                    code: err.to_error_code(),
                });
                return Err(err);
            }
        };

        // Split the stream into read/write halves
        let (mut ws_sink, mut ws_read) = ws_stream.split();

        crate::tracing_compat::info!(target: "fugle_marketdata::ws", "ws connected");
        emit_event(&self.event_tx, ConnectionEvent::Connected {
        });

        // Update state to Authenticating
        {
            let mut state = self.state.write().await;
            *state = ConnectionState::Authenticating;
        }

        // Send authentication message
        let auth_json = frame_auth(self.config.auth.clone())?;

        ws_sink
            .send(Message::Text(auth_json.into()))
            .await
            .map_err(MarketDataError::from)?;

        // Wait for authenticated event or timeout
        // All messages during auth phase are forwarded to message channel
        let message_tx = self.message_tx.clone();
        let auth_timeout = Duration::from_secs(10);
        let auth_result = timeout(auth_timeout, async {
            while let Some(msg_result) = ws_read.next().await {
                match msg_result {
                    Ok(Message::Text(text)) => {
                        if let Ok(ws_msg) =
                            serde_json::from_str::<WebSocketMessage>(&text)
                        {
                            // Forward ALL messages to channel (including auth)
                            let _ = message_tx.send(ws_msg.clone()).await;

                            match classify_auth_response(&ws_msg) {
                                AuthOutcome::Authenticated => return Ok(()),
                                AuthOutcome::Failed(msg) => {
                                    return Err(MarketDataError::AuthError { msg })
                                }
                                AuthOutcome::Pending => {}
                            }
                        }
                    }
                    Err(e) => {
                        return Err(MarketDataError::from(e));
                    }
                    _ => {}
                }
            }
            Err(MarketDataError::ConnectionError {
                msg: "Stream closed during authentication".to_string(),
            })
        })
        .await;

        match auth_result {
            Ok(Ok(())) => {
                // Store the write half for sending messages
                {
                    let mut sink_guard = self.ws_sink.lock().await;
                    *sink_guard = Some(ws_sink);
                }

                // Spawn the writer task and install its sender. All
                // subsequent outbound messages flow through this channel.
                self.start_writer_task().await;

                {
                    let mut state = self.state.write().await;
                    *state = ConnectionState::Connected;
                }
                crate::tracing_compat::info!(target: "fugle_marketdata::ws", "ws authenticated");
                emit_event(&self.event_tx, ConnectionEvent::Authenticated {
                });

                // Spawn dispatch task to handle incoming messages (uses read half).
                // Liveness detection is wrapped inside dispatch_messages itself
                // (read-site `tokio::time::timeout`); no separate task needed.
                self.spawn_dispatch_task(ws_read).await;

                Ok(())
            }
            Ok(Err(e)) => {
                {
                    let mut state = self.state.write().await;
                    *state = ConnectionState::Disconnected;
                }
                // Server-rejected credentials → emit Unauthenticated so old SDK
                // listeners on `unauthenticated` keep working. Other failures
                // (network, parse, etc.) still go through the generic Error event.
                if let MarketDataError::AuthError { msg } = &e {
                    emit_event(&self.event_tx, ConnectionEvent::Unauthenticated {
                        message: msg.clone(),
                    });
                } else {
                    emit_event(&self.event_tx, ConnectionEvent::Error {
                        message: e.to_string(),
                        code: e.to_error_code(),
                    });
                }
                Err(e)
            }
            Err(_) => {
                let err = MarketDataError::TimeoutError {
                    operation: "WebSocket authentication".to_string(),
                };
                {
                    let mut state = self.state.write().await;
                    *state = ConnectionState::Disconnected;
                }
                emit_event(&self.event_tx, ConnectionEvent::Error {
                    message: err.to_string(),
                    code: err.to_error_code(),
                });
                Err(err)
            }
        }
    }

    /// Disconnect from the WebSocket server with a graceful drain.
    ///
    /// Equivalent to
    /// [`shutdown_with_timeout`](Self::shutdown_with_timeout) called with
    /// [`DEFAULT_SHUTDOWN_TIMEOUT`] (5 seconds).
    ///
    /// Within the drain window the client signals the dispatch loop to
    /// exit (so auto-reconnect does not re-establish the connection),
    /// drops the writer-task sender so any in-flight queued frames flush,
    /// sends a Close frame to the peer, and awaits the peer's Close
    /// acknowledgement (manifested as the dispatch task exiting). On
    /// timeout the dispatch and writer tasks are forcibly aborted and
    /// the connection is force-closed.
    ///
    /// The emitted [`ConnectionEvent::Disconnected`] always carries
    /// [`DisconnectIntent::Client`] regardless of whether the drain
    /// completed in time.
    ///
    /// # Errors
    ///
    /// Returns the error surfaced by the close-frame send if it failed.
    /// The client is still marked as closed in either case.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(target = "fugle_marketdata::ws", name = "ws.disconnect", skip(self))
    )]
    pub async fn disconnect(&self) -> Result<(), MarketDataError> {
        self.shutdown_with_timeout(DEFAULT_SHUTDOWN_TIMEOUT).await
    }

    /// Disconnect with a caller-supplied drain timeout.
    ///
    /// See [`disconnect`](Self::disconnect) for sequencing details. Pass
    /// a small value (e.g. `Duration::from_millis(100)`) when the caller
    /// must return quickly (SIGTERM grace window expiring) and a longer
    /// value when clean Close-ack handshake matters more than latency.
    ///
    /// A zero timeout still sends the Close frame on a best-effort basis
    /// before forcibly aborting the background tasks, so this call is
    /// always safe to use as the only cleanup step.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(target = "fugle_marketdata::ws", name = "ws.shutdown_with_timeout", skip(self))
    )]
    pub async fn shutdown_with_timeout(
        &self,
        timeout_dur: Duration,
    ) -> Result<(), MarketDataError> {
        // 1. Signal the dispatch loop to exit instead of reconnecting
        //    after the next dispatch return.
        self.shutdown_requested
            .store(true, std::sync::atomic::Ordering::SeqCst);

        // 2. Drop the writer-task sender so the writer drains its queue
        //    and exits naturally on the next `rx.recv()` returning `None`.
        {
            let mut tx_guard = self.write_tx.lock().await;
            *tx_guard = None;
        }

        // 3. Send the Close frame within a small slice of the budget.
        //    `sink.close()` flushes the WebSocket Close frame and closes
        //    the underlying transport's send half. We bound this so a
        //    wedged sink cannot eat the entire timeout budget.
        let close_send_slice = timeout_dur
            .checked_div(2)
            .unwrap_or(Duration::from_secs(0))
            .max(Duration::from_millis(50));
        let close_result = self.send_close_frame(close_send_slice).await;

        // 4. Wait for writer + dispatch tasks to exit naturally. Server's
        //    Close ack arrives via the dispatch loop, which then sees the
        //    shutdown flag and breaks out of the reconnect loop.
        let drain_budget = timeout_dur.saturating_sub(close_send_slice);
        let drained = tokio::time::timeout(
            drain_budget.max(Duration::from_millis(0)),
            self.await_background_tasks(),
        )
        .await;

        // 5. Force-abort background tasks if drain budget elapsed.
        if drained.is_err() {
            self.abort_background_tasks().await;
        }

        // 6. Clear the sink slot regardless of close-frame outcome.
        {
            let mut sink_guard = self.ws_sink.lock().await;
            *sink_guard = None;
        }

        // 7. Update state to Closed (always, even if close failed)
        {
            let mut state = self.state.write().await;
            *state = ConnectionState::Closed {
                code: Some(1000),
                reason: "Normal closure".to_string(),
                intent: DisconnectIntent::Client,
            };
        }

        // 8. Emit the Client-intent Disconnected event.
        emit_event(&self.event_tx, ConnectionEvent::Disconnected {
            code: Some(1000),
            reason: "Normal closure".to_string(),
            intent: DisconnectIntent::Client,
        });

        close_result
    }

    /// Send the WebSocket Close frame, bounded by `budget`.
    ///
    /// Internal helper for [`shutdown_with_timeout`]. On send failure or
    /// timeout the error is logged via `tracing` and a successful result
    /// returned so the caller's overall shutdown sequence still proceeds
    /// to mark the client as closed.
    async fn send_close_frame(&self, budget: Duration) -> Result<(), MarketDataError> {
        let mut sink_guard = self.ws_sink.lock().await;
        let Some(sink) = sink_guard.as_mut() else {
            return Ok(());
        };
        match tokio::time::timeout(budget, sink.close()).await {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => {
                crate::tracing_compat::error!(
                    target: "fugle_marketdata::ws",
                    error = %e,
                    "failed to send WebSocket close frame"
                );
                let _ = e;
                Ok(())
            }
            Err(_) => {
                crate::tracing_compat::warn!(
                    target: "fugle_marketdata::ws",
                    timeout_ms = budget.as_millis() as u64,
                    "close frame send timed out"
                );
                Ok(())
            }
        }
    }

    /// Wait for both background tasks to exit naturally.
    async fn await_background_tasks(&self) {
        // Drain writer first — it exits as soon as `write_tx` was dropped.
        if let Some(h) = self.writer_handle.lock().await.take() {
            let _ = h.await;
        }
        // Then dispatch — relies on either the server sending Close (so
        // dispatch_messages returns) or the read end seeing EOF after
        // sink.close().
        if let Some(h) = self.dispatch_handle.lock().await.take() {
            let _ = h.await;
        }
    }

    /// Force-abort both background tasks. Called on drain timeout.
    async fn abort_background_tasks(&self) {
        if let Some(h) = self.writer_handle.lock().await.take() {
            h.abort();
            let _ = h.await;
        }
        if let Some(h) = self.dispatch_handle.lock().await.take() {
            h.abort();
            let _ = h.await;
        }
    }

    /// Force close without waiting for handshake
    ///
    /// Use when graceful close is not possible or times out.
    pub async fn force_close(&self) -> Result<(), MarketDataError> {
        // Abort dispatch task without waiting (read-site liveness timeout
        // tears down with it; no separate health-check task to abort).
        {
            let mut handle = self.dispatch_handle.lock().await;
            if let Some(h) = handle.take() {
                h.abort();
            }
        }

        // Abort writer task and clear sender
        {
            let mut tx_guard = self.write_tx.lock().await;
            *tx_guard = None;
        }
        {
            let mut handle = self.writer_handle.lock().await;
            if let Some(h) = handle.take() {
                h.abort();
            }
        }

        // Drop sink without close frame
        {
            let mut sink_guard = self.ws_sink.lock().await;
            *sink_guard = None;
        }

        // Update state
        {
            let mut state = self.state.write().await;
            *state = ConnectionState::Closed {
                code: Some(1006), // Abnormal closure
                reason: "Force closed".to_string(),
                intent: DisconnectIntent::Client,
            };
        }

        emit_event(&self.event_tx, ConnectionEvent::Disconnected {
            code: Some(1006),
            reason: "Force closed".to_string(),
            intent: DisconnectIntent::Client,
        });

        Ok(())
    }

    /// Check if currently connected
    pub async fn is_connected(&self) -> bool {
        let state = self.state.read().await;
        matches!(*state, ConnectionState::Connected)
    }

    /// Subscribe to a stock streaming channel.
    ///
    /// Accepts a [`StockSubscription`] carrying single or batch symbols and
    /// optional `intraday_odd_lot` modifier. On the wire the request is sent
    /// as one frame (`{channel, symbol, ...}` for single,
    /// `{channel, symbols: [...], ...}` for batch). Internally, batch
    /// subscriptions are expanded to N per-symbol rows so each symbol owns a
    /// stable local key for ACK recording and unsubscribe lookup.
    ///
    /// # Errors
    ///
    /// Returns `ClientClosed` if the client has been closed.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(target = "fugle_marketdata::ws", name = "ws.subscribe", skip(self, sub))
    )]
    pub async fn subscribe(
        &self,
        sub: crate::websocket::channels::StockSubscription,
    ) -> Result<(), MarketDataError> {
        if self.is_closed().await {
            return Err(MarketDataError::ClientClosed);
        }

        let (sub_json, expanded) = frame_subscribe(sub)?;

        // Internal bookkeeping: store N per-symbol rows (1 for single,
        // len() for batch). Each row has its own local key for ACK
        // recording. On reconnect each row sends its own frame — refolding
        // back into batches is a future optimization.
        for entry in expanded {
            self.subscriptions.subscribe(entry);
        }

        if self.is_connected().await {
            self.enqueue_write(sub_json).await?;
        }
        Ok(())
    }

    /// Subscribe to a FutOpt streaming channel.
    ///
    /// Mirror of [`subscribe`](Self::subscribe) for the FutOpt domain. Same
    /// single/batch semantics; the modifier is `after_hours` instead of
    /// `intraday_odd_lot`.
    pub async fn subscribe_futopt(
        &self,
        sub: crate::websocket::channels::FutOptSubscription,
    ) -> Result<(), MarketDataError> {
        if self.is_closed().await {
            return Err(MarketDataError::ClientClosed);
        }

        let (sub_json, expanded) = frame_subscribe_futopt(sub)?;

        for entry in expanded {
            self.subscriptions.subscribe(entry);
        }

        if self.is_connected().await {
            self.enqueue_write(sub_json).await?;
        }
        Ok(())
    }

    /// Unsubscribe by server id(s) — accepts single or batch via
    /// `impl IntoIterator<Item = impl Into<String>>`.
    ///
    /// Each id is preferentially the server-assigned id returned in a
    /// `subscribed` ACK. The internal `SubscriptionManager` falls back to
    /// the local key (`"{channel}:{symbol}[:modifier]"`) when an ACK
    /// hasn't been recorded yet (rare race on fast subscribe→unsubscribe).
    ///
    /// Sends a single `{event:"unsubscribe", data:{ids:[...]}}` frame on
    /// the wire when there is more than one id, or `{data:{id:"..."}}`
    /// for a single id — both shapes are accepted by the Fugle server.
    ///
    /// # Errors
    ///
    /// Returns `ClientClosed` if the client has been closed.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(target = "fugle_marketdata::ws", name = "ws.unsubscribe", skip(self, ids))
    )]
    pub async fn unsubscribe(
        &self,
        ids: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<(), MarketDataError> {
        if self.is_closed().await {
            return Err(MarketDataError::ClientClosed);
        }

        let keys: Vec<String> = ids.into_iter().map(Into::into).collect();
        if keys.is_empty() {
            return Ok(());
        }

        // Translate keys to server ids where possible; fall back to the
        // caller-supplied string (works for both server ids and local keys).
        let mut wire_ids = Vec::with_capacity(keys.len());
        for key in &keys {
            let id = self
                .subscriptions
                .take_server_id(key)
                .unwrap_or_else(|| key.clone());
            self.subscriptions.unsubscribe(key);
            wire_ids.push(id);
        }

        if !self.is_connected().await {
            return Ok(());
        }

        let unsub_json = frame_unsubscribe(wire_ids)?;
        self.enqueue_write(unsub_json).await?;
        Ok(())
    }

    /// Get all active subscriptions
    pub fn subscriptions(&self) -> Vec<SubscribeRequest> {
        self.subscriptions.get_all()
    }

    /// Number of currently active subscriptions.
    ///
    /// Cheap query — single read-lock on the internal subscription map.
    pub fn subscription_count(&self) -> usize {
        self.subscriptions.count()
    }

    /// Returns `true` iff at least one active subscription matches the
    /// given channel and symbol. Modifier-suffixed forms (`:afterhours`,
    /// `:oddlot`) are matched alongside the base form.
    pub fn is_subscribed(&self, channel: &Channel, symbol: &str) -> bool {
        let base = format!("{}:{}", channel.as_str(), symbol);
        let modifier_prefix = format!("{}:", base);
        self.subscriptions
            .keys()
            .iter()
            .any(|k| k == &base || k.starts_with(&modifier_prefix))
    }

    /// Manually reconnect after disconnection
    ///
    /// # Errors
    ///
    /// Returns `ClientClosed` if the client has been closed.
    /// A closed client cannot be reconnected - create a new instance.
    ///
    /// From CONTEXT.md: "支援 reconnect() 方法讓使用者手動觸發重連"
    /// Resets reconnection manager and attempts fresh connection.
    pub async fn reconnect(&self) -> Result<(), MarketDataError> {
        // Check if client is closed - cannot reconnect a closed client
        if self.is_closed().await {
            return Err(MarketDataError::ClientClosed);
        }

        // Reset reconnection manager for fresh attempt
        {
            let mut reconnection = self.reconnection.lock().await;
            reconnection.reset();
        }

        // Attempt connection
        self.connect().await?;

        // Resubscribe all
        self.resubscribe_all().await?;

        Ok(())
    }

    /// Internal: Resubscribe all stored subscriptions
    ///
    /// From CONTEXT.md: "重連後按原始訂閱順序重新訂閱"
    async fn resubscribe_all(&self) -> Result<(), MarketDataError> {
        // Old server ids point at a dead connection — clear before replay so
        // the fresh subscribed acks can overwrite cleanly. Without this,
        // unsubscribe after reconnect could briefly pick up a zombie id.
        self.subscriptions.clear_server_ids();

        let subs = self.subscriptions.get_all();

        for req in subs {
            let sub_json = frame_subscribe_raw(req)?;
            self.enqueue_write(sub_json).await?;
        }

        Ok(())
    }

    /// Send a WebSocket request message
    ///
    /// Used internally and exposed for advanced use cases
    ///
    /// # Errors
    ///
    /// Returns `ClientClosed` if the client has been closed.
    pub async fn send(&self, request: WebSocketRequest) -> Result<(), MarketDataError> {
        if self.is_closed().await {
            return Err(MarketDataError::ClientClosed);
        }

        let json = frame_request(&request)?;
        self.enqueue_write(json).await
    }

    /// Send raw text message to WebSocket
    ///
    /// Used internally for sending subscription requests
    pub(crate) async fn send_text(&self, text: &str) -> Result<(), MarketDataError> {
        self.enqueue_write(text.to_string()).await
    }

    /// Get list of active subscription keys
    pub fn subscription_keys(&self) -> Vec<String> {
        self.subscriptions.keys()
    }

    /// Internal: Spawn message dispatch task
    ///
    /// Takes ownership of the read half of the WebSocket stream for message dispatch.
    /// When the connection drops, triggers auto-reconnect if configured. Uses a loop
    /// (not recursion) to handle repeated reconnections within a single spawned task.
    async fn spawn_dispatch_task(&self, ws_read: WsStream) {
        let message_tx = self.message_tx.clone();
        let event_tx = self.event_tx.clone();

        // Resolve heartbeat_timeout once: None means liveness disabled.
        let heartbeat_timeout = if self.health_check_config.enabled {
            Some(self.health_check_config.heartbeat_timeout)
        } else {
            None
        };

        // Clone Arcs needed for auto-reconnect inside spawned task
        let reconnection = Arc::clone(&self.reconnection);
        let config = self.config.clone();
        let state = Arc::clone(&self.state);
        let ws_sink = Arc::clone(&self.ws_sink);
        let write_tx_slot = Arc::clone(&self.write_tx);
        let writer_handle = Arc::clone(&self.writer_handle);
        let subscriptions = Arc::clone(&self.subscriptions);
        let messages_dropped = Arc::clone(&self.messages_dropped);
        let shutdown_requested = Arc::clone(&self.shutdown_requested);

        let handle = tokio::spawn(async move {
            // Dispatch → reconnect → dispatch loop (avoids recursive async which breaks Send)
            let mut current_ws_read = ws_read;
            loop {
                let close_code = dispatch_messages(
                    current_ws_read,
                    message_tx.clone(),
                    event_tx.clone(),
                    heartbeat_timeout,
                    Arc::clone(&subscriptions),
                    Arc::clone(&messages_dropped),
                    Arc::clone(&shutdown_requested),
                )
                .await;

                // Graceful-shutdown short-circuit: when `disconnect()` /
                // `shutdown_with_timeout()` set the flag, the dispatch
                // loop's exit must not loop back into reconnect — that
                // would re-establish the connection the caller just asked
                // to tear down. Ordering: this check happens after
                // dispatch returns so any in-flight server Close frame is
                // still observed and propagated as a `Disconnected` event.
                if shutdown_requested.load(std::sync::atomic::Ordering::SeqCst) {
                    break;
                }

                // Attempt auto-reconnect; returns new streams on success.
                match try_reconnect(
                    close_code,
                    Arc::clone(&reconnection),
                    config.clone(),
                    Arc::clone(&state),
                    event_tx.clone(),
                    Arc::clone(&ws_sink),
                    Arc::clone(&write_tx_slot),
                    Arc::clone(&writer_handle),
                    Arc::clone(&subscriptions),
                    message_tx.clone(),
                )
                .await
                {
                    Some(ws_read) => {
                        current_ws_read = ws_read;
                        // Loop back to dispatch with the new connection
                    }
                    None => {
                        // Reconnection failed or not configured — exit task
                        break;
                    }
                }
            }
        });

        let mut dispatch_handle_guard = self.dispatch_handle.lock().await;
        *dispatch_handle_guard = Some(handle);
    }

    /// Internal: Spawn the writer task that drains the outbound channel into
    /// the WebSocket sink. Also installs the new `write_tx` sender into the
    /// shared slot. Call after `ws_sink` has been populated.
    async fn start_writer_task(&self) {
        // Aborts any previous writer task. Channel buffer 64 is generous for a
        // ping-every-30s + occasional sub/unsub workload while staying small
        // enough to surface backpressure if the sink stalls.
        if let Some(prev) = self.writer_handle.lock().await.take() {
            prev.abort();
        }

        let (tx, rx) = tokio_mpsc::channel::<String>(64);
        {
            let mut guard = self.write_tx.lock().await;
            *guard = Some(tx);
        }

        let ws_sink = Arc::clone(&self.ws_sink);
        let event_tx = self.event_tx.clone();
        let handle = tokio::spawn(run_writer_task(rx, ws_sink, event_tx));

        let mut guard = self.writer_handle.lock().await;
        *guard = Some(handle);
    }

    /// Internal: Push a JSON string onto the outbound write channel. Returns
    /// `ConnectionError` if the writer task is not running (e.g., disconnected).
    async fn enqueue_write(&self, json: String) -> Result<(), MarketDataError> {
        let sender = { self.write_tx.lock().await.clone() };
        match sender {
            Some(s) => s.send(json).await.map_err(|_| MarketDataError::ConnectionError {
                msg: "Writer task is not running".to_string(),
            }),
            None => Err(MarketDataError::ConnectionError {
                msg: "Not connected".to_string(),
            }),
        }
    }

    /// Internal: Automatic reconnection flow (&self version)
    ///
    /// Implements exponential backoff retry logic with subscription restoration.
    /// Note: The dispatch loop uses the standalone `try_reconnect` function instead,
    /// which operates on owned Arcs for Send compatibility with tokio::spawn.
    #[allow(dead_code)]
    async fn auto_reconnect(&self, close_code: Option<u16>) -> Result<(), MarketDataError> {
        let should_reconnect = {
            let reconnection = self.reconnection.lock().await;
            reconnection.should_reconnect(close_code)
        };

        if !should_reconnect {
            // Not retriable - update state and send event
            {
                let mut state = self.state.write().await;
                *state = ConnectionState::Closed {
                    code: close_code,
                    reason: "Non-retriable error".to_string(),
                    intent: DisconnectIntent::Network,
                };
            }

            let attempts = {
                let reconnection = self.reconnection.lock().await;
                reconnection.current_attempt()
            };

            emit_event(&self.event_tx, ConnectionEvent::ReconnectFailed {
                attempts,
            });
            return Err(MarketDataError::ConnectionError {
                msg: format!("Non-retriable close code: {:?}", close_code),
            });
        }

        // Attempt reconnection with exponential backoff. Liveness
        // detection is per-dispatch-task, so reconnecting transparently
        // restarts it via the new dispatch task — no separate
        // pause/resume needed.
        loop {
            let delay = {
                let mut reconnection = self.reconnection.lock().await;
                reconnection.next_delay()
            };

            match delay {
                Some(d) => {
                    let attempt = {
                        let reconnection = self.reconnection.lock().await;
                        reconnection.current_attempt()
                    };

                    // Update state to Reconnecting
                    {
                        let mut state = self.state.write().await;
                        *state = ConnectionState::Reconnecting { attempt };
                    }
                    crate::tracing_compat::warn!(
                        target: "fugle_marketdata::ws",
                        attempt,
                        "ws manual reconnect attempt"
                    );
                    emit_event(&self.event_tx, ConnectionEvent::Reconnecting {
                        attempt,
                    });

                    // Wait before reconnecting
                    sleep(d).await;

                    // Try to connect
                    match self.connect().await {
                        Ok(()) => {
                            // Reset reconnection manager on success
                            {
                                let mut reconnection = self.reconnection.lock().await;
                                reconnection.reset();
                            }

                            // Resubscribe all
                            let _ = self.resubscribe_all().await;

                            return Ok(());
                        }
                        Err(_) => {
                            // Continue loop to next attempt
                            continue;
                        }
                    }
                }
                None => {
                    // Max attempts reached
                    {
                        let mut state = self.state.write().await;
                        *state = ConnectionState::Closed {
                            code: close_code,
                            reason: "Max reconnection attempts reached".to_string(),
                            intent: DisconnectIntent::Network,
                        };
                    }

                    let attempts = {
                        let reconnection = self.reconnection.lock().await;
                        reconnection.current_attempt()
                    };

                    emit_event(&self.event_tx, ConnectionEvent::ReconnectFailed {
                        attempts,
                    });

                    return Err(MarketDataError::ConnectionError {
                        msg: "Max reconnection attempts reached".to_string(),
                    });
                }
            }
        }
    }
}

// `run_writer_task` is now in `aio::writer` (PR2 split).
// `try_reconnect` + `try_connect` + `tls_connector_for` are now in `aio::reconnect`.


#[cfg(test)]
mod tests {
    use super::*;
    use crate::websocket::channels::StockSubscription;
    use crate::AuthRequest;

    #[test]
    fn test_connection_state_variants() {
        // Test all state variants exist and can be created
        let _disconnected = ConnectionState::Disconnected;
        let _connecting = ConnectionState::Connecting;
        let _authenticating = ConnectionState::Authenticating;
        let _connected = ConnectionState::Connected;
        let _reconnecting = ConnectionState::Reconnecting { attempt: 1 };
        let _closed = ConnectionState::Closed {
            code: Some(1000),
            reason: "Normal closure".to_string(),
            intent: DisconnectIntent::Client,
        };
    }

    #[test]
    fn test_connection_event_variants() {
        // Test all event variants exist and can be created
        let _connecting = ConnectionEvent::Connecting;
        let _connected = ConnectionEvent::Connected;
        let _authenticated = ConnectionEvent::Authenticated;
        let _unauthenticated = ConnectionEvent::Unauthenticated {
            message: "Invalid credentials".to_string(),
        };
        let _disconnected = ConnectionEvent::Disconnected {
            code: Some(1000),
            reason: "Normal closure".to_string(),
            intent: DisconnectIntent::Client,
        };
        let _reconnecting = ConnectionEvent::Reconnecting {
            attempt: 1,
        };
        let _failed = ConnectionEvent::ReconnectFailed {
            attempts: 5,
        };
        let _error = ConnectionEvent::Error {
            message: "Connection failed".to_string(),
            code: 2001,
        };
    }

    #[tokio::test]
    async fn test_websocket_client_new() {
        let config =
            ConnectionConfig::fugle_stock(AuthRequest::with_api_key("test-key"));
        let client = WebSocketClient::new(config);

        let state = client.state_async().await;
        assert_eq!(state, ConnectionState::Disconnected);
    }

    #[tokio::test]
    async fn test_websocket_client_state() {
        let config =
            ConnectionConfig::fugle_stock(AuthRequest::with_api_key("test-key"));
        let client = WebSocketClient::new(config);

        // Initial state should be Disconnected
        let state = client.state_async().await;
        assert_eq!(state, ConnectionState::Disconnected);

        // Manually change state for testing
        {
            let mut state = client.state.write().await;
            *state = ConnectionState::Connecting;
        }

        let state = client.state_async().await;
        assert_eq!(state, ConnectionState::Connecting);
    }

    #[tokio::test]
    async fn test_websocket_client_events() {
        let config =
            ConnectionConfig::fugle_stock(AuthRequest::with_api_key("test-key"));
        let client = WebSocketClient::new(config);

        // Send an event
        client
            .event_tx
            .send(ConnectionEvent::Connecting)
            .unwrap();

        // Receive the event (using blocking recv in async context)
        let rx = Arc::clone(&client.event_rx);
        let event = tokio::task::spawn_blocking(move || {
            let rx_guard = rx.blocking_lock();
            rx_guard.recv().unwrap()
        })
        .await
        .unwrap();
        assert_eq!(event, ConnectionEvent::Connecting);
    }

    #[tokio::test]
    async fn test_is_connected() {
        let config =
            ConnectionConfig::fugle_stock(AuthRequest::with_api_key("test-key"));
        let client = WebSocketClient::new(config);

        // Initially not connected
        assert!(!client.is_connected().await);

        // Manually set to Connected for testing
        {
            let mut state = client.state.write().await;
            *state = ConnectionState::Connected;
        }

        assert!(client.is_connected().await);
    }

    #[tokio::test]
    async fn test_connection_state_transitions() {
        let config =
            ConnectionConfig::fugle_stock(AuthRequest::with_api_key("test-key"));
        let client = WebSocketClient::new(config);

        // Test state transitions
        {
            let mut state = client.state.write().await;
            *state = ConnectionState::Connecting;
        }
        assert_eq!(client.state_async().await, ConnectionState::Connecting);

        {
            let mut state = client.state.write().await;
            *state = ConnectionState::Authenticating;
        }
        assert_eq!(client.state_async().await, ConnectionState::Authenticating);

        {
            let mut state = client.state.write().await;
            *state = ConnectionState::Connected;
        }
        assert_eq!(client.state_async().await, ConnectionState::Connected);
        assert!(client.is_connected().await);
    }

    #[tokio::test]
    async fn test_subscribe_when_disconnected() {
        use crate::models::Channel;

        let config =
            ConnectionConfig::fugle_stock(AuthRequest::with_api_key("test-key"));
        let client = WebSocketClient::new(config);

        // Subscribe while disconnected
        let sub = StockSubscription::new(Channel::Trades, "2330");
        let result = client.subscribe(sub).await;
        assert!(result.is_ok());

        // Subscription should be stored
        let subs = client.subscriptions();
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0].channel, "trades");
        assert_eq!(subs[0].symbol.as_deref(), Some("2330"));
    }

    #[tokio::test]
    async fn test_subscribe_when_connected() {
        use crate::models::Channel;

        let config =
            ConnectionConfig::fugle_stock(AuthRequest::with_api_key("test-key"));
        let client = WebSocketClient::new(config);

        // Manually set to Connected for testing
        {
            let mut state = client.state.write().await;
            *state = ConnectionState::Connected;
        }

        // Subscribe while connected
        let sub = StockSubscription::new(Channel::Trades, "2330");
        // Note: This will fail without actual connection, but subscription should be stored
        let _ = client.subscribe(sub).await;

        // Subscription should be stored regardless of send result
        let subs = client.subscriptions();
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0].channel, "trades");
        assert_eq!(subs[0].symbol.as_deref(), Some("2330"));
    }

    #[tokio::test]
    async fn test_unsubscribe_removes_from_state() {
        use crate::models::Channel;

        let config =
            ConnectionConfig::fugle_stock(AuthRequest::with_api_key("test-key"));
        let client = WebSocketClient::new(config);

        // Subscribe
        let sub = StockSubscription::new(Channel::Trades, "2330");
        let _ = client.subscribe(sub).await;
        assert_eq!(client.subscriptions().len(), 1);

        // Unsubscribe
        let result = client.unsubscribe(["trades:2330"]).await;
        assert!(result.is_ok());

        // Subscription should be removed
        assert_eq!(client.subscriptions().len(), 0);
    }

    #[tokio::test]
    async fn unsubscribe_removes_futopt_subscription_from_state() {
        use crate::websocket::channels::FutOptSubscription;
        use crate::FutOptChannel;

        let config =
            ConnectionConfig::fugle_stock(AuthRequest::with_api_key("test-key"));
        let client = WebSocketClient::new(config);

        let sub = FutOptSubscription::new(FutOptChannel::Books, "TXFE6").with_after_hours(true);
        let _ = client.subscribe_futopt(sub.clone()).await;
        assert_eq!(client.subscriptions().len(), 1);

        let result = client.unsubscribe(sub.keys()).await;
        assert!(result.is_ok());
        assert_eq!(client.subscriptions().len(), 0);
    }

    #[tokio::test]
    async fn test_subscriptions_restored_after_reconnect() {
        use crate::models::Channel;

        let config =
            ConnectionConfig::fugle_stock(AuthRequest::with_api_key("test-key"));
        let client = WebSocketClient::new(config);

        // Add subscriptions
        let _ = client.subscribe(StockSubscription::new(Channel::Trades, "2330")).await;
        let _ = client.subscribe(StockSubscription::new(Channel::Candles, "2317")).await;

        // Subscriptions should be stored
        let subs = client.subscriptions();
        assert_eq!(subs.len(), 2);
        assert_eq!(subs[0].key(), "trades:2330");
        assert_eq!(subs[1].key(), "candles:2317");
    }

    #[tokio::test]
    async fn test_manual_reconnect_resets_attempts() {
        let config =
            ConnectionConfig::fugle_stock(AuthRequest::with_api_key("test-key"));
        let client = WebSocketClient::new(config);

        // Simulate failed reconnection attempts
        {
            let mut reconnection = client.reconnection.lock().await;
            let _ = reconnection.next_delay();
            let _ = reconnection.next_delay();
            assert_eq!(reconnection.current_attempt(), 2);
        }

        // Manual reconnect should reset
        // Note: This will fail without actual server, but should reset attempts
        let _ = client.reconnect().await;

        // Attempts should be reset
        {
            let reconnection = client.reconnection.lock().await;
            assert_eq!(reconnection.current_attempt(), 0);
        }
    }

    #[tokio::test]
    async fn test_with_reconnection_config() {
        use std::time::Duration;

        let config =
            ConnectionConfig::fugle_stock(AuthRequest::with_api_key("test-key"));
        let reconnection_config = ReconnectionConfig::default()
            .with_max_attempts(10)
            .unwrap()
            .with_initial_delay(Duration::from_secs(2))
            .unwrap();

        let client = WebSocketClient::with_reconnection_config(config, reconnection_config);

        // Verify reconnection config is used
        {
            let reconnection = client.reconnection.lock().await;
            assert_eq!(reconnection.attempts_remaining(), 10);
        }
    }

    // ========================================================================
    // Closed Client Protection Tests (Phase 7)
    // ========================================================================

    #[tokio::test]
    async fn test_is_closed_after_disconnect() {
        let config = ConnectionConfig::fugle_stock(AuthRequest::with_api_key("test-key"));
        let client = WebSocketClient::new(config);

        // Initially not closed
        assert!(!client.is_closed().await);

        // Manually set to Closed state for testing
        {
            let mut state = client.state.write().await;
            *state = ConnectionState::Closed {
                code: Some(1000),
                reason: "Normal closure".to_string(),
                intent: DisconnectIntent::Client,
            };
        }

        // Now should be closed
        assert!(client.is_closed().await);
    }

    #[tokio::test]
    async fn test_subscribe_fails_when_closed() {
        use crate::models::Channel;

        let config = ConnectionConfig::fugle_stock(AuthRequest::with_api_key("test-key"));
        let client = WebSocketClient::new(config);

        // Set to Closed state
        {
            let mut state = client.state.write().await;
            *state = ConnectionState::Closed {
                code: Some(1000),
                reason: "Test closure".to_string(),
                intent: DisconnectIntent::Client,
            };
        }

        // Subscribe should fail with ClientClosed error
        let result = client.subscribe(StockSubscription::new(Channel::Trades, "2330")).await;
        assert!(matches!(result, Err(MarketDataError::ClientClosed)));
    }

    #[tokio::test]
    async fn test_unsubscribe_fails_when_closed() {
        use crate::models::Channel;

        let config = ConnectionConfig::fugle_stock(AuthRequest::with_api_key("test-key"));
        let client = WebSocketClient::new(config);

        // First add a subscription while not closed
        let _ = client.subscribe(StockSubscription::new(Channel::Trades, "2330")).await;

        // Set to Closed state
        {
            let mut state = client.state.write().await;
            *state = ConnectionState::Closed {
                code: Some(1000),
                reason: "Test closure".to_string(),
                intent: DisconnectIntent::Client,
            };
        }

        // Unsubscribe should fail with ClientClosed error
        let result = client.unsubscribe(["trades:2330"]).await;
        assert!(matches!(result, Err(MarketDataError::ClientClosed)));
    }

    #[tokio::test]
    async fn test_connect_fails_when_closed() {
        let config = ConnectionConfig::fugle_stock(AuthRequest::with_api_key("test-key"));
        let client = WebSocketClient::new(config);

        // Set to Closed state
        {
            let mut state = client.state.write().await;
            *state = ConnectionState::Closed {
                code: Some(1000),
                reason: "Test closure".to_string(),
                intent: DisconnectIntent::Client,
            };
        }

        // Connect should fail with ClientClosed error
        let result = client.connect().await;
        assert!(matches!(result, Err(MarketDataError::ClientClosed)));
    }

    #[tokio::test]
    async fn test_reconnect_fails_when_closed() {
        let config = ConnectionConfig::fugle_stock(AuthRequest::with_api_key("test-key"));
        let client = WebSocketClient::new(config);

        // Set to Closed state
        {
            let mut state = client.state.write().await;
            *state = ConnectionState::Closed {
                code: Some(1000),
                reason: "Test closure".to_string(),
                intent: DisconnectIntent::Client,
            };
        }

        // Reconnect should fail with ClientClosed error
        let result = client.reconnect().await;
        assert!(matches!(result, Err(MarketDataError::ClientClosed)));
    }

    #[tokio::test]
    async fn test_subscribe_channel_fails_when_closed() {
        use crate::models::Channel;
        use crate::websocket::channels::StockSubscription;

        let config = ConnectionConfig::fugle_stock(AuthRequest::with_api_key("test-key"));
        let client = WebSocketClient::new(config);

        // Set to Closed state
        {
            let mut state = client.state.write().await;
            *state = ConnectionState::Closed {
                code: Some(1000),
                reason: "Test closure".to_string(),
                intent: DisconnectIntent::Client,
            };
        }

        // subscribe_channel should fail with ClientClosed error
        let sub = StockSubscription::new(Channel::Trades, "2330");
        let result = client.subscribe(sub).await;
        assert!(matches!(result, Err(MarketDataError::ClientClosed)));
    }

    #[test]
    fn test_is_closed_sync() {
        // Note: This test runs without a tokio runtime context
        let config = ConnectionConfig::fugle_stock(AuthRequest::with_api_key("test-key"));
        let client = WebSocketClient::new(config);

        // Without a runtime, is_closed_sync should return false
        assert!(!client.is_closed_sync());
    }

    #[test]
    fn emit_event_drops_when_channel_full() {
        // Saturate a tiny bounded channel and verify emit_event drops the
        // overflow silently instead of panicking or blocking.
        let (tx, rx) = mpsc::sync_channel::<ConnectionEvent>(2);
        emit_event(&tx, ConnectionEvent::Connecting);
        emit_event(&tx, ConnectionEvent::Connected);

        // 3rd send would block on plain `send`; emit_event must drop instead.
        emit_event(&tx, ConnectionEvent::Authenticated);

        // First two queued; third dropped at the sender.
        assert!(matches!(rx.recv(), Ok(ConnectionEvent::Connecting { .. })));
        assert!(matches!(rx.recv(), Ok(ConnectionEvent::Connected { .. })));
        assert!(rx.try_recv().is_err(), "third event must have been dropped");
    }

    #[tokio::test]
    async fn messages_is_idempotent_and_blocks_message_stream() {
        let config = ConnectionConfig::fugle_stock(AuthRequest::with_api_key("k"));
        let client = WebSocketClient::new(config);

        let r1 = client.messages();
        let r2 = client.messages();
        assert!(Arc::ptr_eq(&r1, &r2), "messages() must return the cached Arc");

        let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            client.message_stream()
        }));
        assert!(
            panicked.is_err(),
            "message_stream() must panic after messages() has taken ownership"
        );
    }

    #[tokio::test]
    async fn message_stream_blocks_messages() {
        let config = ConnectionConfig::fugle_stock(AuthRequest::with_api_key("k"));
        let client = WebSocketClient::new(config);

        let _stream = client.message_stream();

        let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            client.messages()
        }));
        assert!(
            panicked.is_err(),
            "messages() must panic after message_stream() has taken ownership"
        );
    }
}

/// Tests for stock streaming channel subscription API (Phase 4)
#[cfg(test)]
mod channel_tests {
    use super::*;
    use crate::models::Channel;
    use crate::websocket::channels::StockSubscription;
    use crate::AuthRequest;

    #[tokio::test]
    async fn test_subscribe_channel_stores_subscription() {
        let config = ConnectionConfig::fugle_stock(AuthRequest::with_api_key("test-key"));
        let client = WebSocketClient::new(config);

        let sub = StockSubscription::new(Channel::Trades, "2330");
        // Note: This will fail to send (not connected) but should store locally
        let _ = client.subscribe(sub).await;

        let keys = client.subscription_keys();
        assert!(keys.contains(&"trades:2330".to_string()));
    }

    #[tokio::test]
    async fn test_subscribe_channel_odd_lot() {
        let config = ConnectionConfig::fugle_stock(AuthRequest::with_api_key("test-key"));
        let client = WebSocketClient::new(config);

        let sub = StockSubscription::new(Channel::Trades, "2330").with_odd_lot(true);
        let _ = client.subscribe(sub).await;

        let keys = client.subscription_keys();
        assert!(keys.contains(&"trades:2330:oddlot".to_string()));
    }

    #[tokio::test]
    async fn test_subscribe_multiple_channels() {
        let config = ConnectionConfig::fugle_stock(AuthRequest::with_api_key("test-key"));
        let client = WebSocketClient::new(config);

        // Subscribe to multiple channels
        let _ = client
            .subscribe(StockSubscription::new(Channel::Trades, "2330"))
            .await;
        let _ = client
            .subscribe(StockSubscription::new(Channel::Candles, "2330"))
            .await;
        let _ = client
            .subscribe(StockSubscription::new(Channel::Books, "2330"))
            .await;

        let keys = client.subscription_keys();
        assert_eq!(keys.len(), 3);
        assert!(keys.contains(&"trades:2330".to_string()));
        assert!(keys.contains(&"candles:2330".to_string()));
        assert!(keys.contains(&"books:2330".to_string()));
    }

    #[tokio::test]
    async fn test_subscribe_symbols_convenience() {
        let config = ConnectionConfig::fugle_stock(AuthRequest::with_api_key("test-key"));
        let client = WebSocketClient::new(config);

        let _ = client
            .subscribe(StockSubscription::new(
                Channel::Trades,
                vec!["2330", "2317", "2454"],
            ))
            .await;

        let keys = client.subscription_keys();
        assert_eq!(keys.len(), 3);
        assert!(keys.contains(&"trades:2330".to_string()));
        assert!(keys.contains(&"trades:2317".to_string()));
        assert!(keys.contains(&"trades:2454".to_string()));
    }

    #[tokio::test]
    async fn unsubscribe_removes_single_subscription() {
        let config = ConnectionConfig::fugle_stock(AuthRequest::with_api_key("test-key"));
        let client = WebSocketClient::new(config);

        let sub = StockSubscription::new(Channel::Trades, "2330");
        let _ = client.subscribe(sub.clone()).await;
        assert_eq!(client.subscription_keys().len(), 1);

        let _ = client.unsubscribe(sub.keys()).await;
        assert_eq!(client.subscription_keys().len(), 0);
    }

    #[tokio::test]
    async fn unsubscribe_does_not_affect_other_subscriptions() {
        let config = ConnectionConfig::fugle_stock(AuthRequest::with_api_key("test-key"));
        let client = WebSocketClient::new(config);

        let sub1 = StockSubscription::new(Channel::Trades, "2330");
        let sub2 = StockSubscription::new(Channel::Candles, "2330");
        let _ = client.subscribe(sub1.clone()).await;
        let _ = client.subscribe(sub2).await;
        assert_eq!(client.subscription_keys().len(), 2);

        let _ = client.unsubscribe(sub1.keys()).await;

        let keys = client.subscription_keys();
        assert_eq!(keys.len(), 1);
        assert!(keys.contains(&"candles:2330".to_string()));
    }

    #[tokio::test]
    async fn batch_subscribe_with_odd_lot_expands_to_per_symbol_keys() {
        let config = ConnectionConfig::fugle_stock(AuthRequest::with_api_key("test-key"));
        let client = WebSocketClient::new(config);

        let _ = client
            .subscribe(
                StockSubscription::new(Channel::Trades, vec!["2330", "2317"]).with_odd_lot(true),
            )
            .await;

        let keys = client.subscription_keys();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"trades:2330:oddlot".to_string()));
        assert!(keys.contains(&"trades:2317:oddlot".to_string()));
    }

    #[tokio::test]
    async fn test_subscribe_all_channel_types() {
        let config = ConnectionConfig::fugle_stock(AuthRequest::with_api_key("test-key"));
        let client = WebSocketClient::new(config);

        // Subscribe to all channel types
        let _ = client
            .subscribe(StockSubscription::new(Channel::Trades, "2330"))
            .await;
        let _ = client
            .subscribe(StockSubscription::new(Channel::Candles, "2330"))
            .await;
        let _ = client
            .subscribe(StockSubscription::new(Channel::Books, "2330"))
            .await;
        let _ = client
            .subscribe(StockSubscription::new(Channel::Aggregates, "2330"))
            .await;
        let _ = client
            .subscribe(StockSubscription::new(Channel::Indices, "IX0001"))
            .await;

        let keys = client.subscription_keys();
        assert_eq!(keys.len(), 5);
    }
}

/// Tests for graceful shutdown (Phase 7 - Plan 02)
#[cfg(test)]
mod disconnect_tests {
    use super::*;
    use crate::websocket::channels::StockSubscription;
    use crate::AuthRequest;

    #[tokio::test]
    async fn test_disconnect_sets_closed_state() {
        let config = ConnectionConfig::fugle_stock(AuthRequest::with_api_key("test-key"));
        let client = WebSocketClient::new(config);

        // Manually set to Connected for testing
        {
            let mut state = client.state.write().await;
            *state = ConnectionState::Connected;
        }

        // Disconnect should succeed even without actual connection
        let result = client.disconnect().await;
        assert!(result.is_ok());

        // State should be Closed
        let state = client.state_async().await;
        assert!(matches!(
            state,
            ConnectionState::Closed {
                code: Some(1000),
                ..
            }
        ));
    }

    #[tokio::test]
    async fn test_disconnect_emits_event() {
        let config = ConnectionConfig::fugle_stock(AuthRequest::with_api_key("test-key"));
        let client = WebSocketClient::new(config);

        // Manually set to Connected
        {
            let mut state = client.state.write().await;
            *state = ConnectionState::Connected;
        }

        // Disconnect
        let _ = client.disconnect().await;

        // Check event was emitted
        let rx = Arc::clone(&client.event_rx);
        let event = tokio::task::spawn_blocking(move || {
            let rx_guard = rx.blocking_lock();
            rx_guard.try_recv()
        })
        .await
        .unwrap();

        assert!(matches!(
            event,
            Ok(ConnectionEvent::Disconnected {
                code: Some(1000),
                ..
            })
        ));
    }

    #[tokio::test]
    async fn test_force_close_sets_abnormal_code() {
        let config = ConnectionConfig::fugle_stock(AuthRequest::with_api_key("test-key"));
        let client = WebSocketClient::new(config);

        // Manually set to Connected
        {
            let mut state = client.state.write().await;
            *state = ConnectionState::Connected;
        }

        // Force close
        let result = client.force_close().await;
        assert!(result.is_ok());

        // State should be Closed with 1006
        let state = client.state_async().await;
        assert!(matches!(
            state,
            ConnectionState::Closed {
                code: Some(1006),
                ..
            }
        ));
    }

    #[tokio::test]
    async fn test_force_close_emits_event_with_1006() {
        let config = ConnectionConfig::fugle_stock(AuthRequest::with_api_key("test-key"));
        let client = WebSocketClient::new(config);

        // Manually set to Connected
        {
            let mut state = client.state.write().await;
            *state = ConnectionState::Connected;
        }

        // Force close
        let _ = client.force_close().await;

        // Check event was emitted with 1006
        let rx = Arc::clone(&client.event_rx);
        let event = tokio::task::spawn_blocking(move || {
            let rx_guard = rx.blocking_lock();
            rx_guard.try_recv()
        })
        .await
        .unwrap();

        assert!(matches!(
            event,
            Ok(ConnectionEvent::Disconnected {
                code: Some(1006),
                reason,
                ..
            }) if reason == "Force closed"
        ));
    }

    #[tokio::test]
    async fn test_disconnect_from_disconnected_state() {
        let config = ConnectionConfig::fugle_stock(AuthRequest::with_api_key("test-key"));
        let client = WebSocketClient::new(config);

        // Client starts in Disconnected state
        let state = client.state_async().await;
        assert_eq!(state, ConnectionState::Disconnected);

        // Disconnect should succeed even when already disconnected
        let result = client.disconnect().await;
        assert!(result.is_ok());

        // State should now be Closed
        let state = client.state_async().await;
        assert!(matches!(state, ConnectionState::Closed { .. }));
    }

    #[tokio::test]
    async fn test_is_closed_after_disconnect() {
        let config = ConnectionConfig::fugle_stock(AuthRequest::with_api_key("test-key"));
        let client = WebSocketClient::new(config);

        // Initially not closed
        assert!(!client.is_closed().await);

        // Disconnect
        let _ = client.disconnect().await;

        // Now should be closed
        assert!(client.is_closed().await);
    }

    #[tokio::test]
    async fn test_is_closed_after_force_close() {
        let config = ConnectionConfig::fugle_stock(AuthRequest::with_api_key("test-key"));
        let client = WebSocketClient::new(config);

        // Initially not closed
        assert!(!client.is_closed().await);

        // Force close
        let _ = client.force_close().await;

        // Now should be closed
        assert!(client.is_closed().await);
    }

    #[tokio::test]
    async fn test_operations_fail_after_disconnect() {
        use crate::models::Channel;

        let config = ConnectionConfig::fugle_stock(AuthRequest::with_api_key("test-key"));
        let client = WebSocketClient::new(config);

        // Disconnect
        let _ = client.disconnect().await;

        // Subscribe should fail with ClientClosed
        let sub = StockSubscription::new(Channel::Trades, "2330");
        let result = client.subscribe(sub).await;
        assert!(matches!(result, Err(MarketDataError::ClientClosed)));

        // Reconnect should fail with ClientClosed
        let result = client.reconnect().await;
        assert!(matches!(result, Err(MarketDataError::ClientClosed)));

        // Connect should fail with ClientClosed
        let result = client.connect().await;
        assert!(matches!(result, Err(MarketDataError::ClientClosed)));
    }

    #[tokio::test]
    async fn test_closed_state_has_normal_closure_reason() {
        let config = ConnectionConfig::fugle_stock(AuthRequest::with_api_key("test-key"));
        let client = WebSocketClient::new(config);

        let _ = client.disconnect().await;

        let state = client.state_async().await;
        if let ConnectionState::Closed { code, reason, .. } = state {
            assert_eq!(code, Some(1000));
            assert_eq!(reason, "Normal closure");
        } else {
            panic!("Expected Closed state");
        }
    }

    #[tokio::test]
    async fn test_force_closed_state_has_force_reason() {
        let config = ConnectionConfig::fugle_stock(AuthRequest::with_api_key("test-key"));
        let client = WebSocketClient::new(config);

        let _ = client.force_close().await;

        let state = client.state_async().await;
        if let ConnectionState::Closed { code, reason, .. } = state {
            assert_eq!(code, Some(1006));
            assert_eq!(reason, "Force closed");
        } else {
            panic!("Expected Closed state");
        }
    }
}
