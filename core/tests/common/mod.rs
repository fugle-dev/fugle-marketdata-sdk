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

/// Behaviour the server applies after the auth handshake completes (or, for
/// [`AfterAuth::NeverAuthenticate`], in place of answering it).
#[derive(Debug, Clone)]
pub enum AfterAuth {
    /// Park the connection — never send anything, never close.
    Idle,
    /// Send `count` `data` frames as fast as possible, then idle.
    FloodData { count: usize },
    /// Send `count` `data` frames as fast as possible, then drop the TCP
    /// socket without a Close handshake.
    FloodDataThenDrop { count: usize },
    /// Send `count` `data` frames as fast as possible, then a Close frame
    /// with `code`.
    FloodDataThenClose { count: usize, code: u16 },
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
    /// Idle until the connection ends, then report on `ended_by_close`
    /// whether the client sent a Close frame (`true`) or dropped the
    /// socket without one (`false`).
    ReportClientClose {
        ended_by_close: mpsc::UnboundedSender<bool>,
    },
    /// Idle until the connection ends, forwarding every text frame the
    /// client sends on `frames`.
    RecordFrames {
        frames: mpsc::UnboundedSender<String>,
    },
    /// Like [`AfterAuth::RecordFrames`], and answer every `subscribe` frame
    /// with a `subscribed` ack: a `data` object for `symbol`, a `data` array
    /// with one entry per symbol for `symbols`. Each id is
    /// `<id_prefix>:<channel>:<symbol>`, so a test can tell which connection
    /// issued it.
    AckSubscribes {
        frames: mpsc::UnboundedSender<String>,
        id_prefix: String,
    },
    /// Like [`AfterAuth::AckSubscribes`], with each ack held until `notify`
    /// is signalled, so a test can unsubscribe before the ack arrives. Use
    /// `notify_one` per ack: its permit stands even if the ack is not waiting
    /// yet.
    AckSubscribesOnNotify {
        frames: mpsc::UnboundedSender<String>,
        id_prefix: String,
        notify: Arc<tokio::sync::Notify>,
    },
    /// Accept the WebSocket but never answer the auth frame: idle until the
    /// client gives the connection up (its auth timeout, #200).
    NeverAuthenticate,
    /// Answer the auth frame with the server's error shape,
    /// `{"event":"error","code":<code>,"data":{"message":<message>}}`, in
    /// place of `authenticated`. With `close`, follow it with a Close frame
    /// without a code, as the server does for `1000` and `1004`
    /// (`ws-exception.filter.ts`); otherwise idle until the client gives the
    /// connection up (#201).
    RejectAuth {
        code: i32,
        message: String,
        close: bool,
    },
    /// After a brief delay, send an `error` frame with `code` and then a
    /// Close frame without a code: the server's `error{1000}` +
    /// `client.close()` on an authenticated connection (#201).
    ErrorThenClose {
        delay_ms: u64,
        code: i32,
        message: String,
    },
    /// After a brief delay, send a Close frame without a code and nothing
    /// before it (#201 regression: no `error{1000}`, so the client
    /// reconnects).
    CloseWithoutCodeAfter { delay_ms: u64 },
}

/// The server's error frame: `code` at the top level, the message under
/// `data` (`ws-exception.filter.ts`).
pub fn error_frame(code: i32, message: &str) -> String {
    serde_json::json!({ "event": "error", "code": code, "data": { "message": message } }).to_string()
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

/// Send `count` `data` frames, numbered from 0, as fast as the sink takes them.
async fn send_data_frames<S>(sink: &mut S, count: usize)
where
    S: SinkExt<Message> + Unpin,
{
    for i in 0..count {
        let payload = format!(
            r#"{{"event":"data","channel":"trades","symbol":"X","id":"{}","data":{{"i":{}}}}}"#,
            i, i
        );
        if sink.send(Message::Text(payload.into())).await.is_err() {
            break;
        }
    }
}

/// The `subscribed` ack for a `subscribe` frame, or `None` for any other
/// frame. Panics on a `subscribe` frame it cannot read, so a malformed frame
/// fails the test instead of silently going unacked.
fn subscribed_ack(frame: &str, id_prefix: &str) -> Option<String> {
    let frame: serde_json::Value = serde_json::from_str(frame).ok()?;
    if frame["event"] != "subscribe" {
        return None;
    }
    let data = &frame["data"];
    let channel = data["channel"].as_str().unwrap_or_else(|| panic!("subscribe without channel: {frame}"));
    let entry = |symbol: &serde_json::Value| {
        let symbol = symbol.as_str().unwrap_or_else(|| panic!("subscribe without symbol: {frame}"));
        let mut entry = data.clone();
        let entry_obj = entry.as_object_mut().expect("subscribe data is an object");
        entry_obj.remove("symbols");
        entry_obj.insert("symbol".into(), symbol.into());
        entry_obj.insert("id".into(), format!("{id_prefix}:{channel}:{symbol}").into());
        entry
    };
    let data = match data["symbols"].as_array() {
        Some(symbols) => serde_json::Value::Array(symbols.iter().map(entry).collect()),
        None => entry(&data["symbol"]),
    };
    Some(serde_json::json!({"event": "subscribed", "data": data}).to_string())
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
    // `NeverAuthenticate` swallows the frame and idles instead;
    // `RejectAuth` answers with an error frame.
    if let Some(Ok(_first)) = stream.next().await {
        let answer = match &*behaviour {
            AfterAuth::NeverAuthenticate => None,
            AfterAuth::RejectAuth { code, message, .. } => Some(error_frame(*code, message)),
            _ => Some(r#"{"event":"authenticated"}"#.to_string()),
        };
        if let Some(answer) = answer {
            let _ = sink.send(Message::Text(answer.into())).await;
        }
    }

    match (*behaviour).clone() {
        AfterAuth::RejectAuth { close: true, .. } => {
            // `client.close()`: a Close frame without a code.
            let _ = sink.send(Message::Close(None)).await;
            while let Some(Ok(msg)) = stream.next().await {
                if let Message::Close(_) = msg {
                    break;
                }
            }
        }
        AfterAuth::ErrorThenClose { delay_ms, code, message } => {
            tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
            let _ = sink.send(Message::Text(error_frame(code, &message).into())).await;
            let _ = sink.send(Message::Close(None)).await;
            while let Some(Ok(msg)) = stream.next().await {
                if let Message::Close(_) = msg {
                    break;
                }
            }
        }
        AfterAuth::CloseWithoutCodeAfter { delay_ms } => {
            tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
            let _ = sink.send(Message::Close(None)).await;
            while let Some(Ok(msg)) = stream.next().await {
                if let Message::Close(_) = msg {
                    break;
                }
            }
        }
        AfterAuth::Idle | AfterAuth::NeverAuthenticate | AfterAuth::RejectAuth { .. } => loop {
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
            send_data_frames(&mut sink, count).await;
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
        AfterAuth::ReportClientClose { ended_by_close } => {
            let by_close = loop {
                match stream.next().await {
                    Some(Ok(Message::Close(_))) => {
                        let _ = sink.close().await;
                        break true;
                    }
                    Some(Ok(_)) => continue,
                    _ => break false,
                }
            };
            let _ = ended_by_close.send(by_close);
        }
        AfterAuth::RecordFrames { frames } => loop {
            match stream.next().await {
                Some(Ok(Message::Text(text))) => {
                    let _ = frames.send(text.to_string());
                }
                Some(Ok(Message::Close(_))) => {
                    let _ = sink.close().await;
                    break;
                }
                Some(Ok(_)) => continue,
                _ => break,
            }
        },
        AfterAuth::AckSubscribes { frames, id_prefix } => loop {
            match stream.next().await {
                Some(Ok(Message::Text(text))) => {
                    let _ = frames.send(text.to_string());
                    if let Some(ack) = subscribed_ack(&text, &id_prefix) {
                        let _ = sink.send(Message::Text(ack.into())).await;
                    }
                }
                Some(Ok(Message::Close(_))) => {
                    let _ = sink.close().await;
                    break;
                }
                Some(Ok(_)) => continue,
                _ => break,
            }
        },
        AfterAuth::AckSubscribesOnNotify { frames, id_prefix, notify } => loop {
            match stream.next().await {
                Some(Ok(Message::Text(text))) => {
                    let _ = frames.send(text.to_string());
                    if let Some(ack) = subscribed_ack(&text, &id_prefix) {
                        notify.notified().await;
                        let _ = sink.send(Message::Text(ack.into())).await;
                    }
                }
                Some(Ok(Message::Close(_))) => {
                    let _ = sink.close().await;
                    break;
                }
                Some(Ok(_)) => continue,
                _ => break,
            }
        },
        AfterAuth::FloodDataThenClose { count, code } => {
            send_data_frames(&mut sink, count).await;
            let frame = CloseFrame {
                code: CloseCode::from(code),
                reason: "flood done".into(),
            };
            let _ = sink.send(Message::Close(Some(frame))).await;
            while let Some(msg) = stream.next().await {
                if let Ok(Message::Close(_)) = msg {
                    break;
                }
            }
        }
        AfterAuth::FloodDataThenDrop { count } => {
            send_data_frames(&mut sink, count).await;
            if let Ok(mut ws) = sink.reunite(stream) {
                use tokio::io::AsyncWriteExt;
                let _ = ws.get_mut().shutdown().await;
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

/// Every item of a client's stream, as a compact label: `m<id>` for a
/// message with an id, `m:<event>` for one without, and the event's `Debug`
/// otherwise. Makes order assertions readable.
pub fn label(item: &marketdata_core::StreamItem) -> String {
    match item {
        marketdata_core::StreamItem::Message(m) => match &m.id {
            Some(id) => format!("m{id}"),
            None => format!("m:{}", m.event),
        },
        marketdata_core::StreamItem::Event(e) => format!("{e:?}"),
        _ => "unknown".to_string(),
    }
}

/// The events of a client's stream, skipping its messages: the shape tests
/// used before `events()` was folded into the stream (#68).
///
/// It reads the client's one `stream_receiver()` and discards every message
/// it passes. Do not use it together with [`MessageReceiver`] (or another
/// reader of the same stream) on one client: each would silently swallow the
/// items the other is waiting for. Read the stream directly instead.
pub struct EventReceiver(pub std::sync::Arc<marketdata_core::StreamReceiver>);

impl EventReceiver {
    pub fn of_async(client: &marketdata_core::aio::WebSocketClient) -> Self {
        Self(client.stream_receiver())
    }

    pub fn of_sync(client: &marketdata_core::WebSocketClient) -> Self {
        Self(client.stream_receiver())
    }

    /// Next event within `timeout`.
    pub fn recv_timeout(
        &self,
        timeout: std::time::Duration,
    ) -> Result<marketdata_core::ConnectionEvent, std::sync::mpsc::RecvTimeoutError> {
        use std::sync::mpsc::RecvTimeoutError;
        let deadline = std::time::Instant::now() + timeout;
        loop {
            let left = deadline.saturating_duration_since(std::time::Instant::now());
            match self.0.receive_timeout(left) {
                Ok(Some(marketdata_core::StreamItem::Event(event))) => return Ok(event),
                Ok(Some(_)) => continue,
                Ok(None) => return Err(RecvTimeoutError::Timeout),
                Err(_) => return Err(RecvTimeoutError::Disconnected),
            }
        }
    }

    /// Next event, waiting for one; `Err` once the stream is closed.
    pub fn recv(&self) -> Result<marketdata_core::ConnectionEvent, marketdata_core::MarketDataError> {
        loop {
            if let marketdata_core::StreamItem::Event(event) = self.0.receive()? {
                return Ok(event);
            }
        }
    }

    /// Next queued event, without waiting.
    pub fn try_recv(&self) -> Option<marketdata_core::ConnectionEvent> {
        while let Some(item) = self.0.try_receive() {
            if let marketdata_core::StreamItem::Event(event) = item {
                return Some(event);
            }
        }
        None
    }

    /// Every queued event, without waiting.
    pub fn try_iter(&self) -> impl Iterator<Item = marketdata_core::ConnectionEvent> + '_ {
        std::iter::from_fn(|| self.try_recv())
    }
}

/// The messages of a client's stream, skipping its events.
///
/// Like [`EventReceiver`], it discards what it passes over: never combine the
/// two on one client.
pub struct MessageReceiver(pub std::sync::Arc<marketdata_core::StreamReceiver>);

impl MessageReceiver {
    pub fn of_async(client: &marketdata_core::aio::WebSocketClient) -> Self {
        Self(client.stream_receiver())
    }

    /// Next message within `timeout`: `Ok(None)` on timeout, `Err` once the
    /// stream is closed.
    pub fn receive_timeout(
        &self,
        timeout: std::time::Duration,
    ) -> Result<Option<marketdata_core::WebSocketMessage>, marketdata_core::MarketDataError> {
        let deadline = std::time::Instant::now() + timeout;
        loop {
            let left = deadline.saturating_duration_since(std::time::Instant::now());
            match self.0.receive_timeout(left)? {
                Some(marketdata_core::StreamItem::Message(message)) => return Ok(Some(message)),
                Some(_) => continue,
                None => return Ok(None),
            }
        }
    }

    /// Next queued message, without waiting.
    pub fn try_receive(&self) -> Option<marketdata_core::WebSocketMessage> {
        while let Some(item) = self.0.try_receive() {
            if let marketdata_core::StreamItem::Message(message) = item {
                return Some(message);
            }
        }
        None
    }
}
