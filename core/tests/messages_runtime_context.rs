//! `WebSocketClient::messages()` must not require an ambient tokio runtime.
//!
//! FFI bindings call `messages()` from plain threads (py: before
//! `connect()`, js / uniffi: after). The receiver reads the client's
//! message queue directly, so no task has to run on any runtime to
//! deliver messages (#26, #46).

#![cfg(feature = "tokio-comp")]

#[path = "common/mod.rs"]
mod common;

use marketdata_core::aio::WebSocketClient;
use marketdata_core::{AuthRequest, ConnectionConfig, ReconnectionConfig};
use std::time::Duration;
use tokio::runtime::Runtime;

fn client_for(url: String) -> WebSocketClient {
    let config = ConnectionConfig::builder(url, AuthRequest::with_api_key("test-key"))
        .connect_timeout(Duration::from_secs(2))
        .build();
    WebSocketClient::with_reconnection_config(config, ReconnectionConfig::disabled())
}

#[test]
fn messages_before_connect_outside_runtime() {
    let rt = Runtime::new().expect("runtime");
    let server = rt.block_on(common::spawn(common::AfterAuth::FloodData { count: 3 }));
    let client = client_for(server.url.clone());

    let rx = client.messages();
    rt.block_on(client.connect()).expect("connect");

    let msg = rx
        .receive_timeout(Duration::from_secs(2))
        .expect("channel open");
    assert!(
        msg.is_some(),
        "bridge attached at connect() must forward messages"
    );

    let _ = rt.block_on(client.shutdown_with_timeout(Duration::from_millis(200)));
}

#[test]
fn messages_after_connect_outside_runtime() {
    let rt = Runtime::new().expect("runtime");
    let server = rt.block_on(common::spawn(common::AfterAuth::FloodData { count: 3 }));
    let client = client_for(server.url.clone());

    rt.block_on(client.connect()).expect("connect");
    let rx = client.messages();

    let msg = rx
        .receive_timeout(Duration::from_secs(2))
        .expect("channel open");
    assert!(
        msg.is_some(),
        "bridge spawned on the bound runtime must forward messages"
    );

    let _ = rt.block_on(client.shutdown_with_timeout(Duration::from_millis(200)));
}

#[test]
fn receiver_outlives_the_connect_runtime_and_closes_with_the_client() {
    let rt = Runtime::new().expect("runtime");
    // Nothing listens on port 1: connect() fails on this runtime.
    let client = client_for("ws://127.0.0.1:1/".to_string());
    assert!(rt.block_on(client.connect()).is_err());
    drop(rt);

    let rx = client.messages();

    // The queue belongs to the client, not to a runtime: it stays open...
    assert!(matches!(
        rx.receive_timeout(Duration::from_millis(50)),
        Ok(None)
    ));
    // ...until the client is dropped, and the receiver does not hang then.
    drop(client);
    assert!(
        rx.receive_timeout(Duration::from_millis(500)).is_err(),
        "receiver must observe the closed queue"
    );
}
