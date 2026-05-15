//! Candlestick (OHLCV) data models - matches Fugle candles responses

use serde::{Deserialize, Serialize};

/// A single intraday candlestick bar
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "python", pyo3::prelude::pyclass)]
#[cfg_attr(feature = "js", napi_derive::napi(object))]
pub struct IntradayCandle {
    /// Open price
    pub open: f64,

    /// High price
    pub high: f64,

    /// Low price
    pub low: f64,

    /// Close price
    pub close: f64,

    /// Volume
    pub volume: i64,

    /// Average price (VWAP for the candle period)
    pub average: Option<f64>,

    /// Candle timestamp (ISO 8601 with timezone, e.g. "2026-04-17T09:00:00.000+08:00")
    pub date: String,
}

impl IntradayCandle {
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

    /// Get upper wick size
    pub fn upper_wick(&self) -> f64 {
        self.high - self.close.max(self.open)
    }

    /// Get lower wick size
    pub fn lower_wick(&self) -> f64 {
        self.close.min(self.open) - self.low
    }
}

/// Intraday candles response from Fugle API (intraday/candles/{symbol})
///
/// This matches the official SDK's RestStockIntradayCandlesResponse
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "python", pyo3::prelude::pyclass)]
#[cfg_attr(feature = "js", napi_derive::napi(object))]
pub struct IntradayCandlesResponse {
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

    /// Timeframe (e.g., "1", "5", "10", "15", "30", "60")
    pub timeframe: Option<String>,

    /// Candle data
    #[serde(default)]
    pub data: Vec<IntradayCandle>,
}

/// A single historical candlestick bar
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "python", pyo3::prelude::pyclass)]
#[cfg_attr(feature = "js", napi_derive::napi(object))]
pub struct HistoricalCandle {
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

    /// Volume
    pub volume: i64,

    /// Turnover (total value traded)
    pub turnover: Option<f64>,

    /// Price change from previous close
    pub change: Option<f64>,
}

impl HistoricalCandle {
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

    /// Calculate change percent if previous close is known
    pub fn change_percent(&self, prev_close: f64) -> f64 {
        if prev_close == 0.0 {
            return 0.0;
        }
        (self.close - prev_close) / prev_close * 100.0
    }
}

/// Historical candles response from Fugle API (historical/candles/{symbol})
///
/// This matches the official SDK's RestStockHistoricalCandlesResponse
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "python", pyo3::prelude::pyclass)]
#[cfg_attr(feature = "js", napi_derive::napi(object))]
pub struct HistoricalCandlesResponse {
    /// Stock symbol
    pub symbol: String,

    /// Security type
    #[serde(rename = "type")]
    pub data_type: Option<String>,

    /// Exchange code
    pub exchange: Option<String>,

    /// Market
    pub market: Option<String>,

    /// Timeframe (e.g., "D", "W", "M", "1", "5", etc.)
    pub timeframe: Option<String>,

    /// Whether prices are adjusted for splits/dividends
    pub adjusted: Option<bool>,

    /// Candle data
    #[serde(default)]
    pub data: Vec<HistoricalCandle>,
}

impl HistoricalCandlesResponse {
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
    pub fn total_volume(&self) -> i64 {
        self.data.iter().map(|c| c.volume).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intraday_candle_deserialization() {
        let json = r#"{
            "open": 580.0,
            "high": 585.0,
            "low": 578.0,
            "close": 583.0,
            "volume": 10000,
            "average": 581.5,
            "date": "2024-01-15T09:00:00.000+08:00"
        }"#;
        let candle: IntradayCandle = serde_json::from_str(json).unwrap();
        assert_eq!(candle.open, 580.0);
        assert_eq!(candle.close, 583.0);
        assert!(candle.is_bullish());
        assert_eq!(candle.range(), 7.0);
    }

    #[test]
    fn test_intraday_candles_response() {
        let json = r#"{
            "date": "2024-01-15",
            "type": "EQUITY",
            "exchange": "TWSE",
            "market": "TSE",
            "symbol": "2330",
            "timeframe": "5",
            "data": [
                {"open": 580.0, "high": 582.0, "low": 579.0, "close": 581.0, "volume": 5000, "date": "2024-01-15T09:00:00.000+08:00"},
                {"open": 581.0, "high": 585.0, "low": 580.0, "close": 584.0, "volume": 8000, "date": "2024-01-15T09:05:00.000+08:00"}
            ]
        }"#;

        let response: IntradayCandlesResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.symbol, "2330");
        assert_eq!(response.timeframe.as_deref(), Some("5"));
        assert_eq!(response.data.len(), 2);
    }

    #[test]
    fn test_historical_candle_deserialization() {
        let json = r#"{
            "date": "2024-01-15",
            "open": 580.0,
            "high": 590.0,
            "low": 575.0,
            "close": 588.0,
            "volume": 50000000,
            "turnover": 29000000000,
            "change": 8.0
        }"#;
        let candle: HistoricalCandle = serde_json::from_str(json).unwrap();
        assert_eq!(candle.date, "2024-01-15");
        assert_eq!(candle.close, 588.0);
        assert!(candle.is_bullish());
    }

    #[test]
    fn test_historical_candles_response() {
        let json = r#"{
            "symbol": "2330",
            "type": "EQUITY",
            "timeframe": "D",
            "adjusted": true,
            "data": [
                {"date": "2024-01-12", "open": 570.0, "high": 580.0, "low": 568.0, "close": 578.0, "volume": 40000000},
                {"date": "2024-01-15", "open": 580.0, "high": 590.0, "low": 575.0, "close": 588.0, "volume": 50000000}
            ]
        }"#;

        let response: HistoricalCandlesResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.symbol, "2330");
        assert_eq!(response.adjusted, Some(true));
        assert_eq!(response.highest_high(), Some(590.0));
        assert_eq!(response.lowest_low(), Some(568.0));
        assert_eq!(response.total_volume(), 90000000);
    }

    #[test]
    fn test_candle_patterns() {
        // Bullish candle
        let bullish = IntradayCandle {
            open: 100.0,
            high: 105.0,
            low: 99.0,
            close: 104.0,
            volume: 1000,
            average: None,
            date: String::new(),
        };
        assert!(bullish.is_bullish());
        assert_eq!(bullish.body(), 4.0);
        assert_eq!(bullish.upper_wick(), 1.0);  // 105 - 104
        assert_eq!(bullish.lower_wick(), 1.0);  // 100 - 99

        // Bearish candle
        let bearish = IntradayCandle {
            open: 104.0,
            high: 105.0,
            low: 99.0,
            close: 100.0,
            volume: 1000,
            average: None,
            date: String::new(),
        };
        assert!(bearish.is_bearish());
        assert_eq!(bearish.body(), 4.0);
        assert_eq!(bearish.upper_wick(), 1.0);  // 105 - 104
        assert_eq!(bearish.lower_wick(), 1.0);  // 100 - 99
    }
}
