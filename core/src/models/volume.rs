//! Volume data model - matches Fugle intraday/volumes/{symbol} response

use serde::{Deserialize, Serialize};

/// Volume at a specific price level
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "python", pyo3::prelude::pyclass)]
#[cfg_attr(feature = "js", napi_derive::napi(object))]
pub struct VolumeAtPrice {
    /// Price level
    pub price: f64,

    /// Total volume at this price
    pub volume: i64,

    /// Volume traded at bid
    #[serde(rename = "volumeAtBid")]
    pub volume_at_bid: Option<i64>,

    /// Volume traded at ask
    #[serde(rename = "volumeAtAsk")]
    pub volume_at_ask: Option<i64>,
}

impl VolumeAtPrice {
    /// Get buy/sell imbalance at this price level
    /// Positive = more buying, Negative = more selling
    pub fn imbalance(&self) -> Option<i64> {
        match (self.volume_at_ask, self.volume_at_bid) {
            (Some(ask), Some(bid)) => Some(ask - bid),
            _ => None,
        }
    }

    /// Get percentage of volume that was buying
    pub fn buy_ratio(&self) -> Option<f64> {
        match (self.volume_at_ask, self.volume_at_bid) {
            (Some(ask), Some(bid)) if ask + bid > 0 => {
                Some(ask as f64 / (ask + bid) as f64)
            }
            _ => None,
        }
    }
}

/// Volumes response from Fugle API (intraday/volumes/{symbol})
///
/// This matches the official SDK's RestStockIntradayVolumesResponse
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "python", pyo3::prelude::pyclass)]
#[cfg_attr(feature = "js", napi_derive::napi(object))]
pub struct VolumesResponse {
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

    /// Volume data at each price level
    #[serde(default)]
    pub data: Vec<VolumeAtPrice>,
}

impl VolumesResponse {
    /// Get total volume across all price levels
    pub fn total_volume(&self) -> i64 {
        self.data.iter().map(|v| v.volume).sum()
    }

    /// Get total volume at bid
    pub fn total_volume_at_bid(&self) -> i64 {
        self.data.iter().filter_map(|v| v.volume_at_bid).sum()
    }

    /// Get total volume at ask
    pub fn total_volume_at_ask(&self) -> i64 {
        self.data.iter().filter_map(|v| v.volume_at_ask).sum()
    }

    /// Get price level with highest volume
    pub fn price_with_max_volume(&self) -> Option<f64> {
        self.data
            .iter()
            .max_by_key(|v| v.volume)
            .map(|v| v.price)
    }

    /// Get VWAP from volume profile
    pub fn vwap(&self) -> Option<f64> {
        let total_vol = self.total_volume();
        if total_vol == 0 {
            return None;
        }
        let total_value: f64 = self.data
            .iter()
            .map(|v| v.price * v.volume as f64)
            .sum();
        Some(total_value / total_vol as f64)
    }

    /// Get overall buy/sell imbalance
    pub fn net_imbalance(&self) -> i64 {
        self.total_volume_at_ask() - self.total_volume_at_bid()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_volume_at_price_deserialization() {
        let json = r#"{
            "price": 583.0,
            "volume": 10000,
            "volumeAtBid": 4000,
            "volumeAtAsk": 6000
        }"#;
        let vol: VolumeAtPrice = serde_json::from_str(json).unwrap();
        assert_eq!(vol.price, 583.0);
        assert_eq!(vol.volume, 10000);
        assert_eq!(vol.imbalance(), Some(2000)); // 6000 - 4000
        assert_eq!(vol.buy_ratio(), Some(0.6));  // 6000 / 10000
    }

    #[test]
    fn test_volumes_response_deserialization() {
        let json = r#"{
            "date": "2024-01-15",
            "type": "EQUITY",
            "exchange": "TWSE",
            "market": "TSE",
            "symbol": "2330",
            "data": [
                {"price": 580.0, "volume": 5000, "volumeAtBid": 3000, "volumeAtAsk": 2000},
                {"price": 583.0, "volume": 10000, "volumeAtBid": 4000, "volumeAtAsk": 6000},
                {"price": 585.0, "volume": 3000, "volumeAtBid": 1000, "volumeAtAsk": 2000}
            ]
        }"#;

        let response: VolumesResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.symbol, "2330");
        assert_eq!(response.data.len(), 3);
        assert_eq!(response.total_volume(), 18000);
        assert_eq!(response.total_volume_at_bid(), 8000);
        assert_eq!(response.total_volume_at_ask(), 10000);
        assert_eq!(response.price_with_max_volume(), Some(583.0));
        assert_eq!(response.net_imbalance(), 2000); // 10000 - 8000
    }

    #[test]
    fn test_volumes_vwap() {
        let response = VolumesResponse {
            data: vec![
                VolumeAtPrice { price: 100.0, volume: 1000, ..Default::default() },
                VolumeAtPrice { price: 101.0, volume: 1000, ..Default::default() },
            ],
            ..Default::default()
        };
        // VWAP = (100*1000 + 101*1000) / 2000 = 100.5
        assert_eq!(response.vwap(), Some(100.5));
    }
}
