//! Director holdings endpoint - GET /stock/ownership/director-holdings/{symbol}
//!
//! Returns monthly shareholdings and pledges disclosed by a company's directors and supervisors.

use super::range;
use crate::{errors::MarketDataError, rest::client::RestClient};

/// Request builder for the director holdings endpoint
pub struct DirectorHoldingsRequestBuilder<'a> {
    client: &'a RestClient,
    symbol: Option<String>,
    from: Option<String>,
    to: Option<String>,
    sort: Option<String>,
}

impl<'a> DirectorHoldingsRequestBuilder<'a> {
    /// Create a new director holdings request builder
    pub(crate) fn new(client: &'a RestClient) -> Self {
        Self {
            client,
            symbol: None,
            from: None,
            to: None,
            sort: None,
        }
    }

    /// Set the stock symbol (required, e.g. `"2330"`)
    pub fn symbol(mut self, symbol: &str) -> Self {
        self.symbol = Some(symbol.to_string());
        self
    }

    /// Set the start of the date range (format: YYYY-MM-DD)
    pub fn from(mut self, from: &str) -> Self {
        self.from = Some(from.to_string());
        self
    }

    /// Set the end of the date range (format: YYYY-MM-DD)
    pub fn to(mut self, to: &str) -> Self {
        self.to = Some(to.to_string());
        self
    }

    /// Set the sort order of the returned series: `"asc"` (oldest first) or
    /// `"desc"` (newest first). Sent as given; the server rejects anything
    /// else.
    pub fn sort(mut self, sort: &str) -> Self {
        self.sort = Some(sort.to_string());
        self
    }

    /// Execute the request and return the director holdings response
    ///
    /// # Errors
    /// Returns [`MarketDataError`] on transport, deserialization, validation,
    /// or non-2xx API failures.
    pub fn send(self) -> Result<serde_json::Value, MarketDataError> {
        let url = self.url()?;
        range::send(self.client, &url)
    }

    /// Build the request URL, including query parameters.
    fn url(&self) -> Result<String, MarketDataError> {
        range::url(
            self.client.get_base_url(),
            "director-holdings",
            self.symbol.as_deref(),
            self.from.as_deref(),
            self.to.as_deref(),
            self.sort.as_deref(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rest::Auth;

    #[test]
    fn test_director_holdings_builder_requires_symbol() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = DirectorHoldingsRequestBuilder::new(&client);

        let result = builder.send();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MarketDataError::InvalidSymbol { .. }
        ));
    }

    #[test]
    fn test_director_holdings_builder_symbol() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = DirectorHoldingsRequestBuilder::new(&client).symbol("2330");

        assert_eq!(builder.symbol, Some("2330".to_string()));
    }

    #[test]
    fn test_director_holdings_builder_full_params() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = DirectorHoldingsRequestBuilder::new(&client)
            .symbol("2330")
            .from("2026-01-01")
            .to("2026-07-31")
            .sort("desc");

        assert_eq!(builder.symbol, Some("2330".to_string()));
        assert_eq!(builder.from, Some("2026-01-01".to_string()));
        assert_eq!(builder.to, Some("2026-07-31".to_string()));
        assert_eq!(builder.sort, Some("desc".to_string()));
    }

    #[test]
    fn test_director_holdings_url_sends_sort() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let base = client.get_base_url().to_string();
        for sort in ["asc", "desc"] {
            let url = DirectorHoldingsRequestBuilder::new(&client).symbol("2330").sort(sort).url().unwrap();
            assert_eq!(url, format!("{base}/stock/ownership/director-holdings/2330?sort={sort}"));
        }
    }
}
