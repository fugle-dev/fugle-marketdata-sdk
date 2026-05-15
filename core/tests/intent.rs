//! `DisconnectIntent` classification test.
//!
//! Three scenarios prove that `ConnectionEvent::Disconnected.intent`
//! is set correctly at the source:
//!   - Client: caller invokes `disconnect()`.
//!   - Server: server sends a Close frame.
//!   - Network: server drops the TCP socket without a Close handshake.

#![cfg(feature = "tokio-comp")]

#[path = "common/mod.rs"]
mod common;

use marketdata_core::aio::WebSocketClient;
use marketdata_core::websocket::{ConnectionEvent, DisconnectIntent};
use marketdata_core::{AuthRequest, ConnectionConfig, ReconnectionConfig};
use std::time::Duration;

async fn first_disconnect_intent(
    client: &WebSocketClient,
) -> Option<DisconnectIntent> {
    let events = std::sync::Arc::clone(client.state_events());
    tokio::time::timeout(Duration::from_secs(5), async move {
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
    .flatten()
}

fn make_client(url: String) -> WebSocketClient {
    let auth = AuthRequest::with_api_key("test-key");
    let config = ConnectionConfig::new(url, auth);
    WebSocketClient::with_reconnection_config(config, ReconnectionConfig::disabled())
}

#[tokio::test]
async fn intent_client_when_caller_disconnects() {
    let server = common::spawn(common::AfterAuth::Idle).await;
    let client = make_client(server.url.clone());
    client.connect().await.expect("connect");

    client
        .shutdown_with_timeout(Duration::from_millis(500))
        .await
        .expect("shutdown");

    assert_eq!(
        first_disconnect_intent(&client).await,
        Some(DisconnectIntent::Client)
    );
}

#[tokio::test]
async fn intent_server_when_peer_sends_close() {
    let server = common::spawn(common::AfterAuth::ServerCloseAfter {
        delay_ms: 50,
        code: 1000,
        reason: "server requested".to_string(),
    })
    .await;
    let client = make_client(server.url.clone());
    client.connect().await.expect("connect");

    assert_eq!(
        first_disconnect_intent(&client).await,
        Some(DisconnectIntent::Server)
    );
}

#[tokio::test]
async fn intent_network_when_peer_drops_socket() {
    let server = common::spawn(common::AfterAuth::ServerDropAfter { delay_ms: 50 }).await;
    let client = make_client(server.url.clone());
    client.connect().await.expect("connect");

    assert_eq!(
        first_disconnect_intent(&client).await,
        Some(DisconnectIntent::Network)
    );
}
