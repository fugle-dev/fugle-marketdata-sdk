//! Async dispatch loop: reads frames from the WS stream, parses, and pushes
//! messages onto the client's stream, and sends the health check's probe.

use crate::tracing_compat::{debug, warn};
use crate::websocket::aio::writer::WriteFailure;
use crate::websocket::aio::{SharedState, WsStream};
use crate::websocket::connection_event::{
    peer_close_disconnect, will_reconnect_after, ConnectionClose,
};
use crate::websocket::liveness::{
    probe_frame, FailWaitersOnDrop, LatencyWaiters, Liveness, LivenessAction,
};
use crate::websocket::stream_queue::StreamSender;
use crate::websocket::protocol::{
    frame_unsubscribe, handle_subscribed_event, parse_binary_frame, parse_text_frame,
};
use crate::websocket::{
    ConnectionEvent, DisconnectIntent, HealthCheckConfig, ReconnectionManager,
    SubscriptionManager,
};
use futures_util::future::BoxFuture;
use futures_util::{FutureExt, StreamExt};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc as tokio_mpsc, oneshot, Mutex};
use tokio::time::Instant;
use tokio_tungstenite::tungstenite::Message;

/// Most frames read after a failed write before it is reported. Bounds how
/// long a socket that keeps delivering data can postpone the report.
const WRITE_FAILURE_DRAIN_LIMIT: usize = 256;

/// What ended one wait of the dispatch loop.
enum Next {
    Frame(Option<Result<Message, tokio_tungstenite::tungstenite::Error>>),
    /// The liveness deadline passed; poll the liveness state again.
    Deadline,
    /// A probe held back by a full write queue has been queued.
    ProbeQueued,
    /// The writer failed; frames already received are read first.
    WriteFailed(WriteFailure),
    /// No frame is waiting behind a failed write: report it.
    ReportWriteFailure(WriteFailure),
    WriterGone,
}

/// Queue the unsubscribe frame for subscriptions unsubscribed before their
/// ack arrived. A connection without a writer is going away; its
/// subscriptions go with it.
///
/// Queued with `try_send` so the read loop keeps reading: the writer drains
/// the queue concurrently, so a free slot is the normal case. Only a full
/// queue awaits, as the caller's own `unsubscribe()` would — dropping the
/// frame would leave the subscription running on the server.
async fn send_cancels(
    write_tx: &Mutex<Option<tokio_mpsc::Sender<String>>>,
    cancels: Vec<String>,
) {
    if cancels.is_empty() {
        return;
    }
    let Ok(frame) = frame_unsubscribe(cancels) else {
        return;
    };
    let sender = write_tx.lock().await.clone();
    let Some(sender) = sender else {
        return;
    };
    if let Err(tokio_mpsc::error::TrySendError::Full(frame)) = sender.try_send(frame) {
        let _ = sender.send(frame).await;
    }
}

/// Queue a health-check probe. Returns the send still to finish when the
/// queue is full: the caller polls it alongside the read, under the same
/// deadline, so a write path that is stuck cannot hold the verdict back — a
/// probe that never gets out gets no answer and the connection is declared
/// dead on time. With no writer the probe is dropped, with the same result.
async fn send_probe(
    write_tx: &Mutex<Option<tokio_mpsc::Sender<String>>>,
) -> Option<BoxFuture<'static, ()>> {
    let sender = write_tx.lock().await.clone()?;
    match sender.try_send(probe_frame()) {
        Err(tokio_mpsc::error::TrySendError::Full(frame)) => Some(
            async move {
                let _ = sender.send(frame).await;
            }
            .boxed(),
        ),
        _ => None,
    }
}

/// Dispatch incoming WebSocket messages to appropriate channels
///
/// This task runs in the background after connect() succeeds.
/// It will terminate when:
/// 1. WebSocket connection closes (returns no close code)
/// 2. Server sends Close frame (returns the close code from the frame)
/// 3. WebSocket error occurs, reading or writing (returns no close code)
/// 4. Task is aborted by disconnect() (task cancelled at .await point)
///
/// The function is cancellation-safe: aborting at any `.await` point
/// will not leave resources in an inconsistent state.
///
/// # Arguments
///
/// * `ws_read` - The read half of the WebSocket stream
/// * `stream` - The client's stream: parsed messages and connection events
/// * `health` - Liveness detection (see [`HealthCheckConfig`]): each read is
///   bounded by the next [`Liveness`] deadline, a probe is queued on
///   `write_tx` when it is due, and [`ConnectionEvent::HeartbeatTimeout`]
///   followed by `Disconnected { intent: Network }` is emitted when the
///   connection is declared dead. Disabled, reads block indefinitely.
/// * `latency` - Pending `measure_latency()` calls: the SDK's own pongs are
///   routed here instead of the stream, and the calls left waiting fail
///   when this connection ends
/// * `subscriptions` - Subscription manager for `subscribed` event handling
/// * `write_tx` - The client's outbound queue, for unsubscribing what was
///   unsubscribed before its `subscribed` ack arrived (#136)
/// * `reconnection` - Reconnect policy, consulted for each `Disconnected`'s
///   `will_reconnect` so it matches the decision the caller makes next
/// * `state` - The client's connection state, updated to match each
///   `Disconnected` before it is queued (#86)
/// * `write_failed` - Receives this connection's failed write from its
///   writer task; it is reported and ends the connection like a transport
///   error (#97). Borrowed, so the caller can retire the writer before the
///   receiver is dropped (#105)
///
/// # Returns
///
/// How the connection ended: the close code from the WebSocket close frame
/// (`None` if the connection was dropped without a proper close, due to an
/// error, or due to the liveness check declaring the connection dead) and
/// the code of the last `error` frame it delivered, which the caller's
/// reconnect decision takes into account (#201).
#[allow(clippy::too_many_arguments)]
pub(crate) async fn dispatch_messages(
    mut ws_read: WsStream,
    stream: StreamSender,
    health: &HealthCheckConfig,
    latency: &LatencyWaiters,
    subscriptions: Arc<SubscriptionManager>,
    write_tx: Arc<Mutex<Option<tokio_mpsc::Sender<String>>>>,
    shutdown_requested: Arc<AtomicBool>,
    reconnection: Arc<Mutex<ReconnectionManager>>,
    state: SharedState,
    write_failed: &mut oneshot::Receiver<WriteFailure>,
) -> ConnectionClose {
    let _fail_waiters = FailWaitersOnDrop(latency);
    let mut liveness = Liveness::new(health, Instant::now());
    // A probe waiting for room in a full write queue.
    let mut probe_send: Option<BoxFuture<'static, ()>> = None;
    let mut write_failed = Some(write_failed);
    // A failed write waiting for the frames received before it, with the
    // number read so far.
    let mut pending_write_failure: Option<(WriteFailure, usize)> = None;
    // The code of the last `error` frame this connection delivered: `1000`
    // is the server rejecting the credentials before it closes (#201).
    let mut last_error_code: Option<i32> = None;
    let will_reconnect = |intent: DisconnectIntent, close: ConnectionClose| {
        let reconnection = Arc::clone(&reconnection);
        let shutdown_requested = shutdown_requested.load(Ordering::SeqCst);
        async move {
            let mgr = reconnection.lock().await;
            will_reconnect_after(&mgr, intent, close, shutdown_requested)
        }
    };
    // The close this loop returns when no Close frame carried a code.
    let no_code = |last_error_code: Option<i32>| ConnectionClose { code: None, last_error_code };

    loop {
        // A socket that always has data never returns `Pending`, and frames
        // decoded from tungstenite's buffer spend no coop budget, so without
        // this the loop would keep its worker until the socket drains. Tasks
        // it wakes — a `stream()` consumer on the same runtime — would
        // starve meanwhile while the queue fills and drops (#46).
        tokio::task::coop::consume_budget().await;

        // Read-site liveness: the next frame must arrive before the
        // liveness deadline. Past it, the state either calls for a probe
        // (queued here, one per silence) or declares the connection dead.
        // Disabled, the read blocks indefinitely.
        let mut deadline = None;
        if let Some(liveness) = liveness.as_mut() {
            loop {
                match liveness.poll(Instant::now()) {
                    LivenessAction::Wait(at) => {
                        deadline = Some(at);
                        break;
                    }
                    LivenessAction::SendProbe => {
                        debug!(target: "fugle_marketdata::ws", "liveness probe sent");
                        probe_send = send_probe(&write_tx).await;
                    }
                    LivenessAction::Dead(elapsed) => {
                        // A caller-initiated shutdown reports the close itself.
                        if shutdown_requested.load(Ordering::SeqCst) {
                            return no_code(last_error_code);
                        }
                        report_heartbeat_timeout(
                            &stream,
                            &state,
                            elapsed,
                            will_reconnect(DisconnectIntent::Network, no_code(last_error_code)).await,
                        );
                        return no_code(last_error_code);
                    }
                }
            }
        }

        // A failed write on this connection ends it as well (#97); every
        // branch is cancel-safe, so the losing one drops nothing. Reads are
        // polled first, so a frame that is ready wins over a write failure.
        let next = if let Some((failure, drained)) = pending_write_failure.take() {
            // Frames that arrived before the write failed come first: a
            // peer's Close among them decides the report and the reconnect,
            // not the write failure. The write failure can wake this task
            // before the I/O driver has seen them, so yield to it first.
            tokio::task::yield_now().await;
            let ready = if drained < WRITE_FAILURE_DRAIN_LIMIT {
                ws_read.next().now_or_never()
            } else {
                None
            };
            match ready {
                Some(frame) => {
                    pending_write_failure = Some((failure, drained + 1));
                    Next::Frame(frame)
                }
                None => Next::ReportWriteFailure(failure),
            }
        } else {
            tokio::select! {
                biased;
                read = async {
                    match deadline {
                        Some(at) => tokio::time::timeout_at(at, ws_read.next()).await.ok(),
                        None => Some(ws_read.next().await),
                    }
                } => match read {
                    Some(frame) => Next::Frame(frame),
                    None => Next::Deadline,
                },
                failure = async {
                    match write_failed.as_mut() {
                        Some(rx) => rx.await.ok(),
                        None => std::future::pending().await,
                    }
                } => match failure {
                    Some(failure) => Next::WriteFailed(failure),
                    None => Next::WriterGone,
                },
                () = async {
                    match probe_send.as_mut() {
                        Some(send) => send.await,
                        None => std::future::pending().await,
                    }
                } => Next::ProbeQueued,
            }
        };

        if let (Next::Frame(Some(Ok(_))), Some(liveness)) = (&next, liveness.as_mut()) {
            liveness.on_inbound(Instant::now());
        }

        let frame_result = match next {
            Next::Frame(frame) => frame,
            Next::Deadline => continue,
            Next::ProbeQueued => {
                probe_send = None;
                continue;
            }
            Next::WriteFailed(failure) => {
                write_failed = None;
                pending_write_failure = Some((failure, 0));
                continue;
            }
            Next::ReportWriteFailure(WriteFailure { error, message }) => {
                // Mirrors the transport-error arm below and the sync
                // client's write-error path: `Error`, then `Disconnected`,
                // both suppressed when shutdown was caller-initiated, and
                // both held back once the close was reported (#159).
                if shutdown_requested.load(Ordering::SeqCst) {
                    return no_code(last_error_code);
                }
                let will_reconnect =
                    will_reconnect(DisconnectIntent::Network, no_code(last_error_code)).await;
                stream.connection_failed(
                    &state,
                    ConnectionEvent::error_with_message(&error, message.clone()),
                    message,
                    will_reconnect,
                );
                return no_code(last_error_code);
            }
            Next::WriterGone => {
                // The writer stopped without a failure (its queue or sink
                // was cleared); keep reading.
                write_failed = None;
                continue;
            }
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
                    stream.connection_lost(
                        &state,
                        None,
                        "Connection closed".to_string(),
                        DisconnectIntent::Network,
                        will_reconnect(DisconnectIntent::Network, no_code(last_error_code)).await,
                    );
                }
                return no_code(last_error_code);
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
                        if latency.intercept_pong(&ws_msg, std::time::Instant::now()) {
                            continue;
                        }
                        if let Some(code) = ws_msg.error_code() {
                            last_error_code = Some(code);
                        }
                        // Mutex is only taken when event == "subscribed" (cheap
                        // string compare for every other message).
                        let cancels = handle_subscribed_event(&subscriptions, &ws_msg);
                        send_cancels(&write_tx, cancels).await;
                        stream.push_message(ws_msg);
                    }
                    Err(e) => {
                        // Not after the client's close has been reported (#159).
                        stream.emit_unless_closed(ConnectionEvent::error_with_message(
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
                        if latency.intercept_pong(&ws_msg, std::time::Instant::now()) {
                            continue;
                        }
                        if let Some(code) = ws_msg.error_code() {
                            last_error_code = Some(code);
                        }
                        let cancels = handle_subscribed_event(&subscriptions, &ws_msg);
                        send_cancels(&write_tx, cancels).await;
                        stream.push_message(ws_msg);
                    }
                    Err(e) => {
                        // Not after the client's close has been reported (#159).
                        stream.emit_unless_closed(ConnectionEvent::error_with_message(
                            &e,
                            format!("Failed to deserialize binary message: {}", e),
                        ));
                    }
                }
            }
            Ok(Message::Pong(_)) => {
                // RFC 6455 control-frame pong: counted as activity like any
                // frame. The SDK never sends control-frame pings (its probe
                // is the JSON `ping` event); this branch is defensive.
            }
            Ok(Message::Close(close_frame)) => {
                let code = close_frame.as_ref().map(|cf| cf.code.into());
                let close = ConnectionClose { code, last_error_code };
                // The flag and the emit are not atomic: `disconnect()` may
                // set the flag right after this check. The latch keeps the
                // shutdown path from reporting the same close again (#41).
                if let Some((code, reason, intent)) = peer_close_disconnect(
                    code,
                    close_frame.as_ref().map(|cf| cf.reason.to_string()),
                    shutdown_requested.load(Ordering::SeqCst),
                ) {
                    let will_reconnect = will_reconnect(intent, close).await;
                    stream.connection_lost(
                        &state,
                        code,
                        reason,
                        intent,
                        will_reconnect,
                    );
                }
                return close;
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
                // `Disconnected { intent: Client }`. Once it has, neither
                // event follows it (#159).
                if shutdown_requested.load(Ordering::SeqCst) {
                    return no_code(last_error_code);
                }
                let err_msg = format!("WebSocket error: {}", e);
                let will_reconnect =
                    will_reconnect(DisconnectIntent::Network, no_code(last_error_code)).await;
                stream.connection_failed(
                    &state,
                    ConnectionEvent::error_with_message(
                        &crate::MarketDataError::from(e),
                        err_msg.clone(),
                    ),
                    err_msg,
                    will_reconnect,
                );
                return no_code(last_error_code);
            }
            Ok(Message::Frame(_)) => {
                // Raw frames shouldn't appear in normal usage
            }
        }
    }
}

/// Report a connection declared dead by the liveness check:
/// `HeartbeatTimeout`, then `Disconnected { intent: Network }`.
fn report_heartbeat_timeout(
    stream: &StreamSender,
    state: &SharedState,
    elapsed: Duration,
    will_reconnect: bool,
) {
    let elapsed_ms = elapsed.as_millis() as u64;
    warn!(
        target: "fugle_marketdata::ws",
        elapsed_ms,
        "heartbeat timeout: no inbound frame in window"
    );
    // Through the latch, so a racing `disconnect()` cannot report this
    // connection's close a second time (#47), and under one lock, so a
    // `disconnect()` that reported the close first is not followed by the
    // timeout (#159).
    stream.connection_failed(
        state,
        ConnectionEvent::HeartbeatTimeout { elapsed },
        format!("Heartbeat timeout after {elapsed_ms}ms"),
        will_reconnect,
    );
}
