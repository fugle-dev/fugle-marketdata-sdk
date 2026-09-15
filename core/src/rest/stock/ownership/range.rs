//! Shared request plumbing for the ownership endpoints.
//!
//! Every `stock/ownership/*/{symbol}` endpoint takes the same query contract:
//! a required symbol path segment plus optional `from` / `to` / `sort`. The
//! per-endpoint builders keep their own typed fields and delegate the URL
//! assembly and dispatch here.

use serde::de::DeserializeOwned;

use crate::{errors::MarketDataError, rest::client::RestClient};

/// Sort order for an ownership date series.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoldingsSort {
    /// Oldest disclosure date first
    Asc,
    /// Newest disclosure date first
    Desc,
}

impl HoldingsSort {
    pub(super) fn as_str(&self) -> &'static str {
        match self {
            Self::Asc => "asc",
            Self::Desc => "desc",
        }
    }
}

/// Build `{base}/stock/ownership/{endpoint}/{symbol}?from&to&sort`.
pub(super) fn build_url(
    base_url: &str,
    endpoint: &str,
    symbol: &str,
    from: Option<&str>,
    to: Option<&str>,
    sort: Option<HoldingsSort>,
) -> String {
    let mut url = format!(
        "{}/stock/ownership/{}/{}",
        base_url,
        endpoint,
        crate::rest::encode_symbol(symbol)
    );

    let mut query_params = Vec::new();
    if let Some(from) = from {
        query_params.push(format!("from={}", from));
    }
    if let Some(to) = to {
        query_params.push(format!("to={}", to));
    }
    if let Some(sort) = sort {
        query_params.push(format!("sort={}", sort.as_str()));
    }

    if !query_params.is_empty() {
        url.push('?');
        url.push_str(&query_params.join("&"));
    }
    url
}

/// Validate the symbol, send the request and decode the JSON body.
pub(super) fn send<T: DeserializeOwned>(
    client: &RestClient,
    endpoint: &str,
    symbol: Option<String>,
    from: Option<String>,
    to: Option<String>,
    sort: Option<HoldingsSort>,
) -> Result<T, MarketDataError> {
    let symbol = symbol.ok_or_else(|| MarketDataError::InvalidSymbol {
        symbol: "(not provided)".to_string(),
    })?;

    let url = build_url(
        client.get_base_url(),
        endpoint,
        &symbol,
        from.as_deref(),
        to.as_deref(),
        sort,
    );

    let response = client.get(&url)?;
    crate::rest::read_json(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sort_serializes_to_api_values() {
        assert_eq!(HoldingsSort::Asc.as_str(), "asc");
        assert_eq!(HoldingsSort::Desc.as_str(), "desc");
    }

    #[test]
    fn test_build_url_without_query() {
        assert_eq!(
            build_url(
                "https://h/v1.0",
                "institutional-trades",
                "2330",
                None,
                None,
                None
            ),
            "https://h/v1.0/stock/ownership/institutional-trades/2330"
        );
    }

    #[test]
    fn test_build_url_with_full_query() {
        assert_eq!(
            build_url(
                "https://h/v1.0",
                "tdcc-distribution",
                "2330",
                Some("2026-06-01"),
                Some("2026-07-03"),
                Some(HoldingsSort::Desc),
            ),
            "https://h/v1.0/stock/ownership/tdcc-distribution/2330?from=2026-06-01&to=2026-07-03&sort=desc"
        );
    }
}
