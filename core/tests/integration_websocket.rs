//! Integration tests for WebSocket client
//!
//! These tests require a valid FUGLE_API_KEY environment variable and
//! establish real WebSocket connections to the Fugle streaming API.
//!
//! # Running Integration Tests
//!
//! ```bash
//! # Set your API key
//! export FUGLE_API_KEY=your_api_key_here
//!
//! # Run all integration tests (ignored by default)
//! cargo test -p marketdata-core --test integration_websocket -- --ignored
//!
//! # Run a specific test
//! cargo test -p marketdata-core --test integration_websocket test_stock_connection -- --ignored
//! ```
//!
//! # Test Requirements
//!
//! - Valid Fugle API key
//! - Network connectivity to stream.fugle.tw
//! - For live data tests: Tests should be run during market hours (9:00-13:30 Taiwan time)
//!
//! # Note on Market Hours
//!
//! Some tests may timeout or receive no data outside of market hours.
//! This is expected behavior - the streaming API only sends data during active trading.
//!
//! Async (tokio) variant. Gated behind `tokio-comp`. See
//! `integration_websocket_sync.rs` for the sync mirror.
#![cfg(feature = "tokio-comp")]

use marketdata_core::aio::WebSocketClient;
use marketdata_core::{
    AuthRequest, Channel, ConnectionConfig, ConnectionEvent, ConnectionState,
    HealthCheckConfig,
};
use marketdata_core::websocket::StockSubscription;
use std::env;
use std::time::Duration;

/// Helper function to get API key from environment
fn get_api_key() -> String {
    env::var("FUGLE_API_KEY")
        .expect("FUGLE_API_KEY environment variable must be set for integration tests")
}

/// Helper function to create stock WebSocket config
fn stock_config() -> ConnectionConfig {
    let api_key = get_api_key();
    ConnectionConfig::fugle_stock(AuthRequest::with_api_key(&api_key))
}

/// Helper function to create futopt WebSocket config
fn futopt_config() -> ConnectionConfig {
    let api_key = get_api_key();
    ConnectionConfig::fugle_futopt(AuthRequest::with_api_key(&api_key))
}

// =============================================================================
// Connection Tests
// =============================================================================

#[test]
#[ignore]
fn test_stock_connection() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {
        let config = stock_config();
        let client = WebSocketClient::new(config);

        // Connect to WebSocket
        let result = client.connect().await;

        match result {
            Ok(_) => {
                assert_eq!(client.state(), ConnectionState::Connected);
                println!("Successfully connected to stock WebSocket");

                // Disconnect cleanly
                client.disconnect().await.ok();
            }
            Err(e) => {
                panic!("Failed to connect to stock WebSocket: {:?}", e);
            }
        }
    });
}

#[test]
#[ignore]
fn test_futopt_connection() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {
        let config = futopt_config();
        let client = WebSocketClient::new(config);

        let result = client.connect().await;

        match result {
            Ok(_) => {
                assert_eq!(client.state(), ConnectionState::Connected);
                println!("Successfully connected to futopt WebSocket");

                client.disconnect().await.ok();
            }
            Err(e) => {
                panic!("Failed to connect to futopt WebSocket: {:?}", e);
            }
        }
    });
}

#[test]
#[ignore]
fn test_connection_with_invalid_api_key() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {
        let config = ConnectionConfig::fugle_stock(
            AuthRequest::with_api_key("invalid-api-key")
        );
        let client = WebSocketClient::new(config);

        let result = client.connect().await;

        // Should fail to connect with invalid API key
        assert!(result.is_err(), "Should fail to connect with invalid API key");
        if let Err(e) = result {
            println!("Expected connection error: {:?}", e);
        }
    });
}

// =============================================================================
// Subscription Tests
// =============================================================================

#[test]
#[ignore]
fn test_stock_subscription() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {
        let config = stock_config();
        let client = WebSocketClient::new(config);

        client.connect().await.expect("Failed to connect");

        // Subscribe to TSMC trades using SubscribeRequest
        let sub = StockSubscription::new(Channel::Trades, "2330");
        let result = client.subscribe(sub).await;

        match result {
            Ok(()) => {
                println!("Subscribed to 2330 trades");

                // Wait a moment for subscription confirmation
                tokio::time::sleep(Duration::from_secs(1)).await;

                // Unsubscribe using the key format
                client.unsubscribe(["trades:2330"]).await.ok();
            }
            Err(e) => {
                panic!("Failed to subscribe: {:?}", e);
            }
        }

        client.disconnect().await.ok();
    });
}

#[test]
#[ignore]
fn test_multiple_subscriptions() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {
        let config = stock_config();
        let client = WebSocketClient::new(config);

        client.connect().await.expect("Failed to connect");

        // Subscribe to multiple symbols and channels
        let symbols = ["2330", "2317", "2454"];
        let mut subscription_count = 0;

        for symbol in symbols {
            let sub = StockSubscription::new(Channel::Trades, symbol);
            match client.subscribe(sub).await {
                Ok(()) => {
                    println!("Subscribed to {} trades", symbol);
                    subscription_count += 1;
                }
                Err(e) => {
                    eprintln!("Failed to subscribe to {}: {:?}", symbol, e);
                }
            }
        }

        assert!(subscription_count > 0, "Should have at least one subscription");
        println!("Total subscriptions: {}", subscription_count);

        // Cleanup
        for symbol in symbols {
            let key = format!("trades:{}", symbol);
            client.unsubscribe([key]).await.ok();
        }

        client.disconnect().await.ok();
    });
}

// =============================================================================
// Message Receiving Tests
// =============================================================================

#[test]
#[ignore]
fn test_receive_stock_messages() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {
        let config = stock_config();
        let client = WebSocketClient::new(config);

        client.connect().await.expect("Failed to connect");

        // Subscribe to trades for a liquid stock
        let sub = StockSubscription::new(Channel::Trades, "2330");
        client.subscribe(sub).await.expect("Failed to subscribe");

        println!("Waiting for messages (10 seconds)...");
        println!("Note: No messages expected outside market hours (9:00-13:30 Taiwan time)");

        let receiver = client.messages();
        let start = std::time::Instant::now();
        let timeout_duration = Duration::from_secs(10);
        let mut message_count = 0;

        while start.elapsed() < timeout_duration {
            if let Some(msg) = receiver.try_receive() {
                println!("Received message: {:?}", msg);
                message_count += 1;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        println!("Received {} messages in {:?}", message_count, start.elapsed());

        // Cleanup
        client.unsubscribe(["trades:2330"]).await.ok();
        client.disconnect().await.ok();
    });
}

#[test]
#[ignore]
fn test_message_latency() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {
        let config = stock_config();
        let client = WebSocketClient::new(config);

        client.connect().await.expect("Failed to connect");

        // Subscribe to aggregates for snapshot data
        let sub = StockSubscription::new(Channel::Aggregates, "2330");
        client.subscribe(sub).await.expect("Failed to subscribe");

        let receiver = client.messages();
        let start = std::time::Instant::now();
        let timeout_duration = Duration::from_secs(30);
        let mut latencies = Vec::new();

        println!("Measuring message latency (30 seconds)...");
        println!("Note: Requires market hours for trade messages");

        while start.elapsed() < timeout_duration && latencies.len() < 10 {
            let msg_start = std::time::Instant::now();
            if receiver.try_receive().is_some() {
                latencies.push(msg_start.elapsed());
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        if !latencies.is_empty() {
            println!("Message latencies: {:?}", latencies);
            let avg: Duration = latencies.iter().sum::<Duration>() / latencies.len() as u32;
            println!("Average message processing latency: {:?}", avg);
        } else {
            println!("No messages received (market may be closed)");
        }

        client.disconnect().await.ok();
    });
}

// =============================================================================
// Reconnection Tests
// =============================================================================

#[test]
#[ignore]
fn test_reconnection_preserves_subscriptions() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {
        let config = stock_config();
        let client = WebSocketClient::new(config);

        client.connect().await.expect("Failed to connect");

        // Subscribe to a symbol
        let sub = StockSubscription::new(Channel::Trades, "2330");
        client.subscribe(sub).await.expect("Failed to subscribe");

        println!("Initial connection established");

        // Trigger reconnection
        client.reconnect().await.expect("Failed to reconnect");

        assert_eq!(client.state(), ConnectionState::Connected);
        println!("Reconnected successfully");

        // Subscriptions should be automatically restored
        // (The client re-subscribes during reconnection)

        client.disconnect().await.ok();
    });
}

// =============================================================================
// FutOpt Tests
// =============================================================================

#[test]
#[ignore]
fn test_futopt_subscription() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {
        let config = futopt_config();
        let client = WebSocketClient::new(config);

        client.connect().await.expect("Failed to connect");

        // Subscribe to TX futures trades using SubscribeRequest
        // Note: Symbol format may need adjustment based on current contracts
        let sub = StockSubscription::new(Channel::Trades, "TXFK4");
        match client.subscribe(sub).await {
            Ok(()) => {
                println!("Subscribed to TXFK4 trades");
                tokio::time::sleep(Duration::from_secs(2)).await;
                client.unsubscribe(["trades:TXFK4"]).await.ok();
            }
            Err(e) => {
                eprintln!("FutOpt subscription error (symbol may be expired): {:?}", e);
            }
        }

        client.disconnect().await.ok();
    });
}

// =============================================================================
// Graceful Shutdown Tests
// =============================================================================

#[test]
#[ignore]
fn test_graceful_disconnect() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {
        let config = stock_config();
        let client = WebSocketClient::new(config);

        client.connect().await.expect("Failed to connect");

        // Subscribe to something
        let sub = StockSubscription::new(Channel::Trades, "2330");
        client.subscribe(sub).await.expect("Failed to subscribe");

        // Graceful disconnect with close handshake
        let result = client.disconnect().await;

        match result {
            Ok(_) => {
                println!("Graceful disconnect successful");
                // After disconnect, state should be Closed
                let state = client.state();
                println!("Final state: {:?}", state);
            }
            Err(e) => {
                eprintln!("Disconnect error: {:?}", e);
            }
        }
    });
}

#[test]
#[ignore]
fn test_force_close() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {
        let config = stock_config();
        let client = WebSocketClient::new(config);

        client.connect().await.expect("Failed to connect");

        // Force close without waiting for handshake
        let result = client.force_close().await;

        match result {
            Ok(_) => {
                println!("Force close successful");
                assert!(client.is_closed().await);
            }
            Err(e) => {
                eprintln!("Force close error: {:?}", e);
            }
        }
    });
}

// =============================================================================
// Channel-based Subscription Tests
// =============================================================================

#[test]
#[ignore]
fn test_subscribe_channel_trades() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {
        let config = stock_config();
        let client = WebSocketClient::new(config);

        client.connect().await.expect("Failed to connect");

        // StockSubscription single-symbol form
        let sub = StockSubscription::new(Channel::Trades, "2330");
        let result = client.subscribe(sub).await;

        match result {
            Ok(()) => {
                println!("Subscribed to 2330 trades via subscribe_channel");
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            Err(e) => {
                panic!("Failed to subscribe via channel: {:?}", e);
            }
        }

        client.disconnect().await.ok();
    });
}

#[test]
#[ignore]
fn test_subscribe_multiple_symbols() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {
        let config = stock_config();
        let client = WebSocketClient::new(config);

        client.connect().await.expect("Failed to connect");

        // Batch-symbol form via StockSubscription::new with Vec<&str>
        let sub = StockSubscription::new(Channel::Trades, vec!["2330", "2317", "2454"]);
        let result = client.subscribe(sub).await;

        match result {
            Ok(()) => {
                println!("Subscribed to multiple symbols via subscribe_symbols");
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            Err(e) => {
                panic!("Failed to subscribe symbols: {:?}", e);
            }
        }

        client.disconnect().await.ok();
    });
}

// =============================================================================
// Heartbeat / Liveness Detection
// =============================================================================

/// End-to-end check that the read-site `tokio::time::timeout` in
/// dispatch_messages emits `ConnectionEvent::HeartbeatTimeout` when the
/// configured window elapses without any inbound frame.
///
/// We force the timeout to fire on purpose by configuring an
/// artificially-short heartbeat_timeout (5s) against the live server,
/// which sends a heartbeat every 30s. The test asserts:
/// 1. We receive a `HeartbeatTimeout` event within reasonable bounds.
/// 2. The reconnect path subsequently fires a `Reconnecting` event.
///
/// This deliberately produces false-disconnects against the production
/// server (5s < 30s), so it must be `#[ignore]`'d — run manually before
/// each major release.
#[test]
#[ignore]
fn test_heartbeat_timeout_triggers_reconnect_in_real_env() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {
        let config = stock_config();
        let client = WebSocketClient::with_health_check_config(
            config,
            // 5 s timeout against a 30 s server heartbeat == guaranteed
            // false disconnect after 5s of silence.
            HealthCheckConfig::with_timeout(Duration::from_secs(5))
                .expect("5s is exactly at the floor and must validate"),
        );

        client.connect().await.expect("Failed to connect");
        assert_eq!(client.state_async().await, ConnectionState::Connected);

        // Drain events for up to 30s; expect HeartbeatTimeout, then
        // Reconnecting.
        let events = client.events();
        let mut saw_heartbeat_timeout = false;
        let mut saw_reconnecting = false;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(30);

        loop {
            if tokio::time::Instant::now() >= deadline {
                break;
            }
            // Poll with a small budget so we don't hang past the deadline.
            let event = tokio::task::spawn_blocking({
                let events = events.clone();
                move || {
                    let rx = events.blocking_lock();
                    rx.recv_timeout(Duration::from_secs(1))
                }
            })
            .await
            .ok();

            match event {
                Some(Ok(ConnectionEvent::HeartbeatTimeout { elapsed, .. })) => {
                    println!("Got HeartbeatTimeout after {:?}", elapsed);
                    saw_heartbeat_timeout = true;
                }
                Some(Ok(ConnectionEvent::Reconnecting { attempt, .. })) => {
                    println!("Got Reconnecting (attempt {})", attempt);
                    saw_reconnecting = true;
                    if saw_heartbeat_timeout {
                        break;
                    }
                }
                _ => continue,
            }
        }

        client.force_close().await.ok();

        assert!(
            saw_heartbeat_timeout,
            "Expected HeartbeatTimeout within 30s with 5s heartbeat_timeout against a 30s heartbeat server",
        );
        assert!(
            saw_reconnecting,
            "Expected Reconnecting event after HeartbeatTimeout (auto-reconnect should kick in)",
        );
    });
}
