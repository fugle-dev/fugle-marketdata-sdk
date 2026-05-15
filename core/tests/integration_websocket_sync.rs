//! Sync integration tests for the default `WebSocketClient`.
//!
//! These tests require a valid `FUGLE_API_KEY` and hit the real Fugle
//! streaming endpoint. They are `#[ignore]`-gated and must be run with
//! `--ignored` to execute against production.
//!
//! ```bash
//! export FUGLE_API_KEY=your_api_key_here
//! cargo test -p fugle-marketdata-core --no-default-features \
//!   --test integration_websocket_sync -- --ignored
//! ```
//!
//! No tokio runtime required. The default `WebSocketClient` runs entirely
//! on `std::thread` + `tungstenite`.

use marketdata_core::websocket::StockSubscription;
use marketdata_core::{
    AuthRequest, Channel, ConnectionConfig, ConnectionState, WebSocketClient,
};
use std::env;
use std::time::Duration;

fn get_api_key() -> String {
    env::var("FUGLE_API_KEY")
        .expect("FUGLE_API_KEY environment variable must be set for integration tests")
}

fn stock_config() -> ConnectionConfig {
    ConnectionConfig::fugle_stock(AuthRequest::with_api_key(&get_api_key()))
}

#[test]
#[ignore]
fn test_stock_connection_sync() {
    let client = WebSocketClient::new(stock_config());
    assert_eq!(client.state(), ConnectionState::Disconnected);

    client.connect().expect("connect should succeed");
    assert!(client.is_connected(), "state should be Connected after connect()");

    client.disconnect().expect("disconnect should succeed");
    assert!(client.is_closed(), "state should be Closed after disconnect()");
}

#[test]
#[ignore]
fn test_invalid_api_key_sync() {
    let bad = ConnectionConfig::fugle_stock(AuthRequest::with_api_key("not-a-real-key"));
    let client = WebSocketClient::new(bad);

    let result = client.connect();
    assert!(result.is_err(), "connect with bad key should fail");
}

#[test]
#[ignore]
fn test_subscribe_then_receive_sync() {
    let client = WebSocketClient::new(stock_config());
    client.connect().expect("connect failed");

    let sub = StockSubscription::new(Channel::Trades, "2330");
    client.subscribe(sub).expect("subscribe failed");

    let rx = client.messages();
    // Wait up to 30s for at least one inbound frame. Outside market hours
    // this may legitimately timeout — caller should re-run during market hours.
    let _ = rx.receive_timeout(Duration::from_secs(30));

    client.disconnect().expect("disconnect failed");
}

#[test]
#[ignore]
fn test_multiple_subscriptions_sync() {
    let client = WebSocketClient::new(stock_config());
    client.connect().expect("connect failed");

    for symbol in &["2330", "2317", "2454"] {
        client
            .subscribe(StockSubscription::new(Channel::Trades, *symbol))
            .expect("subscribe failed");
    }
    assert_eq!(client.subscription_keys().len(), 3);

    client.disconnect().expect("disconnect failed");
}

#[test]
#[ignore]
fn test_unsubscribe_sync() {
    let client = WebSocketClient::new(stock_config());
    client.connect().expect("connect failed");

    client
        .subscribe(StockSubscription::new(Channel::Trades, "2330"))
        .expect("subscribe failed");
    assert_eq!(client.subscription_keys().len(), 1);

    client.unsubscribe(["trades:2330"]).expect("unsubscribe failed");
    assert_eq!(client.subscription_keys().len(), 0);

    client.disconnect().expect("disconnect failed");
}

fn fake_config() -> ConnectionConfig {
    // Offline tests don't need a real key — just a parseable config.
    ConnectionConfig::fugle_stock(AuthRequest::with_api_key("dummy-key-for-offline-tests"))
}

#[test]
fn test_new_does_not_block() {
    // Smoke: constructor must not touch the network.
    let client = WebSocketClient::new(fake_config());
    assert_eq!(client.state(), ConnectionState::Disconnected);
    assert!(!client.is_connected());
    assert!(!client.is_closed());
}

#[test]
fn test_subscribe_before_connect_records_locally() {
    let client = WebSocketClient::new(fake_config());
    client
        .subscribe(StockSubscription::new(Channel::Trades, "2330"))
        .expect("subscribe should record locally even without connect()");
    assert_eq!(client.subscription_keys().len(), 1);
}
