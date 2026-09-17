//! A consumer handling `Disconnected` reads a connection state that agrees
//! with it (#86), on the async and the sync client alike.
//!
//! - No reconnect follows: the state is already `Closed` with the event's
//!   code, reason and intent.
//! - A reconnect follows: the state is no longer `Connected`.
//!
//! The state is read on the consumer's own thread the moment the event
//! arrives, as a binding's listener would.

#![cfg(feature = "tokio-comp")]

#[path = "common/mod.rs"]
mod common;

use marketdata_core::websocket::{ConnectionEvent, DisconnectIntent};
use marketdata_core::{AuthRequest, ConnectionConfig, ConnectionState, ReconnectionConfig};
use std::time::Duration;

fn config(url: &str) -> ConnectionConfig {
    ConnectionConfig::new(url.to_string(), AuthRequest::with_api_key("test-key"))
}

/// Reconnects, but not before the test has finished reading the state.
fn slow_reconnect() -> ReconnectionConfig {
    ReconnectionConfig::new(5, Duration::from_secs(10), Duration::from_secs(10)).expect("valid")
}

/// Wait for `Disconnected` and read the state as soon as it arrives.
fn state_on_disconnected(
    events: common::EventReceiver,
    state: impl Fn() -> ConnectionState,
) -> (ConnectionEvent, ConnectionState) {
    loop {
        match events
            .recv_timeout(Duration::from_secs(5))
            .expect("Disconnected")
        {
            event @ ConnectionEvent::Disconnected { .. } => return (event, state()),
            _ => continue,
        }
    }
}

fn server_close() -> common::AfterAuth {
    common::AfterAuth::ServerCloseAfter {
        delay_ms: 50,
        code: 4001,
        reason: "bye".to_string(),
    }
}

fn assert_closed_like(event: &ConnectionEvent, state: &ConnectionState) {
    let ConnectionEvent::Disconnected {
        code,
        reason,
        intent,
        will_reconnect,
    } = event
    else {
        panic!("expected Disconnected, got {event:?}");
    };
    assert!(!will_reconnect, "{event:?}");
    assert_eq!(
        *state,
        ConnectionState::Closed {
            code: *code,
            reason: reason.clone(),
            intent: *intent
        },
        "{event:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn aio_server_close_without_reconnect_is_closed_when_reported() {
    let server = common::spawn(server_close()).await;
    let client = marketdata_core::aio::WebSocketClient::with_reconnection_config(
        config(&server.url),
        slow_reconnect(),
    );
    let events = common::EventReceiver::of_async(&client);
    let handle = client.state_handle();
    client.connect().await.expect("connect");

    let (event, state) =
        tokio::task::spawn_blocking(move || state_on_disconnected(events, || handle.state()))
            .await
            .unwrap();
    assert_closed_like(&event, &state);
    assert!(matches!(
        state,
        ConnectionState::Closed {
            code: Some(4001),
            intent: DisconnectIntent::Server,
            ..
        }
    ));
    // The dispatch task ends without overwriting it.
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(client.state(), state);
}

#[tokio::test(flavor = "multi_thread")]
async fn aio_network_drop_without_reconnect_is_closed_when_reported() {
    let server = common::spawn(common::AfterAuth::ServerDropAfter { delay_ms: 50 }).await;
    let client = marketdata_core::aio::WebSocketClient::with_reconnection_config(
        config(&server.url),
        ReconnectionConfig::disabled(),
    );
    let events = common::EventReceiver::of_async(&client);
    let handle = client.state_handle();
    client.connect().await.expect("connect");

    let (event, state) =
        tokio::task::spawn_blocking(move || state_on_disconnected(events, || handle.state()))
            .await
            .unwrap();
    assert_closed_like(&event, &state);
    assert!(matches!(
        state,
        ConnectionState::Closed {
            intent: DisconnectIntent::Network,
            ..
        }
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn aio_drop_before_reconnect_is_not_connected_when_reported() {
    let server = common::spawn(common::AfterAuth::ServerDropAfter { delay_ms: 50 }).await;
    let client = marketdata_core::aio::WebSocketClient::with_reconnection_config(
        config(&server.url),
        slow_reconnect(),
    );
    let events = common::EventReceiver::of_async(&client);
    let handle = client.state_handle();
    client.connect().await.expect("connect");

    let (event, state) =
        tokio::task::spawn_blocking(move || state_on_disconnected(events, || handle.state()))
            .await
            .unwrap();
    assert!(
        matches!(
            event,
            ConnectionEvent::Disconnected {
                will_reconnect: true,
                ..
            }
        ),
        "{event:?}"
    );
    assert!(
        matches!(
            state,
            ConnectionState::Disconnected | ConnectionState::Reconnecting { .. }
        ),
        "{state:?}"
    );
    client
        .shutdown_with_timeout(Duration::from_millis(500))
        .await
        .expect("shutdown");
}

/// Run a sync client against `behaviour` and read its state on `Disconnected`.
async fn sync_state_on_disconnected(
    behaviour: common::AfterAuth,
    reconnection: ReconnectionConfig,
) -> (ConnectionEvent, ConnectionState, ConnectionState) {
    let server = common::spawn(behaviour).await;
    tokio::task::spawn_blocking(move || {
        let client =
            std::sync::Arc::new(marketdata_core::WebSocketClient::with_reconnection_config(
                config(&server.url),
                reconnection,
            ));
        let events = common::EventReceiver::of_sync(&client);
        client.connect().expect("connect");
        let reader = std::sync::Arc::clone(&client);
        let (event, state) =
            std::thread::spawn(move || state_on_disconnected(events, || reader.state()))
                .join()
                .unwrap();
        // Whatever the supervisor does next, give it time to do it.
        std::thread::sleep(Duration::from_millis(300));
        let later = client.state();
        client
            .shutdown_with_timeout(Duration::from_millis(500))
            .expect("shutdown");
        (event, state, later)
    })
    .await
    .unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn sync_server_close_without_reconnect_is_closed_when_reported() {
    let (event, state, later) = sync_state_on_disconnected(server_close(), slow_reconnect()).await;
    assert_closed_like(&event, &state);
    assert!(matches!(
        state,
        ConnectionState::Closed {
            code: Some(4001),
            intent: DisconnectIntent::Server,
            ..
        }
    ));
    // The supervisor ends without overwriting it.
    assert_eq!(later, state);
}

#[tokio::test(flavor = "multi_thread")]
async fn sync_network_drop_without_reconnect_is_closed_when_reported() {
    let (event, state, _) = sync_state_on_disconnected(
        common::AfterAuth::ServerDropAfter { delay_ms: 50 },
        ReconnectionConfig::disabled(),
    )
    .await;
    assert_closed_like(&event, &state);
    assert!(matches!(
        state,
        ConnectionState::Closed {
            intent: DisconnectIntent::Network,
            ..
        }
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn sync_drop_before_reconnect_is_not_connected_when_reported() {
    let (event, state, _) = sync_state_on_disconnected(
        common::AfterAuth::ServerDropAfter { delay_ms: 50 },
        slow_reconnect(),
    )
    .await;
    assert!(
        matches!(
            event,
            ConnectionEvent::Disconnected {
                will_reconnect: true,
                ..
            }
        ),
        "{event:?}"
    );
    assert!(
        matches!(
            state,
            ConnectionState::Disconnected | ConnectionState::Reconnecting { .. }
        ),
        "{state:?}"
    );
}

// A caller closing the client after the connection reported its close keeps
// the state that report recorded (#93).

#[derive(Clone, Copy, Debug)]
enum ClientClose {
    Disconnect,
    ForceClose,
}

const CLOSES: [ClientClose; 2] = [ClientClose::Disconnect, ClientClose::ForceClose];

/// Run an async client against `behaviour`, close it with `close` once
/// `Disconnected` has arrived, and return that event and the state after.
async fn aio_state_after_client_close(
    behaviour: common::AfterAuth,
    reconnection: ReconnectionConfig,
    close: ClientClose,
) -> (ConnectionEvent, ConnectionState) {
    let server = common::spawn(behaviour).await;
    let client =
        marketdata_core::aio::WebSocketClient::with_reconnection_config(config(&server.url), reconnection);
    let events = common::EventReceiver::of_async(&client);
    let handle = client.state_handle();
    client.connect().await.expect("connect");
    let (event, _) =
        tokio::task::spawn_blocking(move || state_on_disconnected(events, || handle.state()))
            .await
            .unwrap();
    match close {
        ClientClose::Disconnect => client
            .shutdown_with_timeout(Duration::from_millis(500))
            .await
            .expect("disconnect"),
        ClientClose::ForceClose => client.force_close().await.expect("force_close"),
    }
    (event, client.state())
}

/// The sync counterpart of [`aio_state_after_client_close`].
async fn sync_state_after_client_close(
    behaviour: common::AfterAuth,
    reconnection: ReconnectionConfig,
    close: ClientClose,
) -> (ConnectionEvent, ConnectionState) {
    let server = common::spawn(behaviour).await;
    tokio::task::spawn_blocking(move || {
        let client =
            marketdata_core::WebSocketClient::with_reconnection_config(config(&server.url), reconnection);
        let events = common::EventReceiver::of_sync(&client);
        client.connect().expect("connect");
        let (event, _) = state_on_disconnected(events, || client.state());
        match close {
            ClientClose::Disconnect => client
                .shutdown_with_timeout(Duration::from_millis(500))
                .expect("disconnect"),
            ClientClose::ForceClose => client.force_close().expect("force_close"),
        }
        // A supervisor still winding down must not overwrite it either.
        std::thread::sleep(Duration::from_millis(300));
        (event, client.state())
    })
    .await
    .unwrap()
}

fn assert_server_close_kept(event: &ConnectionEvent, state: &ConnectionState, close: ClientClose) {
    assert_closed_like(event, state);
    assert!(
        matches!(
            state,
            ConnectionState::Closed {
                code: Some(4001),
                intent: DisconnectIntent::Server,
                ..
            }
        ),
        "{close:?}: {state:?}"
    );
}

fn assert_client_closed(state: &ConnectionState, close: ClientClose) {
    assert!(
        matches!(
            state,
            ConnectionState::Closed {
                intent: DisconnectIntent::Client,
                ..
            }
        ),
        "{close:?}: {state:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn aio_client_close_after_server_close_keeps_its_state() {
    for close in CLOSES {
        let (event, state) =
            aio_state_after_client_close(server_close(), slow_reconnect(), close).await;
        assert_server_close_kept(&event, &state, close);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn sync_client_close_after_server_close_keeps_its_state() {
    for close in CLOSES {
        let (event, state) =
            sync_state_after_client_close(server_close(), slow_reconnect(), close).await;
        assert_server_close_kept(&event, &state, close);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn aio_client_close_while_reconnecting_closes_the_client() {
    for close in CLOSES {
        let (_, state) = aio_state_after_client_close(
            common::AfterAuth::ServerDropAfter { delay_ms: 50 },
            slow_reconnect(),
            close,
        )
        .await;
        assert_client_closed(&state, close);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn sync_client_close_while_reconnecting_closes_the_client() {
    for close in CLOSES {
        let (_, state) = sync_state_after_client_close(
            common::AfterAuth::ServerDropAfter { delay_ms: 50 },
            slow_reconnect(),
            close,
        )
        .await;
        assert_client_closed(&state, close);
    }
}
