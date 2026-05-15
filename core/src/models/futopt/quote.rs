//! FutOpt quote data model - matches Fugle futopt/intraday/quote/{symbol} response

use serde::{Deserialize, Serialize};

/// Bid/Ask price level for FutOpt order book
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct FutOptPriceLevel {
    /// Price at this level
    pub price: f64,
    /// Size (volume) at this level
    pub size: i64,
}

/// Total trading statistics for FutOpt
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct FutOptTotalStats {
    /// Total trade volume
    #[serde(rename = "tradeVolume")]
    pub trade_volume: i64,
    /// Total volume matched at bid price
    #[serde(rename = "totalBidMatch")]
    pub total_bid_match: Option<i64>,
    /// Total volume matched at ask price
    #[serde(rename = "totalAskMatch")]
    pub total_ask_match: Option<i64>,
}

/// Last trade information for FutOpt
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct FutOptLastTrade {
    /// Trade price
    pub price: f64,
    /// Trade size
    pub size: i64,
    /// Trade timestamp (Unix milliseconds)
    pub time: i64,
}

/// Real-time FutOpt quote from Fugle API (futopt/intraday/quote/{symbol})
///
/// This matches the official SDK's RestFutOptIntradayQuoteResponse
///
/// # Example
///
/// ```rust
/// use marketdata_core::models::futopt::FutOptQuote;
///
/// let json = r#"{
///     "date": "2024-01-15",
///     "type": "FUTURE",
///     "exchange": "TAIFEX",
///     "symbol": "TXFC4",
///     "name": "臺股期貨",
///     "previousClose": 17500.0,
///     "openPrice": 17520.0,
///     "openTime": 1705287000000,
///     "highPrice": 17580.0,
///     "highTime": 1705290600000,
///     "lowPrice": 17480.0,
///     "lowTime": 1705288800000,
///     "closePrice": 17550.0,
///     "closeTime": 1705302000000,
///     "lastPrice": 17550.0,
///     "lastSize": 2,
///     "avgPrice": 17530.0,
///     "change": 50.0,
///     "changePercent": 0.29,
///     "amplitude": 0.57,
///     "bids": [{"price": 17549.0, "size": 50}],
///     "asks": [{"price": 17550.0, "size": 30}],
///     "total": {"tradeVolume": 50000, "totalBidMatch": 25000, "totalAskMatch": 25000},
///     "lastTrade": {"price": 17550.0, "size": 2, "time": 1705302000000},
///     "lastUpdated": 1705302000000
/// }"#;
///
/// let quote: FutOptQuote = serde_json::from_str(json).unwrap();
/// assert_eq!(quote.symbol, "TXFC4");
/// assert_eq!(quote.last_price, Some(17550.0));
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct FutOptQuote {
    // === Response metadata ===
    /// Trading date (YYYY-MM-DD)
    pub date: String,

    /// Contract type (FUTURE or OPTION)
    #[serde(rename = "type")]
    pub contract_type: Option<String>,

    /// Exchange code (TAIFEX)
    pub exchange: Option<String>,

    /// Contract symbol (e.g., "TXFC4", "TXO18000C4")
    pub symbol: String,

    /// Contract name
    pub name: Option<String>,

    // === Reference prices ===
    /// Previous close price
    #[serde(rename = "previousClose")]
    pub previous_close: Option<f64>,

    // === OHLC prices with timestamps ===
    /// Open price
    #[serde(rename = "openPrice")]
    pub open_price: Option<f64>,

    /// Open time (Unix milliseconds)
    #[serde(rename = "openTime")]
    pub open_time: Option<i64>,

    /// High price
    #[serde(rename = "highPrice")]
    pub high_price: Option<f64>,

    /// High time (Unix milliseconds)
    #[serde(rename = "highTime")]
    pub high_time: Option<i64>,

    /// Low price
    #[serde(rename = "lowPrice")]
    pub low_price: Option<f64>,

    /// Low time (Unix milliseconds)
    #[serde(rename = "lowTime")]
    pub low_time: Option<i64>,

    /// Close price
    #[serde(rename = "closePrice")]
    pub close_price: Option<f64>,

    /// Close time (Unix milliseconds)
    #[serde(rename = "closeTime")]
    pub close_time: Option<i64>,

    // === Current trading info ===
    /// Last traded price
    #[serde(rename = "lastPrice")]
    pub last_price: Option<f64>,

    /// Last traded size (number of contracts)
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

    /// Price amplitude (high - low) / reference price * 100
    pub amplitude: Option<f64>,

    // === Order book ===
    /// Bid price levels (best to worst)
    #[serde(default)]
    pub bids: Vec<FutOptPriceLevel>,

    /// Ask price levels (best to worst)
    #[serde(default)]
    pub asks: Vec<FutOptPriceLevel>,

    // === Aggregated stats ===
    /// Total trading statistics
    pub total: Option<FutOptTotalStats>,

    /// Last trade information
    #[serde(rename = "lastTrade")]
    pub last_trade: Option<FutOptLastTrade>,

    /// Last updated timestamp (Unix milliseconds)
    #[serde(rename = "lastUpdated")]
    pub last_updated: Option<i64>,
}

impl FutOptQuote {
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

    /// Get the total trade volume
    pub fn total_volume(&self) -> Option<i64> {
        self.total.as_ref().map(|t| t.trade_volume)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_futopt_quote_deserialization() {
        let json = r#"{
            "date": "2024-01-15",
            "type": "FUTURE",
            "exchange": "TAIFEX",
            "symbol": "TXFC4",
            "name": "臺股期貨",
            "previousClose": 17500.0,
            "openPrice": 17520.0,
            "openTime": 1705287000000,
            "highPrice": 17580.0,
            "highTime": 1705290600000,
            "lowPrice": 17480.0,
            "lowTime": 1705288800000,
            "closePrice": 17550.0,
            "closeTime": 1705302000000,
            "lastPrice": 17550.0,
            "lastSize": 2,
            "avgPrice": 17530.0,
            "change": 50.0,
            "changePercent": 0.29,
            "amplitude": 0.57,
            "bids": [
                {"price": 17549.0, "size": 50},
                {"price": 17548.0, "size": 30}
            ],
            "asks": [
                {"price": 17550.0, "size": 30},
                {"price": 17551.0, "size": 40}
            ],
            "total": {
                "tradeVolume": 50000,
                "totalBidMatch": 25000,
                "totalAskMatch": 25000
            },
            "lastTrade": {
                "price": 17550.0,
                "size": 2,
                "time": 1705302000000
            },
            "lastUpdated": 1705302000000
        }"#;

        let quote: FutOptQuote = serde_json::from_str(json).unwrap();
        assert_eq!(quote.symbol, "TXFC4");
        assert_eq!(quote.name.as_deref(), Some("臺股期貨"));
        assert_eq!(quote.contract_type.as_deref(), Some("FUTURE"));
        assert_eq!(quote.exchange.as_deref(), Some("TAIFEX"));
        assert_eq!(quote.last_price, Some(17550.0));
        assert_eq!(quote.previous_close, Some(17500.0));
        assert_eq!(quote.change, Some(50.0));
        assert_eq!(quote.bids.len(), 2);
        assert_eq!(quote.asks.len(), 2);
        assert_eq!(quote.bids[0].price, 17549.0);
        assert_eq!(quote.asks[0].price, 17550.0);
        assert_eq!(quote.total_volume(), Some(50000));
    }

    #[test]
    fn test_futopt_quote_spread() {
        let quote = FutOptQuote {
            bids: vec![FutOptPriceLevel {
                price: 17549.0,
                size: 50,
            }],
            asks: vec![FutOptPriceLevel {
                price: 17550.0,
                size: 30,
            }],
            ..Default::default()
        };
        assert_eq!(quote.spread(), Some(1.0));
    }

    #[test]
    fn test_futopt_quote_mid_price() {
        let quote = FutOptQuote {
            bids: vec![FutOptPriceLevel {
                price: 100.0,
                size: 10,
            }],
            asks: vec![FutOptPriceLevel {
                price: 102.0,
                size: 10,
            }],
            ..Default::default()
        };
        assert_eq!(quote.mid_price(), Some(101.0));
    }

    #[test]
    fn test_futopt_quote_minimal() {
        let json = r#"{"date": "2024-01-15", "symbol": "TXFC4"}"#;
        let quote: FutOptQuote = serde_json::from_str(json).unwrap();
        assert_eq!(quote.symbol, "TXFC4");
        assert!(quote.bids.is_empty());
        assert!(quote.asks.is_empty());
        assert!(!quote.has_price_data());
    }

    #[test]
    fn test_futopt_quote_has_price_data() {
        let mut quote = FutOptQuote::default();
        assert!(!quote.has_price_data());

        quote.last_price = Some(17550.0);
        assert!(quote.has_price_data());

        quote.last_price = None;
        quote.close_price = Some(17550.0);
        assert!(quote.has_price_data());
    }

    #[test]
    fn test_futopt_total_stats_deserialization() {
        let json = r#"{"tradeVolume": 50000, "totalBidMatch": 25000, "totalAskMatch": 25000}"#;
        let stats: FutOptTotalStats = serde_json::from_str(json).unwrap();
        assert_eq!(stats.trade_volume, 50000);
        assert_eq!(stats.total_bid_match, Some(25000));
        assert_eq!(stats.total_ask_match, Some(25000));
    }

    #[test]
    fn test_futopt_last_trade_deserialization() {
        let json = r#"{"price": 17550.0, "size": 2, "time": 1705302000000}"#;
        let trade: FutOptLastTrade = serde_json::from_str(json).unwrap();
        assert_eq!(trade.price, 17550.0);
        assert_eq!(trade.size, 2);
        assert_eq!(trade.time, 1705302000000);
    }
}
