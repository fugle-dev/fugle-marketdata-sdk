//! Shared in-process WebSocket test fixture for `core/tests/*.rs`.
//!
//! Spawns a local `tokio_tungstenite::accept_async` server on
//! `127.0.0.1:0` and returns the bound URL plus a `MockServerHandle`
//! that the test can drive (send frames, close cleanly, drop the
//! socket without ack).
//!
//! Cargo treats `tests/common/mod.rs` as a non-test module: each
//! integration test file pulls it in with `#[path = "common/mod.rs"]
//! mod common;` (or a plain `mod common;`).
//!
//! Only available under `--features tokio-comp`; the integration
//! tests that need it are gated behind the same feature.

#![cfg(feature = "tokio-comp")]
#![allow(dead_code)]

use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, Mutex};
use tokio_tungstenite::tungstenite::protocol::{frame::coding::CloseCode, CloseFrame};
use tokio_tungstenite::tungstenite::Message;

/// Behaviour the server applies after the auth handshake completes.
#[derive(Debug, Clone)]
pub enum AfterAuth {
    /// Park the connection — never send anything, never close.
    Idle,
    /// Send `count` `data` frames as fast as possible, then idle.
    FloodData { count: usize },
    /// After a brief delay, send a Close frame with the given code.
    ServerCloseAfter {
        delay_ms: u64,
        code: u16,
        reason: String,
    },
    /// After a brief delay, drop the TCP socket without a Close handshake.
    ServerDropAfter { delay_ms: u64 },
    /// Wait for `notify`, then after `delay_ms` send a Close frame. Lets a
    /// test land the server's Close while the client is mid-shutdown.
    ServerCloseOnNotify {
        notify: Arc<tokio::sync::Notify>,
        delay_ms: u64,
    },
    /// Idle until the client sends Close, then drop the TCP socket
    /// without acking it (a peer that skips the closing handshake).
    DropOnClientClose,
}

/// Collect items from `recv` until none arrives for [`QUIET`], capped at
/// [`DRAIN_LIMIT`] overall. Used to assert an event did *not* fire without
/// betting on one fixed sleep being long enough.
pub fn drain_until_quiet<T>(mut recv: impl FnMut(std::time::Duration) -> Option<T>) -> Vec<T> {
    let deadline = std::time::Instant::now() + DRAIN_LIMIT;
    let mut items = Vec::new();
    while std::time::Instant::now() < deadline {
        match recv(QUIET) {
            Some(item) => items.push(item),
            None => break,
        }
    }
    items
}

/// Silence window after which [`drain_until_quiet`] stops.
pub const QUIET: std::time::Duration = std::time::Duration::from_millis(500);
/// Upper bound on a single [`drain_until_quiet`] call.
pub const DRAIN_LIMIT: std::time::Duration = std::time::Duration::from_secs(5);

/// Handle returned to the test for inspecting / driving the server.
pub struct MockServerHandle {
    /// Full `ws://` URL the client should connect to (includes path).
    pub url: String,
    /// Channel that fires when the server-side socket has fully
    /// finished serving a connection (closed, dropped, or errored).
    pub done_rx: Mutex<mpsc::UnboundedReceiver<()>>,
}

/// Spawn a local mock WS server on `127.0.0.1:0`.
///
/// The server accepts a single connection, performs the auth
/// handshake (responds `{"event":"authenticated"}` to whatever the
/// client sends first), then applies `behaviour`.
pub async fn spawn(behaviour: AfterAuth) -> MockServerHandle {
    spawn_sequence(vec![behaviour]).await
}

/// Like [`spawn`], but accepts one connection per entry, in order, each
/// served with its own behaviour (e.g. to follow a client `reconnect()`).
pub async fn spawn_sequence(behaviours: Vec<AfterAuth>) -> MockServerHandle {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let port = listener.local_addr().expect("local_addr").port();
    let url = format!("ws://127.0.0.1:{}/", port);
    let (done_tx, done_rx) = mpsc::unbounded_channel();

    tokio::spawn(async move {
        for behaviour in behaviours {
            let Ok((stream, _)) = listener.accept().await else {
                return;
            };
            tokio::spawn(serve(stream, Arc::new(behaviour), done_tx.clone()));
        }
    });

    MockServerHandle {
        url,
        done_rx: Mutex::new(done_rx),
    }
}

async fn serve(
    stream: tokio::net::TcpStream,
    behaviour: Arc<AfterAuth>,
    done_tx: mpsc::UnboundedSender<()>,
) {
    let ws = match tokio_tungstenite::accept_async(stream).await {
        Ok(ws) => ws,
        Err(_) => {
            let _ = done_tx.send(());
            return;
        }
    };
    let (mut sink, mut stream) = ws.split();

    // Auth handshake: read first text frame, send "authenticated".
    if let Some(Ok(_first)) = stream.next().await {
        let _ = sink
            .send(Message::Text(
                r#"{"event":"authenticated"}"#.to_string().into(),
            ))
            .await;
    }

    match (*behaviour).clone() {
        AfterAuth::Idle => loop {
            match stream.next().await {
                Some(Ok(Message::Close(_))) => {
                    // tungstenite already queued the RFC-6455 Close
                    // ack; flush it so the client actually gets it
                    // (a second `send(Close)` would fail and drop
                    // the socket without the ack).
                    let _ = sink.close().await;
                    break;
                }
                Some(Ok(_)) => continue,
                _ => break,
            }
        },
        AfterAuth::FloodData { count } => {
            for i in 0..count {
                let payload = format!(
                    r#"{{"event":"data","channel":"trades","symbol":"X","id":"{}","data":{{"i":{}}}}}"#,
                    i, i
                );
                if sink.send(Message::Text(payload.into())).await.is_err() {
                    break;
                }
            }
            // After the burst, idle until the client closes.
            while let Some(msg) = stream.next().await {
                if let Ok(Message::Close(_)) = msg {
                    let _ = sink.send(Message::Close(None)).await;
                    break;
                }
            }
        }
        AfterAuth::ServerCloseAfter {
            delay_ms,
            code,
            reason,
        } => {
            tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
            let frame = CloseFrame {
                code: CloseCode::from(code),
                reason: reason.into(),
            };
            let _ = sink.send(Message::Close(Some(frame))).await;
            // Wait for the client's Close ack so the read half
            // gets the proper handshake completion event.
            while let Some(msg) = stream.next().await {
                if let Ok(Message::Close(_)) = msg {
                    break;
                }
            }
        }
        AfterAuth::ServerCloseOnNotify { notify, delay_ms } => {
            notify.notified().await;
            tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
            let _ = sink.send(Message::Close(None)).await;
            while let Some(Ok(msg)) = stream.next().await {
                if let Message::Close(_) = msg {
                    break;
                }
            }
        }
        AfterAuth::DropOnClientClose => {
            while let Some(Ok(msg)) = stream.next().await {
                if let Message::Close(_) = msg {
                    if let Ok(mut ws) = sink.reunite(stream) {
                        use tokio::io::AsyncWriteExt;
                        let _ = ws.get_mut().shutdown().await;
                    }
                    break;
                }
            }
        }
        AfterAuth::ServerDropAfter { delay_ms } => {
            tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
            // Reunite + close the underlying transport without
            // sending a Close frame to simulate a network drop.
            // Reuniting via SinkExt::reunite requires owning both
            // halves; the WebSocketStream returned from `unsplit`
            // can then be shutdown explicitly.
            if let Ok(ws) = sink.reunite(stream) {
                let mut inner = ws;
                // `WebSocketStream::get_mut()` exposes the
                // underlying `MaybeTlsStream<TcpStream>`. Calling
                // `shutdown()` triggers an immediate FIN on the
                // OS socket so the client's read half wakes up
                // with EOF instead of waiting for keep-alive.
                use tokio::io::AsyncWriteExt;
                let _ = inner.get_mut().shutdown().await;
                drop(inner);
            }
        }
    }

    let _ = done_tx.send(());
}
