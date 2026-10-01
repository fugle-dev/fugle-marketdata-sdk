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
use pyo3::sync::PyOnceLock;
use pyo3::types::PyDict;
use pyo3_async_runtimes::tokio::future_into_py;
use std::panic::AssertUnwindSafe;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use marketdata_core::aio::admission::{admit, Admission, ConnectClaim, ConnectGate, Delivered, StoredConnection};

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

/// Resolve `(symbol, symbols)` kwargs into a non-empty `Vec<String>`. `call`
/// names the method in error messages.
fn resolve_symbol_args(
    call: &str,
    symbol: Option<&str>,
    symbols: Option<Vec<String>>,
) -> PyResult<Vec<String>> {
    match (symbol, symbols) {
        (Some(s), None) => Ok(vec![s.to_string()]),
        (None, Some(list)) if !list.is_empty() => Ok(list),
        (None, Some(_)) => Err(pyo3::exceptions::PyValueError::new_err(format!(
            "{call}(symbols=[]) is empty - provide at least one symbol"
        ))),
        (Some(_), Some(_)) => Err(pyo3::exceptions::PyValueError::new_err(format!(
            "{call}() accepts either `symbol` or `symbols`, not both"
        ))),
        (None, None) => Err(pyo3::exceptions::PyValueError::new_err(format!(
            "{call}() requires either `symbol` or `symbols`"
        ))),
    }
}

/// The session modifier of a product: its dict key, its kwarg name, and the
/// server's spelling, which the dict also takes (#294).
#[derive(Clone, Copy)]
struct Modifier {
    key: &'static str,
    kwarg: &'static str,
    wire: &'static str,
    product: marketdata_core::websocket::StreamProduct,
}

impl Modifier {
    /// Every dict spelling, the documented one first.
    fn spellings(&self) -> Vec<&'static str> {
        let mut all = vec![self.key, self.kwarg];
        if !all.contains(&self.wire) {
            all.push(self.wire);
        }
        all
    }
}

/// Stock: odd lot. The dict takes `oddLot`, `odd_lot` and the server's
/// `intradayOddLot` (the Node and 1.x spelling).
const ODD_LOT: Modifier = Modifier {
    key: "oddLot",
    kwarg: "odd_lot",
    wire: "intradayOddLot",
    product: marketdata_core::websocket::StreamProduct::Stock,
};
/// FutOpt: after hours.
const AFTER_HOURS: Modifier = Modifier {
    key: "afterHours",
    kwarg: "after_hours",
    wire: "afterHours",
    product: marketdata_core::websocket::StreamProduct::FutOpt,
};

/// `d` without its `None` values: a key set to `None` counts as not given,
/// as `odd_lot=None` does and as `null` does in Node (#294).
/// A non-string key is refused first, whatever its value. `call` names the
/// method.
fn without_none<'py>(call: &str, d: &Bound<'py, PyDict>) -> PyResult<Bound<'py, PyDict>> {
    let given = PyDict::new(d.py());
    for (key, value) in d.iter() {
        if key.extract::<String>().is_err() {
            return Err(pyo3::exceptions::PyTypeError::new_err(format!("{call}(dict): keys must be strings")));
        }
        if !value.is_none() {
            given.set_item(key, value)?;
        }
    }
    Ok(given)
}

/// Refuse a dict key outside `accepted` (#294): the keys used to be read and
/// the rest dropped, so a FutOpt `afterHours` on the stock client subscribed
/// board-lot data. `call` names the method.
fn check_dict_keys(
    call: &str,
    d: &Bound<'_, PyDict>,
    accepted: &[&str],
    product: marketdata_core::websocket::StreamProduct,
) -> PyResult<()> {
    for key in d.keys() {
        let key: String = key.extract().map_err(|_| {
            pyo3::exceptions::PyTypeError::new_err(format!("{call}(dict): keys must be strings"))
        })?;
        if !accepted.contains(&key.as_str()) {
            return Err(pyo3::exceptions::PyTypeError::new_err(format!(
                "{call}(dict): {}",
                marketdata_core::websocket::subscribe_keys::unknown_key(product, &key, accepted)
            )));
        }
    }
    Ok(())
}

/// The modifier flag of a subscribe-shaped dict, under any one of its
/// spellings; `None` if absent.
fn dict_modifier(call: &str, d: &Bound<'_, PyDict>, modifier: Modifier) -> PyResult<Option<bool>> {
    let mut found: Option<(&str, bool)> = None;
    for key in modifier.spellings() {
        let Some(value) = d.get_item(key)? else { continue };
        if let Some((earlier, _)) = found {
            return Err(pyo3::exceptions::PyTypeError::new_err(format!(
                "{call}(dict): '{key}' and '{earlier}' are the same option; give one"
            )));
        }
        let flag = value.extract::<bool>().map_err(|_| {
            pyo3::exceptions::PyTypeError::new_err(format!(
                "{call}(dict): '{key}' must be a boolean, got {}",
                value.get_type().name().map(|n| n.to_string()).unwrap_or_else(|_| "?".into())
            ))
        })?;
        found = Some((key, flag));
    }
    Ok(found.map(|(_, flag)| flag))
}

/// Extract `(channel, symbols, modifier)` from a subscribe-shaped dict.
/// `call` names the method in error messages.
fn extract_subscribe_dict(
    call: &str,
    d: &Bound<'_, PyDict>,
    modifier: Modifier,
) -> PyResult<(String, Vec<String>, bool)> {
    let accepted: Vec<&str> = marketdata_core::websocket::subscribe_keys::SUBSCRIBE_KEYS
        .iter()
        .copied()
        .chain(modifier.spellings())
        .collect();
    check_dict_keys(call, d, &accepted, modifier.product)?;

    let channel = d
        .get_item("channel")?
        .ok_or_else(|| {
            pyo3::exceptions::PyValueError::new_err(format!("{call}(dict): missing 'channel'"))
        })?
        .extract::<String>()?;

    let symbols: Vec<String> = match (d.get_item("symbol")?, d.get_item("symbols")?) {
        (Some(s), None) => vec![s.extract::<String>()?],
        (None, Some(list)) => {
            let v: Vec<String> = list.extract()?;
            if v.is_empty() {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "{call}(dict): 'symbols' is empty"
                )));
            }
            v
        }
        (Some(_), Some(_)) => {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "{call}(dict): provide either 'symbol' or 'symbols', not both"
            )));
        }
        (None, None) => {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "{call}(dict): missing 'symbol' or 'symbols'"
            )));
        }
    };

    let flag = dict_modifier(call, d, modifier)?.unwrap_or(false);

    Ok((channel, symbols, flag))
}

/// Extract a list of subscription IDs from an unsubscribe dict.
/// Accepts `{"id": "..."}` or `{"ids": [...]}`.
fn extract_unsubscribe_dict(d: &Bound<'_, PyDict>) -> PyResult<Vec<String>> {
    // Without `channel` the dict names server ids and takes nothing else.
    for key in d.keys() {
        let key: String = key.extract().map_err(|_| {
            pyo3::exceptions::PyTypeError::new_err("unsubscribe(dict): keys must be strings")
        })?;
        if !marketdata_core::websocket::subscribe_keys::ID_KEYS.contains(&key.as_str()) {
            return Err(pyo3::exceptions::PyTypeError::new_err(format!(
                "unsubscribe(dict): {}",
                marketdata_core::websocket::subscribe_keys::unknown_id_key(&key)
            )));
        }
    }
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

/// What `unsubscribe()` names.
enum UnsubscribeTarget {
    /// Server ids from the `subscribed` message.
    Ids(Vec<String>),
    /// The arguments given to `subscribe()`: channel, symbols, modifier.
    Channel(String, Vec<String>, bool),
}

/// The `unsubscribe()` arguments other than the server id forms.
struct ChannelArgs<'a> {
    channel: Option<String>,
    symbol: Option<&'a str>,
    symbols: Option<Vec<String>>,
    modifier: Option<bool>,
}

/// Resolve every `unsubscribe()` shape: a server id (positional), `ids=`,
/// `{"id"}` / `{"ids"}`, a subscribe-shaped dict, or `channel=` with
/// `symbol=` / `symbols=` and the modifier kwarg. Naming a channel together
/// with an id is 1005 `INVALID_PARAMETER`.
fn resolve_unsubscribe_target(
    subscription_id: Option<&Bound<'_, PyAny>>,
    ids: Option<Vec<String>>,
    args: ChannelArgs<'_>,
    modifier: Modifier,
) -> PyResult<UnsubscribeTarget> {
    let channel_with_id = || {
        errors::to_py_err(marketdata_core::MarketDataError::InvalidParameter {
            name: "channel".to_string(),
            reason: "cannot be combined with 'id' or 'ids'".to_string(),
        })
    };

    if let Some(channel) = args.channel {
        if subscription_id.is_some() || ids.is_some() {
            return Err(channel_with_id());
        }
        let symbols = resolve_symbol_args("unsubscribe", args.symbol, args.symbols)?;
        return Ok(UnsubscribeTarget::Channel(
            channel,
            symbols,
            args.modifier.unwrap_or(false),
        ));
    }
    if args.symbol.is_some() || args.symbols.is_some() || args.modifier.is_some() {
        return Err(pyo3::exceptions::PyValueError::new_err(format!(
            "unsubscribe(): `symbol`, `symbols` and `{}` require `channel`",
            modifier.kwarg
        )));
    }

    let Some(arg) = subscription_id else {
        return resolve_unsubscribe_args(None, ids).map(UnsubscribeTarget::Ids);
    };
    if let Ok(d) = arg.cast::<PyDict>() {
        let d = &without_none("unsubscribe", d)?;
        // The dict is the whole call, as in the 1.x `unsubscribe(params)`.
        if ids.is_some() {
            return Err(pyo3::exceptions::PyTypeError::new_err(
                "unsubscribe(): pass either a dict or ids=, not both",
            ));
        }
        if d.contains("channel")? {
            if d.contains("id")? || d.contains("ids")? {
                return Err(channel_with_id());
            }
            let (channel, symbols, flag) = extract_subscribe_dict("unsubscribe", d, modifier)?;
            return Ok(UnsubscribeTarget::Channel(channel, symbols, flag));
        }
        extract_unsubscribe_dict(d).map(UnsubscribeTarget::Ids)
    } else if let Ok(s) = arg.extract::<String>() {
        resolve_unsubscribe_args(Some(s.as_str()), ids).map(UnsubscribeTarget::Ids)
    } else {
        Err(pyo3::exceptions::PyTypeError::new_err(
            "unsubscribe() first argument must be a dict or subscription id string",
        ))
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
///     max_attempts=10,
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
    /// Maximum number of reconnection attempts; 0 means unlimited
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
    ///     max_attempts: Maximum reconnection attempts; 0 means unlimited
    ///         (default: 0, so the client keeps retrying at most max_delay_ms apart)
    ///     initial_delay_ms: Initial delay for exponential backoff (default: 1000ms, min: 100ms)
    ///     max_delay_ms: Maximum delay cap (default: 60000ms = 60s)
    ///
    /// Raises:
    ///     ConfigError: code 1004 if initial_delay_ms < 100 or
    ///         max_delay_ms < initial_delay_ms
    #[new]
    #[pyo3(signature = (
        *,
        enabled=true,
        max_attempts=marketdata_core::DEFAULT_MAX_ATTEMPTS,
        initial_delay_ms=marketdata_core::DEFAULT_INITIAL_DELAY_MS,
        max_delay_ms=marketdata_core::DEFAULT_MAX_DELAY_MS
    ))]
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
        .map_err(errors::to_py_err)?;

        Ok(Self {
            enabled,
            max_attempts,
            initial_delay_ms,
            max_delay_ms,
        })
    }

    /// Create a default reconnect configuration (enabled, unlimited attempts)
    #[staticmethod]
    pub fn default_config() -> Self {
        Self::default()
    }

    /// Create a disabled reconnect configuration
    #[staticmethod]
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Self::default()
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
            max_attempts: marketdata_core::DEFAULT_MAX_ATTEMPTS,
            initial_delay_ms: marketdata_core::DEFAULT_INITIAL_DELAY_MS,
            max_delay_ms: marketdata_core::DEFAULT_MAX_DELAY_MS,
        }
    }
}

/// Health check configuration for WebSocket connections
///
/// Configures liveness detection: the connection is declared dead when no
/// inbound frame arrives within `heartbeat_timeout_ms`. Enabled by default.
/// With `probe_enabled`, a silent connection is asked with a ping first (see
/// `HealthCheckConfig.__init__`).
///
/// # Example (Python)
///
/// ```python
/// from fugle_marketdata import HealthCheckConfig, WebSocketClient
///
/// # Longer timeout
/// health_check = HealthCheckConfig(heartbeat_timeout_ms=60000)
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
    /// connection dead, in milliseconds. Default 35 000. Does not apply
    /// when `probe_enabled` is true.
    #[pyo3(get)]
    pub heartbeat_timeout_ms: u64,
    /// Confirm a silent connection with a ping before declaring it dead
    /// (default: False).
    #[pyo3(get)]
    pub probe_enabled: bool,
    /// Silence before the probe, in milliseconds. Default 30 000.
    #[pyo3(get)]
    pub idle_probe_after_ms: u64,
    /// Wait for any inbound frame after the probe, in milliseconds.
    /// Default 5 000.
    #[pyo3(get)]
    pub probe_timeout_ms: u64,
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
    ///         will cause repeated false disconnects. **Does not apply
    ///         when `probe_enabled` is True.**
    ///     probe_enabled: Confirm a silent connection with a ping before
    ///         declaring it dead (default: False). After
    ///         `idle_probe_after_ms` of silence one ping is sent; if nothing
    ///         arrives within `probe_timeout_ms` the connection is declared
    ///         dead. With the defaults detection stays at 35 s and no ping
    ///         is sent while the server's heartbeat is on time. Does not
    ///         detect a connection whose writes no longer reach the server
    ///         while the server still sends.
    ///     idle_probe_after_ms: Silence before the probe. Default 30 000 ms
    ///         (the server's heartbeat period); floor 5 000 ms. Below 30 000
    ///         a ping is sent in every gap between heartbeats while no data
    ///         flows.
    ///     probe_timeout_ms: Wait for any inbound frame after the probe.
    ///         Default 5 000 ms; floor 1 000 ms.
    ///     ping_interval, max_missed_pongs: the 2.x fields. They do not
    ///         exist in 3.0: a value other than None is ignored, with a
    ///         `FugleHealthCheckWarning` (#304). They are the 2nd and 3rd
    ///         positional parameters, as in 2.x.
    ///
    /// Raises:
    ///     ConfigError: code 1004 if `heartbeat_timeout_ms` < 5 000,
    ///         `idle_probe_after_ms` < 5 000 or `probe_timeout_ms` < 1 000.
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
    ///
    ///     # Confirm with a ping before disconnecting; know within 10 s
    ///     config = HealthCheckConfig(probe_enabled=True,
    ///                                idle_probe_after_ms=5000,
    ///                                probe_timeout_ms=5000)
    ///     ```
    #[new]
    #[pyo3(signature = (
        enabled=true,
        ping_interval=None,
        max_missed_pongs=None,
        *,
        heartbeat_timeout_ms=marketdata_core::DEFAULT_HEARTBEAT_TIMEOUT_MS,
        probe_enabled=false,
        idle_probe_after_ms=marketdata_core::DEFAULT_IDLE_PROBE_AFTER_MS,
        probe_timeout_ms=marketdata_core::DEFAULT_PROBE_TIMEOUT_MS,
    ))]
    pub fn new(
        py: Python<'_>,
        enabled: bool,
        ping_interval: Option<&Bound<'_, PyAny>>,
        max_missed_pongs: Option<&Bound<'_, PyAny>>,
        heartbeat_timeout_ms: u64,
        probe_enabled: bool,
        idle_probe_after_ms: u64,
        probe_timeout_ms: u64,
    ) -> PyResult<Self> {
        let config = Self {
            enabled,
            heartbeat_timeout_ms,
            probe_enabled,
            idle_probe_after_ms,
            probe_timeout_ms,
        };
        // Validate via core even when disabled, for early feedback on bad input.
        config
            .try_to_core()
            .map_err(errors::to_py_err)?;
        let legacy: Vec<&str> = [("ping_interval", ping_interval), ("max_missed_pongs", max_missed_pongs)]
            .into_iter()
            .filter(|(_, value)| value.is_some_and(|value| !value.is_none()))
            .map(|(name, _)| name)
            .collect();
        errors::warn_legacy_health_check_fields(py, &legacy)?;
        Ok(config)
    }
}

impl HealthCheckConfig {
    /// Convert to core HealthCheckConfig
    ///
    /// This should not fail since validation already happened in __new__
    pub fn to_core(&self) -> marketdata_core::HealthCheckConfig {
        self.try_to_core().expect("Config already validated in constructor")
    }

    fn try_to_core(&self) -> Result<marketdata_core::HealthCheckConfig, marketdata_core::MarketDataError> {
        marketdata_core::HealthCheckConfig::from_parts(
            self.enabled,
            Some(Duration::from_millis(self.heartbeat_timeout_ms)),
            self.probe_enabled,
            Some(Duration::from_millis(self.idle_probe_after_ms)),
            Some(Duration::from_millis(self.probe_timeout_ms)),
        )
    }
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            heartbeat_timeout_ms: marketdata_core::DEFAULT_HEARTBEAT_TIMEOUT_MS,
            probe_enabled: false,
            idle_probe_after_ms: marketdata_core::DEFAULT_IDLE_PROBE_AFTER_MS,
            probe_timeout_ms: marketdata_core::DEFAULT_PROBE_TIMEOUT_MS,
        }
    }
}

/// The last disconnect of a stream client: who closed the connection and
/// whether a reconnect follows (#293). Read it from `last_disconnect`.
///
/// Attributes:
///     code: WebSocket close code, or None when the connection ended without
///         one (transport error, EOF, heartbeat timeout, or a server Close
///         frame without a code)
///     reason: Close reason (may be empty)
///     intent: "client" (your disconnect()), "server" (the server's Close
///         frame, any code) or "network" (transport error, EOF without a
///         Close frame, heartbeat timeout)
///     will_reconnect: True if a `reconnect` event follows (unless
///         disconnect() is called first); False if this connection is over
#[pyclass(frozen, eq, hash, get_all, skip_from_py_object)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DisconnectInfo {
    code: Option<u16>,
    reason: String,
    intent: &'static str,
    will_reconnect: bool,
}

#[pymethods]
impl DisconnectInfo {
    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        let code = self.code.map_or_else(|| "None".to_string(), |code| code.to_string());
        let reason = pyo3::types::PyString::new(py, &self.reason).repr()?;
        let will_reconnect = if self.will_reconnect { "True" } else { "False" };
        Ok(format!(
            "DisconnectInfo(code={code}, reason={reason}, intent='{}', will_reconnect={will_reconnect})",
            self.intent
        ))
    }
}

/// The last `DisconnectInfo`, written by the stream reader before the
/// `disconnect` callbacks run, read by `last_disconnect` from any thread.
type LastDisconnect = Arc<Mutex<Option<DisconnectInfo>>>;

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
    auth: marketdata_core::AuthRequest,
    base_url: Option<String>,
    stock_version: marketdata_core::websocket::StockVersion,
    futopt_version: marketdata_core::websocket::FutOptVersion,
    reconnect_config: ReconnectConfig,
    health_check_config: HealthCheckConfig,
    tls: marketdata_core::TlsConfig,
    message_queue: MessageQueueSettings,
    /// `auth_timeout_ms`, validated in the constructor (#199).
    auth_timeout: Duration,
    /// `ws.stock` / `ws.futopt`, built on first read and returned on every
    /// read after, as 2.x's factory did (#306): `ws.stock.on(...)` then
    /// `ws.stock.connect()` must reach the same client.
    stock: PyOnceLock<Py<StockWebSocketClient>>,
    futopt: PyOnceLock<Py<FutOptWebSocketClient>>,
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
    ///   - auth_timeout_ms: how long the auth handshake may take once the
    ///     WebSocket is open, in milliseconds (default 10000). Applies to
    ///     the first connect and to every reconnect; elapsing it fails the
    ///     attempt with TimeoutError (code 3001). Must be > 0.
    ///
    /// Returns:
    ///     A new WebSocketClient instance
    ///
    /// Raises:
    ///     ConfigError: code 1004 if zero or multiple auth methods provided,
    ///         or if auth_timeout_ms is not greater than 0
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
    ///     # With a longer health check timeout
    ///     hc = HealthCheckConfig(heartbeat_timeout_ms=60000)
    ///     ws = WebSocketClient(api_key="key", health_check=hc)
    ///     ```
    #[new]
    #[pyo3(signature = (*, api_key=None, bearer_token=None, sdk_token=None, base_url=None, version=None, reconnect=None, health_check=None, tls_ca_file=None, tls_root_cert_pem=None, tls_accept_invalid_certs=false, message_overflow=None, message_buffer=None, auth_timeout_ms=None))]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        py: Python<'_>,
        api_key: Option<String>,
        bearer_token: Option<String>,
        sdk_token: Option<String>,
        base_url: Option<String>,
        version: Option<&Bound<'_, PyAny>>,
        reconnect: Option<&Bound<'_, ReconnectConfig>>,
        health_check: Option<&Bound<'_, HealthCheckConfig>>,
        tls_ca_file: Option<String>,
        tls_root_cert_pem: Option<Vec<u8>>,
        tls_accept_invalid_certs: bool,
        message_overflow: Option<String>,
        message_buffer: Option<i64>,
        auth_timeout_ms: Option<i64>,
    ) -> PyResult<Self> {
        // Core requires exactly one non-blank credential (ConfigError, 1004)
        // and sends it in the auth frame field matching its kind (#91).
        let auth = marketdata_core::AuthRequest::from(
            marketdata_core::Auth::from_credentials(api_key, bearer_token, sdk_token)
                .map_err(crate::errors::to_py_err)?,
        );

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
        // Core validates it (ConfigError, 1004): 0 or negative is refused.
        let auth_timeout = match auth_timeout_ms {
            None => marketdata_core::websocket::DEFAULT_AUTH_TIMEOUT,
            Some(ms) => marketdata_core::websocket::auth_timeout_from_millis(
                u64::try_from(ms).unwrap_or(0),
            )
            .map_err(crate::errors::to_py_err)?,
        };

        // Resolve both endpoints now so a bad `base_url` raises here rather
        // than from `.stock.connect()` much later. Matches the official SDK,
        // which rejects a versioned baseUrl at construction.
        for product in [WsProduct::Stock, WsProduct::FutOpt] {
            build_stream_config(
                &auth,
                base_url.as_deref(),
                product,
                stock_version,
                futopt_version,
            )
            .map_err(|e| pyo3::exceptions::PyTypeError::new_err(format!("{e}")))?;
        }

        Ok(Self {
            auth,
            base_url,
            stock_version,
            futopt_version,
            reconnect_config,
            health_check_config,
            tls,
            message_queue,
            auth_timeout,
            stock: PyOnceLock::new(),
            futopt: PyOnceLock::new(),
        })
    }

    /// Access stock market data WebSocket streaming
    ///
    /// Built on first access; every later access returns the same client.
    ///
    /// Returns:
    ///     StockWebSocketClient for stock streaming with inherited config
    #[getter]
    pub fn stock(&self, py: Python<'_>) -> PyResult<Py<StockWebSocketClient>> {
        self.stock
            .get_or_try_init(py, || {
                Py::new(
                    py,
                    StockWebSocketClient::new(
                        self.auth.clone(),
                        self.base_url.clone(),
                        self.stock_version,
                        self.futopt_version,
                        self.reconnect_config.clone(),
                        self.health_check_config.clone(),
                        self.tls.clone(),
                        self.message_queue,
                        self.auth_timeout,
                    ),
                )
            })
            .map(|client| client.clone_ref(py))
    }

    /// Access futures and options WebSocket streaming
    ///
    /// Built on first access; every later access returns the same client.
    ///
    /// Returns:
    ///     FutOptWebSocketClient for FutOpt streaming with inherited config
    #[getter]
    pub fn futopt(&self, py: Python<'_>) -> PyResult<Py<FutOptWebSocketClient>> {
        self.futopt
            .get_or_try_init(py, || {
                Py::new(
                    py,
                    FutOptWebSocketClient::new(
                        self.auth.clone(),
                        self.base_url.clone(),
                        self.stock_version,
                        self.futopt_version,
                        self.reconnect_config.clone(),
                        self.health_check_config.clone(),
                        self.tls.clone(),
                        self.message_queue,
                        self.auth_timeout,
                    ),
                )
            })
            .map(|client| client.clone_ref(py))
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

/// The `version` option: `None`, or a dict from product name to version
/// string. Products and versions are resolved by core (#294); this checks the
/// Python shape and words the errors in dict syntax. Every refusal is a
/// `TypeError`.
fn parse_ws_versions(
    version: Option<&Bound<'_, PyAny>>,
) -> PyResult<(marketdata_core::websocket::StockVersion, marketdata_core::websocket::FutOptVersion)> {
    use marketdata_core::websocket::version::option::{self, VersionOptionError};
    use pyo3::exceptions::PyTypeError;
    use pyo3::types::PyAnyMethods;

    let Some(version) = version.filter(|v| !v.is_none()) else {
        return Ok(Default::default());
    };
    let Ok(map) = version.cast::<pyo3::types::PyDict>() else {
        return Err(PyTypeError::new_err(match version.extract::<String>() {
            Ok(bare) => bare_version_message(&bare),
            Err(_) => format!(
                "version must be a per-product dict like {{'futopt': 'v1.0'}}, got {}",
                version.get_type().name().map(|n| n.to_string()).unwrap_or_else(|_| "?".into())
            ),
        }));
    };

    let mut entries: Vec<(String, String)> = Vec::new();
    for (key, value) in map.iter() {
        let product: String = key.extract().map_err(|_| {
            PyTypeError::new_err("version keys must be product names: 'stock' or 'futopt'")
        })?;
        // `None` counts as not given, like the option itself.
        if value.is_none() {
            continue;
        }
        let requested: String = value.extract().map_err(|_| {
            // An unknown product is the clearer error, whatever its value.
            if !option::product_names().contains(&product.as_str()) {
                return PyTypeError::new_err(format!(
                    "unknown product '{product}' in version mapping (known: {})",
                    option::product_names().join(", ")
                ));
            }
            PyTypeError::new_err(format!(
                "version['{product}'] must be a version string, e.g. 'v1.1', got {}",
                value.get_type().name().map(|n| n.to_string()).unwrap_or_else(|_| "?".into())
            ))
        })?;
        entries.push((product, requested));
    }

    option::resolve(entries.iter().map(|(p, v)| (p.as_str(), v.as_str()))).map_err(|err| {
        PyTypeError::new_err(match &err {
            VersionOptionError::UnknownProduct(key) => format!(
                "unknown product '{key}' in version mapping (known: {})",
                option::product_names().join(", ")
            ),
            VersionOptionError::Unsupported { product, .. } => format!(
                "{} Remove it from the version mapping to use {}.",
                err.describe(),
                option::default_for(product).unwrap_or("the default")
            ),
        })
    })
}

/// The error for `version="v1.0"`: the option is per product, so name the
/// dict that would ask for this version on every product serving it (#294).
fn bare_version_message(bare: &str) -> String {
    let products = marketdata_core::websocket::version::option::products_serving(bare);
    let fix = if products.is_empty() {
        format!("No product serves {bare}.")
    } else {
        let pairs: Vec<String> = products.iter().map(|p| format!("'{p}': '{bare}'")).collect();
        format!("Use version={{{}}}.", pairs.join(", "))
    };
    format!("version must be a per-product dict, not the bare string '{bare}'. {fix}")
}

use marketdata_core::websocket::StreamProduct as WsProduct;

/// Forwards to [`marketdata_core::websocket::stream_config`], which owns the
/// endpoint rules (#252).
fn build_stream_config(
    auth: &marketdata_core::AuthRequest,
    base_url: Option<&str>,
    product: WsProduct,
    stock_version: marketdata_core::websocket::StockVersion,
    futopt_version: marketdata_core::websocket::FutOptVersion,
) -> Result<marketdata_core::ConnectionConfig, marketdata_core::MarketDataError> {
    marketdata_core::websocket::stream_config(auth, base_url, product, stock_version, futopt_version)
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
    /// What the stream reader has delivered to the callbacks, for
    /// `connect()` (#230).
    delivered: Delivered,
    /// This connection's stream reader, for the `disconnect()` that closes
    /// it to wait for (#277).
    reader: Option<std::thread::JoinHandle<()>>,
}

/// Shared so blocking calls can clone it out of its lock before they block.
type SharedRuntime = Arc<tokio::runtime::Runtime>;

/// The stored connection, as core's `admit` reads it. Called with the GIL
/// released, so nothing here may need it: the lock is held only to clone the
/// client and its `Delivered` out. A poisoned lock is read through, as the
/// read cannot report a failure.
fn read_stored(state: &Mutex<Option<WebSocketState>>) -> Option<StoredConnection> {
    let state = state.lock().unwrap_or_else(|e| e.into_inner());
    state
        .as_ref()
        .map(|s| StoredConnection::new(Arc::clone(&s.inner), s.delivered.clone()))
}

/// What a `connect()` does with core's decision: `Some` to open a new
/// connection, holding the claim until it is stored; `None` once it has
/// joined the stored connection's reconnect, with nothing left to do.
fn admitted(
    admission: Result<Admission, marketdata_core::MarketDataError>,
) -> PyResult<Option<ConnectClaim>> {
    match admission.map_err(errors::to_py_err)? {
        Admission::Open(claim) => Ok(Some(claim)),
        Admission::Joined => Ok(None),
    }
}

/// Core's `admit` for a blocking `connect()`, which waits out a join with
/// the GIL released.
fn admit_blocking(
    py: Python<'_>,
    gate: &ConnectGate,
    state: &Arc<Mutex<Option<WebSocketState>>>,
    runtime: &Mutex<Option<SharedRuntime>>,
) -> PyResult<Option<ConnectClaim>> {
    let runtime = runtime.lock().map_err(lock_err)?.clone().ok_or_else(|| {
        pyo3::exceptions::PyRuntimeError::new_err("Runtime not initialized")
    })?;
    let gate = gate.clone();
    let state = Arc::clone(state);
    admitted(block_on_detached(py, runtime, async move {
        admit(&gate, || read_stored(&state)).await
    }))
}

fn lock_err<T>(e: std::sync::PoisonError<T>) -> PyErr {
    pyo3::exceptions::PyRuntimeError::new_err(format!("Lock error: {}", e))
}

/// The core client a `connect()` / `connect_async()` is establishing, and its
/// stream reader (#143). `state` only holds a connection once it is up, so
/// without this a `disconnect()` during the handshake found nothing to close
/// and the connect went on to install a live client.
///
/// Whoever takes it out owns the reader: the connect when it finishes, or a
/// `disconnect()` that aborts it. Lock order: this slot before `state`.
struct PendingConnect {
    client: Arc<marketdata_core::aio::WebSocketClient>,
    reader_thread: std::thread::JoinHandle<()>,
    /// The reader's stop flag, for a `disconnect()` that aborts the connect.
    stop: Arc<AtomicBool>,
}

type PendingSlot = Arc<Mutex<Option<PendingConnect>>>;

/// Clears the pending entry of a `connect_async()` cancelled before it
/// settles, so its client and stream reader do not outlive it. Created after
/// the connect claim, so it drops while the claim still keeps other connects
/// out: the entry, if any, is this connect's.
struct ClearPendingOnDrop(PendingSlot);

impl Drop for ClearPendingOnDrop {
    fn drop(&mut self) {
        let entry = self.0.lock().ok().and_then(|mut slot| slot.take());
        drop(entry);
    }
}

/// What a `disconnect()` has to close: the stored connection, a connect in
/// progress, or both (a new connect may run while `state` still holds a
/// connection that was lost).
struct CloseTarget {
    live: Option<WebSocketState>,
    connecting: Option<Arc<marketdata_core::aio::WebSocketClient>>,
    /// The stop flag of the connect in progress's reader.
    connecting_stop: Option<Arc<AtomicBool>>,
}

impl CloseTarget {
    fn is_empty(&self) -> bool {
        self.live.is_none() && self.connecting.is_none()
    }

    /// Core's `disconnect()` on each: a connect in progress returns
    /// `ConnectionAborted` (code 2010) and closes its socket (#121).
    async fn disconnect(&self) {
        if let Some(state) = &self.live {
            let _ = state.inner.disconnect().await;
        }
        if let Some(client) = &self.connecting {
            let _ = client.disconnect().await;
        }
    }
}

/// Take everything `disconnect()` has to close, and the reader of a connect
/// it aborts, under the same locks [`settle_connect`] installs a connection
/// with, so a disconnect cannot fall between a connect succeeding and its
/// connection being stored.
fn take_close_target(
    pending: &PendingSlot,
    state: &Mutex<Option<WebSocketState>>,
) -> PyResult<(CloseTarget, Option<std::thread::JoinHandle<()>>)> {
    let mut pending = pending.lock().map_err(lock_err)?;
    let mut state = state.lock().map_err(lock_err)?;
    let (connecting, reader, connecting_stop) = match pending.take() {
        Some(p) => (Some(p.client), Some(p.reader_thread), Some(p.stop)),
        None => (None, None, None),
    };
    Ok((CloseTarget { live: state.take(), connecting, connecting_stop }, reader))
}

/// How a `connect()` ended, once its pending entry is settled.
enum ConnectOutcome {
    /// The connection is stored in `state`, with its reader.
    Installed,
    /// The connect failed on its own; the caller joins the reader.
    Failed(marketdata_core::MarketDataError, std::thread::JoinHandle<()>),
    /// A `disconnect()` took the pending entry and closes the connection;
    /// it also joins the reader.
    Aborted(marketdata_core::MarketDataError),
}

/// Settle a finished connect: install the connection if it succeeded and no
/// `disconnect()` took it meanwhile (#143).
///
/// A connection it replaces was lost: its client is dropped here, and its
/// reader parked for the next `disconnect()` to wait for (#277).
fn settle_connect(
    pending: &PendingSlot,
    state: &Mutex<Option<WebSocketState>>,
    parked: &ParkedReaders,
    closed: &AtomicBool,
    result: Result<(), marketdata_core::MarketDataError>,
    make_state: impl FnOnce(std::thread::JoinHandle<()>) -> WebSocketState,
) -> PyResult<ConnectOutcome> {
    let mut pending = pending.lock().map_err(lock_err)?;
    let Some(entry) = pending.take() else {
        // Core returns `ConnectionAborted` when the disconnect reached it
        // during the handshake. One that came just after core succeeded has
        // closed the connection all the same, so it is reported alike.
        let err = result.err().unwrap_or(marketdata_core::MarketDataError::ConnectionAborted);
        return Ok(ConnectOutcome::Aborted(err));
    };
    if let Err(e) = result {
        return Ok(ConnectOutcome::Failed(e, entry.reader_thread));
    }
    let mut state = state.lock().map_err(lock_err)?;
    closed.store(false, Ordering::SeqCst);
    let replaced = state.replace(make_state(entry.reader_thread));
    if let Some(installed) = state.as_ref() {
        // Under the `state` lock, so a `connect()` sees it with the connection.
        installed.delivered.connect_succeeded();
    }
    if let Some(mut lost) = replaced {
        // Its stop is left unset: until a `disconnect()` waits for the reader,
        // iterators still get what core queued for them.
        if let Some(reader) = lost.reader.take() {
            park_reader_thread(parked, reader, Some(Arc::clone(&lost.stop)));
        }
    }
    Ok(ConnectOutcome::Installed)
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

/// [`live_handles`] for `measure_latency`, which reports a missing
/// connection as `ClientClosed` like core does.
fn latency_handles(
    state: &Mutex<Option<WebSocketState>>,
    runtime: &Mutex<Option<SharedRuntime>>,
) -> PyResult<(Arc<marketdata_core::aio::WebSocketClient>, SharedRuntime)> {
    live_handles(state, runtime)
        .map_err(|_| errors::to_py_err(marketdata_core::MarketDataError::ClientClosed))
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
/// authentication never reach `message`. Whether a `message` or
/// `raw_message` callback is registered is checked for each message: if one
/// is, the callbacks get the message — `raw_message` first, with the frame's
/// text, then `message`, with the dict built only when one is registered
/// (#246); otherwise the message waits in `handoff` for a `messages()`
/// iterator. A message in flight while `off()` removes the last callback may
/// reach neither.
fn spawn_stream_reader(
    name: &str,
    stream: Arc<marketdata_core::StreamReceiver>,
    callbacks: Arc<CallbackRegistry>,
    handoff: Arc<Handoff>,
    stop: Arc<AtomicBool>,
    delivered: Delivered,
    last_disconnect: LastDisconnect,
    test_panic: Option<String>,
) -> PyResult<std::thread::JoinHandle<()>> {
    use crate::callback::EventType;
    use marketdata_core::websocket::{ConnectionEvent, StreamItem};

    std::thread::Builder::new()
        .name(name.to_string())
        .spawn(move || {
            ON_STREAM_READER.with(|flag| flag.set(true));
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
                            delivered.observe(&event);
                            Python::attach(|py| {
                                forward_event(py, &callbacks, &last_disconnect, event)
                            });
                        }
                        StreamItem::Message(msg) if authenticated => {
                            handling.set("message");
                            inject_test_panic(test_panic.as_deref(), "ws_messages");
                            let wants_raw = callbacks.count(EventType::RawMessage) > 0;
                            let wants_dict = callbacks.count(EventType::Message) > 0;
                            if !wants_raw && !wants_dict {
                                handoff.push(msg, &stop);
                                continue;
                            }
                            Python::attach(|py| {
                                if wants_raw {
                                    let args = pyo3::types::PyTuple::new(py, [msg.raw.as_str()])
                                        .expect("Failed to create tuple");
                                    callbacks.invoke(py, EventType::RawMessage, &args);
                                }
                                if wants_dict {
                                    inject_test_panic(test_panic.as_deref(), "ws_message_dict");
                                    if let Ok(dict) = message_to_dict(py, &msg) {
                                        let args = pyo3::types::PyTuple::new(py, [dict.into_any()])
                                            .expect("Failed to create tuple");
                                        callbacks.invoke(py, EventType::Message, &args);
                                    }
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

/// Panic if the test asked for one at `here` (`ws_events`, `ws_messages`,
/// `ws_message_dict`: about to build the dict for `message` callbacks).
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
    last_disconnect: &Mutex<Option<DisconnectInfo>>,
    event: marketdata_core::websocket::ConnectionEvent,
) {
    use marketdata_core::websocket::ConnectionEvent;
    match event {
        ConnectionEvent::Connected => callbacks.invoke_connect(py),
        ConnectionEvent::Authenticated { data, frame } => {
            callbacks.invoke_authenticated(py, &frame, data)
        }
        ConnectionEvent::Unauthenticated { data, frame, .. } => {
            callbacks.invoke_unauthenticated(py, &frame, data)
        }
        ConnectionEvent::Disconnected { code, reason, intent, will_reconnect } => {
            // Before the callbacks, so one reading `last_disconnect` sees the
            // disconnect it is handling. The lock is released before any
            // Python code runs.
            *last_disconnect.lock().unwrap_or_else(std::sync::PoisonError::into_inner) =
                Some(DisconnectInfo { code, reason: reason.clone(), intent: intent.as_str(), will_reconnect });
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
        ConnectionEvent::Error(info) if info.code == marketdata_core::error_code::RECONNECT_CONFLICT => {
            callbacks.warn_reconnect_conflict(py, &info)
        }
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

thread_local! {
    /// Set on stream reader threads, where callbacks run.
    static ON_STREAM_READER: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Whether this thread is a stream reader: a `disconnect()` called here runs
/// from a callback.
fn on_stream_reader() -> bool {
    ON_STREAM_READER.with(|flag| flag.get())
}

/// A stream reader no one could wait for when its connection went, and the
/// connection's stop flag, if it has one.
struct ParkedReader {
    handle: std::thread::JoinHandle<()>,
    stop: Option<Arc<AtomicBool>>,
}

/// Stream readers no one could wait for when their connection went: those of
/// the connection a `disconnect()` or `disconnect_async()` from a callback
/// closed and of the connect it aborted — a `connect` callback during the
/// handshake, say — and that of a lost connection a `connect()` replaced.
/// Each connection is closed or gone, so waiting for them always ends once
/// their stop flag is set; the next `disconnect()` or `disconnect_async()`
/// from outside a callback does both (#277, #280).
type ParkedReaders = Mutex<Vec<ParkedReader>>;

/// Park `handle` for the next `disconnect()`. Those parked earlier that have
/// ended are let go without a join: one may still be running its thread-local
/// destructors, and this can run under the connection locks.
fn park_reader_thread(
    parked: &ParkedReaders,
    handle: std::thread::JoinHandle<()>,
    stop: Option<Arc<AtomicBool>>,
) {
    let Ok(mut guard) = parked.lock() else { return };
    guard.retain(|reader| !reader.handle.is_finished());
    guard.push(ParkedReader { handle, stop });
}

/// Take the parked readers to wait for, setting their stop flags: from here
/// they no longer wait for an iterator to make room, as after `disconnect()`.
fn take_parked_readers(parked: &ParkedReaders) -> Vec<std::thread::JoinHandle<()>> {
    let readers = parked.lock().map(|mut guard| std::mem::take(&mut *guard)).unwrap_or_default();
    readers
        .into_iter()
        .map(|reader| {
            if let Some(stop) = &reader.stop {
                stop.store(true, Ordering::SeqCst);
            }
            reader.handle
        })
        .collect()
}

/// [`join_reader_thread`] on the parked readers.
///
/// Not from a callback: a `disconnect()` or `disconnect_async()` there waits
/// for no stream reader, since two readers disconnecting from callbacks would
/// wait for each other. The readers stay parked for the next `disconnect()`
/// or `disconnect_async()` from another thread, which waits for the remaining
/// callbacks.
fn join_parked_reader_threads(py: Python<'_>, parked: &ParkedReaders) {
    if on_stream_reader() {
        return;
    }
    let handles = take_parked_readers(parked);
    if handles.is_empty() {
        return;
    }
    py.detach(move || {
        for handle in handles {
            let _ = handle.join();
        }
    });
}

/// What a `disconnect()` does with the reader of the connection it closes, or
/// of a connect it aborts (#143).
///
/// From a callback — which runs on a stream reader — the handle is parked for
/// a later `disconnect()` to wait on, not joined: the reader may be this
/// thread, or another one waiting for this thread, such as one whose
/// `connect()` failed and waits for this connection's reader before it
/// raises. Call before the close wakes the connect, so the handle is there
/// once `connect()` raises.
///
/// Returns the handle to join after the close.
fn park_own_reader_thread(
    handle: Option<std::thread::JoinHandle<()>>,
    stop: Option<Arc<AtomicBool>>,
    parked: &ParkedReaders,
) -> Option<std::thread::JoinHandle<()>> {
    park_reader_if_from_callback(on_stream_reader(), handle, stop, parked)
}

/// [`park_own_reader_thread`] for a `disconnect()` whose caller was, or was
/// not, on a stream reader.
fn park_reader_if_from_callback(
    from_callback: bool,
    handle: Option<std::thread::JoinHandle<()>>,
    stop: Option<Arc<AtomicBool>>,
    parked: &ParkedReaders,
) -> Option<std::thread::JoinHandle<()>> {
    match handle {
        Some(handle) if from_callback => {
            park_reader_thread(parked, handle, stop);
            None
        }
        other => other,
    }
}

/// Async counterpart of [`join_reader_thread`]: joins on the blocking pool so
/// the awaiting task does not stall a runtime worker. The task runs on a
/// runtime thread, not on its caller's, so whether that caller was a stream
/// reader is decided before the task starts (#280).
async fn join_reader_thread_async(handle: Option<std::thread::JoinHandle<()>>) {
    let Some(handle) = handle else { return };
    let _ = tokio::task::spawn_blocking(move || handle.join()).await;
}

/// Async counterpart of [`join_parked_reader_threads`].
async fn join_parked_reader_threads_async(parked: &ParkedReaders) {
    for handle in take_parked_readers(parked) {
        join_reader_thread_async(Some(handle)).await;
    }
}

// ---------------------------------------------------------------------------
// Async methods shared by the stock and futopt clients (#272)
//
// Each client's `*_async` method only gathers its own handles; the awaitable
// is built here, so both products join a reconnect (#230), admit concurrent
// connects together (#268), wait only for their own reader (#277) and admit
// through core (#271) in the same code.
// ---------------------------------------------------------------------------

/// What `connect_async()` takes from a product client, owned so its
/// awaitable can outlive the call.
struct AsyncConnect {
    /// The same config `connect()` uses, TLS settings included. Should
    /// `build_config()` fail, it falls back to the production endpoint as for
    /// `connect()`; `connect_async()` used to raise `TypeError` on await
    /// instead.
    config: marketdata_core::ConnectionConfig,
    reconnect_config: marketdata_core::ReconnectionConfig,
    health_check_config: marketdata_core::HealthCheckConfig,
    reader_name: &'static str,
    callbacks: Arc<CallbackRegistry>,
    state: Arc<Mutex<Option<WebSocketState>>>,
    parked_readers: Arc<ParkedReaders>,
    messages_dropped: Arc<Mutex<Option<marketdata_core::MessagesDroppedHandle>>>,
    last_disconnect: LastDisconnect,
    reconnect_conflict: marketdata_core::ReconnectConflictHandle,
    connect_gate: ConnectGate,
    pending: PendingSlot,
    closed: Arc<AtomicBool>,
    test_panic: Option<String>,
}

impl AsyncConnect {
    async fn run(self) -> PyResult<()> {
        let Self {
            config,
            reconnect_config,
            health_check_config,
            reader_name,
            callbacks,
            state,
            parked_readers,
            messages_dropped,
            last_disconnect,
            reconnect_conflict,
            connect_gate,
            pending,
            closed,
            test_panic,
        } = self;
        let Some(_claim) = admitted(admit(&connect_gate, || read_stored(&state)).await)? else {
            return Ok(());
        };
        let capacity = handoff_capacity(&config);
        let ws_client = Arc::new(marketdata_core::aio::WebSocketClient::with_full_config(
            config,
            reconnect_config,
            health_check_config,
        ));
        if let Ok(mut slot) = messages_dropped.lock() {
            *slot = Some(ws_client.messages_dropped_handle());
        }
        // Before connect(): it warns about the previous connection's close.
        ws_client.use_reconnect_conflict_handle(&reconnect_conflict);

        let handoff = Arc::new(Handoff::new(capacity));
        let stop = Arc::new(AtomicBool::new(false));
        let delivered = Delivered::default();
        let reader_thread = spawn_stream_reader(
            reader_name,
            ws_client.stream_receiver(),
            callbacks,
            Arc::clone(&handoff),
            Arc::clone(&stop),
            delivered.clone(),
            last_disconnect,
            test_panic,
        )?;

        // From here a `disconnect()` aborts this connect (#143).
        *pending.lock().map_err(lock_err)? = Some(PendingConnect {
            client: Arc::clone(&ws_client),
            reader_thread,
            stop: Arc::clone(&stop),
        });
        let _cancelled = ClearPendingOnDrop(Arc::clone(&pending));

        // Connect without holding GIL
        let result = ws_client.connect().await;
        let outcome = settle_connect(
            &pending,
            &state,
            &parked_readers,
            &closed,
            result,
            |reader| WebSocketState {
                inner: Arc::clone(&ws_client),
                handoff,
                stop,
                delivered,
                reader: Some(reader),
            },
        )?;
        match outcome {
            ConnectOutcome::Installed => Ok(()),
            ConnectOutcome::Failed(e, reader_thread) => {
                // See `connect`: callbacks for the failure fire before we raise.
                drop(ws_client);
                join_reader_thread_async(Some(reader_thread)).await;
                Err(errors::to_py_err(e))
            }
            ConnectOutcome::Aborted(e) => Err(errors::to_py_err(e)),
        }
    }
}

/// The awaitable of `connect_async()`.
fn connect_async_awaitable<'py>(py: Python<'py>, connect: AsyncConnect) -> PyResult<Bound<'py, PyAny>> {
    future_into_py(py, connect.run())
}

/// The awaitable of `__aenter__`: connects, then resolves to the client, so
/// `async with ws.stock as client` binds the client rather than `None`.
fn aenter_awaitable<'py>(
    py: Python<'py>,
    connect: AsyncConnect,
    client: Py<PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    future_into_py(py, async move {
        connect.run().await?;
        Ok(client)
    })
}

/// The awaitable of `disconnect_async()`.
fn disconnect_async_awaitable<'py>(
    py: Python<'py>,
    state_arc: Arc<Mutex<Option<WebSocketState>>>,
    parked_readers: Arc<ParkedReaders>,
    closed: Arc<AtomicBool>,
    pending: PendingSlot,
) -> PyResult<Bound<'py, PyAny>> {
    // Here, not in the task: the task runs on a runtime thread, while a
    // callback awaiting it — with `asyncio.run()`, say — holds its stream
    // reader until it completes (#280).
    let from_callback = on_stream_reader();
    future_into_py(py, async move {
        let (mut target, aborted_reader) = take_close_target(&pending, &state_arc)?;
        // See `disconnect` (#277).
        let (own_reader, own_stop) = match target.live.as_mut() {
            Some(state) => (state.reader.take(), Some(Arc::clone(&state.stop))),
            None => (None, None),
        };
        let own_reader = park_reader_if_from_callback(from_callback, own_reader, own_stop, &parked_readers);
        let aborted_stop = target.connecting_stop.clone();
        let aborted_reader =
            park_reader_if_from_callback(from_callback, aborted_reader, aborted_stop, &parked_readers);

        if !target.is_empty() {
            // See `disconnect`.
            closed.store(true, Ordering::SeqCst);
            if let Some(state) = &target.live {
                state.stop.store(true, Ordering::SeqCst);
            }
            if let Some(stop) = &target.connecting_stop {
                stop.store(true, Ordering::SeqCst);
            }
            target.disconnect().await;
            // Note: do NOT manually invoke_disconnect — core's disconnect()
            // emits ConnectionEvent::Disconnected on its stream and the
            // stream reader fires the user callback.
            //
            // Nothing else to release: this path runs on
            // pyo3-async-runtimes' runtime, not `self.runtime`, and
            // core's disconnect() has already stopped the dispatch and
            // writer tasks, so dropping the client closes its stream
            // (#54).
            drop(target);
        }

        join_reader_thread_async(own_reader).await;
        join_reader_thread_async(aborted_reader).await;
        // See `join_parked_reader_threads`.
        if !from_callback {
            join_parked_reader_threads_async(&parked_readers).await;
        }

        Ok(())
    })
}

/// Resolve `subscribe()` / `subscribe_async()`'s dual-shape input into
/// owned, Send-safe data: `(channel, symbols, modifier)`. `method` names the
/// caller in the error for a first argument of the wrong type.
fn resolve_subscribe_args(
    method: &str,
    channel: &Bound<'_, PyAny>,
    symbol: Option<&str>,
    symbols: Option<Vec<String>>,
    flag: Option<bool>,
    modifier: Modifier,
) -> PyResult<(String, Vec<String>, bool)> {
    if let Ok(d) = channel.cast::<PyDict>() {
        let d = &without_none(method, d)?;
        // The dict is the whole call, as in the 1.x `subscribe(params)`;
        // arguments next to it used to be dropped (#294).
        if symbol.is_some() || symbols.is_some() || flag.is_some() {
            return Err(pyo3::exceptions::PyTypeError::new_err(format!(
                "{method}(): pass either a dict or the channel with symbol / symbols / {}, not both",
                modifier.kwarg
            )));
        }
        extract_subscribe_dict(method, d, modifier)
    } else if let Ok(s) = channel.extract::<String>() {
        let syms = resolve_symbol_args("subscribe", symbol, symbols)?;
        Ok((s, syms, flag.unwrap_or(false)))
    } else {
        Err(pyo3::exceptions::PyTypeError::new_err(
            format!("{method}() first argument must be a dict or channel string"),
        ))
    }
}

/// The connected core client for `subscribe_async()`, cloned out of the lock
/// so no guard is held across its await.
fn subscribe_async_client(
    state_arc: &Mutex<Option<WebSocketState>>,
) -> PyResult<Arc<marketdata_core::aio::WebSocketClient>> {
    let state_guard = state_arc
        .lock()
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Lock error: {}", e)))?;
    let state = state_guard
        .as_ref()
        .ok_or_else(|| pyo3::exceptions::PyRuntimeError::new_err("Not connected. Call connect() first."))?;
    Ok(Arc::clone(&state.inner))
}

/// The awaitable of `measure_latency_async()`.
fn measure_latency_async_awaitable<'py>(
    py: Python<'py>,
    state_arc: Arc<Mutex<Option<WebSocketState>>>,
    timeout_ms: Option<u64>,
) -> PyResult<Bound<'py, PyAny>> {
    let timeout = timeout_ms.map(Duration::from_millis);
    future_into_py(py, async move {
        // Cloned out of the lock so no guard is held across the await.
        let ws_client = state_arc
            .lock()
            .map_err(lock_err)?
            .as_ref()
            .map(|state| Arc::clone(&state.inner))
            .ok_or_else(|| errors::to_py_err(marketdata_core::MarketDataError::ClientClosed))?;
        ws_client
            .measure_latency(timeout)
            .await
            .map(|rtt| rtt.as_secs_f64() * 1000.0)
            .map_err(errors::to_py_err)
    })
}

/// What sets one product's client apart from the other's.
struct ProductSpec {
    stream: WsProduct,
    /// Name of the thread that reads the stream.
    reader_name: &'static str,
    /// The production endpoint, for a client built with a bad `base_url`.
    fallback_config: fn(marketdata_core::AuthRequest) -> marketdata_core::ConnectionConfig,
}

static STOCK: ProductSpec = ProductSpec {
    stream: WsProduct::Stock,
    reader_name: "stock_ws_stream",
    fallback_config: marketdata_core::ConnectionConfig::fugle_stock,
};

static FUTOPT: ProductSpec = ProductSpec {
    stream: WsProduct::FutOpt,
    reader_name: "futopt_ws_stream",
    fallback_config: marketdata_core::ConnectionConfig::fugle_futopt,
};

/// What `StockWebSocketClient` and `FutOptWebSocketClient` share: the
/// connection state and the implementation of every sync method (#281). The
/// two pyclasses deref to it, so their `#[pymethods]` keep only the Python
/// signatures, docstrings and each product's channel and subscription types.
/// `pub` only because the pyclasses' `Deref` names it; the module is private.
pub struct ProductClient {
    product: &'static ProductSpec,
    auth: marketdata_core::AuthRequest,
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
    parked_readers: Arc<ParkedReaders>,
    message_queue: MessageQueueSettings,
    auth_timeout: Duration,
    /// Dropped-message count of the current or last connection; outlives the
    /// core client, which `disconnect()` drops.
    messages_dropped: Arc<Mutex<Option<marketdata_core::MessagesDroppedHandle>>>,
    /// The last disconnect of any of this client's connections; never
    /// cleared, so it outlives `disconnect()` and a reconnect (#293).
    last_disconnect: LastDisconnect,
    /// Carries a close made soon after an automatic reconnect to the next
    /// `connect()`, whose core client is a new one, for the 3006 warning
    /// (#226, #242).
    reconnect_conflict: marketdata_core::ReconnectConflictHandle,
    /// `disconnect()` drops `state`, so "has this client been closed?" cannot
    /// be answered from it — `is_closed()` read `None` as "not closed" and
    /// contradicted its own docstring (#146). Set when a disconnect actually
    /// closes something — a live client or a connect in progress (#143) —
    /// cleared when `connect()` installs a new one.
    closed: Arc<AtomicBool>,
    connect_gate: ConnectGate,
    /// The connect in progress, for `disconnect()` to abort (#143).
    pending: PendingSlot,
}

impl ProductClient {
    #[allow(clippy::too_many_arguments)]
    fn new(
        product: &'static ProductSpec,
        auth: marketdata_core::AuthRequest,
        base_url: Option<String>,
        stock_version: marketdata_core::websocket::StockVersion,
        futopt_version: marketdata_core::websocket::FutOptVersion,
        reconnect_config: ReconnectConfig,
        health_check_config: HealthCheckConfig,
        tls: marketdata_core::TlsConfig,
        message_queue: MessageQueueSettings,
        auth_timeout: Duration,
    ) -> Self {
        Self {
            product,
            auth,
            base_url,
            stock_version,
            futopt_version,
            reconnect_config,
            health_check_config,
            tls,
            callbacks: Arc::new(CallbackRegistry::new()),
            state: Arc::new(Mutex::new(None)),
            runtime: Arc::new(Mutex::new(None)),
            parked_readers: Arc::new(Mutex::new(Vec::new())),
            message_queue,
            auth_timeout,
            messages_dropped: Arc::new(Mutex::new(None)),
            last_disconnect: Arc::new(Mutex::new(None)),
            reconnect_conflict: marketdata_core::ReconnectConflictHandle::default(),
            closed: Arc::new(AtomicBool::new(false)),
            connect_gate: ConnectGate::default(),
            pending: Arc::new(Mutex::new(None)),
        }
    }

    fn build_config(&self) -> marketdata_core::ConnectionConfig {
        // `base_url` was already validated in `WebSocketClient::new`, so the
        // only way this can fail is a caller constructing the product client
        // directly — fall back to the production endpoint rather than panic.
        let mut config = build_stream_config(
            &self.auth,
            self.base_url.as_deref(),
            self.product.stream,
            self.stock_version,
            self.futopt_version,
        )
        .unwrap_or_else(|_| (self.product.fallback_config)(self.auth.clone()));
        config.tls = self.tls.clone();
        self.message_queue.apply(&mut config);
        config.auth_timeout = self.auth_timeout;
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

    /// What `connect_async()` / `__aenter__` take from this client.
    fn async_connect(&self) -> PyResult<AsyncConnect> {
        // `subscribe()` after an async connect runs on this runtime.
        self.ensure_runtime().map_err(pyo3::exceptions::PyRuntimeError::new_err)?;
        Ok(AsyncConnect {
            config: self.build_config(),
            reconnect_config: self.reconnect_config.to_core(),
            health_check_config: self.health_check_config.to_core(),
            reader_name: self.product.reader_name,
            callbacks: Arc::clone(&self.callbacks),
            state: Arc::clone(&self.state),
            parked_readers: Arc::clone(&self.parked_readers),
            messages_dropped: Arc::clone(&self.messages_dropped),
            last_disconnect: Arc::clone(&self.last_disconnect),
            reconnect_conflict: self.reconnect_conflict.clone(),
            connect_gate: self.connect_gate.clone(),
            pending: Arc::clone(&self.pending),
            closed: Arc::clone(&self.closed),
            test_panic: test_panic_site(),
        })
    }

    /// Run `f` on the live core client with the GIL released; raises if not
    /// connected.
    fn block_on_live<F, Fut, T>(&self, py: Python<'_>, f: F) -> PyResult<T>
    where
        F: FnOnce(Arc<marketdata_core::aio::WebSocketClient>) -> Fut,
        Fut: std::future::Future<Output = Result<T, marketdata_core::MarketDataError>> + Send,
        T: Send,
    {
        let (inner, runtime) = live_handles(&self.state, &self.runtime)?;
        block_on_detached(py, runtime, f(inner)).map_err(errors::to_py_err)
    }

    fn connect(&self, py: Python<'_>) -> PyResult<()> {
        // Before the admission: it runs on this runtime.
        self.ensure_runtime().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(e)
        })?;
        let Some(_claim) = admit_blocking(py, &self.connect_gate, &self.state, &self.runtime)? else {
            return Ok(());
        };
        // Again: the admission ran with the GIL released, where a
        // `disconnect()` may have taken the runtime. From here to the
        // connect below the GIL is held, so this one stays.
        self.ensure_runtime().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(e)
        })?;
        let test_panic = test_panic_site();

        // Create WebSocket client with full config
        let config = self.build_config();
        let capacity = handoff_capacity(&config);
        let ws_client = Arc::new(marketdata_core::aio::WebSocketClient::with_full_config(
            config,
            self.reconnect_config.to_core(),
            self.health_check_config.to_core(),
        ));
        *self.messages_dropped.lock().map_err(lock_err)? = Some(ws_client.messages_dropped_handle());
        // Before connect(): it warns about the previous connection's close.
        ws_client.use_reconnect_conflict_handle(&self.reconnect_conflict);

        let handoff = Arc::new(Handoff::new(capacity));
        let stop = Arc::new(AtomicBool::new(false));
        let delivered = Delivered::default();
        let reader_thread = spawn_stream_reader(
            self.product.reader_name,
            ws_client.stream_receiver(),
            Arc::clone(&self.callbacks),
            Arc::clone(&handoff),
            Arc::clone(&stop),
            delivered.clone(),
            Arc::clone(&self.last_disconnect),
            test_panic,
        )?;

        let runtime = self.runtime.lock().map_err(lock_err)?.clone().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err("Runtime not initialized")
        })?;

        // From here a `disconnect()` aborts this connect (#143).
        *self.pending.lock().map_err(lock_err)? = Some(PendingConnect {
            client: Arc::clone(&ws_client),
            reader_thread,
            stop: Arc::clone(&stop),
        });

        // Connect with the GIL released: the handshake and auth ack may come
        // from a server that needs this interpreter to run (#39).
        let (ws_client, result) = py.detach(move || {
            let result = runtime.block_on(ws_client.connect());
            (ws_client, result)
        });

        let outcome = settle_connect(
            &self.pending,
            &self.state,
            &self.parked_readers,
            &self.closed,
            result,
            |reader| WebSocketState {
                inner: Arc::clone(&ws_client),
                handoff,
                stop,
                delivered,
                reader: Some(reader),
            },
        )?;
        match outcome {
            ConnectOutcome::Installed => Ok(()),
            ConnectOutcome::Failed(e, reader_thread) => {
                // Dropping the client closes the stream; the reader first
                // delivers `unauthenticated` / `error`, so they fire before
                // we raise.
                drop(ws_client);
                join_reader_thread(py, reader_thread);
                Err(errors::to_py_err(e))
            }
            // The `disconnect()` that aborted this connect waits for the
            // reader once this client is dropped.
            ConnectOutcome::Aborted(e) => Err(errors::to_py_err(e)),
        }
    }

    fn disconnect(&self, py: Python<'_>) -> PyResult<()> {
        let (mut target, aborted_reader) = take_close_target(&self.pending, &self.state)?;
        // Only this connection's reader: one a `connect()` installs while
        // this call closes is not waited for (#277).
        let (own_reader, own_stop) = match target.live.as_mut() {
            Some(state) => (state.reader.take(), Some(Arc::clone(&state.stop))),
            None => (None, None),
        };
        let own_reader = park_own_reader_thread(own_reader, own_stop, &self.parked_readers);
        let aborted_stop = target.connecting_stop.clone();
        let aborted_reader = park_own_reader_thread(aborted_reader, aborted_stop, &self.parked_readers);

        if !target.is_empty() {
            // Recorded before the close so `is_closed()` is true the moment
            // `disconnect()` returns, even though `state` is gone (#146).
            self.closed.store(true, Ordering::SeqCst);
            if let Some(state) = &target.live {
                // Messages before `Disconnected` still reach the callbacks,
                // but the reader no longer waits for an iterator to make room.
                state.stop.store(true, Ordering::SeqCst);
            }
            // No iterator reads an aborted connect's messages.
            if let Some(stop) = &target.connecting_stop {
                stop.store(true, Ordering::SeqCst);
            }
            // Take ownership of the runtime so dropping it aborts every
            // spawned task (dispatch, writer, health check). Without this,
            // those tasks keep their Arc<WebSocketClient> clones alive, the
            // stream never closes, and the stream reader blocks forever on
            // receive() — preventing Python from shutting down. A connect in
            // progress holds its own clone, so the runtime outlives this call
            // until core aborts that connect (#143).
            let runtime = self.runtime.lock().map_err(lock_err)?.take();

            if let Some(rt) = runtime {
                // The close handshake waits on the server, so release the GIL (#39).
                py.detach(move || {
                    rt.block_on(target.disconnect());
                    // `rt` drops first. The runtime shuts down once every
                    // clone is gone — a connect racing this call holds its
                    // own until it returns — which aborts all spawned tasks
                    // and drops their futures, releasing every
                    // Arc<WebSocketClient> clone they held. `target` drops
                    // next → core's WebSocketClient drops → its stream
                    // closes → the stream reader drains it and exits cleanly.
                    drop(rt);
                    drop(target);
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
        // An aborted connect's reader ends once that connect drops its
        // client too.
        if let Some(handle) = own_reader {
            join_reader_thread(py, handle);
        }
        if let Some(handle) = aborted_reader {
            join_reader_thread(py, handle);
        }
        join_parked_reader_threads(py, &self.parked_readers);

        Ok(())
    }

    fn is_connected(&self, py: Python<'_>) -> bool {
        match live_handles(&self.state, &self.runtime) {
            Ok((inner, runtime)) => {
                block_on_detached(py, runtime, async move { inner.is_connected().await })
            }
            Err(_) => false,
        }
    }

    fn is_closed(&self, py: Python<'_>) -> bool {
        // `disconnect()` drops `state`, so ask the flag first: without it a
        // closed client reports `False` here, contradicting the `is_closed()`
        // docstring on the pyclasses (#146).
        if self.closed.load(Ordering::SeqCst) {
            return true;
        }

        // State still None and never disconnected: never connected, not closed.
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

    fn url(&self) -> PyResult<String> {
        // `base_url` and `version` were validated in `WebSocketClient::new`,
        // so this cannot fail for a client built through it; no fallback to
        // the production endpoint (#245).
        build_stream_config(
            &self.auth,
            self.base_url.as_deref(),
            self.product.stream,
            self.stock_version,
            self.futopt_version,
        )
        .map(|config| config.url)
        .map_err(errors::to_py_err)
    }

    fn messages_dropped_total(&self) -> u64 {
        self.messages_dropped
            .lock()
            .ok()
            .and_then(|slot| slot.as_ref().map(|handle| handle.total()))
            .unwrap_or(0)
    }

    fn last_disconnect(&self) -> Option<DisconnectInfo> {
        self.last_disconnect.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clone()
    }

    fn messages(
        &self,
        py: Python<'_>,
        timeout_ms: Option<u64>,
        raw: bool,
    ) -> PyResult<crate::iterator::MessageIterator> {
        warn_timeout_ms_deprecated(py, timeout_ms)?;
        let state_guard = self.state.lock().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Lock error: {}", e))
        })?;

        let state = state_guard.as_ref().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err("Not connected. Call connect() first.")
        })?;

        Ok(crate::iterator::MessageIterator::new(Arc::clone(&state.handoff), raw))
    }

    fn local_subscriptions(&self) -> Vec<String> {
        let state_guard = match self.state.lock() {
            Ok(g) => g,
            Err(_) => return vec![],
        };

        state_guard
            .as_ref()
            .map(|s| s.inner.subscription_keys())
            .unwrap_or_default()
    }

    fn subscriptions(&self, py: Python<'_>) -> PyResult<()> {
        let request = marketdata_core::WebSocketRequest::subscriptions();
        self.block_on_live(py, move |inner| async move { inner.send(request).await })
    }

    fn ping(&self, py: Python<'_>, state: Option<String>) -> PyResult<()> {
        let request = marketdata_core::WebSocketRequest::ping(state);
        self.block_on_live(py, move |inner| async move { inner.send(request).await })
    }

    fn measure_latency(&self, py: Python<'_>, timeout_ms: Option<u64>) -> PyResult<f64> {
        let (inner, runtime) = latency_handles(&self.state, &self.runtime)?;
        let timeout = timeout_ms.map(Duration::from_millis);
        block_on_detached(py, runtime, async move { inner.measure_latency(timeout).await })
            .map(|rtt| rtt.as_secs_f64() * 1000.0)
            .map_err(errors::to_py_err)
    }
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
    client: ProductClient,
}

impl StockWebSocketClient {
    #[allow(clippy::too_many_arguments)]
    fn new(
        auth: marketdata_core::AuthRequest,
        base_url: Option<String>,
        stock_version: marketdata_core::websocket::StockVersion,
        futopt_version: marketdata_core::websocket::FutOptVersion,
        reconnect_config: ReconnectConfig,
        health_check_config: HealthCheckConfig,
        tls: marketdata_core::TlsConfig,
        message_queue: MessageQueueSettings,
        auth_timeout: Duration,
    ) -> Self {
        Self {
            client: ProductClient::new(
                &STOCK,
                auth,
                base_url,
                stock_version,
                futopt_version,
                reconnect_config,
                health_check_config,
                tls,
                message_queue,
                auth_timeout,
            ),
        }
    }
}

/// The async methods and tests reach the shared state through this. The
/// `#[pymethods]` call `self.client.<method>` by name: a same-named method on
/// this type would otherwise call itself.
impl std::ops::Deref for StockWebSocketClient {
    type Target = ProductClient;

    fn deref(&self) -> &ProductClient {
        &self.client
    }
}

#[pymethods]
impl StockWebSocketClient {
    /// Register a callback for an event type
    ///
    /// Supported events:
    ///   - "message" / "data": Called with message dict when data received
    ///   - "raw_message": Called with the message as the str the server sent, no dict built
    ///   - "connect" / "connected": Called when the WebSocket opens, before authentication
    ///   - "authenticated": Called with the server's `authenticated` frame (dict: `event`, `data`)
    ///     when it accepts credentials, as in 2.x
    ///   - "unauthenticated": Called with the server's rejection frame (dict: `event` "error",
    ///     `code`, `data`) when it refuses credentials, as in 2.x
    ///   - "disconnect" / "disconnected" / "close": Called with (code, reason) when connection
    ///     closed; who closed it and whether a reconnect follows are in `last_disconnect`
    ///   - "reconnect" / "reconnecting": Called when reconnecting
    ///   - "error": Called with a single `err` argument (WebSocketError instance) when error occurs
    ///
    /// A callback `==` to one already registered for the event is not added
    /// again, as in 2.x: it runs once per event.
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

    /// Remove callbacks for an event type
    ///
    /// Args:
    ///     event: Event type string
    ///     listener: The callback to remove (compared with `==`); one that is
    ///         not registered is ignored. Omitted or None removes every
    ///         callback for `event`. 2.x documented `off(event, listener)`
    ///         but it raised `AttributeError`.
    #[pyo3(signature = (event, listener=None))]
    pub fn off(&self, event: &str, listener: Option<&Bound<'_, PyAny>>) -> PyResult<()> {
        self.callbacks.unregister(event, listener)
    }

    /// Connect to WebSocket server
    ///
    /// A background thread delivers the connection's events and messages in
    /// order: each message goes to the `message` callbacks if any are
    /// registered when it arrives, otherwise to `messages()` iterators.
    ///
    /// During an automatic reconnect it opens no connection of its own: it
    /// waits for that reconnect and returns once the connection is back and the
    /// subscriptions are re-sent, so a subscribe() afterwards follows them.
    /// Called from a callback, it holds up the callbacks until the reconnect
    /// ends.
    ///
    /// Raises:
    ///     MarketDataError: If connection fails
    ///     WebSocketError: Code 2011 if already connected or another connect is
    ///         in progress. While waiting on a reconnect: code 2010 if
    ///         disconnect() is called, code 3005 if the reconnect runs out of
    ///         attempts
    ///     AuthError: While waiting on a reconnect, if its credentials are
    ///         rejected
    pub fn connect(&self, py: Python<'_>) -> PyResult<()> {
        self.client.connect(py)
    }

    /// Disconnect from WebSocket server
    #[pyo3(signature = ())]
    pub fn disconnect(&self, py: Python<'_>) -> PyResult<()> {
        self.client.disconnect(py)
    }

    /// Check if currently connected
    #[pyo3(signature = ())]
    pub fn is_connected(&self, py: Python<'_>) -> bool {
        self.client.is_connected(py)
    }

    /// Check if client has been closed
    ///
    /// Returns True once disconnect() has closed a live connection or aborted
    /// a connect() in progress, and False again once a later connect()
    /// succeeds. This client builds a fresh core client per connect(), so it
    /// *can* be reused — see
    /// `test_connect_after_disconnect_succeeds`. Calling disconnect() on a
    /// client that was never connected closes nothing and leaves this False.
    #[pyo3(signature = ())]
    pub fn is_closed(&self, py: Python<'_>) -> bool {
        self.client.is_closed(py)
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
    /// The dict is the whole call, as in the legacy SDK's
    /// `def subscribe(self, params)`: `symbol` / `symbols` / `odd_lot` next
    /// to it, a key it does not take, or a non-boolean flag is a `TypeError`
    /// (#294). The flag may be spelled `oddLot`, `odd_lot` or
    /// `intradayOddLot`.
    #[pyo3(signature = (channel, symbol=None, *, symbols=None, odd_lot=None))]
    pub fn subscribe(
        &self,
        py: Python<'_>,
        channel: &Bound<'_, PyAny>,
        symbol: Option<&str>,
        symbols: Option<Vec<String>>,
        odd_lot: Option<bool>,
    ) -> PyResult<()> {
        let (channel_str, target_symbols, effective_odd_lot) =
            resolve_subscribe_args("subscribe", channel, symbol, symbols, odd_lot, ODD_LOT)?;

        // Checked before the connection, so an unknown channel is 1005 either way.
        let ch = channel_str
            .parse::<marketdata_core::Channel>()
            .map_err(errors::to_py_err)?;

        let sub = marketdata_core::StockSubscription::new(ch, target_symbols)
            .with_odd_lot(effective_odd_lot);
        self.client.block_on_live(py, move |inner| async move { inner.subscribe(sub).await })
    }

    /// Unsubscribe from a channel.
    ///
    /// By the server id from the `subscribed` message (legacy
    /// fugle-marketdata parity):
    ///
    /// ```python
    /// ws.stock.unsubscribe({"id": "abc123"})
    /// ws.stock.unsubscribe({"ids": ["abc123", "def456"]})
    /// ws.stock.unsubscribe("abc123")
    /// ws.stock.unsubscribe(ids=["abc123", "def456"])
    /// ```
    ///
    /// Or by the arguments given to `subscribe()`; `channel` is keyword-only:
    ///
    /// ```python
    /// ws.stock.unsubscribe({"channel": "trades", "symbol": "2330"})
    /// ws.stock.unsubscribe({"channel": "candles", "symbols": ["2330"], "oddLot": True})
    /// ws.stock.unsubscribe(channel="trades", symbol="2330")
    /// ws.stock.unsubscribe(channel="candles", symbols=["2330"], odd_lot=True)
    /// ```
    ///
    /// Naming a channel together with an id is 1005 `INVALID_PARAMETER`.
    #[pyo3(signature = (subscription_id=None, *, ids=None, channel=None, symbol=None, symbols=None, odd_lot=None))]
    #[allow(clippy::too_many_arguments)]
    pub fn unsubscribe(
        &self,
        py: Python<'_>,
        subscription_id: Option<&Bound<'_, PyAny>>,
        ids: Option<Vec<String>>,
        channel: Option<String>,
        symbol: Option<&str>,
        symbols: Option<Vec<String>>,
        odd_lot: Option<bool>,
    ) -> PyResult<()> {
        let args = ChannelArgs { channel, symbol, symbols, modifier: odd_lot };
        // Checked before the connection, as in `subscribe()`.
        let target_ids = match resolve_unsubscribe_target(subscription_id, ids, args, ODD_LOT)? {
            UnsubscribeTarget::Ids(ids) => ids,
            UnsubscribeTarget::Channel(channel, symbols, flag) => {
                let ch = channel
                    .parse::<marketdata_core::Channel>()
                    .map_err(errors::to_py_err)?;
                marketdata_core::StockSubscription::new(ch, symbols).with_odd_lot(flag).keys()
            }
        };

        self.client
            .block_on_live(py, move |inner| async move { inner.unsubscribe(target_ids).await })
    }

    /// The endpoint this client connects to, e.g.
    /// `wss://api.fugle.tw/marketdata/v1.0/stock/streaming`: `base_url`
    /// (host and path prefix), the version segment picked by `version`, then
    /// the product path. Readable before `connect()`.
    #[getter]
    pub fn url(&self) -> PyResult<String> {
        self.client.url()
    }

    /// Messages dropped because they arrived while `message_buffer` unread
    /// messages were already held (`message_overflow="drop_newest"`).
    ///
    /// Counted from the start of the current connection (every `connect()`
    /// or reconnect restarts it); after `disconnect()` it still reads the
    /// last connection's count. 0 before the first `connect()`.
    #[pyo3(signature = ())]
    pub fn messages_dropped_total(&self) -> u64 {
        self.client.messages_dropped_total()
    }

    /// The last disconnect: who closed the connection and whether a reconnect
    /// follows, as a `DisconnectInfo` (#293). None before the first one.
    ///
    /// Written before the `disconnect` callbacks run, so a callback reads the
    /// disconnect it is handling. Never cleared: `connect()`, a reconnect and
    /// `disconnect()` returning keep it, so it is a record of the last
    /// disconnect, not the connection state — ask `is_connected()` for that.
    /// A reconnect given up (error 3005) leaves it at the drop that started
    /// the reconnect. After a `messages()` iterator ends, this is where to
    /// find why. Should two connections overlap (a `disconnect()` from a
    /// callback, then `connect()` from another thread before that callback
    /// returns), it is the disconnect handed to the callbacks last.
    #[getter]
    pub fn last_disconnect(&self) -> Option<DisconnectInfo> {
        self.client.last_disconnect()
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
    ///
    /// With `raw=True` the iterator yields each message as the str the
    /// server sent instead of a dict, and no dict is built from it (#246).
    #[pyo3(signature = (timeout_ms=None, *, raw=false))]
    pub fn messages(
        &self,
        py: Python<'_>,
        timeout_ms: Option<u64>,
        raw: bool,
    ) -> PyResult<crate::iterator::MessageIterator> {
        self.client.messages(py, timeout_ms, raw)
    }

    /// Get the locally cached list of active subscription keys.
    ///
    /// Note: this is the *local* cache maintained by core's SubscriptionManager.
    /// To request the authoritative list from the server (matches the old
    /// fugle-marketdata SDK), call `subscriptions()` instead — the server's
    /// response will arrive via the registered `message` callback.
    #[pyo3(signature = ())]
    pub fn local_subscriptions(&self) -> Vec<String> {
        self.client.local_subscriptions()
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
        self.client.subscriptions(py)
    }

    /// Send a `ping` frame to the server (matches the old fugle-marketdata SDK).
    ///
    /// Fire and forget: the server's `pong` reply is delivered to the message
    /// handlers. To wait for it and get the round trip, use `measure_latency()`.
    ///
    /// Args:
    ///     state: Optional state string echoed back in the server's `pong` reply
    ///
    /// Raises:
    ///     RuntimeError: If not connected
    #[pyo3(signature = (state=None))]
    pub fn ping(&self, py: Python<'_>, state: Option<String>) -> PyResult<()> {
        self.client.ping(py, state)
    }

    /// Measure the round trip to the server: send a ping, wait for its pong,
    /// and return the time between the two in milliseconds.
    ///
    /// Works whether or not `probe_enabled` is set, and sends nothing in the
    /// background. Its pong is not delivered to the message handlers. Blocks
    /// with the GIL released.
    ///
    /// Args:
    ///     timeout_ms: How long to wait for the pong (default: 5000)
    ///
    /// Raises:
    ///     WebSocketError: Code 2010 (ClientClosed) if not connected
    ///     ConnectionError: Code 2001 if the connection closes before the pong
    ///     TimeoutError: Code 3001 if no pong arrives within `timeout_ms`
    ///     MarketDataError: Code 1005 for a `timeout_ms` of 0
    #[pyo3(signature = (timeout_ms=None))]
    pub fn measure_latency(&self, py: Python<'_>, timeout_ms: Option<u64>) -> PyResult<f64> {
        self.client.measure_latency(py, timeout_ms)
    }

    /// Connect to WebSocket server (async version)
    ///
    /// Returns an awaitable that completes when connection is established.
    /// Releases GIL during connection, enabling concurrent Python tasks.
    ///
    /// During an automatic reconnect it opens no connection of its own: it
    /// waits for that reconnect and returns once the connection is back and the
    /// subscriptions are re-sent, so a subscribe() afterwards follows them.
    /// Called from a callback, it holds up the callbacks until the reconnect
    /// ends.
    ///
    /// Raises:
    ///     MarketDataError: If connection fails
    ///     WebSocketError: Code 2011 if already connected or another connect is
    ///         in progress. While waiting on a reconnect: code 2010 if
    ///         disconnect() is called, code 3005 if the reconnect runs out of
    ///         attempts
    ///     AuthError: While waiting on a reconnect, if its credentials are
    ///         rejected
    ///
    /// Example:
    ///     ```python
    ///     await ws.stock.connect_async()
    ///     ```
    pub fn connect_async<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        connect_async_awaitable(py, self.async_connect()?)
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
        disconnect_async_awaitable(
            py,
            Arc::clone(&self.state),
            Arc::clone(&self.parked_readers),
            Arc::clone(&self.closed),
            Arc::clone(&self.pending),
        )
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
    #[pyo3(signature = (channel, symbol=None, *, symbols=None, odd_lot=None))]
    pub fn subscribe_async<'py>(
        &self,
        py: Python<'py>,
        channel: &Bound<'py, PyAny>,
        symbol: Option<&str>,
        symbols: Option<Vec<String>>,
        odd_lot: Option<bool>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let (channel_str, target_symbols, effective_odd_lot) =
            resolve_subscribe_args("subscribe_async", channel, symbol, symbols, odd_lot, ODD_LOT)?;
        let state_arc = Arc::clone(&self.state);

        future_into_py(py, async move {
            // Parsed before the connection check; raised on await.
            let ch = channel_str
                .parse::<marketdata_core::Channel>()
                .map_err(errors::to_py_err)?;
            let ws_client = subscribe_async_client(&state_arc)?;
            let sub = marketdata_core::StockSubscription::new(ch, target_symbols)
                .with_odd_lot(effective_odd_lot);
            ws_client.subscribe(sub).await.map_err(errors::to_py_err)
        })
    }

    /// Measure the round trip to the server (async version of
    /// `measure_latency()`); resolves to milliseconds.
    ///
    /// Args:
    ///     timeout_ms: How long to wait for the pong (default: 5000)
    #[pyo3(signature = (timeout_ms=None))]
    pub fn measure_latency_async<'py>(
        &self,
        py: Python<'py>,
        timeout_ms: Option<u64>,
    ) -> PyResult<Bound<'py, PyAny>> {
        measure_latency_async_awaitable(py, Arc::clone(&self.state), timeout_ms)
    }

    /// Async context manager support: enter
    ///
    /// Connects, then gives the client itself to `as`.
    ///
    /// Example:
    ///     ```python
    ///     async with ws.stock as client:
    ///         await client.subscribe_async("trades", "2330")
    ///     ```
    fn __aenter__<'py>(slf: PyRef<'py, Self>, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let connect = slf.async_connect()?;
        let client: Py<StockWebSocketClient> = slf.into();
        aenter_awaitable(py, connect, client.into_any())
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
#[pyclass]
pub struct FutOptWebSocketClient {
    client: ProductClient,
}

impl FutOptWebSocketClient {
    #[allow(clippy::too_many_arguments)]
    fn new(
        auth: marketdata_core::AuthRequest,
        base_url: Option<String>,
        stock_version: marketdata_core::websocket::StockVersion,
        futopt_version: marketdata_core::websocket::FutOptVersion,
        reconnect_config: ReconnectConfig,
        health_check_config: HealthCheckConfig,
        tls: marketdata_core::TlsConfig,
        message_queue: MessageQueueSettings,
        auth_timeout: Duration,
    ) -> Self {
        Self {
            client: ProductClient::new(
                &FUTOPT,
                auth,
                base_url,
                stock_version,
                futopt_version,
                reconnect_config,
                health_check_config,
                tls,
                message_queue,
                auth_timeout,
            ),
        }
    }
}

/// The async methods and tests reach the shared state through this. The
/// `#[pymethods]` call `self.client.<method>` by name: a same-named method on
/// this type would otherwise call itself.
impl std::ops::Deref for FutOptWebSocketClient {
    type Target = ProductClient;

    fn deref(&self) -> &ProductClient {
        &self.client
    }
}

#[pymethods]
impl FutOptWebSocketClient {
    /// Register a callback for an event type
    ///
    /// Supported events:
    ///   - "message" / "data": Called with message dict when data received
    ///   - "raw_message": Called with the message as the str the server sent, no dict built
    ///   - "connect" / "connected": Called when the WebSocket opens, before authentication
    ///   - "authenticated": Called with the server's `authenticated` frame (dict: `event`, `data`)
    ///     when it accepts credentials, as in 2.x
    ///   - "unauthenticated": Called with the server's rejection frame (dict: `event` "error",
    ///     `code`, `data`) when it refuses credentials, as in 2.x
    ///   - "disconnect" / "disconnected" / "close": Called with (code, reason) when connection
    ///     closed; who closed it and whether a reconnect follows are in `last_disconnect`
    ///   - "reconnect" / "reconnecting": Called when reconnecting
    ///   - "error": Called with a single `err` argument (WebSocketError instance) when error occurs
    ///
    /// A callback `==` to one already registered for the event is not added
    /// again, as in 2.x: it runs once per event.
    ///
    /// Args:
    ///     event: Event type string
    ///     callback: Python callable to invoke
    #[pyo3(signature = (event, callback))]
    pub fn on(&self, event: &str, callback: &Bound<'_, PyAny>) -> PyResult<()> {
        self.callbacks.register(event, callback)
    }

    /// Remove callbacks for an event type
    ///
    /// Args:
    ///     event: Event type string
    ///     listener: The callback to remove (compared with `==`); one that is
    ///         not registered is ignored. Omitted or None removes every
    ///         callback for `event`. 2.x documented `off(event, listener)`
    ///         but it raised `AttributeError`.
    #[pyo3(signature = (event, listener=None))]
    pub fn off(&self, event: &str, listener: Option<&Bound<'_, PyAny>>) -> PyResult<()> {
        self.callbacks.unregister(event, listener)
    }

    /// Connect to WebSocket server
    ///
    /// During an automatic reconnect it opens no connection of its own: it
    /// waits for that reconnect and returns once the connection is back and the
    /// subscriptions are re-sent, so a subscribe() afterwards follows them.
    /// Called from a callback, it holds up the callbacks until the reconnect
    /// ends.
    ///
    /// Raises:
    ///     MarketDataError: If connection fails
    ///     WebSocketError: Code 2011 if already connected or another connect is
    ///         in progress. While waiting on a reconnect: code 2010 if
    ///         disconnect() is called, code 3005 if the reconnect runs out of
    ///         attempts
    ///     AuthError: While waiting on a reconnect, if its credentials are
    ///         rejected
    #[pyo3(signature = ())]
    pub fn connect(&self, py: Python<'_>) -> PyResult<()> {
        self.client.connect(py)
    }

    /// Disconnect from WebSocket server
    #[pyo3(signature = ())]
    pub fn disconnect(&self, py: Python<'_>) -> PyResult<()> {
        self.client.disconnect(py)
    }

    /// Check if currently connected
    #[pyo3(signature = ())]
    pub fn is_connected(&self, py: Python<'_>) -> bool {
        self.client.is_connected(py)
    }

    /// Check if client has been closed
    ///
    /// Returns True once disconnect() has closed a live connection or aborted
    /// a connect() in progress, and False again once a later connect()
    /// succeeds. This client builds a fresh core client per connect(), so it
    /// *can* be reused — see
    /// `test_connect_after_disconnect_succeeds`. Calling disconnect() on a
    /// client that was never connected closes nothing and leaves this False.
    #[pyo3(signature = ())]
    pub fn is_closed(&self, py: Python<'_>) -> bool {
        self.client.is_closed(py)
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
    #[pyo3(signature = (channel, symbol=None, *, symbols=None, after_hours=None))]
    pub fn subscribe(
        &self,
        py: Python<'_>,
        channel: &Bound<'_, PyAny>,
        symbol: Option<&str>,
        symbols: Option<Vec<String>>,
        after_hours: Option<bool>,
    ) -> PyResult<()> {
        let (channel_str, target_symbols, effective_after_hours) =
            resolve_subscribe_args("subscribe", channel, symbol, symbols, after_hours, AFTER_HOURS)?;

        // Checked before the connection; FutOpt has no `indices` channel.
        let ch = channel_str
            .parse::<marketdata_core::FutOptChannel>()
            .map_err(errors::to_py_err)?;

        let sub = marketdata_core::FutOptSubscription::new(ch, target_symbols)
            .with_after_hours(effective_after_hours);
        self.client.block_on_live(py, move |inner| async move { inner.subscribe_futopt(sub).await })
    }

    /// Unsubscribe from a channel.
    ///
    /// By the server id: dict shape (`{"id": "..."}` / `{"ids": [...]}`) or
    /// positional/kwargs shape (`subscription_id` / `ids=`). Or by the
    /// arguments given to `subscribe()`: `{"channel", "symbol" | "symbols",
    /// "afterHours"?}`, or `channel=` with `symbol=` / `symbols=` and
    /// `after_hours=`. Naming a channel together with an id is 1005
    /// `INVALID_PARAMETER`.
    #[pyo3(signature = (subscription_id=None, *, ids=None, channel=None, symbol=None, symbols=None, after_hours=None))]
    #[allow(clippy::too_many_arguments)]
    pub fn unsubscribe(
        &self,
        py: Python<'_>,
        subscription_id: Option<&Bound<'_, PyAny>>,
        ids: Option<Vec<String>>,
        channel: Option<String>,
        symbol: Option<&str>,
        symbols: Option<Vec<String>>,
        after_hours: Option<bool>,
    ) -> PyResult<()> {
        let args = ChannelArgs { channel, symbol, symbols, modifier: after_hours };
        // Checked before the connection, as in `subscribe()`.
        let target_ids = match resolve_unsubscribe_target(subscription_id, ids, args, AFTER_HOURS)? {
            UnsubscribeTarget::Ids(ids) => ids,
            UnsubscribeTarget::Channel(channel, symbols, flag) => {
                let ch = channel
                    .parse::<marketdata_core::FutOptChannel>()
                    .map_err(errors::to_py_err)?;
                marketdata_core::FutOptSubscription::new(ch, symbols).with_after_hours(flag).keys()
            }
        };

        self.client
            .block_on_live(py, move |inner| async move { inner.unsubscribe(target_ids).await })
    }

    /// The endpoint this client connects to, e.g.
    /// `wss://api.fugle.tw/marketdata/v1.1/futopt/streaming`: `base_url`
    /// (host and path prefix), the version segment picked by `version`, then
    /// the product path. Readable before `connect()`.
    #[getter]
    pub fn url(&self) -> PyResult<String> {
        self.client.url()
    }

    /// Messages dropped because they arrived while `message_buffer` unread
    /// messages were already held (`message_overflow="drop_newest"`).
    ///
    /// Counted from the start of the current connection (every `connect()`
    /// or reconnect restarts it); after `disconnect()` it still reads the
    /// last connection's count. 0 before the first `connect()`.
    #[pyo3(signature = ())]
    pub fn messages_dropped_total(&self) -> u64 {
        self.client.messages_dropped_total()
    }

    /// The last disconnect: who closed the connection and whether a reconnect
    /// follows, as a `DisconnectInfo` (#293). None before the first one.
    ///
    /// Written before the `disconnect` callbacks run, so a callback reads the
    /// disconnect it is handling. Never cleared: `connect()`, a reconnect and
    /// `disconnect()` returning keep it, so it is a record of the last
    /// disconnect, not the connection state — ask `is_connected()` for that.
    /// A reconnect given up (error 3005) leaves it at the drop that started
    /// the reconnect. After a `messages()` iterator ends, this is where to
    /// find why. Should two connections overlap (a `disconnect()` from a
    /// callback, then `connect()` from another thread before that callback
    /// returns), it is the disconnect handed to the callbacks last.
    #[getter]
    pub fn last_disconnect(&self) -> Option<DisconnectInfo> {
        self.client.last_disconnect()
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
    ///
    /// With `raw=True` the iterator yields each message as the str the
    /// server sent instead of a dict, and no dict is built from it (#246).
    #[pyo3(signature = (timeout_ms=None, *, raw=false))]
    pub fn messages(
        &self,
        py: Python<'_>,
        timeout_ms: Option<u64>,
        raw: bool,
    ) -> PyResult<crate::iterator::MessageIterator> {
        self.client.messages(py, timeout_ms, raw)
    }

    /// Get the locally cached list of active subscription keys.
    ///
    /// Note: this is the *local* cache maintained by core's SubscriptionManager.
    /// To request the authoritative list from the server (matches the old
    /// fugle-marketdata SDK), call `subscriptions()` instead — the server's
    /// response will arrive via the registered `message` callback.
    #[pyo3(signature = ())]
    pub fn local_subscriptions(&self) -> Vec<String> {
        self.client.local_subscriptions()
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
        self.client.subscriptions(py)
    }

    /// Send a `ping` frame to the server (matches the old fugle-marketdata SDK).
    ///
    /// Fire and forget: the server's `pong` reply is delivered to the message
    /// handlers. To wait for it and get the round trip, use `measure_latency()`.
    ///
    /// Args:
    ///     state: Optional state string echoed back in the server's `pong` reply
    ///
    /// Raises:
    ///     RuntimeError: If not connected
    #[pyo3(signature = (state=None))]
    pub fn ping(&self, py: Python<'_>, state: Option<String>) -> PyResult<()> {
        self.client.ping(py, state)
    }

    /// Measure the round trip to the server: send a ping, wait for its pong,
    /// and return the time between the two in milliseconds.
    ///
    /// Works whether or not `probe_enabled` is set, and sends nothing in the
    /// background. Its pong is not delivered to the message handlers. Blocks
    /// with the GIL released.
    ///
    /// Args:
    ///     timeout_ms: How long to wait for the pong (default: 5000)
    ///
    /// Raises:
    ///     WebSocketError: Code 2010 (ClientClosed) if not connected
    ///     ConnectionError: Code 2001 if the connection closes before the pong
    ///     TimeoutError: Code 3001 if no pong arrives within `timeout_ms`
    ///     MarketDataError: Code 1005 for a `timeout_ms` of 0
    #[pyo3(signature = (timeout_ms=None))]
    pub fn measure_latency(&self, py: Python<'_>, timeout_ms: Option<u64>) -> PyResult<f64> {
        self.client.measure_latency(py, timeout_ms)
    }

    /// Connect to WebSocket server (async version)
    ///
    /// Returns an awaitable that completes when connection is established.
    /// Releases GIL during connection, enabling concurrent Python tasks.
    ///
    /// During an automatic reconnect it opens no connection of its own: it
    /// waits for that reconnect and returns once the connection is back and the
    /// subscriptions are re-sent, so a subscribe() afterwards follows them.
    /// Called from a callback, it holds up the callbacks until the reconnect
    /// ends.
    ///
    /// Raises:
    ///     MarketDataError: If connection fails
    ///     WebSocketError: Code 2011 if already connected or another connect is
    ///         in progress. While waiting on a reconnect: code 2010 if
    ///         disconnect() is called, code 3005 if the reconnect runs out of
    ///         attempts
    ///     AuthError: While waiting on a reconnect, if its credentials are
    ///         rejected
    ///
    /// Example:
    ///     ```python
    ///     await ws.futopt.connect_async()
    ///     ```
    pub fn connect_async<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        connect_async_awaitable(py, self.async_connect()?)
    }

    /// Disconnect from WebSocket server (async version)
    ///
    /// Returns an awaitable that completes when disconnection finishes.
    ///
    /// Example:
    ///     ```python
    ///     await ws.futopt.disconnect_async()
    ///     ```
    pub fn disconnect_async<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        disconnect_async_awaitable(
            py,
            Arc::clone(&self.state),
            Arc::clone(&self.parked_readers),
            Arc::clone(&self.closed),
            Arc::clone(&self.pending),
        )
    }

    /// Subscribe to a channel (async version)
    ///
    /// Args:
    ///     channel: Channel name (trades, candles, books, aggregates)
    ///     symbol: FutOpt symbol (e.g., "TXFC4")
    ///     after_hours: Whether to subscribe to after-hours data (default: False)
    ///
    /// Returns:
    ///     Awaitable that completes when subscription is confirmed
    ///
    /// Example:
    ///     ```python
    ///     await ws.futopt.subscribe_async("trades", "TXFC4")
    ///     await ws.futopt.subscribe_async({"channel": "trades", "symbol": "TXFC4"})
    ///     ```
    #[pyo3(signature = (channel, symbol=None, *, symbols=None, after_hours=None))]
    pub fn subscribe_async<'py>(
        &self,
        py: Python<'py>,
        channel: &Bound<'py, PyAny>,
        symbol: Option<&str>,
        symbols: Option<Vec<String>>,
        after_hours: Option<bool>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let (channel_str, target_symbols, effective_after_hours) =
            resolve_subscribe_args("subscribe_async", channel, symbol, symbols, after_hours, AFTER_HOURS)?;
        let state_arc = Arc::clone(&self.state);

        future_into_py(py, async move {
            // Parsed before the connection check; raised on await.
            let ch = channel_str
                .parse::<marketdata_core::FutOptChannel>()
                .map_err(errors::to_py_err)?;
            let ws_client = subscribe_async_client(&state_arc)?;
            let sub = marketdata_core::FutOptSubscription::new(ch, target_symbols)
                .with_after_hours(effective_after_hours);
            ws_client.subscribe_futopt(sub).await.map_err(errors::to_py_err)
        })
    }

    /// Measure the round trip to the server (async version of
    /// `measure_latency()`); resolves to milliseconds.
    ///
    /// Args:
    ///     timeout_ms: How long to wait for the pong (default: 5000)
    #[pyo3(signature = (timeout_ms=None))]
    pub fn measure_latency_async<'py>(
        &self,
        py: Python<'py>,
        timeout_ms: Option<u64>,
    ) -> PyResult<Bound<'py, PyAny>> {
        measure_latency_async_awaitable(py, Arc::clone(&self.state), timeout_ms)
    }

    /// Async context manager support: enter
    ///
    /// Connects, then gives the client itself to `as`.
    ///
    /// Example:
    ///     ```python
    ///     async with ws.futopt as client:
    ///         await client.subscribe_async("trades", "TXFC4")
    ///     ```
    fn __aenter__<'py>(slf: PyRef<'py, Self>, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let connect = slf.async_connect()?;
        let client: Py<FutOptWebSocketClient> = slf.into();
        aenter_awaitable(py, connect, client.into_any())
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
            &marketdata_core::AuthRequest::with_api_key("k"),
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
            &marketdata_core::AuthRequest::with_api_key("k"),
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
            &marketdata_core::AuthRequest::with_api_key("k"),
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
        let cfg = build_stream_config(&marketdata_core::AuthRequest::with_api_key("k"), None, WsProduct::FutOpt, Default::default(), Default::default())
            .unwrap();
        assert_eq!(cfg.url, marketdata_core::urls::FUTOPT_WS);
    }

    #[test]
    fn test_websocket_client_creation_with_api_key() {
        // WebSocketClient::new requires Python bindings, test the internal child client instead
        let client = StockWebSocketClient::new(
            marketdata_core::AuthRequest::with_api_key("test-key"),
            None,
            Default::default(),
            Default::default(),
            ReconnectConfig::default(),
            HealthCheckConfig::default(),
            marketdata_core::TlsConfig::default(),
            MessageQueueSettings::parse(None, None).unwrap(),
            marketdata_core::websocket::DEFAULT_AUTH_TIMEOUT,
        );
        let state = client.state.lock().unwrap();
        assert!(state.is_none());
    }

    #[test]
    fn test_stock_websocket_client_creation() {
        let client = StockWebSocketClient::new(
            marketdata_core::AuthRequest::with_api_key("test-key"),
            None,
            Default::default(),
            Default::default(),
            ReconnectConfig::default(),
            HealthCheckConfig::default(),
            marketdata_core::TlsConfig::default(),
            MessageQueueSettings::parse(None, None).unwrap(),
            marketdata_core::websocket::DEFAULT_AUTH_TIMEOUT,
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
            marketdata_core::AuthRequest::with_api_key("test-key"),
            None,
            Default::default(),
            Default::default(),
            ReconnectConfig::default(),
            HealthCheckConfig::default(),
            marketdata_core::TlsConfig::default(),
            MessageQueueSettings::parse(None, None).unwrap(),
            marketdata_core::websocket::DEFAULT_AUTH_TIMEOUT,
        );
        let state = client.state.lock().unwrap();
        assert!(state.is_none());
    }
}
