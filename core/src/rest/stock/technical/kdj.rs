//! KDJ endpoint - GET /stock/technical/kdj/{symbol}
//! Will be implemented in Task 2

use crate::{errors::MarketDataError, models::KdjResponse, rest::client::RestClient};

/// Request builder for KDJ Stochastic Oscillator endpoint
pub struct KdjRequestBuilder<'a> {
    client: &'a RestClient,
    symbol: Option<String>,
    from: Option<String>,
    to: Option<String>,
    timeframe: Option<String>,
    period: Option<u32>,
}

impl<'a> KdjRequestBuilder<'a> {
    pub(crate) fn new(client: &'a RestClient) -> Self {
        Self {
            client,
            symbol: None,
            from: None,
            to: None,
            timeframe: None,
            period: None,
        }
    }

    pub fn symbol(mut self, symbol: &str) -> Self {
        self.symbol = Some(symbol.to_string());
        self
    }

    pub fn from(mut self, from: &str) -> Self {
        self.from = Some(from.to_string());
        self
    }

    pub fn to(mut self, to: &str) -> Self {
        self.to = Some(to.to_string());
        self
    }

    pub fn timeframe(mut self, timeframe: &str) -> Self {
        self.timeframe = Some(timeframe.to_string());
        self
    }

    pub fn period(mut self, period: u32) -> Self {
        self.period = Some(period);
        self
    }

    pub fn send(self) -> Result<KdjResponse, MarketDataError> {
        let symbol = self.symbol.ok_or_else(|| MarketDataError::InvalidSymbol {
            symbol: "(not provided)".to_string(),
        })?;

        let mut url = format!(
            "{}/stock/technical/kdj/{}",
            self.client.get_base_url(),
            symbol
        );

        let mut query_params = Vec::new();
        if let Some(from) = self.from {
            query_params.push(format!("from={}", from));
        }
        if let Some(to) = self.to {
            query_params.push(format!("to={}", to));
        }
        if let Some(timeframe) = self.timeframe {
            query_params.push(format!("timeframe={}", timeframe));
        }
        if let Some(period) = self.period {
            query_params.push(format!("period={}", period));
        }

        if !query_params.is_empty() {
            url.push('?');
            url.push_str(&query_params.join("&"));
        }

        let request = self.client.agent().get(&url);
        let request = self.client.auth().apply_to_request(request);

        let response = request.call()?;
        let kdj_response: KdjResponse = response
            .into_json()
            .map_err(|e| MarketDataError::Other(e.into()))?;

        Ok(kdj_response)
    }
}
