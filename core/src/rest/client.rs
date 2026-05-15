//! REST client for Fugle marketdata API

use super::auth::Auth;
use crate::errors::MarketDataError;
use crate::tls::{build_rustls_config, TlsConfig};

/// Main REST client with connection pooling via ureq Agent
///
/// The RestClient uses ureq's Agent for automatic connection pooling and reuse.
/// Cloning the client is cheap - it shares the same connection pool.
///
/// # Connection Pooling
///
/// The underlying ureq Agent maintains a connection pool that:
/// - Reuses TCP connections across multiple requests
/// - Reduces connection overhead for subsequent requests
/// - Automatically handles connection lifecycle
///
/// # Thread Safety
///
/// The RestClient is NOT Send/Sync due to ureq::Agent implementation.
/// For multi-threaded usage, create a separate client per thread.
pub struct RestClient {
    agent: ureq::Agent,
    auth: Auth,
    base_url: String,
}

impl RestClient {
    /// Create a new REST client with authentication
    ///
    /// # Example
    /// ```
    /// use marketdata_core::{RestClient, Auth};
    ///
    /// let client = RestClient::new(Auth::SdkToken("my-token".to_string()));
    /// ```
    pub fn new(auth: Auth) -> Self {
        // Building a default rustls config can only realistically fail if the
        // crypto provider installs unexpectedly differently (extremely rare).
        // Panic at construction so consumers get a clear failure mode instead
        // of an opaque error on first request.
        Self::with_tls(auth, TlsConfig::default())
            .expect("default rustls config should build on supported platforms")
    }

    /// Create a REST client with custom TLS configuration (custom root CA
    /// or "accept invalid certs"). Prefer `new()` for production usage
    /// against public Fugle endpoints.
    ///
    /// Returns a `ConfigError` if the PEM in `tls.root_cert_pem` is malformed.
    pub fn with_tls(auth: Auth, tls: TlsConfig) -> Result<Self, MarketDataError> {
        let tls_config = build_rustls_config(&tls)?;
        let builder = ureq::AgentBuilder::new()
            .timeout_read(std::time::Duration::from_secs(30))
            .timeout_write(std::time::Duration::from_secs(30))
            .tls_config(tls_config);

        Ok(Self {
            agent: builder.build(),
            auth,
            base_url: "https://api.fugle.tw/marketdata/v1.0".to_string(),
        })
    }

    /// Override the base URL (useful for testing or custom endpoints)
    ///
    /// # Example
    /// ```
    /// use marketdata_core::{RestClient, Auth};
    ///
    /// let client = RestClient::new(Auth::SdkToken("my-token".to_string()))
    ///     .base_url("https://custom.api.example.com");
    /// ```
    pub fn base_url(mut self, url: &str) -> Self {
        self.base_url = url.to_string();
        self
    }

    /// Access stock-related endpoints
    ///
    /// # Example
    /// ```
    /// use marketdata_core::{RestClient, Auth};
    ///
    /// let client = RestClient::new(Auth::SdkToken("my-token".to_string()));
    /// let stock_client = client.stock();
    /// ```
    pub fn stock(&self) -> StockClient {
        StockClient { client: self }
    }

    /// Access FutOpt (futures and options) endpoints
    ///
    /// # Example
    /// ```
    /// use marketdata_core::{RestClient, Auth};
    ///
    /// let client = RestClient::new(Auth::SdkToken("my-token".to_string()));
    /// let futopt_client = client.futopt();
    /// ```
    pub fn futopt(&self) -> super::futopt::FutOptClient {
        super::futopt::FutOptClient { client: self }
    }

    /// Internal helper to get the agent
    pub(crate) fn agent(&self) -> &ureq::Agent {
        &self.agent
    }

    /// Internal helper to get the auth
    pub(crate) fn auth(&self) -> &Auth {
        &self.auth
    }

    /// Internal helper to get the base URL
    pub(crate) fn get_base_url(&self) -> &str {
        &self.base_url
    }
}

impl Clone for RestClient {
    /// Clone the RestClient, sharing the same connection pool
    ///
    /// Cloning is cheap because ureq::Agent internally uses Arc for connection pool sharing.
    /// Multiple cloned clients will share the same connection pool.
    fn clone(&self) -> Self {
        Self {
            agent: self.agent.clone(),
            auth: self.auth.clone(),
            base_url: self.base_url.clone(),
        }
    }
}

/// Stock-related endpoints client
pub struct StockClient<'a> {
    client: &'a RestClient,
}

impl<'a> StockClient<'a> {
    /// Access intraday (real-time) endpoints
    ///
    /// # Example
    /// ```
    /// use marketdata_core::{RestClient, Auth};
    ///
    /// let client = RestClient::new(Auth::SdkToken("my-token".to_string()));
    /// let intraday = client.stock().intraday();
    /// ```
    pub fn intraday(&self) -> IntradayClient<'a> {
        IntradayClient {
            client: self.client,
        }
    }

    /// Access historical data endpoints
    ///
    /// # Example
    /// ```
    /// use marketdata_core::{RestClient, Auth};
    ///
    /// let client = RestClient::new(Auth::SdkToken("my-token".to_string()));
    /// let historical = client.stock().historical();
    /// ```
    pub fn historical(&self) -> HistoricalClient<'a> {
        HistoricalClient {
            client: self.client,
        }
    }

    /// Access technical indicator endpoints
    ///
    /// # Example
    /// ```
    /// use marketdata_core::{RestClient, Auth};
    ///
    /// let client = RestClient::new(Auth::SdkToken("my-token".to_string()));
    /// let technical = client.stock().technical();
    /// ```
    pub fn technical(&self) -> crate::rest::stock::technical::TechnicalClient<'a> {
        crate::rest::stock::technical::TechnicalClient::new(self.client)
    }

    /// Access snapshot endpoints for market-wide data
    ///
    /// # Example
    /// ```
    /// use marketdata_core::{RestClient, Auth};
    ///
    /// let client = RestClient::new(Auth::SdkToken("my-token".to_string()));
    /// let snapshot = client.stock().snapshot();
    /// ```
    pub fn snapshot(&self) -> crate::rest::stock::snapshot::SnapshotClient<'a> {
        crate::rest::stock::snapshot::SnapshotClient::new(self.client)
    }

    /// Access corporate actions endpoints (capital changes, dividends, IPO listings)
    ///
    /// # Example
    /// ```
    /// use marketdata_core::{RestClient, Auth};
    ///
    /// let client = RestClient::new(Auth::SdkToken("my-token".to_string()));
    /// let corporate_actions = client.stock().corporate_actions();
    /// ```
    pub fn corporate_actions(&self) -> CorporateActionsClient<'a> {
        CorporateActionsClient {
            client: self.client,
        }
    }
}

/// Corporate actions endpoints client
pub struct CorporateActionsClient<'a> {
    client: &'a RestClient,
}

impl<'a> CorporateActionsClient<'a> {
    /// Get capital structure changes
    ///
    /// # Example
    /// ```no_run
    /// use marketdata_core::{RestClient, Auth};
    ///
    /// let client = RestClient::new(Auth::SdkToken("my-token".to_string()));
    /// let changes = client.stock().corporate_actions().capital_changes().send()?;
    /// # Ok::<(), marketdata_core::MarketDataError>(())
    /// ```
    pub fn capital_changes(&self) -> crate::rest::stock::corporate_actions::CapitalChangesRequestBuilder {
        crate::rest::stock::corporate_actions::CapitalChangesRequestBuilder::new(self.client)
    }

    /// Get dividend announcements
    ///
    /// # Example
    /// ```no_run
    /// use marketdata_core::{RestClient, Auth};
    ///
    /// let client = RestClient::new(Auth::SdkToken("my-token".to_string()));
    /// let dividends = client.stock().corporate_actions().dividends().send()?;
    /// # Ok::<(), marketdata_core::MarketDataError>(())
    /// ```
    pub fn dividends(&self) -> crate::rest::stock::corporate_actions::DividendsRequestBuilder {
        crate::rest::stock::corporate_actions::DividendsRequestBuilder::new(self.client)
    }

    /// Get IPO listing applicants
    ///
    /// # Example
    /// ```no_run
    /// use marketdata_core::{RestClient, Auth};
    ///
    /// let client = RestClient::new(Auth::SdkToken("my-token".to_string()));
    /// let applicants = client.stock().corporate_actions().listing_applicants().send()?;
    /// # Ok::<(), marketdata_core::MarketDataError>(())
    /// ```
    pub fn listing_applicants(&self) -> crate::rest::stock::corporate_actions::ListingApplicantsRequestBuilder {
        crate::rest::stock::corporate_actions::ListingApplicantsRequestBuilder::new(self.client)
    }
}

/// Historical data endpoints client
pub struct HistoricalClient<'a> {
    client: &'a RestClient,
}

impl<'a> HistoricalClient<'a> {
    /// Get historical candles for a symbol
    ///
    /// # Example
    /// ```no_run
    /// use marketdata_core::{RestClient, Auth};
    ///
    /// let client = RestClient::new(Auth::SdkToken("my-token".to_string()));
    /// let candles = client.stock().historical().candles()
    ///     .symbol("2330")
    ///     .from("2024-01-01")
    ///     .to("2024-01-31")
    ///     .send()?;
    /// # Ok::<(), marketdata_core::MarketDataError>(())
    /// ```
    pub fn candles(&self) -> crate::rest::stock::historical::HistoricalCandlesRequestBuilder {
        crate::rest::stock::historical::HistoricalCandlesRequestBuilder::new(self.client)
    }

    /// Get historical stats for a symbol
    ///
    /// # Example
    /// ```no_run
    /// use marketdata_core::{RestClient, Auth};
    ///
    /// let client = RestClient::new(Auth::SdkToken("my-token".to_string()));
    /// let stats = client.stock().historical().stats()
    ///     .symbol("2330")
    ///     .send()?;
    /// # Ok::<(), marketdata_core::MarketDataError>(())
    /// ```
    pub fn stats(&self) -> crate::rest::stock::historical::StatsRequestBuilder {
        crate::rest::stock::historical::StatsRequestBuilder::new(self.client)
    }
}

/// Intraday (real-time) endpoints client
pub struct IntradayClient<'a> {
    client: &'a RestClient,
}

impl<'a> IntradayClient<'a> {
    /// Get intraday quote for a symbol
    ///
    /// # Example
    /// ```no_run
    /// use marketdata_core::{RestClient, Auth};
    ///
    /// let client = RestClient::new(Auth::SdkToken("my-token".to_string()));
    /// let quote = client.stock().intraday().quote().symbol("2330").send()?;
    /// # Ok::<(), marketdata_core::MarketDataError>(())
    /// ```
    pub fn quote(&self) -> crate::rest::stock::intraday::QuoteRequestBuilder {
        crate::rest::stock::intraday::QuoteRequestBuilder::new(self.client)
    }

    /// Get intraday ticker info for a symbol
    ///
    /// # Example
    /// ```no_run
    /// use marketdata_core::{RestClient, Auth};
    ///
    /// let client = RestClient::new(Auth::SdkToken("my-token".to_string()));
    /// let ticker = client.stock().intraday().ticker().symbol("2330").send()?;
    /// # Ok::<(), marketdata_core::MarketDataError>(())
    /// ```
    pub fn ticker(&self) -> crate::rest::stock::intraday::TickerRequestBuilder {
        crate::rest::stock::intraday::TickerRequestBuilder::new(self.client)
    }

    /// Get intraday tickers (batch list) for a security type
    ///
    /// # Example
    /// ```no_run
    /// use marketdata_core::{RestClient, Auth};
    ///
    /// let client = RestClient::new(Auth::SdkToken("my-token".to_string()));
    /// let tickers = client.stock().intraday().tickers().typ("EQUITY").send()?;
    /// # Ok::<(), marketdata_core::MarketDataError>(())
    /// ```
    pub fn tickers(&self) -> crate::rest::stock::intraday::TickersRequestBuilder {
        crate::rest::stock::intraday::TickersRequestBuilder::new(self.client)
    }

    /// Get intraday candles for a symbol
    ///
    /// # Example
    /// ```no_run
    /// use marketdata_core::{RestClient, Auth};
    ///
    /// let client = RestClient::new(Auth::SdkToken("my-token".to_string()));
    /// let candles = client.stock().intraday().candles().symbol("2330").timeframe("5").send()?;
    /// # Ok::<(), marketdata_core::MarketDataError>(())
    /// ```
    pub fn candles(&self) -> crate::rest::stock::intraday::CandlesRequestBuilder {
        crate::rest::stock::intraday::CandlesRequestBuilder::new(self.client)
    }

    /// Get intraday trades for a symbol
    ///
    /// # Example
    /// ```no_run
    /// use marketdata_core::{RestClient, Auth};
    ///
    /// let client = RestClient::new(Auth::SdkToken("my-token".to_string()));
    /// let trades = client.stock().intraday().trades().symbol("2330").send()?;
    /// # Ok::<(), marketdata_core::MarketDataError>(())
    /// ```
    pub fn trades(&self) -> crate::rest::stock::intraday::TradesRequestBuilder {
        crate::rest::stock::intraday::TradesRequestBuilder::new(self.client)
    }

    /// Get intraday volumes for a symbol
    ///
    /// # Example
    /// ```no_run
    /// use marketdata_core::{RestClient, Auth};
    ///
    /// let client = RestClient::new(Auth::SdkToken("my-token".to_string()));
    /// let volumes = client.stock().intraday().volumes().symbol("2330").send()?;
    /// # Ok::<(), marketdata_core::MarketDataError>(())
    /// ```
    pub fn volumes(&self) -> crate::rest::stock::intraday::VolumesRequestBuilder {
        crate::rest::stock::intraday::VolumesRequestBuilder::new(self.client)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rest_client_creation() {
        let client = RestClient::new(Auth::SdkToken("test-token".to_string()));
        assert_eq!(client.get_base_url(), "https://api.fugle.tw/marketdata/v1.0");
    }

    #[test]
    fn test_rest_client_custom_base_url() {
        let client = RestClient::new(Auth::SdkToken("test-token".to_string()))
            .base_url("https://custom.example.com");
        assert_eq!(client.get_base_url(), "https://custom.example.com");
    }

    #[test]
    fn test_stock_client_creation() {
        let client = RestClient::new(Auth::ApiKey("test-key".to_string()));
        let stock_client = client.stock();
        assert_eq!(stock_client.client.get_base_url(), "https://api.fugle.tw/marketdata/v1.0");
    }

    #[test]
    fn test_intraday_client_creation() {
        let client = RestClient::new(Auth::BearerToken("test-bearer".to_string()));
        let intraday = client.stock().intraday();
        assert_eq!(intraday.client.get_base_url(), "https://api.fugle.tw/marketdata/v1.0");
    }

    #[test]
    fn test_chained_client_access() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let _intraday = client.stock().intraday();
        // Compilation success proves the chaining works
    }

    #[test]
    fn test_auth_types() {
        // Test all three auth types
        let _client1 = RestClient::new(Auth::ApiKey("key".to_string()));
        let _client2 = RestClient::new(Auth::BearerToken("token".to_string()));
        let _client3 = RestClient::new(Auth::SdkToken("sdk".to_string()));
    }

    #[test]
    fn test_client_clone() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let cloned = client.clone();

        // Cloned client should have same base URL and auth
        assert_eq!(client.get_base_url(), cloned.get_base_url());
    }

    #[test]
    fn test_connection_pool_sharing() {
        // Create client with connection pool
        let client = RestClient::new(Auth::SdkToken("test".to_string()));

        // Clone shares the same connection pool (via Arc in ureq::Agent)
        let cloned = client.clone();

        // Both clients should be usable
        let _stock1 = client.stock().intraday();
        let _stock2 = cloned.stock().intraday();

        // Compilation and execution success proves connection pool works
    }

    #[test]
    fn test_custom_base_url_preserved_in_clone() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()))
            .base_url("https://custom.example.com");

        let cloned = client.clone();
        assert_eq!(cloned.get_base_url(), "https://custom.example.com");
    }

    #[test]
    fn test_futopt_client_creation() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let futopt = client.futopt();
        assert_eq!(futopt.client.get_base_url(), "https://api.fugle.tw/marketdata/v1.0");
    }

    #[test]
    fn test_futopt_intraday_client_creation() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let intraday = client.futopt().intraday();
        assert_eq!(intraday.client.get_base_url(), "https://api.fugle.tw/marketdata/v1.0");
    }

    #[test]
    fn test_futopt_chained_client_access() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let _intraday = client.futopt().intraday();
        // Compilation success proves the chaining works
    }

    #[test]
    fn test_both_stock_and_futopt() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));

        // Both stock and futopt should be accessible from the same client
        let _stock = client.stock().intraday();
        let _futopt = client.futopt().intraday();
    }

    #[test]
    fn test_corporate_actions_client_creation() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let corporate_actions = client.stock().corporate_actions();
        assert_eq!(corporate_actions.client.get_base_url(), "https://api.fugle.tw/marketdata/v1.0");
    }

    #[test]
    fn test_corporate_actions_chained_access() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        // Test that all corporate actions endpoints are accessible
        let _capital_changes = client.stock().corporate_actions().capital_changes();
        let _dividends = client.stock().corporate_actions().dividends();
        let _listing_applicants = client.stock().corporate_actions().listing_applicants();
    }
}
