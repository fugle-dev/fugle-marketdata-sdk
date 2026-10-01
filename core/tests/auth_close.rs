//! A Close frame received during authentication is reported with its code
//! and reason (#292).
//!
//! The server refuses a connection over its limit by answering the auth
//! frame with `Close(1001, "Maximum number of connections reached")`. The
//! error used to drop the frame and say only "Stream closed during
//! authentication". Each scenario runs against both the async and the sync
//! client:
//!
//! - `connect()` fails with `ConnectionError` (2001, retryable) whose message
//!   carries the close code and reason; a Close without a code leaves the
//!   message as it was;
//! - during auto-reconnect, the failed attempts' `Error` events carry the
//!   same message, and the attempts are still retried until the policy's
//!   limit (`ReconnectFailed { attempts: 2 }`).

#![cfg(feature = "tokio-comp")]

#[path = "common/mod.rs"]
mod common;

use common::AfterAuth;
use marketdata_core::error_code;
use marketdata_core::websocket::ConnectionEvent;
use marketdata_core::{AuthRequest, ConnectionConfig, MarketDataError, ReconnectionConfig};
use std::time::{Duration, Instant};

const REASON: &str = "Maximum number of connections reached";
const WITH_REASON: &str =
    "Stream closed during authentication (close 1001: Maximum number of connections reached)";
const WITHOUT_FRAME: &str = "Stream closed during authentication";
/// Upper bound on every wait; the auth timeout is the same, so a client that
/// ignores the Close fails with a timeout instead of hanging.
const WAIT: Duration = Duration::from_secs(5);

fn config(url: &str) -> ConnectionConfig {
    ConnectionConfig::builder(url, AuthRequest::with_api_key("mock-test-key"))
        .auth_timeout(WAIT)
        .build()
}

fn limit_close() -> AfterAuth {
    AfterAuth::CloseInsteadOfAuth {
        frame: Some((1001, REASON.to_string())),
    }
}

fn bare_close() -> AfterAuth {
    AfterAuth::CloseInsteadOfAuth { frame: None }
}

/// Two attempts after a short backoff.
fn two_attempts() -> ReconnectionConfig {
    ReconnectionConfig::new(2, Duration::from_millis(100), Duration::from_millis(100))
        .expect("valid reconnection config")
}

/// A drop that starts the reconnect, then two attempts refused at auth.
fn drop_then_limit_closes() -> Vec<AfterAuth> {
    vec![
        AfterAuth::ServerDropAfter { delay_ms: 50 },
        limit_close(),
        limit_close(),
    ]
}

fn assert_closed_during_auth(result: &Result<(), MarketDataError>, expected: &str) {
    let Err(err) = result else {
        panic!("connect succeeded")
    };
    assert!(
        matches!(err, MarketDataError::ConnectionError { msg } if msg == expected),
        "{err:?}"
    );
    assert_eq!(err.to_error_code(), error_code::CONNECTION);
    assert!(err.is_retryable(), "{err:?}");
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

/// Both attempts were refused with the limit Close, each reported as an
/// `Error` carrying its code and reason, and the second attempt was made:
/// the failure is still retried.
fn assert_reconnect_reports_close(events: &[ConnectionEvent]) {
    assert!(
        matches!(
            events.last(),
            Some(ConnectionEvent::ReconnectFailed { attempts: 2 })
        ),
        "{events:?}"
    );
    let reported = events
        .iter()
        .filter(|event| {
            matches!(
                event,
                ConnectionEvent::Error(info)
                    if info.code == error_code::CONNECTION && info.message.contains(WITH_REASON)
            )
        })
        .count();
    assert_eq!(reported, 2, "{events:?}");
}

mod aio {
    use super::*;
    use marketdata_core::aio::WebSocketClient;

    async fn connect_once(behaviour: AfterAuth) -> Result<(), MarketDataError> {
        let server = common::spawn(behaviour).await;
        let client = WebSocketClient::with_reconnection_config(
            config(&server.url),
            ReconnectionConfig::disabled(),
        );
        client.connect().await
    }

    #[tokio::test]
    async fn close_during_auth_reports_code_and_reason() {
        assert_closed_during_auth(&connect_once(limit_close()).await, WITH_REASON);
    }

    #[tokio::test]
    async fn close_without_code_during_auth_keeps_message() {
        assert_closed_during_auth(&connect_once(bare_close()).await, WITHOUT_FRAME);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn reconnect_reports_close_during_auth_and_retries() {
        let server = common::spawn_sequence(drop_then_limit_closes()).await;
        let client = WebSocketClient::with_reconnection_config(config(&server.url), two_attempts());
        client.connect().await.expect("connect");
        let rx = common::EventReceiver::of_async(&client);

        let events = tokio::task::spawn_blocking(move || recv_until_reconnect_failed(&rx))
            .await
            .expect("event reader");
        assert_reconnect_reports_close(&events);
    }
}

mod sync {
    use super::*;
    use marketdata_core::WebSocketClient;

    async fn connect_once(behaviour: AfterAuth) -> Result<(), MarketDataError> {
        let server = common::spawn(behaviour).await;
        let config = config(&server.url);
        tokio::task::spawn_blocking(move || {
            WebSocketClient::with_reconnection_config(config, ReconnectionConfig::disabled())
                .connect()
        })
        .await
        .expect("blocking task")
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn close_during_auth_reports_code_and_reason() {
        assert_closed_during_auth(&connect_once(limit_close()).await, WITH_REASON);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn close_without_code_during_auth_keeps_message() {
        assert_closed_during_auth(&connect_once(bare_close()).await, WITHOUT_FRAME);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn reconnect_reports_close_during_auth_and_retries() {
        let server = common::spawn_sequence(drop_then_limit_closes()).await;
        let config = config(&server.url);
        let events = tokio::task::spawn_blocking(move || {
            let client = WebSocketClient::with_reconnection_config(config, two_attempts());
            client.connect().expect("connect");
            recv_until_reconnect_failed(&common::EventReceiver::of_sync(&client))
        })
        .await
        .expect("blocking task");
        assert_reconnect_reports_close(&events);
    }
}
