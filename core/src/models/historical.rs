//! Historical data response models - matches Fugle historical endpoints
//!
//! Note: HistoricalCandle and HistoricalCandlesResponse are in candle.rs
//! This module contains StatsResponse for the historical/stats endpoint

use serde::{Deserialize, Serialize};

/// Historical statistics response from Fugle API (historical/stats/{symbol})
///
/// This matches the official SDK's RestStockHistoricalStatsResponse
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "python", pyo3::prelude::pyclass)]
#[cfg_attr(feature = "js", napi_derive::napi(object))]
pub struct StatsResponse {
    /// Trading date (YYYY-MM-DD)
    pub date: String,

    /// Security type (e.g., "EQUITY")
    #[serde(rename = "type")]
    pub data_type: String,

    /// Exchange code (e.g., "TWSE", "TPEx")
    pub exchange: String,

    /// Market (e.g., "TSE", "OTC")
    pub market: String,

    /// Stock symbol
    pub symbol: String,

    /// Stock name
    pub name: String,

    /// Opening price
    #[serde(rename = "openPrice")]
    pub open_price: f64,

    /// High price
    #[serde(rename = "highPrice")]
    pub high_price: f64,

    /// Low price
    #[serde(rename = "lowPrice")]
    pub low_price: f64,

    /// Closing price
    #[serde(rename = "closePrice")]
    pub close_price: f64,

    /// Price change
    pub change: f64,

    /// Price change percentage
    #[serde(rename = "changePercent")]
    pub change_percent: f64,

    /// Total trading volume
    #[serde(rename = "tradeVolume")]
    pub trade_volume: i64,

    /// Total trading value
    #[serde(rename = "tradeValue")]
    pub trade_value: f64,

    /// Previous close price
    #[serde(rename = "previousClose")]
    pub previous_close: f64,

    /// 52-week high price
    #[serde(rename = "week52High")]
    pub week52_high: f64,

    /// 52-week low price
    #[serde(rename = "week52Low")]
    pub week52_low: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stats_response_deserialization() {
        let json = r#"{
            "date": "2024-01-15",
            "type": "EQUITY",
            "exchange": "TWSE",
            "market": "TSE",
            "symbol": "2330",
            "name": "台積電",
            "openPrice": 580.0,
            "highPrice": 590.0,
            "lowPrice": 575.0,
            "closePrice": 588.0,
            "change": 8.0,
            "changePercent": 1.38,
            "tradeVolume": 50000000,
            "tradeValue": 29000000000,
            "previousClose": 580.0,
            "week52High": 650.0,
            "week52Low": 480.0
        }"#;

        let stats: StatsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(stats.symbol, "2330");
        assert_eq!(stats.date, "2024-01-15");
        assert_eq!(stats.close_price, 588.0);
        assert_eq!(stats.change, 8.0);
        assert_eq!(stats.change_percent, 1.38);
        assert_eq!(stats.week52_high, 650.0);
        assert_eq!(stats.week52_low, 480.0);
    }

    #[test]
    fn test_stats_response_serialization_roundtrip() {
        let stats = StatsResponse {
            date: "2024-01-15".to_string(),
            data_type: "EQUITY".to_string(),
            exchange: "TWSE".to_string(),
            market: "TSE".to_string(),
            symbol: "2330".to_string(),
            name: "台積電".to_string(),
            open_price: 580.0,
            high_price: 590.0,
            low_price: 575.0,
            close_price: 588.0,
            change: 8.0,
            change_percent: 1.38,
            trade_volume: 50000000,
            trade_value: 29000000000.0,
            previous_close: 580.0,
            week52_high: 650.0,
            week52_low: 480.0,
        };

        let json = serde_json::to_string(&stats).unwrap();
        let deserialized: StatsResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(stats, deserialized);
    }
}
