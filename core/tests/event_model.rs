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
//!   `Reconnecting { 1 }`, the refused attempt's `Connecting` → `Error`
//!   (#200) and `ReconnectFailed { 1 }`;
//! - (f) `disconnect()` during a reconnect backoff emits exactly one final
//!   `Disconnected { Client, will_reconnect: false }` and nothing else, and
//!   leaves the state `Closed { Client }` (#98);
//! - (g) a reconnect attempt whose auth response never comes reports
//!   `Connecting` → `Connected` → `Error { TIMEOUT }` between its
//!   `Reconnecting` and the next one, which then succeeds (#200; driven by
//!   the `common` mock, which can withhold the auth response);
//! - (h) a reconnect attempt whose credentials are rejected (`error{1000}`)
//!   reports `Unauthenticated` → `ReconnectFailed { n }`, nothing after it,
//!   and leaves the state `Closed` (#201);
//! - (i) a reconnect attempt answered with `error{1011}` reports `Error`
//!   (not `Unauthenticated`) and the loop goes on to `Authenticated` (#201);
//! - (j) `error{1000}` followed by a Close without a code on a live
//!   connection is final: `Disconnected { will_reconnect: false }`, no
//!   reconnect (#201);
//! - (k) a Close without a code and no `error{1000}` before it reconnects as
//!   before (#201 regression);
//! - (l) a first `connect()` answered with `error{1011}` fails with
//!   `ConnectionError` and reports `Error`, not `Unauthenticated` (#201).

#![cfg(all(feature = "test-utils", feature = "tokio-comp"))]

#[path = "common/mod.rs"]
mod common;

use marketdata_core::testing::MockWsServer;
use marketdata_core::websocket::{ConnectionEvent, DisconnectIntent};
use marketdata_core::{
    error_code, AuthRequest, ConnectionConfig, ConnectionState, ErrorKind, HealthCheckConfig,
    MarketDataError, ReconnectionConfig,
};
use serde_json::json;
use std::time::{Duration, Instant};

const WAIT: Duration = Duration::from_secs(5);
/// Covers the clients' auth timeout plus a backoff. Both timeouts are
/// hard-coded 10 s constants (`sync::owner_thread::AUTH_TIMEOUT`, the
/// `authenticate` call in `aio::reconnect`), with no seam to shorten them.
const AUTH_TIMEOUT_WAIT: Duration = Duration::from_secs(20);

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

fn is_authenticated(event: &ConnectionEvent) -> bool {
    matches!(event, ConnectionEvent::Authenticated { .. })
}

/// The `common` mock: drops the first connection, withholds the auth
/// response on the second, authenticates the third.
async fn auth_timeout_once_server() -> common::MockServerHandle {
    common::spawn_sequence(vec![
        common::AfterAuth::ServerDropAfter { delay_ms: 100 },
        common::AfterAuth::NeverAuthenticate,
        common::AfterAuth::Idle,
    ])
    .await
}

fn common_config(server: &common::MockServerHandle) -> ConnectionConfig {
    ConnectionConfig::new(server.url.clone(), AuthRequest::with_api_key("mock-test-key"))
}

fn three_attempts() -> ReconnectionConfig {
    ReconnectionConfig::new(3, Duration::from_millis(100), Duration::from_millis(100))
        .expect("valid reconnection config")
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
    recv_until_within(rx, done, WAIT)
}

/// Receive until the second `Authenticated` (the reconnect that succeeded)
/// or [`AUTH_TIMEOUT_WAIT`] elapses.
fn recv_until_reconnected(rx: &common::EventReceiver) -> Vec<ConnectionEvent> {
    let seen = std::cell::Cell::new(0);
    recv_until_within(
        rx,
        |e| {
            if is_authenticated(e) {
                seen.set(seen.get() + 1);
            }
            seen.get() == 2
        },
        AUTH_TIMEOUT_WAIT,
    )
}

/// Receive until `done` matches an event (inclusive) or `timeout` elapses.
fn recv_until_within(
    rx: &common::EventReceiver,
    done: impl Fn(&ConnectionEvent) -> bool,
    timeout: Duration,
) -> Vec<ConnectionEvent> {
    let deadline = Instant::now() + timeout;
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

/// The server's rejection: `error` code `1000`, the message under `data`.
fn rejection_frame() -> serde_json::Value {
    json!({ "event": "error", "code": 1000, "data": { "message": "Invalid token" } })
}

const REJECTED: &str = "Invalid authentication credentials";
const AUTH_DOWN: &str = "Auth service unavailable";

/// The `common` mock: drops the first connection, rejects the credentials
/// of the second (`error{1000}` then a Close without a code, as the server
/// does).
async fn reject_on_reconnect_server() -> common::MockServerHandle {
    common::spawn_sequence(vec![
        common::AfterAuth::ServerDropAfter { delay_ms: 100 },
        common::AfterAuth::RejectAuth { code: 1000, message: REJECTED.into(), close: true },
    ])
    .await
}

/// The `common` mock: drops the first connection, answers the second's auth
/// frame with `error{1011}` (and, like the server, does not close),
/// authenticates the third.
async fn auth_down_once_server() -> common::MockServerHandle {
    common::spawn_sequence(vec![
        common::AfterAuth::ServerDropAfter { delay_ms: 100 },
        common::AfterAuth::RejectAuth { code: 1011, message: AUTH_DOWN.into(), close: false },
        common::AfterAuth::Idle,
    ])
    .await
}

/// The `common` mock: authenticates, then sends `error{1000}` and a Close
/// without a code.
async fn error_1000_then_close_server() -> common::MockServerHandle {
    common::spawn(common::AfterAuth::ErrorThenClose {
        delay_ms: 100,
        code: 1000,
        message: REJECTED.into(),
    })
    .await
}

/// The `common` mock: authenticates, closes without a code, authenticates
/// the reconnect.
async fn close_without_code_server() -> common::MockServerHandle {
    common::spawn_sequence(vec![
        common::AfterAuth::CloseWithoutCodeAfter { delay_ms: 100 },
        common::AfterAuth::Idle,
    ])
    .await
}

/// The `common` mock: answers the first auth frame with `error{1011}`.
async fn auth_down_server() -> common::MockServerHandle {
    common::spawn(common::AfterAuth::RejectAuth { code: 1011, message: AUTH_DOWN.into(), close: false })
        .await
}

fn is_unauthenticated(event: &ConnectionEvent) -> bool {
    matches!(event, ConnectionEvent::Unauthenticated { .. })
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

/// The one attempt was refused at the TCP level: it reports `Connecting`
/// and the refusal as `Error` before the loop gives up (#200). Both clients
/// report the refusal with the transport error's code, `WEBSOCKET`, and
/// kind `Network` (#201).
fn assert_reconnect_exhausted(events: &[ConnectionEvent]) {
    let lost = events
        .iter()
        .position(is_disconnected)
        .unwrap_or_else(|| panic!("no Disconnected in {events:?}"));
    assert!(
        matches!(
            &events[lost..],
            [
                ConnectionEvent::Disconnected {
                    intent: DisconnectIntent::Network,
                    will_reconnect: true,
                    ..
                },
                ConnectionEvent::Reconnecting { attempt: 1 },
                ConnectionEvent::Connecting,
                ConnectionEvent::Error(info),
                ConnectionEvent::ReconnectFailed { attempts: 1 },
            ] if info.code == error_code::WEBSOCKET && info.source_kind == ErrorKind::Network
        ),
        "unexpected reconnect lifecycle: {events:?}"
    );
}

/// The attempt's credentials were rejected: `Unauthenticated` with the
/// server's message and `data`, then `ReconnectFailed { 1 }`, and nothing
/// after it (#201).
fn assert_reconnect_rejected(events: &[ConnectionEvent], after: &[ConnectionEvent], state: &ConnectionState) {
    let lost = events
        .iter()
        .position(is_disconnected)
        .unwrap_or_else(|| panic!("no Disconnected in {events:?}"));
    // The dropped transport's `Disconnected` (its reason names the reset).
    assert!(
        matches!(
            &events[lost],
            ConnectionEvent::Disconnected {
                code: None,
                intent: DisconnectIntent::Network,
                will_reconnect: true,
                ..
            }
        ),
        "unexpected reconnect lifecycle: {events:?}"
    );
    assert_eq!(
        &events[lost + 1..],
        [
            ConnectionEvent::Reconnecting { attempt: 1 },
            ConnectionEvent::Connecting,
            ConnectionEvent::Connected,
            ConnectionEvent::Unauthenticated {
                message: REJECTED.to_string(),
                data: json!({ "message": REJECTED }),
            },
            ConnectionEvent::ReconnectFailed { attempts: 1 },
        ],
        "unexpected reconnect lifecycle: {events:?}"
    );
    assert!(after.is_empty(), "events after ReconnectFailed: {after:?}");
    assert_eq!(
        *state,
        ConnectionState::Closed {
            code: None,
            reason: format!("Credentials rejected: {REJECTED}"),
            intent: DisconnectIntent::Server,
        }
    );
}

/// The first attempt was answered with `error{1011}`: it reports
/// `Connecting` → `Connected` → `Error { CONNECTION }` naming the code, then
/// the second attempt runs sequence 2 to `Authenticated`; no
/// `Unauthenticated` anywhere (#201).
fn assert_attempt_auth_down_reported(events: &[ConnectionEvent]) {
    let attempt = |n: u32| {
        events
            .iter()
            .position(|e| matches!(e, ConnectionEvent::Reconnecting { attempt } if *attempt == n))
            .unwrap_or_else(|| panic!("no Reconnecting {{ {n} }} in {events:?}"))
    };
    let (first, second) = (attempt(1), attempt(2));
    assert!(
        matches!(
            &events[first + 1..second],
            [
                ConnectionEvent::Connecting,
                ConnectionEvent::Connected,
                ConnectionEvent::Error(info),
            ] if info.code == error_code::CONNECTION
                && info.message.ends_with(&format!("Authentication failed (server error 1011): {AUTH_DOWN}"))
        ),
        "unexpected first attempt: {events:?}"
    );
    assert!(
        matches!(
            &events[second + 1..],
            [
                ConnectionEvent::Connecting,
                ConnectionEvent::Connected,
                ConnectionEvent::Authenticated { .. },
            ]
        ),
        "unexpected second attempt: {events:?}"
    );
    assert!(!events.iter().any(is_unauthenticated), "{events:?}");
}

/// `error{1000}` then a Close without a code ends the connection for good:
/// one `Disconnected { Server, code: None, will_reconnect: false }`, no
/// `Reconnecting`, the state `Closed` (#201).
fn assert_rejection_close_is_final(events: &[ConnectionEvent], state: &ConnectionState) {
    let disconnects: Vec<_> = events.iter().filter(|e| is_disconnected(e)).collect();
    assert_eq!(
        disconnects,
        [&ConnectionEvent::Disconnected {
            code: None,
            reason: "Server initiated close".to_string(),
            intent: DisconnectIntent::Server,
            will_reconnect: false,
        }],
        "{events:?}"
    );
    assert!(!events.iter().any(is_reconnecting), "no reconnect: {events:?}");
    assert!(!events.iter().any(is_reconnect_failed), "{events:?}");
    assert_eq!(
        *state,
        ConnectionState::Closed {
            code: None,
            reason: "Server initiated close".to_string(),
            intent: DisconnectIntent::Server,
        }
    );
}

/// A Close without a code, and no `error{1000}` before it, reconnects:
/// `Disconnected { will_reconnect: true }` → `Reconnecting { 1 }` →
/// sequence 2 (#201 regression).
fn assert_close_without_code_reconnects(events: &[ConnectionEvent]) {
    let lost = events
        .iter()
        .position(is_disconnected)
        .unwrap_or_else(|| panic!("no Disconnected in {events:?}"));
    assert_eq!(
        &events[lost..],
        [
            ConnectionEvent::Disconnected {
                code: None,
                reason: "Server initiated close".to_string(),
                intent: DisconnectIntent::Server,
                will_reconnect: true,
            },
            ConnectionEvent::Reconnecting { attempt: 1 },
            ConnectionEvent::Connecting,
            ConnectionEvent::Connected,
            ConnectionEvent::Authenticated { data: serde_json::Value::Null },
        ],
        "unexpected reconnect lifecycle: {events:?}"
    );
}

/// A first `connect()` answered with `error{1011}`: `Connecting` →
/// `Connected` → `Error { CONNECTION }`, no `Unauthenticated` (#201).
fn assert_connect_auth_down(result: &Result<(), MarketDataError>, events: &[ConnectionEvent]) {
    assert!(
        matches!(
            result,
            Err(MarketDataError::ConnectionError { msg })
                if *msg == format!("Authentication failed (server error 1011): {AUTH_DOWN}")
        ),
        "{result:?}"
    );
    assert!(
        matches!(
            events,
            [
                ConnectionEvent::Connecting,
                ConnectionEvent::Connected,
                ConnectionEvent::Error(info),
            ] if info.code == error_code::CONNECTION
        ),
        "{events:?}"
    );
}

/// The first attempt got a transport but no auth response: it reports
/// `Connecting` → `Connected` → `Error { TIMEOUT }`, then the second attempt
/// runs sequence 2 to `Authenticated` (#200).
fn assert_attempt_timeout_reported(events: &[ConnectionEvent]) {
    let attempt = |n: u32| {
        events
            .iter()
            .position(|e| matches!(e, ConnectionEvent::Reconnecting { attempt } if *attempt == n))
            .unwrap_or_else(|| panic!("no Reconnecting {{ {n} }} in {events:?}"))
    };
    let (first, second) = (attempt(1), attempt(2));
    assert!(
        matches!(
            &events[first + 1..second],
            [
                ConnectionEvent::Connecting,
                ConnectionEvent::Connected,
                ConnectionEvent::Error(info),
            ] if info.code == error_code::TIMEOUT
        ),
        "unexpected first attempt: {events:?}"
    );
    assert!(
        matches!(
            &events[second + 1..],
            [
                ConnectionEvent::Connecting,
                ConnectionEvent::Connected,
                ConnectionEvent::Authenticated { .. },
            ]
        ),
        "unexpected second attempt: {events:?}"
    );
}

mod aio {
    use super::*;
    use marketdata_core::aio::WebSocketClient;

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
    async fn reconnect_attempt_without_auth_response_reports_timeout_error() {
        let server = auth_timeout_once_server().await;
        let client = WebSocketClient::with_reconnection_config(common_config(&server), three_attempts());
        client.connect().await.expect("connect");

        let events = with_events(&client, recv_until_reconnected).await;
        client.disconnect().await.ok();

        assert_attempt_timeout_reported(&events);
    }

    #[tokio::test]
    async fn rejected_reconnect_attempt_stops_with_reconnect_failed() {
        let server = reject_on_reconnect_server().await;
        let client = WebSocketClient::with_reconnection_config(common_config(&server), three_attempts());
        client.connect().await.expect("connect");

        let events = with_events(&client, |rx| recv_until(rx, is_reconnect_failed)).await;
        let after = with_events(&client, drain).await;

        assert_reconnect_rejected(&events, &after, &client.state_async().await);
    }

    #[tokio::test]
    async fn reconnect_attempt_with_auth_service_down_reports_error_and_goes_on() {
        let server = auth_down_once_server().await;
        let client = WebSocketClient::with_reconnection_config(common_config(&server), three_attempts());
        client.connect().await.expect("connect");

        let events = with_events(&client, recv_until_reconnected).await;
        client.disconnect().await.ok();

        assert_attempt_auth_down_reported(&events);
    }

    #[tokio::test]
    async fn error_1000_then_close_without_code_is_final() {
        let server = error_1000_then_close_server().await;
        let client = WebSocketClient::with_reconnection_config(common_config(&server), three_attempts());
        client.connect().await.expect("connect");

        let mut events = with_events(&client, |rx| recv_until(rx, is_disconnected)).await;
        events.extend(with_events(&client, drain).await);

        assert_rejection_close_is_final(&events, &client.state_async().await);
    }

    #[tokio::test]
    async fn close_without_code_and_no_rejection_reconnects() {
        let server = close_without_code_server().await;
        let client = WebSocketClient::with_reconnection_config(common_config(&server), three_attempts());
        client.connect().await.expect("connect");

        let events = with_events(&client, recv_until_reconnected).await;
        client.disconnect().await.ok();

        assert_close_without_code_reconnects(&events);
    }

    #[tokio::test]
    async fn connect_with_auth_service_down_fails_with_error_not_unauthenticated() {
        let server = auth_down_server().await;
        let client =
            WebSocketClient::with_reconnection_config(common_config(&server), ReconnectionConfig::disabled());

        let result = client.connect().await;

        assert_connect_auth_down(&result, &with_events(&client, queued).await);
        assert_eq!(client.state_async().await, ConnectionState::Disconnected);
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
    use marketdata_core::WebSocketClient;

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
    async fn reconnect_attempt_without_auth_response_reports_timeout_error() {
        let server = auth_timeout_once_server().await;
        let config = common_config(&server);

        let events = blocking(move || {
            let client = WebSocketClient::with_reconnection_config(config, three_attempts());
            client.connect().expect("connect");
            let events = recv_until_reconnected(&event_rx(&client));
            client.disconnect().ok();
            events
        })
        .await;

        assert_attempt_timeout_reported(&events);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn rejected_reconnect_attempt_stops_with_reconnect_failed() {
        let server = reject_on_reconnect_server().await;
        let config = common_config(&server);

        let (events, after, state) = blocking(move || {
            let client = WebSocketClient::with_reconnection_config(config, three_attempts());
            client.connect().expect("connect");
            let events = recv_until(&event_rx(&client), is_reconnect_failed);
            let after = drain(&event_rx(&client));
            (events, after, client.state())
        })
        .await;

        assert_reconnect_rejected(&events, &after, &state);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn reconnect_attempt_with_auth_service_down_reports_error_and_goes_on() {
        let server = auth_down_once_server().await;
        let config = common_config(&server);

        let events = blocking(move || {
            let client = WebSocketClient::with_reconnection_config(config, three_attempts());
            client.connect().expect("connect");
            let events = recv_until_reconnected(&event_rx(&client));
            client.disconnect().ok();
            events
        })
        .await;

        assert_attempt_auth_down_reported(&events);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn error_1000_then_close_without_code_is_final() {
        let server = error_1000_then_close_server().await;
        let config = common_config(&server);

        let (events, state) = blocking(move || {
            let client = WebSocketClient::with_reconnection_config(config, three_attempts());
            client.connect().expect("connect");
            let mut events = recv_until(&event_rx(&client), is_disconnected);
            events.extend(drain(&event_rx(&client)));
            (events, client.state())
        })
        .await;

        assert_rejection_close_is_final(&events, &state);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn close_without_code_and_no_rejection_reconnects() {
        let server = close_without_code_server().await;
        let config = common_config(&server);

        let events = blocking(move || {
            let client = WebSocketClient::with_reconnection_config(config, three_attempts());
            client.connect().expect("connect");
            let events = recv_until_reconnected(&event_rx(&client));
            client.disconnect().ok();
            events
        })
        .await;

        assert_close_without_code_reconnects(&events);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn connect_with_auth_service_down_fails_with_error_not_unauthenticated() {
        let server = auth_down_server().await;
        let config = common_config(&server);

        let (result, events, state) = blocking(move || {
            let client = WebSocketClient::with_reconnection_config(config, ReconnectionConfig::disabled());
            let result = client.connect();
            let events = queued(&event_rx(&client));
            (result, events, client.state())
        })
        .await;

        assert_connect_auth_down(&result, &events);
        assert_eq!(state, ConnectionState::Disconnected);
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
