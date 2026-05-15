//! Listing applicants (IPO) endpoint - GET /stock/corporate-actions/listing-applicants

use crate::{
    errors::MarketDataError,
    models::ListingApplicantsResponse,
    rest::client::RestClient,
};

/// Request builder for listing applicants (IPO) endpoint
pub struct ListingApplicantsRequestBuilder<'a> {
    client: &'a RestClient,
    date: Option<String>,
    start_date: Option<String>,
    end_date: Option<String>,
}

impl<'a> ListingApplicantsRequestBuilder<'a> {
    /// Create a new listing applicants request builder
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

    /// Execute the request and return listing applicants response
    pub fn send(self) -> Result<ListingApplicantsResponse, MarketDataError> {
        // Build URL
        let mut url = format!(
            "{}/stock/corporate-actions/listing-applicants",
            self.client.get_base_url()
        );

        // Add query parameters
        let mut query_params = Vec::new();
        if let Some(date) = self.date {
            query_params.push(format!("date={}", date));
        }
        if let Some(start_date) = self.start_date {
            query_params.push(format!("startDate={}", start_date));
        }
        if let Some(end_date) = self.end_date {
            query_params.push(format!("endDate={}", end_date));
        }

        if !query_params.is_empty() {
            url.push('?');
            url.push_str(&query_params.join("&"));
        }

        // Make request
        let request = self.client.agent().get(&url);
        let request = self.client.auth().apply_to_request(request);

        let response = request.call()?;
        let data: ListingApplicantsResponse = response
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
    fn test_listing_applicants_builder_no_params() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = ListingApplicantsRequestBuilder::new(&client);

        assert!(builder.date.is_none());
        assert!(builder.start_date.is_none());
        assert!(builder.end_date.is_none());
    }

    #[test]
    fn test_listing_applicants_builder_with_date() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = ListingApplicantsRequestBuilder::new(&client)
            .date("2024-03-01");

        assert_eq!(builder.date, Some("2024-03-01".to_string()));
    }

    #[test]
    fn test_listing_applicants_builder_with_date_range() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = ListingApplicantsRequestBuilder::new(&client)
            .start_date("2024-01-01")
            .end_date("2024-06-30");

        assert_eq!(builder.start_date, Some("2024-01-01".to_string()));
        assert_eq!(builder.end_date, Some("2024-06-30".to_string()));
    }
}
