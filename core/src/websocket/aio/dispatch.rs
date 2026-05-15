//! Async dispatch loop: reads frames from the WS stream, parses, and pushes
//! messages onto the inbound channel. Also implements optional outbound ping.

use crate::models::WebSocketMessage;
use crate::websocket::aio::WsStream;
use crate::websocket::connection_event::emit_event;
use crate::websocket::protocol::{handle_subscribed_event, parse_binary_frame, parse_text_frame};
use crate::websocket::{ConnectionEvent, SubscriptionManager};
use futures_util::StreamExt;
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc as tokio_mpsc;
use tokio_tungstenite::tungstenite::Message;

/// Dispatch incoming WebSocket messages to appropriate channels
///
/// This task runs in the background after connect() succeeds.
/// It will terminate when:
/// 1. WebSocket connection closes (returns close code)
/// 2. Server sends Close frame (returns close code from frame)
/// 3. WebSocket error occurs (returns None)
/// 4. Message channel closes (returns None)
/// 5. Task is aborted by disconnect() (task cancelled at .await point)
///
/// The function is cancellation-safe: aborting at any `.await` point
/// will not leave resources in an inconsistent state.
///
/// # Arguments
///
/// * `ws_read` - The read half of the WebSocket stream
/// * `message_tx` - Channel to send parsed messages to consumers
/// * `event_tx` - Channel to send connection events
/// * `heartbeat_timeout` - If `Some(d)`, wrap each `ws_read.next()` in
///   `tokio::time::timeout(d, ...)` and emit
///   [`ConnectionEvent::HeartbeatTimeout`] when the timer fires. If
///   `None`, liveness detection is disabled and reads block indefinitely.
/// * `subscriptions` - Subscription manager for `subscribed` event handling
///
/// # Returns
///
/// Close code from the WebSocket close frame, or None if the connection
/// was dropped without a proper close, due to an error, or due to
/// `heartbeat_timeout` firing. The dispatch-task caller treats `None` as
/// reconnectable per `should_reconnect`'s default arm.
pub(crate) async fn dispatch_messages(
    mut ws_read: WsStream,
    message_tx: tokio_mpsc::Sender<WebSocketMessage>,
    event_tx: mpsc::SyncSender<ConnectionEvent>,
    heartbeat_timeout: Option<Duration>,
    subscriptions: Arc<SubscriptionManager>,
) -> Option<u16> {
    loop {
        // Read-site liveness: if `heartbeat_timeout` is set, the next
        // frame must arrive within that window or we declare the
        // connection dead. When None, fall back to a plain blocking
        // read (no liveness detection).
        let frame_result = match heartbeat_timeout {
            Some(timeout) => match tokio::time::timeout(timeout, ws_read.next()).await {
                Ok(opt) => opt,
                Err(_elapsed) => {
                    emit_event(&event_tx, ConnectionEvent::HeartbeatTimeout {
                        elapsed: timeout,
                    });
                    return None;
                }
            },
            None => ws_read.next().await,
        };

        let msg_result = match frame_result {
            Some(r) => r,
            None => {
                // Stream ended cleanly without close frame.
                emit_event(&event_tx, ConnectionEvent::Disconnected {
                    code: None,
                    reason: "Connection closed".to_string(),
                });
                return None;
            }
        };

        match msg_result {
            Ok(Message::Text(text)) => {
                match parse_text_frame(&text) {
                    Ok(ws_msg) => {
                        // Mutex is only taken when event == "subscribed" (cheap
                        // string compare for every other message).
                        handle_subscribed_event(&subscriptions, &ws_msg);
                        if message_tx.send(ws_msg).await.is_err() {
                            return None;
                        }
                    }
                    Err(e) => {
                        emit_event(&event_tx, ConnectionEvent::Error {
                            message: format!("Failed to deserialize message: {}", e),
                            code: 2003,
                        });
                    }
                }
            }
            Ok(Message::Binary(data)) => {
                match parse_binary_frame(&data) {
                    Ok(ws_msg) => {
                        handle_subscribed_event(&subscriptions, &ws_msg);
                        if message_tx.send(ws_msg).await.is_err() {
                            return None;
                        }
                    }
                    Err(e) => {
                        emit_event(&event_tx, ConnectionEvent::Error {
                            message: format!("Failed to deserialize binary message: {}", e),
                            code: 2003,
                        });
                    }
                }
            }
            Ok(Message::Pong(_)) => {
                // RFC 6455 control-frame pong: counted as activity by
                // virtue of resetting the read-site timeout. Fugle sends
                // pong via JSON message; this branch is defensive.
            }
            Ok(Message::Close(close_frame)) => {
                // Server initiated close - RFC 6455 compliant handling
                let code = close_frame.as_ref().map(|cf| cf.code.into());
                let reason = close_frame
                    .as_ref()
                    .map(|cf| cf.reason.to_string())
                    .unwrap_or_else(|| "Server initiated close".to_string());

                // Send disconnected event with close details
                emit_event(&event_tx, ConnectionEvent::Disconnected {
                    code,
                    reason,
                });

                return code;
            }
            Ok(Message::Ping(_)) => {
                // Server sent ping, tokio-tungstenite auto-responds with pong
                // No action needed
            }
            Err(e) => {
                // WebSocket error - connection likely broken
                emit_event(&event_tx, ConnectionEvent::Error {
                    message: format!("WebSocket error: {}", e),
                    code: 2001,
                });
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
