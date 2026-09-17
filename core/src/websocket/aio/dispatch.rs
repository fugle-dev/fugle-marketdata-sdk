//! Async dispatch loop: reads frames from the WS stream, parses, and pushes
//! messages onto the client's stream. Also implements optional outbound ping.

use crate::tracing_compat::{debug, warn};
use crate::websocket::aio::WsStream;
use crate::websocket::connection_event::{peer_close_disconnect, will_reconnect_after};
use crate::websocket::stream_queue::StreamSender;
use crate::websocket::protocol::{handle_subscribed_event, parse_binary_frame, parse_text_frame};
use crate::websocket::{ConnectionEvent, DisconnectIntent, ReconnectionManager, SubscriptionManager};
use futures_util::StreamExt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::Message;

/// Dispatch incoming WebSocket messages to appropriate channels
///
/// This task runs in the background after connect() succeeds.
/// It will terminate when:
/// 1. WebSocket connection closes (returns close code)
/// 2. Server sends Close frame (returns close code from frame)
/// 3. WebSocket error occurs (returns None)
/// 4. Task is aborted by disconnect() (task cancelled at .await point)
///
/// The function is cancellation-safe: aborting at any `.await` point
/// will not leave resources in an inconsistent state.
///
/// # Arguments
///
/// * `ws_read` - The read half of the WebSocket stream
/// * `stream` - The client's stream: parsed messages and connection events
/// * `heartbeat_timeout` - If `Some(d)`, wrap each `ws_read.next()` in
///   `tokio::time::timeout(d, ...)` and emit
///   [`ConnectionEvent::HeartbeatTimeout`] followed by
///   `Disconnected { intent: Network }` when the timer fires. If
///   `None`, liveness detection is disabled and reads block indefinitely.
/// * `subscriptions` - Subscription manager for `subscribed` event handling
/// * `reconnection` - Reconnect policy, consulted for each `Disconnected`'s
///   `will_reconnect` so it matches the decision the caller makes next
///
/// # Returns
///
/// Close code from the WebSocket close frame, or None if the connection
/// was dropped without a proper close, due to an error, or due to
/// `heartbeat_timeout` firing. The dispatch-task caller treats `None` as
/// reconnectable per `should_reconnect`'s default arm.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn dispatch_messages(
    mut ws_read: WsStream,
    stream: StreamSender,
    heartbeat_timeout: Option<Duration>,
    subscriptions: Arc<SubscriptionManager>,
    shutdown_requested: Arc<AtomicBool>,
    reconnection: Arc<Mutex<ReconnectionManager>>,
) -> Option<u16> {
    let will_reconnect = |intent: DisconnectIntent, code: Option<u16>| {
        let reconnection = Arc::clone(&reconnection);
        let shutdown_requested = shutdown_requested.load(Ordering::SeqCst);
        async move {
            let mgr = reconnection.lock().await;
            will_reconnect_after(&mgr, intent, code, shutdown_requested)
        }
    };

    loop {
        // A socket that always has data never returns `Pending`, and frames
        // decoded from tungstenite's buffer spend no coop budget, so without
        // this the loop would keep its worker until the socket drains. Tasks
        // it wakes — a `stream()` consumer on the same runtime — would
        // starve meanwhile while the queue fills and drops (#46).
        tokio::task::coop::consume_budget().await;

        // Read-site liveness: if `heartbeat_timeout` is set, the next
        // frame must arrive within that window or we declare the
        // connection dead. When None, fall back to a plain blocking
        // read (no liveness detection).
        let frame_result = match heartbeat_timeout {
            Some(timeout) => match tokio::time::timeout(timeout, ws_read.next()).await {
                Ok(opt) => opt,
                Err(_elapsed) => {
                    // A caller-initiated shutdown reports the close itself.
                    if shutdown_requested.load(Ordering::SeqCst) {
                        return None;
                    }
                    let elapsed_ms = timeout.as_millis() as u64;
                    warn!(
                        target: "fugle_marketdata::ws",
                        elapsed_ms,
                        "heartbeat timeout: no inbound frame in window"
                    );
                    stream.emit(ConnectionEvent::HeartbeatTimeout {
                        elapsed: timeout,
                    });
                    // Through the latch, so a racing `disconnect()` cannot
                    // report this connection's close a second time (#47).
                    stream.emit_disconnected(
                        None,
                        format!("Heartbeat timeout after {elapsed_ms}ms"),
                        DisconnectIntent::Network,
                        will_reconnect(DisconnectIntent::Network, None).await,
                    );
                    return None;
                }
            },
            None => ws_read.next().await,
        };

        let msg_result = match frame_result {
            Some(r) => r,
            None => {
                // Stream ended cleanly without close frame. Suppress
                // the Disconnected emit when the caller already issued
                // a `disconnect()` / `shutdown_with_timeout()` — the
                // shutdown path will emit `Disconnected { intent: Client }`
                // itself, and a duplicate `Disconnected { intent: Network }`
                // here would race ahead of it (the client-initiated
                // local socket close manifests as EOF on the read half).
                if !shutdown_requested.load(Ordering::SeqCst) {
                    stream.emit_disconnected(
                        None,
                        "Connection closed".to_string(),
                        DisconnectIntent::Network,
                        will_reconnect(DisconnectIntent::Network, None).await,
                    );
                }
                return None;
            }
        };

        match msg_result {
            Ok(Message::Text(text)) => {
                debug!(
                    target: "fugle_marketdata::ws",
                    bytes = text.len(),
                    kind = "text",
                    "ws frame received"
                );
                match parse_text_frame(&text) {
                    Ok(ws_msg) => {
                        // Mutex is only taken when event == "subscribed" (cheap
                        // string compare for every other message).
                        handle_subscribed_event(&subscriptions, &ws_msg);
                        stream.push_message(ws_msg);
                    }
                    Err(e) => {
                        stream.emit(ConnectionEvent::error_with_message(
                            &e,
                            format!("Failed to deserialize message: {}", e),
                        ));
                    }
                }
            }
            Ok(Message::Binary(data)) => {
                debug!(
                    target: "fugle_marketdata::ws",
                    bytes = data.len(),
                    kind = "binary",
                    "ws frame received"
                );
                match parse_binary_frame(&data) {
                    Ok(ws_msg) => {
                        handle_subscribed_event(&subscriptions, &ws_msg);
                        stream.push_message(ws_msg);
                    }
                    Err(e) => {
                        stream.emit(ConnectionEvent::error_with_message(
                            &e,
                            format!("Failed to deserialize binary message: {}", e),
                        ));
                    }
                }
            }
            Ok(Message::Pong(_)) => {
                // RFC 6455 control-frame pong: counted as activity by
                // virtue of resetting the read-site timeout. Fugle sends
                // pong via JSON message; this branch is defensive.
            }
            Ok(Message::Close(close_frame)) => {
                let code = close_frame.as_ref().map(|cf| cf.code.into());
                // The flag and the emit are not atomic: `disconnect()` may
                // set the flag right after this check. The latch keeps the
                // shutdown path from reporting the same close again (#41).
                if let Some((code, reason, intent)) = peer_close_disconnect(
                    code,
                    close_frame.as_ref().map(|cf| cf.reason.to_string()),
                    shutdown_requested.load(Ordering::SeqCst),
                ) {
                    let will_reconnect = will_reconnect(intent, code).await;
                    stream.emit_disconnected(
                        code,
                        reason,
                        intent,
                        will_reconnect,
                    );
                }
                return code;
            }
            Ok(Message::Ping(_)) => {
                // Server sent ping, tokio-tungstenite auto-responds with pong
                // No action needed
            }
            Err(e) => {
                // WebSocket transport error — connection broken (e.g.
                // "Connection reset without closing handshake"). Emit
                // both `Error` (preserves existing diagnostic surface)
                // *and* `Disconnected { intent: Network }` so consumers
                // pattern-matching on `ConnectionEvent::Disconnected`
                // see the close exactly as they do for the clean-EOF
                // path above. Skip both when shutdown was caller-initiated:
                // a local `shutdown_with_timeout()` typically tears
                // down the socket which surfaces here as a transport
                // error (or a peer that skips TLS close_notify, #22),
                // and the shutdown path already emits the canonical
                // `Disconnected { intent: Client }`.
                if shutdown_requested.load(Ordering::SeqCst) {
                    return None;
                }
                let err_msg = format!("WebSocket error: {}", e);
                stream.emit(ConnectionEvent::error_with_message(
                    &crate::MarketDataError::from(e),
                    err_msg.clone(),
                ));
                stream.emit_disconnected(
                    None,
                    err_msg,
                    DisconnectIntent::Network,
                    will_reconnect(DisconnectIntent::Network, None).await,
                );
                return None;
            }
            Ok(Message::Frame(_)) => {
                // Raw frames shouldn't appear in normal usage
            }
        }
    }
}

/// Internal ping sender
///
/// Sends WebSocket ping frames when signaled by health check
#[allow(dead_code)] // Will be used when ping support is fully implemented
pub(crate) async fn send_pings(
    mut ws_sink: crate::websocket::aio::WsSink,
    ping_rx: mpsc::Receiver<()>,
) {
    use futures_util::SinkExt;
    while ping_rx.recv().is_ok() {
        if ws_sink.send(Message::Ping(vec![].into())).await.is_err() {
            // Failed to send ping, connection likely closed
            break;
        }
    }
}
