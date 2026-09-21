//! Basic WebSocket streaming example (sync, default).
//!
//! Demonstrates the default sync `WebSocketClient`. No tokio runtime
//! required. For the async (tokio) variant, see `websocket_async.rs`.
//!
//! # Prerequisites
//!
//! Set the `FUGLE_API_KEY` environment variable:
//! ```bash
//! export FUGLE_API_KEY="your-api-key"
//! ```
//!
//! # Run
//! ```bash
//! cargo run --example websocket_basic
//! ```

use marketdata_core::{
    websocket::{ConnectionConfig, ConnectionEvent, StockSubscription, StreamItem},
    AuthRequest, Channel, WebSocketClient,
};
use std::time::{Duration, Instant};

fn main() -> Result<(), marketdata_core::MarketDataError> {
    let api_key =
        std::env::var("FUGLE_API_KEY").expect("FUGLE_API_KEY environment variable not set");

    println!("=== WebSocket Streaming Example (sync) ===\n");

    let config = ConnectionConfig::fugle_stock(AuthRequest::with_api_key(&api_key));
    let client = WebSocketClient::new(config);

    println!("Connecting...");
    client.connect()?;
    println!("Connected and authenticated.\n");

    println!("Subscribing to channels...");
    client.subscribe(StockSubscription::new(Channel::Trades, "2330"))?;
    println!("  Subscribed to 2330 trades");
    client.subscribe(StockSubscription::new(Channel::Books, "2330"))?;
    println!("  Subscribed to 2330 books");
    client.subscribe(StockSubscription::new(Channel::Aggregates, "2330"))?;
    println!("  Subscribed to 2330 aggregates");
    client.subscribe(StockSubscription::new(Channel::Trades, vec!["2317", "2454"]))?;
    println!("  Subscribed to 2317, 2454 trades");

    println!("\nListening for messages (10 seconds)...\n");

    // Messages and connection events arrive on one stream, in order.
    let stream = client.stream_receiver();

    let start = Instant::now();
    let duration = Duration::from_secs(10);
    let mut message_count = 0;

    while start.elapsed() < duration {
        match stream.receive_timeout(Duration::from_secs(1)) {
            Ok(Some(StreamItem::Event(ConnectionEvent::Disconnected { code, reason, .. }))) => {
                println!("Event: Disconnected (code: {:?}, reason: {})", code, reason);
                break;
            }
            Ok(Some(StreamItem::Event(event))) => println!("Event: {:?}", event),
            Ok(Some(StreamItem::Message(msg))) => {
                message_count += 1;
                if msg.is_data() {
                    println!(
                        "[{}] Data: channel={:?}, symbol={:?}",
                        message_count,
                        msg.channel.as_deref().unwrap_or("?"),
                        msg.symbol.as_deref().unwrap_or("?")
                    );
                    if let Some(data) = msg.data() {
                        if msg.channel.as_deref() == Some("trades") {
                            let price = data.get("price").and_then(serde_json::Value::as_f64);
                            let size = data.get("size").and_then(serde_json::Value::as_i64);
                            if let (Some(price), Some(size)) = (price, size) {
                                println!("        Trade: price={}, size={}", price, size);
                            }
                        } else if msg.channel.as_deref() == Some("aggregates") {
                            let close = data.get("closePrice").and_then(serde_json::Value::as_f64);
                            if let Some(close) = close {
                                println!("        Aggregate: closePrice={}", close);
                            }
                        }
                    }
                } else if msg.is_subscribed() {
                    println!(
                        "[{}] Subscribed: id={:?}",
                        message_count,
                        msg.id.as_deref().unwrap_or("?")
                    );
                } else if msg.is_error() {
                    println!(
                        "[{}] Error: {:?}",
                        message_count,
                        msg.error_message().unwrap_or_else(|| "Unknown".to_string())
                    );
                } else {
                    println!("[{}] Event: {}", message_count, msg.event);
                }
            }
            Ok(Some(_)) => {}
            Ok(None) => {
                if !client.is_connected() {
                    println!("Connection lost");
                    break;
                }
            }
            Err(e) => {
                println!("Channel error: {}", e);
                break;
            }
        }
    }

    println!("\n=== Summary ===");
    println!("Total messages received: {}", message_count);
    println!("Active subscriptions: {:?}", client.subscription_keys());

    println!("\nDisconnecting...");
    client.disconnect()?;
    println!("Disconnected gracefully");

    if client.is_closed() {
        println!("Client is now closed (cannot be reused)");
    }

    println!("\n=== Complete ===");
    Ok(())
}
