//! Volumes endpoint - GET /stock/intraday/volumes/{symbol}

use crate::{
    errors::MarketDataError,
    rest::client::RestClient,
};

/// Request builder for intraday volumes endpoint
pub struct VolumesRequestBuilder<'a> {
    client: &'a RestClient,
    symbol: Option<String>,
    odd_lot: Option<bool>,
}

impl<'a> VolumesRequestBuilder<'a> {
    /// Create a new volumes request builder
    pub(crate) fn new(client: &'a RestClient) -> Self {
        Self {
            client,
            symbol: None,
            odd_lot: None,
        }
    }

    /// Set the stock symbol (required)
    pub fn symbol(mut self, symbol: &str) -> Self {
        self.symbol = Some(symbol.to_string());
        self
    }

    /// Set whether to query odd lot data
    pub fn odd_lot(mut self, odd_lot: bool) -> Self {
        self.odd_lot = Some(odd_lot);
        self
    }

    /// Execute the request and return the volumes response
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
        let symbol = self.symbol.as_deref().ok_or_else(|| MarketDataError::InvalidSymbol {
            symbol: "(not provided)".to_string(),
        })?;

        // Build URL
        let mut url = format!("{}/stock/intraday/volumes/{}", self.client.get_base_url(), crate::rest::encode_symbol(symbol));

        // Add query parameters
        let mut query_params = Vec::new();
        if self.odd_lot == Some(true) {
            query_params.push("type=oddlot".to_string());
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
    fn test_volumes_builder_requires_symbol() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = VolumesRequestBuilder::new(&client);

        let result = builder.send();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), MarketDataError::InvalidSymbol { .. }));
    }

    #[test]
    fn test_volumes_builder_url_construction() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = VolumesRequestBuilder::new(&client).symbol("2330");

        assert_eq!(builder.symbol, Some("2330".to_string()));
    }

    #[test]
    fn test_volumes_builder_with_odd_lot() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = VolumesRequestBuilder::new(&client)
            .symbol("2330")
            .odd_lot(false);

        assert_eq!(builder.symbol, Some("2330".to_string()));
        assert_eq!(builder.odd_lot, Some(false));
    }

    #[test]
    fn test_volumes_url_odd_lot_uses_type_param() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let base = client.get_base_url().to_string();

        let url = VolumesRequestBuilder::new(&client).symbol("2330").odd_lot(true).url().unwrap();
        assert_eq!(url, format!("{}/stock/intraday/volumes/2330?type=oddlot", base));

        let url = VolumesRequestBuilder::new(&client).symbol("2330").odd_lot(false).url().unwrap();
        assert_eq!(url, format!("{}/stock/intraday/volumes/2330", base));
    }
}
