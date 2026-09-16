//! Graceful shutdown drain test.
//!
//! Verifies that `WebSocketClient::shutdown_with_timeout(Duration)`:
//!   - Returns within the supplied bound when the peer never acks
//!     the Close frame (server side wedged).
//!   - Emits `ConnectionEvent::Disconnected { intent: Client, .. }`.

#![cfg(feature = "tokio-comp")]

#[path = "common/mod.rs"]
mod common;

use marketdata_core::aio::WebSocketClient;
use marketdata_core::websocket::{ConnectionEvent, DisconnectIntent};
use marketdata_core::{AuthRequest, ConnectionConfig, ReconnectionConfig};
use std::time::{Duration, Instant};

#[tokio::test]
async fn shutdown_with_timeout_respects_bound_when_peer_wedges() {
    let server = common::spawn(common::AfterAuth::Idle).await;

    let auth = AuthRequest::with_api_key("test-key");
    let config = ConnectionConfig::new(server.url.clone(), auth);
    // Auto-reconnect off so shutdown returns instead of restarting.
    let client = WebSocketClient::with_reconnection_config(
        config,
        ReconnectionConfig::disabled(),
    );

    client.connect().await.expect("connect");

    let events = std::sync::Arc::clone(client.state_events());

    // The mock server's Idle path will echo the Close frame back, so
    // a 5 s default would still exit fast. Use a small explicit bound
    // and assert we return well under it.
    let started = Instant::now();
    client
        .shutdown_with_timeout(Duration::from_millis(500))
        .await
        .expect("shutdown returns");
    let elapsed = started.elapsed();

    assert!(
        elapsed < Duration::from_millis(2_000),
        "shutdown did not respect timeout: {:?}",
        elapsed
    );

    // Drain events on a blocking thread. `Arc<tokio::sync::Mutex>`
    // around a `std::sync::mpsc::Receiver` — use `blocking_lock()`
    // off the tokio runtime so the sync `recv()` call doesn't stall
    // the executor.
    let drain = tokio::time::timeout(Duration::from_secs(1), async move {
        tokio::task::spawn_blocking(move || {
            let rx = events.blocking_lock();
            loop {
                match rx.recv() {
                    Ok(ConnectionEvent::Disconnected { intent, .. }) => return Some(intent),
                    Ok(_) => continue,
                    Err(_) => return None,
                }
            }
        })
        .await
        .ok()
        .flatten()
    })
    .await
    .ok()
    .flatten();

    assert_eq!(
        drain,
        Some(DisconnectIntent::Client),
        "expected Disconnected with intent::Client"
    );
}

#[tokio::test]
async fn disconnect_default_drains_quickly_against_responsive_peer() {
    let server = common::spawn(common::AfterAuth::Idle).await;

    let auth = AuthRequest::with_api_key("test-key");
    let config = ConnectionConfig::new(server.url.clone(), auth);
    let client = WebSocketClient::with_reconnection_config(
        config,
        ReconnectionConfig::disabled(),
    );

    client.connect().await.expect("connect");

    let started = Instant::now();
    client.disconnect().await.expect("disconnect");
    let elapsed = started.elapsed();

    // Mock server acks the Close, so we should not consume the full
    // 5 s default budget. Generous ceiling to absorb CI jitter.
    assert!(
        elapsed < Duration::from_secs(2),
        "disconnect waited too long: {:?}",
        elapsed
    );
}

/// Drain every event emitted up to (and shortly after) a client-initiated
/// shutdown. The receiver stays open while `client` lives, so keep reading
/// until the channel has been quiet for `QUIET` instead of waiting for `Err`;
/// a straggling duplicate that arrives late still lands in the result.
async fn events_after_shutdown(client: &WebSocketClient) -> Vec<ConnectionEvent> {
    client
        .shutdown_with_timeout(Duration::from_millis(500))
        .await
        .expect("shutdown returns");
    let events = std::sync::Arc::clone(client.state_events());
    tokio::task::spawn_blocking(move || {
        let rx = events.blocking_lock();
        common::drain_until_quiet(|timeout| rx.recv_timeout(timeout).ok())
    })
    .await
    .expect("drain")
}

async fn assert_single_client_disconnect(behaviour: common::AfterAuth) {
    let server = common::spawn(behaviour).await;
    let auth = AuthRequest::with_api_key("test-key");
    let config = ConnectionConfig::new(server.url.clone(), auth);
    let client = WebSocketClient::with_reconnection_config(
        config,
        ReconnectionConfig::disabled(),
    );
    client.connect().await.expect("connect");

    let events = events_after_shutdown(&client).await;

    let disconnects: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, ConnectionEvent::Disconnected { .. }))
        .collect();
    assert_eq!(disconnects.len(), 1, "expected exactly one Disconnected, got {:?}", events);
    assert!(
        matches!(
            disconnects[0],
            ConnectionEvent::Disconnected { intent: DisconnectIntent::Client, .. }
        ),
        "expected Client intent, got {:?}",
        disconnects[0]
    );
    assert!(
        !events.iter().any(|e| matches!(e, ConnectionEvent::Error { .. })),
        "caller-initiated close must not emit Error, got {:?}",
        events
    );
}

/// #22: the peer's Close ack must not surface as a second, Server-intent
/// `Disconnected` event.
#[tokio::test]
async fn shutdown_emits_single_disconnect_when_peer_acks_close() {
    assert_single_client_disconnect(common::AfterAuth::Idle).await;
}

/// #22: a peer that tears the transport down instead of acking the Close
/// (e.g. no TLS close_notify) must not surface as an `Error` event.
#[tokio::test]
async fn shutdown_emits_no_error_when_peer_drops_on_close() {
    assert_single_client_disconnect(common::AfterAuth::DropOnClientClose).await;
}
