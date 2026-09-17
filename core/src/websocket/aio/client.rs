//! Async WebSocket client (tokio-tungstenite).

use crate::models::{Channel, SubscribeRequest, WebSocketRequest};
use crate::websocket::aio::dispatch::dispatch_messages;
use crate::websocket::aio::reconnect::{replay_subscriptions, tls_connector_for, try_reconnect};
use crate::websocket::aio::writer::{spawn_writer, WriteFailure};
use crate::websocket::aio::{read_state, write_state, SharedState, WsSink, WsStream};
use crate::websocket::stream_queue::{QueueReceiver, StreamSender};
use crate::websocket::protocol::{
    frame_request, AuthHandshake, frame_subscribe, frame_subscribe_futopt,
    frame_unsubscribe,
};
use crate::websocket::{
    ConnectionConfig, ConnectionEvent, ConnectionState, ConnectionStateHandle, HealthCheckConfig,
    ConnectionStream, MessagesDroppedHandle, ReconnectionConfig, ReconnectionManager, StreamReceiver,
    SubscriptionManager,
};
use crate::MarketDataError;
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::mpsc as tokio_mpsc;
use tokio::sync::{oneshot, Mutex};
use tokio::task::JoinHandle;
use tokio::time::{timeout, Duration};
use tokio_tungstenite::connect_async_tls_with_config;

/// WebSocket client for real-time market data
pub struct WebSocketClient {
    config: ConnectionConfig,
    state: SharedState,
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
    /// The client's ordered stream of messages and events. The client keeps
    /// a sender so the stream stays open until the client is dropped; the
    /// dispatch, writer and reconnect tasks report through clones. The
    /// receiver is taken by either `stream()` or `stream_receiver()` — see
    /// method docs.
    stream: StreamSender,
    stream_rx: Arc<std::sync::Mutex<Option<QueueReceiver>>>,
    /// Cached `StreamReceiver`, created by the first `stream_receiver()` call.
    stream_receiver: Arc<std::sync::Mutex<Option<Arc<StreamReceiver>>>>,
    /// Counter incremented every time the stream's message allowance is
    /// full and a frame is dropped (drop-newest policy).
    /// Exposed via [`Self::messages_dropped_total`]. Mirrors to the
    /// `metrics` recorder under counter
    /// [`crate::metrics_compat::COUNTER_MESSAGES_DROPPED`] when the
    /// `metrics` feature is enabled.
    messages_dropped: crate::metrics_compat::DropCounter,
    /// Monotonic counter incremented every time a [`ConnectionEvent`] is
    /// dropped because the stream's event allowance was full. Drop-newest
    /// policy mirrors the message allowance. Exposed via
    /// [`Self::events_dropped_total`]. Mirrors to the `metrics` recorder
    /// under counter [`crate::metrics_compat::COUNTER_EVENTS_DROPPED`]
    /// when the `metrics` feature is enabled.
    events_dropped: crate::metrics_compat::DropCounter,
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
        // `metrics` feature integration: register counter descriptions and
        // build per-client counters labelled with endpoint + client_id.
        // No-op without `feature = "metrics"`.
        let (messages_dropped, events_dropped) =
            crate::metrics_compat::build_drop_counters(&config);
        let (stream, stream_rx) = crate::websocket::stream_queue::stream(
            &config,
            messages_dropped.clone(),
            events_dropped.clone(),
        );

        Self {
            config,
            state: Arc::new(std::sync::RwLock::new(ConnectionState::Disconnected)),
            ws_sink: Arc::new(Mutex::new(None)),
            write_tx: Arc::new(Mutex::new(None)),
            reconnection: Arc::new(Mutex::new(ReconnectionManager::new(reconnection_config))),
            subscriptions: Arc::new(SubscriptionManager::new()),
            health_check_config,
            stream,
            stream_rx: Arc::new(std::sync::Mutex::new(Some(stream_rx))),
            stream_receiver: Arc::new(std::sync::Mutex::new(None)),
            messages_dropped,
            events_dropped,
            shutdown_requested: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            dispatch_handle: Arc::new(Mutex::new(None)),
            writer_handle: Arc::new(Mutex::new(None)),
        }
    }

    /// Number of inbound messages dropped because the message queue was
    /// full, counted from the start of the current connection.
    ///
    /// Frames are dropped under the **drop-newest** backpressure policy:
    /// when `message_buffer` is full, new arrivals are discarded rather
    /// than blocking the network read loop. A non-zero value here usually
    /// indicates the downstream consumer (your `stream()` /
    /// `stream_receiver()` reader) is too slow or stalled.
    ///
    /// Thread-safe. Restarts from zero when `connect()` or a reconnect
    /// attempt opens a new connection; after `disconnect()` it still reads the last
    /// connection's count. With the `metrics` feature the exported counter
    /// is not reset and keeps counting across connections.
    pub fn messages_dropped_total(&self) -> u64 {
        self.messages_dropped.load()
    }

    /// A handle reading [`messages_dropped_total`](Self::messages_dropped_total)
    /// that stays readable after this client is dropped.
    pub fn messages_dropped_handle(&self) -> MessagesDroppedHandle {
        MessagesDroppedHandle::new(self.messages_dropped.clone())
    }

    /// Total number of lifecycle [`ConnectionEvent`]s dropped because the
    /// stream already held `event_buffer` unread events, since this client
    /// was constructed.
    ///
    /// Mirrors [`Self::messages_dropped_total`] for events. The event
    /// allowance (default 1024) is separate from the message allowance and
    /// uses the same drop-newest policy: when full, new events are
    /// discarded rather than blocking the producer.
    ///
    /// A non-zero value indicates the consumer of `stream()` /
    /// `stream_receiver()` is too slow or stuck. Event volume is typically
    /// much lower than message volume (one event per heartbeat plus
    /// reconnect/error bursts), so a saturated event channel is almost
    /// always a sign of a stalled consumer.
    ///
    /// Counter is monotonic and thread-safe (`AtomicU64`). Reset only by
    /// constructing a new client.
    #[must_use]
    pub fn events_dropped_total(&self) -> u64 {
        self.events_dropped.load()
    }

    /// Get current connection state (snapshot)
    ///
    /// Never blocks on the runtime: callable from any thread, including a
    /// tokio worker, and returns the real state outside a runtime too.
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
        read_state(&self.state).clone()
    }

    /// A handle reading [`state`](Self::state) that stays readable after
    /// this client is dropped.
    pub fn state_handle(&self) -> ConnectionStateHandle {
        ConnectionStateHandle::new(Arc::clone(&self.state))
    }

    /// Get current connection state. Same as [`state`](Self::state).
    pub async fn state_async(&self) -> ConnectionState {
        let state = read_state(&self.state);
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
        let state = read_state(&self.state);
        matches!(*state, ConnectionState::Closed { .. })
    }

    /// Sync version of [`is_closed`](Self::is_closed) for FFI.
    ///
    /// Callable from any thread, on or off a tokio runtime.
    pub fn is_closed_sync(&self) -> bool {
        matches!(*read_state(&self.state), ConnectionState::Closed { .. })
    }

    /// Blocking receiver of this client's stream, for FFI consumers and
    /// threads outside any runtime.
    ///
    /// Every inbound message and every [`ConnectionEvent`] arrives as a
    /// [`StreamItem`](crate::websocket::StreamItem), in the order the client
    /// produced them; see [`connection_event`](crate::websocket::connection_event)
    /// for the guarantees. The stream exists from construction, so items
    /// queued before this call (such as `Connecting`) are not lost.
    /// Subsequent calls return the same `Arc<StreamReceiver>`.
    ///
    /// Needs no tokio runtime: it may be called from any thread, before or
    /// after [`connect`]. The receiver reports a closed stream once the
    /// client has been dropped and every queued item has been read.
    ///
    /// Event types:
    /// - `Connecting` - Connection attempt started
    /// - `Connected` - WebSocket connection established, not yet authenticated
    /// - `Authenticated { data }` - Authentication successful
    /// - `Unauthenticated { message, data }` - Credentials rejected
    /// - `Disconnected { code, reason, intent, will_reconnect }` - Connection closed
    /// - `Reconnecting { attempt }` - Reconnection attempt started
    /// - `ReconnectFailed { attempts }` - Reconnection failed after max attempts
    /// - `HeartbeatTimeout { elapsed }` - Liveness window elapsed (precedes `Disconnected`)
    /// - `MessagesDropped { dropped, total }` - Inbound messages dropped (queue full)
    /// - `Error { message, code }` - Error occurred
    ///
    /// **Mutually exclusive with [`stream`]**: only one of the two methods
    /// may take the stream. Calling `stream_receiver()` after `stream()` (or
    /// vice versa) panics with a descriptive message.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use marketdata_core::aio::WebSocketClient;
    /// use marketdata_core::websocket::{ConnectionConfig, ConnectionEvent, StreamItem};
    /// use marketdata_core::AuthRequest;
    ///
    /// let client = WebSocketClient::new(
    ///     ConnectionConfig::fugle_stock(AuthRequest::with_api_key("key"))
    /// );
    ///
    /// let items = client.stream_receiver();
    /// std::thread::spawn(move || {
    ///     while let Ok(item) = items.receive() {
    ///         match item {
    ///             StreamItem::Message(msg) => println!("{}", msg.event),
    ///             StreamItem::Event(ConnectionEvent::Disconnected { code, reason, .. }) => {
    ///                 println!("Disconnected: {:?} - {}", code, reason);
    ///                 break;
    ///             }
    ///             _ => {}
    ///         }
    ///     }
    /// });
    /// ```
    ///
    /// [`stream`]: Self::stream
    /// [`connect`]: Self::connect
    pub fn stream_receiver(&self) -> Arc<StreamReceiver> {
        let mut slot = self.stream_receiver.lock().expect("stream_receiver poisoned");
        if let Some(rx) = slot.as_ref() {
            return Arc::clone(rx);
        }
        let rx = self
            .stream_rx
            .lock()
            .expect("stream_rx poisoned")
            .take()
            .expect("stream() already took this client's stream");
        let receiver = Arc::new(StreamReceiver::new(rx));
        *slot = Some(Arc::clone(&receiver));
        receiver
    }

    /// Async stream of this client's messages and events, in order.
    ///
    /// Returns a [`ConnectionStream`]: `.recv().await`, `try_recv()`, or use
    /// it as a `futures::Stream`. See [`stream_receiver`] for what it
    /// carries.
    ///
    /// **Mutually exclusive with [`stream_receiver`]**: takes ownership of
    /// the stream; can only be called once per client and panics if
    /// [`stream_receiver`] has already been called (or this method called
    /// twice).
    ///
    /// [`stream_receiver`]: Self::stream_receiver
    pub fn stream(&self) -> ConnectionStream {
        let rx = self
            .stream_rx
            .lock()
            .expect("stream_rx poisoned")
            .take()
            .expect(
                "stream already taken — `stream()` or `stream_receiver()` may only be called once \
                 between them",
            );
        ConnectionStream::new(rx)
    }

    /// Connect to WebSocket server and authenticate
    ///
    /// A no-op returning `Ok(())` while this client's dispatch task is still
    /// running (connected, or auto-reconnecting), matching the sync client.
    /// Use [`reconnect`](Self::reconnect) to replace a live connection.
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
        // A second dispatch task would orphan the first, whose later close
        // would then be reported through the shared latch as this new
        // connection's `Disconnected` (#41).
        if self.dispatch_task_running().await {
            return Ok(());
        }

        // Update state to Connecting
        {
            let mut state = write_state(&self.state);
            *state = ConnectionState::Connecting;
        }
        self.stream.emit(ConnectionEvent::Connecting {
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
                    let mut state = write_state(&self.state);
                    *state = ConnectionState::Disconnected;
                }
                self.stream.emit(ConnectionEvent::error(&err));
                return Err(err);
            }
            Err(_) => {
                let err = MarketDataError::TimeoutError {
                    operation: "WebSocket connect".to_string(),
                };
                {
                    let mut state = write_state(&self.state);
                    *state = ConnectionState::Disconnected;
                }
                self.stream.emit(ConnectionEvent::error(&err));
                return Err(err);
            }
        };

        // Split the stream into read/write halves
        let (mut ws_sink, mut ws_read) = ws_stream.split();

        crate::tracing_compat::info!(target: "fugle_marketdata::ws", "ws connected");
        self.stream.emit(ConnectionEvent::Connected {
        });

        // Update state to Authenticating
        {
            let mut state = write_state(&self.state);
            *state = ConnectionState::Authenticating;
        }

        // Send the auth frame and wait for the verdict. The frames read on the
        // way are queued after the verdict's event (shared helper with
        // try_connect; see aio/reconnect.rs).
        let handshake = crate::websocket::aio::reconnect::authenticate(
            &mut ws_sink,
            &mut ws_read,
            &self.config,
            &self.stream,
            Duration::from_secs(10),
        )
        .await;

        match handshake {
            AuthHandshake::Authenticated { data, frames } => {
                // Store the write half for sending messages
                {
                    let mut sink_guard = self.ws_sink.lock().await;
                    *sink_guard = Some(ws_sink);
                }

                // Spawn the writer task and install its sender. All
                // subsequent outbound messages flow through this channel.
                let write_failed = self.start_writer_task().await;

                {
                    let mut state = write_state(&self.state);
                    *state = ConnectionState::Connected;
                }
                crate::tracing_compat::info!(target: "fugle_marketdata::ws", "ws authenticated");
                // Before the dispatch task reads anything; a new connection
                // may report its own close.
                self.stream.authenticated(data, frames);

                // Spawn dispatch task to handle incoming messages (uses read half).
                // Liveness detection is wrapped inside dispatch_messages itself
                // (read-site `tokio::time::timeout`); no separate task needed.
                self.spawn_dispatch_task(ws_read, write_failed).await;

                Ok(())
            }
            AuthHandshake::Rejected { message, data, frames } => {
                {
                    let mut state = write_state(&self.state);
                    *state = ConnectionState::Disconnected;
                }
                // Server-rejected credentials are reported only as
                // Unauthenticated, never as a generic Error.
                self.stream.unauthenticated(message.clone(), data, frames);
                Err(MarketDataError::AuthError { msg: message, http: None })
            }
            AuthHandshake::Failed(err) => {
                {
                    let mut state = write_state(&self.state);
                    *state = ConnectionState::Disconnected;
                }
                self.stream.emit(ConnectionEvent::error(&err));
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
    /// The emitted [`ConnectionEvent::Disconnected`] carries
    /// [`DisconnectIntent::Client`](crate::websocket::DisconnectIntent::Client)
    /// regardless of whether the drain completed in time. It is emitted at
    /// most once per connection: if the connection was already reported lost
    /// (a server Close or transport error, including one racing this call) or
    /// this client was already disconnected, no further `Disconnected` is
    /// emitted, and a `Closed` state recorded with that report is kept.
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
    ///
    /// # Errors
    /// Returns [`MarketDataError`] on transport, protocol, deserialization,
    /// validation, or peer-initiated failures.
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

        // 5. Force-abort background tasks if drain budget elapsed. Messages
        //    already queued stay readable until the client is dropped.
        if drained.is_err() {
            self.abort_background_tasks().await;
        }

        // 6. Clear the sink slot regardless of close-frame outcome.
        {
            let mut sink_guard = self.ws_sink.lock().await;
            *sink_guard = None;
        }

        // 7. Mark the client closed (even if the close failed) and emit the
        //    Client-intent Disconnected, unless the dispatch task already
        //    reported this connection's close (#41); a `Closed` state that
        //    report recorded is kept (#93).
        self.stream
            .client_closed(&self.state, 1000, "Normal closure".to_string());

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

    /// True while the dispatch task (dispatch loop + auto-reconnect) runs.
    async fn dispatch_task_running(&self) -> bool {
        self.dispatch_handle
            .lock()
            .await
            .as_ref()
            .is_some_and(|h| !h.is_finished())
    }

    /// Abort the dispatch task and wait until it is gone. `emit_event` never
    /// awaits, so an abort cannot leave an emit half done.
    async fn stop_dispatch_task(&self) {
        if let Some(h) = self.dispatch_handle.lock().await.take() {
            h.abort();
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
    ///
    /// Like [`disconnect`](Self::disconnect), emits
    /// [`ConnectionEvent::Disconnected`] only if this connection has not
    /// already reported one, and keeps a `Closed` state recorded with that
    /// report.
    ///
    /// # Errors
    /// Returns [`MarketDataError`] on transport, protocol, deserialization,
    /// validation, or peer-initiated failures.
    pub async fn force_close(&self) -> Result<(), MarketDataError> {
        // Messages already queued stay readable until the client is dropped.

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

        // 1006: abnormal closure.
        self.stream
            .client_closed(&self.state, 1006, "Force closed".to_string());

        Ok(())
    }

    /// Check if currently connected
    pub async fn is_connected(&self) -> bool {
        let state = read_state(&self.state);
        matches!(*state, ConnectionState::Connected)
    }

    /// Subscribe to a stock streaming channel.
    ///
    /// Accepts a [`StockSubscription`](crate::StockSubscription) carrying single or batch symbols and
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
    ///
    /// # Errors
    /// Returns [`MarketDataError`] on transport, protocol, deserialization,
    /// validation, or peer-initiated failures.
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
    ///
    /// A live connection (or an auto-reconnect in progress) is torn down
    /// first without emitting `Disconnected`, matching the sync client.
    pub async fn reconnect(&self) -> Result<(), MarketDataError> {
        // Check if client is closed - cannot reconnect a closed client
        if self.is_closed().await {
            return Err(MarketDataError::ClientClosed);
        }

        // Stop the old dispatch task before `connect()` re-arms the latch,
        // so its socket's close cannot claim the new connection's
        // `Disconnected` (#41).
        self.stop_dispatch_task().await;

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
    /// A subscription that cannot be re-sent is reported as an `Error` event
    /// naming its key; the rest are still sent and the first failure is
    /// returned.
    async fn resubscribe_all(&self) -> Result<(), MarketDataError> {
        // Old server ids point at a dead connection — clear before replay so
        // the fresh subscribed acks can overwrite cleanly. Without this,
        // unsubscribe after reconnect could briefly pick up a zombie id.
        self.subscriptions.clear_server_ids();

        let sender = { self.write_tx.lock().await.clone() };
        let Some(sender) = sender else {
            return Err(MarketDataError::ConnectionError {
                msg: "Not connected".to_string(),
            });
        };
        replay_subscriptions(self.subscriptions.get_all(), &self.stream, &sender).await
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
    #[allow(dead_code, reason = "kept for future direct-frame test harness")]
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
    async fn spawn_dispatch_task(
        &self,
        ws_read: WsStream,
        write_failed: oneshot::Receiver<WriteFailure>,
    ) {
        let stream = self.stream.clone();

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
        let shutdown_requested = Arc::clone(&self.shutdown_requested);

        let handle = tokio::spawn(async move {
            // Dispatch → reconnect → dispatch loop (avoids recursive async which breaks Send)
            let mut current_ws_read = ws_read;
            let mut current_write_failed = write_failed;
            loop {
                let close_code = dispatch_messages(
                    current_ws_read,
                    stream.clone(),
                    heartbeat_timeout,
                    Arc::clone(&subscriptions),
                    Arc::clone(&shutdown_requested),
                    Arc::clone(&reconnection),
                    Arc::clone(&state),
                    current_write_failed,
                )
                .await;

                // Graceful-shutdown short-circuit: when `disconnect()` /
                // `shutdown_with_timeout()` set the flag, the dispatch
                // loop's exit must not loop back into reconnect — that
                // would re-establish the connection the caller just asked
                // to tear down. A server Close that arrived before the flag
                // was set has already been propagated as a `Disconnected`
                // event; one arriving after it is the ack of our own Close
                // and is swallowed by the dispatch loop.
                if shutdown_requested.load(std::sync::atomic::Ordering::SeqCst) {
                    break;
                }

                // Attempt auto-reconnect; returns new streams on success.
                match try_reconnect(
                    close_code,
                    Arc::clone(&reconnection),
                    config.clone(),
                    Arc::clone(&state),
                    stream.clone(),
                    Arc::clone(&ws_sink),
                    Arc::clone(&write_tx_slot),
                    Arc::clone(&writer_handle),
                    Arc::clone(&subscriptions),
                    Arc::clone(&shutdown_requested),
                )
                .await
                {
                    Some((ws_read, write_failed)) => {
                        current_ws_read = ws_read;
                        current_write_failed = write_failed;
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
    /// shared slot. Call after `ws_sink` has been populated. Returns the
    /// receiver of the writer's failed write, for the dispatch loop.
    async fn start_writer_task(&self) -> oneshot::Receiver<WriteFailure> {
        // Aborts any previous writer task.
        if let Some(prev) = self.writer_handle.lock().await.take() {
            prev.abort();
        }

        let (tx, handle, write_failed) =
            spawn_writer(Arc::clone(&self.ws_sink), self.stream.clone());
        {
            let mut guard = self.write_tx.lock().await;
            *guard = Some(tx);
        }

        let mut guard = self.writer_handle.lock().await;
        *guard = Some(handle);
        write_failed
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
}

// `run_writer_task` is now in `aio::writer` (PR2 split).
// `try_reconnect` + `try_connect` + `tls_connector_for` are now in `aio::reconnect`.


#[cfg(test)]
mod tests {
    use super::*;
    use crate::websocket::channels::StockSubscription;
    use crate::websocket::DisconnectIntent;
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
        let _authenticated = ConnectionEvent::Authenticated {
            data: serde_json::Value::Null,
        };
        let _unauthenticated = ConnectionEvent::Unauthenticated {
            message: "Invalid credentials".to_string(),
            data: serde_json::Value::Null,
        };
        let _disconnected = ConnectionEvent::Disconnected {
            code: Some(1000),
            reason: "Normal closure".to_string(),
            intent: DisconnectIntent::Client,
            will_reconnect: false,
        };
        let _reconnecting = ConnectionEvent::Reconnecting {
            attempt: 1,
        };
        let _failed = ConnectionEvent::ReconnectFailed {
            attempts: 5,
        };
        let _error = ConnectionEvent::Error(crate::errors::ErrorInfo::new(
            crate::errors::error_code::CONNECTION,
            crate::errors::ErrorKind::Network,
            "Connection failed",
        ));
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
            let mut state = write_state(&client.state);
            *state = ConnectionState::Connecting;
        }

        let state = client.state_async().await;
        assert_eq!(state, ConnectionState::Connecting);
    }

    #[test]
    fn state_handle_reads_the_state_after_the_client_is_dropped() {
        let config =
            ConnectionConfig::fugle_stock(AuthRequest::with_api_key("test-key"));
        let client = WebSocketClient::new(config);
        let handle = client.state_handle();
        assert_eq!(handle.state(), ConnectionState::Disconnected);

        *write_state(&client.state) = ConnectionState::Connected;
        assert!(handle.is_connected());
        assert!(!handle.is_closed());

        *write_state(&client.state) = ConnectionState::Closed {
            code: Some(1000),
            reason: "Normal closure".to_string(),
            intent: DisconnectIntent::Client,
        };
        drop(client);
        assert!(!handle.is_connected());
        assert!(handle.is_closed());
    }

    #[tokio::test]
    async fn test_websocket_client_events() {
        let config =
            ConnectionConfig::fugle_stock(AuthRequest::with_api_key("test-key"));
        let client = WebSocketClient::new(config);

        client.stream.emit(ConnectionEvent::Connecting);

        let mut stream = client.stream();
        assert!(matches!(
            stream.recv().await,
            Some(crate::websocket::StreamItem::Event(ConnectionEvent::Connecting))
        ));
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
            let mut state = write_state(&client.state);
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
            let mut state = write_state(&client.state);
            *state = ConnectionState::Connecting;
        }
        assert_eq!(client.state_async().await, ConnectionState::Connecting);

        {
            let mut state = write_state(&client.state);
            *state = ConnectionState::Authenticating;
        }
        assert_eq!(client.state_async().await, ConnectionState::Authenticating);

        {
            let mut state = write_state(&client.state);
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
            let mut state = write_state(&client.state);
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
        let reconnection_config = ReconnectionConfig::builder()
            .max_attempts(10)
            .initial_delay(Duration::from_secs(2))
            .build();

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
            let mut state = write_state(&client.state);
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
            let mut state = write_state(&client.state);
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
            let mut state = write_state(&client.state);
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
            let mut state = write_state(&client.state);
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
            let mut state = write_state(&client.state);
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
            let mut state = write_state(&client.state);
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

    fn closed_client() -> WebSocketClient {
        let config = ConnectionConfig::fugle_stock(AuthRequest::with_api_key("test-key"));
        let client = WebSocketClient::new(config);
        *write_state(&client.state) = ConnectionState::Closed {
            code: Some(1000),
            reason: "Normal closure".to_string(),
            intent: DisconnectIntent::Client,
        };
        client
    }

    fn assert_sync_getters_see_closed(client: &WebSocketClient) {
        assert!(matches!(client.state(), ConnectionState::Closed { .. }));
        assert!(client.is_closed_sync());
    }

    #[test]
    fn test_is_closed_sync() {
        let config = ConnectionConfig::fugle_stock(AuthRequest::with_api_key("test-key"));
        let client = WebSocketClient::new(config);
        assert!(!client.is_closed_sync());
    }

    /// #33: off any runtime, the sync getters used to report a fake
    /// `Disconnected` / not-closed.
    #[test]
    fn sync_getters_read_real_state_off_runtime() {
        assert_sync_getters_see_closed(&closed_client());
    }

    /// #33: on a runtime thread, the sync getters used to panic with
    /// `Cannot start a runtime from within a runtime`.
    #[tokio::test(flavor = "current_thread")]
    async fn sync_getters_do_not_panic_on_current_thread_runtime() {
        assert_sync_getters_see_closed(&closed_client());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn sync_getters_do_not_panic_on_multi_thread_runtime() {
        let client = closed_client();
        assert_sync_getters_see_closed(&client);
        // Also from a worker task, not just the test's block_on thread.
        let client = Arc::new(client);
        let worker = Arc::clone(&client);
        tokio::spawn(async move { assert_sync_getters_see_closed(&worker) })
            .await
            .expect("worker task");
    }

    #[test]
    fn full_event_allowance_drops_and_counts() {
        let config = ConnectionConfig::builder("wss://example.com", AuthRequest::with_api_key("k"))
            .event_buffer(2)
            .build();
        let client = WebSocketClient::new(config);
        client.stream.emit(ConnectionEvent::Connecting);
        client.stream.emit(ConnectionEvent::Connected);
        assert_eq!(client.events_dropped_total(), 0);
        client.stream.emit(ConnectionEvent::Authenticated {
            data: serde_json::Value::Null,
        });
        assert_eq!(client.events_dropped_total(), 1);

        let items = client.stream_receiver();
        assert!(items.try_receive().is_some());
        assert!(items.try_receive().is_some());
        assert!(items.try_receive().is_none(), "third event must have been dropped");
    }

    #[tokio::test]
    async fn stream_receiver_is_idempotent_and_blocks_stream() {
        let config = ConnectionConfig::fugle_stock(AuthRequest::with_api_key("k"));
        let client = WebSocketClient::new(config);

        let r1 = client.stream_receiver();
        let r2 = client.stream_receiver();
        assert!(Arc::ptr_eq(&r1, &r2), "stream_receiver() must return the cached Arc");

        let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            client.stream()
        }));
        assert!(
            panicked.is_err(),
            "stream() must panic after stream_receiver() has taken the stream"
        );
    }

    #[tokio::test]
    async fn stream_blocks_stream_receiver() {
        let config = ConnectionConfig::fugle_stock(AuthRequest::with_api_key("k"));
        let client = WebSocketClient::new(config);

        let _stream = client.stream();

        let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            client.stream_receiver()
        }));
        assert!(
            panicked.is_err(),
            "stream_receiver() must panic after stream() has taken the stream"
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
            let mut state = write_state(&client.state);
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
            let mut state = write_state(&client.state);
            *state = ConnectionState::Connected;
        }

        // Disconnect
        let _ = client.disconnect().await;

        // Check event was emitted
        let event = match client.stream_receiver().try_receive() {
            Some(crate::websocket::StreamItem::Event(event)) => Ok(event),
            other => Err(other),
        };

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
            let mut state = write_state(&client.state);
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
            let mut state = write_state(&client.state);
            *state = ConnectionState::Connected;
        }

        // Force close
        let _ = client.force_close().await;

        // Check event was emitted with 1006
        let event = match client.stream_receiver().try_receive() {
            Some(crate::websocket::StreamItem::Event(event)) => Ok(event),
            other => Err(other),
        };

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

/// A failed write ends the connection like a failed read (#97).
#[cfg(test)]
mod write_failure_tests {
    use super::*;
    use crate::websocket::channels::StockSubscription;
    use crate::websocket::{DisconnectIntent, StreamItem};
    use crate::AuthRequest;
    use futures_util::{SinkExt, StreamExt};
    use tokio::net::TcpListener;
    use tokio_tungstenite::tungstenite::Message;

    const WAIT: Duration = Duration::from_secs(5);

    /// Server that authenticates one client, then neither reads nor closes:
    /// the client's read half keeps waiting whatever happens to its writes.
    async fn silent_server() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let url = format!("ws://{}", listener.local_addr().expect("addr"));
        tokio::spawn(async move {
            let (tcp, _) = listener.accept().await.expect("accept");
            let mut ws = tokio_tungstenite::accept_async(tcp).await.expect("handshake");
            ws.next().await; // auth
            ws.send(Message::Text(r#"{"event":"authenticated"}"#.into()))
                .await
                .expect("authenticated");
            let _held_open = ws;
            std::future::pending::<()>().await;
        });
        url
    }

    async fn connected_client(reconnection: ReconnectionConfig) -> WebSocketClient {
        let url = silent_server().await;
        let config = ConnectionConfig::new(url, AuthRequest::with_api_key("test-key"));
        let client = WebSocketClient::with_reconnection_config(config, reconnection);
        client.connect().await.expect("connect");
        client
    }

    /// Reconnects, but not before the test has finished.
    fn slow_reconnect() -> ReconnectionConfig {
        ReconnectionConfig::new(5, Duration::from_secs(30), Duration::from_secs(30)).expect("valid")
    }

    /// Close the write half locally, so the next write fails while the read
    /// half has not noticed anything.
    async fn break_writes(client: &WebSocketClient) {
        let mut sink = client.ws_sink.lock().await;
        sink.as_mut().expect("sink").close().await.expect("close sink");
    }

    async fn write(client: &WebSocketClient) {
        client
            .subscribe(StockSubscription::new(Channel::Trades, "2330"))
            .await
            .expect("queued");
    }

    /// Next event, with the state read the moment it arrives.
    fn next_event(client: &WebSocketClient) -> Option<(ConnectionEvent, ConnectionState)> {
        let rx = client.stream_receiver();
        loop {
            match rx.receive_timeout(WAIT).expect("receive") {
                Some(StreamItem::Event(event)) => return Some((event, client.state())),
                Some(StreamItem::Message(_)) => continue,
                None => return None,
            }
        }
    }

    /// Nothing but messages is queued.
    fn assert_no_event(client: &WebSocketClient) {
        while let Some(item) = client.stream_receiver().try_receive() {
            assert!(matches!(item, StreamItem::Message(_)), "{item:?}");
        }
    }

    fn skip_handshake(client: &WebSocketClient) {
        for _ in 0..3 {
            let (event, _) = next_event(client).expect("handshake event");
            assert!(
                matches!(
                    event,
                    ConnectionEvent::Connecting
                        | ConnectionEvent::Connected
                        | ConnectionEvent::Authenticated { .. }
                ),
                "{event:?}"
            );
        }
    }

    fn expect_write_error(client: &WebSocketClient) {
        match next_event(client) {
            Some((ConnectionEvent::Error(info), _)) => {
                assert!(info.message.starts_with("WebSocket write error: "), "{info:?}");
            }
            other => panic!("expected Error, got {other:?}"),
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn failed_write_without_reconnect_closes_the_connection() {
        let client = connected_client(ReconnectionConfig::disabled()).await;
        skip_handshake(&client);

        break_writes(&client).await;
        write(&client).await;

        expect_write_error(&client);
        let (event, state) = next_event(&client).expect("Disconnected");
        let ConnectionEvent::Disconnected { code, reason, intent, will_reconnect } = event else {
            panic!("expected Disconnected, got {event:?}");
        };
        assert_eq!((code, intent, will_reconnect), (None, DisconnectIntent::Network, false));
        assert!(reason.starts_with("WebSocket write error: "), "{reason}");
        let closed = ConnectionState::Closed { code, reason, intent };
        assert_eq!(state, closed, "state matches the report when it arrives");

        // A later close keeps the reported state and reports nothing (#93).
        client.disconnect().await.expect("disconnect");
        client.force_close().await.expect("force_close");
        assert_eq!(client.state(), closed);
        assert_no_event(&client);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn failed_write_with_reconnect_leaves_connected_and_reconnects() {
        let client = connected_client(slow_reconnect()).await;
        skip_handshake(&client);

        break_writes(&client).await;
        write(&client).await;

        expect_write_error(&client);
        let (event, state) = next_event(&client).expect("Disconnected");
        assert!(
            matches!(
                event,
                ConnectionEvent::Disconnected {
                    code: None,
                    intent: DisconnectIntent::Network,
                    will_reconnect: true,
                    ..
                }
            ),
            "{event:?}"
        );
        assert_ne!(state, ConnectionState::Connected);
        let (event, _) = next_event(&client).expect("Reconnecting");
        assert!(matches!(event, ConnectionEvent::Reconnecting { attempt: 1 }), "{event:?}");

        client.force_close().await.expect("force_close");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn failed_write_during_shutdown_is_reported_as_the_client_close() {
        let client = connected_client(ReconnectionConfig::disabled()).await;
        skip_handshake(&client);

        client.shutdown_requested.store(true, std::sync::atomic::Ordering::SeqCst);
        break_writes(&client).await;
        write(&client).await;
        // The dispatch task takes the failure and exits without a report.
        tokio::time::timeout(WAIT, async {
            while client.dispatch_task_running().await {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("dispatch task exits");
        assert_no_event(&client);

        client.disconnect().await.expect("disconnect");
        let (event, _) = next_event(&client).expect("Disconnected");
        assert!(
            matches!(
                event,
                ConnectionEvent::Disconnected { intent: DisconnectIntent::Client, .. }
            ),
            "{event:?}"
        );
        assert_no_event(&client);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn failed_write_after_dispatch_stopped_is_still_reported_as_error() {
        let client = connected_client(ReconnectionConfig::disabled()).await;
        skip_handshake(&client);

        client.stop_dispatch_task().await;
        break_writes(&client).await;
        write(&client).await;

        expect_write_error(&client);
        assert_eq!(client.state(), ConnectionState::Connected, "nothing to report the close");
    }
}
