//! Reconnection and fresh-connect helpers for the async client.

use crate::websocket::aio::writer::{start_writer, WriteFailure, WriterGeneration};
use crate::websocket::aio::{SharedState, WsSink, WsStream};
use crate::websocket::stream_queue::StreamSender;
use crate::websocket::protocol::{
    classify_auth_response, frame_auth, frame_resubscribe, AuthHandshake, AuthOutcome,
    ResubscribeFrame,
};
use crate::websocket::{
    ConnectionConfig, ConnectionEvent, ConnectionState, ReconnectionManager,
    SubscriptionManager,
};
use crate::MarketDataError;
use futures_util::{SinkExt, StreamExt};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc as tokio_mpsc;
use tokio::sync::{oneshot, Mutex, Notify};
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

/// Queue each of `frames` (see [`frame_resubscribe`]), in order. A frame
/// that could not be built or queued is reported as an `Error` naming its
/// label, and the rest are still sent. Returns the first failure.
pub(crate) async fn replay_subscriptions(
    frames: Vec<ResubscribeFrame>,
    stream: &StreamSender,
    write_tx: &tokio_mpsc::Sender<String>,
) -> Result<(), MarketDataError> {
    let mut first_err = None;
    for ResubscribeFrame { label, frame } in frames {
        let sent = match frame {
            Ok(json) => write_tx.send(json).await.map_err(|_| MarketDataError::ConnectionError {
                msg: "Writer task is not running".to_string(),
            }),
            Err(e) => Err(e),
        };
        if let Err(e) = sent {
            // Not after the client's close has been reported (#145).
            stream.emit_unless_closed(ConnectionEvent::resubscribe_failed(&label, &e));
            first_err.get_or_insert(e);
        }
    }
    first_err.map_or(Ok(()), Err)
}

/// Send the auth frame, then read frames off `ws_read` until a terminal
/// auth outcome arrives or `auth_timeout` elapses. The text frames read are
/// returned with the outcome, to be queued after the matching event (#68).
/// Shared by
/// `WebSocketClient::connect` and `try_connect` so the auth protocol cannot
/// drift between fresh-connect and reconnect.
pub(crate) async fn authenticate(
    ws_sink: &mut WsSink,
    ws_read: &mut WsStream,
    config: &ConnectionConfig,
    stream: &StreamSender,
    auth_timeout: Duration,
) -> AuthHandshake {
    // The drop count restarts with each connection attempt.
    stream.start_connection();
    let auth_json = match frame_auth(config.auth.clone()) {
        Ok(json) => json,
        Err(e) => return AuthHandshake::Failed(e),
    };
    if let Err(e) = ws_sink.send(Message::Text(auth_json.into())).await {
        return AuthHandshake::Failed(e.into());
    }
    await_auth_response(ws_read, auth_timeout).await
}

/// Read frames off `ws_read` until a terminal auth outcome arrives or
/// `auth_timeout` elapses.
pub(crate) async fn await_auth_response(
    ws_read: &mut WsStream,
    auth_timeout: Duration,
) -> AuthHandshake {
    let result = timeout(auth_timeout, async {
        let mut frames = Vec::new();
        while let Some(msg_result) = ws_read.next().await {
            match msg_result {
                Ok(Message::Text(text)) => {
                    if let Ok(ws_msg) = crate::websocket::protocol::parse_text_frame(&text) {
                        let outcome = classify_auth_response(&ws_msg);
                        frames.push(ws_msg);
                        match outcome {
                            AuthOutcome::Authenticated(data) => {
                                return AuthHandshake::Authenticated { data, frames }
                            }
                            AuthOutcome::Failed { message, data } => {
                                return AuthHandshake::Rejected { message, data, frames }
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
/// Returns the new read half and the receiver of its writer's failed write
/// on successful reconnect, `None` if reconnect is not configured, all
/// attempts are exhausted, or `shutdown_requested` was set.
///
/// Once `disconnect()` sets `shutdown_requested` this emits nothing further
/// (`disconnect()` queues the final `Disconnected` itself, #98):
/// the flag is checked before each attempt, after its backoff sleep, and
/// inside [`try_connect`] before each lifecycle event. `shutdown_notify`
/// ends a backoff sleep or connection attempt in progress, and a connection
/// that authenticates after shutdown was requested is dropped instead of
/// installed, so `disconnect()` need not wait out its drain budget (#110).
#[allow(clippy::too_many_arguments)]
pub(crate) async fn try_reconnect(
    close_code: Option<u16>,
    reconnection: Arc<Mutex<ReconnectionManager>>,
    config: ConnectionConfig,
    state: SharedState,
    stream: StreamSender,
    ws_sink: Arc<Mutex<Option<WsSink>>>,
    write_tx_slot: Arc<Mutex<Option<tokio_mpsc::Sender<String>>>>,
    writer_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
    writer_generation: WriterGeneration,
    subscriptions: Arc<SubscriptionManager>,
    shutdown_requested: Arc<AtomicBool>,
    shutdown_notify: Arc<Notify>,
) -> Option<(WsStream, oneshot::Receiver<WriteFailure>)> {
    let stopping = || shutdown_requested.load(Ordering::SeqCst);
    // Registered before the flag is first read: shutdown sets the flag
    // before notifying, so it is either seen by `stopping()` or wakes this.
    let shutdown = shutdown_notify.notified();
    tokio::pin!(shutdown);
    shutdown.as_mut().enable();

    // Check if we should attempt reconnection
    let should_reconnect = {
        let reconnection = reconnection.lock().await;
        reconnection.should_reconnect(close_code)
    };

    if !should_reconnect {
        // Not retriable. The close was already reported as
        // `Disconnected { will_reconnect: false }`, which recorded the
        // matching `Closed` state (#86); no attempt was made, so there is no
        // `ReconnectFailed` to report.
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

                // A close reported since the check above ends the loop
                // here, so nothing follows the final event (#145).
                if !stream.reconnect_step(
                    &state,
                    ConnectionState::Reconnecting { attempt },
                    Some(ConnectionEvent::Reconnecting { attempt }),
                ) {
                    return None;
                }
                crate::tracing_compat::warn!(
                    target: "fugle_marketdata::ws",
                    attempt,
                    delay_ms = d.as_millis() as u64,
                    "ws reconnect attempt"
                );

                // Wait before reconnecting
                tokio::select! {
                    () = sleep(d) => {}
                    () = shutdown.as_mut() => return None,
                }
                if stopping() {
                    return None;
                }

                // Try to connect and authenticate. A shutdown drops the
                // attempt where it stands; `disconnect()` reports the
                // stopped reconnect itself (#98).
                let connected = tokio::select! {
                    result = try_connect(
                        config.clone(),
                        Arc::clone(&state),
                        stream.clone(),
                        &shutdown_requested,
                    ) => result,
                    () = shutdown.as_mut() => return None,
                };
                match connected {
                    Ok((new_sink, ws_read)) => {
                        // Reset reconnection manager on success
                        {
                            let mut reconnection = reconnection.lock().await;
                            reconnection.reset();
                        }

                        // Replace the old writer, then install the new sink,
                        // unless shutdown was requested meanwhile.
                        let (new_write_tx, write_failed_rx) = start_writer(
                            new_sink,
                            &ws_sink,
                            &write_tx_slot,
                            &writer_handle,
                            &writer_generation,
                            stream.clone(),
                            Some(&shutdown_requested),
                        )
                        .await?;

                        // Resubscribe all stored subscriptions through the new writer
                        subscriptions.clear_server_ids();
                        let _ = replay_subscriptions(
                            frame_resubscribe(subscriptions.get_all()),
                            &stream,
                            &new_write_tx,
                        )
                        .await;

                        // Liveness detection auto-restarts: the caller of
                        // try_reconnect re-enters the dispatch loop with this
                        // new ws_read, and dispatch_messages's read-site
                        // timeout is a fresh `tokio::time::timeout` per loop
                        // iteration.
                        return Some((ws_read, write_failed_rx));
                    }
                    // The close was reported meanwhile (#145).
                    Err(MarketDataError::ClientClosed) => return None,
                    // Stopped mid-attempt; the shutdown path owns the close.
                    Err(_) if stopping() => return None,
                    Err(e) => {
                        // Report why this attempt failed before the next
                        // `Reconnecting` (#200), as the sync client does. A
                        // rejection was already reported as `Unauthenticated`.
                        if !matches!(e, MarketDataError::AuthError { .. })
                            && !stream.emit_unless_closed(ConnectionEvent::error(&e))
                        {
                            return None;
                        }
                        continue;
                    }
                }
            }
            None => {
                // Max attempts reached
                let attempts = {
                    let reconnection = reconnection.lock().await;
                    reconnection.current_attempt()
                };

                // Unless a racing `disconnect()` reported the close first.
                stream.reconnect_failed(&state, close_code, attempts);

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
/// Only the reconnect loop calls this. If `shutdown_requested` is set while
/// connecting, or the client's close has been reported (#145), the new
/// connection is dropped without emitting further events or changing the
/// state, and `ClientClosed` is returned. Any other error is the caller's to
/// report (#200): a transport or handshake failure leaves the state
/// `Disconnected` and its lifecycle events end at `Connecting` or
/// `Connected`.
pub(crate) async fn try_connect(
    config: ConnectionConfig,
    state: SharedState,
    stream: StreamSender,
    shutdown_requested: &AtomicBool,
) -> Result<(WsSink, WsStream), MarketDataError> {
    let stopping = || shutdown_requested.load(Ordering::SeqCst);

    // Each step is reported only if the client's close has not been (#145).
    if !stream.reconnect_step(&state, ConnectionState::Connecting, Some(ConnectionEvent::Connecting {})) {
        return Err(MarketDataError::ClientClosed);
    }

    // Connect to WebSocket
    let tls_connector = tls_connector_for(&config)?;
    let connect_result = timeout(
        config.connect_timeout,
        connect_async_tls_with_config(&config.url, None, false, Some(tls_connector)),
    )
    .await;

    let (ws_stream, _response) = match connect_result {
        Ok(Ok(connected)) => connected,
        Ok(Err(e)) => {
            let err: MarketDataError = e.into();
            if !stream.reconnect_step(&state, ConnectionState::Disconnected, None) {
                return Err(MarketDataError::ClientClosed);
            }
            return Err(err);
        }
        Err(_) => {
            if !stream.reconnect_step(&state, ConnectionState::Disconnected, None) {
                return Err(MarketDataError::ClientClosed);
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
    if !stream.reconnect_step(
        &state,
        ConnectionState::Authenticating,
        Some(ConnectionEvent::Connected {}),
    ) {
        return Err(MarketDataError::ClientClosed);
    }

    // Shared with WebSocketClient::connect
    let handshake = authenticate(
        &mut new_ws_sink,
        &mut ws_read,
        &config,
        &stream,
        Duration::from_secs(10),
    )
    .await;
    if stopping() {
        return Err(MarketDataError::ClientClosed);
    }

    match handshake {
        AuthHandshake::Authenticated { data, frames } => {
            // Queued before this function returns, so ahead of anything the
            // dispatch loop reads next. A new connection: it may report its
            // own close. A close reported meanwhile keeps it from reopening.
            if !stream.reconnect_authenticated(&state, data, frames) {
                return Err(MarketDataError::ClientClosed);
            }
            crate::tracing_compat::info!(target: "fugle_marketdata::ws", "ws authenticated");
            Ok((new_ws_sink, ws_read))
        }
        AuthHandshake::Rejected { message, data, frames } => {
            if !stream.reconnect_step(&state, ConnectionState::Disconnected, None)
                || !stream.reconnect_rejected(message.clone(), data, frames)
            {
                return Err(MarketDataError::ClientClosed);
            }
            Err(MarketDataError::AuthError { msg: message, http: None })
        }
        AuthHandshake::Failed(e) => {
            if !stream.reconnect_step(&state, ConnectionState::Disconnected, None) {
                return Err(MarketDataError::ClientClosed);
            }
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::error_code;
    use crate::metrics_compat::DropCounter;
    use crate::models::{Channel, SubscribeRequest};
    use crate::websocket::stream_queue::stream;
    use crate::websocket::StreamItem;
    use crate::AuthRequest;

    /// Two trades rows fold into one frame; books stays on its own.
    fn frames() -> Vec<ResubscribeFrame> {
        frame_resubscribe(vec![
            SubscribeRequest::new(Channel::Trades, "2330"),
            SubscribeRequest::new(Channel::Books, "2317"),
            SubscribeRequest::new(Channel::Trades, "2454"),
        ])
    }

    fn event_stream() -> (StreamSender, crate::websocket::stream_queue::QueueReceiver) {
        let config = ConnectionConfig::new("ws://localhost", AuthRequest::with_api_key("k"));
        let counter = || DropCounter::new("test", "localhost", "test");
        stream(&config, counter(), counter())
    }

    #[tokio::test]
    async fn replay_queues_one_frame_per_batch_in_order() {
        let (tx, rx) = event_stream();
        let (write_tx, mut write_rx) = tokio_mpsc::channel(8);

        replay_subscriptions(frames(), &tx, &write_tx).await.expect("replay succeeds");

        let first = write_rx.try_recv().expect("first frame");
        let second = write_rx.try_recv().expect("second frame");
        assert!(write_rx.try_recv().is_err(), "one frame per batch");
        assert!(first.contains(r#""symbols":["2330","2454"]"#), "{first}");
        assert!(second.contains(r#""symbol":"2317""#), "{second}");
        assert!(rx.try_recv().is_err(), "no events on success");
    }

    #[tokio::test]
    async fn replay_reports_each_failed_batch_and_keeps_going() {
        let (tx, rx) = event_stream();
        let (write_tx, write_rx) = tokio_mpsc::channel(8);
        drop(write_rx);

        let err = replay_subscriptions(frames(), &tx, &write_tx).await.expect_err("replay fails");
        assert!(matches!(err, MarketDataError::ConnectionError { .. }));

        let errors: Vec<_> = std::iter::from_fn(|| rx.try_recv().ok())
            .map(|item| match item {
                StreamItem::Event(ConnectionEvent::Error(info)) => info,
                other => panic!("expected Error, got {other:?}"),
            })
            .collect();
        assert_eq!(errors.len(), 2, "one Error per batch");
        assert!(errors.iter().all(|info| info.code == error_code::CONNECTION));
        assert!(errors[0].message.contains("trades (2 symbols)"), "{}", errors[0].message);
        assert!(errors[1].message.contains("books:2317"), "{}", errors[1].message);
    }
}
