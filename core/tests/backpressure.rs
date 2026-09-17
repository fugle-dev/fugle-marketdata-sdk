//! Inbound message queue under load (#46).
//!
//! - A consumer on the client's own runtime keeps up with an unpaced flood:
//!   the dispatch loop must yield instead of starving it.
//! - `MessageOverflow::DropNewest` holds exactly `message_buffer` messages,
//!   counts every drop and reports them with `MessagesDropped`.
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
    AuthRequest, ConnectionConfig, MessageOverflow, ReconnectionConfig,
};
use std::sync::mpsc;
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

/// Wait up to [`WAIT`] for `done`, polling.
async fn wait_for(mut done: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + WAIT;
    while Instant::now() < deadline {
        if done() {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    done()
}

fn queued_events(client: &WebSocketClient) -> Vec<ConnectionEvent> {
    client.events().try_lock().expect("events lock").try_iter().collect()
}

fn drop_reports(events: &[ConnectionEvent]) -> Vec<(u64, u64)> {
    events
        .iter()
        .filter_map(|event| match event {
            ConnectionEvent::MessagesDropped { dropped, total } => Some((*dropped, *total)),
            _ => None,
        })
        .collect()
}

async fn message_stream_keeps_up_with_flood() {
    let url = flood_server_on_own_runtime(FLOOD);
    let client = client_for(&url, |b| b);
    let mut stream = client.message_stream();
    client.connect().await.expect("connect");

    // Consumed as a `futures::Stream`, on the client's runtime.
    let consumer = tokio::spawn(async move {
        let mut data = 0;
        while data < FLOOD {
            match tokio::time::timeout(WAIT, stream.next()).await {
                Ok(Some(msg)) if msg.event == "data" => data += 1,
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
async fn message_stream_keeps_up_with_flood_on_current_thread_runtime() {
    message_stream_keeps_up_with_flood().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn message_stream_keeps_up_with_flood_on_multi_thread_runtime() {
    message_stream_keeps_up_with_flood().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn messages_keeps_up_with_flood() {
    let url = flood_server_on_own_runtime(FLOOD);
    let client = client_for(&url, |b| b);
    let receiver = client.messages();
    client.connect().await.expect("connect");

    let data = tokio::task::spawn_blocking(move || {
        let mut data = 0;
        while data < FLOOD {
            match receiver.receive_timeout(WAIT) {
                Ok(Some(msg)) if msg.event == "data" => data += 1,
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
async fn full_queue_drops_newest_and_reports_before_disconnected() {
    let server = common::spawn(common::AfterAuth::FloodData { count: 100 }).await;
    let client = client_for(&server.url, |b| b.message_buffer(8));
    client.connect().await.expect("connect");

    // Nothing reads the queue: `authenticated` and 7 data frames fit, the
    // other 93 data frames are dropped.
    assert!(
        wait_for(|| client.messages_dropped_total() == 93).await,
        "dropped {}",
        client.messages_dropped_total()
    );
    client.disconnect().await.expect("disconnect");
    assert_eq!(client.messages_dropped_total(), 93, "count survives disconnect()");

    let receiver = client.messages();
    let kept: Vec<_> = std::iter::from_fn(|| receiver.try_receive()).collect();
    assert_eq!(kept.len(), 8);
    assert_eq!(kept[0].event, "authenticated");
    assert_eq!(kept.last().expect("kept").id.as_deref(), Some("6"));

    let events = queued_events(&client);
    let reports = drop_reports(&events);
    assert!(!reports.is_empty(), "no MessagesDropped in {events:?}");
    assert_eq!(reports.iter().map(|(dropped, _)| dropped).sum::<u64>(), 93);
    assert_eq!(reports.last().expect("report").1, 93);
    // Reported inside the connection: after Authenticated, before Disconnected.
    let position = |matches: fn(&ConnectionEvent) -> bool| {
        events.iter().position(matches).unwrap_or_else(|| panic!("missing event in {events:?}"))
    };
    let authenticated = position(|e| matches!(e, ConnectionEvent::Authenticated { .. }));
    let first_report = position(|e| matches!(e, ConnectionEvent::MessagesDropped { .. }));
    let disconnected = position(|e| matches!(e, ConnectionEvent::Disconnected { .. }));
    let last_report = events
        .iter()
        .rposition(|e| matches!(e, ConnectionEvent::MessagesDropped { .. }))
        .expect("report");
    assert!(authenticated < first_report, "{events:?}");
    assert!(last_report < disconnected, "{events:?}");
}

#[tokio::test]
async fn unbounded_queue_never_drops() {
    let server = common::spawn(common::AfterAuth::FloodData { count: 100 }).await;
    let client = client_for(&server.url, |b| {
        b.message_buffer(8).message_overflow(MessageOverflow::Unbounded)
    });
    client.connect().await.expect("connect");
    let receiver = client.messages();

    // Let the flood queue up before reading any of it; a bounded queue of
    // 8 would have dropped most of it by then.
    tokio::time::sleep(Duration::from_millis(300)).await;
    let mut kept = Vec::new();
    assert!(
        wait_for(|| {
            kept.extend(std::iter::from_fn(|| receiver.try_receive()));
            kept.len() == 101
        })
        .await,
        "kept {}",
        kept.len()
    );
    client.disconnect().await.expect("disconnect");

    assert_eq!(client.messages_dropped_total(), 0);
    assert_eq!(drop_reports(&queued_events(&client)), vec![]);
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
    client.connect().await.expect("connect");

    // The full queue drops the second `authenticated` frame instead of
    // blocking the handshake until its 10 s timeout.
    let mut events = Vec::new();
    let reauthenticated = wait_for(|| {
        events.extend(queued_events(&client));
        events
            .iter()
            .filter(|e| matches!(e, ConnectionEvent::Authenticated { .. }))
            .count()
            == 2
    })
    .await;
    assert!(reauthenticated, "{events:?}");

    client.disconnect().await.expect("disconnect");
    events.extend(queued_events(&client));
    // The first connection dropped 17 data frames; the second only its
    // `authenticated` frame, counted from zero.
    assert_eq!(client.messages_dropped_total(), 1, "{events:?}");
    let reports = drop_reports(&events);
    assert_eq!(reports.iter().map(|(dropped, _)| dropped).sum::<u64>(), 18, "{events:?}");
    assert_eq!(reports.last(), Some(&(1, 1)), "{events:?}");
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
    let receiver = client.messages();
    client.connect().await.expect("connect");

    let mut events = Vec::new();
    let is_disconnected = |e: &ConnectionEvent| matches!(e, ConnectionEvent::Disconnected { .. });
    assert!(
        wait_for(|| {
            events.extend(queued_events(&client));
            events.iter().any(is_disconnected)
        })
        .await,
        "{events:?}"
    );
    // Between connections: the first connection's count is still readable.
    assert_eq!(client.messages_dropped_total(), 93, "{events:?}");
    assert_eq!(drop_reports(&events).last().map(|r| r.1), Some(93), "{events:?}");
    // Make room so the second connection drops nothing.
    assert_eq!(std::iter::from_fn(|| receiver.try_receive()).count(), 8);

    let authenticated = |e: &ConnectionEvent| matches!(e, ConnectionEvent::Authenticated { .. });
    assert!(
        wait_for(|| {
            events.extend(queued_events(&client));
            events.iter().filter(|e| authenticated(e)).count() == 2
        })
        .await,
        "{events:?}"
    );
    assert_eq!(client.messages_dropped_total(), 0, "{events:?}");
    client.disconnect().await.expect("disconnect");
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
        let receiver = client.messages();
        client.connect().expect("connect");
        let events = client.events().lock().expect("events lock");
        let mut seen = Vec::new();
        let mut wait_until = |done: &dyn Fn(&[ConnectionEvent]) -> bool| {
            let deadline = Instant::now() + WAIT;
            while !done(&seen) {
                let left = deadline.saturating_duration_since(Instant::now());
                match events.recv_timeout(left) {
                    Ok(event) => seen.push(event),
                    Err(_) => panic!("timed out; events so far: {seen:?}"),
                }
            }
        };

        wait_until(&|seen| seen.iter().any(|e| matches!(e, ConnectionEvent::Disconnected { .. })));
        assert_eq!(client.messages_dropped_total(), 93);
        assert_eq!(std::iter::from_fn(|| receiver.try_receive()).count(), 8);

        wait_until(&|seen| {
            seen.iter()
                .filter(|e| matches!(e, ConnectionEvent::Authenticated { .. }))
                .count()
                == 2
        });
        assert_eq!(client.messages_dropped_total(), 0);
        drop(events);
        client.disconnect().expect("disconnect");
    })
    .await
    .expect("sync client");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_client_drops_newest_and_reports_before_disconnected() {
    let server = common::spawn(common::AfterAuth::FloodData { count: 100 }).await;
    let (kept, dropped, events) = tokio::task::spawn_blocking(move || {
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
        let receiver = client.messages();
        let kept = std::iter::from_fn(|| receiver.try_receive()).count();
        let events: Vec<_> = client.events().lock().expect("events lock").try_iter().collect();
        (kept, client.messages_dropped_total(), events)
    })
    .await
    .expect("sync client");

    assert_eq!((kept, dropped), (8, 93));
    let reports = drop_reports(&events);
    assert_eq!(reports.iter().map(|(dropped, _)| dropped).sum::<u64>(), 93, "{events:?}");
    let last_report = events
        .iter()
        .rposition(|e| matches!(e, ConnectionEvent::MessagesDropped { .. }))
        .expect("report");
    let disconnected = events
        .iter()
        .position(|e| matches!(e, ConnectionEvent::Disconnected { .. }))
        .expect("disconnected");
    assert!(last_report < disconnected, "{events:?}");
}
