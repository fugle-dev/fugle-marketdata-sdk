//! WebSocket throughput benchmarks
//!
//! These benchmarks measure message processing throughput, connection configuration,
//! and subscription management overhead. Network throughput tests require a valid
//! API key and are documented separately.
//!
//! # Running Benchmarks
//!
//! ```bash
//! # Run all WebSocket benchmarks (no network required)
//! cargo bench -p marketdata-core --bench websocket_throughput
//!
//! # Run with API key for live throughput tests
//! FUGLE_API_KEY=your_key cargo bench -p marketdata-core --bench websocket_throughput -- --ignored
//! ```

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use marketdata_core::websocket::{
    ReconnectionConfig, HealthCheckConfig,
    StockSubscription,
};
use marketdata_core::{Channel, FutOptChannel, AuthRequest, ConnectionConfig};

/// Benchmark ConnectionConfig construction and building
fn bench_config_construction(c: &mut Criterion) {
    let mut group = c.benchmark_group("websocket_config_construction");

    group.bench_function("connection_config_fugle_stock", |b| {
        b.iter(|| {
            let auth = AuthRequest::with_api_key("test-api-key");
            black_box(ConnectionConfig::fugle_stock(auth))
        })
    });

    group.bench_function("connection_config_fugle_futopt", |b| {
        b.iter(|| {
            let auth = AuthRequest::with_api_key("test-api-key");
            black_box(ConnectionConfig::fugle_futopt(auth))
        })
    });

    group.bench_function("reconnection_config_default", |b| {
        b.iter(|| {
            black_box(ReconnectionConfig::default())
        })
    });

    group.bench_function("health_check_config_default", |b| {
        b.iter(|| {
            black_box(HealthCheckConfig::default())
        })
    });

    group.finish();
}

/// Benchmark StockSubscription creation
fn bench_subscription_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("subscription_creation");

    group.bench_function("stock_subscription_trades", |b| {
        b.iter(|| {
            black_box(StockSubscription::new(Channel::Trades, "2330"))
        })
    });

    group.bench_function("stock_subscription_candles", |b| {
        b.iter(|| {
            black_box(StockSubscription::new(Channel::Candles, "2330"))
        })
    });

    group.bench_function("stock_subscription_books", |b| {
        b.iter(|| {
            black_box(StockSubscription::new(Channel::Books, "2330"))
        })
    });

    group.bench_function("stock_subscription_aggregates", |b| {
        b.iter(|| {
            black_box(StockSubscription::new(Channel::Aggregates, "2330"))
        })
    });

    group.finish();
}

/// Benchmark channel enum operations
fn bench_channel_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("channel_operations");

    let stock_channels = vec![
        Channel::Trades,
        Channel::Candles,
        Channel::Books,
        Channel::Aggregates,
        Channel::Indices,
    ];

    group.bench_function("stock_channel_clone", |b| {
        b.iter(|| {
            for channel in &stock_channels {
                black_box(channel.clone());
            }
        })
    });

    let futopt_channels = vec![
        FutOptChannel::Trades,
        FutOptChannel::Candles,
        FutOptChannel::Books,
        FutOptChannel::Aggregates,
    ];

    group.bench_function("futopt_channel_clone", |b| {
        b.iter(|| {
            for channel in &futopt_channels {
                black_box(channel.clone());
            }
        })
    });

    group.finish();
}

/// Benchmark message JSON parsing throughput
fn bench_message_parsing(c: &mut Criterion) {
    // Typical trade message from WebSocket
    let trade_msg = r#"{
        "event": "data",
        "channel": "trades",
        "data": {
            "symbol": "2330",
            "exchange": "TWSE",
            "market": "TSE",
            "bid": 597.0,
            "ask": 598.0,
            "price": 597.5,
            "size": 100,
            "time": 1705299000000,
            "serial": 12345
        }
    }"#;

    // Typical quote snapshot message
    let quote_msg = r#"{
        "event": "data",
        "channel": "aggregates",
        "data": {
            "date": "2024-01-15",
            "type": "EQUITY",
            "exchange": "TWSE",
            "market": "TSE",
            "symbol": "2330",
            "name": "台積電",
            "referencePrice": 595.0,
            "previousClose": 595.0,
            "openPrice": 597.0,
            "highPrice": 600.0,
            "lowPrice": 595.0,
            "closePrice": 598.0,
            "lastPrice": 598.0,
            "lastSize": 100,
            "total": {"tradeValue": 15000000000, "tradeVolume": 25000},
            "isClose": false,
            "serial": 12345,
            "lastUpdated": 1705299000000
        }
    }"#;

    let mut group = c.benchmark_group("message_parsing");

    group.throughput(Throughput::Bytes(trade_msg.len() as u64));
    group.bench_function("parse_trade_message", |b| {
        b.iter(|| {
            black_box(serde_json::from_str::<serde_json::Value>(trade_msg).unwrap())
        })
    });

    group.throughput(Throughput::Bytes(quote_msg.len() as u64));
    group.bench_function("parse_quote_message", |b| {
        b.iter(|| {
            black_box(serde_json::from_str::<serde_json::Value>(quote_msg).unwrap())
        })
    });

    group.finish();
}

/// Benchmark message throughput with varying batch sizes
fn bench_message_batch_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("message_batch_throughput");

    // Simulate batch of trade messages
    for batch_size in [10, 50, 100, 500].iter() {
        let messages: Vec<String> = (0..*batch_size)
            .map(|i| {
                serde_json::json!({
                    "event": "data",
                    "channel": "trades",
                    "data": {
                        "symbol": "2330",
                        "bid": 597.0,
                        "ask": 598.0,
                        "price": 597.5,
                        "size": 100 + i,
                        "time": 1705299000000_i64 + (i * 100) as i64,
                        "serial": 10000 + i
                    }
                })
                .to_string()
            })
            .collect();

        group.throughput(Throughput::Elements(*batch_size as u64));
        group.bench_with_input(
            BenchmarkId::new("parse_batch", batch_size),
            &messages,
            |b, msgs| {
                b.iter(|| {
                    for msg in msgs {
                        black_box(serde_json::from_str::<serde_json::Value>(msg).unwrap());
                    }
                })
            },
        );
    }

    group.finish();
}

/// Benchmark subscription key generation (used for deduplication)
fn bench_subscription_key_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("subscription_key_generation");

    let subscription = StockSubscription::new(Channel::Trades, "2330");

    group.bench_function("subscription_to_key", |b| {
        b.iter(|| {
            // Use actual subscription key method
            black_box(subscription.key())
        })
    });

    // Test with longer symbol names
    group.bench_function("subscription_to_key_long_symbol", |b| {
        let long_sub = StockSubscription::new(Channel::Trades, "TXFA4-2024-01-W3");
        b.iter(|| {
            black_box(long_sub.key())
        })
    });

    group.finish();
}

/// Benchmark exponential backoff calculation
fn bench_backoff_calculation(c: &mut Criterion) {
    use std::time::Duration;

    let mut group = c.benchmark_group("backoff_calculation");

    let config = ReconnectionConfig::default();

    group.bench_function("calculate_backoff_delay", |b| {
        b.iter(|| {
            // Simulate backoff calculation for various attempt counts
            for attempt in 0..5u32 {
                let base = config.initial_delay;
                let max = config.max_delay;
                let delay = std::cmp::min(
                    Duration::from_millis(base.as_millis() as u64 * (2_u64.pow(attempt))),
                    max,
                );
                black_box(delay);
            }
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_config_construction,
    bench_subscription_creation,
    bench_channel_operations,
    bench_message_parsing,
    bench_message_batch_throughput,
    bench_subscription_key_generation,
    bench_backoff_calculation,
);

criterion_main!(benches);
