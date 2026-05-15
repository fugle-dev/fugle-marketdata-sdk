//! Data models for Fugle Market Data API responses
//!
//! These models match the official Fugle marketdata SDK response structures.
//!
//! # REST API Models
//! - [`Quote`] - Real-time stock quote (intraday/quote/{symbol})
//! - [`Ticker`] - Stock ticker info (intraday/ticker/{symbol})
//! - [`Trade`], [`TradesResponse`] - Trade data (intraday/trades/{symbol})
//! - [`IntradayCandle`], [`IntradayCandlesResponse`] - Intraday candles
//! - [`HistoricalCandle`], [`HistoricalCandlesResponse`] - Historical candles
//! - [`VolumeAtPrice`], [`VolumesResponse`] - Volume profile
//! - [`CapitalChange`], [`CapitalChangesResponse`] - Capital structure changes
//! - [`Dividend`], [`DividendsResponse`] - Dividend announcements
//! - [`ListingApplicant`], [`ListingApplicantsResponse`] - IPO listings
//!
//! # WebSocket Models
//! - [`Channel`] - WebSocket channel types
//! - [`SubscribeRequest`], [`UnsubscribeRequest`] - Subscription management
//! - [`WebSocketMessage`], [`WebSocketRequest`] - Message wrappers
//! - [`AuthRequest`] - Authentication

mod candle;
mod common;
mod corporate;
pub mod futopt;
mod historical;
mod quote;
mod snapshot;
pub mod streaming;
mod subscription;
pub mod symbols;
mod technical;
mod ticker;
mod trade;
mod volume;

// Common types
pub use common::{PriceLevel, ResponseMeta, TotalStats, TradeInfo, TradingHalt};

// REST response types
pub use candle::{
    HistoricalCandle, HistoricalCandlesResponse, IntradayCandle, IntradayCandlesResponse,
};
pub use historical::StatsResponse;
pub use quote::Quote;
pub use ticker::Ticker;
pub use technical::{
    BbDataPoint, BbResponse, KdjDataPoint, KdjResponse, MacdDataPoint, MacdResponse, RsiDataPoint,
    RsiResponse, SmaDataPoint, SmaResponse,
};
pub use snapshot::{
    Active, ActivesResponse, Mover, MoversResponse, SnapshotQuote, SnapshotQuotesResponse,
};
pub use corporate::{
    CapitalChange, CapitalChangesResponse, Dividend, DividendsResponse, ListingApplicant,
    ListingApplicantsResponse,
};
pub use trade::{Trade, TradesResponse};
pub use volume::{VolumeAtPrice, VolumesResponse};

// WebSocket types
pub use subscription::{
    AuthRequest, Channel, SubscribeRequest, UnsubscribeRequest, WebSocketMessage, WebSocketRequest,
};
pub use symbols::Symbols;
