//! Historical candles endpoint - GET /futopt/historical/candles/{product}

use crate::{errors::MarketDataError, rest::client::RestClient};

/// Request builder for FutOpt historical candles endpoint
pub struct FutOptHistoricalCandlesRequestBuilder<'a> {
    client: &'a RestClient,
    symbol: Option<String>,
    from: Option<String>,
    to: Option<String>,
    contract_month: Option<String>,
    fields: Option<String>,
    timeframe: Option<String>,
    sort: Option<String>,
    after_hours: Option<bool>,
}

impl<'a> FutOptHistoricalCandlesRequestBuilder<'a> {
    /// Create a new historical candles request builder
    pub(crate) fn new(client: &'a RestClient) -> Self {
        Self {
            client,
            symbol: None,
            from: None,
            to: None,
            contract_month: None,
            fields: None,
            timeframe: None,
            sort: None,
            after_hours: None,
        }
    }

    /// Set the product code (required, e.g. `"TXF"`).
    ///
    /// This is the **product**, not a contract: a contract code such as
    /// `"TXFC4"` returns HTTP 404. Pick the contract with
    /// [`contract_month`](Self::contract_month).
    pub fn symbol(mut self, symbol: &str) -> Self {
        self.symbol = Some(symbol.to_string());
        self
    }

    /// Set the start date (format: YYYY-MM-DD)
    pub fn from(mut self, from: &str) -> Self {
        self.from = Some(from.to_string());
        self
    }

    /// Set the end date (format: YYYY-MM-DD)
    pub fn to(mut self, to: &str) -> Self {
        self.to = Some(to.to_string());
        self
    }

    /// Set the contract month: `YYYYMM` (e.g. `"202609"`), or a continuous
    /// contract — `"1!"` front month, `"2!"` next, `"3!"` third. The server
    /// defaults to `"1!"`.
    pub fn contract_month(mut self, contract_month: &str) -> Self {
        self.contract_month = Some(contract_month.to_string());
        self
    }

    /// Set the fields to return, comma-separated, from
    /// `open,high,low,close,volume,average,transaction,change`
    /// (`average` and `transaction` only have values on intraday timeframes).
    pub fn fields(mut self, fields: &str) -> Self {
        self.fields = Some(fields.to_string());
        self
    }

    /// Set the timeframe (e.g., "D", "W", "M", "1", "5", "10", "15", "30", "60")
    pub fn timeframe(mut self, timeframe: &str) -> Self {
        self.timeframe = Some(timeframe.to_string());
        self
    }

    /// Set the sort order: `"asc"` or `"desc"`.
    pub fn sort(mut self, sort: &str) -> Self {
        self.sort = Some(sort.to_string());
        self
    }

    /// Query the after-hours session (`session=afterhours`) instead of the
    /// regular session.
    pub fn after_hours(mut self, after_hours: bool) -> Self {
        self.after_hours = Some(after_hours);
        self
    }

    /// Execute the request and return the historical candles response
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
            "{}/futopt/historical/candles/{}",
            self.client.get_base_url(),
            crate::rest::encode_symbol(symbol)
        );

        let mut query_params = Vec::new();
        if let Some(from) = &self.from {
            query_params.push(crate::rest::query_pair("from", from));
        }
        if let Some(to) = &self.to {
            query_params.push(crate::rest::query_pair("to", to));
        }
        if let Some(contract_month) = &self.contract_month {
            query_params.push(crate::rest::query_pair("contractMonth", contract_month));
        }
        if let Some(fields) = &self.fields {
            query_params.push(crate::rest::query_pair("fields", fields));
        }
        if let Some(timeframe) = &self.timeframe {
            query_params.push(crate::rest::query_pair("timeframe", timeframe));
        }
        if let Some(sort) = &self.sort {
            query_params.push(crate::rest::query_pair("sort", sort));
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
    fn test_historical_candles_builder_requires_symbol() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let builder = FutOptHistoricalCandlesRequestBuilder::new(&client);

        let result = builder.send();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MarketDataError::InvalidSymbol { .. }
        ));
    }

    #[test]
    fn test_historical_candles_url_without_query() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let url = FutOptHistoricalCandlesRequestBuilder::new(&client)
            .symbol("TXF")
            .url()
            .unwrap();
        assert_eq!(url, format!("{}/futopt/historical/candles/TXF", client.get_base_url()));
    }

    #[test]
    fn test_historical_candles_url_with_full_query() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let url = FutOptHistoricalCandlesRequestBuilder::new(&client)
            .symbol("TXF")
            .from("2026-09-01")
            .to("2026-09-15")
            .contract_month("202609")
            .fields("open,close,volume")
            .timeframe("5")
            .sort("desc")
            .after_hours(true)
            .url()
            .unwrap();
        assert_eq!(
            url,
            format!(
                "{}/futopt/historical/candles/TXF?from=2026-09-01&to=2026-09-15&contractMonth=202609\
                 &fields=open%2Cclose%2Cvolume&timeframe=5&sort=desc&session=afterhours",
                client.get_base_url()
            )
        );
    }

    #[test]
    fn test_historical_candles_url_continuous_contract_month() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let url = FutOptHistoricalCandlesRequestBuilder::new(&client)
            .symbol("TXF")
            .contract_month("2!")
            .after_hours(false)
            .url()
            .unwrap();
        assert_eq!(
            url,
            format!("{}/futopt/historical/candles/TXF?contractMonth=2!", client.get_base_url())
        );
    }

    #[test]
    fn test_historical_candles_url_encodes_fields_value() {
        // An `&` in a value must not split into an extra `session` param.
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let url = FutOptHistoricalCandlesRequestBuilder::new(&client)
            .symbol("TXF")
            .fields("open&session=afterhours")
            .url()
            .unwrap();
        assert_eq!(
            url,
            format!(
                "{}/futopt/historical/candles/TXF?fields=open%26session%3Dafterhours",
                client.get_base_url()
            )
        );
    }
}
