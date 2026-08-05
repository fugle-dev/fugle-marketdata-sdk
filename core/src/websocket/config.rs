//! WebSocket connection configuration types

use crate::models::AuthRequest;
use crate::tls::TlsConfig;
use std::fmt;
use std::time::Duration;

/// Configuration for WebSocket connection.
///
/// `Debug` is implemented manually so embedded credentials cannot leak.
/// The `auth` field is rendered as `<redacted>` and any query-string
/// parameter on `url` whose name case-insensitively matches a known
/// secret key (`token`, `key`, `apikey`, `api_key`, `secret`, `password`)
/// has its value replaced with `***`.
#[derive(Clone)]
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

    /// Capacity of the inbound message channel that backs `messages()` and
    /// `message_stream()`. Defaults to [`DEFAULT_MESSAGE_BUFFER`]. Use
    /// [`ConnectionConfigBuilder::message_buffer`] to override.
    pub message_buffer: usize,

    /// Capacity of the lifecycle event channel that backs `events()` and
    /// `state_events()`. Defaults to [`DEFAULT_EVENT_BUFFER`]. Use
    /// [`ConnectionConfigBuilder::event_buffer`] to override.
    pub event_buffer: usize,

    /// Optional caller-supplied identifier used as a metric label on the
    /// `metrics` feature. Defaults to `None`. **Low-cardinality only** —
    /// suitable for deployment / instance / service identifiers, never for
    /// per-request UUIDs (Prometheus storage explodes on high-cardinality
    /// labels). Values longer than [`CLIENT_ID_MAX_LEN`] are truncated by
    /// the builder.
    pub client_id: Option<String>,
}

/// Default capacity for the inbound message channel (`message_buffer`).
///
/// Sized for multi-symbol consumers (50–200 symbols across all channels at
/// the TWSE 9:00 open burst, ~2000 msg/s). Bounded mpsc channels in tokio
/// and std do not pre-allocate, so a higher cap costs nothing at idle and
/// only manifests memory cost on saturation. At 4096 this provides ~2 s
/// of headroom at 2000 msg/s before drop-newest backpressure kicks in.
///
/// Pre-0.4.0 the cap was hardcoded to 1024. Increased to 4096 in 0.4.0.
pub const DEFAULT_MESSAGE_BUFFER: usize = 4096;

/// Default capacity for the lifecycle event channel (`event_buffer`).
///
/// Lifecycle events fire on the order of one per heartbeat period (30 s)
/// plus reconnect/error bursts, so 1024 is generous; kept at 1024 for
/// "no event left behind" safety. May be reduced in a future release once
/// production telemetry confirms it is consistently underused.
pub const DEFAULT_EVENT_BUFFER: usize = 1024;

/// Maximum byte length accepted for [`ConnectionConfig::client_id`].
///
/// Values supplied to [`ConnectionConfigBuilder::client_id`] longer than this
/// are truncated to `CLIENT_ID_MAX_LEN` bytes; truncation emits a
/// `tracing::warn!` (no-op without the `tracing` feature). The cap exists
/// to bound metric-label cardinality and Prometheus storage cost.
pub const CLIENT_ID_MAX_LEN: usize = 64;

/// Names whose value should be redacted when seen as a URL query parameter.
/// Matched case-insensitively.
const SENSITIVE_QUERY_KEYS: &[&str] =
    &["token", "key", "apikey", "api_key", "secret", "password"];

/// Return a copy of `url` with sensitive query-parameter values masked as
/// `***`. Falls back to the original string if `url` is not parseable.
fn redact_url_query(url: &str) -> String {
    let mut parsed = match url::Url::parse(url) {
        Ok(u) => u,
        Err(_) => return url.to_string(),
    };

    let original_pairs: Vec<(String, String)> = parsed
        .query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();

    if original_pairs.is_empty() {
        return parsed.to_string();
    }

    let mut serializer = parsed.query_pairs_mut();
    serializer.clear();
    for (k, v) in &original_pairs {
        let redacted = SENSITIVE_QUERY_KEYS
            .iter()
            .any(|s| k.eq_ignore_ascii_case(s));
        if redacted {
            serializer.append_pair(k, "***");
        } else {
            serializer.append_pair(k, v);
        }
    }
    drop(serializer);

    parsed.to_string()
}

impl fmt::Debug for ConnectionConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ConnectionConfig")
            .field("url", &redact_url_query(&self.url))
            .field("auth", &"<redacted>")
            .field("connect_timeout", &self.connect_timeout)
            .field("read_timeout", &self.read_timeout)
            .field("tls", &self.tls)
            .field("message_buffer", &self.message_buffer)
            .field("event_buffer", &self.event_buffer)
            .field("client_id", &self.client_id)
            .finish()
    }
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
            message_buffer: DEFAULT_MESSAGE_BUFFER,
            event_buffer: DEFAULT_EVENT_BUFFER,
            client_id: None,
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
            message_buffer: DEFAULT_MESSAGE_BUFFER,
            event_buffer: DEFAULT_EVENT_BUFFER,
            client_id: None,
        }
    }

    /// Caller-supplied identifier used as a metric label, or `None` if
    /// unset. See [`ConnectionConfigBuilder::client_id`].
    #[must_use]
    pub fn client_id(&self) -> Option<&str> {
        self.client_id.as_deref()
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
        Self::new(crate::urls::STOCK_WS, auth)
    }

    /// Create configuration for Fugle futures/options WebSocket endpoint,
    /// at the default futopt streaming version
    /// ([`FutOptVersion::V1_1`](crate::websocket::FutOptVersion::V1_1)).
    ///
    /// # ⚠️ Behaviour change in 0.8.0
    ///
    /// This moved from `v1.0` to `v1.1`, so trial-matching (試撮) frames now
    /// arrive on `trades` / `books`. Branch on the frame's `is_trial` before
    /// acting on a price. To stay on `v1.0`, build through
    /// [`WebSocketFactory`](crate::WebSocketFactory) with
    /// `.futopt_version(FutOptVersion::V1_0)`.
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
    /// assert_eq!(config.url, "wss://api.fugle.tw/marketdata/v1.1/futopt/streaming");
    /// ```
    pub fn fugle_futopt(auth: AuthRequest) -> Self {
        Self::new(crate::urls::FUTOPT_WS, auth)
    }
}

/// Builder for ConnectionConfig with fluent API
pub struct ConnectionConfigBuilder {
    url: String,
    auth: AuthRequest,
    connect_timeout: Duration,
    read_timeout: Duration,
    tls: TlsConfig,
    message_buffer: usize,
    event_buffer: usize,
    client_id: Option<String>,
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

    /// Override the inbound message-channel capacity.
    ///
    /// Defaults to [`DEFAULT_MESSAGE_BUFFER`] (4096). The channel uses
    /// drop-newest backpressure on saturation; tune this when the consumer
    /// can experience long pauses (e.g. trade peaks while a UI thread is
    /// blocked) or when subscribing to many high-volume symbols at once.
    ///
    /// # Panics
    ///
    /// Panics if `cap` is zero — a zero-capacity channel cannot make
    /// progress and is always a configuration mistake.
    pub fn message_buffer(mut self, cap: usize) -> Self {
        assert!(cap > 0, "message_buffer must be greater than zero");
        self.message_buffer = cap;
        self
    }

    /// Override the lifecycle event-channel capacity.
    ///
    /// Defaults to [`DEFAULT_EVENT_BUFFER`] (1024). Event volume is
    /// orders of magnitude lower than message volume; tune only if you
    /// retain raw events for an extended period without consuming them.
    ///
    /// # Panics
    ///
    /// Panics if `cap` is zero.
    pub fn event_buffer(mut self, cap: usize) -> Self {
        assert!(cap > 0, "event_buffer must be greater than zero");
        self.event_buffer = cap;
        self
    }

    /// Set a low-cardinality identifier used as a `client_id` metric label
    /// when the `metrics` feature is enabled.
    ///
    /// **Cardinality**: pass a deployment, instance, or service identifier
    /// (e.g. `"monitor-stock-probe"`, `"trader-prod-3"`). Per-request UUIDs
    /// or any value derived from request data will explode Prometheus
    /// storage and break the metrics pipeline.
    ///
    /// **Length**: values longer than [`CLIENT_ID_MAX_LEN`] are truncated
    /// to that many bytes; truncation emits `tracing::warn!` so the
    /// overflow is observable in operational logs.
    pub fn client_id(mut self, id: impl Into<String>) -> Self {
        let mut id = id.into();
        if id.len() > CLIENT_ID_MAX_LEN {
            // Truncate at a UTF-8 char boundary at or below CLIENT_ID_MAX_LEN
            // so the resulting string is always valid UTF-8. The tracing
            // warning fires on the original length so the operator sees the
            // intended overflow size.
            let _original_len = id.len();
            let mut cut = CLIENT_ID_MAX_LEN;
            while cut > 0 && !id.is_char_boundary(cut) {
                cut -= 1;
            }
            id.truncate(cut);
            crate::tracing_compat::warn!(
                target: "fugle_marketdata::config",
                original_len = _original_len,
                truncated_len = cut,
                max_len = CLIENT_ID_MAX_LEN,
                "client_id exceeded CLIENT_ID_MAX_LEN; truncated"
            );
        }
        self.client_id = Some(id);
        self
    }

    /// Convenience for `Option<impl Into<String>>` callers (e.g. forwarding
    /// from a binding layer that may have an `Option<String>` in hand).
    /// Equivalent to `client_id(...)` when `Some`; no-op when `None`.
    pub fn maybe_client_id(self, id: Option<impl Into<String>>) -> Self {
        match id {
            Some(id) => self.client_id(id),
            None => self,
        }
    }

    /// Build the configuration
    pub fn build(self) -> ConnectionConfig {
        ConnectionConfig {
            url: self.url,
            auth: self.auth,
            connect_timeout: self.connect_timeout,
            read_timeout: self.read_timeout,
            tls: self.tls,
            message_buffer: self.message_buffer,
            event_buffer: self.event_buffer,
            client_id: self.client_id,
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

        // 0.8.0: the futopt default moved to v1.1, so this convenience
        // constructor and `WebSocketFactory::futopt()` stay in agreement.
        assert_eq!(config.url, "wss://api.fugle.tw/marketdata/v1.1/futopt/streaming");
    }

    #[test]
    fn test_debug_redacts_auth() {
        let auth = AuthRequest::with_api_key("super-secret-api-key");
        let config = ConnectionConfig::fugle_stock(auth);
        let rendered = format!("{:?}", config);

        assert!(rendered.contains("auth: \"<redacted>\""));
        assert!(!rendered.contains("super-secret-api-key"));
    }

    #[test]
    fn test_debug_redacts_url_query_token() {
        let auth = AuthRequest::with_api_key("k");
        let config =
            ConnectionConfig::new("wss://example.com/stream?token=secret&v=1", auth);
        let rendered = format!("{:?}", config);

        assert!(rendered.contains("token=***"));
        assert!(rendered.contains("v=1"));
        assert!(!rendered.contains("token=secret"));
    }

    #[test]
    fn test_debug_redacts_multiple_sensitive_keys() {
        let auth = AuthRequest::with_api_key("k");
        let config = ConnectionConfig::new(
            "wss://example.com/stream?api_key=AAA&secret=BBB&safe=CCC",
            auth,
        );
        let rendered = format!("{:?}", config);

        assert!(rendered.contains("api_key=***"));
        assert!(rendered.contains("secret=***"));
        assert!(rendered.contains("safe=CCC"));
        assert!(!rendered.contains("AAA"));
        assert!(!rendered.contains("BBB"));
    }

    #[test]
    fn test_debug_handles_unparseable_url() {
        let auth = AuthRequest::with_api_key("k");
        let config = ConnectionConfig::new("not a real url", auth);
        let rendered = format!("{:?}", config);
        assert!(rendered.contains("not a real url"));
    }
}
