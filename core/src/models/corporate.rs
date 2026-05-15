//! Corporate action response models for Fugle API
//!
//! This module contains response types for corporate action endpoints:
//! - Capital Changes (capital structure changes)
//! - Dividends (dividend announcements)
//! - Listing Applicants (IPO listings)

use serde::{Deserialize, Serialize};

// =============================================================================
// Capital Changes
// =============================================================================

/// Response for capital changes endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "python", pyo3::prelude::pyclass)]
#[cfg_attr(feature = "js", napi_derive::napi(object))]
pub struct CapitalChangesResponse {
    /// Response type (e.g., "CAPITAL_CHANGES")
    #[serde(rename = "type")]
    pub data_type: String,
    /// Exchange code (e.g., "TWSE")
    pub exchange: String,
    /// Market (e.g., "TSE")
    pub market: String,
    /// List of capital change records
    pub data: Vec<CapitalChange>,
}

/// Single capital change record
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "python", pyo3::prelude::pyclass)]
#[cfg_attr(feature = "js", napi_derive::napi(object))]
pub struct CapitalChange {
    /// Stock symbol (e.g., "2330")
    pub symbol: String,
    /// Company name
    pub name: Option<String>,
    /// Date of the capital change (YYYY-MM-DD)
    pub date: String,
    /// Previous capital (in TWD)
    #[serde(rename = "previousCapital")]
    pub previous_capital: Option<f64>,
    /// Current capital (in TWD)
    #[serde(rename = "currentCapital")]
    pub current_capital: Option<f64>,
    /// Type of change (e.g., "increase", "decrease")
    #[serde(rename = "changeType")]
    pub change_type: Option<String>,
    /// Reason for the change
    pub reason: Option<String>,
}

// =============================================================================
// Dividends
// =============================================================================

/// Response for dividends endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "python", pyo3::prelude::pyclass)]
#[cfg_attr(feature = "js", napi_derive::napi(object))]
pub struct DividendsResponse {
    /// Response type (e.g., "DIVIDENDS")
    #[serde(rename = "type")]
    pub data_type: String,
    /// Exchange code (e.g., "TWSE")
    pub exchange: String,
    /// Market (e.g., "TSE")
    pub market: String,
    /// List of dividend records
    pub data: Vec<Dividend>,
}

/// Single dividend announcement record
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "python", pyo3::prelude::pyclass)]
#[cfg_attr(feature = "js", napi_derive::napi(object))]
pub struct Dividend {
    /// Stock symbol (e.g., "2330")
    pub symbol: String,
    /// Company name
    pub name: Option<String>,
    /// Ex-dividend date (YYYY-MM-DD)
    #[serde(rename = "exDividendDate")]
    pub ex_dividend_date: Option<String>,
    /// Payment date (YYYY-MM-DD)
    #[serde(rename = "paymentDate")]
    pub payment_date: Option<String>,
    /// Cash dividend amount per share (in TWD)
    #[serde(rename = "cashDividend")]
    pub cash_dividend: Option<f64>,
    /// Stock dividend ratio (shares per 1000 shares)
    #[serde(rename = "stockDividend")]
    pub stock_dividend: Option<f64>,
    /// Dividend year (fiscal year)
    #[serde(rename = "dividendYear")]
    pub dividend_year: Option<String>,
}

// =============================================================================
// Listing Applicants (IPO)
// =============================================================================

/// Response for listing applicants (IPO) endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "python", pyo3::prelude::pyclass)]
#[cfg_attr(feature = "js", napi_derive::napi(object))]
pub struct ListingApplicantsResponse {
    /// Response type (e.g., "LISTING_APPLICANTS")
    #[serde(rename = "type")]
    pub data_type: String,
    /// Exchange code (e.g., "TWSE")
    pub exchange: String,
    /// Market (e.g., "TSE")
    pub market: String,
    /// List of listing applicant records
    pub data: Vec<ListingApplicant>,
}

/// Single listing applicant (IPO) record
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "python", pyo3::prelude::pyclass)]
#[cfg_attr(feature = "js", napi_derive::napi(object))]
pub struct ListingApplicant {
    /// Stock symbol (e.g., "6XXX")
    pub symbol: String,
    /// Company name
    pub name: Option<String>,
    /// Application date (YYYY-MM-DD)
    #[serde(rename = "applicationDate")]
    pub application_date: Option<String>,
    /// Expected or actual listing date (YYYY-MM-DD)
    #[serde(rename = "listingDate")]
    pub listing_date: Option<String>,
    /// Application status (e.g., "pending", "approved", "listed")
    pub status: Option<String>,
    /// Industry classification
    pub industry: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capital_changes_response_deserialization() {
        let json = r#"{
            "type": "CAPITAL_CHANGES",
            "exchange": "TWSE",
            "market": "TSE",
            "data": [
                {
                    "symbol": "2330",
                    "name": "TSMC",
                    "date": "2024-01-15",
                    "previousCapital": 259303804580.0,
                    "currentCapital": 259303804580.0,
                    "changeType": "increase",
                    "reason": "Cash capital increase"
                }
            ]
        }"#;

        let response: CapitalChangesResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.data_type, "CAPITAL_CHANGES");
        assert_eq!(response.exchange, "TWSE");
        assert_eq!(response.market, "TSE");
        assert_eq!(response.data.len(), 1);
        assert_eq!(response.data[0].symbol, "2330");
        assert_eq!(response.data[0].name.as_deref(), Some("TSMC"));
        assert_eq!(response.data[0].date, "2024-01-15");
        assert_eq!(response.data[0].change_type.as_deref(), Some("increase"));
    }

    #[test]
    fn test_dividends_response_deserialization() {
        let json = r#"{
            "type": "DIVIDENDS",
            "exchange": "TWSE",
            "market": "TSE",
            "data": [
                {
                    "symbol": "2330",
                    "name": "TSMC",
                    "exDividendDate": "2024-06-15",
                    "paymentDate": "2024-07-15",
                    "cashDividend": 3.0,
                    "stockDividend": 0.0,
                    "dividendYear": "2023"
                }
            ]
        }"#;

        let response: DividendsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.data_type, "DIVIDENDS");
        assert_eq!(response.exchange, "TWSE");
        assert_eq!(response.data.len(), 1);
        assert_eq!(response.data[0].symbol, "2330");
        assert_eq!(response.data[0].cash_dividend, Some(3.0));
        assert_eq!(response.data[0].stock_dividend, Some(0.0));
        assert_eq!(response.data[0].ex_dividend_date.as_deref(), Some("2024-06-15"));
    }

    #[test]
    fn test_listing_applicants_response_deserialization() {
        let json = r#"{
            "type": "LISTING_APPLICANTS",
            "exchange": "TWSE",
            "market": "TSE",
            "data": [
                {
                    "symbol": "6XXX",
                    "name": "New Tech Corp",
                    "applicationDate": "2024-01-01",
                    "listingDate": "2024-03-15",
                    "status": "approved",
                    "industry": "Technology"
                }
            ]
        }"#;

        let response: ListingApplicantsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.data_type, "LISTING_APPLICANTS");
        assert_eq!(response.exchange, "TWSE");
        assert_eq!(response.data.len(), 1);
        assert_eq!(response.data[0].symbol, "6XXX");
        assert_eq!(response.data[0].name.as_deref(), Some("New Tech Corp"));
        assert_eq!(response.data[0].status.as_deref(), Some("approved"));
        assert_eq!(response.data[0].industry.as_deref(), Some("Technology"));
    }

    #[test]
    fn test_capital_changes_minimal() {
        let json = r#"{
            "type": "CAPITAL_CHANGES",
            "exchange": "TWSE",
            "market": "TSE",
            "data": [{"symbol": "2330", "date": "2024-01-15"}]
        }"#;

        let response: CapitalChangesResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.data[0].symbol, "2330");
        assert!(response.data[0].name.is_none());
        assert!(response.data[0].previous_capital.is_none());
    }

    #[test]
    fn test_dividends_minimal() {
        let json = r#"{
            "type": "DIVIDENDS",
            "exchange": "TWSE",
            "market": "TSE",
            "data": [{"symbol": "2330"}]
        }"#;

        let response: DividendsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.data[0].symbol, "2330");
        assert!(response.data[0].cash_dividend.is_none());
        assert!(response.data[0].ex_dividend_date.is_none());
    }

    #[test]
    fn test_listing_applicants_minimal() {
        let json = r#"{
            "type": "LISTING_APPLICANTS",
            "exchange": "TWSE",
            "market": "TSE",
            "data": [{"symbol": "6XXX"}]
        }"#;

        let response: ListingApplicantsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.data[0].symbol, "6XXX");
        assert!(response.data[0].name.is_none());
        assert!(response.data[0].status.is_none());
    }
}
