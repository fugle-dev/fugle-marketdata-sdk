//! Historical candles endpoint - GET /stock/historical/candles/{symbol}

use crate::{errors::MarketDataError, models::HistoricalCandlesResponse, rest::client::RestClient};

/// Request builder for historical candles endpoint
pub struct HistoricalCandlesRequestBuilder<'a> {
    client: &'a RestClient,
    symbol: Option<String>,
    from: Option<String>,
    to: Option<String>,
    timeframe: Option<String>,
    fields: Option<String>,
    sort: Option<String>,
    adjusted: Option<bool>,
}

impl<'a> HistoricalCandlesRequestBuilder<'a> {
    /// Create a new historical candles request builder
    pub(crate) fn new(client: &'a RestClient) -> Self {
        Self {
            client,
            symbol: None,
            from: None,
            to: None,
            timeframe: None,
            fields: None,
            sort: None,
            adjusted: None,
        }
    }

    /// Set the stock symbol (required)
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

    /// Set the fields to return
    pub fn fields(mut self, fields: &str) -> Self {
        self.fields = Some(fields.to_string());
        self
    }

    /// Set the sort order ("asc" or "desc")
    pub fn sort(mut self, sort: &str) -> Self {
        self.sort = Some(sort.to_string());
        self
    }

    /// Set whether to return adjusted prices
    pub fn adjusted(mut self, adjusted: bool) -> Self {
        self.adjusted = Some(adjusted);
        self
    }

    /// Execute the request and return the historical candles response
    pub fn send(self) -> Result<HistoricalCandlesResponse, MarketDataError> {
        let symbol = self.symbol.ok_or_else(|| MarketDataError::InvalidSymbol {
            symbol: "(not provided)".to_string(),
        })?;

        // Build URL
        let mut url = format!(
            "{}/stock/historical/candles/{}",
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
        if let Some(fields) = self.fields {
            query_params.push(format!("fields={}", fields));
        }
        if let Some(sort) = self.sort {
            query_params.push(format!("sort={}", sort));
        }
        if let Some(adjusted) = self.adjusted {
            query_params.push(format!("adjusted={}", adjusted));
        }

        if !query_params.is_empty() {
            url.push('?');
            url.push_str(&query_params.join("&"));
        }

        // Make request
        let request = self.client.agent().get(&url);
        let request = self.client.auth().apply_to_request(request);

        let response = request.call()?;
        let candles: HistoricalCandlesResponse = response
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
        let builder = HistoricalCandlesRequestBuilder::new(&client);

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
        let builder = HistoricalCandlesRequestBuilder::new(&client)
            .symbol("2330")
            .from("2024-01-01")
            .to("2024-01-31")
            .timeframe("D")
            .sort("asc")
            .adjusted(true);

        assert_eq!(builder.symbol, Some("2330".to_string()));
        assert_eq!(builder.from, Some("2024-01-01".to_string()));
        assert_eq!(builder.to, Some("2024-01-31".to_string()));
        assert_eq!(builder.timeframe, Some("D".to_string()));
        assert_eq!(builder.sort, Some("asc".to_string()));
        assert_eq!(builder.adjusted, Some(true));
    }

    #[test]
    fn test_historical_candles_builder_fields() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = HistoricalCandlesRequestBuilder::new(&client)
            .symbol("2330")
            .fields("open,close,volume");

        assert_eq!(builder.fields, Some("open,close,volume".to_string()));
    }
}
