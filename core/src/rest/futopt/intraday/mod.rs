//! Intraday (real-time) FutOpt data endpoints
//!
//! Provides request builders for all FutOpt intraday endpoints:
//! - `quote` - Real-time quote with bid/ask
//! - `ticker` - Contract information
//! - `tickers` - Batch ticker query
//! - `candles` - OHLC candlestick data
//! - `trades` - Trade history
//! - `volumes` - Volume breakdown
//! - `products` - Available products list

mod candles;
mod products;
mod quote;
mod ticker;
mod tickers;
mod trades;
mod volumes;

pub use candles::CandlesRequestBuilder;
pub use products::ProductsRequestBuilder;
pub use quote::QuoteRequestBuilder;
pub use ticker::TickerRequestBuilder;
pub use tickers::TickersRequestBuilder;
pub use trades::TradesRequestBuilder;
pub use volumes::VolumesRequestBuilder;

use super::FutOptIntradayClient;

impl<'a> FutOptIntradayClient<'a> {
    /// Get intraday quote for a FutOpt contract
    ///
    /// # Example
    /// ```no_run
    /// use marketdata_core::{RestClient, Auth};
    ///
    /// let client = RestClient::new(Auth::SdkToken("my-token".to_string()));
    /// let quote = client.futopt().intraday().quote().symbol("TXFC4").send()?;
    /// # Ok::<(), marketdata_core::MarketDataError>(())
    /// ```
    pub fn quote(&self) -> QuoteRequestBuilder<'a> {
        QuoteRequestBuilder::new(self.client)
    }

    /// Get intraday ticker info for a FutOpt contract
    ///
    /// # Example
    /// ```no_run
    /// use marketdata_core::{RestClient, Auth};
    ///
    /// let client = RestClient::new(Auth::SdkToken("my-token".to_string()));
    /// let ticker = client.futopt().intraday().ticker().symbol("TXFC4").send()?;
    /// # Ok::<(), marketdata_core::MarketDataError>(())
    /// ```
    pub fn ticker(&self) -> TickerRequestBuilder<'a> {
        TickerRequestBuilder::new(self.client)
    }

    /// Get batch ticker info for multiple FutOpt contracts
    ///
    /// # Example
    /// ```no_run
    /// use marketdata_core::{RestClient, Auth};
    /// use marketdata_core::models::futopt::FutOptType;
    ///
    /// let client = RestClient::new(Auth::SdkToken("my-token".to_string()));
    /// let tickers = client.futopt().intraday().tickers()
    ///     .typ(FutOptType::Future)
    ///     .send()?;
    /// # Ok::<(), marketdata_core::MarketDataError>(())
    /// ```
    pub fn tickers(&self) -> TickersRequestBuilder<'a> {
        TickersRequestBuilder::new(self.client)
    }

    /// Get intraday candles for a FutOpt contract
    ///
    /// # Example
    /// ```no_run
    /// use marketdata_core::{RestClient, Auth};
    ///
    /// let client = RestClient::new(Auth::SdkToken("my-token".to_string()));
    /// let candles = client.futopt().intraday().candles()
    ///     .symbol("TXFC4")
    ///     .timeframe("5")
    ///     .send()?;
    /// # Ok::<(), marketdata_core::MarketDataError>(())
    /// ```
    pub fn candles(&self) -> CandlesRequestBuilder<'a> {
        CandlesRequestBuilder::new(self.client)
    }

    /// Get intraday trades for a FutOpt contract
    ///
    /// # Example
    /// ```no_run
    /// use marketdata_core::{RestClient, Auth};
    ///
    /// let client = RestClient::new(Auth::SdkToken("my-token".to_string()));
    /// let trades = client.futopt().intraday().trades().symbol("TXFC4").send()?;
    /// # Ok::<(), marketdata_core::MarketDataError>(())
    /// ```
    pub fn trades(&self) -> TradesRequestBuilder<'a> {
        TradesRequestBuilder::new(self.client)
    }

    /// Get intraday volumes for a FutOpt contract
    ///
    /// # Example
    /// ```no_run
    /// use marketdata_core::{RestClient, Auth};
    ///
    /// let client = RestClient::new(Auth::SdkToken("my-token".to_string()));
    /// let volumes = client.futopt().intraday().volumes().symbol("TXFC4").send()?;
    /// # Ok::<(), marketdata_core::MarketDataError>(())
    /// ```
    pub fn volumes(&self) -> VolumesRequestBuilder<'a> {
        VolumesRequestBuilder::new(self.client)
    }

    /// Get available products list
    ///
    /// # Example
    /// ```no_run
    /// use marketdata_core::{RestClient, Auth};
    /// use marketdata_core::models::futopt::FutOptType;
    ///
    /// let client = RestClient::new(Auth::SdkToken("my-token".to_string()));
    /// let products = client.futopt().intraday().products()
    ///     .typ(FutOptType::Future)
    ///     .send()?;
    /// # Ok::<(), marketdata_core::MarketDataError>(())
    /// ```
    pub fn products(&self) -> ProductsRequestBuilder<'a> {
        ProductsRequestBuilder::new(self.client)
    }
}
