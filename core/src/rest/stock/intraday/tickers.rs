//! Tickers endpoint - GET /stock/intraday/tickers

use crate::{
    errors::MarketDataError,
    models::Ticker,
    rest::client::RestClient,
};

/// Request builder for stock intraday tickers (batch) endpoint
pub struct TickersRequestBuilder<'a> {
    client: &'a RestClient,
    typ: Option<String>,
    exchange: Option<String>,
    market: Option<String>,
    industry: Option<String>,
    is_normal: Option<bool>,
}

impl<'a> TickersRequestBuilder<'a> {
    pub(crate) fn new(client: &'a RestClient) -> Self {
        Self {
            client,
            typ: None,
            exchange: None,
            market: None,
            industry: None,
            is_normal: None,
        }
    }

    /// Set the security type filter (e.g., "EQUITY", "INDEX", "ETF") - required
    pub fn typ(mut self, typ: &str) -> Self {
        self.typ = Some(typ.to_string());
        self
    }

    /// Set the exchange filter (e.g., "TWSE", "TPEx")
    pub fn exchange(mut self, exchange: &str) -> Self {
        self.exchange = Some(exchange.to_string());
        self
    }

    /// Set the market filter (e.g., "TSE", "OTC")
    pub fn market(mut self, market: &str) -> Self {
        self.market = Some(market.to_string());
        self
    }

    /// Set the industry filter
    pub fn industry(mut self, industry: &str) -> Self {
        self.industry = Some(industry.to_string());
        self
    }

    /// Filter to normal-status tickers only
    pub fn is_normal(mut self, is_normal: bool) -> Self {
        self.is_normal = Some(is_normal);
        self
    }

    /// Execute the request and return the tickers
    ///
    /// # Errors
    /// Returns [`MarketDataError`] on transport, deserialization, validation,
    /// or non-2xx API failures.
    pub fn send(self) -> Result<Vec<Ticker>, MarketDataError> {
        let typ = self.typ.ok_or_else(|| MarketDataError::ConfigError(
            "type parameter is required for tickers endpoint".to_string(),
        ))?;

        let mut query_params = Vec::new();
        query_params.push(format!("type={}", typ));

        if let Some(exchange) = &self.exchange {
            query_params.push(format!("exchange={}", exchange));
        }
        if let Some(market) = &self.market {
            query_params.push(format!("market={}", market));
        }
        if let Some(industry) = &self.industry {
            query_params.push(format!("industry={}", industry));
        }
        if let Some(is_normal) = self.is_normal {
            query_params.push(format!("isNormal={}", is_normal));
        }

        let url = format!(
            "{}/stock/intraday/tickers?{}",
            self.client.get_base_url(),
            query_params.join("&")
        );

        let request = self.client.agent().get(&url);
        let request = self.client.auth().apply_to_request(request);

        let response = self.client.execute(request)?;
        // Prod wraps the list in an envelope: {date,type,exchange,data:[…]}.
        // Decoding the body straight into Vec<Ticker> fails with
        // "invalid type: map, expected a sequence".
        #[derive(serde::Deserialize)]
        struct Envelope {
            #[serde(default)]
            data: Vec<Ticker>,
        }
        let env: Envelope = response
            .into_json()
            .map_err(|e| MarketDataError::Other(e.into()))?;

        Ok(env.data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rest::Auth;

    #[test]
    fn test_tickers_builder_requires_type() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = TickersRequestBuilder::new(&client);

        let result = builder.send();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), MarketDataError::ConfigError(_)));
    }

    #[test]
    fn test_tickers_builder_with_type() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = TickersRequestBuilder::new(&client).typ("EQUITY");

        assert_eq!(builder.typ, Some("EQUITY".to_string()));
    }

    #[test]
    fn test_tickers_envelope_extraction() {
        // Prod shape: object envelope, not a bare array.
        let body = r#"{"date":"2026-04-16","type":"EQUITY","exchange":"TWSE",
            "data":[{"symbol":"2330","name":"台積電"},{"symbol":"0050"}]}"#;
        #[derive(serde::Deserialize)]
        struct Envelope {
            #[serde(default)]
            data: Vec<Ticker>,
        }
        let env: Envelope = serde_json::from_str(body).unwrap();
        assert_eq!(env.data.len(), 2);
        assert_eq!(env.data[0].symbol, "2330");
    }

    #[test]
    fn test_tickers_builder_full_params() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = TickersRequestBuilder::new(&client)
            .typ("EQUITY")
            .exchange("TWSE")
            .market("TSE")
            .industry("24")
            .is_normal(true);

        assert_eq!(builder.typ, Some("EQUITY".to_string()));
        assert_eq!(builder.exchange, Some("TWSE".to_string()));
        assert_eq!(builder.market, Some("TSE".to_string()));
        assert_eq!(builder.industry, Some("24".to_string()));
        assert_eq!(builder.is_normal, Some(true));
    }
}
