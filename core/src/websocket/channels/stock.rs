//! Stock-specific subscription types

use crate::models::{Channel, SymbolSpec};
use serde_json::{json, Value};

/// Stock-specific subscription parameters.
///
/// Accepts either a single symbol or a batch via `impl Into<SymbolSpec>`.
/// Supports `intradayOddLot` for odd-lot sessions.
///
/// # Example
/// ```rust
/// use marketdata_core::models::Channel;
/// use marketdata_core::websocket::channels::StockSubscription;
///
/// // Single symbol
/// let sub = StockSubscription::new(Channel::Trades, "2330");
/// assert_eq!(sub.keys(), vec!["trades:2330".to_string()]);
///
/// // Batch — one frame, N internal entries
/// let batch = StockSubscription::new(Channel::Trades, vec!["2330", "2454"]);
/// assert_eq!(batch.keys(), vec!["trades:2330".to_string(), "trades:2454".to_string()]);
///
/// // Odd-lot session (modifier applies to every symbol)
/// let odd = StockSubscription::new(Channel::Trades, "2330").with_odd_lot(true);
/// assert_eq!(odd.keys(), vec!["trades:2330:oddlot".to_string()]);
/// ```
#[derive(Debug, Clone)]
pub struct StockSubscription {
    /// Channel to subscribe to.
    pub channel: Channel,
    /// One or many symbols.
    pub symbols: SymbolSpec,
    /// true: 盤中零股, false: 股票 (default).
    pub intraday_odd_lot: bool,
}

impl StockSubscription {
    /// Create a stock subscription. Accepts `&str`, `String`, `Vec<String>`,
    /// array literals (`["A", "B"]`), and slices — see [`SymbolSpec`] for
    /// the full set of `From` impls.
    pub fn new(channel: Channel, symbols: impl Into<SymbolSpec>) -> Self {
        Self {
            channel,
            symbols: symbols.into(),
            intraday_odd_lot: false,
        }
    }

    /// Set the odd-lot session flag (applies to every symbol in a batch).
    pub fn with_odd_lot(mut self, odd_lot: bool) -> Self {
        self.intraday_odd_lot = odd_lot;
        self
    }

    /// Generate one local key per symbol (length 1 for `Single`, N for `Many`).
    ///
    /// Keys are used by `SubscriptionManager` to bookkeep server-id mapping
    /// and unsubscribe lookup. Each batch symbol owns its own row.
    pub fn keys(&self) -> Vec<String> {
        match &self.symbols {
            SymbolSpec::Single(s) => vec![self.format_key(s)],
            SymbolSpec::Many(v) => v.iter().map(|s| self.format_key(s)).collect(),
        }
    }

    fn format_key(&self, symbol: &str) -> String {
        if self.intraday_odd_lot {
            format!("{}:{}:oddlot", self.channel.as_str(), symbol)
        } else {
            format!("{}:{}", self.channel.as_str(), symbol)
        }
    }

    /// Wire-format data field for the subscribe message.
    ///
    /// Routes to `{"channel": ..., "symbol": ...}` for `Single` or
    /// `{"channel": ..., "symbols": [...]}` for `Many`. Adds
    /// `intradayOddLot: true` when set.
    pub fn to_subscribe_data(&self) -> Value {
        let mut data = json!({ "channel": self.channel.as_str() });
        match &self.symbols {
            SymbolSpec::Single(s) => data["symbol"] = json!(s),
            SymbolSpec::Many(v) => data["symbols"] = json!(v),
        }
        if self.intraday_odd_lot {
            data["intradayOddLot"] = json!(true);
        }
        data
    }

    /// Full subscribe request envelope.
    pub fn to_subscribe_request(&self) -> Value {
        json!({
            "event": "subscribe",
            "data": self.to_subscribe_data()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_single_symbol() {
        let sub = StockSubscription::new(Channel::Trades, "2330");
        assert_eq!(sub.channel, Channel::Trades);
        assert!(matches!(sub.symbols, SymbolSpec::Single(ref s) if s == "2330"));
        assert!(!sub.intraday_odd_lot);
    }

    #[test]
    fn new_batch_symbols() {
        let sub = StockSubscription::new(Channel::Trades, vec!["2330", "2454"]);
        assert!(matches!(sub.symbols, SymbolSpec::Many(ref v) if v == &["2330", "2454"]));
    }

    #[test]
    fn keys_returns_per_symbol_entries() {
        let single = StockSubscription::new(Channel::Trades, "2330");
        assert_eq!(single.keys(), vec!["trades:2330".to_string()]);

        let batch = StockSubscription::new(Channel::Books, ["2330", "2454", "2317"]);
        assert_eq!(
            batch.keys(),
            vec![
                "books:2330".to_string(),
                "books:2454".to_string(),
                "books:2317".to_string()
            ]
        );
    }

    #[test]
    fn odd_lot_modifier_applies_to_all_keys() {
        let batch =
            StockSubscription::new(Channel::Trades, ["2330", "2454"]).with_odd_lot(true);
        assert_eq!(
            batch.keys(),
            vec![
                "trades:2330:oddlot".to_string(),
                "trades:2454:oddlot".to_string()
            ]
        );
    }

    #[test]
    fn to_subscribe_data_single_uses_symbol_field() {
        let sub = StockSubscription::new(Channel::Candles, "2454");
        let data = sub.to_subscribe_data();
        assert_eq!(data["channel"], "candles");
        assert_eq!(data["symbol"], "2454");
        assert!(data.get("symbols").is_none());
        assert!(data.get("intradayOddLot").is_none());
    }

    #[test]
    fn to_subscribe_data_batch_uses_symbols_field() {
        let sub = StockSubscription::new(Channel::Aggregates, vec!["2330", "0050", "2603"]);
        let data = sub.to_subscribe_data();
        assert_eq!(data["channel"], "aggregates");
        assert_eq!(data["symbols"], json!(["2330", "0050", "2603"]));
        assert!(data.get("symbol").is_none());
    }

    #[test]
    fn to_subscribe_data_with_odd_lot() {
        let sub = StockSubscription::new(Channel::Trades, "2330").with_odd_lot(true);
        let data = sub.to_subscribe_data();
        assert_eq!(data["symbol"], "2330");
        assert_eq!(data["intradayOddLot"], true);
    }
}
