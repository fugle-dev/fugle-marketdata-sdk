//! Ticker endpoint - GET /stock/intraday/ticker/{symbol}

use crate::{
    errors::MarketDataError,
    models::Ticker,
    rest::client::RestClient,
};

/// Request builder for intraday ticker endpoint
pub struct TickerRequestBuilder<'a> {
    client: &'a RestClient,
    symbol: Option<String>,
    odd_lot: Option<bool>,
}

impl<'a> TickerRequestBuilder<'a> {
    /// Create a new ticker request builder
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

    /// Execute the request and return the ticker info
    pub fn send(self) -> Result<Ticker, MarketDataError> {
        let symbol = self.symbol.ok_or_else(|| MarketDataError::InvalidSymbol {
            symbol: "(not provided)".to_string(),
        })?;

        // Build URL
        let mut url = format!("{}/stock/intraday/ticker/{}", self.client.get_base_url(), symbol);

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
        let ticker: Ticker = response
            .into_json()
            .map_err(|e| MarketDataError::Other(e.into()))?;

        Ok(ticker)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rest::Auth;

    #[test]
    fn test_ticker_builder_requires_symbol() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = TickerRequestBuilder::new(&client);

        let result = builder.send();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), MarketDataError::InvalidSymbol { .. }));
    }

    #[test]
    fn test_ticker_builder_url_construction() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = TickerRequestBuilder::new(&client).symbol("2330");

        assert_eq!(builder.symbol, Some("2330".to_string()));
    }

    #[test]
    fn test_ticker_builder_with_odd_lot() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = TickerRequestBuilder::new(&client)
            .symbol("2330")
            .odd_lot(true);

        assert_eq!(builder.symbol, Some("2330".to_string()));
        assert_eq!(builder.odd_lot, Some(true));
    }
}
