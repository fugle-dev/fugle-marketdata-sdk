//! FutOpt-specific subscription types
//!
//! Accepts either a single symbol or a batch via `impl Into<Symbols>`.
//! Distinct from [`StockSubscription`](super::StockSubscription) in two ways:
//!
//! - Uses [`FutOptChannel`] instead of [`Channel`](crate::models::Channel)
//!   (no `Indices` channel)
//! - Modifier is `after_hours` (盤後) instead of `intraday_odd_lot` (盤中零股)

use crate::models::futopt::FutOptChannel;
use crate::models::Symbols;
use serde_json::{json, Value};

/// FutOpt-specific subscription parameters.
///
/// Supports `afterHours` for after-hours sessions (盤後).
///
/// # Example
///
/// ```rust
/// use marketdata_core::models::futopt::FutOptChannel;
/// use marketdata_core::websocket::channels::FutOptSubscription;
///
/// // Single contract
/// let sub = FutOptSubscription::new(FutOptChannel::Trades, "TXFC4");
/// assert_eq!(sub.keys(), vec!["trades:TXFC4".to_string()]);
///
/// // Batch
/// let batch = FutOptSubscription::new(FutOptChannel::Trades, vec!["TXFC4", "MXFC4"]);
/// assert_eq!(batch.keys(), vec!["trades:TXFC4".to_string(), "trades:MXFC4".to_string()]);
///
/// // After-hours session
/// let ah = FutOptSubscription::new(FutOptChannel::Trades, "TXFC4").with_after_hours(true);
/// assert_eq!(ah.keys(), vec!["trades:TXFC4:afterhours".to_string()]);
/// ```
#[derive(Debug, Clone)]
pub struct FutOptSubscription {
    /// Channel to subscribe to.
    pub channel: FutOptChannel,
    /// One or many contract symbols.
    pub symbols: Symbols,
    /// true: 盤後 (after-hours), false: 一般盤 (regular, default).
    pub after_hours: bool,
}

impl FutOptSubscription {
    /// Create a FutOpt subscription. Accepts the same input shapes as
    /// [`StockSubscription::new`](super::StockSubscription::new).
    pub fn new(channel: FutOptChannel, symbols: impl Into<Symbols>) -> Self {
        Self {
            channel,
            symbols: symbols.into(),
            after_hours: false,
        }
    }

    /// Set the after-hours session flag (applies to every symbol in a batch).
    pub fn with_after_hours(mut self, after_hours: bool) -> Self {
        self.after_hours = after_hours;
        self
    }

    /// Generate one local key per symbol (length 1 for `Single`, N for `Many`).
    pub fn keys(&self) -> Vec<String> {
        match &self.symbols {
            Symbols::Single(s) => vec![self.format_key(s)],
            Symbols::Many(v) => v.iter().map(|s| self.format_key(s)).collect(),
        }
    }

    fn format_key(&self, symbol: &str) -> String {
        if self.after_hours {
            format!("{}:{}:afterhours", self.channel.as_str(), symbol)
        } else {
            format!("{}:{}", self.channel.as_str(), symbol)
        }
    }

    /// Wire-format data field for the subscribe message.
    ///
    /// Routes to `{"channel": ..., "symbol": ...}` for `Single` or
    /// `{"channel": ..., "symbols": [...]}` for `Many`. Adds
    /// `afterHours: true` when set.
    pub fn to_subscribe_data(&self) -> Value {
        let mut data = json!({ "channel": self.channel.as_str() });
        match &self.symbols {
            Symbols::Single(s) => data["symbol"] = json!(s),
            Symbols::Many(v) => data["symbols"] = json!(v),
        }
        if self.after_hours {
            data["afterHours"] = json!(true);
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
        let sub = FutOptSubscription::new(FutOptChannel::Trades, "TXF202502");
        assert!(matches!(sub.symbols, Symbols::Single(ref s) if s == "TXF202502"));
        assert!(!sub.after_hours);
    }

    #[test]
    fn new_batch_symbols() {
        let sub = FutOptSubscription::new(FutOptChannel::Trades, vec!["TXFC4", "MXFC4"]);
        assert!(matches!(sub.symbols, Symbols::Many(ref v) if v == &["TXFC4", "MXFC4"]));
    }

    #[test]
    fn keys_returns_per_symbol_entries() {
        let single = FutOptSubscription::new(FutOptChannel::Trades, "TXFC4");
        assert_eq!(single.keys(), vec!["trades:TXFC4".to_string()]);

        let batch = FutOptSubscription::new(FutOptChannel::Books, ["TXFC4", "MXFC4"]);
        assert_eq!(
            batch.keys(),
            vec!["books:TXFC4".to_string(), "books:MXFC4".to_string()]
        );
    }

    #[test]
    fn after_hours_modifier_applies_to_all_keys() {
        let batch = FutOptSubscription::new(FutOptChannel::Trades, ["TXFC4", "MXFC4"])
            .with_after_hours(true);
        assert_eq!(
            batch.keys(),
            vec![
                "trades:TXFC4:afterhours".to_string(),
                "trades:MXFC4:afterhours".to_string()
            ]
        );
    }

    #[test]
    fn to_subscribe_data_single_uses_symbol_field() {
        let sub = FutOptSubscription::new(FutOptChannel::Candles, "MXFB4");
        let data = sub.to_subscribe_data();
        assert_eq!(data["channel"], "candles");
        assert_eq!(data["symbol"], "MXFB4");
        assert!(data.get("symbols").is_none());
        assert!(data.get("afterHours").is_none());
    }

    #[test]
    fn to_subscribe_data_batch_uses_symbols_field() {
        let sub = FutOptSubscription::new(FutOptChannel::Aggregates, vec!["TXFC4", "MXFC4"]);
        let data = sub.to_subscribe_data();
        assert_eq!(data["symbols"], json!(["TXFC4", "MXFC4"]));
        assert!(data.get("symbol").is_none());
    }

    #[test]
    fn to_subscribe_data_with_after_hours() {
        let sub = FutOptSubscription::new(FutOptChannel::Trades, "TXFC4").with_after_hours(true);
        let data = sub.to_subscribe_data();
        assert_eq!(data["symbol"], "TXFC4");
        assert_eq!(data["afterHours"], true);
    }
}
