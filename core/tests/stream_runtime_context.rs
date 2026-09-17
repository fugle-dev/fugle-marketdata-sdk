//! `WebSocketClient::stream_receiver()` must not require an ambient tokio
//! runtime.
//!
//! FFI bindings take the receiver from plain threads, before or after
//! `connect()`. It reads the client's stream directly, so no task has to
//! run on any runtime to deliver messages (#26, #46, #68).

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

    let rx = common::MessageReceiver(client.stream_receiver());
    rt.block_on(client.connect()).expect("connect");

    let msg = rx
        .receive_timeout(Duration::from_secs(2))
        .expect("channel open");
    assert!(
        msg.is_some(),
        "messages must arrive when the receiver was taken before connect()"
    );

    let _ = rt.block_on(client.shutdown_with_timeout(Duration::from_millis(200)));
}

#[test]
fn messages_after_connect_outside_runtime() {
    let rt = Runtime::new().expect("runtime");
    let server = rt.block_on(common::spawn(common::AfterAuth::FloodData { count: 3 }));
    let client = client_for(server.url.clone());

    rt.block_on(client.connect()).expect("connect");
    let rx = common::MessageReceiver(client.stream_receiver());

    let msg = rx
        .receive_timeout(Duration::from_secs(2))
        .expect("channel open");
    assert!(
        msg.is_some(),
        "messages must arrive when the receiver was taken after connect()"
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

    let rx = client.stream_receiver();
    // `Connecting` and the connect `Error` are already queued.
    while rx.try_receive().is_some() {}

    // The stream belongs to the client, not to a runtime: it stays open...
    assert!(matches!(
        rx.receive_timeout(Duration::from_millis(50)),
        Ok(None)
    ));
    // ...until the client is dropped, and the receiver does not hang then.
    drop(client);
    assert!(
        rx.receive_timeout(Duration::from_millis(500)).is_err(),
        "receiver must observe the closed stream"
    );
}
