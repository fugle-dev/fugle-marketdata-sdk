//! Historical candles endpoint - GET /futopt/historical/candles/{symbol}

use crate::{errors::MarketDataError, models::futopt::FutOptHistoricalCandlesResponse, rest::client::RestClient};

/// Request builder for FutOpt historical candles endpoint
pub struct FutOptHistoricalCandlesRequestBuilder<'a> {
    client: &'a RestClient,
    symbol: Option<String>,
    from: Option<String>,
    to: Option<String>,
    timeframe: Option<String>,
    after_hours: Option<bool>,
}

impl<'a> FutOptHistoricalCandlesRequestBuilder<'a> {
    /// Create a new historical candles request builder
    pub(crate) fn new(client: &'a RestClient) -> Self {
        Self {
            client,
            symbol: None,
            from: None,
            to: None,
            timeframe: None,
            after_hours: None,
        }
    }

    /// Set the contract symbol (required, e.g., "TXFC4")
    pub fn symbol(mut self, symbol: &str) -> Self {
        self.symbol = Some(symbol.to_string());
        self
    }

    /// Set the start date (format: YYYY-MM-DD)
    pub fn from(mut self, from: &str) -> Self {
        self.from = Some(from.to_string());
        self
    }

    /// Set the end date (format: YYYY-MM-DD)
    pub fn to(mut self, to: &str) -> Self {
        self.to = Some(to.to_string());
        self
    }

    /// Set the timeframe (e.g., "D", "W", "M", "1", "5", "10", "15", "30", "60")
    pub fn timeframe(mut self, timeframe: &str) -> Self {
        self.timeframe = Some(timeframe.to_string());
        self
    }

    /// Query after-hours session data
    pub fn after_hours(mut self, after_hours: bool) -> Self {
        self.after_hours = Some(after_hours);
        self
    }

    /// Execute the request and return the historical candles response
    pub fn send(self) -> Result<FutOptHistoricalCandlesResponse, MarketDataError> {
        let symbol = self.symbol.ok_or_else(|| MarketDataError::InvalidSymbol {
            symbol: "(not provided)".to_string(),
        })?;

        // Build URL
        let mut url = format!(
            "{}/futopt/historical/candles/{}",
            self.client.get_base_url(),
            symbol
        );

        // Add query parameters
        let mut query_params = Vec::new();
        if let Some(from) = &self.from {
            query_params.push(format!("from={}", from));
        }
        if let Some(to) = &self.to {
            query_params.push(format!("to={}", to));
        }
        if let Some(timeframe) = &self.timeframe {
            query_params.push(format!("timeframe={}", timeframe));
        }
        if let Some(after_hours) = self.after_hours {
            query_params.push(format!("afterHours={}", after_hours));
        }

        if !query_params.is_empty() {
            url.push('?');
            url.push_str(&query_params.join("&"));
        }

        // Make request
        let request = self.client.agent().get(&url);
        let request = self.client.auth().apply_to_request(request);

        let response = request.call()?;
        let candles: FutOptHistoricalCandlesResponse = response
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
    fn test_historical_candles_builder_requires_symbol() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = FutOptHistoricalCandlesRequestBuilder::new(&client);

        let result = builder.send();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MarketDataError::InvalidSymbol { .. }
        ));
    }

    #[test]
    fn test_historical_candles_builder_with_params() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = FutOptHistoricalCandlesRequestBuilder::new(&client)
            .symbol("TXFC4")
            .from("2024-01-01")
            .to("2024-01-31")
            .timeframe("D")
            .after_hours(true);

        assert_eq!(builder.symbol, Some("TXFC4".to_string()));
        assert_eq!(builder.from, Some("2024-01-01".to_string()));
        assert_eq!(builder.to, Some("2024-01-31".to_string()));
        assert_eq!(builder.timeframe, Some("D".to_string()));
        assert_eq!(builder.after_hours, Some(true));
    }

    #[test]
    fn test_historical_candles_builder_timeframes() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));

        // Test all valid timeframes
        for tf in ["D", "W", "M", "1", "5", "10", "15", "30", "60"] {
            let builder = FutOptHistoricalCandlesRequestBuilder::new(&client)
                .symbol("TXFC4")
                .timeframe(tf);
            assert_eq!(builder.timeframe, Some(tf.to_string()));
        }
    }

    #[test]
    fn test_historical_candles_builder_chaining() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = FutOptHistoricalCandlesRequestBuilder::new(&client)
            .symbol("TXFC4")
            .from("2024-01-01")
            .to("2024-01-31");

        // Verify all fields are set
        assert!(builder.symbol.is_some());
        assert!(builder.from.is_some());
        assert!(builder.to.is_some());
    }
}
