//! Actives endpoint - GET /stock/snapshot/actives/{market}
//!
//! Returns most active stocks by volume or value in a market.

use crate::{
    errors::MarketDataError,
    models::ActivesResponse,
    rest::client::RestClient,
};

/// Request builder for actives endpoint
pub struct ActivesRequestBuilder<'a> {
    client: &'a RestClient,
    market: Option<String>,
    trade: Option<String>,
}

impl<'a> ActivesRequestBuilder<'a> {
    /// Create a new actives request builder
    pub(crate) fn new(client: &'a RestClient) -> Self {
        Self {
            client,
            market: None,
            trade: None,
        }
    }

    /// Set the market (required)
    ///
    /// Valid values: "TSE", "OTC", "ESB", "TIB", "PSB"
    pub fn market(mut self, market: &str) -> Self {
        self.market = Some(market.to_string());
        self
    }

    /// Set the trade metric
    ///
    /// Valid values: "volume", "value"
    pub fn trade(mut self, trade: &str) -> Self {
        self.trade = Some(trade.to_string());
        self
    }

    /// Execute the request and return the actives response
    pub fn send(self) -> Result<ActivesResponse, MarketDataError> {
        let market = self.market.ok_or_else(|| MarketDataError::InvalidParameter {
            name: "market".to_string(),
            reason: "market is required".to_string(),
        })?;

        // Build URL
        let mut url = format!(
            "{}/stock/snapshot/actives/{}",
            self.client.get_base_url(),
            market
        );

        // Add query parameters
        let mut query_params = Vec::new();
        if let Some(trade) = self.trade {
            query_params.push(format!("trade={}", trade));
        }

        if !query_params.is_empty() {
            url.push('?');
            url.push_str(&query_params.join("&"));
        }

        // Make request
        let request = self.client.agent().get(&url);
        let request = self.client.auth().apply_to_request(request);

        let response = request.call()?;
        let actives: ActivesResponse = response
            .into_json()
            .map_err(|e| MarketDataError::Other(e.into()))?;

        Ok(actives)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rest::Auth;

    #[test]
    fn test_actives_builder_requires_market() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = ActivesRequestBuilder::new(&client);

        let result = builder.send();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MarketDataError::InvalidParameter { .. }
        ));
    }

    #[test]
    fn test_actives_builder_with_market() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = ActivesRequestBuilder::new(&client).market("TSE");

        assert_eq!(builder.market, Some("TSE".to_string()));
    }

    #[test]
    fn test_actives_builder_with_trade() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = ActivesRequestBuilder::new(&client)
            .market("TSE")
            .trade("volume");

        assert_eq!(builder.market, Some("TSE".to_string()));
        assert_eq!(builder.trade, Some("volume".to_string()));
    }
}
