//! Stock ownership data models — matches Fugle `stock/ownership/*/{symbol}`
//! responses (etf-holdings, institutional-trades, director-holdings,
//! tdcc-distribution).
//!
//! The three endpoints added in official SDK 1.6.0 / 2.6.0 are modelled from
//! the official TypeScript interfaces, which have been wrong about real
//! payloads before. Numeric fields are therefore `Option` and every list
//! defaults to empty, so a null or missing value degrades to `None` instead of
//! failing the whole response.

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

/// Buy / sell / net shares traded by one class of institutional investor.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "python", pyo3::prelude::pyclass)]
#[cfg_attr(feature = "js", napi_derive::napi(object))]
pub struct InstitutionalInvestorTrade {
    /// Shares bought
    pub buy: Option<f64>,

    /// Shares sold
    pub sell: Option<f64>,

    /// Net shares (buy − sell)
    pub net: Option<f64>,
}

/// Institutional investor trading on a single date.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "python", pyo3::prelude::pyclass)]
#[cfg_attr(feature = "js", napi_derive::napi(object))]
pub struct InstitutionalTradesEntry {
    /// Trading date (YYYY-MM-DD)
    pub date: String,

    /// Foreign investors
    pub foreign: Option<InstitutionalInvestorTrade>,

    /// Investment trusts
    pub trust: Option<InstitutionalInvestorTrade>,

    /// Dealers
    pub dealer: Option<InstitutionalInvestorTrade>,

    /// Combined net shares across all three investor classes
    pub total: Option<f64>,
}

/// Response for `stock/ownership/institutional-trades/{symbol}`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "python", pyo3::prelude::pyclass)]
#[cfg_attr(feature = "js", napi_derive::napi(object))]
pub struct InstitutionalTradesResponse {
    /// Security type
    #[serde(rename = "type")]
    pub data_type: Option<String>,

    /// Exchange code
    pub exchange: Option<String>,

    /// Market
    pub market: Option<String>,

    /// Stock symbol
    pub symbol: String,

    /// Trading by date
    #[serde(default)]
    pub data: Vec<InstitutionalTradesEntry>,
}

/// One director's or supervisor's disclosed holdings.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "python", pyo3::prelude::pyclass)]
#[cfg_attr(feature = "js", napi_derive::napi(object))]
pub struct DirectorHolding {
    /// Position of this director in the disclosure
    pub order: Option<i64>,

    /// Title (e.g. chairman, director, supervisor)
    #[serde(default)]
    pub title: String,

    /// Name
    #[serde(default)]
    pub name: String,

    /// Shares held when elected
    #[serde(rename = "electedShares")]
    pub elected_shares: Option<f64>,

    /// Shares currently held
    #[serde(rename = "heldShares")]
    pub held_shares: Option<f64>,

    /// Shares pledged
    #[serde(rename = "pledgedShares")]
    pub pledged_shares: Option<f64>,

    /// Pledged shares as a ratio of held shares
    #[serde(rename = "pledgeRatio")]
    pub pledge_ratio: Option<f64>,

    /// Shares held by related parties (spouse, minor children, nominees)
    #[serde(rename = "relatedHeldShares")]
    pub related_held_shares: Option<f64>,

    /// Shares pledged by related parties
    #[serde(rename = "relatedPledgedShares")]
    pub related_pledged_shares: Option<f64>,

    /// Related-party pledged shares as a ratio of related-party held shares
    #[serde(rename = "relatedPledgeRatio")]
    pub related_pledge_ratio: Option<f64>,
}

/// Director holdings disclosed for a single month.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "python", pyo3::prelude::pyclass)]
#[cfg_attr(feature = "js", napi_derive::napi(object))]
pub struct DirectorHoldingsEntry {
    /// Disclosure month (YYYY-MM)
    pub date: String,

    /// Directors and supervisors disclosed for this month
    #[serde(default)]
    pub directors: Vec<DirectorHolding>,
}

/// Response for `stock/ownership/director-holdings/{symbol}`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "python", pyo3::prelude::pyclass)]
#[cfg_attr(feature = "js", napi_derive::napi(object))]
pub struct DirectorHoldingsResponse {
    /// Security type
    #[serde(rename = "type")]
    pub data_type: Option<String>,

    /// Exchange code
    pub exchange: Option<String>,

    /// Market
    pub market: Option<String>,

    /// Stock symbol
    pub symbol: String,

    /// Holdings by disclosure month
    #[serde(default)]
    pub data: Vec<DirectorHoldingsEntry>,
}

/// One holding-size bracket of the TDCC shareholder distribution.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "python", pyo3::prelude::pyclass)]
#[cfg_attr(feature = "js", napi_derive::napi(object))]
pub struct TdccDistributionLevel {
    /// Holding-size bracket label, as returned by the API
    #[serde(default)]
    pub range: String,

    /// Number of shareholders in this bracket
    pub holders: Option<i64>,

    /// Shares held by this bracket
    pub shares: Option<f64>,

    /// Share of total outstanding held by this bracket, in percent
    pub proportion: Option<f64>,
}

/// TDCC shareholder distribution on a single date.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "python", pyo3::prelude::pyclass)]
#[cfg_attr(feature = "js", napi_derive::napi(object))]
pub struct TdccDistributionEntry {
    /// Data date (YYYY-MM-DD)
    pub date: String,

    /// Distribution by holding-size bracket
    #[serde(default)]
    pub distributions: Vec<TdccDistributionLevel>,
}

/// Response for `stock/ownership/tdcc-distribution/{symbol}`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "python", pyo3::prelude::pyclass)]
#[cfg_attr(feature = "js", napi_derive::napi(object))]
pub struct TdccDistributionResponse {
    /// Security type
    #[serde(rename = "type")]
    pub data_type: Option<String>,

    /// Exchange code
    pub exchange: Option<String>,

    /// Market
    pub market: Option<String>,

    /// Stock symbol
    pub symbol: String,

    /// Distribution by date
    #[serde(default)]
    pub data: Vec<TdccDistributionEntry>,
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

    #[test]
    fn test_institutional_trades_deserialization() {
        let json = r#"{
            "type": "EQUITY", "exchange": "TWSE", "market": "TSE", "symbol": "2330",
            "data": [{
                "date": "2026-07-31",
                "foreign": {"buy": 30000000, "sell": 25000000, "net": 5000000},
                "trust": {"buy": 1200000, "sell": 800000, "net": 400000},
                "dealer": {"buy": 500000, "sell": 900000, "net": -400000},
                "total": 5000000
            }]
        }"#;

        let response: InstitutionalTradesResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.symbol, "2330");
        let entry = &response.data[0];
        assert_eq!(entry.date, "2026-07-31");
        assert_eq!(entry.foreign.as_ref().unwrap().net, Some(5000000.0));
        assert_eq!(entry.dealer.as_ref().unwrap().net, Some(-400000.0));
        assert_eq!(entry.total, Some(5000000.0));
    }

    #[test]
    fn test_institutional_trades_tolerates_null_and_missing() {
        let json = r#"{
            "symbol": "2330",
            "data": [{"date": "2026-07-31", "foreign": {"buy": null, "sell": 1, "net": null}, "trust": null}]
        }"#;

        let response: InstitutionalTradesResponse = serde_json::from_str(json).unwrap();
        let entry = &response.data[0];
        assert_eq!(entry.foreign.as_ref().unwrap().buy, None);
        assert_eq!(entry.trust, None);
        assert_eq!(entry.dealer, None);
        assert_eq!(entry.total, None);
    }

    #[test]
    fn test_director_holdings_deserialization() {
        let json = r#"{
            "symbol": "2330",
            "data": [{
                "date": "2026-05",
                "directors": [{
                    "order": 1, "title": "董事長", "name": "魏哲家",
                    "electedShares": 1000, "heldShares": 1200, "pledgedShares": 0,
                    "pledgeRatio": 0, "relatedHeldShares": 50,
                    "relatedPledgedShares": 0, "relatedPledgeRatio": 0
                }]
            }]
        }"#;

        let response: DirectorHoldingsResponse = serde_json::from_str(json).unwrap();
        let entry = &response.data[0];
        assert_eq!(entry.date, "2026-05");
        let director = &entry.directors[0];
        assert_eq!(director.order, Some(1));
        assert_eq!(director.title, "董事長");
        assert_eq!(director.held_shares, Some(1200.0));
        assert_eq!(director.related_held_shares, Some(50.0));
    }

    #[test]
    fn test_director_holdings_tolerates_null_ratios() {
        // A director with nothing held has no meaningful pledge ratio; a null
        // there must not sink the whole month.
        let json = r#"{
            "symbol": "2330",
            "data": [{"date": "2026-05", "directors": [
                {"order": 2, "title": "董事", "name": "某法人代表", "heldShares": 0, "pledgeRatio": null}
            ]}]
        }"#;

        let response: DirectorHoldingsResponse = serde_json::from_str(json).unwrap();
        let director = &response.data[0].directors[0];
        assert_eq!(director.pledge_ratio, None);
        assert_eq!(director.elected_shares, None);
    }

    #[test]
    fn test_tdcc_distribution_deserialization() {
        let json = r#"{
            "symbol": "2330",
            "data": [{
                "date": "2026-07-03",
                "distributions": [
                    {"range": "1-999", "holders": 1500000, "shares": 250000000, "proportion": 0.96},
                    {"range": "1,000,001以上", "holders": 1500, "shares": 22000000000, "proportion": 84.8}
                ]
            }]
        }"#;

        let response: TdccDistributionResponse = serde_json::from_str(json).unwrap();
        let entry = &response.data[0];
        assert_eq!(entry.distributions.len(), 2);
        assert_eq!(entry.distributions[0].holders, Some(1500000));
        assert_eq!(entry.distributions[1].shares, Some(22000000000.0));
        assert_eq!(entry.distributions[1].proportion, Some(84.8));
    }

    #[test]
    fn test_new_ownership_responses_tolerate_empty_data() {
        let json = r#"{"symbol": "2330"}"#;
        assert!(serde_json::from_str::<InstitutionalTradesResponse>(json).unwrap().data.is_empty());
        assert!(serde_json::from_str::<DirectorHoldingsResponse>(json).unwrap().data.is_empty());
        assert!(serde_json::from_str::<TdccDistributionResponse>(json).unwrap().data.is_empty());
    }
}
