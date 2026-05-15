//! WebSocket client wrapper for JavaScript
//!
//! This module provides JavaScript-facing WebSocket clients using ThreadsafeFunction
//! for cross-thread callback invocation without blocking the Node.js event loop.
//!
//! Architecture:
//! - WebSocket connection runs in a dedicated background thread with its own tokio runtime
//! - Commands (connect, subscribe, disconnect) are sent via crossbeam channel
//! - Events (message, connect, disconnect, error) are delivered via ThreadsafeFunction callbacks

use napi::bindgen_prelude::Unknown;
use napi::threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode};
use napi::Status;

/// Single-argument event callback.
///
/// Uses napi-rs ThreadsafeFunction with `CalleeHandled = false` so the JS
/// callback signature is `(data) => void` instead of the default
/// `(err, data) => void`. This matches the legacy fugle-marketdata-node
/// SDK shape, e.g.:
/// ```js
/// stock.on('message', (data) => console.log(JSON.parse(data)));
/// ```
pub type EventTsfn = ThreadsafeFunction<String, Unknown<'static>, String, Status, false>;
use napi_derive::napi;
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::thread;
use std::time::Duration;

/// Type alias for JavaScript callback
/// In napi-rs 3.x, ThreadsafeFunction uses const generics instead of ErrorStrategy type.
/// Default CalleeHandled = true means the callee (JS function) handles errors.
/// We use Arc<ThreadsafeFunction> to allow cloning for use across threads.
pub type JsCallback = Arc<EventTsfn>;

/// Reconnection options for WebSocket clients
///
/// All fields are optional - defaults are applied when not specified:
/// - maxAttempts: 5
/// - initialDelayMs: 1000
/// - maxDelayMs: 60000
#[napi(object)]
#[derive(Debug, Clone, Default)]
pub struct ReconnectOptions {
    /// Maximum reconnection attempts (default: 5, min: 1)
    pub max_attempts: Option<u32>,
    /// Initial reconnection delay in milliseconds (default: 1000, min: 100)
    pub initial_delay_ms: Option<f64>,
    /// Maximum reconnection delay in milliseconds (default: 60000)
    pub max_delay_ms: Option<f64>,
}

/// Health check options for WebSocket connections
///
/// All fields are optional - defaults are applied when not specified:
/// - enabled: false
/// - pingInterval: 30000
/// - maxMissedPongs: 2
///
/// Defaults: enabled=true, heartbeatTimeoutMs=35000.
#[napi(object)]
#[derive(Debug, Clone, Default)]
pub struct HealthCheckOptions {
    /// Whether liveness detection is active (default: true in 3.0)
    pub enabled: Option<bool>,
    /// Maximum allowed gap between inbound frames before declaring the
    /// connection dead, in milliseconds. Default 35000 (Fugle server's
    /// 30s heartbeat + 5s buffer); floor 5000.
    pub heartbeat_timeout_ms: Option<f64>,
}

/// REST client options
///
/// Exactly ONE of apiKey, bearerToken, or sdkToken must be provided.
/// baseUrl is optional for custom endpoint override.
#[napi(object)]
#[derive(Default)]
pub struct RestClientOptions {
    /// API key for authentication
    pub api_key: Option<String>,
    /// Bearer token for authentication
    pub bearer_token: Option<String>,
    /// SDK token for authentication
    pub sdk_token: Option<String>,
    /// Override base URL (optional)
    pub base_url: Option<String>,
    /// Additional root CA (PEM bytes). Appended to the OS trust store;
    /// chains signed by either this CA or an OS-trusted root are accepted.
    pub tls_root_cert_pem: Option<napi::bindgen_prelude::Uint8Array>,
    /// Disable ALL TLS verification (chain + hostname + expiry).
    /// Dev/testing only — exposes MITM risk. Defaults to false.
    pub tls_accept_invalid_certs: Option<bool>,
}

/// WebSocket client options
///
/// Exactly ONE of apiKey, bearerToken, or sdkToken must be provided.
/// reconnect and healthCheck are optional configuration objects.
#[napi(object)]
#[derive(Default)]
pub struct WebSocketClientOptions {
    /// API key for authentication
    pub api_key: Option<String>,
    /// Bearer token for authentication
    pub bearer_token: Option<String>,
    /// SDK token for authentication
    pub sdk_token: Option<String>,
    /// Override base URL (optional)
    pub base_url: Option<String>,
    /// Reconnection configuration (optional)
    pub reconnect: Option<ReconnectOptions>,
    /// Health check configuration (optional)
    pub health_check: Option<HealthCheckOptions>,
    /// Additional root CA (PEM bytes). Appended to the OS trust store.
    pub tls_root_cert_pem: Option<napi::bindgen_prelude::Uint8Array>,
    /// Disable ALL TLS verification (chain + hostname + expiry).
    /// Dev/testing only — exposes MITM risk. Defaults to false.
    pub tls_accept_invalid_certs: Option<bool>,
}

/// Command sent to WebSocket worker thread
#[derive(Debug)]
enum WsCommand {
    Subscribe { channel: String, symbols: Vec<String>, extra: Option<bool> },
    Unsubscribe { ids: Vec<String> },
    /// Send a `ping` frame (mirrors old fugle-marketdata SDK's `ping()`)
    Ping { state: Option<String> },
    /// Ask the server for its current subscription list (response arrives via `message`)
    QuerySubscriptions,
    Disconnect,
}

/// Callback storage for event handlers
struct EventCallbacks {
    message: Option<JsCallback>,
    connect: Option<JsCallback>,
    disconnect: Option<JsCallback>,
    reconnect: Option<JsCallback>,
    error: Option<JsCallback>,
    authenticated: Option<JsCallback>,
    unauthenticated: Option<JsCallback>,
}

impl Default for EventCallbacks {
    fn default() -> Self {
        Self {
            message: None,
            connect: None,
            disconnect: None,
            reconnect: None,
            error: None,
            authenticated: None,
            unauthenticated: None,
        }
    }
}

/// WebSocket client for real-time market data (JavaScript wrapper)
///
/// # JavaScript Usage
///
/// ```javascript
/// const { WebSocketClient } = require('@fubon/marketdata-js');
///
/// // Create client with API key
/// const ws = new WebSocketClient('your-api-key');
///
/// // Register event handlers for stock data
/// ws.stock.on('message', (data) => console.log(JSON.parse(data)));
/// ws.stock.on('connect', () => console.log('Connected!'));
/// ws.stock.on('error', (err) => console.error(err));
///
/// // Connect and subscribe
/// ws.stock.connect();
/// ws.stock.subscribe({ channel: 'trades', symbol: '2330' });
/// ```
#[napi]
pub struct WebSocketClient {
    api_key: String,
    base_url: Option<String>,
    reconnect_config: marketdata_core::ReconnectionConfig,
    health_check_config: marketdata_core::HealthCheckConfig,
    tls_config: marketdata_core::TlsConfig,
    // Shared state for child clients — created once in constructor so that
    // every `ws.stock` / `ws.futopt` getter access shares the same Arcs.
    stock_callbacks: Arc<Mutex<EventCallbacks>>,
    stock_connected: Arc<AtomicBool>,
    stock_closed: Arc<AtomicBool>,
    stock_command_tx: Arc<Mutex<Option<std::sync::mpsc::Sender<WsCommand>>>>,
    futopt_callbacks: Arc<Mutex<EventCallbacks>>,
    futopt_connected: Arc<AtomicBool>,
    futopt_closed: Arc<AtomicBool>,
    futopt_command_tx: Arc<Mutex<Option<std::sync::mpsc::Sender<WsCommand>>>>,
}

#[napi]
impl WebSocketClient {
    /// Create a new WebSocket client with configuration
    ///
    /// @param options - Client configuration options
    /// @throws {Error} If validation fails (zero or multiple auth methods, invalid config values)
    ///
    /// @example
    /// ```javascript
    /// const { WebSocketClient } = require('@fugle/marketdata');
    ///
    /// // Simple usage with defaults
    /// const ws = new WebSocketClient({ apiKey: 'your-key' });
    ///
    /// // Custom reconnection config
    /// const ws = new WebSocketClient({
    ///   apiKey: 'your-key',
    ///   reconnect: { maxAttempts: 10, initialDelayMs: 2000 }
    /// });
    ///
    /// // Enable health check
    /// const ws = new WebSocketClient({
    ///   apiKey: 'your-key',
    ///   healthCheck: { enabled: true, pingInterval: 20000 }
    /// });
    /// ```
    #[napi(constructor)]
    pub fn new(options: WebSocketClientOptions) -> napi::Result<Self> {
        use marketdata_core::{
            DEFAULT_MAX_ATTEMPTS, DEFAULT_INITIAL_DELAY_MS, DEFAULT_MAX_DELAY_MS,
            DEFAULT_HEALTH_CHECK_ENABLED, DEFAULT_HEARTBEAT_TIMEOUT_MS,
        };
        use std::time::Duration;

        // Validate exactly one auth method (fail fast per CONTEXT.md)
        let auth_count = [
            options.api_key.is_some(),
            options.bearer_token.is_some(),
            options.sdk_token.is_some(),
        ]
        .iter()
        .filter(|&&x| x)
        .count();

        if auth_count == 0 {
            return Err(napi::Error::from_reason(
                "Provide exactly one of: apiKey, bearerToken, sdkToken"
            ));
        }

        if auth_count > 1 {
            return Err(napi::Error::from_reason(
                "Provide exactly one of: apiKey, bearerToken, sdkToken"
            ));
        }

        // Extract the one provided auth method
        let api_key = options.api_key
            .or(options.bearer_token)
            .or(options.sdk_token)
            .unwrap();

        // Build reconnection config with validation via core
        let reconnect_cfg = if let Some(r) = &options.reconnect {
            let max = r.max_attempts.map(|v| v as u32).unwrap_or(DEFAULT_MAX_ATTEMPTS);
            let initial = Duration::from_millis(
                r.initial_delay_ms.map(|v| v as u64).unwrap_or(DEFAULT_INITIAL_DELAY_MS)
            );
            let max_delay = Duration::from_millis(
                r.max_delay_ms.map(|v| v as u64).unwrap_or(DEFAULT_MAX_DELAY_MS)
            );
            marketdata_core::ReconnectionConfig::new(max, initial, max_delay)
                .map_err(|e| napi::Error::from_reason(e.to_string()))?
        } else {
            marketdata_core::ReconnectionConfig::default()
        };

        // Build health check config with validation via core
        let health_check_cfg = if let Some(hc) = &options.health_check {
            let enabled = hc.enabled.unwrap_or(DEFAULT_HEALTH_CHECK_ENABLED);
            let timeout = Duration::from_millis(
                hc.heartbeat_timeout_ms.map(|v| v as u64).unwrap_or(DEFAULT_HEARTBEAT_TIMEOUT_MS)
            );
            let mut cfg = marketdata_core::HealthCheckConfig::with_timeout(timeout)
                .map_err(|e| napi::Error::from_reason(e.to_string()))?;
            cfg.enabled = enabled;
            cfg
        } else {
            marketdata_core::HealthCheckConfig::default()
        };

        // Build TLS config from options. Default config matches previous
        // behaviour (OS trust store, no overrides); custom CA / accept_invalid
        // flow through to the worker thread's ConnectionConfig.
        let tls_config = marketdata_core::TlsConfig {
            root_cert_pem: options.tls_root_cert_pem.map(|arr| arr.to_vec()),
            accept_invalid_certs: options.tls_accept_invalid_certs.unwrap_or(false),
        };

        Ok(Self {
            api_key,
            base_url: options.base_url,
            reconnect_config: reconnect_cfg,
            health_check_config: health_check_cfg,
            tls_config,
            stock_callbacks: Arc::new(Mutex::new(EventCallbacks::default())),
            stock_connected: Arc::new(AtomicBool::new(false)),
            stock_closed: Arc::new(AtomicBool::new(false)),
            stock_command_tx: Arc::new(Mutex::new(None)),
            futopt_callbacks: Arc::new(Mutex::new(EventCallbacks::default())),
            futopt_connected: Arc::new(AtomicBool::new(false)),
            futopt_closed: Arc::new(AtomicBool::new(false)),
            futopt_command_tx: Arc::new(Mutex::new(None)),
        })
    }

    /// Get the stock WebSocket client for real-time stock data.
    ///
    /// Every access returns a new JS wrapper but all wrappers share the same
    /// underlying state (callbacks, connected flag, command channel), so the
    /// legacy `ws.stock.on(...); ws.stock.connect()` pattern works correctly.
    #[napi(getter)]
    pub fn stock(&self) -> StockWebSocketClient {
        StockWebSocketClient::from_shared(
            self.api_key.clone(),
            self.base_url.clone(),
            self.reconnect_config.clone(),
            self.health_check_config.clone(),
            self.tls_config.clone(),
            Arc::clone(&self.stock_callbacks),
            Arc::clone(&self.stock_connected),
            Arc::clone(&self.stock_closed),
            Arc::clone(&self.stock_command_tx),
        )
    }

    /// Get the FutOpt WebSocket client for real-time futures/options data.
    ///
    /// Same shared-state semantics as `stock` — see its doc comment.
    #[napi(getter)]
    pub fn futopt(&self) -> FutOptWebSocketClient {
        FutOptWebSocketClient::from_shared(
            self.api_key.clone(),
            self.base_url.clone(),
            self.reconnect_config.clone(),
            self.health_check_config.clone(),
            self.tls_config.clone(),
            Arc::clone(&self.futopt_callbacks),
            Arc::clone(&self.futopt_connected),
            Arc::clone(&self.futopt_closed),
            Arc::clone(&self.futopt_command_tx),
        )
    }
}

/// Stock WebSocket client for real-time stock market data
///
/// # JavaScript Usage
///
/// ```javascript
/// // Event handlers
/// ws.stock.on('message', (data) => {
///   const msg = JSON.parse(data);
///   console.log(msg);
/// });
/// ws.stock.on('connect', () => console.log('Stock WebSocket connected'));
/// ws.stock.on('disconnect', (reason) => console.log('Disconnected:', reason));
/// ws.stock.on('reconnect', (info) => console.log('Reconnecting:', info));
/// ws.stock.on('error', (err) => console.error('Error:', err));
///
/// // Connect
/// ws.stock.connect();
///
/// // Subscribe to channels
/// ws.stock.subscribe({ channel: 'trades', symbol: '2330' });
/// ws.stock.subscribe({ channel: 'candles', symbol: '2330' });
/// ```
#[napi]
pub struct StockWebSocketClient {
    api_key: String,
    base_url: Option<String>,
    reconnect_config: marketdata_core::ReconnectionConfig,
    health_check_config: marketdata_core::HealthCheckConfig,
    tls_config: marketdata_core::TlsConfig,
    callbacks: Arc<Mutex<EventCallbacks>>,
    connected: Arc<AtomicBool>,
    closed: Arc<AtomicBool>,
    command_tx: Arc<Mutex<Option<std::sync::mpsc::Sender<WsCommand>>>>,
}

#[napi]
impl StockWebSocketClient {
    /// Create from pre-existing shared state (called by WebSocketClient getter).
    /// All mutable state lives behind Arc so multiple JS wrappers returned by
    /// the `ws.stock` getter share the same underlying callbacks, connection
    /// flag, and command channel.
    fn from_shared(
        api_key: String,
        base_url: Option<String>,
        reconnect_config: marketdata_core::ReconnectionConfig,
        health_check_config: marketdata_core::HealthCheckConfig,
        tls_config: marketdata_core::TlsConfig,
        callbacks: Arc<Mutex<EventCallbacks>>,
        connected: Arc<AtomicBool>,
        closed: Arc<AtomicBool>,
        command_tx: Arc<Mutex<Option<std::sync::mpsc::Sender<WsCommand>>>>,
    ) -> Self {
        Self {
            api_key,
            base_url,
            reconnect_config,
            health_check_config,
            tls_config,
            callbacks,
            connected,
            closed,
            command_tx,
        }
    }

    /// Register an event handler
    ///
    /// @param event - Event type: "message", "connect", "disconnect", "reconnect", "error"
    /// @param callback - JavaScript callback function receiving string data
    ///
    /// @example
    /// ```javascript
    /// ws.stock.on('message', (data) => console.log(data));
    /// ws.stock.on('connect', () => console.log('Connected'));
    /// ws.stock.on('error', (err) => console.error(err));
    /// ```
    #[napi(ts_args_type = "event: WebSocketEvent, callback: (data: string) => void")]
    pub fn on(&self, event: String, callback: EventTsfn) -> napi::Result<()> {
        let mut callbacks = self
            .callbacks
            .lock()
            .map_err(|e| napi::Error::from_reason(format!("Lock error: {}", e)))?;

        // Wrap in Arc for thread-safe sharing (napi-rs 3.x pattern)
        let arc_callback = Arc::new(callback);

        match event.as_str() {
            "message" => callbacks.message = Some(arc_callback),
            "connect" => callbacks.connect = Some(arc_callback),
            "disconnect" => callbacks.disconnect = Some(arc_callback),
            "reconnect" => callbacks.reconnect = Some(arc_callback),
            "error" => callbacks.error = Some(arc_callback),
            "authenticated" => callbacks.authenticated = Some(arc_callback),
            "unauthenticated" => callbacks.unauthenticated = Some(arc_callback),
            _ => {
                return Err(napi::Error::from_reason(format!(
                    "Unknown event type: {}. Valid events: message, connect, disconnect, reconnect, error, authenticated, unauthenticated",
                    event
                )))
            }
        }
        Ok(())
    }

    /// Connect to the stock WebSocket server.
    ///
    /// Returns a Promise that resolves when authentication completes, matching
    /// the legacy fugle-marketdata Node SDK shape:
    ///
    /// ```js
    /// stock.connect().then(() => {
    ///   stock.subscribe({ channel: 'trades', symbol: '2330' });
    /// });
    /// ```
    ///
    /// On rejection, the Promise carries the underlying error message. The
    /// `connect` event callback also fires after the Promise resolves, so
    /// existing callback-style code keeps working.
    #[napi]
    pub async fn connect(&self) -> napi::Result<()> {
        // Create command channel
        let (cmd_tx, cmd_rx) = std::sync::mpsc::channel::<WsCommand>();

        // Store command sender
        {
            let mut tx_guard = self.command_tx.lock().map_err(|e| {
                napi::Error::from_reason(format!("Lock error: {}", e))
            })?;
            *tx_guard = Some(cmd_tx);
        }

        // Oneshot channel for auth completion signal back to the awaiting Promise.
        let (auth_tx, auth_rx) = tokio::sync::oneshot::channel::<Result<(), String>>();

        // Clone data for the worker thread
        let api_key = self.api_key.clone();
        let base_url = self.base_url.clone();
        let reconnect_config = self.reconnect_config.clone();
        let health_check_config = self.health_check_config.clone();
        let tls_config = self.tls_config.clone();
        let callbacks = Arc::clone(&self.callbacks);
        let connected = Arc::clone(&self.connected);
        let closed = Arc::clone(&self.closed);

        // Spawn worker thread that owns WebSocketClient
        thread::Builder::new()
            .name("stock_ws_worker".to_string())
            .spawn(move || {
                use marketdata_core::aio::WebSocketClient as CoreClient;
                use marketdata_core::websocket::ConnectionConfig;
                use marketdata_core::AuthRequest;
                use marketdata_core::models::Channel;
                use marketdata_core::websocket::channels::StockSubscription;

                // Create tokio runtime
                // Multi-thread runtime so core's dispatch/writer/health-check
                // tasks keep running while the worker loop blocks on std::mpsc
                // receive_timeout. With a current_thread runtime those tasks
                // starve the moment the worker stops driving the executor,
                // causing incoming frames (subscribed, snapshot, heartbeat...)
                // to stall in tokio-tungstenite's buffer.
                let rt = match tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(2)
                    .enable_all()
                    .build()
                {
                    Ok(rt) => rt,
                    Err(e) => {
                        let msg = format!("Failed to create runtime: {}", e);
                        let _ = auth_tx.send(Err(msg.clone()));
                        fire_callback(&callbacks, "error", msg);
                        return;
                    }
                };

                // Build connection config. If base_url is provided, append the
                // stock streaming path (legacy SDK parity); otherwise fall back
                // to the production Fugle endpoint.
                let auth = AuthRequest::with_api_key(&api_key);
                let mut config = match base_url {
                    Some(base) => {
                        let url = format!("{}/stock/streaming", base.trim_end_matches('/'));
                        ConnectionConfig::new(url, auth)
                    }
                    None => ConnectionConfig::fugle_stock(auth),
                };
                config.tls = tls_config;
                let client = CoreClient::with_full_config(config, reconnect_config, health_check_config);

                // Connect (core's connect().await returns once authenticated)
                let connect_result = rt.block_on(async {
                    client.connect().await
                });

                if let Err(e) = connect_result {
                    let msg = format!("[{}] {}", e.to_error_code(), e);
                    let _ = auth_tx.send(Err(msg.clone()));
                    fire_callback(&callbacks, "error", msg);
                    return;
                }

                // Signal the awaiting Promise that authentication succeeded.
                let _ = auth_tx.send(Ok(()));

                // Mark as connected
                connected.store(true, Ordering::SeqCst);
                fire_callback(&callbacks, "connect", "connected".to_string());

                // Monitor state events for reconnect/error callbacks
                let events = Arc::clone(client.state_events());
                let callbacks_for_events = Arc::clone(&callbacks);
                std::thread::spawn(move || {
                    loop {
                        let event = {
                            let rx = events.blocking_lock();
                            rx.recv()
                        };
                        match event {
                            Ok(event) => {
                                use marketdata_core::websocket::ConnectionEvent;
                                match event {
                                    ConnectionEvent::Reconnecting { attempt } => {
                                        fire_callback(&callbacks_for_events, "reconnect", format!("{{\"attempt\":{}}}", attempt));
                                    }
                                    ConnectionEvent::Error { message, code } => {
                                        fire_callback(&callbacks_for_events, "error", format!("[{}] {}", code, message));
                                    }
                                    ConnectionEvent::Disconnected { code, reason } => {
                                        fire_callback(&callbacks_for_events, "disconnect", format!("{{\"code\":{},\"reason\":\"{}\"}}", code.unwrap_or(0), reason));
                                    }
                                    ConnectionEvent::ReconnectFailed { attempts } => {
                                        fire_callback(&callbacks_for_events, "error", format!("Reconnection failed after {} attempts", attempts));
                                    }
                                    ConnectionEvent::Authenticated => {
                                        fire_callback(&callbacks_for_events, "authenticated", "authenticated".to_string());
                                    }
                                    ConnectionEvent::Unauthenticated { message } => {
                                        fire_callback(&callbacks_for_events, "unauthenticated", message);
                                    }
                                    ConnectionEvent::HeartbeatTimeout { elapsed } => {
                                        // Reuse "disconnect" callback with synthesized reason
                                        fire_callback(&callbacks_for_events, "disconnect",
                                            format!("{{\"code\":null,\"reason\":\"Heartbeat timeout after {:?}\"}}", elapsed));
                                    }
                                    _ => {}
                                }
                            }
                            Err(_) => break,
                        }
                    }
                });

                // Get message receiver
                let receiver = client.messages();

                // Main event loop
                loop {
                    // Check for commands (non-blocking)
                    match cmd_rx.try_recv() {
                        Ok(WsCommand::Subscribe { channel, symbols, extra }) => {
                            let channel_enum = match channel.to_lowercase().as_str() {
                                "trades" => Channel::Trades,
                                "candles" => Channel::Candles,
                                "books" => Channel::Books,
                                "aggregates" => Channel::Aggregates,
                                "indices" => Channel::Indices,
                                _ => continue,
                            };
                            let odd_lot = extra.unwrap_or(false);
                            let sub = StockSubscription::new(channel_enum, symbols).with_odd_lot(odd_lot);
                            let _ = rt.block_on(client.subscribe(sub));
                        }
                        Ok(WsCommand::Unsubscribe { ids }) => {
                            let _ = rt.block_on(client.unsubscribe(ids));
                        }
                        Ok(WsCommand::Ping { state }) => {
                            let request = marketdata_core::WebSocketRequest::ping(state);
                            let _ = rt.block_on(client.send(request));
                        }
                        Ok(WsCommand::QuerySubscriptions) => {
                            let request = marketdata_core::WebSocketRequest::subscriptions();
                            let _ = rt.block_on(client.send(request));
                        }
                        Ok(WsCommand::Disconnect) => {
                            let _ = rt.block_on(client.disconnect());
                            connected.store(false, Ordering::SeqCst);
                            closed.store(true, Ordering::SeqCst);
                            fire_callback(&callbacks, "disconnect", "disconnected".to_string());
                            break;
                        }
                        Err(std::sync::mpsc::TryRecvError::Empty) => {}
                        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                            // Command channel closed, cleanup
                            let _ = rt.block_on(client.disconnect());
                            connected.store(false, Ordering::SeqCst);
                            closed.store(true, Ordering::SeqCst);
                            break;
                        }
                    }

                    // Check for messages (with timeout)
                    match receiver.receive_timeout(Duration::from_millis(50)) {
                        Ok(Some(msg)) => {
                            if let Ok(json_str) = serde_json::to_string(&msg) {
                                fire_callback(&callbacks, "message", json_str);
                            }
                        }
                        Ok(None) => {
                            // Timeout, continue loop
                        }
                        Err(_) => {
                            // Channel closed
                            connected.store(false, Ordering::SeqCst);
                            closed.store(true, Ordering::SeqCst);
                            fire_callback(&callbacks, "disconnect", "channel_closed".to_string());
                            break;
                        }
                    }
                }
            })
            .map_err(|e| napi::Error::from_reason(format!("Failed to spawn worker thread: {}", e)))?;

        // Await the auth signal from the worker thread so the returned
        // Promise resolves only after authentication completes.
        match auth_rx.await {
            Ok(Ok(())) => Ok(()),
            Ok(Err(msg)) => Err(napi::Error::from_reason(msg)),
            Err(_) => Err(napi::Error::from_reason(
                "Worker thread terminated before authentication signal",
            )),
        }
    }

    /// Subscribe to a channel
    ///
    /// @param options - Subscription options. Provide either `symbol` (single)
    ///                  or `symbols` (batch list) — exactly one is required, matching
    ///                  the old `@fugle/marketdata` shape.
    ///                  Shape: `{ channel, symbol?, symbols?, intradayOddLot? }`
    #[napi(ts_args_type = "options: StockSubscribeOptions")]
    pub fn subscribe(&self, options: serde_json::Value) -> napi::Result<()> {
        let channel_str = options
            .get("channel")
            .and_then(|v| v.as_str())
            .ok_or_else(|| napi::Error::from_reason("Missing 'channel' field"))?;

        let single_symbol = options.get("symbol").and_then(|v| v.as_str()).map(String::from);
        let batch_symbols = options
            .get("symbols")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect::<Vec<_>>()
            });

        let target_symbols: Vec<String> = match (single_symbol, batch_symbols) {
            (Some(s), None) => vec![s],
            (None, Some(list)) if !list.is_empty() => list,
            (None, Some(_)) => {
                return Err(napi::Error::from_reason(
                    "subscribe({symbols:[]}) is empty - provide at least one symbol",
                ));
            }
            (Some(_), Some(_)) => {
                return Err(napi::Error::from_reason(
                    "subscribe() accepts either 'symbol' or 'symbols', not both",
                ));
            }
            (None, None) => {
                return Err(napi::Error::from_reason(
                    "subscribe() requires 'symbol' or 'symbols'",
                ));
            }
        };

        let odd_lot = options.get("intradayOddLot").and_then(|v| v.as_bool());

        let tx_guard = self.command_tx.lock().map_err(|e| {
            napi::Error::from_reason(format!("Lock error: {}", e))
        })?;

        let tx = tx_guard.as_ref().ok_or_else(|| {
            napi::Error::from_reason("Not connected. Call connect() first.")
        })?;

        tx.send(WsCommand::Subscribe {
            channel: channel_str.to_string(),
            symbols: target_symbols,
            extra: odd_lot,
        })
        .map_err(|_| napi::Error::from_reason("Failed to send subscribe command"))?;

        Ok(())
    }

    /// Unsubscribe from a channel
    ///
    /// Accepts either `{ id: "..." }` (single) or `{ ids: ["...", "..."] }` (batch).
    /// Mirrors the old `@fugle/marketdata` Node SDK shape.
    #[napi(ts_args_type = "options: string | UnsubscribeOptions")]
    pub fn unsubscribe(&self, options: serde_json::Value) -> napi::Result<()> {
        // Accept legacy positional string for backward compat with the previous
        // `unsubscribe(id: string)` signature.
        let target_ids: Vec<String> = if let Some(s) = options.as_str() {
            vec![s.to_string()]
        } else {
            let single = options.get("id").and_then(|v| v.as_str()).map(String::from);
            let batch = options
                .get("ids")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect::<Vec<_>>()
                });

            match (single, batch) {
                (Some(id), None) => vec![id],
                (None, Some(list)) if !list.is_empty() => list,
                (None, Some(_)) => {
                    return Err(napi::Error::from_reason(
                        "unsubscribe({ids:[]}) is empty - provide at least one id",
                    ));
                }
                (Some(_), Some(_)) => {
                    return Err(napi::Error::from_reason(
                        "unsubscribe() accepts either 'id' or 'ids', not both",
                    ));
                }
                (None, None) => {
                    return Err(napi::Error::from_reason(
                        "unsubscribe() requires 'id' or 'ids'",
                    ));
                }
            }
        };

        let tx_guard = self.command_tx.lock().map_err(|e| {
            napi::Error::from_reason(format!("Lock error: {}", e))
        })?;

        let tx = tx_guard.as_ref().ok_or_else(|| {
            napi::Error::from_reason("Not connected. Call connect() first.")
        })?;

        tx.send(WsCommand::Unsubscribe { ids: target_ids })
            .map_err(|_| napi::Error::from_reason("Failed to send unsubscribe command"))?;

        Ok(())
    }

    /// Send a `ping` frame to the server.
    ///
    /// Mirrors the old `@fugle/marketdata` Node SDK. The server's `pong` reply
    /// is delivered via the `message` callback (or processed internally by the
    /// health check, if enabled).
    ///
    /// @param state - Optional state string echoed back in the server's pong reply
    #[napi]
    pub fn ping(&self, state: Option<String>) -> napi::Result<()> {
        let tx_guard = self
            .command_tx
            .lock()
            .map_err(|e| napi::Error::from_reason(format!("Lock error: {}", e)))?;
        let tx = tx_guard
            .as_ref()
            .ok_or_else(|| napi::Error::from_reason("Not connected. Call connect() first."))?;
        tx.send(WsCommand::Ping { state })
            .map_err(|_| napi::Error::from_reason("Failed to send ping command"))?;
        Ok(())
    }

    /// Ask the server for its current subscription list.
    ///
    /// Sends `{ event: "subscriptions" }` to the server. The reply is delivered
    /// asynchronously via the `message` callback, matching the old
    /// `@fugle/marketdata` Node SDK semantics.
    #[napi]
    pub fn subscriptions(&self) -> napi::Result<()> {
        let tx_guard = self
            .command_tx
            .lock()
            .map_err(|e| napi::Error::from_reason(format!("Lock error: {}", e)))?;
        let tx = tx_guard
            .as_ref()
            .ok_or_else(|| napi::Error::from_reason("Not connected. Call connect() first."))?;
        tx.send(WsCommand::QuerySubscriptions)
            .map_err(|_| napi::Error::from_reason("Failed to send subscriptions command"))?;
        Ok(())
    }

    /// Disconnect from the WebSocket server
    #[napi]
    pub fn disconnect(&self) -> napi::Result<()> {
        let tx_guard = self.command_tx.lock().map_err(|e| {
            napi::Error::from_reason(format!("Lock error: {}", e))
        })?;

        if let Some(ref tx) = *tx_guard {
            let _ = tx.send(WsCommand::Disconnect);
        }

        Ok(())
    }

    /// Check if connected
    #[napi(getter)]
    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }

    /// Check if client has been closed
    ///
    /// Returns true if disconnect() has been called and client is closed.
    /// Once closed, the client cannot be reused - create a new instance.
    #[napi(getter)]
    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst)
    }
}

/// FutOpt WebSocket client for real-time futures/options market data
///
/// # JavaScript Usage
///
/// ```javascript
/// // Event handlers
/// ws.futopt.on('message', (data) => {
///   const msg = JSON.parse(data);
///   console.log(msg);
/// });
/// ws.futopt.on('connect', () => console.log('FutOpt WebSocket connected'));
///
/// // Connect
/// ws.futopt.connect();
///
/// // Subscribe to channels
/// ws.futopt.subscribe({ channel: 'trades', symbol: 'TXFC4' });
/// ws.futopt.subscribe({ channel: 'books', symbol: 'MXFB4', afterHours: true });
/// ```
#[napi]
pub struct FutOptWebSocketClient {
    api_key: String,
    base_url: Option<String>,
    reconnect_config: marketdata_core::ReconnectionConfig,
    health_check_config: marketdata_core::HealthCheckConfig,
    tls_config: marketdata_core::TlsConfig,
    callbacks: Arc<Mutex<EventCallbacks>>,
    connected: Arc<AtomicBool>,
    closed: Arc<AtomicBool>,
    command_tx: Arc<Mutex<Option<std::sync::mpsc::Sender<WsCommand>>>>,
}

#[napi]
impl FutOptWebSocketClient {
    /// Create from pre-existing shared state (called by WebSocketClient getter).
    /// See StockWebSocketClient::from_shared for rationale.
    fn from_shared(
        api_key: String,
        base_url: Option<String>,
        reconnect_config: marketdata_core::ReconnectionConfig,
        health_check_config: marketdata_core::HealthCheckConfig,
        tls_config: marketdata_core::TlsConfig,
        callbacks: Arc<Mutex<EventCallbacks>>,
        connected: Arc<AtomicBool>,
        closed: Arc<AtomicBool>,
        command_tx: Arc<Mutex<Option<std::sync::mpsc::Sender<WsCommand>>>>,
    ) -> Self {
        Self {
            api_key,
            base_url,
            reconnect_config,
            health_check_config,
            tls_config,
            callbacks,
            connected,
            closed,
            command_tx,
        }
    }

    /// Register an event handler
    ///
    /// @param event - Event type: "message", "connect", "disconnect", "reconnect", "error"
    /// @param callback - JavaScript callback function receiving string data
    #[napi(ts_args_type = "event: WebSocketEvent, callback: (data: string) => void")]
    pub fn on(&self, event: String, callback: EventTsfn) -> napi::Result<()> {
        let mut callbacks = self
            .callbacks
            .lock()
            .map_err(|e| napi::Error::from_reason(format!("Lock error: {}", e)))?;

        // Wrap in Arc for thread-safe sharing (napi-rs 3.x pattern)
        let arc_callback = Arc::new(callback);

        match event.as_str() {
            "message" => callbacks.message = Some(arc_callback),
            "connect" => callbacks.connect = Some(arc_callback),
            "disconnect" => callbacks.disconnect = Some(arc_callback),
            "reconnect" => callbacks.reconnect = Some(arc_callback),
            "error" => callbacks.error = Some(arc_callback),
            "authenticated" => callbacks.authenticated = Some(arc_callback),
            "unauthenticated" => callbacks.unauthenticated = Some(arc_callback),
            _ => {
                return Err(napi::Error::from_reason(format!(
                    "Unknown event type: {}. Valid events: message, connect, disconnect, reconnect, error, authenticated, unauthenticated",
                    event
                )))
            }
        }
        Ok(())
    }

    /// Connect to the FutOpt WebSocket server.
    ///
    /// Returns a Promise that resolves when authentication completes.
    /// See `StockWebSocketClient::connect` for the rationale and example.
    #[napi]
    pub async fn connect(&self) -> napi::Result<()> {
        let (cmd_tx, cmd_rx) = std::sync::mpsc::channel::<WsCommand>();

        {
            let mut tx_guard = self.command_tx.lock().map_err(|e| {
                napi::Error::from_reason(format!("Lock error: {}", e))
            })?;
            *tx_guard = Some(cmd_tx);
        }

        let (auth_tx, auth_rx) = tokio::sync::oneshot::channel::<Result<(), String>>();

        let api_key = self.api_key.clone();
        let base_url = self.base_url.clone();
        let reconnect_config = self.reconnect_config.clone();
        let health_check_config = self.health_check_config.clone();
        let tls_config = self.tls_config.clone();
        let callbacks = Arc::clone(&self.callbacks);
        let connected = Arc::clone(&self.connected);
        let closed = Arc::clone(&self.closed);

        thread::Builder::new()
            .name("futopt_ws_worker".to_string())
            .spawn(move || {
                use marketdata_core::aio::WebSocketClient as CoreClient;
                use marketdata_core::websocket::ConnectionConfig;
                use marketdata_core::AuthRequest;
                use marketdata_core::models::futopt::FutOptChannel;
                use marketdata_core::websocket::channels::FutOptSubscription;

                // Multi-thread runtime so core's dispatch/writer/health-check
                // tasks keep running while the worker loop blocks on std::mpsc
                // receive_timeout. With a current_thread runtime those tasks
                // starve the moment the worker stops driving the executor,
                // causing incoming frames (subscribed, snapshot, heartbeat...)
                // to stall in tokio-tungstenite's buffer.
                let rt = match tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(2)
                    .enable_all()
                    .build()
                {
                    Ok(rt) => rt,
                    Err(e) => {
                        let msg = format!("Failed to create runtime: {}", e);
                        let _ = auth_tx.send(Err(msg.clone()));
                        fire_callback(&callbacks, "error", msg);
                        return;
                    }
                };

                // Build connection config. If base_url is provided, append the
                // futopt streaming path (legacy SDK parity).
                let auth = AuthRequest::with_api_key(&api_key);
                let mut config = match base_url {
                    Some(base) => {
                        let url = format!("{}/futopt/streaming", base.trim_end_matches('/'));
                        ConnectionConfig::new(url, auth)
                    }
                    None => ConnectionConfig::fugle_futopt(auth),
                };
                config.tls = tls_config;
                let client = CoreClient::with_full_config(config, reconnect_config, health_check_config);

                let connect_result = rt.block_on(async {
                    client.connect().await
                });

                if let Err(e) = connect_result {
                    let msg = format!("[{}] {}", e.to_error_code(), e);
                    let _ = auth_tx.send(Err(msg.clone()));
                    fire_callback(&callbacks, "error", msg);
                    return;
                }

                let _ = auth_tx.send(Ok(()));

                connected.store(true, Ordering::SeqCst);
                fire_callback(&callbacks, "connect", "connected".to_string());

                // Monitor state events for reconnect/error callbacks
                let events = Arc::clone(client.state_events());
                let callbacks_for_events = Arc::clone(&callbacks);
                std::thread::spawn(move || {
                    loop {
                        let event = {
                            let rx = events.blocking_lock();
                            rx.recv()
                        };
                        match event {
                            Ok(event) => {
                                use marketdata_core::websocket::ConnectionEvent;
                                match event {
                                    ConnectionEvent::Reconnecting { attempt } => {
                                        fire_callback(&callbacks_for_events, "reconnect", format!("{{\"attempt\":{}}}", attempt));
                                    }
                                    ConnectionEvent::Error { message, code } => {
                                        fire_callback(&callbacks_for_events, "error", format!("[{}] {}", code, message));
                                    }
                                    ConnectionEvent::Disconnected { code, reason } => {
                                        fire_callback(&callbacks_for_events, "disconnect", format!("{{\"code\":{},\"reason\":\"{}\"}}", code.unwrap_or(0), reason));
                                    }
                                    ConnectionEvent::ReconnectFailed { attempts } => {
                                        fire_callback(&callbacks_for_events, "error", format!("Reconnection failed after {} attempts", attempts));
                                    }
                                    ConnectionEvent::Authenticated => {
                                        fire_callback(&callbacks_for_events, "authenticated", "authenticated".to_string());
                                    }
                                    ConnectionEvent::Unauthenticated { message } => {
                                        fire_callback(&callbacks_for_events, "unauthenticated", message);
                                    }
                                    ConnectionEvent::HeartbeatTimeout { elapsed } => {
                                        // Reuse "disconnect" callback with synthesized reason
                                        fire_callback(&callbacks_for_events, "disconnect",
                                            format!("{{\"code\":null,\"reason\":\"Heartbeat timeout after {:?}\"}}", elapsed));
                                    }
                                    _ => {}
                                }
                            }
                            Err(_) => break,
                        }
                    }
                });

                // Get message receiver
                let receiver = client.messages();

                // Main event loop
                loop {
                    // Check for commands (non-blocking)
                    match cmd_rx.try_recv() {
                        Ok(WsCommand::Subscribe { channel, symbols, extra }) => {
                            let channel_enum = match channel.to_lowercase().as_str() {
                                "trades" => FutOptChannel::Trades,
                                "candles" => FutOptChannel::Candles,
                                "books" => FutOptChannel::Books,
                                "aggregates" => FutOptChannel::Aggregates,
                                _ => continue,
                            };
                            let after_hours = extra.unwrap_or(false);
                            let sub = FutOptSubscription::new(channel_enum, symbols).with_after_hours(after_hours);
                            let _ = rt.block_on(client.subscribe_futopt(sub));
                        }
                        Ok(WsCommand::Unsubscribe { ids }) => {
                            let _ = rt.block_on(client.unsubscribe(ids));
                        }
                        Ok(WsCommand::Ping { state }) => {
                            let request = marketdata_core::WebSocketRequest::ping(state);
                            let _ = rt.block_on(client.send(request));
                        }
                        Ok(WsCommand::QuerySubscriptions) => {
                            let request = marketdata_core::WebSocketRequest::subscriptions();
                            let _ = rt.block_on(client.send(request));
                        }
                        Ok(WsCommand::Disconnect) => {
                            let _ = rt.block_on(client.disconnect());
                            connected.store(false, Ordering::SeqCst);
                            closed.store(true, Ordering::SeqCst);
                            fire_callback(&callbacks, "disconnect", "disconnected".to_string());
                            break;
                        }
                        Err(std::sync::mpsc::TryRecvError::Empty) => {}
                        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                            // Command channel closed, cleanup
                            let _ = rt.block_on(client.disconnect());
                            connected.store(false, Ordering::SeqCst);
                            closed.store(true, Ordering::SeqCst);
                            break;
                        }
                    }

                    // Check for messages (with timeout)
                    match receiver.receive_timeout(Duration::from_millis(50)) {
                        Ok(Some(msg)) => {
                            if let Ok(json_str) = serde_json::to_string(&msg) {
                                fire_callback(&callbacks, "message", json_str);
                            }
                        }
                        Ok(None) => {
                            // Timeout, continue loop
                        }
                        Err(_) => {
                            // Channel closed
                            connected.store(false, Ordering::SeqCst);
                            closed.store(true, Ordering::SeqCst);
                            fire_callback(&callbacks, "disconnect", "channel_closed".to_string());
                            break;
                        }
                    }
                }
            })
            .map_err(|e| napi::Error::from_reason(format!("Failed to spawn worker thread: {}", e)))?;

        match auth_rx.await {
            Ok(Ok(())) => Ok(()),
            Ok(Err(msg)) => Err(napi::Error::from_reason(msg)),
            Err(_) => Err(napi::Error::from_reason(
                "Worker thread terminated before authentication signal",
            )),
        }
    }

    /// Subscribe to a channel
    ///
    /// @param options - Subscription options. Provide either `symbol` (single)
    ///                  or `symbols` (batch list) — exactly one is required.
    ///                  Shape: `{ channel, symbol?, symbols?, afterHours? }`
    #[napi(ts_args_type = "options: FutOptSubscribeOptions")]
    pub fn subscribe(&self, options: serde_json::Value) -> napi::Result<()> {
        let channel_str = options
            .get("channel")
            .and_then(|v| v.as_str())
            .ok_or_else(|| napi::Error::from_reason("Missing 'channel' field"))?;

        let single_symbol = options.get("symbol").and_then(|v| v.as_str()).map(String::from);
        let batch_symbols = options
            .get("symbols")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect::<Vec<_>>()
            });

        let target_symbols: Vec<String> = match (single_symbol, batch_symbols) {
            (Some(s), None) => vec![s],
            (None, Some(list)) if !list.is_empty() => list,
            (None, Some(_)) => {
                return Err(napi::Error::from_reason(
                    "subscribe({symbols:[]}) is empty - provide at least one symbol",
                ));
            }
            (Some(_), Some(_)) => {
                return Err(napi::Error::from_reason(
                    "subscribe() accepts either 'symbol' or 'symbols', not both",
                ));
            }
            (None, None) => {
                return Err(napi::Error::from_reason(
                    "subscribe() requires 'symbol' or 'symbols'",
                ));
            }
        };

        let after_hours = options.get("afterHours").and_then(|v| v.as_bool());

        let tx_guard = self.command_tx.lock().map_err(|e| {
            napi::Error::from_reason(format!("Lock error: {}", e))
        })?;

        let tx = tx_guard.as_ref().ok_or_else(|| {
            napi::Error::from_reason("Not connected. Call connect() first.")
        })?;

        tx.send(WsCommand::Subscribe {
            channel: channel_str.to_string(),
            symbols: target_symbols,
            extra: after_hours,
        })
        .map_err(|_| napi::Error::from_reason("Failed to send subscribe command"))?;

        Ok(())
    }

    /// Unsubscribe from a channel
    ///
    /// Accepts either `{ id: "..." }` (single) or `{ ids: ["...", "..."] }` (batch).
    #[napi(ts_args_type = "options: string | UnsubscribeOptions")]
    pub fn unsubscribe(&self, options: serde_json::Value) -> napi::Result<()> {
        let target_ids: Vec<String> = if let Some(s) = options.as_str() {
            vec![s.to_string()]
        } else {
            let single = options.get("id").and_then(|v| v.as_str()).map(String::from);
            let batch = options
                .get("ids")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect::<Vec<_>>()
                });

            match (single, batch) {
                (Some(id), None) => vec![id],
                (None, Some(list)) if !list.is_empty() => list,
                (None, Some(_)) => {
                    return Err(napi::Error::from_reason(
                        "unsubscribe({ids:[]}) is empty - provide at least one id",
                    ));
                }
                (Some(_), Some(_)) => {
                    return Err(napi::Error::from_reason(
                        "unsubscribe() accepts either 'id' or 'ids', not both",
                    ));
                }
                (None, None) => {
                    return Err(napi::Error::from_reason(
                        "unsubscribe() requires 'id' or 'ids'",
                    ));
                }
            }
        };

        let tx_guard = self.command_tx.lock().map_err(|e| {
            napi::Error::from_reason(format!("Lock error: {}", e))
        })?;

        let tx = tx_guard.as_ref().ok_or_else(|| {
            napi::Error::from_reason("Not connected. Call connect() first.")
        })?;

        tx.send(WsCommand::Unsubscribe { ids: target_ids })
            .map_err(|_| napi::Error::from_reason("Failed to send unsubscribe command"))?;

        Ok(())
    }

    /// Send a `ping` frame to the server.
    ///
    /// Mirrors the old `@fugle/marketdata` Node SDK. The server's `pong` reply
    /// is delivered via the `message` callback (or processed internally by the
    /// health check, if enabled).
    ///
    /// @param state - Optional state string echoed back in the server's pong reply
    #[napi]
    pub fn ping(&self, state: Option<String>) -> napi::Result<()> {
        let tx_guard = self
            .command_tx
            .lock()
            .map_err(|e| napi::Error::from_reason(format!("Lock error: {}", e)))?;
        let tx = tx_guard
            .as_ref()
            .ok_or_else(|| napi::Error::from_reason("Not connected. Call connect() first."))?;
        tx.send(WsCommand::Ping { state })
            .map_err(|_| napi::Error::from_reason("Failed to send ping command"))?;
        Ok(())
    }

    /// Ask the server for its current subscription list.
    ///
    /// Sends `{ event: "subscriptions" }` to the server. The reply is delivered
    /// asynchronously via the `message` callback, matching the old
    /// `@fugle/marketdata` Node SDK semantics.
    #[napi]
    pub fn subscriptions(&self) -> napi::Result<()> {
        let tx_guard = self
            .command_tx
            .lock()
            .map_err(|e| napi::Error::from_reason(format!("Lock error: {}", e)))?;
        let tx = tx_guard
            .as_ref()
            .ok_or_else(|| napi::Error::from_reason("Not connected. Call connect() first."))?;
        tx.send(WsCommand::QuerySubscriptions)
            .map_err(|_| napi::Error::from_reason("Failed to send subscriptions command"))?;
        Ok(())
    }

    /// Disconnect from the WebSocket server
    #[napi]
    pub fn disconnect(&self) -> napi::Result<()> {
        let tx_guard = self.command_tx.lock().map_err(|e| {
            napi::Error::from_reason(format!("Lock error: {}", e))
        })?;

        if let Some(ref tx) = *tx_guard {
            let _ = tx.send(WsCommand::Disconnect);
        }

        Ok(())
    }

    /// Check if connected
    #[napi(getter)]
    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }

    /// Check if client has been closed
    ///
    /// Returns true if disconnect() has been called and client is closed.
    /// Once closed, the client cannot be reused - create a new instance.
    #[napi(getter)]
    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst)
    }
}

/// Helper function to fire callbacks from worker thread
///
/// Uses Arc::clone to get a reference to the callback, allowing safe use from
/// any thread. The ThreadsafeFunction with CalleeHandled=true (default) uses
/// call(Ok(data), mode) to pass data to JavaScript.
fn fire_callback(callbacks: &Arc<Mutex<EventCallbacks>>, event: &str, data: String) {
    if let Ok(cb) = callbacks.lock() {
        let callback = match event {
            "message" => cb.message.as_ref(),
            "connect" => cb.connect.as_ref(),
            "disconnect" => cb.disconnect.as_ref(),
            "reconnect" => cb.reconnect.as_ref(),
            "error" => cb.error.as_ref(),
            "authenticated" => cb.authenticated.as_ref(),
            "unauthenticated" => cb.unauthenticated.as_ref(),
            _ => None,
        };

        if let Some(callback) = callback {
            // Clone the Arc (not the callback itself) for thread-safe access
            let callback_ref = Arc::clone(callback);
            // In napi-rs 3.x with CalleeHandled=true (default), call() takes Result<T, ErrorStatus>
            callback_ref.call(data, ThreadsafeFunctionCallMode::NonBlocking);
        }
    }
}

// Unit tests are disabled because ThreadsafeFunction requires Node.js runtime
// Integration tests are done via JavaScript (test_websocket.js)
