//! FutOpt historical data models - matches Fugle futopt/historical/* responses

use serde::{Deserialize, Serialize};

/// FutOpt Historical Candles response from Fugle API (futopt/historical/candles/{symbol})
///
/// This matches the official SDK's RestFutOptHistoricalCandlesResponse
///
/// # Example
///
/// ```rust
/// use marketdata_core::models::futopt::FutOptHistoricalCandlesResponse;
///
/// let json = r#"{
///     "symbol": "TXFC4",
///     "type": "FUTURE",
///     "exchange": "TAIFEX",
///     "timeframe": "D",
///     "data": [
///         {"date": "2024-01-15", "open": 17500.0, "high": 17580.0, "low": 17480.0, "close": 17550.0, "volume": 50000}
///     ]
/// }"#;
///
/// let response: FutOptHistoricalCandlesResponse = serde_json::from_str(json).unwrap();
/// assert_eq!(response.symbol, "TXFC4");
/// assert_eq!(response.candles.len(), 1);
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "python", pyo3::prelude::pyclass)]
#[cfg_attr(feature = "js", napi_derive::napi(object))]
pub struct FutOptHistoricalCandlesResponse {
    /// Contract symbol (e.g., "TXFC4")
    pub symbol: String,

    /// Contract type (e.g., "FUTURE", "OPTION")
    #[serde(rename = "type")]
    pub data_type: Option<String>,

    /// Exchange code (e.g., "TAIFEX")
    pub exchange: Option<String>,

    /// Timeframe (e.g., "D", "W", "M", "1", "5", etc.)
    pub timeframe: Option<String>,

    /// Candle data
    #[serde(default, rename = "data")]
    pub candles: Vec<FutOptHistoricalCandle>,
}

impl FutOptHistoricalCandlesResponse {
    /// Get the highest high in the series
    pub fn highest_high(&self) -> Option<f64> {
        self.candles.iter().map(|c| c.high).fold(None, |acc, h| {
            Some(acc.map_or(h, |a: f64| a.max(h)))
        })
    }

    /// Get the lowest low in the series
    pub fn lowest_low(&self) -> Option<f64> {
        self.candles.iter().map(|c| c.low).fold(None, |acc, l| {
            Some(acc.map_or(l, |a: f64| a.min(l)))
        })
    }

    /// Get total volume over the series
    pub fn total_volume(&self) -> u64 {
        self.candles.iter().map(|c| c.volume).sum()
    }
}

/// A single historical candlestick bar for FutOpt
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "python", pyo3::prelude::pyclass)]
#[cfg_attr(feature = "js", napi_derive::napi(object))]
pub struct FutOptHistoricalCandle {
    /// Date (YYYY-MM-DD)
    pub date: String,

    /// Open price
    pub open: f64,

    /// High price
    pub high: f64,

    /// Low price
    pub low: f64,

    /// Close price
    pub close: f64,

    /// Volume (number of contracts)
    pub volume: u64,

    /// Open interest (total outstanding contracts)
    #[serde(rename = "openInterest")]
    pub open_interest: Option<u64>,

    /// Price change from previous close
    pub change: Option<f64>,

    /// Percentage change from previous close
    #[serde(rename = "changePercent")]
    pub change_percent: Option<f64>,
}

impl FutOptHistoricalCandle {
    /// Check if bullish (close > open)
    pub fn is_bullish(&self) -> bool {
        self.close > self.open
    }

    /// Check if bearish (close < open)
    pub fn is_bearish(&self) -> bool {
        self.close < self.open
    }

    /// Get the candle body size
    pub fn body(&self) -> f64 {
        (self.close - self.open).abs()
    }

    /// Get the candle range (high - low)
    pub fn range(&self) -> f64 {
        self.high - self.low
    }
}

/// FutOpt Daily response from Fugle API (futopt/historical/daily/{symbol})
///
/// This matches the official SDK's RestFutOptHistoricalDailyResponse
///
/// # Example
///
/// ```rust
/// use marketdata_core::models::futopt::FutOptDailyResponse;
///
/// let json = r#"{
///     "symbol": "TXFC4",
///     "type": "FUTURE",
///     "exchange": "TAIFEX",
///     "data": [
///         {"date": "2024-01-15", "open": 17500.0, "high": 17580.0, "low": 17480.0, "close": 17550.0, "volume": 50000, "settlementPrice": 17545.0}
///     ]
/// }"#;
///
/// let response: FutOptDailyResponse = serde_json::from_str(json).unwrap();
/// assert_eq!(response.symbol, "TXFC4");
/// assert_eq!(response.data.len(), 1);
/// assert_eq!(response.data[0].settlement_price, Some(17545.0));
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "python", pyo3::prelude::pyclass)]
#[cfg_attr(feature = "js", napi_derive::napi(object))]
pub struct FutOptDailyResponse {
    /// Contract symbol (e.g., "TXFC4")
    pub symbol: String,

    /// Contract type (e.g., "FUTURE", "OPTION")
    #[serde(rename = "type")]
    pub data_type: Option<String>,

    /// Exchange code (e.g., "TAIFEX")
    pub exchange: Option<String>,

    /// Daily data
    #[serde(default)]
    pub data: Vec<FutOptDailyData>,
}

impl FutOptDailyResponse {
    /// Get the highest high in the series
    pub fn highest_high(&self) -> Option<f64> {
        self.data.iter().map(|c| c.high).fold(None, |acc, h| {
            Some(acc.map_or(h, |a: f64| a.max(h)))
        })
    }

    /// Get the lowest low in the series
    pub fn lowest_low(&self) -> Option<f64> {
        self.data.iter().map(|c| c.low).fold(None, |acc, l| {
            Some(acc.map_or(l, |a: f64| a.min(l)))
        })
    }

    /// Get total volume over the series
    pub fn total_volume(&self) -> u64 {
        self.data.iter().map(|c| c.volume).sum()
    }
}

/// A single daily data point for FutOpt
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "python", pyo3::prelude::pyclass)]
#[cfg_attr(feature = "js", napi_derive::napi(object))]
pub struct FutOptDailyData {
    /// Date (YYYY-MM-DD)
    pub date: String,

    /// Open price
    pub open: f64,

    /// High price
    pub high: f64,

    /// Low price
    pub low: f64,

    /// Close price
    pub close: f64,

    /// Volume (number of contracts)
    pub volume: u64,

    /// Open interest (total outstanding contracts)
    #[serde(rename = "openInterest")]
    pub open_interest: Option<u64>,

    /// Settlement price (official closing price for margin calculation)
    #[serde(rename = "settlementPrice")]
    pub settlement_price: Option<f64>,
}

impl FutOptDailyData {
    /// Check if bullish (close > open)
    pub fn is_bullish(&self) -> bool {
        self.close > self.open
    }

    /// Check if bearish (close < open)
    pub fn is_bearish(&self) -> bool {
        self.close < self.open
    }

    /// Get the daily range (high - low)
    pub fn range(&self) -> f64 {
        self.high - self.low
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_futopt_historical_candles_response() {
        let json = r#"{
            "symbol": "TXFC4",
            "type": "FUTURE",
            "exchange": "TAIFEX",
            "timeframe": "D",
            "data": [
                {"date": "2024-01-12", "open": 17400.0, "high": 17500.0, "low": 17380.0, "close": 17480.0, "volume": 45000, "openInterest": 120000},
                {"date": "2024-01-15", "open": 17500.0, "high": 17580.0, "low": 17480.0, "close": 17550.0, "volume": 50000, "openInterest": 121000, "change": 70.0, "changePercent": 0.4}
            ]
        }"#;

        let response: FutOptHistoricalCandlesResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.symbol, "TXFC4");
        assert_eq!(response.data_type.as_deref(), Some("FUTURE"));
        assert_eq!(response.exchange.as_deref(), Some("TAIFEX"));
        assert_eq!(response.timeframe.as_deref(), Some("D"));
        assert_eq!(response.candles.len(), 2);
        assert_eq!(response.highest_high(), Some(17580.0));
        assert_eq!(response.lowest_low(), Some(17380.0));
        assert_eq!(response.total_volume(), 95000);
    }

    #[test]
    fn test_futopt_historical_candle_methods() {
        let candle = FutOptHistoricalCandle {
            date: "2024-01-15".to_string(),
            open: 17500.0,
            high: 17580.0,
            low: 17480.0,
            close: 17550.0,
            volume: 50000,
            open_interest: Some(121000),
            change: Some(70.0),
            change_percent: Some(0.4),
        };

        assert!(candle.is_bullish());
        assert!(!candle.is_bearish());
        assert_eq!(candle.body(), 50.0);
        assert_eq!(candle.range(), 100.0);
    }

    #[test]
    fn test_futopt_daily_response() {
        let json = r#"{
            "symbol": "TXFC4",
            "type": "FUTURE",
            "exchange": "TAIFEX",
            "data": [
                {"date": "2024-01-12", "open": 17400.0, "high": 17500.0, "low": 17380.0, "close": 17480.0, "volume": 45000, "openInterest": 120000, "settlementPrice": 17475.0},
                {"date": "2024-01-15", "open": 17500.0, "high": 17580.0, "low": 17480.0, "close": 17550.0, "volume": 50000, "openInterest": 121000, "settlementPrice": 17545.0}
            ]
        }"#;

        let response: FutOptDailyResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.symbol, "TXFC4");
        assert_eq!(response.data_type.as_deref(), Some("FUTURE"));
        assert_eq!(response.data.len(), 2);
        assert_eq!(response.data[0].settlement_price, Some(17475.0));
        assert_eq!(response.data[1].settlement_price, Some(17545.0));
        assert_eq!(response.highest_high(), Some(17580.0));
        assert_eq!(response.lowest_low(), Some(17380.0));
        assert_eq!(response.total_volume(), 95000);
    }

    #[test]
    fn test_futopt_daily_data_methods() {
        let data = FutOptDailyData {
            date: "2024-01-15".to_string(),
            open: 17500.0,
            high: 17580.0,
            low: 17480.0,
            close: 17550.0,
            volume: 50000,
            open_interest: Some(121000),
            settlement_price: Some(17545.0),
        };

        assert!(data.is_bullish());
        assert!(!data.is_bearish());
        assert_eq!(data.range(), 100.0);
    }

    #[test]
    fn test_futopt_historical_minimal() {
        let json = r#"{"symbol": "TXFC4", "data": []}"#;
        let response: FutOptHistoricalCandlesResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.symbol, "TXFC4");
        assert!(response.candles.is_empty());
        assert_eq!(response.highest_high(), None);
        assert_eq!(response.lowest_low(), None);
        assert_eq!(response.total_volume(), 0);
    }

    #[test]
    fn test_futopt_daily_minimal() {
        let json = r#"{"symbol": "TXFC4", "data": []}"#;
        let response: FutOptDailyResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.symbol, "TXFC4");
        assert!(response.data.is_empty());
    }
}
