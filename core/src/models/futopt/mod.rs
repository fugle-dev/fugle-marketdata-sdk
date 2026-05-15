//! FutOpt (Futures and Options) data models for Fugle Market Data API
//!
//! This module contains response models specific to Taiwan Futures Exchange (TAIFEX)
//! futures and options market data.
//!
//! # Overview
//!
//! FutOpt models are separate from Stock models because they have different fields
//! and semantics:
//! - Contracts have expiration dates (`startDate`, `endDate`, `settlementDate`)
//! - Options have strike prices and rights (call/put)
//! - Different trading sessions (regular vs after-hours)
//!
//! # Types
//!
//! ## Common Types
//! - [`FutOptType`] - Futures vs Options distinction
//! - [`ContractType`] - Index, Stock, ETF, etc.
//! - [`FutOptSession`] - Regular vs after-hours trading
//! - [`OptionRight`] - Call vs Put for options
//!
//! ## REST API Models
//! - [`FutOptQuote`] - Real-time quote data with OHLC, bid/ask, totals
//! - [`FutOptTicker`] - Contract information with expiration dates
//! - [`Product`] - Tradeable product info from products endpoint
//! - [`ProductsResponse`] - Products endpoint response wrapper
//!
//! ## Helper Types
//! - [`FutOptPriceLevel`] - Bid/ask price level
//! - [`FutOptTotalStats`] - Trading statistics
//! - [`FutOptLastTrade`] - Last trade information
//!
//! # Example
//!
//! ```rust
//! use marketdata_core::models::futopt::{
//!     FutOptType, ContractType, FutOptSession, FutOptQuote, FutOptTicker
//! };
//!
//! // Type-safe enum usage
//! let typ = FutOptType::Future;
//! let contract = ContractType::Index;
//! let session = FutOptSession::AfterHours;
//!
//! // Serialization for API requests
//! assert_eq!(typ.as_str(), "FUTURE");
//! assert_eq!(contract.as_code(), "I");
//! assert_eq!(session.as_str(), "afterhours");
//!
//! // Deserialize quote from JSON
//! let quote_json = r#"{"date": "2024-01-15", "symbol": "TXFC4", "lastPrice": 17550.0}"#;
//! let quote: FutOptQuote = serde_json::from_str(quote_json).unwrap();
//! assert_eq!(quote.last_price, Some(17550.0));
//!
//! // Deserialize ticker with contract dates
//! let ticker_json = r#"{
//!     "date": "2024-01-15",
//!     "symbol": "TXFC4",
//!     "type": "FUTURE",
//!     "startDate": "2023-12-20",
//!     "endDate": "2024-03-20"
//! }"#;
//! let ticker: FutOptTicker = serde_json::from_str(ticker_json).unwrap();
//! assert!(ticker.is_future());
//! assert!(ticker.has_contract_dates());
//! ```

mod channel;
mod common;
mod historical;
mod product;
mod quote;
mod ticker;

// Re-export all types
pub use channel::FutOptChannel;
pub use common::{ContractType, FutOptSession, FutOptType, OptionRight};
pub use historical::{
    FutOptDailyData, FutOptDailyResponse, FutOptHistoricalCandle, FutOptHistoricalCandlesResponse,
};
pub use product::{Product, ProductsResponse};
pub use quote::{FutOptLastTrade, FutOptPriceLevel, FutOptQuote, FutOptTotalStats};
pub use ticker::FutOptTicker;
