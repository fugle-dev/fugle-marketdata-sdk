//! Snapshot quotes endpoint - GET /stock/snapshot/quotes/{market}
//!
//! Returns market-wide quote snapshot for all stocks in a market.

use crate::{
    errors::MarketDataError,
    models::SnapshotQuotesResponse,
    rest::client::RestClient,
};

/// Request builder for snapshot quotes endpoint
pub struct SnapshotQuotesRequestBuilder<'a> {
    client: &'a RestClient,
    market: Option<String>,
    type_filter: Option<String>,
}

impl<'a> SnapshotQuotesRequestBuilder<'a> {
    /// Create a new snapshot quotes request builder
    pub(crate) fn new(client: &'a RestClient) -> Self {
        Self {
            client,
            market: None,
            type_filter: None,
        }
    }

    /// Set the market (required)
    ///
    /// Valid values: "TSE", "OTC", "ESB", "TIB", "PSB"
    pub fn market(mut self, market: &str) -> Self {
        self.market = Some(market.to_string());
        self
    }

    /// Set the type filter for stock filtering
    ///
    /// Valid values: "ALL", "ALLBUT0999", "COMMONSTOCK"
    pub fn type_filter(mut self, type_filter: &str) -> Self {
        self.type_filter = Some(type_filter.to_string());
        self
    }

    /// Execute the request and return the snapshot quotes response
    pub fn send(self) -> Result<SnapshotQuotesResponse, MarketDataError> {
        let market = self.market.ok_or_else(|| MarketDataError::InvalidParameter {
            name: "market".to_string(),
            reason: "market is required".to_string(),
        })?;

        // Build URL
        let mut url = format!(
            "{}/stock/snapshot/quotes/{}",
            self.client.get_base_url(),
            market
        );

        // Add query parameters
        let mut query_params = Vec::new();
        if let Some(type_filter) = self.type_filter {
            query_params.push(format!("type={}", type_filter));
        }

        if !query_params.is_empty() {
            url.push('?');
            url.push_str(&query_params.join("&"));
        }

        // Make request
        let request = self.client.agent().get(&url);
        let request = self.client.auth().apply_to_request(request);

        let response = request.call()?;
        let quotes: SnapshotQuotesResponse = response
            .into_json()
            .map_err(|e| MarketDataError::Other(e.into()))?;

        Ok(quotes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rest::Auth;

    #[test]
    fn test_snapshot_quotes_builder_requires_market() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = SnapshotQuotesRequestBuilder::new(&client);

        let result = builder.send();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MarketDataError::InvalidParameter { .. }
        ));
    }

    #[test]
    fn test_snapshot_quotes_builder_with_market() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = SnapshotQuotesRequestBuilder::new(&client).market("TSE");

        assert_eq!(builder.market, Some("TSE".to_string()));
    }

    #[test]
    fn test_snapshot_quotes_builder_with_type_filter() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = SnapshotQuotesRequestBuilder::new(&client)
            .market("TSE")
            .type_filter("COMMONSTOCK");

        assert_eq!(builder.market, Some("TSE".to_string()));
        assert_eq!(builder.type_filter, Some("COMMONSTOCK".to_string()));
    }
}
