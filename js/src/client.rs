//! REST client wrapper for JavaScript
//!
//! This module provides the JavaScript-facing RestClient that wraps
//! marketdata-core::RestClient for NAPI-RS bindings.

use crate::errors::{BuildError, Settled};
use crate::options::{ExtraArg, JsVal};
use crate::websocket::RestClientOptions;
use napi_derive::napi;
use napi::bindgen_prelude::{FromNapiValue, TypeName, ValueType};
use napi::sys;
use marketdata_core::rest::params::EndpointSpec;
use serde_json::{Map, Value};

// ---------------------------------------------------------------------------
// Legacy fugle-marketdata-node compatibility helpers
//
// The legacy `@fugle/marketdata` SDK calls REST methods with a single object
// argument (e.g. `stock.intraday.quote({ symbol: '2330', type: 'oddlot' })`).
// Every REST method here accepts that object as its first argument: the keys
// are checked against `core::rest::params` and sent under the API names
// through `RestClient::get_json`; a string first argument keeps the
// positional form, which goes through the typed builders.
// ---------------------------------------------------------------------------

/// The first argument of a REST method: positional, or the legacy params object.
///
/// A value of another type does not fail the conversion: it is kept as
/// `Invalid` and rejects the call's promise with a `TypeError`, like every
/// other argument error of a REST method (#294). Only a getter or Proxy trap
/// that throws while the value is read throws, with its own error.
pub enum RestArg {
    Positional(String),
    Params(Map<String, Value>),
    /// Neither: the type as a message names it.
    Invalid(String),
}

impl RestArg {
    /// Unwrap a required first argument (`undefined` / `null` arrive as
    /// `None`); a missing one rejects with a `TypeError`, like the other
    /// argument errors of a REST method (#294).
    fn required(method: &str, arg: Option<Self>, name: &str) -> Result<Self, Settled> {
        arg.ok_or_else(|| {
            let short = method.rsplit('.').next().unwrap_or(method);
            Settled::type_error(format!(
                "`{method}`: {name} is required, as a string or a params object, e.g. \
                 {short}({{ {name}: ... }})"
            ))
        })
    }
}

impl TypeName for RestArg {
    fn type_name() -> &'static str {
        "string | object"
    }

    fn value_type() -> ValueType {
        ValueType::Unknown
    }
}

impl FromNapiValue for RestArg {
    unsafe fn from_napi_value(env: sys::napi_env, value: sys::napi_value) -> napi::Result<Self> {
        Ok(match unsafe { crate::options::read(env, value, 0, false)? } {
            JsVal::String(s) => Self::Positional(s),
            JsVal::Object(_) => match unsafe { Map::from_napi_value(env, value) } {
                Ok(params) => Self::Params(params),
                // A getter that threw: its own error, thrown now, as for a
                // constructor option.
                Err(err) if exception_pending(env) => return Err(err),
                Err(err) => Self::Invalid(format!("an object that cannot be sent ({})", err.reason)),
            },
            other => Self::Invalid(other.describe()),
        })
    }
}

fn exception_pending(env: sys::napi_env) -> bool {
    let mut pending = false;
    let status = unsafe { sys::napi_is_exception_pending(env, &mut pending) };
    status == sys::Status::napi_ok && pending
}

/// A positional REST argument after the first. Converting does not fail on
/// a value of the wrong type; the method checks the type with [`pos_string`] / [`pos_bool`] / [`pos_u32`],
/// so a wrong one rejects the promise with a `TypeError` instead of
/// throwing napi's error, and an integer is not coerced (`-1`, `1.5`, `NaN`)
/// (#294). `undefined` / `null` arrive as `None`.
pub struct PosArg(JsVal);

impl TypeName for PosArg {
    fn type_name() -> &'static str {
        "unknown"
    }

    fn value_type() -> ValueType {
        ValueType::Unknown
    }
}

impl FromNapiValue for PosArg {
    unsafe fn from_napi_value(env: sys::napi_env, value: sys::napi_value) -> napi::Result<Self> {
        Ok(Self(unsafe { crate::options::read(env, value, 0, false)? }))
    }
}

fn wrong_type(method: &str, name: &str, expected: &str, got: &JsVal) -> Settled {
    Settled::type_error(format!("`{method}`: {name} must be {expected}, got {}", got.describe()))
}

fn pos_string(method: &str, name: &str, arg: Option<PosArg>) -> Result<Option<String>, Settled> {
    match arg {
        None => Ok(None),
        Some(PosArg(JsVal::String(s))) => Ok(Some(s)),
        Some(PosArg(other)) => Err(wrong_type(method, name, "a string", &other)),
    }
}

fn pos_bool(method: &str, name: &str, arg: Option<PosArg>) -> Result<Option<bool>, Settled> {
    match arg {
        None => Ok(None),
        Some(PosArg(JsVal::Bool(b))) => Ok(Some(b)),
        Some(PosArg(other)) => Err(wrong_type(method, name, "a boolean", &other)),
    }
}

fn pos_u32(method: &str, name: &str, arg: Option<PosArg>) -> Result<Option<u32>, Settled> {
    match arg {
        None => Ok(None),
        Some(PosArg(JsVal::Number(n))) if n.is_finite() && n.fract() == 0.0 && n >= 0.0 && n <= f64::from(u32::MAX) => {
            Ok(Some(n as u32))
        }
        Some(PosArg(other)) => Err(wrong_type(method, name, "a non-negative integer", &other)),
    }
}

/// The value of a `pos_*` / [`unused_args`] check, or return its rejection.
macro_rules! take {
    ($check:expr) => {
        match $check {
            Ok(value) => value,
            Err(rejected) => return Ok(rejected),
        }
    };
}

/// The keys `spec` accepts in the object form, path param first.
fn accepted_keys(spec: &EndpointSpec) -> String {
    spec.path_param
        .into_iter()
        .chain(spec.path_param_aliases.iter().copied())
        .chain(spec.names())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Refuse the arguments a call does not use (#294), as a `TypeError`
/// rejection: with the params object first, any further argument (`later`
/// names the declared ones and whether each is set); with a positional first
/// argument, one past the last declared (`extra`).
///
/// `method` is the JS name (`stock.intraday.candles`), `path` its row in
/// `core::rest::params`, `first` the name of the first parameter.
fn unused_args(
    method: &str,
    path: &[&str],
    first: (&str, &Option<RestArg>),
    later: &[(&str, bool)],
    extra: Option<ExtraArg>,
) -> Option<Settled> {
    if let Some(RestArg::Invalid(kind)) = first.1 {
        return Some(Settled::type_error(format!(
            "`{method}`: {} must be a string or a params object, got {kind}",
            first.0
        )));
    }
    let keys = || {
        let spec = EndpointSpec::for_path(path)
            .unwrap_or_else(|| panic!("{} has no entry in core::rest::params", path.join("/")));
        accepted_keys(spec)
    };
    if matches!(first.1, Some(RestArg::Params(_))) {
        let further = match (later.iter().find(|(_, set)| *set), extra) {
            (Some((name, _)), _) => format!("`{name}` as a further argument"),
            (None, Some(ExtraArg(kind))) => format!("a further {kind} argument"),
            (None, None) => return None,
        };
        return Some(Settled::type_error(format!(
            "`{method}` got a params object and {further}; the object form takes no other \
             arguments. Put every parameter in the object; accepted keys: {}",
            keys()
        )));
    }
    let ExtraArg(kind) = extra?;
    let names: Vec<&str> = std::iter::once(first.0).chain(later.iter().map(|(name, _)| *name)).collect();
    let short = method.rsplit('.').next().unwrap_or(method);
    Some(Settled::type_error(format!(
        "`{method}` takes at most {} positional argument{} ({}), got a further {kind} one. Pass \
         other parameters in the params object instead, e.g. {short}({{ {}: ..., ... }}); accepted \
         keys: {keys}",
        names.len(),
        if names.len() == 1 { "" } else { "s" },
        names.join(", "),
        first.0,
        keys = keys(),
    )))
}

/// Send the object form: the path param (`symbol` / `market`, or `product`
/// where the table allows it) becomes the last path segment and every other
/// key is checked against `core::rest::params` before it is sent.
///
/// An unknown key is refused with the accepted keys in the message; the
/// server would ignore it or answer 400 depending on the endpoint (see
/// `core::rest::params`, #164). Values are sent as given; the server checks
/// those.
async fn get_with_params(
    client: &marketdata_core::RestClient,
    path: &[&str],
    mut params: Map<String, Value>,
) -> napi::Result<Settled> {
    let spec = EndpointSpec::for_path(path)
        .unwrap_or_else(|| panic!("{} has no entry in core::rest::params", path.join("/")));
    let mut segments: Vec<String> = path.iter().map(|s| s.to_string()).collect();
    let mut path_key_used = None;
    if let Some(key) = spec.path_param {
        let given = params.keys().find(|k| spec.is_path_param(k)).cloned();
        match given.as_ref().and_then(|k| params.remove(k)).as_ref().and_then(scalar_to_string) {
            Some(value) => {
                segments.push(value);
                path_key_used = given;
            }
            None => return Ok(invalid(key, format!("`{key}` is required and must be a string"))),
        }
    }
    let query = match query_pairs(spec, path_key_used.as_deref(), params) {
        Ok(query) => query,
        Err(err) => return Ok(err),
    };

    let client = client.clone();
    let result = tokio::task::spawn_blocking(move || {
        let segments: Vec<&str> = segments.iter().map(String::as_str).collect();
        client.get_json(&segments, &query)
    })
    .await
    .map_err(|e| napi::Error::from_reason(format!("Task error: {}", e)))?;

    Ok(Settled::from(result))
}

/// A rejection carrying the unified error fields (`code` 1005, `sourceKind`
/// `client`), built on the JS thread when the promise settles.
fn invalid(name: &str, reason: String) -> Settled {
    Settled::from(Err(marketdata_core::MarketDataError::InvalidParameter {
        name: name.to_string(),
        reason,
    }))
}

/// Resolve each key through the endpoint's table and flatten the values into
/// query pairs the way the legacy SDK's `query-string` does: `null` entries
/// are skipped and an array repeats its key per element.
///
/// A key may be the API name (`isTrial`), the snake_case form (`is_trial`) or
/// a listed alias (`oddLot`); it is sent under the API name. The flag forms
/// (`odd_lot`, `after_hours`) take a boolean and send the table's literal.
fn query_pairs(
    spec: &EndpointSpec,
    path_key_used: Option<&str>,
    params: Map<String, Value>,
) -> Result<Vec<(String, String)>, Settled> {
    let mut pairs = Vec::new();
    let mut given: Vec<(&'static str, String)> = Vec::new();
    for (key, value) in params {
        if let (true, Some(used)) = (spec.is_path_param(&key), path_key_used) {
            return Err(invalid(&key, format!("`{key}` and `{used}` both name the path param; give one")));
        }
        let Some(resolved) = spec.resolve(&key) else {
            let hint = match spec.suggest(&key) {
                Some(name) => format!("did you mean `{name}`? "),
                None => String::new(),
            };
            return Err(invalid(
                &key,
                format!(
                    "`{}` does not accept `{key}`; {hint}accepted keys: {}",
                    spec.path.join("."),
                    accepted_keys(spec)
                ),
            ));
        };
        let name = resolved.spec.name;
        if let Some((_, earlier)) = given.iter().find(|(n, _)| *n == name) {
            return Err(invalid(&key, format!("`{key}` is `{name}`, already given as `{earlier}`")));
        }
        given.push((name, key.clone()));

        if resolved.as_flag {
            let flag = resolved.spec.flag.expect("as_flag implies a flag value");
            match value {
                Value::Bool(true) => pairs.push((name.to_string(), flag.to_string())),
                Value::Bool(false) | Value::Null => {}
                _ => return Err(invalid(&key, format!("`{key}` must be a boolean"))),
            }
            continue;
        }
        let items = match value {
            Value::Array(items) => items,
            other => vec![other],
        };
        for item in items {
            if item.is_null() {
                continue;
            }
            let Some(item) = scalar_to_string(&item) else {
                return Err(invalid(
                    &key,
                    format!("`{key}` must be a string, number, boolean or an array of them"),
                ));
            };
            pairs.push((name.to_string(), item));
        }
    }
    Ok(pairs)
}

fn scalar_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// REST client for Fugle market data API (JavaScript wrapper)
///
/// # JavaScript Usage
///
/// ```javascript
/// const { RestClient } = require('@fugle/marketdata');
///
/// // Create client with API key
/// const client = new RestClient('your-api-key');
///
/// // Access stock market data
/// const quote = client.stock.intraday.quote('2330');
/// console.log(quote.lastPrice, quote.symbol);
///
/// // Access futures/options market data
/// const futoptQuote = client.futopt.intraday.quote('TXFC4');
/// console.log(futoptQuote.lastPrice, futoptQuote.symbol);
/// ```
#[napi]
pub struct RestClient {
    inner: marketdata_core::RestClient,
}

impl RestClient {
    /// Build the client; [`RestClient::new`] turns the error into a JS one.
    fn from_options(options: RestClientOptions) -> Result<Self, BuildError> {
        // Core requires exactly one non-blank credential (ConfigError, 1004).
        let auth = marketdata_core::Auth::from_credentials(
            options.api_key,
            options.bearer_token,
            options.sdk_token,
        )?;

        // Build TLS config from optional kwargs. When both are default we
        // use `with_tls(TlsConfig::default())` — same path `new(auth)` takes
        // internally, so behaviour is preserved for consumers not touching TLS.
        let tls = marketdata_core::TlsConfig {
            root_cert_pem: options.tls_root_cert_pem.map(|arr| arr.to_vec()),
            accept_invalid_certs: options.tls_accept_invalid_certs.unwrap_or(false),
        };

        let mut inner = marketdata_core::RestClient::with_tls(auth, tls)?;
        if let Some(url) = options.base_url {
            // `tryBaseUrl` semantics: a JS caller expects a bad option to throw
            // from `new RestClient(...)`, matching the official SDK's TypeError,
            // not to surface later from an unrelated request.
            inner = inner.try_base_url_worded(&url, crate::websocket::BASE_URL_WORDING)?;
        }

        Ok(Self { inner })
    }
}

#[napi]
impl RestClient {
    /// Create a new REST client with options
    ///
    /// @param options - Client configuration options
    /// @throws {Error} If validation fails (zero or multiple auth methods)
    ///
    /// @example
    /// ```javascript
    /// const { RestClient } = require('@fugle/marketdata');
    ///
    /// // API key auth
    /// const client = new RestClient({ apiKey: 'your-key' });
    ///
    /// // Bearer token auth with custom base URL
    /// const client = new RestClient({
    ///   bearerToken: 'token',
    ///   baseUrl: 'https://custom.api'
    /// });
    /// ```
    #[napi(constructor, ts_args_type = "options: RestClientOptions")]
    pub fn new(env: napi::Env, options: crate::options::Checked<RestClientOptions>) -> napi::Result<Self> {
        // Checked against `options::REST_CLIENT_FIELDS` first (#294).
        Self::from_options(options.0).map_err(|e| e.into_napi(&env))
    }

    /// Replace the credential later requests send, with any of the three
    /// kinds. Product clients already taken from this one (`client.stock`,
    /// `client.stock.intraday`, ...) send it too; a request already sent
    /// keeps the credential it was sent with.
    ///
    /// @param credentials - Exactly one of apiKey, bearerToken, sdkToken
    /// @throws {TypeError} If `credentials` is not an object of those keys
    ///   holding strings
    /// @throws {Error} code 1004 unless exactly one non-empty credential is
    ///   given, or if it cannot be sent in an HTTP header; the current
    ///   credential is then kept
    ///
    /// @example
    /// ```javascript
    /// client.setCredentials({ sdkToken: newToken });
    /// ```
    #[napi(ts_args_type = "credentials: Credentials")]
    pub fn set_credentials(
        &self,
        env: napi::Env,
        credentials: crate::options::Checked<crate::websocket::Credentials>,
    ) -> napi::Result<()> {
        credentials
            .0
            .into_auth()
            .and_then(|auth| self.inner.set_credentials(auth))
            .map_err(|e| crate::errors::to_napi_error(&env, e))
    }

    /// The prefix every request from this client is built on, fully resolved —
    /// host, path prefix and version segment. Endpoints are appended to it.
    ///
    /// The version segment is chosen by the SDK rather than written by the
    /// caller, so this is the only way to see what a client resolved to.
    #[napi(getter)]
    pub fn base_url(&self) -> String {
        self.inner.resolved_base_url().to_string()
    }

    /// Get the stock client for accessing stock market data
    #[napi(getter)]
    pub fn stock(&self) -> StockClient {
        StockClient {
            inner: self.inner.clone(),
        }
    }

    /// Get the FutOpt client for accessing futures/options market data
    #[napi(getter)]
    pub fn futopt(&self) -> FutOptClient {
        FutOptClient {
            inner: self.inner.clone(),
        }
    }
}

/// Stock market data client
#[napi]
pub struct StockClient {
    inner: marketdata_core::RestClient,
}

#[napi]
impl StockClient {
    /// Get intraday client for real-time stock data
    #[napi(getter)]
    pub fn intraday(&self) -> StockIntradayClient {
        StockIntradayClient {
            inner: self.inner.clone(),
        }
    }

    /// Get historical client for historical stock data
    #[napi(getter)]
    pub fn historical(&self) -> StockHistoricalClient {
        StockHistoricalClient {
            inner: self.inner.clone(),
        }
    }

    /// Get snapshot client for market-wide data
    #[napi(getter)]
    pub fn snapshot(&self) -> StockSnapshotClient {
        StockSnapshotClient {
            inner: self.inner.clone(),
        }
    }

    /// Get technical indicators client
    #[napi(getter)]
    pub fn technical(&self) -> StockTechnicalClient {
        StockTechnicalClient {
            inner: self.inner.clone(),
        }
    }

    /// Get corporate actions client
    #[napi(getter)]
    pub fn corporate_actions(&self) -> StockCorporateActionsClient {
        StockCorporateActionsClient {
            inner: self.inner.clone(),
        }
    }

    /// Get ownership client (ETF holdings, institutional trades, director holdings, TDCC distribution)
    #[napi(getter)]
    pub fn ownership(&self) -> StockOwnershipClient {
        StockOwnershipClient {
            inner: self.inner.clone(),
        }
    }

    /// The fully resolved request prefix for this product client, product
    /// segment included (`…/v1.0/stock`), for 1.x compatibility: it is what
    /// `@fugle/marketdata` 1.x returned (#307). `RestClient.baseUrl` is the prefix without it.
    #[napi(getter)]
    pub fn base_url(&self) -> String {
        self.inner.stock().resolved_base_url()
    }
}

/// `stock.ownership.etfHoldings` params (object form, matching the official SDK).
/// Any other key is refused.
// The TypeScript type only: the call reads the object through
// `core::rest::params` (#294).
#[napi(object)]
pub struct EtfHoldingsParams {
    pub symbol: String,
    pub from: Option<String>,
    pub to: Option<String>,
    pub sort: Option<String>,
}

/// `stock.ownership.institutionalTrades` params (object form, matching the official SDK).
/// Any other key is refused.
// The TypeScript type only: the call reads the object through
// `core::rest::params` (#294).
#[napi(object)]
pub struct InstitutionalTradesParams {
    pub symbol: String,
    pub from: Option<String>,
    pub to: Option<String>,
    pub sort: Option<String>,
}

/// `stock.ownership.directorHoldings` params (object form, matching the official SDK).
/// Any other key is refused.
// The TypeScript type only: the call reads the object through
// `core::rest::params` (#294).
#[napi(object)]
pub struct DirectorHoldingsParams {
    pub symbol: String,
    pub from: Option<String>,
    pub to: Option<String>,
    pub sort: Option<String>,
}

/// `stock.ownership.tdccDistribution` params (object form, matching the official SDK).
/// Any other key is refused.
// The TypeScript type only: the call reads the object through
// `core::rest::params` (#294).
#[napi(object)]
pub struct TdccDistributionParams {
    pub symbol: String,
    pub from: Option<String>,
    pub to: Option<String>,
    pub sort: Option<String>,
}

/// Stock ownership data client
#[napi]
pub struct StockOwnershipClient {
    inner: marketdata_core::RestClient,
}

#[napi]
impl StockOwnershipClient {
    /// Get the constituents an ETF held over a date range.
    ///
    /// ```javascript
    /// await client.stock.ownership.etfHoldings({ symbol: '0050' });
    /// await client.stock.ownership.etfHoldings({ symbol: '0050', from: '2026-01-01', sort: 'desc' });
    /// ```
    #[napi(ts_return_type = "Promise<EtfHoldingsResponse>", ts_args_type = "params: EtfHoldingsParams")]
    pub async fn etf_holdings(&self, params: Option<RestArg>, extra: Option<ExtraArg>) -> napi::Result<Settled> {
        ownership(&self.inner, "stock.ownership.etfHoldings", "etf-holdings", params, extra).await
    }

    /// Get daily trading by the three major institutional investors (foreign, investment trust, dealer).
    ///
    /// ```javascript
    /// await client.stock.ownership.institutionalTrades({ symbol: '2330' });
    /// await client.stock.ownership.institutionalTrades({ symbol: '2330', from: '2026-01-01', sort: 'desc' });
    /// ```
    #[napi(ts_return_type = "Promise<InstitutionalTradesResponse>", ts_args_type = "params: InstitutionalTradesParams")]
    pub async fn institutional_trades(&self, params: Option<RestArg>, extra: Option<ExtraArg>) -> napi::Result<Settled> {
        ownership(&self.inner, "stock.ownership.institutionalTrades", "institutional-trades", params, extra).await
    }

    /// Get monthly holdings and pledges disclosed by directors and supervisors.
    ///
    /// ```javascript
    /// await client.stock.ownership.directorHoldings({ symbol: '2330' });
    /// await client.stock.ownership.directorHoldings({ symbol: '2330', from: '2026-01-01', sort: 'desc' });
    /// ```
    #[napi(ts_return_type = "Promise<DirectorHoldingsResponse>", ts_args_type = "params: DirectorHoldingsParams")]
    pub async fn director_holdings(&self, params: Option<RestArg>, extra: Option<ExtraArg>) -> napi::Result<Settled> {
        ownership(&self.inner, "stock.ownership.directorHoldings", "director-holdings", params, extra).await
    }

    /// Get the weekly TDCC shareholder distribution by holding-size bracket.
    ///
    /// ```javascript
    /// await client.stock.ownership.tdccDistribution({ symbol: '2330' });
    /// await client.stock.ownership.tdccDistribution({ symbol: '2330', from: '2026-01-01', sort: 'desc' });
    /// ```
    #[napi(ts_return_type = "Promise<TdccDistributionResponse>", ts_args_type = "params: TdccDistributionParams")]
    pub async fn tdcc_distribution(&self, params: Option<RestArg>, extra: Option<ExtraArg>) -> napi::Result<Settled> {
        ownership(&self.inner, "stock.ownership.tdccDistribution", "tdcc-distribution", params, extra).await
    }
}

/// Every `stock.ownership.*` method: the params object, checked against
/// `core::rest::params` like the other object forms (#294). These methods
/// have no positional form.
async fn ownership(
    client: &marketdata_core::RestClient,
    method: &str,
    segment: &str,
    params: Option<RestArg>,
    extra: Option<ExtraArg>,
) -> napi::Result<Settled> {
    let path = ["stock", "ownership", segment];
    if !matches!(params, Some(RestArg::Params(_))) {
        let short = method.rsplit('.').next().unwrap_or(method);
        return Ok(Settled::type_error(format!(
            "`{method}` takes a params object, e.g. {short}({{ symbol: '2330' }})"
        )));
    }
    if let Some(rejected) = unused_args(method, &path, ("params", &params), &[], extra) {
        return Ok(rejected);
    }
    let Some(RestArg::Params(object)) = params else {
        unreachable!("checked above");
    };
    get_with_params(client, &path, object).await
}

/// Stock intraday data client
#[napi]
pub struct StockIntradayClient {
    inner: marketdata_core::RestClient,
}

#[napi]
impl StockIntradayClient {
    /// Get intraday quote for a stock symbol.
    ///
    /// Two call shapes are supported (legacy fugle-marketdata-node parity):
    ///
    /// ```javascript
    /// // Object shape (matches legacy SDK README)
    /// await client.stock.intraday.quote({ symbol: '2330' });
    /// await client.stock.intraday.quote({ symbol: '2330', type: 'oddlot' });
    ///
    /// // Positional shape
    /// await client.stock.intraday.quote('2330');
    /// await client.stock.intraday.quote('2330', true);
    /// ```
    #[napi(
        ts_return_type = "Promise<QuoteResponse>",
        ts_args_type = "symbol: string | RestStockIntradayQuoteParams, oddLot?: boolean | undefined | null"
    )]
    pub async fn quote(&self, symbol: Option<RestArg>, odd_lot: Option<PosArg>, extra: Option<ExtraArg>) -> napi::Result<Settled> {
        if let Some(rejected) = unused_args(
            "stock.intraday.quote",
            &["stock", "intraday", "quote"],
            ("symbol", &symbol),
            // The positional flag also applies to the object form (below),
            // so it is never "unused" there.
            &[("oddLot", false)],
            extra,
        ) {
            return Ok(rejected);
        }
        let odd_lot = take!(pos_bool("stock.intraday.quote", "oddLot", odd_lot));
        let (symbol, effective_odd_lot) = match take!(RestArg::required("stock.intraday.quote", symbol, "symbol")) {
            RestArg::Invalid(_) => unreachable!("refused by unused_args"),
            RestArg::Positional(symbol) => (symbol, odd_lot),
            RestArg::Params(mut params) => {
                // The positional flag still applies to the object form unless
                // the object already sets `type` under any spelling; the
                // table turns `oddLot` into `type=oddlot`.
                let path = ["stock", "intraday", "quote"];
                let spec = EndpointSpec::for_path(&path).expect("quote is in the table");
                let sets_type = params
                    .iter()
                    .any(|(k, v)| !v.is_null() && spec.resolve(k).is_some_and(|r| r.spec.name == "type"));
                if odd_lot == Some(true) && !sets_type {
                    params.insert("oddLot".to_string(), Value::Bool(true));
                }
                return get_with_params(&self.inner, &path, params).await;
            }
        };

        let inner = self.inner.clone();

        // Use spawn_blocking since core uses synchronous HTTP (ureq)
        let result = tokio::task::spawn_blocking(move || {
            let stock = inner.stock();
            let intraday = stock.intraday();
            let mut builder = intraday.quote().symbol(&symbol);
            if let Some(ol) = effective_odd_lot {
                builder = builder.odd_lot(ol);
            }
            builder.send()
        })
        .await
        .map_err(|e| napi::Error::from_reason(format!("Task error: {}", e)))?;

        Ok(Settled::from(result))
    }

    /// Get intraday ticker for a stock symbol
    ///
    /// @param symbol - Stock symbol (e.g., "2330" for TSMC)
    /// @returns Promise resolving to Ticker object with last trade info
    #[napi(
        ts_return_type = "Promise<TickerResponse>",
        ts_args_type = "symbol: string | RestStockIntradayTickerParams"
    )]
    pub async fn ticker(&self, symbol: Option<RestArg>, extra: Option<ExtraArg>) -> napi::Result<Settled> {
        if let Some(rejected) = unused_args(
            "stock.intraday.ticker",
            &["stock", "intraday", "ticker"],
            ("symbol", &symbol),
            &[],
            extra,
        ) {
            return Ok(rejected);
        }
        let symbol = match take!(RestArg::required("stock.intraday.ticker", symbol, "symbol")) {
            RestArg::Invalid(_) => unreachable!("refused by unused_args"),
            RestArg::Positional(value) => value,
            RestArg::Params(params) => return get_with_params(&self.inner, &["stock", "intraday", "ticker"], params).await,
        };

        let inner = self.inner.clone();

        let result = tokio::task::spawn_blocking(move || {
            inner.stock().intraday().ticker().symbol(&symbol).send()
        })
        .await
        .map_err(|e| napi::Error::from_reason(format!("Task error: {}", e)))?;

        Ok(Settled::from(result))
    }

    /// Get intraday candles for a stock symbol
    ///
    /// @param symbol - Stock symbol (e.g., "2330" for TSMC)
    /// @param timeframe - Candle timeframe: "1", "5", "10", "15", "30", "60" (minutes)
    /// @returns Promise resolving to Candles response with OHLCV data
    #[napi(
        ts_return_type = "Promise<CandlesResponse>",
        ts_args_type = "symbol: string | RestStockIntradayCandlesParams, timeframe?: string | undefined | null"
    )]
    pub async fn candles(&self, symbol: Option<RestArg>, timeframe: Option<PosArg>, extra: Option<ExtraArg>) -> napi::Result<Settled> {
        if let Some(rejected) = unused_args(
            "stock.intraday.candles",
            &["stock", "intraday", "candles"],
            ("symbol", &symbol),
            &[("timeframe", timeframe.is_some())],
            extra,
        ) {
            return Ok(rejected);
        }
        let timeframe = take!(pos_string("stock.intraday.candles", "timeframe", timeframe));
        let symbol = match take!(RestArg::required("stock.intraday.candles", symbol, "symbol")) {
            RestArg::Invalid(_) => unreachable!("refused by unused_args"),
            RestArg::Positional(value) => value,
            RestArg::Params(params) => return get_with_params(&self.inner, &["stock", "intraday", "candles"], params).await,
        };

        let inner = self.inner.clone();

        let result = tokio::task::spawn_blocking(move || {
            let stock = inner.stock();
            let intraday = stock.intraday();
            let mut builder = intraday.candles().symbol(&symbol);
            if let Some(tf) = &timeframe {
                builder = builder.timeframe(tf);
            }
            builder.send()
        })
        .await
        .map_err(|e| napi::Error::from_reason(format!("Task error: {}", e)))?;

        Ok(Settled::from(result))
    }

    /// Get intraday trades for a stock symbol
    ///
    /// @param symbol - Stock symbol (e.g., "2330" for TSMC)
    /// @returns Promise resolving to Trades response with recent trade history
    #[napi(
        ts_return_type = "Promise<TradesResponse>",
        ts_args_type = "symbol: string | RestStockIntradayTradesParams"
    )]
    pub async fn trades(&self, symbol: Option<RestArg>, extra: Option<ExtraArg>) -> napi::Result<Settled> {
        if let Some(rejected) = unused_args(
            "stock.intraday.trades",
            &["stock", "intraday", "trades"],
            ("symbol", &symbol),
            &[],
            extra,
        ) {
            return Ok(rejected);
        }
        let symbol = match take!(RestArg::required("stock.intraday.trades", symbol, "symbol")) {
            RestArg::Invalid(_) => unreachable!("refused by unused_args"),
            RestArg::Positional(value) => value,
            RestArg::Params(params) => return get_with_params(&self.inner, &["stock", "intraday", "trades"], params).await,
        };

        let inner = self.inner.clone();

        let result = tokio::task::spawn_blocking(move || {
            inner.stock().intraday().trades().symbol(&symbol).send()
        })
        .await
        .map_err(|e| napi::Error::from_reason(format!("Task error: {}", e)))?;

        Ok(Settled::from(result))
    }

    /// Get intraday volumes for a stock symbol
    ///
    /// @param symbol - Stock symbol (e.g., "2330" for TSMC)
    /// @returns Promise resolving to Volumes response with volume at each price level
    #[napi(
        ts_return_type = "Promise<VolumesResponse>",
        ts_args_type = "symbol: string | RestStockIntradayVolumesParams"
    )]
    pub async fn volumes(&self, symbol: Option<RestArg>, extra: Option<ExtraArg>) -> napi::Result<Settled> {
        if let Some(rejected) = unused_args(
            "stock.intraday.volumes",
            &["stock", "intraday", "volumes"],
            ("symbol", &symbol),
            &[],
            extra,
        ) {
            return Ok(rejected);
        }
        let symbol = match take!(RestArg::required("stock.intraday.volumes", symbol, "symbol")) {
            RestArg::Invalid(_) => unreachable!("refused by unused_args"),
            RestArg::Positional(value) => value,
            RestArg::Params(params) => return get_with_params(&self.inner, &["stock", "intraday", "volumes"], params).await,
        };

        let inner = self.inner.clone();

        let result = tokio::task::spawn_blocking(move || {
            inner.stock().intraday().volumes().symbol(&symbol).send()
        })
        .await
        .map_err(|e| napi::Error::from_reason(format!("Task error: {}", e)))?;

        Ok(Settled::from(result))
    }

    /// Get batch ticker list for a security type
    ///
    /// @param type - Security type ("EQUITY", "INDEX", "ETF", ...)
    /// @param exchange - Optional exchange filter (e.g., "TWSE", "TPEx")
    /// @param market - Optional market filter (e.g., "TSE", "OTC")
    /// @param industry - Optional industry code filter
    /// @param isNormal - Filter to normal-status tickers only
    /// @returns Promise resolving to an array of ticker info objects
    #[napi(
        ts_return_type = "Promise<TickersResponse>",
        ts_args_type = "type: string | RestStockIntradayTickersParams, exchange?: string | undefined | null, market?: string | undefined | null, industry?: string | undefined | null, isNormal?: boolean | undefined | null"
    )]
    pub async fn tickers(
        &self,
        r#type: Option<RestArg>,
        exchange: Option<PosArg>,
        market: Option<PosArg>,
        industry: Option<PosArg>,
        is_normal: Option<PosArg>,
        extra: Option<ExtraArg>,
    ) -> napi::Result<Settled> {
        if let Some(rejected) = unused_args(
            "stock.intraday.tickers",
            &["stock", "intraday", "tickers"],
            ("type", &r#type),
            &[("exchange", exchange.is_some()), ("market", market.is_some()), ("industry", industry.is_some()), ("isNormal", is_normal.is_some())],
            extra,
        ) {
            return Ok(rejected);
        }
        let exchange = take!(pos_string("stock.intraday.tickers", "exchange", exchange));
        let market = take!(pos_string("stock.intraday.tickers", "market", market));
        let industry = take!(pos_string("stock.intraday.tickers", "industry", industry));
        let is_normal = take!(pos_bool("stock.intraday.tickers", "isNormal", is_normal));
        let r#type = match take!(RestArg::required("stock.intraday.tickers", r#type, "type")) {
            RestArg::Invalid(_) => unreachable!("refused by unused_args"),
            RestArg::Positional(value) => value,
            RestArg::Params(params) => return get_with_params(&self.inner, &["stock", "intraday", "tickers"], params).await,
        };

        let inner = self.inner.clone();

        let result = tokio::task::spawn_blocking(move || {
            let stock = inner.stock();
            let intraday = stock.intraday();
            let mut builder = intraday.tickers().typ(&r#type);
            if let Some(e) = &exchange {
                builder = builder.exchange(e);
            }
            if let Some(m) = &market {
                builder = builder.market(m);
            }
            if let Some(i) = &industry {
                builder = builder.industry(i);
            }
            if let Some(n) = is_normal {
                builder = builder.is_normal(n);
            }
            builder.send()
        })
        .await
        .map_err(|e| napi::Error::from_reason(format!("Task error: {}", e)))?;

        Ok(Settled::from(result))
    }
}

/// Stock historical data client
#[napi]
pub struct StockHistoricalClient {
    inner: marketdata_core::RestClient,
}

#[napi]
impl StockHistoricalClient {
    /// Get historical candles for a stock symbol
    ///
    /// @param symbol - Stock symbol (e.g., "2330")
    /// @param from - Start date (YYYY-MM-DD)
    /// @param to - End date (YYYY-MM-DD)
    /// @param timeframe - Timeframe ("D", "W", "M", "1", "5", etc.)
    /// @returns Promise resolving to historical candles data
    #[napi(
        ts_return_type = "Promise<HistoricalCandlesResponse>",
        ts_args_type = "symbol: string | RestStockHistoricalCandlesParams, from?: string | undefined | null, to?: string | undefined | null, timeframe?: string | undefined | null"
    )]
    pub async fn candles(
        &self,
        symbol: Option<RestArg>,
        from: Option<PosArg>,
        to: Option<PosArg>,
        timeframe: Option<PosArg>,
        extra: Option<ExtraArg>,
    ) -> napi::Result<Settled> {
        if let Some(rejected) = unused_args(
            "stock.historical.candles",
            &["stock", "historical", "candles"],
            ("symbol", &symbol),
            &[("from", from.is_some()), ("to", to.is_some()), ("timeframe", timeframe.is_some())],
            extra,
        ) {
            return Ok(rejected);
        }
        let from = take!(pos_string("stock.historical.candles", "from", from));
        let to = take!(pos_string("stock.historical.candles", "to", to));
        let timeframe = take!(pos_string("stock.historical.candles", "timeframe", timeframe));
        let symbol = match take!(RestArg::required("stock.historical.candles", symbol, "symbol")) {
            RestArg::Invalid(_) => unreachable!("refused by unused_args"),
            RestArg::Positional(value) => value,
            RestArg::Params(params) => return get_with_params(&self.inner, &["stock", "historical", "candles"], params).await,
        };

        let inner = self.inner.clone();

        let result = tokio::task::spawn_blocking(move || {
            let stock = inner.stock();
            let hist = stock.historical();
            let mut builder = hist.candles().symbol(&symbol);
            if let Some(f) = from {
                builder = builder.from(&f);
            }
            if let Some(t) = to {
                builder = builder.to(&t);
            }
            if let Some(tf) = timeframe {
                builder = builder.timeframe(&tf);
            }
            builder.send()
        })
        .await
        .map_err(|e| napi::Error::from_reason(format!("Task error: {}", e)))?;

        Ok(Settled::from(result))
    }

    /// Get historical stats for a stock symbol
    ///
    /// @param symbol - Stock symbol (e.g., "2330")
    /// @returns Promise resolving to historical stats data
    #[napi(
        ts_return_type = "Promise<StatsResponse>",
        ts_args_type = "symbol: string | RestStockHistoricalStatsParams"
    )]
    pub async fn stats(&self, symbol: Option<RestArg>, extra: Option<ExtraArg>) -> napi::Result<Settled> {
        if let Some(rejected) = unused_args(
            "stock.historical.stats",
            &["stock", "historical", "stats"],
            ("symbol", &symbol),
            &[],
            extra,
        ) {
            return Ok(rejected);
        }
        let symbol = match take!(RestArg::required("stock.historical.stats", symbol, "symbol")) {
            RestArg::Invalid(_) => unreachable!("refused by unused_args"),
            RestArg::Positional(value) => value,
            RestArg::Params(params) => return get_with_params(&self.inner, &["stock", "historical", "stats"], params).await,
        };

        let inner = self.inner.clone();

        let result = tokio::task::spawn_blocking(move || {
            inner.stock().historical().stats().symbol(&symbol).send()
        })
        .await
        .map_err(|e| napi::Error::from_reason(format!("Task error: {}", e)))?;

        Ok(Settled::from(result))
    }
}

/// Stock snapshot data client
#[napi]
pub struct StockSnapshotClient {
    inner: marketdata_core::RestClient,
}

#[napi]
impl StockSnapshotClient {
    /// Get snapshot quotes for a market
    ///
    /// @param market - Market code (e.g., "TSE", "OTC")
    /// @param typeFilter - Optional type filter (e.g., "ALL", "COMMONSTOCK")
    /// @returns Promise resolving to snapshot quotes data
    #[napi(
        ts_return_type = "Promise<SnapshotQuotesResponse>",
        ts_args_type = "market: string | RestStockSnapshotQuotesParams, typeFilter?: string | undefined | null"
    )]
    pub async fn quotes(&self, market: Option<RestArg>, type_filter: Option<PosArg>, extra: Option<ExtraArg>) -> napi::Result<Settled> {
        if let Some(rejected) = unused_args(
            "stock.snapshot.quotes",
            &["stock", "snapshot", "quotes"],
            ("market", &market),
            &[("typeFilter", type_filter.is_some())],
            extra,
        ) {
            return Ok(rejected);
        }
        let type_filter = take!(pos_string("stock.snapshot.quotes", "typeFilter", type_filter));
        let market = match take!(RestArg::required("stock.snapshot.quotes", market, "market")) {
            RestArg::Invalid(_) => unreachable!("refused by unused_args"),
            RestArg::Positional(value) => value,
            RestArg::Params(params) => return get_with_params(&self.inner, &["stock", "snapshot", "quotes"], params).await,
        };

        let inner = self.inner.clone();

        let result = tokio::task::spawn_blocking(move || {
            let stock = inner.stock();
            let snap = stock.snapshot();
            let mut builder = snap.quotes().market(&market);
            if let Some(tf) = type_filter {
                builder = builder.type_filter(&tf);
            }
            builder.send()
        })
        .await
        .map_err(|e| napi::Error::from_reason(format!("Task error: {}", e)))?;

        Ok(Settled::from(result))
    }

    /// Get movers (top gainers/losers) for a market
    ///
    /// @param market - Market code (e.g., "TSE", "OTC")
    /// @param direction - Direction filter ("up" or "down")
    /// @param change - Change type ("percent" or "value")
    /// @returns Promise resolving to movers data
    #[napi(
        ts_return_type = "Promise<MoversResponse>",
        ts_args_type = "market: string | RestStockSnapshotMoversParams, direction?: string | undefined | null, change?: string | undefined | null"
    )]
    pub async fn movers(
        &self,
        market: Option<RestArg>,
        direction: Option<PosArg>,
        change: Option<PosArg>,
        extra: Option<ExtraArg>,
    ) -> napi::Result<Settled> {
        if let Some(rejected) = unused_args(
            "stock.snapshot.movers",
            &["stock", "snapshot", "movers"],
            ("market", &market),
            &[("direction", direction.is_some()), ("change", change.is_some())],
            extra,
        ) {
            return Ok(rejected);
        }
        let direction = take!(pos_string("stock.snapshot.movers", "direction", direction));
        let change = take!(pos_string("stock.snapshot.movers", "change", change));
        let market = match take!(RestArg::required("stock.snapshot.movers", market, "market")) {
            RestArg::Invalid(_) => unreachable!("refused by unused_args"),
            RestArg::Positional(value) => value,
            RestArg::Params(params) => return get_with_params(&self.inner, &["stock", "snapshot", "movers"], params).await,
        };

        let inner = self.inner.clone();

        let result = tokio::task::spawn_blocking(move || {
            let stock = inner.stock();
            let snap = stock.snapshot();
            let mut builder = snap.movers().market(&market);
            if let Some(d) = direction {
                builder = builder.direction(&d);
            }
            if let Some(c) = change {
                builder = builder.change(&c);
            }
            builder.send()
        })
        .await
        .map_err(|e| napi::Error::from_reason(format!("Task error: {}", e)))?;

        Ok(Settled::from(result))
    }

    /// Get most actively traded stocks for a market
    ///
    /// @param market - Market code (e.g., "TSE", "OTC")
    /// @param trade - Trade type filter ("volume" or "value")
    /// @returns Promise resolving to actives data
    #[napi(
        ts_return_type = "Promise<ActivesResponse>",
        ts_args_type = "market: string | RestStockSnapshotActivesParams, trade?: string | undefined | null"
    )]
    pub async fn actives(&self, market: Option<RestArg>, trade: Option<PosArg>, extra: Option<ExtraArg>) -> napi::Result<Settled> {
        if let Some(rejected) = unused_args(
            "stock.snapshot.actives",
            &["stock", "snapshot", "actives"],
            ("market", &market),
            &[("trade", trade.is_some())],
            extra,
        ) {
            return Ok(rejected);
        }
        let trade = take!(pos_string("stock.snapshot.actives", "trade", trade));
        let market = match take!(RestArg::required("stock.snapshot.actives", market, "market")) {
            RestArg::Invalid(_) => unreachable!("refused by unused_args"),
            RestArg::Positional(value) => value,
            RestArg::Params(params) => return get_with_params(&self.inner, &["stock", "snapshot", "actives"], params).await,
        };

        let inner = self.inner.clone();

        let result = tokio::task::spawn_blocking(move || {
            let stock = inner.stock();
            let snap = stock.snapshot();
            let mut builder = snap.actives().market(&market);
            if let Some(t) = trade {
                builder = builder.trade(&t);
            }
            builder.send()
        })
        .await
        .map_err(|e| napi::Error::from_reason(format!("Task error: {}", e)))?;

        Ok(Settled::from(result))
    }
}

/// Stock technical indicators client
#[napi]
pub struct StockTechnicalClient {
    inner: marketdata_core::RestClient,
}

#[napi]
impl StockTechnicalClient {
    /// Get SMA (Simple Moving Average) for a stock
    ///
    /// @param symbol - Stock symbol (e.g., "2330")
    /// @param from - Start date (YYYY-MM-DD)
    /// @param to - End date (YYYY-MM-DD)
    /// @param timeframe - Timeframe ("D", "W", "M")
    /// @param period - SMA period (e.g., 20)
    /// @returns Promise resolving to SMA data
    #[napi(
        ts_return_type = "Promise<SmaResponse>",
        ts_args_type = "symbol: string | RestStockTechnicalSmaParams, from?: string | undefined | null, to?: string | undefined | null, timeframe?: string | undefined | null, period?: number | undefined | null"
    )]
    pub async fn sma(
        &self,
        symbol: Option<RestArg>,
        from: Option<PosArg>,
        to: Option<PosArg>,
        timeframe: Option<PosArg>,
        period: Option<PosArg>,
        extra: Option<ExtraArg>,
    ) -> napi::Result<Settled> {
        if let Some(rejected) = unused_args(
            "stock.technical.sma",
            &["stock", "technical", "sma"],
            ("symbol", &symbol),
            &[("from", from.is_some()), ("to", to.is_some()), ("timeframe", timeframe.is_some()), ("period", period.is_some())],
            extra,
        ) {
            return Ok(rejected);
        }
        let from = take!(pos_string("stock.technical.sma", "from", from));
        let to = take!(pos_string("stock.technical.sma", "to", to));
        let timeframe = take!(pos_string("stock.technical.sma", "timeframe", timeframe));
        let period = take!(pos_u32("stock.technical.sma", "period", period));
        let symbol = match take!(RestArg::required("stock.technical.sma", symbol, "symbol")) {
            RestArg::Invalid(_) => unreachable!("refused by unused_args"),
            RestArg::Positional(value) => value,
            RestArg::Params(params) => return get_with_params(&self.inner, &["stock", "technical", "sma"], params).await,
        };

        let inner = self.inner.clone();

        let result = tokio::task::spawn_blocking(move || {
            let stock = inner.stock();
            let tech = stock.technical();
            let mut builder = tech.sma().symbol(&symbol);
            if let Some(f) = from {
                builder = builder.from(&f);
            }
            if let Some(t) = to {
                builder = builder.to(&t);
            }
            if let Some(tf) = timeframe {
                builder = builder.timeframe(&tf);
            }
            if let Some(p) = period {
                builder = builder.period(p);
            }
            builder.send()
        })
        .await
        .map_err(|e| napi::Error::from_reason(format!("Task error: {}", e)))?;

        Ok(Settled::from(result))
    }

    /// Get RSI (Relative Strength Index) for a stock
    ///
    /// @param symbol - Stock symbol (e.g., "2330")
    /// @param from - Start date (YYYY-MM-DD)
    /// @param to - End date (YYYY-MM-DD)
    /// @param timeframe - Timeframe ("D", "W", "M")
    /// @param period - RSI period (e.g., 14)
    /// @returns Promise resolving to RSI data
    #[napi(
        ts_return_type = "Promise<RsiResponse>",
        ts_args_type = "symbol: string | RestStockTechnicalRsiParams, from?: string | undefined | null, to?: string | undefined | null, timeframe?: string | undefined | null, period?: number | undefined | null"
    )]
    pub async fn rsi(
        &self,
        symbol: Option<RestArg>,
        from: Option<PosArg>,
        to: Option<PosArg>,
        timeframe: Option<PosArg>,
        period: Option<PosArg>,
        extra: Option<ExtraArg>,
    ) -> napi::Result<Settled> {
        if let Some(rejected) = unused_args(
            "stock.technical.rsi",
            &["stock", "technical", "rsi"],
            ("symbol", &symbol),
            &[("from", from.is_some()), ("to", to.is_some()), ("timeframe", timeframe.is_some()), ("period", period.is_some())],
            extra,
        ) {
            return Ok(rejected);
        }
        let from = take!(pos_string("stock.technical.rsi", "from", from));
        let to = take!(pos_string("stock.technical.rsi", "to", to));
        let timeframe = take!(pos_string("stock.technical.rsi", "timeframe", timeframe));
        let period = take!(pos_u32("stock.technical.rsi", "period", period));
        let symbol = match take!(RestArg::required("stock.technical.rsi", symbol, "symbol")) {
            RestArg::Invalid(_) => unreachable!("refused by unused_args"),
            RestArg::Positional(value) => value,
            RestArg::Params(params) => return get_with_params(&self.inner, &["stock", "technical", "rsi"], params).await,
        };

        let inner = self.inner.clone();

        let result = tokio::task::spawn_blocking(move || {
            let stock = inner.stock();
            let tech = stock.technical();
            let mut builder = tech.rsi().symbol(&symbol);
            if let Some(f) = from {
                builder = builder.from(&f);
            }
            if let Some(t) = to {
                builder = builder.to(&t);
            }
            if let Some(tf) = timeframe {
                builder = builder.timeframe(&tf);
            }
            if let Some(p) = period {
                builder = builder.period(p);
            }
            builder.send()
        })
        .await
        .map_err(|e| napi::Error::from_reason(format!("Task error: {}", e)))?;

        Ok(Settled::from(result))
    }

    /// Get KDJ (Stochastic Oscillator) for a stock
    ///
    /// @param symbol - Stock symbol (e.g., "2330")
    /// @param from - Start date (YYYY-MM-DD)
    /// @param to - End date (YYYY-MM-DD)
    /// @param timeframe - Timeframe ("D", "W", "M")
    /// @param rPeriod - RSV period (e.g., 9)
    /// @param kPeriod - K smoothing period (e.g., 3)
    /// @param dPeriod - D smoothing period (e.g., 3)
    /// @returns Promise resolving to KDJ data
    #[napi(
        ts_return_type = "Promise<KdjResponse>",
        ts_args_type = "symbol: string | RestStockTechnicalKdjParams, from?: string | undefined | null, to?: string | undefined | null, timeframe?: string | undefined | null, rPeriod?: number | undefined | null, kPeriod?: number | undefined | null, dPeriod?: number | undefined | null"
    )]
    pub async fn kdj(
        &self,
        symbol: Option<RestArg>,
        from: Option<PosArg>,
        to: Option<PosArg>,
        timeframe: Option<PosArg>,
        r_period: Option<PosArg>,
        k_period: Option<PosArg>,
        d_period: Option<PosArg>,
        extra: Option<ExtraArg>,
    ) -> napi::Result<Settled> {
        if let Some(rejected) = unused_args(
            "stock.technical.kdj",
            &["stock", "technical", "kdj"],
            ("symbol", &symbol),
            &[("from", from.is_some()), ("to", to.is_some()), ("timeframe", timeframe.is_some()), ("rPeriod", r_period.is_some()), ("kPeriod", k_period.is_some()), ("dPeriod", d_period.is_some())],
            extra,
        ) {
            return Ok(rejected);
        }
        let from = take!(pos_string("stock.technical.kdj", "from", from));
        let to = take!(pos_string("stock.technical.kdj", "to", to));
        let timeframe = take!(pos_string("stock.technical.kdj", "timeframe", timeframe));
        let r_period = take!(pos_u32("stock.technical.kdj", "rPeriod", r_period));
        let k_period = take!(pos_u32("stock.technical.kdj", "kPeriod", k_period));
        let d_period = take!(pos_u32("stock.technical.kdj", "dPeriod", d_period));
        let symbol = match take!(RestArg::required("stock.technical.kdj", symbol, "symbol")) {
            RestArg::Invalid(_) => unreachable!("refused by unused_args"),
            RestArg::Positional(value) => value,
            RestArg::Params(params) => return get_with_params(&self.inner, &["stock", "technical", "kdj"], params).await,
        };

        let inner = self.inner.clone();

        let result = tokio::task::spawn_blocking(move || {
            let stock = inner.stock();
            let tech = stock.technical();
            let mut builder = tech.kdj().symbol(&symbol);
            if let Some(f) = from {
                builder = builder.from(&f);
            }
            if let Some(t) = to {
                builder = builder.to(&t);
            }
            if let Some(tf) = timeframe {
                builder = builder.timeframe(&tf);
            }
            if let Some(p) = r_period {
                builder = builder.r_period(p);
            }
            if let Some(p) = k_period {
                builder = builder.k_period(p);
            }
            if let Some(p) = d_period {
                builder = builder.d_period(p);
            }
            builder.send()
        })
        .await
        .map_err(|e| napi::Error::from_reason(format!("Task error: {}", e)))?;

        Ok(Settled::from(result))
    }

    /// Get MACD (Moving Average Convergence Divergence) for a stock
    ///
    /// @param symbol - Stock symbol (e.g., "2330")
    /// @param from - Start date (YYYY-MM-DD)
    /// @param to - End date (YYYY-MM-DD)
    /// @param timeframe - Timeframe ("D", "W", "M")
    /// @param fast - Fast EMA period (default: 12)
    /// @param slow - Slow EMA period (default: 26)
    /// @param signal - Signal line period (default: 9)
    /// @returns Promise resolving to MACD data
    #[napi(
        ts_return_type = "Promise<MacdResponse>",
        ts_args_type = "symbol: string | RestStockTechnicalMacdParams, from?: string | undefined | null, to?: string | undefined | null, timeframe?: string | undefined | null, fast?: number | undefined | null, slow?: number | undefined | null, signal?: number | undefined | null"
    )]
    pub async fn macd(
        &self,
        symbol: Option<RestArg>,
        from: Option<PosArg>,
        to: Option<PosArg>,
        timeframe: Option<PosArg>,
        fast: Option<PosArg>,
        slow: Option<PosArg>,
        signal: Option<PosArg>,
        extra: Option<ExtraArg>,
    ) -> napi::Result<Settled> {
        if let Some(rejected) = unused_args(
            "stock.technical.macd",
            &["stock", "technical", "macd"],
            ("symbol", &symbol),
            &[("from", from.is_some()), ("to", to.is_some()), ("timeframe", timeframe.is_some()), ("fast", fast.is_some()), ("slow", slow.is_some()), ("signal", signal.is_some())],
            extra,
        ) {
            return Ok(rejected);
        }
        let from = take!(pos_string("stock.technical.macd", "from", from));
        let to = take!(pos_string("stock.technical.macd", "to", to));
        let timeframe = take!(pos_string("stock.technical.macd", "timeframe", timeframe));
        let fast = take!(pos_u32("stock.technical.macd", "fast", fast));
        let slow = take!(pos_u32("stock.technical.macd", "slow", slow));
        let signal = take!(pos_u32("stock.technical.macd", "signal", signal));
        let symbol = match take!(RestArg::required("stock.technical.macd", symbol, "symbol")) {
            RestArg::Invalid(_) => unreachable!("refused by unused_args"),
            RestArg::Positional(value) => value,
            RestArg::Params(params) => return get_with_params(&self.inner, &["stock", "technical", "macd"], params).await,
        };

        let inner = self.inner.clone();

        let result = tokio::task::spawn_blocking(move || {
            let stock = inner.stock();
            let tech = stock.technical();
            let mut builder = tech.macd().symbol(&symbol);
            if let Some(f) = from {
                builder = builder.from(&f);
            }
            if let Some(t) = to {
                builder = builder.to(&t);
            }
            if let Some(tf) = timeframe {
                builder = builder.timeframe(&tf);
            }
            if let Some(f) = fast {
                builder = builder.fast(f);
            }
            if let Some(s) = slow {
                builder = builder.slow(s);
            }
            if let Some(s) = signal {
                builder = builder.signal(s);
            }
            builder.send()
        })
        .await
        .map_err(|e| napi::Error::from_reason(format!("Task error: {}", e)))?;

        Ok(Settled::from(result))
    }

    /// Get Bollinger Bands for a stock
    ///
    /// @param symbol - Stock symbol (e.g., "2330")
    /// @param from - Start date (YYYY-MM-DD)
    /// @param to - End date (YYYY-MM-DD)
    /// @param timeframe - Timeframe ("D", "W", "M")
    /// @param period - SMA period (default: 20)
    /// @returns Promise resolving to Bollinger Bands data
    #[napi(
        ts_return_type = "Promise<BbResponse>",
        ts_args_type = "symbol: string | RestStockTechnicalBbParams, from?: string | undefined | null, to?: string | undefined | null, timeframe?: string | undefined | null, period?: number | undefined | null"
    )]
    pub async fn bb(
        &self,
        symbol: Option<RestArg>,
        from: Option<PosArg>,
        to: Option<PosArg>,
        timeframe: Option<PosArg>,
        period: Option<PosArg>,
        extra: Option<ExtraArg>,
    ) -> napi::Result<Settled> {
        if let Some(rejected) = unused_args(
            "stock.technical.bb",
            &["stock", "technical", "bb"],
            ("symbol", &symbol),
            &[("from", from.is_some()), ("to", to.is_some()), ("timeframe", timeframe.is_some()), ("period", period.is_some())],
            extra,
        ) {
            return Ok(rejected);
        }
        let from = take!(pos_string("stock.technical.bb", "from", from));
        let to = take!(pos_string("stock.technical.bb", "to", to));
        let timeframe = take!(pos_string("stock.technical.bb", "timeframe", timeframe));
        let period = take!(pos_u32("stock.technical.bb", "period", period));
        let symbol = match take!(RestArg::required("stock.technical.bb", symbol, "symbol")) {
            RestArg::Invalid(_) => unreachable!("refused by unused_args"),
            RestArg::Positional(value) => value,
            RestArg::Params(params) => return get_with_params(&self.inner, &["stock", "technical", "bb"], params).await,
        };

        let inner = self.inner.clone();

        let result = tokio::task::spawn_blocking(move || {
            let stock = inner.stock();
            let tech = stock.technical();
            let mut builder = tech.bb().symbol(&symbol);
            if let Some(f) = from {
                builder = builder.from(&f);
            }
            if let Some(t) = to {
                builder = builder.to(&t);
            }
            if let Some(tf) = timeframe {
                builder = builder.timeframe(&tf);
            }
            if let Some(p) = period {
                builder = builder.period(p);
            }
            builder.send()
        })
        .await
        .map_err(|e| napi::Error::from_reason(format!("Task error: {}", e)))?;

        Ok(Settled::from(result))
    }
}

/// Stock corporate actions client
#[napi]
pub struct StockCorporateActionsClient {
    inner: marketdata_core::RestClient,
}

/// `date` used to be the first positional argument of the corporate-actions
/// methods. The server never accepted it (`capital-changes` and
/// `listing-applicants` answer 400, `dividends` ignores it), so it is gone and
/// `startDate` moved into its slot. Two old call shapes would now run with a
/// shifted date range and no complaint, so both are refused with directions
/// instead of being dropped:
/// - a third positional argument: the old `(date, startDate, endDate)`;
/// - an `undefined` / `null` first argument with a second one given: the old
///   `(date, startDate)`, indistinguishable from a new call that only wants
///   `endDate`, which the object form covers.
fn reject_legacy_date_args(
    method: &str,
    first: &Option<RestArg>,
    second: &Option<PosArg>,
    third: &Option<PosArg>,
) -> napi::Result<()> {
    // `undefined` / `null` arrive as `None`; any other value, whatever its
    // type, is the old third argument.
    if third.is_some() {
        return Err(napi::Error::from_reason(format!(
            "`{method}` no longer takes `date` as its first argument: the server rejects it \
             (capital-changes and listing-applicants respond 400, dividends ignores it). \
             The first argument is now `startDate`, so a third argument means the old \
             `(date, startDate, endDate)` form: call `{method}(startDate, endDate)` or \
             `{method}({{ start_date, end_date }})` instead."
        )));
    }
    if first.is_none() && second.is_some() {
        return Err(napi::Error::from_reason(format!(
            "`{method}` got an undefined first argument with a second one: this is either \
             the old `(date, startDate)` form (`date` was removed; the server rejects it) or \
             a call that only wants `endDate`. Both read the same, so use the object form: \
             `{method}({{ start_date, end_date }})`, or `{method}({{ end_date }})` for an end \
             date alone."
        )));
    }
    Ok(())
}

#[napi]
impl StockCorporateActionsClient {
    /// Get capital changes (capital structure changes)
    ///
    /// @param startDate - Start date for range query (YYYY-MM-DD)
    /// @param endDate - End date for range query (YYYY-MM-DD)
    /// @returns Promise resolving to capital changes data
    #[napi(
        ts_return_type = "Promise<CapitalChangesResponse>",
        ts_args_type = "startDate?: string | RestStockCorporateActionsCapitalChangesParams | undefined | null, endDate?: string | undefined | null"
    )]
    pub async fn capital_changes(
        &self,
        start_date: Option<RestArg>,
        end_date: Option<PosArg>,
        legacy_third_arg: Option<PosArg>,
        extra: Option<ExtraArg>,
    ) -> napi::Result<Settled> {
        reject_legacy_date_args("capitalChanges", &start_date, &end_date, &legacy_third_arg)?;
        if let Some(rejected) = unused_args(
            "stock.corporateActions.capitalChanges",
            &["stock", "corporate-actions", "capital-changes"],
            ("startDate", &start_date),
            &[("endDate", end_date.is_some())],
            extra,
        ) {
            return Ok(rejected);
        }
        let end_date = take!(pos_string("stock.corporateActions.capitalChanges", "endDate", end_date));
        let start_date = match start_date {
            Some(RestArg::Positional(start_date)) => Some(start_date),
            Some(RestArg::Invalid(_)) => unreachable!("refused by unused_args"),
            Some(RestArg::Params(params)) => return get_with_params(&self.inner, &["stock", "corporate-actions", "capital-changes"], params).await,
            None => None,
        };

        let inner = self.inner.clone();

        let result = tokio::task::spawn_blocking(move || {
            let stock = inner.stock();
            let ca = stock.corporate_actions();
            let mut builder = ca.capital_changes();
            if let Some(sd) = start_date {
                builder = builder.start_date(&sd);
            }
            if let Some(ed) = end_date {
                builder = builder.end_date(&ed);
            }
            builder.send()
        })
        .await
        .map_err(|e| napi::Error::from_reason(format!("Task error: {}", e)))?;

        Ok(Settled::from(result))
    }

    /// Get dividend announcements
    ///
    /// @param startDate - Start date for range query (YYYY-MM-DD)
    /// @param endDate - End date for range query (YYYY-MM-DD)
    /// @returns Promise resolving to dividends data
    #[napi(
        ts_return_type = "Promise<DividendsResponse>",
        ts_args_type = "startDate?: string | RestStockCorporateActionsDividendsParams | undefined | null, endDate?: string | undefined | null"
    )]
    pub async fn dividends(
        &self,
        start_date: Option<RestArg>,
        end_date: Option<PosArg>,
        legacy_third_arg: Option<PosArg>,
        extra: Option<ExtraArg>,
    ) -> napi::Result<Settled> {
        reject_legacy_date_args("dividends", &start_date, &end_date, &legacy_third_arg)?;
        if let Some(rejected) = unused_args(
            "stock.corporateActions.dividends",
            &["stock", "corporate-actions", "dividends"],
            ("startDate", &start_date),
            &[("endDate", end_date.is_some())],
            extra,
        ) {
            return Ok(rejected);
        }
        let end_date = take!(pos_string("stock.corporateActions.dividends", "endDate", end_date));
        let start_date = match start_date {
            Some(RestArg::Positional(start_date)) => Some(start_date),
            Some(RestArg::Invalid(_)) => unreachable!("refused by unused_args"),
            Some(RestArg::Params(params)) => return get_with_params(&self.inner, &["stock", "corporate-actions", "dividends"], params).await,
            None => None,
        };

        let inner = self.inner.clone();

        let result = tokio::task::spawn_blocking(move || {
            let stock = inner.stock();
            let ca = stock.corporate_actions();
            let mut builder = ca.dividends();
            if let Some(sd) = start_date {
                builder = builder.start_date(&sd);
            }
            if let Some(ed) = end_date {
                builder = builder.end_date(&ed);
            }
            builder.send()
        })
        .await
        .map_err(|e| napi::Error::from_reason(format!("Task error: {}", e)))?;

        Ok(Settled::from(result))
    }

    /// Get IPO listing applicants
    ///
    /// @param startDate - Start date for range query (YYYY-MM-DD)
    /// @param endDate - End date for range query (YYYY-MM-DD)
    /// @returns Promise resolving to listing applicants data
    #[napi(
        ts_return_type = "Promise<ListingApplicantsResponse>",
        ts_args_type = "startDate?: string | RestStockCorporateActionsListingApplicantsParams | undefined | null, endDate?: string | undefined | null"
    )]
    pub async fn listing_applicants(
        &self,
        start_date: Option<RestArg>,
        end_date: Option<PosArg>,
        legacy_third_arg: Option<PosArg>,
        extra: Option<ExtraArg>,
    ) -> napi::Result<Settled> {
        reject_legacy_date_args("listingApplicants", &start_date, &end_date, &legacy_third_arg)?;
        if let Some(rejected) = unused_args(
            "stock.corporateActions.listingApplicants",
            &["stock", "corporate-actions", "listing-applicants"],
            ("startDate", &start_date),
            &[("endDate", end_date.is_some())],
            extra,
        ) {
            return Ok(rejected);
        }
        let end_date = take!(pos_string("stock.corporateActions.listingApplicants", "endDate", end_date));
        let start_date = match start_date {
            Some(RestArg::Positional(start_date)) => Some(start_date),
            Some(RestArg::Invalid(_)) => unreachable!("refused by unused_args"),
            Some(RestArg::Params(params)) => return get_with_params(&self.inner, &["stock", "corporate-actions", "listing-applicants"], params).await,
            None => None,
        };

        let inner = self.inner.clone();

        let result = tokio::task::spawn_blocking(move || {
            let stock = inner.stock();
            let ca = stock.corporate_actions();
            let mut builder = ca.listing_applicants();
            if let Some(sd) = start_date {
                builder = builder.start_date(&sd);
            }
            if let Some(ed) = end_date {
                builder = builder.end_date(&ed);
            }
            builder.send()
        })
        .await
        .map_err(|e| napi::Error::from_reason(format!("Task error: {}", e)))?;

        Ok(Settled::from(result))
    }
}

/// Futures and Options market data client
#[napi]
pub struct FutOptClient {
    inner: marketdata_core::RestClient,
}

#[napi]
impl FutOptClient {
    /// Get intraday client for real-time futures/options data
    #[napi(getter)]
    pub fn intraday(&self) -> FutOptIntradayClient {
        FutOptIntradayClient {
            inner: self.inner.clone(),
        }
    }

    /// Get historical client for historical futures/options data
    #[napi(getter)]
    pub fn historical(&self) -> FutOptHistoricalClient {
        FutOptHistoricalClient {
            inner: self.inner.clone(),
        }
    }

    /// The fully resolved request prefix for this product client, product
    /// segment included (`…/v1.0/futopt`), for 1.x compatibility: it is what
    /// `@fugle/marketdata` 1.x returned (#307). `RestClient.baseUrl` is the prefix without it.
    #[napi(getter)]
    pub fn base_url(&self) -> String {
        self.inner.futopt().resolved_base_url()
    }
}

/// FutOpt intraday data client
#[napi]
pub struct FutOptIntradayClient {
    inner: marketdata_core::RestClient,
}

#[napi]
impl FutOptIntradayClient {
    /// Get intraday quote for a futures/options contract
    ///
    /// @param symbol - Contract symbol (e.g., "TXFC4" for TX futures, "TXO18000C4" for options)
    /// @returns Promise resolving to Quote object with current price and volume data
    ///
    /// @example
    /// ```javascript
    /// const client = new RestClient('your-api-key');
    /// const quote = await client.futopt.intraday.quote('TXFC4');
    /// console.log(quote.lastPrice);  // 17550.0
    /// console.log(quote.symbol);     // "TXFC4"
    /// ```
    #[napi(
        ts_return_type = "Promise<FutOptQuoteResponse>",
        ts_args_type = "symbol: string | RestFutOptIntradayQuoteParams"
    )]
    pub async fn quote(&self, symbol: Option<RestArg>, extra: Option<ExtraArg>) -> napi::Result<Settled> {
        if let Some(rejected) = unused_args(
            "futopt.intraday.quote",
            &["futopt", "intraday", "quote"],
            ("symbol", &symbol),
            &[],
            extra,
        ) {
            return Ok(rejected);
        }
        let symbol = match take!(RestArg::required("futopt.intraday.quote", symbol, "symbol")) {
            RestArg::Invalid(_) => unreachable!("refused by unused_args"),
            RestArg::Positional(value) => value,
            RestArg::Params(params) => return get_with_params(&self.inner, &["futopt", "intraday", "quote"], params).await,
        };

        let inner = self.inner.clone();

        let result = tokio::task::spawn_blocking(move || {
            inner.futopt().intraday().quote().symbol(&symbol).send()
        })
        .await
        .map_err(|e| napi::Error::from_reason(format!("Task error: {}", e)))?;

        Ok(Settled::from(result))
    }

    /// Get intraday ticker for a futures/options contract
    ///
    /// @param symbol - Contract symbol (e.g., "TXFC4")
    /// @returns Promise resolving to Ticker object with last trade info
    #[napi(
        ts_return_type = "Promise<FutOptTickerResponse>",
        ts_args_type = "symbol: string | RestFutOptIntradayTickerParams"
    )]
    pub async fn ticker(&self, symbol: Option<RestArg>, extra: Option<ExtraArg>) -> napi::Result<Settled> {
        if let Some(rejected) = unused_args(
            "futopt.intraday.ticker",
            &["futopt", "intraday", "ticker"],
            ("symbol", &symbol),
            &[],
            extra,
        ) {
            return Ok(rejected);
        }
        let symbol = match take!(RestArg::required("futopt.intraday.ticker", symbol, "symbol")) {
            RestArg::Invalid(_) => unreachable!("refused by unused_args"),
            RestArg::Positional(value) => value,
            RestArg::Params(params) => return get_with_params(&self.inner, &["futopt", "intraday", "ticker"], params).await,
        };

        let inner = self.inner.clone();

        let result = tokio::task::spawn_blocking(move || {
            inner.futopt().intraday().ticker().symbol(&symbol).send()
        })
        .await
        .map_err(|e| napi::Error::from_reason(format!("Task error: {}", e)))?;

        Ok(Settled::from(result))
    }

    /// Get intraday candles for a futures/options contract
    ///
    /// @param symbol - Contract symbol (e.g., "TXFC4")
    /// @param timeframe - Candle timeframe: "1", "5", "10", "15", "30", "60" (minutes)
    /// @returns Promise resolving to Candles response with OHLCV data
    #[napi(
        ts_return_type = "Promise<CandlesResponse>",
        ts_args_type = "symbol: string | RestFutOptIntradayCandlesParams, timeframe?: string | undefined | null"
    )]
    pub async fn candles(&self, symbol: Option<RestArg>, timeframe: Option<PosArg>, extra: Option<ExtraArg>) -> napi::Result<Settled> {
        if let Some(rejected) = unused_args(
            "futopt.intraday.candles",
            &["futopt", "intraday", "candles"],
            ("symbol", &symbol),
            &[("timeframe", timeframe.is_some())],
            extra,
        ) {
            return Ok(rejected);
        }
        let timeframe = take!(pos_string("futopt.intraday.candles", "timeframe", timeframe));
        let symbol = match take!(RestArg::required("futopt.intraday.candles", symbol, "symbol")) {
            RestArg::Invalid(_) => unreachable!("refused by unused_args"),
            RestArg::Positional(value) => value,
            RestArg::Params(params) => return get_with_params(&self.inner, &["futopt", "intraday", "candles"], params).await,
        };

        let inner = self.inner.clone();

        let result = tokio::task::spawn_blocking(move || {
            let futopt = inner.futopt();
            let intraday = futopt.intraday();
            let mut builder = intraday.candles().symbol(&symbol);
            if let Some(tf) = &timeframe {
                builder = builder.timeframe(tf);
            }
            builder.send()
        })
        .await
        .map_err(|e| napi::Error::from_reason(format!("Task error: {}", e)))?;

        Ok(Settled::from(result))
    }

    /// Get intraday trades for a futures/options contract
    ///
    /// @param symbol - Contract symbol (e.g., "TXFC4")
    /// @returns Promise resolving to Trades response with recent trade history
    #[napi(
        ts_return_type = "Promise<TradesResponse>",
        ts_args_type = "symbol: string | RestFutOptIntradayTradesParams"
    )]
    pub async fn trades(&self, symbol: Option<RestArg>, extra: Option<ExtraArg>) -> napi::Result<Settled> {
        if let Some(rejected) = unused_args(
            "futopt.intraday.trades",
            &["futopt", "intraday", "trades"],
            ("symbol", &symbol),
            &[],
            extra,
        ) {
            return Ok(rejected);
        }
        let symbol = match take!(RestArg::required("futopt.intraday.trades", symbol, "symbol")) {
            RestArg::Invalid(_) => unreachable!("refused by unused_args"),
            RestArg::Positional(value) => value,
            RestArg::Params(params) => return get_with_params(&self.inner, &["futopt", "intraday", "trades"], params).await,
        };

        let inner = self.inner.clone();

        let result = tokio::task::spawn_blocking(move || {
            inner.futopt().intraday().trades().symbol(&symbol).send()
        })
        .await
        .map_err(|e| napi::Error::from_reason(format!("Task error: {}", e)))?;

        Ok(Settled::from(result))
    }

    /// Get intraday volumes for a futures/options contract
    ///
    /// @param symbol - Contract symbol (e.g., "TXFC4")
    /// @returns Promise resolving to Volumes response with volume at each price level
    #[napi(
        ts_return_type = "Promise<VolumesResponse>",
        ts_args_type = "symbol: string | RestFutOptIntradayVolumesParams"
    )]
    pub async fn volumes(&self, symbol: Option<RestArg>, extra: Option<ExtraArg>) -> napi::Result<Settled> {
        if let Some(rejected) = unused_args(
            "futopt.intraday.volumes",
            &["futopt", "intraday", "volumes"],
            ("symbol", &symbol),
            &[],
            extra,
        ) {
            return Ok(rejected);
        }
        let symbol = match take!(RestArg::required("futopt.intraday.volumes", symbol, "symbol")) {
            RestArg::Invalid(_) => unreachable!("refused by unused_args"),
            RestArg::Positional(value) => value,
            RestArg::Params(params) => return get_with_params(&self.inner, &["futopt", "intraday", "volumes"], params).await,
        };

        let inner = self.inner.clone();

        let result = tokio::task::spawn_blocking(move || {
            inner.futopt().intraday().volumes().symbol(&symbol).send()
        })
        .await
        .map_err(|e| napi::Error::from_reason(format!("Task error: {}", e)))?;

        Ok(Settled::from(result))
    }

    /// Get batch ticker list for a FutOpt contract type
    ///
    /// @param type - Contract type: "FUTURE" or "OPTION"
    /// @param exchange - Optional exchange filter (e.g., "TAIFEX")
    /// @param afterHours - Query after-hours session data
    /// @param contractType - Optional contract type code: "I" / "R" / "B" / "C" / "S" / "E"
    /// @returns Promise resolving to an array of FutOpt ticker info objects
    #[napi(
        ts_return_type = "Promise<FutOptTickersResponse>",
        ts_args_type = "type: FutOptType | RestFutOptIntradayTickersParams, exchange?: string | undefined | null, afterHours?: boolean | undefined | null, contractType?: ContractType | undefined | null, isSpread?: boolean | undefined | null"
    )]
    pub async fn tickers(
        &self,
        typ: Option<RestArg>,
        exchange: Option<PosArg>,
        after_hours: Option<PosArg>,
        contract_type: Option<PosArg>,
        is_spread: Option<PosArg>,
        extra: Option<ExtraArg>,
    ) -> napi::Result<Settled> {
        if let Some(rejected) = unused_args(
            "futopt.intraday.tickers",
            &["futopt", "intraday", "tickers"],
            ("type", &typ),
            &[("exchange", exchange.is_some()), ("afterHours", after_hours.is_some()), ("contractType", contract_type.is_some()), ("isSpread", is_spread.is_some())],
            extra,
        ) {
            return Ok(rejected);
        }
        let exchange = take!(pos_string("futopt.intraday.tickers", "exchange", exchange));
        let after_hours = take!(pos_bool("futopt.intraday.tickers", "afterHours", after_hours));
        let contract_type = take!(pos_string("futopt.intraday.tickers", "contractType", contract_type));
        let is_spread = take!(pos_bool("futopt.intraday.tickers", "isSpread", is_spread));
        let typ = match take!(RestArg::required("futopt.intraday.tickers", typ, "type")) {
            RestArg::Invalid(_) => unreachable!("refused by unused_args"),
            RestArg::Positional(value) => value,
            RestArg::Params(params) => return get_with_params(&self.inner, &["futopt", "intraday", "tickers"], params).await,
        };

        use marketdata_core::models::futopt::{ContractType, FutOptType};

        let fut_opt_type = match typ.to_uppercase().as_str() {
            "FUTURE" => FutOptType::Future,
            "OPTION" => FutOptType::Option,
            _ => {
                return Err(napi::Error::from_reason(format!(
                    "Invalid type '{}': must be 'FUTURE' or 'OPTION'",
                    typ
                )))
            }
        };

        let ct_enum = if let Some(ct) = contract_type {
            Some(match ct.to_uppercase().as_str() {
                "I" | "INDEX" => ContractType::Index,
                "R" | "RATE" => ContractType::Rate,
                "B" | "BOND" => ContractType::Bond,
                "C" | "CURRENCY" => ContractType::Currency,
                "S" | "STOCK" => ContractType::Stock,
                "E" | "ETF" => ContractType::Etf,
                _ => {
                    return Err(napi::Error::from_reason(format!(
                        "Invalid contractType '{}': must be I/R/B/C/S/E",
                        ct
                    )))
                }
            })
        } else {
            None
        };

        let inner = self.inner.clone();

        let result = tokio::task::spawn_blocking(move || {
            let futopt = inner.futopt();
            let intraday = futopt.intraday();
            let mut builder = intraday.tickers().typ(fut_opt_type);
            if let Some(e) = &exchange {
                builder = builder.exchange(e);
            }
            if after_hours.unwrap_or(false) {
                builder = builder.after_hours();
            }
            if let Some(ct) = ct_enum {
                builder = builder.contract_type(ct);
            }
            if let Some(sp) = is_spread {
                builder = builder.is_spread(sp);
            }
            builder.send()
        })
        .await
        .map_err(|e| napi::Error::from_reason(format!("Task error: {}", e)))?;

        Ok(Settled::from(result))
    }

    /// Get product list for futures/options
    ///
    /// @param typ - Type: "FUTURE" or "OPTION" (required)
    /// @param contractType - Contract type filter (optional): "I" (index), "R" (rate), "B" (bond), "C" (currency), "S" (stock), "E" (ETF)
    /// @returns Promise resolving to Products response with available contracts
    #[napi(
        ts_return_type = "Promise<ProductsResponse>",
        ts_args_type = "type: FutOptType | RestFutOptIntradayProductsParams, contractType?: ContractType | undefined | null"
    )]
    pub async fn products(&self, typ: Option<RestArg>, contract_type: Option<PosArg>, extra: Option<ExtraArg>) -> napi::Result<Settled> {
        if let Some(rejected) = unused_args(
            "futopt.intraday.products",
            &["futopt", "intraday", "products"],
            ("type", &typ),
            &[("contractType", contract_type.is_some())],
            extra,
        ) {
            return Ok(rejected);
        }
        let contract_type = take!(pos_string("futopt.intraday.products", "contractType", contract_type));
        let typ = match take!(RestArg::required("futopt.intraday.products", typ, "type")) {
            RestArg::Invalid(_) => unreachable!("refused by unused_args"),
            RestArg::Positional(value) => value,
            RestArg::Params(params) => return get_with_params(&self.inner, &["futopt", "intraday", "products"], params).await,
        };

        use marketdata_core::models::futopt::{ContractType, FutOptType};

        // Parse typ parameter before spawn_blocking
        let fut_opt_type = match typ.to_uppercase().as_str() {
            "FUTURE" => FutOptType::Future,
            "OPTION" => FutOptType::Option,
            _ => {
                return Err(napi::Error::from_reason(format!(
                    "Invalid type '{}': must be 'FUTURE' or 'OPTION'",
                    typ
                )))
            }
        };

        // Parse optional contract type
        let ct_enum = if let Some(ct) = contract_type {
            Some(match ct.to_uppercase().as_str() {
                "I" | "INDEX" => ContractType::Index,
                "R" | "RATE" => ContractType::Rate,
                "B" | "BOND" => ContractType::Bond,
                "C" | "CURRENCY" => ContractType::Currency,
                "S" | "STOCK" => ContractType::Stock,
                "E" | "ETF" => ContractType::Etf,
                _ => {
                    return Err(napi::Error::from_reason(format!(
                        "Invalid contractType '{}': must be I/R/B/C/S/E",
                        ct
                    )))
                }
            })
        } else {
            None
        };

        let inner = self.inner.clone();

        let result = tokio::task::spawn_blocking(move || {
            let mut builder = inner.futopt().intraday().products().typ(fut_opt_type);
            if let Some(ct) = ct_enum {
                builder = builder.contract_type(ct);
            }
            builder.send()
        })
        .await
        .map_err(|e| napi::Error::from_reason(format!("Task error: {}", e)))?;

        Ok(Settled::from(result))
    }
}

/// FutOpt historical data client
#[napi]
pub struct FutOptHistoricalClient {
    inner: marketdata_core::RestClient,
}

/// FutOpt historical paths take a product code. Accept the API's own name for
/// it, `product`, as well as the legacy SDK's `symbol`; either way it becomes
/// the path segment rather than a query param.
#[napi]
impl FutOptHistoricalClient {
    /// Get historical candles for a futures/options product
    ///
    /// @param symbol - Product code (e.g., "TXF"); a contract code such as "TXFC4" returns 404
    /// @param from - Start date (YYYY-MM-DD)
    /// @param to - End date (YYYY-MM-DD)
    /// @param timeframe - Timeframe ("D", "W", "M", "1", "5", "10", "15", "30", "60")
    /// @param afterHours - Query the after-hours session
    /// @param contractMonth - "YYYYMM", or a continuous contract: "1!" (default), "2!", "3!"
    /// @param fields - Comma-separated fields, e.g. "open,high,low,close,volume"
    /// @param sort - "asc" or "desc"
    /// @returns Promise resolving to historical candles data
    #[napi(
        ts_return_type = "Promise<FutOptHistoricalCandlesResponse>",
        ts_args_type = "symbol: string | RestFutOptHistoricalCandlesParams, from?: string | undefined | null, to?: string | undefined | null, timeframe?: string | undefined | null, afterHours?: boolean | undefined | null, contractMonth?: string | undefined | null, fields?: string | undefined | null, sort?: 'asc' | 'desc' | undefined | null"
    )]
    #[allow(clippy::too_many_arguments, reason = "positional JS signature; the object form covers the same params")]
    pub async fn candles(
        &self,
        symbol: Option<RestArg>,
        from: Option<PosArg>,
        to: Option<PosArg>,
        timeframe: Option<PosArg>,
        after_hours: Option<PosArg>,
        contract_month: Option<PosArg>,
        fields: Option<PosArg>,
        sort: Option<PosArg>,
        extra: Option<ExtraArg>,
    ) -> napi::Result<Settled> {
        if let Some(rejected) = unused_args(
            "futopt.historical.candles",
            &["futopt", "historical", "candles"],
            ("symbol", &symbol),
            &[("from", from.is_some()), ("to", to.is_some()), ("timeframe", timeframe.is_some()), ("afterHours", after_hours.is_some()), ("contractMonth", contract_month.is_some()), ("fields", fields.is_some()), ("sort", sort.is_some())],
            extra,
        ) {
            return Ok(rejected);
        }
        let from = take!(pos_string("futopt.historical.candles", "from", from));
        let to = take!(pos_string("futopt.historical.candles", "to", to));
        let timeframe = take!(pos_string("futopt.historical.candles", "timeframe", timeframe));
        let after_hours = take!(pos_bool("futopt.historical.candles", "afterHours", after_hours));
        let contract_month = take!(pos_string("futopt.historical.candles", "contractMonth", contract_month));
        let fields = take!(pos_string("futopt.historical.candles", "fields", fields));
        let sort = take!(pos_string("futopt.historical.candles", "sort", sort));
        let symbol = match take!(RestArg::required("futopt.historical.candles", symbol, "symbol")) {
            RestArg::Invalid(_) => unreachable!("refused by unused_args"),
            RestArg::Positional(value) => value,
            RestArg::Params(params) => {
                return get_with_params(&self.inner, &["futopt", "historical", "candles"], params).await;
            }
        };

        let inner = self.inner.clone();

        let result = tokio::task::spawn_blocking(move || {
            let futopt = inner.futopt();
            let hist = futopt.historical();
            let mut builder = hist.candles().symbol(&symbol);
            if let Some(f) = from {
                builder = builder.from(&f);
            }
            if let Some(t) = to {
                builder = builder.to(&t);
            }
            if let Some(tf) = timeframe {
                builder = builder.timeframe(&tf);
            }
            if let Some(ah) = after_hours {
                builder = builder.after_hours(ah);
            }
            if let Some(cm) = contract_month {
                builder = builder.contract_month(&cm);
            }
            if let Some(f) = fields {
                builder = builder.fields(&f);
            }
            if let Some(s) = sort {
                builder = builder.sort(&s);
            }
            builder.send()
        })
        .await
        .map_err(|e| napi::Error::from_reason(format!("Task error: {}", e)))?;

        Ok(Settled::from(result))
    }

    /// Get one trading day's daily quotes for every contract month of a futures/options product
    ///
    /// @param symbol - Product code (e.g., "TXF"); a contract code such as "TXFC4" returns 404
    /// @param date - Trading date (YYYY-MM-DD); the server defaults to today
    /// @param afterHours - Query the after-hours session
    /// @returns Promise resolving to daily historical data
    #[napi(
        ts_return_type = "Promise<FutOptDailyResponse>",
        ts_args_type = "symbol: string | RestFutOptHistoricalDailyParams, date?: string | undefined | null, afterHours?: boolean | undefined | null"
    )]
    pub async fn daily(
        &self,
        symbol: Option<RestArg>,
        date: Option<PosArg>,
        after_hours: Option<PosArg>,
        extra: Option<ExtraArg>,
    ) -> napi::Result<Settled> {
        if let Some(rejected) = unused_args(
            "futopt.historical.daily",
            &["futopt", "historical", "daily"],
            ("symbol", &symbol),
            &[("date", date.is_some()), ("afterHours", after_hours.is_some())],
            extra,
        ) {
            return Ok(rejected);
        }
        let date = take!(pos_string("futopt.historical.daily", "date", date));
        let after_hours = take!(pos_bool("futopt.historical.daily", "afterHours", after_hours));
        let symbol = match take!(RestArg::required("futopt.historical.daily", symbol, "symbol")) {
            RestArg::Invalid(_) => unreachable!("refused by unused_args"),
            RestArg::Positional(value) => value,
            RestArg::Params(params) => {
                return get_with_params(&self.inner, &["futopt", "historical", "daily"], params).await;
            }
        };

        let inner = self.inner.clone();

        let result = tokio::task::spawn_blocking(move || {
            let futopt = inner.futopt();
            let hist = futopt.historical();
            let mut builder = hist.daily().symbol(&symbol);
            if let Some(d) = date {
                builder = builder.date(&d);
            }
            if let Some(ah) = after_hours {
                builder = builder.after_hours(ah);
            }
            builder.send()
        })
        .await
        .map_err(|e| napi::Error::from_reason(format!("Task error: {}", e)))?;

        Ok(Settled::from(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_options(api_key: &str) -> RestClientOptions {
        RestClientOptions {
            api_key: Some(api_key.to_string()),
            bearer_token: None,
            sdk_token: None,
            base_url: None,
            tls_root_cert_pem: None,
            tls_accept_invalid_certs: None,
        }
    }

    #[test]
    fn test_rest_client_creation_with_api_key() {
        let client = RestClient::from_options(make_options("test-api-key")).unwrap();
        // Verify client was created (compilation success is the test)
        let _ = client.stock();
        let _ = client.futopt();
    }

    #[test]
    fn test_rest_client_creation_with_bearer_token() {
        let options = RestClientOptions {
            api_key: None,
            bearer_token: Some("test-token".to_string()),
            sdk_token: None,
            base_url: None,
            tls_root_cert_pem: None,
            tls_accept_invalid_certs: None,
        };
        let client = RestClient::from_options(options).unwrap();
        let _ = client.stock();
    }

    #[test]
    fn test_rest_client_creation_with_base_url() {
        let options = RestClientOptions {
            api_key: Some("test-key".to_string()),
            bearer_token: None,
            sdk_token: None,
            base_url: Some("https://custom.api".to_string()),
            tls_root_cert_pem: None,
            tls_accept_invalid_certs: None,
        };
        let client = RestClient::from_options(options).unwrap();
        let _ = client.stock();
    }

    #[test]
    fn test_rest_client_no_auth_fails() {
        let options = RestClientOptions {
            api_key: None,
            bearer_token: None,
            sdk_token: None,
            base_url: None,
            tls_root_cert_pem: None,
            tls_accept_invalid_certs: None,
        };
        let result = RestClient::from_options(options);
        assert!(result.is_err());
        match result {
            Err(BuildError::Core(err)) => {
                assert_eq!(err.info().code, marketdata_core::error_code::CONFIG);
                assert!(err.to_string().contains("exactly one non-empty credential"));
            }
            _ => panic!("expected a core ConfigError"),
        }
    }

    #[test]
    fn test_rest_client_multiple_auth_fails() {
        let options = RestClientOptions {
            api_key: Some("key".to_string()),
            bearer_token: Some("token".to_string()),
            sdk_token: None,
            base_url: None,
            tls_root_cert_pem: None,
            tls_accept_invalid_certs: None,
        };
        let result = RestClient::from_options(options);
        assert!(result.is_err());
        match result {
            Err(BuildError::Core(err)) => {
                assert_eq!(err.info().code, marketdata_core::error_code::CONFIG);
                assert!(err.to_string().contains("exactly one non-empty credential"));
            }
            _ => panic!("expected a core ConfigError"),
        }
    }

    #[test]
    fn test_rest_client_blank_auth_fails() {
        for (api_key, bearer_token) in [(Some(""), None), (None, Some("   ")), (Some(""), Some(""))] {
            let options = RestClientOptions {
                api_key: api_key.map(String::from),
                bearer_token: bearer_token.map(String::from),
                sdk_token: None,
                base_url: None,
                tls_root_cert_pem: None,
                tls_accept_invalid_certs: None,
            };
            match RestClient::from_options(options) {
                Err(BuildError::Core(err)) => {
                    assert_eq!(err.info().code, marketdata_core::error_code::CONFIG)
                }
                _ => panic!("expected a core ConfigError"),
            }
        }
    }

    #[test]
    fn test_stock_client_chain() {
        let client = RestClient::from_options(make_options("test-api-key")).unwrap();
        let stock = client.stock();
        let _intraday = stock.intraday();
        let _historical = stock.historical();
        let _snapshot = stock.snapshot();
        let _technical = stock.technical();
        let _corporate_actions = stock.corporate_actions();
    }

    #[test]
    fn test_futopt_client_chain() {
        let client = RestClient::from_options(make_options("test-api-key")).unwrap();
        let futopt = client.futopt();
        let _intraday = futopt.intraday();
        let _historical = futopt.historical();
    }
}
