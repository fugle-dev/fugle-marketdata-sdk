//! Dividends endpoint - GET /stock/corporate-actions/dividends

use crate::{
    errors::MarketDataError,
    rest::client::RestClient,
};

/// Request builder for dividends endpoint
pub struct DividendsRequestBuilder<'a> {
    client: &'a RestClient,
    date: Option<String>,
    start_date: Option<String>,
    end_date: Option<String>,
}

impl<'a> DividendsRequestBuilder<'a> {
    /// Create a new dividends request builder
    pub(crate) fn new(client: &'a RestClient) -> Self {
        Self {
            client,
            date: None,
            start_date: None,
            end_date: None,
        }
    }

    /// Set a specific date filter (format: YYYY-MM-DD)
    pub fn date(mut self, date: &str) -> Self {
        self.date = Some(date.to_string());
        self
    }

    /// Set the start date for range filter (format: YYYY-MM-DD)
    pub fn start_date(mut self, start_date: &str) -> Self {
        self.start_date = Some(start_date.to_string());
        self
    }

    /// Set the end date for range filter (format: YYYY-MM-DD)
    pub fn end_date(mut self, end_date: &str) -> Self {
        self.end_date = Some(end_date.to_string());
        self
    }

    /// Execute the request and return dividends response
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
        // Build URL
        let mut url = format!(
            "{}/stock/corporate-actions/dividends",
            self.client.get_base_url()
        );

        // Add query parameters
        let mut query_params = Vec::new();
        if let Some(date) = &self.date {
            query_params.push(crate::rest::query_pair("date", date));
        }
        if let Some(start_date) = &self.start_date {
            query_params.push(crate::rest::query_pair("start_date", start_date));
        }
        if let Some(end_date) = &self.end_date {
            query_params.push(crate::rest::query_pair("end_date", end_date));
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
    fn test_dividends_builder_no_params() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = DividendsRequestBuilder::new(&client);

        assert!(builder.date.is_none());
        assert!(builder.start_date.is_none());
        assert!(builder.end_date.is_none());
    }

    #[test]
    fn test_dividends_builder_with_date() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = DividendsRequestBuilder::new(&client)
            .date("2024-01-15");

        assert_eq!(builder.date, Some("2024-01-15".to_string()));
    }

    #[test]
    fn test_dividends_builder_with_date_range() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = DividendsRequestBuilder::new(&client)
            .start_date("2024-01-01")
            .end_date("2024-12-31");

        assert_eq!(builder.start_date, Some("2024-01-01".to_string()));
        assert_eq!(builder.end_date, Some("2024-12-31".to_string()));
    }

    #[test]
    fn test_dividends_url_encodes_date_value() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let url = DividendsRequestBuilder::new(&client)
            .start_date("2026-08-01&end_date=2026-08-02")
            .url()
            .unwrap();
        assert_eq!(
            url,
            format!(
                "{}/stock/corporate-actions/dividends?start_date=2026-08-01%26end_date%3D2026-08-02",
                client.get_base_url()
            )
        );
    }

    #[test]
    fn test_dividends_url_uses_snake_case_date_range() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let url = DividendsRequestBuilder::new(&client)
            .start_date("2026-08-01")
            .end_date("2026-09-30")
            .url()
            .unwrap();
        assert_eq!(
            url,
            format!(
                "{}/stock/corporate-actions/dividends?start_date=2026-08-01&end_date=2026-09-30",
                client.get_base_url()
            )
        );
    }
}
