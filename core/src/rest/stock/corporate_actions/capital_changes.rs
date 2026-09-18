//! Capital changes endpoint - GET /stock/corporate-actions/capital-changes

use crate::{
    errors::MarketDataError,
    rest::client::RestClient,
};

/// Request builder for capital changes endpoint
pub struct CapitalChangesRequestBuilder<'a> {
    client: &'a RestClient,
    start_date: Option<String>,
    end_date: Option<String>,
    sort: Option<String>,
}

impl<'a> CapitalChangesRequestBuilder<'a> {
    /// Create a new capital changes request builder
    pub(crate) fn new(client: &'a RestClient) -> Self {
        Self {
            client,
            start_date: None,
            end_date: None,
            sort: None,
        }
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

    /// Set the sort order: `"asc"` or `"desc"`.
    pub fn sort(mut self, sort: &str) -> Self {
        self.sort = Some(sort.to_string());
        self
    }

    /// Execute the request and return capital changes response
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
            "{}/stock/corporate-actions/capital-changes",
            self.client.get_base_url()
        );

        // Add query parameters
        let mut query_params = Vec::new();
        if let Some(start_date) = &self.start_date {
            query_params.push(crate::rest::query_pair("start_date", start_date));
        }
        if let Some(end_date) = &self.end_date {
            query_params.push(crate::rest::query_pair("end_date", end_date));
        }
        if let Some(sort) = &self.sort {
            query_params.push(crate::rest::query_pair("sort", sort));
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
    fn test_capital_changes_builder_no_params() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = CapitalChangesRequestBuilder::new(&client);

        // All params should be None by default
        assert!(builder.start_date.is_none());
        assert!(builder.end_date.is_none());
    }


    #[test]
    fn test_capital_changes_builder_with_date_range() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = CapitalChangesRequestBuilder::new(&client)
            .start_date("2024-01-01")
            .end_date("2024-01-31");

        assert_eq!(builder.start_date, Some("2024-01-01".to_string()));
        assert_eq!(builder.end_date, Some("2024-01-31".to_string()));
    }

    #[test]
    fn test_capital_changes_url_uses_snake_case_date_range() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let url = CapitalChangesRequestBuilder::new(&client)
            .start_date("2026-08-01")
            .end_date("2026-09-30")
            .url()
            .unwrap();
        assert_eq!(
            url,
            format!(
                "{}/stock/corporate-actions/capital-changes?start_date=2026-08-01&end_date=2026-09-30",
                client.get_base_url()
            )
        );
    }

    #[test]
    fn test_capital_changes_url_includes_sort() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let url = CapitalChangesRequestBuilder::new(&client)
            .start_date("2026-01-01")
            .end_date("2026-06-30")
            .sort("asc")
            .url()
            .unwrap();
        assert_eq!(
            url,
            format!(
                "{}/stock/corporate-actions/capital-changes?start_date=2026-01-01&end_date=2026-06-30&sort=asc",
                client.get_base_url()
            )
        );
    }
}
