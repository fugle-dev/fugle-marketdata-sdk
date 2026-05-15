//! FutOpt ticker data model - matches Fugle futopt/intraday/ticker/{symbol} response

use serde::{Deserialize, Serialize};

/// FutOpt contract information from Fugle API (futopt/intraday/ticker/{symbol})
///
/// This matches the official SDK's RestFutOptIntradayTickerResponse.
/// Contains contract metadata including expiration dates which are critical
/// for futures and options trading.
///
/// # Example
///
/// ```rust
/// use marketdata_core::models::futopt::FutOptTicker;
///
/// let json = r#"{
///     "date": "2024-01-15",
///     "type": "FUTURE",
///     "exchange": "TAIFEX",
///     "symbol": "TXFC4",
///     "name": "臺股期貨 03",
///     "referencePrice": 17500.0,
///     "startDate": "2023-12-20",
///     "endDate": "2024-03-20",
///     "settlementDate": "2024-03-20"
/// }"#;
///
/// let ticker: FutOptTicker = serde_json::from_str(json).unwrap();
/// assert_eq!(ticker.symbol, "TXFC4");
/// assert_eq!(ticker.end_date.as_deref(), Some("2024-03-20"));
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct FutOptTicker {
    // === Response metadata ===
    /// Trading date (YYYY-MM-DD)
    pub date: String,

    /// Contract type (FUTURE or OPTION)
    #[serde(rename = "type")]
    pub contract_type: Option<String>,

    /// Exchange code (TAIFEX)
    pub exchange: Option<String>,

    /// Contract symbol (e.g., "TXFC4", "TXO18000C4")
    pub symbol: String,

    /// Contract name (e.g., "臺股期貨 03", "臺指選擇權 18000C 03")
    pub name: Option<String>,

    // === Reference price ===
    /// Reference price (previous settlement price)
    #[serde(rename = "referencePrice")]
    pub reference_price: Option<f64>,

    // === Contract dates (critical for FutOpt) ===
    /// Contract start date (YYYY-MM-DD) - when the contract starts trading
    #[serde(rename = "startDate")]
    pub start_date: Option<String>,

    /// Contract end date (YYYY-MM-DD) - last trading date
    #[serde(rename = "endDate")]
    pub end_date: Option<String>,

    /// Settlement date (YYYY-MM-DD) - when the contract settles
    #[serde(rename = "settlementDate")]
    pub settlement_date: Option<String>,

    // === Additional fields from tickers endpoint ===
    /// Contract sub-type (e.g., "I" for Index)
    #[serde(rename = "contractType")]
    pub contract_sub_type: Option<String>,

    /// Whether dynamic price banding is enabled
    #[serde(rename = "isDynamicBanding", default)]
    pub is_dynamic_banding: bool,

    /// Flow group for trading
    #[serde(rename = "flowGroup")]
    pub flow_group: Option<i32>,
}

impl FutOptTicker {
    /// Check if the contract has valid date information
    pub fn has_contract_dates(&self) -> bool {
        self.start_date.is_some() && self.end_date.is_some()
    }

    /// Check if this is likely a futures contract based on type
    pub fn is_future(&self) -> bool {
        self.contract_type
            .as_ref()
            .is_some_and(|t| t.to_uppercase() == "FUTURE")
    }

    /// Check if this is likely an options contract based on type
    pub fn is_option(&self) -> bool {
        self.contract_type
            .as_ref()
            .is_some_and(|t| t.to_uppercase() == "OPTION")
    }

    /// Get days until expiration from a given date
    /// Returns None if end_date is not set or date parsing fails
    ///
    /// Note: This is a simple string comparison, not actual date calculation.
    /// For production use, consider using a proper date library.
    pub fn is_expired_on(&self, date: &str) -> Option<bool> {
        self.end_date.as_ref().map(|end| date > end.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_futopt_ticker_deserialization() {
        let json = r#"{
            "date": "2024-01-15",
            "type": "FUTURE",
            "exchange": "TAIFEX",
            "symbol": "TXFC4",
            "name": "臺股期貨 03",
            "referencePrice": 17500.0,
            "startDate": "2023-12-20",
            "endDate": "2024-03-20",
            "settlementDate": "2024-03-20"
        }"#;

        let ticker: FutOptTicker = serde_json::from_str(json).unwrap();
        assert_eq!(ticker.symbol, "TXFC4");
        assert_eq!(ticker.name.as_deref(), Some("臺股期貨 03"));
        assert_eq!(ticker.contract_type.as_deref(), Some("FUTURE"));
        assert_eq!(ticker.exchange.as_deref(), Some("TAIFEX"));
        assert_eq!(ticker.reference_price, Some(17500.0));
        assert_eq!(ticker.start_date.as_deref(), Some("2023-12-20"));
        assert_eq!(ticker.end_date.as_deref(), Some("2024-03-20"));
        assert_eq!(ticker.settlement_date.as_deref(), Some("2024-03-20"));
    }

    #[test]
    fn test_futopt_ticker_with_tickers_fields() {
        let json = r#"{
            "date": "2024-01-15",
            "type": "FUTURE",
            "contractType": "I",
            "symbol": "TXFC4",
            "name": "臺股期貨 03",
            "referencePrice": 17500.0,
            "isDynamicBanding": true,
            "flowGroup": 1,
            "startDate": "2023-12-20",
            "endDate": "2024-03-20",
            "settlementDate": "2024-03-20"
        }"#;

        let ticker: FutOptTicker = serde_json::from_str(json).unwrap();
        assert_eq!(ticker.contract_sub_type.as_deref(), Some("I"));
        assert!(ticker.is_dynamic_banding);
        assert_eq!(ticker.flow_group, Some(1));
    }

    #[test]
    fn test_futopt_ticker_minimal() {
        let json = r#"{"date": "2024-01-15", "symbol": "TXFC4"}"#;
        let ticker: FutOptTicker = serde_json::from_str(json).unwrap();
        assert_eq!(ticker.symbol, "TXFC4");
        assert!(ticker.name.is_none());
        assert!(ticker.reference_price.is_none());
        assert!(!ticker.has_contract_dates());
    }

    #[test]
    fn test_futopt_ticker_has_contract_dates() {
        let mut ticker = FutOptTicker::default();
        assert!(!ticker.has_contract_dates());

        ticker.start_date = Some("2023-12-20".to_string());
        assert!(!ticker.has_contract_dates());

        ticker.end_date = Some("2024-03-20".to_string());
        assert!(ticker.has_contract_dates());
    }

    #[test]
    fn test_futopt_ticker_is_future() {
        let mut ticker = FutOptTicker::default();
        assert!(!ticker.is_future());

        ticker.contract_type = Some("FUTURE".to_string());
        assert!(ticker.is_future());

        ticker.contract_type = Some("Future".to_string());
        assert!(ticker.is_future());

        ticker.contract_type = Some("OPTION".to_string());
        assert!(!ticker.is_future());
    }

    #[test]
    fn test_futopt_ticker_is_option() {
        let mut ticker = FutOptTicker::default();
        assert!(!ticker.is_option());

        ticker.contract_type = Some("OPTION".to_string());
        assert!(ticker.is_option());

        ticker.contract_type = Some("Option".to_string());
        assert!(ticker.is_option());

        ticker.contract_type = Some("FUTURE".to_string());
        assert!(!ticker.is_option());
    }

    #[test]
    fn test_futopt_ticker_is_expired_on() {
        let mut ticker = FutOptTicker::default();
        assert!(ticker.is_expired_on("2024-01-15").is_none());

        ticker.end_date = Some("2024-03-20".to_string());
        assert_eq!(ticker.is_expired_on("2024-03-19"), Some(false));
        assert_eq!(ticker.is_expired_on("2024-03-20"), Some(false));
        assert_eq!(ticker.is_expired_on("2024-03-21"), Some(true));
    }

    #[test]
    fn test_option_ticker_example() {
        let json = r#"{
            "date": "2024-01-15",
            "type": "OPTION",
            "exchange": "TAIFEX",
            "symbol": "TXO18000C4",
            "name": "臺指選擇權 18000C 03",
            "referencePrice": 150.0,
            "startDate": "2023-12-20",
            "endDate": "2024-03-20",
            "settlementDate": "2024-03-20"
        }"#;

        let ticker: FutOptTicker = serde_json::from_str(json).unwrap();
        assert_eq!(ticker.symbol, "TXO18000C4");
        assert!(ticker.is_option());
        assert!(!ticker.is_future());
    }
}
