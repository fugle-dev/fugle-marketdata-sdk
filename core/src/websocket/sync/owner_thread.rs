//! Owner-thread + reconnect supervisor for the sync WebSocket client.
//!
//! Single thread per connected client: owns the `tungstenite::WebSocket`,
//! drains the outbound write queue, and parses inbound frames.
//! On disconnect, optionally runs the reconnect loop and rebuilds the
//! WebSocket+queue+state in place.

use crate::models::WebSocketMessage;
use crate::websocket::connection_event::emit_event;
use crate::websocket::protocol::{
    classify_auth_response, frame_auth, frame_subscribe_raw, parse_binary_frame, parse_text_frame,
    AuthOutcome,
};
use crate::websocket::{
    ConnectionConfig, ConnectionEvent, ConnectionState, HealthCheckConfig, ReconnectionManager,
    SubscriptionManager,
};
use crate::MarketDataError;
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

/// Auth handshake timeout. Mirrors the async client.
const AUTH_TIMEOUT: Duration = Duration::from_secs(10);

/// Outbound queue capacity. Matches the async path (`aio::writer` uses 64).
pub(crate) const WRITE_QUEUE_CAPACITY: usize = 64;

pub(crate) type SyncWs = WebSocket<MaybeTlsStream<TcpStream>>;

/// Shared state owned by the `WebSocketClient` and updated by the owner thread.
pub(crate) struct OwnerShared {
    pub config: ConnectionConfig,
    pub tls_config: Arc<rustls::ClientConfig>,
    pub health: HealthCheckConfig,
    pub reconnection: Mutex<ReconnectionManager>,
    pub state: Arc<RwLock<ConnectionState>>,
    pub subscriptions: Arc<SubscriptionManager>,
    pub event_tx: mpsc::SyncSender<ConnectionEvent>,
    pub message_tx: mpsc::SyncSender<WebSocketMessage>,
    /// Current outbound sender. Replaced on every reconnect so live `subscribe`
    /// callers pick up the new channel via `.lock().clone()`.
    pub write_tx_slot: Mutex<Option<mpsc::SyncSender<String>>>,
    pub should_stop: Arc<AtomicBool>,
}

/// Build a fresh TLS-wrapped WebSocket via `tungstenite::client_tls_with_config`.
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

    let addrs: Vec<_> = (host, port)
        .to_socket_addrs()
        .map_err(|e| MarketDataError::ConnectionError {
            msg: format!("DNS lookup failed: {e}"),
        })?
        .collect();
    if addrs.is_empty() {
        return Err(MarketDataError::ConnectionError {
            msg: "DNS returned no addresses".to_string(),
        });
    }

    let tcp = TcpStream::connect_timeout(&addrs[0], config.connect_timeout).map_err(|e| {
        MarketDataError::ConnectionError {
            msg: format!("TCP connect failed: {e}"),
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
    .map_err(|e| MarketDataError::ConnectionError {
        msg: format!("WebSocket handshake failed: {e}"),
    })?;

    Ok(ws)
}

/// Apply `set_read_timeout` to the underlying `TcpStream`, going through the
/// `MaybeTlsStream::Rustls` wrapper. Without the explicit downcast,
/// `set_read_timeout` is unreachable through the WebSocket facade.
pub(crate) fn set_read_timeout(ws: &mut SyncWs, t: Option<Duration>) {
    match ws.get_mut() {
        MaybeTlsStream::Plain(s) => {
            let _ = s.set_read_timeout(t);
        }
        MaybeTlsStream::Rustls(s) => {
            let _ = s.sock.set_read_timeout(t);
        }
        _ => {
            // NativeTls variant not enabled by our feature flags. The catch-all
            // here is intentional but should be made exhaustive if we add
            // native-tls support in the future.
        }
    }
}

/// Run the full auth handshake on a freshly-connected WebSocket.
pub(crate) fn do_auth_handshake(
    ws: &mut SyncWs,
    config: &ConnectionConfig,
    message_tx: &mpsc::SyncSender<WebSocketMessage>,
) -> Result<(), MarketDataError> {
    // Send auth frame
    let auth_json = frame_auth(config.auth.clone())?;
    ws.send(Message::Text(auth_json.into())).map_err(|e| {
        MarketDataError::ConnectionError {
            msg: format!("Failed to send auth frame: {e}"),
        }
    })?;

    // Read auth response with overall wall-clock timeout
    set_read_timeout(ws, Some(AUTH_TIMEOUT));
    let deadline = Instant::now() + AUTH_TIMEOUT;
    loop {
        if Instant::now() >= deadline {
            return Err(MarketDataError::TimeoutError {
                operation: "WebSocket authentication".to_string(),
            });
        }

        match ws.read() {
            Ok(Message::Text(text)) => {
                let parsed = parse_text_frame(&text);
                if let Ok(ws_msg) = parsed {
                    let _ = message_tx.try_send(ws_msg.clone());
                    match classify_auth_response(&ws_msg) {
                        AuthOutcome::Authenticated => return Ok(()),
                        AuthOutcome::Failed(msg) => {
                            return Err(MarketDataError::AuthError { msg });
                        }
                        AuthOutcome::Pending => continue,
                    }
                }
            }
            Ok(Message::Close(_)) => {
                return Err(MarketDataError::ConnectionError {
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
                return Err(MarketDataError::ConnectionError {
                    msg: format!("Auth read error: {e}"),
                });
            }
        }
    }
}

/// The owner loop. Reads frames + drains outbound queue + emits events.
/// Returns the close code observed (Some on clean close, None on
/// error/heartbeat-timeout/stream-end). On `should_stop` returns
/// `Some(1000)` after sending a close frame.
fn owner_loop(
    mut ws: SyncWs,
    write_rx: mpsc::Receiver<String>,
    shared: &OwnerShared,
) -> Option<u16> {
    set_read_timeout(&mut ws, Some(READ_POLL_INTERVAL));
    let heartbeat_window = if shared.health.enabled {
        Some(shared.health.heartbeat_timeout)
    } else {
        None
    };
    let mut last_activity = Instant::now();

    loop {
        if shared.should_stop.load(Ordering::SeqCst) {
            // Best-effort close frame; ignore errors during shutdown.
            let _ = ws.close(None);
            return Some(1000);
        }

        // 1. Try a bounded read
        match ws.read() {
            Ok(Message::Text(text)) => {
                last_activity = Instant::now();
                match parse_text_frame(&text) {
                    Ok(ws_msg) => {
                        crate::websocket::protocol::handle_subscribed_event(
                            &shared.subscriptions,
                            &ws_msg,
                        );
                        // Best-effort: if the receiver is dropped/full, we drop the
                        // message. The receive queue is bounded (1024) and shapes
                        // backpressure at the consumer side.
                        let _ = shared.message_tx.try_send(ws_msg);
                    }
                    Err(e) => {
                        emit_event(&shared.event_tx, ConnectionEvent::Error {
                            message: format!("Failed to deserialize message: {e}"),
                            code: 2003,
                        });
                    }
                }
            }
            Ok(Message::Binary(data)) => {
                last_activity = Instant::now();
                match parse_binary_frame(&data) {
                    Ok(ws_msg) => {
                        crate::websocket::protocol::handle_subscribed_event(
                            &shared.subscriptions,
                            &ws_msg,
                        );
                        let _ = shared.message_tx.try_send(ws_msg);
                    }
                    Err(e) => {
                        emit_event(&shared.event_tx, ConnectionEvent::Error {
                            message: format!("Failed to deserialize binary message: {e}"),
                            code: 2003,
                        });
                    }
                }
            }
            Ok(Message::Ping(payload)) => {
                last_activity = Instant::now();
                // tungstenite does not auto-pong in blocking mode — respond manually.
                let _ = ws.send(Message::Pong(payload));
            }
            Ok(Message::Pong(_)) => {
                last_activity = Instant::now();
            }
            Ok(Message::Close(frame)) => {
                let code = frame.as_ref().map(|cf| u16::from(cf.code));
                let reason = frame
                    .as_ref()
                    .map(|cf| cf.reason.to_string())
                    .unwrap_or_else(|| "Server initiated close".to_string());
                emit_event(&shared.event_tx, ConnectionEvent::Disconnected { code, reason });
                let _ = ws.close(None);
                return code;
            }
            Ok(Message::Frame(_)) => {
                last_activity = Instant::now();
            }
            Err(tungstenite::Error::Io(e))
                if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut =>
            {
                // Polling interval elapsed — fall through to write drain + heartbeat check
            }
            Err(tungstenite::Error::ConnectionClosed)
            | Err(tungstenite::Error::AlreadyClosed) => {
                emit_event(&shared.event_tx, ConnectionEvent::Disconnected {
                    code: None,
                    reason: "Connection closed".to_string(),
                });
                return None;
            }
            Err(e) => {
                emit_event(&shared.event_tx, ConnectionEvent::Error {
                    message: format!("WebSocket read error: {e}"),
                    code: 2001,
                });
                return None;
            }
        }

        // 2. Heartbeat liveness check
        if let Some(window) = heartbeat_window {
            if last_activity.elapsed() > window {
                emit_event(&shared.event_tx, ConnectionEvent::HeartbeatTimeout {
                    elapsed: window,
                });
                return None;
            }
        }

        // 3. Drain outbound queue (non-blocking)
        loop {
            match write_rx.try_recv() {
                Ok(json) => {
                    if let Err(e) = ws.send(Message::Text(json.into())) {
                        emit_event(&shared.event_tx, ConnectionEvent::Error {
                            message: format!("WebSocket write error: {e}"),
                            code: 2002,
                        });
                        return None;
                    }
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    // Client dropped its sender — typically a disconnect()
                    // signal. Send close frame and exit.
                    let _ = ws.close(None);
                    return Some(1000);
                }
            }
        }
    }
}

/// Run the auth handshake on a freshly-reconnected stream and emit lifecycle events.
fn reconnect_and_authenticate(
    shared: &Arc<OwnerShared>,
) -> Result<(SyncWs, mpsc::Receiver<String>), MarketDataError> {
    set_state(shared, ConnectionState::Connecting);
    emit_event(&shared.event_tx, ConnectionEvent::Connecting);

    let mut ws = do_blocking_connect(&shared.config, Arc::clone(&shared.tls_config))?;
    emit_event(&shared.event_tx, ConnectionEvent::Connected);

    set_state(shared, ConnectionState::Authenticating);
    do_auth_handshake(&mut ws, &shared.config, &shared.message_tx)?;

    // Build fresh write channel + install into shared slot
    let (write_tx, write_rx) = mpsc::sync_channel::<String>(WRITE_QUEUE_CAPACITY);
    *shared.write_tx_slot.lock().expect("write_tx_slot lock poisoned") = Some(write_tx.clone());

    // Reset reconnection counter
    {
        let mut mgr = shared.reconnection.lock().expect("reconnection lock poisoned");
        mgr.reset();
    }

    // Replay subscriptions
    shared.subscriptions.clear_server_ids();
    for req in shared.subscriptions.get_all() {
        if let Ok(json) = frame_subscribe_raw(req) {
            let _ = write_tx.send(json);
        }
    }

    set_state(shared, ConnectionState::Connected);
    emit_event(&shared.event_tx, ConnectionEvent::Authenticated);
    Ok((ws, write_rx))
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

        let close_code = owner_loop(ws, write_rx, &shared);

        if shared.should_stop.load(Ordering::SeqCst) {
            set_state(&shared, ConnectionState::Closed {
                code: Some(1000),
                reason: "Client disconnected".to_string(),
            });
            return;
        }

        let should_reconnect = {
            let mgr = shared.reconnection.lock().expect("reconnection lock poisoned");
            mgr.should_reconnect(close_code)
        };
        if !should_reconnect {
            let attempts = {
                let mgr = shared.reconnection.lock().expect("reconnection lock poisoned");
                mgr.current_attempt()
            };
            set_state(&shared, ConnectionState::Closed {
                code: close_code,
                reason: "Non-retriable error".to_string(),
            });
            emit_event(&shared.event_tx, ConnectionEvent::ReconnectFailed { attempts });
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
                set_state(&shared, ConnectionState::Closed {
                    code: close_code,
                    reason: "Max reconnection attempts reached".to_string(),
                });
                emit_event(&shared.event_tx, ConnectionEvent::ReconnectFailed { attempts });
                return;
            };

            let attempt = {
                let mgr = shared.reconnection.lock().expect("reconnection lock poisoned");
                mgr.current_attempt()
            };
            set_state(&shared, ConnectionState::Reconnecting { attempt });
            emit_event(&shared.event_tx, ConnectionEvent::Reconnecting { attempt });

            std::thread::sleep(d);

            match reconnect_and_authenticate(&shared) {
                Ok(pair) => break Some(pair),
                Err(e) => {
                    if let MarketDataError::AuthError { msg } = &e {
                        emit_event(&shared.event_tx, ConnectionEvent::Unauthenticated {
                            message: msg.clone(),
                        });
                    } else {
                        emit_event(&shared.event_tx, ConnectionEvent::Error {
                            message: e.to_string(),
                            code: e.to_error_code(),
                        });
                    }
                    continue;
                }
            }
        };
        connection = new_conn;
    }
}

fn set_state(shared: &OwnerShared, new_state: ConnectionState) {
    let mut st = shared.state.write().expect("state lock poisoned");
    *st = new_state;
}

// Suppress warning: AtomicBool re-export is only used through shared.should_stop.
#[allow(dead_code)]
fn _atomic_bool_used(_: &AtomicBool) {}
