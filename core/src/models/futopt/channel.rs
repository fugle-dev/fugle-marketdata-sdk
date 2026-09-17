//! FutOpt WebSocket channel types
//!
//! This module defines the available WebSocket channels for futures and options
//! market data streaming.
//!
//! # Differences from Stock Channels
//!
//! FutOpt channels are similar to stock channels but **do not include Indices**.
//! The indices channel is stock-market specific (e.g., TAIEX).
//!
//! # Example
//!
//! ```rust
//! use marketdata_core::models::futopt::FutOptChannel;
//!
//! let channel = FutOptChannel::Trades;
//! assert_eq!(channel.as_str(), "trades");
//!
//! // Serialize for API requests
//! let json = serde_json::to_string(&channel).unwrap();
//! assert_eq!(json, r#""trades""#);
//! ```

use serde::{Deserialize, Serialize};

/// FutOpt WebSocket channels
///
/// Available channels for futures and options market data streaming.
///
/// # Variants
///
/// - `Trades` - Real-time trade data (成交明細)
/// - `Books` - Order book depth with bid/ask levels (五檔報價)
/// - `Candles` - OHLCV candlestick data (K線)
/// - `Aggregates` - Aggregated market statistics (統計資訊)
///
/// # Note
///
/// FutOpt does not have an `Indices` channel. The indices channel is
/// stock-market specific (e.g., TAIEX weighted index).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FutOptChannel {
    /// Real-time trade data (成交明細)
    Trades,
    /// Order book depth with bid/ask levels (五檔報價)
    Books,
    /// OHLCV candlestick data (K線)
    Candles,
    /// Aggregated market statistics (統計資訊)
    Aggregates,
}

impl FutOptChannel {
    /// Get the string representation for WebSocket wire format
    ///
    /// # Example
    ///
    /// ```rust
    /// use marketdata_core::models::futopt::FutOptChannel;
    ///
    /// assert_eq!(FutOptChannel::Trades.as_str(), "trades");
    /// assert_eq!(FutOptChannel::Books.as_str(), "books");
    /// assert_eq!(FutOptChannel::Candles.as_str(), "candles");
    /// assert_eq!(FutOptChannel::Aggregates.as_str(), "aggregates");
    /// ```
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Trades => "trades",
            Self::Books => "books",
            Self::Candles => "candles",
            Self::Aggregates => "aggregates",
        }
    }
}

/// Parses a channel name, ignoring case. `indices` is not a FutOpt channel.
///
/// ```rust
/// use marketdata_core::models::futopt::FutOptChannel;
///
/// assert_eq!("Books".parse::<FutOptChannel>().unwrap(), FutOptChannel::Books);
/// assert!("indices".parse::<FutOptChannel>().is_err());
/// ```
impl std::str::FromStr for FutOptChannel {
    type Err = crate::MarketDataError;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        // Listed in the order Python's error message uses.
        const ALL: [FutOptChannel; 4] = [
            FutOptChannel::Trades,
            FutOptChannel::Candles,
            FutOptChannel::Books,
            FutOptChannel::Aggregates,
        ];
        ALL.into_iter()
            .find(|channel| channel.as_str().eq_ignore_ascii_case(name))
            .ok_or_else(|| {
                crate::models::subscription::invalid_channel(
                    name,
                    &ALL.map(|channel| channel.as_str()),
                )
            })
    }
}

impl std::fmt::Display for FutOptChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_futopt_channel_as_str() {
        assert_eq!(FutOptChannel::Trades.as_str(), "trades");
        assert_eq!(FutOptChannel::Books.as_str(), "books");
        assert_eq!(FutOptChannel::Candles.as_str(), "candles");
        assert_eq!(FutOptChannel::Aggregates.as_str(), "aggregates");
    }

    #[test]
    fn futopt_channel_parses_names_ignoring_case() {
        assert_eq!(
            "trades".parse::<FutOptChannel>().unwrap(),
            FutOptChannel::Trades
        );
        assert_eq!(
            "BOOKS".parse::<FutOptChannel>().unwrap(),
            FutOptChannel::Books
        );
        assert_eq!(
            "Candles".parse::<FutOptChannel>().unwrap(),
            FutOptChannel::Candles
        );
        assert_eq!(
            "aggregates".parse::<FutOptChannel>().unwrap(),
            FutOptChannel::Aggregates
        );
    }

    #[test]
    fn futopt_channel_rejects_indices_with_valid_list() {
        let info = "indices".parse::<FutOptChannel>().unwrap_err().info();
        assert_eq!(info.code, crate::error_code::INVALID_PARAMETER);
        assert_eq!(
            info.message,
            "Invalid parameter 'channel': unknown channel 'indices'. \
             Valid channels: trades, candles, books, aggregates"
        );
    }

    #[test]
    fn test_futopt_channel_display() {
        assert_eq!(format!("{}", FutOptChannel::Trades), "trades");
        assert_eq!(format!("{}", FutOptChannel::Books), "books");
        assert_eq!(format!("{}", FutOptChannel::Candles), "candles");
        assert_eq!(format!("{}", FutOptChannel::Aggregates), "aggregates");
    }

    #[test]
    fn test_futopt_channel_serialize() {
        assert_eq!(
            serde_json::to_string(&FutOptChannel::Trades).unwrap(),
            r#""trades""#
        );
        assert_eq!(
            serde_json::to_string(&FutOptChannel::Books).unwrap(),
            r#""books""#
        );
        assert_eq!(
            serde_json::to_string(&FutOptChannel::Candles).unwrap(),
            r#""candles""#
        );
        assert_eq!(
            serde_json::to_string(&FutOptChannel::Aggregates).unwrap(),
            r#""aggregates""#
        );
    }

    #[test]
    fn test_futopt_channel_deserialize() {
        assert_eq!(
            serde_json::from_str::<FutOptChannel>(r#""trades""#).unwrap(),
            FutOptChannel::Trades
        );
        assert_eq!(
            serde_json::from_str::<FutOptChannel>(r#""books""#).unwrap(),
            FutOptChannel::Books
        );
        assert_eq!(
            serde_json::from_str::<FutOptChannel>(r#""candles""#).unwrap(),
            FutOptChannel::Candles
        );
        assert_eq!(
            serde_json::from_str::<FutOptChannel>(r#""aggregates""#).unwrap(),
            FutOptChannel::Aggregates
        );
    }

    #[test]
    #[allow(
        clippy::clone_on_copy,
        reason = "the explicit clone IS the assertion — this test exists to \
                  pin that FutOptChannel implements both Clone and Copy, so \
                  taking clippy's advice to drop it would delete the coverage"
    )]
    fn test_futopt_channel_clone_copy() {
        let channel = FutOptChannel::Trades;
        let cloned = channel.clone();
        let copied = channel;
        assert_eq!(channel, cloned);
        assert_eq!(channel, copied);
    }

    #[test]
    fn test_futopt_channel_eq_hash() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(FutOptChannel::Trades);
        set.insert(FutOptChannel::Books);
        set.insert(FutOptChannel::Candles);
        set.insert(FutOptChannel::Aggregates);

        assert_eq!(set.len(), 4);
        assert!(set.contains(&FutOptChannel::Trades));
        assert!(set.contains(&FutOptChannel::Books));
        assert!(set.contains(&FutOptChannel::Candles));
        assert!(set.contains(&FutOptChannel::Aggregates));
    }
}
