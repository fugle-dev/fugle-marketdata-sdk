//! FutOpt quote data model - matches Fugle futopt/intraday/quote/{symbol} response

use serde::{Deserialize, Serialize};

/// Bid/Ask price level for FutOpt order book
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct FutOptPriceLevel {
    /// Price at this level
    pub price: f64,
    /// Size (volume) at this level
    pub size: i64,
}

/// Total trading statistics for FutOpt
///
/// Every field past `tradeVolume` is optional: which of them the server sends
/// varies by endpoint and by session, and a missing field must never fail the
/// whole decode.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct FutOptTotalStats {
    /// Total trade volume
    #[serde(rename = "tradeVolume", default)]
    pub trade_volume: i64,
    /// Total traded value
    #[serde(rename = "tradeValue", default)]
    pub trade_value: Option<f64>,
    /// Total volume matched at bid price
    #[serde(rename = "totalBidMatch")]
    pub total_bid_match: Option<i64>,
    /// Total volume matched at ask price
    #[serde(rename = "totalAskMatch")]
    pub total_ask_match: Option<i64>,
    /// Volume traded at the bid
    #[serde(rename = "tradeVolumeAtBid")]
    pub trade_volume_at_bid: Option<i64>,
    /// Volume traded at the ask
    #[serde(rename = "tradeVolumeAtAsk")]
    pub trade_volume_at_ask: Option<i64>,
    /// Number of transactions
    pub transaction: Option<i64>,
    /// Timestamp (Unix milliseconds)
    pub time: Option<i64>,
}

/// Last trade information for FutOpt
///
/// Also used for `lastTrial`, which carries the same shape.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct FutOptLastTrade {
    /// Best bid price at trade time
    pub bid: Option<f64>,
    /// Best ask price at trade time
    pub ask: Option<f64>,
    /// Trade price
    #[serde(default)]
    pub price: f64,
    /// Trade size
    #[serde(default)]
    pub size: i64,
    /// Trade timestamp (Unix milliseconds)
    #[serde(default)]
    pub time: i64,
    /// Exchange sequence number for this trade.
    ///
    /// A **zero-padded string** (`"00379320"`) on futopt, despite the official
    /// TypeScript interface declaring `serial: number`. Verified against a
    /// live payload; the padding is fixed-width and significant, so parsing to
    /// an integer would discard it.
    ///
    /// Note this differs from [`FutOptQuote::serial`] on the *same* response,
    /// which really is a number.
    #[serde(default, deserialize_with = "crate::models::common::deserialize_serial")]
    pub serial: Option<String>,
}

/// Daily price limits and the reference prices they are derived from.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct FutOptPriceLimits {
    /// Limit on the traded price
    pub price: Option<f64>,
    /// Limit on the bid side
    pub bid: Option<f64>,
    /// Limit on the ask side
    pub ask: Option<f64>,
    /// Circuit-breaker (curb) level
    pub curb: Option<f64>,
}

/// Trading halt status for FutOpt
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct FutOptTradingHalt {
    /// Whether trading is currently halted
    #[serde(rename = "isHalted", default)]
    pub is_halted: bool,
    /// Timestamp of the halt state (Unix milliseconds)
    pub time: Option<i64>,
}

/// Real-time FutOpt quote from Fugle API (futopt/intraday/quote/{symbol})
///
/// Matches the official SDK's `RestFutOptIntradayQuoteResponse`.
///
/// Every field but `date` and `symbol` is optional or `#[serde(default)]`.
/// Which fields the server actually sends varies by contract, by session and
/// by whether a trial (試撮) is running, and a payload that omits one must not
/// fail the whole decode.
///
/// # Example
///
/// ```rust
/// use marketdata_core::models::futopt::FutOptQuote;
///
/// let json = r#"{
///     "date": "2026-08-05",
///     "type": "FUTURE",
///     "exchange": "TAIFEX",
///     "market": "FUTURES",
///     "symbol": "TXFH6",
///     "openPrice": 17520.0,
///     "highPrice": 17580.0,
///     "lowPrice": 17480.0,
///     "lastPrice": 17550.0,
///     "lastSize": 2,
///     "avgPrice": 17530.0,
///     "change": 50.0,
///     "changePercent": 0.29,
///     "amplitude": 0.57,
///     "bids": [{"price": 17549.0, "size": 50}],
///     "asks": [{"price": 17550.0, "size": 30}],
///     "total": {
///         "tradeValue": 8765000000.0,
///         "tradeVolume": 50000,
///         "tradeVolumeAtBid": 24000,
///         "tradeVolumeAtAsk": 26000,
///         "transaction": 31234,
///         "time": 1785900000000
///     },
///     "priceLimits": {"price": 19250.0, "bid": 19250.0, "ask": 15750.0, "curb": 0.0},
///     "lastTrade": {
///         "bid": 17549.0, "ask": 17550.0, "price": 17550.0,
///         "size": 2, "time": 1785900000000, "serial": "00981234"
///     },
///     "tradingHalt": {"isHalted": false, "time": 0},
///     "isContinuous": true,
///     "isOpen": true,
///     "serial": 981234,
///     "lastUpdated": 1785900000000
/// }"#;
///
/// let quote: FutOptQuote = serde_json::from_str(json).unwrap();
/// assert_eq!(quote.symbol, "TXFH6");
/// assert_eq!(quote.last_price, Some(17550.0));
/// assert!(!quote.is_trial, "no trial session in this payload");
/// assert_eq!(quote.price_limits.unwrap().price, Some(19250.0));
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct FutOptQuote {
    // === Response metadata ===
    /// Trading date (YYYY-MM-DD)
    pub date: String,

    /// Contract type (FUTURE or OPTION)
    #[serde(rename = "type")]
    pub contract_type: Option<String>,

    /// Exchange code (TAIFEX)
    pub exchange: Option<String>,

    /// Market
    pub market: Option<String>,

    /// Contract symbol (e.g., "TXFC4", "TXO18000C4")
    pub symbol: String,

    /// Contract name
    ///
    /// The official SDK dropped this field in 1.5.0; it is kept here as an
    /// `Option` so payloads that still carry it keep decoding.
    pub name: Option<String>,

    // === Reference prices ===
    /// Previous close price
    ///
    /// The official SDK dropped this field in 1.5.0; it is kept here as an
    /// `Option` so payloads that still carry it keep decoding.
    #[serde(rename = "previousClose")]
    pub previous_close: Option<f64>,

    /// Daily price limits
    #[serde(rename = "priceLimits")]
    pub price_limits: Option<FutOptPriceLimits>,

    // === OHLC prices with timestamps ===
    /// Open price
    #[serde(rename = "openPrice")]
    pub open_price: Option<f64>,

    /// Open time (Unix milliseconds)
    #[serde(rename = "openTime")]
    pub open_time: Option<i64>,

    /// High price
    #[serde(rename = "highPrice")]
    pub high_price: Option<f64>,

    /// High time (Unix milliseconds)
    #[serde(rename = "highTime")]
    pub high_time: Option<i64>,

    /// Low price
    #[serde(rename = "lowPrice")]
    pub low_price: Option<f64>,

    /// Low time (Unix milliseconds)
    #[serde(rename = "lowTime")]
    pub low_time: Option<i64>,

    /// Close price
    #[serde(rename = "closePrice")]
    pub close_price: Option<f64>,

    /// Close time (Unix milliseconds)
    #[serde(rename = "closeTime")]
    pub close_time: Option<i64>,

    // === Current trading info ===
    /// Last traded price
    #[serde(rename = "lastPrice")]
    pub last_price: Option<f64>,

    /// Last traded size (number of contracts)
    #[serde(rename = "lastSize")]
    pub last_size: Option<i64>,

    /// Average price
    #[serde(rename = "avgPrice")]
    pub avg_price: Option<f64>,

    /// Price change from previous close
    pub change: Option<f64>,

    /// Percentage change from previous close
    #[serde(rename = "changePercent")]
    pub change_percent: Option<f64>,

    /// Price amplitude (high - low) / reference price * 100
    pub amplitude: Option<f64>,

    // === Order book ===
    /// Bid price levels (best to worst)
    #[serde(default)]
    pub bids: Vec<FutOptPriceLevel>,

    /// Ask price levels (best to worst)
    #[serde(default)]
    pub asks: Vec<FutOptPriceLevel>,

    // === Aggregated stats ===
    /// Total trading statistics
    pub total: Option<FutOptTotalStats>,

    /// Last trade information
    #[serde(rename = "lastTrade")]
    pub last_trade: Option<FutOptLastTrade>,

    /// Last trial match (試撮). Absent outside a trial session.
    #[serde(rename = "lastTrial")]
    pub last_trial: Option<FutOptLastTrade>,

    /// Trading halt status
    #[serde(rename = "tradingHalt")]
    pub trading_halt: Option<FutOptTradingHalt>,

    // === Session flags ===
    /// Marks the quote as trial-matching (試撮) — a simulated match, not a
    /// trade. Omitted rather than sent as `false` outside a trial session,
    /// during which `last_price` / `last_size` are the trial values.
    ///
    /// Branch on this before acting on a price.
    #[serde(
        rename = "isTrial",
        default,
        deserialize_with = "crate::models::common::deserialize_bool_lenient"
    )]
    pub is_trial: bool,

    /// Is delayed open
    #[serde(rename = "isDelayedOpen", default)]
    pub is_delayed_open: bool,

    /// Is delayed close
    #[serde(rename = "isDelayedClose", default)]
    pub is_delayed_close: bool,

    /// Is in continuous trading
    #[serde(rename = "isContinuous", default)]
    pub is_continuous: bool,

    /// Is the session open
    #[serde(rename = "isOpen", default)]
    pub is_open: bool,

    /// Is the session closed
    #[serde(rename = "isClose", default)]
    pub is_close: bool,

    /// Exchange sequence number for this quote
    pub serial: Option<i64>,

    /// Last updated timestamp (Unix milliseconds)
    #[serde(rename = "lastUpdated")]
    pub last_updated: Option<i64>,
}

impl FutOptQuote {
    /// Get the bid-ask spread
    pub fn spread(&self) -> Option<f64> {
        let best_bid = self.bids.first().map(|l| l.price);
        let best_ask = self.asks.first().map(|l| l.price);
        match (best_ask, best_bid) {
            (Some(ask), Some(bid)) => Some(ask - bid),
            _ => None,
        }
    }

    /// Get the mid price between best bid and ask
    pub fn mid_price(&self) -> Option<f64> {
        let best_bid = self.bids.first().map(|l| l.price);
        let best_ask = self.asks.first().map(|l| l.price);
        match (best_ask, best_bid) {
            (Some(ask), Some(bid)) => Some((ask + bid) / 2.0),
            _ => None,
        }
    }

    /// Check if quote has essential price data
    pub fn has_price_data(&self) -> bool {
        self.last_price.is_some() || self.close_price.is_some()
    }

    /// Get the total trade volume
    pub fn total_volume(&self) -> Option<i64> {
        self.total.as_ref().map(|t| t.trade_volume)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_futopt_quote_deserialization() {
        let json = r#"{
            "date": "2024-01-15",
            "type": "FUTURE",
            "exchange": "TAIFEX",
            "symbol": "TXFC4",
            "name": "臺股期貨",
            "previousClose": 17500.0,
            "openPrice": 17520.0,
            "openTime": 1705287000000,
            "highPrice": 17580.0,
            "highTime": 1705290600000,
            "lowPrice": 17480.0,
            "lowTime": 1705288800000,
            "closePrice": 17550.0,
            "closeTime": 1705302000000,
            "lastPrice": 17550.0,
            "lastSize": 2,
            "avgPrice": 17530.0,
            "change": 50.0,
            "changePercent": 0.29,
            "amplitude": 0.57,
            "bids": [
                {"price": 17549.0, "size": 50},
                {"price": 17548.0, "size": 30}
            ],
            "asks": [
                {"price": 17550.0, "size": 30},
                {"price": 17551.0, "size": 40}
            ],
            "total": {
                "tradeVolume": 50000,
                "totalBidMatch": 25000,
                "totalAskMatch": 25000
            },
            "lastTrade": {
                "price": 17550.0,
                "size": 2,
                "time": 1705302000000
            },
            "lastUpdated": 1705302000000
        }"#;

        let quote: FutOptQuote = serde_json::from_str(json).unwrap();
        assert_eq!(quote.symbol, "TXFC4");
        assert_eq!(quote.name.as_deref(), Some("臺股期貨"));
        assert_eq!(quote.contract_type.as_deref(), Some("FUTURE"));
        assert_eq!(quote.exchange.as_deref(), Some("TAIFEX"));
        assert_eq!(quote.last_price, Some(17550.0));
        assert_eq!(quote.previous_close, Some(17500.0));
        assert_eq!(quote.change, Some(50.0));
        assert_eq!(quote.bids.len(), 2);
        assert_eq!(quote.asks.len(), 2);
        assert_eq!(quote.bids[0].price, 17549.0);
        assert_eq!(quote.asks[0].price, 17550.0);
        assert_eq!(quote.total_volume(), Some(50000));
    }

    #[test]
    fn test_futopt_quote_spread() {
        let quote = FutOptQuote {
            bids: vec![FutOptPriceLevel {
                price: 17549.0,
                size: 50,
            }],
            asks: vec![FutOptPriceLevel {
                price: 17550.0,
                size: 30,
            }],
            ..Default::default()
        };
        assert_eq!(quote.spread(), Some(1.0));
    }

    #[test]
    fn test_futopt_quote_mid_price() {
        let quote = FutOptQuote {
            bids: vec![FutOptPriceLevel {
                price: 100.0,
                size: 10,
            }],
            asks: vec![FutOptPriceLevel {
                price: 102.0,
                size: 10,
            }],
            ..Default::default()
        };
        assert_eq!(quote.mid_price(), Some(101.0));
    }

    #[test]
    fn test_futopt_quote_minimal() {
        let json = r#"{"date": "2024-01-15", "symbol": "TXFC4"}"#;
        let quote: FutOptQuote = serde_json::from_str(json).unwrap();
        assert_eq!(quote.symbol, "TXFC4");
        assert!(quote.bids.is_empty());
        assert!(quote.asks.is_empty());
        assert!(!quote.has_price_data());
    }

    #[test]
    fn test_futopt_quote_has_price_data() {
        let mut quote = FutOptQuote::default();
        assert!(!quote.has_price_data());

        quote.last_price = Some(17550.0);
        assert!(quote.has_price_data());

        quote.last_price = None;
        quote.close_price = Some(17550.0);
        assert!(quote.has_price_data());
    }

    #[test]
    fn test_futopt_total_stats_deserialization() {
        let json = r#"{"tradeVolume": 50000, "totalBidMatch": 25000, "totalAskMatch": 25000}"#;
        let stats: FutOptTotalStats = serde_json::from_str(json).unwrap();
        assert_eq!(stats.trade_volume, 50000);
        assert_eq!(stats.total_bid_match, Some(25000));
        assert_eq!(stats.total_ask_match, Some(25000));
    }

    #[test]
    fn test_futopt_total_stats_new_fields() {
        let json = r#"{
            "tradeValue": 8765000000.0,
            "tradeVolume": 50000,
            "tradeVolumeAtBid": 24000,
            "tradeVolumeAtAsk": 26000,
            "transaction": 31234,
            "time": 1785900000000
        }"#;
        let stats: FutOptTotalStats = serde_json::from_str(json).unwrap();
        assert_eq!(stats.trade_value, Some(8765000000.0));
        assert_eq!(stats.trade_volume_at_bid, Some(24000));
        assert_eq!(stats.trade_volume_at_ask, Some(26000));
        assert_eq!(stats.transaction, Some(31234));
        assert_eq!(stats.time, Some(1785900000000));
    }

    #[test]
    fn test_futopt_total_stats_tolerates_every_field_missing() {
        // Which of these the server sends varies by endpoint and session.
        // Decoding must survive an empty object rather than failing the whole
        // quote — this is the 0.7.2 lesson encoded as a test.
        let stats: FutOptTotalStats = serde_json::from_str("{}").unwrap();
        assert_eq!(stats.trade_volume, 0);
        assert_eq!(stats.trade_value, None);
    }

    #[test]
    fn test_futopt_last_trade_deserialization() {
        let json = r#"{"price": 17550.0, "size": 2, "time": 1705302000000}"#;
        let trade: FutOptLastTrade = serde_json::from_str(json).unwrap();
        assert_eq!(trade.price, 17550.0);
        assert_eq!(trade.size, 2);
        assert_eq!(trade.time, 1705302000000);
        assert_eq!(trade.bid, None);
        assert_eq!(trade.serial, None);
    }

    #[test]
    fn test_futopt_last_trade_with_bid_ask_serial() {
        // Shape taken from a live futopt/intraday/quote payload: the nested
        // trade serial is a ZERO-PADDED STRING, even though the official
        // TypeScript interface declares `serial: number`.
        let json = r#"{
            "bid": 44906.0, "ask": 45777.0, "price": 45777.0,
            "size": 1, "time": 1785901265049000, "serial": "00379320"
        }"#;
        let trade: FutOptLastTrade = serde_json::from_str(json).unwrap();
        assert_eq!(trade.bid, Some(44906.0));
        assert_eq!(trade.ask, Some(45777.0));
        assert_eq!(
            trade.serial.as_deref(),
            Some("00379320"),
            "leading zeros are fixed-width and must survive decoding"
        );
    }

    #[test]
    fn test_futopt_last_trade_serial_accepts_a_number_too() {
        // Stock sends a number for the same field. Accepting both is what
        // lets one type serve the futopt REST quote and the streaming
        // `aggregates` frame that mirrors it.
        let json = r#"{"price": 1.0, "size": 1, "time": 1, "serial": 17738549}"#;
        let trade: FutOptLastTrade = serde_json::from_str(json).unwrap();
        assert_eq!(trade.serial.as_deref(), Some("17738549"));
    }

    #[test]
    fn test_futopt_quote_trial_session() {
        // During a trial, `lastPrice` / `lastSize` are the trial values and
        // only `isTrial` distinguishes them from a real trade.
        let json = r#"{
            "date": "2026-08-05",
            "symbol": "TXFH6",
            "lastPrice": 17550.0,
            "lastSize": 2,
            "isTrial": true,
            "lastTrial": {"price": 17550.0, "size": 2, "time": 1785900000000, "serial": "00000005"}
        }"#;
        let quote: FutOptQuote = serde_json::from_str(json).unwrap();
        assert!(quote.is_trial);
        assert_eq!(quote.last_trial.unwrap().serial.as_deref(), Some("00000005"));
    }

    #[test]
    fn test_futopt_quote_is_trial_defaults_false_when_omitted() {
        // The server omits `isTrial` entirely rather than sending false.
        let json = r#"{"date": "2026-08-05", "symbol": "TXFH6"}"#;
        let quote: FutOptQuote = serde_json::from_str(json).unwrap();
        assert!(!quote.is_trial);
    }

    #[test]
    fn test_futopt_quote_session_flags_and_serial() {
        let json = r#"{
            "date": "2026-08-05",
            "symbol": "TXFH6",
            "market": "FUTURES",
            "isDelayedOpen": true,
            "isContinuous": true,
            "isOpen": true,
            "serial": 981234,
            "tradingHalt": {"isHalted": true, "time": 1785900000000}
        }"#;
        let quote: FutOptQuote = serde_json::from_str(json).unwrap();
        assert_eq!(quote.market.as_deref(), Some("FUTURES"));
        assert!(quote.is_delayed_open);
        assert!(quote.is_continuous);
        assert!(quote.is_open);
        assert!(!quote.is_close);
        assert_eq!(quote.serial, Some(981234));

        let halt = quote.trading_halt.unwrap();
        assert!(halt.is_halted);
        assert_eq!(halt.time, Some(1785900000000));
    }

    #[test]
    fn test_futopt_price_limits() {
        let json = r#"{"price": 19250.0, "bid": 19250.0, "ask": 15750.0, "curb": 0.0}"#;
        let limits: FutOptPriceLimits = serde_json::from_str(json).unwrap();
        assert_eq!(limits.price, Some(19250.0));
        assert_eq!(limits.bid, Some(19250.0));
        assert_eq!(limits.ask, Some(15750.0));
        assert_eq!(limits.curb, Some(0.0));
    }

    #[test]
    fn test_futopt_quote_dropped_fields_still_decode() {
        // The official SDK dropped `name` / `previousClose` in 1.5.0. We keep
        // them so a payload that still carries them does not fail to decode.
        let json = r#"{
            "date": "2026-08-05",
            "symbol": "TXFH6",
            "name": "臺股期貨",
            "previousClose": 17500.0
        }"#;
        let quote: FutOptQuote = serde_json::from_str(json).unwrap();
        assert_eq!(quote.name.as_deref(), Some("臺股期貨"));
        assert_eq!(quote.previous_close, Some(17500.0));
    }
}
