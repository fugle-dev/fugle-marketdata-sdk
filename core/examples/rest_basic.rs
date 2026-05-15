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
    println!("   Close Price: {:?}", quote.close_price);
    println!("   Change: {:?}", quote.change);
    println!("   Change %: {:?}", quote.change_percent);
    if let Some(ref total) = quote.total {
        println!("   Volume: {:?}", total.trade_volume);
        println!("   Value: {:?}", total.trade_value);
    }
    println!();

    // 2. Get stock ticker info
    println!("2. Stock Ticker Info:");
    let ticker = client.stock().intraday().ticker().symbol("2330").send()?;
    println!("   Symbol: {}", ticker.symbol);
    println!("   Name: {:?}", ticker.name);
    println!("   Exchange: {:?}", ticker.exchange);
    println!("   Type: {:?}", ticker.data_type);
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
    println!("   Total candles: {}", candles.data.len());
    if let Some(first) = candles.data.first() {
        println!("   First candle:");
        println!("      Date: {}", first.date);
        println!("      Open: {}", first.open);
        println!("      High: {}", first.high);
        println!("      Low: {}", first.low);
        println!("      Close: {}", first.close);
        println!("      Volume: {}", first.volume);
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
    println!("   Total trades: {}", trades.data.len());
    for (i, trade) in trades.data.iter().take(3).enumerate() {
        println!("   Trade {}:", i + 1);
        println!("      Price: {}", trade.price);
        println!("      Size: {}", trade.size);
        println!("      Time: {}", trade.time);
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
    println!("   Price levels: {}", volumes.data.len());
    for level in volumes.data.iter().take(3) {
        println!("   Price {}: {} shares", level.price, level.volume);
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
    println!("   Total products: {}", products.data.len());
    for product in products.data.iter().take(3) {
        println!("   - {} ({:?})", product.symbol, product.name);
    }
    println!();

    // Note: FutOpt quote requires a valid contract symbol
    // Example: TXF202502 (Taiwan Futures Feb 2025)

    println!("=== Complete ===");
    println!("REST API examples finished successfully.");

    Ok(())
}
