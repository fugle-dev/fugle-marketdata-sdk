//! WebSocket client with foreign trait callbacks for UniFFI bindings
//!
//! This module provides a WebSocket client that delivers typed StreamMessage events
//! to foreign language consumers (C#, Go) via the WebSocketListener trait.
//!
//! # Architecture
//!
//! ```text
//! Foreign Code (C#/Go)                 Rust (UniFFI)
//! ┌─────────────────────┐              ┌─────────────────────┐
//! │ class MyListener    │              │ WebSocketClient     │
//! │   implements        │──callback────│   spawns message    │
//! │   IWebSocketListener│              │   forwarding task   │
//! │                     │              │                     │
//! │ OnMessage(msg) ◄────│──────────────│ stream reader       │
//! │ OnConnected()  ◄────│              │                     │
//! │ OnDisconnected()◄───│              │ CoreWebSocketClient │
//! │ OnError(err)   ◄────│              │  .stream_receiver() │
//! └─────────────────────┘              └─────────────────────┘
//! ```
//!
//! Lifecycle callbacks (`on_connected`, `on_authenticated`,
//! `on_unauthenticated`, `on_disconnected`, `on_reconnecting`,
//! `on_reconnect_failed`, `on_error`) are forwarded one-to-one from core's
//! `ConnectionEvent`s, so they inherit core's delivery guarantees; this
//! module never synthesizes them.
//!
//! # Thread Safety
//!
//! The `WebSocketListener` trait requires `Send + Sync` for thread-safe
//! callback invocation. Foreign implementations must be thread-safe.

use crate::errors::{ErrorInfo, MarketDataError};
use crate::models::StreamMessage;
use marketdata_core::aio::WebSocketClient as CoreWebSocketClient;
use marketdata_core::websocket::{ConnectionEvent, StreamItem, StreamReceiver};
use marketdata_core::AuthRequest;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Callback interface for WebSocket events
///
/// Foreign code (C#, Go) implements this trait to receive WebSocket events.
/// The implementation must be thread-safe (Send + Sync) as callbacks may be
/// invoked from background tokio tasks.
///
/// # Example (C#)
///
/// ```csharp
/// class MyListener : IWebSocketListener {
///     public void OnConnected() {
///         Console.WriteLine("Connected!");
///     }
///     public void OnAuthenticated(string? dataJson) {
///         Console.WriteLine("Authenticated");
///     }
///     public void OnUnauthenticated(string? dataJson) {
///         Console.WriteLine($"Rejected: {dataJson}");
///     }
///     public void OnDisconnected(bool willReconnect) {
///         Console.WriteLine($"Disconnected (will reconnect: {willReconnect})");
///     }
///     public void OnMessage(StreamMessage message) {
///         Console.WriteLine($"Got {message.Event} for {message.Symbol}");
///     }
///     public void OnError(ErrorInfo error) {
///         Console.WriteLine($"Error: {error.Message}");
///     }
/// }
/// ```
#[uniffi::export(with_foreign)]
pub trait WebSocketListener: Send + Sync {
    /// Called when the transport is established, before the server has
    /// answered the auth frame. Fires again on every successful reconnect.
    /// Wait for `on_authenticated` before treating the connection as usable.
    fn on_connected(&self);

    /// Called when the server accepts the credentials.
    ///
    /// `data_json` is the `data` member of the server's `authenticated`
    /// frame, still encoded as JSON, or `None` when the frame has none.
    fn on_authenticated(&self, data_json: Option<String>);

    /// Called when the server rejects the credentials. `connect()` also
    /// fails with an auth error; no `on_error` is emitted for the rejection.
    ///
    /// `data_json` is the `data` member of the server's rejection frame
    /// (the server's message is under `message`), still encoded as JSON, or
    /// `None` when the frame has none.
    fn on_unauthenticated(&self, data_json: Option<String>);

    /// Called when the connection is closed, at most once per connection.
    ///
    /// `will_reconnect` is `true` when the client will try to reconnect
    /// (`on_reconnecting` follows unless `disconnect()` is called first) and
    /// `false` when this connection's lifecycle has ended.
    fn on_disconnected(&self, will_reconnect: bool);

    /// Called when a message is received
    fn on_message(&self, message: StreamMessage);

    /// Called when an error occurs
    fn on_error(&self, error: ErrorInfo);

    /// Called when a reconnection attempt starts
    fn on_reconnecting(&self, attempt: u32);

    /// Called when all reconnection attempts are exhausted. Terminal: no
    /// further lifecycle callbacks follow for this connection.
    fn on_reconnect_failed(&self, attempts: u32);

    /// Called when messages were dropped because `on_message` fell behind
    /// while the client's message queue held `buffer` unread messages
    /// (`MessageOverflowRecord::DropNewest`).
    ///
    /// `count` is the number dropped since the previous call. The first drop
    /// on a connection is reported at once, later ones at most once per
    /// second, and the rest before `on_disconnected`. The connection's total
    /// is `WebSocketClient::messages_dropped_total()`.
    fn on_messages_dropped(&self, count: u64);
}

/// The credentials a WebSocket client authenticates with.
///
/// Exactly one must be non-empty; an empty or whitespace-only value counts
/// as not provided.
///
/// Its fields are secrets: do not log this record. `Debug` here redacts
/// them, but the generated types may not — a C# record's `ToString()` and
/// Go's `fmt` `%v` print every field.
#[derive(Clone, uniffi::Record)]
pub struct CredentialsRecord {
    /// Fugle API key, sent as `apikey`
    pub api_key: Option<String>,
    /// OAuth bearer token, sent as `token`
    pub bearer_token: Option<String>,
    /// Fugle SDK token, sent as `sdkToken`
    pub sdk_token: Option<String>,
}

impl std::fmt::Debug for CredentialsRecord {
    /// Prints `Some(***)` for a set credential, like `AuthRequest`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        /// Prints `***` in place of a secret.
        struct Redacted;
        impl std::fmt::Debug for Redacted {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("***")
            }
        }
        let redact = |value: &Option<String>| value.as_ref().map(|_| Redacted);

        f.debug_struct("CredentialsRecord")
            .field("api_key", &redact(&self.api_key))
            .field("bearer_token", &redact(&self.bearer_token))
            .field("sdk_token", &redact(&self.sdk_token))
            .finish()
    }
}

/// Reconnection configuration record for FFI
///
/// All fields are optional — zero/false values mean "use default".
#[derive(Debug, Clone, uniffi::Record)]
pub struct ReconnectConfigRecord {
    /// Maximum reconnection attempts (default: 5, min: 1)
    pub max_attempts: u32,
    /// Initial reconnection delay in milliseconds (default: 1000, min: 100)
    pub initial_delay_ms: u64,
    /// Maximum reconnection delay in milliseconds (default: 60000)
    pub max_delay_ms: u64,
}

impl ReconnectConfigRecord {
    fn to_core(&self) -> marketdata_core::ReconnectionConfig {
        // Explicit opt-in path: the user passed a ReconnectConfigRecord, so
        // they want auto-reconnect. `default()` returns `enabled = true` in
        // core 0.4.0 — same intent, no override needed here.
        let mut cfg = marketdata_core::ReconnectionConfig::default();
        if self.max_attempts > 0 {
            cfg.max_attempts = self.max_attempts;
        }
        if self.initial_delay_ms > 0 {
            cfg.initial_delay = std::time::Duration::from_millis(self.initial_delay_ms);
        }
        if self.max_delay_ms > 0 {
            cfg.max_delay = std::time::Duration::from_millis(self.max_delay_ms);
        }
        cfg
    }
}

/// Health check configuration record for FFI
///
/// All fields are optional — zero/false values mean "use default".
#[derive(Debug, Clone, uniffi::Record)]
pub struct HealthCheckConfigRecord {
    /// Whether liveness detection is active (default: true in 3.0)
    pub enabled: bool,
    /// Maximum allowed gap between inbound frames before declaring the
    /// connection dead, in milliseconds. Default 35000; floor 5000.
    /// Pass 0 to use the default.
    pub heartbeat_timeout_ms: u64,
}

impl HealthCheckConfigRecord {
    fn to_core(&self) -> marketdata_core::HealthCheckConfig {
        let default = marketdata_core::HealthCheckConfig::default();
        marketdata_core::HealthCheckConfig {
            enabled: self.enabled,
            // 0 means "unset" across the FFI boundary — there is no Option<u64>
            // that reads naturally in C#/Go/Java, so fall back to the default.
            heartbeat_timeout: if self.heartbeat_timeout_ms > 0 {
                std::time::Duration::from_millis(self.heartbeat_timeout_ms)
            } else {
                default.heartbeat_timeout
            },
        }
    }
}

/// What the client does with an inbound message while its queue already
/// holds `buffer` unread messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum MessageOverflowRecord {
    /// Drop new messages and report them through `on_messages_dropped`.
    DropNewest,
    /// Never drop: the queue grows while `on_message` lags.
    Unbounded,
}

/// Message queue configuration record for FFI
///
/// `buffer` is 0 for the default (4096).
#[derive(Debug, Clone, uniffi::Record)]
pub struct MessageQueueConfigRecord {
    /// What happens to new messages while `buffer` are unread
    pub overflow: MessageOverflowRecord,
    /// Unread messages held (default 4096; 0 means default)
    pub buffer: u32,
}

impl MessageQueueConfigRecord {
    fn apply(&self, config: &mut marketdata_core::ConnectionConfig) {
        config.message_overflow = match self.overflow {
            MessageOverflowRecord::DropNewest => marketdata_core::MessageOverflow::DropNewest,
            MessageOverflowRecord::Unbounded => marketdata_core::MessageOverflow::Unbounded,
        };
        config.message_buffer = if self.buffer > 0 {
            self.buffer as usize
        } else {
            marketdata_core::websocket::DEFAULT_MESSAGE_BUFFER
        };
    }
}

/// Endpoint type for WebSocket connection
#[derive(Debug, Clone, Copy, uniffi::Enum)]
pub enum WebSocketEndpoint {
    /// Stock market data endpoint
    Stock,
    /// Futures and options market data endpoint
    FutOpt,
}

/// Per-product streaming version selection.
///
/// UniFFI has no way to express core's one-enum-per-product typing across
/// C#/Go/Java/C++ at once, so this carries optional strings and validates
/// them — the same shape the official SDK's version map has.
#[derive(uniffi::Record, Clone, Debug, Default)]
pub struct StreamingVersionRecord {
    /// Stock streaming version. Only "v1.0" is served. None means latest.
    pub stock: Option<String>,
    /// FutOpt streaming version: "v1.0" or "v1.1". None means latest (v1.1).
    ///
    /// v1.1 adds trial-matching (試撮) frames on trades / books — check the
    /// frame's `isTrial` before acting on a price.
    pub futopt: Option<String>,
}

impl StreamingVersionRecord {
    fn resolve(
        &self,
    ) -> Result<
        (
            marketdata_core::websocket::StockVersion,
            marketdata_core::websocket::FutOptVersion,
        ),
        MarketDataError,
    > {
        use marketdata_core::websocket::{FutOptVersion, StockVersion};

        let stock = match self.stock.as_deref() {
            None => StockVersion::default(),
            Some("v1.0") => StockVersion::V1_0,
            Some(other) => {
                return Err(crate::errors::config_error(format!(
                    "stock streaming does not support {other} (supported: v1.0). \
                     Leave it unset to use v1.0."
                )))
            }
        };
        let futopt = match self.futopt.as_deref() {
            None => FutOptVersion::default(),
            Some("v1.0") => FutOptVersion::V1_0,
            Some("v1.1") => FutOptVersion::V1_1,
            Some(other) => {
                return Err(crate::errors::config_error(format!(
                    "futopt streaming does not support {other} (supported: v1.0, v1.1). \
                     Leave it unset to use v1.1."
                )))
            }
        };
        Ok((stock, futopt))
    }
}

/// WebSocket client for real-time market data streaming
///
/// Wraps the core WebSocketClient and forwards messages to the provided
/// WebSocketListener implementation via a background task.
#[derive(uniffi::Object)]
pub struct WebSocketClient {
    /// Async methods clone the client out before awaiting.
    inner: std::sync::Mutex<Option<Arc<CoreWebSocketClient>>>,
    listener: Arc<dyn WebSocketListener>,
    /// Sent in the auth frame; its field follows the credential kind (#91).
    auth: AuthRequest,
    base_url: Option<String>,
    version: StreamingVersionRecord,
    endpoint: WebSocketEndpoint,
    /// Core's connection state of the current or last connection, read by
    /// `is_connected()` and `is_closed()`; outlives the core client, which
    /// `disconnect()` drops.
    state: std::sync::Mutex<Option<marketdata_core::ConnectionStateHandle>>,
    /// Tells the current connection's stream reader that `disconnect()` was
    /// called. One per `connect()`, so a reader still draining a previous
    /// connection never sees a later connection's flag.
    stopping: std::sync::Mutex<Arc<AtomicBool>>,
    reconnect_config: Option<marketdata_core::ReconnectionConfig>,
    health_check_config: Option<marketdata_core::HealthCheckConfig>,
    tls_config: Option<marketdata_core::TlsConfig>,
    message_queue: Option<MessageQueueConfigRecord>,
    /// Dropped-message count of the current or last connection; outlives the
    /// core client, which `disconnect()` drops.
    messages_dropped: std::sync::Mutex<Option<marketdata_core::MessagesDroppedHandle>>,
    /// Throttles the reports of failed listener calls (#83) across the
    /// client's connections, like the Node, Python, C# and Java bindings.
    callback_failures: CallbackFailures,
    /// Tokio runtime for sync wrappers (C++ feature). Kept alive for background tasks.
    #[cfg(feature = "cpp")]
    sync_runtime: std::sync::Mutex<Option<tokio::runtime::Runtime>>,
}

impl WebSocketClient {
    /// Create a new WebSocket client (internal constructor)
    fn new_internal(
        auth: AuthRequest,
        listener: Arc<dyn WebSocketListener>,
        endpoint: WebSocketEndpoint,
        reconnect_config: Option<marketdata_core::ReconnectionConfig>,
        health_check_config: Option<marketdata_core::HealthCheckConfig>,
        base_url: Option<String>,
        tls_config: Option<marketdata_core::TlsConfig>,
        version: StreamingVersionRecord,
        message_queue: Option<MessageQueueConfigRecord>,
    ) -> Arc<Self> {
        Arc::new(Self {
            inner: std::sync::Mutex::new(None),
            listener,
            auth,
            base_url,
            version,
            endpoint,
            state: std::sync::Mutex::new(None),
            stopping: std::sync::Mutex::new(Arc::new(AtomicBool::new(false))),
            reconnect_config,
            health_check_config,
            tls_config,
            message_queue,
            messages_dropped: std::sync::Mutex::new(None),
            callback_failures: CallbackFailures::default(),
            #[cfg(feature = "cpp")]
            sync_runtime: std::sync::Mutex::new(None),
        })
    }
}

#[uniffi::export]
impl WebSocketClient {
    /// Create a new WebSocket client for stock market data
    ///
    /// # Arguments
    /// * `api_key` - Fugle API key for authentication
    /// * `listener` - Callback interface for receiving WebSocket events
    #[uniffi::constructor]
    pub fn new(api_key: String, listener: Arc<dyn WebSocketListener>) -> Arc<Self> {
        Self::new_internal(AuthRequest::with_api_key(api_key), listener, WebSocketEndpoint::Stock, None, None, None, None, Default::default(), None)
    }

    /// Create a new WebSocket client for a specific endpoint
    ///
    /// # Arguments
    /// * `api_key` - Fugle API key for authentication
    /// * `listener` - Callback interface for receiving WebSocket events
    /// * `endpoint` - The market data endpoint (Stock or FutOpt)
    #[uniffi::constructor]
    pub fn new_with_endpoint(
        api_key: String,
        listener: Arc<dyn WebSocketListener>,
        endpoint: WebSocketEndpoint,
    ) -> Arc<Self> {
        Self::new_internal(AuthRequest::with_api_key(api_key), listener, endpoint, None, None, None, None, Default::default(), None)
    }

    /// Create a new WebSocket client with full configuration
    ///
    /// # Arguments
    /// * `api_key` - Fugle API key for authentication
    /// * `listener` - Callback interface for receiving WebSocket events
    /// * `endpoint` - The market data endpoint (Stock or FutOpt)
    /// * `reconnect_config` - Optional reconnection configuration
    /// * `health_check_config` - Optional health check configuration
    #[uniffi::constructor]
    pub fn new_with_config(
        api_key: String,
        listener: Arc<dyn WebSocketListener>,
        endpoint: WebSocketEndpoint,
        reconnect_config: Option<ReconnectConfigRecord>,
        health_check_config: Option<HealthCheckConfigRecord>,
    ) -> Arc<Self> {
        Self::new_internal(
            AuthRequest::with_api_key(api_key),
            listener,
            endpoint,
            reconnect_config.map(|c| c.to_core()),
            health_check_config.map(|c| c.to_core()),
            None,
            None,
            Default::default(),
            None,
        )
    }

    /// Create a new WebSocket client with full configuration including custom base URL
    #[uniffi::constructor]
    pub fn new_with_url(
        api_key: String,
        listener: Arc<dyn WebSocketListener>,
        endpoint: WebSocketEndpoint,
        base_url: String,
        reconnect_config: Option<ReconnectConfigRecord>,
        health_check_config: Option<HealthCheckConfigRecord>,
    ) -> Arc<Self> {
        Self::new_internal(
            AuthRequest::with_api_key(api_key),
            listener,
            endpoint,
            reconnect_config.map(|c| c.to_core()),
            health_check_config.map(|c| c.to_core()),
            Some(base_url),
            None,
            Default::default(),
            None,
        )
    }

    /// Create a new WebSocket client with full configuration including TLS.
    ///
    /// All optional parameters can be None to use defaults. This is the
    /// TLS-aware variant of `new_with_url` — use this when you need to
    /// pin a custom CA or disable cert verification.
    ///
    /// # Arguments
    /// * `api_key` - Fugle API key for authentication
    /// * `listener` - Callback interface for receiving WebSocket events
    /// * `endpoint` - The market data endpoint (Stock or FutOpt)
    /// * `base_url` - Optional base URL override
    /// * `reconnect_config` - Optional reconnection configuration
    /// * `health_check_config` - Optional health check configuration
    /// * `tls` - Optional TLS customization (custom CA or accept_invalid_certs)
    #[uniffi::constructor]
    pub fn new_with_full_config(
        api_key: String,
        listener: Arc<dyn WebSocketListener>,
        endpoint: WebSocketEndpoint,
        base_url: Option<String>,
        reconnect_config: Option<ReconnectConfigRecord>,
        health_check_config: Option<HealthCheckConfigRecord>,
        tls: Option<crate::tls::TlsConfigRecord>,
        version: Option<StreamingVersionRecord>,
    ) -> Arc<Self> {
        Self::new_internal(
            AuthRequest::with_api_key(api_key),
            listener,
            endpoint,
            reconnect_config.map(|c| c.to_core()),
            health_check_config.map(|c| c.to_core()),
            base_url,
            tls.map(|t| t.to_core()),
            version.unwrap_or_default(),
            None,
        )
    }

    /// Create a new WebSocket client with full configuration plus the
    /// message queue settings.
    ///
    /// Same as `new_with_full_config`, with `message_queue` choosing what
    /// happens while `on_message` falls behind (None for the defaults:
    /// `DropNewest`, 4096 messages).
    ///
    /// # Arguments
    /// * `api_key` - Fugle API key for authentication
    /// * `listener` - Callback interface for receiving WebSocket events
    /// * `endpoint` - The market data endpoint (Stock or FutOpt)
    /// * `base_url` - Optional base URL override
    /// * `reconnect_config` - Optional reconnection configuration
    /// * `health_check_config` - Optional health check configuration
    /// * `tls` - Optional TLS customization (custom CA or accept_invalid_certs)
    /// * `version` - Optional per-product streaming version
    /// * `message_queue` - Optional message queue configuration
    #[uniffi::constructor]
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_options(
        api_key: String,
        listener: Arc<dyn WebSocketListener>,
        endpoint: WebSocketEndpoint,
        base_url: Option<String>,
        reconnect_config: Option<ReconnectConfigRecord>,
        health_check_config: Option<HealthCheckConfigRecord>,
        tls: Option<crate::tls::TlsConfigRecord>,
        version: Option<StreamingVersionRecord>,
        message_queue: Option<MessageQueueConfigRecord>,
    ) -> Arc<Self> {
        Self::new_internal(
            AuthRequest::with_api_key(api_key),
            listener,
            endpoint,
            reconnect_config.map(|c| c.to_core()),
            health_check_config.map(|c| c.to_core()),
            base_url,
            tls.map(|t| t.to_core()),
            version.unwrap_or_default(),
            message_queue,
        )
    }

    /// Create a new WebSocket client from whichever credential was given.
    ///
    /// Takes the same three credentials as the REST client: exactly one must
    /// be non-empty (an empty or whitespace-only value counts as not
    /// provided), otherwise this returns a `ConfigError` (code 1004). The
    /// auth frame then carries it as `apikey`, `token` or `sdkToken`.
    /// The other arguments are those of `new_with_options`.
    ///
    /// The credentials are one record rather than three arguments: with three
    /// more buffers than `new_with_options` the Java binding (JNA) passed
    /// garbage to Rust on macOS arm64.
    #[uniffi::constructor]
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_credentials(
        credentials: CredentialsRecord,
        listener: Arc<dyn WebSocketListener>,
        endpoint: WebSocketEndpoint,
        base_url: Option<String>,
        reconnect_config: Option<ReconnectConfigRecord>,
        health_check_config: Option<HealthCheckConfigRecord>,
        tls: Option<crate::tls::TlsConfigRecord>,
        version: Option<StreamingVersionRecord>,
        message_queue: Option<MessageQueueConfigRecord>,
    ) -> Result<Arc<Self>, MarketDataError> {
        let CredentialsRecord { api_key, bearer_token, sdk_token } = credentials;
        let auth = marketdata_core::Auth::from_credentials(api_key, bearer_token, sdk_token)?;
        Ok(Self::new_internal(
            AuthRequest::from(auth),
            listener,
            endpoint,
            reconnect_config.map(|c| c.to_core()),
            health_check_config.map(|c| c.to_core()),
            base_url,
            tls.map(|t| t.to_core()),
            version.unwrap_or_default(),
            message_queue,
        ))
    }

    /// Messages dropped because they arrived while the message queue held
    /// `buffer` unread messages (`MessageOverflowRecord::DropNewest`).
    ///
    /// Counted from the start of the current connection (every `connect()` or
    /// reconnect restarts it); after `disconnect()` it still reads the last
    /// connection's count. 0 before the first `connect()`.
    pub fn messages_dropped_total(&self) -> u64 {
        self.messages_dropped
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_ref()
            .map_or(0, |handle| handle.total())
    }

    /// Check if the client is currently connected
    ///
    /// Reads core's connection state, so it is false while reconnecting and
    /// right after the connection drops, without waiting for the event thread.
    pub fn is_connected(&self) -> bool {
        self.state_handle().is_some_and(|state| state.is_connected())
    }

    /// Check if the connection has ended
    ///
    /// Reads core's connection state: true after `disconnect()`, and after
    /// the server closes the connection when no reconnect follows (disabled
    /// or attempts exhausted). False while reconnecting and before the first
    /// `connect()`.
    pub fn is_closed(&self) -> bool {
        self.state_handle().is_some_and(|state| state.is_closed())
    }
}

#[cfg(not(feature = "cpp"))]
#[uniffi::export(async_runtime = "tokio")]
impl WebSocketClient {
    pub async fn connect(&self) -> Result<(), MarketDataError> {
        self.connect_impl().await
    }

    /// Subscribe to a channel for a symbol.
    ///
    /// After-hours (盤後) is FutOpt only: on the Stock endpoint, any value
    /// other than null is 1005 `INVALID_PARAMETER`.
    #[uniffi::method(default(after_hours = None))]
    pub async fn subscribe(
        &self,
        channel: String,
        symbol: String,
        after_hours: Option<bool>,
    ) -> Result<(), MarketDataError> {
        let sub = self.subscription(&channel, &symbol, after_hours)?;
        self.subscribe_impl(sub).await
    }

    /// Unsubscribe from a channel for a symbol.
    ///
    /// Pass the same after-hours value as the `subscribe` call: an after-hours
    /// subscription is a separate subscription from the regular one.
    #[uniffi::method(default(after_hours = None))]
    pub async fn unsubscribe(
        &self,
        channel: String,
        symbol: String,
        after_hours: Option<bool>,
    ) -> Result<(), MarketDataError> {
        let sub = self.subscription(&channel, &symbol, after_hours)?;
        self.unsubscribe_impl(sub).await
    }

    pub async fn ping(&self, state: Option<String>) -> Result<(), MarketDataError> {
        self.ping_impl(state).await
    }

    pub async fn query_subscriptions(&self) -> Result<(), MarketDataError> {
        self.query_subscriptions_impl().await
    }

    pub async fn disconnect(&self) {
        self.disconnect_impl().await
    }
}

impl WebSocketClient {
    /// Connect to the WebSocket server (implementation).
    async fn connect_impl(&self) -> Result<(), MarketDataError> {
        // Resolve the endpoint through core's factory so `base_url` semantics
        // and the per-product version live in one place. Before 0.8.0 this
        // hand-rolled `format!("{base}/stock/streaming")`, which is how the
        // version segment ended up being the caller's problem.
        let (stock_version, futopt_version) = self.version.resolve()?;
        let auth = self.auth.clone();
        let mut factory = marketdata_core::WebSocketFactory::new()
            .stock_version(stock_version)
            .futopt_version(futopt_version);
        if let Some(ref url) = self.base_url {
            factory = factory.base_url(url);
        }
        let factory = factory.auth(auth);
        let mut config = match self.endpoint {
            WebSocketEndpoint::Stock => factory.stock()?.build(),
            WebSocketEndpoint::FutOpt => factory.futopt()?.build(),
        };
        // Apply TLS customization if provided (custom CA / accept_invalid_certs).
        if let Some(ref tls) = self.tls_config {
            config.tls = tls.clone();
        }
        if let Some(ref message_queue) = self.message_queue {
            message_queue.apply(&mut config);
        }

        // Create core WebSocket client with optional reconnection/health-check config
        let core_ws = if let (Some(rc), Some(hc)) = (&self.reconnect_config, &self.health_check_config) {
            CoreWebSocketClient::with_full_config(config, rc.clone(), hc.clone())
        } else if let Some(rc) = &self.reconnect_config {
            CoreWebSocketClient::with_full_config(
                config,
                rc.clone(),
                marketdata_core::HealthCheckConfig::default(),
            )
        } else if let Some(hc) = &self.health_check_config {
            // Binding-side compensation for the core 0.4.0 default flip:
            // when the caller did NOT supply a `ReconnectConfigRecord`,
            // preserve the historical FFI semantics (no auto-reconnect)
            // by explicitly disabling reconnect. Bypasses
            // `ReconnectionConfig::default()` which now returns
            // `enabled = true`.
            CoreWebSocketClient::with_full_config(
                config,
                marketdata_core::ReconnectionConfig::disabled(),
                hc.clone(),
            )
        } else {
            // Same compensation as above: when the caller passed neither
            // a reconnect record nor a health-check record, route through
            // `with_reconnection_config(_, disabled())` instead of `new()`
            // so the binding default stays "no auto-reconnect".
            CoreWebSocketClient::with_reconnection_config(
                config,
                marketdata_core::ReconnectionConfig::disabled(),
            )
        };

        *self
            .messages_dropped
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(core_ws.messages_dropped_handle());
        *lock_state(&self.state) = Some(core_ws.state_handle());

        // Forward core's stream: messages and lifecycle events in the order
        // core produced them (#68). The thread starts before `connect()` so
        // events of a failed attempt (`Unauthenticated`, `Error`) are still
        // delivered.
        let stopping = Arc::new(AtomicBool::new(false));
        *lock_stopping(&self.stopping) = Arc::clone(&stopping);
        spawn_stream_reader(
            core_ws.stream_receiver(),
            Arc::clone(&self.listener),
            stopping,
            Arc::clone(&self.callback_failures),
        );

        // Connect to server
        core_ws.connect().await?;

        // Store client in inner
        *lock_inner(&self.inner) = Some(Arc::new(core_ws));

        Ok(())
    }

    /// The subscription for this client's endpoint: FutOpt channels and
    /// `after_hours` on FutOpt, stock channels on Stock.
    ///
    /// Callers build it before checking the connection, so an unknown channel,
    /// or `after_hours` on the Stock endpoint, is 1005 `INVALID_PARAMETER`
    /// whether or not the client is connected.
    fn subscription(
        &self,
        channel: &str,
        symbol: &str,
        after_hours: Option<bool>,
    ) -> Result<Subscription, MarketDataError> {
        match self.endpoint {
            WebSocketEndpoint::Stock => {
                // The channel first, as on the FutOpt endpoint.
                let channel = channel.parse()?;
                if after_hours.is_some() {
                    return Err(marketdata_core::MarketDataError::InvalidParameter {
                        name: "afterHours".to_string(),
                        reason: "only supported on the FutOpt endpoint".to_string(),
                    }
                    .into());
                }
                Ok(Subscription::Stock(marketdata_core::StockSubscription::new(channel, symbol)))
            }
            WebSocketEndpoint::FutOpt => Ok(Subscription::FutOpt(
                marketdata_core::FutOptSubscription::new(channel.parse()?, symbol)
                    .with_after_hours(after_hours.unwrap_or(false)),
            )),
        }
    }

    /// Subscribe to a channel for a symbol
    ///
    /// # Errors
    ///
    /// Returns error if not connected or subscription fails.
    async fn subscribe_impl(&self, sub: Subscription) -> Result<(), MarketDataError> {
        if let Some(ws) = self.client() {
            match sub {
                Subscription::Stock(sub) => ws.subscribe(sub).await?,
                Subscription::FutOpt(sub) => ws.subscribe_futopt(sub).await?,
            }
            Ok(())
        } else {
            Err(crate::errors::not_connected_error("Not connected"))
        }
    }

    /// Unsubscribe the subscription `sub` names
    ///
    /// # Errors
    ///
    /// Returns error if not connected.
    async fn unsubscribe_impl(&self, sub: Subscription) -> Result<(), MarketDataError> {
        if let Some(ws) = self.client() {
            ws.unsubscribe(sub.keys()).await?;
            Ok(())
        } else {
            Err(crate::errors::not_connected_error("Not connected"))
        }
    }

    /// Send a ping message to the server
    ///
    /// # Arguments
    /// * `state` - Optional state string echoed back in the pong response
    async fn ping_impl(&self, state: Option<String>) -> Result<(), MarketDataError> {
        if let Some(ws) = self.client() {
            let request = marketdata_core::WebSocketRequest::ping(state);
            ws.send(request).await?;
            Ok(())
        } else {
            Err(crate::errors::not_connected_error("Not connected"))
        }
    }

    async fn query_subscriptions_impl(&self) -> Result<(), MarketDataError> {
        if let Some(ws) = self.client() {
            let request = marketdata_core::WebSocketRequest::subscriptions();
            ws.send(request).await?;
            Ok(())
        } else {
            Err(crate::errors::not_connected_error("Not connected"))
        }
    }

    async fn disconnect_impl(&self) {
        // No messages are delivered once `disconnect()` has been called; the
        // connection's remaining events still are.
        lock_stopping(&self.stopping).store(true, Ordering::SeqCst);

        // Take and disconnect the client. `on_disconnected` comes from
        // core's `Disconnected` event, which `ws.disconnect()` emits.
        let ws = lock_inner(&self.inner).take();
        if let Some(ws) = ws {
            let _ = ws.disconnect().await;
        }
    }

    /// Core's connection state of the current or last connection.
    fn state_handle(&self) -> Option<marketdata_core::ConnectionStateHandle> {
        lock_state(&self.state).clone()
    }

    /// The current core client, cloned out so no lock is held across awaits.
    fn client(&self) -> Option<Arc<CoreWebSocketClient>> {
        lock_inner(&self.inner).clone()
    }
}

/// A subscription built for the client's endpoint.
enum Subscription {
    Stock(marketdata_core::StockSubscription),
    FutOpt(marketdata_core::FutOptSubscription),
}

impl Subscription {
    /// Core's subscription keys, which `unsubscribe` looks up.
    fn keys(&self) -> Vec<String> {
        match self {
            Subscription::Stock(sub) => sub.keys(),
            Subscription::FutOpt(sub) => sub.keys(),
        }
    }
}

/// Sync (blocking) wrappers for C++ compatibility.
/// Uses a persistent tokio runtime stored in the client to keep background tasks alive.
#[cfg(feature = "cpp")]
#[uniffi::export]
impl WebSocketClient {
    /// Connect to the WebSocket server (blocking).
    pub fn connect_sync(&self) -> Result<(), MarketDataError> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| crate::errors::other_error(e.to_string()))?;
        let result = rt.block_on(self.connect_impl());
        // Store runtime to keep background tasks alive
        if let Ok(mut guard) = self.sync_runtime.lock() {
            *guard = Some(rt);
        }
        result
    }

    /// Subscribe to a channel for a symbol (blocking).
    pub fn subscribe_sync(&self, channel: String, symbol: String) -> Result<(), MarketDataError> {
        // Before the runtime check, as in `subscribe`.
        let sub = self.subscription(&channel, &symbol, None)?;
        let guard = self.sync_runtime.lock().unwrap();
        if let Some(ref rt) = *guard {
            rt.block_on(self.subscribe_impl(sub))
        } else {
            Err(crate::errors::not_connected_error("Not connected (call connect_sync first)"))
        }
    }

    /// Unsubscribe from a channel for a symbol (blocking).
    pub fn unsubscribe_sync(&self, channel: String, symbol: String) -> Result<(), MarketDataError> {
        let sub = self.subscription(&channel, &symbol, None)?;
        let guard = self.sync_runtime.lock().unwrap();
        if let Some(ref rt) = *guard {
            rt.block_on(self.unsubscribe_impl(sub))
        } else {
            Err(crate::errors::not_connected_error("Not connected"))
        }
    }

    /// Send a ping message (blocking).
    pub fn ping_sync(&self, state: Option<String>) -> Result<(), MarketDataError> {
        let guard = self.sync_runtime.lock().unwrap();
        if let Some(ref rt) = *guard {
            rt.block_on(self.ping_impl(state))
        } else {
            Err(crate::errors::not_connected_error("Not connected"))
        }
    }

    /// Query server subscriptions (blocking).
    pub fn query_subscriptions_sync(&self) -> Result<(), MarketDataError> {
        let guard = self.sync_runtime.lock().unwrap();
        if let Some(ref rt) = *guard {
            rt.block_on(self.query_subscriptions_impl())
        } else {
            Err(crate::errors::not_connected_error("Not connected"))
        }
    }

    /// Disconnect from the WebSocket server (blocking).
    pub fn disconnect_sync(&self) {
        let guard = self.sync_runtime.lock().unwrap();
        if let Some(ref rt) = *guard {
            rt.block_on(self.disconnect_impl());
        }
        drop(guard);
        // Drop runtime to clean up
        if let Ok(mut guard) = self.sync_runtime.lock() {
            *guard = None;
        }
    }
}

/// Spawn the thread that forwards core's stream to the listener.
///
/// Messages are forwarded only while the connection that produced them is
/// authenticated as far as this reader has reported, so frames of a rejected
/// attempt never reach `on_message`, and none are forwarded once `stopping`
/// is set by `disconnect()`.
///
/// Exits after a terminal event (`Disconnected { will_reconnect: false }` or
/// `ReconnectFailed`) — core emits nothing after those — or once the stream
/// closes, which covers a failed `connect()` and a `disconnect()` issued
/// mid-reconnect (both drop every sender without a terminal event).
fn spawn_stream_reader(
    stream: Arc<StreamReceiver>,
    listener: Arc<dyn WebSocketListener>,
    stopping: Arc<AtomicBool>,
    callback_failures: CallbackFailures,
) {
    std::thread::Builder::new()
        .name("ws_stream_reader".to_string())
        .spawn(move || {
            let mut authenticated = false;
            let mut calls = ListenerCalls::new(listener.as_ref(), callback_failures);
            while let Ok(item) = stream.receive() {
                match item {
                    StreamItem::Message(message)
                        if authenticated && !stopping.load(Ordering::SeqCst) =>
                    {
                        calls.call("on_message", |l| l.on_message(StreamMessage::from(message)));
                    }
                    StreamItem::Event(event) => {
                        match event {
                            ConnectionEvent::Authenticated { .. } => authenticated = true,
                            ConnectionEvent::Unauthenticated { .. }
                            | ConnectionEvent::Disconnected { .. } => authenticated = false,
                            _ => {}
                        }
                        if !forward_event(event, &mut calls) {
                            break;
                        }
                    }
                    _ => {}
                }
            }
        })
        .ok();
}

/// Forward one core event to the listener. Returns `false` after a terminal
/// event.
fn forward_event(event: ConnectionEvent, calls: &mut ListenerCalls<'_>) -> bool {
    match event {
        ConnectionEvent::Connected => calls.call("on_connected", |l| l.on_connected()),
        ConnectionEvent::Authenticated { data } => {
            calls.call("on_authenticated", |l| l.on_authenticated(json_or_none(data)))
        }
        ConnectionEvent::Unauthenticated { data, .. } => {
            calls.call("on_unauthenticated", |l| l.on_unauthenticated(json_or_none(data)))
        }
        ConnectionEvent::Disconnected { will_reconnect, .. } => {
            calls.call("on_disconnected", |l| l.on_disconnected(will_reconnect));
            return will_reconnect;
        }
        ConnectionEvent::Reconnecting { attempt } => {
            calls.call("on_reconnecting", |l| l.on_reconnecting(attempt))
        }
        ConnectionEvent::ReconnectFailed { attempts } => {
            calls.call("on_reconnect_failed", |l| l.on_reconnect_failed(attempts));
            return false;
        }
        ConnectionEvent::Error(info) => calls.call("on_error", |l| l.on_error(ErrorInfo::from(&info))),
        ConnectionEvent::MessagesDropped { dropped, .. } => {
            calls.call("on_messages_dropped", |l| l.on_messages_dropped(dropped))
        }
        _ => {}
    }
    true
}

/// The stream reader's calls into the listener (#83).
///
/// A foreign listener method that throws makes UniFFI panic on this thread,
/// which would end the reader and silence every later event. Each call is
/// caught instead and reported through `on_error` (code
/// [`CALLBACK_FAILED`](marketdata_core::error_code::CALLBACK_FAILED)),
/// throttled: the first at once, later ones at most once per second with the
/// number of failures since the previous report. A failing `on_error` is only
/// printed to stderr.
///
/// The C# and Java wrappers catch their listener's exceptions before they
/// reach here; this covers listeners implemented on the generated bindings
/// directly (C++, or the generated C# / Java interfaces).
struct ListenerCalls<'a> {
    listener: &'a dyn WebSocketListener,
    failures: CallbackFailures,
}

/// The client's [`ReportThrottle`](marketdata_core::websocket::ReportThrottle)
/// for failed listener calls, shared by its connections' stream readers.
type CallbackFailures = Arc<std::sync::Mutex<marketdata_core::websocket::ReportThrottle>>;

impl<'a> ListenerCalls<'a> {
    fn new(listener: &'a dyn WebSocketListener, failures: CallbackFailures) -> Self {
        Self { listener, failures }
    }

    /// Run `call` on the listener; `method` names it in a failure report.
    fn call(&mut self, method: &'static str, call: impl FnOnce(&dyn WebSocketListener)) {
        let listener = self.listener;
        let Err(payload) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| call(listener))) else {
            return;
        };
        let detail = panic_detail(&*payload);
        if method == "on_error" {
            eprintln!("[fugle-marketdata] listener on_error failed: {detail}");
            return;
        }
        let count = self
            .failures
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .record(std::time::Instant::now());
        let Some(count) = count else {
            return;
        };
        let info = marketdata_core::ErrorInfo::new(
            marketdata_core::error_code::CALLBACK_FAILED,
            marketdata_core::ErrorKind::Client,
            format!("Listener {method} failed: {detail} ({count} in the last 1s)"),
        );
        let report = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            listener.on_error(ErrorInfo::from(&info))
        }));
        if let Err(payload) = report {
            eprintln!(
                "[fugle-marketdata] listener on_error failed: {} (while reporting: {})",
                panic_detail(&*payload),
                info.message
            );
        }
    }
}

/// The message of a panic, without the prefix UniFFI adds when a foreign
/// callback throws.
fn panic_detail(payload: &(dyn std::any::Any + Send)) -> String {
    let detail = payload
        .downcast_ref::<&str>()
        .map(|message| message.to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "unknown panic".to_string());
    match detail.strip_prefix("Callback interface failure: ") {
        Some(foreign) => foreign.to_string(),
        None => detail,
    }
}

/// Lock `stopping`, recovering from poison: it only holds an `Arc`.
fn lock_stopping(stopping: &std::sync::Mutex<Arc<AtomicBool>>) -> std::sync::MutexGuard<'_, Arc<AtomicBool>> {
    stopping.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Lock `state`, recovering from poison: it only holds a handle.
fn lock_state(
    state: &std::sync::Mutex<Option<marketdata_core::ConnectionStateHandle>>,
) -> std::sync::MutexGuard<'_, Option<marketdata_core::ConnectionStateHandle>> {
    state.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Lock `inner`, recovering from poison: the slot holds no invariant a
/// panicking holder could break.
fn lock_inner(
    inner: &std::sync::Mutex<Option<Arc<CoreWebSocketClient>>>,
) -> std::sync::MutexGuard<'_, Option<Arc<CoreWebSocketClient>>> {
    inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// `Null` means the frame carried no `data`.
fn json_or_none(data: serde_json::Value) -> Option<String> {
    (!data.is_null()).then(|| data.to_string())
}

/// Create a new WebSocket client for stock market data
///
/// # Arguments
/// * `api_key` - Fugle API key for authentication
/// * `listener` - Callback interface for receiving WebSocket events
///
/// # Returns
/// A WebSocketClient instance wrapped in Arc for thread-safe access
#[uniffi::export]
pub fn new_websocket_client(
    api_key: String,
    listener: Arc<dyn WebSocketListener>,
) -> Arc<WebSocketClient> {
    WebSocketClient::new(api_key, listener)
}

/// Create a new WebSocket client for a specific endpoint
///
/// # Arguments
/// * `api_key` - Fugle API key for authentication
/// * `listener` - Callback interface for receiving WebSocket events
/// * `endpoint` - The market data endpoint (Stock or FutOpt)
///
/// # Returns
/// A WebSocketClient instance wrapped in Arc for thread-safe access
#[uniffi::export]
pub fn new_websocket_client_with_endpoint(
    api_key: String,
    listener: Arc<dyn WebSocketListener>,
    endpoint: WebSocketEndpoint,
) -> Arc<WebSocketClient> {
    WebSocketClient::new_with_endpoint(api_key, listener, endpoint)
}

/// Create a new WebSocket client with full configuration
///
/// # Arguments
/// * `api_key` - Fugle API key for authentication
/// * `listener` - Callback interface for receiving WebSocket events
/// * `endpoint` - The market data endpoint (Stock or FutOpt)
/// * `reconnect_config` - Optional reconnection configuration
/// * `health_check_config` - Optional health check configuration
///
/// # Returns
/// A WebSocketClient instance wrapped in Arc for thread-safe access
#[uniffi::export]
pub fn new_websocket_client_with_config(
    api_key: String,
    listener: Arc<dyn WebSocketListener>,
    endpoint: WebSocketEndpoint,
    reconnect_config: Option<ReconnectConfigRecord>,
    health_check_config: Option<HealthCheckConfigRecord>,
) -> Arc<WebSocketClient> {
    WebSocketClient::new_with_config(api_key, listener, endpoint, reconnect_config, health_check_config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use marketdata_core::testing::MockWsServer;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Test listener that tracks callback invocations
    struct TestListener {
        connected_count: AtomicUsize,
        disconnected_count: AtomicUsize,
        message_count: AtomicUsize,
        error_count: AtomicUsize,
        reconnecting_count: AtomicUsize,
        reconnect_failed_count: AtomicUsize,
        messages_dropped: AtomicUsize,
        last_error: Mutex<Option<ErrorInfo>>,
        /// Lifecycle callbacks in delivery order.
        events: Mutex<Vec<String>>,
        /// Also record `on_message` in `events`, as `message(<event>)`.
        record_messages: std::sync::atomic::AtomicBool,
        /// Milliseconds `on_message` blocks, to fall behind on purpose.
        message_delay_ms: std::sync::atomic::AtomicU64,
        /// The client `on_disconnected` reads `is_connected()` of, if set.
        client: std::sync::OnceLock<std::sync::Weak<WebSocketClient>>,
        /// `is_connected()` as `on_disconnected` read it, per call.
        connected_on_disconnect: Mutex<Vec<bool>>,
    }

    impl TestListener {
        fn new() -> Self {
            Self {
                connected_count: AtomicUsize::new(0),
                disconnected_count: AtomicUsize::new(0),
                message_count: AtomicUsize::new(0),
                error_count: AtomicUsize::new(0),
                reconnecting_count: AtomicUsize::new(0),
                reconnect_failed_count: AtomicUsize::new(0),
                messages_dropped: AtomicUsize::new(0),
                last_error: Mutex::new(None),
                events: Mutex::new(Vec::new()),
                record_messages: std::sync::atomic::AtomicBool::new(false),
                message_delay_ms: std::sync::atomic::AtomicU64::new(0),
                client: std::sync::OnceLock::new(),
                connected_on_disconnect: Mutex::new(Vec::new()),
            }
        }

        fn recording_messages() -> Self {
            let listener = Self::new();
            listener.record_messages.store(true, Ordering::SeqCst);
            listener
        }

        fn record(&self, event: String) {
            self.events.lock().unwrap().push(event);
        }

        fn events(&self) -> Vec<String> {
            self.events.lock().unwrap().clone()
        }

        /// Wait until `event` has been recorded (the forwarder runs on its
        /// own thread), then give stray duplicates a moment to show up.
        async fn wait_for(&self, event: &str) {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            while !self.events().iter().any(|e| e == event) {
                assert!(
                    std::time::Instant::now() < deadline,
                    "timed out waiting for {event}; got {:?}",
                    self.events()
                );
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        }
    }

    impl WebSocketListener for TestListener {
        fn on_connected(&self) {
            self.connected_count.fetch_add(1, Ordering::SeqCst);
            self.record("connected".to_string());
        }

        fn on_authenticated(&self, data_json: Option<String>) {
            self.record(format!("authenticated({data_json:?})"));
        }

        fn on_unauthenticated(&self, data_json: Option<String>) {
            self.record(format!("unauthenticated({data_json:?})"));
        }

        fn on_disconnected(&self, will_reconnect: bool) {
            if let Some(client) = self.client.get().and_then(std::sync::Weak::upgrade) {
                self.connected_on_disconnect.lock().unwrap().push(client.is_connected());
            }
            self.disconnected_count.fetch_add(1, Ordering::SeqCst);
            self.record(format!("disconnected({will_reconnect})"));
        }

        fn on_message(&self, message: StreamMessage) {
            let delay = self.message_delay_ms.load(Ordering::SeqCst);
            if delay > 0 {
                std::thread::sleep(std::time::Duration::from_millis(delay));
            }
            self.message_count.fetch_add(1, Ordering::SeqCst);
            if self.record_messages.load(Ordering::SeqCst) {
                self.record(format!("message({})", message.event));
            }
        }

        fn on_error(&self, error: ErrorInfo) {
            self.error_count.fetch_add(1, Ordering::SeqCst);
            self.record(format!("error({})", error.message));
            if let Ok(mut guard) = self.last_error.lock() {
                *guard = Some(error);
            }
        }

        fn on_reconnecting(&self, attempt: u32) {
            self.reconnecting_count.fetch_add(1, Ordering::SeqCst);
            self.record(format!("reconnecting({attempt})"));
        }

        fn on_reconnect_failed(&self, attempts: u32) {
            self.reconnect_failed_count.fetch_add(1, Ordering::SeqCst);
            self.record(format!("reconnect_failed({attempts})"));
        }

        fn on_messages_dropped(&self, count: u64) {
            self.messages_dropped.fetch_add(count as usize, Ordering::SeqCst);
            self.record(format!("messages_dropped({count})"));
        }
    }

    // Use std::sync::Mutex for tests instead of tokio::sync::Mutex
    use std::sync::Mutex;

    #[test]
    fn test_websocket_client_creation() {
        let listener = Arc::new(TestListener::new());
        let client = WebSocketClient::new("test-key".to_string(), listener);
        assert!(!client.is_connected());
    }

    #[test]
    fn test_websocket_client_with_endpoint() {
        let listener = Arc::new(TestListener::new());
        let client = WebSocketClient::new_with_endpoint(
            "test-key".to_string(),
            listener,
            WebSocketEndpoint::FutOpt,
        );
        assert!(!client.is_connected());
    }

    #[test]
    fn test_websocket_listener_receives_message() {
        // This test verifies the callback wiring works
        let listener = Arc::new(TestListener::new());

        // Simulate calling on_message
        let test_msg = StreamMessage {
            raw: r#"{"event":"data","channel":"trades","symbol":"2330"}"#.to_string(),
            event: "data".to_string(),
            channel: Some("trades".to_string()),
            symbol: Some("2330".to_string()),
            id: None,
            data_json: Some("{}".to_string()),
            error_code: None,
            error_message: None,
        };
        listener.on_message(test_msg);

        assert_eq!(
            listener.message_count.load(Ordering::SeqCst),
            1,
            "on_message callback should have been invoked"
        );
    }

    #[test]
    fn test_websocket_listener_lifecycle_callbacks() {
        let listener = Arc::new(TestListener::new());

        // Simulate connection lifecycle
        listener.on_connected();
        assert_eq!(listener.connected_count.load(Ordering::SeqCst), 1);

        listener.on_disconnected(false);
        assert_eq!(listener.disconnected_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_websocket_listener_error_callback() {
        let listener = Arc::new(TestListener::new());

        listener.on_error(ErrorInfo {
            code: 0,
            source_kind: crate::errors::ErrorSourceKind::Client,
            message: "Test error".to_string(),
            status: None,
            body: None,
            request_id: None,
            headers: std::collections::HashMap::new(),
        });
        assert_eq!(listener.error_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_factory_functions() {
        let listener: Arc<dyn WebSocketListener> = Arc::new(TestListener::new());
        let _client = new_websocket_client("test-key".to_string(), Arc::clone(&listener));

        let listener2: Arc<dyn WebSocketListener> = Arc::new(TestListener::new());
        let _client2 = new_websocket_client_with_endpoint(
            "test-key".to_string(),
            listener2,
            WebSocketEndpoint::Stock,
        );
    }

    fn mock_client(
        server: &MockWsServer,
        listener: Arc<TestListener>,
        reconnect: Option<ReconnectConfigRecord>,
    ) -> Arc<WebSocketClient> {
        mock_client_for(server, listener, WebSocketEndpoint::Stock, reconnect)
    }

    fn mock_client_for(
        server: &MockWsServer,
        listener: Arc<TestListener>,
        endpoint: WebSocketEndpoint,
        reconnect: Option<ReconnectConfigRecord>,
    ) -> Arc<WebSocketClient> {
        WebSocketClient::new_with_full_config(
            "test-key".to_string(),
            listener,
            endpoint,
            Some(format!("ws://{}/marketdata", server.address())),
            reconnect,
            None,
            None,
            None,
        )
    }

    /// A client with `message_queue`, whose listener takes 5 ms per message.
    fn slow_queue_client(
        server: &MockWsServer,
        listener: &Arc<TestListener>,
        message_queue: MessageQueueConfigRecord,
    ) -> Arc<WebSocketClient> {
        listener.message_delay_ms.store(5, Ordering::SeqCst);
        WebSocketClient::new_with_options(
            "test-key".to_string(),
            Arc::clone(listener) as Arc<dyn WebSocketListener>,
            WebSocketEndpoint::Stock,
            Some(format!("ws://{}/marketdata", server.address())),
            None,
            None,
            None,
            None,
            Some(message_queue),
        )
    }

    /// Inject `n` frames, wait until each was delivered or dropped (none
    /// reach `on_message` once `disconnect()` is called), disconnect, and
    /// wait for the final `disconnected`.
    async fn burst_then_disconnect(
        server: &MockWsServer,
        client: &WebSocketClient,
        listener: &TestListener,
        n: usize,
    ) {
        client.connect_impl().await.expect("connect");
        for _ in 0..n {
            server
                .inject_frame(marketdata_core::models::streaming::StreamMessage::Pong { state: None })
                .await;
        }
        // Plus the `authenticated` frame.
        let settled = || {
            listener.message_count.load(Ordering::SeqCst) as u64 + client.messages_dropped_total()
                == n as u64 + 1
        };
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !settled() {
            assert!(std::time::Instant::now() < deadline, "burst did not settle");
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        client.disconnect_impl().await;
        listener.wait_for("disconnected(false)").await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn slow_listener_with_drop_newest_reports_every_drop() {
        let server = MockWsServer::start().await;
        let listener = Arc::new(TestListener::new());
        let client = slow_queue_client(
            &server,
            &listener,
            MessageQueueConfigRecord { overflow: MessageOverflowRecord::DropNewest, buffer: 8 },
        );
        assert_eq!(client.messages_dropped_total(), 0);

        burst_then_disconnect(&server, &client, &listener, 200).await;

        let total = client.messages_dropped_total();
        assert!(total > 0, "nothing dropped: {:?}", listener.events());
        // Reported before `disconnected`, adding up to the total that stays
        // readable after disconnect().
        assert_eq!(listener.messages_dropped.load(Ordering::SeqCst) as u64, total);
        let events = listener.events();
        let last_report = events.iter().rposition(|e| e.starts_with("messages_dropped("));
        let disconnected = events.iter().position(|e| e == "disconnected(false)");
        assert!(last_report < disconnected, "{events:?}");
        // The authenticated frame plus the 200 injected ones.
        assert_eq!(listener.message_count.load(Ordering::SeqCst) as u64 + total, 201);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn slow_listener_with_unbounded_gets_every_message() {
        let server = MockWsServer::start().await;
        let listener = Arc::new(TestListener::new());
        let client = slow_queue_client(
            &server,
            &listener,
            MessageQueueConfigRecord { overflow: MessageOverflowRecord::Unbounded, buffer: 8 },
        );

        burst_then_disconnect(&server, &client, &listener, 200).await;

        assert_eq!(client.messages_dropped_total(), 0);
        assert_eq!(listener.messages_dropped.load(Ordering::SeqCst), 0);
        assert_eq!(listener.message_count.load(Ordering::SeqCst), 201);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn lifecycle_events_fire_exactly_once_on_client_disconnect() {
        let server = MockWsServer::start().await;
        server.set_auth_response(serde_json::json!({
            "event": "authenticated",
            "data": {"message": "Authenticated successfully"}
        }));
        let listener = Arc::new(TestListener::new());
        let client = mock_client(&server, Arc::clone(&listener), None);

        client.connect_impl().await.expect("connect");
        client.disconnect_impl().await;
        listener.wait_for("disconnected(false)").await;
        // A second disconnect must not report the same close again.
        client.disconnect_impl().await;
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        assert_eq!(
            listener.events(),
            vec![
                "connected".to_string(),
                r#"authenticated(Some("{\"message\":\"Authenticated successfully\"}"))"#.to_string(),
                "disconnected(false)".to_string(),
            ]
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn messages_arrive_between_authenticated_and_disconnected() {
        let server = MockWsServer::start().await;
        let listener = Arc::new(TestListener::recording_messages());
        let client = mock_client(&server, Arc::clone(&listener), None);

        client.connect_impl().await.expect("connect");
        for _ in 0..3 {
            server
                .inject_frame(marketdata_core::models::streaming::StreamMessage::Pong { state: None })
                .await;
        }
        server.close(1000, "bye").await;
        listener.wait_for("disconnected(false)").await;
        client.disconnect_impl().await;

        assert_eq!(
            listener.events(),
            vec![
                "connected".to_string(),
                "authenticated(None)".to_string(),
                "message(authenticated)".to_string(),
                "message(pong)".to_string(),
                "message(pong)".to_string(),
                "message(pong)".to_string(),
                "disconnected(false)".to_string(),
            ]
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn rejected_credentials_never_reach_on_message() {
        let server = MockWsServer::start().await;
        server.set_auth_response(serde_json::json!({
            "event": "error",
            "data": {"message": "Invalid token"}
        }));
        let listener = Arc::new(TestListener::recording_messages());
        let client = mock_client(&server, Arc::clone(&listener), None);

        assert!(client.connect_impl().await.is_err());
        listener
            .wait_for(r#"unauthenticated(Some("{\"message\":\"Invalid token\"}"))"#)
            .await;
        assert_eq!(listener.message_count.load(Ordering::SeqCst), 0, "{:?}", listener.events());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn authenticated_without_data_passes_none() {
        let server = MockWsServer::start().await;
        let listener = Arc::new(TestListener::new());
        let client = mock_client(&server, Arc::clone(&listener), None);

        client.connect_impl().await.expect("connect");
        listener.wait_for("authenticated(None)").await;
        client.disconnect_impl().await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn rejected_credentials_fire_unauthenticated_without_error() {
        let server = MockWsServer::start().await;
        server.set_auth_response(serde_json::json!({
            "event": "error",
            "data": {"message": "Invalid token"}
        }));
        let listener = Arc::new(TestListener::new());
        let client = mock_client(&server, Arc::clone(&listener), None);

        assert!(client.connect_impl().await.is_err());
        listener
            .wait_for(r#"unauthenticated(Some("{\"message\":\"Invalid token\"}"))"#)
            .await;

        assert_eq!(
            listener.events(),
            vec![
                "connected".to_string(),
                r#"unauthenticated(Some("{\"message\":\"Invalid token\"}"))"#.to_string(),
            ]
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn server_close_without_reconnect_reports_final_disconnect() {
        let server = MockWsServer::start().await;
        let listener = Arc::new(TestListener::new());
        let client = mock_client(&server, Arc::clone(&listener), None);

        client.connect_impl().await.expect("connect");
        server.close(1000, "bye").await;
        listener.wait_for("disconnected(false)").await;
        client.disconnect_impl().await;
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        assert_eq!(
            listener.events(),
            vec![
                "connected".to_string(),
                "authenticated(None)".to_string(),
                "disconnected(false)".to_string(),
            ]
        );
        assert!(!client.is_connected());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn dropped_transport_with_reconnect_reports_will_reconnect() {
        let server = MockWsServer::start_with_capacity(2).await;
        let listener = Arc::new(TestListener::new());
        let client = mock_client(
            &server,
            Arc::clone(&listener),
            Some(ReconnectConfigRecord {
                max_attempts: 3,
                initial_delay_ms: 100,
                max_delay_ms: 200,
            }),
        );

        client.connect_impl().await.expect("connect");
        server.drop_transport_for(0).await;
        listener.wait_for("reconnecting(1)").await;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while listener.events().iter().filter(|e| e.starts_with("authenticated")).count() < 2 {
            assert!(std::time::Instant::now() < deadline, "no reconnect: {:?}", listener.events());
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        client.disconnect_impl().await;
        listener.wait_for("disconnected(false)").await;

        // A transport error reports `Error` before `Disconnected` (core's
        // delivery guarantee); its text is platform-dependent.
        let (errors, lifecycle): (Vec<_>, Vec<_>) =
            listener.events().into_iter().partition(|e| e.starts_with("error("));
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert_eq!(
            lifecycle,
            vec![
                "connected".to_string(),
                "authenticated(None)".to_string(),
                "disconnected(true)".to_string(),
                "reconnecting(1)".to_string(),
                "connected".to_string(),
                "authenticated(None)".to_string(),
                "disconnected(false)".to_string(),
            ]
        );
    }

    /// Poll `is_connected()` until it equals `expected`.
    async fn wait_connected(client: &WebSocketClient, expected: bool) {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while client.is_connected() != expected {
            assert!(std::time::Instant::now() < deadline, "is_connected() never became {expected}");
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn is_connected_follows_core_state_across_reconnect() {
        let server = MockWsServer::start_with_capacity(2).await;
        let listener = Arc::new(TestListener::new());
        let client = mock_client(
            &server,
            Arc::clone(&listener),
            Some(ReconnectConfigRecord {
                max_attempts: 3,
                initial_delay_ms: 500,
                max_delay_ms: 500,
            }),
        );

        client.connect_impl().await.expect("connect");
        assert!(client.is_connected(), "connected as soon as connect() returns");

        // Core's state flips on the drop itself; no listener event is awaited.
        server.drop_transport_for(0).await;
        wait_connected(&client, false).await;

        wait_connected(&client, true).await;
        assert_eq!(
            listener.events().iter().filter(|e| e.starts_with("authenticated")).count(),
            2,
            "{:?}",
            listener.events()
        );

        client.disconnect_impl().await;
        assert!(!client.is_connected(), "false as soon as disconnect() returns");
    }

    /// Poll `is_closed()` until it equals `expected`.
    async fn wait_closed(client: &WebSocketClient, expected: bool) {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while client.is_closed() != expected {
            assert!(std::time::Instant::now() < deadline, "is_closed() never became {expected}");
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn is_closed_after_server_close_without_reconnect() {
        // Used to read a flag only `disconnect()` set (#95).
        let server = MockWsServer::start().await;
        let listener = Arc::new(TestListener::new());
        let client = mock_client(&server, Arc::clone(&listener), None);
        assert!(!client.is_closed(), "not closed before connect()");

        client.connect_impl().await.expect("connect");
        assert!(!client.is_closed());

        server.drop_transport().await;
        wait_closed(&client, true).await;
        assert!(!client.is_connected());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn is_closed_is_false_while_reconnecting_and_true_after_disconnect() {
        let server = MockWsServer::start_with_capacity(2).await;
        let listener = Arc::new(TestListener::new());
        let client = mock_client(
            &server,
            Arc::clone(&listener),
            Some(ReconnectConfigRecord {
                max_attempts: 3,
                initial_delay_ms: 1000,
                max_delay_ms: 1000,
            }),
        );

        client.connect_impl().await.expect("connect");
        server.drop_transport_for(0).await;
        listener.wait_for("reconnecting(1)").await;
        assert!(!client.is_closed(), "false while reconnecting");

        client.disconnect_impl().await;
        assert!(client.is_closed(), "true as soon as disconnect() returns");
        assert!(!client.is_connected());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn is_connected_is_false_inside_on_disconnected() {
        // Core records the close before reporting it (#86): with or without
        // a reconnect to follow, the listener reads not connected.
        for reconnect in [
            None,
            Some(ReconnectConfigRecord {
                max_attempts: 3,
                initial_delay_ms: 500,
                max_delay_ms: 500,
            }),
        ] {
            let server = MockWsServer::start().await;
            let listener = Arc::new(TestListener::new());
            let will_reconnect = reconnect.is_some();
            let client = mock_client(&server, Arc::clone(&listener), reconnect);
            let _ = listener.client.set(Arc::downgrade(&client));

            client.connect_impl().await.expect("connect");
            if will_reconnect {
                server.drop_transport().await;
            } else {
                server.close(4001, "bye").await;
            }
            listener.wait_for(&format!("disconnected({will_reconnect})")).await;
            assert_eq!(*listener.connected_on_disconnect.lock().unwrap(), vec![false]);

            client.disconnect_impl().await;
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn disconnect_while_reconnecting_emits_nothing_and_stops_forwarder() {
        let server = MockWsServer::start_with_capacity(2).await;
        let listener = Arc::new(TestListener::new());
        let client = mock_client(
            &server,
            Arc::clone(&listener),
            Some(ReconnectConfigRecord {
                max_attempts: 3,
                initial_delay_ms: 1000,
                max_delay_ms: 1000,
            }),
        );

        client.connect_impl().await.expect("connect");
        server.drop_transport_for(0).await;
        listener.wait_for("reconnecting(1)").await;
        client.disconnect_impl().await;
        let before = listener.events();

        // Outlast the backoff: a reconnect that was not cancelled would have
        // reported `connected` by now.
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
        assert_eq!(listener.events(), before, "events after disconnect()");
        assert_eq!(before.last().map(String::as_str), Some("reconnecting(1)"));

        // The stream reader holds a listener clone; once it exits only this
        // test and `client` remain.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while Arc::strong_count(&listener) > 2 {
            assert!(
                std::time::Instant::now() < deadline,
                "forwarder thread still running ({} listener refs)",
                Arc::strong_count(&listener)
            );
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }

    /// A listener whose `on_message` fails the way a throwing foreign
    /// listener does: UniFFI panics with "Callback interface failure: ...".
    struct ThrowingListener {
        messages: AtomicUsize,
        connected: AtomicUsize,
        errors: Mutex<Vec<ErrorInfo>>,
        throw_in_on_error: bool,
    }

    impl ThrowingListener {
        fn new(throw_in_on_error: bool) -> Self {
            Self {
                messages: AtomicUsize::new(0),
                connected: AtomicUsize::new(0),
                errors: Mutex::new(Vec::new()),
                throw_in_on_error,
            }
        }
    }

    impl WebSocketListener for ThrowingListener {
        fn on_connected(&self) {
            self.connected.fetch_add(1, Ordering::SeqCst);
        }
        fn on_authenticated(&self, _data_json: Option<String>) {}
        fn on_unauthenticated(&self, _data_json: Option<String>) {}
        fn on_disconnected(&self, _will_reconnect: bool) {}
        fn on_message(&self, _message: StreamMessage) {
            self.messages.fetch_add(1, Ordering::SeqCst);
            panic!("Callback interface failure: java.lang.RuntimeException: boom");
        }
        fn on_error(&self, error: ErrorInfo) {
            self.errors.lock().unwrap().push(error);
            if self.throw_in_on_error {
                panic!("Callback interface failure: on_error boom");
            }
        }
        fn on_reconnecting(&self, _attempt: u32) {}
        fn on_reconnect_failed(&self, _attempts: u32) {}
        fn on_messages_dropped(&self, _count: u64) {}
    }

    fn stream_message() -> StreamMessage {
        let message: marketdata_core::WebSocketMessage =
            serde_json::from_str(r#"{"event":"data","data":{"price":1},"channel":"trades"}"#).unwrap();
        StreamMessage::from(message)
    }

    #[test]
    fn failing_listener_call_is_reported_through_on_error_and_later_calls_still_run() {
        let listener = ThrowingListener::new(false);
        let mut calls = ListenerCalls::new(&listener, CallbackFailures::default());

        for _ in 0..3 {
            calls.call("on_message", |l| l.on_message(stream_message()));
        }
        assert!(forward_event(ConnectionEvent::Connected, &mut calls));

        assert_eq!(listener.messages.load(Ordering::SeqCst), 3);
        assert_eq!(listener.connected.load(Ordering::SeqCst), 1);
        // Three failures within a second: the first is reported.
        let errors = listener.errors.lock().unwrap();
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert_eq!(errors[0].code, marketdata_core::error_code::CALLBACK_FAILED);
        assert!(matches!(errors[0].source_kind, crate::errors::ErrorSourceKind::Client));
        assert_eq!(
            errors[0].message,
            "Listener on_message failed: java.lang.RuntimeException: boom (1 in the last 1s)"
        );
        assert_eq!(calls.failures.lock().unwrap().pending(), 2);
    }

    #[test]
    fn throttle_is_shared_by_the_readers_of_one_client() {
        let listener = ThrowingListener::new(false);
        let failures = CallbackFailures::default();

        // A new connection's reader does not start a new throttle interval.
        ListenerCalls::new(&listener, Arc::clone(&failures))
            .call("on_message", |l| l.on_message(stream_message()));
        ListenerCalls::new(&listener, Arc::clone(&failures))
            .call("on_message", |l| l.on_message(stream_message()));

        assert_eq!(listener.errors.lock().unwrap().len(), 1);
        assert_eq!(failures.lock().unwrap().pending(), 1);
    }

    #[test]
    fn failing_on_error_is_not_re_reported() {
        let listener = ThrowingListener::new(true);
        let mut calls = ListenerCalls::new(&listener, CallbackFailures::default());

        calls.call("on_message", |l| l.on_message(stream_message()));
        let info = marketdata_core::ErrorInfo::new(
            marketdata_core::error_code::CONNECTION,
            marketdata_core::ErrorKind::Network,
            "down",
        );
        assert!(forward_event(ConnectionEvent::Error(info), &mut calls));

        // One report of the on_message failure, one SDK error; neither
        // failing on_error call led to another.
        let codes: Vec<i32> = listener.errors.lock().unwrap().iter().map(|e| e.code).collect();
        assert_eq!(
            codes,
            vec![marketdata_core::error_code::CALLBACK_FAILED, marketdata_core::error_code::CONNECTION]
        );
    }

    fn client_with_credentials(
        api_key: Option<&str>,
        bearer_token: Option<&str>,
        sdk_token: Option<&str>,
    ) -> Result<Arc<WebSocketClient>, MarketDataError> {
        WebSocketClient::new_with_credentials(
            CredentialsRecord {
                api_key: api_key.map(String::from),
                bearer_token: bearer_token.map(String::from),
                sdk_token: sdk_token.map(String::from),
            },
            Arc::new(TestListener::new()),
            WebSocketEndpoint::Stock,
            None,
            None,
            None,
            None,
            None,
            None,
        )
    }

    #[test]
    fn new_with_credentials_sends_each_kind_in_its_own_field() {
        // #91: every kind used to go out as `apikey`.
        let auth_data = |client: Arc<WebSocketClient>| serde_json::to_value(&client.auth).unwrap();
        assert_eq!(
            auth_data(client_with_credentials(Some("k"), None, None).unwrap()),
            serde_json::json!({ "apikey": "k" })
        );
        assert_eq!(
            auth_data(client_with_credentials(None, Some("t"), None).unwrap()),
            serde_json::json!({ "token": "t" })
        );
        assert_eq!(
            auth_data(client_with_credentials(Some("  "), None, Some("s")).unwrap()),
            serde_json::json!({ "sdkToken": "s" })
        );
    }

    #[test]
    fn credentials_record_debug_redacts_secrets() {
        let credentials = CredentialsRecord {
            api_key: None,
            bearer_token: Some("secret-bearer".into()),
            sdk_token: Some("secret-sdk".into()),
        };
        let printed = format!("{credentials:?}");
        assert_eq!(
            printed,
            "CredentialsRecord { api_key: None, bearer_token: Some(***), sdk_token: Some(***) }"
        );
        assert!(!printed.contains("secret"), "{printed}");
    }

    #[test]
    fn new_with_credentials_rejects_none_or_several() {
        for (api_key, bearer_token, sdk_token) in
            [(None, None, None), (Some(" "), None, None), (Some("k"), Some("t"), None)]
        {
            match client_with_credentials(api_key, bearer_token, sdk_token) {
                Err(MarketDataError::ConfigError { .. }) => {}
                Err(other) => panic!("expected ConfigError, got {other:?}"),
                Ok(_) => panic!("expected ConfigError for {api_key:?}/{bearer_token:?}/{sdk_token:?}"),
            }
        }
    }

    fn assert_invalid_parameter(result: Result<(), MarketDataError>, expected: &str) {
        match result {
            Err(MarketDataError::ApiError { msg, info }) => {
                assert_eq!(info.code, marketdata_core::error_code::INVALID_PARAMETER);
                assert_eq!(msg, expected);
            }
            other => panic!("expected INVALID_PARAMETER, got {other:?}"),
        }
    }

    fn assert_unknown_channel(result: Result<(), MarketDataError>) {
        assert_invalid_parameter(
            result,
            "Invalid parameter 'channel': unknown channel 'trade'. \
             Valid channels: trades, candles, books, aggregates, indices",
        );
    }

    const FUTOPT_INDICES: &str = "Invalid parameter 'channel': unknown channel 'indices'. \
                                  Valid channels: trades, candles, books, aggregates";

    #[cfg(not(feature = "cpp"))]
    const STOCK_AFTER_HOURS: &str =
        "Invalid parameter 'afterHours': only supported on the FutOpt endpoint";

    fn futopt_client() -> Arc<WebSocketClient> {
        WebSocketClient::new_with_endpoint(
            "test-key".to_string(),
            Arc::new(TestListener::new()),
            WebSocketEndpoint::FutOpt,
        )
    }

    #[cfg(not(feature = "cpp"))]
    fn assert_not_connected(result: Result<(), MarketDataError>) {
        match result {
            Err(MarketDataError::WebSocketError { msg, .. }) if msg == "Not connected" => {}
            other => panic!("expected \"Not connected\", got {other:?}"),
        }
    }

    #[cfg(not(feature = "cpp"))]
    #[tokio::test(flavor = "multi_thread")]
    async fn subscribe_rejects_unknown_channel_before_connecting() {
        let client = WebSocketClient::new("test-key".to_string(), Arc::new(TestListener::new()));
        assert_unknown_channel(client.subscribe("trade".into(), "2330".into(), None).await);
        assert_unknown_channel(client.unsubscribe("trade".into(), "2330".into(), None).await);
        // The channel is checked before after-hours.
        assert_unknown_channel(client.subscribe("trade".into(), "2330".into(), Some(true)).await);
        // A known name, in any case, gets past the check to "not connected".
        assert_not_connected(client.subscribe("Trades".into(), "2330".into(), None).await);
        assert_not_connected(client.unsubscribe("Trades".into(), "2330".into(), None).await);
    }

    #[cfg(not(feature = "cpp"))]
    #[tokio::test(flavor = "multi_thread")]
    async fn futopt_endpoint_parses_futopt_channels_before_connecting() {
        let client = futopt_client();
        assert_invalid_parameter(
            client.subscribe("indices".into(), "TXFE6".into(), None).await,
            FUTOPT_INDICES,
        );
        assert_invalid_parameter(
            client.unsubscribe("indices".into(), "TXFE6".into(), Some(true)).await,
            FUTOPT_INDICES,
        );
        assert_not_connected(client.subscribe("Books".into(), "TXFE6".into(), Some(true)).await);
        assert_not_connected(client.unsubscribe("books".into(), "TXFE6".into(), None).await);
    }

    #[cfg(not(feature = "cpp"))]
    #[tokio::test(flavor = "multi_thread")]
    async fn stock_endpoint_rejects_after_hours_before_connecting() {
        let client = WebSocketClient::new("test-key".to_string(), Arc::new(TestListener::new()));
        for after_hours in [true, false] {
            assert_invalid_parameter(
                client.subscribe("trades".into(), "2330".into(), Some(after_hours)).await,
                STOCK_AFTER_HOURS,
            );
            assert_invalid_parameter(
                client.unsubscribe("trades".into(), "2330".into(), Some(after_hours)).await,
                STOCK_AFTER_HOURS,
            );
        }
    }

    #[cfg(not(feature = "cpp"))]
    #[tokio::test(flavor = "multi_thread")]
    async fn futopt_after_hours_subscription_is_unsubscribed_with_after_hours() {
        let server = MockWsServer::start().await;
        let client = mock_client_for(
            &server,
            Arc::new(TestListener::new()),
            WebSocketEndpoint::FutOpt,
            None,
        );
        client.connect_impl().await.expect("connect");
        let core = client.client().expect("connected");

        client.subscribe("books".into(), "TXFE6".into(), Some(true)).await.expect("subscribe");
        client.subscribe("books".into(), "TXFE6".into(), None).await.expect("subscribe");
        let mut keys: Vec<_> = core.subscriptions().iter().map(|sub| sub.key()).collect();
        keys.sort();
        assert_eq!(keys, ["books:TXFE6", "books:TXFE6:afterhours"]);

        client.unsubscribe("books".into(), "TXFE6".into(), Some(true)).await.expect("unsubscribe");
        let keys: Vec<_> = core.subscriptions().iter().map(|sub| sub.key()).collect();
        assert_eq!(keys, ["books:TXFE6"]);

        client.unsubscribe("Books".into(), "TXFE6".into(), None).await.expect("unsubscribe");
        assert_eq!(core.subscription_count(), 0);
        client.disconnect_impl().await;
    }

    #[cfg(feature = "cpp")]
    #[test]
    fn subscribe_sync_rejects_unknown_channel_before_connecting() {
        let client = WebSocketClient::new("test-key".to_string(), Arc::new(TestListener::new()));
        assert_unknown_channel(client.subscribe_sync("trade".into(), "2330".into()));
        assert_unknown_channel(client.unsubscribe_sync("trade".into(), "2330".into()));
    }

    #[cfg(feature = "cpp")]
    #[test]
    fn sync_futopt_endpoint_parses_futopt_channels() {
        let client = futopt_client();
        assert_invalid_parameter(client.subscribe_sync("indices".into(), "TXFE6".into()), FUTOPT_INDICES);
        assert_invalid_parameter(client.unsubscribe_sync("indices".into(), "TXFE6".into()), FUTOPT_INDICES);
    }
}
