//! Daily historical endpoint - GET /futopt/historical/daily/{product}

use crate::{errors::MarketDataError, rest::client::RestClient};

/// Request builder for FutOpt daily historical endpoint
pub struct FutOptDailyRequestBuilder<'a> {
    client: &'a RestClient,
    symbol: Option<String>,
    date: Option<String>,
    after_hours: Option<bool>,
}

impl<'a> FutOptDailyRequestBuilder<'a> {
    /// Create a new daily historical request builder
    pub(crate) fn new(client: &'a RestClient) -> Self {
        Self {
            client,
            symbol: None,
            date: None,
            after_hours: None,
        }
    }

    /// Set the product code (required, e.g. `"TXF"`).
    ///
    /// This is the **product**, not a contract: a contract code such as
    /// `"TXFC4"` returns HTTP 404. The response lists every contract month of
    /// the product.
    pub fn symbol(mut self, symbol: &str) -> Self {
        self.symbol = Some(symbol.to_string());
        self
    }

    /// Set the trading date (`YYYY-MM-DD`). The server defaults to today.
    pub fn date(mut self, date: &str) -> Self {
        self.date = Some(date.to_string());
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
        let symbol = self.symbol.as_deref().ok_or_else(|| MarketDataError::InvalidSymbol {
            symbol: "(not provided)".to_string(),
        })?;

        let mut url = format!(
            "{}/futopt/historical/daily/{}",
            self.client.get_base_url(),
            crate::rest::encode_symbol(symbol)
        );

        let mut query_params = Vec::new();
        if let Some(date) = &self.date {
            query_params.push(crate::rest::query_pair("date", date));
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
        let url = FutOptDailyRequestBuilder::new(&client).symbol("TXF").url().unwrap();
        assert_eq!(url, format!("{}/futopt/historical/daily/TXF", client.get_base_url()));
    }

    #[test]
    fn test_daily_url_uses_date_and_session() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let url = FutOptDailyRequestBuilder::new(&client)
            .symbol("TXF")
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
            .symbol("TXF")
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
            .symbol("TXF")
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
}
