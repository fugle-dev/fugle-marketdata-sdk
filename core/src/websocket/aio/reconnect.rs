//! Reconnection and fresh-connect helpers for the async client.

use crate::models::{WebSocketMessage};
use crate::websocket::aio::writer::run_writer_task;
use crate::websocket::aio::{WsSink, WsStream};
use crate::websocket::connection_event::emit_event;
use crate::websocket::protocol::{
    classify_auth_response, frame_auth, frame_subscribe_raw, AuthOutcome,
};
use crate::websocket::{
    ConnectionConfig, ConnectionEvent, ConnectionState, DisconnectIntent, ReconnectionManager,
    SubscriptionManager,
};
use crate::MarketDataError;
use futures_util::{SinkExt, StreamExt};
use std::sync::{mpsc, Arc};
use tokio::sync::mpsc as tokio_mpsc;
use tokio::sync::{Mutex, RwLock};
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

/// Attempt auto-reconnection after a disconnect.
///
/// Called from within the dispatch loop's spawned task. Takes owned values
/// (cloned from the spawned task) because `mpsc::Sender` is `!Sync` and
/// holding `&mpsc::Sender` across await points would make the future `!Send`.
/// Returns `Some(ws_read)` on successful reconnect, `None` if reconnect is not
/// configured or all attempts are exhausted.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn try_reconnect(
    close_code: Option<u16>,
    reconnection: Arc<Mutex<ReconnectionManager>>,
    config: ConnectionConfig,
    state: Arc<RwLock<ConnectionState>>,
    event_tx: mpsc::SyncSender<ConnectionEvent>,
    ws_sink: Arc<Mutex<Option<WsSink>>>,
    write_tx_slot: Arc<Mutex<Option<tokio_mpsc::Sender<String>>>>,
    writer_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
    subscriptions: Arc<SubscriptionManager>,
    message_tx: tokio_mpsc::Sender<WebSocketMessage>,
) -> Option<WsStream> {
    // Check if we should attempt reconnection
    let should_reconnect = {
        let reconnection = reconnection.lock().await;
        reconnection.should_reconnect(close_code)
    };

    if !should_reconnect {
        // Not retriable - update state and send event
        {
            let mut st = state.write().await;
            *st = ConnectionState::Closed {
                code: close_code,
                reason: "Non-retriable error".to_string(),
                intent: DisconnectIntent::Network,
            };
        }

        let attempts = {
            let reconnection = reconnection.lock().await;
            reconnection.current_attempt()
        };

        emit_event(&event_tx, ConnectionEvent::ReconnectFailed {
            attempts,
        });
        return None;
    }

    // Attempt reconnection with exponential backoff. Liveness detection
    // is owned by each dispatch-task instance via the read-site timeout;
    // a successful reconnect spawns a fresh dispatch task that picks up
    // a fresh timeout window. No separate pause/resume needed.
    loop {
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
                    let mut st = state.write().await;
                    *st = ConnectionState::Reconnecting { attempt };
                }
                let delay_ms = d.as_millis() as u64;
                crate::tracing_compat::warn!(
                    target: "fugle_marketdata::ws",
                    attempt,
                    delay_ms,
                    "ws reconnect attempt"
                );
                emit_event(&event_tx, ConnectionEvent::Reconnecting {
                    attempt,
                });

                // Wait before reconnecting
                sleep(d).await;

                // Try to connect and authenticate
                match try_connect(
                    config.clone(),
                    Arc::clone(&state),
                    event_tx.clone(),
                    message_tx.clone(),
                )
                .await
                {
                    Ok((new_sink, ws_read)) => {
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
                    let mut st = state.write().await;
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

                emit_event(&event_tx, ConnectionEvent::ReconnectFailed {
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
pub(crate) async fn try_connect(
    config: ConnectionConfig,
    state: Arc<RwLock<ConnectionState>>,
    event_tx: mpsc::SyncSender<ConnectionEvent>,
    message_tx: tokio_mpsc::Sender<WebSocketMessage>,
) -> Result<(WsSink, WsStream), MarketDataError> {
    // Update state to Connecting
    {
        let mut st = state.write().await;
        *st = ConnectionState::Connecting;
    }
    emit_event(&event_tx, ConnectionEvent::Connecting {
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
                let mut st = state.write().await;
                *st = ConnectionState::Disconnected;
            }
            return Err(err);
        }
        Err(_) => {
            {
                let mut st = state.write().await;
                *st = ConnectionState::Disconnected;
            }
            return Err(MarketDataError::TimeoutError {
                operation: "WebSocket connect".to_string(),
            });
        }
    };

    // Split the stream
    let (mut new_ws_sink, mut ws_read) = ws_stream.split();

    crate::tracing_compat::info!(target: "fugle_marketdata::ws", "ws connected");
    emit_event(&event_tx, ConnectionEvent::Connected {
    });

    // Authenticate
    {
        let mut st = state.write().await;
        *st = ConnectionState::Authenticating;
    }

    let auth_json = frame_auth(config.auth.clone())?;

    new_ws_sink
        .send(Message::Text(auth_json.into()))
        .await
        .map_err(MarketDataError::from)?;

    // Wait for auth response (same pattern as WebSocketClient::connect)
    let msg_tx = message_tx.clone();
    let auth_timeout = Duration::from_secs(10);
    let auth_result = timeout(auth_timeout, async {
        while let Some(msg_result) = ws_read.next().await {
            match msg_result {
                Ok(Message::Text(text)) => {
                    if let Ok(ws_msg) = serde_json::from_str::<WebSocketMessage>(&text) {
                        let _ = msg_tx.send(ws_msg.clone()).await;
                        match classify_auth_response(&ws_msg) {
                            AuthOutcome::Authenticated => return Ok(()),
                            AuthOutcome::Failed(msg) => {
                                return Err(MarketDataError::AuthError { msg })
                            }
                            AuthOutcome::Pending => {}
                        }
                    }
                }
                Err(e) => return Err(MarketDataError::from(e)),
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
            {
                let mut st = state.write().await;
                *st = ConnectionState::Connected;
            }
            crate::tracing_compat::info!(target: "fugle_marketdata::ws", "ws authenticated");
            emit_event(&event_tx, ConnectionEvent::Authenticated {
            });
            Ok((new_ws_sink, ws_read))
        }
        Ok(Err(e)) => {
            {
                let mut st = state.write().await;
                *st = ConnectionState::Disconnected;
            }
            // Same auth-vs-other split as the primary connect() flow
            if let MarketDataError::AuthError { msg } = &e {
                emit_event(&event_tx, ConnectionEvent::Unauthenticated {
                    message: msg.clone(),
                });
            }
            Err(e)
        }
        Err(_) => {
            {
                let mut st = state.write().await;
                *st = ConnectionState::Disconnected;
            }
            Err(MarketDataError::TimeoutError {
                operation: "WebSocket authentication".to_string(),
            })
        }
    }
}
