//! Async WebSocket client (tokio-tungstenite).

use crate::models::{Channel, SubscribeRequest, WebSocketRequest};
use crate::websocket::aio::dispatch::dispatch_messages;
use crate::websocket::aio::reconnect::{replay_subscriptions, tls_connector_for, try_reconnect};
use crate::websocket::aio::writer::{retire_writer, start_writer, WriteFailure, WriterGeneration};
use crate::websocket::aio::{read_state, write_state, SharedState, WsSink, WsStream};
use crate::websocket::connect_gate::ConnectGate;
use crate::websocket::liveness::{
    latency_connection_lost, latency_frame, latency_timeout, latency_timeout_or_default,
    LatencyWaiters,
};
use crate::websocket::stream_queue::{QueueReceiver, StreamSender};
use crate::websocket::protocol::{
    frame_request, AuthHandshake, frame_resubscribe, frame_subscribe, frame_subscribe_futopt,
    frame_unsubscribe, unsubscribe_wire_ids,
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
use tokio::sync::{oneshot, Mutex, Notify};
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
    /// Health check / liveness configuration. The dispatch loop bounds each
    /// `ws_read.next()` by the liveness deadline it derives from this and
    /// queues the probe itself; no background polling task is needed.
    health_check_config: HealthCheckConfig,
    /// Pending [`measure_latency`](Self::measure_latency) calls, answered by
    /// the dispatch loop.
    latency: Arc<LatencyWaiters>,
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
    /// Notified right after `shutdown_requested` is set, to wake an
    /// auto-reconnect waiting on its backoff or handshake (#110).
    shutdown_notify: Arc<Notify>,
    // Internal handles
    dispatch_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
    writer_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
    /// Generation of the current writer; see [`WriterGeneration`].
    writer_generation: WriterGeneration,
    /// Held for the duration of each `connect()`, so a concurrent one is
    /// refused rather than opening a second connection (#119).
    connect_gate: ConnectGate,
    /// Held by `connect()` from its first shutdown check until it has either
    /// installed the connection or given it up. `disconnect()` and
    /// `force_close()` wait for it after setting `shutdown_requested`, so an
    /// aborted `connect()` never writes state or events after their
    /// `Disconnected` (#121). Never held across network I/O once shutdown is
    /// requested: `connect()` releases it before closing a given-up socket,
    /// so the wait stays within a small `shutdown_with_timeout()` budget.
    connecting: Mutex<()>,
}

/// How long a `connect()` aborted by `disconnect()` waits for the Close frame
/// of the connection it gives up to be sent (#121).
const ABORTED_CLOSE_TIMEOUT: Duration = Duration::from_millis(500);

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
            latency: Arc::default(),
            stream,
            stream_rx: Arc::new(std::sync::Mutex::new(Some(stream_rx))),
            stream_receiver: Arc::new(std::sync::Mutex::new(None)),
            messages_dropped,
            events_dropped,
            shutdown_requested: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            shutdown_notify: Arc::new(Notify::new()),
            dispatch_handle: Arc::new(Mutex::new(None)),
            writer_handle: Arc::new(Mutex::new(None)),
            writer_generation: Arc::default(),
            connect_gate: ConnectGate::default(),
            connecting: Mutex::new(()),
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
    /// Refused while this client is connected, connecting or
    /// auto-reconnecting, matching the sync client. Use
    /// [`reconnect`](Self::reconnect) to replace a live connection.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Client has been closed (ClientClosed)
    /// - `disconnect()` or `force_close()` was called before the connection
    ///   was established (ConnectionAborted, code 2010); a socket already
    ///   opened is closed and the client stays `Closed`
    /// - Already connected, connecting or reconnecting (code 2011)
    /// - The credential is missing, blank or ambiguous (ConfigError)
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
        // Bindings reject bad credentials at construction; this catches a
        // config built directly in Rust or through the UniFFI constructors.
        self.config.auth.validate()?;
        // Held until this connection's dispatch task is running (#119).
        let Some(_claim) = self.connect_gate.try_claim() else {
            return Err(MarketDataError::AlreadyConnected);
        };
        // A second dispatch task would orphan the first, whose later close
        // would then be reported through the shared latch as this new
        // connection's `Disconnected` (#41).
        if self.dispatch_task_running().await {
            return Err(MarketDataError::AlreadyConnected);
        }
        // Before the state leaves its current value, so a bad TLS setting
        // does not leave the client `Connecting`.
        let tls_connector = tls_connector_for(&self.config)?;

        // A `disconnect()` from here on waits for this call to finish, and
        // wakes it: the connection is given up at the next step instead of
        // installed (#121).
        let connecting = self.connecting.lock().await;
        // Registered before the flag is first read: shutdown sets the flag
        // before notifying, so it is either seen or wakes this.
        let shutdown = self.shutdown_notify.notified();
        tokio::pin!(shutdown);
        shutdown.as_mut().enable();
        if self.stopping() {
            return Err(MarketDataError::ConnectionAborted);
        }

        // Update state to Connecting
        {
            let mut state = write_state(&self.state);
            *state = ConnectionState::Connecting;
        }
        self.stream.emit(ConnectionEvent::Connecting {
        });

        // Connect to WebSocket (with optional TLS customization).
        let connect_result = tokio::select! {
            result = timeout(
                self.config.connect_timeout,
                connect_async_tls_with_config(&self.config.url, None, false, Some(tls_connector)),
            ) => result,
            () = shutdown.as_mut() => return Err(MarketDataError::ConnectionAborted),
        };
        if self.stopping() {
            // Released first: closing is this call's own business, not
            // something `disconnect()` has to wait for.
            drop(connecting);
            if let Ok(Ok((mut ws_stream, _))) = connect_result {
                let _ = timeout(ABORTED_CLOSE_TIMEOUT, ws_stream.close(None)).await;
            }
            return Err(MarketDataError::ConnectionAborted);
        }

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
        let handshake = tokio::select! {
            handshake = crate::websocket::aio::reconnect::authenticate(
                &mut ws_sink,
                &mut ws_read,
                &self.config,
                &self.stream,
                Duration::from_secs(10),
            ) => Some(handshake),
            () = shutdown.as_mut() => None,
        };
        // Whatever the verdict, a connection `disconnect()` asked to stop is
        // closed rather than installed; the state stays as `disconnect()`
        // leaves it, and its `Disconnected` is the only close reported.
        let Some(handshake) = handshake.filter(|_| !self.stopping()) else {
            drop(connecting);
            let _ = timeout(ABORTED_CLOSE_TIMEOUT, ws_sink.close()).await;
            return Err(MarketDataError::ConnectionAborted);
        };

        match handshake {
            AuthHandshake::Authenticated { data, frames } => {
                // Install the write half and spawn its writer. All
                // subsequent outbound messages flow through its channel. A
                // `disconnect()` from here on waits for `connecting`, then
                // closes this connection as usual.
                let write_failed = self.start_writer_task(ws_sink).await;

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
    /// A `connect()` in progress is stopped: it closes the socket it opened,
    /// if any, and returns [`MarketDataError::ConnectionAborted`] (code
    /// 2010), and the client ends `Closed` as usual (#121).
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
        //    after the next dispatch return, and stop a reconnect (#110) or
        //    `connect()` (#121) in progress.
        self.request_shutdown().await;

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

    /// Set `shutdown_requested` and wake whatever waits on it, then wait for
    /// a `connect()` in progress to give its connection up, or to finish
    /// installing it, before the caller closes the client (#121). No network
    /// I/O happens under that wait: a given-up socket is closed after
    /// `connect()` releases the lock.
    async fn request_shutdown(&self) {
        self.shutdown_requested
            .store(true, std::sync::atomic::Ordering::SeqCst);
        self.shutdown_notify.notify_waiters();
        drop(self.connecting.lock().await);
    }

    /// True once `disconnect()` or `force_close()` has been called.
    fn stopping(&self) -> bool {
        self.shutdown_requested
            .load(std::sync::atomic::Ordering::SeqCst)
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
    /// A `connect()` in progress is stopped first and returns
    /// [`MarketDataError::ConnectionAborted`]; this waits for it to close the
    /// socket it opened, if any.
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

        // A closed client stays closed: a `connect()` in progress gives up
        // its connection instead of installing it (#121).
        self.request_shutdown().await;

        // Abort dispatch task without waiting (read-site liveness timeout
        // tears down with it; no separate health-check task to abort).
        {
            let mut handle = self.dispatch_handle.lock().await;
            if let Some(h) = handle.take() {
                h.abort();
            }
        }
        // The writer is aborted without waiting; a write it fails meanwhile
        // belongs to the connection being closed, not reported (#105).
        retire_writer(&self.writer_generation);

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
        // recording. On reconnect the rows fold back into one frame per
        // channel and modifier (see `frame_resubscribe`).
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

    /// Unsubscribe by server id(s) or local key(s) — accepts single or batch
    /// via `impl IntoIterator<Item = impl Into<String>>`.
    ///
    /// Each entry is either the server-assigned id returned in a `subscribed`
    /// ACK or a local key (`"{channel}:{symbol}[:modifier]"`); both remove
    /// the local subscription, so it is not restored on reconnect. A key
    /// whose ACK has not arrived yet is unsubscribed when the ACK brings its
    /// server id (#136). An entry that is neither is sent as is.
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

        let wire_ids = unsubscribe_wire_ids(&self.subscriptions, &keys);
        if wire_ids.is_empty() || !self.is_connected().await {
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
    /// Returns `ConnectionAborted` if `disconnect()` or `force_close()` is
    /// called before the new connection is established (#121).
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
        // The old writer may still fail a write before `connect()` replaces
        // it; that failure belongs to the connection being dropped (#105).
        retire_writer(&self.writer_generation);

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
    /// Subscriptions are re-sent as one frame per channel and modifier. A
    /// frame that cannot be re-sent is reported as an `Error` event naming
    /// its key or batch; the rest are still sent and the first failure is
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
        replay_subscriptions(frame_resubscribe(self.subscriptions.get_all()), &self.stream, &sender)
            .await
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

    /// Measure the round trip to the server: send a `ping` and wait for its
    /// `pong`, returning the time between the two on the local clock.
    ///
    /// Unlike [`send`](Self::send)ing a [`WebSocketRequest::ping`] — fire and
    /// forget, with the pong delivered on the stream — this waits for the
    /// answer, and its pong is not delivered on the stream. It works whether
    /// or not the health check's probe is enabled, and costs nothing in the
    /// background.
    ///
    /// `timeout` bounds the whole call, queueing the ping included; `None`
    /// means [`DEFAULT_LATENCY_TIMEOUT_MS`](crate::DEFAULT_LATENCY_TIMEOUT_MS)
    /// (5s).
    ///
    /// # Errors
    ///
    /// - `InvalidParameter` (1005) for a zero `timeout`.
    /// - `ClientClosed` when not connected.
    /// - `ConnectionError` when the connection closes before the pong.
    /// - `TimeoutError` (3001) when no pong arrives within `timeout`.
    pub async fn measure_latency(
        &self,
        timeout: Option<Duration>,
    ) -> Result<Duration, MarketDataError> {
        let timeout = latency_timeout_or_default(timeout)?;
        if !self.is_connected().await {
            return Err(MarketDataError::ClientClosed);
        }
        let (tx, rx) = oneshot::channel();
        let state = self.latency.register(move |arrived| {
            let _ = tx.send(arrived);
        });
        let sent = std::time::Instant::now();
        let exchange = async {
            self.enqueue_write(latency_frame(state.clone())?).await?;
            rx.await.map_err(|_| latency_connection_lost())
        };
        let result = match tokio::time::timeout(timeout, exchange).await {
            Ok(result) => result,
            Err(_elapsed) => Err(latency_timeout()),
        };
        if result.is_err() {
            self.latency.cancel(&state);
        }
        result.map(|arrived| arrived.saturating_duration_since(sent))
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
        let health = self.health_check_config.clone();
        let latency = Arc::clone(&self.latency);

        // Clone Arcs needed for auto-reconnect inside spawned task
        let reconnection = Arc::clone(&self.reconnection);
        let config = self.config.clone();
        let state = Arc::clone(&self.state);
        let ws_sink = Arc::clone(&self.ws_sink);
        let write_tx_slot = Arc::clone(&self.write_tx);
        let writer_handle = Arc::clone(&self.writer_handle);
        let writer_generation = Arc::clone(&self.writer_generation);
        let subscriptions = Arc::clone(&self.subscriptions);
        let shutdown_requested = Arc::clone(&self.shutdown_requested);
        let shutdown_notify = Arc::clone(&self.shutdown_notify);

        let handle = tokio::spawn(async move {
            // Dispatch → reconnect → dispatch loop (avoids recursive async which breaks Send)
            let mut current_ws_read = ws_read;
            let mut current_write_failed = write_failed;
            loop {
                let close_code = dispatch_messages(
                    current_ws_read,
                    stream.clone(),
                    &health,
                    &latency,
                    Arc::clone(&subscriptions),
                    Arc::clone(&write_tx_slot),
                    Arc::clone(&shutdown_requested),
                    Arc::clone(&reconnection),
                    Arc::clone(&state),
                    &mut current_write_failed,
                )
                .await;
                // The connection is gone. Retire its writer while the
                // receiver is still alive, so a write failing from here on
                // is never reported as `Error` (#105).
                retire_writer(&writer_generation);

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
                    Arc::clone(&writer_generation),
                    Arc::clone(&subscriptions),
                    Arc::clone(&shutdown_requested),
                    Arc::clone(&shutdown_notify),
                )
                .await
                {
                    Some((ws_read, write_failed)) => {
                        // `disconnect()` may have set the flag after the new
                        // writer was installed; it closes that connection, so
                        // do not wait on it for the peer's Close ack.
                        if shutdown_requested.load(std::sync::atomic::Ordering::SeqCst) {
                            break;
                        }
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

    /// Internal: Install `sink` and spawn the writer task that drains the
    /// outbound channel into it, replacing any previous writer (see
    /// [`start_writer`]). Returns the receiver of the writer's failed write,
    /// for the dispatch loop.
    async fn start_writer_task(&self, sink: WsSink) -> oneshot::Receiver<WriteFailure> {
        let (_, write_failed) = start_writer(
            sink,
            &self.ws_sink,
            &self.write_tx,
            &self.writer_handle,
            &self.writer_generation,
            self.stream.clone(),
            None,
        )
        .await
        .expect("installed unconditionally");
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
    async fn test_connect_rejects_blank_credential_before_connecting() {
        for auth in [AuthRequest::with_api_key(""), AuthRequest::with_token("  ")] {
            let config = ConnectionConfig::new("ws://127.0.0.1:1", auth);
            let client = WebSocketClient::new(config);

            let err = client.connect().await.expect_err("blank credential");
            assert!(matches!(err, MarketDataError::ConfigError(_)), "{err:?}");
            assert_eq!(client.state_async().await, ConnectionState::Disconnected);
        }
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

    #[test]
    fn state_handle_is_active_while_connecting_connected_or_reconnecting() {
        let config =
            ConnectionConfig::fugle_stock(AuthRequest::with_api_key("test-key"));
        let client = WebSocketClient::new(config);
        let handle = client.state_handle();
        let cases = [
            (ConnectionState::Disconnected, false),
            (ConnectionState::Connecting, true),
            (ConnectionState::Authenticating, true),
            (ConnectionState::Connected, true),
            (ConnectionState::Reconnecting { attempt: 1 }, true),
            (
                ConnectionState::Closed {
                    code: None,
                    reason: String::new(),
                    intent: DisconnectIntent::Server,
                },
                false,
            ),
        ];
        for (state, active) in cases {
            *write_state(&client.state) = state.clone();
            assert_eq!(handle.is_active(), active, "{state:?}");
        }
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
            assert_eq!(reconnection.attempts_remaining(), Some(10));
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

    /// Server that authenticates one client, then sends `Close(4001, "bye")`
    /// when `close` fires. A plain thread, so it runs while the test's
    /// runtime thread is blocked.
    fn closing_server() -> (String, std::sync::mpsc::Sender<()>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let url = format!("ws://{}", listener.local_addr().expect("addr"));
        let (close, close_rx) = std::sync::mpsc::channel::<()>();
        std::thread::spawn(move || {
            let (tcp, _) = listener.accept().expect("accept");
            let mut ws = tungstenite::accept(tcp).expect("handshake");
            ws.read().expect("auth");
            ws.send(tungstenite::Message::Text(r#"{"event":"authenticated"}"#.into()))
                .expect("authenticated");
            let _ = close_rx.recv();
            let frame = tungstenite::protocol::CloseFrame {
                code: 4001.into(),
                reason: "bye".into(),
            };
            let _ = ws.close(Some(frame));
            let _ = ws.flush();
            // Hold the socket open until the test ends.
            let _ = close_rx.recv();
        });
        (url, close)
    }

    /// The server's Close already sits in the socket when the write fails:
    /// the close is reported with its code and reason, and the reconnect
    /// decision follows that code, not the write failure's.
    #[tokio::test(flavor = "current_thread")]
    async fn failed_write_does_not_hide_a_close_already_received() {
        let (url, close) = closing_server();
        let config = ConnectionConfig::new(url, AuthRequest::with_api_key("test-key"));
        let client = WebSocketClient::with_reconnection_config(config, slow_reconnect());
        client.connect().await.expect("connect");
        skip_handshake(&client);

        break_writes(&client).await;
        close.send(()).expect("server closes");
        // Block the only runtime thread until the Close has arrived, so the
        // dispatch task has not seen it when the writer fails.
        std::thread::sleep(Duration::from_millis(200));
        write(&client).await;

        let rx = client.stream_receiver();
        let deadline = tokio::time::Instant::now() + WAIT;
        let (event, state) = loop {
            match rx.try_receive() {
                Some(StreamItem::Event(event @ ConnectionEvent::Disconnected { .. })) => {
                    break (event, client.state());
                }
                Some(_) => continue,
                None => {
                    assert!(tokio::time::Instant::now() < deadline, "no Disconnected");
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            }
        };
        assert_eq!(
            event,
            ConnectionEvent::Disconnected {
                code: Some(4001),
                reason: "bye".to_string(),
                intent: DisconnectIntent::Server,
                will_reconnect: false,
            }
        );
        let closed = ConnectionState::Closed {
            code: Some(4001),
            reason: "bye".to_string(),
            intent: DisconnectIntent::Server,
        };
        assert_eq!(state, closed);

        // No reconnect follows.
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert_no_event(&client);
        assert_eq!(client.state(), closed);
        drop(close);
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

    /// Text frames received after authentication, per connection in accept order.
    type Received = Arc<std::sync::Mutex<Vec<Vec<String>>>>;

    /// Server that authenticates every connection and records the text
    /// frames each one sends afterwards. Keeps reading, so writes succeed.
    async fn recording_server() -> (String, Received) {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let url = format!("ws://{}", listener.local_addr().expect("addr"));
        let received: Received = Arc::default();
        let log = Arc::clone(&received);
        tokio::spawn(async move {
            loop {
                let (tcp, _) = listener.accept().await.expect("accept");
                let index = {
                    let mut log = log.lock().unwrap();
                    log.push(Vec::new());
                    log.len() - 1
                };
                let log = Arc::clone(&log);
                tokio::spawn(async move {
                    let mut ws = tokio_tungstenite::accept_async(tcp).await.expect("handshake");
                    ws.next().await; // auth
                    ws.send(Message::Text(r#"{"event":"authenticated"}"#.into()))
                        .await
                        .expect("authenticated");
                    while let Some(Ok(msg)) = ws.next().await {
                        if let Message::Text(text) = msg {
                            log.lock().unwrap()[index].push(text.to_string());
                        }
                    }
                });
            }
        });
        (url, received)
    }

    /// Frames still queued for a replaced connection never reach the new
    /// one (#105).
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn reconnect_does_not_write_old_frames_to_the_new_connection() {
        const ROUNDS: usize = 30;
        let (url, received) = recording_server().await;
        let config = ConnectionConfig::new(url, AuthRequest::with_api_key("test-key"));
        let client =
            WebSocketClient::with_reconnection_config(config, ReconnectionConfig::disabled());
        client.connect().await.expect("connect");

        for round in 0..ROUNDS {
            // Keep this connection's writer busy until it is stopped.
            let old_tx = client.write_tx.lock().await.clone().expect("writer");
            let pump = tokio::spawn(async move {
                while old_tx.send(format!("old-{round}")).await.is_ok() {}
            });
            tokio::time::sleep(Duration::from_millis(5)).await;
            client.reconnect().await.expect("reconnect");
            tokio::time::timeout(WAIT, pump).await.expect("old writer stops").expect("pump");
        }
        client.send_text("fresh").await.expect("queued");

        let deadline = tokio::time::Instant::now() + WAIT;
        while !received.lock().unwrap().get(ROUNDS).is_some_and(|f| f.iter().any(|f| f == "fresh")) {
            assert!(tokio::time::Instant::now() < deadline, "fresh frame not received");
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        let received = received.lock().unwrap().clone();
        for (conn, frames) in received.iter().enumerate() {
            let own = format!("old-{conn}");
            let foreign: Vec<_> =
                frames.iter().filter(|f| **f != own && *f != "fresh").collect();
            assert!(foreign.is_empty(), "connection {conn} received {foreign:?}");
        }
        client.force_close().await.expect("force_close");
    }

    /// Server that authenticates one client, then drops the connection when
    /// `drop_it` fires, sending `last` first if given. Later connections are
    /// accepted and never answered.
    async fn dropping_server(last: Option<&'static str>) -> (String, tokio::sync::oneshot::Sender<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let url = format!("ws://{}", listener.local_addr().expect("addr"));
        let (drop_it, dropped) = tokio::sync::oneshot::channel::<()>();
        tokio::spawn(async move {
            let (tcp, _) = listener.accept().await.expect("accept");
            let mut ws = tokio_tungstenite::accept_async(tcp).await.expect("handshake");
            ws.next().await; // auth
            ws.send(Message::Text(r#"{"event":"authenticated"}"#.into()))
                .await
                .expect("authenticated");
            let _ = dropped.await;
            if let Some(last) = last {
                ws.send(Message::Text(last.into())).await.expect("last frame");
            }
            drop(ws);
            std::future::pending::<()>().await;
        });
        (url, drop_it)
    }

    /// Drop the connection on the server, then fail a write on it: once the
    /// close is reported, the failure is not reported as `Error` (#105).
    async fn assert_lost_connection_write_is_silent(reconnection: ReconnectionConfig) {
        let will = reconnection.enabled;
        let (url, drop_it) = dropping_server(None).await;
        let config = ConnectionConfig::new(url, AuthRequest::with_api_key("test-key"));
        let client = WebSocketClient::with_reconnection_config(config, reconnection);
        client.connect().await.expect("connect");
        skip_handshake(&client);

        drop_it.send(()).expect("server drops the connection");
        loop {
            match next_event(&client) {
                Some((ConnectionEvent::Error(_), _)) => {}
                Some((ConnectionEvent::Disconnected { intent, will_reconnect, .. }, _)) => {
                    assert_eq!((intent, will_reconnect), (DisconnectIntent::Network, will));
                    break;
                }
                other => panic!("expected Disconnected, got {other:?}"),
            }
        }
        if will {
            let (event, _) = next_event(&client).expect("Reconnecting");
            assert!(matches!(event, ConnectionEvent::Reconnecting { attempt: 1 }), "{event:?}");
        } else {
            // The dispatch task is gone, and with it the writer's receiver.
            tokio::time::timeout(WAIT, async {
                while client.dispatch_task_running().await {
                    tokio::task::yield_now().await;
                }
            })
            .await
            .expect("dispatch task exits");
        }

        // The lost connection's writer is still running; its write fails.
        break_writes(&client).await;
        client.send_text("late").await.expect("queued");
        tokio::time::sleep(Duration::from_millis(300)).await;

        assert_no_event(&client);
        client.force_close().await.expect("force_close");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn failed_write_while_reconnecting_is_not_reported() {
        assert_lost_connection_write_is_silent(slow_reconnect()).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn failed_write_after_the_connection_closed_is_not_reported() {
        assert_lost_connection_write_is_silent(ReconnectionConfig::disabled()).await;
    }

    /// A probe that cannot be queued — the write path is stuck — still
    /// gets its verdict on time: the connection is declared dead at
    /// `idle_probe_after + probe_timeout` (#150).
    #[tokio::test(flavor = "multi_thread")]
    async fn stuck_write_path_does_not_hold_the_probe_verdict_back() {
        const IDLE: Duration = Duration::from_millis(200);
        const PROBE_TIMEOUT: Duration = Duration::from_millis(200);
        let (url, received) = recording_server().await;
        let config = ConnectionConfig::new(url, AuthRequest::with_api_key("test-key"));
        let health = HealthCheckConfig {
            probe_enabled: true,
            idle_probe_after: Some(IDLE),
            probe_timeout: Some(PROBE_TIMEOUT),
            ..HealthCheckConfig::default()
        };
        let client =
            WebSocketClient::with_full_config(config, ReconnectionConfig::disabled(), health);
        client.connect().await.expect("connect");

        // A full queue nobody drains stands in for a writer stuck on the sink.
        let (stuck_tx, _stuck_rx) = tokio_mpsc::channel(1);
        stuck_tx.try_send("filler".to_string()).expect("fill");
        *client.write_tx.lock().await = Some(stuck_tx);
        let started = std::time::Instant::now();

        let receiver = client.stream_receiver();
        let timeout = tokio::task::spawn_blocking(move || loop {
            match receiver.receive_timeout(Duration::from_secs(5)) {
                Ok(Some(StreamItem::Event(ConnectionEvent::HeartbeatTimeout { elapsed }))) => {
                    return Some(elapsed);
                }
                Ok(Some(_)) => continue,
                _ => return None,
            }
        })
        .await
        .expect("reader");

        assert_eq!(timeout, Some(IDLE + PROBE_TIMEOUT));
        assert!(started.elapsed() < Duration::from_secs(2), "took {:?}", started.elapsed());
        assert!(received.lock().unwrap()[0].is_empty(), "the probe got out");
    }

    /// `force_close()` sets `shutdown_requested` and then reports the
    /// client's close; the dispatch task can read the flag before and
    /// report after. Its report must then be held back (#159). Reporting
    /// the close without the flag replays that interleaving: what the task
    /// sees is exactly the flag unset and the close reported.
    async fn assert_nothing_follows_the_client_s_close(
        client: WebSocketClient,
        fail: impl FnOnce(),
    ) {
        skip_handshake(&client);
        client
            .stream
            .client_closed(&client.state, 1006, "Force closed".into());
        let closed = client.state();
        let (event, _) = next_event(&client).expect("the client's close");
        assert!(
            matches!(
                event,
                ConnectionEvent::Disconnected { intent: DisconnectIntent::Client, will_reconnect: false, .. }
            ),
            "{event:?}"
        );

        fail();
        tokio::time::timeout(WAIT, async {
            while client.dispatch_task_running().await {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("dispatch task exits");

        assert_no_event(&client);
        assert_eq!(client.state(), closed);
        client.force_close().await.expect("force_close");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn heartbeat_timeout_after_the_client_s_close_is_reported_is_not_reported() {
        let url = silent_server().await;
        let config = ConnectionConfig::new(url, AuthRequest::with_api_key("test-key"));
        let health = HealthCheckConfig {
            heartbeat_timeout: Duration::from_millis(300),
            ..HealthCheckConfig::default()
        };
        let client =
            WebSocketClient::with_full_config(config, ReconnectionConfig::disabled(), health);
        client.connect().await.expect("connect");
        assert_nothing_follows_the_client_s_close(client, || {}).await;
    }

    /// Report the client's close, then have the server send `last`, if
    /// given, and drop the connection.
    async fn assert_nothing_follows_the_client_s_close_dropped(last: Option<&'static str>) {
        let (url, drop_it) = dropping_server(last).await;
        let config = ConnectionConfig::new(url, AuthRequest::with_api_key("test-key"));
        let client = WebSocketClient::with_full_config(
            config,
            ReconnectionConfig::disabled(),
            HealthCheckConfig::disabled(),
        );
        client.connect().await.expect("connect");
        assert_nothing_follows_the_client_s_close(client, || {
            drop_it.send(()).expect("server drops the connection");
        })
        .await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn transport_error_after_the_client_s_close_is_reported_is_not_reported() {
        assert_nothing_follows_the_client_s_close_dropped(None).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn deserialize_error_after_the_client_s_close_is_reported_is_not_reported() {
        assert_nothing_follows_the_client_s_close_dropped(Some("not json")).await;
    }
}

/// `disconnect()` during an auto-reconnect stops it promptly (#110).
#[cfg(test)]
mod disconnect_during_reconnect_tests {
    use super::*;
    use crate::websocket::StreamItem;
    use crate::AuthRequest;
    use futures_util::{SinkExt, StreamExt};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::net::TcpListener;
    use tokio::time::Instant;
    use tokio_tungstenite::tungstenite::Message;

    const WAIT: Duration = Duration::from_secs(5);
    /// Well under the drain budget `disconnect()` would otherwise wait out.
    const PROMPT: Duration = Duration::from_secs(1);

    /// What the server does with connections after the first.
    #[derive(Clone, Copy)]
    enum Later {
        /// Accept, never answer the auth frame.
        NeverAuthenticate,
        /// Authenticate, then keep reading, so a Close is answered.
        AuthenticateAndServe,
    }

    /// Server that authenticates the first connection and drops it when
    /// `drop_it` fires; later connections are handled as `later` says.
    /// Returns the URL and the number of connections accepted.
    async fn server(later: Later) -> (String, Arc<AtomicUsize>, oneshot::Sender<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let url = format!("ws://{}", listener.local_addr().expect("addr"));
        let accepted = Arc::new(AtomicUsize::new(0));
        let count = Arc::clone(&accepted);
        let (drop_it, dropped) = oneshot::channel::<()>();
        tokio::spawn(async move {
            let mut dropped = Some(dropped);
            loop {
                let Ok((tcp, _)) = listener.accept().await else { return };
                let first = count.fetch_add(1, Ordering::SeqCst) == 0;
                let dropped = dropped.take();
                tokio::spawn(async move {
                    let Ok(mut ws) = tokio_tungstenite::accept_async(tcp).await else { return };
                    ws.next().await; // auth
                    if !first && matches!(later, Later::NeverAuthenticate) {
                        std::future::pending::<()>().await;
                    }
                    let authenticated = r#"{"event":"authenticated"}"#;
                    if ws.send(Message::Text(authenticated.into())).await.is_err() {
                        return;
                    }
                    match dropped {
                        Some(dropped) => {
                            let _ = dropped.await;
                        }
                        None => while let Some(Ok(_)) = ws.next().await {},
                    }
                });
            }
        });
        (url, accepted, drop_it)
    }

    async fn client(url: String, delay: Duration) -> WebSocketClient {
        let config = ConnectionConfig::new(url, AuthRequest::with_api_key("test-key"));
        let reconnection = ReconnectionConfig::new(u32::MAX, delay, delay).expect("valid");
        let client = WebSocketClient::with_reconnection_config(config, reconnection);
        client.connect().await.expect("connect");
        client
    }

    /// Wait for an event matching `wanted`, skipping everything else.
    fn wait_for(client: &WebSocketClient, wanted: impl Fn(&ConnectionEvent) -> bool) {
        let rx = client.stream_receiver();
        let deadline = std::time::Instant::now() + WAIT;
        loop {
            let left = deadline.saturating_duration_since(std::time::Instant::now());
            match rx.receive_timeout(left).expect("receive") {
                Some(StreamItem::Event(event)) if wanted(&event) => return,
                Some(_) => continue,
                None => panic!("event not received"),
            }
        }
    }

    /// `disconnect()` returns promptly, the client is closed with no writer
    /// left, and no connection is opened afterwards.
    async fn assert_disconnect_stops_reconnecting(client: &WebSocketClient, accepted: &AtomicUsize) {
        let started = Instant::now();
        client.disconnect().await.expect("disconnect");
        let elapsed = started.elapsed();
        assert!(elapsed < PROMPT, "disconnect took {elapsed:?}");

        assert!(client.is_closed().await);
        assert!(client.write_tx.lock().await.is_none(), "writer sender left");
        assert!(client.writer_handle.lock().await.is_none(), "writer task left");
        let connections = accepted.load(Ordering::SeqCst);
        tokio::time::sleep(Duration::from_millis(300)).await;
        assert_eq!(accepted.load(Ordering::SeqCst), connections, "connected after disconnect");
        assert!(client.is_closed().await, "state rewritten after disconnect");
    }

    /// A reconnect that authenticates once shutdown was requested leaves the
    /// current writer in place.
    #[tokio::test(flavor = "multi_thread")]
    async fn writer_is_not_installed_once_shutdown_was_requested() {
        let (url, _accepted, _drop_it) = server(Later::NeverAuthenticate).await;
        let client = client(url.clone(), Duration::from_secs(30)).await;
        let (ws, _) = tokio_tungstenite::connect_async(&url).await.expect("connect");
        let (sink, _read) = ws.split();
        let current = client.write_tx.lock().await.clone().expect("writer");
        let generation = client.writer_generation.load(Ordering::SeqCst);

        client.shutdown_requested.store(true, Ordering::SeqCst);
        let started = start_writer(
            sink,
            &client.ws_sink,
            &client.write_tx,
            &client.writer_handle,
            &client.writer_generation,
            client.stream.clone(),
            Some(&client.shutdown_requested),
        )
        .await;

        assert!(started.is_none());
        let slot = client.write_tx.lock().await.clone().expect("writer kept");
        assert!(slot.same_channel(&current), "writer replaced");
        assert_eq!(client.writer_generation.load(Ordering::SeqCst), generation);
        assert!(!current.is_closed(), "current writer stopped");
        client.force_close().await.expect("force_close");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn disconnect_during_reconnect_backoff_returns_promptly() {
        let (url, accepted, drop_it) = server(Later::NeverAuthenticate).await;
        let client = client(url, Duration::from_secs(30)).await;

        drop_it.send(()).expect("server drops the connection");
        wait_for(&client, |e| matches!(e, ConnectionEvent::Reconnecting { .. }));

        assert_disconnect_stops_reconnecting(&client, &accepted).await;
        assert_eq!(accepted.load(Ordering::SeqCst), 1);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn disconnect_while_reconnect_authenticates_returns_promptly() {
        let (url, accepted, drop_it) = server(Later::NeverAuthenticate).await;
        let client = client(url, Duration::from_millis(100)).await;
        // The first connection's `Connected`.
        wait_for(&client, |e| matches!(e, ConnectionEvent::Connected));

        drop_it.send(()).expect("server drops the connection");
        wait_for(&client, |e| matches!(e, ConnectionEvent::Connected));

        assert_disconnect_stops_reconnecting(&client, &accepted).await;
        assert_eq!(accepted.load(Ordering::SeqCst), 2);
    }

    /// `disconnect()` spread over a reconnect that succeeds: during the
    /// backoff, while connecting, or right after it has authenticated.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn disconnect_racing_reconnects_returns_promptly() {
        const ROUNDS: u64 = 10;
        for round in 0..ROUNDS {
            let (url, accepted, drop_it) = server(Later::AuthenticateAndServe).await;
            let client = client(url, Duration::from_millis(100)).await;
            drop_it.send(()).expect("server drops the connection");
            wait_for(&client, |e| matches!(e, ConnectionEvent::Reconnecting { .. }));
            // Spread the call over a reconnect cycle (~100ms backoff).
            tokio::time::sleep(Duration::from_millis(80 + round * 6)).await;

            assert_disconnect_stops_reconnecting(&client, &accepted).await;
        }
    }
}

/// `disconnect()` / `force_close()` while `connect()` is still establishing
/// the connection (#121).
#[cfg(test)]
mod connect_abort_tests {
    use super::*;
    use crate::websocket::{DisconnectIntent, StreamItem};
    use crate::AuthRequest;
    use futures_util::StreamExt;
    use std::sync::atomic::Ordering;
    use tokio::net::TcpListener;
    use tokio::sync::oneshot;
    use tokio_tungstenite::tungstenite::Message;

    /// Far below the 10s auth timeout, so a pass means `connect()` was woken.
    const PROMPT: Duration = Duration::from_secs(3);

    /// What the server saw after the client's auth frame.
    #[derive(Debug, PartialEq)]
    enum Seen {
        CloseFrame,
        Eof,
    }

    /// Server that reads the auth frame, reports it on `authing`, answers
    /// with `verdict` once `answer` fires (never if it is dropped), then
    /// reports how the client ended the connection.
    async fn auth_server(
        verdict: &'static str,
    ) -> (String, oneshot::Receiver<()>, oneshot::Sender<()>, oneshot::Receiver<Seen>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let url = format!("ws://{}", listener.local_addr().expect("addr"));
        let (authing_tx, authing) = oneshot::channel();
        let (answer, answer_rx) = oneshot::channel::<()>();
        let (seen_tx, seen) = oneshot::channel();
        tokio::spawn(async move {
            let (tcp, _) = listener.accept().await.expect("accept");
            let mut ws = tokio_tungstenite::accept_async(tcp).await.expect("handshake");
            ws.next().await; // auth
            let _ = authing_tx.send(());
            let frame = tokio::select! {
                Ok(()) = answer_rx => {
                    let _ = ws.send(Message::Text(verdict.into())).await;
                    ws.next().await
                }
                frame = ws.next() => frame,
            };
            let _ = seen_tx.send(match frame {
                Some(Ok(Message::Close(_))) => Seen::CloseFrame,
                Some(Ok(other)) => panic!("unexpected frame {other:?}"),
                _ => Seen::Eof,
            });
        });
        (url, authing, answer, seen)
    }

    fn client(url: String) -> Arc<WebSocketClient> {
        let config = ConnectionConfig::new(url, AuthRequest::with_api_key("test-key"));
        Arc::new(WebSocketClient::with_reconnection_config(
            config,
            ReconnectionConfig::disabled(),
        ))
    }

    fn spawn_connect(
        client: &Arc<WebSocketClient>,
    ) -> JoinHandle<Result<(), MarketDataError>> {
        let client = Arc::clone(client);
        tokio::spawn(async move { client.connect().await })
    }

    async fn aborted(connect: JoinHandle<Result<(), MarketDataError>>) {
        let result = timeout(PROMPT, connect).await.expect("connect returns promptly");
        let err = result.expect("join").expect_err("connect is aborted");
        assert!(matches!(err, MarketDataError::ConnectionAborted), "{err:?}");
        assert_eq!(err.to_error_code(), 2010);
    }

    /// Every queued event, in order.
    fn events(client: &WebSocketClient) -> Vec<ConnectionEvent> {
        let rx = client.stream_receiver();
        let mut events = Vec::new();
        while let Some(item) = rx.try_receive() {
            if let StreamItem::Event(event) = item {
                events.push(event);
            }
        }
        events
    }

    fn assert_client_closed(client: &WebSocketClient, code: u16) {
        assert!(
            matches!(
                client.state(),
                ConnectionState::Closed { code: Some(c), intent: DisconnectIntent::Client, .. }
                    if c == code
            ),
            "{:?}",
            client.state()
        );
    }

    /// Nothing of the given-up connection is left installed.
    async fn assert_nothing_installed(client: &WebSocketClient) {
        assert!(client.write_tx.lock().await.is_none());
        assert!(client.ws_sink.lock().await.is_none());
        assert!(client.dispatch_handle.lock().await.is_none());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn disconnect_while_authenticating_aborts_connect_and_closes_the_socket() {
        let (url, authing, _answer, seen) = auth_server(r#"{"event":"authenticated"}"#).await;
        let client = client(url);
        let connect = spawn_connect(&client);
        authing.await.expect("auth frame sent");

        client.disconnect().await.expect("disconnect");

        aborted(connect).await;
        assert_eq!(timeout(PROMPT, seen).await.expect("seen").expect("seen"), Seen::CloseFrame);
        assert_client_closed(&client, 1000);
        assert_nothing_installed(&client).await;
        let events = events(&client);
        assert!(
            matches!(
                events.as_slice(),
                [
                    ConnectionEvent::Connecting,
                    ConnectionEvent::Connected,
                    ConnectionEvent::Disconnected { intent: DisconnectIntent::Client, .. },
                ]
            ),
            "{events:?}"
        );
        assert!(!client.is_connected().await);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn force_close_while_authenticating_aborts_connect() {
        let (url, authing, _answer, seen) = auth_server(r#"{"event":"authenticated"}"#).await;
        let client = client(url);
        let connect = spawn_connect(&client);
        authing.await.expect("auth frame sent");

        client.force_close().await.expect("force_close");

        aborted(connect).await;
        assert_eq!(timeout(PROMPT, seen).await.expect("seen").expect("seen"), Seen::CloseFrame);
        assert_client_closed(&client, 1006);
        assert_nothing_installed(&client).await;
    }

    /// The verdict arrives after shutdown was requested but before
    /// `connect()` was woken: authenticated or not, the connection is given
    /// up and closed, and reported neither as authenticated nor rejected.
    #[tokio::test(flavor = "multi_thread")]
    async fn verdict_after_shutdown_request_is_not_installed() {
        for verdict in [
            r#"{"event":"authenticated"}"#,
            r#"{"event":"error","data":{"message":"Invalid authentication credentials"}}"#,
        ] {
            let (url, authing, answer, seen) = auth_server(verdict).await;
            let client = client(url);
            let connect = spawn_connect(&client);
            authing.await.expect("auth frame sent");

            // The flag alone, without the wake-up `disconnect()` sends.
            client.shutdown_requested.store(true, Ordering::SeqCst);
            answer.send(()).expect("answer");

            aborted(connect).await;
            assert_eq!(
                timeout(PROMPT, seen).await.expect("seen").expect("seen"),
                Seen::CloseFrame,
                "{verdict}"
            );
            assert_nothing_installed(&client).await;
            let events = events(&client);
            assert!(
                matches!(events.as_slice(), [ConnectionEvent::Connecting, ConnectionEvent::Connected]),
                "{verdict}: {events:?}"
            );

            client.disconnect().await.expect("disconnect");
            assert_client_closed(&client, 1000);
        }
    }

    /// The server accepts TCP but never completes the WebSocket upgrade.
    #[tokio::test(flavor = "multi_thread")]
    async fn disconnect_while_opening_the_socket_aborts_connect() {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let url = format!("ws://{}", listener.local_addr().expect("addr"));
        let (accepted_tx, accepted) = oneshot::channel();
        tokio::spawn(async move {
            let (tcp, _) = listener.accept().await.expect("accept");
            let _ = accepted_tx.send(());
            let _held_open = tcp;
            std::future::pending::<()>().await;
        });
        let client = client(url);
        let connect = spawn_connect(&client);
        accepted.await.expect("accepted");

        client.disconnect().await.expect("disconnect");

        aborted(connect).await;
        assert_client_closed(&client, 1000);
        let events = events(&client);
        assert!(
            matches!(
                events.as_slice(),
                [
                    ConnectionEvent::Connecting,
                    ConnectionEvent::Disconnected { intent: DisconnectIntent::Client, .. },
                ]
            ),
            "{events:?}"
        );
    }

    /// Shutdown already requested once `connect()` is past its closed check
    /// (`disconnect()` completing in between): nothing is written. The flag
    /// is set before the call, which that check does not read, so no timing
    /// is involved.
    #[tokio::test(flavor = "multi_thread")]
    async fn shutdown_requested_before_connect_starts_leaves_the_state_alone() {
        let (url, _authing, _answer, _seen) = auth_server(r#"{"event":"authenticated"}"#).await;
        let client = client(url);
        client.shutdown_requested.store(true, Ordering::SeqCst);

        aborted(spawn_connect(&client)).await;
        assert_eq!(client.state(), ConnectionState::Disconnected);
        assert!(events(&client).is_empty());
    }

    /// Closing the given-up socket can take `connect()` up to
    /// `ABORTED_CLOSE_TIMEOUT`; `disconnect()` does not wait for that, so a
    /// short budget is kept. The server reads one byte of the auth frame (so
    /// the client has built it and is writing) and no more: the frame, far
    /// larger than the socket buffers, is still being written when
    /// `connect()` is woken, and the Close frame queued behind it cannot be
    /// sent.
    #[tokio::test(flavor = "multi_thread")]
    async fn short_shutdown_budget_is_kept_while_connect_closes_its_socket() {
        const BUDGET: Duration = Duration::from_millis(100);
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let url = format!("ws://{}", listener.local_addr().expect("addr"));
        let (writing_tx, writing) = oneshot::channel();
        tokio::spawn(async move {
            use tokio::io::AsyncReadExt;
            let (tcp, _) = listener.accept().await.expect("accept");
            let mut ws = tokio_tungstenite::accept_async(tcp).await.expect("handshake");
            ws.get_mut().read_exact(&mut [0u8; 1]).await.expect("auth frame starts");
            let _ = writing_tx.send(());
            std::future::pending::<()>().await;
        });
        let key = "k".repeat(32 * 1024 * 1024);
        let config = ConnectionConfig::new(url, AuthRequest::with_api_key(key));
        let client = Arc::new(WebSocketClient::with_reconnection_config(
            config,
            ReconnectionConfig::disabled(),
        ));
        let connect = spawn_connect(&client);
        writing.await.expect("auth frame being written");

        let started = std::time::Instant::now();
        client.shutdown_with_timeout(BUDGET).await.expect("shutdown");
        let elapsed = started.elapsed();

        assert!(elapsed < ABORTED_CLOSE_TIMEOUT - BUDGET, "took {elapsed:?}");
        assert_client_closed(&client, 1000);
        aborted(connect).await;
    }
}
