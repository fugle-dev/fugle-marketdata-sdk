//! Connection state machine and event types.
//!
//! Runtime-free: this module depends only on `std::sync::mpsc` and
//! `std::time::Duration`. It is shared by both the sync `WebSocketClient`
//! (always compiled) and the async `aio::WebSocketClient` (behind the
//! `tokio-comp` feature).
//!
//! # Backpressure policy
//!
//! Events flow over `std::sync::mpsc::sync_channel(N)` where `N` is the
//! per-client `event_buffer` (default
//! [`DEFAULT_EVENT_BUFFER`](crate::websocket::DEFAULT_EVENT_BUFFER)). The
//! channel is **drop-newest**: when full, `emit_event` (internal) discards
//! the incoming event rather than blocking the network task. This is the only
//! safe choice because `std::sync::mpsc` does not expose receiver-side
//! access to the sender; switching to a primitive that does (e.g.
//! `tokio::sync::broadcast`) would break the `events()` /
//! `state_events()` API surface that bindings depend on.
//!
//! Drops are surfaced via:
//! - the per-client
//!   [`messages_dropped_total`](crate::aio::WebSocketClient::messages_dropped_total)
//!   counter (for the inbound *message* channel; the *event* channel
//!   shares the same drop-newest discipline but is small enough that
//!   saturation is rare); and
//! - a `tracing::warn!` at the saturation site when the `tracing` feature
//!   is enabled.
//!
//! Saturation is itself the bug signal — a healthy consumer never
//! approaches the configured cap.
//!
//! # Delivery guarantees
//!
//! Bindings forward these events instead of re-deriving connection
//! semantics, so the following hold for both the sync and async clients:
//!
//! 1. The event channel is created when the client is constructed. It is
//!    FIFO with capacity `event_buffer` (default 1024) and drop-newest when
//!    full; a consumer that starts reading late still receives the retained
//!    events in order.
//! 2. Before `connect()` returns, the channel already holds
//!    [`Connecting`](ConnectionEvent::Connecting) →
//!    [`Connected`](ConnectionEvent::Connected) (transport established) →
//!    exactly one of [`Authenticated { data }`](ConnectionEvent::Authenticated),
//!    [`Unauthenticated { message, data }`](ConnectionEvent::Unauthenticated) or
//!    [`Error`](ConnectionEvent::Error). If the transport cannot be
//!    established the sequence is `Connecting` → `Error`. A successful
//!    reconnect goes through the same sequence.
//! 3. Each authenticated connection yields at most one
//!    [`Disconnected`](ConnectionEvent::Disconnected); a
//!    [`HeartbeatTimeout`](ConnectionEvent::HeartbeatTimeout) precedes it.
//!    `will_reconnect == true` implies at least one
//!    [`Reconnecting { attempt }`](ConnectionEvent::Reconnecting) follows,
//!    then either sequence 2 or
//!    [`ReconnectFailed { attempts >= 1 }`](ConnectionEvent::ReconnectFailed).
//!    If `disconnect()` is called in the meantime no further events are
//!    emitted and the state becomes `Closed { intent: Client, .. }`.
//! 4. `Error` is diagnostic and may accompany `Disconnected` (a transport
//!    error emits both, `Error` first).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::Duration;

use crate::websocket::ReconnectionManager;

/// Who initiated the disconnect captured by
/// [`ConnectionEvent::Disconnected`] / [`ConnectionState::Closed`].
///
/// Lets consumers branch on the cause without string-matching the
/// `reason` field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisconnectIntent {
    /// Local caller invoked `disconnect()` or `shutdown_with_timeout(...)`.
    Client,
    /// Server sent a Close frame (regardless of close code).
    Server,
    /// Transport-level failure: I/O error, EOF without Close frame,
    /// heartbeat timeout, etc.
    Network,
}

/// WebSocket connection state machine
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    /// Not connected
    Disconnected,
    /// Connecting to server
    Connecting,
    /// Authenticating with server
    Authenticating,
    /// Connected and authenticated
    Connected,
    /// Reconnecting after disconnection
    Reconnecting {
        /// Current attempt number (1-indexed).
        attempt: u32,
    },
    /// Connection closed. `intent` mirrors the matching
    /// [`ConnectionEvent::Disconnected`] field so state inspection by the
    /// caller does not lose classification information.
    Closed {
        /// WebSocket close code, if the peer supplied one.
        code: Option<u16>,
        /// Human-readable close reason (may be empty).
        reason: String,
        /// Who initiated the disconnect.
        intent: DisconnectIntent,
    },
}

/// Events emitted by WebSocket connection.
///
/// Consumers attribute events to their source client via the
/// [`events()`](crate::aio::WebSocketClient::events) /
/// [`state_events()`](crate::aio::WebSocketClient::state_events)
/// `Receiver` they were yielded from — `tokio::select!` arms naturally
/// label by source, and code that merges streams from multiple clients
/// is expected to wrap with its own labeling adapter (3 lines via
/// `tokio_stream::StreamExt::map`). The SDK does not pre-empt that
/// decision by stuffing a label on every event.
///
/// See the module-level [delivery guarantees](self#delivery-guarantees) for
/// the ordering bindings can rely on.
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionEvent {
    /// Connection attempt started (initial `connect()` or a reconnect).
    Connecting,
    /// Transport established; the auth frame has not been answered yet.
    /// Corresponds to the 1.x SDKs' `connect` event.
    Connected,
    /// Server accepted the credentials.
    Authenticated {
        /// `data` of the server's `authenticated` frame, or
        /// [`Null`](serde_json::Value::Null) when the frame has none.
        data: serde_json::Value,
    },
    /// Server rejected the credentials (parallels the 1.x SDKs'
    /// `unauthenticated` event). `connect()` fails with
    /// [`AuthError`](crate::MarketDataError::AuthError); no `Error` event is
    /// emitted for the rejection.
    Unauthenticated {
        /// Server-provided rejection message (`"Unknown error"` if absent).
        message: String,
        /// `data` of the server's rejection frame, or
        /// [`Null`](serde_json::Value::Null) when the frame has none.
        data: serde_json::Value,
    },
    /// Connection closed.
    ///
    /// `intent` classifies the originator: [`Client`](DisconnectIntent::Client)
    /// for local-initiated, [`Server`](DisconnectIntent::Server) for a
    /// peer Close frame, [`Network`](DisconnectIntent::Network) for
    /// transport errors / EOF / heartbeat timeout.
    ///
    /// Emitted **at most once per connection**: whichever side observes the
    /// close first reports it. A server Close racing `disconnect()` yields a
    /// single event, and calling `disconnect()` / `force_close()` after the
    /// connection was already reported lost (or calling them twice) emits no
    /// further `Disconnected`. A successful reconnect starts a new connection.
    Disconnected {
        /// WebSocket close code, if the peer supplied one.
        code: Option<u16>,
        /// Human-readable close reason (may be empty).
        reason: String,
        /// Who initiated the disconnect.
        intent: DisconnectIntent,
        /// Whether the client will try to reconnect. `true` means at least
        /// one [`Reconnecting`](Self::Reconnecting) follows (unless
        /// `disconnect()` is called first); `false` means this connection's
        /// lifecycle has ended.
        will_reconnect: bool,
    },
    /// Reconnection attempt started
    Reconnecting {
        /// Current attempt number (1-indexed).
        attempt: u32,
    },
    /// Reconnection gave up after exhausting the configured attempts. Only
    /// emitted after at least one attempt; a close the reconnect policy
    /// does not retry is reported solely via
    /// `Disconnected { will_reconnect: false, .. }`.
    ReconnectFailed {
        /// Total attempts performed before giving up (always `>= 1`).
        attempts: u32,
    },
    /// Heartbeat timeout: no inbound frame received within the configured
    /// `heartbeat_timeout` window. Diagnostic precursor: it is always
    /// immediately followed by `Disconnected { intent: Network, .. }` for
    /// the same connection, which is the event to react to.
    HeartbeatTimeout {
        /// Wall-clock interval that elapsed since the last inbound frame.
        elapsed: Duration,
    },
    /// Error occurred. Diagnostic: a transport error emits both `Error` and
    /// (afterwards) `Disconnected`.
    Error {
        /// Diagnostic message describing the error.
        message: String,
        /// Numeric error code (mirrors [`MarketDataError::to_error_code`](crate::MarketDataError::to_error_code)).
        code: i32,
    },
}

/// Emit a [`ConnectionEvent`] on the bounded event channel.
///
/// See the module-level documentation for the drop-newest backpressure
/// policy and how saturation is surfaced. The `dropped` counter is
/// incremented once per drop so consumers can observe saturation via
/// [`crate::WebSocketClient::events_dropped_total`] /
/// [`crate::aio::WebSocketClient::events_dropped_total`]. When the
/// `metrics` feature is enabled, the increment also bumps the
/// `fugle_marketdata_ws_events_dropped_total` counter on the active
/// `metrics` recorder.
pub(crate) fn emit_event(
    tx: &mpsc::SyncSender<ConnectionEvent>,
    dropped: &crate::metrics_compat::DropCounter,
    event: ConnectionEvent,
) {
    if let Err(mpsc::TrySendError::Full(dropped_event)) = tx.try_send(event) {
        dropped.bump();
        crate::tracing_compat::warn!(
            target: "fugle_marketdata::ws",
            dropped = ?dropped_event,
            "event channel saturated; consumer is likely stuck"
        );
        let _ = dropped_event; // suppress unused warning when tracing feature is off
    }
}

/// Guarantees at most one [`ConnectionEvent::Disconnected`] per connection.
///
/// The dispatch side (server Close, transport error, EOF) and the caller
/// side (`disconnect()` / `shutdown_with_timeout()` / `force_close()`) run
/// on different threads and may both observe the same close. Each must win
/// [`claim`](Self::claim) before emitting; the loser stays silent (#41).
/// [`reset`](Self::reset) re-arms the latch once a new connection has
/// authenticated.
#[derive(Debug, Default)]
pub(crate) struct DisconnectLatch(AtomicBool);

impl DisconnectLatch {
    /// Take the right to emit this connection's `Disconnected`. Returns
    /// `false` if it was already taken.
    pub(crate) fn claim(&self) -> bool {
        self.0
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    }

    /// Re-arm for a freshly authenticated connection.
    pub(crate) fn reset(&self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

/// [`emit_event`] a `Disconnected`, unless this connection already has one.
pub(crate) fn emit_disconnected(
    tx: &mpsc::SyncSender<ConnectionEvent>,
    dropped: &crate::metrics_compat::DropCounter,
    latch: &DisconnectLatch,
    code: Option<u16>,
    reason: String,
    intent: DisconnectIntent,
    will_reconnect: bool,
) {
    if latch.claim() {
        emit_event(
            tx,
            dropped,
            ConnectionEvent::Disconnected { code, reason, intent, will_reconnect },
        );
    }
}

/// The `will_reconnect` a `Disconnected` should carry: whether the client
/// is about to enter its reconnect loop for this close.
///
/// A caller-initiated close (or one observed after shutdown was requested)
/// never reconnects; otherwise the reconnect policy decides.
pub(crate) fn will_reconnect_after(
    mgr: &ReconnectionManager,
    intent: DisconnectIntent,
    code: Option<u16>,
    shutdown_requested: bool,
) -> bool {
    if intent == DisconnectIntent::Client || shutdown_requested {
        return false;
    }
    mgr.should_reconnect(code)
}

/// The `Disconnected` a peer Close frame should produce, if any.
///
/// Once the caller has requested shutdown, a Close is either the peer's ack
/// of ours or a server close racing it; either way the shutdown path
/// reports the close as `Client`, so this returns `None` (#22). Shared by
/// the async dispatch loop and the sync owner thread.
pub(crate) fn peer_close_disconnect(
    code: Option<u16>,
    reason: Option<String>,
    shutdown_requested: bool,
) -> Option<(Option<u16>, String, DisconnectIntent)> {
    if shutdown_requested {
        return None;
    }
    let reason = reason.unwrap_or_else(|| "Server initiated close".to_string());
    Some((code, reason, DisconnectIntent::Server))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latch_allows_one_claim_until_reset() {
        let latch = DisconnectLatch::default();
        assert!(latch.claim());
        assert!(!latch.claim());
        latch.reset();
        assert!(latch.claim());
    }

    #[test]
    fn latch_admits_exactly_one_of_concurrent_claimers() {
        for _ in 0..200 {
            let latch = std::sync::Arc::new(DisconnectLatch::default());
            let winners: usize = (0..4)
                .map(|_| {
                    let latch = std::sync::Arc::clone(&latch);
                    std::thread::spawn(move || latch.claim())
                })
                .collect::<Vec<_>>()
                .into_iter()
                .map(|h| usize::from(h.join().expect("claimer")))
                .sum();
            assert_eq!(winners, 1);
        }
    }

    #[test]
    fn emit_disconnected_sends_only_first() {
        let (tx, rx) = mpsc::sync_channel(8);
        let dropped = crate::metrics_compat::DropCounter::new("test_events_dropped", "localhost", "test");
        let latch = DisconnectLatch::default();
        emit_disconnected(&tx, &dropped, &latch, None, "server".into(), DisconnectIntent::Server, true);
        emit_disconnected(&tx, &dropped, &latch, Some(1000), "client".into(), DisconnectIntent::Client, false);
        let events: Vec<_> = rx.try_iter().collect();
        assert_eq!(
            events,
            vec![ConnectionEvent::Disconnected {
                code: None,
                reason: "server".into(),
                intent: DisconnectIntent::Server,
                will_reconnect: true,
            }]
        );
    }

    fn mgr(enabled: bool) -> ReconnectionManager {
        let config = if enabled {
            crate::websocket::ReconnectionConfig::default()
        } else {
            crate::websocket::ReconnectionConfig::disabled()
        };
        ReconnectionManager::new(config)
    }

    #[test]
    fn will_reconnect_after_matrix() {
        use DisconnectIntent::{Client, Network, Server};
        // Caller-initiated closes never reconnect, even when the policy would.
        assert!(!will_reconnect_after(&mgr(true), Client, Some(1006), false));
        // Disabled policy never reconnects.
        assert!(!will_reconnect_after(&mgr(false), Network, Some(1006), false));
        assert!(!will_reconnect_after(&mgr(false), Network, None, false));
        // Normal closure is final.
        assert!(!will_reconnect_after(&mgr(true), Server, Some(1000), false));
        // Abnormal closure with reconnect enabled retries.
        assert!(will_reconnect_after(&mgr(true), Network, Some(1006), false));
        assert!(will_reconnect_after(&mgr(true), Network, None, false));
        // Application errors are final.
        assert!(!will_reconnect_after(&mgr(true), Server, Some(4001), false));
        assert!(!will_reconnect_after(&mgr(true), Server, Some(4999), false));
        // A close observed after shutdown was requested is final.
        assert!(!will_reconnect_after(&mgr(true), Network, Some(1006), true));
    }

    #[test]
    fn peer_close_is_server_intent_before_shutdown() {
        assert_eq!(
            peer_close_disconnect(Some(1001), Some("bye".into()), false),
            Some((Some(1001), "bye".to_string(), DisconnectIntent::Server))
        );
        assert_eq!(
            peer_close_disconnect(None, None, false),
            Some((None, "Server initiated close".to_string(), DisconnectIntent::Server))
        );
    }

    #[test]
    fn peer_close_is_silent_once_shutdown_requested() {
        assert_eq!(peer_close_disconnect(Some(1000), Some("ack".into()), true), None);
    }
}
