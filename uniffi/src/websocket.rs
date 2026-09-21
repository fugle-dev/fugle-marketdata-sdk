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
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
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

    /// Called when the server rejects the credentials: it answered the auth
    /// frame with an `error` of code 1000. On `connect()` the call also
    /// fails with an auth error; no `on_error` is emitted for the rejection.
    /// During an auto-reconnect, `on_reconnect_failed` follows at once: the
    /// same credentials would be rejected again, so the client stops and
    /// stays closed (#201). An auth-phase `error` with any other code (1011
    /// auth service unavailable, 1004 no auth request received) is not a
    /// rejection: it is reported to `on_error` (code 2001) and a reconnect
    /// goes on.
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
    ///
    /// Also carries one warning, code 3006 (`RECONNECT_CONFLICT`), at most
    /// once per client: `connect()` was called less than 30 seconds after
    /// `disconnect()` closed a connection that automatic reconnect had
    /// restored less than 30 seconds before, which is what code that also
    /// reconnects on its own does (#226, #242). It comes from that
    /// `connect()`, which goes ahead; the message says how to resolve it.
    fn on_error(&self, error: ErrorInfo);

    /// Called when a reconnection attempt starts
    fn on_reconnecting(&self, attempt: u32);

    /// Called when the reconnect gives up: all attempts are exhausted, or an
    /// attempt's credentials were rejected (`on_unauthenticated` precedes
    /// it, #201). Terminal: no further lifecycle callbacks follow for this
    /// connection.
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
/// Every field's zero value means "use default", so a zero-initialized
/// record (C++ `ReconnectConfigRecord{}`, a Go `ReconnectConfigRecord{}`
/// literal) is the full default: auto-reconnect on with the core delays
/// (#158, #161). Omitting the record gives the same result.
#[derive(Debug, Clone, Default, uniffi::Record)]
pub struct ReconnectConfigRecord {
    /// Whether auto-reconnect is active; `false` turns it off. Unset (the
    /// zero value) takes the core default, which is on.
    #[uniffi(default = None)]
    pub enabled: Option<bool>,
    /// Maximum reconnection attempts; 0 means unlimited (the default)
    pub max_attempts: u32,
    /// Initial reconnection delay in milliseconds (default: 1000, min: 100)
    pub initial_delay_ms: u64,
    /// Maximum reconnection delay in milliseconds (default: 60000)
    pub max_delay_ms: u64,
}

impl ReconnectConfigRecord {
    /// Core's config, validated by core (#153): an `initial_delay_ms` below
    /// `MIN_INITIAL_DELAY_MS` or a `max_delay_ms` below `initial_delay_ms`
    /// is a `ConfigError` (1004), as in Node and Python.
    fn to_core(&self) -> Result<marketdata_core::ReconnectionConfig, marketdata_core::MarketDataError> {
        let default = marketdata_core::ReconnectionConfig::default();
        // 0 means "unset" across the FFI boundary, so it takes the default
        // *before* validation: a zero record must stay the full default
        // (#158, #161), not fail the delay floor.
        let ms_or = |ms: u64, fallback| {
            if ms > 0 { std::time::Duration::from_millis(ms) } else { fallback }
        };
        let mut config = marketdata_core::ReconnectionConfig::new(
            // 0 is both "unset" and "unlimited": the core default is unlimited.
            self.max_attempts,
            ms_or(self.initial_delay_ms, default.initial_delay),
            ms_or(self.max_delay_ms, default.max_delay),
        )?;
        // Validated even when disabled, like the other bindings, so a bad
        // value surfaces before auto-reconnect is switched on.
        config.enabled = self.enabled.unwrap_or(default.enabled);
        Ok(config)
    }
}

/// A reconnect record converted for a constructor that cannot fail: a
/// config error is kept and returned by `connect()`.
fn reconnect_or_deferred(
    record: Option<ReconnectConfigRecord>,
) -> Result<Option<marketdata_core::ReconnectionConfig>, String> {
    record.map(|r| r.to_core()).transpose().map_err(config_error_message)
}

/// Health check configuration record for FFI
///
/// Every field's zero value means "use default", so a zero-initialized
/// record (C++ `HealthCheckConfigRecord{}`, a Go `HealthCheckConfigRecord{}`
/// literal) is the full default: detection on, no probe, 35 s timeout
/// (#158, #161).
#[derive(Debug, Clone, Default, uniffi::Record)]
pub struct HealthCheckConfigRecord {
    /// Whether liveness detection is active; `false` turns it off. Unset
    /// (the zero value) takes the core default, which is on.
    #[uniffi(default = None)]
    pub enabled: Option<bool>,
    /// Maximum allowed gap between inbound frames before declaring the
    /// connection dead, in milliseconds. Default 35000; floor 5000.
    /// Pass 0 to use the default. Does not apply when `probe_enabled` is
    /// true.
    pub heartbeat_timeout_ms: u64,
    /// Confirm a silent connection with a ping before declaring it dead
    /// (default: false). After `idle_probe_after_ms` of silence one ping is
    /// sent; if nothing arrives within `probe_timeout_ms` the connection is
    /// declared dead.
    #[uniffi(default = false)]
    pub probe_enabled: bool,
    /// Silence before the probe, in milliseconds. Default 30000 (the
    /// server's heartbeat period); floor 5000. Pass 0 to use the default.
    #[uniffi(default = 0)]
    pub idle_probe_after_ms: u64,
    /// Wait for any inbound frame after the probe, in milliseconds.
    /// Default 5000; floor 1000. Pass 0 to use the default.
    #[uniffi(default = 0)]
    pub probe_timeout_ms: u64,
}

impl HealthCheckConfigRecord {
    /// Core's config, validated by core: a value below its floor is a
    /// `ConfigError` (1004).
    fn to_core(&self) -> Result<marketdata_core::HealthCheckConfig, marketdata_core::MarketDataError> {
        // 0 means "unset" across the FFI boundary — there is no Option<u64>
        // that reads naturally in C#/Go/Java, so fall back to the default.
        // A bool has no spare value to mean "unset", so `enabled` is an
        // Option instead.
        let ms = |ms: u64| (ms > 0).then(|| std::time::Duration::from_millis(ms));
        let enabled = self
            .enabled
            .unwrap_or_else(|| marketdata_core::HealthCheckConfig::default().enabled);
        marketdata_core::HealthCheckConfig::from_parts(
            enabled,
            ms(self.heartbeat_timeout_ms),
            self.probe_enabled,
            ms(self.idle_probe_after_ms),
            ms(self.probe_timeout_ms),
        )
    }
}

/// A health check record converted for a constructor that cannot fail: a
/// config error is kept and returned by `connect()`.
fn health_check_or_deferred(
    record: Option<HealthCheckConfigRecord>,
) -> Result<Option<marketdata_core::HealthCheckConfig>, String> {
    record.map(|r| r.to_core()).transpose().map_err(config_error_message)
}

/// The message a deferred config error is re-raised with by `connect()`,
/// without a doubled "Configuration error:" prefix.
fn config_error_message(e: marketdata_core::MarketDataError) -> String {
    match e {
        marketdata_core::MarketDataError::ConfigError(message) => message,
        other => other.to_string(),
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

/// Connection configuration record for FFI: the timeouts of the connection
/// itself (#199).
///
/// Every field's zero value means "use default", so a zero-initialized
/// record (C++ `ConnectionConfigRecord{}`, a Go `ConnectionConfigRecord{}`
/// literal) is the full default. Omitting the record gives the same result.
#[derive(Debug, Clone, Default, uniffi::Record)]
pub struct ConnectionConfigRecord {
    /// How long the auth handshake may take once the WebSocket is open, in
    /// milliseconds: from the auth frame being sent until the server's
    /// verdict. Default 10000. Pass 0 to use the default. Applies to the
    /// first `connect()` and to every reconnect; elapsing it fails the
    /// attempt with a `TimeoutError` (3001). The server itself allows 60 s.
    #[uniffi(default = 0)]
    pub auth_timeout_ms: u64,
}

impl ConnectionConfigRecord {
    fn apply(&self, config: &mut marketdata_core::ConnectionConfig) {
        // 0 means "unset" across the FFI boundary, so it keeps the core
        // default; any other value is valid (there is no floor).
        if self.auth_timeout_ms > 0 {
            config.auth_timeout = std::time::Duration::from_millis(self.auth_timeout_ms);
        }
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
    /// The connection and what `connect()` / `disconnect()` hand over to
    /// each other, under one lock (#126, #121).
    connection: std::sync::Mutex<ConnectionSlot>,
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
    /// Reconnect config, or the message of the `ConfigError` its record
    /// raised in a constructor that cannot fail; `connect()` returns it.
    reconnect_config: Result<Option<marketdata_core::ReconnectionConfig>, String>,
    /// Health check config, likewise.
    health_check_config: Result<Option<marketdata_core::HealthCheckConfig>, String>,
    tls_config: Option<marketdata_core::TlsConfig>,
    message_queue: Option<MessageQueueConfigRecord>,
    connection_config: Option<ConnectionConfigRecord>,
    /// Dropped-message count of the current or last connection; outlives the
    /// core client, which `disconnect()` drops.
    messages_dropped: std::sync::Mutex<Option<marketdata_core::MessagesDroppedHandle>>,
    /// Throttles the reports of failed listener calls (#83) across the
    /// client's connections, like the Node, Python, C# and Java bindings.
    callback_failures: CallbackFailures,
    /// Carries a close made soon after an automatic reconnect to the next
    /// `connect()`, whose core client is a new one, for the 3006 warning;
    /// core gives it once per handle (#226, #242).
    reconnect_conflict: marketdata_core::ReconnectConflictHandle,
    /// Held for the duration of each `connect()`, so a concurrent one is
    /// refused rather than opening a second connection (#119).
    connect_gate: tokio::sync::Mutex<()>,
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
        reconnect_config: Result<Option<marketdata_core::ReconnectionConfig>, String>,
        health_check_config: Result<Option<marketdata_core::HealthCheckConfig>, String>,
        base_url: Option<String>,
        tls_config: Option<marketdata_core::TlsConfig>,
        version: StreamingVersionRecord,
        message_queue: Option<MessageQueueConfigRecord>,
        connection_config: Option<ConnectionConfigRecord>,
    ) -> Arc<Self> {
        Arc::new(Self {
            connection: std::sync::Mutex::new(ConnectionSlot::default()),
            listener,
            auth,
            base_url,
            version,
            endpoint,
            state: std::sync::Mutex::new(None),
            reconnect_config,
            health_check_config,
            tls_config,
            message_queue,
            connection_config,
            messages_dropped: std::sync::Mutex::new(None),
            callback_failures: CallbackFailures::default(),
            reconnect_conflict: marketdata_core::ReconnectConflictHandle::default(),
            connect_gate: tokio::sync::Mutex::new(()),
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
        Self::new_internal(AuthRequest::with_api_key(api_key), listener, WebSocketEndpoint::Stock, Ok(None), Ok(None), None, None, Default::default(), None, None)
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
        Self::new_internal(AuthRequest::with_api_key(api_key), listener, endpoint, Ok(None), Ok(None), None, None, Default::default(), None, None)
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
            reconnect_or_deferred(reconnect_config),
            health_check_or_deferred(health_check_config),
            None,
            None,
            Default::default(),
            None,
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
            reconnect_or_deferred(reconnect_config),
            health_check_or_deferred(health_check_config),
            Some(base_url),
            None,
            Default::default(),
            None,
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
            reconnect_or_deferred(reconnect_config),
            health_check_or_deferred(health_check_config),
            base_url,
            tls.map(|t| t.to_core()),
            version.unwrap_or_default(),
            None,
            None,
        )
    }

    /// Create a new WebSocket client with full configuration plus the
    /// message queue and connection settings.
    ///
    /// Same as `new_with_full_config`, with `message_queue` choosing what
    /// happens while `on_message` falls behind (None for the defaults:
    /// `DropNewest`, 4096 messages) and `connection` setting the
    /// connection's own timeouts (None for the defaults: 10 s auth
    /// timeout).
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
    /// * `connection` - Optional connection configuration (auth timeout)
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
        connection: Option<ConnectionConfigRecord>,
    ) -> Arc<Self> {
        Self::new_internal(
            AuthRequest::with_api_key(api_key),
            listener,
            endpoint,
            reconnect_or_deferred(reconnect_config),
            health_check_or_deferred(health_check_config),
            base_url,
            tls.map(|t| t.to_core()),
            version.unwrap_or_default(),
            message_queue,
            connection,
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
        connection: Option<ConnectionConfigRecord>,
    ) -> Result<Arc<Self>, MarketDataError> {
        let CredentialsRecord { api_key, bearer_token, sdk_token } = credentials;
        let auth = marketdata_core::Auth::from_credentials(api_key, bearer_token, sdk_token)?;
        let reconnect_config = reconnect_config.map(|c| c.to_core()).transpose()?;
        let health_check_config = health_check_config.map(|c| c.to_core()).transpose()?;
        Ok(Self::new_internal(
            AuthRequest::from(auth),
            listener,
            endpoint,
            Ok(reconnect_config),
            Ok(health_check_config),
            base_url,
            tls.map(|t| t.to_core()),
            version.unwrap_or_default(),
            message_queue,
            connection,
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
    /// Connect to the WebSocket server and authenticate.
    ///
    /// Refused with code 2011 (`ALREADY_CONNECTED`) while connected or while
    /// another `connect()` is in progress.
    ///
    /// During an automatic reconnect it opens no connection of its own: it
    /// waits for that reconnect and returns once the connection is back and
    /// the subscriptions are re-sent, so a `subscribe()` afterwards follows
    /// them. The wait fails with 2010 (`ClientClosed`) if `disconnect()` is
    /// called, 3005 (`RECONNECT_FAILED`) if the reconnect runs out of
    /// attempts, and `AuthError` (2002) if its credentials are rejected.
    /// Called from a listener method, it holds up the listener until the
    /// reconnect ends.
    pub async fn connect(&self) -> Result<(), MarketDataError> {
        self.connect_impl().await
    }

    /// Subscribe to a channel for one or more symbols.
    ///
    /// One symbol is sent as `symbol`, several as `symbols` in one frame;
    /// each symbol is its own subscription afterwards. An empty list is
    /// 1005 `INVALID_PARAMETER`.
    ///
    /// `opts` selects the session: `intraday_odd_lot` is Stock only and
    /// `after_hours` is FutOpt only; setting either on the other endpoint,
    /// to any value, is 1005 `INVALID_PARAMETER`.
    #[uniffi::method(default(opts = None))]
    pub async fn subscribe(
        &self,
        channel: String,
        symbols: Vec<String>,
        opts: Option<SubscribeOptions>,
    ) -> Result<(), MarketDataError> {
        let sub = self.subscription(&channel, symbols, opts)?;
        self.subscribe_impl(sub).await
    }

    /// Unsubscribe from a channel for one or more symbols.
    ///
    /// Pass the same options as the `subscribe` call: an odd-lot or
    /// after-hours subscription is a separate subscription from the regular
    /// one.
    #[uniffi::method(default(opts = None))]
    pub async fn unsubscribe(
        &self,
        channel: String,
        symbols: Vec<String>,
        opts: Option<SubscribeOptions>,
    ) -> Result<(), MarketDataError> {
        let sub = self.subscription(&channel, symbols, opts)?;
        self.unsubscribe_impl(sub).await
    }

    /// Unsubscribe by the ids the server issued in its `subscribed` messages.
    ///
    /// Removes the subscriptions those ids name, so a reconnect does not
    /// restore them. An empty list is 1005 `INVALID_PARAMETER`.
    pub async fn unsubscribe_ids(&self, ids: Vec<String>) -> Result<(), MarketDataError> {
        let ids = non_empty_ids(ids)?;
        self.unsubscribe_ids_impl(ids).await
    }

    pub async fn ping(&self, state: Option<String>) -> Result<(), MarketDataError> {
        self.ping_impl(state).await
    }

    /// Measure the round trip to the server: send a ping, wait for its pong,
    /// and return the time between the two in milliseconds.
    ///
    /// Unlike `ping()` (fire and forget, pong delivered to `on_message`),
    /// this waits for the answer, and its pong is not delivered. Works
    /// whether or not `probe_enabled` is set, and sends nothing in the
    /// background. `timeout_ms` defaults to 5000 when `None`.
    ///
    /// Errors: `ClientClosed` (2010) when not connected, `ConnectionError`
    /// (2001) when the connection closes before the pong, `TimeoutError`
    /// (3001) when no pong arrives within `timeout_ms`, and
    /// `InvalidParameter` (1005) for a `timeout_ms` of 0.
    pub async fn measure_latency(&self, timeout_ms: Option<u64>) -> Result<f64, MarketDataError> {
        self.measure_latency_impl(timeout_ms).await
    }

    pub async fn query_subscriptions(&self) -> Result<(), MarketDataError> {
        self.query_subscriptions_impl().await
    }

    /// Disconnect, returning once the listener has handled the connection's
    /// remaining events, `on_disconnected` included.
    ///
    /// There is no timeout on that wait: a listener method that blocks keeps
    /// `disconnect()` waiting for as long as it does.
    ///
    /// Called from a listener method, it returns without that wait: those
    /// events are delivered on the thread running the method, after it
    /// returns.
    pub async fn disconnect(&self) {
        self.disconnect_impl().await
    }
}

impl WebSocketClient {
    /// Connect to the WebSocket server (implementation); see `connect`.
    /// `connect_sync` needs the path, so the C++ build has no other caller.
    #[cfg(any(not(feature = "cpp"), test))]
    async fn connect_impl(&self) -> Result<(), MarketDataError> {
        self.connect_with_path().await.1
    }

    /// [`connect_impl`](Self::connect_impl), and whether it opened a
    /// connection of its own.
    ///
    /// Decided before anything of the stored connection is replaced
    /// (#119, #230): none stored, or a closed one, opens a new connection;
    /// one the listener was last handed as up is refused with 2011; any other
    /// is reconnecting, and this waits on core's `wait_connected()` with the
    /// connect gate released, so any number of calls wait together, as on
    /// core's client. Not core's `is_active()`: between reconnect attempts
    /// the state is `Disconnected`, and a connect let through there would
    /// leave the old reconnect loop running beside a new connection.
    ///
    /// A wait that ends with nothing left to wait for — the client closed
    /// without the reconnect giving up, or core has no reconnect under way
    /// (`ConnectionError`) — claims the gate again and opens a new
    /// connection in place of that one.
    async fn connect_with_path(&self) -> (ConnectPath, Result<(), MarketDataError>) {
        use marketdata_core::MarketDataError as CoreError;
        // Read before the first await, as in `disconnect_impl`.
        let caller = std::thread::current().id();
        let already = || Err(CoreError::AlreadyConnected.into());
        let mut gave_up: Option<Arc<CoreWebSocketClient>> = None;
        loop {
            // Held until a fresh connect ends; released for a join.
            let Ok(claim) = self.connect_gate.try_lock() else {
                return (ConnectPath::Refused, already());
            };
            let stored = lock_connection(&self.connection).current.clone();
            let join = match stored {
                Some(c) if c.ws.is_closed_sync() => None,
                Some(c) if gave_up.as_ref().is_some_and(|g| Arc::ptr_eq(g, &c.ws)) => None,
                Some(c) if c.delivered.is_authenticated() => {
                    return (ConnectPath::Refused, already());
                }
                Some(c) => Some(c.ws),
                None => None,
            };
            let Some(ws) = join else {
                let result = self.open_connection(caller).await;
                drop(claim);
                return (ConnectPath::Opened, result);
            };
            drop(claim);
            match ws.wait_connected().await {
                Ok(()) => return (ConnectPath::Joined, Ok(())),
                Err(CoreError::ClientClosed | CoreError::ConnectionError { .. }) => gave_up = Some(ws),
                Err(e) => return (ConnectPath::Joined, Err(e.into())),
            }
        }
    }

    /// Open, authenticate and store a new connection, for a `connect()` that
    /// holds the connect gate and was first polled on `caller`.
    async fn open_connection(&self, caller: std::thread::ThreadId) -> Result<(), MarketDataError> {
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
        if let Some(ref connection) = self.connection_config {
            connection.apply(&mut config);
        }

        let reconnect_config = self
            .reconnect_config
            .clone()
            .map_err(marketdata_core::MarketDataError::ConfigError)?;
        let health_check_config = self
            .health_check_config
            .clone()
            .map_err(marketdata_core::MarketDataError::ConfigError)?;
        // Create core WebSocket client with optional reconnection/health-check config
        let core_ws = if let (Some(rc), Some(hc)) = (&reconnect_config, &health_check_config) {
            CoreWebSocketClient::with_full_config(config, rc.clone(), hc.clone())
        } else if let Some(rc) = &reconnect_config {
            CoreWebSocketClient::with_full_config(
                config,
                rc.clone(),
                marketdata_core::HealthCheckConfig::default(),
            )
        } else if let Some(hc) = &health_check_config {
            CoreWebSocketClient::with_full_config(
                config,
                marketdata_core::ReconnectionConfig::default(),
                hc.clone(),
            )
        } else {
            CoreWebSocketClient::new(config)
        };

        *self
            .messages_dropped
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(core_ws.messages_dropped_handle());
        *lock_state(&self.state) = Some(core_ws.state_handle());
        // Before connect(): it warns about the previous connection's close.
        core_ws.use_reconnect_conflict_handle(&self.reconnect_conflict);

        // Forward core's stream: messages and lifecycle events in the order
        // core produced them (#68). The thread starts before `connect()` so
        // events of a failed attempt (`Unauthenticated`, `Error`) are still
        // delivered.
        // Its own `stopping` per connection, so a reader still draining a
        // previous connection never sees a later connection's flag.
        let stopping = Arc::new(AtomicBool::new(false));
        let delivered = Delivered::default();
        let reader = spawn_stream_reader(
            core_ws.stream_receiver(),
            Arc::clone(&self.listener),
            Arc::clone(&stopping),
            delivered.clone(),
            Arc::clone(&self.callback_failures),
        );
        {
            // A `disconnect()` from here on is for this connection: it has a
            // reader to wait for, and the request is this attempt's, not one
            // left over from before it.
            let mut slot = lock_connection(&self.connection);
            slot.disconnect_requested = false;
            slot.pending = reader.clone();
        }

        // Connect to server
        let connected = core_ws.connect().await;
        let connection = Connection { ws: Arc::new(core_ws), stopping, reader, delivered };

        // Stored in one step, or given up in one step: a `disconnect()` that
        // ran during the handshake found no connection to close and asked for
        // this one instead (#121).
        let cancelled = {
            let mut slot = lock_connection(&self.connection);
            slot.pending = None;
            let cancelled = std::mem::take(&mut slot.disconnect_requested);
            match (connected.is_ok(), cancelled) {
                (true, false) => {
                    // Under the lock, so a `connect()` sees it with the
                    // connection.
                    connection.delivered.connect_succeeded();
                    slot.current = Some(connection.clone());
                }
                // What that `disconnect()` waits for, once it stops waiting
                // for the handshake's reader.
                (true, true) => slot.closing = connection.reader.clone(),
                (false, _) => {}
            }
            cancelled
        };

        connected?;
        if cancelled {
            // The same close as `disconnect()`, so `on_disconnected` is
            // delivered and the reader ends.
            self.close(connection, caller).await;
            return Err(marketdata_core::MarketDataError::ConnectionAborted.into());
        }

        Ok(())
    }

    /// Close `connection` and wait for its listener calls, unless `caller` is
    /// the thread running them (#126).
    async fn close(&self, connection: Connection, caller: std::thread::ThreadId) {
        let Connection { ws, stopping, reader, .. } = connection;
        // No messages are delivered once `disconnect()` has been called; the
        // connection's remaining events still are.
        stopping.store(true, Ordering::SeqCst);

        // `on_disconnected` comes from core's `Disconnected` event, which
        // `ws.disconnect()` emits.
        let _ = ws.disconnect().await;
        // Mid-reconnect core emits no `Disconnected`: the reader ends when
        // the stream closes, once the client is dropped.
        drop(ws);

        Self::wait_for(reader, caller).await;
    }

    /// Wait until `reader` has ended, unless `caller` is its own thread.
    async fn wait_for(reader: Option<StreamReader>, caller: std::thread::ThreadId) {
        if let Some(reader) = reader {
            if reader.thread != caller {
                reader.finished().await;
            }
        }
    }

    /// The subscription for this client's endpoint: FutOpt channels and
    /// `after_hours` on FutOpt, stock channels and `intraday_odd_lot` on
    /// Stock.
    ///
    /// Callers build it before checking the connection, so an unknown
    /// channel, an empty symbol list, or the other endpoint's session option
    /// is 1005 `INVALID_PARAMETER` whether or not the client is connected.
    /// The checks run in that order.
    ///
    /// Core normalises the symbols: a one-element list is sent as `symbol`,
    /// as before, and a longer one as `symbols` in a single frame.
    fn subscription(
        &self,
        channel: &str,
        symbols: Vec<String>,
        opts: Option<SubscribeOptions>,
    ) -> Result<Subscription, MarketDataError> {
        let opts = opts.unwrap_or_default();
        let invalid = |name: &str, reason: &str| {
            Err(marketdata_core::MarketDataError::InvalidParameter {
                name: name.to_string(),
                reason: reason.to_string(),
            }
            .into())
        };
        match self.endpoint {
            WebSocketEndpoint::Stock => {
                // The channel first, as on the FutOpt endpoint.
                let channel: marketdata_core::Channel = channel.parse()?;
                let symbols = non_empty_symbols(symbols)?;
                if opts.after_hours.is_some() {
                    return invalid("afterHours", "only supported on the FutOpt endpoint");
                }
                Ok(Subscription::Stock(
                    marketdata_core::StockSubscription::new(channel, symbols)
                        .with_odd_lot(opts.intraday_odd_lot.unwrap_or(false)),
                ))
            }
            WebSocketEndpoint::FutOpt => {
                let channel: marketdata_core::FutOptChannel = channel.parse()?;
                let symbols = non_empty_symbols(symbols)?;
                if opts.intraday_odd_lot.is_some() {
                    return invalid("intradayOddLot", "only supported on the Stock endpoint");
                }
                Ok(Subscription::FutOpt(
                    marketdata_core::FutOptSubscription::new(channel, symbols)
                        .with_after_hours(opts.after_hours.unwrap_or(false)),
                ))
            }
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

    /// Unsubscribe by server ids
    ///
    /// # Errors
    ///
    /// Returns error if not connected.
    async fn unsubscribe_ids_impl(&self, ids: Vec<String>) -> Result<(), MarketDataError> {
        if let Some(ws) = self.client() {
            ws.unsubscribe(ids).await?;
            Ok(())
        } else {
            Err(crate::errors::not_connected_error("Not connected"))
        }
    }

    /// Send a ping message to the server
    ///
    /// # Arguments
    /// * `state` - Optional state string echoed back in the pong response
    async fn measure_latency_impl(&self, timeout_ms: Option<u64>) -> Result<f64, MarketDataError> {
        let ws = self.client().ok_or(marketdata_core::MarketDataError::ClientClosed)?;
        let rtt = ws
            .measure_latency(timeout_ms.map(std::time::Duration::from_millis))
            .await?;
        Ok(rtt.as_secs_f64() * 1000.0)
    }

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

    /// See `disconnect` (#126).
    async fn disconnect_impl(&self) {
        // Read before the first await: a listener callback polls this future
        // first on the stream reader (C#, Go, C++), later polls may run
        // elsewhere.
        let caller = std::thread::current().id();

        // The connection this call closes, handed over in one step with the
        // reader the other callers are to wait for.
        let taken = {
            let mut slot = lock_connection(&self.connection);
            match slot.current.take() {
                Some(taken) => {
                    slot.closing = taken.reader.clone();
                    Some(taken)
                }
                None => {
                    // Nothing to close: a `connect()` in flight gives up its
                    // connection instead of storing it (#121), and any other
                    // caller is left the reader to wait for.
                    slot.disconnect_requested = true;
                    None
                }
            }
        };

        match taken {
            Some(connection) => self.close(connection, caller).await,
            None => {
                // The handshake's reader while a `connect()` runs — that
                // connection is closed before `connect()` returns — else the
                // reader of the connection another `disconnect()` closed.
                let reader = {
                    let slot = lock_connection(&self.connection);
                    slot.pending.clone().or_else(|| slot.closing.clone())
                };
                Self::wait_for(reader, caller).await;
            }
        }
    }

    /// Core's connection state of the current or last connection.
    fn state_handle(&self) -> Option<marketdata_core::ConnectionStateHandle> {
        lock_state(&self.state).clone()
    }

    /// The current core client, cloned out so no lock is held across awaits.
    fn client(&self) -> Option<Arc<CoreWebSocketClient>> {
        lock_connection(&self.connection).current.as_ref().map(|connection| Arc::clone(&connection.ws))
    }
}

/// Session options for `subscribe` / `unsubscribe` (#202).
///
/// Unset is the regular session, so an omitted or default record subscribes
/// as before. Each option belongs to one endpoint — `intraday_odd_lot`
/// (盤中零股) to Stock, `after_hours` (盤後) to FutOpt — and setting it on
/// the other, to any value, is 1005 `INVALID_PARAMETER`.
#[derive(Debug, Clone, Default, uniffi::Record)]
pub struct SubscribeOptions {
    /// FutOpt only: `true` subscribes to the after-hours session.
    #[uniffi(default = None)]
    pub after_hours: Option<bool>,
    /// Stock only: `true` subscribes to the intraday odd-lot session.
    #[uniffi(default = None)]
    pub intraday_odd_lot: Option<bool>,
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

/// `symbols` for `subscribe` / `unsubscribe`: a list with no symbol left
/// after trimming names no subscription and is 1005 `INVALID_PARAMETER`,
/// checked before the connection. Core trims and de-duplicates the rest.
fn non_empty_symbols(symbols: Vec<String>) -> Result<Vec<String>, MarketDataError> {
    if symbols.iter().all(|s| s.trim().is_empty()) {
        return Err(marketdata_core::MarketDataError::InvalidParameter {
            name: "symbols".to_string(),
            reason: "at least one symbol is required".to_string(),
        }
        .into());
    }
    Ok(symbols)
}

/// `ids` for `unsubscribe_ids`: an empty list names no subscription and is
/// 1005 `INVALID_PARAMETER`, checked before the connection.
fn non_empty_ids(ids: Vec<String>) -> Result<Vec<String>, MarketDataError> {
    if ids.is_empty() {
        return Err(marketdata_core::MarketDataError::InvalidParameter {
            name: "ids".to_string(),
            reason: "must not be empty".to_string(),
        }
        .into());
    }
    Ok(ids)
}

/// Sync (blocking) wrappers for C++ compatibility.
/// Uses a persistent tokio runtime stored in the client to keep background tasks alive.
#[cfg(feature = "cpp")]
#[uniffi::export]
impl WebSocketClient {
    /// Connect to the WebSocket server (blocking).
    ///
    /// Refused with code 2011 (`ALREADY_CONNECTED`) while connected or while
    /// another `connect()` is in progress.
    ///
    /// During an automatic reconnect it opens no connection of its own: it
    /// waits for that reconnect and returns once the connection is back and
    /// the subscriptions are re-sent, so a `subscribe()` afterwards follows
    /// them. The wait fails with 2010 (`ClientClosed`) if `disconnect()` is
    /// called, 3005 (`RECONNECT_FAILED`) if the reconnect runs out of
    /// attempts, and `AuthError` (2002) if its credentials are rejected.
    /// Called from a listener method, it holds up the listener until the
    /// reconnect ends.
    pub fn connect_sync(&self) -> Result<(), MarketDataError> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| crate::errors::other_error(e.to_string()))?;
        let (path, result) = rt.block_on(self.connect_with_path());
        // A connect that opened nothing — refused, or joined the reconnect of
        // the live connection — leaves that connection's runtime in place;
        // replacing it would abort that connection's tasks (#119, #230).
        if path != ConnectPath::Opened {
            return result;
        }
        // Store runtime to keep background tasks alive
        if let Ok(mut guard) = self.sync_runtime.lock() {
            *guard = Some(rt);
        }
        result
    }

    /// Subscribe to a channel for one or more symbols (blocking).
    ///
    /// Same arguments and checks as `subscribe`.
    #[uniffi::method(default(opts = None))]
    pub fn subscribe_sync(
        &self,
        channel: String,
        symbols: Vec<String>,
        opts: Option<SubscribeOptions>,
    ) -> Result<(), MarketDataError> {
        // Before the runtime check, as in `subscribe`.
        let sub = self.subscription(&channel, symbols, opts)?;
        let guard = self.sync_runtime.lock().unwrap();
        if let Some(ref rt) = *guard {
            rt.block_on(self.subscribe_impl(sub))
        } else {
            Err(crate::errors::not_connected_error("Not connected (call connect_sync first)"))
        }
    }

    /// Unsubscribe from a channel for one or more symbols (blocking).
    ///
    /// Same arguments and checks as `unsubscribe`.
    #[uniffi::method(default(opts = None))]
    pub fn unsubscribe_sync(
        &self,
        channel: String,
        symbols: Vec<String>,
        opts: Option<SubscribeOptions>,
    ) -> Result<(), MarketDataError> {
        let sub = self.subscription(&channel, symbols, opts)?;
        let guard = self.sync_runtime.lock().unwrap();
        if let Some(ref rt) = *guard {
            rt.block_on(self.unsubscribe_impl(sub))
        } else {
            Err(crate::errors::not_connected_error("Not connected"))
        }
    }

    /// Unsubscribe by the ids the server issued (blocking).
    ///
    /// An empty list is 1005 `INVALID_PARAMETER`.
    pub fn unsubscribe_ids_sync(&self, ids: Vec<String>) -> Result<(), MarketDataError> {
        let ids = non_empty_ids(ids)?;
        let guard = self.sync_runtime.lock().unwrap();
        if let Some(ref rt) = *guard {
            rt.block_on(self.unsubscribe_ids_impl(ids))
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

    /// Measure the round trip to the server in milliseconds (blocking).
    pub fn measure_latency_sync(&self, timeout_ms: Option<u64>) -> Result<f64, MarketDataError> {
        let guard = self.sync_runtime.lock().unwrap();
        if let Some(ref rt) = *guard {
            rt.block_on(self.measure_latency_impl(timeout_ms))
        } else {
            Err(marketdata_core::MarketDataError::ClientClosed.into())
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

    /// Disconnect from the WebSocket server (blocking), returning once the
    /// listener has handled the connection's remaining events,
    /// `on_disconnected` included. There is no timeout on that wait: a
    /// listener method that blocks keeps it waiting for as long as it does.
    /// Called from a listener method, it returns without that wait.
    pub fn disconnect_sync(&self) {
        // Taken out rather than held: while this waits for the listener
        // (#126), a callback calling a `*_sync` method must not block on
        // the lock.
        let rt = self
            .sync_runtime
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        if let Some(rt) = rt {
            rt.block_on(self.disconnect_impl());
            // Dropped to clean up
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
    delivered: Delivered,
    callback_failures: CallbackFailures,
) -> Option<StreamReader> {
    let (running, finished) = tokio::sync::watch::channel(());
    let handle = std::thread::Builder::new()
        .name("ws_stream_reader".to_string())
        .spawn(move || {
            // Dropped when the thread ends, however it ends.
            let _running = running;
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
                        delivered.observe(&event);
                        if !forward_event(event, &mut calls) {
                            break;
                        }
                    }
                    _ => {}
                }
            }
        })
        .ok()?;
    Some(StreamReader { thread: handle.thread().id(), finished })
}

/// What `connect()` and `disconnect()` hand over to each other, all under
/// the `connection` lock so neither can miss the other (#121, #126).
#[derive(Default)]
struct ConnectionSlot {
    /// The open connection, until `disconnect()` takes it.
    current: Option<Connection>,
    /// Reader of the connection a `connect()` is still handshaking, so a
    /// `disconnect()` during the handshake waits for the connection that
    /// `connect()` then closes.
    pending: Option<StreamReader>,
    /// A `disconnect()` found no connection to close: the `connect()` in
    /// flight, if any, closes its connection instead of storing it (#121).
    /// Cleared by the next `connect()`, so it never reaches a later one.
    disconnect_requested: bool,
    /// Reader of the connection last given up, which is closed or closing, so
    /// waiting for it always ends. A connection still in use never lands here.
    closing: Option<StreamReader>,
}

/// A connection whose `connect()` succeeded.
#[derive(Clone)]
struct Connection {
    /// Async methods clone the client out before awaiting.
    ws: Arc<CoreWebSocketClient>,
    /// Tells this connection's stream reader that `disconnect()` was called.
    stopping: Arc<AtomicBool>,
    /// `None` if the thread could not be started.
    reader: Option<StreamReader>,
    /// What the stream reader has handed the listener, for `connect()`.
    delivered: Delivered,
}

/// What a `connect()` did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConnectPath {
    /// Refused with 2011: nothing opened or waited on.
    Refused,
    /// Waited on the stored connection's automatic reconnect (#230).
    Joined,
    /// Built a new core client, whatever the outcome.
    Opened,
}

/// Whether the listener was last handed the connection as up, as a
/// `connect()` decides between refusing and waiting on a reconnect (#230).
///
/// Read from what the stream reader delivered rather than core's state: an
/// `on_disconnected` that calls `connect()` may run after core has already
/// reconnected, and has to wait on that reconnect, not be refused.
#[derive(Clone, Default)]
struct Delivered(Arc<AtomicU8>);

impl Delivered {
    /// No `on_authenticated` delivered yet for this connection.
    const PENDING: u8 = 0;
    /// `on_authenticated` delivered, and nothing since that ends it.
    const AUTHENTICATED: u8 = 1;
    /// `on_unauthenticated` or `on_disconnected` delivered since.
    const LOST: u8 = 2;

    /// Record `event`. Called by the stream reader before the listener
    /// runs, so a listener method calling `connect()` reads the event it is
    /// handling.
    fn observe(&self, event: &ConnectionEvent) {
        match event {
            ConnectionEvent::Authenticated { .. } => self.0.store(Self::AUTHENTICATED, Ordering::SeqCst),
            ConnectionEvent::Unauthenticated { .. } | ConnectionEvent::Disconnected { .. } => {
                self.0.store(Self::LOST, Ordering::SeqCst)
            }
            _ => {}
        }
    }

    /// The connect that opened this connection succeeded: it counts as
    /// delivered even if the reader has not got to `on_authenticated` yet, so
    /// a second `connect()` right after is refused rather than waiting.
    /// Unless the reader has already delivered the connection's loss.
    fn connect_succeeded(&self) {
        let _ = self.0.compare_exchange(
            Self::PENDING,
            Self::AUTHENTICATED,
            Ordering::SeqCst,
            Ordering::SeqCst,
        );
    }

    fn is_authenticated(&self) -> bool {
        self.0.load(Ordering::SeqCst) == Self::AUTHENTICATED
    }
}

/// A connection's stream reader thread, as `disconnect()` waits for it.
#[derive(Clone)]
struct StreamReader {
    thread: std::thread::ThreadId,
    /// Closed when the thread ends.
    finished: tokio::sync::watch::Receiver<()>,
}

impl StreamReader {
    /// Wait until the thread has ended: every event it forwards has been
    /// handled by the listener.
    async fn finished(mut self) {
        // No value is ever sent, so this returns once the sender is dropped.
        let _ = self.finished.changed().await;
    }
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

/// Lock `state`, recovering from poison: it only holds a handle.
fn lock_state(
    state: &std::sync::Mutex<Option<marketdata_core::ConnectionStateHandle>>,
) -> std::sync::MutexGuard<'_, Option<marketdata_core::ConnectionStateHandle>> {
    state.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Lock `connection`, recovering from poison: the slot is only ever
/// assigned or taken whole.
fn lock_connection(connection: &std::sync::Mutex<ConnectionSlot>) -> std::sync::MutexGuard<'_, ConnectionSlot> {
    connection.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
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
        /// Milliseconds `on_disconnected` blocks before recording.
        disconnected_delay_ms: std::sync::atomic::AtomicU64,
        /// Set when `on_disconnected` starts, before that delay.
        disconnected_entered: std::sync::atomic::AtomicBool,
        /// `on_message` disconnects `client`, blocking until that returns.
        disconnect_on_message: std::sync::atomic::AtomicBool,
        /// `on_authenticated` does the same, during the handshake.
        disconnect_on_authenticated: std::sync::atomic::AtomicBool,
        /// The next `on_disconnected` calls `connect()` on `client`, blocking
        /// until it returns, and records `connect(ok)` or `connect(<code>)`.
        connect_on_disconnect: std::sync::atomic::AtomicBool,
        /// Milliseconds `on_connected` blocks, to hold up the reader.
        connected_delay_ms: std::sync::atomic::AtomicU64,
        /// `on_disconnected` calls `ping_sync` on `client` and records the
        /// result (C++ sync API only).
        #[cfg(feature = "cpp")]
        ping_sync_on_disconnect: std::sync::atomic::AtomicBool,
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
                disconnected_delay_ms: std::sync::atomic::AtomicU64::new(0),
                disconnected_entered: std::sync::atomic::AtomicBool::new(false),
                disconnect_on_message: std::sync::atomic::AtomicBool::new(false),
                disconnect_on_authenticated: std::sync::atomic::AtomicBool::new(false),
                connect_on_disconnect: std::sync::atomic::AtomicBool::new(false),
                connected_delay_ms: std::sync::atomic::AtomicU64::new(0),
                #[cfg(feature = "cpp")]
                ping_sync_on_disconnect: std::sync::atomic::AtomicBool::new(false),
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

        /// Disconnect `client` from the callback this runs in, the way a
        /// foreign listener blocks on the returned future.
        fn disconnect_here(&self) {
            let client = self.client.get().and_then(std::sync::Weak::upgrade).expect("client");
            let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
            rt.block_on(client.disconnect_impl());
            self.record("disconnect returned".to_string());
        }

        /// `connect()` on `client` from the callback this runs in, the way a
        /// foreign listener blocks on the returned future.
        fn connect_here(&self) {
            let client = self.client.get().and_then(std::sync::Weak::upgrade).expect("client");
            let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
            let result = rt.block_on(client.connect_impl());
            self.record(match result {
                Ok(()) => "connect(ok)".to_string(),
                Err(e) => format!("connect({})", error_code(&e)),
            });
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

        /// Wait until exactly `n` `authenticated` events have been recorded.
        ///
        /// Core writes `Connected` before it emits the event (#86), and the
        /// forwarder runs on its own thread, so `is_connected()` is true
        /// well before the handshake's event reaches the listener: reading
        /// the count off the state alone is a race (#116).
        async fn wait_authenticated(&self, n: usize) {
            let count = || self.events().iter().filter(|e| e.starts_with("authenticated")).count();
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            while count() < n {
                assert!(
                    std::time::Instant::now() < deadline,
                    "timed out waiting for {n} authenticated events; got {:?}",
                    self.events()
                );
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
            // Like `wait_for`, let a stray extra one show up before asserting.
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            assert_eq!(count(), n, "{:?}", self.events());
        }
    }

    impl WebSocketListener for TestListener {
        fn on_connected(&self) {
            let delay = self.connected_delay_ms.load(Ordering::SeqCst);
            if delay > 0 {
                std::thread::sleep(std::time::Duration::from_millis(delay));
            }
            self.connected_count.fetch_add(1, Ordering::SeqCst);
            self.record("connected".to_string());
        }

        fn on_authenticated(&self, data_json: Option<String>) {
            self.record(format!("authenticated({data_json:?})"));
            if self.disconnect_on_authenticated.swap(false, Ordering::SeqCst) {
                self.disconnect_here();
            }
        }

        fn on_unauthenticated(&self, data_json: Option<String>) {
            self.record(format!("unauthenticated({data_json:?})"));
        }

        fn on_disconnected(&self, will_reconnect: bool) {
            if let Some(client) = self.client.get().and_then(std::sync::Weak::upgrade) {
                self.connected_on_disconnect.lock().unwrap().push(client.is_connected());
            }
            self.disconnected_entered.store(true, Ordering::SeqCst);
            let delay = self.disconnected_delay_ms.load(Ordering::SeqCst);
            if delay > 0 {
                std::thread::sleep(std::time::Duration::from_millis(delay));
            }
            #[cfg(feature = "cpp")]
            if self.ping_sync_on_disconnect.load(Ordering::SeqCst) {
                if let Some(client) = self.client.get().and_then(std::sync::Weak::upgrade) {
                    let result = match client.ping_sync(None) {
                        Ok(()) => "ok".to_string(),
                        Err(e) => e.to_string(),
                    };
                    self.record(format!("ping_sync({result})"));
                }
            }
            self.disconnected_count.fetch_add(1, Ordering::SeqCst);
            self.record(format!("disconnected({will_reconnect})"));
            if self.connect_on_disconnect.swap(false, Ordering::SeqCst) {
                self.connect_here();
            }
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
            if self.disconnect_on_message.swap(false, Ordering::SeqCst) {
                self.disconnect_here();
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
            None,
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
        // Already handled when disconnect() returns (#126).
        assert_eq!(listener.events().last().map(String::as_str), Some("disconnected(false)"));
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
        // The server's rejection: `error` code 1000 (#201).
        server.set_auth_response(serde_json::json!({
            "event": "error",
            "code": 1000,
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
        // The server's rejection: `error` code 1000 (#201).
        server.set_auth_response(serde_json::json!({
            "event": "error",
            "code": 1000,
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
                enabled: Some(true),
                max_attempts: 3,
                initial_delay_ms: 100,
                max_delay_ms: 200,
            }),
        );

        client.connect_impl().await.expect("connect");
        server.drop_transport_for(0).await;
        listener.wait_for("reconnecting(1)").await;
        listener.wait_authenticated(2).await;
        client.disconnect_impl().await;
        listener.wait_for("disconnected(false)").await;

        // A transport error reports `Error` before `Disconnected` (core's
        // delivery guarantee); its text is platform-dependent. The
        // disconnect() right after the reconnect, with no connect() after it,
        // is not warned about (#242).
        let (errors, lifecycle): (Vec<_>, Vec<_>) =
            listener.events().into_iter().partition(|e| e.starts_with("error("));
        let (warnings, errors): (Vec<_>, Vec<_>) =
            errors.into_iter().partition(|e| e.contains("automatic reconnect"));
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert_eq!(warnings, Vec::<String>::new());
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

    fn probe_record(idle_probe_after_ms: u64, probe_timeout_ms: u64) -> HealthCheckConfigRecord {
        HealthCheckConfigRecord {
            enabled: Some(true),
            heartbeat_timeout_ms: 0,
            probe_enabled: true,
            idle_probe_after_ms,
            probe_timeout_ms,
        }
    }

    /// A zero-valued record is the full default: every field's zero means
    /// "use default", so C++ `Record{}` and a Go `Record{}` literal keep
    /// auto-reconnect and health check on (#158, #161).
    #[test]
    fn zero_valued_records_are_the_core_defaults() {
        let reconnect = ReconnectConfigRecord::default()
            .to_core()
            .expect("defaults are valid");
        let default = marketdata_core::ReconnectionConfig::default();
        assert!(
            reconnect.enabled,
            "zero ReconnectConfigRecord must keep auto-reconnect on"
        );
        assert_eq!(reconnect.max_attempts, default.max_attempts);
        assert_eq!(reconnect.initial_delay, default.initial_delay);
        assert_eq!(reconnect.max_delay, default.max_delay);

        let health = HealthCheckConfigRecord::default()
            .to_core()
            .expect("defaults are valid");
        assert!(
            health.enabled,
            "zero HealthCheckConfigRecord must keep health check on"
        );
        assert!(
            !health.probe_enabled,
            "zero HealthCheckConfigRecord must leave probing off"
        );
        assert_eq!(health.heartbeat_timeout, std::time::Duration::from_secs(35));
    }

    /// `auth_timeout_ms` reaches core's `ConnectionConfig`; its zero keeps
    /// the core default (10 s), so a zero record is the full default (#199).
    #[test]
    fn connection_record_sets_auth_timeout_and_zero_keeps_the_default() {
        let mut config = marketdata_core::ConnectionConfig::builder(
            "ws://127.0.0.1:1/x",
            AuthRequest::with_api_key("k"),
        )
        .build();

        ConnectionConfigRecord::default().apply(&mut config);
        assert_eq!(config.auth_timeout, marketdata_core::websocket::DEFAULT_AUTH_TIMEOUT);
        assert_eq!(config.auth_timeout, std::time::Duration::from_secs(10));

        ConnectionConfigRecord { auth_timeout_ms: 15_000 }.apply(&mut config);
        assert_eq!(config.auth_timeout, std::time::Duration::from_secs(15));

        // A later zero record leaves an earlier value alone: 0 is "unset".
        ConnectionConfigRecord { auth_timeout_ms: 0 }.apply(&mut config);
        assert_eq!(config.auth_timeout, std::time::Duration::from_secs(15));
    }

    /// A server that never answers the auth frame: `connect()` fails with
    /// `TimeoutError` (3001) after `auth_timeout_ms`, not the old 10 s (#199).
    #[tokio::test(flavor = "multi_thread")]
    async fn unanswered_auth_times_out_after_auth_timeout_ms() {
        let server = MockWsServer::start().await;
        server.set_answer_auth(false);
        let listener = Arc::new(TestListener::new());
        let client = WebSocketClient::new_with_credentials(
            CredentialsRecord {
                api_key: Some("test-key".to_string()),
                bearer_token: None,
                sdk_token: None,
            },
            Arc::clone(&listener) as Arc<dyn WebSocketListener>,
            WebSocketEndpoint::Stock,
            Some(format!("ws://{}/marketdata", server.address())),
            Some(ReconnectConfigRecord { enabled: Some(false), ..Default::default() }),
            None,
            None,
            None,
            None,
            Some(ConnectionConfigRecord { auth_timeout_ms: 300 }),
        )
        .expect("valid config");

        let started = std::time::Instant::now();
        let result = client.connect_impl().await;
        let elapsed = started.elapsed();

        match result {
            Err(MarketDataError::TimeoutError { msg, info }) => {
                assert_eq!(info.code, marketdata_core::error_code::TIMEOUT, "{info:?}");
                assert_eq!(msg, "WebSocket authentication");
            }
            other => panic!("expected TimeoutError, got {other:?}"),
        }
        assert!(elapsed >= std::time::Duration::from_millis(300), "{elapsed:?}");
        assert!(elapsed < std::time::Duration::from_secs(3), "took the old 10 s: {elapsed:?}");
        // `on_error` arrives from the event thread, after connect() returned.
        listener.wait_for("error(Timeout error: WebSocket authentication)").await;
        let last_error = listener.last_error.lock().unwrap().clone();
        assert_eq!(
            last_error.as_ref().map(|e| e.code),
            Some(marketdata_core::error_code::TIMEOUT),
            "{last_error:?}"
        );
    }

    /// Validation runs after the zero fields take their defaults (#153):
    /// `initial_delay_ms: 0` is "use 1000", never "below the 100 ms floor",
    /// so a zero record stays legal (#158, #161). Each field is checked on
    /// its own, since either one zeroed could trip the floor or the
    /// `max >= initial` check if it were validated before defaulting.
    #[test]
    fn reconnect_record_zeroes_are_defaulted_before_validation() {
        let default = marketdata_core::ReconnectionConfig::default();
        let cases = [
            ("all zero", ReconnectConfigRecord::default()),
            ("disabled", ReconnectConfigRecord { enabled: Some(false), ..Default::default() }),
            ("initial only", ReconnectConfigRecord { initial_delay_ms: 1_000, ..Default::default() }),
            ("max only", ReconnectConfigRecord { max_delay_ms: 60_000, ..Default::default() }),
        ];
        for (name, record) in cases {
            let config = record.to_core().unwrap_or_else(|e| panic!("{name}: {e}"));
            assert_eq!(config.enabled, record.enabled.unwrap_or(default.enabled), "{name}");
            assert_eq!(config.max_attempts, default.max_attempts, "{name}");
            assert_eq!(config.initial_delay, default.initial_delay, "{name}");
            assert_eq!(config.max_delay, default.max_delay, "{name}");
        }
    }

    /// Core validates the record (#153): a delay below `MIN_INITIAL_DELAY_MS`
    /// or a `max_delay_ms` below `initial_delay_ms` is 1004, raised by the
    /// fallible constructor, as Node and Python raise it.
    #[test]
    fn reconnect_record_invalid_delays_are_a_config_error() {
        let cases = [
            (
                "initial below floor",
                ReconnectConfigRecord { initial_delay_ms: 1, ..Default::default() },
                "initial_delay must be >= 100ms",
            ),
            (
                "max below initial",
                ReconnectConfigRecord { initial_delay_ms: 5_000, max_delay_ms: 2_000, ..Default::default() },
                "max_delay (2000ms) must be >= initial_delay (5000ms)",
            ),
            (
                "invalid while disabled",
                ReconnectConfigRecord { enabled: Some(false), initial_delay_ms: 1, ..Default::default() },
                "initial_delay must be >= 100ms",
            ),
        ];
        for (name, record, expected) in cases {
            let result = WebSocketClient::new_with_credentials(
                CredentialsRecord {
                    api_key: Some("test-key".to_string()),
                    bearer_token: None,
                    sdk_token: None,
                },
                Arc::new(TestListener::new()),
                WebSocketEndpoint::Stock,
                None,
                Some(record),
                None,
                None,
                None,
                None,
                None,
            );
            match result {
                Err(MarketDataError::ConfigError { info, .. }) => {
                    assert_eq!(info.code, marketdata_core::error_code::CONFIG, "{name}: {info:?}");
                    assert!(info.message.contains(expected), "{name}: {info:?}");
                }
                Err(other) => panic!("{name}: expected CONFIG, got {other:?}"),
                Ok(_) => panic!("{name}: expected CONFIG, got a client"),
            }
        }
    }

    /// A constructor that cannot fail returns the reconnect error from
    /// `connect()`, as it does the health check error (#153).
    #[tokio::test(flavor = "multi_thread")]
    async fn deferred_reconnect_error_is_returned_by_connect() {
        let server = MockWsServer::start().await;
        let listener = Arc::new(TestListener::new());
        let client = mock_client(
            &server,
            Arc::clone(&listener),
            Some(ReconnectConfigRecord { initial_delay_ms: 1, ..Default::default() }),
        );
        match client.connect_impl().await {
            Err(MarketDataError::ConfigError { info, .. }) => {
                assert_eq!(info.code, marketdata_core::error_code::CONFIG, "{info:?}");
                assert!(info.message.contains("initial_delay must be >= 100ms (got 1ms)"), "{info:?}");
                assert!(!info.message.contains("Configuration error: Configuration error"), "{info:?}");
            }
            other => panic!("expected CONFIG, got {other:?}"),
        }
        assert_eq!(listener.connected_count.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn health_check_record_zeroes_take_the_core_defaults() {
        let config = probe_record(0, 0).to_core().expect("defaults are valid");
        assert!(config.probe_enabled);
        assert_eq!(config.heartbeat_timeout, std::time::Duration::from_secs(35));
        assert_eq!(config.idle_probe_after_or_default(), std::time::Duration::from_secs(30));
        assert_eq!(config.probe_timeout_or_default(), std::time::Duration::from_secs(5));
    }

    /// Core validates the record: a value below its floor is 1004, raised
    /// by the fallible constructor (#150).
    #[test]
    fn health_check_below_its_floor_is_a_config_error() {
        let listener = Arc::new(TestListener::new());
        let result = WebSocketClient::new_with_credentials(
            CredentialsRecord {
                api_key: Some("test-key".to_string()),
                bearer_token: None,
                sdk_token: None,
            },
            listener,
            WebSocketEndpoint::Stock,
            None,
            None,
            Some(probe_record(1_000, 0)),
            None,
            None,
            None,
            None,
        );
        match result {
            Err(MarketDataError::ConfigError { info, .. }) => {
                assert_eq!(info.code, marketdata_core::error_code::CONFIG, "{info:?}");
            }
            Err(other) => panic!("expected CONFIG, got {other:?}"),
            Ok(_) => panic!("expected CONFIG, got a client"),
        }
    }

    /// A constructor that cannot fail returns the error from `connect()`.
    #[tokio::test(flavor = "multi_thread")]
    async fn deferred_health_check_error_is_returned_by_connect() {
        let server = MockWsServer::start().await;
        let client = WebSocketClient::new_with_full_config(
            "test-key".to_string(),
            Arc::new(TestListener::new()),
            WebSocketEndpoint::Stock,
            Some(format!("ws://{}/marketdata", server.address())),
            None,
            Some(HealthCheckConfigRecord {
                enabled: Some(true),
                heartbeat_timeout_ms: 1_000,
                probe_enabled: false,
                idle_probe_after_ms: 0,
                probe_timeout_ms: 0,
            }),
            None,
            None,
        );
        match client.connect_impl().await {
            Err(MarketDataError::ConfigError { info, .. }) => {
                assert_eq!(info.code, marketdata_core::error_code::CONFIG, "{info:?}");
                assert!(!info.message.contains("Configuration error: Configuration error"), "{info:?}");
            }
            other => panic!("expected CONFIG, got {other:?}"),
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn measure_latency_returns_milliseconds_and_keeps_its_pong() {
        let server = MockWsServer::start().await;
        let listener = Arc::new(TestListener::recording_messages());
        let client = mock_client(&server, Arc::clone(&listener), None);

        client.connect_impl().await.expect("connect");
        let latency = client.measure_latency_impl(None).await.expect("latency");
        client.disconnect_impl().await;
        listener.wait_for("disconnected(false)").await;

        assert!((0.0..5_000.0).contains(&latency), "{latency}");
        assert!(!listener.events().iter().any(|e| e == "message(pong)"), "{:?}", listener.events());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn measure_latency_needs_a_connection() {
        let server = MockWsServer::start().await;
        let client = mock_client(&server, Arc::new(TestListener::new()), None);
        match client.measure_latency_impl(None).await {
            Err(MarketDataError::ClientClosed { info }) => {
                assert_eq!(info.code, marketdata_core::error_code::CLIENT_CLOSED, "{info:?}");
            }
            other => panic!("expected CLIENT_CLOSED, got {other:?}"),
        }
    }

    /// Assert `error` is the 2010 `connect()` gave up on a `disconnect()`
    /// (#121).
    fn assert_client_closed(error: MarketDataError) {
        match error {
            MarketDataError::ClientClosed { info } => {
                assert_eq!(info.code, marketdata_core::error_code::CLIENT_CLOSED, "{info:?}");
                assert!(info.message.starts_with("Connection aborted"), "{info:?}");
            }
            other => panic!("expected CLIENT_CLOSED, got {other:?}"),
        }
    }

    /// Assert `result` is the 2011 refusal of a second `connect()` (#119).
    fn assert_already_connected(result: Result<(), MarketDataError>) {
        match result {
            Err(MarketDataError::WebSocketError { info, .. }) => {
                assert_eq!(info.code, marketdata_core::error_code::ALREADY_CONNECTED, "{info:?}");
            }
            other => panic!("expected ALREADY_CONNECTED, got {other:?}"),
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn connect_while_connected_is_refused_and_keeps_the_connection() {
        // Room for a second connection, so opening one would show.
        let server = MockWsServer::start_with_capacity(2).await;
        let listener = Arc::new(TestListener::recording_messages());
        let client = mock_client(&server, Arc::clone(&listener), None);

        client.connect_impl().await.expect("connect");
        assert_already_connected(client.connect_impl().await);
        assert!(client.is_connected());

        // The first connection's messages are still forwarded.
        server
            .inject_frame_for(0, marketdata_core::models::streaming::StreamMessage::Pong { state: None })
            .await;
        listener.wait_for("message(pong)").await;
        client.disconnect_impl().await;
        listener.wait_for("disconnected(false)").await;
        assert_eq!(
            listener.events(),
            vec![
                "connected".to_string(),
                "authenticated(None)".to_string(),
                "message(authenticated)".to_string(),
                "message(pong)".to_string(),
                "disconnected(false)".to_string(),
            ]
        );

        // After disconnect() a new connection is allowed.
        client.connect_impl().await.expect("connect after disconnect");
        assert!(client.is_connected());
        client.disconnect_impl().await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn concurrent_connect_is_refused() {
        let server = MockWsServer::start_with_capacity(2).await;
        let listener = Arc::new(TestListener::new());
        let client = mock_client(&server, Arc::clone(&listener), None);

        let (first, second) = tokio::join!(client.connect_impl(), client.connect_impl());

        first.expect("first connect");
        assert_already_connected(second);
        assert!(client.is_connected());
        client.disconnect_impl().await;
        assert_eq!(listener.connected_count.load(Ordering::SeqCst), 1);
    }

    /// `info.code` of any error.
    fn error_code(error: &MarketDataError) -> i32 {
        match error {
            MarketDataError::ConnectionError { info, .. }
            | MarketDataError::AuthError { info, .. }
            | MarketDataError::RateLimitError { info, .. }
            | MarketDataError::InvalidSymbol { info, .. }
            | MarketDataError::ParseError { info, .. }
            | MarketDataError::TimeoutError { info, .. }
            | MarketDataError::WebSocketError { info, .. }
            | MarketDataError::ClientClosed { info }
            | MarketDataError::ConfigError { info, .. }
            | MarketDataError::ApiError { info, .. }
            | MarketDataError::Other { info, .. } => info.code,
        }
    }

    fn reconnect_every(max_attempts: u32, delay_ms: u64) -> Option<ReconnectConfigRecord> {
        Some(ReconnectConfigRecord {
            enabled: Some(true),
            max_attempts,
            initial_delay_ms: delay_ms,
            max_delay_ms: delay_ms,
        })
    }

    /// A connected client whose connection the server then drops, once
    /// `on_disconnected(true)` has been handled. Beyond `capacity` the server
    /// accepts no connection, so reconnect attempts fail.
    async fn lost_connection(
        capacity: usize,
        reconnect: Option<ReconnectConfigRecord>,
    ) -> (MockWsServer, Arc<TestListener>, Arc<WebSocketClient>) {
        let server = MockWsServer::start_with_capacity(capacity).await;
        let listener = Arc::new(TestListener::new());
        let client = mock_client(&server, Arc::clone(&listener), reconnect);
        let _ = listener.client.set(Arc::downgrade(&client));
        client.connect_impl().await.expect("connect");
        server.drop_transport_for(0).await;
        listener.wait_for("disconnected(true)").await;
        (server, listener, client)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn connect_in_on_disconnected_joins_the_reconnect() {
        let server = MockWsServer::start_with_capacity(3).await;
        let listener = Arc::new(TestListener::new());
        listener.connect_on_disconnect.store(true, Ordering::SeqCst);
        let client = mock_client(&server, Arc::clone(&listener), reconnect_every(0, 100));
        let _ = listener.client.set(Arc::downgrade(&client));

        client.connect_impl().await.expect("connect");
        server.drop_transport_for(0).await;
        listener.wait_for("connect(ok)").await;
        listener.wait_authenticated(2).await;
        // The reconnect's connection only: no third one.
        assert_eq!(listener.connected_count.load(Ordering::SeqCst), 2, "{:?}", listener.events());
        assert!(client.is_connected());

        // Joining the reconnect is not warned about (#242).
        assert_eq!(conflict_warnings(&listener), Vec::<String>::new());

        // Once its `on_authenticated` is delivered, it is refused again.
        assert_already_connected(client.connect_impl().await);
        client.disconnect_impl().await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn connect_during_backoff_waits_and_disconnect_ends_it_with_2010() {
        let (_server, _listener, client) = lost_connection(1, reconnect_every(0, 5000)).await;

        let join = tokio::spawn({
            let client = Arc::clone(&client);
            async move { client.connect_impl().await }
        });
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        assert!(!join.is_finished(), "connect() did not wait on the reconnect");

        let started = std::time::Instant::now();
        client.disconnect_impl().await;
        let result = tokio::time::timeout(std::time::Duration::from_secs(5), join)
            .await
            .expect("connect() returned")
            .unwrap();
        assert!(started.elapsed() < std::time::Duration::from_secs(2), "disconnect() waited out the backoff");
        assert_client_closed(result.expect_err("connect() during the backoff"));
    }

    /// Like core's client, every caller waits on the one reconnect and gets
    /// its outcome.
    #[tokio::test(flavor = "multi_thread")]
    async fn concurrent_connects_during_reconnect_all_join_it() {
        let (_server, listener, client) = lost_connection(2, reconnect_every(0, 1000)).await;

        let (first, second) = tokio::join!(client.connect_impl(), client.connect_impl());
        first.expect("first join");
        second.expect("second join");
        assert!(client.is_connected());
        listener.wait_authenticated(2).await;
        assert_eq!(listener.connected_count.load(Ordering::SeqCst), 2);
        client.disconnect_impl().await;
    }

    fn conflict_warnings(listener: &TestListener) -> Vec<String> {
        listener.events().into_iter().filter(|e| e.contains("automatic reconnect")).collect()
    }

    /// `disconnect()` soon after an automatic reconnect and then
    /// `connect()`, as code that also reconnects on its own does, reaches
    /// `on_error` as code 3006 from that `connect()`, once per client,
    /// though each connection has its own core client (#226, #242).
    #[tokio::test(flavor = "multi_thread")]
    async fn connect_after_a_disconnect_soon_after_a_reconnect_is_reported_once_per_client() {
        let (server, listener, client) = lost_connection(5, reconnect_every(0, 100)).await;
        for round in 0..2 {
            listener.wait_authenticated(2 * round + 2).await;
            client.disconnect_impl().await;
            listener.wait_for("disconnected(false)").await;
            if round == 0 {
                assert_eq!(conflict_warnings(&listener), Vec::<String>::new());
            }
            client.connect_impl().await.expect("connect");
            if round == 0 {
                let last = listener.last_error.lock().unwrap().clone().expect("an error");
                assert_eq!(last.code, marketdata_core::error_code::RECONNECT_CONFLICT);
                server.drop_transport_for(2).await;
            }
        }
        assert_eq!(conflict_warnings(&listener).len(), 1, "{:?}", listener.events());
        client.disconnect_impl().await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn concurrent_connects_during_reconnect_all_get_3005() {
        let (_server, _listener, client) = lost_connection(1, reconnect_every(1, 1000)).await;

        let (first, second) = tokio::join!(client.connect_impl(), client.connect_impl());
        for result in [first, second] {
            let error = result.expect_err("the only attempt fails");
            assert_eq!(error_code(&error), marketdata_core::error_code::RECONNECT_FAILED, "{error:?}");
        }
        client.disconnect_impl().await;
    }

    /// The join runs on the stream reader, which `disconnect()` waits for.
    #[tokio::test(flavor = "multi_thread")]
    async fn disconnect_while_on_disconnected_waits_on_the_reconnect() {
        let server = MockWsServer::start_with_capacity(1).await;
        let listener = Arc::new(TestListener::new());
        listener.connect_on_disconnect.store(true, Ordering::SeqCst);
        let client = mock_client(&server, Arc::clone(&listener), reconnect_every(0, 5000));
        let _ = listener.client.set(Arc::downgrade(&client));

        client.connect_impl().await.expect("connect");
        server.drop_transport_for(0).await;
        listener.wait_for("disconnected(true)").await;
        assert!(!listener.events().iter().any(|e| e.starts_with("connect(")), "{:?}", listener.events());

        let started = std::time::Instant::now();
        tokio::time::timeout(std::time::Duration::from_secs(5), client.disconnect_impl())
            .await
            .expect("disconnect() returned");
        assert!(started.elapsed() < std::time::Duration::from_secs(2), "disconnect() waited out the backoff");
        // disconnect() waited for the listener, this call included.
        let code = marketdata_core::error_code::CLIENT_CLOSED;
        assert!(listener.events().contains(&format!("connect({code})")), "{:?}", listener.events());
        assert_eq!(Arc::strong_count(&listener), 2, "the stream reader is still running");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn connect_during_reconnect_fails_with_3005_when_the_attempts_run_out() {
        let (_server, _listener, client) = lost_connection(1, reconnect_every(1, 1000)).await;

        let error = client.connect_impl().await.expect_err("the only attempt fails");
        assert!(matches!(error, MarketDataError::WebSocketError { .. }), "{error:?}");
        assert_eq!(error_code(&error), marketdata_core::error_code::RECONNECT_FAILED);
        client.disconnect_impl().await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn connect_during_reconnect_fails_with_auth_error_when_it_is_rejected() {
        let server = MockWsServer::start_with_capacity(2).await;
        let listener = Arc::new(TestListener::new());
        let client = mock_client(&server, Arc::clone(&listener), reconnect_every(0, 1000));
        client.connect_impl().await.expect("connect");
        server.set_auth_response(serde_json::json!({
            "event": "error", "code": 1000, "data": {"message": "Invalid token"}
        }));
        server.drop_transport_for(0).await;
        listener.wait_for("disconnected(true)").await;

        let error = client.connect_impl().await.expect_err("the reconnect is rejected");
        assert!(matches!(error, MarketDataError::AuthError { .. }), "{error:?}");
        client.disconnect_impl().await;
    }

    /// `connect()` returns before the reader has delivered
    /// `on_authenticated`; a second one is still refused, not joined.
    #[tokio::test(flavor = "multi_thread")]
    async fn second_connect_is_refused_before_on_authenticated_is_delivered() {
        let server = MockWsServer::start_with_capacity(2).await;
        let listener = Arc::new(TestListener::new());
        listener.connected_delay_ms.store(300, Ordering::SeqCst);
        let client = mock_client(&server, Arc::clone(&listener), reconnect_every(0, 100));

        client.connect_impl().await.expect("connect");
        assert!(!listener.events().iter().any(|e| e.starts_with("authenticated")));
        assert_already_connected(client.connect_impl().await);
        listener.wait_authenticated(1).await;
        assert_eq!(listener.connected_count.load(Ordering::SeqCst), 1);
        client.disconnect_impl().await;
    }

    /// Poll `listener` until `event` is recorded, off any runtime.
    #[cfg(feature = "cpp")]
    fn wait_for_blocking(listener: &TestListener, event: &str) {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !listener.events().iter().any(|e| e == event) {
            assert!(std::time::Instant::now() < deadline, "no {event}; got {:?}", listener.events());
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    /// A `connect_sync()` that joined the reconnect keeps the runtime the
    /// connection runs on (#230): replacing it would abort that connection's
    /// tasks.
    #[cfg(feature = "cpp")]
    #[test]
    fn connect_sync_that_joins_keeps_the_connection_running() {
        let server_rt = tokio::runtime::Runtime::new().unwrap();
        let server = server_rt.block_on(MockWsServer::start_with_capacity(3));
        let listener = Arc::new(TestListener::new());
        // Long enough a backoff that the second `connect_sync()` is within it.
        let client = mock_client(&server, Arc::clone(&listener), reconnect_every(0, 1500));

        client.connect_sync().expect("connect_sync");
        server_rt.block_on(server.drop_transport_for(0));
        wait_for_blocking(&listener, "disconnected(true)");
        client.connect_sync().expect("connect_sync joins the reconnect");

        client.ping_sync(Some("after-join".to_string())).expect("ping_sync");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !server.pings_received().iter().any(|p| p["state"] == "after-join") {
            assert!(std::time::Instant::now() < deadline, "the ping never reached the server");
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(client.is_connected());
        assert_eq!(listener.connected_count.load(Ordering::SeqCst), 2);
        client.disconnect_sync();
    }

    /// One that joined and failed keeps it too; the connection is gone, and
    /// the next `connect_sync()` opens a working one on its own runtime.
    #[cfg(feature = "cpp")]
    #[test]
    fn connect_sync_after_a_failed_join_opens_a_working_connection() {
        let server_rt = tokio::runtime::Runtime::new().unwrap();
        let server = server_rt.block_on(MockWsServer::start_with_capacity(3));
        let listener = Arc::new(TestListener::new());
        let client = mock_client(&server, Arc::clone(&listener), reconnect_every(1, 1500));

        client.connect_sync().expect("connect_sync");
        server.set_auth_response(serde_json::json!({
            "event": "error", "code": 1000, "data": {"message": "Invalid token"}
        }));
        server_rt.block_on(server.drop_transport_for(0));
        wait_for_blocking(&listener, "disconnected(true)");
        let error = client.connect_sync().expect_err("the reconnect is rejected");
        assert!(matches!(error, MarketDataError::AuthError { .. }), "{error:?}");

        server.set_auth_response(serde_json::json!({ "event": "authenticated" }));
        client.connect_sync().expect("connect_sync after the failed join");
        client.ping_sync(Some("fresh".to_string())).expect("ping_sync");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !server.pings_received().iter().any(|p| p["state"] == "fresh") {
            assert!(std::time::Instant::now() < deadline, "the ping never reached the server");
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        client.disconnect_sync();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn connect_after_server_close_without_reconnect_is_allowed() {
        let server = MockWsServer::start_with_capacity(2).await;
        let listener = Arc::new(TestListener::new());
        let client = mock_client(&server, Arc::clone(&listener), None);

        client.connect_impl().await.expect("connect");
        server.close_for(0, 1000, "bye").await;
        wait_closed(&client, true).await;

        client.connect_impl().await.expect("connect after the server closed");
        assert!(client.is_connected());
        client.disconnect_impl().await;
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
    async fn reconnects_by_default_without_a_record() {
        // No binding-side override: omitting the record keeps the core
        // default, auto-reconnect on (#149).
        let server = MockWsServer::start_with_capacity(2).await;
        let listener = Arc::new(TestListener::new());
        let client = mock_client(&server, Arc::clone(&listener), None);

        client.connect_impl().await.expect("connect");
        server.drop_transport_for(0).await;
        wait_connected(&client, false).await;

        wait_connected(&client, true).await;
        listener.wait_authenticated(2).await;
        client.disconnect_impl().await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn is_connected_follows_core_state_across_reconnect() {
        let server = MockWsServer::start_with_capacity(2).await;
        let listener = Arc::new(TestListener::new());
        let client = mock_client(
            &server,
            Arc::clone(&listener),
            Some(ReconnectConfigRecord {
                enabled: Some(true),
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
        listener.wait_authenticated(2).await;

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
        let client = mock_client(
            &server,
            Arc::clone(&listener),
            Some(ReconnectConfigRecord {
                enabled: Some(false),
                max_attempts: 0,
                initial_delay_ms: 0,
                max_delay_ms: 0,
            }),
        );
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
                enabled: Some(true),
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
                enabled: Some(true),
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
                // A close the (default, enabled) reconnect policy does not
                // retry: 1000, normal closure (4xxx reconnects since #201).
                server.close(1000, "bye").await;
            }
            listener.wait_for(&format!("disconnected({will_reconnect})")).await;
            assert_eq!(*listener.connected_on_disconnect.lock().unwrap(), vec![false]);

            client.disconnect_impl().await;
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn disconnect_returns_after_the_listener_handled_disconnected() {
        let server = MockWsServer::start().await;
        let listener = Arc::new(TestListener::new());
        listener.disconnected_delay_ms.store(300, Ordering::SeqCst);
        let client = mock_client(&server, Arc::clone(&listener), None);

        client.connect_impl().await.expect("connect");
        listener.wait_for("authenticated(None)").await;
        client.disconnect_impl().await;
        assert_eq!(listener.events().last().map(String::as_str), Some("disconnected(false)"));
        // Nothing left to close or wait for.
        tokio::time::timeout(std::time::Duration::from_secs(5), client.disconnect_impl())
            .await
            .expect("second disconnect()");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_concurrent_disconnect_waits_for_the_same_connection() {
        let server = MockWsServer::start().await;
        let listener = Arc::new(TestListener::new());
        listener.disconnected_delay_ms.store(300, Ordering::SeqCst);
        let client = mock_client(&server, Arc::clone(&listener), None);

        client.connect_impl().await.expect("connect");
        listener.wait_for("authenticated(None)").await;

        let first = tokio::spawn({
            let client = Arc::clone(&client);
            async move { client.disconnect_impl().await }
        });
        // Past the point where the first call took the connection, so the
        // second finds none and has only the closing reader to wait for.
        while !listener.disconnected_entered.load(Ordering::SeqCst) {
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
        tokio::time::timeout(std::time::Duration::from_secs(5), client.disconnect_impl())
            .await
            .expect("concurrent disconnect()");
        assert_eq!(
            listener.events().last().map(String::as_str),
            Some("disconnected(false)"),
            "the concurrent disconnect() returned before the listener handled it"
        );
        first.await.unwrap();
    }

    /// A server whose auth handshake only completes once `Authenticated` is
    /// injected, so a test can act while `connect()` is still running.
    async fn server_holding_the_handshake() -> MockWsServer {
        let server = MockWsServer::start().await;
        server.set_auth_response(serde_json::json!({"event": "pong"}));
        server
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn disconnect_during_connect_closes_that_connection(){
        let server = server_holding_the_handshake().await;
        let listener = Arc::new(TestListener::new());
        let client = mock_client(&server, Arc::clone(&listener), None);

        let connecting = tokio::spawn({
            let client = Arc::clone(&client);
            async move { client.connect_impl().await }
        });
        listener.wait_for("connected").await;

        let disconnecting = tokio::spawn({
            let client = Arc::clone(&client);
            async move { client.disconnect_impl().await }
        });
        // The handshake completes only now: `connect()` gives the connection
        // up instead of storing it (#121).
        server.inject_frame(marketdata_core::models::streaming::StreamMessage::Authenticated).await;

        let error = tokio::time::timeout(std::time::Duration::from_secs(10), connecting)
            .await
            .expect("connect() returned")
            .unwrap()
            .expect_err("connect() cancelled by disconnect()");
        assert_client_closed(error);
        tokio::time::timeout(std::time::Duration::from_secs(10), disconnecting)
            .await
            .expect("disconnect() returned")
            .unwrap();

        assert!(!client.is_connected());
        assert!(client.is_closed());
        // Closed like any other connection: the listener saw it end, and its
        // reader is gone (this test and `client` hold the last references).
        assert_eq!(listener.events().last().map(String::as_str), Some("disconnected(false)"));
        assert_eq!(Arc::strong_count(&listener), 2, "the stream reader is still running");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn disconnect_from_on_authenticated_closes_that_connection() {
        // `on_authenticated` reaches the listener around the moment
        // `connect()` stores the connection: either it is there to close, or
        // `connect()` gives it up (#121). The connection ends either way, so
        // the run is repeated to cover both orders.
        for _ in 0..10 {
            let server = MockWsServer::start().await;
            let listener = Arc::new(TestListener::new());
            listener.disconnect_on_authenticated.store(true, Ordering::SeqCst);
            let client = mock_client(&server, Arc::clone(&listener), None);
            let _ = listener.client.set(Arc::downgrade(&client));

            // The listener disconnects on the reader thread, so this must not
            // wait for itself either.
            let result = tokio::time::timeout(std::time::Duration::from_secs(10), client.connect_impl())
                .await
                .expect("connect() returned");
            if let Err(error) = result {
                assert_client_closed(error);
            }

            listener.wait_for("disconnected(false)").await;
            assert!(!client.is_connected(), "{:?}", listener.events());
            let events = listener.events();
            let returned = events.iter().position(|e| e == "disconnect returned");
            assert!(
                returned.is_some() && returned < events.iter().position(|e| e == "disconnected(false)"),
                "{events:?}"
            );
            assert_eq!(Arc::strong_count(&listener), 2, "the stream reader is still running");
        }
    }

    #[cfg(feature = "cpp")]
    #[test]
    fn listener_calling_a_sync_method_during_disconnect_sync_does_not_block() {
        let server_rt = tokio::runtime::Runtime::new().unwrap();
        let server = server_rt.block_on(MockWsServer::start());
        let listener = Arc::new(TestListener::new());
        listener.ping_sync_on_disconnect.store(true, Ordering::SeqCst);
        let client = mock_client(&server, Arc::clone(&listener), None);
        let _ = listener.client.set(Arc::downgrade(&client));

        client.connect_sync().expect("connect_sync");
        let (done_tx, done_rx) = std::sync::mpsc::channel();
        std::thread::spawn({
            let client = Arc::clone(&client);
            move || {
                client.disconnect_sync();
                let _ = done_tx.send(());
            }
        });
        done_rx
            .recv_timeout(std::time::Duration::from_secs(10))
            .expect("disconnect_sync() did not return");

        // disconnect_sync() waited for on_disconnected, whose ping_sync()
        // found the runtime already gone.
        let events = listener.events();
        assert_eq!(
            &events[events.len() - 2..],
            ["ping_sync(Connection error: Not connected)".to_string(), "disconnected(false)".to_string()],
            "{events:?}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn disconnect_from_a_listener_callback_does_not_wait_for_itself() {
        let server = MockWsServer::start().await;
        let listener = Arc::new(TestListener::recording_messages());
        let client = mock_client(&server, Arc::clone(&listener), None);
        let _ = listener.client.set(Arc::downgrade(&client));

        client.connect_impl().await.expect("connect");
        listener.disconnect_on_message.store(true, Ordering::SeqCst);
        server
            .inject_frame(marketdata_core::models::streaming::StreamMessage::Pong { state: None })
            .await;
        listener.wait_for("disconnected(false)").await;
        tokio::time::timeout(std::time::Duration::from_secs(5), client.disconnect_impl())
            .await
            .expect("disconnect() after the callback's");

        let events = listener.events();
        let returned = events.iter().position(|e| e == "disconnect returned");
        let disconnected = events.iter().position(|e| e == "disconnected(false)");
        assert!(returned.is_some() && returned < disconnected, "{events:?}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn disconnect_while_reconnecting_reports_final_disconnect_and_stops_forwarder() {
        let server = MockWsServer::start_with_capacity(2).await;
        let listener = Arc::new(TestListener::new());
        let client = mock_client(
            &server,
            Arc::clone(&listener),
            Some(ReconnectConfigRecord {
                enabled: Some(true),
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

        // `disconnect()` returns once `on_disconnected(false)`, the final
        // event core queues for the stopped reconnect, was delivered (#98).
        let lifecycle: Vec<_> = before.iter().filter(|e| !e.starts_with("error(")).collect();
        assert_eq!(
            lifecycle,
            [
                "connected",
                "authenticated(None)",
                "disconnected(true)",
                "reconnecting(1)",
                "disconnected(false)",
            ],
            "{before:?}"
        );

        // Outlast the backoff: a reconnect that was not cancelled would have
        // reported `connected` by now.
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
        assert_eq!(listener.events(), before, "events after disconnect()");

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

    #[cfg(not(feature = "cpp"))]
    const FUTOPT_ODD_LOT: &str =
        "Invalid parameter 'intradayOddLot': only supported on the Stock endpoint";

    const EMPTY_SYMBOLS: &str = "Invalid parameter 'symbols': at least one symbol is required";

    fn futopt_client() -> Arc<WebSocketClient> {
        WebSocketClient::new_with_endpoint(
            "test-key".to_string(),
            Arc::new(TestListener::new()),
            WebSocketEndpoint::FutOpt,
        )
    }

    fn syms(symbols: &[&str]) -> Vec<String> {
        symbols.iter().map(|s| s.to_string()).collect()
    }

    fn after_hours(value: bool) -> Option<SubscribeOptions> {
        Some(SubscribeOptions { after_hours: Some(value), intraday_odd_lot: None })
    }

    fn odd_lot(value: bool) -> Option<SubscribeOptions> {
        Some(SubscribeOptions { after_hours: None, intraday_odd_lot: Some(value) })
    }

    /// The subscribe frame's `data` and the local keys `subscription()` builds.
    fn frame_and_keys(sub: Subscription) -> (serde_json::Value, Vec<String>) {
        match sub {
            Subscription::Stock(sub) => (sub.to_subscribe_data(), sub.keys()),
            Subscription::FutOpt(sub) => (sub.to_subscribe_data(), sub.keys()),
        }
    }

    #[cfg(not(feature = "cpp"))]
    fn assert_not_connected(result: Result<(), MarketDataError>) {
        match result {
            Err(MarketDataError::ConnectionError { msg, info }) if msg == "Not connected" => {
                assert_eq!(info.code, marketdata_core::error_code::CONNECTION, "{info:?}");
            }
            other => panic!("expected \"Not connected\", got {other:?}"),
        }
    }

    #[cfg(not(feature = "cpp"))]
    #[tokio::test(flavor = "multi_thread")]
    async fn subscribe_rejects_unknown_channel_before_connecting() {
        let client = WebSocketClient::new("test-key".to_string(), Arc::new(TestListener::new()));
        assert_unknown_channel(client.subscribe("trade".into(), syms(&["2330"]), None).await);
        assert_unknown_channel(client.unsubscribe("trade".into(), syms(&["2330"]), None).await);
        // The channel is checked before the symbols and the options.
        assert_unknown_channel(client.subscribe("trade".into(), Vec::new(), after_hours(true)).await);
        // A known name, in any case, gets past the check to "not connected".
        assert_not_connected(client.subscribe("Trades".into(), syms(&["2330"]), None).await);
        assert_not_connected(client.unsubscribe("Trades".into(), syms(&["2330"]), None).await);
    }

    #[cfg(not(feature = "cpp"))]
    #[tokio::test(flavor = "multi_thread")]
    async fn futopt_endpoint_parses_futopt_channels_before_connecting() {
        let client = futopt_client();
        assert_invalid_parameter(
            client.subscribe("indices".into(), syms(&["TXFE6"]), None).await,
            FUTOPT_INDICES,
        );
        assert_invalid_parameter(
            client.unsubscribe("indices".into(), syms(&["TXFE6"]), after_hours(true)).await,
            FUTOPT_INDICES,
        );
        assert_not_connected(client.subscribe("Books".into(), syms(&["TXFE6"]), after_hours(true)).await);
        assert_not_connected(client.unsubscribe("books".into(), syms(&["TXFE6"]), None).await);
    }

    #[cfg(not(feature = "cpp"))]
    #[tokio::test(flavor = "multi_thread")]
    async fn stock_endpoint_rejects_after_hours_before_connecting() {
        let client = WebSocketClient::new("test-key".to_string(), Arc::new(TestListener::new()));
        for value in [true, false] {
            assert_invalid_parameter(
                client.subscribe("trades".into(), syms(&["2330"]), after_hours(value)).await,
                STOCK_AFTER_HOURS,
            );
            assert_invalid_parameter(
                client.unsubscribe("trades".into(), syms(&["2330"]), after_hours(value)).await,
                STOCK_AFTER_HOURS,
            );
        }
    }

    #[cfg(not(feature = "cpp"))]
    #[tokio::test(flavor = "multi_thread")]
    async fn futopt_endpoint_rejects_intraday_odd_lot_before_connecting() {
        let client = futopt_client();
        for value in [true, false] {
            assert_invalid_parameter(
                client.subscribe("trades".into(), syms(&["TXFE6"]), odd_lot(value)).await,
                FUTOPT_ODD_LOT,
            );
            assert_invalid_parameter(
                client.unsubscribe("trades".into(), syms(&["TXFE6"]), odd_lot(value)).await,
                FUTOPT_ODD_LOT,
            );
        }
    }

    #[cfg(not(feature = "cpp"))]
    #[tokio::test(flavor = "multi_thread")]
    async fn subscribe_rejects_empty_symbols_before_connecting() {
        for client in [
            WebSocketClient::new("test-key".to_string(), Arc::new(TestListener::new())),
            futopt_client(),
        ] {
            for symbols in [Vec::new(), syms(&[""]), syms(&["  ", "\t"])] {
                assert_invalid_parameter(
                    client.subscribe("trades".into(), symbols.clone(), None).await,
                    EMPTY_SYMBOLS,
                );
                assert_invalid_parameter(
                    client.unsubscribe("trades".into(), symbols, None).await,
                    EMPTY_SYMBOLS,
                );
            }
        }
        // The symbols are checked before the endpoint's session option.
        let stock = WebSocketClient::new("test-key".to_string(), Arc::new(TestListener::new()));
        assert_invalid_parameter(
            stock.subscribe("trades".into(), Vec::new(), after_hours(true)).await,
            EMPTY_SYMBOLS,
        );
        assert_invalid_parameter(
            futopt_client().subscribe("trades".into(), Vec::new(), odd_lot(true)).await,
            EMPTY_SYMBOLS,
        );
    }

    /// One symbol is sent as `symbol`, as the single-symbol API did; several
    /// as `symbols` in one frame. The options add the session field and the
    /// key suffix.
    #[test]
    fn subscription_frames_and_keys() {
        let stock = WebSocketClient::new("test-key".to_string(), Arc::new(TestListener::new()));
        let (frame, keys) = frame_and_keys(stock.subscription("trades", syms(&["2330"]), None).unwrap());
        assert_eq!(frame, serde_json::json!({ "channel": "trades", "symbol": "2330" }));
        assert_eq!(keys, ["trades:2330"]);

        let (frame, keys) =
            frame_and_keys(stock.subscription("trades", syms(&["2330", "2317"]), None).unwrap());
        assert_eq!(frame, serde_json::json!({ "channel": "trades", "symbols": ["2330", "2317"] }));
        assert_eq!(keys, ["trades:2330", "trades:2317"]);

        let (frame, keys) =
            frame_and_keys(stock.subscription("trades", syms(&["2330"]), odd_lot(true)).unwrap());
        assert_eq!(
            frame,
            serde_json::json!({ "channel": "trades", "symbol": "2330", "intradayOddLot": true })
        );
        assert_eq!(keys, ["trades:2330:oddlot"]);

        // `false` is the regular session, as is a default record.
        let (frame, _) = frame_and_keys(stock.subscription("trades", syms(&["2330"]), odd_lot(false)).unwrap());
        assert_eq!(frame, serde_json::json!({ "channel": "trades", "symbol": "2330" }));
        let (frame, _) = frame_and_keys(
            stock.subscription("trades", syms(&["2330"]), Some(SubscribeOptions::default())).unwrap(),
        );
        assert_eq!(frame, serde_json::json!({ "channel": "trades", "symbol": "2330" }));

        // Core normalises: trimmed, de-duplicated, one left is `symbol`.
        let (frame, _) =
            frame_and_keys(stock.subscription("books", syms(&[" 2330 ", "2330", ""]), None).unwrap());
        assert_eq!(frame, serde_json::json!({ "channel": "books", "symbol": "2330" }));

        let futopt = futopt_client();
        let (frame, keys) = frame_and_keys(
            futopt.subscription("books", syms(&["TXFE6", "MXFE6"]), after_hours(true)).unwrap(),
        );
        assert_eq!(
            frame,
            serde_json::json!({ "channel": "books", "symbols": ["TXFE6", "MXFE6"], "afterHours": true })
        );
        assert_eq!(keys, ["books:TXFE6:afterhours", "books:MXFE6:afterhours"]);
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

        client.subscribe("books".into(), syms(&["TXFE6"]), after_hours(true)).await.expect("subscribe");
        client.subscribe("books".into(), syms(&["TXFE6"]), None).await.expect("subscribe");
        let mut keys: Vec<_> = core.subscriptions().iter().map(|sub| sub.key()).collect();
        keys.sort();
        assert_eq!(keys, ["books:TXFE6", "books:TXFE6:afterhours"]);

        client.unsubscribe("books".into(), syms(&["TXFE6"]), after_hours(true)).await.expect("unsubscribe");
        let keys: Vec<_> = core.subscriptions().iter().map(|sub| sub.key()).collect();
        assert_eq!(keys, ["books:TXFE6"]);

        client.unsubscribe("Books".into(), syms(&["TXFE6"]), None).await.expect("unsubscribe");
        assert_eq!(core.subscription_count(), 0);
        client.disconnect_impl().await;
    }

    /// A batch is one frame and N subscriptions; `unsubscribe` with the same
    /// list removes them all, and odd-lot is a separate subscription.
    #[cfg(not(feature = "cpp"))]
    #[tokio::test(flavor = "multi_thread")]
    async fn stock_batch_and_odd_lot_subscriptions_are_tracked_per_symbol() {
        let server = MockWsServer::start().await;
        let client = mock_client_for(
            &server,
            Arc::new(TestListener::new()),
            WebSocketEndpoint::Stock,
            None,
        );
        client.connect_impl().await.expect("connect");
        let core = client.client().expect("connected");

        client.subscribe("trades".into(), syms(&["2330", "2317"]), None).await.expect("subscribe");
        client.subscribe("trades".into(), syms(&["2330"]), odd_lot(true)).await.expect("subscribe");
        let mut keys: Vec<_> = core.subscriptions().iter().map(|sub| sub.key()).collect();
        keys.sort();
        assert_eq!(keys, ["trades:2317", "trades:2330", "trades:2330:oddlot"]);

        client.unsubscribe("trades".into(), syms(&["2330", "2317"]), None).await.expect("unsubscribe");
        let keys: Vec<_> = core.subscriptions().iter().map(|sub| sub.key()).collect();
        assert_eq!(keys, ["trades:2330:oddlot"]);

        client.unsubscribe("trades".into(), syms(&["2330"]), odd_lot(true)).await.expect("unsubscribe");
        assert_eq!(core.subscription_count(), 0);
        client.disconnect_impl().await;
    }

    const EMPTY_IDS: &str = "Invalid parameter 'ids': must not be empty";

    #[cfg(not(feature = "cpp"))]
    #[tokio::test(flavor = "multi_thread")]
    async fn unsubscribe_ids_rejects_empty_list_before_connecting() {
        let client = WebSocketClient::new("test-key".to_string(), Arc::new(TestListener::new()));
        assert_invalid_parameter(client.unsubscribe_ids(Vec::new()).await, EMPTY_IDS);
        assert_not_connected(client.unsubscribe_ids(vec!["abc".into()]).await);
    }

    /// The mock server does not ack, so this unsubscribes by local key with
    /// the ack outstanding: core records the cancel and removes the
    /// subscription. `core/tests/unsubscribe.rs` covers the server-id path.
    #[cfg(not(feature = "cpp"))]
    #[tokio::test(flavor = "multi_thread")]
    async fn unsubscribe_ids_removes_subscriptions_it_names() {
        let server = MockWsServer::start().await;
        let client = mock_client_for(
            &server,
            Arc::new(TestListener::new()),
            WebSocketEndpoint::Stock,
            None,
        );
        client.connect_impl().await.expect("connect");
        let core = client.client().expect("connected");

        client.subscribe("trades".into(), syms(&["2330"]), None).await.expect("subscribe");
        client.subscribe("books".into(), syms(&["2330"]), None).await.expect("subscribe");
        client.unsubscribe_ids(vec!["trades:2330".into()]).await.expect("unsubscribe");
        let keys: Vec<_> = core.subscriptions().iter().map(|sub| sub.key()).collect();
        assert_eq!(keys, ["books:2330"]);
        client.disconnect_impl().await;
    }

    #[cfg(feature = "cpp")]
    #[test]
    fn unsubscribe_ids_sync_rejects_empty_list_before_connecting() {
        let client = WebSocketClient::new("test-key".to_string(), Arc::new(TestListener::new()));
        assert_invalid_parameter(client.unsubscribe_ids_sync(Vec::new()), EMPTY_IDS);
    }

    #[cfg(feature = "cpp")]
    #[test]
    fn subscribe_sync_rejects_unknown_channel_before_connecting() {
        let client = WebSocketClient::new("test-key".to_string(), Arc::new(TestListener::new()));
        assert_unknown_channel(client.subscribe_sync("trade".into(), syms(&["2330"]), None));
        assert_unknown_channel(client.unsubscribe_sync("trade".into(), syms(&["2330"]), None));
        assert_invalid_parameter(client.subscribe_sync("trades".into(), Vec::new(), None), EMPTY_SYMBOLS);
        assert_invalid_parameter(
            client.subscribe_sync("trades".into(), syms(&["2330"]), after_hours(true)),
            "Invalid parameter 'afterHours': only supported on the FutOpt endpoint",
        );
    }

    #[cfg(feature = "cpp")]
    #[test]
    fn sync_futopt_endpoint_parses_futopt_channels() {
        let client = futopt_client();
        assert_invalid_parameter(client.subscribe_sync("indices".into(), syms(&["TXFE6"]), None), FUTOPT_INDICES);
        assert_invalid_parameter(client.unsubscribe_sync("indices".into(), syms(&["TXFE6"]), None), FUTOPT_INDICES);
        assert_invalid_parameter(
            client.subscribe_sync("trades".into(), syms(&["TXFE6"]), odd_lot(true)),
            "Invalid parameter 'intradayOddLot': only supported on the Stock endpoint",
        );
    }
}
