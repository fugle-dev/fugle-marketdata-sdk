//! Disconnect event test for the default (sync) `WebSocketClient`.
//!
//! Mirrors the aio cases in `shutdown.rs`: a caller-initiated
//! `disconnect()` must emit exactly one `Disconnected { intent: Client }`
//! and no `Error`, whether the peer acks the Close or drops the socket
//! (#22). The mock server needs tokio, hence the feature gate; the client
//! itself runs on `std::thread`.

#![cfg(feature = "tokio-comp")]

#[path = "common/mod.rs"]
mod common;

use marketdata_core::websocket::{ConnectionEvent, DisconnectIntent};
use marketdata_core::{
    error_code, AuthRequest, ConnectionConfig, MarketDataError, ReconnectionConfig, WebSocketClient,
};
use std::sync::Arc;
use std::time::Duration;

async fn assert_single_client_disconnect(behaviour: common::AfterAuth) {
    let notify = match &behaviour {
        common::AfterAuth::ServerCloseOnNotify { notify, .. } => Some(Arc::clone(notify)),
        _ => None,
    };
    let server = common::spawn(behaviour).await;

    let events = tokio::task::spawn_blocking(move || {
        let auth = AuthRequest::with_api_key("test-key");
        let config = ConnectionConfig::new(server.url.clone(), auth);
        let client =
            WebSocketClient::with_reconnection_config(config, ReconnectionConfig::disabled());
        client.connect().expect("connect");
        if let Some(notify) = notify {
            // The server's Close lands `delay_ms` after `should_stop` is set.
            notify.notify_one();
        }
        client
            .shutdown_with_timeout(Duration::from_secs(3))
            .expect("shutdown returns");

        let rx = common::EventReceiver::of_sync(&client);
        common::drain_until_quiet(|timeout| rx.recv_timeout(timeout).ok())
    })
    .await
    .expect("sync client thread");

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

#[tokio::test(flavor = "multi_thread")]
async fn sync_shutdown_emits_single_disconnect_when_peer_acks_close() {
    assert_single_client_disconnect(common::AfterAuth::Idle).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn sync_shutdown_emits_no_error_when_peer_drops_on_close() {
    assert_single_client_disconnect(common::AfterAuth::DropOnClientClose).await;
}

/// #22: a server Close that lands after `disconnect()` set `should_stop`
/// must not add a second, Server-intent `Disconnected`. The Close reaches
/// the owner thread either mid-read (the race branch) or as the reply to our
/// own Close; the assertion holds for both, so this test samples the race
/// without depending on timing. The race branch's decision itself is
/// covered deterministically by `peer_close_disconnect`'s unit tests (#41).
#[tokio::test(flavor = "multi_thread")]
async fn sync_shutdown_ignores_server_close_racing_disconnect() {
    assert_single_client_disconnect(common::AfterAuth::ServerCloseOnNotify {
        notify: Arc::new(tokio::sync::Notify::new()),
        delay_ms: 30,
    })
    .await;
}

/// #41: once the server's Close has been reported, a later `disconnect()`
/// (called twice here) emits no further `Disconnected`.
#[tokio::test(flavor = "multi_thread")]
async fn sync_disconnect_after_server_close_emits_no_second_disconnect() {
    let server = common::spawn(common::AfterAuth::ServerCloseAfter {
        delay_ms: 20,
        code: 1001,
        reason: "going away".to_string(),
    })
    .await;

    let events = tokio::task::spawn_blocking(move || {
        let config = ConnectionConfig::new(server.url.clone(), AuthRequest::with_api_key("k"));
        let client =
            WebSocketClient::with_reconnection_config(config, ReconnectionConfig::disabled());
        client.connect().expect("connect");

        let rx = common::EventReceiver::of_sync(&client);
        let mut events = Vec::new();
        while let Ok(event) = rx.recv_timeout(Duration::from_secs(5)) {
            let done = matches!(event, ConnectionEvent::Disconnected { .. });
            events.push(event);
            if done {
                break;
            }
        }
        client.disconnect().expect("disconnect");
        client.disconnect().expect("second disconnect");
        events.extend(common::drain_until_quiet(|timeout| rx.recv_timeout(timeout).ok()));
        events
    })
    .await
    .expect("sync client thread");

    let disconnects: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, ConnectionEvent::Disconnected { .. }))
        .collect();
    assert_eq!(disconnects.len(), 1, "expected exactly one Disconnected, got {:?}", events);
    assert!(
        matches!(
            disconnects[0],
            ConnectionEvent::Disconnected { intent: DisconnectIntent::Server, code: Some(1001), .. }
        ),
        "expected the server's Close, got {:?}",
        disconnects[0]
    );
}

/// #79: `force_close()` aborts like the async client — no Close frame, and
/// the socket is gone within one read-poll interval instead of after the
/// graceful drain / Close-ack wait.
#[tokio::test(flavor = "multi_thread")]
async fn sync_force_close_drops_socket_without_close_frame() {
    let (ended_tx, mut ended_rx) = tokio::sync::mpsc::unbounded_channel();
    let server = common::spawn(common::AfterAuth::ReportClientClose {
        ended_by_close: ended_tx,
    })
    .await;

    let client = tokio::task::spawn_blocking(move || {
        let config = ConnectionConfig::new(server.url.clone(), AuthRequest::with_api_key("k"));
        let client =
            WebSocketClient::with_reconnection_config(config, ReconnectionConfig::disabled());
        client.connect().expect("connect");
        client.force_close().expect("force close");
        // Kept alive past the assertion so only `force_close` can close the socket.
        client
    })
    .await
    .expect("sync client thread");

    let ended_by_close = tokio::time::timeout(Duration::from_secs(1), ended_rx.recv())
        .await
        .expect("socket still open 1s after force_close")
        .expect("server reports how the connection ended");
    assert!(!ended_by_close, "force_close must not send a Close frame");
    drop(client);
}

/// `connect()` while connected, or while another `connect()` is in its
/// handshake, is refused with 2011 and leaves the connection up (#119).
#[tokio::test(flavor = "multi_thread")]
async fn sync_connect_while_connected_or_connecting_is_refused() {
    let server = common::spawn(common::AfterAuth::Idle).await;

    tokio::task::spawn_blocking(move || {
        let config = ConnectionConfig::new(server.url.clone(), AuthRequest::with_api_key("k"));
        let client = Arc::new(WebSocketClient::with_reconnection_config(
            config,
            ReconnectionConfig::disabled(),
        ));

        let results: Vec<_> = (0..2)
            .map(|_| {
                let client = Arc::clone(&client);
                std::thread::spawn(move || client.connect())
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|t| t.join().expect("connect thread"))
            .collect();
        assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1, "{results:?}");
        assert!(
            results.iter().any(|r| matches!(r, Err(MarketDataError::AlreadyConnected))),
            "{results:?}"
        );

        let err = client.connect().expect_err("connect while connected");
        assert!(matches!(err, MarketDataError::AlreadyConnected), "{err:?}");
        assert_eq!(err.info().code, error_code::ALREADY_CONNECTED);
        assert!(client.is_connected());
        client.shutdown_with_timeout(Duration::from_secs(3)).expect("shutdown");
    })
    .await
    .expect("sync client thread");
}

/// Events up to and including the final `Disconnected { will_reconnect:
/// false }`, and those after it.
fn split_at_final_disconnect(events: &[ConnectionEvent]) -> (&[ConnectionEvent], &[ConnectionEvent]) {
    let end = events
        .iter()
        .position(|e| matches!(e, ConnectionEvent::Disconnected { will_reconnect: false, .. }))
        .unwrap_or_else(|| panic!("no final Disconnected in {events:?}"));
    events.split_at(end + 1)
}

/// `force_close()` while auto-reconnecting does not join the supervisor:
/// whatever step of the loop it lands in, nothing follows the final
/// `Disconnected`, and `Closed { Client }` is not overwritten (#145). The
/// window is narrow, so this guards against the loop reporting on
/// unchecked rather than reproducing the race every run.
#[tokio::test(flavor = "multi_thread")]
async fn sync_force_close_while_reconnecting_is_the_last_word() {
    for round in 0..8u64 {
        // One connection only: every reconnect attempt is refused at once.
        let server = common::spawn(common::AfterAuth::ServerDropAfter { delay_ms: 50 }).await;
        let (after, states) = tokio::task::spawn_blocking(move || {
            let config = ConnectionConfig::new(server.url.clone(), AuthRequest::with_api_key("k"));
            let reconnect = ReconnectionConfig::new(1_000, Duration::from_millis(100), Duration::from_millis(100))
                .expect("reconnection config");
            let client = WebSocketClient::with_reconnection_config(config, reconnect);
            client.connect().expect("connect");
            // Land in a different step of the loop each round.
            std::thread::sleep(Duration::from_millis(120 + round * 37));
            client.force_close().expect("force close");

            let mut states = Vec::new();
            for _ in 0..30 {
                states.push(client.state());
                std::thread::sleep(Duration::from_millis(10));
            }
            let rx = common::EventReceiver::of_sync(&client);
            let events = common::drain_until_quiet(|timeout| rx.recv_timeout(timeout).ok());
            let (_, after) = split_at_final_disconnect(&events);
            (after.to_vec(), states)
        })
        .await
        .expect("sync client thread");

        assert!(after.is_empty(), "round {round}: events after the final Disconnected: {after:?}");
        for state in states {
            assert!(
                matches!(
                    state,
                    marketdata_core::ConnectionState::Closed { intent: DisconnectIntent::Client, .. }
                ),
                "round {round}: state left Closed: {state:?}"
            );
        }
    }
}
