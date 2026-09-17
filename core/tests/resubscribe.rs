//! Subscription replay after a reconnect (#82, #111).
//!
//! A reconnect must re-send every stored subscription, one frame per channel
//! and modifier, on all four paths: auto-reconnect and manual `reconnect()`
//! of both clients. The acks for the batched frames must refresh the server
//! ids, so unsubscribing one symbol afterwards sends the new id. Failures to
//! re-send are covered by the unit tests next to each replay helper.
//!
//! Once reconnect attempts run out the client is closed: `reconnect()`
//! returns `ClientClosed` on both clients.

#![cfg(feature = "tokio-comp")]

#[path = "common/mod.rs"]
mod common;

use marketdata_core::aio::WebSocketClient as AsyncWebSocketClient;
use marketdata_core::websocket::{ConnectionEvent, StockSubscription};
use marketdata_core::{
    AuthRequest, Channel, ConnectionConfig, MarketDataError, ReconnectionConfig, StreamItem,
    StreamReceiver, WebSocketClient,
};
use serde_json::json;
use std::time::Duration;
use tokio::sync::mpsc;

/// Collect subscribe frames from `frames` until `count` arrived or `limit` elapsed.
async fn subscribe_frames(
    frames: &mut mpsc::UnboundedReceiver<String>,
    count: usize,
    limit: Duration,
) -> Vec<String> {
    let mut seen = Vec::new();
    let _ = tokio::time::timeout(limit, async {
        while seen.len() < count {
            match frames.recv().await {
                Some(frame) if frame.contains("\"subscribe\"") => seen.push(frame),
                Some(_) => continue,
                None => break,
            }
        }
    })
    .await;
    seen
}

/// Subscribe calls in this order: trades batch, books batch, odd-lot
/// trades, then another trades symbol that must join the first batch.
fn stock_subscriptions() -> Vec<StockSubscription> {
    vec![
        StockSubscription::new(Channel::Trades, vec!["2330", "2454"]),
        StockSubscription::new(Channel::Books, vec!["2317", "0050"]),
        StockSubscription::new(Channel::Trades, "2603").with_odd_lot(true),
        StockSubscription::new(Channel::Trades, "2881"),
    ]
}

/// The `data` of each resubscribe frame for [`stock_subscriptions`].
fn expected_resubscribe_data() -> Vec<serde_json::Value> {
    vec![
        json!({"channel": "trades", "symbols": ["2330", "2454", "2881"]}),
        json!({"channel": "books", "symbols": ["2317", "0050"]}),
        json!({"channel": "trades", "symbol": "2603", "intradayOddLot": true}),
    ]
}

/// Assert `frames` are exactly the resubscribe frames for [`stock_subscriptions`].
async fn assert_batched_resubscribe(frames: &mut mpsc::UnboundedReceiver<String>) {
    let expected = expected_resubscribe_data();
    let seen = subscribe_frames(frames, expected.len(), Duration::from_secs(5)).await;
    let data: Vec<serde_json::Value> = seen
        .iter()
        .map(|frame| serde_json::from_str::<serde_json::Value>(frame).unwrap()["data"].clone())
        .collect();
    assert_eq!(data, expected, "one frame per channel and modifier, in order");
    let extra = subscribe_frames(frames, 1, Duration::from_millis(300)).await;
    assert!(extra.is_empty(), "no frames beyond the batches, got {extra:?}");
}

/// Block until `count` `subscribed` acks have been read off `stream`.
fn wait_for_acks(stream: &StreamReceiver, count: usize) {
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    let mut acks = 0;
    while acks < count {
        let left = deadline.saturating_duration_since(std::time::Instant::now());
        match stream.receive_timeout(left) {
            Ok(Some(StreamItem::Message(m))) if m.event == "subscribed" => acks += 1,
            Ok(Some(_)) => continue,
            other => panic!("expected {count} subscribed acks, got {acks} then {other:?}"),
        }
    }
}

/// The unsubscribe frame's `data`, skipping other frames.
async fn unsubscribe_data(frames: &mut mpsc::UnboundedReceiver<String>) -> serde_json::Value {
    let frame = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let frame = frames.recv().await.expect("frame stream open");
            if frame.contains("\"unsubscribe\"") {
                return frame;
            }
        }
    })
    .await
    .expect("unsubscribe frame");
    serde_json::from_str::<serde_json::Value>(&frame).unwrap()["data"].clone()
}

/// Two acking connections: `old` for `connect()`, `new` for `reconnect()`.
async fn acking_servers() -> (
    common::MockServerHandle,
    mpsc::UnboundedReceiver<String>,
    mpsc::UnboundedReceiver<String>,
) {
    let (old_tx, old_rx) = mpsc::unbounded_channel();
    let (new_tx, new_rx) = mpsc::unbounded_channel();
    let server = common::spawn_sequence(vec![
        common::AfterAuth::AckSubscribes { frames: old_tx, id_prefix: "old".into() },
        common::AfterAuth::AckSubscribes { frames: new_tx, id_prefix: "new".into() },
    ])
    .await;
    (server, old_rx, new_rx)
}

fn remaining_keys() -> Vec<String> {
    ["trades:2330", "trades:2881", "books:2317", "books:0050", "trades:2603:oddlot"]
        .map(String::from)
        .to_vec()
}

fn sorted(mut keys: Vec<String>) -> Vec<String> {
    keys.sort();
    keys
}

#[tokio::test(flavor = "multi_thread")]
async fn async_manual_reconnect_batches_and_unsubscribe_uses_new_id() {
    let (server, _old_rx, mut new_rx) = acking_servers().await;
    let config = ConnectionConfig::new(server.url.clone(), AuthRequest::with_api_key("test-key"));
    let client = AsyncWebSocketClient::with_reconnection_config(config, ReconnectionConfig::disabled());
    let stream = client.stream_receiver();
    client.connect().await.expect("connect");
    for sub in stock_subscriptions() {
        client.subscribe(sub).await.expect("subscribe");
    }
    tokio::task::block_in_place(|| wait_for_acks(&stream, 4));

    client.reconnect().await.expect("reconnect");
    assert_batched_resubscribe(&mut new_rx).await;
    tokio::task::block_in_place(|| wait_for_acks(&stream, 3));

    client.unsubscribe(["trades:2454"]).await.expect("unsubscribe");
    assert_eq!(unsubscribe_data(&mut new_rx).await, json!({"id": "new:trades:2454"}));
    assert_eq!(sorted(client.subscription_keys()), sorted(remaining_keys()));
}

#[tokio::test]
async fn sync_manual_reconnect_batches_and_unsubscribe_uses_new_id() {
    let (server, _old_rx, mut new_rx) = acking_servers().await;
    let url = server.url.clone();
    let client = tokio::task::spawn_blocking(move || {
        let config = ConnectionConfig::new(url, AuthRequest::with_api_key("test-key"));
        let client =
            WebSocketClient::with_reconnection_config(config, ReconnectionConfig::disabled());
        let stream = client.stream_receiver();
        client.connect().expect("connect");
        for sub in stock_subscriptions() {
            client.subscribe(sub).expect("subscribe");
        }
        wait_for_acks(&stream, 4);
        client.reconnect().expect("reconnect");
        // Stopping the supervisor recorded `Closed { Client }`, which
        // `reconnect()` reopened (#82, #93).
        assert!(client.is_connected(), "{:?}", client.state());
        wait_for_acks(&stream, 3);
        client
    })
    .await
    .expect("sync client thread");
    assert_batched_resubscribe(&mut new_rx).await;

    let client = tokio::task::spawn_blocking(move || {
        client.unsubscribe(["trades:2454"]).expect("unsubscribe");
        client
    })
    .await
    .expect("sync client thread");
    assert_eq!(unsubscribe_data(&mut new_rx).await, json!({"id": "new:trades:2454"}));
    assert_eq!(sorted(client.subscription_keys()), sorted(remaining_keys()));

    tokio::task::spawn_blocking(move || drop(client)).await.expect("drop client");
}

/// One connection that drops shortly after auth, then one recording frames.
async fn server_that_drops_then_records() -> (common::MockServerHandle, mpsc::UnboundedReceiver<String>) {
    let (frames_tx, frames_rx) = mpsc::unbounded_channel();
    let server = common::spawn_sequence(vec![
        common::AfterAuth::ServerDropAfter { delay_ms: 200 },
        common::AfterAuth::RecordFrames { frames: frames_tx },
    ])
    .await;
    (server, frames_rx)
}

fn quick_reconnect() -> ReconnectionConfig {
    ReconnectionConfig::new(3, Duration::from_millis(100), Duration::from_millis(200))
        .expect("reconnection config")
}

#[tokio::test(flavor = "multi_thread")]
async fn async_auto_reconnect_resends_one_frame_per_channel_and_modifier() {
    let (server, mut frames_rx) = server_that_drops_then_records().await;
    let config = ConnectionConfig::new(server.url.clone(), AuthRequest::with_api_key("test-key"));
    let client = AsyncWebSocketClient::with_reconnection_config(config, quick_reconnect());
    // Stored while disconnected; only the reconnect replays them.
    for sub in stock_subscriptions() {
        client.subscribe(sub).await.expect("subscribe");
    }
    client.connect().await.expect("connect");

    assert_batched_resubscribe(&mut frames_rx).await;
}

#[tokio::test]
async fn sync_auto_reconnect_resends_one_frame_per_channel_and_modifier() {
    let (server, mut frames_rx) = server_that_drops_then_records().await;
    let url = server.url.clone();
    let client = tokio::task::spawn_blocking(move || {
        let config = ConnectionConfig::new(url, AuthRequest::with_api_key("test-key"));
        let client = WebSocketClient::with_reconnection_config(config, quick_reconnect());
        // Stored while disconnected; only the reconnect replays them.
        for sub in stock_subscriptions() {
            client.subscribe(sub).expect("subscribe");
        }
        client.connect().expect("connect");
        client
    })
    .await
    .expect("sync client thread");

    // A stuck owner thread would make dropping the client (which joins it)
    // hang the test instead of failing it.
    let expected = expected_resubscribe_data().len();
    let seen = subscribe_frames(&mut frames_rx, expected, Duration::from_secs(10)).await;
    if seen.len() != expected {
        std::mem::forget(client);
        panic!("expected {expected} resubscribe frames, got {seen:?}");
    }
    let data: Vec<serde_json::Value> = seen
        .iter()
        .map(|frame| serde_json::from_str::<serde_json::Value>(frame).unwrap()["data"].clone())
        .collect();
    assert_eq!(data, expected_resubscribe_data(), "one frame per channel and modifier, in order");

    tokio::task::spawn_blocking(move || drop(client)).await.expect("drop client");
}

#[tokio::test]
async fn sync_auto_reconnect_resends_a_large_batch_as_one_frame() {
    const SYMBOLS: usize = 1000;
    let (server, mut frames_rx) = server_that_drops_then_records().await;
    let url = server.url.clone();
    let client = tokio::task::spawn_blocking(move || {
        let config = ConnectionConfig::new(url, AuthRequest::with_api_key("test-key"));
        let client = WebSocketClient::with_reconnection_config(config, quick_reconnect());
        let symbols: Vec<String> = (0..SYMBOLS).map(|i| format!("{}", 10000 + i)).collect();
        client
            .subscribe(StockSubscription::new(Channel::Trades, symbols))
            .expect("subscribe");
        client.connect().expect("connect");
        client
    })
    .await
    .expect("sync client thread");

    let seen = subscribe_frames(&mut frames_rx, 1, Duration::from_secs(10)).await;
    if seen.len() != 1 {
        std::mem::forget(client);
        panic!("the batch must be re-sent without blocking the owner thread");
    }
    let data = serde_json::from_str::<serde_json::Value>(&seen[0]).unwrap()["data"].clone();
    assert_eq!(data["symbols"].as_array().map(Vec::len), Some(SYMBOLS));
    let extra = subscribe_frames(&mut frames_rx, 1, Duration::from_millis(300)).await;
    assert!(extra.is_empty(), "one frame for the whole batch, got {} more", extra.len());

    tokio::task::spawn_blocking(move || drop(client)).await.expect("drop client");
}

/// One connection that drops shortly after auth; later connects are refused.
async fn server_that_drops_once() -> common::MockServerHandle {
    common::spawn(common::AfterAuth::ServerDropAfter { delay_ms: 100 }).await
}

fn single_attempt() -> ReconnectionConfig {
    ReconnectionConfig::new(1, Duration::from_millis(100), Duration::from_millis(100))
        .expect("reconnection config")
}

fn saw_reconnect_failed(rx: &common::EventReceiver) -> bool {
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while std::time::Instant::now() < deadline {
        if let Ok(ConnectionEvent::ReconnectFailed { .. }) = rx.recv_timeout(Duration::from_millis(200)) {
            return true;
        }
    }
    false
}

#[tokio::test]
async fn sync_reconnect_after_attempts_exhausted_is_client_closed() {
    let server = server_that_drops_once().await;
    let url = server.url.clone();
    let result = tokio::task::spawn_blocking(move || {
        let config = ConnectionConfig::new(url, AuthRequest::with_api_key("test-key"));
        let client = WebSocketClient::with_reconnection_config(config, single_attempt());
        client.connect().expect("connect");
        let rx = common::EventReceiver::of_sync(&client);
        assert!(saw_reconnect_failed(&rx), "reconnect attempts should run out");
        assert!(client.is_closed());
        client.reconnect()
    })
    .await
    .expect("sync client thread");

    assert!(matches!(result, Err(MarketDataError::ClientClosed)), "got {result:?}");
}

#[tokio::test(flavor = "multi_thread")]
async fn async_reconnect_after_attempts_exhausted_is_client_closed() {
    let server = server_that_drops_once().await;
    let config = ConnectionConfig::new(server.url.clone(), AuthRequest::with_api_key("test-key"));
    let client = AsyncWebSocketClient::with_reconnection_config(config, single_attempt());
    client.connect().await.expect("connect");
    let rx = common::EventReceiver::of_async(&client);
    let exhausted = tokio::task::block_in_place(|| saw_reconnect_failed(&rx));
    assert!(exhausted, "reconnect attempts should run out");
    assert!(client.is_closed().await);

    let result = client.reconnect().await;
    assert!(matches!(result, Err(MarketDataError::ClientClosed)), "got {result:?}");
}
