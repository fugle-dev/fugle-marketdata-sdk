//! Trades endpoint - GET /stock/intraday/trades/{symbol}

use crate::{
    errors::MarketDataError,
    rest::client::RestClient,
};

/// Request builder for intraday trades endpoint
pub struct TradesRequestBuilder<'a> {
    client: &'a RestClient,
    symbol: Option<String>,
    odd_lot: Option<bool>,
    offset: Option<u32>,
    limit: Option<u32>,
    sort: Option<String>,
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

    /// Set the sort order (`"asc"` oldest first, `"desc"` newest first — the
    /// server default). Sent as given; the server rejects anything else.
    pub fn sort(mut self, sort: &str) -> Self {
        self.sort = Some(sort.to_string());
        self
    }

    /// Fetch trial-matching (試撮合) trades only
    pub fn is_trial(mut self, is_trial: bool) -> Self {
        self.is_trial = Some(is_trial);
        self
    }

    /// Execute the request and return the trades response
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
        let mut url = format!("{}/stock/intraday/trades/{}", self.client.get_base_url(), crate::rest::encode_symbol(symbol));

        // Add query parameters
        let mut query_params = Vec::new();
        if self.odd_lot == Some(true) {
            query_params.push("type=oddlot".to_string());
        }
        if let Some(offset) = &self.offset {
            query_params.push(crate::rest::query_pair("offset", offset));
        }
        if let Some(limit) = &self.limit {
            query_params.push(crate::rest::query_pair("limit", limit));
        }
        if let Some(sort) = &self.sort {
            query_params.push(crate::rest::query_pair("sort", sort));
        }
        if let Some(is_trial) = &self.is_trial {
            query_params.push(crate::rest::query_pair("isTrial", is_trial));
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
            .sort("desc")
            .is_trial(true);

        assert_eq!(builder.offset, Some(50));
        assert_eq!(builder.limit, Some(100));
        assert_eq!(builder.sort, Some("desc".to_string()));
        assert_eq!(builder.is_trial, Some(true));
    }

    #[test]
    fn test_trades_url_odd_lot_uses_type_param() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let base = client.get_base_url().to_string();

        let url = TradesRequestBuilder::new(&client).symbol("2330").odd_lot(true).url().unwrap();
        assert_eq!(url, format!("{}/stock/intraday/trades/2330?type=oddlot", base));

        let url = TradesRequestBuilder::new(&client).symbol("2330").odd_lot(false).url().unwrap();
        assert_eq!(url, format!("{}/stock/intraday/trades/2330", base));
    }

    #[test]
    fn test_trades_url_sort_is_sent_as_given() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let base = client.get_base_url().to_string();

        let url = TradesRequestBuilder::new(&client).symbol("2330").sort("asc").url().unwrap();
        assert_eq!(url, format!("{}/stock/intraday/trades/2330?sort=asc", base));

        let url = TradesRequestBuilder::new(&client).symbol("2330").sort("desc").url().unwrap();
        assert_eq!(url, format!("{}/stock/intraday/trades/2330?sort=desc", base));

        // Keys are checked, values are not (#164): the server answers a bad value.
        let url = TradesRequestBuilder::new(&client).symbol("2330").sort("newest").url().unwrap();
        assert_eq!(url, format!("{}/stock/intraday/trades/2330?sort=newest", base));
    }
}
