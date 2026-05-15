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
//! # Multi-client topology
//!
//! Construct a server that accepts up to N clients on a single port using
//! [`MockWsServer::start_with_capacity`] and the convenience pair-helper
//! [`aio_pair_n`]:
//!
//! ```ignore
//! use marketdata_core::testing::{MockWsServer, aio_pair_n};
//! use marketdata_core::models::streaming::StreamMessage;
//!
//! # async fn example() {
//! let (server, clients) = aio_pair_n(2).await;
//! for c in &clients {
//!     c.connect().await.unwrap();
//! }
//! server.inject_frame_for(0, StreamMessage::Authenticated).await;
//! server.inject_frame_for(1, StreamMessage::Authenticated).await;
//! # }
//! ```
//!
//! When the server is constructed with `capacity > 1`, the bare
//! `inject_frame` / `next_subscribe_id` / `close` methods panic — every
//! injection MUST target a specific client via the `_for(idx, ...)`
//! variant. The single-client `start()` / `aio_pair()` constructors
//! preserve the 0.6 surface for callers that only need one client.
//!
//! # Disconnect intent injection
//!
//! [`MockWsServer::close`] sends a graceful WebSocket Close frame, which
//! the SDK observes as [`crate::DisconnectIntent::Server`].
//! [`MockWsServer::drop_transport`] (and `drop_transport_for`) closes the
//! underlying TCP socket without a Close frame, forcing
//! [`crate::DisconnectIntent::Network`]. The two cover every server-side
//! disconnect intent the SDK can emit; client-side
//! [`crate::DisconnectIntent::Client`] is reachable by calling
//! [`crate::aio::WebSocketClient::disconnect`].
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
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio_tungstenite::tungstenite::Message;

/// Per-client handle stored on [`MockWsServer`]. Owned by the server,
/// referenced by `_for(idx, ...)` methods.
struct ClientHandle {
    /// FIFO queue of pre-assigned subscribe-ack ids for this specific
    /// client. `next_subscribe_id_for` pushes; the per-client task pops.
    pending_sub_ids: Arc<Mutex<VecDeque<String>>>,
    /// Outbound injection channel for this client. `inject_frame_for` /
    /// `close_for` push; the per-client task drains.
    inject_tx: mpsc::UnboundedSender<MockInjection>,
    /// One-shot signal that, when fired, instructs the per-client task to
    /// drop its `WebSocketStream` (and the underlying `TcpStream`)
    /// immediately, producing `DisconnectIntent::Network` on the client
    /// side. Wrapped in a `Mutex<Option<…>>` because `oneshot::Sender`
    /// consumes itself on `send`.
    transport_drop: Mutex<Option<oneshot::Sender<()>>>,
}

/// Handle to the running mock server. Drop ends the accept loop and all
/// per-client tasks; explicit [`Self::close`] / [`Self::close_for`] sends
/// a WebSocket Close frame first.
pub struct MockWsServer {
    addr: SocketAddr,
    clients: Vec<ClientHandle>,
}

enum MockInjection {
    Frame(StreamMessage),
    Close { code: u16, reason: String },
}

impl MockWsServer {
    /// Bind to an ephemeral 127.0.0.1 port and begin accepting one client.
    /// Equivalent to [`Self::start_with_capacity(1)`](Self::start_with_capacity).
    pub async fn start() -> Self {
        Self::start_with_capacity(1).await
    }

    /// Bind to an ephemeral 127.0.0.1 port and begin accepting up to
    /// `capacity` clients. Each accepted client is assigned a zero-based
    /// `client_idx` in accept order; injection methods take the index to
    /// target a specific client.
    ///
    /// # Panics
    ///
    /// Panics on `capacity == 0` — a zero-capacity mock cannot make any
    /// progress and is always a configuration mistake.
    pub async fn start_with_capacity(capacity: usize) -> Self {
        assert!(
            capacity > 0,
            "MockWsServer::start_with_capacity: capacity must be > 0"
        );

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind ephemeral port");
        let addr = listener.local_addr().expect("local_addr");

        // Build per-client handles up-front. Each gets its own injection
        // channel + transport-drop oneshot. The accept loop receives the
        // matching halves and dispatches them as connections arrive.
        let mut clients = Vec::with_capacity(capacity);
        let mut accept_seeds = Vec::with_capacity(capacity);

        for _ in 0..capacity {
            let pending_sub_ids: Arc<Mutex<VecDeque<String>>> =
                Arc::new(Mutex::new(VecDeque::new()));
            let (inject_tx, inject_rx) = mpsc::unbounded_channel::<MockInjection>();
            let (drop_tx, drop_rx) = oneshot::channel::<()>();

            clients.push(ClientHandle {
                pending_sub_ids: Arc::clone(&pending_sub_ids),
                inject_tx,
                transport_drop: Mutex::new(Some(drop_tx)),
            });
            accept_seeds.push(AcceptSeed {
                pending_sub_ids,
                inject_rx,
                drop_rx,
            });
        }

        tokio::spawn(async move {
            run_accept_loop(listener, accept_seeds).await;
        });

        Self { addr, clients }
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

    /// Configured client capacity (number of accept slots, set at
    /// construction by [`Self::start_with_capacity`]).
    pub fn capacity(&self) -> usize {
        self.clients.len()
    }

    /// Pre-assign the server-side id for the next subscribe ACK on a
    /// single-client mock (`capacity == 1`). Multiple calls queue FIFO.
    ///
    /// # Panics
    ///
    /// Panics if the mock was constructed with `capacity > 1`. Use
    /// [`Self::next_subscribe_id_for`] to target a specific client.
    pub async fn next_subscribe_id(&self, id: impl Into<String>) {
        self.assert_single_client("next_subscribe_id");
        self.next_subscribe_id_for(0, id).await;
    }

    /// Pre-assign the server-side id for the next subscribe ACK on the
    /// `client_idx`-th client. Multiple calls queue FIFO per client.
    ///
    /// # Panics
    ///
    /// Panics if `client_idx >= capacity`.
    pub async fn next_subscribe_id_for(&self, client_idx: usize, id: impl Into<String>) {
        let client = self.client_or_panic(client_idx, "next_subscribe_id_for");
        client.pending_sub_ids.lock().await.push_back(id.into());
    }

    /// Push a stream message to be delivered to the connected client on
    /// its next read. Single-client convenience.
    ///
    /// # Panics
    ///
    /// Panics if the mock was constructed with `capacity > 1`. Use
    /// [`Self::inject_frame_for`] to target a specific client.
    pub async fn inject_frame(&self, frame: StreamMessage) {
        self.assert_single_client("inject_frame");
        self.inject_frame_for(0, frame).await;
    }

    /// Push a stream message targeted at the `client_idx`-th client. After
    /// `drop_transport_for(client_idx)` or `close_for(client_idx)` the
    /// per-client task has exited; subsequent calls become no-ops (the
    /// channel send is dropped silently).
    ///
    /// # Panics
    ///
    /// Panics if `client_idx >= capacity`.
    pub async fn inject_frame_for(&self, client_idx: usize, frame: StreamMessage) {
        let client = self.client_or_panic(client_idx, "inject_frame_for");
        let _ = client.inject_tx.send(MockInjection::Frame(frame));
    }

    /// Initiate a server-side WebSocket Close handshake on a single-client
    /// mock. Produces `DisconnectIntent::Server` on the client side.
    ///
    /// # Panics
    ///
    /// Panics if the mock was constructed with `capacity > 1`. Use
    /// [`Self::close_for`] to target a specific client.
    pub async fn close(&self, code: u16, reason: impl Into<String>) {
        self.assert_single_client("close");
        self.close_for(0, code, reason).await;
    }

    /// Initiate a server-side WebSocket Close handshake on the
    /// `client_idx`-th client. Produces `DisconnectIntent::Server` on the
    /// client side.
    ///
    /// # Panics
    ///
    /// Panics if `client_idx >= capacity`.
    pub async fn close_for(&self, client_idx: usize, code: u16, reason: impl Into<String>) {
        let client = self.client_or_panic(client_idx, "close_for");
        let _ = client.inject_tx.send(MockInjection::Close {
            code,
            reason: reason.into(),
        });
    }

    /// Close the underlying TCP socket of a single-client mock without
    /// sending a WebSocket Close frame. Produces
    /// `DisconnectIntent::Network` on the client side. Idempotent — calling
    /// twice is a no-op.
    ///
    /// # Panics
    ///
    /// Panics if the mock was constructed with `capacity > 1`. Use
    /// [`Self::drop_transport_for`] to target a specific client.
    pub async fn drop_transport(&self) {
        self.assert_single_client("drop_transport");
        self.drop_transport_for(0).await;
    }

    /// Close the underlying TCP socket of the `client_idx`-th client
    /// without sending a WebSocket Close frame. Produces
    /// `DisconnectIntent::Network` on the client side. Idempotent.
    ///
    /// # Panics
    ///
    /// Panics if `client_idx >= capacity`.
    pub async fn drop_transport_for(&self, client_idx: usize) {
        let client = self.client_or_panic(client_idx, "drop_transport_for");
        let mut slot = client.transport_drop.lock().await;
        if let Some(tx) = slot.take() {
            // Receiver may already have been dropped (per-client task
            // exited via Close frame); ignore the error.
            let _ = tx.send(());
        }
    }

    fn assert_single_client(&self, method: &str) {
        assert!(
            self.clients.len() == 1,
            "MockWsServer::{method} requires capacity == 1; use {method}_for(client_idx, ...) for multi-client mocks"
        );
    }

    fn client_or_panic(&self, client_idx: usize, method: &str) -> &ClientHandle {
        let capacity = self.clients.len();
        self.clients.get(client_idx).unwrap_or_else(|| {
            panic!(
                "MockWsServer::{method}: client_idx {client_idx} out of range (capacity = {capacity})"
            )
        })
    }
}

/// Per-client state seeded into the accept loop.
struct AcceptSeed {
    pending_sub_ids: Arc<Mutex<VecDeque<String>>>,
    inject_rx: mpsc::UnboundedReceiver<MockInjection>,
    drop_rx: oneshot::Receiver<()>,
}

/// Convenience: spin up a fresh single-client [`MockWsServer`] and a
/// paired `aio::WebSocketClient` configured to talk to it. Auto-reconnect
/// is disabled so tests don't race with the reconnect path.
pub async fn aio_pair() -> (MockWsServer, WebSocketClient) {
    let (server, mut clients) = aio_pair_n(1).await;
    let client = clients.remove(0);
    (server, client)
}

/// Convenience: spin up a [`MockWsServer`] with capacity `n` and `n`
/// `aio::WebSocketClient`s all targeted at the same server URL.
/// Auto-reconnect is disabled on every client.
///
/// # Panics
///
/// Panics if `n == 0`.
pub async fn aio_pair_n(n: usize) -> (MockWsServer, Vec<WebSocketClient>) {
    let server = MockWsServer::start_with_capacity(n).await;
    let mut clients = Vec::with_capacity(n);
    for _ in 0..n {
        let auth = AuthRequest::with_api_key("mock-test-key");
        let config = ConnectionConfig::builder(server.url(), auth).build();
        let client = WebSocketClient::with_reconnection_config(config, ReconnectionConfig::disabled());
        clients.push(client);
    }
    (server, clients)
}

async fn run_accept_loop(listener: TcpListener, seeds: Vec<AcceptSeed>) {
    for seed in seeds {
        let Ok((stream, _peer)) = listener.accept().await else {
            // Listener closed (server dropped) — abort remaining seeds.
            return;
        };
        let ws = match tokio_tungstenite::accept_async(stream).await {
            Ok(ws) => ws,
            Err(_) => continue,
        };
        tokio::spawn(run_client_loop(ws, seed));
    }
}

async fn run_client_loop(
    mut ws: tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
    seed: AcceptSeed,
) {
    let AcceptSeed {
        pending_sub_ids,
        mut inject_rx,
        mut drop_rx,
    } = seed;

    loop {
        tokio::select! {
            // Transport-drop signal: drop the WebSocket stream immediately
            // so the inner TcpStream is dropped (kernel sends FIN/RST and
            // the client's read returns ConnectionReset). Producing
            // DisconnectIntent::Network on the client side.
            _ = &mut drop_rx => {
                drop(ws);
                return;
            }
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
