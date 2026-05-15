//! Trades endpoint - GET /stock/intraday/trades/{symbol}

use crate::{
    errors::MarketDataError,
    models::TradesResponse,
    rest::client::RestClient,
};

/// Request builder for intraday trades endpoint
pub struct TradesRequestBuilder<'a> {
    client: &'a RestClient,
    symbol: Option<String>,
    odd_lot: Option<bool>,
    offset: Option<u32>,
    limit: Option<u32>,
    sort: Option<&'static str>,
    is_trial: Option<bool>,
}

impl<'a> TradesRequestBuilder<'a> {
    /// Create a new trades request builder
    pub(crate) fn new(client: &'a RestClient) -> Self {
        Self {
            client,
            symbol: None,
            odd_lot: None,
            offset: None,
            limit: None,
            sort: None,
            is_trial: None,
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

    /// Pagination start offset (0 = most recent with default sort=desc)
    pub fn offset(mut self, offset: u32) -> Self {
        self.offset = Some(offset);
        self
    }

    /// Max number of trades to return
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Oldest-first ordering
    pub fn sort_asc(mut self) -> Self {
        self.sort = Some("asc");
        self
    }

    /// Newest-first ordering (server default)
    pub fn sort_desc(mut self) -> Self {
        self.sort = Some("desc");
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
        let mut url = format!("{}/stock/intraday/trades/{}", self.client.get_base_url(), symbol);

        // Add query parameters
        let mut query_params = Vec::new();
        if let Some(odd_lot) = self.odd_lot {
            query_params.push(format!("oddLot={}", odd_lot));
        }
        if let Some(offset) = self.offset {
            query_params.push(format!("offset={}", offset));
        }
        if let Some(limit) = self.limit {
            query_params.push(format!("limit={}", limit));
        }
        if let Some(sort) = self.sort {
            query_params.push(format!("sort={}", sort));
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
    fn test_trades_builder_url_construction() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = TradesRequestBuilder::new(&client).symbol("2330").odd_lot(true);

        assert_eq!(builder.symbol, Some("2330".to_string()));
        assert_eq!(builder.odd_lot, Some(true));
    }

    #[test]
    fn test_trades_builder_with_pagination() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = TradesRequestBuilder::new(&client)
            .symbol("2330")
            .offset(50)
            .limit(100)
            .sort_desc()
            .is_trial(true);

        assert_eq!(builder.offset, Some(50));
        assert_eq!(builder.limit, Some(100));
        assert_eq!(builder.sort, Some("desc"));
        assert_eq!(builder.is_trial, Some(true));
    }
}
