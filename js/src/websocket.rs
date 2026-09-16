//! WebSocket client wrapper for JavaScript
//!
//! This module provides JavaScript-facing WebSocket clients using ThreadsafeFunction
//! for cross-thread callback invocation without blocking the Node.js event loop.
//!
//! Architecture:
//! - WebSocket connection runs in a dedicated background thread with its own tokio runtime
//! - Commands (connect, subscribe, disconnect) are sent via crossbeam channel
//! - Core connection events are forwarded to listeners via ThreadsafeFunction callbacks,
//!   with the argument shapes of `@fugle/marketdata` 1.x (#23)

use napi::bindgen_prelude::{JsValuesTupleIntoVec, PromiseRaw, ToNapiValue, Unknown};
use napi::JsValue;
use napi::Env;
use napi::threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode};
use napi::{sys, Status};

/// Event callback.
///
/// Uses napi-rs ThreadsafeFunction with `CalleeHandled = false` so the JS
/// callback receives the event's own arguments instead of a leading `err`,
/// matching the legacy `@fugle/marketdata` 1.x EventEmitter shape (#23):
/// ```js
/// stock.on('message', (data) => console.log(JSON.parse(data)));
/// stock.on('authenticated', (data) => console.log(data.message));
/// ```
///
/// `Weak = true`: a registered listener does not keep the Node event loop
/// alive, matching an EventEmitter listener. An open connection does, via
/// the [`KeepAlive`] handle held for the connection's lifetime (#30).
pub type EventTsfn = ThreadsafeFunction<EventArgs, Unknown<'static>, EventArgs, Status, false, true>;

/// Arguments an event listener is called with, built on the JS thread.
pub enum EventArgs {
    /// No arguments at all (`connect`), not a single `undefined`.
    None,
    /// A string (`message`: the frame verbatim).
    Text(String),
    /// A plain object; JSON `null` becomes `undefined`, as destructuring the
    /// frame's absent `data` did in 1.x.
    Json(serde_json::Value),
    /// An `Error` whose message is `message`, with a numeric `code` property
    /// when one is given.
    Error { message: String, code: Option<i32> },
}

impl JsValuesTupleIntoVec for EventArgs {
    // `env` comes from napi on the JS thread, as in napi-rs's own impls.
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn into_vec(self, env: sys::napi_env) -> napi::Result<Vec<sys::napi_value>> {
        let value = match self {
            EventArgs::None => return Ok(Vec::new()),
            EventArgs::Text(text) => unsafe { String::to_napi_value(env, text)? },
            EventArgs::Json(value) => json_to_napi(env, value)?,
            EventArgs::Error { message, code } => error_to_napi(env, &message, code)?,
        };
        Ok(vec![value])
    }
}

fn json_to_napi(env: sys::napi_env, value: serde_json::Value) -> napi::Result<sys::napi_value> {
    unsafe {
        match value {
            serde_json::Value::Null => <()>::to_napi_value(env, ()),
            value => serde_json::Value::to_napi_value(env, value),
        }
    }
}

/// A plain `new Error(message)` plus `code`. `Env::create_error` would set
/// `code` to the napi status name instead.
fn error_to_napi(
    env: sys::napi_env,
    message: &str,
    code: Option<i32>,
) -> napi::Result<sys::napi_value> {
    let env_ref = Env::from_raw(env);
    let message = env_ref.create_string(message)?;
    let mut error = std::ptr::null_mut();
    napi::check_status!(unsafe {
        sys::napi_create_error(env, std::ptr::null_mut(), message.raw(), &mut error)
    })?;
    if let Some(code) = code {
        let code = unsafe { i32::to_napi_value(env, code)? };
        napi::check_status!(unsafe {
            sys::napi_set_named_property(env, error, c"code".as_ptr(), code)
        })?;
    }
    Ok(error)
}
use napi_derive::napi;
use std::panic::AssertUnwindSafe;
use std::sync::{Arc, Mutex, atomic::{AtomicBool, AtomicU8, Ordering}};
use std::thread;
use std::time::Duration;

/// Type alias for JavaScript callback
/// In napi-rs 3.x, ThreadsafeFunction uses const generics instead of ErrorStrategy type.
/// Default CalleeHandled = true means the callee (JS function) handles errors.
/// We use Arc<ThreadsafeFunction> to allow cloning for use across threads.
pub type JsCallback = Arc<EventTsfn>;

/// Keeps the Node event loop alive while a connection is open (#30).
///
/// Wraps a strong (ref'd) threadsafe function that is never called; the loop
/// is released when the last clone drops. The worker thread and the event
/// thread each hold a clone, and so does every callback [`fire_callback`]
/// queues — dropped on the JS thread only after that callback has returned.
/// The final event (`disconnect`, a connect `error`, …) therefore always runs
/// before the process is allowed to exit, whichever event turns out to be
/// last and whether or not any listener is registered.
type KeepAlive = Arc<ThreadsafeFunction<(), (), (), Status, false>>;

/// How a `connect()` settles (#23).
enum AuthOutcome {
    /// Authenticated: resolve with the server's `data`.
    Authenticated(serde_json::Value),
    /// Credentials rejected: reject with the server's `data` object itself.
    Rejected(serde_json::Value),
    /// Any other failure: reject with `Error(message)`.
    Failed(String),
}

type AuthTx = tokio::sync::oneshot::Sender<AuthOutcome>;

/// Signal that authentication finished (or why it failed).
type AuthRx = tokio::sync::oneshot::Receiver<AuthOutcome>;

/// The pending `connect()` settlement, shared by the worker and the event
/// thread: whichever settles first takes it, so a Promise settles once.
type AuthSlot = Arc<Mutex<Option<AuthTx>>>;

/// Who decided how the initial authentication of a connection is reported
/// (#44): the forwarder on `Authenticated`, or the worker aborting because
/// disconnect() was called while authenticating. Whichever moves it off
/// `AUTH_PENDING` first wins, so `authenticated` never fires for a
/// connection whose `connect()` rejects as aborted.
type AuthDecision = Arc<AtomicU8>;
const AUTH_PENDING: u8 = 0;
const AUTH_REPORTED: u8 = 1;
const AUTH_ABORTED: u8 = 2;

/// Move `decision` from pending to `to`; false if the other side decided.
fn decide(decision: &AtomicU8, to: u8) -> bool {
    decision
        .compare_exchange(AUTH_PENDING, to, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok()
}

/// Test-only delay after the worker's `client.connect()` returns, before it
/// checks for an abort (#44): `FUGLE_MARKETDATA_TEST_DELAY_AFTER_CONNECT_MS`,
/// read on the JS thread when connecting. Debug builds only.
#[cfg(debug_assertions)]
fn test_delay_after_connect() -> Option<Duration> {
    std::env::var("FUGLE_MARKETDATA_TEST_DELAY_AFTER_CONNECT_MS")
        .ok()
        .and_then(|ms| ms.parse().ok())
        .map(Duration::from_millis)
}

#[cfg(not(debug_assertions))]
fn test_delay_after_connect() -> Option<Duration> {
    None
}

/// Settle the pending `connect()`, if it has not been settled already.
fn settle(slot: &AuthSlot, outcome: AuthOutcome) {
    if let Some(tx) = slot.lock().ok().and_then(|mut guard| guard.take()) {
        let _ = tx.send(outcome);
    }
}

/// Promise returned by `connect()`, settled by [`AuthOutcome`]. A failure to
/// start the worker rejects it too, so `connect()` never throws
/// synchronously.
fn auth_promise<'env>(
    env: &'env Env,
    started: napi::Result<AuthRx>,
) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
    env.spawn_future_with_callback(
        async move {
            Ok(started?.await.unwrap_or_else(|_| {
                AuthOutcome::Failed("Worker thread terminated before authentication signal".to_string())
            }))
        },
        |env, outcome| match outcome {
            AuthOutcome::Authenticated(data) => {
                Ok(unsafe { Unknown::from_raw_unchecked(env.raw(), json_to_napi(env.raw(), data)?) })
            }
            AuthOutcome::Rejected(data) => {
                let data = unsafe { Unknown::from_raw_unchecked(env.raw(), json_to_napi(env.raw(), data)?) };
                Err(napi::Error::from(data))
            }
            AuthOutcome::Failed(message) => Err(napi::Error::from_reason(message)),
        },
    )
}

fn loop_keep_alive(env: &Env) -> napi::Result<KeepAlive> {
    env.create_function_from_closure::<(), (), _>("fugleWsKeepAlive", |_| Ok(()))?
        .build_threadsafe_function::<()>()
        .callee_handled::<false>()
        .build()
        .map(Arc::new)
}

/// Reconnection options for WebSocket clients
///
/// All fields are optional - defaults are applied when not specified:
/// - maxAttempts: 5
/// - initialDelayMs: 1000
/// - maxDelayMs: 60000
#[napi(object)]
#[derive(Debug, Clone, Default)]
pub struct ReconnectOptions {
    /// Whether auto-reconnect is enabled (default: true when this object is
    /// supplied; when the entire `reconnect` option is omitted the binding
    /// preserves the historical Node SDK default of `false` — set this
    /// explicitly to opt in or out)
    pub enabled: Option<bool>,
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
    /// Override base URL (optional). Host and path prefix ONLY — the SDK
    /// appends the version segment.
    pub base_url: Option<String>,
    /// Per-product streaming version, e.g. `{ futopt: 'v1.0' }`.
    /// Omitted products get their latest: stock v1.0, futopt v1.1.
    pub version: Option<StreamingVersionOptions>,
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

/// Per-product streaming version selection.
///
/// The official SDK takes a free-form map and validates at runtime; expressing
/// it as a struct lets TypeScript reject an unknown product at compile time,
/// while the string values still need checking here.
#[napi(object)]
pub struct StreamingVersionOptions {
    /// Stock streaming version. Only "v1.0" is served.
    pub stock: Option<String>,
    /// FutOpt streaming version: "v1.0" or "v1.1" (default).
    ///
    /// v1.1 adds trial-matching (試撮) frames on trades / books — branch on
    /// the frame's `isTrial` before acting on a price.
    pub futopt: Option<String>,
}

#[derive(Clone, Copy)]
pub(crate) enum WsProduct {
    Stock,
    FutOpt,
}

/// Validate the `version` option into core's per-product enums.
pub(crate) fn parse_ws_versions(
    version: &Option<StreamingVersionOptions>,
) -> napi::Result<(
    marketdata_core::websocket::StockVersion,
    marketdata_core::websocket::FutOptVersion,
)> {
    use marketdata_core::websocket::{FutOptVersion, StockVersion};

    let mut stock = StockVersion::default();
    let mut futopt = FutOptVersion::default();

    if let Some(opts) = version {
        if let Some(v) = opts.stock.as_deref() {
            stock = match v {
                "v1.0" => StockVersion::V1_0,
                other => {
                    return Err(napi::Error::from_reason(format!(
                        "stock streaming does not support {other} (supported: v1.0). \
                         Omit it to use v1.0."
                    )))
                }
            };
        }
        if let Some(v) = opts.futopt.as_deref() {
            futopt = match v {
                "v1.0" => FutOptVersion::V1_0,
                "v1.1" => FutOptVersion::V1_1,
                other => {
                    return Err(napi::Error::from_reason(format!(
                        "futopt streaming does not support {other} (supported: v1.0, v1.1). \
                         Omit it to use v1.1."
                    )))
                }
            };
        }
    }

    Ok((stock, futopt))
}

/// Resolve a streaming endpoint through core's factory.
///
/// Centralised so the two call sites cannot drift on base-URL semantics —
/// each used to hand-roll `format!("{base}/stock/streaming")`, which is
/// exactly the duplication 0.8.0 removes.
pub(crate) fn build_stream_config(
    api_key: &str,
    base_url: Option<&str>,
    product: WsProduct,
    stock_version: marketdata_core::websocket::StockVersion,
    futopt_version: marketdata_core::websocket::FutOptVersion,
) -> Result<marketdata_core::ConnectionConfig, marketdata_core::MarketDataError> {
    let auth = marketdata_core::AuthRequest::with_api_key(api_key);
    let mut factory = marketdata_core::WebSocketFactory::new()
        .stock_version(stock_version)
        .futopt_version(futopt_version);
    if let Some(base) = base_url {
        factory = factory.base_url(base);
    }
    let factory = factory.auth(auth);
    let builder = match product {
        WsProduct::Stock => factory.stock()?,
        WsProduct::FutOpt => factory.futopt()?,
    };
    Ok(builder.build())
}

/// Command sent to WebSocket worker thread
#[derive(Debug)]
enum WsCommand {
    Subscribe { channel: String, symbols: Vec<String>, extra: Option<bool> },
    Unsubscribe { ids: Vec<String> },
    /// Send a `ping` frame carrying `data` (mirrors 1.x `ping(params)`)
    Ping { data: Option<serde_json::Value> },
    /// Ask the server for its current subscription list (response arrives via `message`)
    QuerySubscriptions,
    Disconnect,
}

/// A `ping()` argument as the frame's `data` (#23): an object (or any other
/// value) as-is, 1.x style; a string as `{ state }`, as rc.2 sent it; nothing
/// (or `null`) omits `data`.
fn ping_data(params: Option<serde_json::Value>) -> Option<serde_json::Value> {
    match params? {
        serde_json::Value::Null => None,
        serde_json::Value::String(state) => Some(serde_json::json!({ "state": state })),
        value => Some(value),
    }
}

/// Rejection for `connect()` while a connection is open or still being
/// established (#44). JS-binding-only code; core has no counterpart.
const ALREADY_CONNECTED: &str = "[2011] Already connected; call disconnect() first";

/// Rejection for a `connect()` whose connection was given up because
/// `disconnect()` was called before authentication completed (#44).
const CONNECT_ABORTED: &str =
    "[2010] Connection aborted: disconnect() called before authentication completed";

/// The worker thread that owns a client's connection (#44).
struct Worker {
    tx: std::sync::mpsc::Sender<WsCommand>,
    handle: thread::JoinHandle<()>,
    /// Set once the connection is on its way out for good: `disconnect()` was
    /// called, authentication failed, or core stopped with no reconnect left.
    /// Always set *before* the JS side can observe the end (a `disconnect` /
    /// `error` callback, a rejected `connect()`), so calling `connect()` from
    /// there is accepted rather than racing the worker's exit.
    ending: Arc<AtomicBool>,
}

/// A client's current worker, shared by every `ws.stock` / `ws.futopt` wrapper.
type WorkerSlot = Arc<Mutex<Option<Worker>>>;

/// Make room in `slot` for a new worker.
///
/// Rejects while the current worker is connecting or connected. A worker
/// that is ending (or already gone) is taken out and its handle returned: the
/// new worker joins it before connecting, so the old worker's final writes
/// to the shared `connected` / `closed` flags cannot land on the new
/// connection.
fn claim_worker_slot(
    slot: &mut Option<Worker>,
) -> napi::Result<Option<thread::JoinHandle<()>>> {
    match slot.take() {
        None => Ok(None),
        Some(worker)
            if worker.ending.load(Ordering::SeqCst) || worker.handle.is_finished() =>
        {
            Ok(Some(worker.handle))
        }
        Some(worker) => {
            *slot = Some(worker);
            Err(napi::Error::from_reason(ALREADY_CONNECTED))
        }
    }
}

/// Queue `command` for the running worker.
fn send_command(slot: &WorkerSlot, command: WsCommand, name: &str) -> napi::Result<()> {
    let guard = slot
        .lock()
        .map_err(|e| napi::Error::from_reason(format!("Lock error: {}", e)))?;
    let worker = guard
        .as_ref()
        .ok_or_else(|| napi::Error::from_reason("Not connected. Call connect() first."))?;
    worker
        .tx
        .send(command)
        .map_err(|_| napi::Error::from_reason(format!("Failed to send {} command", name)))
}

/// Mark the worker as ending and ask it to disconnect; no-op without one.
fn request_disconnect(slot: &WorkerSlot) -> napi::Result<()> {
    let guard = slot
        .lock()
        .map_err(|e| napi::Error::from_reason(format!("Lock error: {}", e)))?;
    if let Some(worker) = guard.as_ref() {
        worker.ending.store(true, Ordering::SeqCst);
        let _ = worker.tx.send(WsCommand::Disconnect);
    }
    Ok(())
}

/// Callback storage for event handlers
#[derive(Default)]
struct EventCallbacks {
    message: Option<JsCallback>,
    connect: Option<JsCallback>,
    disconnect: Option<JsCallback>,
    reconnect: Option<JsCallback>,
    error: Option<JsCallback>,
    authenticated: Option<JsCallback>,
    unauthenticated: Option<JsCallback>,
}

/// WebSocket client for real-time market data (JavaScript wrapper)
///
/// # JavaScript Usage
///
/// ```javascript
/// const { WebSocketClient } = require('@fugle/marketdata');
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
    stock_version: marketdata_core::websocket::StockVersion,
    futopt_version: marketdata_core::websocket::FutOptVersion,
    reconnect_config: marketdata_core::ReconnectionConfig,
    health_check_config: marketdata_core::HealthCheckConfig,
    tls_config: marketdata_core::TlsConfig,
    // Shared state for child clients — created once in constructor so that
    // every `ws.stock` / `ws.futopt` getter access shares the same Arcs.
    stock_callbacks: Arc<Mutex<EventCallbacks>>,
    stock_connected: Arc<AtomicBool>,
    stock_closed: Arc<AtomicBool>,
    stock_worker: WorkerSlot,
    futopt_callbacks: Arc<Mutex<EventCallbacks>>,
    futopt_connected: Arc<AtomicBool>,
    futopt_closed: Arc<AtomicBool>,
    futopt_worker: WorkerSlot,
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

        let (stock_version, futopt_version) = parse_ws_versions(&options.version)?;

        // Resolve both endpoints now so a bad `baseUrl` throws from the
        // constructor rather than from `.stock.connect()` much later. Matches
        // the official SDK, which rejects a versioned baseUrl up front.
        for product in [WsProduct::Stock, WsProduct::FutOpt] {
            build_stream_config(
                options
                    .api_key
                    .as_deref()
                    .or(options.bearer_token.as_deref())
                    .or(options.sdk_token.as_deref())
                    .unwrap_or_default(),
                options.base_url.as_deref(),
                product,
                stock_version,
                futopt_version,
            )
            .map_err(crate::errors::to_napi_error)?;
        }

        // Extract the one provided auth method
        let api_key = options.api_key
            .or(options.bearer_token)
            .or(options.sdk_token)
            .unwrap();

        // Build reconnection config with validation via core.
        //
        // Binding-side default: omitting `options.reconnect` preserves the
        // historical Node SDK semantic of "no auto-reconnect" by routing
        // through `ReconnectionConfig::disabled()`. Core 0.4.0 flipped its
        // own `default()` to `enabled: true`; this branch compensates so the
        // JS API surface is unchanged. Pass `{ reconnect: { enabled: true } }`
        // (or any populated reconnect object — `enabled` defaults to true
        // when the object itself is provided) to opt in.
        let reconnect_cfg = if let Some(r) = &options.reconnect {
            let max = r.max_attempts.unwrap_or(DEFAULT_MAX_ATTEMPTS);
            let initial = Duration::from_millis(
                r.initial_delay_ms.map(|v| v as u64).unwrap_or(DEFAULT_INITIAL_DELAY_MS)
            );
            let max_delay = Duration::from_millis(
                r.max_delay_ms.map(|v| v as u64).unwrap_or(DEFAULT_MAX_DELAY_MS)
            );
            let mut cfg = marketdata_core::ReconnectionConfig::new(max, initial, max_delay)
                .map_err(|e| napi::Error::from_reason(e.to_string()))?;
            // Honor explicit opt-out: `{ reconnect: { enabled: false } }`.
            if let Some(enabled) = r.enabled {
                cfg.enabled = enabled;
            }
            cfg
        } else {
            marketdata_core::ReconnectionConfig::disabled()
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
            stock_version,
            futopt_version,
            reconnect_config: reconnect_cfg,
            health_check_config: health_check_cfg,
            tls_config,
            stock_callbacks: Arc::new(Mutex::new(EventCallbacks::default())),
            stock_connected: Arc::new(AtomicBool::new(false)),
            stock_closed: Arc::new(AtomicBool::new(false)),
            stock_worker: Arc::new(Mutex::new(None)),
            futopt_callbacks: Arc::new(Mutex::new(EventCallbacks::default())),
            futopt_connected: Arc::new(AtomicBool::new(false)),
            futopt_closed: Arc::new(AtomicBool::new(false)),
            futopt_worker: Arc::new(Mutex::new(None)),
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
            self.stock_version,
            self.futopt_version,
            self.reconnect_config.clone(),
            self.health_check_config.clone(),
            self.tls_config.clone(),
            Arc::clone(&self.stock_callbacks),
            Arc::clone(&self.stock_connected),
            Arc::clone(&self.stock_closed),
            Arc::clone(&self.stock_worker),
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
            self.stock_version,
            self.futopt_version,
            self.reconnect_config.clone(),
            self.health_check_config.clone(),
            self.tls_config.clone(),
            Arc::clone(&self.futopt_callbacks),
            Arc::clone(&self.futopt_connected),
            Arc::clone(&self.futopt_closed),
            Arc::clone(&self.futopt_worker),
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
    stock_version: marketdata_core::websocket::StockVersion,
    futopt_version: marketdata_core::websocket::FutOptVersion,
    reconnect_config: marketdata_core::ReconnectionConfig,
    health_check_config: marketdata_core::HealthCheckConfig,
    tls_config: marketdata_core::TlsConfig,
    callbacks: Arc<Mutex<EventCallbacks>>,
    connected: Arc<AtomicBool>,
    closed: Arc<AtomicBool>,
    worker: WorkerSlot,
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
        stock_version: marketdata_core::websocket::StockVersion,
        futopt_version: marketdata_core::websocket::FutOptVersion,
        reconnect_config: marketdata_core::ReconnectionConfig,
        health_check_config: marketdata_core::HealthCheckConfig,
        tls_config: marketdata_core::TlsConfig,
        callbacks: Arc<Mutex<EventCallbacks>>,
        connected: Arc<AtomicBool>,
        closed: Arc<AtomicBool>,
        worker: WorkerSlot,
    ) -> Self {
        Self {
            api_key,
            base_url,
            stock_version,
            futopt_version,
            reconnect_config,
            health_check_config,
            tls_config,
            callbacks,
            connected,
            closed,
            worker,
        }
    }

    /// Register an event handler
    ///
    /// Arguments match `@fugle/marketdata` 1.x (#23): `message(data: string)`,
    /// `connect()` when the socket opens, `authenticated(data)` /
    /// `unauthenticated(data)` with the server's `data`,
    /// `disconnect({ code, reason })`, `reconnect({ attempt })`, and
    /// `error(Error)` with a numeric `code` when core supplied one. Without an
    /// `error` listener errors are ignored rather than thrown.
    ///
    /// @param event - Event type: "message", "connect", "authenticated",
    ///                "unauthenticated", "disconnect", "reconnect", "error"
    /// @param callback - Listener for that event
    ///
    /// @example
    /// ```javascript
    /// ws.stock.on('message', (data) => console.log(data));
    /// ws.stock.on('connect', () => console.log('Connected'));
    /// ws.stock.on('disconnect', ({ code, reason }) => console.log(code, reason));
    /// ws.stock.on('error', (err) => console.error(err.code, err.message));
    /// ```
    #[napi(
        ts_generic_types = "E extends WebSocketEvent",
        ts_args_type = "event: E, callback: WebSocketEventMap[E]"
    )]
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
    /// Returns a Promise that resolves with the server's `authenticated`
    /// `data` once authentication completes, matching `@fugle/marketdata` 1.x
    /// (#23):
    ///
    /// ```js
    /// stock.connect().then((data) => {
    ///   stock.subscribe({ channel: 'trades', symbol: '2330' });
    /// });
    /// ```
    ///
    /// If the server rejects the credentials, the Promise rejects with the
    /// server's `data` object itself (after `unauthenticated` fires); any other
    /// failure rejects with an `Error` whose message is `[code] message`.
    ///
    /// Rejects with `[2011] Already connected` while a connection is open or
    /// being established (#44). Call disconnect() first to reconnect; calling
    /// connect() right after disconnect(), or from a `disconnect` handler once
    /// no auto-reconnect will follow, is fine.
    #[napi(ts_return_type = "Promise<WebSocketAuthData | undefined>")]
    pub fn connect<'env>(&self, env: &'env Env) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        auth_promise(env, self.start_worker(env))
    }

    /// Spawn the worker thread; the receiver fires once it has authenticated.
    fn start_worker(&self, env: &Env) -> napi::Result<AuthRx> {
        // Held until the new worker is stored, so concurrent connect() calls
        // cannot both claim the slot (#44).
        let mut slot = self.worker.lock().map_err(|e| {
            napi::Error::from_reason(format!("Lock error: {}", e))
        })?;
        let previous = claim_worker_slot(&mut slot)?;
        let keep_alive = loop_keep_alive(env)?;
        let (cmd_tx, cmd_rx) = std::sync::mpsc::channel::<WsCommand>();
        let ending = Arc::new(AtomicBool::new(false));
        let decision: AuthDecision = Arc::new(AtomicU8::new(AUTH_PENDING));
        let panic_reported = Arc::new(AtomicBool::new(false));
        let delay_after_connect = test_delay_after_connect();

        let (auth_tx, auth_rx) = tokio::sync::oneshot::channel::<AuthOutcome>();
        let auth: AuthSlot = Arc::new(Mutex::new(Some(auth_tx)));

        // Clone data for the worker thread
        let api_key = self.api_key.clone();
        let base_url = self.base_url.clone();
        let stock_version = self.stock_version;
        let futopt_version = self.futopt_version;
        let reconnect_config = self.reconnect_config.clone();
        let health_check_config = self.health_check_config.clone();
        let tls_config = self.tls_config.clone();
        let callbacks = Arc::clone(&self.callbacks);
        let connected = Arc::clone(&self.connected);
        let closed = Arc::clone(&self.closed);
        let ending_for_worker = Arc::clone(&ending);
        let test_panic = test_panic_site();

        // Spawn worker thread that owns WebSocketClient
        let handle = thread::Builder::new()
            .name("stock_ws_worker".to_string())
            .spawn(move || {
                use marketdata_core::aio::WebSocketClient as CoreClient;
                use marketdata_core::websocket::ConnectionConfig;
                use marketdata_core::AuthRequest;
                use marketdata_core::models::Channel;
                use marketdata_core::websocket::channels::StockSubscription;

                let ending = ending_for_worker;
                // Supervised: a panic anywhere in here is reported instead of
                // leaving the connection silently dead (#25).
                let run = || {
                    // A connection being reused: let the previous worker finish
                    // its teardown before this one touches the shared flags. Only
                    // the worker is joined, not its event thread, so the old
                    // connection's `disconnect` callback may still arrive after
                    // this connection's `connect`.
                    if let Some(previous) = previous {
                        let _ = previous.join();
                    }
                    closed.store(false, Ordering::SeqCst);

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
                            // No core client yet, so no event to forward: only
                            // the Promise reports it.
                            ending.store(true, Ordering::SeqCst);
                            settle(&auth, AuthOutcome::Failed(format!("Failed to create runtime: {}", e)));
                            return;
                        }
                    };

                    // `baseUrl` was validated in the constructor, so the only way
                    // this can fail is a client built by other means — fall back to
                    // production rather than kill the worker thread.
                    let mut config = build_stream_config(
                        &api_key,
                        base_url.as_deref(),
                        WsProduct::Stock,
                        stock_version,
                        futopt_version,
                    )
                    .unwrap_or_else(|_| {
                        ConnectionConfig::fugle_stock(AuthRequest::with_api_key(&api_key))
                    });
                    config.tls = tls_config;
                    let client = CoreClient::with_full_config(config, reconnect_config.clone(), health_check_config);

                    // Forward core events from before connect(): `Connected` is
                    // emitted when the socket opens, ahead of authentication, and
                    // the authentication outcome settles the Promise (#23).
                    let dispatch_ended = Arc::new(AtomicBool::new(false));
                    spawn_event_forwarder(
                        Arc::clone(client.state_events()),
                        Arc::clone(&callbacks),
                        Arc::clone(&keep_alive),
                        Arc::clone(&auth),
                        Arc::clone(&connected),
                        Arc::clone(&closed),
                        Arc::clone(&ending),
                        Arc::clone(&dispatch_ended),
                        test_panic.clone(),
                        Arc::clone(&decision),
                        Arc::clone(&panic_reported),
                    );

                    // Core's connect().await returns once authenticated. Its
                    // failures were already emitted as `Unauthenticated` or
                    // `Error`, which the forwarder turns into the rejection.
                    if rt.block_on(client.connect()).is_err() {
                        ending.store(true, Ordering::SeqCst);
                        return;
                    }

                    if let Some(delay) = delay_after_connect {
                        thread::sleep(delay);
                    }

                    // disconnect() arrived while authenticating, and connect() may
                    // already have started a newer worker that is joining this
                    // one: abandon the connection instead of reporting success
                    // (#44) — unless the forwarder already reported it, in which
                    // case the queued Disconnect ends it like any other.
                    if ending.load(Ordering::SeqCst) && decide(&decision, AUTH_ABORTED) {
                        let _ = rt.block_on(client.disconnect());
                        connected.store(false, Ordering::SeqCst);
                        closed.store(true, Ordering::SeqCst);
                        settle(&auth, AuthOutcome::Failed(CONNECT_ABORTED.to_string()));
                        return;
                    }

                    // `messages()` needs no runtime context since #36.
                    let receiver = client.messages();

                    // Main event loop
                    loop {
                        if connected.load(Ordering::SeqCst) {
                            inject_test_panic(test_panic.as_deref(), "ws_worker");
                        }
                        if dispatch_ended.load(Ordering::SeqCst) {
                            connected.store(false, Ordering::SeqCst);
                            closed.store(true, Ordering::SeqCst);
                            break;
                        }

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
                            Ok(WsCommand::Ping { data }) => {
                                let request = marketdata_core::WebSocketRequest {
                                    event: "ping".to_string(),
                                    data,
                                };
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
                                // Core's disconnect() emits `Disconnected` on the
                                // event channel and the event thread forwards it;
                                // firing here too duplicates the callback (#22).
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
                                // The frame verbatim: re-serializing the routing
                                // struct would drop unknown fields and emit nulls
                                // for the ones the server omitted.
                                fire_callback(&callbacks, &keep_alive, "message", EventArgs::Text(msg.raw));
                            }
                            Ok(None) => {
                                // Timeout, continue loop
                            }
                            Err(_) => {
                                // Channel closed: the dispatch task has ended and
                                // already reported why via the event channel.
                                connected.store(false, Ordering::SeqCst);
                                closed.store(true, Ordering::SeqCst);
                                break;
                            }
                        }
                    }
                    ending.store(true, Ordering::SeqCst);
                };
                if let Err(payload) = std::panic::catch_unwind(AssertUnwindSafe(run)) {
                    report_panic(
                        &PanicContext {
                            callbacks: &callbacks,
                            keep_alive: &keep_alive,
                            auth: &auth,
                            connected: &connected,
                            closed: &closed,
                            ending: &ending,
                            reported: &panic_reported,
                        },
                        "worker",
                        &*payload,
                    );
                }
            })
            .map_err(|e| napi::Error::from_reason(format!("Failed to spawn worker thread: {}", e)))?;

        *slot = Some(Worker { tx: cmd_tx, handle, ending });
        Ok(auth_rx)
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

        send_command(&self.worker, WsCommand::Subscribe {
            channel: channel_str.to_string(),
            symbols: target_symbols,
            extra: odd_lot,
        }, "subscribe")
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

        send_command(&self.worker, WsCommand::Unsubscribe { ids: target_ids }, "unsubscribe")
    }

    /// Send a `ping` frame to the server.
    ///
    /// Mirrors the old `@fugle/marketdata` Node SDK. The server's `pong` reply
    /// is delivered via the `message` callback (or processed internally by the
    /// health check, if enabled).
    ///
    /// @param params - Sent as the frame's `data`, e.g. `{ state: 'x' }`, whose
    ///                 `state` the server echoes back in its pong. A string is
    ///                 accepted for compatibility and sent as `{ state }`.
    #[napi(ts_args_type = "params?: string | WebSocketPingParams")]
    pub fn ping(&self, params: Option<serde_json::Value>) -> napi::Result<()> {
        send_command(&self.worker, WsCommand::Ping { data: ping_data(params) }, "ping")
    }

    /// Ask the server for its current subscription list.
    ///
    /// Sends `{ event: "subscriptions" }` to the server. The reply is delivered
    /// asynchronously via the `message` callback, matching the old
    /// `@fugle/marketdata` Node SDK semantics.
    #[napi]
    pub fn subscriptions(&self) -> napi::Result<()> {
        send_command(&self.worker, WsCommand::QuerySubscriptions, "subscriptions")
    }

    /// Disconnect from the WebSocket server
    #[napi]
    pub fn disconnect(&self) -> napi::Result<()> {
        request_disconnect(&self.worker)
    }

    /// Check if connected
    #[napi(getter)]
    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }

    /// Check if client has been closed
    ///
    /// Returns true once the connection has closed: after disconnect(), or
    /// after the server or network ended it with no reconnect left. A closed
    /// client can connect() again; isClosed turns false once the new
    /// connection starts.
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
    stock_version: marketdata_core::websocket::StockVersion,
    futopt_version: marketdata_core::websocket::FutOptVersion,
    reconnect_config: marketdata_core::ReconnectionConfig,
    health_check_config: marketdata_core::HealthCheckConfig,
    tls_config: marketdata_core::TlsConfig,
    callbacks: Arc<Mutex<EventCallbacks>>,
    connected: Arc<AtomicBool>,
    closed: Arc<AtomicBool>,
    worker: WorkerSlot,
}

#[napi]
impl FutOptWebSocketClient {
    /// Create from pre-existing shared state (called by WebSocketClient getter).
    /// See StockWebSocketClient::from_shared for rationale.
    fn from_shared(
        api_key: String,
        base_url: Option<String>,
        stock_version: marketdata_core::websocket::StockVersion,
        futopt_version: marketdata_core::websocket::FutOptVersion,
        reconnect_config: marketdata_core::ReconnectionConfig,
        health_check_config: marketdata_core::HealthCheckConfig,
        tls_config: marketdata_core::TlsConfig,
        callbacks: Arc<Mutex<EventCallbacks>>,
        connected: Arc<AtomicBool>,
        closed: Arc<AtomicBool>,
        worker: WorkerSlot,
    ) -> Self {
        Self {
            api_key,
            base_url,
            stock_version,
            futopt_version,
            reconnect_config,
            health_check_config,
            tls_config,
            callbacks,
            connected,
            closed,
            worker,
        }
    }

    /// Register an event handler
    ///
    /// Same events and arguments as `StockWebSocketClient::on` (#23).
    #[napi(
        ts_generic_types = "E extends WebSocketEvent",
        ts_args_type = "event: E, callback: WebSocketEventMap[E]"
    )]
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
    /// Returns a Promise that resolves with the server's `authenticated`
    /// `data`. See `StockWebSocketClient::connect` for the rejections,
    /// including `[2011] Already connected` (#44).
    #[napi(ts_return_type = "Promise<WebSocketAuthData | undefined>")]
    pub fn connect<'env>(&self, env: &'env Env) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        auth_promise(env, self.start_worker(env))
    }

    /// Spawn the worker thread; the receiver fires once it has authenticated.
    fn start_worker(&self, env: &Env) -> napi::Result<AuthRx> {
        // Held until the new worker is stored, so concurrent connect() calls
        // cannot both claim the slot (#44).
        let mut slot = self.worker.lock().map_err(|e| {
            napi::Error::from_reason(format!("Lock error: {}", e))
        })?;
        let previous = claim_worker_slot(&mut slot)?;
        let keep_alive = loop_keep_alive(env)?;
        let (cmd_tx, cmd_rx) = std::sync::mpsc::channel::<WsCommand>();
        let ending = Arc::new(AtomicBool::new(false));
        let decision: AuthDecision = Arc::new(AtomicU8::new(AUTH_PENDING));
        let panic_reported = Arc::new(AtomicBool::new(false));
        let delay_after_connect = test_delay_after_connect();

        let (auth_tx, auth_rx) = tokio::sync::oneshot::channel::<AuthOutcome>();
        let auth: AuthSlot = Arc::new(Mutex::new(Some(auth_tx)));

        let api_key = self.api_key.clone();
        let base_url = self.base_url.clone();
        let stock_version = self.stock_version;
        let futopt_version = self.futopt_version;
        let reconnect_config = self.reconnect_config.clone();
        let health_check_config = self.health_check_config.clone();
        let tls_config = self.tls_config.clone();
        let callbacks = Arc::clone(&self.callbacks);
        let connected = Arc::clone(&self.connected);
        let closed = Arc::clone(&self.closed);
        let ending_for_worker = Arc::clone(&ending);
        let test_panic = test_panic_site();

        let handle = thread::Builder::new()
            .name("futopt_ws_worker".to_string())
            .spawn(move || {
                use marketdata_core::aio::WebSocketClient as CoreClient;
                use marketdata_core::websocket::ConnectionConfig;
                use marketdata_core::AuthRequest;
                use marketdata_core::models::futopt::FutOptChannel;
                use marketdata_core::websocket::channels::FutOptSubscription;

                let ending = ending_for_worker;
                // Supervised: a panic anywhere in here is reported instead of
                // leaving the connection silently dead (#25).
                let run = || {
                    // A connection being reused: let the previous worker finish
                    // its teardown before this one touches the shared flags. Only
                    // the worker is joined, not its event thread, so the old
                    // connection's `disconnect` callback may still arrive after
                    // this connection's `connect`.
                    if let Some(previous) = previous {
                        let _ = previous.join();
                    }
                    closed.store(false, Ordering::SeqCst);

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
                            // No core client yet, so no event to forward: only
                            // the Promise reports it.
                            ending.store(true, Ordering::SeqCst);
                            settle(&auth, AuthOutcome::Failed(format!("Failed to create runtime: {}", e)));
                            return;
                        }
                    };

                    // See the stock sibling: `baseUrl` was validated in the
                    // constructor, so this cannot fail for a normally-built client.
                    let mut config = build_stream_config(
                        &api_key,
                        base_url.as_deref(),
                        WsProduct::FutOpt,
                        stock_version,
                        futopt_version,
                    )
                    .unwrap_or_else(|_| {
                        ConnectionConfig::fugle_futopt(AuthRequest::with_api_key(&api_key))
                    });
                    config.tls = tls_config;
                    let client = CoreClient::with_full_config(config, reconnect_config.clone(), health_check_config);

                    // Forward core events from before connect(): `Connected` is
                    // emitted when the socket opens, ahead of authentication, and
                    // the authentication outcome settles the Promise (#23).
                    let dispatch_ended = Arc::new(AtomicBool::new(false));
                    spawn_event_forwarder(
                        Arc::clone(client.state_events()),
                        Arc::clone(&callbacks),
                        Arc::clone(&keep_alive),
                        Arc::clone(&auth),
                        Arc::clone(&connected),
                        Arc::clone(&closed),
                        Arc::clone(&ending),
                        Arc::clone(&dispatch_ended),
                        test_panic.clone(),
                        Arc::clone(&decision),
                        Arc::clone(&panic_reported),
                    );

                    // Core's connect().await returns once authenticated. Its
                    // failures were already emitted as `Unauthenticated` or
                    // `Error`, which the forwarder turns into the rejection.
                    if rt.block_on(client.connect()).is_err() {
                        ending.store(true, Ordering::SeqCst);
                        return;
                    }

                    if let Some(delay) = delay_after_connect {
                        thread::sleep(delay);
                    }

                    // disconnect() arrived while authenticating, and connect() may
                    // already have started a newer worker that is joining this
                    // one: abandon the connection instead of reporting success
                    // (#44) — unless the forwarder already reported it, in which
                    // case the queued Disconnect ends it like any other.
                    if ending.load(Ordering::SeqCst) && decide(&decision, AUTH_ABORTED) {
                        let _ = rt.block_on(client.disconnect());
                        connected.store(false, Ordering::SeqCst);
                        closed.store(true, Ordering::SeqCst);
                        settle(&auth, AuthOutcome::Failed(CONNECT_ABORTED.to_string()));
                        return;
                    }

                    // `messages()` needs no runtime context since #36.
                    let receiver = client.messages();

                    // Main event loop
                    loop {
                        if connected.load(Ordering::SeqCst) {
                            inject_test_panic(test_panic.as_deref(), "ws_worker");
                        }
                        if dispatch_ended.load(Ordering::SeqCst) {
                            connected.store(false, Ordering::SeqCst);
                            closed.store(true, Ordering::SeqCst);
                            break;
                        }

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
                            Ok(WsCommand::Ping { data }) => {
                                let request = marketdata_core::WebSocketRequest {
                                    event: "ping".to_string(),
                                    data,
                                };
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
                                // Core's disconnect() emits `Disconnected` on the
                                // event channel and the event thread forwards it;
                                // firing here too duplicates the callback (#22).
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
                                // The frame verbatim: re-serializing the routing
                                // struct would drop unknown fields and emit nulls
                                // for the ones the server omitted.
                                fire_callback(&callbacks, &keep_alive, "message", EventArgs::Text(msg.raw));
                            }
                            Ok(None) => {
                                // Timeout, continue loop
                            }
                            Err(_) => {
                                // Channel closed: the dispatch task has ended and
                                // already reported why via the event channel.
                                connected.store(false, Ordering::SeqCst);
                                closed.store(true, Ordering::SeqCst);
                                break;
                            }
                        }
                    }
                    ending.store(true, Ordering::SeqCst);
                };
                if let Err(payload) = std::panic::catch_unwind(AssertUnwindSafe(run)) {
                    report_panic(
                        &PanicContext {
                            callbacks: &callbacks,
                            keep_alive: &keep_alive,
                            auth: &auth,
                            connected: &connected,
                            closed: &closed,
                            ending: &ending,
                            reported: &panic_reported,
                        },
                        "worker",
                        &*payload,
                    );
                }
            })
            .map_err(|e| napi::Error::from_reason(format!("Failed to spawn worker thread: {}", e)))?;

        *slot = Some(Worker { tx: cmd_tx, handle, ending });
        Ok(auth_rx)
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

        send_command(&self.worker, WsCommand::Subscribe {
            channel: channel_str.to_string(),
            symbols: target_symbols,
            extra: after_hours,
        }, "subscribe")
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

        send_command(&self.worker, WsCommand::Unsubscribe { ids: target_ids }, "unsubscribe")
    }

    /// Send a `ping` frame to the server.
    ///
    /// Mirrors the old `@fugle/marketdata` Node SDK. The server's `pong` reply
    /// is delivered via the `message` callback (or processed internally by the
    /// health check, if enabled).
    ///
    /// @param params - Sent as the frame's `data`, e.g. `{ state: 'x' }`, whose
    ///                 `state` the server echoes back in its pong. A string is
    ///                 accepted for compatibility and sent as `{ state }`.
    #[napi(ts_args_type = "params?: string | WebSocketPingParams")]
    pub fn ping(&self, params: Option<serde_json::Value>) -> napi::Result<()> {
        send_command(&self.worker, WsCommand::Ping { data: ping_data(params) }, "ping")
    }

    /// Ask the server for its current subscription list.
    ///
    /// Sends `{ event: "subscriptions" }` to the server. The reply is delivered
    /// asynchronously via the `message` callback, matching the old
    /// `@fugle/marketdata` Node SDK semantics.
    #[napi]
    pub fn subscriptions(&self) -> napi::Result<()> {
        send_command(&self.worker, WsCommand::QuerySubscriptions, "subscriptions")
    }

    /// Disconnect from the WebSocket server
    #[napi]
    pub fn disconnect(&self) -> napi::Result<()> {
        request_disconnect(&self.worker)
    }

    /// Check if connected
    #[napi(getter)]
    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }

    /// Check if client has been closed
    ///
    /// Returns true once the connection has closed: after disconnect(), or
    /// after the server or network ended it with no reconnect left. A closed
    /// client can connect() again; isClosed turns false once the new
    /// connection starts.
    #[napi(getter)]
    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst)
    }
}

/// Forward a connection's core events to the JS listeners until core drops
/// the event channel (#23). Only core events are forwarded; the worker
/// emits none of its own.
///
/// The first authentication outcome settles `connect()`: `Authenticated`
/// resolves with the server's `data`, `Unauthenticated` rejects with it, and
/// an `Error` before either rejects with `Error("[code] message")` — each
/// after its listener has run (see [`fire_and_settle`]).
fn spawn_event_forwarder(
    events: Arc<tokio::sync::Mutex<std::sync::mpsc::Receiver<marketdata_core::websocket::ConnectionEvent>>>,
    callbacks: Arc<Mutex<EventCallbacks>>,
    keep_alive: KeepAlive,
    auth: AuthSlot,
    connected: Arc<AtomicBool>,
    closed: Arc<AtomicBool>,
    ending: Arc<AtomicBool>,
    dispatch_ended: Arc<AtomicBool>,
    test_panic: Option<String>,
    decision: AuthDecision,
    panic_reported: Arc<AtomicBool>,
) {
    use marketdata_core::websocket::ConnectionEvent;

    std::thread::spawn(move || {
        let run = || loop {
            let event = {
                let rx = events.blocking_lock();
                rx.recv()
            };
            let Ok(event) = event else { break };
            match event {
                ConnectionEvent::Connected => {
                    fire_callback(&callbacks, &keep_alive, "connect", EventArgs::None);
                }
                ConnectionEvent::Authenticated { data } => {
                    inject_test_panic(test_panic.as_deref(), "ws_events");
                    // The initial authentication: decide, atomically with the
                    // worker's abort, whether it is reported (#44).
                    if decision.load(Ordering::SeqCst) == AUTH_PENDING {
                        if ending.load(Ordering::SeqCst) {
                            // disconnect() came first. The queued Disconnect
                            // closes the connection; nothing else settles.
                            if decide(&decision, AUTH_ABORTED) {
                                settle(&auth, AuthOutcome::Failed(CONNECT_ABORTED.to_string()));
                            }
                            continue;
                        }
                        if !decide(&decision, AUTH_REPORTED) {
                            continue; // the worker aborted first
                        }
                    } else if decision.load(Ordering::SeqCst) == AUTH_ABORTED || ending.load(Ordering::SeqCst) {
                        continue; // a re-authentication after the connection was given up
                    }
                    connected.store(true, Ordering::SeqCst);
                    fire_and_settle(
                        &callbacks,
                        &keep_alive,
                        "authenticated",
                        EventArgs::Json(data.clone()),
                        &auth,
                        AuthOutcome::Authenticated(data),
                        None,
                    );
                }
                ConnectionEvent::Unauthenticated { data, .. } => {
                    fire_and_settle(
                        &callbacks,
                        &keep_alive,
                        "unauthenticated",
                        EventArgs::Json(data.clone()),
                        &auth,
                        AuthOutcome::Rejected(data),
                        Some(&ending),
                    );
                }
                ConnectionEvent::Error { message, code } => {
                    let rejection = AuthOutcome::Failed(format!("[{}] {}", code, message));
                    fire_and_settle(
                        &callbacks,
                        &keep_alive,
                        "error",
                        EventArgs::Error { message, code: Some(code) },
                        &auth,
                        rejection,
                        Some(&ending),
                    );
                }
                ConnectionEvent::Disconnected { code, reason, will_reconnect, .. } => {
                    if !will_reconnect {
                        ending.store(true, Ordering::SeqCst);
                        // Core's dispatch task ends without reconnecting.
                        dispatch_ended.store(true, Ordering::SeqCst);
                    }
                    fire_callback(
                        &callbacks,
                        &keep_alive,
                        "disconnect",
                        EventArgs::Json(serde_json::json!({ "code": code, "reason": reason })),
                    );
                }
                ConnectionEvent::Reconnecting { attempt } => {
                    fire_callback(
                        &callbacks,
                        &keep_alive,
                        "reconnect",
                        EventArgs::Json(serde_json::json!({ "attempt": attempt })),
                    );
                }
                ConnectionEvent::ReconnectFailed { attempts } => {
                    ending.store(true, Ordering::SeqCst);
                    fire_callback(
                        &callbacks,
                        &keep_alive,
                        "error",
                        EventArgs::Error {
                            message: format!("Reconnection failed after {} attempts", attempts),
                            code: None,
                        },
                    );
                    // Core's dispatch task has ended for good.
                    dispatch_ended.store(true, Ordering::SeqCst);
                }
                // `HeartbeatTimeout` is followed by `Disconnected`, which reports it.
                _ => {}
            }
        };
        if let Err(payload) = std::panic::catch_unwind(AssertUnwindSafe(run)) {
            report_panic(
                &PanicContext {
                    callbacks: &callbacks,
                    keep_alive: &keep_alive,
                    auth: &auth,
                    connected: &connected,
                    closed: &closed,
                    ending: &ending,
                    reported: &panic_reported,
                },
                "event",
                &*payload,
            );
            // Nothing reports the connection's end any more, and the worker
            // only exits once told core stopped: shut the connection down
            // rather than leave it (and the process) running unobserved.
            dispatch_ended.store(true, Ordering::SeqCst);
        }
    });
}

/// Error code reported for a panicked WebSocket thread (#25).
const PANIC_CODE: i32 = -1;

/// What a panicked worker or event thread needs to report it (#25).
struct PanicContext<'a> {
    callbacks: &'a Arc<Mutex<EventCallbacks>>,
    keep_alive: &'a KeepAlive,
    auth: &'a AuthSlot,
    connected: &'a AtomicBool,
    closed: &'a AtomicBool,
    ending: &'a AtomicBool,
    /// Shared by the connection's worker and event thread, so a panic on
    /// both reports one `error`.
    reported: &'a AtomicBool,
}

/// Report a panic on a connection's `thread` so the connection does not go
/// silently dead (#25): mark it closed, fire `error` (code -1) — rejecting a
/// still-pending `connect()` after it — and `disconnect` if it was connected.
/// Only the first panic of a connection fires them.
///
/// Both events go through [`fire_callback`], so each carries a keep-alive
/// clone and is delivered before the process may exit (#30). `ending` is set
/// first, so connect() may be called again from either listener (#44).
fn report_panic(ctx: &PanicContext<'_>, thread: &str, payload: &(dyn std::any::Any + Send)) {
    let detail = payload
        .downcast_ref::<&str>()
        .map(|message| message.to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "unknown panic".to_string());
    let reason = format!("WebSocket {} thread panicked: {}", thread, detail);

    ctx.ending.store(true, Ordering::SeqCst);
    let was_connected = ctx.connected.swap(false, Ordering::SeqCst);
    ctx.closed.store(true, Ordering::SeqCst);
    // The other thread already reported this connection's end.
    if ctx.reported.swap(true, Ordering::SeqCst) {
        return;
    }

    fire_and_settle(
        ctx.callbacks,
        ctx.keep_alive,
        "error",
        EventArgs::Error { message: reason.clone(), code: Some(PANIC_CODE) },
        ctx.auth,
        AuthOutcome::Failed(format!("[{}] {}", PANIC_CODE, reason)),
        None,
    );
    if was_connected {
        fire_callback(
            ctx.callbacks,
            ctx.keep_alive,
            "disconnect",
            EventArgs::Json(serde_json::json!({ "code": null, "reason": reason })),
        );
    }
}

/// `FUGLE_MARKETDATA_TEST_PANIC`, naming where a test wants a WebSocket
/// thread to panic (#25). Read on the JS thread when connecting, so tests can
/// set it through `process.env`. Debug builds only.
#[cfg(debug_assertions)]
fn test_panic_site() -> Option<String> {
    std::env::var("FUGLE_MARKETDATA_TEST_PANIC").ok()
}

#[cfg(not(debug_assertions))]
fn test_panic_site() -> Option<String> {
    None
}

/// Panic if the test asked for one at `here` (`ws_worker`, `ws_events`).
#[cfg(debug_assertions)]
fn inject_test_panic(site: Option<&str>, here: &str) {
    if site == Some(here) {
        panic!("injected test panic at {}", here);
    }
}

#[cfg(not(debug_assertions))]
#[inline(always)]
fn inject_test_panic(_site: Option<&str>, _here: &str) {}

/// Queue `event` for the registered JS listener, if any.
///
/// Callable from any thread. The listener's threadsafe function is weak, so
/// the call carries a `keep_alive` clone that is dropped on the JS thread
/// once the listener has returned (see [`KeepAlive`]). An exception the
/// listener throws is handed back unchanged, which napi-rs reports as an
/// uncaught exception exactly as a plain `call()` would.
fn fire_callback(
    callbacks: &Arc<Mutex<EventCallbacks>>,
    keep_alive: &KeepAlive,
    event: &str,
    data: EventArgs,
) {
    fire_callback_then(callbacks, keep_alive, event, data, || {});
}

/// Fire `event`, then settle the pending `connect()` with `outcome` once the
/// listener has returned, so it runs before the Promise settles as it did in
/// 1.x, which emitted before settling (#23). Nothing is settled if
/// `connect()` already was.
///
/// `ending`, when given and `connect()` is still pending, is set before the
/// event fires, so calling connect() again from the listener or the
/// rejection is not refused as already connected (#44).
fn fire_and_settle(
    callbacks: &Arc<Mutex<EventCallbacks>>,
    keep_alive: &KeepAlive,
    event: &str,
    data: EventArgs,
    auth: &AuthSlot,
    outcome: AuthOutcome,
    ending: Option<&AtomicBool>,
) {
    let pending = auth.lock().map(|guard| guard.is_some()).unwrap_or(false);
    if !pending {
        fire_callback(callbacks, keep_alive, event, data);
        return;
    }
    if let Some(ending) = ending {
        ending.store(true, Ordering::SeqCst);
    }
    let auth = Arc::clone(auth);
    fire_callback_then(callbacks, keep_alive, event, data, move || settle(&auth, outcome));
}

/// [`fire_callback`], running `then` once the listener has returned — or
/// right away when no listener is registered or the call cannot be queued.
fn fire_callback_then(
    callbacks: &Arc<Mutex<EventCallbacks>>,
    keep_alive: &KeepAlive,
    event: &str,
    data: EventArgs,
    then: impl FnOnce() + Send + 'static,
) {
    fn run_then<F: FnOnce()>(then: &Mutex<Option<F>>) {
        if let Some(then) = then.lock().ok().and_then(|mut guard| guard.take()) {
            then();
        }
    }

    let then = Arc::new(Mutex::new(Some(then)));
    // A panic while holding the lock must not also silence the events that
    // report it (#25).
    {
        let cb = callbacks.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
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
            let keep_alive = Arc::clone(keep_alive);
            let then_after_call = Arc::clone(&then);
            let status = callback.call_with_return_value(
                data,
                ThreadsafeFunctionCallMode::NonBlocking,
                move |result, _env| {
                    run_then(&then_after_call);
                    drop(keep_alive);
                    result.map(|_| ())
                },
            );
            if status != Status::Ok {
                run_then(&then);
            }
            return;
        }
    }
    run_then(&then);
}

// Unit tests are disabled because ThreadsafeFunction requires Node.js runtime
// Integration tests are done via JavaScript (test_websocket.js)
