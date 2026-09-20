//! Owner-thread + reconnect supervisor for the sync WebSocket client.
//!
//! Single thread per connected client: owns the `tungstenite::WebSocket`,
//! drains the outbound write queue, and parses inbound frames.
//! On disconnect, optionally runs the reconnect loop and rebuilds the
//! WebSocket+queue+state in place.

use crate::websocket::connection_event::{
    peer_close_disconnect, will_reconnect_after, ConnectionClose,
};
use crate::websocket::liveness::{
    probe_frame, FailWaitersOnDrop, LatencyWaiters, Liveness, LivenessAction,
};
use crate::websocket::stream_queue::{rejected_reason, StreamSender, MAX_ATTEMPTS_REASON};
use crate::websocket::protocol::{
    classify_auth_response, frame_auth, frame_resubscribe, parse_binary_frame, parse_text_frame,
    AuthHandshake, AuthOutcome, ResubscribeFrame,
};
use crate::websocket::{
    ConnectionConfig, ConnectionEvent, ConnectionState, DisconnectIntent, HealthCheckConfig,
    ReconnectionManager, SubscriptionManager,
};
use crate::models::WebSocketMessage;
use crate::MarketDataError;
use crate::tracing_compat::{debug, warn};
use std::collections::VecDeque;
use std::io::ErrorKind;
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{Connector, Message, WebSocket};

/// Short read-timeout used as the polling interval on the owner thread.
/// The owner cycles between blocking reads (bounded by this duration) and
/// draining the outbound write queue. Keeping it small (200ms) caps the
/// worst-case outbound write latency under low inbound traffic.
const READ_POLL_INTERVAL: Duration = Duration::from_millis(200);

/// Write timeout of the socket while the health check's probe is enabled.
///
/// Writes here block the owner thread, so a stuck socket would also stop
/// the liveness check that is meant to notice it. The value is fixed rather
/// than tied to `probe_timeout`, so a short probe timeout never fails an
/// ordinary write such as a large batch of subscriptions; the probe's own
/// write is bounded by its deadline instead (see [`write_probe`]).
const PROBE_MODE_WRITE_TIMEOUT: Duration = Duration::from_secs(5);

/// Outbound queue capacity. Matches the async path (`aio::writer` uses 64).
pub(crate) const WRITE_QUEUE_CAPACITY: usize = 64;

/// Bounded wait for the server's Close acknowledgement after the local
/// Close frame is sent. Caps the worst-case latency of `disconnect()` /
/// `shutdown_with_timeout()` on the sync side; the supervisor thread
/// returns even if the peer never replies.
const CLOSE_ACK_DEADLINE: Duration = Duration::from_secs(2);

pub(crate) type SyncWs = WebSocket<MaybeTlsStream<TcpStream>>;

/// Drain pending outbound JSON frames from `write_rx` synchronously.
/// Best-effort: write failures are ignored (the connection is shutting
/// down anyway). Returns when the queue is empty or disconnected.
fn drain_write_queue(ws: &mut SyncWs, write_rx: &mpsc::Receiver<String>) {
    while let Ok(json) = write_rx.try_recv() {
        if ws.send(Message::Text(json.into())).is_err() {
            return;
        }
    }
}

/// Bounded wait for the peer's Close acknowledgement.
///
/// After we issue our Close frame, RFC 6455 requires reading until the
/// peer also closes (manifesting as `Message::Close` or
/// `tungstenite::Error::ConnectionClosed`/`AlreadyClosed`). This loop
/// short-circuits within `deadline` so a wedged peer cannot block
/// `disconnect()` indefinitely.
fn await_close_ack(ws: &mut SyncWs, deadline: Duration) {
    let stop_at = Instant::now() + deadline;
    set_read_timeout(ws, Some(Duration::from_millis(50)));
    while Instant::now() < stop_at {
        match ws.read() {
            Ok(Message::Close(_)) => return,
            Err(tungstenite::Error::ConnectionClosed)
            | Err(tungstenite::Error::AlreadyClosed) => return,
            Err(tungstenite::Error::Io(e))
                if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut =>
            {
                continue;
            }
            // Any other variant — peer already gone; stop reading.
            Err(_) => return,
            // Drain remaining frames quietly during shutdown.
            Ok(_) => continue,
        }
    }
}

/// Shared state owned by the `WebSocketClient` and updated by the owner thread.
pub(crate) struct OwnerShared {
    pub config: ConnectionConfig,
    pub tls_config: Arc<rustls::ClientConfig>,
    pub health: HealthCheckConfig,
    /// Pending `measure_latency()` calls, answered by the owner loop.
    pub latency: LatencyWaiters,
    pub reconnection: Mutex<ReconnectionManager>,
    pub state: Arc<RwLock<ConnectionState>>,
    pub subscriptions: Arc<SubscriptionManager>,
    /// The client's ordered stream of messages and events.
    pub stream: StreamSender,
    /// Current outbound sender. Replaced on every reconnect so live `subscribe`
    /// callers pick up the new channel via `.lock().clone()`.
    pub write_tx_slot: Mutex<Option<mpsc::SyncSender<String>>>,
    pub should_stop: Arc<AtomicBool>,
    /// Set by `force_close()` before `should_stop`: the owner loop then
    /// drops the socket without draining writes or sending Close, like the
    /// async client's abort.
    pub abort: AtomicBool,
    /// Drop counter for the inbound message queue (drop-newest backpressure).
    /// Exposed via `WebSocketClient::messages_dropped_total`. Mirrors to
    /// `metrics_compat::COUNTER_MESSAGES_DROPPED` when the `metrics` feature
    /// is enabled.
    pub messages_dropped: crate::metrics_compat::DropCounter,
    /// Drop counter for the stream's event allowance (drop-newest backpressure).
    /// Exposed via `WebSocketClient::events_dropped_total`. Mirrors to
    /// `metrics_compat::COUNTER_EVENTS_DROPPED` when the `metrics` feature
    /// is enabled.
    pub events_dropped: crate::metrics_compat::DropCounter,
}

/// Build a fresh TLS-wrapped WebSocket via `tungstenite::client_tls_with_config`.
///
/// Transport failures — DNS, TCP, TLS and the HTTP upgrade — are reported
/// through `From<tungstenite::Error>` like the async client's, so both
/// clients give a refused connection the same `WebSocketError` (code 3002,
/// kind `Io`), a rejected upgrade the same `Http(status)` and a connect
/// that runs out of `connect_timeout` the same `TimeoutError` (#201). Only
/// a URL the client cannot take apart is a `ConnectionError`.
pub(crate) fn do_blocking_connect(
    config: &ConnectionConfig,
    tls_config: Arc<rustls::ClientConfig>,
) -> Result<SyncWs, MarketDataError> {
    use std::net::ToSocketAddrs;

    let url: url::Url = config.url.parse().map_err(|e: url::ParseError| {
        MarketDataError::ConnectionError {
            msg: format!("Invalid URL: {e}"),
        }
    })?;
    let host = url.host_str().ok_or_else(|| MarketDataError::ConnectionError {
        msg: "URL missing host".to_string(),
    })?;
    let port = url.port_or_known_default().ok_or_else(|| {
        MarketDataError::ConnectionError {
            msg: "URL missing port".to_string(),
        }
    })?;

    // Reported as tungstenite's `Io` error, like the async client's.
    let io_error = |e: std::io::Error, what: &str| {
        tungstenite::Error::Io(std::io::Error::new(e.kind(), format!("{what}: {e}")))
    };
    let addrs: Vec<_> = (host, port)
        .to_socket_addrs()
        .map_err(|e| io_error(e, "DNS lookup failed"))?
        .collect();
    if addrs.is_empty() {
        return Err(tungstenite::Error::Io(std::io::Error::new(
            ErrorKind::NotFound,
            "DNS returned no addresses",
        ))
        .into());
    }

    // A TCP connect that runs out of `connect_timeout` is a `TimeoutError`,
    // as on the async client (whose timeout also covers TLS and the upgrade).
    let tcp = TcpStream::connect_timeout(&addrs[0], config.connect_timeout).map_err(|e| {
        if e.kind() == ErrorKind::TimedOut {
            MarketDataError::TimeoutError {
                operation: "WebSocket connect".to_string(),
            }
        } else {
            io_error(e, "TCP connect failed").into()
        }
    })?;
    tcp.set_nodelay(true).ok();

    let connector = Connector::Rustls(tls_config);
    let (ws, _resp) = tungstenite::client_tls_with_config(
        config.url.as_str(),
        tcp,
        None,
        Some(connector),
    )
    .map_err(|e| match e {
        tungstenite::HandshakeError::Failure(e) => e,
        // A blocking socket never leaves the handshake mid-way.
        tungstenite::HandshakeError::Interrupted(_) => tungstenite::Error::Io(
            std::io::Error::new(ErrorKind::WouldBlock, "WebSocket handshake interrupted"),
        ),
    })?;

    Ok(ws)
}

/// Apply `set_read_timeout` to the underlying `TcpStream`, going through the
/// `MaybeTlsStream::Rustls` wrapper. Without the explicit downcast,
/// `set_read_timeout` is unreachable through the WebSocket facade.
pub(crate) fn set_read_timeout(ws: &mut SyncWs, t: Option<Duration>) {
    if let Some(tcp) = tcp_stream(ws) {
        let _ = tcp.set_read_timeout(t);
    }
}

/// Apply `set_write_timeout` to the underlying `TcpStream`; see
/// [`set_read_timeout`].
fn set_write_timeout(ws: &mut SyncWs, t: Option<Duration>) {
    if let Some(tcp) = tcp_stream(ws) {
        let _ = tcp.set_write_timeout(t);
    }
}

fn tcp_stream(ws: &mut SyncWs) -> Option<&mut TcpStream> {
    match ws.get_mut() {
        MaybeTlsStream::Plain(s) => Some(s),
        MaybeTlsStream::Rustls(s) => Some(&mut s.sock),
        _ => {
            // NativeTls variant not enabled by our feature flags. The catch-all
            // here is intentional but should be made exhaustive if we add
            // native-tls support in the future.
            None
        }
    }
}

/// Any inbound frame proves the connection alive.
fn on_inbound(liveness: &mut Option<Liveness<Instant>>) {
    if let Some(liveness) = liveness.as_mut() {
        liveness.on_inbound(Instant::now());
    }
}

/// What became of a probe written by the owner thread.
enum ProbeWrite {
    Sent,
    /// It could not be written before its deadline: no answer can come.
    TimedOut,
    Failed(tungstenite::Error),
}

/// Write a probe, bounded by `deadline` so a stuck socket cannot hold the
/// verdict back, then restore the connection's write timeout.
fn write_probe(ws: &mut SyncWs, deadline: Instant) -> ProbeWrite {
    let remaining = deadline
        .saturating_duration_since(Instant::now())
        .max(Duration::from_millis(1));
    set_write_timeout(ws, Some(remaining));
    let result = ws.send(Message::Text(probe_frame().into()));
    set_write_timeout(ws, Some(PROBE_MODE_WRITE_TIMEOUT));
    match result {
        Ok(()) => ProbeWrite::Sent,
        Err(tungstenite::Error::Io(e))
            if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut =>
        {
            ProbeWrite::TimedOut
        }
        Err(e) => ProbeWrite::Failed(e),
    }
}

/// Run the full auth handshake on a freshly-connected WebSocket. The text
/// frames read are returned with the outcome, to be queued after the
/// matching event (#68).
pub(crate) fn do_auth_handshake(
    ws: &mut SyncWs,
    config: &ConnectionConfig,
    stream: &StreamSender,
) -> AuthHandshake {
    // The drop count restarts with each connection attempt.
    stream.start_connection();
    let mut frames = Vec::new();
    // Send auth frame
    let auth_json = match frame_auth(config.auth.clone()) {
        Ok(json) => json,
        Err(e) => return AuthHandshake::Failed(e),
    };
    if let Err(e) = ws.send(Message::Text(auth_json.into())) {
        return AuthHandshake::Failed(MarketDataError::ConnectionError {
            msg: format!("Failed to send auth frame: {e}"),
        });
    }

    // Read auth response with overall wall-clock timeout
    // (`ConnectionConfig::auth_timeout`, shared with the async client).
    set_read_timeout(ws, Some(config.auth_timeout));
    let deadline = Instant::now() + config.auth_timeout;
    loop {
        if Instant::now() >= deadline {
            return AuthHandshake::Failed(MarketDataError::TimeoutError {
                operation: "WebSocket authentication".to_string(),
            });
        }

        match ws.read() {
            Ok(Message::Text(text)) => {
                let parsed = parse_text_frame(&text);
                if let Ok(ws_msg) = parsed {
                    let outcome = classify_auth_response(&ws_msg);
                    frames.push(ws_msg);
                    match outcome {
                        AuthOutcome::Authenticated(data) => {
                            return AuthHandshake::Authenticated { data, frames };
                        }
                        AuthOutcome::Rejected { message, data } => {
                            return AuthHandshake::Rejected { message, data, frames };
                        }
                        AuthOutcome::Failed(e) => return AuthHandshake::Failed(e),
                        AuthOutcome::Pending => continue,
                    }
                }
            }
            Ok(Message::Close(_)) => {
                return AuthHandshake::Failed(MarketDataError::ConnectionError {
                    msg: "Stream closed during authentication".to_string(),
                });
            }
            Ok(_) => continue,
            Err(tungstenite::Error::Io(e))
                if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut =>
            {
                // Timed-out poll. Loop to re-check overall deadline.
                continue;
            }
            Err(e) => {
                return AuthHandshake::Failed(MarketDataError::ConnectionError {
                    msg: format!("Auth read error: {e}"),
                });
            }
        }
    }
}

/// The `will_reconnect` for a close observed by the owner thread.
fn will_reconnect(shared: &OwnerShared, intent: DisconnectIntent, close: ConnectionClose) -> bool {
    let mgr = shared.reconnection.lock().expect("reconnection lock poisoned");
    will_reconnect_after(&mgr, intent, close, shared.should_stop.load(Ordering::SeqCst))
}

/// The owner loop. Reads frames + drains outbound queue + emits events.
/// Returns how the connection ended: the close code observed (`Some` on a
/// clean close, `None` on error/heartbeat-timeout/stream-end) and the code
/// of the last `error` frame it delivered, which the supervisor's reconnect
/// decision takes into account (#201). On `should_stop` returns code
/// `Some(1000)` after sending a close frame, or `None` without one when
/// `abort` is also set.
fn owner_loop(
    mut ws: SyncWs,
    write_rx: mpsc::Receiver<String>,
    shared: &OwnerShared,
) -> ConnectionClose {
    set_read_timeout(&mut ws, Some(READ_POLL_INTERVAL));
    let _fail_waiters = FailWaitersOnDrop(&shared.latency);
    let mut liveness = Liveness::new(&shared.health, Instant::now());
    if liveness.as_ref().is_some_and(Liveness::probes) {
        set_write_timeout(&mut ws, Some(PROBE_MODE_WRITE_TIMEOUT));
    }
    // Unsubscribe frames for subscriptions unsubscribed before their ack
    // arrived (#136); written ahead of the outbound queue.
    let mut cancel_frames: VecDeque<String> = VecDeque::new();
    // The code of the last `error` frame this connection delivered: `1000`
    // is the server rejecting the credentials before it closes (#201).
    let mut last_error_code: Option<i32> = None;
    // The close this loop returns when no Close frame carried a code.
    let no_code = |last_error_code: Option<i32>| ConnectionClose { code: None, last_error_code };

    loop {
        if shared.should_stop.load(Ordering::SeqCst) {
            // `force_close()`: dropping `ws` closes the TCP socket as is.
            if shared.abort.load(Ordering::SeqCst) {
                return no_code(last_error_code);
            }
            // Graceful shutdown sequence:
            //   a) drain any queued writes so subscribe/unsubscribe acks
            //      that the caller already enqueued reach the wire,
            //   b) send the WebSocket Close frame,
            //   c) bounded read until the server's Close ack arrives so
            //      the peer can flush its own pending acks before TCP
            //      teardown (RFC 6455 close handshake).
            drain_write_queue(&mut ws, &write_rx);
            let _ = ws.close(None);
            let _ = ws.flush();
            await_close_ack(&mut ws, CLOSE_ACK_DEADLINE);
            return ConnectionClose { code: Some(1000), last_error_code };
        }

        // 1. Try a bounded read
        match ws.read() {
            Ok(Message::Text(text)) => {
                on_inbound(&mut liveness);
                debug!(
                    target: "fugle_marketdata::ws",
                    bytes = text.len(),
                    kind = "text",
                    "ws frame received"
                );
                match parse_text_frame(&text) {
                    Ok(ws_msg) if shared.latency.intercept_pong(&ws_msg, Instant::now()) => {}
                    Ok(ws_msg) => {
                        if let Some(code) = ws_msg.error_code() {
                            last_error_code = Some(code);
                        }
                        queue_cancels(
                            &mut cancel_frames,
                            crate::websocket::protocol::handle_subscribed_event(
                                &shared.subscriptions,
                                &ws_msg,
                            ),
                        );
                        shared.stream.push_message(ws_msg);
                    }
                    Err(e) => {
                        // Not after the client's close has been reported (#159).
                        shared.stream.emit_unless_closed(ConnectionEvent::error_with_message(
                            &e,
                            format!("Failed to deserialize message: {e}"),
                        ));
                    }
                }
            }
            Ok(Message::Binary(data)) => {
                on_inbound(&mut liveness);
                debug!(
                    target: "fugle_marketdata::ws",
                    bytes = data.len(),
                    kind = "binary",
                    "ws frame received"
                );
                match parse_binary_frame(&data) {
                    Ok(ws_msg) if shared.latency.intercept_pong(&ws_msg, Instant::now()) => {}
                    Ok(ws_msg) => {
                        if let Some(code) = ws_msg.error_code() {
                            last_error_code = Some(code);
                        }
                        queue_cancels(
                            &mut cancel_frames,
                            crate::websocket::protocol::handle_subscribed_event(
                                &shared.subscriptions,
                                &ws_msg,
                            ),
                        );
                        shared.stream.push_message(ws_msg);
                    }
                    Err(e) => {
                        // Not after the client's close has been reported (#159).
                        shared.stream.emit_unless_closed(ConnectionEvent::error_with_message(
                            &e,
                            format!("Failed to deserialize binary message: {e}"),
                        ));
                    }
                }
            }
            Ok(Message::Ping(payload)) => {
                on_inbound(&mut liveness);
                // tungstenite does not auto-pong in blocking mode — respond manually.
                let _ = ws.send(Message::Pong(payload));
            }
            Ok(Message::Pong(_)) => {
                on_inbound(&mut liveness);
            }
            Ok(Message::Close(frame)) => {
                let code = frame.as_ref().map(|cf| u16::from(cf.code));
                let close = ConnectionClose { code, last_error_code };
                // `should_stop` is checked before the read, so a caller's
                // `disconnect()` can land while this read is blocked; the
                // shutdown path then reports the close as `Client` (#22).
                // The check and the emit are not atomic, so the latch keeps
                // both sides from reporting it (#41).
                if let Some((code, reason, intent)) = peer_close_disconnect(
                    code,
                    frame.as_ref().map(|cf| cf.reason.to_string()),
                    shared.should_stop.load(Ordering::SeqCst),
                ) {
                    shared.stream.connection_lost(
                        &shared.state,
                        code,
                        reason,
                        intent,
                        will_reconnect(shared, intent, close),
                    );
                }
                // Unlike the graceful path at the loop top, there is no
                // write-queue drain here even when `should_stop` is set: the
                // peer closed first, so tungstenite is in `ClosedByPeer` and
                // rejects further data frames with `SendAfterClosing`.
                // `close` only flushes the Close reply it already queued.
                let _ = ws.close(None);
                return close;
            }
            Ok(Message::Frame(_)) => {
                on_inbound(&mut liveness);
            }
            Err(tungstenite::Error::Io(e))
                if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut =>
            {
                // Polling interval elapsed — fall through to write drain + heartbeat check
            }
            Err(tungstenite::Error::ConnectionClosed)
            | Err(tungstenite::Error::AlreadyClosed) => {
                // Suppress when caller initiated shutdown — the
                // shutdown path emits the canonical
                // `Disconnected { intent: Client }` itself, mirroring
                // the async `dispatch.rs` short-circuit.
                if !shared.should_stop.load(Ordering::SeqCst) {
                    shared.stream.connection_lost(
                        &shared.state,
                        None,
                        "Connection closed".to_string(),
                        DisconnectIntent::Network,
                        will_reconnect(shared, DisconnectIntent::Network, no_code(last_error_code)),
                    );
                }
                return no_code(last_error_code);
            }
            Err(e) => {
                // WebSocket transport error — emit both `Error`
                // (preserves diagnostic surface) and
                // `Disconnected { intent: Network }` so consumers
                // pattern-matching on `ConnectionEvent::Disconnected`
                // observe abnormal closes the same way they observe
                // clean closes. Mirrors the async `dispatch.rs` Err
                // arm. Suppressed when caller initiated shutdown, and
                // held back once the close was reported (#159).
                if shared.should_stop.load(Ordering::SeqCst) {
                    return no_code(last_error_code);
                }
                let err_msg = format!("WebSocket read error: {e}");
                shared.stream.connection_failed(
                    &shared.state,
                    ConnectionEvent::error_with_message(&MarketDataError::from(e), err_msg.clone()),
                    err_msg,
                    will_reconnect(shared, DisconnectIntent::Network, no_code(last_error_code)),
                );
                return no_code(last_error_code);
            }
        }

        // 2. Liveness check: send the probe when it is due, or declare the
        // connection dead. A caller-initiated shutdown reports the close
        // itself, and a `force_close()` sends nothing further.
        if let Some(liveness) = liveness.as_mut() {
            loop {
                match liveness.poll(Instant::now()) {
                    LivenessAction::Wait(_) => break,
                    LivenessAction::SendProbe => {
                        if shared.abort.load(Ordering::SeqCst) {
                            return no_code(last_error_code);
                        }
                        let deadline =
                            liveness.probe_deadline().expect("a probe was just sent");
                        debug!(target: "fugle_marketdata::ws", "liveness probe sent");
                        match write_probe(&mut ws, deadline) {
                            ProbeWrite::Sent => {}
                            ProbeWrite::TimedOut => {
                                if shared.should_stop.load(Ordering::SeqCst) {
                                    return no_code(last_error_code);
                                }
                                report_heartbeat_timeout(shared, liveness.window(), last_error_code);
                                return no_code(last_error_code);
                            }
                            ProbeWrite::Failed(e) => {
                                report_write_error(shared, e, last_error_code);
                                return no_code(last_error_code);
                            }
                        }
                    }
                    LivenessAction::Dead(elapsed) => {
                        if shared.should_stop.load(Ordering::SeqCst) {
                            return no_code(last_error_code);
                        }
                        report_heartbeat_timeout(shared, elapsed, last_error_code);
                        return no_code(last_error_code);
                    }
                }
            }
        }

        // 3. Drain outbound queue (non-blocking). A `force_close()` that
        // landed during the read sends nothing further, not even the Close
        // of the `Disconnected` arm below.
        if shared.abort.load(Ordering::SeqCst) {
            return no_code(last_error_code);
        }
        loop {
            let json = match cancel_frames.pop_front() {
                Some(json) => json,
                None => match write_rx.try_recv() {
                    Ok(json) => json,
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        // Client dropped its sender — typically a disconnect()
                        // signal. Send close frame and exit.
                        let _ = ws.close(None);
                        return ConnectionClose { code: Some(1000), last_error_code };
                    }
                },
            };
            if let Err(e) = ws.send(Message::Text(json.into())) {
                report_write_error(shared, e, last_error_code);
                return no_code(last_error_code);
            }
        }
    }
}

/// Report a connection declared dead by the liveness check:
/// `HeartbeatTimeout`, then `Disconnected { intent: Network }`.
fn report_heartbeat_timeout(shared: &OwnerShared, elapsed: Duration, last_error_code: Option<i32>) {
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
    shared.stream.connection_failed(
        &shared.state,
        ConnectionEvent::HeartbeatTimeout { elapsed },
        format!("Heartbeat timeout after {elapsed_ms}ms"),
        will_reconnect(
            shared,
            DisconnectIntent::Network,
            ConnectionClose { code: None, last_error_code },
        ),
    );
}

/// A failed write ends the connection just like a failed read: report
/// `Error`, then `Disconnected`, unless the caller is shutting down or
/// has reported the close already (#159).
fn report_write_error(shared: &OwnerShared, e: tungstenite::Error, last_error_code: Option<i32>) {
    if shared.should_stop.load(Ordering::SeqCst) {
        return;
    }
    let err_msg = format!("WebSocket write error: {e}");
    shared.stream.connection_failed(
        &shared.state,
        ConnectionEvent::error_with_message(&MarketDataError::from(e), err_msg.clone()),
        err_msg,
        will_reconnect(
            shared,
            DisconnectIntent::Network,
            ConnectionClose { code: None, last_error_code },
        ),
    );
}

/// Queue each of `frames` (see [`frame_resubscribe`]), in order. A frame
/// that could not be built or queued is reported as an `Error` naming its
/// label, and the rest are still sent. Returns the first failure.
pub(crate) fn replay_subscriptions(
    frames: Vec<ResubscribeFrame>,
    stream: &StreamSender,
    write_tx: &mpsc::SyncSender<String>,
) -> Result<(), MarketDataError> {
    report_resubscribe_failures(stream, queue_subscriptions(frames, write_tx))
}

/// A resubscribe frame that could not be built or queued: its label and
/// why, for [`report_resubscribe_failures`].
type ResubscribeFailure = (String, MarketDataError);

/// The queueing half of [`replay_subscriptions`]: queue each of `frames`,
/// in order, skipping the ones that cannot be, and return those failures
/// unreported.
fn queue_subscriptions(
    frames: Vec<ResubscribeFrame>,
    write_tx: &mpsc::SyncSender<String>,
) -> Vec<ResubscribeFailure> {
    let mut failures = Vec::new();
    for ResubscribeFrame { label, frame } in frames {
        let sent = frame.and_then(|json| {
            write_tx.send(json).map_err(|_| MarketDataError::ConnectionError {
                msg: "Writer queue closed (supervisor exited)".to_string(),
            })
        });
        if let Err(e) = sent {
            failures.push((label, e));
        }
    }
    failures
}

/// The reporting half of [`replay_subscriptions`]: an `Error` per failure,
/// in order, unless the client's close has been reported (#145). Returns
/// the first failure.
fn report_resubscribe_failures(
    stream: &StreamSender,
    failures: Vec<ResubscribeFailure>,
) -> Result<(), MarketDataError> {
    let mut first_err = None;
    for (label, e) in failures {
        stream.emit_unless_closed(ConnectionEvent::resubscribe_failed(&label, &e));
        first_err.get_or_insert(e);
    }
    first_err.map_or(Ok(()), Err)
}

/// Run the auth handshake on a freshly-reconnected stream and emit lifecycle events.
///
/// If `should_stop` is set while connecting, or the client's close has been
/// reported (#145), the new connection is dropped without emitting further
/// events and `ClientClosed` is returned. Any other error leaves the state
/// `Disconnected`, as on the async client: a transport or handshake failure
/// is the caller's to report (#200); rejected credentials are reported here
/// as `Unauthenticated` and returned as `AuthError`, on which the caller
/// stops (#201).
fn reconnect_and_authenticate(
    shared: &Arc<OwnerShared>,
) -> Result<(SyncWs, mpsc::Receiver<String>), MarketDataError> {
    let stopping = || shared.should_stop.load(Ordering::SeqCst);
    // A failed attempt leaves the state `Disconnected` until the next
    // `Reconnecting`, unless the client's close has been reported (#145).
    let failed = |e: MarketDataError| {
        if !shared.stream.reconnect_step(&shared.state, ConnectionState::Disconnected, None) {
            return MarketDataError::ClientClosed;
        }
        e
    };

    // Each step is reported only if the client's close has not been (#145).
    if !shared.stream.reconnect_step(
        &shared.state,
        ConnectionState::Connecting,
        Some(ConnectionEvent::Connecting {}),
    ) {
        return Err(MarketDataError::ClientClosed);
    }

    let mut ws = do_blocking_connect(&shared.config, Arc::clone(&shared.tls_config)).map_err(failed)?;
    if stopping() {
        return Err(MarketDataError::ClientClosed);
    }
    crate::tracing_compat::info!(target: "fugle_marketdata::ws", "ws reconnected");
    if !shared.stream.reconnect_step(
        &shared.state,
        ConnectionState::Authenticating,
        Some(ConnectionEvent::Connected {}),
    ) {
        return Err(MarketDataError::ClientClosed);
    }
    let handshake = do_auth_handshake(&mut ws, &shared.config, &shared.stream);
    if stopping() {
        return Err(MarketDataError::ClientClosed);
    }
    let (data, frames) = match handshake {
        AuthHandshake::Authenticated { data, frames } => (data, frames),
        AuthHandshake::Rejected { message, data, frames } => {
            if !shared.stream.reconnect_step(&shared.state, ConnectionState::Disconnected, None)
                || !shared.stream.reconnect_rejected(message.clone(), data, frames)
            {
                return Err(MarketDataError::ClientClosed);
            }
            return Err(MarketDataError::AuthError { msg: message, http: None });
        }
        AuthHandshake::Failed(e) => return Err(failed(e)),
    };

    // Before the replay is read: the old ids are stale, and a cancel whose
    // key this leaves unsubscribed can be dropped with them (#136).
    shared.subscriptions.clear_server_ids();
    let resubscribe = frame_resubscribe(shared.subscriptions.get_all());
    let write_rx = open_reconnected(shared, data, frames, resubscribe)?;
    crate::tracing_compat::info!(target: "fugle_marketdata::ws", "ws re-authenticated");
    Ok((ws, write_rx))
}

/// Install the write channel of a connection whose auth handshake just
/// succeeded, report it `Authenticated` and replay `resubscribe` on it.
/// Returns the channel's receiver for the owner loop to drain, or
/// `ClientClosed` if the client's close was reported meanwhile (#145).
///
/// Event order (#174, the `connection_event` module's guarantee 2): the
/// `Error` for a frame that could not be replayed follows this connection's
/// `Authenticated` and reads the state `Connected`, as on the async client
/// and on this client's own `reconnect()` (which replays after `connect()`
/// returns), so it is not mistaken for a failure of the handshake.
///
/// The frames are still *queued* before `Authenticated` is reported; only
/// their failures are reported after it. Moving the whole replay after
/// `reconnect_authenticated` would look simpler but can deadlock: the write
/// slot is installed before `Authenticated` (a consumer must be able to
/// `subscribe()` as soon as it sees the event), `enqueue_write` blocks on a
/// full channel, and this thread — the only one draining it — does not start
/// until this function returns. A consumer filling the channel between the
/// event and the replay would block the replay's `send` here, forever.
/// Queueing first keeps the existing guarantee that the channel holds every
/// replay frame (`WRITE_QUEUE_CAPACITY + resubscribe.len()`) before any
/// consumer can see the connection. Installing the slot after the replay
/// instead would make a `subscribe()` right after `Authenticated` fail with
/// "Not connected" in the state `Connected`: a regression of what the event
/// promises.
fn open_reconnected(
    shared: &OwnerShared,
    data: serde_json::Value,
    frames: Vec<WebSocketMessage>,
    resubscribe: Vec<ResubscribeFrame>,
) -> Result<mpsc::Receiver<String>, MarketDataError> {
    // Build fresh write channel + install into shared slot. The replay
    // queues every resubscribe frame before this thread starts draining, so
    // the channel must hold them all or `send` would block forever.
    let (write_tx, write_rx) = mpsc::sync_channel::<String>(WRITE_QUEUE_CAPACITY + resubscribe.len());
    *shared.write_tx_slot.lock().expect("write_tx_slot lock poisoned") = Some(write_tx.clone());

    // Reset reconnection counter
    {
        let mut mgr = shared.reconnection.lock().expect("reconnection lock poisoned");
        mgr.reset();
    }

    let failures = queue_subscriptions(resubscribe, &write_tx);

    // Before this thread reads the new connection; it may report its own
    // close. A close reported meanwhile keeps it from reopening (#145).
    if !shared.stream.reconnect_authenticated(&shared.state, data, frames) {
        return Err(MarketDataError::ClientClosed);
    }
    let _ = report_resubscribe_failures(&shared.stream, failures);
    Ok(write_rx)
}

/// Queue the unsubscribe frame for `cancels`, the ids of subscriptions
/// unsubscribed before their ack arrived.
fn queue_cancels(cancel_frames: &mut VecDeque<String>, cancels: Vec<String>) {
    if cancels.is_empty() {
        return;
    }
    if let Ok(frame) = crate::websocket::protocol::frame_unsubscribe(cancels) {
        cancel_frames.push_back(frame);
    }
}

/// Entry point for the owner+supervisor thread.
///
/// Runs the owner loop. On disconnect, consults the reconnect policy and
/// either rebuilds the WebSocket+queue+state in place or exits cleanly.
pub(crate) fn run_supervisor(
    initial_ws: SyncWs,
    initial_write_rx: mpsc::Receiver<String>,
    shared: Arc<OwnerShared>,
) {
    let mut connection = Some((initial_ws, initial_write_rx));

    loop {
        let (ws, write_rx) = match connection.take() {
            Some(c) => c,
            None => return,
        };

        let close = owner_loop(ws, write_rx, &shared);

        if shared.should_stop.load(Ordering::SeqCst) {
            // A `Closed` recorded with this connection's `Disconnected` (a
            // server Close racing the stop) keeps agreeing with it (#93).
            // Otherwise `reconnect()` relies on finding `Closed { Client }`.
            let mut st = shared.state.write().expect("state lock poisoned");
            if !matches!(*st, ConnectionState::Closed { .. }) {
                *st = ConnectionState::Closed {
                    code: Some(1000),
                    reason: "Client disconnected".to_string(),
                    intent: DisconnectIntent::Client,
                };
            }
            return;
        }

        let should_reconnect = {
            let mgr = shared.reconnection.lock().expect("reconnection lock poisoned");
            mgr.should_reconnect(close.code, close.last_error_code)
        };
        if !should_reconnect {
            // Already reported as `Disconnected { will_reconnect: false }`,
            // which recorded the matching `Closed` state (#86); no attempt
            // was made, so there is no `ReconnectFailed`.
            return;
        }

        // Backoff + reconnect inner loop
        let new_conn = loop {
            if shared.should_stop.load(Ordering::SeqCst) {
                return;
            }

            let delay = {
                let mut mgr = shared.reconnection.lock().expect("reconnection lock poisoned");
                mgr.next_delay()
            };
            let Some(d) = delay else {
                let attempts = {
                    let mgr = shared.reconnection.lock().expect("reconnection lock poisoned");
                    mgr.current_attempt()
                };
                // Unless a racing `disconnect()` reported the close first.
                shared.stream.reconnect_failed(
                    &shared.state,
                    close.code,
                    attempts,
                    MAX_ATTEMPTS_REASON.to_string(),
                    DisconnectIntent::Network,
                );
                return;
            };

            let attempt = {
                let mgr = shared.reconnection.lock().expect("reconnection lock poisoned");
                mgr.current_attempt()
            };
            // A close reported since the check above ends the loop here,
            // so nothing follows the final event (#145).
            if !shared.stream.reconnect_step(
                &shared.state,
                ConnectionState::Reconnecting { attempt },
                Some(ConnectionEvent::Reconnecting { attempt }),
            ) {
                return;
            }
            warn!(
                target: "fugle_marketdata::ws",
                attempt,
                delay_ms = d.as_millis() as u64,
                "ws reconnect attempt"
            );

            std::thread::sleep(d);
            // `disconnect()` during the backoff: it reports the final
            // `Disconnected` itself (#98); nothing further from here.
            if shared.should_stop.load(Ordering::SeqCst) {
                return;
            }

            match reconnect_and_authenticate(&shared) {
                Ok(pair) => break Some(pair),
                // Stopped mid-attempt, or the close was reported meanwhile
                // (#145); the shutdown path owns the close.
                Err(MarketDataError::ClientClosed) => return,
                Err(_) if shared.should_stop.load(Ordering::SeqCst) => return,
                // The server rejected the credentials (reported as
                // `Unauthenticated` by `reconnect_and_authenticate`): the
                // same ones would be rejected again, so the loop stops here
                // with `ReconnectFailed`, the state `Closed` (#201).
                Err(MarketDataError::AuthError { msg, .. }) => {
                    let attempts = {
                        let mgr = shared.reconnection.lock().expect("reconnection lock poisoned");
                        mgr.current_attempt()
                    };
                    // Unless a racing `disconnect()` reported the close first.
                    shared.stream.reconnect_failed(
                        &shared.state,
                        close.code,
                        attempts,
                        rejected_reason(&msg),
                        DisconnectIntent::Server,
                    );
                    return;
                }
                Err(e) => {
                    // Report why this attempt failed before the next
                    // `Reconnecting` (#200).
                    if !shared.stream.emit_unless_closed(ConnectionEvent::error(&e)) {
                        return;
                    }
                    continue;
                }
            }
        };
        connection = new_conn;
    }
}

// Suppress warning: AtomicBool re-export is only used through shared.should_stop.
#[allow(dead_code)]
fn _atomic_bool_used(_: &AtomicBool) {}

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

    #[test]
    fn replay_queues_one_frame_per_batch_in_order() {
        let (tx, rx) = event_stream();
        let (write_tx, write_rx) = mpsc::sync_channel(8);

        replay_subscriptions(frames(), &tx, &write_tx).expect("replay succeeds");

        let sent: Vec<String> = write_rx.try_iter().collect();
        assert_eq!(sent.len(), 2, "one frame per batch");
        assert!(sent[0].contains(r#""symbols":["2330","2454"]"#), "{}", sent[0]);
        assert!(sent[1].contains(r#""symbol":"2317""#), "{}", sent[1]);
        assert!(rx.try_recv().is_err(), "no events on success");
    }

    #[test]
    fn replay_reports_each_failed_batch_and_keeps_going() {
        let (tx, rx) = event_stream();
        let (write_tx, write_rx) = mpsc::sync_channel(8);
        drop(write_rx);

        let err = replay_subscriptions(frames(), &tx, &write_tx).expect_err("replay fails");
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

    /// [`frames`] with the books frame unbuildable. The real replay failures
    /// cannot be reached deterministically: a stored subscription always
    /// serializes, and the write channel's receiver is created inside
    /// `open_reconnected` and not dropped before the owner loop drains it.
    /// So the order tests below inject the failure; the report path is the
    /// one `replay_subscriptions` takes.
    fn frames_with_books_unbuildable() -> Vec<ResubscribeFrame> {
        let mut frames = frames();
        frames[1].frame = Err(MarketDataError::ConnectionError { msg: "injected".to_string() });
        frames
    }

    /// The `Error` for a subscription that could not be replayed follows
    /// the connection's `Authenticated` and is read in the state
    /// `Connected`, as on the async client (#174) — not in `Authenticating`,
    /// where it reads as a failure of the handshake. The frames that could
    /// be built are still queued ahead of anything else.
    #[test]
    fn resubscribe_failure_is_reported_after_authenticated_in_state_connected() {
        let (stream, rx) = event_stream();
        let shared = owner_shared(stream, HealthCheckConfig::disabled());
        *shared.state.write().unwrap() = ConnectionState::Authenticating;

        let write_rx = open_reconnected(
            &shared,
            serde_json::json!({"k": 1}),
            Vec::new(),
            frames_with_books_unbuildable(),
        )
        .expect("the connection opens");

        assert_eq!(*shared.state.read().unwrap(), ConnectionState::Connected);
        let events: Vec<_> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
        match events.as_slice() {
            [
                StreamItem::Event(ConnectionEvent::Authenticated { data }),
                StreamItem::Event(ConnectionEvent::Error(info)),
            ] => {
                assert_eq!(*data, serde_json::json!({"k": 1}));
                assert!(info.message.contains("Failed to resubscribe books:2317"), "{}", info.message);
                assert!(info.message.contains("injected"), "{}", info.message);
            }
            other => panic!("expected Authenticated then Error, got {other:?}"),
        }
        let queued: Vec<String> = write_rx.try_iter().collect();
        assert_eq!(queued.len(), 1, "the buildable frame is queued: {queued:?}");
        assert!(queued[0].contains(r#""symbols":["2330","2454"]"#), "{}", queued[0]);
        assert!(
            shared.write_tx_slot.lock().unwrap().is_some(),
            "the write slot is installed for the consumer of Authenticated"
        );
    }

    /// A close reported before the handshake's result is installed drops
    /// the connection: neither `Authenticated` nor the replay's `Error`
    /// follows the client's `Disconnected` (#145).
    #[test]
    fn resubscribe_failure_after_the_client_s_close_is_reported_is_not_reported() {
        let (stream, rx) = event_stream();
        let shared = owner_shared(stream, HealthCheckConfig::disabled());
        shared.stream.client_closed(&shared.state, 1006, "Force closed".into());
        let closed = shared.state.read().unwrap().clone();

        let result = open_reconnected(
            &shared,
            serde_json::Value::Null,
            Vec::new(),
            frames_with_books_unbuildable(),
        );

        assert!(matches!(result, Err(MarketDataError::ClientClosed)), "{result:?}");
        let events: Vec<String> = std::iter::from_fn(|| rx.try_recv().ok())
            .map(|item| format!("{item:?}"))
            .collect();
        assert_eq!(events.len(), 1, "nothing follows the client's close: {events:?}");
        assert!(events[0].contains("Disconnected") && events[0].contains("Client"), "{events:?}");
        assert_eq!(*shared.state.read().unwrap(), closed);
    }

    /// The owner loop's shared state for a connected client with `health`
    /// and reconnection disabled.
    fn owner_shared(stream: StreamSender, health: HealthCheckConfig) -> OwnerShared {
        let config = ConnectionConfig::new("ws://127.0.0.1", AuthRequest::with_api_key("k"));
        let counter = || DropCounter::new("test", "localhost", "test");
        OwnerShared {
            tls_config: crate::tls::build_rustls_config(&config.tls).expect("tls"),
            config,
            health,
            latency: LatencyWaiters::default(),
            reconnection: Mutex::new(ReconnectionManager::new(
                crate::websocket::ReconnectionConfig::disabled(),
            )),
            state: Arc::new(RwLock::new(ConnectionState::Connected)),
            subscriptions: Arc::new(SubscriptionManager::new()),
            stream,
            write_tx_slot: Mutex::new(None),
            should_stop: Arc::new(AtomicBool::new(false)),
            abort: AtomicBool::new(false),
            messages_dropped: counter(),
            events_dropped: counter(),
        }
    }

    /// A `ws://` URL nothing listens on: the port of a listener that was
    /// just closed.
    fn refusing_url() -> String {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let url = format!("ws://{}", listener.local_addr().expect("addr"));
        drop(listener);
        url
    }

    /// A `ws://` URL of a server that answers the first frame it reads (the
    /// auth frame) with `answer` and then waits for the client to go away.
    fn answering_url(answer: &'static str) -> String {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let url = format!("ws://{}", listener.local_addr().expect("addr"));
        std::thread::spawn(move || {
            let (tcp, _) = listener.accept().expect("accept");
            let mut ws = tungstenite::accept(tcp).expect("handshake");
            ws.read().expect("auth");
            ws.send(Message::Text(answer.into())).expect("answer");
            while ws.read().is_ok() {}
        });
        url
    }

    /// `owner_shared` mid-reconnect: `url` as the server, the state
    /// `Reconnecting { 1 }`.
    fn reconnecting_shared(url: String) -> (Arc<OwnerShared>, crate::websocket::stream_queue::QueueReceiver) {
        let (stream, rx) = event_stream();
        let mut shared = owner_shared(stream, HealthCheckConfig::disabled());
        shared.config = ConnectionConfig::new(url, AuthRequest::with_api_key("k"));
        *shared.state.write().unwrap() = ConnectionState::Reconnecting { attempt: 1 };
        (Arc::new(shared), rx)
    }

    fn events(rx: &crate::websocket::stream_queue::QueueReceiver) -> Vec<String> {
        std::iter::from_fn(|| rx.try_recv().ok())
            .map(|item| match item {
                StreamItem::Event(event) => format!("{event:?}"),
                StreamItem::Message(message) => format!("m:{}", message.event),
            })
            .collect()
    }

    /// A reconnect attempt the transport refuses ends in the state
    /// `Disconnected`, as on the async client (#201): not `Connecting`,
    /// where it failed. The refusal is tungstenite's error (3002, `Io`),
    /// also as on the async client, and the caller's to report (#200).
    #[test]
    fn refused_reconnect_attempt_leaves_state_disconnected() {
        let (shared, rx) = reconnecting_shared(refusing_url());

        let err = reconnect_and_authenticate(&shared).expect_err("refused");

        assert_eq!(err.info().code, error_code::WEBSOCKET, "{err:?}");
        assert_eq!(*shared.state.read().unwrap(), ConnectionState::Disconnected);
        assert_eq!(events(&rx), ["Connecting"]);
    }

    /// A reconnect attempt the server answers with an `error` other than
    /// `1000` fails (`ConnectionError`, for the caller to report) and ends in
    /// the state `Disconnected`, with no `Unauthenticated` (#201).
    #[test]
    fn reconnect_attempt_with_auth_service_down_leaves_state_disconnected() {
        let (shared, rx) = reconnecting_shared(answering_url(
            r#"{"event":"error","code":1011,"data":{"message":"Auth service unavailable"}}"#,
        ));

        let err = reconnect_and_authenticate(&shared).expect_err("failed");

        assert!(
            matches!(&err, MarketDataError::ConnectionError { msg }
                if msg == "Authentication failed (server error 1011): Auth service unavailable"),
            "{err:?}"
        );
        assert_eq!(*shared.state.read().unwrap(), ConnectionState::Disconnected);
        assert_eq!(events(&rx), ["Connecting", "Connected"]);
    }

    /// A reconnect attempt whose credentials are rejected reports
    /// `Unauthenticated` itself, returns `AuthError` for the supervisor to
    /// stop on, and ends in the state `Disconnected` (the supervisor's
    /// `reconnect_failed` then records `Closed`, #201).
    #[test]
    fn rejected_reconnect_attempt_reports_unauthenticated_and_returns_auth_error() {
        let (shared, rx) = reconnecting_shared(answering_url(
            r#"{"event":"error","code":1000,"data":{"message":"Invalid token"}}"#,
        ));

        let err = reconnect_and_authenticate(&shared).expect_err("rejected");

        assert!(matches!(&err, MarketDataError::AuthError { msg, .. } if msg == "Invalid token"), "{err:?}");
        assert_eq!(*shared.state.read().unwrap(), ConnectionState::Disconnected);
        // The rejection frame follows `Unauthenticated` (guarantee 4).
        let events = events(&rx);
        assert_eq!(events.len(), 4, "{events:?}");
        assert_eq!(&events[..2], ["Connecting", "Connected"]);
        assert!(events[2].starts_with("Unauthenticated"), "{events:?}");
        assert_eq!(events[3], "m:error");
    }

    /// A plain WebSocket to a peer that completes the handshake and then
    /// runs `peer` with its end of the connection.
    fn socket_to(peer: impl FnOnce(WebSocket<TcpStream>) + Send + 'static) -> SyncWs {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");
        std::thread::spawn(move || {
            let (tcp, _) = listener.accept().expect("accept");
            peer(tungstenite::accept(tcp).expect("handshake"));
        });
        let tcp = TcpStream::connect(addr).expect("connect");
        let (ws, _) = tungstenite::client(
            format!("ws://{addr}/"),
            MaybeTlsStream::Plain(tcp),
        )
        .expect("client handshake");
        ws
    }

    /// A plain WebSocket to a peer that completes the handshake and then
    /// never reads, its send buffers filled so the next write blocks. The
    /// returned sender releases the peer.
    fn stuck_socket() -> (SyncWs, mpsc::Sender<()>) {
        use std::io::Write;
        let (release, released) = mpsc::channel::<()>();
        let mut ws = socket_to(move |_peer| {
            let _ = released.recv();
        });
        let tcp = tcp_stream(&mut ws).expect("plain tcp");
        tcp.set_nonblocking(true).expect("nonblocking");
        // Fill until not even one byte has fit for a while: the kernel can
        // free a little room after the first `WouldBlock`.
        let chunk = [0u8; 64 * 1024];
        let mut full_since: Option<Instant> = None;
        while full_since.is_none_or(|since| since.elapsed() < Duration::from_millis(300)) {
            match tcp.write(&chunk) {
                Ok(_) => full_since = None,
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    full_since.get_or_insert_with(Instant::now);
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(e) => panic!("filling the socket: {e}"),
            }
        }
        tcp.set_nonblocking(false).expect("blocking");
        (ws, release)
    }

    /// A probe that cannot be written gets its verdict at its own deadline,
    /// not after the fixed write timeout of probe mode (#150).
    #[test]
    fn stuck_socket_does_not_hold_the_probe_verdict_back() {
        const IDLE: Duration = Duration::from_millis(200);
        const PROBE_TIMEOUT: Duration = Duration::from_millis(300);
        let (stream, rx) = event_stream();
        let shared = owner_shared(
            stream,
            HealthCheckConfig {
                probe_enabled: true,
                idle_probe_after: Some(IDLE),
                probe_timeout: Some(PROBE_TIMEOUT),
                ..HealthCheckConfig::default()
            },
        );
        let (ws, _release) = stuck_socket();
        let (_write_tx, write_rx) = mpsc::sync_channel(1);

        let started = Instant::now();
        let close = owner_loop(ws, write_rx, &shared);
        let elapsed = started.elapsed();

        assert_eq!(close, ConnectionClose::default());
        // Well short of the fixed write timeout of probe mode (5s).
        assert!(
            elapsed < IDLE + PROBE_TIMEOUT + Duration::from_secs(1),
            "the verdict waited for the write timeout: {elapsed:?}"
        );
        assert!(elapsed >= IDLE + PROBE_TIMEOUT, "{elapsed:?}");
        let events: Vec<_> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
        assert!(
            matches!(
                events.as_slice(),
                [
                    StreamItem::Event(ConnectionEvent::HeartbeatTimeout { elapsed }),
                    StreamItem::Event(ConnectionEvent::Disconnected {
                        intent: DisconnectIntent::Network,
                        ..
                    }),
                ] if *elapsed == IDLE + PROBE_TIMEOUT
            ),
            "{events:?}"
        );
    }

    /// `force_close()` sets `should_stop` and then reports the client's
    /// close; the owner loop can read the flag before and report after. The
    /// loop's report must then be held back (#159). Reporting the close
    /// without the flag replays that interleaving: what the loop sees is
    /// exactly the flag unset and the close reported.
    fn assert_nothing_follows_the_client_s_close(
        health: HealthCheckConfig,
        peer: impl FnOnce(WebSocket<TcpStream>) + Send + 'static,
    ) {
        let (stream, rx) = event_stream();
        let shared = owner_shared(stream, health);
        let ws = socket_to(peer);
        let (_write_tx, write_rx) = mpsc::sync_channel(1);
        shared.stream.client_closed(&shared.state, 1006, "Force closed".into());
        let closed = shared.state.read().unwrap().clone();

        let close = owner_loop(ws, write_rx, &shared);

        assert_eq!(close, ConnectionClose::default());
        let events: Vec<String> = std::iter::from_fn(|| rx.try_recv().ok())
            .map(|item| format!("{item:?}"))
            .collect();
        assert_eq!(events.len(), 1, "nothing follows the client's close: {events:?}");
        assert!(events[0].contains("Disconnected") && events[0].contains("Client"), "{events:?}");
        assert_eq!(*shared.state.read().unwrap(), closed);
    }

    #[test]
    fn heartbeat_timeout_after_the_client_s_close_is_reported_is_not_reported() {
        assert_nothing_follows_the_client_s_close(
            HealthCheckConfig {
                heartbeat_timeout: Duration::from_millis(300),
                ..HealthCheckConfig::default()
            },
            |_peer| std::thread::sleep(Duration::from_secs(5)),
        );
    }

    #[test]
    fn read_error_after_the_client_s_close_is_reported_is_not_reported() {
        // The peer drops the connection without a Close: a transport error
        // (`ResetWithoutClosingHandshake`, the read's `Err(e)` arm).
        assert_nothing_follows_the_client_s_close(HealthCheckConfig::disabled(), drop);
    }

    #[test]
    fn deserialize_error_after_the_client_s_close_is_reported_is_not_reported() {
        assert_nothing_follows_the_client_s_close(HealthCheckConfig::disabled(), |mut peer| {
            peer.send(Message::Text("not json".into())).expect("send");
            // Then the transport error above ends the loop.
        });
    }
}
