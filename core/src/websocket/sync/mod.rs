//! Synchronous (blocking) WebSocket client.
//!
//! Always compiled. Default `WebSocketClient` in 0.3.0. No tokio dependency.
//! Uses `tungstenite` 0.29 (blocking) + a single owner thread per client.

pub mod client;
pub(crate) mod owner_thread;

pub use client::WebSocketClient;
