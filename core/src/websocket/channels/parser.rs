//! WebSocket message parsing utilities

use crate::models::streaming::{
    AggregatesData, BooksData, CandleData, CandlesSnapshot, IndicesData, StreamMessage,
    TradesData,
};
use crate::MarketDataError;
use serde_json::Value;

/// Parsed channel data with type information
///
/// Note: Variants have different sizes due to AggregatesData being larger.
/// Boxing would add indirection overhead for a type frequently used in hot paths.
/// The size difference is acceptable for streaming message handling.
#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum ChannelData {
    /// Trades channel data
    Trades(TradesData),
    /// Candles snapshot (entire day of 1-min candles)
    CandlesSnapshot(CandlesSnapshot),
    /// Candles real-time data (single candle)
    CandleData(CandleData),
    /// Books channel data (order book depth)
    Books(BooksData),
    /// Aggregates channel data (comprehensive quote-like)
    Aggregates(AggregatesData),
    /// Indices channel data
    Indices(IndicesData),
    /// Unknown channel, raw JSON preserved
    Unknown(Value),
}

/// Parse raw WebSocket message text to StreamMessage
///
/// # Example
/// ```rust
/// use marketdata_core::websocket::channels::parse_stream_message;
///
/// let json = r#"{"event": "subscribed", "id": "sub-1", "channel": "trades", "symbol": "2330"}"#;
/// let msg = parse_stream_message(json).unwrap();
/// ```
pub fn parse_stream_message(text: &str) -> Result<StreamMessage, MarketDataError> {
    serde_json::from_str(text).map_err(|e| MarketDataError::DeserializationError {
        source: serde_json::Error::io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Failed to parse stream message: {}", e),
        )),
    })
}

/// Parse channel-specific data from snapshot or data payload
///
/// Uses channel name to determine target type:
/// - "trades" -> TradesData
/// - "candles" -> CandlesSnapshot (for snapshot) or CandleData (for data)
/// - "books" -> BooksData
/// - "aggregates" -> AggregatesData
/// - "indices" -> IndicesData
///
/// # Arguments
/// * `channel` - Channel name (e.g., "trades", "candles")
/// * `data` - Raw JSON data payload
/// * `is_snapshot` - Whether this is a snapshot event (affects candles parsing)
pub fn parse_channel_data(
    channel: &str,
    data: &Value,
    is_snapshot: bool,
) -> Result<ChannelData, MarketDataError> {
    match channel {
        "trades" => {
            let trades: TradesData =
                serde_json::from_value(data.clone()).map_err(|e| MarketDataError::DeserializationError {
                    source: serde_json::Error::io(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("Failed to parse trades data: {}", e),
                    )),
                })?;
            Ok(ChannelData::Trades(trades))
        }
        "candles" => {
            if is_snapshot {
                let snapshot: CandlesSnapshot = serde_json::from_value(data.clone()).map_err(|e| {
                    MarketDataError::DeserializationError {
                        source: serde_json::Error::io(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!("Failed to parse candles snapshot: {}", e),
                        )),
                    }
                })?;
                Ok(ChannelData::CandlesSnapshot(snapshot))
            } else {
                let candle: CandleData = serde_json::from_value(data.clone()).map_err(|e| {
                    MarketDataError::DeserializationError {
                        source: serde_json::Error::io(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!("Failed to parse candle data: {}", e),
                        )),
                    }
                })?;
                Ok(ChannelData::CandleData(candle))
            }
        }
        "books" => {
            let books: BooksData =
                serde_json::from_value(data.clone()).map_err(|e| MarketDataError::DeserializationError {
                    source: serde_json::Error::io(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("Failed to parse books data: {}", e),
                    )),
                })?;
            Ok(ChannelData::Books(books))
        }
        "aggregates" => {
            let aggregates: AggregatesData =
                serde_json::from_value(data.clone()).map_err(|e| MarketDataError::DeserializationError {
                    source: serde_json::Error::io(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("Failed to parse aggregates data: {}", e),
                    )),
                })?;
            Ok(ChannelData::Aggregates(aggregates))
        }
        "indices" => {
            let indices: IndicesData =
                serde_json::from_value(data.clone()).map_err(|e| MarketDataError::DeserializationError {
                    source: serde_json::Error::io(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("Failed to parse indices data: {}", e),
                    )),
                })?;
            Ok(ChannelData::Indices(indices))
        }
        _ => {
            // Unknown channel, preserve raw JSON
            Ok(ChannelData::Unknown(data.clone()))
        }
    }
}

impl ChannelData {
    /// Get the symbol from channel data
    pub fn symbol(&self) -> Option<&str> {
        match self {
            ChannelData::Trades(d) => Some(&d.symbol),
            ChannelData::CandlesSnapshot(d) => Some(&d.symbol),
            ChannelData::CandleData(d) => Some(&d.symbol),
            ChannelData::Books(d) => Some(&d.symbol),
            ChannelData::Aggregates(d) => Some(&d.symbol),
            ChannelData::Indices(d) => Some(&d.symbol),
            ChannelData::Unknown(_) => None,
        }
    }

    /// Get channel name
    pub fn channel(&self) -> &'static str {
        match self {
            ChannelData::Trades(_) => "trades",
            ChannelData::CandlesSnapshot(_) => "candles",
            ChannelData::CandleData(_) => "candles",
            ChannelData::Books(_) => "books",
            ChannelData::Aggregates(_) => "aggregates",
            ChannelData::Indices(_) => "indices",
            ChannelData::Unknown(_) => "unknown",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_stream_message_subscribed() {
        let json = r#"{
            "event": "subscribed",
            "id": "sub-123",
            "channel": "trades",
            "symbol": "2330"
        }"#;
        let msg = parse_stream_message(json).unwrap();
        match msg {
            StreamMessage::Subscribed { data } => {
                assert_eq!(data.id, "sub-123");
                assert_eq!(data.channel, Some("trades".to_string()));
                assert_eq!(data.symbol, Some("2330".to_string()));
            }
            _ => panic!("Expected Subscribed variant"),
        }
    }

    #[test]
    fn test_parse_stream_message_snapshot() {
        let json = r#"{
            "event": "snapshot",
            "id": "sub-123",
            "channel": "trades",
            "data": {
                "symbol": "2330",
                "trades": [{"price": 583.0, "size": 100}]
            }
        }"#;
        let msg = parse_stream_message(json).unwrap();
        match msg {
            StreamMessage::Snapshot {
                id,
                channel,
                payload,
            } => {
                assert_eq!(id, "sub-123");
                assert_eq!(channel, "trades");
                assert!(payload.data.is_object());
            }
            _ => panic!("Expected Snapshot variant"),
        }
    }

    #[test]
    fn test_parse_stream_message_data() {
        let json = r#"{
            "event": "data",
            "id": "sub-123",
            "channel": "candles",
            "data": {
                "symbol": "2330",
                "date": "2026-01-30T09:00:00.000+08:00",
                "open": 580.0, "high": 585.0, "low": 578.0, "close": 583.0,
                "volume": 12345
            }
        }"#;
        let msg = parse_stream_message(json).unwrap();
        match msg {
            StreamMessage::Data {
                id,
                channel,
                payload,
            } => {
                assert_eq!(id, "sub-123");
                assert_eq!(channel, "candles");
                assert!(payload.data.is_object());
            }
            _ => panic!("Expected Data variant"),
        }
    }

    #[test]
    fn test_parse_stream_message_error() {
        let json = r#"{
            "event": "error",
            "code": 4001,
            "message": "Invalid symbol"
        }"#;
        let msg = parse_stream_message(json).unwrap();
        match msg {
            StreamMessage::Error { data } => {
                assert_eq!(data.code, Some(4001));
                assert_eq!(data.message, Some("Invalid symbol".to_string()));
            }
            _ => panic!("Expected Error variant"),
        }
    }

    #[test]
    fn test_parse_channel_data_trades() {
        let data = serde_json::json!({
            "symbol": "2330",
            "type": "EQUITY",
            "trades": [{"price": 583.0, "size": 100}],
            "time": 1704067200123456_i64
        });
        let result = parse_channel_data("trades", &data, false).unwrap();
        match result {
            ChannelData::Trades(t) => {
                assert_eq!(t.symbol, "2330");
                assert_eq!(t.trades.len(), 1);
            }
            _ => panic!("Expected Trades variant"),
        }
    }

    #[test]
    fn test_parse_channel_data_candles_snapshot() {
        let data = serde_json::json!({
            "symbol": "2330",
            "date": "2026-01-30",
            "timeframe": "1",
            "data": [
                {"date": "2026-01-30T09:00:00.000+08:00", "open": 580.0, "high": 581.0, "low": 579.0, "close": 580.5, "volume": 1000}
            ]
        });
        let result = parse_channel_data("candles", &data, true).unwrap();
        match result {
            ChannelData::CandlesSnapshot(s) => {
                assert_eq!(s.symbol, "2330");
                assert_eq!(s.timeframe, Some("1".to_string()));
                assert_eq!(s.data.len(), 1);
            }
            _ => panic!("Expected CandlesSnapshot variant"),
        }
    }

    #[test]
    fn test_parse_channel_data_candles_data() {
        let data = serde_json::json!({
            "symbol": "2330",
            "date": "2026-01-30T09:00:00.000+08:00",
            "open": 580.0, "high": 585.0, "low": 578.0, "close": 583.0,
            "volume": 12345
        });
        let result = parse_channel_data("candles", &data, false).unwrap();
        match result {
            ChannelData::CandleData(c) => {
                assert_eq!(c.symbol, "2330");
                assert_eq!(c.volume, 12345);
            }
            _ => panic!("Expected CandleData variant"),
        }
    }

    #[test]
    fn test_parse_channel_data_books() {
        let data = serde_json::json!({
            "symbol": "2330",
            "bids": [{"price": 582.0, "size": 100}],
            "asks": [{"price": 583.0, "size": 50}]
        });
        let result = parse_channel_data("books", &data, false).unwrap();
        match result {
            ChannelData::Books(b) => {
                assert_eq!(b.symbol, "2330");
                assert_eq!(b.bids.len(), 1);
                assert_eq!(b.asks.len(), 1);
            }
            _ => panic!("Expected Books variant"),
        }
    }

    #[test]
    fn test_parse_channel_data_aggregates() {
        let data = serde_json::json!({
            "symbol": "2330",
            "type": "EQUITY",
            "openPrice": 580.0,
            "closePrice": 585.0
        });
        let result = parse_channel_data("aggregates", &data, false).unwrap();
        match result {
            ChannelData::Aggregates(a) => {
                assert_eq!(a.symbol, "2330");
                assert_eq!(a.open_price, Some(580.0));
            }
            _ => panic!("Expected Aggregates variant"),
        }
    }

    #[test]
    fn test_parse_channel_data_indices() {
        let data = serde_json::json!({
            "symbol": "IX0001",
            "type": "INDEX",
            "index": 17500.5
        });
        let result = parse_channel_data("indices", &data, false).unwrap();
        match result {
            ChannelData::Indices(i) => {
                assert_eq!(i.symbol, "IX0001");
                assert_eq!(i.index, Some(17500.5));
            }
            _ => panic!("Expected Indices variant"),
        }
    }

    #[test]
    fn test_parse_channel_data_unknown() {
        let data = serde_json::json!({"foo": "bar"});
        let result = parse_channel_data("unknown_channel", &data, false).unwrap();
        match result {
            ChannelData::Unknown(v) => {
                assert_eq!(v["foo"], "bar");
            }
            _ => panic!("Expected Unknown variant"),
        }
    }

    #[test]
    fn test_channel_data_symbol() {
        let data = serde_json::json!({"symbol": "2330"});
        let trades = parse_channel_data("trades", &data, false).unwrap();
        assert_eq!(trades.symbol(), Some("2330"));
    }

    #[test]
    fn test_channel_data_channel() {
        let data = serde_json::json!({"symbol": "2330"});
        let trades = parse_channel_data("trades", &data, false).unwrap();
        assert_eq!(trades.channel(), "trades");
    }

    #[test]
    fn test_channel_data_unknown_symbol() {
        let data = serde_json::json!({"foo": "bar"});
        let unknown = parse_channel_data("unknown", &data, false).unwrap();
        assert_eq!(unknown.symbol(), None);
        assert_eq!(unknown.channel(), "unknown");
    }
}
