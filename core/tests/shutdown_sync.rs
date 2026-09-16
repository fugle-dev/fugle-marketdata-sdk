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
use marketdata_core::{AuthRequest, ConnectionConfig, ReconnectionConfig, WebSocketClient};
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

        let rx = client.state_events().lock().expect("events lock");
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

        let rx = client.state_events().lock().expect("events lock");
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
