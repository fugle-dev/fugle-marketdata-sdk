//! Test utilities: in-process WebSocket server.
//!
//! Gated behind `features = ["test-utils"]`. Depends on `tokio-comp` —
//! the mock is async-only because building a synchronous tokio-tungstenite
//! server would force a tokio runtime regardless.
//!
//! # Usage
//!
//! Add `core` as a dev-dependency with the feature flag, then spin up the
//! mock alongside a real `aio::WebSocketClient`:
//!
//! ```ignore
//! use marketdata_core::testing::{MockWsServer, aio_pair};
//! use marketdata_core::models::streaming::StreamMessage;
//!
//! # async fn example() {
//! let (server, client) = aio_pair().await;
//! client.connect().await.unwrap();
//! server.inject_frame(StreamMessage::Authenticated).await;
//! // ...assertions on client state...
//! server.close(1001, "going away").await;
//! # }
//! ```
//!
//! The mock implements the same subscribe-ACK protocol that
//! `core/src/websocket/protocol.rs` produces — both single-symbol and
//! batch forms. Whenever the wire protocol changes, the mock MUST be
//! updated in lockstep; the smoke test at
//! `core/tests/mock_server_smoke.rs` is the CI gate that catches drift.

#![cfg(all(feature = "test-utils", feature = "tokio-comp"))]
#![allow(missing_docs, reason = "internal test utilities — public for cross-crate test use")]

use crate::models::streaming::StreamMessage;
use crate::websocket::aio::WebSocketClient;
use crate::websocket::{ConnectionConfig, ReconnectionConfig};
use crate::AuthRequest;
use futures_util::{SinkExt, StreamExt};
use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, Mutex};
use tokio_tungstenite::tungstenite::Message;

/// Handle to the running mock server. Drop ends the accept loop; explicit
/// [`Self::close`] sends a WebSocket Close frame first.
pub struct MockWsServer {
    addr: SocketAddr,
    /// Pre-assigned subscribe-ack ids, FIFO. `next_subscribe_id(...)`
    /// pushes; the accept loop pops on each `subscribe` frame.
    pending_sub_ids: Arc<Mutex<VecDeque<String>>>,
    /// Outbound queue. `inject_frame` pushes; the accept loop drains
    /// onto the live socket.
    inject_tx: mpsc::UnboundedSender<MockInjection>,
}

enum MockInjection {
    Frame(StreamMessage),
    Close { code: u16, reason: String },
}

impl MockWsServer {
    /// Bind to an ephemeral 127.0.0.1 port and begin accepting one client.
    /// Returns immediately; the accept loop runs in a spawned tokio task.
    pub async fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind ephemeral port");
        let addr = listener.local_addr().expect("local_addr");

        let pending_sub_ids: Arc<Mutex<VecDeque<String>>> = Arc::new(Mutex::new(VecDeque::new()));
        let (inject_tx, inject_rx) = mpsc::unbounded_channel::<MockInjection>();

        let pending_clone = Arc::clone(&pending_sub_ids);
        tokio::spawn(async move {
            run_accept_loop(listener, pending_clone, inject_rx).await;
        });

        Self {
            addr,
            pending_sub_ids,
            inject_tx,
        }
    }

    /// `ws://127.0.0.1:<port>/marketdata/v1.0/stock/streaming`.
    ///
    /// The mock validates auth-message *shape* (must be JSON with an
    /// `event` field) but not the *content* — pass any non-empty token.
    pub fn url(&self) -> String {
        format!("ws://{}/marketdata/v1.0/stock/streaming", self.addr)
    }

    /// Underlying socket address.
    pub fn address(&self) -> SocketAddr {
        self.addr
    }

    /// Pre-assign the server-side id for the next subscribe ACK. Multiple
    /// calls queue FIFO; one id is consumed per subscribe frame received.
    pub async fn next_subscribe_id(&self, id: impl Into<String>) {
        self.pending_sub_ids.lock().await.push_back(id.into());
    }

    /// Push a stream message to be delivered to the connected client on
    /// its next read. Useful for fabricating `Data` / `Snapshot` events.
    pub async fn inject_frame(&self, frame: StreamMessage) {
        let _ = self.inject_tx.send(MockInjection::Frame(frame));
    }

    /// Initiate a server-side WebSocket Close handshake.
    pub async fn close(&self, code: u16, reason: impl Into<String>) {
        let _ = self.inject_tx.send(MockInjection::Close {
            code,
            reason: reason.into(),
        });
    }
}

/// Convenience: spin up a fresh [`MockWsServer`] and a paired
/// `aio::WebSocketClient` configured to talk to it. Auto-reconnect is
/// disabled so tests don't race with the reconnect path.
pub async fn aio_pair() -> (MockWsServer, WebSocketClient) {
    let server = MockWsServer::start().await;
    let auth = AuthRequest::with_api_key("mock-test-key");
    let config = ConnectionConfig::builder(server.url(), auth).build();
    let client = WebSocketClient::with_reconnection_config(config, ReconnectionConfig::disabled());
    (server, client)
}

async fn run_accept_loop(
    listener: TcpListener,
    pending_sub_ids: Arc<Mutex<VecDeque<String>>>,
    mut inject_rx: mpsc::UnboundedReceiver<MockInjection>,
) {
    let Ok((stream, _peer)) = listener.accept().await else {
        return;
    };
    let mut ws = match tokio_tungstenite::accept_async(stream).await {
        Ok(ws) => ws,
        Err(_) => return,
    };

    loop {
        tokio::select! {
            client_frame = ws.next() => {
                match client_frame {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                            let event = json.get("event").and_then(|v| v.as_str()).unwrap_or("");
                            match event {
                                "auth" => {
                                    let ack = serde_json::json!({ "event": "authenticated" });
                                    let _ = ws.send(Message::Text(ack.to_string().into())).await;
                                }
                                "subscribe" => {
                                    let id = pending_sub_ids
                                        .lock()
                                        .await
                                        .pop_front()
                                        .unwrap_or_else(|| "mock-id".to_string());
                                    let channel = json
                                        .get("channel")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("trades");
                                    let symbol = json
                                        .get("symbol")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("");
                                    let ack = serde_json::json!({
                                        "event": "subscribed",
                                        "id": id,
                                        "channel": channel,
                                        "symbol": symbol,
                                    });
                                    let _ = ws.send(Message::Text(ack.to_string().into())).await;
                                }
                                _ => {}
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Err(_)) => break,
                    _ => {}
                }
            }
            injection = inject_rx.recv() => {
                match injection {
                    Some(MockInjection::Frame(frame)) => {
                        if let Ok(text) = serde_json::to_string(&frame) {
                            let _ = ws.send(Message::Text(text.into())).await;
                        }
                    }
                    Some(MockInjection::Close { code, reason }) => {
                        let _ = ws
                            .send(Message::Close(Some(
                                tokio_tungstenite::tungstenite::protocol::CloseFrame {
                                    code: code.into(),
                                    reason: reason.into(),
                                },
                            )))
                            .await;
                        break;
                    }
                    None => break,
                }
            }
        }
    }
}
