//! Async (tokio) WebSocket client implementation.
//!
//! In 0.3.0 this module will be feature-gated behind `tokio-comp`. For now
//! (PR2 of the redis-rs-style refactor) it is always compiled so that the
//! existing public API (`marketdata_core::WebSocketClient` etc.) keeps
//! working.

pub mod client;
pub mod dispatch;
pub mod reconnect;
pub mod runtime;
pub mod writer;

pub use client::{WebSocketClient, DEFAULT_SHUTDOWN_TIMEOUT};
pub use runtime::AsyncRuntime;

use futures_util::stream::{SplitSink, SplitStream};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

/// WebSocket write half (tokio-tungstenite split sink).
pub(crate) type WsSink = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;
/// WebSocket read half (tokio-tungstenite split stream).
pub(crate) type WsStream = SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;

/// Connection state shared by the client and its background tasks.
///
/// A plain `std` lock, not a tokio one: every critical section is a single
/// read or assignment, and the synchronous getters (`state()`,
/// `is_closed_sync()`) must work on and off a runtime thread without
/// `block_on` (#33). Take guards through [`read_state`] / [`write_state`]
/// in a block that does not `.await`.
pub(crate) type SharedState = std::sync::Arc<std::sync::RwLock<crate::websocket::ConnectionState>>;

/// Read guard on [`SharedState`]. A poisoned lock still yields the value:
/// writers only assign, so it can never be left half-updated.
pub(crate) fn read_state(
    state: &SharedState,
) -> std::sync::RwLockReadGuard<'_, crate::websocket::ConnectionState> {
    state.read().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Write guard on [`SharedState`]; see [`read_state`] for poisoning.
pub(crate) fn write_state(
    state: &SharedState,
) -> std::sync::RwLockWriteGuard<'_, crate::websocket::ConnectionState> {
    state.write().unwrap_or_else(std::sync::PoisonError::into_inner)
}
