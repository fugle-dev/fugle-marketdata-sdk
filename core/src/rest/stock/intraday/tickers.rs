//! Tickers endpoint - GET /stock/intraday/tickers

use crate::{
    errors::MarketDataError,
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
    is_attention: Option<bool>,
    is_disposition: Option<bool>,
    is_halted: Option<bool>,
    symbol: Option<String>,
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
            is_attention: None,
            is_disposition: None,
            is_halted: None,
            symbol: None,
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

    /// Filter to attention-stock (注意股) tickers
    pub fn is_attention(mut self, is_attention: bool) -> Self {
        self.is_attention = Some(is_attention);
        self
    }

    /// Filter to disposition-stock (處置股) tickers
    pub fn is_disposition(mut self, is_disposition: bool) -> Self {
        self.is_disposition = Some(is_disposition);
        self
    }

    /// Filter to halted (暫停交易) tickers
    pub fn is_halted(mut self, is_halted: bool) -> Self {
        self.is_halted = Some(is_halted);
        self
    }

    /// Restrict to the given symbols (comma-separated, e.g. "2330,2317")
    pub fn symbol(mut self, symbol: &str) -> Self {
        self.symbol = Some(symbol.to_string());
        self
    }

    /// Execute the request and return the tickers
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
        let typ = self.typ.as_deref().ok_or_else(|| MarketDataError::ConfigError(
            "type parameter is required for tickers endpoint".to_string(),
        ))?;

        let mut query_params = Vec::new();
        query_params.push(crate::rest::query_pair("type", typ));

        if let Some(exchange) = &self.exchange {
            query_params.push(crate::rest::query_pair("exchange", exchange));
        }
        if let Some(market) = &self.market {
            query_params.push(crate::rest::query_pair("market", market));
        }
        if let Some(industry) = &self.industry {
            query_params.push(crate::rest::query_pair("industry", industry));
        }
        if let Some(is_normal) = self.is_normal {
            query_params.push(crate::rest::query_pair("isNormal", is_normal));
        }
        if let Some(is_attention) = self.is_attention {
            query_params.push(crate::rest::query_pair("isAttention", is_attention));
        }
        if let Some(is_disposition) = self.is_disposition {
            query_params.push(crate::rest::query_pair("isDisposition", is_disposition));
        }
        if let Some(is_halted) = self.is_halted {
            query_params.push(crate::rest::query_pair("isHalted", is_halted));
        }
        if let Some(symbol) = &self.symbol {
            query_params.push(crate::rest::query_pair("symbol", symbol));
        }

        Ok(format!(
            "{}/stock/intraday/tickers?{}",
            self.client.get_base_url(),
            query_params.join("&")
        ))
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
    fn test_tickers_envelope_is_passed_through() {
        // Prod shape: object envelope, not a bare array. Earlier releases
        // unwrapped this to `Vec<Ticker>`, which dropped the sibling metadata
        // and diverged from the official SDK. The envelope now reaches the
        // caller untouched.
        let body = r#"{"date":"2026-04-16","type":"EQUITY","exchange":"TWSE",
            "data":[{"symbol":"2330","name":"台積電"},{"symbol":"0050"}]}"#;
        let raw: serde_json::Value = serde_json::from_str(body).unwrap();

        assert_eq!(raw["date"], "2026-04-16");
        assert_eq!(raw["exchange"], "TWSE");
        assert_eq!(raw["data"].as_array().unwrap().len(), 2);
        assert_eq!(raw["data"][0]["symbol"], "2330");
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

    #[test]
    fn test_tickers_url_includes_status_filters_and_symbol() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let url = TickersRequestBuilder::new(&client)
            .typ("EQUITY")
            .is_attention(true)
            .is_disposition(false)
            .is_halted(true)
            .symbol("2330,2317")
            .url()
            .unwrap();
        assert_eq!(
            url,
            format!(
                "{}/stock/intraday/tickers?type=EQUITY&isAttention=true&isDisposition=false\
                 &isHalted=true&symbol=2330%2C2317",
                client.get_base_url()
            )
        );
    }
}
