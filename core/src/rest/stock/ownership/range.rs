//! Shared request plumbing for the ownership endpoints.
//!
//! Every `stock/ownership/*/{symbol}` endpoint takes the same query contract:
//! a required symbol path segment plus optional `from` / `to` / `sort`. The
//! per-endpoint builders keep their own typed fields and delegate the URL
//! assembly and dispatch here. `sort` is sent as given: keys are checked,
//! values are not (#164).

use serde::de::DeserializeOwned;

use crate::{errors::MarketDataError, rest::client::RestClient};

/// Build `{base}/stock/ownership/{endpoint}/{symbol}?from&to&sort`.
pub(super) fn build_url(
    base_url: &str,
    endpoint: &str,
    symbol: &str,
    from: Option<&str>,
    to: Option<&str>,
    sort: Option<&str>,
) -> String {
    let mut url = format!(
        "{}/stock/ownership/{}/{}",
        base_url,
        endpoint,
        crate::rest::encode_symbol(symbol)
    );

    let mut query_params = Vec::new();
    if let Some(from) = from {
        query_params.push(crate::rest::query_pair("from", from));
    }
    if let Some(to) = to {
        query_params.push(crate::rest::query_pair("to", to));
    }
    if let Some(sort) = sort {
        query_params.push(crate::rest::query_pair("sort", sort));
    }

    if !query_params.is_empty() {
        url.push('?');
        url.push_str(&query_params.join("&"));
    }
    url
}

/// Validate the symbol and build the request URL.
pub(super) fn url(
    base_url: &str,
    endpoint: &str,
    symbol: Option<&str>,
    from: Option<&str>,
    to: Option<&str>,
    sort: Option<&str>,
) -> Result<String, MarketDataError> {
    let symbol = symbol.ok_or_else(|| MarketDataError::InvalidSymbol {
        symbol: "(not provided)".to_string(),
    })?;
    Ok(build_url(base_url, endpoint, symbol, from, to, sort))
}

/// Send the request and decode the JSON body.
pub(super) fn send<T: DeserializeOwned>(
    client: &RestClient,
    url: &str,
) -> Result<T, MarketDataError> {
    let response = client.get(url)?;
    crate::rest::read_json(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_url_sort_is_sent_as_given() {
        for sort in ["asc", "desc", "newest"] {
            assert_eq!(
                build_url("https://h/v1.0", "etf-holdings", "0050", None, None, Some(sort)),
                format!("https://h/v1.0/stock/ownership/etf-holdings/0050?sort={sort}"),
                "keys are checked, values are not (#164)"
            );
        }
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
                Some("desc"),
            ),
            "https://h/v1.0/stock/ownership/tdcc-distribution/2330?from=2026-06-01&to=2026-07-03&sort=desc"
        );
    }

    #[test]
    fn test_build_url_encodes_date_values() {
        assert_eq!(
            build_url(
                "https://h/v1.0",
                "etf-holdings",
                "0050",
                Some("2026-06-01 00:00"),
                Some("2026-07-03+08"),
                None,
            ),
            "https://h/v1.0/stock/ownership/etf-holdings/0050?from=2026-06-01+00%3A00&to=2026-07-03%2B08"
        );
    }
}
