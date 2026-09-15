//! Institutional trades endpoint - GET /stock/ownership/institutional-trades/{symbol}
//!
//! Returns the three major institutional investors' (foreign, investment trust, dealer) daily buy/sell/net trading in a stock.

use super::range::{self, HoldingsSort};
use crate::{
    errors::MarketDataError, models::InstitutionalTradesResponse, rest::client::RestClient,
};

/// Request builder for the institutional trades endpoint
pub struct InstitutionalTradesRequestBuilder<'a> {
    client: &'a RestClient,
    symbol: Option<String>,
    from: Option<String>,
    to: Option<String>,
    sort: Option<HoldingsSort>,
}

impl<'a> InstitutionalTradesRequestBuilder<'a> {
    /// Create a new institutional trades request builder
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

    /// Execute the request and return the institutional trades response
    ///
    /// # Errors
    /// Returns [`MarketDataError`] on transport, deserialization, validation,
    /// or non-2xx API failures.
    pub fn send(self) -> Result<InstitutionalTradesResponse, MarketDataError> {
        range::send(
            self.client,
            "institutional-trades",
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
    fn test_institutional_trades_builder_requires_symbol() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = InstitutionalTradesRequestBuilder::new(&client);

        let result = builder.send();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MarketDataError::InvalidSymbol { .. }
        ));
    }

    #[test]
    fn test_institutional_trades_builder_symbol() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = InstitutionalTradesRequestBuilder::new(&client).symbol("2330");

        assert_eq!(builder.symbol, Some("2330".to_string()));
    }

    #[test]
    fn test_institutional_trades_builder_full_params() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = InstitutionalTradesRequestBuilder::new(&client)
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
