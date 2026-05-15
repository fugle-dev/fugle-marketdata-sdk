//! Common types for FutOpt (Futures and Options) market data
//!
//! Contains enums and shared types used across FutOpt REST and WebSocket APIs.

use serde::{Deserialize, Serialize};

/// Type of futures/options contract
///
/// Serializes to uppercase strings: "FUTURE", "OPTION"
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "UPPERCASE")]
pub enum FutOptType {
    /// Futures contract
    Future,
    /// Options contract
    Option,
}

impl FutOptType {
    /// Get the string representation for API requests
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Future => "FUTURE",
            Self::Option => "OPTION",
        }
    }
}

impl std::fmt::Display for FutOptType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Contract type for futures/options
///
/// Serializes to single-letter codes: I, R, B, C, S, E
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ContractType {
    /// Index futures/options (e.g., TX, TE)
    #[serde(rename = "I")]
    Index,
    /// Interest rate futures
    #[serde(rename = "R")]
    Rate,
    /// Bond futures
    #[serde(rename = "B")]
    Bond,
    /// Currency futures (e.g., USD/TWD)
    #[serde(rename = "C")]
    Currency,
    /// Stock futures/options
    #[serde(rename = "S")]
    Stock,
    /// ETF futures/options
    #[serde(rename = "E")]
    Etf,
}

impl ContractType {
    /// Get the single-letter code for API requests
    pub fn as_code(&self) -> &'static str {
        match self {
            Self::Index => "I",
            Self::Rate => "R",
            Self::Bond => "B",
            Self::Currency => "C",
            Self::Stock => "S",
            Self::Etf => "E",
        }
    }

    /// Get the full name of the contract type
    pub fn name(&self) -> &'static str {
        match self {
            Self::Index => "Index",
            Self::Rate => "Rate",
            Self::Bond => "Bond",
            Self::Currency => "Currency",
            Self::Stock => "Stock",
            Self::Etf => "ETF",
        }
    }
}

impl std::fmt::Display for ContractType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_code())
    }
}

/// Trading session for FutOpt markets
///
/// - `Regular`: Normal trading hours
/// - `AfterHours`: After-hours trading session
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
pub enum FutOptSession {
    /// Regular trading session (default)
    #[default]
    #[serde(rename = "regular")]
    Regular,
    /// After-hours trading session
    #[serde(rename = "afterhours")]
    AfterHours,
}

impl FutOptSession {
    /// Get the string representation for API requests
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Regular => "regular",
            Self::AfterHours => "afterhours",
        }
    }

    /// Check if this is the after-hours session
    pub fn is_after_hours(&self) -> bool {
        matches!(self, Self::AfterHours)
    }
}

impl std::fmt::Display for FutOptSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Option right type (call or put)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum OptionRight {
    /// Call option (right to buy)
    Call,
    /// Put option (right to sell)
    Put,
}

impl OptionRight {
    /// Get the string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Call => "call",
            Self::Put => "put",
        }
    }
}

impl std::fmt::Display for OptionRight {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_futopt_type_serialization() {
        assert_eq!(
            serde_json::to_string(&FutOptType::Future).unwrap(),
            "\"FUTURE\""
        );
        assert_eq!(
            serde_json::to_string(&FutOptType::Option).unwrap(),
            "\"OPTION\""
        );
    }

    #[test]
    fn test_futopt_type_deserialization() {
        assert_eq!(
            serde_json::from_str::<FutOptType>("\"FUTURE\"").unwrap(),
            FutOptType::Future
        );
        assert_eq!(
            serde_json::from_str::<FutOptType>("\"OPTION\"").unwrap(),
            FutOptType::Option
        );
    }

    #[test]
    fn test_futopt_type_as_str() {
        assert_eq!(FutOptType::Future.as_str(), "FUTURE");
        assert_eq!(FutOptType::Option.as_str(), "OPTION");
    }

    #[test]
    fn test_contract_type_serialization() {
        assert_eq!(serde_json::to_string(&ContractType::Index).unwrap(), "\"I\"");
        assert_eq!(serde_json::to_string(&ContractType::Rate).unwrap(), "\"R\"");
        assert_eq!(serde_json::to_string(&ContractType::Bond).unwrap(), "\"B\"");
        assert_eq!(
            serde_json::to_string(&ContractType::Currency).unwrap(),
            "\"C\""
        );
        assert_eq!(serde_json::to_string(&ContractType::Stock).unwrap(), "\"S\"");
        assert_eq!(serde_json::to_string(&ContractType::Etf).unwrap(), "\"E\"");
    }

    #[test]
    fn test_contract_type_deserialization() {
        assert_eq!(
            serde_json::from_str::<ContractType>("\"I\"").unwrap(),
            ContractType::Index
        );
        assert_eq!(
            serde_json::from_str::<ContractType>("\"S\"").unwrap(),
            ContractType::Stock
        );
        assert_eq!(
            serde_json::from_str::<ContractType>("\"E\"").unwrap(),
            ContractType::Etf
        );
    }

    #[test]
    fn test_contract_type_as_code() {
        assert_eq!(ContractType::Index.as_code(), "I");
        assert_eq!(ContractType::Stock.as_code(), "S");
        assert_eq!(ContractType::Etf.as_code(), "E");
    }

    #[test]
    fn test_futopt_session_serialization() {
        assert_eq!(
            serde_json::to_string(&FutOptSession::Regular).unwrap(),
            "\"regular\""
        );
        assert_eq!(
            serde_json::to_string(&FutOptSession::AfterHours).unwrap(),
            "\"afterhours\""
        );
    }

    #[test]
    fn test_futopt_session_deserialization() {
        assert_eq!(
            serde_json::from_str::<FutOptSession>("\"regular\"").unwrap(),
            FutOptSession::Regular
        );
        assert_eq!(
            serde_json::from_str::<FutOptSession>("\"afterhours\"").unwrap(),
            FutOptSession::AfterHours
        );
    }

    #[test]
    fn test_futopt_session_default() {
        assert_eq!(FutOptSession::default(), FutOptSession::Regular);
    }

    #[test]
    fn test_futopt_session_is_after_hours() {
        assert!(!FutOptSession::Regular.is_after_hours());
        assert!(FutOptSession::AfterHours.is_after_hours());
    }

    #[test]
    fn test_option_right_serialization() {
        assert_eq!(serde_json::to_string(&OptionRight::Call).unwrap(), "\"call\"");
        assert_eq!(serde_json::to_string(&OptionRight::Put).unwrap(), "\"put\"");
    }

    #[test]
    fn test_option_right_deserialization() {
        assert_eq!(
            serde_json::from_str::<OptionRight>("\"call\"").unwrap(),
            OptionRight::Call
        );
        assert_eq!(
            serde_json::from_str::<OptionRight>("\"put\"").unwrap(),
            OptionRight::Put
        );
    }
}
