//! SMA endpoint - GET /stock/technical/sma/{symbol}

use crate::{errors::MarketDataError, models::SmaResponse, rest::client::RestClient};

/// Request builder for Simple Moving Average (SMA) endpoint
pub struct SmaRequestBuilder<'a> {
    client: &'a RestClient,
    symbol: Option<String>,
    from: Option<String>,
    to: Option<String>,
    timeframe: Option<String>,
    period: Option<u32>,
}

impl<'a> SmaRequestBuilder<'a> {
    /// Create a new SMA request builder
    pub(crate) fn new(client: &'a RestClient) -> Self {
        Self {
            client,
            symbol: None,
            from: None,
            to: None,
            timeframe: None,
            period: None,
        }
    }

    /// Set the stock symbol (required)
    pub fn symbol(mut self, symbol: &str) -> Self {
        self.symbol = Some(symbol.to_string());
        self
    }

    /// Set the start date (YYYY-MM-DD)
    pub fn from(mut self, from: &str) -> Self {
        self.from = Some(from.to_string());
        self
    }

    /// Set the end date (YYYY-MM-DD)
    pub fn to(mut self, to: &str) -> Self {
        self.to = Some(to.to_string());
        self
    }

    /// Set the timeframe ("D", "W", "M", or minute values like "1", "5", "10", "15", "30", "60")
    pub fn timeframe(mut self, timeframe: &str) -> Self {
        self.timeframe = Some(timeframe.to_string());
        self
    }

    /// Set the SMA period (e.g., 5, 10, 20, 60, 120)
    pub fn period(mut self, period: u32) -> Self {
        self.period = Some(period);
        self
    }

    /// Execute the request and return the SMA response
    pub fn send(self) -> Result<SmaResponse, MarketDataError> {
        let symbol = self.symbol.ok_or_else(|| MarketDataError::InvalidSymbol {
            symbol: "(not provided)".to_string(),
        })?;

        // Build URL
        let mut url = format!(
            "{}/stock/technical/sma/{}",
            self.client.get_base_url(),
            symbol
        );

        // Add query parameters
        let mut query_params = Vec::new();
        if let Some(from) = self.from {
            query_params.push(format!("from={}", from));
        }
        if let Some(to) = self.to {
            query_params.push(format!("to={}", to));
        }
        if let Some(timeframe) = self.timeframe {
            query_params.push(format!("timeframe={}", timeframe));
        }
        if let Some(period) = self.period {
            query_params.push(format!("period={}", period));
        }

        if !query_params.is_empty() {
            url.push('?');
            url.push_str(&query_params.join("&"));
        }

        // Make request
        let request = self.client.agent().get(&url);
        let request = self.client.auth().apply_to_request(request);

        let response = request.call()?;
        let sma_response: SmaResponse = response
            .into_json()
            .map_err(|e| MarketDataError::Other(e.into()))?;

        Ok(sma_response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rest::Auth;

    #[test]
    fn test_sma_builder_requires_symbol() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = SmaRequestBuilder::new(&client);

        let result = builder.send();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MarketDataError::InvalidSymbol { .. }
        ));
    }

    #[test]
    fn test_sma_builder_chain() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = SmaRequestBuilder::new(&client)
            .symbol("2330")
            .from("2024-01-01")
            .to("2024-01-31")
            .timeframe("D")
            .period(20);

        assert_eq!(builder.symbol, Some("2330".to_string()));
        assert_eq!(builder.from, Some("2024-01-01".to_string()));
        assert_eq!(builder.to, Some("2024-01-31".to_string()));
        assert_eq!(builder.timeframe, Some("D".to_string()));
        assert_eq!(builder.period, Some(20));
    }
}
