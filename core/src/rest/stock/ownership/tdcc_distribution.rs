//! TDCC distribution endpoint - GET /stock/ownership/tdcc-distribution/{symbol}
//!
//! Returns the weekly TDCC (Taiwan Depository & Clearing Corporation) shareholder distribution by holding-size bracket.

use super::range::{self, HoldingsSort};
use crate::{errors::MarketDataError, models::TdccDistributionResponse, rest::client::RestClient};

/// Request builder for the TDCC shareholder distribution endpoint
pub struct TdccDistributionRequestBuilder<'a> {
    client: &'a RestClient,
    symbol: Option<String>,
    from: Option<String>,
    to: Option<String>,
    sort: Option<HoldingsSort>,
}

impl<'a> TdccDistributionRequestBuilder<'a> {
    /// Create a new TDCC shareholder distribution request builder
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

    /// Set the sort order of the returned series
    pub fn sort(mut self, sort: HoldingsSort) -> Self {
        self.sort = Some(sort);
        self
    }

    /// Execute the request and return the TDCC shareholder distribution response
    ///
    /// # Errors
    /// Returns [`MarketDataError`] on transport, deserialization, validation,
    /// or non-2xx API failures.
    pub fn send(self) -> Result<TdccDistributionResponse, MarketDataError> {
        range::send(
            self.client,
            "tdcc-distribution",
            self.symbol,
            self.from,
            self.to,
            self.sort,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rest::Auth;

    #[test]
    fn test_tdcc_distribution_builder_requires_symbol() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = TdccDistributionRequestBuilder::new(&client);

        let result = builder.send();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MarketDataError::InvalidSymbol { .. }
        ));
    }

    #[test]
    fn test_tdcc_distribution_builder_symbol() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = TdccDistributionRequestBuilder::new(&client).symbol("2330");

        assert_eq!(builder.symbol, Some("2330".to_string()));
    }

    #[test]
    fn test_tdcc_distribution_builder_full_params() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = TdccDistributionRequestBuilder::new(&client)
            .symbol("2330")
            .from("2026-01-01")
            .to("2026-07-31")
            .sort(HoldingsSort::Desc);

        assert_eq!(builder.symbol, Some("2330".to_string()));
        assert_eq!(builder.from, Some("2026-01-01".to_string()));
        assert_eq!(builder.to, Some("2026-07-31".to_string()));
        assert_eq!(builder.sort, Some(HoldingsSort::Desc));
    }
}
