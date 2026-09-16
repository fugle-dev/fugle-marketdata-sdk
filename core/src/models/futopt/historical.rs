//! FutOpt historical data models - matches Fugle futopt/historical/* responses
//!
//! Shapes follow the API gateway's `futopt/historical` entities (fugle-realtime
//! #727): both endpoints are keyed by **product** (`TXF`), not by contract, and
//! echo the `session` they were queried for.

use serde::{Deserialize, Serialize};

/// FutOpt Historical Candles response from Fugle API (futopt/historical/candles/{product})
///
/// # Example
///
/// ```rust
/// use marketdata_core::models::futopt::FutOptHistoricalCandlesResponse;
///
/// let json = r#"{
///     "product": "TXF",
///     "contractMonth": "202609",
///     "exchange": "TAIFEX",
///     "session": "REGULAR",
///     "timeframe": "D",
///     "sort": "asc",
///     "data": [
///         {"date": "2026-09-15", "contractMonth": "202609", "open": 17500.0, "high": 17580.0, "low": 17480.0, "close": 17550.0, "volume": 50000}
///     ]
/// }"#;
///
/// let response: FutOptHistoricalCandlesResponse = serde_json::from_str(json).unwrap();
/// assert_eq!(response.product, "TXF");
/// assert_eq!(response.candles.len(), 1);
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "python", pyo3::prelude::pyclass)]
#[cfg_attr(feature = "js", napi_derive::napi(object))]
pub struct FutOptHistoricalCandlesResponse {
    /// Product code, echoed from the request path (e.g., "TXF")
    pub product: String,

    /// Contract month queried: `YYYYMM`, or the continuous alias (`1!`) as requested
    #[serde(rename = "contractMonth")]
    pub contract_month: Option<String>,

    /// Exchange code (e.g., "TAIFEX")
    pub exchange: Option<String>,

    /// Trading session (e.g., "REGULAR", "AFTERHOURS")
    pub session: Option<String>,

    /// Timeframe (e.g., "D", "W", "M", "1", "5", etc.)
    pub timeframe: Option<String>,

    /// Sort order ("asc" or "desc")
    pub sort: Option<String>,

    /// Candle data
    #[serde(default, rename = "data")]
    pub candles: Vec<FutOptHistoricalCandle>,
}

impl FutOptHistoricalCandlesResponse {
    /// Get the highest high in the series (bars without `high` are skipped).
    pub fn highest_high(&self) -> Option<f64> {
        self.candles.iter().filter_map(|c| c.high).reduce(f64::max)
    }

    /// Get the lowest low in the series (bars without `low` are skipped).
    pub fn lowest_low(&self) -> Option<f64> {
        self.candles.iter().filter_map(|c| c.low).reduce(f64::min)
    }

    /// Get total volume over the series (bars without volume count as 0).
    pub fn total_volume(&self) -> u64 {
        self.candles.iter().filter_map(|c| c.volume).sum()
    }
}

/// A single historical candlestick bar for FutOpt.
///
/// Every price field is optional: the request's `fields` param chooses which
/// ones the server returns.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "python", pyo3::prelude::pyclass)]
#[cfg_attr(feature = "js", napi_derive::napi(object))]
pub struct FutOptHistoricalCandle {
    /// Date (YYYY-MM-DD), or a timestamp for intraday timeframes
    pub date: String,

    /// Contract month this bar belongs to — differs bar to bar when a
    /// continuous alias (`1!`) rolls over
    #[serde(rename = "contractMonth")]
    pub contract_month: Option<String>,

    /// Open price
    pub open: Option<f64>,

    /// High price
    pub high: Option<f64>,

    /// Low price
    pub low: Option<f64>,

    /// Close price
    pub close: Option<f64>,

    /// Volume (number of contracts)
    pub volume: Option<u64>,

    /// Average price (intraday timeframes only)
    pub average: Option<f64>,

    /// Number of transactions (intraday timeframes only)
    pub transaction: Option<u64>,

    /// Price change from previous close
    pub change: Option<f64>,
}

impl FutOptHistoricalCandle {
    /// Check if bullish (close > open). `false` if either is missing.
    pub fn is_bullish(&self) -> bool {
        matches!((self.open, self.close), (Some(o), Some(c)) if c > o)
    }

    /// Check if bearish (close < open). `false` if either is missing.
    pub fn is_bearish(&self) -> bool {
        matches!((self.open, self.close), (Some(o), Some(c)) if c < o)
    }

    /// Get the candle body size, if open and close are present
    pub fn body(&self) -> Option<f64> {
        Some((self.close? - self.open?).abs())
    }

    /// Get the candle range (high - low), if both are present
    pub fn range(&self) -> Option<f64> {
        Some(self.high? - self.low?)
    }
}

/// FutOpt Daily response from Fugle API (futopt/historical/daily/{product})
///
/// One trading day, one row per contract month of the product.
///
/// # Example
///
/// ```rust
/// use marketdata_core::models::futopt::FutOptDailyResponse;
///
/// let json = r#"{
///     "date": "2026-09-15",
///     "product": "TXF",
///     "exchange": "TAIFEX",
///     "session": "REGULAR",
///     "data": [
///         {"contractMonth": "202609", "openPrice": 17500.0, "highPrice": 17580.0, "lowPrice": 17480.0, "closePrice": 17550.0,
///          "change": 50.0, "changePercent": 0.29, "volume": 50000, "volumeSpread": 120, "openInterest": 90000, "settlementPrice": 17545.0}
///     ]
/// }"#;
///
/// let response: FutOptDailyResponse = serde_json::from_str(json).unwrap();
/// assert_eq!(response.product, "TXF");
/// assert_eq!(response.data[0].settlement_price, Some(17545.0));
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "python", pyo3::prelude::pyclass)]
#[cfg_attr(feature = "js", napi_derive::napi(object))]
pub struct FutOptDailyResponse {
    /// Trading date (YYYY-MM-DD)
    pub date: Option<String>,

    /// Product code, echoed from the request path (e.g., "TXF")
    pub product: String,

    /// Exchange code (e.g., "TAIFEX")
    pub exchange: Option<String>,

    /// Trading session (e.g., "REGULAR", "AFTERHOURS")
    pub session: Option<String>,

    /// One row per contract month
    #[serde(default)]
    pub data: Vec<FutOptDailyData>,
}

impl FutOptDailyResponse {
    /// Total volume across contract months (rows without volume count as 0).
    pub fn total_volume(&self) -> u64 {
        self.data.iter().filter_map(|d| d.volume).sum()
    }
}

/// One contract month's daily quote for FutOpt
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "python", pyo3::prelude::pyclass)]
#[cfg_attr(feature = "js", napi_derive::napi(object))]
pub struct FutOptDailyData {
    /// Contract month (e.g., "202609", or "202609/202610" for a spread)
    #[serde(rename = "contractMonth")]
    pub contract_month: String,

    /// Option right ("CALL" / "PUT"); `None` for futures
    #[serde(rename = "callPut")]
    pub call_put: Option<String>,

    /// Option strike price; `None` for futures
    #[serde(rename = "strikePrice")]
    pub strike_price: Option<f64>,

    /// Exchange code (e.g., "TAIFEX")
    pub exchange: Option<String>,

    /// Open price
    #[serde(rename = "openPrice")]
    pub open_price: Option<f64>,

    /// High price
    #[serde(rename = "highPrice")]
    pub high_price: Option<f64>,

    /// Low price
    #[serde(rename = "lowPrice")]
    pub low_price: Option<f64>,

    /// Close price
    #[serde(rename = "closePrice")]
    pub close_price: Option<f64>,

    /// Price change from previous close
    pub change: Option<f64>,

    /// Percentage change from previous close
    #[serde(rename = "changePercent")]
    pub change_percent: Option<f64>,

    /// Volume (number of contracts)
    pub volume: Option<u64>,

    /// Spread-order volume
    #[serde(rename = "volumeSpread")]
    pub volume_spread: Option<u64>,

    /// Open interest (total outstanding contracts)
    #[serde(rename = "openInterest")]
    pub open_interest: Option<u64>,

    /// Settlement price (official closing price for margin calculation)
    #[serde(rename = "settlementPrice")]
    pub settlement_price: Option<f64>,
}

impl FutOptDailyData {
    /// Check if bullish (close > open). `false` if either is missing.
    pub fn is_bullish(&self) -> bool {
        matches!((self.open_price, self.close_price), (Some(o), Some(c)) if c > o)
    }

    /// Check if bearish (close < open). `false` if either is missing.
    pub fn is_bearish(&self) -> bool {
        matches!((self.open_price, self.close_price), (Some(o), Some(c)) if c < o)
    }

    /// Get the daily range (high - low), if both are present
    pub fn range(&self) -> Option<f64> {
        Some(self.high_price? - self.low_price?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_futopt_historical_candles_response() {
        let json = r#"{
            "product": "TXF",
            "contractMonth": "1!",
            "exchange": "TAIFEX",
            "session": "AFTERHOURS",
            "timeframe": "D",
            "sort": "asc",
            "data": [
                {"date": "2026-09-14", "contractMonth": "202609", "open": 17400.0, "high": 17500.0, "low": 17380.0, "close": 17480.0, "volume": 45000},
                {"date": "2026-09-15", "contractMonth": "202610", "open": 17500.0, "high": 17580.0, "low": 17480.0, "close": 17550.0, "volume": 50000, "change": 70.0}
            ]
        }"#;

        let response: FutOptHistoricalCandlesResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.product, "TXF");
        assert_eq!(response.contract_month.as_deref(), Some("1!"));
        assert_eq!(response.session.as_deref(), Some("AFTERHOURS"));
        assert_eq!(response.sort.as_deref(), Some("asc"));
        assert_eq!(response.candles.len(), 2);
        assert_eq!(response.candles[1].contract_month.as_deref(), Some("202610"));
        assert_eq!(response.highest_high(), Some(17580.0));
        assert_eq!(response.lowest_low(), Some(17380.0));
        assert_eq!(response.total_volume(), 95000);
    }

    #[test]
    fn test_futopt_historical_candles_partial_fields() {
        // `fields=close` returns only the requested price.
        let json = r#"{"product":"TXF","data":[{"date":"2026-09-15","contractMonth":"202609","close":17550.0}]}"#;
        let response: FutOptHistoricalCandlesResponse = serde_json::from_str(json).unwrap();
        let candle = &response.candles[0];
        assert_eq!(candle.close, Some(17550.0));
        assert_eq!(candle.open, None);
        assert!(!candle.is_bullish());
        assert_eq!(candle.body(), None);
        assert_eq!(response.highest_high(), None);
        assert_eq!(response.total_volume(), 0);
    }

    #[test]
    fn test_futopt_historical_candle_methods() {
        let candle = FutOptHistoricalCandle {
            date: "2026-09-15".to_string(),
            contract_month: Some("202609".to_string()),
            open: Some(17500.0),
            high: Some(17580.0),
            low: Some(17480.0),
            close: Some(17550.0),
            volume: Some(50000),
            average: None,
            transaction: None,
            change: Some(70.0),
        };

        assert!(candle.is_bullish());
        assert!(!candle.is_bearish());
        assert_eq!(candle.body(), Some(50.0));
        assert_eq!(candle.range(), Some(100.0));
    }

    #[test]
    fn test_futopt_daily_response() {
        let json = r#"{
            "date": "2026-09-15",
            "product": "TXF",
            "exchange": "TAIFEX",
            "session": "REGULAR",
            "data": [
                {"contractMonth": "202609", "openPrice": 17400.0, "highPrice": 17500.0, "lowPrice": 17380.0, "closePrice": 17480.0,
                 "change": 30.0, "changePercent": 0.17, "volume": 45000, "volumeSpread": 100, "openInterest": 120000, "settlementPrice": 17475.0},
                {"contractMonth": "202610", "openPrice": 17500.0, "highPrice": 17580.0, "lowPrice": 17480.0, "closePrice": 17550.0,
                 "change": 50.0, "changePercent": 0.29, "volume": 5000, "volumeSpread": 20, "openInterest": 21000, "settlementPrice": 17545.0}
            ]
        }"#;

        let response: FutOptDailyResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.date.as_deref(), Some("2026-09-15"));
        assert_eq!(response.product, "TXF");
        assert_eq!(response.session.as_deref(), Some("REGULAR"));
        assert_eq!(response.data.len(), 2);
        assert_eq!(response.data[0].contract_month, "202609");
        assert_eq!(response.data[0].volume_spread, Some(100));
        assert_eq!(response.data[1].settlement_price, Some(17545.0));
        assert_eq!(response.total_volume(), 50000);
    }

    #[test]
    fn test_futopt_daily_standby_shape() {
        // Recorded from the #727 gateway (api-dev, 2026-09-16), trimmed to a
        // futures row, a spread row and an options row. Prices are integers
        // on the wire and many fields are null.
        let json = r#"{"date":"2026-09-16","product":"TXF","exchange":"TAIFEX","session":"REGULAR","data":[
            {"callPut":null,"contractMonth":"202609","strikePrice":null,"change":167,"changePercent":0.37,"closePrice":45759,"exchange":"TAIFEX","highPrice":46093,"lowPrice":45518,"openInterest":9684,"openPrice":45534,"settlementPrice":0,"volume":28491,"volumeSpread":null},
            {"contractMonth":"202609/202610","callPut":null,"strikePrice":null,"change":null,"changePercent":null,"closePrice":231,"exchange":"TAIFEX","highPrice":272,"lowPrice":144,"openInterest":null,"openPrice":155,"settlementPrice":null,"volume":1946,"volumeSpread":6980},
            {"contractMonth":"202609","callPut":"CALL","strikePrice":45000,"change":null,"changePercent":null,"closePrice":null,"exchange":"TAIFEX","highPrice":null,"lowPrice":null,"openInterest":12,"openPrice":null,"settlementPrice":820,"volume":0}
        ]}"#;
        let response: FutOptDailyResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.session.as_deref(), Some("REGULAR"));
        assert_eq!(response.data[0].close_price, Some(45759.0));
        assert_eq!(response.data[0].volume_spread, None);
        assert_eq!(response.data[1].volume_spread, Some(6980));
        assert_eq!(response.data[2].call_put.as_deref(), Some("CALL"));
        assert_eq!(response.data[2].strike_price, Some(45000.0));
        assert_eq!(response.data[2].range(), None);
        assert_eq!(response.total_volume(), 30437);
    }

    #[test]
    fn test_futopt_historical_candles_standby_shape() {
        // Recorded from the #727 gateway (api-dev, 2026-09-16): an intraday
        // timeframe dates bars with a full timestamp and `fields` trims prices.
        let json = r#"{"product":"TXF","contractMonth":"1!","exchange":"TAIFEX","session":"REGULAR","timeframe":"5","sort":"desc","data":[
            {"date":"2026-09-16T13:25:00.000+08:00","contractMonth":"202609","open":45759,"close":45759,"volume":610,"average":45759.21,"transaction":249}
        ]}"#;
        let response: FutOptHistoricalCandlesResponse = serde_json::from_str(json).unwrap();
        let bar = &response.candles[0];
        assert_eq!(bar.average, Some(45759.21));
        assert_eq!(bar.transaction, Some(249));
        assert_eq!(bar.high, None);
        assert_eq!(bar.body(), Some(0.0));
    }

    #[test]
    fn test_futopt_daily_data_methods() {
        let data = FutOptDailyData {
            contract_month: "202609".to_string(),
            open_price: Some(17500.0),
            high_price: Some(17580.0),
            low_price: Some(17480.0),
            close_price: Some(17550.0),
            ..Default::default()
        };

        assert!(data.is_bullish());
        assert!(!data.is_bearish());
        assert_eq!(data.range(), Some(100.0));
    }

    #[test]
    fn test_futopt_historical_minimal() {
        let json = r#"{"product": "TXF", "data": []}"#;
        let response: FutOptHistoricalCandlesResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.product, "TXF");
        assert!(response.candles.is_empty());
        assert_eq!(response.highest_high(), None);
        assert_eq!(response.lowest_low(), None);
        assert_eq!(response.total_volume(), 0);
    }

    #[test]
    fn test_futopt_daily_minimal() {
        let json = r#"{"product": "TXF", "data": []}"#;
        let response: FutOptDailyResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.product, "TXF");
        assert!(response.data.is_empty());
    }
}
