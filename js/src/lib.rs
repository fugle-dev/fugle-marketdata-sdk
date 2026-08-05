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
// The napi surface mirrors the JavaScript call signature one-to-one: each
// optional JS argument is a separate Rust parameter, because napi-rs has no
// way to express "an options object" other than as positional parameters.
// Collapsing them into a struct to satisfy the lint would change the public
// JavaScript API, which is the one thing these bindings must not do.
#![allow(clippy::too_many_arguments)]

mod client;
mod errors;
mod websocket;

// Re-export NAPI-RS types
pub use client::*;
pub use errors::*;
pub use websocket::*;
