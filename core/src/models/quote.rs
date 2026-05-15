//! Stock quote data model - matches Fugle intraday/quote/{symbol} response

use serde::{Deserialize, Serialize};

use super::common::{PriceLevel, TotalStats, TradeInfo, TradingHalt};

/// Real-time stock quote from Fugle API (intraday/quote/{symbol})
///
/// This matches the official SDK's RestStockIntradayQuoteResponse
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "python", pyo3::prelude::pyclass)]
#[cfg_attr(feature = "js", napi_derive::napi(object))]
pub struct Quote {
    // === Response metadata ===
    /// Trading date (YYYY-MM-DD)
    pub date: String,

    /// Security type (e.g., "EQUITY", "ODDLOT")
    #[serde(rename = "type")]
    pub data_type: Option<String>,

    /// Exchange code (e.g., "TWSE", "TPEx")
    pub exchange: Option<String>,

    /// Market (e.g., "TSE", "OTC")
    pub market: Option<String>,

    /// Stock symbol (e.g., "2330")
    pub symbol: String,

    /// Stock name
    pub name: Option<String>,

    /// Previous trading day's close price
    #[serde(rename = "previousClose", default)]
    pub previous_close: Option<f64>,

    // === OHLC prices with timestamps ===
    /// Open price
    #[serde(rename = "openPrice")]
    pub open_price: Option<f64>,

    /// Open time (Unix ms)
    #[serde(rename = "openTime")]
    pub open_time: Option<i64>,

    /// High price
    #[serde(rename = "highPrice")]
    pub high_price: Option<f64>,

    /// High time (Unix ms)
    #[serde(rename = "highTime")]
    pub high_time: Option<i64>,

    /// Low price
    #[serde(rename = "lowPrice")]
    pub low_price: Option<f64>,

    /// Low time (Unix ms)
    #[serde(rename = "lowTime")]
    pub low_time: Option<i64>,

    /// Close price
    #[serde(rename = "closePrice")]
    pub close_price: Option<f64>,

    /// Close time (Unix ms)
    #[serde(rename = "closeTime")]
    pub close_time: Option<i64>,

    // === Current trading info ===
    /// Last traded price
    #[serde(rename = "lastPrice")]
    pub last_price: Option<f64>,

    /// Last traded size
    #[serde(rename = "lastSize")]
    pub last_size: Option<i64>,

    /// Average price
    #[serde(rename = "avgPrice")]
    pub avg_price: Option<f64>,

    /// Price change from previous close
    pub change: Option<f64>,

    /// Percentage change from previous close
    #[serde(rename = "changePercent")]
    pub change_percent: Option<f64>,

    /// Price amplitude (high - low) / previous close * 100
    pub amplitude: Option<f64>,

    // === Order book ===
    /// Bid price levels
    #[serde(default)]
    pub bids: Vec<PriceLevel>,

    /// Ask price levels
    #[serde(default)]
    pub asks: Vec<PriceLevel>,

    // === Aggregated stats ===
    /// Total trading statistics
    pub total: Option<TotalStats>,

    /// Last trade info
    #[serde(rename = "lastTrade")]
    pub last_trade: Option<TradeInfo>,

    /// Last trial (simulated matching) info
    #[serde(rename = "lastTrial")]
    pub last_trial: Option<TradeInfo>,

    /// Trading halt status
    #[serde(rename = "tradingHalt")]
    pub trading_halt: Option<TradingHalt>,

    // === Limit price flags ===
    /// Is at limit down price
    #[serde(rename = "isLimitDownPrice", default)]
    pub is_limit_down_price: bool,

    /// Is at limit up price
    #[serde(rename = "isLimitUpPrice", default)]
    pub is_limit_up_price: bool,

    /// Is limit down bid
    #[serde(rename = "isLimitDownBid", default)]
    pub is_limit_down_bid: bool,

    /// Is limit up bid
    #[serde(rename = "isLimitUpBid", default)]
    pub is_limit_up_bid: bool,

    /// Is limit down ask
    #[serde(rename = "isLimitDownAsk", default)]
    pub is_limit_down_ask: bool,

    /// Is limit up ask
    #[serde(rename = "isLimitUpAsk", default)]
    pub is_limit_up_ask: bool,

    /// Is limit down halt
    #[serde(rename = "isLimitDownHalt", default)]
    pub is_limit_down_halt: bool,

    /// Is limit up halt
    #[serde(rename = "isLimitUpHalt", default)]
    pub is_limit_up_halt: bool,

    // === Trading session flags ===
    /// Is in trial (simulated matching) period
    #[serde(rename = "isTrial", default)]
    pub is_trial: bool,

    /// Is delayed open
    #[serde(rename = "isDelayedOpen", default)]
    pub is_delayed_open: bool,

    /// Is delayed close
    #[serde(rename = "isDelayedClose", default)]
    pub is_delayed_close: bool,

    /// Is continuous trading
    #[serde(rename = "isContinuous", default)]
    pub is_continuous: bool,

    /// Is market open
    #[serde(rename = "isOpen", default)]
    pub is_open: bool,

    /// Is market closed
    #[serde(rename = "isClose", default)]
    pub is_close: bool,

    /// Last updated timestamp (Unix ms)
    #[serde(rename = "lastUpdated")]
    pub last_updated: Option<i64>,
}

impl Quote {
    /// Get the bid-ask spread
    pub fn spread(&self) -> Option<f64> {
        let best_bid = self.bids.first().map(|l| l.price);
        let best_ask = self.asks.first().map(|l| l.price);
        match (best_ask, best_bid) {
            (Some(ask), Some(bid)) => Some(ask - bid),
            _ => None,
        }
    }

    /// Get the mid price between best bid and ask
    pub fn mid_price(&self) -> Option<f64> {
        let best_bid = self.bids.first().map(|l| l.price);
        let best_ask = self.asks.first().map(|l| l.price);
        match (best_ask, best_bid) {
            (Some(ask), Some(bid)) => Some((ask + bid) / 2.0),
            _ => None,
        }
    }

    /// Check if quote has essential price data
    pub fn has_price_data(&self) -> bool {
        self.last_price.is_some() || self.close_price.is_some()
    }

    /// Check if the stock is at limit up
    pub fn is_at_limit_up(&self) -> bool {
        self.is_limit_up_price
    }

    /// Check if the stock is at limit down
    pub fn is_at_limit_down(&self) -> bool {
        self.is_limit_down_price
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quote_deserialization() {
        let json = r#"{
            "date": "2024-01-15",
            "type": "EQUITY",
            "exchange": "TWSE",
            "market": "TSE",
            "symbol": "2330",
            "name": "台積電",
            "previousClose": 580.0,
            "openPrice": 580.0,
            "openTime": 1705287000000,
            "highPrice": 585.0,
            "highTime": 1705290600000,
            "lowPrice": 578.0,
            "lowTime": 1705288800000,
            "closePrice": 583.0,
            "closeTime": 1705302000000,
            "lastPrice": 583.0,
            "lastSize": 1000,
            "avgPrice": 581.5,
            "change": 3.0,
            "changePercent": 0.52,
            "amplitude": 1.21,
            "bids": [
                {"price": 582.0, "size": 500},
                {"price": 581.0, "size": 300}
            ],
            "asks": [
                {"price": 583.0, "size": 200},
                {"price": 584.0, "size": 400}
            ],
            "total": {
                "tradeValue": 5815000000,
                "tradeVolume": 10000000,
                "transaction": 50000
            },
            "isLimitUpPrice": false,
            "isLimitDownPrice": false,
            "isTrial": false,
            "isOpen": true,
            "isClose": false,
            "lastUpdated": 1705302000000
        }"#;

        let quote: Quote = serde_json::from_str(json).unwrap();
        assert_eq!(quote.symbol, "2330");
        assert_eq!(quote.name.as_deref(), Some("台積電"));
        assert_eq!(quote.previous_close, Some(580.0));
        assert_eq!(quote.last_price, Some(583.0));
        assert_eq!(quote.bids.len(), 2);
        assert_eq!(quote.asks.len(), 2);
        assert_eq!(quote.bids[0].price, 582.0);
        assert_eq!(quote.asks[0].price, 583.0);
    }

    #[test]
    fn test_quote_spread() {
        let quote = Quote {
            bids: vec![PriceLevel { price: 100.0, size: 100 }],
            asks: vec![PriceLevel { price: 100.5, size: 100 }],
            ..Default::default()
        };
        assert_eq!(quote.spread(), Some(0.5));
    }

    #[test]
    fn test_quote_mid_price() {
        let quote = Quote {
            bids: vec![PriceLevel { price: 100.0, size: 100 }],
            asks: vec![PriceLevel { price: 101.0, size: 100 }],
            ..Default::default()
        };
        assert_eq!(quote.mid_price(), Some(100.5));
    }

    #[test]
    fn test_quote_minimal() {
        let json = r#"{"date": "2024-01-15", "symbol": "2330"}"#;
        let quote: Quote = serde_json::from_str(json).unwrap();
        assert_eq!(quote.symbol, "2330");
        assert!(quote.bids.is_empty());
        assert!(quote.asks.is_empty());
    }

    #[test]
    fn test_quote_limit_flags() {
        let json = r#"{
            "date": "2024-01-15",
            "symbol": "2330",
            "isLimitUpPrice": true,
            "isLimitDownPrice": false
        }"#;
        let quote: Quote = serde_json::from_str(json).unwrap();
        assert!(quote.is_at_limit_up());
        assert!(!quote.is_at_limit_down());
    }
}
