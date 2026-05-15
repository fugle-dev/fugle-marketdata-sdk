//! Trade data model - matches Fugle intraday/trades/{symbol} response

use serde::{Deserialize, Serialize};

/// A single trade execution
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "python", pyo3::prelude::pyclass)]
#[cfg_attr(feature = "js", napi_derive::napi(object))]
pub struct Trade {
    /// Best bid price at trade time
    pub bid: Option<f64>,

    /// Best ask price at trade time
    pub ask: Option<f64>,

    /// Trade price
    pub price: f64,

    /// Trade size (volume)
    pub size: i64,

    /// Trade timestamp (Unix milliseconds)
    pub time: i64,

    /// Server-assigned monotonic sequence number (dedup / pagination anchor)
    #[serde(default)]
    pub serial: Option<i64>,

    /// Cumulative volume at this trade (session total so far)
    #[serde(default)]
    pub volume: Option<i64>,
}

impl Trade {
    /// Infer trade direction based on price vs bid/ask
    /// Returns "B" for buy (at ask), "S" for sell (at bid), "N" for neutral
    pub fn infer_direction(&self) -> &'static str {
        match (self.bid, self.ask) {
            (Some(bid), _) if (self.price - bid).abs() < 0.0001 => "S",
            (_, Some(ask)) if (self.price - ask).abs() < 0.0001 => "B",
            _ => "N",
        }
    }

    /// Check if trade was at ask (buyer initiated)
    pub fn is_buyer_initiated(&self) -> bool {
        self.infer_direction() == "B"
    }

    /// Check if trade was at bid (seller initiated)
    pub fn is_seller_initiated(&self) -> bool {
        self.infer_direction() == "S"
    }
}

/// Trades response from Fugle API (intraday/trades/{symbol})
///
/// This matches the official SDK's RestStockIntradayTradesResponse
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "python", pyo3::prelude::pyclass)]
#[cfg_attr(feature = "js", napi_derive::napi(object))]
pub struct TradesResponse {
    /// Trading date (YYYY-MM-DD)
    pub date: String,

    /// Security type (e.g., "EQUITY", "ODDLOT")
    #[serde(rename = "type")]
    pub data_type: Option<String>,

    /// Exchange code (e.g., "TWSE", "TPEx")
    pub exchange: Option<String>,

    /// Market (e.g., "TSE", "OTC")
    pub market: Option<String>,

    /// Stock symbol
    pub symbol: String,

    /// List of trades
    #[serde(default)]
    pub data: Vec<Trade>,
}

impl TradesResponse {
    /// Get total volume from all trades
    pub fn total_volume(&self) -> i64 {
        self.data.iter().map(|t| t.size).sum()
    }

    /// Get total value from all trades
    pub fn total_value(&self) -> f64 {
        self.data.iter().map(|t| t.price * t.size as f64).sum()
    }

    /// Get VWAP (volume-weighted average price)
    pub fn vwap(&self) -> Option<f64> {
        let total_volume = self.total_volume();
        if total_volume == 0 {
            return None;
        }
        Some(self.total_value() / total_volume as f64)
    }

    /// Get trades in a time range (Unix ms)
    pub fn trades_in_range(&self, start: i64, end: i64) -> Vec<&Trade> {
        self.data
            .iter()
            .filter(|t| t.time >= start && t.time <= end)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trade_deserialization() {
        let json = r#"{
            "bid": 582.0,
            "ask": 583.0,
            "price": 583.0,
            "size": 1000,
            "time": 1705287000000
        }"#;
        let trade: Trade = serde_json::from_str(json).unwrap();
        assert_eq!(trade.price, 583.0);
        assert_eq!(trade.size, 1000);
        assert!(trade.is_buyer_initiated());
    }

    #[test]
    fn test_trade_direction() {
        // Trade at ask = buyer initiated
        let buy_trade = Trade {
            bid: Some(100.0),
            ask: Some(100.5),
            price: 100.5,
            size: 100,
            time: 0,
            ..Default::default()
        };
        assert_eq!(buy_trade.infer_direction(), "B");

        // Trade at bid = seller initiated
        let sell_trade = Trade {
            bid: Some(100.0),
            ask: Some(100.5),
            price: 100.0,
            size: 100,
            time: 0,
            ..Default::default()
        };
        assert_eq!(sell_trade.infer_direction(), "S");

        // Trade in between = neutral
        let neutral_trade = Trade {
            bid: Some(100.0),
            ask: Some(101.0),
            price: 100.5,
            size: 100,
            time: 0,
            ..Default::default()
        };
        assert_eq!(neutral_trade.infer_direction(), "N");
    }

    #[test]
    fn test_trades_response_deserialization() {
        let json = r#"{
            "date": "2024-01-15",
            "type": "EQUITY",
            "exchange": "TWSE",
            "market": "TSE",
            "symbol": "2330",
            "data": [
                {"bid": 582.0, "ask": 583.0, "price": 583.0, "size": 1000, "time": 1705287000000},
                {"bid": 582.0, "ask": 583.0, "price": 582.0, "size": 500, "time": 1705287001000}
            ]
        }"#;

        let response: TradesResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.symbol, "2330");
        assert_eq!(response.data.len(), 2);
        assert_eq!(response.total_volume(), 1500);
    }

    #[test]
    fn test_trades_vwap() {
        let response = TradesResponse {
            data: vec![
                Trade { price: 100.0, size: 1000, ..Default::default() },
                Trade { price: 101.0, size: 1000, ..Default::default() },
            ],
            ..Default::default()
        };
        // VWAP = (100*1000 + 101*1000) / 2000 = 100.5
        assert_eq!(response.vwap(), Some(100.5));
    }

    #[test]
    fn test_trades_empty() {
        let response = TradesResponse::default();
        assert_eq!(response.total_volume(), 0);
        assert_eq!(response.vwap(), None);
    }
}
