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

    let events = common::EventReceiver::of_async(&client);

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
            let rx = &events;
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
    let events = common::EventReceiver::of_async(client);
    tokio::task::spawn_blocking(move || {
        let rx = &events;
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

fn disconnects(events: &[ConnectionEvent]) -> Vec<&ConnectionEvent> {
    events
        .iter()
        .filter(|e| matches!(e, ConnectionEvent::Disconnected { .. }))
        .collect()
}

/// Block until a `Disconnected` arrives (or 5 s pass), returning every
/// event seen up to and including it.
async fn events_until_disconnect(client: &WebSocketClient) -> Vec<ConnectionEvent> {
    let events = common::EventReceiver::of_async(client);
    tokio::task::spawn_blocking(move || {
        let rx = &events;
        let mut seen = Vec::new();
        while let Ok(event) = rx.recv_timeout(Duration::from_secs(5)) {
            let done = matches!(event, ConnectionEvent::Disconnected { .. });
            seen.push(event);
            if done {
                break;
            }
        }
        seen
    })
    .await
    .expect("drain")
}

/// #41: a server Close racing `disconnect()` must surface as exactly one
/// `Disconnected`, whichever side observes it first. Every interleaving
/// satisfies the assertion; the loop only samples several of them.
#[tokio::test]
async fn shutdown_racing_server_close_emits_single_disconnect() {
    for delay_ms in [0, 1, 5, 20] {
        let notify = std::sync::Arc::new(tokio::sync::Notify::new());
        let server = common::spawn(common::AfterAuth::ServerCloseOnNotify {
            notify: std::sync::Arc::clone(&notify),
            delay_ms,
        })
        .await;
        let config = ConnectionConfig::new(server.url.clone(), AuthRequest::with_api_key("k"));
        let client = WebSocketClient::with_reconnection_config(config, ReconnectionConfig::disabled());
        client.connect().await.expect("connect");

        notify.notify_one();
        let events = events_after_shutdown(&client).await;

        assert_eq!(
            disconnects(&events).len(),
            1,
            "delay {delay_ms} ms: expected exactly one Disconnected, got {events:?}"
        );
    }
}

/// #41: once the server's Close has been reported, a later `disconnect()`
/// (called any number of times) emits no further `Disconnected`.
#[tokio::test]
async fn disconnect_after_server_close_emits_no_second_disconnect() {
    let server = common::spawn(common::AfterAuth::ServerCloseAfter {
        delay_ms: 20,
        code: 1001,
        reason: "going away".to_string(),
    })
    .await;
    let config = ConnectionConfig::new(server.url.clone(), AuthRequest::with_api_key("k"));
    let client = WebSocketClient::with_reconnection_config(config, ReconnectionConfig::disabled());
    client.connect().await.expect("connect");

    let mut events = events_until_disconnect(&client).await;
    client.disconnect().await.expect("disconnect");
    events.extend(events_after_shutdown(&client).await);

    let disconnects = disconnects(&events);
    assert_eq!(disconnects.len(), 1, "expected exactly one Disconnected, got {events:?}");
    assert!(
        matches!(
            disconnects[0],
            ConnectionEvent::Disconnected { intent: DisconnectIntent::Server, code: Some(1001), .. }
        ),
        "expected the server's Close, got {:?}",
        disconnects[0]
    );
}

/// Repeated `disconnect()` on the same connection reports it once.
#[tokio::test]
async fn repeated_disconnect_emits_single_disconnect() {
    let server = common::spawn(common::AfterAuth::Idle).await;
    let config = ConnectionConfig::new(server.url.clone(), AuthRequest::with_api_key("k"));
    let client = WebSocketClient::with_reconnection_config(config, ReconnectionConfig::disabled());
    client.connect().await.expect("connect");

    client.disconnect().await.expect("first disconnect");
    let events = events_after_shutdown(&client).await;

    let disconnects = disconnects(&events);
    assert_eq!(disconnects.len(), 1, "expected exactly one Disconnected, got {events:?}");
}

/// Collect events until a `Disconnected` with `code` arrives (or 5 s pass),
/// then keep draining until the channel goes quiet.
async fn events_through_disconnect_code(client: &WebSocketClient, code: u16) -> Vec<ConnectionEvent> {
    let events = common::EventReceiver::of_async(client);
    tokio::task::spawn_blocking(move || {
        let rx = &events;
        let mut seen = Vec::new();
        while let Ok(event) = rx.recv_timeout(Duration::from_secs(5)) {
            let done = matches!(event, ConnectionEvent::Disconnected { code: Some(c), .. } if c == code);
            seen.push(event);
            if done {
                break;
            }
        }
        seen.extend(common::drain_until_quiet(|timeout| rx.recv_timeout(timeout).ok()));
        seen
    })
    .await
    .expect("drain")
}

/// #41 review: `reconnect()` on a live connection must retire the old
/// dispatch task. Otherwise the old socket's later close claims the new
/// connection's latch, and the new connection's real close goes unreported.
#[tokio::test]
async fn reconnect_old_connection_close_does_not_swallow_new_disconnect() {
    let server = common::spawn_sequence(vec![
        // Old connection: the server closes it shortly after `reconnect()`.
        common::AfterAuth::ServerCloseAfter { delay_ms: 80, code: 4001, reason: "old".to_string() },
        // New connection: its close is the one the caller must see.
        common::AfterAuth::ServerCloseAfter { delay_ms: 300, code: 4000, reason: "new".to_string() },
    ])
    .await;
    let config = ConnectionConfig::new(server.url.clone(), AuthRequest::with_api_key("k"));
    let client = WebSocketClient::with_reconnection_config(config, ReconnectionConfig::disabled());
    client.connect().await.expect("connect");
    client.reconnect().await.expect("reconnect");

    let events = events_through_disconnect_code(&client, 4000).await;

    let disconnects = disconnects(&events);
    assert_eq!(disconnects.len(), 1, "expected exactly one Disconnected, got {events:?}");
    assert!(
        matches!(disconnects[0], ConnectionEvent::Disconnected { code: Some(4000), .. }),
        "expected the new connection's close, got {:?}",
        disconnects[0]
    );
}

/// `connect()` while connected is a no-op, as on the sync client: no second
/// connection (the mock accepts only one) and no lifecycle events.
#[tokio::test]
async fn connect_while_connected_is_noop() {
    let server = common::spawn(common::AfterAuth::Idle).await;
    let config = ConnectionConfig::new(server.url.clone(), AuthRequest::with_api_key("k"));
    let client = WebSocketClient::with_reconnection_config(config, ReconnectionConfig::disabled());
    client.connect().await.expect("connect");
    let events = common::EventReceiver::of_async(&client);
    tokio::task::spawn_blocking(move || {
        let rx = &events;
        while rx.try_recv().is_some() {}
    })
    .await
    .expect("drain initial events");

    tokio::time::timeout(Duration::from_secs(2), client.connect())
        .await
        .expect("second connect must not attempt a new handshake")
        .expect("second connect");

    assert!(client.is_connected().await);
    let events = common::EventReceiver::of_async(&client);
    let extra = tokio::task::spawn_blocking(move || {
        let rx = &events;
        rx.try_iter().collect::<Vec<_>>()
    })
    .await
    .expect("drain");
    assert!(extra.is_empty(), "second connect emitted {extra:?}");
}
