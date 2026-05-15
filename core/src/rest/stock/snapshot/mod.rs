//! Stock snapshot endpoints
//!
//! Provides access to market-wide snapshot data:
//! - quotes: Market-wide quotes snapshot
//! - movers: Top gainers and losers
//! - actives: Most active stocks by volume/value

mod actives;
mod movers;
mod quotes;

pub use actives::ActivesRequestBuilder;
pub use movers::MoversRequestBuilder;
pub use quotes::SnapshotQuotesRequestBuilder;

use crate::rest::client::RestClient;

/// Snapshot endpoints client for market-wide data
pub struct SnapshotClient<'a> {
    pub(crate) client: &'a RestClient,
}

impl<'a> SnapshotClient<'a> {
    /// Create a new snapshot client
    pub(crate) fn new(client: &'a RestClient) -> Self {
        Self { client }
    }

    /// Get market-wide quotes snapshot
    ///
    /// # Example
    /// ```no_run
    /// use marketdata_core::{RestClient, Auth};
    ///
    /// let client = RestClient::new(Auth::SdkToken("my-token".to_string()));
    /// let quotes = client.stock().snapshot().quotes().market("TSE").send()?;
    /// # Ok::<(), marketdata_core::MarketDataError>(())
    /// ```
    pub fn quotes(&self) -> SnapshotQuotesRequestBuilder {
        SnapshotQuotesRequestBuilder::new(self.client)
    }

    /// Get top movers (gainers/losers) in a market
    ///
    /// # Example
    /// ```no_run
    /// use marketdata_core::{RestClient, Auth};
    ///
    /// let client = RestClient::new(Auth::SdkToken("my-token".to_string()));
    /// let movers = client.stock().snapshot().movers().market("TSE").direction("up").send()?;
    /// # Ok::<(), marketdata_core::MarketDataError>(())
    /// ```
    pub fn movers(&self) -> MoversRequestBuilder {
        MoversRequestBuilder::new(self.client)
    }

    /// Get most active stocks by volume or value
    ///
    /// # Example
    /// ```no_run
    /// use marketdata_core::{RestClient, Auth};
    ///
    /// let client = RestClient::new(Auth::SdkToken("my-token".to_string()));
    /// let actives = client.stock().snapshot().actives().market("TSE").trade("volume").send()?;
    /// # Ok::<(), marketdata_core::MarketDataError>(())
    /// ```
    pub fn actives(&self) -> ActivesRequestBuilder {
        ActivesRequestBuilder::new(self.client)
    }
}
