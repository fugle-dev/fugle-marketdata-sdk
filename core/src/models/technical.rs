//! Technical indicator response models for Fugle API
//!
//! This module contains response types for technical analysis indicators:
//! - SMA (Simple Moving Average)
//! - RSI (Relative Strength Index)
//! - KDJ (Stochastic Oscillator)
//! - MACD (Moving Average Convergence Divergence)
//! - BB (Bollinger Bands)

use serde::{Deserialize, Serialize};

// =============================================================================
// SMA (Simple Moving Average)
// =============================================================================

/// Response for SMA (Simple Moving Average) endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "python", pyo3::prelude::pyclass)]
#[cfg_attr(feature = "js", napi_derive::napi(object))]
pub struct SmaResponse {
    /// Stock symbol
    pub symbol: String,
    /// Security type (e.g., "EQUITY"). Optional — prod technical responses
    /// omit it (only `symbol`, `period`/params and `data` are returned).
    #[serde(rename = "type", default)]
    pub data_type: Option<String>,
    /// Exchange code (e.g., "TWSE"). Optional — omitted by prod.
    #[serde(default)]
    pub exchange: Option<String>,
    /// Market (e.g., "TSE"). Optional — omitted by prod.
    #[serde(default)]
    pub market: Option<String>,
    /// Timeframe of the data (e.g., "D", "W", "M"). Optional — omitted by prod.
    #[serde(default)]
    pub timeframe: Option<String>,
    /// SMA period used for calculation
    pub period: u32,
    /// SMA data points
    pub data: Vec<SmaDataPoint>,
}

/// Single data point for SMA response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "python", pyo3::prelude::pyclass)]
#[cfg_attr(feature = "js", napi_derive::napi(object))]
pub struct SmaDataPoint {
    /// Date of the data point (YYYY-MM-DD)
    pub date: String,
    /// SMA value
    pub sma: f64,
}

// =============================================================================
// RSI (Relative Strength Index)
// =============================================================================

/// Response for RSI (Relative Strength Index) endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "python", pyo3::prelude::pyclass)]
#[cfg_attr(feature = "js", napi_derive::napi(object))]
pub struct RsiResponse {
    /// Stock symbol
    pub symbol: String,
    /// Security type (e.g., "EQUITY"). Optional — prod technical responses
    /// omit it (only `symbol`, `period`/params and `data` are returned).
    #[serde(rename = "type", default)]
    pub data_type: Option<String>,
    /// Exchange code (e.g., "TWSE"). Optional — omitted by prod.
    #[serde(default)]
    pub exchange: Option<String>,
    /// Market (e.g., "TSE"). Optional — omitted by prod.
    #[serde(default)]
    pub market: Option<String>,
    /// Timeframe of the data (e.g., "D", "W", "M"). Optional — omitted by prod.
    #[serde(default)]
    pub timeframe: Option<String>,
    /// RSI period used for calculation
    pub period: u32,
    /// RSI data points
    pub data: Vec<RsiDataPoint>,
}

/// Single data point for RSI response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "python", pyo3::prelude::pyclass)]
#[cfg_attr(feature = "js", napi_derive::napi(object))]
pub struct RsiDataPoint {
    /// Date of the data point (YYYY-MM-DD)
    pub date: String,
    /// RSI value (0-100)
    pub rsi: f64,
}

// =============================================================================
// KDJ (Stochastic Oscillator)
// =============================================================================

/// Response for KDJ (Stochastic Oscillator) endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "python", pyo3::prelude::pyclass)]
#[cfg_attr(feature = "js", napi_derive::napi(object))]
pub struct KdjResponse {
    /// Stock symbol
    pub symbol: String,
    /// Security type (e.g., "EQUITY"). Optional — prod technical responses
    /// omit it (only `symbol`, `period`/params and `data` are returned).
    #[serde(rename = "type", default)]
    pub data_type: Option<String>,
    /// Exchange code (e.g., "TWSE"). Optional — omitted by prod.
    #[serde(default)]
    pub exchange: Option<String>,
    /// Market (e.g., "TSE"). Optional — omitted by prod.
    #[serde(default)]
    pub market: Option<String>,
    /// Timeframe of the data (e.g., "D", "W", "M"). Optional — omitted by prod.
    #[serde(default)]
    pub timeframe: Option<String>,
    /// RSV period (`rPeriod`). Prod returns this, not a single `period`.
    #[serde(rename = "rPeriod", default)]
    pub r_period: Option<u32>,
    /// K smoothing period (`kPeriod`).
    #[serde(rename = "kPeriod", default)]
    pub k_period: Option<u32>,
    /// D smoothing period (`dPeriod`).
    #[serde(rename = "dPeriod", default)]
    pub d_period: Option<u32>,
    /// KDJ data points
    pub data: Vec<KdjDataPoint>,
}

/// Single data point for KDJ response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "python", pyo3::prelude::pyclass)]
#[cfg_attr(feature = "js", napi_derive::napi(object))]
pub struct KdjDataPoint {
    /// Date of the data point (YYYY-MM-DD)
    pub date: String,
    /// K value (Fast Stochastic)
    pub k: f64,
    /// D value (Slow Stochastic)
    pub d: f64,
    /// J value (J line)
    pub j: f64,
}

// =============================================================================
// MACD (Moving Average Convergence Divergence)
// =============================================================================

/// Response for MACD (Moving Average Convergence Divergence) endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "python", pyo3::prelude::pyclass)]
#[cfg_attr(feature = "js", napi_derive::napi(object))]
pub struct MacdResponse {
    /// Stock symbol
    pub symbol: String,
    /// Security type (e.g., "EQUITY"). Optional — prod technical responses
    /// omit it (only `symbol`, `period`/params and `data` are returned).
    #[serde(rename = "type", default)]
    pub data_type: Option<String>,
    /// Exchange code (e.g., "TWSE"). Optional — omitted by prod.
    #[serde(default)]
    pub exchange: Option<String>,
    /// Market (e.g., "TSE"). Optional — omitted by prod.
    #[serde(default)]
    pub market: Option<String>,
    /// Timeframe of the data (e.g., "D", "W", "M"). Optional — omitted by prod.
    #[serde(default)]
    pub timeframe: Option<String>,
    /// Fast EMA period (typically 12)
    pub fast: u32,
    /// Slow EMA period (typically 26)
    pub slow: u32,
    /// Signal line period (typically 9)
    pub signal: u32,
    /// MACD data points
    pub data: Vec<MacdDataPoint>,
}

/// Single data point for MACD response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "python", pyo3::prelude::pyclass)]
#[cfg_attr(feature = "js", napi_derive::napi(object))]
pub struct MacdDataPoint {
    /// Date of the data point (YYYY-MM-DD)
    pub date: String,
    /// MACD line value (fast EMA - slow EMA). Prod wire key: `macdLine`.
    #[serde(rename = "macdLine")]
    pub macd: f64,
    /// Signal line value (EMA of MACD). Prod wire key: `signalLine`.
    #[serde(rename = "signalLine")]
    pub signal_value: f64,
    /// Histogram value (MACD - Signal). Optional — prod does not return it.
    #[serde(default)]
    pub histogram: Option<f64>,
}

// =============================================================================
// BB (Bollinger Bands)
// =============================================================================

/// Response for BB (Bollinger Bands) endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "python", pyo3::prelude::pyclass)]
#[cfg_attr(feature = "js", napi_derive::napi(object))]
pub struct BbResponse {
    /// Stock symbol
    pub symbol: String,
    /// Security type (e.g., "EQUITY"). Optional — prod technical responses
    /// omit it (only `symbol`, `period`/params and `data` are returned).
    #[serde(rename = "type", default)]
    pub data_type: Option<String>,
    /// Exchange code (e.g., "TWSE"). Optional — omitted by prod.
    #[serde(default)]
    pub exchange: Option<String>,
    /// Market (e.g., "TSE"). Optional — omitted by prod.
    #[serde(default)]
    pub market: Option<String>,
    /// Timeframe of the data (e.g., "D", "W", "M"). Optional — omitted by prod.
    #[serde(default)]
    pub timeframe: Option<String>,
    /// Period used for SMA calculation
    pub period: u32,
    /// Standard deviation multiplier used. Optional — prod omits it (only
    /// `symbol`/`period`/`data` are returned).
    #[serde(default)]
    pub stddev: Option<f64>,
    /// Bollinger Bands data points
    pub data: Vec<BbDataPoint>,
}

/// Single data point for Bollinger Bands response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "python", pyo3::prelude::pyclass)]
#[cfg_attr(feature = "js", napi_derive::napi(object))]
pub struct BbDataPoint {
    /// Date of the data point (YYYY-MM-DD)
    pub date: String,
    /// Upper band value (middle + stddev * standard deviation)
    pub upper: f64,
    /// Middle band value (SMA)
    pub middle: f64,
    /// Lower band value (middle - stddev * standard deviation)
    pub lower: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sma_response_deserialization() {
        let json = r#"{
            "symbol": "2330",
            "type": "EQUITY",
            "exchange": "TWSE",
            "market": "TSE",
            "timeframe": "D",
            "period": 20,
            "data": [
                {"date": "2024-01-15", "sma": 580.5},
                {"date": "2024-01-16", "sma": 581.2}
            ]
        }"#;

        let response: SmaResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.symbol, "2330");
        assert_eq!(response.data_type.as_deref(), Some("EQUITY"));
        assert_eq!(response.period, 20);
        assert_eq!(response.data.len(), 2);
        assert_eq!(response.data[0].sma, 580.5);
    }

    #[test]
    fn test_sma_response_prod_shape() {
        // Real prod payload: only symbol/period/data.
        let json = r#"{"symbol":"2330","period":20,
            "data":[{"date":"2026-04-16","sma":1895.0}]}"#;
        let r: SmaResponse = serde_json::from_str(json).unwrap();
        assert_eq!(r.data_type, None);
        assert_eq!(r.exchange, None);
        assert_eq!(r.timeframe, None);
        assert_eq!(r.data[0].sma, 1895.0);
    }

    #[test]
    fn test_rsi_response_deserialization() {
        let json = r#"{
            "symbol": "2330",
            "type": "EQUITY",
            "exchange": "TWSE",
            "market": "TSE",
            "timeframe": "D",
            "period": 14,
            "data": [
                {"date": "2024-01-15", "rsi": 65.5},
                {"date": "2024-01-16", "rsi": 68.2}
            ]
        }"#;

        let response: RsiResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.symbol, "2330");
        assert_eq!(response.period, 14);
        assert_eq!(response.data.len(), 2);
        assert_eq!(response.data[0].rsi, 65.5);
    }

    #[test]
    fn test_kdj_response_deserialization() {
        // Real prod shape: rPeriod/kPeriod/dPeriod, no type/exchange/etc.
        let json = r#"{
            "symbol": "2330",
            "rPeriod": 9,
            "kPeriod": 3,
            "dPeriod": 3,
            "data": [
                {"date": "2024-01-15", "k": 75.5, "d": 70.2, "j": 86.1},
                {"date": "2024-01-16", "k": 78.3, "d": 72.8, "j": 89.3}
            ]
        }"#;

        let response: KdjResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.symbol, "2330");
        assert_eq!(response.r_period, Some(9));
        assert_eq!(response.k_period, Some(3));
        assert_eq!(response.d_period, Some(3));
        assert_eq!(response.data_type, None);
        assert_eq!(response.data.len(), 2);
        assert_eq!(response.data[0].k, 75.5);
        assert_eq!(response.data[0].d, 70.2);
        assert_eq!(response.data[0].j, 86.1);
    }

    #[test]
    fn test_macd_response_deserialization() {
        // Real prod shape: macdLine/signalLine, no histogram, no type.
        let json = r#"{
            "symbol": "2330",
            "fast": 12,
            "slow": 26,
            "signal": 9,
            "data": [
                {"date": "2024-01-15", "macdLine": 5.5, "signalLine": 4.2},
                {"date": "2024-01-16", "macdLine": 6.2, "signalLine": 4.8}
            ]
        }"#;

        let response: MacdResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.symbol, "2330");
        assert_eq!(response.fast, 12);
        assert_eq!(response.slow, 26);
        assert_eq!(response.signal, 9);
        assert_eq!(response.data_type, None);
        assert_eq!(response.data.len(), 2);
        assert_eq!(response.data[0].macd, 5.5);
        assert_eq!(response.data[0].signal_value, 4.2);
        assert_eq!(response.data[0].histogram, None);
    }

    #[test]
    fn test_bb_response_deserialization() {
        let json = r#"{
            "symbol": "2330",
            "type": "EQUITY",
            "exchange": "TWSE",
            "market": "TSE",
            "timeframe": "D",
            "period": 20,
            "stddev": 2.0,
            "data": [
                {"date": "2024-01-15", "upper": 600.5, "middle": 580.0, "lower": 559.5},
                {"date": "2024-01-16", "upper": 602.2, "middle": 581.0, "lower": 559.8}
            ]
        }"#;

        let response: BbResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.symbol, "2330");
        assert_eq!(response.period, 20);
        assert_eq!(response.stddev, Some(2.0));
        assert_eq!(response.data.len(), 2);
    }

    #[test]
    fn test_bb_response_prod_shape() {
        // Real prod payload: only symbol/period/data — no stddev/type/etc.
        let json = r#"{"symbol":"2330","period":20,
            "data":[{"date":"2026-04-16","upper":2092.4,"middle":1895.0,"lower":1697.6}]}"#;
        let r: BbResponse = serde_json::from_str(json).unwrap();
        assert_eq!(r.stddev, None);
        assert_eq!(r.data_type, None);
        assert_eq!(r.data[0].middle, 1895.0);
    }
}
