//! Candles endpoint - GET /stock/intraday/candles/{symbol}

use crate::{
    errors::MarketDataError,
    rest::client::RestClient,
};

/// Request builder for intraday candles endpoint
pub struct CandlesRequestBuilder<'a> {
    client: &'a RestClient,
    symbol: Option<String>,
    timeframe: Option<String>,
    odd_lot: Option<bool>,
    sort: Option<String>,
}

impl<'a> CandlesRequestBuilder<'a> {
    /// Create a new candles request builder
    pub(crate) fn new(client: &'a RestClient) -> Self {
        Self {
            client,
            symbol: None,
            timeframe: None,
            odd_lot: None,
            sort: None,
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

    /// Set the sort order: `"asc"` or `"desc"`.
    pub fn sort(mut self, sort: &str) -> Self {
        self.sort = Some(sort.to_string());
        self
    }

    /// Execute the request and return the candles response
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
        let mut url = format!("{}/stock/intraday/candles/{}", self.client.get_base_url(), crate::rest::encode_symbol(symbol));

        // Add query parameters
        let mut query_params = Vec::new();
        if let Some(timeframe) = &self.timeframe {
            query_params.push(crate::rest::query_pair("timeframe", timeframe));
        }
        if self.odd_lot == Some(true) {
            query_params.push("type=oddlot".to_string());
        }
        if let Some(sort) = &self.sort {
            query_params.push(crate::rest::query_pair("sort", sort));
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

    #[test]
    fn test_candles_url_odd_lot_uses_type_param() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let base = client.get_base_url().to_string();

        let url = CandlesRequestBuilder::new(&client).symbol("2330").odd_lot(true).url().unwrap();
        assert_eq!(url, format!("{}/stock/intraday/candles/2330?type=oddlot", base));

        let url = CandlesRequestBuilder::new(&client).symbol("2330").odd_lot(false).url().unwrap();
        assert_eq!(url, format!("{}/stock/intraday/candles/2330", base));
    }

    #[test]
    fn test_candles_url_includes_sort() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let url = CandlesRequestBuilder::new(&client)
            .symbol("2330")
            .timeframe("5")
            .sort("desc")
            .url()
            .unwrap();
        assert_eq!(
            url,
            format!("{}/stock/intraday/candles/2330?timeframe=5&sort=desc", client.get_base_url())
        );
    }
}
