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
