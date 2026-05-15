//! WebSocket connection configuration types

use crate::models::AuthRequest;
use crate::tls::TlsConfig;
use std::time::Duration;

/// Configuration for WebSocket connection
#[derive(Debug, Clone)]
pub struct ConnectionConfig {
    /// WebSocket endpoint URL
    pub url: String,

    /// Authentication credentials
    pub auth: AuthRequest,

    /// Connection timeout (default: 30 seconds)
    pub connect_timeout: Duration,

    /// Read timeout for messages (default: 30 seconds)
    pub read_timeout: Duration,

    /// Optional TLS customization (custom CA / accept invalid certs).
    /// Default means "use the OS trust store" — identical to pre-3.0.1
    /// behaviour.
    pub tls: TlsConfig,
}

impl ConnectionConfig {
    /// Create a new connection configuration
    pub fn new(url: impl Into<String>, auth: AuthRequest) -> Self {
        Self {
            url: url.into(),
            auth,
            connect_timeout: Duration::from_secs(30),
            read_timeout: Duration::from_secs(30),
            tls: TlsConfig::default(),
        }
    }

    /// Create a builder for fluent configuration
    pub fn builder(url: impl Into<String>, auth: AuthRequest) -> ConnectionConfigBuilder {
        ConnectionConfigBuilder {
            url: url.into(),
            auth,
            connect_timeout: Duration::from_secs(30),
            read_timeout: Duration::from_secs(30),
            tls: TlsConfig::default(),
        }
    }

    /// Create configuration for Fugle stock WebSocket endpoint
    ///
    /// # Example
    ///
    /// ```rust
    /// use marketdata_core::websocket::ConnectionConfig;
    /// use marketdata_core::AuthRequest;
    ///
    /// let config = ConnectionConfig::fugle_stock(
    ///     AuthRequest::with_api_key("my-api-key")
    /// );
    /// assert_eq!(config.url, "wss://api.fugle.tw/marketdata/v1.0/stock/streaming");
    /// ```
    pub fn fugle_stock(auth: AuthRequest) -> Self {
        Self::new("wss://api.fugle.tw/marketdata/v1.0/stock/streaming", auth)
    }

    /// Create configuration for Fugle futures/options WebSocket endpoint
    ///
    /// # Example
    ///
    /// ```rust
    /// use marketdata_core::websocket::ConnectionConfig;
    /// use marketdata_core::AuthRequest;
    ///
    /// let config = ConnectionConfig::fugle_futopt(
    ///     AuthRequest::with_api_key("my-api-key")
    /// );
    /// assert_eq!(config.url, "wss://api.fugle.tw/marketdata/v1.0/futopt/streaming");
    /// ```
    pub fn fugle_futopt(auth: AuthRequest) -> Self {
        Self::new("wss://api.fugle.tw/marketdata/v1.0/futopt/streaming", auth)
    }
}

/// Builder for ConnectionConfig with fluent API
pub struct ConnectionConfigBuilder {
    url: String,
    auth: AuthRequest,
    connect_timeout: Duration,
    read_timeout: Duration,
    tls: TlsConfig,
}

impl ConnectionConfigBuilder {
    /// Set connection timeout
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    /// Set read timeout
    pub fn read_timeout(mut self, timeout: Duration) -> Self {
        self.read_timeout = timeout;
        self
    }

    /// Replace the TLS config wholesale
    pub fn tls(mut self, tls: TlsConfig) -> Self {
        self.tls = tls;
        self
    }

    /// Build the configuration
    pub fn build(self) -> ConnectionConfig {
        ConnectionConfig {
            url: self.url,
            auth: self.auth,
            connect_timeout: self.connect_timeout,
            read_timeout: self.read_timeout,
            tls: self.tls,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_config_new() {
        let auth = AuthRequest::with_api_key("test-key");
        let config = ConnectionConfig::new("wss://example.com", auth);

        assert_eq!(config.url, "wss://example.com");
        assert_eq!(config.connect_timeout, Duration::from_secs(30));
        assert_eq!(config.read_timeout, Duration::from_secs(30));
    }

    #[test]
    fn test_connection_config_builder() {
        let auth = AuthRequest::with_api_key("test-key");
        let config = ConnectionConfig::builder("wss://example.com", auth)
            .connect_timeout(Duration::from_secs(10))
            .read_timeout(Duration::from_secs(20))
            .build();

        assert_eq!(config.url, "wss://example.com");
        assert_eq!(config.connect_timeout, Duration::from_secs(10));
        assert_eq!(config.read_timeout, Duration::from_secs(20));
    }

    #[test]
    fn test_fugle_stock_config() {
        let auth = AuthRequest::with_api_key("test-key");
        let config = ConnectionConfig::fugle_stock(auth);

        assert_eq!(config.url, "wss://api.fugle.tw/marketdata/v1.0/stock/streaming");
    }

    #[test]
    fn test_fugle_futopt_config() {
        let auth = AuthRequest::with_api_key("test-key");
        let config = ConnectionConfig::fugle_futopt(auth);

        assert_eq!(config.url, "wss://api.fugle.tw/marketdata/v1.0/futopt/streaming");
    }
}
