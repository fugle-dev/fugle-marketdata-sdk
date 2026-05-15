//! Streaming message types for WebSocket data
//!
//! These types are used to parse WebSocket messages from the streaming API.
//! The top-level [`StreamMessage`] enum uses serde's internally tagged enum
//! pattern to distinguish between event types.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::models::common::{PriceLevel, TotalStats, TradeInfo};

// ============================================================================
// Top-level Message Types
// ============================================================================

/// Top-level WebSocket message
///
/// Uses serde's internally tagged enum to parse based on "event" field.
///
/// # Example
/// ```rust
/// use marketdata_core::models::streaming::StreamMessage;
///
/// let json = r#"{"event": "authenticated"}"#;
/// let msg: StreamMessage = serde_json::from_str(json).unwrap();
/// assert!(matches!(msg, StreamMessage::Authenticated));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "lowercase")]
pub enum StreamMessage {
    /// Authentication successful
    Authenticated,

    /// Subscription confirmed
    Subscribed {
        #[serde(flatten)]
        data: SubscribedData,
    },

    /// Snapshot after subscription (channel-specific)
    Snapshot {
        /// Subscription ID
        id: String,
        /// Channel name
        channel: String,
        /// Snapshot payload (channel-specific, parsed later)
        #[serde(flatten)]
        payload: SnapshotPayload,
    },

    /// Real-time data (channel-specific)
    Data {
        /// Subscription ID
        id: String,
        /// Channel name
        channel: String,
        /// Data payload (channel-specific, parsed later)
        #[serde(flatten)]
        payload: DataPayload,
    },

    /// Error event
    Error {
        #[serde(flatten)]
        data: ErrorData,
    },

    /// Pong response (health check)
    Pong {
        #[serde(default)]
        state: Option<String>,
    },
}

/// Subscribed event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscribedData {
    /// Subscription ID (used for unsubscribe)
    pub id: String,
    /// Channel name
    #[serde(default)]
    pub channel: Option<String>,
    /// Symbol
    #[serde(default)]
    pub symbol: Option<String>,
}

/// Snapshot payload (channel-specific, parsed later)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotPayload {
    /// Raw data to be parsed based on channel
    pub data: Value,
}

/// Data payload (channel-specific, parsed later)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataPayload {
    /// Raw data to be parsed based on channel
    pub data: Value,
}

/// Error event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorData {
    /// Error code
    #[serde(default)]
    pub code: Option<i32>,
    /// Error message
    #[serde(default)]
    pub message: Option<String>,
}

// ============================================================================
// Channel-Specific Data Types
// ============================================================================

/// Trades data (snapshot and real-time share same structure)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradesData {
    /// Stock symbol
    pub symbol: String,
    /// Data type (e.g., "EQUITY", "ODDLOT")
    #[serde(rename = "type", default)]
    pub data_type: Option<String>,
    /// Exchange code
    #[serde(default)]
    pub exchange: Option<String>,
    /// Market
    #[serde(default)]
    pub market: Option<String>,
    /// Trade list
    #[serde(default)]
    pub trades: Vec<StreamTrade>,
    /// Total statistics
    #[serde(default)]
    pub total: Option<TotalStats>,
    /// Unix microseconds
    #[serde(default)]
    pub time: Option<i64>,
    /// Serial number
    #[serde(default)]
    pub serial: Option<i64>,
}

/// Single trade in streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamTrade {
    /// Trade price
    pub price: f64,
    /// Trade size
    pub size: i64,
    /// Best bid at trade time
    #[serde(default)]
    pub bid: Option<f64>,
    /// Best ask at trade time
    #[serde(default)]
    pub ask: Option<f64>,
}

/// Candles snapshot (special: entire day of 1-min candles)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandlesSnapshot {
    /// Stock symbol
    pub symbol: String,
    /// Trading date (YYYY-MM-DD)
    pub date: String,
    /// Timeframe (always "1" for snapshot)
    #[serde(default)]
    pub timeframe: Option<String>,
    /// Array of 1-minute candles for the day
    pub data: Vec<CandleHistoryItem>,
}

/// Single candle in snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandleHistoryItem {
    /// ISO 8601 timestamp
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
    /// Average price
    #[serde(default)]
    pub average: Option<f64>,
}

/// Candles real-time data (single candle update)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandleData {
    /// Stock symbol
    pub symbol: String,
    /// ISO 8601 timestamp
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
    /// Average price
    #[serde(default)]
    pub average: Option<f64>,
}

/// Books data (order book depth)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BooksData {
    /// Stock symbol
    pub symbol: String,
    /// Bid levels
    #[serde(default)]
    pub bids: Vec<PriceLevel>,
    /// Ask levels
    #[serde(default)]
    pub asks: Vec<PriceLevel>,
    /// Unix microseconds
    #[serde(default)]
    pub time: Option<i64>,
    /// Serial number
    #[serde(default)]
    pub serial: Option<i64>,
}

/// Aggregates data (comprehensive quote-like)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatesData {
    /// Stock symbol
    pub symbol: String,
    /// Data type
    #[serde(rename = "type", default)]
    pub data_type: Option<String>,
    /// Exchange code
    #[serde(default)]
    pub exchange: Option<String>,
    /// Market
    #[serde(default)]
    pub market: Option<String>,
    /// Trading date
    #[serde(default)]
    pub date: Option<String>,
    // Price fields
    /// Reference price
    #[serde(rename = "referencePrice", default)]
    pub reference_price: Option<f64>,
    /// Previous close
    #[serde(rename = "previousClose", default)]
    pub previous_close: Option<f64>,
    /// Open price
    #[serde(rename = "openPrice", default)]
    pub open_price: Option<f64>,
    /// High price
    #[serde(rename = "highPrice", default)]
    pub high_price: Option<f64>,
    /// Low price
    #[serde(rename = "lowPrice", default)]
    pub low_price: Option<f64>,
    /// Close price
    #[serde(rename = "closePrice", default)]
    pub close_price: Option<f64>,
    /// Average price
    #[serde(rename = "avgPrice", default)]
    pub avg_price: Option<f64>,
    /// Last trade price
    #[serde(rename = "lastPrice", default)]
    pub last_price: Option<f64>,
    /// Last trade size
    #[serde(rename = "lastSize", default)]
    pub last_size: Option<i64>,
    // Order book
    /// Bid levels
    #[serde(default)]
    pub bids: Vec<PriceLevel>,
    /// Ask levels
    #[serde(default)]
    pub asks: Vec<PriceLevel>,
    // Statistics
    /// Total statistics
    #[serde(default)]
    pub total: Option<TotalStats>,
    /// Last trade info
    #[serde(rename = "lastTrade", default)]
    pub last_trade: Option<TradeInfo>,
    // Timestamps
    /// Unix microseconds
    #[serde(default)]
    pub time: Option<i64>,
    /// Serial number
    #[serde(default)]
    pub serial: Option<i64>,
    /// Last updated timestamp
    #[serde(rename = "lastUpdated", default)]
    pub last_updated: Option<i64>,
}

/// Indices data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndicesData {
    /// Index symbol
    pub symbol: String,
    /// Data type
    #[serde(rename = "type", default)]
    pub data_type: Option<String>,
    /// Exchange code
    #[serde(default)]
    pub exchange: Option<String>,
    /// Index value
    #[serde(default)]
    pub index: Option<f64>,
    /// Unix microseconds
    #[serde(default)]
    pub time: Option<i64>,
    /// Serial number
    #[serde(default)]
    pub serial: Option<i64>,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_authenticated_event() {
        let json = r#"{"event": "authenticated"}"#;
        let msg: StreamMessage = serde_json::from_str(json).unwrap();
        assert!(matches!(msg, StreamMessage::Authenticated));
    }

    #[test]
    fn test_parse_subscribed_event() {
        let json = r#"{
            "event": "subscribed",
            "id": "sub-123",
            "channel": "trades",
            "symbol": "2330"
        }"#;
        let msg: StreamMessage = serde_json::from_str(json).unwrap();
        if let StreamMessage::Subscribed { data } = msg {
            assert_eq!(data.id, "sub-123");
            assert_eq!(data.channel.as_deref(), Some("trades"));
            assert_eq!(data.symbol.as_deref(), Some("2330"));
        } else {
            panic!("Expected Subscribed event");
        }
    }

    #[test]
    fn test_parse_snapshot_event() {
        let json = r#"{
            "event": "snapshot",
            "id": "sub-123",
            "channel": "trades",
            "data": {
                "symbol": "2330",
                "trades": [{"price": 583.0, "size": 100}],
                "time": 1704067200123456
            }
        }"#;
        let msg: StreamMessage = serde_json::from_str(json).unwrap();
        if let StreamMessage::Snapshot { id, channel, payload } = msg {
            assert_eq!(id, "sub-123");
            assert_eq!(channel, "trades");
            assert!(payload.data.is_object());
        } else {
            panic!("Expected Snapshot event");
        }
    }

    #[test]
    fn test_parse_data_event() {
        let json = r#"{
            "event": "data",
            "id": "sub-123",
            "channel": "candles",
            "data": {
                "symbol": "2330",
                "date": "2026-01-30T09:00:00.000+08:00",
                "open": 580.0,
                "high": 585.0,
                "low": 578.0,
                "close": 583.0,
                "volume": 12345
            }
        }"#;
        let msg: StreamMessage = serde_json::from_str(json).unwrap();
        if let StreamMessage::Data { id, channel, payload } = msg {
            assert_eq!(id, "sub-123");
            assert_eq!(channel, "candles");
            assert!(payload.data.is_object());
        } else {
            panic!("Expected Data event");
        }
    }

    #[test]
    fn test_parse_error_event() {
        let json = r#"{
            "event": "error",
            "code": 4001,
            "message": "Invalid symbol"
        }"#;
        let msg: StreamMessage = serde_json::from_str(json).unwrap();
        if let StreamMessage::Error { data } = msg {
            assert_eq!(data.code, Some(4001));
            assert_eq!(data.message.as_deref(), Some("Invalid symbol"));
        } else {
            panic!("Expected Error event");
        }
    }

    #[test]
    fn test_parse_pong_event() {
        let json = r#"{"event": "pong", "state": "ok"}"#;
        let msg: StreamMessage = serde_json::from_str(json).unwrap();
        if let StreamMessage::Pong { state } = msg {
            assert_eq!(state.as_deref(), Some("ok"));
        } else {
            panic!("Expected Pong event");
        }
    }

    #[test]
    fn test_parse_pong_without_state() {
        let json = r#"{"event": "pong"}"#;
        let msg: StreamMessage = serde_json::from_str(json).unwrap();
        if let StreamMessage::Pong { state } = msg {
            assert!(state.is_none());
        } else {
            panic!("Expected Pong event");
        }
    }

    #[test]
    fn test_parse_candles_snapshot() {
        let json = r#"{
            "symbol": "2330",
            "date": "2026-01-30",
            "timeframe": "1",
            "data": [
                {"date": "2026-01-30T09:00:00.000+08:00", "open": 580.0, "high": 581.0, "low": 579.0, "close": 580.5, "volume": 1000},
                {"date": "2026-01-30T09:01:00.000+08:00", "open": 580.5, "high": 582.0, "low": 580.0, "close": 581.5, "volume": 1500}
            ]
        }"#;
        let snapshot: CandlesSnapshot = serde_json::from_str(json).unwrap();
        assert_eq!(snapshot.symbol, "2330");
        assert_eq!(snapshot.date, "2026-01-30");
        assert_eq!(snapshot.timeframe.as_deref(), Some("1"));
        assert_eq!(snapshot.data.len(), 2);
        assert_eq!(snapshot.data[0].open, 580.0);
        assert_eq!(snapshot.data[1].volume, 1500);
    }

    #[test]
    fn test_parse_trades_data() {
        let json = r#"{
            "symbol": "2330",
            "type": "EQUITY",
            "trades": [{"price": 583.0, "size": 100, "bid": 582.0, "ask": 583.0}],
            "total": {"tradeVolume": 12345678, "tradeValue": 7201234567.0}
        }"#;
        let trades: TradesData = serde_json::from_str(json).unwrap();
        assert_eq!(trades.symbol, "2330");
        assert_eq!(trades.data_type.as_deref(), Some("EQUITY"));
        assert_eq!(trades.trades.len(), 1);
        assert_eq!(trades.trades[0].price, 583.0);
        assert_eq!(trades.trades[0].bid, Some(582.0));
        assert!(trades.total.is_some());
    }

    #[test]
    fn test_parse_candle_data() {
        let json = r#"{
            "symbol": "2330",
            "date": "2026-01-30T09:15:00.000+08:00",
            "open": 580.0,
            "high": 585.0,
            "low": 578.0,
            "close": 583.0,
            "volume": 12345,
            "average": 581.5
        }"#;
        let candle: CandleData = serde_json::from_str(json).unwrap();
        assert_eq!(candle.symbol, "2330");
        assert_eq!(candle.open, 580.0);
        assert_eq!(candle.close, 583.0);
        assert_eq!(candle.average, Some(581.5));
    }

    #[test]
    fn test_parse_books_data() {
        let json = r#"{
            "symbol": "2330",
            "bids": [{"price": 582.0, "size": 100}],
            "asks": [{"price": 583.0, "size": 50}],
            "time": 1704067200123456
        }"#;
        let books: BooksData = serde_json::from_str(json).unwrap();
        assert_eq!(books.symbol, "2330");
        assert_eq!(books.bids.len(), 1);
        assert_eq!(books.asks.len(), 1);
        assert_eq!(books.bids[0].price, 582.0);
        assert_eq!(books.asks[0].size, 50);
    }

    #[test]
    fn test_parse_indices_data() {
        let json = r#"{
            "symbol": "IX0001",
            "type": "INDEX",
            "index": 17500.5,
            "time": 1704067200123456
        }"#;
        let indices: IndicesData = serde_json::from_str(json).unwrap();
        assert_eq!(indices.symbol, "IX0001");
        assert_eq!(indices.data_type.as_deref(), Some("INDEX"));
        assert_eq!(indices.index, Some(17500.5));
    }

    #[test]
    fn test_parse_aggregates_data() {
        let json = r#"{
            "symbol": "2330",
            "type": "EQUITY",
            "openPrice": 580.0,
            "highPrice": 590.0,
            "lowPrice": 578.0,
            "closePrice": 585.0,
            "lastPrice": 585.0,
            "lastSize": 100,
            "bids": [{"price": 584.0, "size": 500}],
            "asks": [{"price": 585.0, "size": 300}]
        }"#;
        let agg: AggregatesData = serde_json::from_str(json).unwrap();
        assert_eq!(agg.symbol, "2330");
        assert_eq!(agg.open_price, Some(580.0));
        assert_eq!(agg.last_price, Some(585.0));
        assert_eq!(agg.bids.len(), 1);
        assert_eq!(agg.asks.len(), 1);
    }
}
