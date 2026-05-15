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

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;

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
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionEvent {
    /// Connection attempt started
    Connecting,
    /// Connection established
    Connected,
    /// Authentication successful
    Authenticated,
    /// Authentication rejected by the server (parallels old SDKs' `unauthenticated` event)
    Unauthenticated {
        /// Server-provided rejection message.
        message: String,
    },
    /// Connection closed.
    ///
    /// `intent` classifies the originator: [`Client`](DisconnectIntent::Client)
    /// for local-initiated, [`Server`](DisconnectIntent::Server) for a
    /// peer Close frame, [`Network`](DisconnectIntent::Network) for
    /// transport errors / EOF / heartbeat timeout.
    Disconnected {
        /// WebSocket close code, if the peer supplied one.
        code: Option<u16>,
        /// Human-readable close reason (may be empty).
        reason: String,
        /// Who initiated the disconnect.
        intent: DisconnectIntent,
    },
    /// Reconnection attempt started
    Reconnecting {
        /// Current attempt number (1-indexed).
        attempt: u32,
    },
    /// Reconnection failed after max attempts
    ReconnectFailed {
        /// Total attempts performed before giving up.
        attempts: u32,
    },
    /// Heartbeat timeout: no inbound frame received within the configured
    /// `heartbeat_timeout` window. Emitted by the dispatch loop when the
    /// read-site timeout fires; the dispatch loop returns immediately
    /// afterwards, which lets the reconnect path take over.
    HeartbeatTimeout {
        /// Wall-clock interval that elapsed since the last inbound frame.
        elapsed: Duration,
    },
    /// Error occurred
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
/// policy and how saturation is surfaced. The `dropped` atomic is
/// incremented once per drop so consumers can observe saturation via
/// [`crate::WebSocketClient::events_dropped_total`] /
/// [`crate::aio::WebSocketClient::events_dropped_total`].
pub(crate) fn emit_event(
    tx: &mpsc::SyncSender<ConnectionEvent>,
    dropped: &Arc<AtomicU64>,
    event: ConnectionEvent,
) {
    if let Err(mpsc::TrySendError::Full(dropped_event)) = tx.try_send(event) {
        dropped.fetch_add(1, Ordering::Relaxed);
        crate::tracing_compat::warn!(
            target: "fugle_marketdata::ws",
            dropped = ?dropped_event,
            "event channel saturated; consumer is likely stuck"
        );
        let _ = dropped_event; // suppress unused warning when tracing feature is off
    }
}
