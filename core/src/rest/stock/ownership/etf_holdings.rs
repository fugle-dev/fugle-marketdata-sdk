//! ETF holdings endpoint - GET /stock/ownership/etf-holdings/{symbol}

use crate::{errors::MarketDataError, models::EtfHoldingsResponse, rest::client::RestClient};

/// Sort order for the holdings series.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoldingsSort {
    /// Oldest disclosure date first
    Asc,
    /// Newest disclosure date first
    Desc,
}

impl HoldingsSort {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Asc => "asc",
            Self::Desc => "desc",
        }
    }
}

/// Request builder for the ETF holdings endpoint
pub struct EtfHoldingsRequestBuilder<'a> {
    client: &'a RestClient,
    symbol: Option<String>,
    from: Option<String>,
    to: Option<String>,
    sort: Option<HoldingsSort>,
}

impl<'a> EtfHoldingsRequestBuilder<'a> {
    /// Create a new ETF holdings request builder
    pub(crate) fn new(client: &'a RestClient) -> Self {
        Self {
            client,
            symbol: None,
            from: None,
            to: None,
            sort: None,
        }
    }

    /// Set the ETF symbol (required, e.g. `"0050"`)
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

    /// Execute the request and return the ETF holdings response
    ///
    /// # Errors
    /// Returns [`MarketDataError`] on transport, deserialization, validation,
    /// or non-2xx API failures.
    pub fn send(self) -> Result<EtfHoldingsResponse, MarketDataError> {
        let symbol = self.symbol.ok_or_else(|| MarketDataError::InvalidSymbol {
            symbol: "(not provided)".to_string(),
        })?;

        // Build URL
        let mut url = format!(
            "{}/stock/ownership/etf-holdings/{}",
            self.client.get_base_url(),
            crate::rest::encode_symbol(&symbol)
        );

        // Add query parameters
        let mut query_params = Vec::new();
        if let Some(from) = self.from {
            query_params.push(format!("from={}", from));
        }
        if let Some(to) = self.to {
            query_params.push(format!("to={}", to));
        }
        if let Some(sort) = self.sort {
            query_params.push(format!("sort={}", sort.as_str()));
        }

        if !query_params.is_empty() {
            url.push('?');
            url.push_str(&query_params.join("&"));
        }

        // Make request
        let request = self.client.agent().get(&url);
        let request = self.client.auth().apply_to_request(request);

        let response = self.client.execute(request)?;
        let data: EtfHoldingsResponse = response
            .into_json()
            .map_err(|e| MarketDataError::Other(e.into()))?;

        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rest::Auth;

    #[test]
    fn test_etf_holdings_builder_requires_symbol() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = EtfHoldingsRequestBuilder::new(&client);

        let result = builder.send();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MarketDataError::InvalidSymbol { .. }
        ));
    }

    #[test]
    fn test_etf_holdings_builder_symbol() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = EtfHoldingsRequestBuilder::new(&client).symbol("0050");

        assert_eq!(builder.symbol, Some("0050".to_string()));
    }

    #[test]
    fn test_etf_holdings_builder_full_params() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = EtfHoldingsRequestBuilder::new(&client)
            .symbol("0050")
            .from("2026-01-01")
            .to("2026-07-31")
            .sort(HoldingsSort::Desc);

        assert_eq!(builder.symbol, Some("0050".to_string()));
        assert_eq!(builder.from, Some("2026-01-01".to_string()));
        assert_eq!(builder.to, Some("2026-07-31".to_string()));
        assert_eq!(builder.sort, Some(HoldingsSort::Desc));
    }

    #[test]
    fn test_sort_serializes_to_api_values() {
        assert_eq!(HoldingsSort::Asc.as_str(), "asc");
        assert_eq!(HoldingsSort::Desc.as_str(), "desc");
    }
}
