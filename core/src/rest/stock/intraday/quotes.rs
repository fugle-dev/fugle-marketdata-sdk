//! Quotes endpoint - GET /stock/intraday/quotes
//!
//! The batch form of `quote/{symbol}`: one request for several symbols,
//! answered with an array of the same quote objects. There is no path
//! parameter; the symbols go in the `symbol` query key, comma-separated
//! (`src/stock/intraday/dto/get-quotes.dto.ts`).

use crate::{
    errors::MarketDataError,
    rest::client::RestClient,
};

/// Request builder for intraday quotes (batch) endpoint
pub struct QuotesRequestBuilder<'a> {
    client: &'a RestClient,
    symbol: Option<String>,
    odd_lot: Option<bool>,
}

impl<'a> QuotesRequestBuilder<'a> {
    /// Create a new quotes request builder
    pub(crate) fn new(client: &'a RestClient) -> Self {
        Self {
            client,
            symbol: None,
            odd_lot: None,
        }
    }

    /// Set the stock symbols to quote, comma-separated (e.g. `"2330,2317"`) - required
    pub fn symbol(mut self, symbol: &str) -> Self {
        self.symbol = Some(symbol.to_string());
        self
    }

    /// Set whether to query odd lot data
    pub fn odd_lot(mut self, odd_lot: bool) -> Self {
        self.odd_lot = Some(odd_lot);
        self
    }

    /// Execute the request and return the quotes, one object per symbol
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

        let mut query_params = vec![crate::rest::query_pair("symbol", symbol)];
        if self.odd_lot == Some(true) {
            query_params.push("type=oddlot".to_string());
        }

        Ok(format!(
            "{}/stock/intraday/quotes?{}",
            self.client.get_base_url(),
            query_params.join("&")
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rest::Auth;

    #[test]
    fn test_quotes_builder_requires_symbol() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = QuotesRequestBuilder::new(&client);

        let result = builder.send();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), MarketDataError::InvalidSymbol { .. }));
    }

    #[test]
    fn test_quotes_url_symbol_is_a_query_key_not_a_path_segment() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let url = QuotesRequestBuilder::new(&client)
            .symbol("2330,2317")
            .url()
            .unwrap();
        assert_eq!(
            url,
            format!("{}/stock/intraday/quotes?symbol=2330%2C2317", client.get_base_url())
        );
    }

    #[test]
    fn test_quotes_url_odd_lot_uses_type_param() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let base = client.get_base_url().to_string();

        let url = QuotesRequestBuilder::new(&client).symbol("2330").odd_lot(true).url().unwrap();
        assert_eq!(url, format!("{}/stock/intraday/quotes?symbol=2330&type=oddlot", base));

        let url = QuotesRequestBuilder::new(&client).symbol("2330").odd_lot(false).url().unwrap();
        assert_eq!(url, format!("{}/stock/intraday/quotes?symbol=2330", base));
    }
}
