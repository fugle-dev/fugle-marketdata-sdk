//! Subscription replay after a reconnect (#82).
//!
//! A reconnect must re-send every stored subscription: the sync client's
//! manual `reconnect()`, and its auto-reconnect even when there are more
//! subscriptions than the write queue's base capacity. Failures to re-send
//! are covered by the unit tests next to each replay helper.
//!
//! Once reconnect attempts run out the client is closed: `reconnect()`
//! returns `ClientClosed` on both clients.

#![cfg(feature = "tokio-comp")]

#[path = "common/mod.rs"]
mod common;

use marketdata_core::aio::WebSocketClient as AsyncWebSocketClient;
use marketdata_core::websocket::{ConnectionEvent, StockSubscription};
use marketdata_core::{
    AuthRequest, Channel, ConnectionConfig, MarketDataError, ReconnectionConfig, WebSocketClient,
};
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

#[tokio::test]
async fn sync_manual_reconnect_resends_subscriptions() {
    let (frames_tx, mut frames_rx) = mpsc::unbounded_channel();
    let server = common::spawn_sequence(vec![
        common::AfterAuth::Idle,
        common::AfterAuth::RecordFrames { frames: frames_tx },
    ])
    .await;

    let url = server.url.clone();
    let client = tokio::task::spawn_blocking(move || {
        let config = ConnectionConfig::new(url, AuthRequest::with_api_key("test-key"));
        let client =
            WebSocketClient::with_reconnection_config(config, ReconnectionConfig::disabled());
        client.connect().expect("connect");
        client
            .subscribe(StockSubscription::new(Channel::Trades, "2330"))
            .expect("subscribe 2330");
        client
            .subscribe(StockSubscription::new(Channel::Books, "2317"))
            .expect("subscribe 2317");
        client.reconnect().expect("reconnect");
        // Stopping the supervisor recorded `Closed { Client }`, which
        // `reconnect()` reopened (#82, #93).
        assert!(client.is_connected(), "{:?}", client.state());
        client
    })
    .await
    .expect("sync client thread");

    let frames = subscribe_frames(&mut frames_rx, 2, Duration::from_secs(5)).await;
    assert_eq!(frames.len(), 2, "expected both subscriptions re-sent, got {frames:?}");
    assert!(frames[0].contains("2330") && frames[1].contains("2317"));

    tokio::task::spawn_blocking(move || drop(client)).await.expect("drop client");
}

#[tokio::test]
async fn sync_auto_reconnect_resends_more_subscriptions_than_queue_capacity() {
    const SUBSCRIPTIONS: usize = 100;
    let (frames_tx, mut frames_rx) = mpsc::unbounded_channel();
    let server = common::spawn_sequence(vec![
        common::AfterAuth::ServerDropAfter { delay_ms: 200 },
        common::AfterAuth::RecordFrames { frames: frames_tx },
    ])
    .await;

    let url = server.url.clone();
    let client = tokio::task::spawn_blocking(move || {
        let config = ConnectionConfig::new(url, AuthRequest::with_api_key("test-key"));
        let reconnection =
            ReconnectionConfig::new(3, Duration::from_millis(100), Duration::from_millis(200))
                .expect("reconnection config");
        let client = WebSocketClient::with_reconnection_config(config, reconnection);
        // Stored while disconnected; only the reconnect replays them.
        for i in 0..SUBSCRIPTIONS {
            client
                .subscribe(StockSubscription::new(Channel::Trades, format!("{}", 1000 + i)))
                .expect("subscribe");
        }
        client.connect().expect("connect");
        client
    })
    .await
    .expect("sync client thread");

    let frames = subscribe_frames(&mut frames_rx, SUBSCRIPTIONS, Duration::from_secs(10)).await;
    if frames.len() != SUBSCRIPTIONS {
        // A stuck owner thread would make dropping the client (which joins
        // it) hang the test instead of failing it.
        std::mem::forget(client);
        panic!(
            "every subscription must be re-sent without blocking the owner thread, got {}",
            frames.len()
        );
    }

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
