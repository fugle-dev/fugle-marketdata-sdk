//! Snapshot data models for market-wide quotes, movers, and actives
//!
//! These models match the official Fugle marketdata SDK response structures
//! for snapshot endpoints.

use serde::{Deserialize, Serialize};

/// Response for snapshot quotes endpoint
///
/// Contains market-wide quote data for all stocks in a market.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "python", pyo3::prelude::pyclass)]
#[cfg_attr(feature = "js", napi_derive::napi(object))]
pub struct SnapshotQuotesResponse {
    /// Trading date (YYYY-MM-DD)
    pub date: String,

    /// Time of snapshot (HH:MM:SS)
    pub time: String,

    /// Market code (e.g., "TSE", "OTC")
    pub market: String,

    /// Array of quote data for each stock
    pub data: Vec<SnapshotQuote>,
}

/// Individual stock quote in a snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "python", pyo3::prelude::pyclass)]
#[cfg_attr(feature = "js", napi_derive::napi(object))]
pub struct SnapshotQuote {
    /// Security type (e.g., "EQUITY")
    #[serde(rename = "type")]
    pub data_type: Option<String>,

    /// Stock symbol (e.g., "2330")
    pub symbol: String,

    /// Stock name
    pub name: Option<String>,

    /// Opening price
    #[serde(rename = "openPrice")]
    pub open_price: Option<f64>,

    /// Highest price of the day
    #[serde(rename = "highPrice")]
    pub high_price: Option<f64>,

    /// Lowest price of the day
    #[serde(rename = "lowPrice")]
    pub low_price: Option<f64>,

    /// Closing/last price
    #[serde(rename = "closePrice")]
    pub close_price: Option<f64>,

    /// Price change from previous close
    pub change: Option<f64>,

    /// Percentage change from previous close
    #[serde(rename = "changePercent")]
    pub change_percent: Option<f64>,

    /// Trading volume (number of shares)
    #[serde(rename = "tradeVolume")]
    pub trade_volume: Option<i64>,

    /// Trading value (total value traded)
    #[serde(rename = "tradeValue")]
    pub trade_value: Option<f64>,

    /// Last updated timestamp (Unix milliseconds)
    #[serde(rename = "lastUpdated")]
    pub last_updated: Option<i64>,
}

/// Response for movers endpoint
///
/// Contains top gainers or losers in a market.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "python", pyo3::prelude::pyclass)]
#[cfg_attr(feature = "js", napi_derive::napi(object))]
pub struct MoversResponse {
    /// Trading date (YYYY-MM-DD)
    pub date: String,

    /// Time of snapshot (HH:MM:SS)
    pub time: String,

    /// Market code (e.g., "TSE", "OTC")
    pub market: String,

    /// Array of mover data
    pub data: Vec<Mover>,
}

/// Individual stock in movers response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "python", pyo3::prelude::pyclass)]
#[cfg_attr(feature = "js", napi_derive::napi(object))]
pub struct Mover {
    /// Security type (e.g., "EQUITY")
    #[serde(rename = "type")]
    pub data_type: Option<String>,

    /// Stock symbol (e.g., "2330")
    pub symbol: String,

    /// Stock name
    pub name: Option<String>,

    /// Opening price
    #[serde(rename = "openPrice")]
    pub open_price: Option<f64>,

    /// Highest price of the day
    #[serde(rename = "highPrice")]
    pub high_price: Option<f64>,

    /// Lowest price of the day
    #[serde(rename = "lowPrice")]
    pub low_price: Option<f64>,

    /// Closing/last price
    #[serde(rename = "closePrice")]
    pub close_price: Option<f64>,

    /// Price change from previous close
    pub change: Option<f64>,

    /// Percentage change from previous close
    #[serde(rename = "changePercent")]
    pub change_percent: Option<f64>,

    /// Trading volume (number of shares)
    #[serde(rename = "tradeVolume")]
    pub trade_volume: Option<i64>,

    /// Trading value (total value traded)
    #[serde(rename = "tradeValue")]
    pub trade_value: Option<f64>,

    /// Last updated timestamp (Unix milliseconds)
    #[serde(rename = "lastUpdated")]
    pub last_updated: Option<i64>,
}

/// Response for actives endpoint
///
/// Contains most actively traded stocks in a market.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "python", pyo3::prelude::pyclass)]
#[cfg_attr(feature = "js", napi_derive::napi(object))]
pub struct ActivesResponse {
    /// Trading date (YYYY-MM-DD)
    pub date: String,

    /// Time of snapshot (HH:MM:SS)
    pub time: String,

    /// Market code (e.g., "TSE", "OTC")
    pub market: String,

    /// Array of active stock data
    pub data: Vec<Active>,
}

/// Individual stock in actives response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "python", pyo3::prelude::pyclass)]
#[cfg_attr(feature = "js", napi_derive::napi(object))]
pub struct Active {
    /// Security type (e.g., "EQUITY")
    #[serde(rename = "type")]
    pub data_type: Option<String>,

    /// Stock symbol (e.g., "2330")
    pub symbol: String,

    /// Stock name
    pub name: Option<String>,

    /// Opening price
    #[serde(rename = "openPrice")]
    pub open_price: Option<f64>,

    /// Highest price of the day
    #[serde(rename = "highPrice")]
    pub high_price: Option<f64>,

    /// Lowest price of the day
    #[serde(rename = "lowPrice")]
    pub low_price: Option<f64>,

    /// Closing/last price
    #[serde(rename = "closePrice")]
    pub close_price: Option<f64>,

    /// Price change from previous close
    pub change: Option<f64>,

    /// Percentage change from previous close
    #[serde(rename = "changePercent")]
    pub change_percent: Option<f64>,

    /// Trading volume (number of shares)
    #[serde(rename = "tradeVolume")]
    pub trade_volume: Option<i64>,

    /// Trading value (total value traded)
    #[serde(rename = "tradeValue")]
    pub trade_value: Option<f64>,

    /// Last updated timestamp (Unix milliseconds)
    #[serde(rename = "lastUpdated")]
    pub last_updated: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_quotes_response_deserialization() {
        let json = r#"{
            "date": "2024-01-15",
            "time": "13:30:00",
            "market": "TSE",
            "data": [
                {
                    "type": "EQUITY",
                    "symbol": "2330",
                    "name": "TSMC",
                    "openPrice": 580.0,
                    "highPrice": 585.0,
                    "lowPrice": 578.0,
                    "closePrice": 583.0,
                    "change": 3.0,
                    "changePercent": 0.52,
                    "tradeVolume": 10000000,
                    "tradeValue": 5815000000,
                    "lastUpdated": 1705302000000
                }
            ]
        }"#;

        let response: SnapshotQuotesResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.date, "2024-01-15");
        assert_eq!(response.time, "13:30:00");
        assert_eq!(response.market, "TSE");
        assert_eq!(response.data.len(), 1);

        let quote = &response.data[0];
        assert_eq!(quote.symbol, "2330");
        assert_eq!(quote.data_type, Some("EQUITY".to_string()));
        assert_eq!(quote.close_price, Some(583.0));
        assert_eq!(quote.change, Some(3.0));
    }

    #[test]
    fn test_movers_response_deserialization() {
        let json = r#"{
            "date": "2024-01-15",
            "time": "13:30:00",
            "market": "TSE",
            "data": [
                {
                    "type": "EQUITY",
                    "symbol": "3008",
                    "name": "LARGAN",
                    "openPrice": 2500.0,
                    "highPrice": 2600.0,
                    "lowPrice": 2480.0,
                    "closePrice": 2590.0,
                    "change": 90.0,
                    "changePercent": 3.6,
                    "tradeVolume": 500000,
                    "tradeValue": 1295000000,
                    "lastUpdated": 1705302000000
                }
            ]
        }"#;

        let response: MoversResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.market, "TSE");
        assert_eq!(response.data.len(), 1);

        let mover = &response.data[0];
        assert_eq!(mover.symbol, "3008");
        assert_eq!(mover.change_percent, Some(3.6));
    }

    #[test]
    fn test_actives_response_deserialization() {
        let json = r#"{
            "date": "2024-01-15",
            "time": "13:30:00",
            "market": "TSE",
            "data": [
                {
                    "type": "EQUITY",
                    "symbol": "2330",
                    "name": "TSMC",
                    "openPrice": 580.0,
                    "highPrice": 585.0,
                    "lowPrice": 578.0,
                    "closePrice": 583.0,
                    "change": 3.0,
                    "changePercent": 0.52,
                    "tradeVolume": 50000000,
                    "tradeValue": 29150000000,
                    "lastUpdated": 1705302000000
                }
            ]
        }"#;

        let response: ActivesResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.market, "TSE");
        assert_eq!(response.data.len(), 1);

        let active = &response.data[0];
        assert_eq!(active.symbol, "2330");
        assert_eq!(active.trade_volume, Some(50000000));
    }

    #[test]
    fn test_minimal_snapshot_quote() {
        let json = r#"{
            "symbol": "2330"
        }"#;

        let quote: SnapshotQuote = serde_json::from_str(json).unwrap();
        assert_eq!(quote.symbol, "2330");
        assert!(quote.name.is_none());
        assert!(quote.close_price.is_none());
    }
}
