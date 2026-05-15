//! Movers endpoint - GET /stock/snapshot/movers/{market}
//!
//! Returns top gainers and losers in a market.

use crate::{
    errors::MarketDataError,
    models::MoversResponse,
    rest::client::RestClient,
};

/// Request builder for movers endpoint
pub struct MoversRequestBuilder<'a> {
    client: &'a RestClient,
    market: Option<String>,
    direction: Option<String>,
    change: Option<String>,
}

impl<'a> MoversRequestBuilder<'a> {
    /// Create a new movers request builder
    pub(crate) fn new(client: &'a RestClient) -> Self {
        Self {
            client,
            market: None,
            direction: None,
            change: None,
        }
    }

    /// Set the market (required)
    ///
    /// Valid values: "TSE", "OTC", "ESB", "TIB", "PSB"
    pub fn market(mut self, market: &str) -> Self {
        self.market = Some(market.to_string());
        self
    }

    /// Set the direction filter
    ///
    /// Valid values: "up" (gainers), "down" (losers)
    pub fn direction(mut self, direction: &str) -> Self {
        self.direction = Some(direction.to_string());
        self
    }

    /// Set the change type
    ///
    /// Valid values: "percent", "value"
    pub fn change(mut self, change: &str) -> Self {
        self.change = Some(change.to_string());
        self
    }

    /// Execute the request and return the movers response
    pub fn send(self) -> Result<MoversResponse, MarketDataError> {
        let market = self.market.ok_or_else(|| MarketDataError::InvalidParameter {
            name: "market".to_string(),
            reason: "market is required".to_string(),
        })?;

        // Build URL
        let mut url = format!(
            "{}/stock/snapshot/movers/{}",
            self.client.get_base_url(),
            market
        );

        // Add query parameters
        let mut query_params = Vec::new();
        if let Some(direction) = self.direction {
            query_params.push(format!("direction={}", direction));
        }
        if let Some(change) = self.change {
            query_params.push(format!("change={}", change));
        }

        if !query_params.is_empty() {
            url.push('?');
            url.push_str(&query_params.join("&"));
        }

        // Make request
        let request = self.client.agent().get(&url);
        let request = self.client.auth().apply_to_request(request);

        let response = request.call()?;
        let movers: MoversResponse = response
            .into_json()
            .map_err(|e| MarketDataError::Other(e.into()))?;

        Ok(movers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rest::Auth;

    #[test]
    fn test_movers_builder_requires_market() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = MoversRequestBuilder::new(&client);

        let result = builder.send();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MarketDataError::InvalidParameter { .. }
        ));
    }

    #[test]
    fn test_movers_builder_with_market() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = MoversRequestBuilder::new(&client).market("TSE");

        assert_eq!(builder.market, Some("TSE".to_string()));
    }

    #[test]
    fn test_movers_builder_with_all_params() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = MoversRequestBuilder::new(&client)
            .market("TSE")
            .direction("up")
            .change("percent");

        assert_eq!(builder.market, Some("TSE".to_string()));
        assert_eq!(builder.direction, Some("up".to_string()));
        assert_eq!(builder.change, Some("percent".to_string()));
    }
}
