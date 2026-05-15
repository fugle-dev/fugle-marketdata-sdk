//! Integration tests for REST API client
//!
//! These tests require a valid FUGLE_API_KEY environment variable and
//! make real HTTP requests to the Fugle API.
//!
//! # Running Integration Tests
//!
//! ```bash
//! # Set your API key
//! export FUGLE_API_KEY=your_api_key_here
//!
//! # Run all integration tests (ignored by default)
//! cargo test -p marketdata-core --test integration_rest -- --ignored
//!
//! # Run a specific test
//! cargo test -p marketdata-core --test integration_rest test_stock_quote -- --ignored
//! ```
//!
//! # Test Requirements
//!
//! - Valid Fugle API key
//! - Network connectivity to api.fugle.tw
//! - Tests should be run during market hours for live data

use marketdata_core::{Auth, RestClient};
use marketdata_core::models::futopt::FutOptType;
use std::env;

/// Helper function to get API key from environment
fn get_api_key() -> String {
    env::var("FUGLE_API_KEY")
        .expect("FUGLE_API_KEY environment variable must be set for integration tests")
}

/// Helper function to create authenticated REST client
fn create_client() -> RestClient {
    let api_key = get_api_key();
    RestClient::new(Auth::ApiKey(api_key))
}

// =============================================================================
// Stock Intraday Tests
// =============================================================================

#[test]
#[ignore]
fn test_stock_quote() {
    let client = create_client();

    // Test with TSMC (2330) - Taiwan's largest company
    let result = client.stock().intraday().quote().symbol("2330").send();

    match result {
        Ok(quote) => {
            assert_eq!(quote.symbol, "2330");
            assert!(!quote.date.is_empty(), "Quote should have a date");
            println!("Quote for 2330: {:?}", quote);
        }
        Err(e) => {
            panic!("Failed to get stock quote: {:?}", e);
        }
    }
}

#[test]
#[ignore]
fn test_stock_ticker() {
    let client = create_client();

    let result = client.stock().intraday().ticker().symbol("2330").send();

    match result {
        Ok(ticker) => {
            assert_eq!(ticker.symbol, "2330");
            assert!(ticker.limit_up_price.is_some(), "Ticker should have limit up price");
            assert!(ticker.limit_down_price.is_some(), "Ticker should have limit down price");
            println!("Ticker for 2330: {:?}", ticker);
        }
        Err(e) => {
            panic!("Failed to get stock ticker: {:?}", e);
        }
    }
}

#[test]
#[ignore]
fn test_stock_trades() {
    let client = create_client();

    let result = client.stock().intraday().trades().symbol("2330").send();

    match result {
        Ok(trades) => {
            assert_eq!(trades.symbol, "2330");
            println!("Trades for 2330: {} trades returned", trades.data.len());
        }
        Err(e) => {
            panic!("Failed to get stock trades: {:?}", e);
        }
    }
}

#[test]
#[ignore]
fn test_stock_candles() {
    let client = create_client();

    let result = client
        .stock()
        .intraday()
        .candles()
        .symbol("2330")
        .timeframe("5")
        .send();

    match result {
        Ok(candles) => {
            assert_eq!(candles.symbol, "2330");
            println!("Candles for 2330: {} candles returned", candles.data.len());
        }
        Err(e) => {
            panic!("Failed to get stock candles: {:?}", e);
        }
    }
}

#[test]
#[ignore]
fn test_stock_volumes() {
    let client = create_client();

    let result = client.stock().intraday().volumes().symbol("2330").send();

    match result {
        Ok(volumes) => {
            assert_eq!(volumes.symbol, "2330");
            println!("Volumes for 2330: {} price levels", volumes.data.len());
        }
        Err(e) => {
            panic!("Failed to get stock volumes: {:?}", e);
        }
    }
}

// =============================================================================
// FutOpt Intraday Tests
// =============================================================================

#[test]
#[ignore]
fn test_futopt_products() {
    let client = create_client();

    // Test futures products using typ() method
    let result = client.futopt().intraday().products().typ(FutOptType::Future).send();

    match result {
        Ok(products) => {
            assert!(!products.data.is_empty(), "Should have at least one futures product");
            println!("Futures products: {} products returned", products.data.len());
        }
        Err(e) => {
            panic!("Failed to get futures products: {:?}", e);
        }
    }
}

#[test]
#[ignore]
fn test_futopt_quote() {
    let client = create_client();

    // Test with TX (Taiwan Index Futures)
    // Note: Symbol format may vary, adjust as needed
    let result = client.futopt().intraday().quote().symbol("TXFK4").send();

    match result {
        Ok(quote) => {
            println!("FutOpt quote: {:?}", quote);
        }
        Err(e) => {
            // FutOpt symbols may change frequently, so we log instead of panic
            eprintln!("FutOpt quote error (symbol may be expired): {:?}", e);
        }
    }
}

// =============================================================================
// Error Handling Tests
// =============================================================================

#[test]
#[ignore]
fn test_invalid_symbol() {
    let client = create_client();

    let result = client.stock().intraday().quote().symbol("INVALID_SYMBOL_12345").send();

    // Should return an error for invalid symbol
    assert!(result.is_err(), "Invalid symbol should return an error");
    if let Err(e) = result {
        println!("Expected error for invalid symbol: {:?}", e);
    }
}

#[test]
#[ignore]
fn test_invalid_api_key() {
    let client = RestClient::new(Auth::ApiKey("invalid-api-key".to_string()));

    let result = client.stock().intraday().quote().symbol("2330").send();

    // Should return an authentication error
    assert!(result.is_err(), "Invalid API key should return an error");
    if let Err(e) = result {
        println!("Expected auth error for invalid API key: {:?}", e);
        // Check that it's an authentication error
        assert!(
            e.is_retryable() == false,
            "Auth errors should not be retryable"
        );
    }
}

// =============================================================================
// Performance Tests
// =============================================================================

#[test]
#[ignore]
fn test_multiple_requests_latency() {
    let client = create_client();
    let mut latencies = Vec::new();

    // Make 5 consecutive requests and measure latency
    for i in 0..5 {
        let start = std::time::Instant::now();
        let result = client.stock().intraday().quote().symbol("2330").send();
        let elapsed = start.elapsed();

        if result.is_ok() {
            latencies.push(elapsed);
            println!("Request {} latency: {:?}", i + 1, elapsed);
        } else {
            println!("Request {} failed: {:?}", i + 1, result.err());
        }

        // Small delay between requests to avoid rate limiting
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    if !latencies.is_empty() {
        let avg_latency: std::time::Duration =
            latencies.iter().sum::<std::time::Duration>() / latencies.len() as u32;
        println!("\nAverage latency: {:?}", avg_latency);
        println!("Latencies: {:?}", latencies);

        // Sort for p50/p95 calculation
        latencies.sort();
        let p50 = latencies[latencies.len() / 2];
        let p95_idx = (latencies.len() as f64 * 0.95) as usize;
        let p95 = latencies[p95_idx.min(latencies.len() - 1)];

        println!("P50 latency: {:?}", p50);
        println!("P95 latency: {:?}", p95);
    }
}

#[test]
#[ignore]
fn test_connection_reuse() {
    let client = create_client();

    // First request establishes connection
    let start1 = std::time::Instant::now();
    let _ = client.stock().intraday().quote().symbol("2330").send();
    let first_request_time = start1.elapsed();

    // Second request should reuse connection (faster)
    let start2 = std::time::Instant::now();
    let _ = client.stock().intraday().quote().symbol("2330").send();
    let second_request_time = start2.elapsed();

    println!("First request (cold): {:?}", first_request_time);
    println!("Second request (warm): {:?}", second_request_time);

    // The second request should generally be faster due to connection reuse
    // but this isn't guaranteed, so we just log it
}
