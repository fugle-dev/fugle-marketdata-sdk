//! The client's stream under load (#46).
//!
//! - A consumer on the client's own runtime keeps up with an unpaced flood:
//!   the dispatch loop must yield instead of starving it.
//! - `MessageOverflow::DropNewest` holds exactly `message_buffer` messages,
//!   counts every drop and reports them with `MessagesDropped`, and never
//!   costs an event.
//! - `MessageOverflow::Unbounded` never drops.
//! - A full queue does not stall the auth handshake of a reconnect.
//! - The drop count covers the current connection: it survives the end of
//!   the connection and restarts from zero on the next one.

#![cfg(feature = "tokio-comp")]

#[path = "common/mod.rs"]
mod common;

use futures_util::StreamExt;
use marketdata_core::aio::WebSocketClient;
use marketdata_core::websocket::{ConnectionConfigBuilder, ConnectionEvent};
use marketdata_core::{
    AuthRequest, ConnectionConfig, MessageOverflow, ReconnectionConfig, StreamItem, StreamReceiver,
};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

/// Frames in a flood: far more than the default 4096-message queue.
const FLOOD: usize = 50_000;
/// Longest wait for any single expected message or event.
const WAIT: Duration = Duration::from_secs(5);

fn client_for(
    url: &str,
    configure: impl FnOnce(ConnectionConfigBuilder) -> ConnectionConfigBuilder,
) -> WebSocketClient {
    let builder = ConnectionConfig::builder(url, AuthRequest::with_api_key("test-key"));
    WebSocketClient::with_reconnection_config(configure(builder).build(), ReconnectionConfig::disabled())
}

/// A flood server on a runtime of its own, so it keeps writing no matter
/// how the client's runtime schedules. Serves one connection of `count`
/// `data` frames, then idles until the client closes.
fn flood_server_on_own_runtime(count: usize) -> String {
    let (url_tx, url_rx) = mpsc::channel();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .expect("server runtime");
        rt.block_on(async {
            let server = common::spawn(common::AfterAuth::FloodData { count }).await;
            url_tx.send(server.url.clone()).expect("send url");
            server.done_rx.lock().await.recv().await;
        });
    });
    url_rx.recv().expect("server url")
}

/// Items read off a stream so far.
struct Seen {
    rx: Arc<StreamReceiver>,
    items: Vec<StreamItem>,
}

impl Seen {
    fn new(rx: Arc<StreamReceiver>) -> Self {
        Self { rx, items: Vec::new() }
    }

    /// Take everything queued, without waiting.
    fn pull(&mut self) -> &[StreamItem] {
        self.items.extend(std::iter::from_fn(|| self.rx.try_receive()));
        &self.items
    }

    /// Poll (without blocking the runtime) until `done` holds, up to [`WAIT`].
    async fn until(&mut self, done: impl Fn(&[StreamItem]) -> bool) -> bool {
        let deadline = Instant::now() + WAIT;
        loop {
            if done(self.pull()) {
                return true;
            }
            if Instant::now() >= deadline {
                return false;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    fn labels(&self) -> Vec<String> {
        self.items.iter().map(common::label).collect()
    }
}

fn events(items: &[StreamItem]) -> impl Iterator<Item = &ConnectionEvent> {
    items.iter().filter_map(|item| match item {
        StreamItem::Event(event) => Some(event),
        _ => None,
    })
}

fn messages(items: &[StreamItem]) -> usize {
    items.iter().filter(|item| matches!(item, StreamItem::Message(_))).count()
}

fn count(items: &[StreamItem], matches: fn(&ConnectionEvent) -> bool) -> usize {
    events(items).filter(|e| matches(e)).count()
}

fn is_authenticated(event: &ConnectionEvent) -> bool {
    matches!(event, ConnectionEvent::Authenticated { .. })
}

fn is_disconnected(event: &ConnectionEvent) -> bool {
    matches!(event, ConnectionEvent::Disconnected { .. })
}

fn drop_reports(items: &[StreamItem]) -> Vec<(u64, u64)> {
    events(items)
        .filter_map(|event| match event {
            ConnectionEvent::MessagesDropped { dropped, total } => Some((*dropped, *total)),
            _ => None,
        })
        .collect()
}

async fn stream_keeps_up_with_flood() {
    let url = flood_server_on_own_runtime(FLOOD);
    let client = client_for(&url, |b| b);
    let mut stream = client.stream();
    client.connect().await.expect("connect");

    // Consumed as a `futures::Stream`, on the client's runtime.
    let consumer = tokio::spawn(async move {
        let mut data = 0;
        while data < FLOOD {
            match tokio::time::timeout(WAIT, stream.next()).await {
                Ok(Some(StreamItem::Message(msg))) if msg.event == "data" => data += 1,
                Ok(Some(_)) => {}
                _ => break,
            }
        }
        data
    });
    let data = consumer.await.expect("consumer");

    assert_eq!((data, client.messages_dropped_total()), (FLOOD, 0));
    let _ = client.shutdown_with_timeout(Duration::from_millis(500)).await;
}

#[tokio::test(flavor = "current_thread")]
async fn stream_keeps_up_with_flood_on_current_thread_runtime() {
    stream_keeps_up_with_flood().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stream_keeps_up_with_flood_on_multi_thread_runtime() {
    stream_keeps_up_with_flood().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stream_receiver_keeps_up_with_flood() {
    let url = flood_server_on_own_runtime(FLOOD);
    let client = client_for(&url, |b| b);
    let receiver = client.stream_receiver();
    client.connect().await.expect("connect");

    let data = tokio::task::spawn_blocking(move || {
        let mut data = 0;
        while data < FLOOD {
            match receiver.receive_timeout(WAIT) {
                Ok(Some(StreamItem::Message(msg))) if msg.event == "data" => data += 1,
                Ok(Some(_)) => {}
                _ => break,
            }
        }
        data
    })
    .await
    .expect("consumer");

    assert_eq!((data, client.messages_dropped_total()), (FLOOD, 0));
    let _ = client.shutdown_with_timeout(Duration::from_millis(500)).await;
}

#[tokio::test]
async fn full_queue_drops_newest_reports_in_place_and_keeps_every_event() {
    let server = common::spawn(common::AfterAuth::FloodData { count: 100 }).await;
    let client = client_for(&server.url, |b| b.message_buffer(8));
    let dropped = client.messages_dropped_handle();
    client.connect().await.expect("connect");

    // Nothing reads the stream: `authenticated` and 7 data frames fit, the
    // other 93 data frames are dropped.
    let deadline = Instant::now() + WAIT;
    while client.messages_dropped_total() < 93 && Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    client.disconnect().await.expect("disconnect");
    assert_eq!(client.messages_dropped_total(), 93, "count survives disconnect()");
    assert_eq!(client.events_dropped_total(), 0);

    let mut seen = Seen::new(client.stream_receiver());
    seen.pull();
    let labels = seen.labels();
    let dropped_at_once = format!("{:?}", ConnectionEvent::MessagesDropped { dropped: 1, total: 1 });
    let expected_head = vec![
        "Connecting".to_string(),
        "Connected".into(),
        "Authenticated { data: Null }".into(),
        "m:authenticated".into(),
        "m0".into(),
        "m1".into(),
        "m2".into(),
        "m3".into(),
        "m4".into(),
        "m5".into(),
        "m6".into(),
        // Reported at once, right where the first drop happened.
        dropped_at_once,
    ];
    assert_eq!(labels[..expected_head.len()], expected_head[..], "{labels:?}");
    // The rest is reported before `Disconnected`, which comes last.
    let reports = drop_reports(&seen.items);
    assert_eq!(reports.iter().map(|(dropped, _)| dropped).sum::<u64>(), 93, "{labels:?}");
    assert_eq!(reports.last().map(|r| r.1), Some(93), "{labels:?}");
    assert!(labels.last().expect("items").starts_with("Disconnected"), "{labels:?}");

    // The handle outlives the client.
    drop(seen);
    drop(client);
    assert_eq!(dropped.total(), 93);
}

#[tokio::test]
async fn unbounded_queue_never_drops() {
    let server = common::spawn(common::AfterAuth::FloodData { count: 100 }).await;
    let client = client_for(&server.url, |b| {
        b.message_buffer(8).message_overflow(MessageOverflow::Unbounded)
    });
    client.connect().await.expect("connect");
    let mut seen = Seen::new(client.stream_receiver());

    // Let the flood queue up before reading any of it; a bounded queue of
    // 8 would have dropped most of it by then.
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(seen.until(|items| messages(items) == 101).await, "{:?}", seen.labels());
    client.disconnect().await.expect("disconnect");

    assert_eq!(client.messages_dropped_total(), 0);
    assert_eq!(drop_reports(seen.pull()), vec![]);
}

#[tokio::test]
async fn reconnect_authenticates_while_the_queue_is_full() {
    let server = common::spawn_sequence(vec![
        common::AfterAuth::FloodDataThenDrop { count: 20 },
        common::AfterAuth::Idle,
    ])
    .await;
    let config = ConnectionConfig::builder(&server.url, AuthRequest::with_api_key("test-key"))
        .message_buffer(4)
        .build();
    let reconnect = ReconnectionConfig::new(1, Duration::from_millis(100), Duration::from_millis(100))
        .expect("reconnection config");
    let client = WebSocketClient::with_reconnection_config(config, reconnect);
    let receiver = client.stream_receiver();
    client.connect().await.expect("connect");

    // Nothing reads messages, so the queue stays full. The handshake of the
    // reconnect drops its `authenticated` frame instead of blocking until
    // its 10 s timeout.
    let deadline = Instant::now() + WAIT;
    while client.state() != marketdata_core::ConnectionState::Connected
        || client.messages_dropped_total() != 1
    {
        assert!(Instant::now() < deadline, "no reconnect: {:?}", client.state());
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    client.disconnect().await.expect("disconnect");

    let mut seen = Seen::new(receiver);
    seen.pull();
    assert_eq!(count(&seen.items, is_authenticated), 2, "{:?}", seen.labels());
    // The first connection dropped 17 data frames; the second only its
    // `authenticated` frame, counted from zero.
    assert_eq!(client.messages_dropped_total(), 1);
    let reports = drop_reports(&seen.items);
    assert_eq!(reports.iter().map(|(dropped, _)| dropped).sum::<u64>(), 18, "{:?}", seen.labels());
    assert_eq!(reports.last(), Some(&(1, 1)), "{:?}", seen.labels());
}

/// Reconnect policy slow enough to inspect the client between connections.
fn slow_single_reconnect() -> ReconnectionConfig {
    ReconnectionConfig::new(1, Duration::from_millis(500), Duration::from_millis(500))
        .expect("reconnection config")
}

#[tokio::test]
async fn drop_count_survives_the_connection_and_restarts_on_reconnect() {
    let server = common::spawn_sequence(vec![
        common::AfterAuth::FloodDataThenDrop { count: 100 },
        common::AfterAuth::Idle,
    ])
    .await;
    let config = ConnectionConfig::builder(&server.url, AuthRequest::with_api_key("test-key"))
        .message_buffer(8)
        .build();
    let client = WebSocketClient::with_reconnection_config(config, slow_single_reconnect());
    let receiver = client.stream_receiver();
    client.connect().await.expect("connect");

    // Wait for the first connection to end without reading, so it drops.
    let deadline = Instant::now() + WAIT;
    while client.state() == marketdata_core::ConnectionState::Connected {
        assert!(Instant::now() < deadline, "first connection did not end");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    // Between connections: the first connection's count is still readable.
    assert_eq!(client.messages_dropped_total(), 93);
    let mut seen = Seen::new(receiver);
    // Reading makes room, so the second connection drops nothing.
    assert!(seen.until(|items| count(items, is_disconnected) == 1).await, "{:?}", seen.labels());
    assert_eq!(drop_reports(&seen.items).last().map(|r| r.1), Some(93), "{:?}", seen.labels());

    assert!(seen.until(|items| count(items, is_authenticated) == 2).await, "{:?}", seen.labels());
    assert_eq!(client.messages_dropped_total(), 0, "{:?}", seen.labels());
    client.disconnect().await.expect("disconnect");
}

/// Items of a sync client's stream until `done` holds, blocking up to [`WAIT`].
fn sync_until(rx: &StreamReceiver, seen: &mut Vec<StreamItem>, done: impl Fn(&[StreamItem]) -> bool) {
    let deadline = Instant::now() + WAIT;
    while !done(seen) {
        let left = deadline.saturating_duration_since(Instant::now());
        match rx.receive_timeout(left) {
            Ok(Some(item)) => seen.push(item),
            _ => panic!(
                "timed out; items so far: {:?}",
                seen.iter().map(common::label).collect::<Vec<_>>()
            ),
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_client_drop_count_survives_the_connection_and_restarts_on_reconnect() {
    let server = common::spawn_sequence(vec![
        common::AfterAuth::FloodDataThenDrop { count: 100 },
        common::AfterAuth::Idle,
    ])
    .await;
    tokio::task::spawn_blocking(move || {
        let config = ConnectionConfig::builder(&server.url, AuthRequest::with_api_key("test-key"))
            .message_buffer(8)
            .build();
        let client =
            marketdata_core::WebSocketClient::with_reconnection_config(config, slow_single_reconnect());
        let receiver = client.stream_receiver();
        client.connect().expect("connect");

        let deadline = Instant::now() + WAIT;
        while client.state() == marketdata_core::ConnectionState::Connected {
            assert!(Instant::now() < deadline, "first connection did not end");
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(client.messages_dropped_total(), 93);

        let mut seen = Vec::new();
        sync_until(&receiver, &mut seen, |items| count(items, is_disconnected) == 1);
        sync_until(&receiver, &mut seen, |items| count(items, is_authenticated) == 2);
        assert_eq!(client.messages_dropped_total(), 0);
        client.disconnect().expect("disconnect");
    })
    .await
    .expect("sync client");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_client_drops_newest_and_reports_before_disconnected() {
    let server = common::spawn(common::AfterAuth::FloodData { count: 100 }).await;
    let (dropped, items) = tokio::task::spawn_blocking(move || {
        let config = ConnectionConfig::builder(&server.url, AuthRequest::with_api_key("test-key"))
            .message_buffer(8)
            .build();
        let client = marketdata_core::WebSocketClient::with_reconnection_config(
            config,
            ReconnectionConfig::disabled(),
        );
        client.connect().expect("connect");
        let deadline = Instant::now() + WAIT;
        while client.messages_dropped_total() < 93 && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        client.disconnect().expect("disconnect");
        let receiver = client.stream_receiver();
        let items: Vec<_> = std::iter::from_fn(|| receiver.try_receive()).collect();
        (client.messages_dropped_total(), items)
    })
    .await
    .expect("sync client");

    let labels: Vec<_> = items.iter().map(common::label).collect();
    assert_eq!((messages(&items), dropped), (8, 93), "{labels:?}");
    let reports = drop_reports(&items);
    assert_eq!(reports.iter().map(|(dropped, _)| dropped).sum::<u64>(), 93, "{labels:?}");
    assert!(labels.last().expect("items").starts_with("Disconnected"), "{labels:?}");
}
