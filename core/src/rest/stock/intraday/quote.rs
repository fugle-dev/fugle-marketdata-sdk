//! Quote endpoint - GET /stock/intraday/quote/{symbol}

use crate::{
    errors::MarketDataError,
    models::Quote,
    rest::client::RestClient,
};

/// Request builder for intraday quote endpoint
pub struct QuoteRequestBuilder<'a> {
    client: &'a RestClient,
    symbol: Option<String>,
    odd_lot: Option<bool>,
}

impl<'a> QuoteRequestBuilder<'a> {
    /// Create a new quote request builder
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

    /// Execute the request and return the quote
    pub fn send(self) -> Result<Quote, MarketDataError> {
        let symbol = self.symbol.ok_or_else(|| MarketDataError::InvalidSymbol {
            symbol: "(not provided)".to_string(),
        })?;

        // Build URL
        let mut url = format!("{}/stock/intraday/quote/{}", self.client.get_base_url(), symbol);

        // Add query parameters
        let mut query_params = Vec::new();
        if let Some(odd_lot) = self.odd_lot {
            query_params.push(format!("oddLot={}", odd_lot));
        }

        if !query_params.is_empty() {
            url.push('?');
            url.push_str(&query_params.join("&"));
        }

        // Make request
        let request = self.client.agent().get(&url);
        let request = self.client.auth().apply_to_request(request);

        let response = request.call()?;
        let quote: Quote = response
            .into_json()
            .map_err(|e| MarketDataError::Other(e.into()))?;

        Ok(quote)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rest::Auth;

    #[test]
    fn test_quote_builder_requires_symbol() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = QuoteRequestBuilder::new(&client);

        // send() should fail without symbol
        let result = builder.send();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), MarketDataError::InvalidSymbol { .. }));
    }

    #[test]
    fn test_quote_builder_url_construction() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = QuoteRequestBuilder::new(&client).symbol("2330");

        // We can't easily test the actual HTTP call without mocking,
        // but we can verify the builder chain compiles
        assert!(builder.symbol.is_some());
        assert_eq!(builder.symbol.unwrap(), "2330");
    }

    #[test]
    fn test_quote_builder_with_odd_lot() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = QuoteRequestBuilder::new(&client)
            .symbol("2330")
            .odd_lot(true);

        assert_eq!(builder.symbol, Some("2330".to_string()));
        assert_eq!(builder.odd_lot, Some(true));
    }
}
