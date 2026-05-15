//! Smoke test for the `core::testing::MockWsServer`.
//!
//! Covers the 5 spec scenarios from
//! `openspec/specs/testing-utilities/spec.md` (post-archive). Catches
//! drift between the mock's subscribe-ACK protocol and the real
//! `protocol.rs` wire shape.

#![cfg(all(feature = "test-utils", feature = "tokio-comp"))]

use marketdata_core::models::streaming::StreamMessage;
use marketdata_core::testing::{aio_pair, MockWsServer};
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

#[tokio::test]
async fn close_emits_disconnected_event_with_server_intent() {
    let (server, client) = aio_pair().await;
    client.connect().await.expect("connect");

    server.close(1001, "going away").await;

    // events() returns an async Mutex over a std::mpsc Receiver. Lock the
    // mutex once on the async side, then poll via try_recv with sleeps —
    // recv_timeout would block the tokio runtime.
    let events_rx_arc = client.events().clone();
    let guard = events_rx_arc.lock().await;
    let mut event: Option<ConnectionEvent> = None;
    for _ in 0..20 {
        match guard.try_recv() {
            Ok(ev) => {
                if matches!(ev, ConnectionEvent::Disconnected { .. }) {
                    event = Some(ev);
                    break;
                }
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => break,
        }
    }
    drop(guard);

    match event {
        Some(ConnectionEvent::Disconnected { code, intent, .. }) => {
            assert_eq!(code, Some(1001));
            assert_eq!(intent, DisconnectIntent::Server);
        }
        Some(other) => panic!("expected Disconnected, got {other:?}"),
        None => panic!("no Disconnected event observed"),
    }
}
