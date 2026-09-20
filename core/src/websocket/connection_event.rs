//! Connection state machine and event types.
//!
//! Runtime-free: shared by both the sync `WebSocketClient` (always compiled)
//! and the async `aio::WebSocketClient` (behind the `tokio-comp` feature).
//!
//! # Backpressure policy
//!
//! A client reports every message and event on one ordered stream
//! ([`stream`](crate::websocket::stream)), created with the client. Messages
//! and events have separate allowances, so a consumer falling behind on
//! messages never costs an event:
//!
//! - Messages follow [`MessageOverflow`](crate::websocket::MessageOverflow):
//!   under the default `DropNewest` at most `message_buffer` unread messages
//!   are held, new ones are dropped while that many are, counted by
//!   `messages_dropped_total()` and reported with
//!   [`MessagesDropped`](ConnectionEvent::MessagesDropped) (#46).
//! - At most `event_buffer` (default
//!   [`DEFAULT_EVENT_BUFFER`](crate::websocket::DEFAULT_EVENT_BUFFER)) unread
//!   events are held; further events are dropped and counted by
//!   `events_dropped_total()`. A healthy consumer never approaches the cap.
//!
//! Neither ever blocks the network task.
//!
//! # Delivery guarantees
//!
//! Bindings forward the stream instead of re-deriving connection semantics,
//! so the following hold for both the sync and async clients:
//!
//! 1. The stream is FIFO across messages and events: items are delivered in
//!    the order the client produced them, and a consumer that starts
//!    reading late still receives the retained items in order.
//! 2. Before `connect()` returns, the stream already holds
//!    [`Connecting`](ConnectionEvent::Connecting) →
//!    [`Connected`](ConnectionEvent::Connected) (transport established) →
//!    exactly one of [`Authenticated { data }`](ConnectionEvent::Authenticated),
//!    [`Unauthenticated { message, data }`](ConnectionEvent::Unauthenticated) or
//!    [`Error`](ConnectionEvent::Error). `Unauthenticated` means exactly one
//!    thing: the server answered the auth frame with `error` code `1000`,
//!    credentials rejected; `connect()` then fails with `AuthError` (2002).
//!    An `error` with any other code (`1011` auth service unavailable,
//!    `1004` no auth request seen, an unknown code) or none is not a verdict
//!    on the credentials: it is reported as `Error` with code `CONNECTION`
//!    (2001) and a message naming the server's code, and `connect()` fails
//!    with `ConnectionError` (#201); the frames read during that handshake
//!    are discarded. If the transport cannot be established the sequence is
//!    `Connecting` → `Error`, on both clients with code `WEBSOCKET` (3002)
//!    and the transport error's kind (DNS, TCP, TLS, or the upgrade's HTTP
//!    status), or `TIMEOUT` (3001) when `connect_timeout` ran out. A successful reconnect goes through the same sequence, and
//!    replays the stored subscriptions only after its `Authenticated`: an
//!    `Error` for one that could not be replayed (`Failed to resubscribe …`)
//!    follows that `Authenticated` and is read in the state `Connected`
//!    (#174). It is a failure of the replay, never of the handshake, which
//!    ends in exactly one of the three events above.
//! 3. Each authenticated connection yields at most one
//!    [`Disconnected`](ConnectionEvent::Disconnected); a
//!    [`HeartbeatTimeout`](ConnectionEvent::HeartbeatTimeout) precedes it.
//!    `will_reconnect == true` implies at least one
//!    [`Reconnecting { attempt }`](ConnectionEvent::Reconnecting) follows,
//!    then either sequence 2 or
//!    [`ReconnectFailed { attempts >= 1 }`](ConnectionEvent::ReconnectFailed).
//!    Whether a lost connection is retried is decided by
//!    [`ReconnectionManager::should_reconnect`]: not reconnecting is the
//!    enumerated case — reconnect disabled, `disconnect()`, a close with
//!    code `1000`, or a connection whose last `error` frame had code `1000`
//!    (the server rejected the credentials and then closed without a code)
//!    — and every other close reconnects, whatever its code (#201).
//!    Each attempt that fails says why (#200): `Reconnecting { n }` →
//!    `Connecting` → (`Connected` →) exactly one of `Error` (the transport
//!    was refused or timed out, the auth response never came, or it was an
//!    `error` other than `1000`) or `Unauthenticated` (the credentials were
//!    rejected). After an `Error` the loop goes on with
//!    `Reconnecting { n + 1 }` or `ReconnectFailed`; the `Error` is
//!    diagnostic and, between the attempt and the next `Reconnecting`, the
//!    state is [`ConnectionState::Disconnected`] on both clients. After an
//!    `Unauthenticated` the loop stops (#201): the same credentials would be
//!    rejected again, so `ReconnectFailed { attempts: n }` follows at once,
//!    the state becomes `Closed { intent: Server, .. }` (the server refused
//!    the connection, as when it rejects on a live connection) with the
//!    code of the close that started the loop and a reason naming the
//!    rejection, and nothing is emitted after it. When the attempts run out
//!    instead, the state is `Closed { intent: Network, .. }`. (A rejected first `connect()` starts
//!    no reconnect loop: it fails with `AuthError` and leaves the state
//!    `Disconnected`, sequence 2.)
//!    If `disconnect()` or `force_close()` is called in the meantime, the
//!    reconnect stops: the state becomes `Closed { intent: Client, .. }`,
//!    then a final `Disconnected { intent: Client, will_reconnect: false }`
//!    is emitted and nothing after it (#98). Either way each disconnect ends
//!    in exactly one final event: a `Disconnected` with
//!    `will_reconnect == false`, or `ReconnectFailed`.
//!    By the time a consumer receives a lost connection's `Disconnected`,
//!    the state already reflects it (#86): `Closed` with the event's `code`,
//!    `reason` and `intent` when `will_reconnect == false`, otherwise
//!    [`ConnectionState::Disconnected`] until the reconnect loop moves on.
//! 4. Every message of an authenticated connection, including the server's
//!    `authenticated` frame, comes after that connection's `Authenticated`
//!    and before its `Disconnected`. Frames a connection receives after it
//!    was reported closed are discarded, uncounted (#68). The frames read
//!    while credentials are rejected follow `Unauthenticated`.
//! 5. `Error` is diagnostic and may accompany `Disconnected` (a transport
//!    error emits both, `Error` first).
//! 6. [`MessagesDropped`](ConnectionEvent::MessagesDropped) is diagnostic and
//!    leaves the state unchanged. It appears only between a connection's
//!    `Authenticated` and its `Disconnected`, right after the message whose
//!    push found a report due: the first drop is reported at once, later ones
//!    at most once per second, and the rest right before `Disconnected`. Like
//!    any event it is lost if the event allowance is full;
//!    `messages_dropped_total()` is the authoritative count.
//! 7. [`ConnectionEvent`] and [`StreamItem`](crate::websocket::StreamItem)
//!    are `#[non_exhaustive]`: matches need a `_` arm, and a new diagnostic
//!    variant may be added in a minor release.

use std::time::Duration;

use crate::errors::{ErrorInfo, MarketDataError};
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

/// A handle reading a client's [`ConnectionState`] that stays readable after
/// the client is dropped, returned by
/// [`aio::WebSocketClient::state_handle`](crate::aio::WebSocketClient::state_handle).
///
/// Lets a binding that drops its client once the connection ends still
/// report the last connection's state, without keeping the client — and
/// with it the client's stream — alive. Cheap to clone; every clone reads the
/// same state. Callable from any thread, on or off a runtime.
#[derive(Clone)]
pub struct ConnectionStateHandle {
    state: std::sync::Arc<std::sync::RwLock<ConnectionState>>,
}

impl ConnectionStateHandle {
    pub(crate) fn new(state: std::sync::Arc<std::sync::RwLock<ConnectionState>>) -> Self {
        Self { state }
    }

    /// Read guard on the state. A poisoned lock still yields the value:
    /// writers only assign, so it can never be left half-updated.
    fn read(&self) -> std::sync::RwLockReadGuard<'_, ConnectionState> {
        self.state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// The current (or last) state.
    pub fn state(&self) -> ConnectionState {
        self.read().clone()
    }

    /// Whether the state is [`Connected`](ConnectionState::Connected).
    pub fn is_connected(&self) -> bool {
        matches!(*self.read(), ConnectionState::Connected)
    }

    /// Whether the state is [`Closed`](ConnectionState::Closed).
    pub fn is_closed(&self) -> bool {
        matches!(*self.read(), ConnectionState::Closed { .. })
    }

    /// Whether the connection is open, being established or auto-reconnecting
    /// ([`Connecting`](ConnectionState::Connecting),
    /// [`Authenticating`](ConnectionState::Authenticating),
    /// [`Connected`](ConnectionState::Connected) or
    /// [`Reconnecting`](ConnectionState::Reconnecting)): the states in which
    /// `connect()` is refused with [`MarketDataError::AlreadyConnected`].
    /// Lets a binding that opens each connection on a new client refuse the
    /// same way while its previous client is still live.
    pub fn is_active(&self) -> bool {
        matches!(
            *self.read(),
            ConnectionState::Connecting
                | ConnectionState::Authenticating
                | ConnectionState::Connected
                | ConnectionState::Reconnecting { .. }
        )
    }
}

impl std::fmt::Debug for ConnectionStateHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConnectionStateHandle")
            .field("state", &self.state())
            .finish()
    }
}

/// Events emitted by WebSocket connection.
///
/// Delivered as [`StreamItem::Event`](crate::websocket::StreamItem::Event)
/// on the client's stream. Consumers attribute events to their source
/// client via the stream they were yielded from — `tokio::select!` arms naturally
/// label by source, and code that merges streams from multiple clients
/// is expected to wrap with its own labeling adapter (3 lines via
/// `tokio_stream::StreamExt::map`). The SDK does not pre-empt that
/// decision by stuffing a label on every event.
///
/// See the module-level [delivery guarantees](self#delivery-guarantees) for
/// the ordering bindings can rely on.
#[non_exhaustive]
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
    /// Server rejected the credentials: it answered the auth frame with an
    /// `error` frame of code `1000` (parallels the 1.x SDKs'
    /// `unauthenticated` event). `connect()` fails with
    /// [`AuthError`](crate::MarketDataError::AuthError); no `Error` event is
    /// emitted for the rejection. During an auto-reconnect it is followed at
    /// once by [`ReconnectFailed`](Self::ReconnectFailed): the same
    /// credentials would be rejected again, so the loop stops (#201).
    ///
    /// An auth-phase `error` with any other code (`1011` auth service
    /// unavailable, `1004` no auth request seen) is not a rejection: it is
    /// reported as [`Error`](Self::Error) and, during a reconnect, retried.
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
    /// Emitted **at most once per connection**, except that stopping a
    /// reconnect adds the final one: whichever side observes the close first
    /// reports it. A server Close racing `disconnect()` yields a single
    /// event, and calling `disconnect()` / `force_close()` after a
    /// `will_reconnect: false` close or a `ReconnectFailed` (or calling them
    /// twice) emits no further `Disconnected`. Calling them after a
    /// `will_reconnect: true` close, while the client reconnects, emits
    /// `Disconnected { intent: Client, will_reconnect: false }` (#98). A
    /// successful reconnect starts a new connection.
    Disconnected {
        /// WebSocket close code, if the peer supplied one.
        code: Option<u16>,
        /// Human-readable close reason (may be empty).
        reason: String,
        /// Who initiated the disconnect.
        intent: DisconnectIntent,
        /// Whether the client will try to reconnect. `true` means at least
        /// one [`Reconnecting`](Self::Reconnecting) follows (unless
        /// `disconnect()` is called first, which emits a final
        /// `will_reconnect: false` instead); `false` means this connection's
        /// lifecycle has ended.
        will_reconnect: bool,
    },
    /// Reconnection attempt started
    Reconnecting {
        /// Current attempt number (1-indexed).
        attempt: u32,
    },
    /// Reconnection gave up: it exhausted the configured attempts, or an
    /// attempt's credentials were rejected (`Unauthenticated` precedes it,
    /// #201). Terminal: the state is `Closed` and nothing follows. Only
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
    /// Inbound messages were dropped because the message queue was full
    /// (the consumer fell behind under
    /// [`MessageOverflow::DropNewest`](crate::websocket::MessageOverflow::DropNewest)).
    /// Diagnostic: the connection stays up.
    ///
    /// Throttled: the first drop on a connection is reported at once, later
    /// ones at most once per second, and any drops not yet reported are
    /// reported right before that connection's `Disconnected`. Messages
    /// arriving after the connection was reported closed are discarded
    /// without being counted.
    /// `messages_dropped_total()` is the authoritative count.
    MessagesDropped {
        /// Messages dropped since the previous `MessagesDropped`.
        dropped: u64,
        /// Messages dropped on this connection so far: counted from the
        /// start of its `connect()` (or reconnect), like
        /// `messages_dropped_total()`.
        total: u64,
    },
    /// Error occurred. Diagnostic: a transport error emits both `Error` and
    /// (afterwards) `Disconnected`.
    ///
    /// Carries the same [`ErrorInfo`] fields as a returned error: `code`,
    /// `source_kind`, `message` and, for a rejected WebSocket upgrade, `status`.
    Error(ErrorInfo),
}

impl ConnectionEvent {
    /// `Error` for `err`, with `message` in place of the error's own text.
    pub(crate) fn error_with_message(err: &MarketDataError, message: String) -> Self {
        Self::Error(ErrorInfo { message, ..err.info() })
    }

    /// `Error` for `err`.
    pub(crate) fn error(err: &MarketDataError) -> Self {
        Self::Error(err.info())
    }

    /// `Error` for a subscription that could not be re-sent after a
    /// reconnect; the message names its key, or for a batch the channel,
    /// modifier and symbol count label (e.g. `trades:oddlot (3 symbols)`).
    pub(crate) fn resubscribe_failed(key: &str, err: &MarketDataError) -> Self {
        Self::error_with_message(err, format!("Failed to resubscribe {key}: {err}"))
    }
}

/// How a connection ended, as far as the reconnect policy is concerned:
/// returned by the sync owner loop and the async dispatch loop, consumed by
/// their reconnect loops.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct ConnectionClose {
    /// The peer's close code; `None` for a Close frame without one, a
    /// dropped transport, a transport error or a heartbeat timeout.
    pub(crate) code: Option<u16>,
    /// The `code` of the last `error` frame the connection delivered after
    /// it was authenticated, if any. `1000` means the server rejected the
    /// credentials before closing (#201).
    pub(crate) last_error_code: Option<i32>,
}

/// The `will_reconnect` a `Disconnected` should carry: whether the client
/// is about to enter its reconnect loop for this close.
///
/// A caller-initiated close (or one observed after shutdown was requested)
/// never reconnects; otherwise the reconnect policy decides from the close
/// code and the last `error` frame's code
/// ([`ReconnectionManager::should_reconnect`]).
pub(crate) fn will_reconnect_after(
    mgr: &ReconnectionManager,
    intent: DisconnectIntent,
    close: ConnectionClose,
    shutdown_requested: bool,
) -> bool {
    if intent == DisconnectIntent::Client || shutdown_requested {
        return false;
    }
    mgr.should_reconnect(close.code, close.last_error_code)
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

    fn mgr(enabled: bool) -> ReconnectionManager {
        let config = if enabled {
            crate::websocket::ReconnectionConfig::default()
        } else {
            crate::websocket::ReconnectionConfig::disabled()
        };
        ReconnectionManager::new(config)
    }

    #[test]
    fn error_events_carry_the_error_info() {
        use crate::errors::{error_code, ErrorKind};
        let read = crate::MarketDataError::from(tungstenite::Error::ConnectionClosed);
        match ConnectionEvent::error_with_message(&read, "WebSocket read error: closed".into()) {
            ConnectionEvent::Error(info) => {
                assert_eq!(info.code, error_code::WEBSOCKET);
                assert_eq!(info.source_kind, ErrorKind::Network);
                assert_eq!(info.message, "WebSocket read error: closed");
            }
            other => panic!("expected Error, got {other:?}"),
        }
        let parse = crate::websocket::protocol::parse_text_frame("{").unwrap_err();
        match ConnectionEvent::error(&parse) {
            ConnectionEvent::Error(info) => assert_eq!(info.code, error_code::DESERIALIZATION),
            other => panic!("expected Error, got {other:?}"),
        }
    }

    /// The full decision table (#201): who closed, with which code, after
    /// which last `error` frame, and whether shutdown was requested. Not
    /// reconnecting is the enumerated case; every other row reconnects.
    #[test]
    fn will_reconnect_after_matrix() {
        use DisconnectIntent::{Client, Network, Server};
        let close = |code: Option<u16>, last_error_code: Option<i32>| ConnectionClose {
            code,
            last_error_code,
        };
        let plain = |code: Option<u16>| close(code, None);

        // Caller-initiated closes never reconnect, even when the policy would.
        assert!(!will_reconnect_after(&mgr(true), Client, plain(Some(1006)), false));
        assert!(!will_reconnect_after(&mgr(true), Client, plain(None), false));
        // Disabled policy never reconnects.
        assert!(!will_reconnect_after(&mgr(false), Network, plain(Some(1006)), false));
        assert!(!will_reconnect_after(&mgr(false), Server, plain(Some(1001)), false));
        assert!(!will_reconnect_after(&mgr(false), Network, plain(None), false));
        // A close observed after shutdown was requested is final.
        assert!(!will_reconnect_after(&mgr(true), Network, plain(Some(1006)), true));
        assert!(!will_reconnect_after(&mgr(true), Server, plain(Some(1001)), true));
        // Normal closure is final.
        assert!(!will_reconnect_after(&mgr(true), Server, plain(Some(1000)), false));
        // Credentials rejected: `error{1000}` then the server's Close
        // without a code, or a Close with one, or the transport dropped.
        assert!(!will_reconnect_after(&mgr(true), Server, close(None, Some(1000)), false));
        assert!(!will_reconnect_after(&mgr(true), Server, close(Some(1001), Some(1000)), false));
        assert!(!will_reconnect_after(&mgr(true), Network, close(None, Some(1000)), false));

        // Codes the server sends.
        assert!(will_reconnect_after(&mgr(true), Server, plain(Some(1001)), false));
        assert!(will_reconnect_after(&mgr(true), Server, plain(Some(1008)), false));
        // A Close without a code, absent a rejection (regression: the
        // server's plain `close()` after `error{1004}`, or any other).
        assert!(will_reconnect_after(&mgr(true), Server, plain(None), false));
        assert!(will_reconnect_after(&mgr(true), Server, close(None, Some(1004)), false));
        assert!(will_reconnect_after(&mgr(true), Server, close(None, Some(1011)), false));
        assert!(will_reconnect_after(&mgr(true), Server, close(None, Some(1003)), false));
        // Transport closes: abnormal closure, EOF, error, heartbeat timeout.
        assert!(will_reconnect_after(&mgr(true), Network, plain(Some(1006)), false));
        assert!(will_reconnect_after(&mgr(true), Network, plain(None), false));
        assert!(will_reconnect_after(&mgr(true), Network, close(None, Some(1003)), false));
        // Unknown codes, 4xxx included: the server never sends them.
        assert!(will_reconnect_after(&mgr(true), Server, plain(Some(1002)), false));
        assert!(will_reconnect_after(&mgr(true), Server, plain(Some(4001)), false));
        assert!(will_reconnect_after(&mgr(true), Server, plain(Some(4999)), false));
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
