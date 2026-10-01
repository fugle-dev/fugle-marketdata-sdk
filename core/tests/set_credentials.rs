//! `set_credentials()` changes what later connection attempts send (#322).
//!
//! The server takes only credential B. Each scenario runs against both the
//! async and the sync client:
//!
//! - a client connected with A, given B, sends B in the auth frame of the
//!   automatic reconnect after a drop, which then succeeds; the kind may
//!   change on the way (API key to SDK token);
//! - a credential set while the reconnect waits out its backoff is the one
//!   the attempt sends;
//! - on a live connection nothing is sent; a `reconnect()` sends B;
//! - a first `connect()` rejected for A succeeds again once B is set;
//! - a client whose reconnect was rejected is closed: a new client handed
//!   the old one's credentials handle connects with the credential set on it;
//! - a blank credential is refused with `ConfigError` (1004) and the held one
//!   kept.

#![cfg(feature = "tokio-comp")]

#[path = "common/mod.rs"]
mod common;

use common::AfterAuth;
use marketdata_core::error_code;
use marketdata_core::websocket::ConnectionEvent;
use marketdata_core::{Auth, AuthRequest, ConnectionConfig, MarketDataError, ReconnectionConfig};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

const A: &str = "token-a";
const B: &str = "token-b";
/// Upper bound on every wait.
const WAIT: Duration = Duration::from_secs(5);

fn config(url: &str) -> ConnectionConfig {
    ConnectionConfig::builder(url, AuthRequest::with_api_key(A))
        .auth_timeout(WAIT)
        .build()
}

/// Reconnect quickly, twice at most.
fn quick_reconnect() -> ReconnectionConfig {
    ReconnectionConfig::new(2, Duration::from_millis(100), Duration::from_millis(100))
        .expect("valid reconnection config")
}

/// One attempt, after a backoff long enough to set a credential in.
fn slow_reconnect() -> ReconnectionConfig {
    ReconnectionConfig::new(1, Duration::from_millis(500), Duration::from_millis(500))
        .expect("valid reconnection config")
}

/// Accept only `credential`, then go on with `then`.
fn require(
    credential: &str,
    auth_frames: &mpsc::UnboundedSender<String>,
    then: AfterAuth,
) -> AfterAuth {
    AfterAuth::RequireCredential {
        credential: credential.to_string(),
        auth_frames: auth_frames.clone(),
        then: Box::new(then),
    }
}

/// The credential fields of the next auth frame the server received.
async fn next_auth(rx: &mut mpsc::UnboundedReceiver<String>) -> serde_json::Value {
    let frame = tokio::time::timeout(WAIT, rx.recv())
        .await
        .expect("an auth frame arrived")
        .expect("server running");
    let frame: serde_json::Value = serde_json::from_str(&frame).expect("auth frame is JSON");
    assert_eq!(frame["event"], "auth", "{frame}");
    frame["data"].clone()
}

/// Receive events until one matches `done` or [`WAIT`] elapses.
fn recv_until(
    rx: &common::EventReceiver,
    done: impl Fn(&ConnectionEvent) -> bool,
) -> Vec<ConnectionEvent> {
    let deadline = Instant::now() + WAIT;
    let mut events = Vec::new();
    while let Some(left) = deadline.checked_duration_since(Instant::now()) {
        match rx.recv_timeout(left) {
            Ok(event) => {
                let finished = done(&event);
                events.push(event);
                if finished {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    events
}

fn authenticated_count(events: &[ConnectionEvent]) -> usize {
    events.iter().filter(|e| matches!(e, ConnectionEvent::Authenticated { .. })).count()
}

/// Events up to the reconnect's `Authenticated`, the second one.
fn until_reauthenticated(rx: &common::EventReceiver) -> Vec<ConnectionEvent> {
    let seen = std::cell::Cell::new(0);
    recv_until(rx, |e| {
        if matches!(e, ConnectionEvent::Authenticated { .. }) {
            seen.set(seen.get() + 1);
        }
        seen.get() == 2
    })
}

fn assert_blank_refused(result: Result<(), MarketDataError>) {
    let err = result.expect_err("a blank credential is refused");
    assert_eq!(err.to_error_code(), error_code::CONFIG, "{err:?}");
}

mod aio {
    use super::*;
    use marketdata_core::aio::WebSocketClient;
    use std::sync::Arc;

    #[tokio::test(flavor = "multi_thread")]
    async fn automatic_reconnect_sends_the_new_credential_of_another_kind() {
        let (tx, mut auth) = mpsc::unbounded_channel();
        let server = common::spawn_sequence(vec![
            require(A, &tx, AfterAuth::ServerDropAfter { delay_ms: 300 }),
            require(B, &tx, AfterAuth::Idle),
        ])
        .await;
        let client = WebSocketClient::with_reconnection_config(config(&server.url), quick_reconnect());
        client.connect().await.expect("connect with A");
        assert_eq!(next_auth(&mut auth).await["apikey"], A);

        client.set_credentials(Auth::SdkToken(B.into())).expect("set B");

        let rx = common::EventReceiver::of_async(&client);
        let events = tokio::task::spawn_blocking(move || until_reauthenticated(&rx))
            .await
            .expect("event reader");
        assert_eq!(authenticated_count(&events), 2, "{events:?}");
        let data = next_auth(&mut auth).await;
        assert_eq!(data["sdkToken"], B, "{data}");
        assert!(data.get("apikey").is_none(), "{data}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_credential_set_during_the_backoff_is_sent() {
        let (tx, mut auth) = mpsc::unbounded_channel();
        let server = common::spawn_sequence(vec![
            require(A, &tx, AfterAuth::ServerDropAfter { delay_ms: 50 }),
            require(B, &tx, AfterAuth::Idle),
        ])
        .await;
        let client = Arc::new(WebSocketClient::with_reconnection_config(
            config(&server.url),
            slow_reconnect(),
        ));
        client.connect().await.expect("connect with A");
        let rx = common::EventReceiver::of_async(&client);
        let events = tokio::task::spawn_blocking(move || {
            recv_until(&rx, |e| matches!(e, ConnectionEvent::Reconnecting { .. }))
        })
        .await
        .expect("event reader");
        assert!(matches!(events.last(), Some(ConnectionEvent::Reconnecting { .. })), "{events:?}");

        client.set_credentials(Auth::BearerToken(B.into())).expect("set B");

        let rx = common::EventReceiver::of_async(&client);
        let events = tokio::task::spawn_blocking(move || {
            recv_until(&rx, |e| matches!(e, ConnectionEvent::Authenticated { .. }))
        })
        .await
        .expect("event reader");
        assert_eq!(authenticated_count(&events), 1, "{events:?}");
        next_auth(&mut auth).await;
        assert_eq!(next_auth(&mut auth).await["token"], B);
    }

    #[tokio::test]
    async fn live_connection_is_left_alone_and_reconnect_sends_the_new_credential() {
        let (tx, mut auth) = mpsc::unbounded_channel();
        let (frames_tx, mut frames) = mpsc::unbounded_channel();
        let server = common::spawn_sequence(vec![
            require(A, &tx, AfterAuth::RecordFrames { frames: frames_tx }),
            require(B, &tx, AfterAuth::Idle),
        ])
        .await;
        let client = WebSocketClient::with_reconnection_config(
            config(&server.url),
            ReconnectionConfig::disabled(),
        );
        client.connect().await.expect("connect with A");
        next_auth(&mut auth).await;

        client.set_credentials(Auth::BearerToken(B.into())).expect("set B");
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert!(frames.try_recv().is_err(), "a frame was sent on the live connection");

        client.reconnect().await.expect("reconnect with B");
        assert_eq!(next_auth(&mut auth).await["token"], B);
    }

    #[tokio::test]
    async fn rejected_connect_succeeds_after_the_credential_is_set() {
        let (tx, mut auth) = mpsc::unbounded_channel();
        let server = common::spawn_sequence(vec![
            require(B, &tx, AfterAuth::Idle),
            require(B, &tx, AfterAuth::Idle),
        ])
        .await;
        let client = WebSocketClient::new(config(&server.url));
        let rejected = client.connect().await;
        assert!(matches!(rejected, Err(MarketDataError::AuthError { .. })), "{rejected:?}");
        assert_eq!(next_auth(&mut auth).await["apikey"], A);

        client.set_credentials(Auth::ApiKey(B.into())).expect("set B");
        client.connect().await.expect("connect with B");
        assert_eq!(next_auth(&mut auth).await["apikey"], B);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_new_client_takes_the_handle_of_one_whose_reconnect_was_rejected() {
        let (tx, mut auth) = mpsc::unbounded_channel();
        let server = common::spawn_sequence(vec![
            require(A, &tx, AfterAuth::ServerDropAfter { delay_ms: 50 }),
            require(B, &tx, AfterAuth::Idle),
            require(B, &tx, AfterAuth::Idle),
        ])
        .await;
        let client = WebSocketClient::with_reconnection_config(config(&server.url), quick_reconnect());
        client.connect().await.expect("connect with A");
        let rx = common::EventReceiver::of_async(&client);
        let events = tokio::task::spawn_blocking(move || {
            recv_until(&rx, |e| matches!(e, ConnectionEvent::ReconnectFailed { .. }))
        })
        .await
        .expect("event reader");
        assert!(
            matches!(events.last(), Some(ConnectionEvent::ReconnectFailed { attempts: 1, .. })),
            "rejection ends the reconnect: {events:?}"
        );
        assert!(client.is_closed().await);

        let handle = client.credentials_handle();
        handle.set(Auth::SdkToken(B.into())).expect("set B");
        let next = WebSocketClient::new(config(&server.url));
        next.use_credentials_handle(&handle);
        next.connect().await.expect("connect with B");

        next_auth(&mut auth).await;
        next_auth(&mut auth).await;
        assert_eq!(next_auth(&mut auth).await["sdkToken"], B);
    }

    #[tokio::test]
    async fn a_blank_credential_is_refused_and_the_held_one_kept() {
        let (tx, mut auth) = mpsc::unbounded_channel();
        let server = common::spawn(require(A, &tx, AfterAuth::Idle)).await;
        let client = WebSocketClient::new(config(&server.url));

        assert_blank_refused(client.set_credentials(Auth::SdkToken("   ".into())));

        client.connect().await.expect("connect with A");
        assert_eq!(next_auth(&mut auth).await["apikey"], A);
    }
}

mod sync {
    use super::*;
    use marketdata_core::WebSocketClient;
    use std::sync::Arc;

    async fn blocking<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
        tokio::task::spawn_blocking(f).await.expect("blocking task")
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn automatic_reconnect_sends_the_new_credential_of_another_kind() {
        let (tx, mut auth) = mpsc::unbounded_channel();
        let server = common::spawn_sequence(vec![
            require(A, &tx, AfterAuth::ServerDropAfter { delay_ms: 300 }),
            require(B, &tx, AfterAuth::Idle),
        ])
        .await;
        let client = Arc::new(WebSocketClient::with_reconnection_config(
            config(&server.url),
            quick_reconnect(),
        ));
        let c = Arc::clone(&client);
        blocking(move || c.connect()).await.expect("connect with A");
        assert_eq!(next_auth(&mut auth).await["apikey"], A);

        client.set_credentials(Auth::SdkToken(B.into())).expect("set B");

        let rx = common::EventReceiver::of_sync(&client);
        let events = blocking(move || until_reauthenticated(&rx)).await;
        assert_eq!(authenticated_count(&events), 2, "{events:?}");
        let data = next_auth(&mut auth).await;
        assert_eq!(data["sdkToken"], B, "{data}");
        assert!(data.get("apikey").is_none(), "{data}");
        blocking(move || client.disconnect()).await.expect("disconnect");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_credential_set_during_the_backoff_is_sent() {
        let (tx, mut auth) = mpsc::unbounded_channel();
        let server = common::spawn_sequence(vec![
            require(A, &tx, AfterAuth::ServerDropAfter { delay_ms: 50 }),
            require(B, &tx, AfterAuth::Idle),
        ])
        .await;
        let client = Arc::new(WebSocketClient::with_reconnection_config(
            config(&server.url),
            slow_reconnect(),
        ));
        let c = Arc::clone(&client);
        blocking(move || c.connect()).await.expect("connect with A");
        let rx = common::EventReceiver::of_sync(&client);
        let events = tokio::task::spawn_blocking(move || {
            recv_until(&rx, |e| matches!(e, ConnectionEvent::Reconnecting { .. }))
        })
        .await
        .expect("event reader");
        assert!(matches!(events.last(), Some(ConnectionEvent::Reconnecting { .. })), "{events:?}");

        client.set_credentials(Auth::BearerToken(B.into())).expect("set B");

        let rx = common::EventReceiver::of_sync(&client);
        let events = tokio::task::spawn_blocking(move || {
            recv_until(&rx, |e| matches!(e, ConnectionEvent::Authenticated { .. }))
        })
        .await
        .expect("event reader");
        assert_eq!(authenticated_count(&events), 1, "{events:?}");
        next_auth(&mut auth).await;
        assert_eq!(next_auth(&mut auth).await["token"], B);
        blocking(move || client.disconnect()).await.expect("disconnect");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn live_connection_is_left_alone_and_reconnect_sends_the_new_credential() {
        let (tx, mut auth) = mpsc::unbounded_channel();
        let (frames_tx, mut frames) = mpsc::unbounded_channel();
        let server = common::spawn_sequence(vec![
            require(A, &tx, AfterAuth::RecordFrames { frames: frames_tx }),
            require(B, &tx, AfterAuth::Idle),
        ])
        .await;
        let client = Arc::new(WebSocketClient::with_reconnection_config(
            config(&server.url),
            ReconnectionConfig::disabled(),
        ));
        let c = Arc::clone(&client);
        blocking(move || c.connect()).await.expect("connect with A");
        next_auth(&mut auth).await;

        client.set_credentials(Auth::BearerToken(B.into())).expect("set B");
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert!(frames.try_recv().is_err(), "a frame was sent on the live connection");

        let c = Arc::clone(&client);
        blocking(move || c.reconnect()).await.expect("reconnect with B");
        assert_eq!(next_auth(&mut auth).await["token"], B);
        blocking(move || client.disconnect()).await.expect("disconnect");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn rejected_connect_succeeds_after_the_credential_is_set() {
        let (tx, mut auth) = mpsc::unbounded_channel();
        let server = common::spawn_sequence(vec![
            require(B, &tx, AfterAuth::Idle),
            require(B, &tx, AfterAuth::Idle),
        ])
        .await;
        let client = Arc::new(WebSocketClient::new(config(&server.url)));
        let c = Arc::clone(&client);
        let rejected = blocking(move || c.connect()).await;
        assert!(matches!(rejected, Err(MarketDataError::AuthError { .. })), "{rejected:?}");
        assert_eq!(next_auth(&mut auth).await["apikey"], A);

        client.set_credentials(Auth::ApiKey(B.into())).expect("set B");
        let c = Arc::clone(&client);
        blocking(move || c.connect()).await.expect("connect with B");
        assert_eq!(next_auth(&mut auth).await["apikey"], B);
        blocking(move || client.disconnect()).await.expect("disconnect");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_new_client_takes_the_handle_of_one_whose_reconnect_was_rejected() {
        let (tx, mut auth) = mpsc::unbounded_channel();
        let server = common::spawn_sequence(vec![
            require(A, &tx, AfterAuth::ServerDropAfter { delay_ms: 50 }),
            require(B, &tx, AfterAuth::Idle),
            require(B, &tx, AfterAuth::Idle),
        ])
        .await;
        let client = Arc::new(WebSocketClient::with_reconnection_config(
            config(&server.url),
            quick_reconnect(),
        ));
        let c = Arc::clone(&client);
        blocking(move || c.connect()).await.expect("connect with A");
        let rx = common::EventReceiver::of_sync(&client);
        let events = blocking(move || {
            recv_until(&rx, |e| matches!(e, ConnectionEvent::ReconnectFailed { .. }))
        })
        .await;
        assert!(
            matches!(events.last(), Some(ConnectionEvent::ReconnectFailed { attempts: 1, .. })),
            "rejection ends the reconnect: {events:?}"
        );
        assert!(client.is_closed());

        let handle = client.credentials_handle();
        handle.set(Auth::SdkToken(B.into())).expect("set B");
        let next = Arc::new(WebSocketClient::new(config(&server.url)));
        next.use_credentials_handle(&handle);
        let n = Arc::clone(&next);
        blocking(move || n.connect()).await.expect("connect with B");

        next_auth(&mut auth).await;
        next_auth(&mut auth).await;
        assert_eq!(next_auth(&mut auth).await["sdkToken"], B);
        blocking(move || next.disconnect()).await.expect("disconnect");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_blank_credential_is_refused_and_the_held_one_kept() {
        let (tx, mut auth) = mpsc::unbounded_channel();
        let server = common::spawn(require(A, &tx, AfterAuth::Idle)).await;
        let client = Arc::new(WebSocketClient::new(config(&server.url)));

        assert_blank_refused(client.set_credentials(Auth::SdkToken("   ".into())));

        let c = Arc::clone(&client);
        blocking(move || c.connect()).await.expect("connect with A");
        assert_eq!(next_auth(&mut auth).await["apikey"], A);
        blocking(move || client.disconnect()).await.expect("disconnect");
    }
}
