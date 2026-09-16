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
    /// Get historical candles for a FutOpt product.
    ///
    /// The path takes a **product** code (`TXF`); the contract is chosen with
    /// `contract_month`, which the server defaults to the front month (`1!`).
    ///
    /// # Example
    /// ```no_run
    /// use marketdata_core::{RestClient, Auth};
    ///
    /// let client = RestClient::new(Auth::SdkToken("my-token".to_string()));
    /// let candles = client.futopt().historical().candles()
    ///     .symbol("TXF")
    ///     .contract_month("202609")
    ///     .from("2026-09-01")
    ///     .to("2026-09-15")
    ///     .timeframe("D")
    ///     .send()?;
    /// # Ok::<(), marketdata_core::MarketDataError>(())
    /// ```
    pub fn candles(&self) -> FutOptHistoricalCandlesRequestBuilder<'a> {
        FutOptHistoricalCandlesRequestBuilder::new(self.client)
    }

    /// Get one trading day's daily quotes for every contract month of a
    /// FutOpt product.
    ///
    /// The path takes a **product** code (`TXF`); a contract code such as
    /// `TXFC4` returns HTTP 404.
    ///
    /// # Example
    /// ```no_run
    /// use marketdata_core::{RestClient, Auth};
    ///
    /// let client = RestClient::new(Auth::SdkToken("my-token".to_string()));
    /// let daily = client.futopt().historical().daily()
    ///     .symbol("TXF")
    ///     .date("2026-09-15")
    ///     .after_hours(true)
    ///     .send()?;
    /// # Ok::<(), marketdata_core::MarketDataError>(())
    /// ```
    pub fn daily(&self) -> FutOptDailyRequestBuilder<'a> {
        FutOptDailyRequestBuilder::new(self.client)
    }
}
