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
            // Let the owner thread settle into a blocking read, then have the
            // server's Close arrive just after `should_stop` is set.
            std::thread::sleep(Duration::from_millis(50));
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

/// #22: a server Close that lands while the owner thread is blocked in a
/// read after `disconnect()` set `should_stop` must not add a second,
/// Server-intent `Disconnected`. The read window is `READ_POLL_INTERVAL`
/// (200 ms), so a Close 30 ms after the call usually hits it.
#[tokio::test(flavor = "multi_thread")]
async fn sync_shutdown_ignores_server_close_racing_disconnect() {
    assert_single_client_disconnect(common::AfterAuth::ServerCloseOnNotify {
        notify: Arc::new(tokio::sync::Notify::new()),
        delay_ms: 30,
    })
    .await;
}
