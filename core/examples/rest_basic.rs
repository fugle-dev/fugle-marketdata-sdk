//! Basic REST API usage example
//!
//! This example demonstrates how to use the REST client to fetch market data.
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
//! cargo run --example rest_basic
//! ```

use marketdata_core::{Auth, FutOptType, RestClient};

fn main() -> Result<(), marketdata_core::MarketDataError> {
    // Get API key from environment
    let api_key = std::env::var("FUGLE_API_KEY").expect("FUGLE_API_KEY environment variable not set");

    // Create REST client with API key authentication
    let client = RestClient::new(Auth::ApiKey(api_key));

    println!("=== Stock Market Data ===\n");

    // 1. Get stock quote
    println!("1. Stock Quote (2330 TSMC):");
    let quote = client.stock().intraday().quote().symbol("2330").send()?;
    println!("   Close Price: {:?}", quote["closePrice"].as_f64());
    println!("   Change: {:?}", quote["change"].as_f64());
    println!("   Change %: {:?}", quote["changePercent"].as_f64());
    if !quote["total"].is_null() {
        println!("   Volume: {:?}", quote["total"]["tradeVolume"].as_i64());
        println!("   Value: {:?}", quote["total"]["tradeValue"].as_f64());
    }
    println!();

    // 2. Get stock ticker info
    println!("2. Stock Ticker Info:");
    let ticker = client.stock().intraday().ticker().symbol("2330").send()?;
    println!("   Symbol: {}", ticker["symbol"].as_str().unwrap_or(""));
    println!("   Name: {:?}", ticker["name"].as_str());
    println!("   Exchange: {:?}", ticker["exchange"].as_str());
    println!("   Type: {:?}", ticker["type"].as_str());
    println!();

    // 3. Get intraday candles
    println!("3. Intraday Candles (5-minute):");
    let candles = client
        .stock()
        .intraday()
        .candles()
        .symbol("2330")
        .timeframe("5")
        .send()?;
    let candle_data = candles["data"].as_array().cloned().unwrap_or_default();
    println!("   Total candles: {}", candle_data.len());
    if let Some(first) = candle_data.first() {
        println!("   First candle:");
        println!("      Date: {}", first["date"].as_str().unwrap_or(""));
        println!("      Open: {}", first["open"].as_f64().unwrap_or(0.0));
        println!("      High: {}", first["high"].as_f64().unwrap_or(0.0));
        println!("      Low: {}", first["low"].as_f64().unwrap_or(0.0));
        println!("      Close: {}", first["close"].as_f64().unwrap_or(0.0));
        println!("      Volume: {}", first["volume"].as_i64().unwrap_or(0));
    }
    println!();

    // 4. Get recent trades
    println!("4. Recent Trades:");
    let trades = client
        .stock()
        .intraday()
        .trades()
        .symbol("2330")
        .send()?;
    let trade_data = trades["data"].as_array().cloned().unwrap_or_default();
    println!("   Total trades: {}", trade_data.len());
    for (i, trade) in trade_data.iter().take(3).enumerate() {
        println!("   Trade {}:", i + 1);
        println!("      Price: {}", trade["price"].as_f64().unwrap_or(0.0));
        println!("      Size: {}", trade["size"].as_i64().unwrap_or(0));
        println!("      Time: {}", trade["time"].as_i64().unwrap_or(0));
    }
    println!();

    // 5. Get volume by price
    println!("5. Volume by Price:");
    let volumes = client
        .stock()
        .intraday()
        .volumes()
        .symbol("2330")
        .send()?;
    let volume_data = volumes["data"].as_array().cloned().unwrap_or_default();
    println!("   Price levels: {}", volume_data.len());
    for level in volume_data.iter().take(3) {
        println!(
            "   Price {}: {} shares",
            level["price"].as_f64().unwrap_or(0.0),
            level["volume"].as_i64().unwrap_or(0)
        );
    }
    println!();

    println!("=== FutOpt Market Data ===\n");

    // 6. Get futures/options products
    println!("6. Available Futures Products:");
    let products = client
        .futopt()
        .intraday()
        .products()
        .typ(FutOptType::Future)
        .send()?;
    let product_data = products["data"].as_array().cloned().unwrap_or_default();
    println!("   Total products: {}", product_data.len());
    for product in product_data.iter().take(3) {
        println!(
            "   - {} ({:?})",
            product["symbol"].as_str().unwrap_or(""),
            product["name"].as_str()
        );
    }
    println!();

    // Note: FutOpt quote requires a valid contract symbol
    // Example: TXF202502 (Taiwan Futures Feb 2025)

    println!("=== Complete ===");
    println!("REST API examples finished successfully.");

    Ok(())
}
