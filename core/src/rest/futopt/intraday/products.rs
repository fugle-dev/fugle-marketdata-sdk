//! Products endpoint - GET /futopt/intraday/products

use crate::{
    errors::MarketDataError,
    models::futopt::{ContractType, FutOptType, ProductsResponse},
    rest::client::RestClient,
};

/// Request builder for FutOpt intraday products endpoint
///
/// Note: The `type` parameter is required for this endpoint.
pub struct ProductsRequestBuilder<'a> {
    client: &'a RestClient,
    typ: Option<FutOptType>,
    exchange: Option<String>,
    session: Option<String>,
    contract_type: Option<ContractType>,
}

impl<'a> ProductsRequestBuilder<'a> {
    /// Create a new products request builder
    pub(crate) fn new(client: &'a RestClient) -> Self {
        Self {
            client,
            typ: None,
            exchange: None,
            session: None,
            contract_type: None,
        }
    }

    /// Set the contract type (FUTURE or OPTION) - required
    pub fn typ(mut self, typ: FutOptType) -> Self {
        self.typ = Some(typ);
        self
    }

    /// Set the exchange filter (e.g., "TAIFEX")
    pub fn exchange(mut self, exchange: &str) -> Self {
        self.exchange = Some(exchange.to_string());
        self
    }

    /// Query after-hours session data
    ///
    /// Sets `session=afterhours` query parameter
    pub fn after_hours(mut self) -> Self {
        self.session = Some("afterhours".to_string());
        self
    }

    /// Set the contract type filter (I, R, B, C, S, E)
    pub fn contract_type(mut self, contract_type: ContractType) -> Self {
        self.contract_type = Some(contract_type);
        self
    }

    /// Execute the request and return the products response
    pub fn send(self) -> Result<ProductsResponse, MarketDataError> {
        // type is required for products endpoint
        let typ = self.typ.ok_or_else(|| MarketDataError::ConfigError(
            "type parameter is required for products endpoint".to_string(),
        ))?;

        // Build URL with query parameters
        let mut query_params = Vec::new();
        query_params.push(format!("type={}", typ.as_str()));

        if let Some(exchange) = &self.exchange {
            query_params.push(format!("exchange={}", exchange));
        }
        if let Some(session) = &self.session {
            query_params.push(format!("session={}", session));
        }
        if let Some(contract_type) = &self.contract_type {
            query_params.push(format!("contractType={}", contract_type.as_code()));
        }

        let url = format!(
            "{}/futopt/intraday/products?{}",
            self.client.get_base_url(),
            query_params.join("&")
        );

        // Make request
        let request = self.client.agent().get(&url);
        let request = self.client.auth().apply_to_request(request);

        let response = request.call()?;
        let products: ProductsResponse = response
            .into_json()
            .map_err(|e| MarketDataError::Other(e.into()))?;

        Ok(products)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rest::Auth;

    #[test]
    fn test_products_builder_requires_type() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = ProductsRequestBuilder::new(&client);

        let result = builder.send();
        assert!(result.is_err());
        match result.unwrap_err() {
            MarketDataError::ConfigError(msg) => {
                assert!(msg.contains("type parameter is required"));
            }
            _ => panic!("Expected ConfigError"),
        }
    }

    #[test]
    fn test_products_builder_with_type() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = ProductsRequestBuilder::new(&client).typ(FutOptType::Future);

        assert_eq!(builder.typ, Some(FutOptType::Future));
    }

    #[test]
    fn test_products_builder_with_option_type() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = ProductsRequestBuilder::new(&client).typ(FutOptType::Option);

        assert_eq!(builder.typ, Some(FutOptType::Option));
    }

    #[test]
    fn test_products_builder_full_params() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = ProductsRequestBuilder::new(&client)
            .typ(FutOptType::Future)
            .exchange("TAIFEX")
            .after_hours()
            .contract_type(ContractType::Index);

        assert_eq!(builder.typ, Some(FutOptType::Future));
        assert_eq!(builder.exchange, Some("TAIFEX".to_string()));
        assert_eq!(builder.session, Some("afterhours".to_string()));
        assert_eq!(builder.contract_type, Some(ContractType::Index));
    }

    #[test]
    fn test_products_builder_chaining() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = ProductsRequestBuilder::new(&client)
            .typ(FutOptType::Option)
            .exchange("TAIFEX")
            .contract_type(ContractType::Stock);

        // Verify all fields are set
        assert!(builder.typ.is_some());
        assert!(builder.exchange.is_some());
        assert!(builder.contract_type.is_some());
    }

    #[test]
    fn test_products_builder_contract_types() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));

        // Test all contract types
        for ct in [
            ContractType::Index,
            ContractType::Rate,
            ContractType::Bond,
            ContractType::Currency,
            ContractType::Stock,
            ContractType::Etf,
        ] {
            let builder = ProductsRequestBuilder::new(&client)
                .typ(FutOptType::Future)
                .contract_type(ct);
            assert_eq!(builder.contract_type, Some(ct));
        }
    }
}
