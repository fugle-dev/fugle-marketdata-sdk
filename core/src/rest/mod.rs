//! REST client module for Fugle marketdata API
//!
//! This module provides:
//! - Authentication mechanisms (API Key, Bearer Token, SDK Token)
//! - HTTP client with connection pooling via ureq
//! - Error conversion from ureq to MarketDataError
//! - Stock endpoints (intraday, historical)
//! - FutOpt endpoints (futures and options)

mod auth;
mod client;
mod error;

// Stock endpoints module
pub mod stock;

// FutOpt (Futures and Options) endpoints module
pub mod futopt;

// Re-export public types
pub use auth::Auth;
pub use client::{IntradayClient, RestClient, StockClient};
pub use futopt::{FutOptClient, FutOptIntradayClient};
pub use stock::snapshot::SnapshotClient;
