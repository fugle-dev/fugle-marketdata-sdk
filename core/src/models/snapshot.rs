//! Snapshot data models for market-wide quotes, movers, actives, and the
//! index heatmap
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

/// Response for snapshot heatmap endpoint
///
/// The constituents of an index (`symbol` is an index code such as
/// `IX0001`) with their change, the data behind a market heatmap. `time` is
/// set when the snapshot is an intraday one, `period` when the change is
/// over a period (`1w`, `1m`, ...) instead of the day.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "python", pyo3::prelude::pyclass)]
#[cfg_attr(feature = "js", napi_derive::napi(object))]
pub struct SnapshotHeatmapResponse {
    /// Trading date (YYYY-MM-DD)
    pub date: String,

    /// Snapshot time (HHmmss); absent for a period snapshot
    pub time: Option<String>,

    /// Change period ("1w", "1m", "3m", "6m", "1y", "ytd"); absent for an intraday snapshot
    pub period: Option<String>,

    /// Index code the heatmap is of (e.g., "IX0001")
    pub symbol: String,

    /// The index itself, its sub-indices and its constituent stocks
    pub data: Vec<SnapshotHeatmapData>,
}

/// One index or stock in a heatmap response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "python", pyo3::prelude::pyclass)]
#[cfg_attr(feature = "js", napi_derive::napi(object))]
pub struct SnapshotHeatmapData {
    /// Trading date (YYYY-MM-DD); absent in a period snapshot
    pub date: Option<String>,

    /// Security type ("INDEX" or "EQUITY")
    #[serde(rename = "type")]
    pub data_type: Option<String>,

    /// Exchange code (e.g., "TWSE")
    pub exchange: Option<String>,

    /// Index code or stock symbol
    pub symbol: String,

    /// Name
    pub name: Option<String>,

    /// Opening price
    #[serde(rename = "openPrice")]
    pub open_price: Option<f64>,

    /// Highest price
    #[serde(rename = "highPrice")]
    pub high_price: Option<f64>,

    /// Lowest price
    #[serde(rename = "lowPrice")]
    pub low_price: Option<f64>,

    /// Closing/last price
    #[serde(rename = "closePrice")]
    pub close_price: Option<f64>,

    /// Price change over the day or the period
    pub change: Option<f64>,

    /// Percentage change over the day or the period
    #[serde(rename = "changePercent")]
    pub change_percent: Option<f64>,

    /// Previous close
    #[serde(rename = "previousClose")]
    pub previous_close: Option<f64>,

    /// Trading volume (number of shares)
    #[serde(rename = "tradeVolume")]
    pub trade_volume: Option<i64>,

    /// Trading value (total value traded)
    #[serde(rename = "tradeValue")]
    pub trade_value: Option<f64>,

    /// Share of the index's trading value, in percent (stocks only)
    #[serde(rename = "tradeValueWeight")]
    pub trade_value_weight: Option<f64>,

    /// Share of the index's market value, in percent (stocks only)
    #[serde(rename = "marketValueWeight")]
    pub market_value_weight: Option<f64>,

    /// Industry code (stocks only)
    pub industry: Option<String>,

    /// Last updated timestamp (Unix microseconds)
    #[serde(rename = "lastUpdated")]
    pub last_updated: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_heatmap_response_deserialization() {
        // Two rows from a prod `heatmap/IX0001` answer (2026-09-18): the
        // index itself and one constituent, which carries the weights.
        let json = r#"{
            "date": "2026-09-18",
            "time": "133500",
            "symbol": "IX0001",
            "data": [
                {
                    "date": "2026-09-18",
                    "type": "INDEX",
                    "symbol": "IX0001",
                    "name": "發行量加權股價指數",
                    "openPrice": 46449.56,
                    "highPrice": 47180.75,
                    "lowPrice": 46449.56,
                    "closePrice": 47180.75,
                    "change": 892.75,
                    "changePercent": 1.93,
                    "previousClose": 46288,
                    "tradeVolume": 11751121,
                    "tradeValue": 1075065501380,
                    "lastUpdated": 1789709400000000
                },
                {
                    "date": "2026-09-18",
                    "type": "EQUITY",
                    "symbol": "2330",
                    "name": "台積電",
                    "openPrice": 2460,
                    "highPrice": 2460,
                    "lowPrice": 2435,
                    "closePrice": 2460,
                    "change": 35,
                    "changePercent": 1.44,
                    "previousClose": 2425,
                    "tradeVolume": 35250,
                    "tradeValue": 86542720000,
                    "tradeValueWeight": 8.049995082988904,
                    "marketValueWeight": 41.4777,
                    "industry": "24",
                    "lastUpdated": 1789709400000000
                }
            ]
        }"#;

        let response: SnapshotHeatmapResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.symbol, "IX0001");
        assert_eq!(response.time.as_deref(), Some("133500"));
        assert_eq!(response.period, None);
        assert_eq!(response.data.len(), 2);

        let index = &response.data[0];
        assert_eq!(index.data_type.as_deref(), Some("INDEX"));
        assert_eq!(index.market_value_weight, None);

        let stock = &response.data[1];
        assert_eq!(stock.symbol, "2330");
        assert_eq!(stock.industry.as_deref(), Some("24"));
        assert_eq!(stock.market_value_weight, Some(41.4777));
        assert_eq!(stock.last_updated, Some(1789709400000000));
    }

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
