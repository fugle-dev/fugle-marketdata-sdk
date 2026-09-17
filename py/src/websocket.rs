//! Python WebSocket client wrapper
//!
//! Provides Python-friendly interface to marketdata-core WebSocket streaming.
//! Supports callback-based event handling and iterator-based message consumption.
//!
//! # Example (Python)
//!
//! ```python
//! from fugle_marketdata import WebSocketClient
//!
//! # Create client with API key
//! ws = WebSocketClient("your-api-key")
//!
//! # Callback mode: ws.stock.on("message", handler)
//! def on_message(msg):
//!     print(f"Received: {msg}")
//!
//! ws.stock.on("message", on_message)
//! ws.stock.connect()
//! ws.stock.subscribe("trades", "2330")
//!
//! # Or iterator mode:
//! for msg in ws.stock.messages():
//!     print(msg)
//! ```

use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_async_runtimes::tokio::future_into_py;
use std::panic::AssertUnwindSafe;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::callback::CallbackRegistry;
use crate::errors;
use crate::handoff::Handoff;

// ---------------------------------------------------------------------------
// Subscribe / unsubscribe parameter helpers
//
// Legacy fugle-marketdata-python's WebSocket client takes a single dict of
// arbitrary keys (`stock.subscribe({"channel": "trades", "symbol": "2330"})`).
// Our binding originally exposed only the positional shape; these helpers
// let the same methods accept dict OR string positional + kwargs without
// breaking either form.
// ---------------------------------------------------------------------------

/// Resolve `(symbol, symbols)` kwargs into a non-empty `Vec<String>`.
fn resolve_symbol_args(
    symbol: Option<&str>,
    symbols: Option<Vec<String>>,
) -> PyResult<Vec<String>> {
    match (symbol, symbols) {
        (Some(s), None) => Ok(vec![s.to_string()]),
        (None, Some(list)) if !list.is_empty() => Ok(list),
        (None, Some(_)) => Err(pyo3::exceptions::PyValueError::new_err(
            "subscribe(symbols=[]) is empty - provide at least one symbol",
        )),
        (Some(_), Some(_)) => Err(pyo3::exceptions::PyValueError::new_err(
            "subscribe() accepts either `symbol` or `symbols`, not both",
        )),
        (None, None) => Err(pyo3::exceptions::PyValueError::new_err(
            "subscribe() requires either `symbol` or `symbols`",
        )),
    }
}

/// Try to read a boolean from `dict[primary]`, falling back to `dict[fallback]`.
/// Returns `None` if neither key is present.
fn dict_bool_alias(
    d: &Bound<'_, PyDict>,
    primary: &str,
    fallback: &str,
) -> PyResult<Option<bool>> {
    if let Some(v) = d.get_item(primary)? {
        return Ok(Some(v.extract::<bool>()?));
    }
    if let Some(v) = d.get_item(fallback)? {
        return Ok(Some(v.extract::<bool>()?));
    }
    Ok(None)
}

/// Extract `(channel, symbols, odd_lot)` from a stock subscribe dict.
/// Accepts both `oddLot` and `odd_lot` keys for legacy parity.
fn extract_stock_subscribe_dict(
    d: &Bound<'_, PyDict>,
) -> PyResult<(String, Vec<String>, bool)> {
    let channel = d
        .get_item("channel")?
        .ok_or_else(|| {
            pyo3::exceptions::PyValueError::new_err("subscribe(dict): missing 'channel'")
        })?
        .extract::<String>()?;

    let symbols: Vec<String> = match (d.get_item("symbol")?, d.get_item("symbols")?) {
        (Some(s), None) => vec![s.extract::<String>()?],
        (None, Some(list)) => {
            let v: Vec<String> = list.extract()?;
            if v.is_empty() {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "subscribe(dict): 'symbols' is empty",
                ));
            }
            v
        }
        (Some(_), Some(_)) => {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "subscribe(dict): provide either 'symbol' or 'symbols', not both",
            ));
        }
        (None, None) => {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "subscribe(dict): missing 'symbol' or 'symbols'",
            ));
        }
    };

    let odd_lot = dict_bool_alias(d, "oddLot", "odd_lot")?.unwrap_or(false);

    Ok((channel, symbols, odd_lot))
}

/// Extract `(channel, symbols, after_hours)` from a futopt subscribe dict.
fn extract_futopt_subscribe_dict(
    d: &Bound<'_, PyDict>,
) -> PyResult<(String, Vec<String>, bool)> {
    let channel = d
        .get_item("channel")?
        .ok_or_else(|| {
            pyo3::exceptions::PyValueError::new_err("subscribe(dict): missing 'channel'")
        })?
        .extract::<String>()?;

    let symbols: Vec<String> = match (d.get_item("symbol")?, d.get_item("symbols")?) {
        (Some(s), None) => vec![s.extract::<String>()?],
        (None, Some(list)) => {
            let v: Vec<String> = list.extract()?;
            if v.is_empty() {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "subscribe(dict): 'symbols' is empty",
                ));
            }
            v
        }
        (Some(_), Some(_)) => {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "subscribe(dict): provide either 'symbol' or 'symbols', not both",
            ));
        }
        (None, None) => {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "subscribe(dict): missing 'symbol' or 'symbols'",
            ));
        }
    };

    let after_hours = dict_bool_alias(d, "afterHours", "after_hours")?.unwrap_or(false);

    Ok((channel, symbols, after_hours))
}

/// Extract a list of subscription IDs from an unsubscribe dict.
/// Accepts `{"id": "..."}` or `{"ids": [...]}`.
fn extract_unsubscribe_dict(d: &Bound<'_, PyDict>) -> PyResult<Vec<String>> {
    match (d.get_item("id")?, d.get_item("ids")?) {
        (Some(s), None) => Ok(vec![s.extract::<String>()?]),
        (None, Some(list)) => {
            let v: Vec<String> = list.extract()?;
            if v.is_empty() {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "unsubscribe(dict): 'ids' is empty",
                ));
            }
            Ok(v)
        }
        (Some(_), Some(_)) => Err(pyo3::exceptions::PyValueError::new_err(
            "unsubscribe(dict): provide either 'id' or 'ids', not both",
        )),
        (None, None) => Err(pyo3::exceptions::PyValueError::new_err(
            "unsubscribe(dict): missing 'id' or 'ids'",
        )),
    }
}

/// Resolve `(subscription_id, ids)` kwargs into a non-empty `Vec<String>`.
fn resolve_unsubscribe_args(
    subscription_id: Option<&str>,
    ids: Option<Vec<String>>,
) -> PyResult<Vec<String>> {
    match (subscription_id, ids) {
        (Some(id), None) => Ok(vec![id.to_string()]),
        (None, Some(list)) if !list.is_empty() => Ok(list),
        (None, Some(_)) => Err(pyo3::exceptions::PyValueError::new_err(
            "unsubscribe(ids=[]) is empty - provide at least one id",
        )),
        (Some(_), Some(_)) => Err(pyo3::exceptions::PyValueError::new_err(
            "unsubscribe() accepts either `subscription_id` or `ids`, not both",
        )),
        (None, None) => Err(pyo3::exceptions::PyValueError::new_err(
            "unsubscribe() requires either `subscription_id` or `ids`",
        )),
    }
}

/// Auto-reconnect configuration
///
/// Controls automatic reconnection behavior when connection is lost.
///
/// # Example (Python)
///
/// ```python
/// from fugle_marketdata import ReconnectConfig
///
/// config = ReconnectConfig(
///     enabled=True,
///     max_attempts=5,
///     initial_delay_ms=1000,
///     max_delay_ms=30000
/// )
/// ```
#[pyclass(from_py_object)]
#[derive(Clone)]
pub struct ReconnectConfig {
    /// Whether auto-reconnect is enabled
    #[pyo3(get)]
    pub enabled: bool,
    /// Maximum number of reconnection attempts
    #[pyo3(get)]
    pub max_attempts: u32,
    /// Initial delay in milliseconds for exponential backoff
    #[pyo3(get)]
    pub initial_delay_ms: u64,
    /// Maximum delay in milliseconds (caps exponential backoff)
    #[pyo3(get)]
    pub max_delay_ms: u64,
}

#[pymethods]
impl ReconnectConfig {
    /// Create a new reconnect configuration
    ///
    /// Args:
    ///     enabled: Whether auto-reconnect is enabled (default: True)
    ///     max_attempts: Maximum reconnection attempts (default: 5, min: 1)
    ///     initial_delay_ms: Initial delay for exponential backoff (default: 1000ms, min: 100ms)
    ///     max_delay_ms: Maximum delay cap (default: 60000ms = 60s)
    ///
    /// Raises:
    ///     ValueError: If validation fails
    #[new]
    #[pyo3(signature = (*, enabled=true, max_attempts=5, initial_delay_ms=1000, max_delay_ms=60000))]
    pub fn new(
        enabled: bool,
        max_attempts: u32,
        initial_delay_ms: u64,
        max_delay_ms: u64,
    ) -> PyResult<Self> {
        // Validate using core's validation logic (fail fast)
        let _ = marketdata_core::ReconnectionConfig::new(
            max_attempts,
            Duration::from_millis(initial_delay_ms),
            Duration::from_millis(max_delay_ms),
        )
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

        Ok(Self {
            enabled,
            max_attempts,
            initial_delay_ms,
            max_delay_ms,
        })
    }

    /// Create a default reconnect configuration (enabled with 5 attempts)
    #[staticmethod]
    pub fn default_config() -> Self {
        Self {
            enabled: true,
            max_attempts: 5,
            initial_delay_ms: 1000,
            max_delay_ms: 60000,
        }
    }

    /// Create a disabled reconnect configuration
    #[staticmethod]
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            max_attempts: 5,
            initial_delay_ms: 1000,
            max_delay_ms: 60000,
        }
    }
}

impl ReconnectConfig {
    /// Convert to core ReconnectionConfig
    ///
    /// This should not fail since validation already happened in __new__
    pub fn to_core(&self) -> marketdata_core::ReconnectionConfig {
        let mut cfg = marketdata_core::ReconnectionConfig::new(
            self.max_attempts,
            Duration::from_millis(self.initial_delay_ms),
            Duration::from_millis(self.max_delay_ms),
        )
        .expect("Config already validated in constructor");
        // Honor the Python-level `enabled` flag. Without this, the binding
        // silently ignored `ReconnectConfig(enabled=False)`. Pre-0.4 the core
        // default was `enabled: false` so the bug was masked; in 0.4 the core
        // default flipped to `true`, making the leak observable.
        cfg.enabled = self.enabled;
        cfg
    }
}

impl Default for ReconnectConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_attempts: 5,
            initial_delay_ms: 1000,
            max_delay_ms: 60000,
        }
    }
}

/// Health check configuration for WebSocket connections
///
/// Configures ping/pong based connection monitoring.
///
/// # Example (Python)
///
/// ```python
/// from fugle_marketdata import HealthCheckConfig, WebSocketClient
///
/// # Custom health check
/// health_check = HealthCheckConfig(
///     enabled=True,
///     ping_interval=15000,  # 15 seconds
///     max_missed_pongs=3
/// )
///
/// ws = WebSocketClient(
///     api_key="your-key",
///     health_check=health_check
/// )
/// ```
#[pyclass(from_py_object)]
#[derive(Clone)]
pub struct HealthCheckConfig {
    /// Whether liveness detection is active (default: True in 3.0)
    #[pyo3(get)]
    pub enabled: bool,
    /// Maximum allowed gap between inbound frames before declaring the
    /// connection dead, in milliseconds. Default 35 000.
    #[pyo3(get)]
    pub heartbeat_timeout_ms: u64,
}

#[pymethods]
impl HealthCheckConfig {
    /// Create a new health check configuration.
    ///
    /// Args:
    ///     enabled: Whether liveness detection is active (default: True).
    ///     heartbeat_timeout_ms: Max gap between inbound frames before
    ///         the connection is declared dead. Default 35 000 ms (Fugle
    ///         server's 30 s heartbeat + 5 s buffer). Floor is 5 000 ms;
    ///         values below the live server's heartbeat period (30 s)
    ///         will cause repeated false disconnects.
    ///
    /// Raises:
    ///     ValueError: If `heartbeat_timeout_ms` < 5 000.
    ///
    /// Example:
    ///     ```python
    ///     # Default (enabled, 35s timeout)
    ///     config = HealthCheckConfig()
    ///
    ///     # Tighter detection (only safe once server side supports
    ///     # negotiated heartbeat interval)
    ///     config = HealthCheckConfig(heartbeat_timeout_ms=10000)
    ///
    ///     # Opt out of liveness detection
    ///     config = HealthCheckConfig(enabled=False)
    ///     ```
    #[new]
    #[pyo3(signature = (*, enabled=true, heartbeat_timeout_ms=35_000))]
    pub fn new(enabled: bool, heartbeat_timeout_ms: u64) -> PyResult<Self> {
        // Validate via core even when disabled, for early feedback on bad input.
        let duration = Duration::from_millis(heartbeat_timeout_ms);
        let _ = marketdata_core::HealthCheckConfig::with_timeout(duration)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

        Ok(Self {
            enabled,
            heartbeat_timeout_ms,
        })
    }
}

impl HealthCheckConfig {
    /// Convert to core HealthCheckConfig
    ///
    /// This should not fail since validation already happened in __new__
    pub fn to_core(&self) -> marketdata_core::HealthCheckConfig {
        let mut cfg = marketdata_core::HealthCheckConfig::with_timeout(
            Duration::from_millis(self.heartbeat_timeout_ms),
        )
        .expect("Config already validated in constructor");
        cfg.enabled = self.enabled;
        cfg
    }
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            heartbeat_timeout_ms: 35_000,
        }
    }
}

/// Python WebSocket client for Fugle market data streaming
///
/// # Example (Python)
///
/// ```python
/// from fugle_marketdata import WebSocketClient, ReconnectConfig, HealthCheckConfig
///
/// # Create client with API key
/// ws = WebSocketClient(api_key="your-api-key")
///
/// # With custom reconnect config
/// rc = ReconnectConfig(max_attempts=10)
/// ws = WebSocketClient(api_key="your-key", reconnect=rc)
///
/// # Access stock streaming
/// ws.stock.connect()
/// ws.stock.subscribe("trades", "2330")
/// ```
#[pyclass]
pub struct WebSocketClient {
    api_key: String,
    base_url: Option<String>,
    stock_version: marketdata_core::websocket::StockVersion,
    futopt_version: marketdata_core::websocket::FutOptVersion,
    reconnect_config: ReconnectConfig,
    health_check_config: HealthCheckConfig,
    tls: marketdata_core::TlsConfig,
    message_queue: MessageQueueSettings,
}

#[pymethods]
impl WebSocketClient {
    /// Create a new WebSocket client with authentication
    ///
    /// Provide exactly one authentication method (empty or whitespace-only
    /// values count as not provided):
    ///   - api_key: Your Fugle API key
    ///   - bearer_token: Bearer token for authentication
    ///   - sdk_token: SDK token for authentication
    ///
    /// Optional configuration:
    ///   - base_url: Custom base URL for WebSocket endpoint
    ///   - reconnect: ReconnectConfig for auto-reconnection behavior
    ///   - health_check: HealthCheckConfig for connection monitoring
    ///   - message_overflow: "drop_newest" (default) keeps at most
    ///     `message_buffer` unread messages and drops new ones meanwhile,
    ///     reporting them through the `messages_dropped` callback;
    ///     "unbounded" never drops, and memory grows while you lag
    ///   - message_buffer: unread messages held (default 4096)
    ///
    /// Returns:
    ///     A new WebSocketClient instance
    ///
    /// Raises:
    ///     MarketDataError: code 1004 if zero or multiple auth methods provided
    ///
    /// Example:
    ///     ```python
    ///     # Simple usage
    ///     ws = WebSocketClient(api_key="your-key")
    ///
    ///     # With custom reconnect
    ///     rc = ReconnectConfig(max_attempts=10)
    ///     ws = WebSocketClient(api_key="key", reconnect=rc)
    ///
    ///     # With health check
    ///     hc = HealthCheckConfig(enabled=True, interval_ms=15000)
    ///     ws = WebSocketClient(api_key="key", health_check=hc)
    ///     ```
    #[new]
    #[pyo3(signature = (*, api_key=None, bearer_token=None, sdk_token=None, base_url=None, version=None, reconnect=None, health_check=None, tls_ca_file=None, tls_root_cert_pem=None, tls_accept_invalid_certs=false, message_overflow=None, message_buffer=None))]
    pub fn new(
        py: Python<'_>,
        api_key: Option<String>,
        bearer_token: Option<String>,
        sdk_token: Option<String>,
        base_url: Option<String>,
        version: Option<&Bound<'_, pyo3::types::PyDict>>,
        reconnect: Option<&Bound<'_, ReconnectConfig>>,
        health_check: Option<&Bound<'_, HealthCheckConfig>>,
        tls_ca_file: Option<String>,
        tls_root_cert_pem: Option<Vec<u8>>,
        tls_accept_invalid_certs: bool,
        message_overflow: Option<String>,
        message_buffer: Option<i64>,
    ) -> PyResult<Self> {
        // Core requires exactly one non-blank credential (ConfigError, 1004).
        // Every kind is still sent as the API key here (#91).
        let auth_key = match marketdata_core::Auth::from_credentials(api_key, bearer_token, sdk_token)
            .map_err(crate::errors::to_py_err)?
        {
            marketdata_core::Auth::ApiKey(key)
            | marketdata_core::Auth::BearerToken(key)
            | marketdata_core::Auth::SdkToken(key) => key,
        };

        // Extract configs with defaults (clone from Bound to avoid lifetime issues)
        let reconnect_config = if let Some(cfg) = reconnect {
            cfg.borrow().clone()
        } else {
            ReconnectConfig::default()
        };

        let health_check_config = if let Some(cfg) = health_check {
            cfg.borrow().clone()
        } else {
            HealthCheckConfig::default()
        };

        let tls = crate::tls_kwargs::parse_tls_kwargs(
            py,
            tls_ca_file,
            tls_root_cert_pem,
            tls_accept_invalid_certs,
        )?;

        let (stock_version, futopt_version) = parse_ws_versions(version)?;
        let message_queue = MessageQueueSettings::parse(message_overflow.as_deref(), message_buffer)?;

        // Resolve both endpoints now so a bad `base_url` raises here rather
        // than from `.stock.connect()` much later. Matches the official SDK,
        // which rejects a versioned baseUrl at construction.
        for product in [WsProduct::Stock, WsProduct::FutOpt] {
            build_stream_config(
                &auth_key,
                base_url.as_deref(),
                product,
                stock_version,
                futopt_version,
            )
            .map_err(|e| pyo3::exceptions::PyTypeError::new_err(format!("{e}")))?;
        }

        Ok(Self {
            api_key: auth_key,
            base_url,
            stock_version,
            futopt_version,
            reconnect_config,
            health_check_config,
            tls,
            message_queue,
        })
    }

    /// Access stock market data WebSocket streaming
    ///
    /// Returns:
    ///     StockWebSocketClient for stock streaming with inherited config
    #[getter]
    pub fn stock(&self) -> StockWebSocketClient {
        StockWebSocketClient::new(
            self.api_key.clone(),
            self.base_url.clone(),
            self.stock_version,
            self.futopt_version,
            self.reconnect_config.clone(),
            self.health_check_config.clone(),
            self.tls.clone(),
            self.message_queue,
        )
    }

    /// Access futures and options WebSocket streaming
    ///
    /// Returns:
    ///     FutOptWebSocketClient for FutOpt streaming with inherited config
    #[getter]
    pub fn futopt(&self) -> FutOptWebSocketClient {
        FutOptWebSocketClient::new(
            self.api_key.clone(),
            self.base_url.clone(),
            self.stock_version,
            self.futopt_version,
            self.reconnect_config.clone(),
            self.health_check_config.clone(),
            self.tls.clone(),
            self.message_queue,
        )
    }
}

/// Resolve the `version` kwarg into the two per-product enums.
///
/// The official SDK takes a per-product mapping (`{'futopt': 'v1.1'}`) and
/// validates it at runtime. Core models the same thing as one enum per product
/// so an unsupported pairing cannot be built at all — but a Python caller
/// hands us untyped strings, so the validation has to happen here, with the
/// same error text the official SDK raises.
/// `message_overflow` / `message_buffer` of a `WebSocketClient` (#46).
#[derive(Clone, Copy)]
struct MessageQueueSettings {
    overflow: marketdata_core::MessageOverflow,
    buffer: usize,
}

impl MessageQueueSettings {
    fn parse(overflow: Option<&str>, buffer: Option<i64>) -> PyResult<Self> {
        let overflow = match overflow {
            None | Some("drop_newest") => marketdata_core::MessageOverflow::DropNewest,
            Some("unbounded") => marketdata_core::MessageOverflow::Unbounded,
            Some(other) => {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "message_overflow must be \"drop_newest\" or \"unbounded\", got {other:?}"
                )))
            }
        };
        let buffer = match buffer {
            None => marketdata_core::websocket::DEFAULT_MESSAGE_BUFFER,
            Some(n) if n > 0 => usize::try_from(n).map_err(|_| {
                pyo3::exceptions::PyValueError::new_err(format!("message_buffer is too large: {n}"))
            })?,
            Some(n) => {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "message_buffer must be a positive integer, got {n}"
                )))
            }
        };
        Ok(Self { overflow, buffer })
    }

    fn apply(self, config: &mut marketdata_core::ConnectionConfig) {
        config.message_overflow = self.overflow;
        config.message_buffer = self.buffer;
    }
}

fn parse_ws_versions(
    version: Option<&Bound<'_, pyo3::types::PyDict>>,
) -> PyResult<(marketdata_core::websocket::StockVersion, marketdata_core::websocket::FutOptVersion)> {
    use marketdata_core::websocket::{FutOptVersion, StockVersion};
    use pyo3::types::PyAnyMethods;

    let mut stock = StockVersion::default();
    let mut futopt = FutOptVersion::default();

    let Some(map) = version else {
        return Ok((stock, futopt));
    };

    for (key, value) in map.iter() {
        let product: String = key.extract().map_err(|_| {
            pyo3::exceptions::PyTypeError::new_err(
                "version keys must be product names: 'stock' or 'futopt'",
            )
        })?;
        let requested: String = value.extract().map_err(|_| {
            pyo3::exceptions::PyTypeError::new_err(format!(
                "version['{product}'] must be a version string, e.g. 'v1.1'"
            ))
        })?;

        match product.as_str() {
            "stock" => {
                stock = match requested.as_str() {
                    "v1.0" => StockVersion::V1_0,
                    other => {
                        return Err(pyo3::exceptions::PyTypeError::new_err(format!(
                            "stock streaming does not support {other} (supported: v1.0). \
                             Remove it from the version mapping to use v1.0."
                        )))
                    }
                }
            }
            "futopt" => {
                futopt = match requested.as_str() {
                    "v1.0" => FutOptVersion::V1_0,
                    "v1.1" => FutOptVersion::V1_1,
                    other => {
                        return Err(pyo3::exceptions::PyTypeError::new_err(format!(
                            "futopt streaming does not support {other} (supported: v1.0, v1.1). \
                             Remove it from the version mapping to use v1.1."
                        )))
                    }
                }
            }
            other => {
                return Err(pyo3::exceptions::PyTypeError::new_err(format!(
                    "unknown product '{other}' in version mapping (known: stock, futopt)"
                )))
            }
        }
    }

    Ok((stock, futopt))
}

/// Resolve a streaming endpoint through core's factory.
///
/// Centralised so the three call sites (sync stock, sync futopt, async stock)
/// cannot drift on base-URL semantics — which is exactly what happened before
/// 0.8.0, when each one hand-rolled `format!("{base}/stock/streaming")`.
fn build_stream_config(
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

#[derive(Clone, Copy)]
enum WsProduct {
    Stock,
    FutOpt,
}

/// Internal WebSocket state (not Send/Sync safe, managed via Mutex)
///
/// The `inner` is wrapped in Arc to allow cloning the reference out of
/// the Mutex before async operations (avoiding holding MutexGuard across await).
struct WebSocketState {
    inner: Arc<marketdata_core::aio::WebSocketClient>,
    /// Messages no `message` callback took, for `messages()` iterators.
    handoff: Arc<Handoff>,
    /// Set by `disconnect()`: the stream reader stops waiting on a full
    /// `handoff`.
    stop: Arc<AtomicBool>,
}

/// Shared so blocking calls can clone it out of its lock before they block.
type SharedRuntime = Arc<tokio::runtime::Runtime>;

fn lock_err<T>(e: std::sync::PoisonError<T>) -> PyErr {
    pyo3::exceptions::PyRuntimeError::new_err(format!("Lock error: {}", e))
}

// Blocking calls wait with the GIL released (#39), so they must not hold a
// client lock either: a thread that holds the GIL could be queued on that lock
// while the blocked call needs the GIL back to return. Locks are therefore
// only taken briefly, to clone handles out, and never across `py.detach`.

/// Clone the connected client and its runtime out of their locks.
fn live_handles(
    state: &Mutex<Option<WebSocketState>>,
    runtime: &Mutex<Option<SharedRuntime>>,
) -> PyResult<(Arc<marketdata_core::aio::WebSocketClient>, SharedRuntime)> {
    let inner = state
        .lock()
        .map_err(lock_err)?
        .as_ref()
        .map(|s| Arc::clone(&s.inner))
        .ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err("Not connected. Call connect() first.")
        })?;
    let runtime = runtime.lock().map_err(lock_err)?.clone().ok_or_else(|| {
        pyo3::exceptions::PyRuntimeError::new_err("Runtime not initialized")
    })?;
    Ok((inner, runtime))
}

/// Run `fut` to completion on `runtime` with the GIL released.
fn block_on_detached<F>(py: Python<'_>, runtime: SharedRuntime, fut: F) -> F::Output
where
    F: std::future::Future + Send,
    F::Output: Send,
{
    py.detach(move || runtime.block_on(fut))
}

/// `messages(timeout_ms=...)` no longer changes iteration (#68): warn when it
/// is passed.
fn warn_timeout_ms_deprecated(py: Python<'_>, timeout_ms: Option<u64>) -> PyResult<()> {
    if timeout_ms.is_none() {
        return Ok(());
    }
    PyErr::warn(
        py,
        &py.get_type::<pyo3::exceptions::PyDeprecationWarning>(),
        c"messages(timeout_ms=...) is deprecated and ignored: iteration yields messages only and stops once the connection is gone",
        1,
    )
}

/// `messages()` iterators hold as many unread messages as core's queue.
fn handoff_capacity(config: &marketdata_core::ConnectionConfig) -> Option<usize> {
    match config.message_overflow {
        marketdata_core::MessageOverflow::Unbounded => None,
        _ => Some(config.message_buffer),
    }
}

/// Start the thread that forwards a connection's core stream — its events
/// and messages, in the order core produced them (#68) — to the callbacks.
///
/// Started before `connect()`: core queues `Connected` and the
/// `Authenticated` / `Unauthenticated` outcome while `connect()` runs, and a
/// rejected connect still has to reach `unauthenticated`. The thread exits
/// once the core client is dropped and the stream closes.
///
/// A message is delivered only between an `Authenticated` this thread
/// forwarded and the next `Disconnected`, so frames of a rejected
/// authentication never reach `message`. Whether a `message` callback is
/// registered is checked for each message: if one is, it gets the message;
/// otherwise the message waits in `handoff` for a `messages()` iterator.
fn spawn_stream_reader(
    name: &str,
    stream: Arc<marketdata_core::StreamReceiver>,
    callbacks: Arc<CallbackRegistry>,
    handoff: Arc<Handoff>,
    stop: Arc<AtomicBool>,
    test_panic: Option<String>,
) -> PyResult<std::thread::JoinHandle<()>> {
    use marketdata_core::websocket::{ConnectionEvent, StreamItem};

    std::thread::Builder::new()
        .name(name.to_string())
        .spawn(move || {
            // The kind of item being handled, to name the thread in a panic
            // report (#25).
            let handling = std::cell::Cell::new("event");
            let run = || {
                let mut authenticated = false;
                while let Ok(item) = stream.receive() {
                    match item {
                        StreamItem::Event(event) => {
                            handling.set("event");
                            inject_test_panic(test_panic.as_deref(), "ws_events");
                            match event {
                                ConnectionEvent::Authenticated { .. } => authenticated = true,
                                ConnectionEvent::Unauthenticated { .. }
                                | ConnectionEvent::Disconnected { .. } => authenticated = false,
                                _ => {}
                            }
                            Python::attach(|py| forward_event(py, &callbacks, event));
                        }
                        StreamItem::Message(msg) if authenticated => {
                            handling.set("message");
                            inject_test_panic(test_panic.as_deref(), "ws_messages");
                            if callbacks.count(crate::callback::EventType::Message) == 0 {
                                handoff.push(msg, &stop);
                                continue;
                            }
                            Python::attach(|py| {
                                if let Ok(dict) = message_to_dict(py, &msg) {
                                    let args = pyo3::types::PyTuple::new(py, [dict.into_any()])
                                        .expect("Failed to create tuple");
                                    callbacks.invoke(py, crate::callback::EventType::Message, &args);
                                }
                            });
                        }
                        _ => {}
                    }
                }
            };
            // Later items are lost, but the panic is not silent (#25).
            let result = std::panic::catch_unwind(AssertUnwindSafe(run));
            // Iterators end once they have read what is queued.
            handoff.close();
            if let Err(payload) = result {
                report_thread_panic(&callbacks, handling.get(), &*payload);
            }
        })
        .map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!(
                "Failed to spawn WebSocket stream reader thread: {e}"
            ))
        })
}

/// Report a panic on a WebSocket `thread` through the `error` callbacks, so
/// it does not go unnoticed (#25).
fn report_thread_panic(
    callbacks: &CallbackRegistry,
    thread: &str,
    payload: &(dyn std::any::Any + Send),
) {
    let detail = payload
        .downcast_ref::<&str>()
        .map(|message| message.to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "unknown panic".to_string());
    let message = format!("WebSocket {thread} thread panicked: {detail}");
    let info = marketdata_core::ErrorInfo::new(
        marketdata_core::error_code::THREAD_PANIC,
        marketdata_core::ErrorKind::Protocol,
        message,
    );
    Python::attach(|py| callbacks.invoke_error(py, &info));
}

/// `FUGLE_MARKETDATA_TEST_PANIC`, naming where a test wants a WebSocket
/// thread to panic (#25). Read when connecting. Debug builds only.
#[cfg(debug_assertions)]
fn test_panic_site() -> Option<String> {
    std::env::var("FUGLE_MARKETDATA_TEST_PANIC").ok()
}

#[cfg(not(debug_assertions))]
fn test_panic_site() -> Option<String> {
    None
}

/// Panic if the test asked for one at `here` (`ws_events`, `ws_messages`).
#[cfg(debug_assertions)]
fn inject_test_panic(site: Option<&str>, here: &str) {
    if site == Some(here) {
        panic!("injected test panic at {here}");
    }
}

#[cfg(not(debug_assertions))]
#[inline(always)]
fn inject_test_panic(_site: Option<&str>, _here: &str) {}

/// Map one core connection event onto the Python callback it stands for.
fn forward_event(
    py: Python<'_>,
    callbacks: &CallbackRegistry,
    event: marketdata_core::websocket::ConnectionEvent,
) {
    use marketdata_core::websocket::ConnectionEvent;
    match event {
        ConnectionEvent::Connected => callbacks.invoke_connect(py),
        ConnectionEvent::Authenticated { data } => callbacks.invoke_authenticated(py, &data),
        ConnectionEvent::Unauthenticated { data, .. } => {
            callbacks.invoke_unauthenticated(py, &data)
        }
        ConnectionEvent::Disconnected { code, reason, .. } => {
            callbacks.invoke_disconnect(py, code, &reason)
        }
        ConnectionEvent::Reconnecting { attempt } => callbacks.invoke_reconnect(py, attempt),
        ConnectionEvent::ReconnectFailed { attempts } => callbacks.invoke_error(
            py,
            &marketdata_core::ErrorInfo::new(
                marketdata_core::error_code::RECONNECT_FAILED,
                marketdata_core::ErrorKind::Network,
                format!("Reconnection failed after {} attempts", attempts),
            ),
        ),
        ConnectionEvent::Error(info) => callbacks.invoke_error(py, &info),
        ConnectionEvent::MessagesDropped { dropped, total } => {
            callbacks.invoke_messages_dropped(py, dropped, total)
        }
        _ => {}
    }
}

/// Wait for the stream reader with the GIL released, so every event core
/// queued before the stream closed has reached its callback (#54).
///
/// Call only after the core client is dropped, or the thread never ends. The
/// stream closes once the last `Arc` of the client goes, so this also waits
/// for calls still in flight on the same client from other threads.
fn join_reader_thread(py: Python<'_>, handle: std::thread::JoinHandle<()>) {
    py.detach(move || {
        let _ = handle.join();
    });
}

/// [`join_reader_thread`] on the thread stored by `connect()`.
///
/// A callback that disconnects runs on the stream reader and cannot wait for
/// itself, so it puts the handle back: the `disconnect()` whose close fired
/// that callback still finds it and waits for the remaining callbacks.
fn join_stored_reader_thread(py: Python<'_>, slot: &Mutex<Option<std::thread::JoinHandle<()>>>) {
    let Some(handle) = take_thread(slot) else { return };
    if handle.thread().id() == std::thread::current().id() {
        if let Ok(mut guard) = slot.lock() {
            *guard = Some(handle);
        }
        return;
    }
    join_reader_thread(py, handle);
}

/// Async counterpart of [`join_reader_thread`]: joins on the blocking pool so
/// the awaiting task does not stall a runtime worker. The awaiting task never
/// runs on the stream reader, so no self-join check is needed.
async fn join_reader_thread_async(handle: Option<std::thread::JoinHandle<()>>) {
    let Some(handle) = handle else { return };
    let _ = tokio::task::spawn_blocking(move || handle.join()).await;
}

fn take_thread(
    slot: &Mutex<Option<std::thread::JoinHandle<()>>>,
) -> Option<std::thread::JoinHandle<()>> {
    slot.lock().ok().and_then(|mut guard| guard.take())
}

/// Stock market WebSocket client
///
/// Access via `ws.stock`
///
/// Supports both iterator-based and callback-based message consumption.
/// A background thread delivers the connection's events and messages in
/// order: each message goes to the `message` callbacks if any are registered
/// when it arrives, otherwise to `messages()` iterators.
#[pyclass]
pub struct StockWebSocketClient {
    api_key: String,
    base_url: Option<String>,
    stock_version: marketdata_core::websocket::StockVersion,
    futopt_version: marketdata_core::websocket::FutOptVersion,
    reconnect_config: ReconnectConfig,
    health_check_config: HealthCheckConfig,
    tls: marketdata_core::TlsConfig,
    callbacks: Arc<CallbackRegistry>,
    // State is wrapped in Mutex<Option<>> for thread-safety
    state: Arc<Mutex<Option<WebSocketState>>>,
    runtime: Arc<Mutex<Option<SharedRuntime>>>,
    // Background thread control
    reader_thread_handle: Arc<Mutex<Option<std::thread::JoinHandle<()>>>>,
    message_queue: MessageQueueSettings,
    /// Dropped-message count of the current or last connection; outlives the
    /// core client, which `disconnect()` drops.
    messages_dropped: Arc<Mutex<Option<marketdata_core::MessagesDroppedHandle>>>,
}

impl StockWebSocketClient {
    fn new(
        api_key: String,
        base_url: Option<String>,
        stock_version: marketdata_core::websocket::StockVersion,
        futopt_version: marketdata_core::websocket::FutOptVersion,
        reconnect_config: ReconnectConfig,
        health_check_config: HealthCheckConfig,
        tls: marketdata_core::TlsConfig,
        message_queue: MessageQueueSettings,
    ) -> Self {
        Self {
            api_key,
            base_url,
            stock_version,
            futopt_version,
            reconnect_config,
            health_check_config,
            tls,
            callbacks: Arc::new(CallbackRegistry::new()),
            state: Arc::new(Mutex::new(None)),
            runtime: Arc::new(Mutex::new(None)),
            reader_thread_handle: Arc::new(Mutex::new(None)),
            message_queue,
            messages_dropped: Arc::new(Mutex::new(None)),
        }
    }

    fn build_config(&self) -> marketdata_core::ConnectionConfig {
        // `base_url` was already validated in `WebSocketClient::new`, so the
        // only way this can fail is a caller constructing the product client
        // directly — fall back to the production endpoint rather than panic.
        let mut config = build_stream_config(
            &self.api_key,
            self.base_url.as_deref(),
            WsProduct::Stock,
            self.stock_version,
            self.futopt_version,
        )
        .unwrap_or_else(|_| {
            marketdata_core::ConnectionConfig::fugle_stock(
                marketdata_core::AuthRequest::with_api_key(&self.api_key),
            )
        });
        config.tls = self.tls.clone();
        self.message_queue.apply(&mut config);
        config
    }

    /// Get or create the tokio runtime
    fn ensure_runtime(&self) -> Result<(), String> {
        let mut runtime_guard = self.runtime.lock().map_err(|e| e.to_string())?;
        if runtime_guard.is_none() {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .map_err(|e| format!("Failed to create tokio runtime: {}", e))?;
            *runtime_guard = Some(Arc::new(rt));
        }
        Ok(())
    }

}

#[pymethods]
impl StockWebSocketClient {
    /// Register a callback for an event type
    ///
    /// Supported events:
    ///   - "message" / "data": Called with message dict when data received
    ///   - "connect" / "connected": Called when the WebSocket opens, before authentication
    ///   - "authenticated": Called with the server's data (dict, or None) when it accepts credentials
    ///   - "unauthenticated": Called with the server's data (dict, or None) when it refuses credentials
    ///   - "disconnect" / "disconnected" / "close": Called when connection closed
    ///   - "reconnect" / "reconnecting": Called when reconnecting
    ///   - "error": Called with a single `err` argument (WebSocketError instance) when error occurs
    ///
    /// Args:
    ///     event: Event type string
    ///     callback: Python callable to invoke
    ///
    /// Example:
    ///     ```python
    ///     def on_message(msg):
    ///         print(f"Symbol: {msg.get('symbol')}, Price: {msg.get('price')}")
    ///
    ///     ws.stock.on("message", on_message)
    ///     ```
    #[pyo3(signature = (event, callback))]
    pub fn on(&self, event: &str, callback: &Bound<'_, PyAny>) -> PyResult<()> {
        self.callbacks.register(event, callback)
    }

    /// Remove all callbacks for an event type
    #[pyo3(signature = (event))]
    pub fn off(&self, event: &str) -> PyResult<()> {
        self.callbacks.unregister(event)
    }

    /// Connect to WebSocket server
    ///
    /// A background thread delivers the connection's events and messages in
    /// order: each message goes to the `message` callbacks if any are
    /// registered when it arrives, otherwise to `messages()` iterators.
    ///
    /// Raises:
    ///     MarketDataError: If connection fails
    pub fn connect(&self, py: Python<'_>) -> PyResult<()> {
        let test_panic = test_panic_site();
        // Ensure runtime exists
        self.ensure_runtime().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(e)
        })?;

        // Create WebSocket client with full config
        let config = self.build_config();
        let capacity = handoff_capacity(&config);
        let ws_client = marketdata_core::aio::WebSocketClient::with_full_config(
            config,
            self.reconnect_config.to_core(),
            self.health_check_config.to_core(),
        );
        *self.messages_dropped.lock().map_err(lock_err)? = Some(ws_client.messages_dropped_handle());

        let handoff = Arc::new(Handoff::new(capacity));
        let stop = Arc::new(AtomicBool::new(false));
        let reader_thread = spawn_stream_reader(
            "stock_ws_stream",
            ws_client.stream_receiver(),
            Arc::clone(&self.callbacks),
            Arc::clone(&handoff),
            Arc::clone(&stop),
            test_panic,
        )?;

        let runtime = self.runtime.lock().map_err(lock_err)?.clone().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err("Runtime not initialized")
        })?;

        // Connect with the GIL released: the handshake and auth ack may come
        // from a server that needs this interpreter to run (#39).
        let (ws_client, result) = py.detach(move || {
            let result = runtime.block_on(ws_client.connect());
            (ws_client, result)
        });

        if let Err(e) = result {
            // Dropping the client closes the stream; the reader first delivers
            // `unauthenticated` / `error`, so they fire before we raise.
            drop(ws_client);
            join_reader_thread(py, reader_thread);
            return Err(errors::to_py_err(e));
        }

        *self.state.lock().map_err(lock_err)? = Some(WebSocketState {
            inner: Arc::new(ws_client),
            handoff,
            stop,
        });
        *self.reader_thread_handle.lock().map_err(lock_err)? = Some(reader_thread);
        Ok(())
    }

    /// Disconnect from WebSocket server
    #[pyo3(signature = ())]
    pub fn disconnect(&self, py: Python<'_>) -> PyResult<()> {
        let state = self.state.lock().map_err(lock_err)?.take();

        if let Some(state) = state {
            // Messages before `Disconnected` still reach the callbacks, but
            // the reader no longer waits for an iterator to make room.
            state.stop.store(true, Ordering::SeqCst);
            // Take ownership of the runtime so dropping it aborts every
            // spawned task (dispatch, writer, health check). Without this,
            // those tasks keep their Arc<WebSocketClient> clones alive, the
            // stream never closes, and the stream reader blocks forever on
            // receive() — preventing Python from shutting down.
            let runtime = self.runtime.lock().map_err(lock_err)?.take();

            if let Some(rt) = runtime {
                // The close handshake waits on the server, so release the GIL (#39).
                py.detach(move || {
                    rt.block_on(async {
                        let _ = state.inner.disconnect().await;
                    });
                    // `rt` drops first → all spawned tasks aborted and futures
                    // dropped, releasing every Arc<WebSocketClient> clone they
                    // held. `state` drops next → core's WebSocketClient drops →
                    // its stream closes → the stream reader drains it and
                    // exits cleanly.
                    drop(rt);
                    drop(state);
                });
            }

            // Note: do NOT manually invoke_disconnect here. Core's
            // disconnect() emits a ConnectionEvent::Disconnected on its
            // stream, and the stream reader dispatches it to the user
            // callback. Calling it explicitly fires the
            // callback twice.
        }

        // The client is gone, so the stream reader drains and exits: the
        // `disconnect` callback has fired by the time this returns (#54).
        join_stored_reader_thread(py, &self.reader_thread_handle);

        Ok(())
    }

    /// Check if currently connected
    #[pyo3(signature = ())]
    pub fn is_connected(&self, py: Python<'_>) -> bool {
        match live_handles(&self.state, &self.runtime) {
            Ok((inner, runtime)) => {
                block_on_detached(py, runtime, async move { inner.is_connected().await })
            }
            Err(_) => false,
        }
    }

    /// Check if client has been closed
    ///
    /// Returns true if disconnect() has been called and client is closed.
    /// Once closed, the client cannot be reused - create a new instance.
    #[pyo3(signature = ())]
    pub fn is_closed(&self, py: Python<'_>) -> bool {
        // If state is None (never connected), not closed
        let inner = match self.state.lock() {
            Ok(g) => match g.as_ref() {
                Some(s) => Arc::clone(&s.inner),
                None => return false,
            },
            Err(_) => return false,
        };

        let runtime = match self.runtime.lock() {
            Ok(g) => g.clone(),
            Err(_) => return false,
        };

        match runtime {
            Some(runtime) => block_on_detached(py, runtime, async move { inner.is_closed().await }),
            // If no runtime, use sync version
            None => inner.is_closed_sync(),
        }
    }

    /// Subscribe to a channel for one or more symbols.
    ///
    /// Two call shapes are supported (legacy fugle-marketdata parity):
    ///
    /// **Dict shape** (matches the legacy SDK README):
    /// ```python
    /// ws.stock.subscribe({"channel": "trades", "symbol": "2330"})
    /// ws.stock.subscribe({"channel": "trades", "symbols": ["2330", "2317"]})
    /// ws.stock.subscribe({"channel": "candles", "symbol": "2330", "oddLot": True})
    /// ```
    ///
    /// **Positional shape**:
    /// ```python
    /// ws.stock.subscribe("trades", "2330")
    /// ws.stock.subscribe("trades", symbols=["2330", "2317"])
    /// ws.stock.subscribe("candles", "2330", odd_lot=True)
    /// ```
    ///
    /// When a dict is supplied, the kwargs `symbol`/`symbols`/`odd_lot` are
    /// ignored — the dict is the single source of truth, matching the legacy
    /// SDK's `def subscribe(self, params)` behavior.
    #[pyo3(signature = (channel, symbol=None, *, symbols=None, odd_lot=false))]
    pub fn subscribe(
        &self,
        py: Python<'_>,
        channel: &Bound<'_, PyAny>,
        symbol: Option<&str>,
        symbols: Option<Vec<String>>,
        odd_lot: bool,
    ) -> PyResult<()> {
        // Resolve dual-shape input into (channel_str, symbols, odd_lot)
        let (channel_str, target_symbols, effective_odd_lot) =
            if let Ok(d) = channel.cast::<PyDict>() {
                extract_stock_subscribe_dict(d)?
            } else if let Ok(s) = channel.extract::<String>() {
                let syms = resolve_symbol_args(symbol, symbols)?;
                (s, syms, odd_lot)
            } else {
                return Err(pyo3::exceptions::PyTypeError::new_err(
                    "subscribe() first argument must be a dict or channel string",
                ));
            };

        let (inner, runtime) = live_handles(&self.state, &self.runtime)?;

        let ch = match channel_str.to_lowercase().as_str() {
            "trades" => marketdata_core::Channel::Trades,
            "candles" => marketdata_core::Channel::Candles,
            "books" => marketdata_core::Channel::Books,
            "aggregates" => marketdata_core::Channel::Aggregates,
            "indices" => marketdata_core::Channel::Indices,
            _ => {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "Invalid channel: '{}'. Valid channels: trades, candles, books, aggregates, indices",
                    channel_str
                )));
            }
        };

        let sub = marketdata_core::StockSubscription::new(ch, target_symbols)
            .with_odd_lot(effective_odd_lot);
        let result = block_on_detached(py, runtime, async move { inner.subscribe(sub).await });
        result.map_err(errors::to_py_err)?;

        Ok(())
    }

    /// Unsubscribe from a channel.
    ///
    /// Two call shapes are supported (legacy fugle-marketdata parity):
    ///
    /// **Dict shape**:
    /// ```python
    /// ws.stock.unsubscribe({"id": "abc123"})
    /// ws.stock.unsubscribe({"ids": ["abc123", "def456"]})
    /// ```
    ///
    /// **Positional shape**:
    /// ```python
    /// ws.stock.unsubscribe("abc123")
    /// ws.stock.unsubscribe(ids=["abc123", "def456"])
    /// ```
    #[pyo3(signature = (subscription_id=None, *, ids=None))]
    pub fn unsubscribe(
        &self,
        py: Python<'_>,
        subscription_id: Option<&Bound<'_, PyAny>>,
        ids: Option<Vec<String>>,
    ) -> PyResult<()> {
        let target_ids: Vec<String> = if let Some(arg) = subscription_id {
            if let Ok(d) = arg.cast::<PyDict>() {
                extract_unsubscribe_dict(d)?
            } else if let Ok(s) = arg.extract::<String>() {
                resolve_unsubscribe_args(Some(s.as_str()), ids)?
            } else {
                return Err(pyo3::exceptions::PyTypeError::new_err(
                    "unsubscribe() first argument must be a dict or subscription id string",
                ));
            }
        } else {
            resolve_unsubscribe_args(None, ids)?
        };

        let (inner, runtime) = live_handles(&self.state, &self.runtime)?;

        let result =
            block_on_detached(py, runtime, async move { inner.unsubscribe(target_ids).await });
        result.map_err(errors::to_py_err)?;

        Ok(())
    }

    /// Messages dropped because they arrived while `message_buffer` unread
    /// messages were already held (`message_overflow="drop_newest"`).
    ///
    /// Counted from the start of the current connection (every `connect()`
    /// or reconnect restarts it); after `disconnect()` it still reads the
    /// last connection's count. 0 before the first `connect()`.
    #[pyo3(signature = ())]
    pub fn messages_dropped_total(&self) -> u64 {
        self.messages_dropped
            .lock()
            .ok()
            .and_then(|slot| slot.as_ref().map(|handle| handle.total()))
            .unwrap_or(0)
    }

    /// Get message iterator for consuming streaming data
    ///
    /// Returns:
    ///     MessageIterator for iterating over messages
    ///
    /// Example:
    ///     ```python
    ///     for msg in ws.stock.messages():
    ///         print(msg)
    ///     ```
    ///
    /// Iteration yields messages only: it waits while none arrive and stops
    /// once the connection is gone. `timeout_ms` is deprecated and ignored.
    #[pyo3(signature = (timeout_ms=None))]
    pub fn messages(
        &self,
        py: Python<'_>,
        timeout_ms: Option<u64>,
    ) -> PyResult<crate::iterator::MessageIterator> {
        warn_timeout_ms_deprecated(py, timeout_ms)?;
        let state_guard = self.state.lock().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Lock error: {}", e))
        })?;

        let state = state_guard.as_ref().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err("Not connected. Call connect() first.")
        })?;

        Ok(crate::iterator::MessageIterator::new(Arc::clone(&state.handoff)))
    }

    /// Get the locally cached list of active subscription keys.
    ///
    /// Note: this is the *local* cache maintained by core's SubscriptionManager.
    /// To request the authoritative list from the server (matches the old
    /// fugle-marketdata SDK), call `subscriptions()` instead — the server's
    /// response will arrive via the registered `message` callback.
    #[pyo3(signature = ())]
    pub fn local_subscriptions(&self) -> Vec<String> {
        let state_guard = match self.state.lock() {
            Ok(g) => g,
            Err(_) => return vec![],
        };

        state_guard
            .as_ref()
            .map(|s| s.inner.subscription_keys())
            .unwrap_or_default()
    }

    /// Ask the server for its current subscription list.
    ///
    /// Sends `{"event": "subscriptions"}` to the server. The server replies
    /// asynchronously and the response is delivered via the `message` callback,
    /// matching the old `fugle-marketdata` SDK's `subscriptions()` semantics.
    ///
    /// Raises:
    ///     RuntimeError: If not connected
    #[pyo3(signature = ())]
    pub fn subscriptions(&self, py: Python<'_>) -> PyResult<()> {
        let (inner, runtime) = live_handles(&self.state, &self.runtime)?;

        let request = marketdata_core::WebSocketRequest::subscriptions();
        block_on_detached(py, runtime, async move { inner.send(request).await })
            .map_err(errors::to_py_err)
    }

    /// Send a `ping` frame to the server (matches the old fugle-marketdata SDK).
    ///
    /// Args:
    ///     state: Optional state string echoed back in the server's `pong` reply
    ///
    /// Raises:
    ///     RuntimeError: If not connected
    #[pyo3(signature = (state=None))]
    pub fn ping(&self, py: Python<'_>, state: Option<String>) -> PyResult<()> {
        let (inner, runtime) = live_handles(&self.state, &self.runtime)?;

        let request = marketdata_core::WebSocketRequest::ping(state);
        block_on_detached(py, runtime, async move { inner.send(request).await })
            .map_err(errors::to_py_err)
    }

    /// Connect to WebSocket server (async version)
    ///
    /// Returns an awaitable that completes when connection is established.
    /// Releases GIL during connection, enabling concurrent Python tasks.
    ///
    /// Raises:
    ///     MarketDataError: If connection fails
    ///
    /// Example:
    ///     ```python
    ///     await ws.stock.connect_async()
    ///     ```
    pub fn connect_async<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        // Ensure runtime exists
        self.ensure_runtime().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(e)
        })?;

        let api_key = self.api_key.clone();
        let stock_version = self.stock_version;
        let futopt_version = self.futopt_version;
        let base_url = self.base_url.clone();
        let reconnect_config = self.reconnect_config.to_core();
        let health_check_config = self.health_check_config.to_core();
        let callbacks = Arc::clone(&self.callbacks);
        let state_arc = Arc::clone(&self.state);
        let reader_thread_handle = Arc::clone(&self.reader_thread_handle);
        let message_queue = self.message_queue;
        let messages_dropped = Arc::clone(&self.messages_dropped);
        let test_panic = test_panic_site();

        future_into_py(py, async move {
            // Create WebSocket client with full config
            let config = build_stream_config(
                &api_key,
                base_url.as_deref(),
                WsProduct::Stock,
                stock_version,
                futopt_version,
            )
            .map_err(|e| pyo3::exceptions::PyTypeError::new_err(format!("{e}")))?;
            let mut config = config;
            message_queue.apply(&mut config);
            let capacity = handoff_capacity(&config);
            let ws_client = marketdata_core::aio::WebSocketClient::with_full_config(
                config,
                reconnect_config,
                health_check_config,
            );
            if let Ok(mut slot) = messages_dropped.lock() {
                *slot = Some(ws_client.messages_dropped_handle());
            }

            let handoff = Arc::new(Handoff::new(capacity));
            let stop = Arc::new(AtomicBool::new(false));
            let reader_thread = spawn_stream_reader(
                "stock_ws_stream",
                ws_client.stream_receiver(),
                Arc::clone(&callbacks),
                Arc::clone(&handoff),
                Arc::clone(&stop),
                test_panic,
            )?;

            // Connect without holding GIL
            if let Err(e) = ws_client.connect().await {
                // See `connect`: callbacks for the failure fire before we raise.
                drop(ws_client);
                join_reader_thread_async(Some(reader_thread)).await;
                return Err(crate::errors::to_py_err(e));
            }

            // Store state
            let state = WebSocketState {
                inner: Arc::new(ws_client),
                handoff,
                stop,
            };

            let mut state_guard = state_arc.lock()
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Lock error: {}", e)))?;
            *state_guard = Some(state);
            drop(state_guard);
            if let Ok(mut guard) = reader_thread_handle.lock() {
                *guard = Some(reader_thread);
            }

            Ok(())
        })
    }

    /// Disconnect from WebSocket server (async version)
    ///
    /// Returns an awaitable that completes when disconnection finishes.
    ///
    /// Example:
    ///     ```python
    ///     await ws.stock.disconnect_async()
    ///     ```
    pub fn disconnect_async<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let state_arc = Arc::clone(&self.state);
        let reader_thread_handle = Arc::clone(&self.reader_thread_handle);

        future_into_py(py, async move {
            let state_opt = {
                let mut state_guard = state_arc.lock()
                    .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Lock error: {}", e)))?;
                state_guard.take()
            };

            if let Some(state) = state_opt {
                // See `disconnect`.
                state.stop.store(true, Ordering::SeqCst);
                let _ = state.inner.disconnect().await;
                // Note: do NOT manually invoke_disconnect — core's disconnect()
                // emits ConnectionEvent::Disconnected on its stream and the
                // stream reader fires the user callback.
                //
                // Nothing else to release: this path runs on
                // pyo3-async-runtimes' runtime, not `self.runtime`, and
                // core's disconnect() has already stopped the dispatch and
                // writer tasks, so dropping the client closes its stream
                // (#54).
                drop(state);
            }

            join_reader_thread_async(take_thread(&reader_thread_handle)).await;

            Ok(())
        })
    }

    /// Subscribe to a channel (async version)
    ///
    /// Args:
    ///     channel: Channel name (trades, candles, books, aggregates, indices)
    ///     symbol: Stock symbol (e.g., "2330")
    ///     odd_lot: Whether to subscribe to odd lot data (default: False)
    ///
    /// Returns:
    ///     Awaitable that completes when subscription is confirmed
    ///
    /// Example:
    ///     ```python
    ///     await ws.stock.subscribe_async("trades", "2330")
    ///     await ws.stock.subscribe_async({"channel": "trades", "symbol": "2330"})
    ///     ```
    #[pyo3(signature = (channel, symbol=None, *, symbols=None, odd_lot=false))]
    pub fn subscribe_async<'py>(
        &self,
        py: Python<'py>,
        channel: &Bound<'py, PyAny>,
        symbol: Option<&str>,
        symbols: Option<Vec<String>>,
        odd_lot: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        // Resolve dual-shape input synchronously (we still hold the GIL here)
        // so the async block only deals with owned, Send-safe data.
        let (channel_str, target_symbols, effective_odd_lot) =
            if let Ok(d) = channel.cast::<PyDict>() {
                extract_stock_subscribe_dict(d)?
            } else if let Ok(s) = channel.extract::<String>() {
                let syms = resolve_symbol_args(symbol, symbols)?;
                (s, syms, odd_lot)
            } else {
                return Err(pyo3::exceptions::PyTypeError::new_err(
                    "subscribe_async() first argument must be a dict or channel string",
                ));
            };

        let state_arc = Arc::clone(&self.state);

        future_into_py(py, async move {
            // Parse channel
            let ch = match channel_str.to_lowercase().as_str() {
                "trades" => marketdata_core::Channel::Trades,
                "candles" => marketdata_core::Channel::Candles,
                "books" => marketdata_core::Channel::Books,
                "aggregates" => marketdata_core::Channel::Aggregates,
                "indices" => marketdata_core::Channel::Indices,
                _ => {
                    return Err(pyo3::exceptions::PyValueError::new_err(format!(
                        "Invalid channel: '{}'. Valid channels: trades, candles, books, aggregates, indices",
                        channel_str
                    )));
                }
            };

            // Clone the Arc<WebSocketClient> out of mutex to avoid holding guard across await
            let ws_client = {
                let state_guard = state_arc.lock()
                    .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Lock error: {}", e)))?;
                let state = state_guard.as_ref()
                    .ok_or_else(|| pyo3::exceptions::PyRuntimeError::new_err("Not connected. Call connect() first."))?;
                Arc::clone(&state.inner)
            };

            let sub = marketdata_core::StockSubscription::new(ch, target_symbols)
                .with_odd_lot(effective_odd_lot);
            ws_client.subscribe(sub).await
                .map_err(crate::errors::to_py_err)?;

            Ok(())
        })
    }

    /// Async context manager support: enter
    ///
    /// Example:
    ///     ```python
    ///     async with ws.stock as client:
    ///         await client.subscribe("trades", "2330")
    ///     ```
    fn __aenter__<'py>(slf: PyRef<'py, Self>, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        slf.connect_async(py)
    }

    /// Async context manager support: exit
    ///
    /// Automatically disconnects when exiting async with block.
    fn __aexit__<'py>(
        &self,
        py: Python<'py>,
        _exc_type: &Bound<'py, PyAny>,
        _exc_value: &Bound<'py, PyAny>,
        _traceback: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        self.disconnect_async(py)
    }
}

/// FutOpt (futures and options) WebSocket client
///
/// Access via `ws.futopt`
///
/// Note: `unsendable` is required because the underlying WebSocket state contains
/// `std::sync::mpsc::Receiver` which is not `Sync`. This means the client
/// should only be used from the thread that created it.
#[pyclass(unsendable)]
pub struct FutOptWebSocketClient {
    api_key: String,
    base_url: Option<String>,
    stock_version: marketdata_core::websocket::StockVersion,
    futopt_version: marketdata_core::websocket::FutOptVersion,
    reconnect_config: ReconnectConfig,
    health_check_config: HealthCheckConfig,
    tls: marketdata_core::TlsConfig,
    callbacks: Arc<CallbackRegistry>,
    state: Arc<Mutex<Option<WebSocketState>>>,
    runtime: Arc<Mutex<Option<SharedRuntime>>>,
    reader_thread_handle: Arc<Mutex<Option<std::thread::JoinHandle<()>>>>,
    message_queue: MessageQueueSettings,
    /// Dropped-message count of the current or last connection; outlives the
    /// core client, which `disconnect()` drops.
    messages_dropped: Arc<Mutex<Option<marketdata_core::MessagesDroppedHandle>>>,
}

impl FutOptWebSocketClient {
    fn new(
        api_key: String,
        base_url: Option<String>,
        stock_version: marketdata_core::websocket::StockVersion,
        futopt_version: marketdata_core::websocket::FutOptVersion,
        reconnect_config: ReconnectConfig,
        health_check_config: HealthCheckConfig,
        tls: marketdata_core::TlsConfig,
        message_queue: MessageQueueSettings,
    ) -> Self {
        Self {
            api_key,
            base_url,
            stock_version,
            futopt_version,
            reconnect_config,
            health_check_config,
            tls,
            callbacks: Arc::new(CallbackRegistry::new()),
            state: Arc::new(Mutex::new(None)),
            runtime: Arc::new(Mutex::new(None)),
            reader_thread_handle: Arc::new(Mutex::new(None)),
            message_queue,
            messages_dropped: Arc::new(Mutex::new(None)),
        }
    }

    fn build_config(&self) -> marketdata_core::ConnectionConfig {
        // See the stock sibling: validation already happened at construction.
        let mut config = build_stream_config(
            &self.api_key,
            self.base_url.as_deref(),
            WsProduct::FutOpt,
            self.stock_version,
            self.futopt_version,
        )
        .unwrap_or_else(|_| {
            marketdata_core::ConnectionConfig::fugle_futopt(
                marketdata_core::AuthRequest::with_api_key(&self.api_key),
            )
        });
        config.tls = self.tls.clone();
        self.message_queue.apply(&mut config);
        config
    }

    /// Get or create the tokio runtime
    fn ensure_runtime(&self) -> Result<(), String> {
        let mut runtime_guard = self.runtime.lock().map_err(|e| e.to_string())?;
        if runtime_guard.is_none() {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .map_err(|e| format!("Failed to create tokio runtime: {}", e))?;
            *runtime_guard = Some(Arc::new(rt));
        }
        Ok(())
    }
}

#[pymethods]
impl FutOptWebSocketClient {
    /// Register a callback for an event type
    ///
    /// Supported events:
    ///   - "message" / "data": Called with message dict when data received
    ///   - "connect" / "connected": Called when the WebSocket opens, before authentication
    ///   - "authenticated": Called with the server's data (dict, or None) when it accepts credentials
    ///   - "unauthenticated": Called with the server's data (dict, or None) when it refuses credentials
    ///   - "disconnect" / "disconnected" / "close": Called when connection closed
    ///   - "reconnect" / "reconnecting": Called when reconnecting
    ///   - "error": Called with a single `err` argument (WebSocketError instance) when error occurs
    ///
    /// Args:
    ///     event: Event type string
    ///     callback: Python callable to invoke
    #[pyo3(signature = (event, callback))]
    pub fn on(&self, event: &str, callback: &Bound<'_, PyAny>) -> PyResult<()> {
        self.callbacks.register(event, callback)
    }

    /// Remove all callbacks for an event type
    #[pyo3(signature = (event))]
    pub fn off(&self, event: &str) -> PyResult<()> {
        self.callbacks.unregister(event)
    }

    /// Connect to WebSocket server
    ///
    /// Raises:
    ///     MarketDataError: If connection fails
    #[pyo3(signature = ())]
    pub fn connect(&self, py: Python<'_>) -> PyResult<()> {
        let test_panic = test_panic_site();
        // Ensure runtime exists
        self.ensure_runtime().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(e)
        })?;

        // Create WebSocket client for FutOpt endpoint with full config
        let config = self.build_config();
        let capacity = handoff_capacity(&config);
        let ws_client = marketdata_core::aio::WebSocketClient::with_full_config(
            config,
            self.reconnect_config.to_core(),
            self.health_check_config.to_core(),
        );
        *self.messages_dropped.lock().map_err(lock_err)? = Some(ws_client.messages_dropped_handle());

        let handoff = Arc::new(Handoff::new(capacity));
        let stop = Arc::new(AtomicBool::new(false));
        let reader_thread = spawn_stream_reader(
            "futopt_ws_stream",
            ws_client.stream_receiver(),
            Arc::clone(&self.callbacks),
            Arc::clone(&handoff),
            Arc::clone(&stop),
            test_panic,
        )?;

        let runtime = self.runtime.lock().map_err(lock_err)?.clone().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err("Runtime not initialized")
        })?;

        // Connect with the GIL released: the handshake and auth ack may come
        // from a server that needs this interpreter to run (#39).
        let (ws_client, result) = py.detach(move || {
            let result = runtime.block_on(ws_client.connect());
            (ws_client, result)
        });

        if let Err(e) = result {
            // Dropping the client closes the stream; the reader first delivers
            // `unauthenticated` / `error`, so they fire before we raise.
            drop(ws_client);
            join_reader_thread(py, reader_thread);
            return Err(errors::to_py_err(e));
        }

        *self.state.lock().map_err(lock_err)? = Some(WebSocketState {
            inner: Arc::new(ws_client),
            handoff,
            stop,
        });
        *self.reader_thread_handle.lock().map_err(lock_err)? = Some(reader_thread);
        Ok(())
    }

    /// Disconnect from WebSocket server
    #[pyo3(signature = ())]
    pub fn disconnect(&self, py: Python<'_>) -> PyResult<()> {
        let state = self.state.lock().map_err(lock_err)?.take();

        if let Some(state) = state {
            // Messages before `Disconnected` still reach the callbacks, but
            // the reader no longer waits for an iterator to make room.
            state.stop.store(true, Ordering::SeqCst);
            // Take ownership of the runtime — see StockWebSocketClient::disconnect
            // for the rationale (forces all spawned tasks to drop their
            // Arc<WebSocketClient> clones so the stream can close).
            let runtime = self.runtime.lock().map_err(lock_err)?.take();

            if let Some(rt) = runtime {
                py.detach(move || {
                    rt.block_on(async {
                        let _ = state.inner.disconnect().await;
                    });
                    drop(rt);
                    drop(state);
                });
            }

            // Note: do NOT manually invoke_disconnect here. Core's
            // disconnect() emits ConnectionEvent::Disconnected and the stream
            // reader fires the callback. Manual invocation would
            // double-fire.
        }

        // See StockWebSocketClient::disconnect (#54).
        join_stored_reader_thread(py, &self.reader_thread_handle);

        Ok(())
    }

    /// Check if currently connected
    #[pyo3(signature = ())]
    pub fn is_connected(&self, py: Python<'_>) -> bool {
        match live_handles(&self.state, &self.runtime) {
            Ok((inner, runtime)) => {
                block_on_detached(py, runtime, async move { inner.is_connected().await })
            }
            Err(_) => false,
        }
    }

    /// Check if client has been closed
    ///
    /// Returns true if disconnect() has been called and client is closed.
    /// Once closed, the client cannot be reused - create a new instance.
    #[pyo3(signature = ())]
    pub fn is_closed(&self, py: Python<'_>) -> bool {
        // If state is None (never connected), not closed
        let inner = match self.state.lock() {
            Ok(g) => match g.as_ref() {
                Some(s) => Arc::clone(&s.inner),
                None => return false,
            },
            Err(_) => return false,
        };

        let runtime = match self.runtime.lock() {
            Ok(g) => g.clone(),
            Err(_) => return false,
        };

        match runtime {
            Some(runtime) => block_on_detached(py, runtime, async move { inner.is_closed().await }),
            // If no runtime, use sync version
            None => inner.is_closed_sync(),
        }
    }

    /// Subscribe to a channel for one or more FutOpt symbols.
    ///
    /// Two call shapes are supported (legacy fugle-marketdata parity):
    ///
    /// **Dict shape**:
    /// ```python
    /// ws.futopt.subscribe({"channel": "trades", "symbol": "TXFC4"})
    /// ws.futopt.subscribe({"channel": "books", "symbol": "MXFB4", "afterHours": True})
    /// ```
    ///
    /// **Positional shape**:
    /// ```python
    /// ws.futopt.subscribe("trades", "TXFC4")
    /// ws.futopt.subscribe("books", "MXFB4", after_hours=True)
    /// ```
    #[pyo3(signature = (channel, symbol=None, *, symbols=None, after_hours=false))]
    pub fn subscribe(
        &self,
        py: Python<'_>,
        channel: &Bound<'_, PyAny>,
        symbol: Option<&str>,
        symbols: Option<Vec<String>>,
        after_hours: bool,
    ) -> PyResult<()> {
        let (channel_str, target_symbols, effective_after_hours) =
            if let Ok(d) = channel.cast::<PyDict>() {
                extract_futopt_subscribe_dict(d)?
            } else if let Ok(s) = channel.extract::<String>() {
                let syms = resolve_symbol_args(symbol, symbols)?;
                (s, syms, after_hours)
            } else {
                return Err(pyo3::exceptions::PyTypeError::new_err(
                    "subscribe() first argument must be a dict or channel string",
                ));
            };

        let (inner, runtime) = live_handles(&self.state, &self.runtime)?;

        // Parse channel (FutOpt doesn't have indices channel)
        let ch = match channel_str.to_lowercase().as_str() {
            "trades" => marketdata_core::FutOptChannel::Trades,
            "candles" => marketdata_core::FutOptChannel::Candles,
            "books" => marketdata_core::FutOptChannel::Books,
            "aggregates" => marketdata_core::FutOptChannel::Aggregates,
            _ => {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "Invalid channel: '{}'. Valid channels: trades, candles, books, aggregates",
                    channel_str
                )));
            }
        };

        let sub = marketdata_core::FutOptSubscription::new(ch, target_symbols)
            .with_after_hours(effective_after_hours);
        let result =
            block_on_detached(py, runtime, async move { inner.subscribe_futopt(sub).await });
        result.map_err(errors::to_py_err)?;

        Ok(())
    }

    /// Unsubscribe from a channel.
    ///
    /// Accepts dict shape (`{"id": "..."}` / `{"ids": [...]}`) or
    /// positional/kwargs shape (`subscription_id` / `ids=`).
    #[pyo3(signature = (subscription_id=None, *, ids=None))]
    pub fn unsubscribe(
        &self,
        py: Python<'_>,
        subscription_id: Option<&Bound<'_, PyAny>>,
        ids: Option<Vec<String>>,
    ) -> PyResult<()> {
        let target_ids: Vec<String> = if let Some(arg) = subscription_id {
            if let Ok(d) = arg.cast::<PyDict>() {
                extract_unsubscribe_dict(d)?
            } else if let Ok(s) = arg.extract::<String>() {
                resolve_unsubscribe_args(Some(s.as_str()), ids)?
            } else {
                return Err(pyo3::exceptions::PyTypeError::new_err(
                    "unsubscribe() first argument must be a dict or subscription id string",
                ));
            }
        } else {
            resolve_unsubscribe_args(None, ids)?
        };

        let (inner, runtime) = live_handles(&self.state, &self.runtime)?;

        let result =
            block_on_detached(py, runtime, async move { inner.unsubscribe(target_ids).await });
        result.map_err(errors::to_py_err)?;

        Ok(())
    }

    /// Messages dropped because they arrived while `message_buffer` unread
    /// messages were already held (`message_overflow="drop_newest"`).
    ///
    /// Counted from the start of the current connection (every `connect()`
    /// or reconnect restarts it); after `disconnect()` it still reads the
    /// last connection's count. 0 before the first `connect()`.
    #[pyo3(signature = ())]
    pub fn messages_dropped_total(&self) -> u64 {
        self.messages_dropped
            .lock()
            .ok()
            .and_then(|slot| slot.as_ref().map(|handle| handle.total()))
            .unwrap_or(0)
    }

    /// Get message iterator for consuming streaming data
    ///
    /// Returns:
    ///     MessageIterator for iterating over messages
    ///
    /// Example:
    ///     ```python
    ///     for msg in ws.futopt.messages():
    ///         print(msg)
    ///     ```
    ///
    /// Iteration yields messages only: it waits while none arrive and stops
    /// once the connection is gone. `timeout_ms` is deprecated and ignored.
    #[pyo3(signature = (timeout_ms=None))]
    pub fn messages(
        &self,
        py: Python<'_>,
        timeout_ms: Option<u64>,
    ) -> PyResult<crate::iterator::MessageIterator> {
        warn_timeout_ms_deprecated(py, timeout_ms)?;
        let state_guard = self.state.lock().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Lock error: {}", e))
        })?;

        let state = state_guard.as_ref().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err("Not connected. Call connect() first.")
        })?;

        Ok(crate::iterator::MessageIterator::new(Arc::clone(&state.handoff)))
    }

    /// Get the locally cached list of active subscription keys.
    ///
    /// Note: this is the *local* cache maintained by core's SubscriptionManager.
    /// To request the authoritative list from the server (matches the old
    /// fugle-marketdata SDK), call `subscriptions()` instead — the server's
    /// response will arrive via the registered `message` callback.
    #[pyo3(signature = ())]
    pub fn local_subscriptions(&self) -> Vec<String> {
        let state_guard = match self.state.lock() {
            Ok(g) => g,
            Err(_) => return vec![],
        };

        state_guard
            .as_ref()
            .map(|s| s.inner.subscription_keys())
            .unwrap_or_default()
    }

    /// Ask the server for its current subscription list.
    ///
    /// Sends `{"event": "subscriptions"}` to the server. The server replies
    /// asynchronously and the response is delivered via the `message` callback,
    /// matching the old `fugle-marketdata` SDK's `subscriptions()` semantics.
    ///
    /// Raises:
    ///     RuntimeError: If not connected
    #[pyo3(signature = ())]
    pub fn subscriptions(&self, py: Python<'_>) -> PyResult<()> {
        let (inner, runtime) = live_handles(&self.state, &self.runtime)?;

        let request = marketdata_core::WebSocketRequest::subscriptions();
        block_on_detached(py, runtime, async move { inner.send(request).await })
            .map_err(errors::to_py_err)
    }

    /// Send a `ping` frame to the server (matches the old fugle-marketdata SDK).
    ///
    /// Args:
    ///     state: Optional state string echoed back in the server's `pong` reply
    ///
    /// Raises:
    ///     RuntimeError: If not connected
    #[pyo3(signature = (state=None))]
    pub fn ping(&self, py: Python<'_>, state: Option<String>) -> PyResult<()> {
        let (inner, runtime) = live_handles(&self.state, &self.runtime)?;

        let request = marketdata_core::WebSocketRequest::ping(state);
        block_on_detached(py, runtime, async move { inner.send(request).await })
            .map_err(errors::to_py_err)
    }
}

/// Convert an inbound WebSocket frame to a Python dict.
///
/// The frame is decoded straight from the wire text, so the dict holds exactly
/// what the server sent. Rebuilding it from the routing fields this SDK parses
/// out (`event` / `channel` / `symbol` / `id` / `data`) would silently drop any
/// field those five do not cover.
pub fn message_to_dict(py: Python<'_>, msg: &marketdata_core::WebSocketMessage) -> PyResult<Py<PyDict>> {
    let value: serde_json::Value = serde_json::from_str(&msg.raw).map_err(|e| {
        pyo3::exceptions::PyValueError::new_err(format!("Malformed frame: {}", e))
    })?;
    crate::types::value_to_dict(py, &value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_versions_match_core() {
        // The Python default must not drift from core's: futopt on v1.1 means
        // trial frames arrive without opting in, and that has to be the same
        // story in every language.
        let (stock, futopt) = parse_ws_versions(None).unwrap();
        assert_eq!(stock, marketdata_core::websocket::StockVersion::V1_0);
        assert_eq!(futopt, marketdata_core::websocket::FutOptVersion::V1_1);
    }

    #[test]
    fn test_build_stream_config_appends_version_per_product() {
        // One base URL, two different version segments — the thing a version
        // baked into base_url could never express.
        let stock = build_stream_config(
            "k",
            Some("wss://staging.fugle.tw/marketdata"),
            WsProduct::Stock,
            Default::default(),
            Default::default(),
        )
        .unwrap();
        assert_eq!(
            stock.url,
            "wss://staging.fugle.tw/marketdata/v1.0/stock/streaming"
        );

        let futopt = build_stream_config(
            "k",
            Some("wss://staging.fugle.tw/marketdata"),
            WsProduct::FutOpt,
            Default::default(),
            Default::default(),
        )
        .unwrap();
        assert_eq!(
            futopt.url,
            "wss://staging.fugle.tw/marketdata/v1.1/futopt/streaming"
        );
    }

    #[test]
    fn test_build_stream_config_rejects_versioned_base_url() {
        // The 0.6-era form. Python callers see this as a TypeError at
        // construction, matching the official SDK.
        let err = build_stream_config(
            "k",
            Some("wss://staging.fugle.tw/marketdata/v1.0"),
            WsProduct::Stock,
            Default::default(),
            Default::default(),
        )
        .expect_err("a versioned base_url must be rejected");
        assert!(err.to_string().contains("must not include a version segment"));
    }

    #[test]
    fn test_build_stream_config_defaults_to_production() {
        let cfg = build_stream_config("k", None, WsProduct::FutOpt, Default::default(), Default::default())
            .unwrap();
        assert_eq!(cfg.url, marketdata_core::urls::FUTOPT_WS);
    }

    #[test]
    fn test_websocket_client_creation_with_api_key() {
        // WebSocketClient::new requires Python bindings, test the internal child client instead
        let client = StockWebSocketClient::new(
            "test-key".to_string(),
            None,
            Default::default(),
            Default::default(),
            ReconnectConfig::default(),
            HealthCheckConfig::default(),
            marketdata_core::TlsConfig::default(),
            MessageQueueSettings::parse(None, None).unwrap(),
        );
        let state = client.state.lock().unwrap();
        assert!(state.is_none());
    }

    #[test]
    fn test_stock_websocket_client_creation() {
        let client = StockWebSocketClient::new(
            "test-key".to_string(),
            None,
            Default::default(),
            Default::default(),
            ReconnectConfig::default(),
            HealthCheckConfig::default(),
            marketdata_core::TlsConfig::default(),
            MessageQueueSettings::parse(None, None).unwrap(),
        );
        let state = client.state.lock().unwrap();
        assert!(state.is_none());
    }

    #[test]
    fn message_queue_settings_parse_names_and_reject_bad_values() {
        let default = MessageQueueSettings::parse(None, None).unwrap();
        assert_eq!(default.overflow, marketdata_core::MessageOverflow::DropNewest);
        assert_eq!(default.buffer, marketdata_core::websocket::DEFAULT_MESSAGE_BUFFER);
        let unbounded = MessageQueueSettings::parse(Some("unbounded"), Some(16)).unwrap();
        assert_eq!(unbounded.overflow, marketdata_core::MessageOverflow::Unbounded);
        assert_eq!(unbounded.buffer, 16);
        assert!(MessageQueueSettings::parse(Some("dropNewest"), None).is_err());
        assert!(MessageQueueSettings::parse(None, Some(0)).is_err());
        assert!(MessageQueueSettings::parse(None, Some(-1)).is_err());
    }

    #[test]
    fn test_futopt_websocket_client_creation() {
        let client = FutOptWebSocketClient::new(
            "test-key".to_string(),
            None,
            Default::default(),
            Default::default(),
            ReconnectConfig::default(),
            HealthCheckConfig::default(),
            marketdata_core::TlsConfig::default(),
            MessageQueueSettings::parse(None, None).unwrap(),
        );
        let state = client.state.lock().unwrap();
        assert!(state.is_none());
    }
}
