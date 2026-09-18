//! Movers endpoint - GET /stock/snapshot/movers/{market}
//!
//! Returns top gainers and losers in a market.

use crate::{
    errors::MarketDataError,
    rest::client::RestClient,
};

/// Request builder for movers endpoint
pub struct MoversRequestBuilder<'a> {
    client: &'a RestClient,
    market: Option<String>,
    direction: Option<String>,
    change: Option<String>,
    type_filter: Option<String>,
    gt: Option<f64>,
    gte: Option<f64>,
    lt: Option<f64>,
    lte: Option<f64>,
    eq: Option<f64>,
}

impl<'a> MoversRequestBuilder<'a> {
    /// Create a new movers request builder
    pub(crate) fn new(client: &'a RestClient) -> Self {
        Self {
            client,
            market: None,
            direction: None,
            change: None,
            type_filter: None,
            gt: None,
            gte: None,
            lt: None,
            lte: None,
            eq: None,
        }
    }

    /// Set the market (required)
    ///
    /// Valid values: "TSE", "OTC", "ESB", "TIB", "PSB"
    pub fn market(mut self, market: &str) -> Self {
        self.market = Some(market.to_string());
        self
    }

    /// Set the direction filter
    ///
    /// Valid values: "up" (gainers), "down" (losers)
    pub fn direction(mut self, direction: &str) -> Self {
        self.direction = Some(direction.to_string());
        self
    }

    /// Set the change type
    ///
    /// Valid values: "percent", "value"
    pub fn change(mut self, change: &str) -> Self {
        self.change = Some(change.to_string());
        self
    }

    /// Set the type filter for stock filtering (sent as `type`)
    ///
    /// Valid values: "ALLBUT0999", "COMMONSTOCK"
    pub fn type_filter(mut self, type_filter: &str) -> Self {
        self.type_filter = Some(type_filter.to_string());
        self
    }

    /// Keep only movers whose change is greater than `gt`
    pub fn gt(mut self, gt: f64) -> Self {
        self.gt = Some(gt);
        self
    }

    /// Keep only movers whose change is greater than or equal to `gte`
    pub fn gte(mut self, gte: f64) -> Self {
        self.gte = Some(gte);
        self
    }

    /// Keep only movers whose change is less than `lt`
    pub fn lt(mut self, lt: f64) -> Self {
        self.lt = Some(lt);
        self
    }

    /// Keep only movers whose change is less than or equal to `lte`
    pub fn lte(mut self, lte: f64) -> Self {
        self.lte = Some(lte);
        self
    }

    /// Keep only movers whose change equals `eq`
    pub fn eq(mut self, eq: f64) -> Self {
        self.eq = Some(eq);
        self
    }

    /// Execute the request and return the movers response
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
        let market = self.market.as_deref().ok_or_else(|| MarketDataError::InvalidParameter {
            name: "market".to_string(),
            reason: "market is required".to_string(),
        })?;

        // Build URL
        let mut url = format!(
            "{}/stock/snapshot/movers/{}",
            self.client.get_base_url(),
            crate::rest::encode_symbol(market)
        );

        // Add query parameters
        let mut query_params = Vec::new();
        if let Some(direction) = &self.direction {
            query_params.push(crate::rest::query_pair("direction", direction));
        }
        if let Some(change) = &self.change {
            query_params.push(crate::rest::query_pair("change", change));
        }
        if let Some(type_filter) = &self.type_filter {
            query_params.push(crate::rest::query_pair("type", type_filter));
        }
        for (key, value) in [
            ("gt", self.gt),
            ("gte", self.gte),
            ("lt", self.lt),
            ("lte", self.lte),
            ("eq", self.eq),
        ] {
            if let Some(value) = value {
                query_params.push(crate::rest::query_pair(key, value));
            }
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
    fn test_movers_builder_requires_market() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = MoversRequestBuilder::new(&client);

        let result = builder.send();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MarketDataError::InvalidParameter { .. }
        ));
    }

    #[test]
    fn test_movers_builder_with_market() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = MoversRequestBuilder::new(&client).market("TSE");

        assert_eq!(builder.market, Some("TSE".to_string()));
    }

    #[test]
    fn test_movers_builder_with_all_params() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = MoversRequestBuilder::new(&client)
            .market("TSE")
            .direction("up")
            .change("percent");

        assert_eq!(builder.market, Some("TSE".to_string()));
        assert_eq!(builder.direction, Some("up".to_string()));
        assert_eq!(builder.change, Some("percent".to_string()));
    }

    #[test]
    fn test_movers_url_includes_type_and_thresholds() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let url = MoversRequestBuilder::new(&client)
            .market("TSE")
            .direction("up")
            .change("percent")
            .type_filter("COMMONSTOCK")
            .gt(1.5)
            .gte(2.0)
            .lt(9.5)
            .lte(10.0)
            .eq(-3.25)
            .url()
            .unwrap();
        assert_eq!(
            url,
            format!(
                "{}/stock/snapshot/movers/TSE?direction=up&change=percent&type=COMMONSTOCK\
                 &gt=1.5&gte=2&lt=9.5&lte=10&eq=-3.25",
                client.get_base_url()
            )
        );
    }

    #[test]
    fn test_movers_url_omits_unset_params() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let url = MoversRequestBuilder::new(&client).market("TSE").url().unwrap();
        assert_eq!(url, format!("{}/stock/snapshot/movers/TSE", client.get_base_url()));
    }
}
