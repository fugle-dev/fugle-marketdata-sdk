//! FutOpt product data model - matches Fugle futopt/intraday/products response

use serde::{Deserialize, Serialize};

/// Tradeable product from FutOpt products endpoint (futopt/intraday/products)
///
/// This matches the data array items in RestFutOptIntradayProductsResponse.
/// Used to discover available futures and options contracts.
///
/// # Example
///
/// ```rust
/// use marketdata_core::models::futopt::Product;
///
/// let json = r#"{
///     "type": "FUTURE",
///     "exchange": "TAIFEX",
///     "symbol": "TX",
///     "name": "臺股期貨",
///     "underlyingSymbol": "FITX",
///     "contractType": "I",
///     "contractSize": 200,
///     "statusCode": "N",
///     "tradingCurrency": "TWD",
///     "quoteAcceptable": true,
///     "startDate": "1998-07-21",
///     "canBlockTrade": true,
///     "expiryType": "S",
///     "underlyingType": "I",
///     "marketCloseGroup": 1,
///     "endSession": 2
/// }"#;
///
/// let product: Product = serde_json::from_str(json).unwrap();
/// assert_eq!(product.symbol, "TX");
/// assert_eq!(product.contract_type.as_deref(), Some("I"));
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Product {
    // === Product identification ===
    /// Product type (FUTURE or OPTION)
    #[serde(rename = "type")]
    pub product_type: Option<String>,

    /// Exchange code (TAIFEX)
    pub exchange: Option<String>,

    /// Product symbol (e.g., "TX", "TXO")
    pub symbol: String,

    /// Product name
    pub name: Option<String>,

    // === Contract details ===
    /// Underlying symbol (e.g., "FITX" for TAIEX index)
    #[serde(rename = "underlyingSymbol")]
    pub underlying_symbol: Option<String>,

    /// Contract type code (I=Index, R=Rate, B=Bond, C=Currency, S=Stock, E=ETF)
    #[serde(rename = "contractType")]
    pub contract_type: Option<String>,

    /// Contract size (multiplier)
    #[serde(rename = "contractSize")]
    pub contract_size: Option<i64>,

    /// Underlying type (I=Index, S=Stock, etc.)
    #[serde(rename = "underlyingType")]
    pub underlying_type: Option<String>,

    // === Trading rules ===
    /// Status code (N=Normal, P=Pre-market, U=Unknown)
    #[serde(rename = "statusCode")]
    pub status_code: Option<String>,

    /// Trading currency (e.g., "TWD", "USD")
    #[serde(rename = "tradingCurrency")]
    pub trading_currency: Option<String>,

    /// Whether quote orders are acceptable
    #[serde(rename = "quoteAcceptable", default)]
    pub quote_acceptable: bool,

    /// Whether block trading is allowed
    #[serde(rename = "canBlockTrade", default)]
    pub can_block_trade: bool,

    // === Dates ===
    /// Product start date (YYYY-MM-DD) - when the product began trading
    #[serde(rename = "startDate")]
    pub start_date: Option<String>,

    // === Session info ===
    /// Expiry type (S=Standard, W=Weekly, etc.)
    #[serde(rename = "expiryType")]
    pub expiry_type: Option<String>,

    /// Market close group
    #[serde(rename = "marketCloseGroup")]
    pub market_close_group: Option<i32>,

    /// End session number
    #[serde(rename = "endSession")]
    pub end_session: Option<i32>,
}

impl Product {
    /// Check if this is a futures product
    pub fn is_future(&self) -> bool {
        self.product_type
            .as_ref()
            .is_some_and(|t| t.to_uppercase() == "FUTURE")
    }

    /// Check if this is an options product
    pub fn is_option(&self) -> bool {
        self.product_type
            .as_ref()
            .is_some_and(|t| t.to_uppercase() == "OPTION")
    }

    /// Check if this is an index-based product
    pub fn is_index_product(&self) -> bool {
        self.contract_type.as_deref() == Some("I")
    }

    /// Check if this is a stock-based product
    pub fn is_stock_product(&self) -> bool {
        self.contract_type.as_deref() == Some("S")
    }

    /// Check if this is an ETF-based product
    pub fn is_etf_product(&self) -> bool {
        self.contract_type.as_deref() == Some("E")
    }
}

/// Response wrapper for products endpoint (futopt/intraday/products)
///
/// Contains metadata about the query and the list of matching products.
///
/// # Example
///
/// ```rust
/// use marketdata_core::models::futopt::ProductsResponse;
///
/// let json = r#"{
///     "date": "2024-01-15",
///     "type": "FUTURE",
///     "session": "REGULAR",
///     "contractType": "I",
///     "status": "N",
///     "data": [
///         {"symbol": "TX", "name": "臺股期貨", "contractType": "I"},
///         {"symbol": "TE", "name": "電子期貨", "contractType": "I"}
///     ]
/// }"#;
///
/// let response: ProductsResponse = serde_json::from_str(json).unwrap();
/// assert_eq!(response.data.len(), 2);
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ProductsResponse {
    /// Response date (YYYY-MM-DD)
    pub date: Option<String>,

    /// Queried product type (FUTURE or OPTION)
    #[serde(rename = "type")]
    pub product_type: Option<String>,

    /// Queried session (REGULAR or AFTERHOURS)
    pub session: Option<String>,

    /// Queried contract type (I, R, B, C, S, E)
    #[serde(rename = "contractType")]
    pub contract_type: Option<String>,

    /// Queried status filter
    pub status: Option<String>,

    /// List of matching products
    #[serde(default)]
    pub data: Vec<Product>,
}

impl ProductsResponse {
    /// Get the number of products in the response
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if the response contains no products
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Filter products by symbol prefix
    pub fn filter_by_prefix(&self, prefix: &str) -> Vec<&Product> {
        self.data
            .iter()
            .filter(|p| p.symbol.starts_with(prefix))
            .collect()
    }

    /// Get only futures products
    pub fn futures(&self) -> Vec<&Product> {
        self.data.iter().filter(|p| p.is_future()).collect()
    }

    /// Get only options products
    pub fn options(&self) -> Vec<&Product> {
        self.data.iter().filter(|p| p.is_option()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_product_deserialization() {
        let json = r#"{
            "type": "FUTURE",
            "exchange": "TAIFEX",
            "symbol": "TX",
            "name": "臺股期貨",
            "underlyingSymbol": "FITX",
            "contractType": "I",
            "contractSize": 200,
            "statusCode": "N",
            "tradingCurrency": "TWD",
            "quoteAcceptable": true,
            "startDate": "1998-07-21",
            "canBlockTrade": true,
            "expiryType": "S",
            "underlyingType": "I",
            "marketCloseGroup": 1,
            "endSession": 2
        }"#;

        let product: Product = serde_json::from_str(json).unwrap();
        assert_eq!(product.symbol, "TX");
        assert_eq!(product.name.as_deref(), Some("臺股期貨"));
        assert_eq!(product.product_type.as_deref(), Some("FUTURE"));
        assert_eq!(product.exchange.as_deref(), Some("TAIFEX"));
        assert_eq!(product.underlying_symbol.as_deref(), Some("FITX"));
        assert_eq!(product.contract_type.as_deref(), Some("I"));
        assert_eq!(product.contract_size, Some(200));
        assert_eq!(product.status_code.as_deref(), Some("N"));
        assert!(product.quote_acceptable);
        assert!(product.can_block_trade);
    }

    #[test]
    fn test_product_minimal() {
        let json = r#"{"symbol": "TX"}"#;
        let product: Product = serde_json::from_str(json).unwrap();
        assert_eq!(product.symbol, "TX");
        assert!(product.name.is_none());
    }

    #[test]
    fn test_product_is_future() {
        let mut product = Product::default();
        assert!(!product.is_future());

        product.product_type = Some("FUTURE".to_string());
        assert!(product.is_future());

        product.product_type = Some("Future".to_string());
        assert!(product.is_future());

        product.product_type = Some("OPTION".to_string());
        assert!(!product.is_future());
    }

    #[test]
    fn test_product_is_option() {
        let mut product = Product::default();
        assert!(!product.is_option());

        product.product_type = Some("OPTION".to_string());
        assert!(product.is_option());
    }

    #[test]
    fn test_product_contract_type_checks() {
        let mut product = Product::default();
        assert!(!product.is_index_product());
        assert!(!product.is_stock_product());
        assert!(!product.is_etf_product());

        product.contract_type = Some("I".to_string());
        assert!(product.is_index_product());
        assert!(!product.is_stock_product());

        product.contract_type = Some("S".to_string());
        assert!(!product.is_index_product());
        assert!(product.is_stock_product());

        product.contract_type = Some("E".to_string());
        assert!(product.is_etf_product());
    }

    #[test]
    fn test_products_response_deserialization() {
        let json = r#"{
            "date": "2024-01-15",
            "type": "FUTURE",
            "session": "REGULAR",
            "contractType": "I",
            "status": "N",
            "data": [
                {"symbol": "TX", "name": "臺股期貨", "type": "FUTURE", "contractType": "I"},
                {"symbol": "TE", "name": "電子期貨", "type": "FUTURE", "contractType": "I"}
            ]
        }"#;

        let response: ProductsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.date.as_deref(), Some("2024-01-15"));
        assert_eq!(response.product_type.as_deref(), Some("FUTURE"));
        assert_eq!(response.session.as_deref(), Some("REGULAR"));
        assert_eq!(response.contract_type.as_deref(), Some("I"));
        assert_eq!(response.data.len(), 2);
        assert_eq!(response.data[0].symbol, "TX");
        assert_eq!(response.data[1].symbol, "TE");
    }

    #[test]
    fn test_products_response_empty() {
        let json = r#"{"data": []}"#;
        let response: ProductsResponse = serde_json::from_str(json).unwrap();
        assert!(response.is_empty());
        assert_eq!(response.len(), 0);
    }

    #[test]
    fn test_products_response_filter_by_prefix() {
        let json = r#"{
            "data": [
                {"symbol": "TX", "name": "臺股期貨"},
                {"symbol": "TXO", "name": "臺指選擇權"},
                {"symbol": "TE", "name": "電子期貨"}
            ]
        }"#;

        let response: ProductsResponse = serde_json::from_str(json).unwrap();
        let tx_products = response.filter_by_prefix("TX");
        assert_eq!(tx_products.len(), 2);
        assert_eq!(tx_products[0].symbol, "TX");
        assert_eq!(tx_products[1].symbol, "TXO");
    }

    #[test]
    fn test_products_response_futures_options_filter() {
        let json = r#"{
            "data": [
                {"symbol": "TX", "type": "FUTURE"},
                {"symbol": "TXO", "type": "OPTION"},
                {"symbol": "TE", "type": "FUTURE"}
            ]
        }"#;

        let response: ProductsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.futures().len(), 2);
        assert_eq!(response.options().len(), 1);
    }

    #[test]
    fn test_option_product_example() {
        let json = r#"{
            "type": "OPTION",
            "exchange": "TAIFEX",
            "symbol": "TXO",
            "name": "臺指選擇權",
            "underlyingSymbol": "FITX",
            "contractType": "I",
            "contractSize": 50,
            "statusCode": "N"
        }"#;

        let product: Product = serde_json::from_str(json).unwrap();
        assert_eq!(product.symbol, "TXO");
        assert!(product.is_option());
        assert!(!product.is_future());
        assert!(product.is_index_product());
    }
}
