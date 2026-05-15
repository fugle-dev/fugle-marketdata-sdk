//! Historical FutOpt data endpoints
//!
//! Provides request builders for all FutOpt historical endpoints:
//! - `candles` - Historical OHLC candlestick data
//! - `daily` - Daily historical data with settlement prices

mod candles;
mod daily;

pub use candles::FutOptHistoricalCandlesRequestBuilder;
pub use daily::FutOptDailyRequestBuilder;

use super::FutOptHistoricalClient;

impl<'a> FutOptHistoricalClient<'a> {
    /// Get historical candles for a FutOpt contract
    ///
    /// # Example
    /// ```no_run
    /// use marketdata_core::{RestClient, Auth};
    ///
    /// let client = RestClient::new(Auth::SdkToken("my-token".to_string()));
    /// let candles = client.futopt().historical().candles()
    ///     .symbol("TXFC4")
    ///     .from("2024-01-01")
    ///     .to("2024-01-31")
    ///     .timeframe("D")
    ///     .send()?;
    /// # Ok::<(), marketdata_core::MarketDataError>(())
    /// ```
    pub fn candles(&self) -> FutOptHistoricalCandlesRequestBuilder<'a> {
        FutOptHistoricalCandlesRequestBuilder::new(self.client)
    }

    /// Get daily historical data for a FutOpt contract
    ///
    /// # Example
    /// ```no_run
    /// use marketdata_core::{RestClient, Auth};
    ///
    /// let client = RestClient::new(Auth::SdkToken("my-token".to_string()));
    /// let daily = client.futopt().historical().daily()
    ///     .symbol("TXFC4")
    ///     .from("2024-01-01")
    ///     .to("2024-01-31")
    ///     .send()?;
    /// # Ok::<(), marketdata_core::MarketDataError>(())
    /// ```
    pub fn daily(&self) -> FutOptDailyRequestBuilder<'a> {
        FutOptDailyRequestBuilder::new(self.client)
    }
}
