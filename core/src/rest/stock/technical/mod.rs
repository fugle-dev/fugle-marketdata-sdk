//! Stock technical indicator endpoints
//!
//! Provides access to technical analysis indicators:
//! - SMA (Simple Moving Average)
//! - RSI (Relative Strength Index)
//! - KDJ (Stochastic Oscillator)
//! - MACD (Moving Average Convergence Divergence)
//! - BB (Bollinger Bands)

mod bb;
mod kdj;
mod macd;
mod rsi;
mod sma;

pub use bb::BbRequestBuilder;
pub use kdj::KdjRequestBuilder;
pub use macd::MacdRequestBuilder;
pub use rsi::RsiRequestBuilder;
pub use sma::SmaRequestBuilder;

use crate::rest::client::RestClient;

/// Client for accessing stock technical indicator endpoints
pub struct TechnicalClient<'a> {
    client: &'a RestClient,
}

impl<'a> TechnicalClient<'a> {
    /// Create a new technical client
    pub(crate) fn new(client: &'a RestClient) -> Self {
        Self { client }
    }

    /// Create a Simple Moving Average (SMA) request builder
    ///
    /// # Example
    /// ```ignore
    /// let response = client.stock().technical().sma()
    ///     .symbol("2330")
    ///     .period(20)
    ///     .send()?;
    /// ```
    pub fn sma(&self) -> SmaRequestBuilder {
        SmaRequestBuilder::new(self.client)
    }

    /// Create a Relative Strength Index (RSI) request builder
    ///
    /// # Example
    /// ```ignore
    /// let response = client.stock().technical().rsi()
    ///     .symbol("2330")
    ///     .period(14)
    ///     .send()?;
    /// ```
    pub fn rsi(&self) -> RsiRequestBuilder {
        RsiRequestBuilder::new(self.client)
    }

    /// Create a KDJ Stochastic Oscillator request builder
    ///
    /// # Example
    /// ```ignore
    /// let response = client.stock().technical().kdj()
    ///     .symbol("2330")
    ///     .period(9)
    ///     .send()?;
    /// ```
    pub fn kdj(&self) -> KdjRequestBuilder {
        KdjRequestBuilder::new(self.client)
    }

    /// Create a MACD (Moving Average Convergence Divergence) request builder
    ///
    /// # Example
    /// ```ignore
    /// let response = client.stock().technical().macd()
    ///     .symbol("2330")
    ///     .fast(12)
    ///     .slow(26)
    ///     .signal(9)
    ///     .send()?;
    /// ```
    pub fn macd(&self) -> MacdRequestBuilder {
        MacdRequestBuilder::new(self.client)
    }

    /// Create a Bollinger Bands (BB) request builder
    ///
    /// # Example
    /// ```ignore
    /// let response = client.stock().technical().bb()
    ///     .symbol("2330")
    ///     .period(20)
    ///     .stddev(2.0)
    ///     .send()?;
    /// ```
    pub fn bb(&self) -> BbRequestBuilder {
        BbRequestBuilder::new(self.client)
    }
}
