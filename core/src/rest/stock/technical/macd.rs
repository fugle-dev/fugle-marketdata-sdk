//! MACD endpoint - GET /stock/technical/macd/{symbol}
//! Will be completed in Task 2

use crate::{errors::MarketDataError, models::MacdResponse, rest::client::RestClient};

/// Request builder for MACD (Moving Average Convergence Divergence) endpoint
pub struct MacdRequestBuilder<'a> {
    client: &'a RestClient,
    symbol: Option<String>,
    from: Option<String>,
    to: Option<String>,
    timeframe: Option<String>,
    fast: Option<u32>,
    slow: Option<u32>,
    signal: Option<u32>,
}

impl<'a> MacdRequestBuilder<'a> {
    pub(crate) fn new(client: &'a RestClient) -> Self {
        Self {
            client,
            symbol: None,
            from: None,
            to: None,
            timeframe: None,
            fast: None,
            slow: None,
            signal: None,
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

    /// Set the fast EMA period (typically 12)
    pub fn fast(mut self, fast: u32) -> Self {
        self.fast = Some(fast);
        self
    }

    /// Set the slow EMA period (typically 26)
    pub fn slow(mut self, slow: u32) -> Self {
        self.slow = Some(slow);
        self
    }

    /// Set the signal line period (typically 9)
    pub fn signal(mut self, signal: u32) -> Self {
        self.signal = Some(signal);
        self
    }

    pub fn send(self) -> Result<MacdResponse, MarketDataError> {
        let symbol = self.symbol.ok_or_else(|| MarketDataError::InvalidSymbol {
            symbol: "(not provided)".to_string(),
        })?;

        let mut url = format!(
            "{}/stock/technical/macd/{}",
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
        if let Some(fast) = self.fast {
            query_params.push(format!("fast={}", fast));
        }
        if let Some(slow) = self.slow {
            query_params.push(format!("slow={}", slow));
        }
        if let Some(signal) = self.signal {
            query_params.push(format!("signal={}", signal));
        }

        if !query_params.is_empty() {
            url.push('?');
            url.push_str(&query_params.join("&"));
        }

        let request = self.client.agent().get(&url);
        let request = self.client.auth().apply_to_request(request);

        let response = request.call()?;
        let macd_response: MacdResponse = response
            .into_json()
            .map_err(|e| MarketDataError::Other(e.into()))?;

        Ok(macd_response)
    }
}
