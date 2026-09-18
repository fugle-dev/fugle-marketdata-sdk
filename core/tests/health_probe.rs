//! Health check probe and `measure_latency()` (#150), for both the async and
//! the sync client, driven by `core::testing::MockWsServer`:
//!
//! - an answered probe keeps a silent connection alive, and its pong is not
//!   delivered to the caller;
//! - an unanswered probe declares the connection dead after
//!   `idle_probe_after + probe_timeout`;
//! - inbound traffic keeps the silence short, so no probe is sent;
//! - passive mode never sends a ping;
//! - `measure_latency()` returns the round trip, fails with `TimeoutError`,
//!   `ClientClosed` or `ConnectionError`, and its pong is not delivered;
//! - a caller's own `ping()` pong is still delivered.

#![cfg(all(feature = "test-utils", feature = "tokio-comp"))]

#[path = "common/mod.rs"]
mod common;

use marketdata_core::models::streaming::StreamMessage;
use marketdata_core::testing::MockWsServer;
use marketdata_core::websocket::{ConnectionEvent, DisconnectIntent};
use marketdata_core::{AuthRequest, ConnectionConfig, HealthCheckConfig, ReconnectionConfig};
use std::time::{Duration, Instant};

const IDLE: Duration = Duration::from_millis(300);
const PROBE_TIMEOUT: Duration = Duration::from_millis(300);
const WAIT: Duration = Duration::from_secs(5);

fn config(server: &MockWsServer) -> ConnectionConfig {
    ConnectionConfig::new(server.url(), AuthRequest::with_api_key("mock-test-key"))
}

/// Probe mode with test-sized windows (below the public floors).
fn probe() -> HealthCheckConfig {
    HealthCheckConfig {
        probe_enabled: true,
        idle_probe_after: Some(IDLE),
        probe_timeout: Some(PROBE_TIMEOUT),
        ..HealthCheckConfig::default()
    }
}

fn passive() -> HealthCheckConfig {
    HealthCheckConfig {
        heartbeat_timeout: Duration::from_millis(300),
        ..HealthCheckConfig::default()
    }
}

fn probe_pings(server: &MockWsServer) -> usize {
    server
        .pings_received()
        .iter()
        .filter(|data| data["state"] == "fugle-sdk:probe")
        .count()
}

fn is_disconnected(event: &ConnectionEvent) -> bool {
    matches!(event, ConnectionEvent::Disconnected { .. })
}

/// Events until the first `Disconnected` (inclusive) or [`WAIT`].
fn until_disconnected(rx: &common::EventReceiver) -> Vec<ConnectionEvent> {
    let deadline = Instant::now() + WAIT;
    let mut events = Vec::new();
    while let Some(left) = deadline.checked_duration_since(Instant::now()) {
        match rx.recv_timeout(left) {
            Ok(event) => {
                let stop = is_disconnected(&event);
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

fn assert_probe_timeout(events: &[ConnectionEvent]) {
    let idx = events
        .iter()
        .position(|e| matches!(e, ConnectionEvent::HeartbeatTimeout { .. }))
        .unwrap_or_else(|| panic!("no HeartbeatTimeout in {events:?}"));
    assert_eq!(
        events[idx],
        ConnectionEvent::HeartbeatTimeout { elapsed: IDLE + PROBE_TIMEOUT },
    );
    assert!(
        matches!(
            events.get(idx + 1),
            Some(ConnectionEvent::Disconnected { intent: DisconnectIntent::Network, .. })
        ),
        "{events:?}"
    );
}

fn no_disconnect(events: &[ConnectionEvent]) {
    assert!(!events.iter().any(is_disconnected), "{events:?}");
}

/// The pongs among `messages`.
fn pongs(messages: &[marketdata_core::WebSocketMessage]) -> Vec<String> {
    messages.iter().filter(|m| m.is_pong()).map(|m| format!("{:?}", m.data)).collect()
}

/// Everything `receiver` yields during `window`, split into events and
/// messages.
fn items_for(
    receiver: &marketdata_core::StreamReceiver,
    window: Duration,
) -> (Vec<ConnectionEvent>, Vec<marketdata_core::WebSocketMessage>) {
    let deadline = Instant::now() + window;
    let (mut events, mut messages) = (Vec::new(), Vec::new());
    while let Some(left) = deadline.checked_duration_since(Instant::now()) {
        match receiver.receive_timeout(left) {
            Ok(Some(marketdata_core::StreamItem::Event(e))) => events.push(e),
            Ok(Some(marketdata_core::StreamItem::Message(m))) => messages.push(m),
            Ok(_) => {}
            Err(_) => break,
        }
    }
    (events, messages)
}

fn messages(receiver: &marketdata_core::StreamReceiver) -> Vec<marketdata_core::WebSocketMessage> {
    common::drain_until_quiet(|d| match receiver.receive_timeout(d) {
        Ok(Some(item)) => Some(item),
        _ => None,
    })
    .into_iter()
    .filter_map(|item| match item {
        marketdata_core::StreamItem::Message(m) => Some(m),
        _ => None,
    })
    .collect()
}

mod aio {
    use super::*;
    use marketdata_core::aio::WebSocketClient;
    use marketdata_core::{MarketDataError, WebSocketRequest};

    async fn client(server: &MockWsServer, health: HealthCheckConfig) -> WebSocketClient {
        let client =
            WebSocketClient::with_full_config(config(server), ReconnectionConfig::disabled(), health);
        client.connect().await.expect("connect");
        client
    }

    #[tokio::test]
    async fn answered_probe_keeps_a_silent_connection_alive() {
        let server = MockWsServer::start().await;
        let client = client(&server, probe()).await;

        let receiver = client.stream_receiver();
        let (events, delivered) = tokio::task::spawn_blocking(move || {
            items_for(&receiver, Duration::from_millis(1_500))
        })
        .await
        .unwrap();

        no_disconnect(&events);
        assert!(probe_pings(&server) >= 2, "{:?}", server.pings_received());
        assert!(pongs(&delivered).is_empty(), "probe pong delivered: {delivered:?}");
        client.disconnect().await.ok();
    }

    #[tokio::test]
    async fn unanswered_probe_declares_the_connection_dead() {
        let server = MockWsServer::start().await;
        server.set_answer_pings(false);
        let started = Instant::now();
        let client = client(&server, probe()).await;

        let rx = common::EventReceiver::of_async(&client);
        let events = tokio::task::spawn_blocking(move || until_disconnected(&rx)).await.unwrap();

        assert_probe_timeout(&events);
        assert!(started.elapsed() >= IDLE + PROBE_TIMEOUT);
        assert_eq!(probe_pings(&server), 1, "one probe per silence");
    }

    #[tokio::test]
    async fn inbound_traffic_sends_no_probe() {
        let server = MockWsServer::start().await;
        let client = client(&server, probe()).await;

        for _ in 0..12 {
            server.inject_frame(StreamMessage::Pong { state: Some("traffic".into()) }).await;
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        assert_eq!(probe_pings(&server), 0);
        assert!(client.is_connected().await);
        client.disconnect().await.ok();
    }

    #[tokio::test]
    async fn passive_mode_sends_no_ping() {
        let server = MockWsServer::start().await;
        let client = client(&server, passive()).await;

        let rx = common::EventReceiver::of_async(&client);
        let events = tokio::task::spawn_blocking(move || until_disconnected(&rx)).await.unwrap();

        assert!(events.iter().any(is_disconnected), "{events:?}");
        assert!(server.pings_received().is_empty());
    }

    #[tokio::test]
    async fn measure_latency_returns_the_round_trip() {
        let server = MockWsServer::start().await;
        let client = client(&server, HealthCheckConfig::default()).await;

        let latency = client.measure_latency(None).await.expect("latency");

        assert!(latency < Duration::from_secs(1), "{latency:?}");
        let receiver = client.stream_receiver();
        let delivered = tokio::task::spawn_blocking(move || messages(&receiver)).await.unwrap();
        assert!(pongs(&delivered).is_empty(), "latency pong delivered: {delivered:?}");
        client.disconnect().await.ok();
    }

    #[tokio::test]
    async fn measure_latency_times_out() {
        let server = MockWsServer::start().await;
        server.set_answer_pings(false);
        let client = client(&server, HealthCheckConfig::default()).await;

        let result = client.measure_latency(Some(Duration::from_millis(200))).await;

        assert!(matches!(result, Err(MarketDataError::TimeoutError { .. })), "{result:?}");
        client.disconnect().await.ok();
    }

    #[tokio::test]
    async fn measure_latency_needs_a_connection() {
        let server = MockWsServer::start().await;
        let client =
            WebSocketClient::with_reconnection_config(config(&server), ReconnectionConfig::disabled());

        let result = client.measure_latency(None).await;

        assert!(matches!(result, Err(MarketDataError::ClientClosed)), "{result:?}");
    }

    #[tokio::test]
    async fn measure_latency_fails_when_the_connection_drops() {
        let server = MockWsServer::start().await;
        server.set_answer_pings(false);
        let client = client(&server, HealthCheckConfig::default()).await;

        let (result, ()) = tokio::join!(client.measure_latency(None), async {
            tokio::time::sleep(Duration::from_millis(200)).await;
            server.drop_transport().await;
        });

        assert!(matches!(result, Err(MarketDataError::ConnectionError { .. })), "{result:?}");
    }

    #[tokio::test]
    async fn caller_ping_pong_is_still_delivered() {
        let server = MockWsServer::start().await;
        let client = client(&server, HealthCheckConfig::default()).await;

        client.send(WebSocketRequest::ping(Some("mine".into()))).await.expect("ping");

        let receiver = client.stream_receiver();
        let delivered = tokio::task::spawn_blocking(move || messages(&receiver)).await.unwrap();
        assert_eq!(pongs(&delivered).len(), 1, "{delivered:?}");
        client.disconnect().await.ok();
    }
}

mod sync {
    use super::*;
    use marketdata_core::{MarketDataError, WebSocketClient, WebSocketRequest};

    async fn blocking<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
        tokio::task::spawn_blocking(f).await.expect("blocking task")
    }

    fn client(config: ConnectionConfig, health: HealthCheckConfig) -> WebSocketClient {
        let client = WebSocketClient::with_full_config(config, ReconnectionConfig::disabled(), health);
        client.connect().expect("connect");
        client
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn answered_probe_keeps_a_silent_connection_alive() {
        let server = MockWsServer::start().await;
        let config = config(&server);

        let (events, delivered) = blocking(move || {
            let client = client(config, probe());
            let items = items_for(&client.stream_receiver(), Duration::from_millis(1_500));
            client.disconnect().ok();
            items
        })
        .await;

        no_disconnect(&events);
        assert!(probe_pings(&server) >= 2, "{:?}", server.pings_received());
        assert!(pongs(&delivered).is_empty(), "probe pong delivered: {delivered:?}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn unanswered_probe_declares_the_connection_dead() {
        let server = MockWsServer::start().await;
        server.set_answer_pings(false);
        let config = config(&server);

        let events = blocking(move || {
            let client = client(config, probe());
            until_disconnected(&common::EventReceiver::of_sync(&client))
        })
        .await;

        assert_probe_timeout(&events);
        assert_eq!(probe_pings(&server), 1, "one probe per silence");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn inbound_traffic_sends_no_probe() {
        let server = MockWsServer::start().await;
        let config = config(&server);
        let client = blocking(move || client(config, probe())).await;

        for _ in 0..12 {
            server.inject_frame(StreamMessage::Pong { state: Some("traffic".into()) }).await;
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        assert_eq!(probe_pings(&server), 0);
        blocking(move || {
            assert!(client.is_connected());
            client.disconnect().ok();
        })
        .await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn passive_mode_sends_no_ping() {
        let server = MockWsServer::start().await;
        let config = config(&server);

        let events = blocking(move || {
            let client = client(config, passive());
            until_disconnected(&common::EventReceiver::of_sync(&client))
        })
        .await;

        assert!(events.iter().any(is_disconnected), "{events:?}");
        assert!(server.pings_received().is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn measure_latency_returns_the_round_trip() {
        let server = MockWsServer::start().await;
        let config = config(&server);

        let (latency, delivered) = blocking(move || {
            let client = client(config, HealthCheckConfig::default());
            let latency = client.measure_latency(None);
            let delivered = messages(&client.stream_receiver());
            client.disconnect().ok();
            (latency, delivered)
        })
        .await;

        let latency = latency.expect("latency");
        assert!(latency < Duration::from_secs(1), "{latency:?}");
        assert!(pongs(&delivered).is_empty(), "latency pong delivered: {delivered:?}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn measure_latency_times_out() {
        let server = MockWsServer::start().await;
        server.set_answer_pings(false);
        let config = config(&server);

        let result = blocking(move || {
            let client = client(config, HealthCheckConfig::default());
            let result = client.measure_latency(Some(Duration::from_millis(200)));
            client.disconnect().ok();
            result
        })
        .await;

        assert!(matches!(result, Err(MarketDataError::TimeoutError { .. })), "{result:?}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn measure_latency_needs_a_connection() {
        let server = MockWsServer::start().await;
        let config = config(&server);

        let result = blocking(move || {
            WebSocketClient::with_reconnection_config(config, ReconnectionConfig::disabled())
                .measure_latency(None)
        })
        .await;

        assert!(matches!(result, Err(MarketDataError::ClientClosed)), "{result:?}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn measure_latency_fails_when_the_connection_drops() {
        let server = MockWsServer::start().await;
        server.set_answer_pings(false);
        let config = config(&server);
        let client = std::sync::Arc::new(blocking(move || client(config, HealthCheckConfig::default())).await);

        let measuring = {
            let client = std::sync::Arc::clone(&client);
            tokio::task::spawn_blocking(move || client.measure_latency(None))
        };
        tokio::time::sleep(Duration::from_millis(200)).await;
        server.drop_transport().await;
        let result = measuring.await.expect("measure task");

        assert!(matches!(result, Err(MarketDataError::ConnectionError { .. })), "{result:?}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn caller_ping_pong_is_still_delivered() {
        let server = MockWsServer::start().await;
        let config = config(&server);

        let delivered = blocking(move || {
            let client = client(config, HealthCheckConfig::default());
            client.send(WebSocketRequest::ping(Some("mine".into()))).expect("ping");
            let delivered = messages(&client.stream_receiver());
            client.disconnect().ok();
            delivered
        })
        .await;

        assert_eq!(pongs(&delivered).len(), 1, "{delivered:?}");
    }
}
