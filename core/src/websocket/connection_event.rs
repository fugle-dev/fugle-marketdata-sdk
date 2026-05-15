//! Connection state machine and event types.
//!
//! Runtime-free: this module depends only on `std::sync::mpsc` and
//! `std::time::Duration`. It is shared by both the sync `WebSocketClient`
//! (always compiled) and the async `aio::WebSocketClient` (behind the
//! `tokio-comp` feature).

use std::sync::mpsc;
use std::time::Duration;

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
    Reconnecting { attempt: u32 },
    /// Connection closed
    Closed { code: Option<u16>, reason: String },
}

/// Events emitted by WebSocket connection
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionEvent {
    /// Connection attempt started
    Connecting,
    /// Connection established
    Connected,
    /// Authentication successful
    Authenticated,
    /// Authentication rejected by the server (parallels old SDKs' `unauthenticated` event)
    Unauthenticated { message: String },
    /// Connection closed
    Disconnected { code: Option<u16>, reason: String },
    /// Reconnection attempt started
    Reconnecting { attempt: u32 },
    /// Reconnection failed after max attempts
    ReconnectFailed { attempts: u32 },
    /// Heartbeat timeout: no inbound frame received within the configured
    /// `heartbeat_timeout` window. Emitted by the dispatch loop when the
    /// read-site timeout fires; the dispatch loop returns immediately
    /// afterwards, which lets the reconnect path take over.
    HeartbeatTimeout { elapsed: Duration },
    /// Error occurred
    Error { message: String, code: i32 },
}

/// Emit a [`ConnectionEvent`] on the bounded event channel.
///
/// The channel is `std::sync::mpsc::sync_channel(1024)`. We use `try_send`
/// here so that a stuck consumer can never block the connection task. On
/// saturation we drop the new event and surface a `stderr` warning so an
/// operator can detect the wedge.
///
/// Drop-newest is chosen over drop-oldest because `std::sync::mpsc` does
/// not expose receiver-side access to the sender, and switching to a
/// primitive that does (e.g. `tokio::sync::broadcast`) would break the
/// public `events()` / `state_events()` API shape that the binding crates
/// depend on. The cap of 1024 is large enough that saturation is itself
/// the bug signal — a healthy consumer never approaches it.
pub(crate) fn emit_event(tx: &mpsc::SyncSender<ConnectionEvent>, event: ConnectionEvent) {
    if let Err(mpsc::TrySendError::Full(dropped)) = tx.try_send(event) {
        eprintln!(
            "[fugle-marketdata-core] event channel saturated (cap=1024); \
             dropped {:?}. Consumer is likely stuck.",
            dropped
        );
    }
}
