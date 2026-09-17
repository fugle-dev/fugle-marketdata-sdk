//! Order across messages and events on a client's stream (#68).
//!
//! Every message of a connection, including the server's `authenticated`
//! frame, comes after that connection's `Authenticated` and before its
//! `Disconnected`; frames a connection receives after it was reported closed
//! are discarded.

#![cfg(feature = "tokio-comp")]

#[path = "common/mod.rs"]
mod common;

use marketdata_core::aio::WebSocketClient;
use marketdata_core::{
    AuthRequest, ConnectionConfig, ConnectionEvent, ReconnectionConfig, StreamItem, StreamReceiver,
};
use std::time::{Duration, Instant};

const WAIT: Duration = Duration::from_secs(5);

/// Read `rx` until `done` holds for the items so far, up to [`WAIT`].
fn read_until(rx: &StreamReceiver, done: impl Fn(&[StreamItem]) -> bool) -> Vec<StreamItem> {
    let deadline = Instant::now() + WAIT;
    let mut items = Vec::new();
    while !done(&items) {
        let left = deadline.saturating_duration_since(Instant::now());
        match rx.receive_timeout(left) {
            Ok(Some(item)) => items.push(item),
            _ => panic!("timed out; items so far: {:?}", labels(&items)),
        }
    }
    items
}

fn labels(items: &[StreamItem]) -> Vec<String> {
    items.iter().map(common::label).collect()
}

fn is_event(item: &StreamItem, matches: fn(&ConnectionEvent) -> bool) -> bool {
    matches!(item, StreamItem::Event(event) if matches(event))
}

fn authenticated(event: &ConnectionEvent) -> bool {
    matches!(event, ConnectionEvent::Authenticated { .. })
}

fn disconnected(event: &ConnectionEvent) -> bool {
    matches!(event, ConnectionEvent::Disconnected { .. })
}

fn is_message(item: &StreamItem) -> bool {
    matches!(item, StreamItem::Message(_))
}

/// The messages between each `Authenticated` and the next `Disconnected`,
/// and the items outside any such span.
fn connections(items: &[StreamItem]) -> (Vec<Vec<String>>, Vec<String>) {
    let mut spans = Vec::new();
    let mut outside = Vec::new();
    let mut current: Option<Vec<String>> = None;
    for item in items {
        if is_event(item, authenticated) {
            current = Some(Vec::new());
        } else if is_event(item, disconnected) {
            spans.extend(current.take());
        } else if let Some(span) = current.as_mut() {
            if is_message(item) {
                span.push(common::label(item));
            }
        } else {
            outside.push(common::label(item));
        }
    }
    spans.extend(current);
    (spans, outside)
}

fn data_labels(n: usize) -> Vec<String> {
    std::iter::once("m:authenticated".to_string())
        .chain((0..n).map(|i| format!("m{i}")))
        .collect()
}

#[tokio::test]
async fn server_close_delivers_every_message_before_disconnected() {
    let server = common::spawn(common::AfterAuth::FloodDataThenClose { count: 500, code: 1001 }).await;
    let config = ConnectionConfig::new(server.url.clone(), AuthRequest::with_api_key("k"));
    let client = WebSocketClient::with_reconnection_config(config, ReconnectionConfig::disabled());
    let rx = client.stream_receiver();
    client.connect().await.expect("connect");

    let items = tokio::task::spawn_blocking(move || {
        read_until(&rx, |items| items.last().is_some_and(|i| is_event(i, disconnected)))
    })
    .await
    .expect("reader");

    let labels = labels(&items);
    assert_eq!(&labels[..2], ["Connecting", "Connected"], "{labels:?}");
    assert!(is_event(&items[2], authenticated), "{labels:?}");
    // Exactly the frames of the connection, in order, then its close.
    let (spans, outside) = connections(&items);
    assert_eq!(spans, vec![data_labels(500)], "{labels:?}");
    assert_eq!(outside, ["Connecting", "Connected"], "{labels:?}");
}

#[tokio::test]
async fn reconnect_keeps_each_connections_messages_inside_it() {
    let server = common::spawn_sequence(vec![
        common::AfterAuth::FloodDataThenDrop { count: 200 },
        common::AfterAuth::FloodDataThenClose { count: 200, code: 1000 },
    ])
    .await;
    let config = ConnectionConfig::new(server.url.clone(), AuthRequest::with_api_key("k"));
    let reconnect = ReconnectionConfig::new(1, Duration::from_millis(100), Duration::from_millis(100))
        .expect("reconnection config");
    let client = WebSocketClient::with_reconnection_config(config, reconnect);
    let rx = client.stream_receiver();
    client.connect().await.expect("connect");

    let items = tokio::task::spawn_blocking(move || {
        read_until(&rx, |items| items.iter().filter(|i| is_event(i, disconnected)).count() == 2)
    })
    .await
    .expect("reader");

    let labels = labels(&items);
    let (spans, outside) = connections(&items);
    assert_eq!(spans, vec![data_labels(200), data_labels(200)], "{labels:?}");
    // Between the connections only lifecycle events.
    assert!(outside.iter().all(|l| !l.starts_with('m')), "{labels:?}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn force_close_mid_flood_delivers_nothing_after_disconnected() {
    // Far more frames than can be read before `force_close()` lands.
    let server = common::spawn(common::AfterAuth::FloodData { count: 200_000 }).await;
    let config = ConnectionConfig::builder(server.url.clone(), AuthRequest::with_api_key("k"))
        .message_overflow(marketdata_core::MessageOverflow::Unbounded)
        .build();
    let client = WebSocketClient::with_reconnection_config(config, ReconnectionConfig::disabled());
    let rx = client.stream_receiver();
    client.connect().await.expect("connect");

    let deadline = Instant::now() + WAIT;
    while !rx.try_receive().is_some_and(|i| matches!(&i, StreamItem::Message(m) if m.id.is_some())) {
        assert!(Instant::now() < deadline, "no data frame arrived");
    }
    // `force_close()` aborts the dispatch task without waiting for it.
    client.force_close().await.expect("force close");
    tokio::time::sleep(Duration::from_millis(200)).await;

    let items: Vec<_> = std::iter::from_fn(|| rx.try_receive()).collect();
    let closed = items
        .iter()
        .position(|i| is_event(i, disconnected))
        .unwrap_or_else(|| panic!("no Disconnected in {} items", items.len()));
    assert!(
        !items[closed..].iter().any(is_message),
        "messages after Disconnected: {:?}",
        labels(&items[closed..])
    );
}

#[cfg(feature = "test-utils")]
#[tokio::test]
async fn rejected_credentials_queue_the_rejection_frame_after_unauthenticated() {
    let server = marketdata_core::testing::MockWsServer::start().await;
    server.set_auth_response(serde_json::json!({
        "event": "error",
        "data": { "message": "Invalid authentication credentials" }
    }));
    let config = ConnectionConfig::new(server.url(), AuthRequest::with_api_key("k"));
    let client = WebSocketClient::with_reconnection_config(config, ReconnectionConfig::disabled());
    let rx = client.stream_receiver();
    assert!(client.connect().await.is_err());

    let items: Vec<_> = std::iter::from_fn(|| rx.try_receive()).collect();
    let labels = labels(&items);
    assert_eq!(labels.len(), 4, "{labels:?}");
    assert_eq!(&labels[..2], ["Connecting", "Connected"], "{labels:?}");
    assert!(labels[2].starts_with("Unauthenticated"), "{labels:?}");
    assert_eq!(labels[3], "m:error", "{labels:?}");
    // Never inside an authenticated connection.
    let (spans, _) = connections(&items);
    assert!(spans.is_empty(), "{labels:?}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_client_server_close_delivers_every_message_before_disconnected() {
    let server = common::spawn(common::AfterAuth::FloodDataThenClose { count: 500, code: 1001 }).await;
    let items = tokio::task::spawn_blocking(move || {
        let config = ConnectionConfig::new(server.url.clone(), AuthRequest::with_api_key("k"));
        let client =
            marketdata_core::WebSocketClient::with_reconnection_config(config, ReconnectionConfig::disabled());
        let rx = client.stream_receiver();
        client.connect().expect("connect");
        read_until(&rx, |items| items.last().is_some_and(|i| is_event(i, disconnected)))
    })
    .await
    .expect("sync client");

    let labels = labels(&items);
    let (spans, outside) = connections(&items);
    assert_eq!(spans, vec![data_labels(500)], "{labels:?}");
    assert_eq!(outside, ["Connecting", "Connected"], "{labels:?}");
}

// The sync client authenticates, reconnects and force-closes through code of
// its own (`sync::client::connect`, `owner_thread::reconnect_and_authenticate`,
// `sync::client::force_close`), so each async case above has a sync twin.

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_client_reconnect_keeps_each_connections_messages_inside_it() {
    let server = common::spawn_sequence(vec![
        common::AfterAuth::FloodDataThenDrop { count: 200 },
        common::AfterAuth::FloodDataThenClose { count: 200, code: 1000 },
    ])
    .await;
    let items = tokio::task::spawn_blocking(move || {
        let config = ConnectionConfig::new(server.url.clone(), AuthRequest::with_api_key("k"));
        let reconnect =
            ReconnectionConfig::new(1, Duration::from_millis(100), Duration::from_millis(100))
                .expect("reconnection config");
        let client = marketdata_core::WebSocketClient::with_reconnection_config(config, reconnect);
        let rx = client.stream_receiver();
        client.connect().expect("connect");
        let items = read_until(&rx, |items| {
            items.iter().filter(|i| is_event(i, disconnected)).count() == 2
        });
        let _ = client.disconnect();
        items
    })
    .await
    .expect("sync client");

    let labels = labels(&items);
    let (spans, outside) = connections(&items);
    assert_eq!(spans, vec![data_labels(200), data_labels(200)], "{labels:?}");
    // Between the connections only lifecycle events.
    assert!(outside.iter().all(|l| !l.starts_with('m')), "{labels:?}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_client_force_close_mid_flood_delivers_nothing_after_disconnected() {
    // The owner thread checks its stop flag before every read, so only a
    // frame whose read was already under way when `force_close()` landed can
    // follow `Disconnected` — about one run in three. Repeat to catch it.
    const RUNS: usize = 20;
    let mut urls = Vec::new();
    let mut servers = Vec::new();
    for _ in 0..RUNS {
        let server = common::spawn(common::AfterAuth::FloodData { count: 200_000 }).await;
        urls.push(server.url.clone());
        servers.push(server);
    }
    let runs = tokio::task::spawn_blocking(move || {
        urls.into_iter()
            .map(|url| {
                let config = ConnectionConfig::builder(url, AuthRequest::with_api_key("k"))
                    .message_overflow(marketdata_core::MessageOverflow::Unbounded)
                    .build();
                let client = marketdata_core::WebSocketClient::with_reconnection_config(
                    config,
                    ReconnectionConfig::disabled(),
                );
                let rx = client.stream_receiver();
                client.connect().expect("connect");

                let deadline = Instant::now() + WAIT;
                while !rx
                    .try_receive()
                    .is_some_and(|i| matches!(&i, StreamItem::Message(m) if m.id.is_some()))
                {
                    assert!(Instant::now() < deadline, "no data frame arrived");
                }
                // Detaches the owner thread without waiting for it.
                client.force_close().expect("force close");
                std::thread::sleep(Duration::from_millis(100));
                std::iter::from_fn(|| rx.try_receive()).collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    })
    .await
    .expect("sync client");
    drop(servers);

    for (run, items) in runs.iter().enumerate() {
        let closed = items
            .iter()
            .position(|i| is_event(i, disconnected))
            .unwrap_or_else(|| panic!("run {run}: no Disconnected in {} items", items.len()));
        assert!(
            !items[closed..].iter().any(is_message),
            "run {run}: messages after Disconnected: {:?}",
            labels(&items[closed..])
        );
    }
}

#[cfg(feature = "test-utils")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_client_rejected_credentials_queue_the_rejection_frame_after_unauthenticated() {
    let server = marketdata_core::testing::MockWsServer::start().await;
    server.set_auth_response(serde_json::json!({
        "event": "error",
        "data": { "message": "Invalid authentication credentials" }
    }));
    let url = server.url();
    let items = tokio::task::spawn_blocking(move || {
        let config = ConnectionConfig::new(url, AuthRequest::with_api_key("k"));
        let client =
            marketdata_core::WebSocketClient::with_reconnection_config(config, ReconnectionConfig::disabled());
        let rx = client.stream_receiver();
        assert!(client.connect().is_err());
        std::iter::from_fn(|| rx.try_receive()).collect::<Vec<_>>()
    })
    .await
    .expect("sync client");

    let labels = labels(&items);
    assert_eq!(labels.len(), 4, "{labels:?}");
    assert_eq!(&labels[..2], ["Connecting", "Connected"], "{labels:?}");
    assert!(labels[2].starts_with("Unauthenticated"), "{labels:?}");
    assert_eq!(labels[3], "m:error", "{labels:?}");
    // Never inside an authenticated connection.
    let (spans, _) = connections(&items);
    assert!(spans.is_empty(), "{labels:?}");
}
