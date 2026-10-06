//! Daily historical endpoint - GET /futopt/historical/daily/{product}

use crate::{errors::MarketDataError, rest::client::RestClient};

/// Request builder for FutOpt daily historical endpoint
pub struct FutOptDailyRequestBuilder<'a> {
    client: &'a RestClient,
    product: Option<String>,
    date: Option<String>,
    contract_month: Option<String>,
    after_hours: Option<bool>,
}

impl<'a> FutOptDailyRequestBuilder<'a> {
    /// Create a new daily historical request builder
    pub(crate) fn new(client: &'a RestClient) -> Self {
        Self {
            client,
            product: None,
            date: None,
            contract_month: None,
            after_hours: None,
        }
    }

    /// Set the product code (required): the three-letter `"TXF"` or `"TXO"`.
    ///
    /// This is the **product**, not a contract: a contract code such as
    /// `"MXFJ6"` or `"TXO47700J6"` returns HTTP 400 telling you to pass the
    /// product here and the month to [`contract_month`](Self::contract_month).
    /// Without a contract month the response lists every contract listed that
    /// day (an option's full chain, or every futures month).
    pub fn product(mut self, product: &str) -> Self {
        self.product = Some(product.to_string());
        self
    }

    /// Same as [`product`](Self::product); the path segment was called
    /// `symbol` before the server named it.
    #[deprecated(note = "use `product()`")]
    pub fn symbol(self, symbol: &str) -> Self {
        self.product(symbol)
    }

    /// Set the trading date (`YYYY-MM-DD`). The server defaults to today.
    pub fn date(mut self, date: &str) -> Self {
        self.date = Some(date.to_string());
        self
    }

    /// Only return one contract month: `YYYYMM`, a weekly `YYYYMMWn` /
    /// `YYYYMMFn`, a futures spread `YYYYMM/YYYYMM`, or (futures only) a
    /// continuous contract `"1!"` / `"2!"` / `"3!"`, which the server resolves
    /// to the actual month on `date` and echoes as the response's
    /// `contractMonth`. Unset returns every contract month.
    pub fn contract_month(mut self, contract_month: &str) -> Self {
        self.contract_month = Some(contract_month.to_string());
        self
    }

    /// Query the after-hours session (`session=afterhours`) instead of the
    /// regular session.
    pub fn after_hours(mut self, after_hours: bool) -> Self {
        self.after_hours = Some(after_hours);
        self
    }

    /// Execute the request and return the daily historical response
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
        let product = self.product.as_deref().ok_or_else(|| MarketDataError::InvalidSymbol {
            symbol: "(not provided)".to_string(),
        })?;

        let mut url = format!(
            "{}/futopt/historical/daily/{}",
            self.client.get_base_url(),
            crate::rest::encode_symbol(product)
        );

        let mut query_params = Vec::new();
        if let Some(date) = &self.date {
            query_params.push(crate::rest::query_pair("date", date));
        }
        if let Some(contract_month) = &self.contract_month {
            query_params.push(crate::rest::query_pair("contractMonth", contract_month));
        }
        if self.after_hours == Some(true) {
            query_params.push("session=afterhours".to_string());
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
    fn test_daily_builder_requires_symbol() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = FutOptDailyRequestBuilder::new(&client);

        let result = builder.send();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MarketDataError::InvalidSymbol { .. }
        ));
    }

    #[test]
    fn test_daily_url_without_query() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let url = FutOptDailyRequestBuilder::new(&client).product("TXF").url().unwrap();
        assert_eq!(url, format!("{}/futopt/historical/daily/TXF", client.get_base_url()));
    }

    #[test]
    fn test_daily_url_uses_date_and_session() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let url = FutOptDailyRequestBuilder::new(&client)
            .product("TXF")
            .date("2026-09-15")
            .after_hours(true)
            .url()
            .unwrap();
        assert_eq!(
            url,
            format!(
                "{}/futopt/historical/daily/TXF?date=2026-09-15&session=afterhours",
                client.get_base_url()
            )
        );
    }

    #[test]
    fn test_daily_url_regular_session_sends_no_session_param() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let url = FutOptDailyRequestBuilder::new(&client)
            .product("TXF")
            .after_hours(false)
            .url()
            .unwrap();
        assert_eq!(url, format!("{}/futopt/historical/daily/TXF", client.get_base_url()));
    }

    #[test]
    fn test_daily_url_encodes_date_value() {
        // A `#` would otherwise start a fragment and drop `session`.
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let url = FutOptDailyRequestBuilder::new(&client)
            .product("TXF")
            .date("2026-09-15#x")
            .after_hours(true)
            .url()
            .unwrap();
        assert_eq!(
            url,
            format!(
                "{}/futopt/historical/daily/TXF?date=2026-09-15%23x&session=afterhours",
                client.get_base_url()
            )
        );
    }

    #[test]
    fn test_daily_url_sends_contract_month() {
        // A spread month carries `/`, which must stay inside the query value.
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let url = FutOptDailyRequestBuilder::new(&client)
            .product("TXF")
            .date("2026-10-02")
            .contract_month("202610/202611")
            .url()
            .unwrap();
        assert_eq!(
            url,
            format!(
                "{}/futopt/historical/daily/TXF?date=2026-10-02&contractMonth=202610%2F202611",
                client.get_base_url()
            )
        );
    }

    #[test]
    #[allow(deprecated)]
    fn test_daily_symbol_still_sets_the_product() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let url = FutOptDailyRequestBuilder::new(&client).symbol("TXO").url().unwrap();
        assert_eq!(url, format!("{}/futopt/historical/daily/TXO", client.get_base_url()));
    }
}
