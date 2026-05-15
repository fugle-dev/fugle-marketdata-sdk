//! Stock ticker info model - matches Fugle intraday/ticker/{symbol} response

use serde::{Deserialize, Serialize};

/// Stock ticker information from Fugle API (intraday/ticker/{symbol})
///
/// This matches the official SDK's RestStockIntradayTickerResponse
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "python", pyo3::prelude::pyclass)]
#[cfg_attr(feature = "js", napi_derive::napi(object))]
pub struct Ticker {
    // === Response metadata ===
    /// Trading date (YYYY-MM-DD)
    pub date: String,

    /// Security type (e.g., "EQUITY", "ODDLOT")
    #[serde(rename = "type")]
    pub data_type: Option<String>,

    /// Exchange code (e.g., "TWSE", "TPEx")
    pub exchange: Option<String>,

    /// Market (e.g., "TSE", "OTC")
    pub market: Option<String>,

    /// Stock symbol (e.g., "2330")
    pub symbol: String,

    // === Stock info ===
    /// Stock name (Chinese)
    pub name: Option<String>,

    /// Stock name (English)
    #[serde(rename = "nameEn")]
    pub name_en: Option<String>,

    /// Industry category
    pub industry: Option<String>,

    /// Security type classification
    #[serde(rename = "securityType")]
    pub security_type: Option<String>,

    // === Price limits ===
    /// Reference price (previous close)
    #[serde(rename = "referencePrice")]
    pub reference_price: Option<f64>,

    /// Limit up price
    #[serde(rename = "limitUpPrice")]
    pub limit_up_price: Option<f64>,

    /// Limit down price
    #[serde(rename = "limitDownPrice")]
    pub limit_down_price: Option<f64>,

    /// Previous close price
    #[serde(rename = "previousClose")]
    pub previous_close: Option<f64>,

    // === Trading rules ===
    /// Can day trade
    #[serde(rename = "canDayTrade", default)]
    pub can_day_trade: bool,

    /// Can buy day trade
    #[serde(rename = "canBuyDayTrade", default)]
    pub can_buy_day_trade: bool,

    /// Can below flat margin short sell
    #[serde(rename = "canBelowFlatMarginShortSell", default)]
    pub can_below_flat_margin_short_sell: bool,

    /// Can below flat SBL short sell
    #[serde(rename = "canBelowFlatSBLShortSell", default)]
    pub can_below_flat_sbl_short_sell: bool,

    // === Attention flags ===
    /// Is attention stock
    #[serde(rename = "isAttention", default)]
    pub is_attention: bool,

    /// Is disposition stock
    #[serde(rename = "isDisposition", default)]
    pub is_disposition: bool,

    /// Is unusually recommended
    #[serde(rename = "isUnusuallyRecommended", default)]
    pub is_unusually_recommended: bool,

    /// Is specific abnormally
    #[serde(rename = "isSpecificAbnormally", default)]
    pub is_specific_abnormally: bool,

    /// Is newly compiled
    #[serde(rename = "isNewlyCompiled", default)]
    pub is_newly_compiled: bool,

    // === Trading parameters ===
    /// Matching interval (seconds)
    #[serde(rename = "matchingInterval")]
    pub matching_interval: Option<i32>,

    /// Security status
    #[serde(rename = "securityStatus")]
    pub security_status: Option<String>,

    /// Board lot size
    #[serde(rename = "boardLot")]
    pub board_lot: Option<i32>,

    /// Trading currency
    #[serde(rename = "tradingCurrency")]
    pub trading_currency: Option<String>,

    // === Warrant/ETN specific ===
    /// Exercise price (for warrants)
    #[serde(rename = "exercisePrice")]
    pub exercise_price: Option<f64>,

    /// Exercised volume
    #[serde(rename = "exercisedVolume")]
    pub exercised_volume: Option<i64>,

    /// Cancelled volume
    #[serde(rename = "cancelledVolume")]
    pub cancelled_volume: Option<i64>,

    /// Remaining volume
    #[serde(rename = "remainingVolume")]
    pub remaining_volume: Option<i64>,

    /// Exercise ratio
    #[serde(rename = "exerciseRatio")]
    pub exercise_ratio: Option<f64>,

    /// Cap price
    #[serde(rename = "capPrice")]
    pub cap_price: Option<f64>,

    /// Floor price
    #[serde(rename = "floorPrice")]
    pub floor_price: Option<f64>,

    /// Maturity date
    #[serde(rename = "maturityDate")]
    pub maturity_date: Option<String>,

    // === Session times ===
    /// Open time
    #[serde(rename = "openTime")]
    pub open_time: Option<String>,

    /// Close time
    #[serde(rename = "closeTime")]
    pub close_time: Option<String>,
}

impl Ticker {
    /// Get the price limit range
    pub fn price_range(&self) -> Option<(f64, f64)> {
        match (self.limit_down_price, self.limit_up_price) {
            (Some(down), Some(up)) => Some((down, up)),
            _ => None,
        }
    }

    /// Check if stock is tradeable (not halted or restricted)
    pub fn is_tradeable(&self) -> bool {
        !self.is_attention && !self.is_disposition
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ticker_deserialization() {
        let json = r#"{
            "date": "2024-01-15",
            "type": "EQUITY",
            "exchange": "TWSE",
            "market": "TSE",
            "symbol": "2330",
            "name": "台積電",
            "nameEn": "TSMC",
            "industry": "半導體業",
            "securityType": "STOCK",
            "referencePrice": 580.0,
            "limitUpPrice": 638.0,
            "limitDownPrice": 522.0,
            "canDayTrade": true,
            "canBuyDayTrade": true,
            "isAttention": false,
            "isDisposition": false,
            "matchingInterval": 5,
            "boardLot": 1000
        }"#;

        let ticker: Ticker = serde_json::from_str(json).unwrap();
        assert_eq!(ticker.symbol, "2330");
        assert_eq!(ticker.name.as_deref(), Some("台積電"));
        assert_eq!(ticker.name_en.as_deref(), Some("TSMC"));
        assert_eq!(ticker.limit_up_price, Some(638.0));
        assert!(ticker.can_day_trade);
        assert!(ticker.is_tradeable());
    }

    #[test]
    fn test_ticker_price_range() {
        let ticker = Ticker {
            limit_down_price: Some(522.0),
            limit_up_price: Some(638.0),
            ..Default::default()
        };
        assert_eq!(ticker.price_range(), Some((522.0, 638.0)));
    }

    #[test]
    fn test_ticker_attention_stock() {
        let ticker = Ticker {
            is_attention: true,
            ..Default::default()
        };
        assert!(!ticker.is_tradeable());
    }
}
