//! FutOpt (Futures and Options) market data endpoints
//!
//! This module provides REST API clients for TAIFEX futures and options market data.
//!
//! # Endpoints
//!
//! ## Intraday (Real-time)
//! - `/futopt/intraday/quote/{symbol}` - Real-time quote data
//! - `/futopt/intraday/ticker/{symbol}` - Contract information
//! - `/futopt/intraday/tickers` - Batch ticker info
//! - `/futopt/intraday/candles/{symbol}` - OHLC candles
//! - `/futopt/intraday/trades/{symbol}` - Trade history
//! - `/futopt/intraday/volumes/{symbol}` - Volume data
//! - `/futopt/intraday/products` - Available products
//!
//! ## Historical
//! - `/futopt/historical/candles/{symbol}` - Historical OHLC candles
//! - `/futopt/historical/daily/{symbol}` - Daily historical data
//!
//! # Example
//!
//! ```no_run
//! use marketdata_core::{RestClient, Auth};
//!
//! let client = RestClient::new(Auth::SdkToken("my-token".to_string()));
//!
//! // Get quote for a futures contract
//! let quote = client.futopt().intraday().quote().symbol("TXFC4").send()?;
//!
//! // Get after-hours quote
//! let ah_quote = client.futopt().intraday().quote()
//!     .symbol("TXFC4")
//!     .after_hours()
//!     .send()?;
//!
//! // Get historical candles
//! let candles = client.futopt().historical().candles()
//!     .symbol("TXFC4")
//!     .from("2024-01-01")
//!     .to("2024-01-31")
//!     .timeframe("D")
//!     .send()?;
//!
//! # Ok::<(), marketdata_core::MarketDataError>(())
//! ```

pub mod historical;
pub mod intraday;

use super::client::RestClient;

/// FutOpt market data client
///
/// Provides access to futures and options market data endpoints.
pub struct FutOptClient<'a> {
    pub(crate) client: &'a RestClient,
}

impl<'a> FutOptClient<'a> {
    /// Access intraday (real-time) endpoints
    ///
    /// # Example
    /// ```
    /// use marketdata_core::{RestClient, Auth};
    ///
    /// let client = RestClient::new(Auth::SdkToken("my-token".to_string()));
    /// let intraday = client.futopt().intraday();
    /// ```
    pub fn intraday(&self) -> FutOptIntradayClient<'a> {
        FutOptIntradayClient {
            client: self.client,
        }
    }

    /// Access historical endpoints
    ///
    /// # Example
    /// ```
    /// use marketdata_core::{RestClient, Auth};
    ///
    /// let client = RestClient::new(Auth::SdkToken("my-token".to_string()));
    /// let historical = client.futopt().historical();
    /// ```
    pub fn historical(&self) -> FutOptHistoricalClient<'a> {
        FutOptHistoricalClient {
            client: self.client,
        }
    }
}

/// FutOpt intraday (real-time) endpoints client
///
/// Provides access to real-time market data for futures and options contracts.
pub struct FutOptIntradayClient<'a> {
    pub(crate) client: &'a RestClient,
}

/// FutOpt historical endpoints client
///
/// Provides access to historical market data for futures and options contracts.
pub struct FutOptHistoricalClient<'a> {
    pub(crate) client: &'a RestClient,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rest::Auth;

    #[test]
    fn test_futopt_client_creation() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let futopt = FutOptClient { client: &client };
        assert_eq!(
            futopt.client.get_base_url(),
            "https://api.fugle.tw/marketdata/v1.0"
        );
    }

    #[test]
    fn test_futopt_intraday_client_creation() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let futopt = FutOptClient { client: &client };
        let intraday = futopt.intraday();
        assert_eq!(
            intraday.client.get_base_url(),
            "https://api.fugle.tw/marketdata/v1.0"
        );
    }

    #[test]
    fn test_futopt_historical_client_creation() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let futopt = FutOptClient { client: &client };
        let historical = futopt.historical();
        assert_eq!(
            historical.client.get_base_url(),
            "https://api.fugle.tw/marketdata/v1.0"
        );
    }

    #[test]
    fn test_futopt_client_chaining() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let _intraday = FutOptClient { client: &client }.intraday();
        let _historical = FutOptClient { client: &client }.historical();
        // Compilation success proves chaining works
    }
}
