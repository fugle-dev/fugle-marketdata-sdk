//! Reconnection and fresh-connect helpers for the async client.

use crate::metrics_compat::DropCounter;
use crate::models::{WebSocketMessage};
use crate::websocket::aio::writer::run_writer_task;
use crate::websocket::aio::{write_state, SharedState, WsSink, WsStream};
use crate::websocket::connection_event::{emit_event, DisconnectLatch};
use crate::websocket::message_queue::QueueSender;
use crate::websocket::protocol::{
    classify_auth_response, frame_auth, frame_subscribe_raw, AuthHandshake, AuthOutcome,
};
use crate::websocket::{
    ConnectionConfig, ConnectionEvent, ConnectionState, DisconnectIntent, ReconnectionManager,
    SubscriptionManager,
};
use crate::MarketDataError;
use futures_util::{SinkExt, StreamExt};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use tokio::sync::mpsc as tokio_mpsc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio::time::{sleep, timeout, Duration};
use tokio_tungstenite::{connect_async_tls_with_config, Connector};
use tokio_tungstenite::tungstenite::Message;

/// Build the rustls `Connector` shared by initial connect and reconnect paths.
/// Same `Arc<ClientConfig>` is reused across reconnects so the OS trust-store
/// load (`rustls-native-certs`) is amortized.
pub(crate) fn tls_connector_for(
    config: &ConnectionConfig,
) -> Result<Connector, MarketDataError> {
    let client_config = crate::tls::build_rustls_config(&config.tls)?;
    Ok(Connector::Rustls(client_config))
}

/// Send the auth frame, then read frames off `ws_read` until a terminal
/// auth outcome arrives or `auth_timeout` elapses. Each text frame is
/// pushed to `message_tx` so subscribers see the auth payloads; a full
/// queue drops them rather than stalling the handshake. Shared by
/// `WebSocketClient::connect` and `try_connect` so the auth protocol cannot
/// drift between fresh-connect and reconnect.
pub(crate) async fn authenticate(
    ws_sink: &mut WsSink,
    ws_read: &mut WsStream,
    config: &ConnectionConfig,
    message_tx: &QueueSender<WebSocketMessage>,
    auth_timeout: Duration,
) -> AuthHandshake {
    let auth_json = match frame_auth(config.auth.clone()) {
        Ok(json) => json,
        Err(e) => return AuthHandshake::Failed(e),
    };
    if let Err(e) = ws_sink.send(Message::Text(auth_json.into())).await {
        return AuthHandshake::Failed(e.into());
    }
    await_auth_response(ws_read, message_tx, auth_timeout).await
}

/// Read frames off `ws_read` until a terminal auth outcome arrives or
/// `auth_timeout` elapses.
pub(crate) async fn await_auth_response(
    ws_read: &mut WsStream,
    message_tx: &QueueSender<WebSocketMessage>,
    auth_timeout: Duration,
) -> AuthHandshake {
    let result = timeout(auth_timeout, async {
        while let Some(msg_result) = ws_read.next().await {
            match msg_result {
                Ok(Message::Text(text)) => {
                    if let Ok(ws_msg) = crate::websocket::protocol::parse_text_frame(&text) {
                        // No drop report yet: `Authenticated` has not been
                        // emitted. The dispatch loop reports these drops.
                        message_tx.push(ws_msg.clone());
                        match classify_auth_response(&ws_msg) {
                            AuthOutcome::Authenticated(data) => {
                                return AuthHandshake::Authenticated(data)
                            }
                            AuthOutcome::Failed { message, data } => {
                                return AuthHandshake::Rejected { message, data }
                            }
                            AuthOutcome::Pending => {}
                        }
                    }
                }
                Err(e) => return AuthHandshake::Failed(MarketDataError::from(e)),
                _ => {}
            }
        }
        AuthHandshake::Failed(MarketDataError::ConnectionError {
            msg: "Stream closed during authentication".to_string(),
        })
    })
    .await;
    result.unwrap_or_else(|_| {
        AuthHandshake::Failed(MarketDataError::TimeoutError {
            operation: "WebSocket authentication".to_string(),
        })
    })
}

/// Attempt auto-reconnection after a disconnect.
///
/// Called from within the dispatch loop's spawned task. Takes owned values
/// (cloned from the spawned task) because `mpsc::Sender` is `!Sync` and
/// holding `&mpsc::Sender` across await points would make the future `!Send`.
/// Returns `Some(ws_read)` on successful reconnect, `None` if reconnect is not
/// configured, all attempts are exhausted, or `shutdown_requested` was set.
///
/// Once `disconnect()` sets `shutdown_requested` this emits nothing further:
/// the flag is checked before each attempt, after its backoff sleep, and
/// inside [`try_connect`] before each lifecycle event.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn try_reconnect(
    close_code: Option<u16>,
    reconnection: Arc<Mutex<ReconnectionManager>>,
    config: ConnectionConfig,
    state: SharedState,
    event_tx: mpsc::SyncSender<ConnectionEvent>,
    events_dropped: DropCounter,
    ws_sink: Arc<Mutex<Option<WsSink>>>,
    write_tx_slot: Arc<Mutex<Option<tokio_mpsc::Sender<String>>>>,
    writer_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
    subscriptions: Arc<SubscriptionManager>,
    message_tx: QueueSender<WebSocketMessage>,
    disconnect_latch: Arc<DisconnectLatch>,
    shutdown_requested: Arc<AtomicBool>,
) -> Option<WsStream> {
    let stopping = || shutdown_requested.load(Ordering::SeqCst);

    // Check if we should attempt reconnection
    let should_reconnect = {
        let reconnection = reconnection.lock().await;
        reconnection.should_reconnect(close_code)
    };

    if !should_reconnect {
        // Not retriable. The close was already reported as
        // `Disconnected { will_reconnect: false }`; no attempt was made, so
        // there is no `ReconnectFailed` to report.
        let mut st = write_state(&state);
        *st = ConnectionState::Closed {
            code: close_code,
            reason: "Non-retriable error".to_string(),
            intent: DisconnectIntent::Network,
        };
        return None;
    }

    // Attempt reconnection with exponential backoff. Liveness detection
    // is owned by each dispatch-task instance via the read-site timeout;
    // a successful reconnect spawns a fresh dispatch task that picks up
    // a fresh timeout window. No separate pause/resume needed.
    loop {
        if stopping() {
            return None;
        }
        let delay = {
            let mut reconnection = reconnection.lock().await;
            reconnection.next_delay()
        };

        match delay {
            Some(d) => {
                let attempt = {
                    let reconnection = reconnection.lock().await;
                    reconnection.current_attempt()
                };

                // Update state to Reconnecting
                {
                    let mut st = write_state(&state);
                    *st = ConnectionState::Reconnecting { attempt };
                }
                crate::tracing_compat::warn!(
                    target: "fugle_marketdata::ws",
                    attempt,
                    delay_ms = d.as_millis() as u64,
                    "ws reconnect attempt"
                );
                emit_event(&event_tx, &events_dropped, ConnectionEvent::Reconnecting {
                    attempt,
                });

                // Wait before reconnecting
                sleep(d).await;
                if stopping() {
                    return None;
                }

                // Try to connect and authenticate
                match try_connect(
                    config.clone(),
                    Arc::clone(&state),
                    event_tx.clone(),
                    events_dropped.clone(),
                    message_tx.clone(),
                    &shutdown_requested,
                )
                .await
                {
                    Ok((new_sink, ws_read)) => {
                        // New connection: it may report its own close.
                        disconnect_latch.reset();
                        message_tx.start_connection();

                        // Store the new write half
                        {
                            let mut sink_guard = ws_sink.lock().await;
                            *sink_guard = Some(new_sink);
                        }

                        // Reset reconnection manager on success
                        {
                            let mut reconnection = reconnection.lock().await;
                            reconnection.reset();
                        }

                        // Rebuild the writer task for the new sink
                        if let Some(prev) = writer_handle.lock().await.take() {
                            prev.abort();
                        }
                        let (new_write_tx, new_write_rx) = tokio_mpsc::channel::<String>(64);
                        {
                            let mut guard = write_tx_slot.lock().await;
                            *guard = Some(new_write_tx.clone());
                        }
                        let writer_task_handle = tokio::spawn(run_writer_task(
                            new_write_rx,
                            Arc::clone(&ws_sink),
                            event_tx.clone(),
                            events_dropped.clone(),
                        ));
                        {
                            let mut guard = writer_handle.lock().await;
                            *guard = Some(writer_task_handle);
                        }

                        // Resubscribe all stored subscriptions through the new writer
                        let subs = subscriptions.get_all();
                        for req in subs {
                            if let Ok(sub_json) = frame_subscribe_raw(req) {
                                let _ = new_write_tx.send(sub_json).await;
                            }
                        }

                        // Liveness detection auto-restarts: the caller of
                        // try_reconnect re-enters the dispatch loop with this
                        // new ws_read, and dispatch_messages's read-site
                        // timeout is a fresh `tokio::time::timeout` per loop
                        // iteration.
                        return Some(ws_read);
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
                    let mut st = write_state(&state);
                    *st = ConnectionState::Closed {
                        code: close_code,
                        reason: "Max reconnection attempts reached".to_string(),
                        intent: DisconnectIntent::Network,
                    };
                }

                let attempts = {
                    let reconnection = reconnection.lock().await;
                    reconnection.current_attempt()
                };

                emit_event(&event_tx, &events_dropped, ConnectionEvent::ReconnectFailed {
                    attempts,
                });

                return None;
            }
        }
    }
}

/// Attempt a fresh connection: connect to WebSocket and authenticate.
///
/// On success, returns the write sink and read stream. The caller is responsible
/// for storing the sink and setting up dispatch. Takes owned values for Send safety.
///
/// If `shutdown_requested` is set while connecting, the new connection is
/// dropped without emitting further events and `ClientClosed` is returned.
pub(crate) async fn try_connect(
    config: ConnectionConfig,
    state: SharedState,
    event_tx: mpsc::SyncSender<ConnectionEvent>,
    events_dropped: DropCounter,
    message_tx: QueueSender<WebSocketMessage>,
    shutdown_requested: &AtomicBool,
) -> Result<(WsSink, WsStream), MarketDataError> {
    let stopping = || shutdown_requested.load(Ordering::SeqCst);

    // Update state to Connecting
    {
        let mut st = write_state(&state);
        *st = ConnectionState::Connecting;
    }
    emit_event(&event_tx, &events_dropped, ConnectionEvent::Connecting {
    });

    // Connect to WebSocket
    let tls_connector = tls_connector_for(&config)?;
    let connect_result = timeout(
        config.connect_timeout,
        connect_async_tls_with_config(&config.url, None, false, Some(tls_connector)),
    )
    .await;

    let (ws_stream, _response) = match connect_result {
        Ok(Ok((stream, response))) => (stream, response),
        Ok(Err(e)) => {
            let err: MarketDataError = e.into();
            {
                let mut st = write_state(&state);
                *st = ConnectionState::Disconnected;
            }
            return Err(err);
        }
        Err(_) => {
            {
                let mut st = write_state(&state);
                *st = ConnectionState::Disconnected;
            }
            return Err(MarketDataError::TimeoutError {
                operation: "WebSocket connect".to_string(),
            });
        }
    };

    // Split the stream
    let (mut new_ws_sink, mut ws_read) = ws_stream.split();

    if stopping() {
        return Err(MarketDataError::ClientClosed);
    }
    crate::tracing_compat::info!(target: "fugle_marketdata::ws", "ws connected");
    emit_event(&event_tx, &events_dropped, ConnectionEvent::Connected {
    });

    // Authenticate
    {
        let mut st = write_state(&state);
        *st = ConnectionState::Authenticating;
    }

    // Shared with WebSocketClient::connect
    let handshake = authenticate(
        &mut new_ws_sink,
        &mut ws_read,
        &config,
        &message_tx,
        Duration::from_secs(10),
    )
    .await;
    if stopping() {
        return Err(MarketDataError::ClientClosed);
    }

    match handshake {
        AuthHandshake::Authenticated(data) => {
            {
                let mut st = write_state(&state);
                *st = ConnectionState::Connected;
            }
            crate::tracing_compat::info!(target: "fugle_marketdata::ws", "ws authenticated");
            emit_event(&event_tx, &events_dropped, ConnectionEvent::Authenticated { data });
            Ok((new_ws_sink, ws_read))
        }
        AuthHandshake::Rejected { message, data } => {
            {
                let mut st = write_state(&state);
                *st = ConnectionState::Disconnected;
            }
            emit_event(&event_tx, &events_dropped, ConnectionEvent::Unauthenticated {
                message: message.clone(),
                data,
            });
            Err(MarketDataError::AuthError { msg: message })
        }
        AuthHandshake::Failed(e) => {
            {
                let mut st = write_state(&state);
                *st = ConnectionState::Disconnected;
            }
            Err(e)
        }
    }
}
