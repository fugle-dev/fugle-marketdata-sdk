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
//! │ OnMessage(msg) ◄────│──────────────│ run_message_loop()  │
//! │ OnConnected()  ◄────│              │                     │
//! │ OnDisconnected()◄───│              │ CoreWebSocketClient │
//! │ OnError(err)   ◄────│              │   .messages()       │
//! └─────────────────────┘              └─────────────────────┘
//! ```
//!
//! # Thread Safety
//!
//! The `WebSocketListener` trait requires `Send + Sync` for thread-safe
//! callback invocation. Foreign implementations must be thread-safe.

use crate::errors::MarketDataError;
use crate::models::StreamMessage;
use marketdata_core::aio::WebSocketClient as CoreWebSocketClient;
use marketdata_core::websocket::MessageReceiver;
use marketdata_core::AuthRequest;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;

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
///     public void OnDisconnected() {
///         Console.WriteLine("Disconnected");
///     }
///     public void OnMessage(StreamMessage message) {
///         Console.WriteLine($"Got {message.Event} for {message.Symbol}");
///     }
///     public void OnError(string errorMessage) {
///         Console.WriteLine($"Error: {errorMessage}");
///     }
/// }
/// ```
#[uniffi::export(with_foreign)]
pub trait WebSocketListener: Send + Sync {
    /// Called when WebSocket connection is established
    fn on_connected(&self);

    /// Called when WebSocket connection is closed
    fn on_disconnected(&self);

    /// Called when a message is received
    fn on_message(&self, message: StreamMessage);

    /// Called when an error occurs
    fn on_error(&self, error_message: String);

    /// Called when a reconnection attempt starts
    fn on_reconnecting(&self, attempt: u32);

    /// Called when all reconnection attempts are exhausted
    fn on_reconnect_failed(&self, attempts: u32);
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
                return Err(MarketDataError::ConfigError {
                    msg: format!(
                        "stock streaming does not support {other} (supported: v1.0). \
                         Leave it unset to use v1.0."
                    ),
                })
            }
        };
        let futopt = match self.futopt.as_deref() {
            None => FutOptVersion::default(),
            Some("v1.0") => FutOptVersion::V1_0,
            Some("v1.1") => FutOptVersion::V1_1,
            Some(other) => {
                return Err(MarketDataError::ConfigError {
                    msg: format!(
                        "futopt streaming does not support {other} (supported: v1.0, v1.1). \
                         Leave it unset to use v1.1."
                    ),
                })
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
    inner: Arc<Mutex<Option<CoreWebSocketClient>>>,
    listener: Arc<dyn WebSocketListener>,
    api_key: String,
    base_url: Option<String>,
    version: StreamingVersionRecord,
    endpoint: WebSocketEndpoint,
    connected: Arc<AtomicBool>,
    shutdown: Arc<AtomicBool>,
    reconnect_config: Option<marketdata_core::ReconnectionConfig>,
    health_check_config: Option<marketdata_core::HealthCheckConfig>,
    tls_config: Option<marketdata_core::TlsConfig>,
    /// Tokio runtime for sync wrappers (C++ feature). Kept alive for background tasks.
    #[cfg(feature = "cpp")]
    sync_runtime: std::sync::Mutex<Option<tokio::runtime::Runtime>>,
}

impl WebSocketClient {
    /// Create a new WebSocket client (internal constructor)
    fn new_internal(
        api_key: String,
        listener: Arc<dyn WebSocketListener>,
        endpoint: WebSocketEndpoint,
        reconnect_config: Option<marketdata_core::ReconnectionConfig>,
        health_check_config: Option<marketdata_core::HealthCheckConfig>,
        base_url: Option<String>,
        tls_config: Option<marketdata_core::TlsConfig>,
        version: StreamingVersionRecord,
    ) -> Arc<Self> {
        Arc::new(Self {
            inner: Arc::new(Mutex::new(None)),
            listener,
            api_key,
            base_url,
            version,
            endpoint,
            connected: Arc::new(AtomicBool::new(false)),
            shutdown: Arc::new(AtomicBool::new(false)),
            reconnect_config,
            health_check_config,
            tls_config,
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
        Self::new_internal(api_key, listener, WebSocketEndpoint::Stock, None, None, None, None, Default::default())
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
        Self::new_internal(api_key, listener, endpoint, None, None, None, None, Default::default())
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
            api_key,
            listener,
            endpoint,
            reconnect_config.map(|c| c.to_core()),
            health_check_config.map(|c| c.to_core()),
            None,
            None,
            Default::default(),
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
            api_key,
            listener,
            endpoint,
            reconnect_config.map(|c| c.to_core()),
            health_check_config.map(|c| c.to_core()),
            Some(base_url),
            None,
            Default::default(),
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
            api_key,
            listener,
            endpoint,
            reconnect_config.map(|c| c.to_core()),
            health_check_config.map(|c| c.to_core()),
            base_url,
            tls.map(|t| t.to_core()),
            version.unwrap_or_default(),
        )
    }

    /// Check if the client is currently connected
    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }

    /// Check if the client has been shut down
    pub fn is_closed(&self) -> bool {
        self.shutdown.load(Ordering::SeqCst)
    }
}

#[cfg(not(feature = "cpp"))]
#[uniffi::export(async_runtime = "tokio")]
impl WebSocketClient {
    pub async fn connect(&self) -> Result<(), MarketDataError> {
        self.connect_impl().await
    }

    pub async fn subscribe(&self, channel: String, symbol: String) -> Result<(), MarketDataError> {
        self.subscribe_impl(channel, symbol).await
    }

    pub async fn unsubscribe(&self, channel: String, symbol: String) -> Result<(), MarketDataError> {
        self.unsubscribe_impl(channel, symbol).await
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
        let auth = AuthRequest::with_api_key(&self.api_key);
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

        // Connect to server
        core_ws.connect().await?;

        // CRITICAL: Obtain message receiver from core WebSocket API
        // The core WebSocket client exposes messages via client.messages() method
        // which returns Arc<MessageReceiver>
        let receiver: Arc<MessageReceiver> = core_ws.messages();

        // Capture state events receiver BEFORE storing core_ws (move would lose access)
        let events = Arc::clone(core_ws.state_events());

        // Store client in inner
        {
            let mut guard = self.inner.lock().await;
            *guard = Some(core_ws);
        }

        // Update connected state
        self.connected.store(true, Ordering::SeqCst);

        // Reset shutdown flag for this connection
        self.shutdown.store(false, Ordering::SeqCst);

        // Notify listener
        self.listener.on_connected();

        // Spawn dedicated thread for message forwarding (not tokio spawn_blocking)
        // Using a dedicated thread avoids per-message spawn_blocking overhead and
        // eliminates the 100ms polling timeout, delivering messages immediately.
        let listener = Arc::clone(&self.listener);
        let shutdown = Arc::clone(&self.shutdown);
        let connected = Arc::clone(&self.connected);
        std::thread::Builder::new()
            .name("ws_message_loop".to_string())
            .spawn(move || {
                run_message_loop_blocking(receiver, listener, shutdown, connected);
            })
            .ok();

        // Spawn thread to monitor state events and forward to listener
        let event_listener = Arc::clone(&self.listener);
        let event_connected = Arc::clone(&self.connected);
        let event_shutdown = Arc::clone(&self.shutdown);
        std::thread::Builder::new()
            .name("ws_event_monitor".to_string())
            .spawn(move || {
                loop {
                    if event_shutdown.load(Ordering::SeqCst) {
                        break;
                    }
                    let event = {
                        let rx = events.blocking_lock();
                        rx.recv()
                    };
                    match event {
                        Ok(event) => {
                            use marketdata_core::websocket::ConnectionEvent;
                            match event {
                                ConnectionEvent::Reconnecting { attempt, .. } => {
                                    event_listener.on_reconnecting(attempt);
                                }
                                ConnectionEvent::ReconnectFailed { attempts, .. } => {
                                    event_listener.on_reconnect_failed(attempts);
                                    event_connected.store(false, Ordering::SeqCst);
                                }
                                ConnectionEvent::Error { message, .. } => {
                                    event_listener.on_error(message);
                                }
                                ConnectionEvent::Disconnected { .. } => {
                                    event_listener.on_disconnected();
                                    event_connected.store(false, Ordering::SeqCst);
                                }
                                ConnectionEvent::Authenticated => {
                                    event_connected.store(true, Ordering::SeqCst);
                                }
                                // Map Unauthenticated to on_error so existing UniFFI
                                // listeners (Java/Go/C#) get notified of credential
                                // rejection without needing a new trait method.
                                // To expose a dedicated callback, add `on_unauthenticated`
                                // to the WebSocketListener trait and re-run uniffi-bindgen.
                                ConnectionEvent::Unauthenticated { message, .. } => {
                                    event_listener.on_error(format!(
                                        "Unauthenticated: {}",
                                        message
                                    ));
                                }
                                // Heartbeat timeout: route to on_disconnected for now.
                                // A dedicated on_heartbeat_timeout listener method can be
                                // added in a follow-up if user code needs to discriminate.
                                ConnectionEvent::HeartbeatTimeout { .. } => {
                                    event_listener.on_disconnected();
                                    event_connected.store(false, Ordering::SeqCst);
                                }
                                _ => {}
                            }
                        }
                        Err(_) => break, // Channel closed
                    }
                }
            })
            .ok();

        Ok(())
    }

    /// Subscribe to a channel for a symbol
    ///
    /// # Arguments
    /// * `channel` - Channel name (e.g., "trades", "candles", "books")
    /// * `symbol` - Symbol to subscribe (e.g., "2330")
    ///
    /// # Errors
    ///
    /// Returns error if not connected or subscription fails.
    async fn subscribe_impl(&self, channel: String, symbol: String) -> Result<(), MarketDataError> {
        let guard = self.inner.lock().await;
        if let Some(ref ws) = *guard {
            use marketdata_core::models::Channel;

            // Parse channel string to Channel enum
            let channel_enum = match channel.as_str() {
                "trades" => Channel::Trades,
                "candles" => Channel::Candles,
                "books" => Channel::Books,
                "aggregates" => Channel::Aggregates,
                "indices" => Channel::Indices,
                _ => {
                    return Err(MarketDataError::ConfigError {
                        msg: format!("Unknown channel: {}", channel),
                    })
                }
            };

            let sub = marketdata_core::StockSubscription::new(channel_enum, &symbol);
            ws.subscribe(sub).await?;
            Ok(())
        } else {
            Err(MarketDataError::WebSocketError {
                msg: "Not connected".to_string(),
            })
        }
    }

    /// Unsubscribe from a channel for a symbol
    ///
    /// # Arguments
    /// * `channel` - Channel name
    /// * `symbol` - Symbol to unsubscribe
    ///
    /// # Errors
    ///
    /// Returns error if not connected.
    async fn unsubscribe_impl(&self, channel: String, symbol: String) -> Result<(), MarketDataError> {
        let guard = self.inner.lock().await;
        if let Some(ref ws) = *guard {
            let key = format!("{}:{}", channel, symbol);
            ws.unsubscribe([key]).await?;
            Ok(())
        } else {
            Err(MarketDataError::WebSocketError {
                msg: "Not connected".to_string(),
            })
        }
    }

    /// Send a ping message to the server
    ///
    /// # Arguments
    /// * `state` - Optional state string echoed back in the pong response
    async fn ping_impl(&self, state: Option<String>) -> Result<(), MarketDataError> {
        let guard = self.inner.lock().await;
        if let Some(ref ws) = *guard {
            let request = marketdata_core::WebSocketRequest::ping(state);
            ws.send(request).await?;
            Ok(())
        } else {
            Err(MarketDataError::WebSocketError {
                msg: "Not connected".to_string(),
            })
        }
    }

    async fn query_subscriptions_impl(&self) -> Result<(), MarketDataError> {
        let guard = self.inner.lock().await;
        if let Some(ref ws) = *guard {
            let request = marketdata_core::WebSocketRequest::subscriptions();
            ws.send(request).await?;
            Ok(())
        } else {
            Err(MarketDataError::WebSocketError {
                msg: "Not connected".to_string(),
            })
        }
    }

    async fn disconnect_impl(&self) {
        // Signal shutdown to message loop
        self.shutdown.store(true, Ordering::SeqCst);

        // Take and disconnect the client
        let mut guard = self.inner.lock().await;
        if let Some(ws) = guard.take() {
            let _ = ws.disconnect().await;
        }

        // Update connected state
        self.connected.store(false, Ordering::SeqCst);

        // Notify listener
        self.listener.on_disconnected();
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
            .map_err(|e| MarketDataError::Other { msg: e.to_string() })?;
        let result = rt.block_on(self.connect_impl());
        // Store runtime to keep background tasks alive
        if let Ok(mut guard) = self.sync_runtime.lock() {
            *guard = Some(rt);
        }
        result
    }

    /// Subscribe to a channel for a symbol (blocking).
    pub fn subscribe_sync(&self, channel: String, symbol: String) -> Result<(), MarketDataError> {
        let guard = self.sync_runtime.lock().unwrap();
        if let Some(ref rt) = *guard {
            rt.block_on(self.subscribe_impl(channel, symbol))
        } else {
            Err(MarketDataError::WebSocketError { msg: "Not connected (call connect_sync first)".to_string() })
        }
    }

    /// Unsubscribe from a channel for a symbol (blocking).
    pub fn unsubscribe_sync(&self, channel: String, symbol: String) -> Result<(), MarketDataError> {
        let guard = self.sync_runtime.lock().unwrap();
        if let Some(ref rt) = *guard {
            rt.block_on(self.unsubscribe_impl(channel, symbol))
        } else {
            Err(MarketDataError::WebSocketError { msg: "Not connected".to_string() })
        }
    }

    /// Send a ping message (blocking).
    pub fn ping_sync(&self, state: Option<String>) -> Result<(), MarketDataError> {
        let guard = self.sync_runtime.lock().unwrap();
        if let Some(ref rt) = *guard {
            rt.block_on(self.ping_impl(state))
        } else {
            Err(MarketDataError::WebSocketError { msg: "Not connected".to_string() })
        }
    }

    /// Query server subscriptions (blocking).
    pub fn query_subscriptions_sync(&self) -> Result<(), MarketDataError> {
        let guard = self.sync_runtime.lock().unwrap();
        if let Some(ref rt) = *guard {
            rt.block_on(self.query_subscriptions_impl())
        } else {
            Err(MarketDataError::WebSocketError { msg: "Not connected".to_string() })
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

/// Dedicated thread that forwards messages from core MessageReceiver to listener.
///
/// Uses a short `receive_timeout` to allow periodic shutdown checks, but runs on
/// a dedicated thread (not tokio spawn_blocking) to avoid per-message task overhead.
/// The timeout is kept short (5ms) so messages are delivered with minimal latency.
fn run_message_loop_blocking(
    receiver: Arc<MessageReceiver>,
    listener: Arc<dyn WebSocketListener>,
    shutdown: Arc<AtomicBool>,
    connected: Arc<AtomicBool>,
) {
    use std::time::Duration;

    loop {
        if shutdown.load(Ordering::SeqCst) {
            break;
        }

        match receiver.receive_timeout(Duration::from_millis(5)) {
            Ok(Some(ws_msg)) => {
                let stream_msg = StreamMessage::from(ws_msg);
                listener.on_message(stream_msg);
            }
            Ok(None) => {
                // Timeout — loop back to check shutdown
                continue;
            }
            Err(_) => {
                // Channel closed
                break;
            }
        }
    }

    connected.store(false, Ordering::SeqCst);

    if !shutdown.load(Ordering::SeqCst) {
        listener.on_disconnected();
    }
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
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Test listener that tracks callback invocations
    struct TestListener {
        connected_count: AtomicUsize,
        disconnected_count: AtomicUsize,
        message_count: AtomicUsize,
        error_count: AtomicUsize,
        reconnecting_count: AtomicUsize,
        reconnect_failed_count: AtomicUsize,
        last_error: Mutex<Option<String>>,
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
                last_error: Mutex::new(None),
            }
        }
    }

    impl WebSocketListener for TestListener {
        fn on_connected(&self) {
            self.connected_count.fetch_add(1, Ordering::SeqCst);
        }

        fn on_disconnected(&self) {
            self.disconnected_count.fetch_add(1, Ordering::SeqCst);
        }

        fn on_message(&self, _message: StreamMessage) {
            self.message_count.fetch_add(1, Ordering::SeqCst);
        }

        fn on_error(&self, error_message: String) {
            self.error_count.fetch_add(1, Ordering::SeqCst);
            if let Ok(mut guard) = self.last_error.lock() {
                *guard = Some(error_message);
            }
        }

        fn on_reconnecting(&self, _attempt: u32) {
            self.reconnecting_count.fetch_add(1, Ordering::SeqCst);
        }

        fn on_reconnect_failed(&self, _attempts: u32) {
            self.reconnect_failed_count.fetch_add(1, Ordering::SeqCst);
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

        listener.on_disconnected();
        assert_eq!(listener.disconnected_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_websocket_listener_error_callback() {
        let listener = Arc::new(TestListener::new());

        listener.on_error("Test error".to_string());
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
}
