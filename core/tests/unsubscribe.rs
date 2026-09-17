//! Unsubscribe sends the id the server issued and keeps local state in step
//! with it (#136), on both clients.
//!
//! - Unsubscribing by the server id removes the local subscription, so a
//!   reconnect does not restore it.
//! - Unsubscribing by local key before the `subscribed` ack arrives sends
//!   nothing until the ack brings the server id, then sends that id.

#![cfg(feature = "tokio-comp")]

#[path = "common/mod.rs"]
mod common;

use marketdata_core::aio::WebSocketClient as AsyncWebSocketClient;
use marketdata_core::websocket::StockSubscription;
use marketdata_core::{
    AuthRequest, Channel, ConnectionConfig, ReconnectionConfig, StreamItem, StreamReceiver,
    WebSocketClient,
};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::sync::Notify;

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

/// The `data` of every frame with `event` received within `limit`.
async fn frames_data(
    frames: &mut mpsc::UnboundedReceiver<String>,
    event: &str,
    limit: Duration,
) -> Vec<serde_json::Value> {
    let mut seen = Vec::new();
    let _ = tokio::time::timeout(limit, async {
        while let Some(frame) = frames.recv().await {
            let frame: serde_json::Value = serde_json::from_str(&frame).unwrap();
            if frame["event"] == event {
                seen.push(frame["data"].clone());
            }
        }
    })
    .await;
    seen
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

/// One connection that holds each ack until the returned `Notify` is
/// signalled, so a test can unsubscribe before the ack arrives.
async fn held_ack_server() -> (
    common::MockServerHandle,
    mpsc::UnboundedReceiver<String>,
    Arc<Notify>,
) {
    let (frames_tx, frames_rx) = mpsc::unbounded_channel();
    let notify = Arc::new(Notify::new());
    let server = common::spawn_sequence(vec![common::AfterAuth::AckSubscribesOnNotify {
        frames: frames_tx,
        id_prefix: "srv".into(),
        notify: Arc::clone(&notify),
    }])
    .await;
    (server, frames_rx, notify)
}

fn subscriptions() -> [StockSubscription; 2] {
    [
        StockSubscription::new(Channel::Trades, "2330"),
        StockSubscription::new(Channel::Trades, "2317"),
    ]
}

const LIMIT: Duration = Duration::from_secs(2);

#[tokio::test(flavor = "multi_thread")]
async fn async_unsubscribe_by_server_id_is_not_restored_on_reconnect() {
    let (server, mut old_rx, mut new_rx) = acking_servers().await;
    let config = ConnectionConfig::new(server.url.clone(), AuthRequest::with_api_key("test-key"));
    let client = AsyncWebSocketClient::with_reconnection_config(config, ReconnectionConfig::disabled());
    let stream = client.stream_receiver();
    client.connect().await.expect("connect");
    for sub in subscriptions() {
        client.subscribe(sub).await.expect("subscribe");
    }
    tokio::task::block_in_place(|| wait_for_acks(&stream, 2));

    client.unsubscribe(["old:trades:2330"]).await.expect("unsubscribe");
    assert_eq!(client.subscription_keys(), ["trades:2317"]);
    assert_eq!(
        frames_data(&mut old_rx, "unsubscribe", LIMIT).await,
        [json!({"id": "old:trades:2330"})]
    );

    client.reconnect().await.expect("reconnect");
    assert_eq!(
        frames_data(&mut new_rx, "subscribe", LIMIT).await,
        [json!({"channel": "trades", "symbol": "2317"})]
    );
}

#[tokio::test]
async fn sync_unsubscribe_by_server_id_is_not_restored_on_reconnect() {
    let (server, mut old_rx, mut new_rx) = acking_servers().await;
    let url = server.url.clone();
    let client = tokio::task::spawn_blocking(move || {
        let config = ConnectionConfig::new(url, AuthRequest::with_api_key("test-key"));
        let client =
            WebSocketClient::with_reconnection_config(config, ReconnectionConfig::disabled());
        let stream = client.stream_receiver();
        client.connect().expect("connect");
        for sub in subscriptions() {
            client.subscribe(sub).expect("subscribe");
        }
        wait_for_acks(&stream, 2);
        client.unsubscribe(["old:trades:2330"]).expect("unsubscribe");
        assert_eq!(client.subscription_keys(), ["trades:2317"]);
        client
    })
    .await
    .expect("sync client thread");
    assert_eq!(
        frames_data(&mut old_rx, "unsubscribe", LIMIT).await,
        [json!({"id": "old:trades:2330"})]
    );

    let client = tokio::task::spawn_blocking(move || {
        client.reconnect().expect("reconnect");
        client
    })
    .await
    .expect("sync client thread");
    assert_eq!(
        frames_data(&mut new_rx, "subscribe", LIMIT).await,
        [json!({"channel": "trades", "symbol": "2317"})]
    );

    tokio::task::spawn_blocking(move || drop(client)).await.expect("drop client");
}

#[tokio::test(flavor = "multi_thread")]
async fn async_unsubscribe_before_ack_sends_id_from_ack() {
    let (server, mut frames, notify) = held_ack_server().await;
    let config = ConnectionConfig::new(server.url.clone(), AuthRequest::with_api_key("test-key"));
    let client = AsyncWebSocketClient::with_reconnection_config(config, ReconnectionConfig::disabled());
    client.connect().await.expect("connect");

    let [sub, _] = subscriptions();
    client.subscribe(sub).await.expect("subscribe");
    client.unsubscribe(["trades:2330"]).await.expect("unsubscribe");
    assert_eq!(client.subscription_count(), 0);
    // Only now does the server ack the subscribe. `notify_one` stores a
    // permit, so the signal stands even if the server is not yet waiting.
    notify.notify_one();

    assert_eq!(
        frames_data(&mut frames, "unsubscribe", LIMIT).await,
        [json!({"id": "srv:trades:2330"})]
    );
}

#[tokio::test]
async fn sync_unsubscribe_before_ack_sends_id_from_ack() {
    let (server, mut frames, notify) = held_ack_server().await;
    let url = server.url.clone();
    let client = tokio::task::spawn_blocking(move || {
        let config = ConnectionConfig::new(url, AuthRequest::with_api_key("test-key"));
        let client =
            WebSocketClient::with_reconnection_config(config, ReconnectionConfig::disabled());
        client.connect().expect("connect");
        let [sub, _] = subscriptions();
        client.subscribe(sub).expect("subscribe");
        client.unsubscribe(["trades:2330"]).expect("unsubscribe");
        assert_eq!(client.subscription_count(), 0);
        client
    })
    .await
    .expect("sync client thread");
    // Only now does the server ack the subscribe. `notify_one` stores a
    // permit, so the signal stands even if the server is not yet waiting.
    notify.notify_one();

    assert_eq!(
        frames_data(&mut frames, "unsubscribe", LIMIT).await,
        [json!({"id": "srv:trades:2330"})]
    );

    tokio::task::spawn_blocking(move || drop(client)).await.expect("drop client");
}
