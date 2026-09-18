//! Connection event model delivery guarantees (#55).
//!
//! Each scenario runs against both the async and the sync client, driven by
//! `core::testing::MockWsServer`:
//!
//! - (a) events emitted during `connect()` are retained for a late reader;
//! - (b) an auth rejection yields `Unauthenticated { message, data }` and no
//!   `Error`;
//! - (c) a heartbeat timeout yields exactly one `Disconnected { Network }`,
//!   even when `disconnect()` follows (#47);
//! - (d) a close the reconnect policy does not retry carries
//!   `will_reconnect: false` and emits no `ReconnectFailed`;
//! - (e) a retried close carries `will_reconnect: true`, followed by
//!   `Reconnecting { 1 }` and `ReconnectFailed { 1 }`;
//! - (f) `disconnect()` during a reconnect backoff emits exactly one final
//!   `Disconnected { Client, will_reconnect: false }` and nothing else, and
//!   leaves the state `Closed { Client }` (#98).

#![cfg(all(feature = "test-utils", feature = "tokio-comp"))]

#[path = "common/mod.rs"]
mod common;

use marketdata_core::testing::MockWsServer;
use marketdata_core::websocket::{ConnectionEvent, DisconnectIntent};
use marketdata_core::{
    AuthRequest, ConnectionConfig, ConnectionState, HealthCheckConfig, ReconnectionConfig,
};
use serde_json::json;
use std::time::{Duration, Instant};

const WAIT: Duration = Duration::from_secs(5);

fn config(server: &MockWsServer) -> ConnectionConfig {
    ConnectionConfig::new(server.url(), AuthRequest::with_api_key("mock-test-key"))
}

fn short_heartbeat() -> HealthCheckConfig {
    HealthCheckConfig {
        enabled: true,
        heartbeat_timeout: Duration::from_millis(300),
        ..HealthCheckConfig::default()
    }
}

fn one_attempt() -> ReconnectionConfig {
    ReconnectionConfig::new(1, Duration::from_millis(100), Duration::from_millis(100))
        .expect("valid reconnection config")
}

/// Long enough to call `disconnect()` before the reconnect attempt starts.
fn slow_retry() -> ReconnectionConfig {
    ReconnectionConfig::new(3, Duration::from_millis(500), Duration::from_millis(500))
        .expect("valid reconnection config")
}

fn is_reconnecting(event: &ConnectionEvent) -> bool {
    matches!(event, ConnectionEvent::Reconnecting { .. })
}

/// `disconnect()` interrupted the reconnect with one final
/// `Disconnected { Client, will_reconnect: false }` and nothing else (#98).
fn assert_reconnect_stopped(after: &[ConnectionEvent], state: &ConnectionState) {
    assert!(
        matches!(
            after,
            [ConnectionEvent::Disconnected {
                intent: DisconnectIntent::Client,
                will_reconnect: false,
                ..
            }]
        ),
        "events after disconnect(): {after:?}"
    );
    assert!(
        matches!(state, ConnectionState::Closed { intent: DisconnectIntent::Client, .. }),
        "{state:?}"
    );
}

/// Everything currently queued, without waiting.
fn queued(rx: &common::EventReceiver) -> Vec<ConnectionEvent> {
    rx.try_iter().collect()
}

/// Receive until `done` matches an event (inclusive) or [`WAIT`] elapses.
fn recv_until(
    rx: &common::EventReceiver,
    done: impl Fn(&ConnectionEvent) -> bool,
) -> Vec<ConnectionEvent> {
    let deadline = Instant::now() + WAIT;
    let mut events = Vec::new();
    while let Some(left) = deadline.checked_duration_since(Instant::now()) {
        match rx.recv_timeout(left) {
            Ok(event) => {
                let stop = done(&event);
                events.push(event);
                if stop {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    events
}

fn drain(rx: &common::EventReceiver) -> Vec<ConnectionEvent> {
    common::drain_until_quiet(|d| rx.recv_timeout(d).ok())
}

fn is_disconnected(event: &ConnectionEvent) -> bool {
    matches!(event, ConnectionEvent::Disconnected { .. })
}

fn is_reconnect_failed(event: &ConnectionEvent) -> bool {
    matches!(event, ConnectionEvent::ReconnectFailed { .. })
}

fn authenticated_frame() -> serde_json::Value {
    json!({ "event": "authenticated", "data": { "message": "Authenticated successfully" } })
}

fn rejection_frame() -> serde_json::Value {
    json!({ "event": "error", "data": { "message": "Invalid token" } })
}

fn assert_connect_sequence(events: &[ConnectionEvent]) {
    assert_eq!(
        events,
        [
            ConnectionEvent::Connecting,
            ConnectionEvent::Connected,
            ConnectionEvent::Authenticated {
                data: json!({ "message": "Authenticated successfully" }),
            },
        ]
    );
}

fn assert_rejection_sequence(events: &[ConnectionEvent]) {
    assert_eq!(
        events,
        [
            ConnectionEvent::Connecting,
            ConnectionEvent::Connected,
            ConnectionEvent::Unauthenticated {
                message: "Invalid token".to_string(),
                data: json!({ "message": "Invalid token" }),
            },
        ]
    );
}

/// `HeartbeatTimeout` immediately followed by the connection's single
/// `Disconnected { Network }`.
fn assert_heartbeat_then_disconnected(events: &[ConnectionEvent]) {
    let idx = events
        .iter()
        .position(|e| matches!(e, ConnectionEvent::HeartbeatTimeout { .. }))
        .unwrap_or_else(|| panic!("no HeartbeatTimeout in {events:?}"));
    assert!(
        matches!(
            events.get(idx + 1),
            Some(ConnectionEvent::Disconnected {
                code: None,
                intent: DisconnectIntent::Network,
                will_reconnect: false,
                ..
            })
        ),
        "expected Disconnected {{ Network }} right after HeartbeatTimeout: {events:?}"
    );
}

fn assert_final_close(events: &[ConnectionEvent]) {
    let disconnects: Vec<_> = events.iter().filter(|e| is_disconnected(e)).collect();
    assert!(
        matches!(
            disconnects.as_slice(),
            [ConnectionEvent::Disconnected {
                intent: DisconnectIntent::Network,
                will_reconnect: false,
                ..
            }]
        ),
        "expected one final Disconnected: {events:?}"
    );
    assert!(
        !events.iter().any(is_reconnect_failed),
        "no ReconnectFailed without an attempt: {events:?}"
    );
}

fn assert_reconnect_exhausted(events: &[ConnectionEvent]) {
    let lifecycle: Vec<_> = events
        .iter()
        .filter(|e| {
            matches!(
                e,
                ConnectionEvent::Disconnected { .. }
                    | ConnectionEvent::Reconnecting { .. }
                    | ConnectionEvent::ReconnectFailed { .. }
            )
        })
        .collect();
    assert!(
        matches!(
            lifecycle.as_slice(),
            [
                ConnectionEvent::Disconnected {
                    intent: DisconnectIntent::Network,
                    will_reconnect: true,
                    ..
                },
                ConnectionEvent::Reconnecting { attempt: 1 },
                ConnectionEvent::ReconnectFailed { attempts: 1 },
            ]
        ),
        "unexpected reconnect lifecycle: {events:?}"
    );
}

mod aio {
    use super::*;
    use marketdata_core::aio::WebSocketClient;
    use marketdata_core::MarketDataError;

    async fn with_events<T: Send + 'static>(
        client: &WebSocketClient,
        f: impl FnOnce(&common::EventReceiver) -> T + Send + 'static,
    ) -> T {
        let events = common::EventReceiver::of_async(client);
        tokio::task::spawn_blocking(move || f(&events))
            .await
            .expect("event reader")
    }

    #[tokio::test]
    async fn connect_events_are_retained_for_a_late_reader() {
        let server = MockWsServer::start().await;
        server.set_auth_response(authenticated_frame());
        let client =
            WebSocketClient::with_reconnection_config(config(&server), ReconnectionConfig::disabled());

        client.connect().await.expect("connect");

        assert_connect_sequence(&with_events(&client, queued).await);
        client.disconnect().await.ok();
    }

    #[tokio::test]
    async fn rejection_emits_unauthenticated_with_data_and_no_error() {
        let server = MockWsServer::start().await;
        server.set_auth_response(rejection_frame());
        let client =
            WebSocketClient::with_reconnection_config(config(&server), ReconnectionConfig::disabled());

        let result = client.connect().await;

        assert!(
            matches!(result, Err(MarketDataError::AuthError { ref msg, .. }) if msg == "Invalid token"),
            "{result:?}"
        );
        assert_rejection_sequence(&with_events(&client, queued).await);
    }

    #[tokio::test]
    async fn heartbeat_timeout_reports_a_single_disconnect() {
        let server = MockWsServer::start().await;
        let client = WebSocketClient::with_full_config(
            config(&server),
            ReconnectionConfig::disabled(),
            short_heartbeat(),
        );
        client.connect().await.expect("connect");

        let mut events = with_events(&client, |rx| recv_until(rx, is_disconnected)).await;
        client.disconnect().await.ok();
        events.extend(with_events(&client, drain).await);

        assert_heartbeat_then_disconnected(&events);
        assert_eq!(events.iter().filter(|e| is_disconnected(e)).count(), 1, "{events:?}");
    }

    #[tokio::test]
    async fn unretried_close_is_final_without_reconnect_failed() {
        let server = MockWsServer::start().await;
        let client =
            WebSocketClient::with_reconnection_config(config(&server), ReconnectionConfig::disabled());
        client.connect().await.expect("connect");

        server.drop_transport().await;

        let mut events = with_events(&client, |rx| recv_until(rx, is_disconnected)).await;
        events.extend(with_events(&client, drain).await);
        assert_final_close(&events);
    }

    #[tokio::test]
    async fn retried_close_reconnects_then_fails() {
        let server = MockWsServer::start().await;
        let client = WebSocketClient::with_reconnection_config(config(&server), one_attempt());
        client.connect().await.expect("connect");

        // Capacity 1: the mock stops listening after the first client, so
        // the reconnect attempt is refused.
        server.drop_transport().await;

        let events = with_events(&client, |rx| recv_until(rx, is_reconnect_failed)).await;
        assert_reconnect_exhausted(&events);
    }

    #[tokio::test]
    async fn disconnect_during_reconnect_backoff_reports_the_final_disconnect() {
        // Capacity 2 so a reconnect that slipped through would succeed.
        let server = MockWsServer::start_with_capacity(2).await;
        let client = WebSocketClient::with_reconnection_config(config(&server), slow_retry());
        client.connect().await.expect("connect");

        server.drop_transport_for(0).await;
        let before = with_events(&client, |rx| recv_until(rx, is_reconnecting)).await;
        assert!(before.last().is_some_and(is_reconnecting), "{before:?}");

        client.disconnect().await.ok();

        let after = with_events(&client, drain).await;
        assert_reconnect_stopped(&after, &client.state_async().await);
    }
}

mod sync {
    use super::*;
    use marketdata_core::{MarketDataError, WebSocketClient};

    /// Run the sync client work off the runtime that drives the mock.
    async fn blocking<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
        tokio::task::spawn_blocking(f).await.expect("blocking task")
    }

    fn event_rx(client: &WebSocketClient) -> common::EventReceiver {
        common::EventReceiver::of_sync(client)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn connect_events_are_retained_for_a_late_reader() {
        let server = MockWsServer::start().await;
        server.set_auth_response(authenticated_frame());
        let config = config(&server);

        let events = blocking(move || {
            let client = WebSocketClient::with_reconnection_config(config, ReconnectionConfig::disabled());
            client.connect().expect("connect");
            let events = queued(&event_rx(&client));
            client.disconnect().ok();
            events
        })
        .await;

        assert_connect_sequence(&events);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn rejection_emits_unauthenticated_with_data_and_no_error() {
        let server = MockWsServer::start().await;
        server.set_auth_response(rejection_frame());
        let config = config(&server);

        let (result, events) = blocking(move || {
            let client = WebSocketClient::with_reconnection_config(config, ReconnectionConfig::disabled());
            let result = client.connect();
            let events = queued(&event_rx(&client));
            (result, events)
        })
        .await;

        assert!(
            matches!(result, Err(MarketDataError::AuthError { ref msg, .. }) if msg == "Invalid token"),
            "{result:?}"
        );
        assert_rejection_sequence(&events);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn heartbeat_timeout_reports_a_single_disconnect() {
        let server = MockWsServer::start().await;
        let config = config(&server);

        let events = blocking(move || {
            let client = WebSocketClient::with_full_config(
                config,
                ReconnectionConfig::disabled(),
                short_heartbeat(),
            );
            client.connect().expect("connect");
            let mut events = recv_until(&event_rx(&client), is_disconnected);
            client.disconnect().ok();
            events.extend(drain(&event_rx(&client)));
            events
        })
        .await;

        assert_heartbeat_then_disconnected(&events);
        assert_eq!(events.iter().filter(|e| is_disconnected(e)).count(), 1, "{events:?}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn unretried_close_is_final_without_reconnect_failed() {
        let server = MockWsServer::start().await;
        let config = config(&server);
        let client = blocking(move || {
            let client = WebSocketClient::with_reconnection_config(config, ReconnectionConfig::disabled());
            client.connect().expect("connect");
            client
        })
        .await;

        server.drop_transport().await;

        let events = blocking(move || {
            let mut events = recv_until(&event_rx(&client), is_disconnected);
            events.extend(drain(&event_rx(&client)));
            events
        })
        .await;
        assert_final_close(&events);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn retried_close_reconnects_then_fails() {
        let server = MockWsServer::start().await;
        let config = config(&server);
        let client = blocking(move || {
            let client = WebSocketClient::with_reconnection_config(config, one_attempt());
            client.connect().expect("connect");
            client
        })
        .await;

        server.drop_transport().await;

        let events = blocking(move || recv_until(&event_rx(&client), is_reconnect_failed)).await;
        assert_reconnect_exhausted(&events);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn disconnect_during_reconnect_backoff_reports_the_final_disconnect() {
        let server = MockWsServer::start_with_capacity(2).await;
        let config = config(&server);
        let client = blocking(move || {
            let client = WebSocketClient::with_reconnection_config(config, slow_retry());
            client.connect().expect("connect");
            client
        })
        .await;

        server.drop_transport_for(0).await;

        let (before, after, state) = blocking(move || {
            let before = recv_until(&event_rx(&client), is_reconnecting);
            client.disconnect().ok();
            let after = drain(&event_rx(&client));
            (before, after, client.state())
        })
        .await;
        assert!(before.last().is_some_and(is_reconnecting), "{before:?}");
        assert_reconnect_stopped(&after, &state);
    }
}
