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
