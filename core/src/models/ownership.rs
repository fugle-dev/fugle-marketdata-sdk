//! Stock ownership data models — matches Fugle
//! `stock/ownership/etf-holdings/{symbol}` responses.

use serde::{Deserialize, Serialize};

/// One constituent of an ETF's holdings on a given date.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "python", pyo3::prelude::pyclass)]
#[cfg_attr(feature = "js", napi_derive::napi(object))]
pub struct EtfHoldingComponent {
    /// Constituent symbol (e.g. `"2330"`)
    pub symbol: String,

    /// Constituent name
    pub name: String,

    /// Number of shares held
    pub quantity: f64,

    /// Portfolio weight, in percent
    pub weight: f64,

    /// Change in shares held versus the previous disclosure. Absent on the
    /// first date in a series, where there is nothing to compare against.
    #[serde(rename = "quantityChange")]
    pub quantity_change: Option<f64>,

    /// Change in portfolio weight versus the previous disclosure. Absent on
    /// the first date in a series.
    #[serde(rename = "weightChange")]
    pub weight_change: Option<f64>,
}

/// Holdings disclosed on a single date.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "python", pyo3::prelude::pyclass)]
#[cfg_attr(feature = "js", napi_derive::napi(object))]
pub struct EtfHoldingsEntry {
    /// Disclosure date (YYYY-MM-DD)
    pub date: String,

    /// Constituents held on this date
    #[serde(default)]
    pub components: Vec<EtfHoldingComponent>,
}

/// Response for `stock/ownership/etf-holdings/{symbol}`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "python", pyo3::prelude::pyclass)]
#[cfg_attr(feature = "js", napi_derive::napi(object))]
pub struct EtfHoldingsResponse {
    /// Security type
    #[serde(rename = "type")]
    pub data_type: Option<String>,

    /// Exchange code
    pub exchange: Option<String>,

    /// Market
    pub market: Option<String>,

    /// The ETF symbol these holdings belong to
    pub symbol: String,

    /// Holdings by disclosure date
    #[serde(default)]
    pub data: Vec<EtfHoldingsEntry>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_etf_holdings_deserialization() {
        let json = r#"{
            "type": "EQUITY",
            "exchange": "TWSE",
            "market": "TSE",
            "symbol": "0050",
            "data": [
                {
                    "date": "2026-07-31",
                    "components": [
                        {
                            "symbol": "2330",
                            "name": "台積電",
                            "quantity": 123456789.0,
                            "weight": 57.12,
                            "quantityChange": -1000.0,
                            "weightChange": 0.35
                        }
                    ]
                }
            ]
        }"#;

        let response: EtfHoldingsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.symbol, "0050");
        assert_eq!(response.data.len(), 1);

        let entry = &response.data[0];
        assert_eq!(entry.date, "2026-07-31");

        let component = &entry.components[0];
        assert_eq!(component.symbol, "2330");
        assert_eq!(component.weight, 57.12);
        assert_eq!(component.quantity_change, Some(-1000.0));
    }

    #[test]
    fn test_change_fields_are_optional() {
        // The first date in a series has nothing to diff against, so the
        // server omits both change fields rather than sending zeros.
        let json = r#"{
            "symbol": "0050",
            "data": [{
                "date": "2026-07-31",
                "components": [
                    {"symbol": "2330", "name": "台積電", "quantity": 1.0, "weight": 57.12}
                ]
            }]
        }"#;

        let response: EtfHoldingsResponse = serde_json::from_str(json).unwrap();
        let component = &response.data[0].components[0];
        assert_eq!(component.quantity_change, None);
        assert_eq!(component.weight_change, None);
    }

    #[test]
    fn test_empty_data_is_tolerated() {
        let response: EtfHoldingsResponse =
            serde_json::from_str(r#"{"symbol": "0050"}"#).unwrap();
        assert!(response.data.is_empty());
    }
}
