//! A Close frame received during authentication is reported with its code
//! and reason (#292).
//!
//! The server refuses a connection over its limit by answering the auth
//! frame with `Close(1001, "Maximum number of connections reached")`. The
//! error used to drop the frame and say only "Stream closed during
//! authentication". Each scenario runs against both the async and the sync
//! client:
//!
//! - `connect()` fails with an error whose message carries the close code
//!   and reason; a Close without a code leaves the message as it was. The
//!   connection limit — that Close, `Close(1013)`, or the auth answered with
//!   `error{1003}` — is `ConnectionLimit` (2012, #300); any other close is
//!   `ConnectionError` (2001); both are retryable;
//! - the client answers the server's Close with its own before giving the
//!   connection up, so the server sees the close completed, not `1006`; it
//!   closes a connection whose credentials were rejected the same way;
//! - during auto-reconnect, every failed attempt's `Error` event carries the
//!   same code and message, and the attempts are still retried until the
//!   policy's limit (`ReconnectFailed { attempts: 2 }`).

#![cfg(feature = "tokio-comp")]

#[path = "common/mod.rs"]
mod common;

use common::AfterAuth;
use marketdata_core::error_code;
use marketdata_core::websocket::ConnectionEvent;
use marketdata_core::{AuthRequest, ConnectionConfig, MarketDataError, ReconnectionConfig};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

const REASON: &str = "Maximum number of connections reached";
const WITH_REASON: &str =
    "Stream closed during authentication (close 1001: Maximum number of connections reached)";
const WITHOUT_FRAME: &str = "Stream closed during authentication";
const RESTART: &str = "Server restarting";
const WITH_RESTART: &str = "Stream closed during authentication (close 1001: Server restarting)";
const WITH_1013: &str = "Stream closed during authentication (close 1013)";
const WITH_1003: &str =
    "Authentication failed (server error 1003): Maximum number of connections reached";
/// Upper bound on every wait; the auth timeout is the same, so a client that
/// ignores the Close fails with a timeout instead of hanging.
const WAIT: Duration = Duration::from_secs(5);

fn config(url: &str) -> ConnectionConfig {
    ConnectionConfig::builder(url, AuthRequest::with_api_key("mock-test-key"))
        .auth_timeout(WAIT)
        .build()
}

/// The limit Close, reporting on `ended_by_close` whether the client replied.
fn limit_close(ended_by_close: Option<mpsc::UnboundedSender<bool>>) -> AfterAuth {
    AfterAuth::CloseInsteadOfAuth {
        frame: Some((1001, REASON.to_string())),
        ended_by_close,
    }
}

/// The limit Close of a server with fugle-realtime !640 (`1013`, no reason).
fn limit_close_1013(ended_by_close: Option<mpsc::UnboundedSender<bool>>) -> AfterAuth {
    AfterAuth::CloseInsteadOfAuth {
        frame: Some((1013, String::new())),
        ended_by_close,
    }
}

/// `1001` for a restart: same code as the old limit Close, another reason.
fn restart_close(ended_by_close: Option<mpsc::UnboundedSender<bool>>) -> AfterAuth {
    AfterAuth::CloseInsteadOfAuth {
        frame: Some((1001, RESTART.to_string())),
        ended_by_close,
    }
}

/// The limit `error{1003}` answering the auth frame, then a Close.
fn limit_error(ended_by_close: Option<mpsc::UnboundedSender<bool>>) -> AfterAuth {
    AfterAuth::RejectAuth {
        code: 1003,
        message: REASON.to_string(),
        close: true,
        ended_by_close,
    }
}

/// A Close without a code, reporting on `ended_by_close` whether the client
/// replied.
fn bare_close(ended_by_close: Option<mpsc::UnboundedSender<bool>>) -> AfterAuth {
    AfterAuth::CloseInsteadOfAuth {
        frame: None,
        ended_by_close,
    }
}

/// The server's rejection of the credentials (`error` 1000) and no Close,
/// reporting on `ended_by_close` whether the client sent one.
fn rejected(ended_by_close: Option<mpsc::UnboundedSender<bool>>) -> AfterAuth {
    AfterAuth::RejectAuth {
        code: 1000,
        message: "Invalid authentication credentials".to_string(),
        close: false,
        ended_by_close,
    }
}

fn assert_rejected(result: &Result<(), MarketDataError>) {
    assert!(matches!(result, Err(MarketDataError::AuthError { .. })), "{result:?}");
}

/// Serve one connection with `behaviour` and run `connect` against its URL.
/// Returns the result and whether the client answered the server's Close
/// with its own (`false` if it dropped the socket without one).
async fn connect_once<F>(
    behaviour: fn(Option<mpsc::UnboundedSender<bool>>) -> AfterAuth,
    connect: impl FnOnce(String) -> F,
) -> (Result<(), MarketDataError>, bool)
where
    F: std::future::Future<Output = Result<(), MarketDataError>>,
{
    let (tx, mut rx) = mpsc::unbounded_channel();
    let server = common::spawn(behaviour(Some(tx))).await;
    let result = connect(server.url.clone()).await;
    let by_close = tokio::time::timeout(WAIT, rx.recv())
        .await
        .expect("server saw the connection end")
        .expect("server reported");
    (result, by_close)
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
        limit_close(None),
        limit_close(None),
    ]
}

/// The same with `error{1003}`, the refusal of a server with
/// fugle-realtime !640.
fn drop_then_limit_errors() -> Vec<AfterAuth> {
    vec![
        AfterAuth::ServerDropAfter { delay_ms: 50 },
        limit_error(None),
        limit_error(None),
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

fn assert_connection_limit(result: &Result<(), MarketDataError>, expected: &str) {
    let Err(err) = result else {
        panic!("connect succeeded")
    };
    assert!(
        matches!(err, MarketDataError::ConnectionLimit { msg } if msg == expected),
        "{err:?}"
    );
    assert_eq!(err.to_error_code(), error_code::CONNECTION_LIMIT);
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

/// Both attempts were refused at the limit, each reported as an `Error` with
/// code 2012 whose message contains `expected`, and the second attempt was
/// made: the failure is still retried.
fn assert_reconnect_reports_limit(events: &[ConnectionEvent], expected: &str) {
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
                    if info.code == error_code::CONNECTION_LIMIT && info.message.contains(expected)
            )
        })
        .count();
    assert_eq!(reported, 2, "{events:?}");
}

mod aio {
    use super::*;
    use marketdata_core::aio::WebSocketClient;

    async fn connect(url: String) -> Result<(), MarketDataError> {
        let client =
            WebSocketClient::with_reconnection_config(config(&url), ReconnectionConfig::disabled());
        client.connect().await
    }

    #[tokio::test]
    async fn close_during_auth_reports_code_and_reason() {
        let (result, by_close) = connect_once(limit_close, connect).await;
        assert_connection_limit(&result, WITH_REASON);
        assert!(by_close, "the client dropped the socket without a reply Close");
    }

    #[tokio::test]
    async fn close_1013_during_auth_is_the_connection_limit() {
        let (result, by_close) = connect_once(limit_close_1013, connect).await;
        assert_connection_limit(&result, WITH_1013);
        assert!(by_close, "the client dropped the socket without a reply Close");
    }

    #[tokio::test]
    async fn error_1003_during_auth_is_the_connection_limit() {
        let (result, by_close) = connect_once(limit_error, connect).await;
        assert_connection_limit(&result, WITH_1003);
        assert!(by_close, "the client dropped the socket without a Close");
    }

    #[tokio::test]
    async fn close_1001_with_another_reason_is_a_connection_error() {
        let (result, by_close) = connect_once(restart_close, connect).await;
        assert_closed_during_auth(&result, WITH_RESTART);
        assert!(by_close, "the client dropped the socket without a reply Close");
    }

    #[tokio::test]
    async fn close_without_code_during_auth_keeps_message() {
        let (result, by_close) = connect_once(bare_close, connect).await;
        assert_closed_during_auth(&result, WITHOUT_FRAME);
        assert!(by_close, "the client dropped the socket without a reply Close");
    }

    #[tokio::test]
    async fn rejected_auth_closes_the_connection() {
        let (result, by_close) = connect_once(rejected, connect).await;
        assert_rejected(&result);
        assert!(by_close, "the client dropped the socket without a Close");
    }

    async fn reconnect_events(sequence: Vec<AfterAuth>) -> Vec<ConnectionEvent> {
        let server = common::spawn_sequence(sequence).await;
        let client = WebSocketClient::with_reconnection_config(config(&server.url), two_attempts());
        client.connect().await.expect("connect");
        let rx = common::EventReceiver::of_async(&client);

        tokio::task::spawn_blocking(move || recv_until_reconnect_failed(&rx))
            .await
            .expect("event reader")
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn reconnect_reports_close_during_auth_and_retries() {
        let events = reconnect_events(drop_then_limit_closes()).await;
        assert_reconnect_reports_limit(&events, WITH_REASON);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn reconnect_reports_error_1003_and_retries() {
        let events = reconnect_events(drop_then_limit_errors()).await;
        assert_reconnect_reports_limit(&events, WITH_1003);
    }
}

mod sync {
    use super::*;
    use marketdata_core::WebSocketClient;

    async fn connect(url: String) -> Result<(), MarketDataError> {
        tokio::task::spawn_blocking(move || {
            WebSocketClient::with_reconnection_config(config(&url), ReconnectionConfig::disabled())
                .connect()
        })
        .await
        .expect("blocking task")
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn close_during_auth_reports_code_and_reason() {
        let (result, by_close) = connect_once(limit_close, connect).await;
        assert_connection_limit(&result, WITH_REASON);
        assert!(by_close, "the client dropped the socket without a reply Close");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn close_1013_during_auth_is_the_connection_limit() {
        let (result, by_close) = connect_once(limit_close_1013, connect).await;
        assert_connection_limit(&result, WITH_1013);
        assert!(by_close, "the client dropped the socket without a reply Close");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn error_1003_during_auth_is_the_connection_limit() {
        let (result, by_close) = connect_once(limit_error, connect).await;
        assert_connection_limit(&result, WITH_1003);
        assert!(by_close, "the client dropped the socket without a Close");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn close_1001_with_another_reason_is_a_connection_error() {
        let (result, by_close) = connect_once(restart_close, connect).await;
        assert_closed_during_auth(&result, WITH_RESTART);
        assert!(by_close, "the client dropped the socket without a reply Close");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn close_without_code_during_auth_keeps_message() {
        let (result, by_close) = connect_once(bare_close, connect).await;
        assert_closed_during_auth(&result, WITHOUT_FRAME);
        assert!(by_close, "the client dropped the socket without a reply Close");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn rejected_auth_closes_the_connection() {
        let (result, by_close) = connect_once(rejected, connect).await;
        assert_rejected(&result);
        assert!(by_close, "the client dropped the socket without a Close");
    }

    async fn reconnect_events(sequence: Vec<AfterAuth>) -> Vec<ConnectionEvent> {
        let server = common::spawn_sequence(sequence).await;
        let config = config(&server.url);
        tokio::task::spawn_blocking(move || {
            let client = WebSocketClient::with_reconnection_config(config, two_attempts());
            client.connect().expect("connect");
            recv_until_reconnect_failed(&common::EventReceiver::of_sync(&client))
        })
        .await
        .expect("blocking task")
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn reconnect_reports_close_during_auth_and_retries() {
        let events = reconnect_events(drop_then_limit_closes()).await;
        assert_reconnect_reports_limit(&events, WITH_REASON);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn reconnect_reports_error_1003_and_retries() {
        let events = reconnect_events(drop_then_limit_errors()).await;
        assert_reconnect_reports_limit(&events, WITH_1003);
    }
}
