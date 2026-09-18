//! Actives endpoint - GET /stock/snapshot/actives/{market}
//!
//! Returns most active stocks by volume or value in a market.

use crate::{
    errors::MarketDataError,
    rest::client::RestClient,
};

/// Request builder for actives endpoint
pub struct ActivesRequestBuilder<'a> {
    client: &'a RestClient,
    market: Option<String>,
    trade: Option<String>,
    type_filter: Option<String>,
}

impl<'a> ActivesRequestBuilder<'a> {
    /// Create a new actives request builder
    pub(crate) fn new(client: &'a RestClient) -> Self {
        Self {
            client,
            market: None,
            trade: None,
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

    /// Set the trade metric
    ///
    /// Valid values: "volume", "value"
    pub fn trade(mut self, trade: &str) -> Self {
        self.trade = Some(trade.to_string());
        self
    }

    /// Set the type filter for stock filtering (sent as `type`)
    ///
    /// Valid values: "ALLBUT0999", "COMMONSTOCK"
    pub fn type_filter(mut self, type_filter: &str) -> Self {
        self.type_filter = Some(type_filter.to_string());
        self
    }

    /// Execute the request and return the actives response
    ///
    /// # Errors
    /// Returns [`MarketDataError`] on transport, deserialization, validation,
    /// or non-2xx API failures.
    pub fn send(self) -> Result<serde_json::Value, MarketDataError> {
        let url = self.url()?;
        let response = self.client.get(&url)?;
        crate::rest::read_json(response)
    }

    /// Build the request URL, including query parameters.
    fn url(&self) -> Result<String, MarketDataError> {
        let market = self.market.as_deref().ok_or_else(|| MarketDataError::InvalidParameter {
            name: "market".to_string(),
            reason: "market is required".to_string(),
        })?;

        // Build URL
        let mut url = format!(
            "{}/stock/snapshot/actives/{}",
            self.client.get_base_url(),
            crate::rest::encode_symbol(market)
        );

        // Add query parameters
        let mut query_params = Vec::new();
        if let Some(trade) = &self.trade {
            query_params.push(crate::rest::query_pair("trade", trade));
        }
        if let Some(type_filter) = &self.type_filter {
            query_params.push(crate::rest::query_pair("type", type_filter));
        }

        if !query_params.is_empty() {
            url.push('?');
            url.push_str(&query_params.join("&"));
        }

        Ok(url)
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

    #[test]
    fn test_actives_url_includes_type() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let url = ActivesRequestBuilder::new(&client)
            .market("OTC")
            .trade("value")
            .type_filter("ALLBUT0999")
            .url()
            .unwrap();
        assert_eq!(
            url,
            format!(
                "{}/stock/snapshot/actives/OTC?trade=value&type=ALLBUT0999",
                client.get_base_url()
            )
        );
    }
}
