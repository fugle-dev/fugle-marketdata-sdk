//! `ConnectionConfig::auth_timeout` bounds the auth handshake (#199).
//!
//! Each scenario runs against both the async and the sync client, driven by
//! `core::testing::MockWsServer` told not to answer the auth frame:
//!
//! - `connect()` fails with `TimeoutError { "WebSocket authentication" }`
//!   once the configured timeout elapses — not the 10 s that used to be
//!   hardcoded, and not `connect_timeout`, which covers only the upgrade;
//! - the stream reports the same failure as an `Error` event (code 3001);
//! - a reconnect attempt is bounded by the same value: the server that stops
//!   answering auth costs `auth_timeout`, then `ReconnectFailed`.

#![cfg(all(feature = "test-utils", feature = "tokio-comp"))]

#[path = "common/mod.rs"]
mod common;

use marketdata_core::error_code;
use marketdata_core::testing::MockWsServer;
use marketdata_core::websocket::ConnectionEvent;
use marketdata_core::{AuthRequest, ConnectionConfig, MarketDataError, ReconnectionConfig};
use std::time::{Duration, Instant};

const AUTH_TIMEOUT: Duration = Duration::from_millis(300);
/// Well under the old hardcoded 10 s: a client still using it fails this.
const WAIT: Duration = Duration::from_secs(3);

/// A config whose auth handshake gives up long before the default, with a
/// `connect_timeout` far longer so it cannot be the one that fires.
fn config(server: &MockWsServer) -> ConnectionConfig {
    ConnectionConfig::builder(server.url(), AuthRequest::with_api_key("mock-test-key"))
        .connect_timeout(Duration::from_secs(30))
        .auth_timeout(AUTH_TIMEOUT)
        .build()
}

fn assert_auth_timeout(result: &Result<(), MarketDataError>, elapsed: Duration) {
    assert!(
        matches!(
            result,
            Err(MarketDataError::TimeoutError { operation }) if operation == "WebSocket authentication"
        ),
        "{result:?}"
    );
    assert!(elapsed >= AUTH_TIMEOUT, "gave up before the timeout: {elapsed:?}");
    assert!(elapsed < WAIT, "took the old 10 s rather than auth_timeout: {elapsed:?}");
}

/// One attempt after a short backoff, so the reconnect's own wait is the
/// auth timeout and nothing else.
fn one_attempt() -> ReconnectionConfig {
    ReconnectionConfig::new(1, Duration::from_millis(100), Duration::from_millis(100))
        .expect("valid reconnection config")
}

/// Receive until `ReconnectFailed` arrives (inclusive) or [`WAIT`] elapses.
fn recv_until_reconnect_failed(rx: &common::EventReceiver) -> Vec<ConnectionEvent> {
    let deadline = Instant::now() + WAIT;
    let mut events = Vec::new();
    while let Some(left) = deadline.checked_duration_since(Instant::now()) {
        match rx.recv_timeout(left) {
            Ok(event) => {
                let done = matches!(event, ConnectionEvent::ReconnectFailed { .. });
                events.push(event);
                if done {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    events
}

/// The one attempt gave up on the unanswered auth after `auth_timeout`,
/// which is what the wall clock from the drop to `ReconnectFailed` shows.
fn assert_reconnect_timed_out(events: &[ConnectionEvent], elapsed: Duration) {
    assert!(
        matches!(events.last(), Some(ConnectionEvent::ReconnectFailed { attempts: 1 })),
        "{events:?}"
    );
    assert!(elapsed >= AUTH_TIMEOUT, "gave up before the timeout: {elapsed:?}");
    assert!(elapsed < WAIT, "took the old 10 s rather than auth_timeout: {elapsed:?}");
}

/// An `Error` event for the auth timeout (code 3001) is among `events`.
fn assert_error_event(events: &[ConnectionEvent]) {
    let timed_out = events.iter().any(|event| {
        matches!(
            event,
            ConnectionEvent::Error(info)
                if info.code == error_code::TIMEOUT
                    && info.message.contains("WebSocket authentication")
        )
    });
    assert!(timed_out, "no auth timeout Error event: {events:?}");
}

mod aio {
    use super::*;
    use marketdata_core::aio::WebSocketClient;

    #[tokio::test]
    async fn unanswered_auth_times_out_after_auth_timeout() {
        let server = MockWsServer::start().await;
        server.set_answer_auth(false);
        let client =
            WebSocketClient::with_reconnection_config(config(&server), ReconnectionConfig::disabled());

        let started = Instant::now();
        let result = client.connect().await;
        assert_auth_timeout(&result, started.elapsed());

        // Queued by the time connect() returned; try_iter() does not block.
        let events: Vec<_> = common::EventReceiver::of_async(&client).try_iter().collect();
        assert_error_event(&events);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn reconnect_gives_up_on_unanswered_auth_after_auth_timeout() {
        // Capacity 2: the reconnect attempt is accepted, then left hanging.
        let server = MockWsServer::start_with_capacity(2).await;
        let client = WebSocketClient::with_reconnection_config(config(&server), one_attempt());
        client.connect().await.expect("connect");
        let rx = common::EventReceiver::of_async(&client);
        let _: Vec<_> = rx.try_iter().collect();

        server.set_answer_auth(false);
        let dropped = Instant::now();
        server.drop_transport_for(0).await;

        let events = tokio::task::spawn_blocking(move || {
            let events = recv_until_reconnect_failed(&rx);
            (events, dropped.elapsed())
        })
        .await
        .expect("event reader");
        assert_reconnect_timed_out(&events.0, events.1);
        // The failed attempt is reported as an Error too (#200).
        assert_error_event(&events.0);
    }
}

mod sync {
    use super::*;
    use marketdata_core::WebSocketClient;

    #[tokio::test(flavor = "multi_thread")]
    async fn unanswered_auth_times_out_after_auth_timeout() {
        let server = MockWsServer::start().await;
        server.set_answer_auth(false);
        let config = config(&server);

        let (result, elapsed, events) = tokio::task::spawn_blocking(move || {
            let client =
                WebSocketClient::with_reconnection_config(config, ReconnectionConfig::disabled());
            let started = Instant::now();
            let result = client.connect();
            let elapsed = started.elapsed();
            let events: Vec<_> = common::EventReceiver::of_sync(&client).try_iter().collect();
            (result, elapsed, events)
        })
        .await
        .expect("blocking task");

        assert_auth_timeout(&result, elapsed);
        assert_error_event(&events);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn reconnect_gives_up_on_unanswered_auth_after_auth_timeout() {
        let server = MockWsServer::start_with_capacity(2).await;
        let config = config(&server);

        let client = tokio::task::spawn_blocking(move || {
            let client = WebSocketClient::with_reconnection_config(config, one_attempt());
            client.connect().expect("connect");
            let _: Vec<_> = common::EventReceiver::of_sync(&client).try_iter().collect();
            client
        })
        .await
        .expect("blocking task");

        server.set_answer_auth(false);
        let dropped = Instant::now();
        server.drop_transport_for(0).await;

        let (events, elapsed) = tokio::task::spawn_blocking(move || {
            let events = recv_until_reconnect_failed(&common::EventReceiver::of_sync(&client));
            (events, dropped.elapsed())
        })
        .await
        .expect("blocking task");
        assert_reconnect_timed_out(&events, elapsed);
        // The failed attempt is reported as an Error too, after the dropped
        // transport's own read error.
        assert_error_event(&events);
    }
}
