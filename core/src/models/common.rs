//! Common types shared across market data models

use serde::{Deserialize, Deserializer, Serialize};

/// Deserialize an exchange sequence number that the server types
/// inconsistently, normalising to `String`.
///
/// Verified against live payloads: on `stock/intraday/quote`,
/// `lastTrade.serial` is a JSON number (`17738549`); on
/// `futopt/intraday/quote` the same field is a zero-padded JSON string
/// (`"00379320"`). The official TypeScript interface declares `serial: number`
/// for both, which is wrong for futopt.
///
/// This matters beyond the two REST endpoints: futopt's streaming
/// `aggregates` frame carries the same object as the futopt REST quote, so a
/// numeric-only type would fail to decode it.
///
/// `String` is the honest target type — a serial is an opaque identifier, never
/// an operand, and futopt's zero padding is fixed-width and significant.
pub(crate) fn deserialize_serial<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Serial {
        Str(String),
        Int(i64),
    }

    Ok(Option::<Serial>::deserialize(deserializer)?.map(|s| match s {
        Serial::Str(s) => s,
        Serial::Int(i) => i.to_string(),
    }))
}

/// Common response metadata for all API responses
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "python", pyo3::prelude::pyclass)]
#[cfg_attr(feature = "js", napi_derive::napi(object))]
pub struct ResponseMeta {
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
}

/// Price level for order book (bid/ask)
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "python", pyo3::prelude::pyclass)]
#[cfg_attr(feature = "js", napi_derive::napi(object))]
pub struct PriceLevel {
    /// Price at this level
    pub price: f64,

    /// Size (volume) at this level
    pub size: i64,
}

/// Trade execution info (used in quote.lastTrade, quote.lastTrial)
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "python", pyo3::prelude::pyclass)]
#[cfg_attr(feature = "js", napi_derive::napi(object))]
pub struct TradeInfo {
    /// Best bid price at trade time
    pub bid: Option<f64>,

    /// Best ask price at trade time
    pub ask: Option<f64>,

    /// Trade price
    pub price: f64,

    /// Trade size
    pub size: i64,

    /// Trade timestamp (Unix milliseconds)
    pub time: i64,

    /// Exchange sequence number, normalised to a string.
    ///
    /// The server sends a number here for stock and a zero-padded string for
    /// futopt; see [`deserialize_serial`].
    #[serde(default, deserialize_with = "deserialize_serial")]
    pub serial: Option<String>,
}

/// Total trading statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "python", pyo3::prelude::pyclass)]
#[cfg_attr(feature = "js", napi_derive::napi(object))]
pub struct TotalStats {
    /// Total trade value. Absent on FutOpt aggregates — server sends
    /// `{time, totalBidMatch, totalAskMatch, tradeVolume}` with no
    /// `tradeValue`, so a missing field must not fail deserialization.
    #[serde(rename = "tradeValue", default)]
    pub trade_value: f64,

    /// Total trade volume
    #[serde(rename = "tradeVolume", default)]
    pub trade_volume: i64,

    /// Volume traded at bid
    #[serde(rename = "tradeVolumeAtBid")]
    pub trade_volume_at_bid: Option<i64>,

    /// Volume traded at ask
    #[serde(rename = "tradeVolumeAtAsk")]
    pub trade_volume_at_ask: Option<i64>,

    /// Number of transactions
    pub transaction: Option<i64>,

    /// Timestamp
    pub time: Option<i64>,
}

/// Trading halt status
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "python", pyo3::prelude::pyclass)]
#[cfg_attr(feature = "js", napi_derive::napi(object))]
pub struct TradingHalt {
    /// Whether trading is halted
    #[serde(rename = "isHalted")]
    pub is_halted: bool,

    /// Halt timestamp
    pub time: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_price_level_deserialization() {
        let json = r#"{"price": 100.5, "size": 1000}"#;
        let level: PriceLevel = serde_json::from_str(json).unwrap();
        assert_eq!(level.price, 100.5);
        assert_eq!(level.size, 1000);
    }

    #[test]
    fn test_trade_info_deserialization() {
        let json = r#"{"bid": 100.0, "ask": 100.5, "price": 100.5, "size": 500, "time": 1704067200000}"#;
        let info: TradeInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.price, 100.5);
        assert_eq!(info.size, 500);
        assert_eq!(info.time, 1704067200000);
        assert_eq!(info.serial, None);
    }

    #[test]
    fn test_trade_info_serial_from_number() {
        // Live stock/intraday/quote shape.
        let json = r#"{"price": 2405.0, "size": 4021, "time": 1785907800000000, "serial": 17738549}"#;
        let info: TradeInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.serial.as_deref(), Some("17738549"));
    }

    #[test]
    fn test_trade_info_serial_from_padded_string() {
        // Live futopt/intraday/quote shape — same field, different JSON type.
        // futopt's streaming `aggregates` frame carries this same object, so
        // both spellings have to decode through one type.
        let json = r#"{"price": 45777.0, "size": 1, "time": 1785901265049000, "serial": "00379320"}"#;
        let info: TradeInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.serial.as_deref(), Some("00379320"));
    }

    #[test]
    fn test_response_meta_deserialization() {
        let json = r#"{
            "date": "2024-01-15",
            "type": "EQUITY",
            "exchange": "TWSE",
            "market": "TSE",
            "symbol": "2330"
        }"#;
        let meta: ResponseMeta = serde_json::from_str(json).unwrap();
        assert_eq!(meta.date, "2024-01-15");
        assert_eq!(meta.symbol, "2330");
        assert_eq!(meta.data_type.as_deref(), Some("EQUITY"));
    }
}
