//! Quote endpoint - GET /futopt/intraday/quote/{symbol}

use crate::{
    errors::MarketDataError,
    models::futopt::FutOptQuote,
    rest::client::RestClient,
};

/// Request builder for FutOpt intraday quote endpoint
pub struct QuoteRequestBuilder<'a> {
    client: &'a RestClient,
    symbol: Option<String>,
    session: Option<String>,
}

impl<'a> QuoteRequestBuilder<'a> {
    /// Create a new quote request builder
    pub(crate) fn new(client: &'a RestClient) -> Self {
        Self {
            client,
            symbol: None,
            session: None,
        }
    }

    /// Set the contract symbol (required)
    pub fn symbol(mut self, symbol: &str) -> Self {
        self.symbol = Some(symbol.to_string());
        self
    }

    /// Query after-hours session data
    ///
    /// Sets `session=afterhours` query parameter
    pub fn after_hours(mut self) -> Self {
        self.session = Some("afterhours".to_string());
        self
    }

    /// Execute the request and return the quote
    pub fn send(self) -> Result<FutOptQuote, MarketDataError> {
        let symbol = self.symbol.ok_or_else(|| MarketDataError::InvalidSymbol {
            symbol: "(not provided)".to_string(),
        })?;

        // Build URL
        let mut url = format!("{}/futopt/intraday/quote/{}", self.client.get_base_url(), symbol);

        // Add query parameters
        let mut query_params = Vec::new();
        if let Some(session) = &self.session {
            query_params.push(format!("session={}", session));
        }

        if !query_params.is_empty() {
            url.push('?');
            url.push_str(&query_params.join("&"));
        }

        // Make request
        let request = self.client.agent().get(&url);
        let request = self.client.auth().apply_to_request(request);

        let response = request.call()?;
        let quote: FutOptQuote = response
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

        let result = builder.send();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), MarketDataError::InvalidSymbol { .. }));
    }

    #[test]
    fn test_quote_builder_symbol() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = QuoteRequestBuilder::new(&client).symbol("TXFC4");

        assert!(builder.symbol.is_some());
        assert_eq!(builder.symbol.unwrap(), "TXFC4");
    }

    #[test]
    fn test_quote_builder_after_hours() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = QuoteRequestBuilder::new(&client)
            .symbol("TXFC4")
            .after_hours();

        assert_eq!(builder.symbol, Some("TXFC4".to_string()));
        assert_eq!(builder.session, Some("afterhours".to_string()));
    }

    #[test]
    fn test_quote_builder_chaining() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = QuoteRequestBuilder::new(&client)
            .symbol("TXFC4")
            .after_hours();

        // Verify both fields are set
        assert!(builder.symbol.is_some());
        assert!(builder.session.is_some());
    }
}
