//! WebSocket streaming channel support
//!
//! This module provides typed subscription and message parsing for stock and FutOpt channels.
//!
//! # Stock Subscription
//!
//! Use [`StockSubscription`] to create typed stock subscriptions:
//!
//! ```rust
//! use marketdata_core::models::Channel;
//! use marketdata_core::websocket::channels::StockSubscription;
//!
//! // Subscribe to trades
//! let sub = StockSubscription::new(Channel::Trades, "2330");
//!
//! // Subscribe to odd lot trades
//! let odd_lot_sub = StockSubscription::new(Channel::Trades, "2330")
//!     .with_odd_lot(true);
//! ```
//!
//! # FutOpt Subscription
//!
//! Use [`FutOptSubscription`] to create typed futures/options subscriptions:
//!
//! ```rust
//! use marketdata_core::models::futopt::FutOptChannel;
//! use marketdata_core::websocket::channels::FutOptSubscription;
//!
//! // Subscribe to futures trades
//! let sub = FutOptSubscription::new(FutOptChannel::Trades, "TXF202502");
//!
//! // Subscribe to after-hours trading
//! let ah_sub = FutOptSubscription::new(FutOptChannel::Trades, "TXF202502")
//!     .with_after_hours(true);
//! ```
//!
//! # Message Parsing
//!
//! Parse incoming WebSocket messages:
//!
//! ```rust
//! use marketdata_core::websocket::channels::{parse_stream_message, parse_channel_data};
//!
//! // Parse top-level message
//! let json = r#"{"event": "subscribed", "id": "sub-1", "channel": "trades", "symbol": "2330"}"#;
//! let msg = parse_stream_message(json).unwrap();
//!
//! // Parse channel-specific data (from snapshot or data events)
//! use marketdata_core::models::streaming::StreamMessage;
//! if let StreamMessage::Snapshot { channel, payload, .. } = msg {
//!     let data = parse_channel_data(&channel, &payload.data, true);
//! }
//! ```

mod futopt;
mod parser;
mod stock;

pub use futopt::FutOptSubscription;
pub use parser::{parse_channel_data, parse_stream_message, ChannelData};
pub use stock::StockSubscription;
