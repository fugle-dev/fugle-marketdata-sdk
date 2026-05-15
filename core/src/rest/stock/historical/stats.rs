//! Historical stats endpoint - GET /stock/historical/stats/{symbol}

use crate::{errors::MarketDataError, models::StatsResponse, rest::client::RestClient};

/// Request builder for historical stats endpoint
pub struct StatsRequestBuilder<'a> {
    client: &'a RestClient,
    symbol: Option<String>,
}

impl<'a> StatsRequestBuilder<'a> {
    /// Create a new stats request builder
    pub(crate) fn new(client: &'a RestClient) -> Self {
        Self {
            client,
            symbol: None,
        }
    }

    /// Set the stock symbol (required)
    pub fn symbol(mut self, symbol: &str) -> Self {
        self.symbol = Some(symbol.to_string());
        self
    }

    /// Execute the request and return the stats response
    pub fn send(self) -> Result<StatsResponse, MarketDataError> {
        let symbol = self.symbol.ok_or_else(|| MarketDataError::InvalidSymbol {
            symbol: "(not provided)".to_string(),
        })?;

        // Build URL
        let url = format!(
            "{}/stock/historical/stats/{}",
            self.client.get_base_url(),
            symbol
        );

        // Make request
        let request = self.client.agent().get(&url);
        let request = self.client.auth().apply_to_request(request);

        let response = request.call()?;
        let stats: StatsResponse = response
            .into_json()
            .map_err(|e| MarketDataError::Other(e.into()))?;

        Ok(stats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rest::Auth;

    #[test]
    fn test_stats_builder_requires_symbol() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = StatsRequestBuilder::new(&client);

        let result = builder.send();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MarketDataError::InvalidSymbol { .. }
        ));
    }

    #[test]
    fn test_stats_builder_with_symbol() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = StatsRequestBuilder::new(&client).symbol("2330");

        assert_eq!(builder.symbol, Some("2330".to_string()));
    }
}
