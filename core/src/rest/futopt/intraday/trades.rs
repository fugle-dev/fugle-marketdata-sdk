//! Trades endpoint - GET /futopt/intraday/trades/{symbol}

use crate::{
    errors::MarketDataError,
    models::TradesResponse,
    rest::client::RestClient,
};

/// Request builder for FutOpt intraday trades endpoint
pub struct TradesRequestBuilder<'a> {
    client: &'a RestClient,
    symbol: Option<String>,
    offset: Option<i32>,
    limit: Option<i32>,
    session: Option<String>,
    is_trial: Option<bool>,
}

impl<'a> TradesRequestBuilder<'a> {
    /// Create a new trades request builder
    pub(crate) fn new(client: &'a RestClient) -> Self {
        Self {
            client,
            symbol: None,
            offset: None,
            limit: None,
            session: None,
            is_trial: None,
        }
    }

    /// Set the contract symbol (required)
    pub fn symbol(mut self, symbol: &str) -> Self {
        self.symbol = Some(symbol.to_string());
        self
    }

    /// Set the offset for pagination
    pub fn offset(mut self, offset: i32) -> Self {
        self.offset = Some(offset);
        self
    }

    /// Set the limit for number of results
    pub fn limit(mut self, limit: i32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Query after-hours session data
    ///
    /// Sets `session=afterhours` query parameter
    pub fn after_hours(mut self) -> Self {
        self.session = Some("afterhours".to_string());
        self
    }

    /// Fetch trial-matching (試撮合) trades only
    pub fn is_trial(mut self, is_trial: bool) -> Self {
        self.is_trial = Some(is_trial);
        self
    }

    /// Execute the request and return the trades response
    pub fn send(self) -> Result<TradesResponse, MarketDataError> {
        let symbol = self.symbol.ok_or_else(|| MarketDataError::InvalidSymbol {
            symbol: "(not provided)".to_string(),
        })?;

        // Build URL
        let mut url = format!("{}/futopt/intraday/trades/{}", self.client.get_base_url(), symbol);

        // Add query parameters
        let mut query_params = Vec::new();
        if let Some(offset) = self.offset {
            query_params.push(format!("offset={}", offset));
        }
        if let Some(limit) = self.limit {
            query_params.push(format!("limit={}", limit));
        }
        if let Some(session) = &self.session {
            query_params.push(format!("session={}", session));
        }
        if let Some(is_trial) = self.is_trial {
            query_params.push(format!("isTrial={}", is_trial));
        }

        if !query_params.is_empty() {
            url.push('?');
            url.push_str(&query_params.join("&"));
        }

        // Make request
        let request = self.client.agent().get(&url);
        let request = self.client.auth().apply_to_request(request);

        let response = request.call()?;
        let trades: TradesResponse = response
            .into_json()
            .map_err(|e| MarketDataError::Other(e.into()))?;

        Ok(trades)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rest::Auth;

    #[test]
    fn test_trades_builder_requires_symbol() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = TradesRequestBuilder::new(&client);

        let result = builder.send();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), MarketDataError::InvalidSymbol { .. }));
    }

    #[test]
    fn test_trades_builder_with_symbol() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = TradesRequestBuilder::new(&client).symbol("TXFC4");

        assert_eq!(builder.symbol, Some("TXFC4".to_string()));
    }

    #[test]
    fn test_trades_builder_with_pagination() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = TradesRequestBuilder::new(&client)
            .symbol("TXFC4")
            .offset(100)
            .limit(50);

        assert_eq!(builder.symbol, Some("TXFC4".to_string()));
        assert_eq!(builder.offset, Some(100));
        assert_eq!(builder.limit, Some(50));
    }

    #[test]
    fn test_trades_builder_with_after_hours() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = TradesRequestBuilder::new(&client)
            .symbol("TXFC4")
            .after_hours();

        assert_eq!(builder.symbol, Some("TXFC4".to_string()));
        assert_eq!(builder.session, Some("afterhours".to_string()));
    }

    #[test]
    fn test_trades_builder_with_is_trial() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = TradesRequestBuilder::new(&client)
            .symbol("TXFC4")
            .is_trial(true);

        assert_eq!(builder.is_trial, Some(true));
    }

    #[test]
    fn test_trades_builder_trial_with_pagination() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = TradesRequestBuilder::new(&client)
            .symbol("TXFC4")
            .offset(50)
            .limit(100)
            .is_trial(true);

        assert_eq!(builder.offset, Some(50));
        assert_eq!(builder.limit, Some(100));
        assert_eq!(builder.is_trial, Some(true));
    }

    #[test]
    fn test_trades_builder_after_hours_and_trial() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = TradesRequestBuilder::new(&client)
            .symbol("TXFC4")
            .after_hours()
            .is_trial(true);

        assert_eq!(builder.session, Some("afterhours".to_string()));
        assert_eq!(builder.is_trial, Some(true));
    }
}
