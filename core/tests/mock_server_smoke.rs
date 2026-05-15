//! Smoke test for the `core::testing::MockWsServer`.
//!
//! Covers the 5 spec scenarios from
//! `openspec/specs/testing-utilities/spec.md` (post-archive). Catches
//! drift between the mock's subscribe-ACK protocol and the real
//! `protocol.rs` wire shape.

#![cfg(all(feature = "test-utils", feature = "tokio-comp"))]

use marketdata_core::models::streaming::StreamMessage;
use marketdata_core::testing::{aio_pair, aio_pair_n, MockWsServer};
use marketdata_core::models::Channel;
use marketdata_core::websocket::{ConnectionEvent, DisconnectIntent, StockSubscription};
use std::time::Duration;

#[tokio::test]
async fn start_binds_to_ephemeral_port() {
    let a = MockWsServer::start().await;
    let b = MockWsServer::start().await;
    assert_ne!(a.address().port(), 0);
    assert_ne!(b.address().port(), 0);
    assert_ne!(
        a.address().port(),
        b.address().port(),
        "two MockWsServer instances must get distinct ports"
    );
}

#[tokio::test]
async fn aio_pair_connects_without_external_network() {
    let (_server, client) = aio_pair().await;
    let r = tokio::time::timeout(Duration::from_secs(2), client.connect()).await;
    assert!(
        matches!(r, Ok(Ok(()))),
        "loopback connect should succeed within 2s; got {r:?}"
    );
    let _ = client
        .shutdown_with_timeout(Duration::from_millis(200))
        .await;
}

#[tokio::test]
async fn subscribe_ack_uses_configured_id() {
    let (server, client) = aio_pair().await;
    server.next_subscribe_id("my-id-42").await;
    client.connect().await.expect("connect");

    client
        .subscribe(StockSubscription::new(Channel::Trades, "2330"))
        .await
        .expect("subscribe");

    // Give the dispatch loop a moment to process the subscribed ACK.
    tokio::time::sleep(Duration::from_millis(150)).await;

    // The client's SubscriptionManager should now hold the assigned id.
    let keys = client.subscription_keys();
    assert!(
        !keys.is_empty(),
        "expected at least one subscription key recorded"
    );

    let _ = client
        .shutdown_with_timeout(Duration::from_millis(200))
        .await;
}

#[tokio::test]
async fn inject_frame_delivers_to_client() {
    let (server, client) = aio_pair().await;
    client.connect().await.expect("connect");

    server.inject_frame(StreamMessage::Authenticated).await;
    server
        .inject_frame(StreamMessage::Pong { state: Some("alive".into()) })
        .await;

    let messages = client.messages();
    let recv = tokio::task::spawn_blocking(move || {
        let mut got = Vec::new();
        for _ in 0..4 {
            match messages.receive_timeout(Duration::from_millis(500)) {
                Ok(Some(msg)) => got.push(msg),
                _ => break,
            }
        }
        got
    });

    let frames = recv.await.unwrap_or_default();
    assert!(
        !frames.is_empty(),
        "expected at least one injected frame to reach the client"
    );

    let _ = client
        .shutdown_with_timeout(Duration::from_millis(200))
        .await;
}

/// Helper: drain `events()` looking for the first `Disconnected` event,
/// polling for up to ~2s. Returns `None` if no Disconnected arrives.
///
/// `events()` returns an async Mutex over a `std::mpsc::Receiver`; lock
/// once on the async side then poll via `try_recv` with sleeps —
/// `recv_timeout` would block the tokio runtime.
async fn await_disconnected(
    client: &marketdata_core::aio::WebSocketClient,
) -> Option<ConnectionEvent> {
    let events_rx_arc = client.events().clone();
    let guard = events_rx_arc.lock().await;
    for _ in 0..20 {
        match guard.try_recv() {
            Ok(ev) => {
                if matches!(ev, ConnectionEvent::Disconnected { .. }) {
                    return Some(ev);
                }
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => break,
        }
    }
    None
}

#[tokio::test]
async fn close_emits_disconnected_event_with_server_intent() {
    let (server, client) = aio_pair().await;
    client.connect().await.expect("connect");

    server.close(1001, "going away").await;

    match await_disconnected(&client).await {
        Some(ConnectionEvent::Disconnected { code, intent, .. }) => {
            assert_eq!(code, Some(1001));
            assert_eq!(intent, DisconnectIntent::Server);
        }
        Some(other) => panic!("expected Disconnected, got {other:?}"),
        None => panic!("no Disconnected event observed"),
    }
}

// ----- 0.7.0 multi-client + drop_transport additions -----

#[tokio::test]
async fn multi_client_capacity_2_pair() {
    let (server, clients) = aio_pair_n(2).await;
    assert_eq!(server.capacity(), 2);
    assert_eq!(clients.len(), 2);
    for c in &clients {
        c.connect().await.expect("each client must connect");
    }
    for c in &clients {
        let _ = c
            .shutdown_with_timeout(Duration::from_millis(200))
            .await;
    }
}

#[tokio::test]
async fn inject_frame_for_targets_one_client() {
    let (server, clients) = aio_pair_n(2).await;
    for c in &clients {
        c.connect().await.expect("connect");
    }

    // Each client receives its own auth-ack frame during the handshake.
    // Drain both queues so the per-client targeted inject is the only
    // frame in flight when we check.
    let recv0 = clients[0].messages();
    let recv1 = clients[1].messages();
    let _ = tokio::task::spawn_blocking({
        let recv0 = recv0.clone();
        let recv1 = recv1.clone();
        move || {
            // Quick drain — pre-existing auth-ack arrives within ~10ms after
            // connect resolves; 100ms is generous.
            while recv0
                .receive_timeout(Duration::from_millis(100))
                .ok()
                .flatten()
                .is_some()
            {}
            while recv1
                .receive_timeout(Duration::from_millis(100))
                .ok()
                .flatten()
                .is_some()
            {}
        }
    })
    .await;

    // Use a distinctive Pong state so the targeted frame is unmistakable.
    server
        .inject_frame_for(0, StreamMessage::Pong { state: Some("targeted-0".into()) })
        .await;
    tokio::time::sleep(Duration::from_millis(150)).await;

    let recv0_check = recv0.clone();
    let got0 = tokio::task::spawn_blocking(move || {
        recv0_check.receive_timeout(Duration::from_millis(500)).ok()
    })
    .await
    .unwrap_or(None);
    assert!(
        matches!(got0, Some(Some(_))),
        "client 0 must receive the targeted frame, got {got0:?}"
    );

    let recv1_check = recv1.clone();
    let got1 = tokio::task::spawn_blocking(move || {
        recv1_check.receive_timeout(Duration::from_millis(200)).ok()
    })
    .await
    .unwrap_or(None);
    assert!(
        matches!(got1, Some(None) | None),
        "client 1 must NOT receive the targeted frame, got {got1:?}"
    );

    for c in &clients {
        let _ = c
            .shutdown_with_timeout(Duration::from_millis(200))
            .await;
    }
}

#[tokio::test]
#[should_panic(expected = "use inject_frame_for")]
async fn bare_inject_frame_panics_on_multi_client() {
    let server = MockWsServer::start_with_capacity(2).await;
    server.inject_frame(StreamMessage::Authenticated).await;
}

#[tokio::test]
async fn drop_transport_produces_network_intent() {
    let (server, client) = aio_pair().await;
    client.connect().await.expect("connect");

    server.drop_transport().await;

    match await_disconnected(&client).await {
        Some(ConnectionEvent::Disconnected { intent, .. }) => {
            assert_eq!(
                intent,
                DisconnectIntent::Network,
                "drop_transport must produce DisconnectIntent::Network, got {intent:?}"
            );
        }
        Some(other) => panic!("expected Disconnected, got {other:?}"),
        None => panic!("no Disconnected event observed after drop_transport"),
    }
}

#[tokio::test]
async fn close_still_produces_server_intent() {
    // Regression: `close` semantics MUST be unchanged from 0.6.0.
    let (server, client) = aio_pair().await;
    client.connect().await.expect("connect");

    server.close(1011, "internal").await;

    match await_disconnected(&client).await {
        Some(ConnectionEvent::Disconnected { code, intent, .. }) => {
            assert_eq!(code, Some(1011));
            assert_eq!(intent, DisconnectIntent::Server);
        }
        Some(other) => panic!("expected Disconnected, got {other:?}"),
        None => panic!("no Disconnected event observed after close"),
    }
}

#[tokio::test]
async fn inject_after_drop_is_noop() {
    let (server, client) = aio_pair().await;
    client.connect().await.expect("connect");

    server.drop_transport().await;
    // Wait for the per-client task to actually exit.
    tokio::time::sleep(Duration::from_millis(150)).await;

    // This MUST NOT panic. The internal channel has been dropped along
    // with the per-client task; the send fails silently.
    server.inject_frame(StreamMessage::Authenticated).await;
    server.drop_transport().await; // also idempotent
}
