//! marketdata-js: NAPI-RS bindings for marketdata-core
//!
//! This crate provides Node.js bindings for the marketdata-core library,
//! enabling JavaScript/TypeScript developers to access Fugle market data APIs.
//!
//! # Features
//!
//! - REST client for stock and FutOpt market data
//! - Type-safe error handling with error codes
//! - Automatic TypeScript type definitions
//!
//! # Usage (JavaScript/TypeScript)
//!
//! ```javascript
//! const { RestClient } = require('@fubon/marketdata-js');
//!
//! const client = new RestClient('your-api-key');
//! const quote = await client.stock.intraday.quote('2330');
//! console.log(quote);
//! ```

#![deny(clippy::all)]

mod client;
mod errors;
mod websocket;

// Re-export NAPI-RS types
pub use client::*;
pub use errors::*;
pub use websocket::*;
