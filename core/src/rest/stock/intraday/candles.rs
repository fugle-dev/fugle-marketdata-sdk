//! Candles endpoint - GET /stock/intraday/candles/{symbol}

use crate::{
    errors::MarketDataError,
    models::IntradayCandlesResponse,
    rest::client::RestClient,
};

/// Request builder for intraday candles endpoint
pub struct CandlesRequestBuilder<'a> {
    client: &'a RestClient,
    symbol: Option<String>,
    timeframe: Option<String>,
    odd_lot: Option<bool>,
}

impl<'a> CandlesRequestBuilder<'a> {
    /// Create a new candles request builder
    pub(crate) fn new(client: &'a RestClient) -> Self {
        Self {
            client,
            symbol: None,
            timeframe: None,
            odd_lot: None,
        }
    }

    /// Set the stock symbol (required)
    pub fn symbol(mut self, symbol: &str) -> Self {
        self.symbol = Some(symbol.to_string());
        self
    }

    /// Set the timeframe (e.g., "1", "5", "10", "15", "30", "60")
    pub fn timeframe(mut self, timeframe: &str) -> Self {
        self.timeframe = Some(timeframe.to_string());
        self
    }

    /// Set whether to query odd lot data
    pub fn odd_lot(mut self, odd_lot: bool) -> Self {
        self.odd_lot = Some(odd_lot);
        self
    }

    /// Execute the request and return the candles response
    pub fn send(self) -> Result<IntradayCandlesResponse, MarketDataError> {
        let symbol = self.symbol.ok_or_else(|| MarketDataError::InvalidSymbol {
            symbol: "(not provided)".to_string(),
        })?;

        // Build URL
        let mut url = format!("{}/stock/intraday/candles/{}", self.client.get_base_url(), symbol);

        // Add query parameters
        let mut query_params = Vec::new();
        if let Some(timeframe) = self.timeframe {
            query_params.push(format!("timeframe={}", timeframe));
        }
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
        let candles: IntradayCandlesResponse = response
            .into_json()
            .map_err(|e| MarketDataError::Other(e.into()))?;

        Ok(candles)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rest::Auth;

    #[test]
    fn test_candles_builder_requires_symbol() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = CandlesRequestBuilder::new(&client);

        let result = builder.send();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), MarketDataError::InvalidSymbol { .. }));
    }

    #[test]
    fn test_candles_builder_with_params() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = CandlesRequestBuilder::new(&client)
            .symbol("2330")
            .timeframe("5")
            .odd_lot(false);

        assert_eq!(builder.symbol, Some("2330".to_string()));
        assert_eq!(builder.timeframe, Some("5".to_string()));
        assert_eq!(builder.odd_lot, Some(false));
    }
}
