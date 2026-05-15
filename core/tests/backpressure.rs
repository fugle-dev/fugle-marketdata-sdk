//! Backpressure / drop-newest test.
//!
//! Configures `ConnectionConfig::builder().message_buffer(N)` to a
//! tiny capacity, lets the mock server flood data faster than the
//! test consumes it, and asserts `messages_dropped_total > 0`.

#![cfg(feature = "tokio-comp")]

#[path = "common/mod.rs"]
mod common;

use marketdata_core::aio::WebSocketClient;
use marketdata_core::{AuthRequest, ConnectionConfig, ReconnectionConfig};
use std::time::Duration;

#[tokio::test]
async fn drops_increment_when_consumer_starves_message_channel() {
    let server = common::spawn(common::AfterAuth::FloodData { count: 200 }).await;

    let auth = AuthRequest::with_api_key("test-key");
    let config = ConnectionConfig::builder(server.url.clone(), auth)
        .message_buffer(2) // tiny — anything beyond 2 unread frames drops
        .build();
    let client = WebSocketClient::with_reconnection_config(
        config,
        ReconnectionConfig::disabled(),
    );

    client.connect().await.expect("connect");

    // Deliberately do NOT consume `client.messages()`. Give the
    // dispatch task time to forward all 200 frames against the
    // 2-slot channel — most will fail try_send and bump the counter.
    tokio::time::sleep(Duration::from_millis(500)).await;

    let dropped = client.messages_dropped_total();
    assert!(
        dropped > 0,
        "expected drops > 0, got {}; channel may have been drained externally",
        dropped
    );

    let _ = client
        .shutdown_with_timeout(Duration::from_millis(200))
        .await;
}

#[tokio::test]
async fn no_drops_when_consumer_keeps_up() {
    let server = common::spawn(common::AfterAuth::FloodData { count: 50 }).await;

    let auth = AuthRequest::with_api_key("test-key");
    let config = ConnectionConfig::builder(server.url.clone(), auth)
        .message_buffer(4096)
        .build();
    let client = WebSocketClient::with_reconnection_config(
        config,
        ReconnectionConfig::disabled(),
    );

    client.connect().await.expect("connect");

    let receiver = client.messages();
    let drained = tokio::task::spawn_blocking(move || {
        let mut n = 0usize;
        while n < 50 {
            match receiver.receive_timeout(Duration::from_millis(500)) {
                Ok(Some(_)) => n += 1,
                _ => break,
            }
        }
        n
    });

    let _received = drained.await.unwrap_or(0);
    let dropped = client.messages_dropped_total();
    assert_eq!(
        dropped, 0,
        "no drops expected with adequate buffer + active consumer"
    );

    let _ = client
        .shutdown_with_timeout(Duration::from_millis(200))
        .await;
}
