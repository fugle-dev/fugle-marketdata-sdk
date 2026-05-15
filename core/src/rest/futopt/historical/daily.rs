//! Daily historical endpoint - GET /futopt/historical/daily/{symbol}

use crate::{errors::MarketDataError, models::futopt::FutOptDailyResponse, rest::client::RestClient};

/// Request builder for FutOpt daily historical endpoint
pub struct FutOptDailyRequestBuilder<'a> {
    client: &'a RestClient,
    symbol: Option<String>,
    from: Option<String>,
    to: Option<String>,
    after_hours: Option<bool>,
}

impl<'a> FutOptDailyRequestBuilder<'a> {
    /// Create a new daily historical request builder
    pub(crate) fn new(client: &'a RestClient) -> Self {
        Self {
            client,
            symbol: None,
            from: None,
            to: None,
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

    /// Query after-hours session data
    pub fn after_hours(mut self, after_hours: bool) -> Self {
        self.after_hours = Some(after_hours);
        self
    }

    /// Execute the request and return the daily historical response
    pub fn send(self) -> Result<FutOptDailyResponse, MarketDataError> {
        let symbol = self.symbol.ok_or_else(|| MarketDataError::InvalidSymbol {
            symbol: "(not provided)".to_string(),
        })?;

        // Build URL
        let mut url = format!(
            "{}/futopt/historical/daily/{}",
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
        let daily: FutOptDailyResponse = response
            .into_json()
            .map_err(|e| MarketDataError::Other(e.into()))?;

        Ok(daily)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rest::Auth;

    #[test]
    fn test_daily_builder_requires_symbol() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = FutOptDailyRequestBuilder::new(&client);

        let result = builder.send();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MarketDataError::InvalidSymbol { .. }
        ));
    }

    #[test]
    fn test_daily_builder_with_params() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = FutOptDailyRequestBuilder::new(&client)
            .symbol("TXFC4")
            .from("2024-01-01")
            .to("2024-01-31")
            .after_hours(true);

        assert_eq!(builder.symbol, Some("TXFC4".to_string()));
        assert_eq!(builder.from, Some("2024-01-01".to_string()));
        assert_eq!(builder.to, Some("2024-01-31".to_string()));
        assert_eq!(builder.after_hours, Some(true));
    }

    #[test]
    fn test_daily_builder_chaining() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = FutOptDailyRequestBuilder::new(&client)
            .symbol("TXFC4")
            .from("2024-01-01")
            .to("2024-01-31");

        // Verify all fields are set
        assert!(builder.symbol.is_some());
        assert!(builder.from.is_some());
        assert!(builder.to.is_some());
    }
}
