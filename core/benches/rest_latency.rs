//! REST API latency benchmarks
//!
//! These benchmarks measure client-side serialization/deserialization overhead
//! and request building time. Network latency tests require a valid API key
//! and are documented separately.
//!
//! # Running Benchmarks
//!
//! ```bash
//! # Run all REST benchmarks (no network required)
//! cargo bench -p marketdata-core --bench rest_latency
//!
//! # Run with API key for live latency tests
//! FUGLE_API_KEY=your_key cargo bench -p marketdata-core --bench rest_latency -- --ignored
//! ```

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use marketdata_core::{Auth, RestClient};

/// Benchmark RestClient construction overhead
fn bench_client_construction(c: &mut Criterion) {
    let mut group = c.benchmark_group("rest_client_construction");

    group.bench_function("new_with_api_key", |b| {
        b.iter(|| {
            black_box(RestClient::new(Auth::ApiKey("test-key".to_string())))
        })
    });

    group.bench_function("new_with_bearer_token", |b| {
        b.iter(|| {
            black_box(RestClient::new(Auth::BearerToken("test-token".to_string())))
        })
    });

    group.bench_function("new_with_sdk_token", |b| {
        b.iter(|| {
            black_box(RestClient::new(Auth::SdkToken("test-sdk-token".to_string())))
        })
    });

    group.finish();
}

/// Benchmark client cloning (connection pool sharing)
fn bench_client_clone(c: &mut Criterion) {
    let client = RestClient::new(Auth::SdkToken("test-token".to_string()));

    c.bench_function("rest_client_clone", |b| {
        b.iter(|| {
            black_box(client.clone())
        })
    });
}

/// Benchmark client method chaining overhead
fn bench_client_chaining(c: &mut Criterion) {
    let client = RestClient::new(Auth::SdkToken("test-token".to_string()));

    let mut group = c.benchmark_group("rest_client_chaining");

    group.bench_function("stock_intraday_chain", |b| {
        b.iter(|| {
            black_box(client.stock().intraday())
        })
    });

    group.bench_function("futopt_intraday_chain", |b| {
        b.iter(|| {
            black_box(client.futopt().intraday())
        })
    });

    group.finish();
}

/// Benchmark response deserialization with mock data
///
/// This measures JSON parsing performance for typical API responses.
fn bench_response_deserialization(c: &mut Criterion) {
    use marketdata_core::{Quote, Ticker};

    // Typical quote response JSON
    let quote_json = r#"{
        "date": "2024-01-15",
        "type": "EQUITY",
        "exchange": "TWSE",
        "market": "TSE",
        "symbol": "2330",
        "name": "台積電",
        "referencePrice": 595.0,
        "previousClose": 595.0,
        "openPrice": 597.0,
        "openTime": 1705287000000,
        "highPrice": 600.0,
        "highTime": 1705290600000,
        "lowPrice": 595.0,
        "lowTime": 1705287000000,
        "closePrice": 598.0,
        "closeTime": 1705299000000,
        "avgPrice": 597.5,
        "change": 3.0,
        "changePercent": 0.5042,
        "amplitude": 0.84,
        "lastPrice": 598.0,
        "lastSize": 100,
        "bids": [{"price": 597.0, "size": 500}],
        "asks": [{"price": 598.0, "size": 300}],
        "total": {"tradeValue": 15000000000, "tradeVolume": 25000, "tradeVolumeAtBid": 12000, "tradeVolumeAtAsk": 13000, "transaction": 5000, "time": 1705299000000},
        "lastTrade": {"bid": 597.0, "ask": 598.0, "price": 598.0, "size": 100, "time": 1705299000000, "serial": 12345},
        "lastTrial": null,
        "isClose": true,
        "serial": 12345,
        "lastUpdated": 1705299000000
    }"#;

    // Typical ticker response JSON
    let ticker_json = r#"{
        "date": "2024-01-15",
        "type": "EQUITY",
        "exchange": "TWSE",
        "market": "TSE",
        "symbol": "2330",
        "name": "台積電",
        "industry": "24",
        "previousClose": 595.0,
        "referencePrice": 595.0,
        "limitUpPrice": 654.0,
        "limitDownPrice": 536.0,
        "canDayTrade": true,
        "canBuyDayTrade": true,
        "isAttention": false,
        "isDisposition": false,
        "matchingInterval": 5,
        "boardLot": 1000
    }"#;

    let mut group = c.benchmark_group("response_deserialization");

    group.bench_function("quote_response", |b| {
        b.iter(|| {
            black_box(serde_json::from_str::<Quote>(quote_json).unwrap())
        })
    });

    group.bench_function("ticker_response", |b| {
        b.iter(|| {
            black_box(serde_json::from_str::<Ticker>(ticker_json).unwrap())
        })
    });

    group.finish();
}

/// Benchmark Auth serialization for request headers
fn bench_auth_header_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("auth_header_generation");

    let api_key = Auth::ApiKey("demo-api-key-12345".to_string());
    let bearer_token = Auth::BearerToken("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.test".to_string());
    let sdk_token = Auth::SdkToken("sdk-token-abcdef123456".to_string());

    group.bench_function("api_key_header", |b| {
        b.iter(|| {
            black_box(match &api_key {
                Auth::ApiKey(key) => format!("X-API-KEY: {}", key),
                _ => unreachable!(),
            })
        })
    });

    group.bench_function("bearer_token_header", |b| {
        b.iter(|| {
            black_box(match &bearer_token {
                Auth::BearerToken(token) => format!("Authorization: Bearer {}", token),
                _ => unreachable!(),
            })
        })
    });

    group.bench_function("sdk_token_header", |b| {
        b.iter(|| {
            black_box(match &sdk_token {
                Auth::SdkToken(token) => format!("Authorization: Bearer {}", token),
                _ => unreachable!(),
            })
        })
    });

    group.finish();
}

/// Benchmark varying payload sizes for deserialization
fn bench_payload_size_scaling(c: &mut Criterion) {
    #[allow(unused_imports)]
    use marketdata_core::Trade;

    let mut group = c.benchmark_group("payload_size_scaling");

    // Generate trade arrays of varying sizes
    for size in [10, 100, 500, 1000].iter() {
        let trades: Vec<serde_json::Value> = (0..*size)
            .map(|i| {
                serde_json::json!({
                    "bid": 597.0,
                    "ask": 598.0,
                    "price": 597.5,
                    "size": 100 + i,
                    "time": 1705299000000_i64 + (i * 1000) as i64,
                    "serial": 10000 + i
                })
            })
            .collect();

        let json = serde_json::json!({
            "date": "2024-01-15",
            "exchange": "TWSE",
            "market": "TSE",
            "symbol": "2330",
            "data": trades
        });
        let json_str = serde_json::to_string(&json).unwrap();

        group.bench_with_input(
            BenchmarkId::new("trades_response", size),
            &json_str,
            |b, json| {
                b.iter(|| {
                    black_box(serde_json::from_str::<serde_json::Value>(json).unwrap())
                })
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_client_construction,
    bench_client_clone,
    bench_client_chaining,
    bench_response_deserialization,
    bench_auth_header_generation,
    bench_payload_size_scaling,
);

criterion_main!(benches);
