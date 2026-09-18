//! Snapshot heatmap endpoint - GET /stock/snapshot/heatmap/{symbol}
//!
//! Returns the constituents of an **index** with their day's change, the
//! data behind a market heatmap. The path parameter is an index code
//! (`IX0001`, `IX0027`); a stock symbol or a market code such as `TSE` is
//! HTTP 404 (`src/stock/snapshot/dto/get-snapshot-heatmap.dto.ts`).

use crate::{
    errors::MarketDataError,
    rest::client::RestClient,
};

/// Request builder for snapshot heatmap endpoint
pub struct HeatmapRequestBuilder<'a> {
    client: &'a RestClient,
    symbol: Option<String>,
    time: Option<String>,
    period: Option<String>,
}

impl<'a> HeatmapRequestBuilder<'a> {
    /// Create a new snapshot heatmap request builder
    pub(crate) fn new(client: &'a RestClient) -> Self {
        Self {
            client,
            symbol: None,
            time: None,
            period: None,
        }
    }

    /// Set the **index code** (required), e.g. `"IX0001"` for the TAIEX or
    /// `"IX0027"` for the TPEx index.
    ///
    /// This is not a stock symbol or a market: `"2330"` and `"TSE"` both
    /// return HTTP 404.
    pub fn symbol(mut self, symbol: &str) -> Self {
        self.symbol = Some(symbol.to_string());
        self
    }

    /// Set the intraday snapshot time (`HHmmss`, e.g. `"100000"`).
    ///
    /// Without `time` or `period` the server returns the latest snapshot.
    pub fn time(mut self, time: &str) -> Self {
        self.time = Some(time.to_string());
        self
    }

    /// Set the change period instead of the day's change.
    ///
    /// Server values: "1w", "1m", "3m", "6m", "1y", "ytd"
    pub fn period(mut self, period: &str) -> Self {
        self.period = Some(period.to_string());
        self
    }

    /// Execute the request and return the heatmap response
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

        let mut url = format!(
            "{}/stock/snapshot/heatmap/{}",
            self.client.get_base_url(),
            crate::rest::encode_symbol(symbol)
        );

        let mut query_params = Vec::new();
        if let Some(time) = &self.time {
            query_params.push(crate::rest::query_pair("time", time));
        }
        if let Some(period) = &self.period {
            query_params.push(crate::rest::query_pair("period", period));
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
    fn test_heatmap_builder_requires_symbol() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = HeatmapRequestBuilder::new(&client);

        let result = builder.send();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), MarketDataError::InvalidSymbol { .. }));
    }

    #[test]
    fn test_heatmap_url_without_query() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let url = HeatmapRequestBuilder::new(&client).symbol("IX0001").url().unwrap();
        assert_eq!(url, format!("{}/stock/snapshot/heatmap/IX0001", client.get_base_url()));
    }

    #[test]
    fn test_heatmap_url_time_and_period() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let url = HeatmapRequestBuilder::new(&client)
            .symbol("IX0001")
            .time("100000")
            .period("1m")
            .url()
            .unwrap();
        assert_eq!(
            url,
            format!("{}/stock/snapshot/heatmap/IX0001?time=100000&period=1m", client.get_base_url())
        );
    }

    #[test]
    fn test_heatmap_url_encodes_symbol_path_segment() {
        // A `/` or `?` in the path param must not change the endpoint or start the query.
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let url = HeatmapRequestBuilder::new(&client)
            .symbol("IX0001/../quotes?period=1m")
            .url()
            .unwrap();
        assert_eq!(
            url,
            format!(
                "{}/stock/snapshot/heatmap/IX0001%2F..%2Fquotes%3Fperiod%3D1m",
                client.get_base_url()
            )
        );
    }
}
