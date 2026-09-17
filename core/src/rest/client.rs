//! REST client for Fugle marketdata API

use super::auth::Auth;
use super::retry::{self, RetryPolicy};
use crate::errors::{HttpErrorContext, MarketDataError};
use super::error::{status_error, transport_error};
use crate::tls::{build_ureq_tls_config, TlsConfig};

/// Idle connections kept per host.
///
/// With too few idle slots, concurrent callers sharing a client (for example
/// `Promise.all` in Node or threads in Python) open a fresh TCP + TLS
/// connection for every overlapping request and leave sockets in TIME_WAIT.
/// ureq 2 kept one by default and ureq 3 keeps three.
/// Against the live API a new connection costs roughly 60 ms versus about
/// 20 ms on a reused one.
const MAX_IDLE_CONNECTIONS_PER_HOST: usize = 16;

/// Network timeout applied to each phase of a request.
const REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

/// Response type returned by the transport.
pub(crate) type HttpResponse = ureq::http::Response<ureq::Body>;

/// Main REST client with connection pooling.
///
/// Cloning the client is cheap: clones share the same connection pool, and
/// the client is `Send + Sync`, so one client can serve many threads.
///
/// # Connection Pooling
///
/// The underlying HTTP agent:
/// - Reuses TCP and TLS connections across requests
/// - Keeps up to 16 idle connections per host for concurrent callers
/// - Drops idle connections after 15 seconds
pub struct RestClient {
    agent: ureq::Agent,
    /// Credential header, validated once at construction. `Err` holds the
    /// message for a blank credential or one that is not a valid header
    /// value; it is reported from the first request so construction stays
    /// infallible.
    auth_header: Result<(&'static str, ureq::http::HeaderValue), String>,
    base_url: String,
    /// Optional retry policy. `None` (default) means each request is
    /// attempted exactly once and any error propagates to the caller.
    retry_policy: Option<RetryPolicy>,
    /// Rejection message from [`RestClient::base_url`], held until the first
    /// request so the builder chain stays infallible.
    ///
    /// Stored as a `String` rather than a `MarketDataError` because the error
    /// type isn't `Clone` and `execute` only has `&self`.
    config_error: Option<String>,
}

impl RestClient {
    /// Create a new REST client with authentication
    ///
    /// # Example
    /// ```
    /// use marketdata_core::{RestClient, Auth};
    ///
    /// let client = RestClient::new(Auth::SdkToken("my-token".to_string()));
    /// ```
    pub fn new(auth: Auth) -> Self {
        // Building a default rustls config can only realistically fail if the
        // crypto provider installs unexpectedly differently (extremely rare).
        // Panic at construction so consumers get a clear failure mode instead
        // of an opaque error on first request.
        Self::with_tls(auth, TlsConfig::default())
            .expect("default rustls config should build on supported platforms")
    }

    /// Create a REST client with custom TLS configuration (custom root CA
    /// or "accept invalid certs"). Prefer `new()` for production usage
    /// against public Fugle endpoints.
    ///
    /// Returns a `ConfigError` if the PEM in `tls.root_cert_pem` is malformed.
    ///
    /// # Errors
    /// Returns [`MarketDataError`] on transport, deserialization, validation,
    /// or non-2xx API failures.
    pub fn with_tls(auth: Auth, tls: TlsConfig) -> Result<Self, MarketDataError> {
        Self::with_tls_and_timeout(auth, tls, REQUEST_TIMEOUT)
    }

    /// `with_tls` with a custom network timeout. Crate-internal so tests can
    /// exercise the timeout path without waiting the full default.
    pub(crate) fn with_tls_and_timeout(
        auth: Auth,
        tls: TlsConfig,
        timeout: std::time::Duration,
    ) -> Result<Self, MarketDataError> {
        let config = ureq::Agent::config_builder()
            .tls_config(build_ureq_tls_config(&tls)?)
            // ureq 3 reads HTTP_PROXY / HTTPS_PROXY / ALL_PROXY by default;
            // the SDK has never done that, so keep proxies off explicitly.
            .proxy(None)
            // Error statuses come back as responses so their body can become
            // the error message.
            .http_status_as_error(false)
            .max_idle_connections_per_host(MAX_IDLE_CONNECTIONS_PER_HOST)
            .timeout_connect(Some(timeout))
            .timeout_send_request(Some(timeout))
            .timeout_send_body(Some(timeout))
            .timeout_recv_response(Some(timeout))
            .timeout_recv_body(Some(timeout))
            .build();

        let (name, value) = auth.header();
        let auth_header = match auth.validate() {
            Err(MarketDataError::ConfigError(message)) => Err(message),
            Err(err) => Err(err.to_string()),
            Ok(()) => ureq::http::HeaderValue::from_str(&value)
                .map(|mut v| {
                    v.set_sensitive(true);
                    (name, v)
                })
                .map_err(|_| {
                    format!("{name} credential contains characters not allowed in an HTTP header")
                }),
        };

        Ok(Self {
            agent: config.into(),
            auth_header,
            base_url: crate::urls::REST_BASE.to_string(),
            retry_policy: None,
            config_error: None,
        })
    }

    /// Enable transparent retry of failed requests.
    ///
    /// By default the client does not retry — observability use cases
    /// need real failures visible. With a [`RetryPolicy`] installed,
    /// errors for which [`MarketDataError::is_retryable`] returns `true`
    /// (HTTP 429, HTTP 5xx, transport timeouts and connection errors)
    /// are retried with exponential backoff plus jitter, up to
    /// `max_attempts` total attempts. Other errors propagate immediately.
    ///
    /// # Example
    /// ```
    /// use marketdata_core::{Auth, RestClient, RetryPolicy};
    ///
    /// let client = RestClient::new(Auth::SdkToken("t".into()))
    ///     .with_retry(RetryPolicy::conservative());
    /// ```
    pub fn with_retry(mut self, policy: RetryPolicy) -> Self {
        self.retry_policy = Some(policy);
        self
    }

    /// Send an authenticated GET, applying any installed [`RetryPolicy`].
    ///
    /// Every endpoint builder routes its request through here, so retry,
    /// status handling and error mapping stay in one place. Error statuses
    /// (4xx/5xx) are returned as `Err` with the response body as message.
    pub(crate) fn get(&self, url: &str) -> Result<HttpResponse, MarketDataError> {
        // `base_url` is an infallible builder setter, so a rejected prefix is
        // parked until here — the first point on the path that can report it.
        // Every endpoint routes its request through `execute`, so there is no
        // way to reach the network with a poisoned base URL.
        if let Some(message) = &self.config_error {
            return Err(MarketDataError::ConfigError(message.clone()));
        }

        match self.retry_policy {
            Some(policy) => retry::run(&policy, || self.send_get(url)),
            None => self.send_get(url),
        }
    }

    /// One GET attempt: a fresh request per call, so retries resend it.
    fn send_get(&self, url: &str) -> Result<HttpResponse, MarketDataError> {
        let (name, value) = self
            .auth_header
            .as_ref()
            .map_err(|message| MarketDataError::ConfigError(message.clone()))?;
        let mut response = self
            .agent
            .get(url)
            .header(*name, value)
            .call()
            .map_err(|e| transport_error(url, e))?;

        let status = response.status().as_u16();
        if status >= 400 {
            let headers: Vec<(String, String)> = response
                .headers()
                .iter()
                .map(|(name, value)| {
                    (name.as_str().to_string(), String::from_utf8_lossy(value.as_bytes()).into_owned())
                })
                .collect();
            let body = response.body_mut().read_to_string().ok();
            return Err(status_error(HttpErrorContext::new(status, body, headers)));
        }
        Ok(response)
    }

    /// Override the base URL (useful for testing or custom endpoints).
    ///
    /// `url` carries the **host and path prefix and nothing else** — the SDK
    /// appends the version segment. REST serves a single version, so unlike
    /// streaming there is no option to choose it with, but a version written
    /// into `url` is still rejected rather than silently doubled.
    ///
    /// # Example
    /// ```
    /// use marketdata_core::{RestClient, Auth};
    ///
    /// let client = RestClient::new(Auth::SdkToken("my-token".to_string()))
    ///     .base_url("https://custom.api.example.com/marketdata");
    ///
    /// assert_eq!(
    ///     client.resolved_base_url(),
    ///     "https://custom.api.example.com/marketdata/v1.0"
    /// );
    /// ```
    ///
    /// # Deferred rejection
    ///
    /// This setter is infallible so it stays chainable. A `url` that already
    /// ends in a version segment is parked and surfaces as
    /// [`MarketDataError::ConfigError`] from the first request made with this
    /// client. [`try_base_url`](Self::try_base_url) reports it immediately.
    ///
    /// # ⚠️ Breaking change in 0.8.0
    ///
    /// 0.6.0 through 0.7.x required `url` to *include* `/v1.0`. That form now
    /// fails. See `MIGRATION-0.8.md`.
    #[must_use]
    pub fn base_url(mut self, url: &str) -> Self {
        match crate::urls::with_version(url, crate::urls::API_VERSION, "") {
            Ok(resolved) => {
                self.base_url = resolved;
                self.config_error = None;
            }
            // Store the inner message, not the formatted error: `execute`
            // re-wraps it in `ConfigError`, and `err.to_string()` already
            // carries that prefix — keeping it would double it.
            Err(MarketDataError::ConfigError(message)) => self.config_error = Some(message),
            Err(err) => self.config_error = Some(err.to_string()),
        }
        self
    }

    /// Same as [`base_url`](Self::base_url), but reports a rejected prefix
    /// immediately instead of parking it until the first request.
    ///
    /// # Errors
    ///
    /// Returns [`MarketDataError::ConfigError`] if `url` already ends in a
    /// version segment.
    pub fn try_base_url(self, url: &str) -> Result<Self, MarketDataError> {
        let client = self.base_url(url);
        match &client.config_error {
            Some(message) => Err(MarketDataError::ConfigError(message.clone())),
            None => Ok(client),
        }
    }

    /// The prefix every request from this client is built on, fully resolved —
    /// host, path prefix and version segment. Endpoints are appended to it.
    ///
    /// The version segment is chosen by the SDK rather than written by the
    /// caller, so this is the only way to see what a client actually resolved
    /// to. If [`base_url`](Self::base_url) rejected its argument, this still
    /// reports the last accepted prefix — the rejection surfaces from the
    /// request itself.
    #[must_use]
    pub fn resolved_base_url(&self) -> &str {
        &self.base_url
    }

    /// Send a GET to `path` with an arbitrary query string and return the
    /// response body as-is.
    ///
    /// The typed builders under [`stock`](Self::stock) and
    /// [`futopt`](Self::futopt) only emit the query params they declare. This
    /// is the escape hatch for bindings that forward caller-supplied params
    /// verbatim, the way the official Node SDK does, so a param the API
    /// documents is reachable without an SDK release.
    ///
    /// Each `path` element is one URL path segment and each query key and
    /// value is percent-encoded, so a spread symbol such as `TXFC4/TXFD4`
    /// stays a single segment. Auth, retry and status handling are the same
    /// as for the typed builders.
    ///
    /// # Example
    /// ```no_run
    /// use marketdata_core::{RestClient, Auth};
    ///
    /// let client = RestClient::new(Auth::ApiKey("my-key".to_string()));
    /// let trades = client.get_json(
    ///     &["stock", "intraday", "trades", "2330"],
    ///     &[("limit", "5"), ("isTrial", "false")],
    /// )?;
    /// # Ok::<(), marketdata_core::MarketDataError>(())
    /// ```
    ///
    /// # Errors
    /// Same as the typed builders' `send()`: [`MarketDataError::ApiError`] on
    /// a non-2xx status, transport and timeout errors, and
    /// [`MarketDataError::Other`] if the body is not JSON.
    ///
    /// Hidden from the docs and the public-API baseline: it exists for the
    /// bindings, not as a supported Rust API, and bypasses every typed check.
    #[doc(hidden)]
    pub fn get_json<K, V>(
        &self,
        path: &[&str],
        query: &[(K, V)],
    ) -> Result<serde_json::Value, MarketDataError>
    where
        K: AsRef<str>,
        V: AsRef<str>,
    {
        let url = self.endpoint_url(path, query);
        let response = self.get(&url)?;
        super::read_json(response)
    }

    /// Build `{base}/{segment}/...?{key}={value}&...`, encoding every part.
    fn endpoint_url<K, V>(&self, path: &[&str], query: &[(K, V)]) -> String
    where
        K: AsRef<str>,
        V: AsRef<str>,
    {
        let mut url = self.base_url.clone();
        for segment in path {
            url.push('/');
            url.push_str(&super::encode_symbol(segment));
        }
        if !query.is_empty() {
            let query = url::form_urlencoded::Serializer::new(String::new())
                .extend_pairs(query.iter().map(|(k, v)| (k.as_ref(), v.as_ref())))
                .finish();
            url.push('?');
            url.push_str(&query);
        }
        url
    }

    /// Access stock-related endpoints
    ///
    /// # Example
    /// ```
    /// use marketdata_core::{RestClient, Auth};
    ///
    /// let client = RestClient::new(Auth::SdkToken("my-token".to_string()));
    /// let stock_client = client.stock();
    /// ```
    pub fn stock(&self) -> StockClient<'_> {
        StockClient { client: self }
    }

    /// Access FutOpt (futures and options) endpoints
    ///
    /// # Example
    /// ```
    /// use marketdata_core::{RestClient, Auth};
    ///
    /// let client = RestClient::new(Auth::SdkToken("my-token".to_string()));
    /// let futopt_client = client.futopt();
    /// ```
    pub fn futopt(&self) -> super::futopt::FutOptClient<'_> {
        super::futopt::FutOptClient { client: self }
    }

    /// Internal helper to get the base URL
    pub(crate) fn get_base_url(&self) -> &str {
        &self.base_url
    }
}

impl Clone for RestClient {
    /// Clone the RestClient, sharing the same connection pool
    ///
    /// Cloning is cheap because the agent shares its connection pool through an `Arc`.
    /// Multiple cloned clients will share the same connection pool.
    fn clone(&self) -> Self {
        Self {
            agent: self.agent.clone(),
            auth_header: self.auth_header.clone(),
            base_url: self.base_url.clone(),
            retry_policy: self.retry_policy,
            config_error: self.config_error.clone(),
        }
    }
}

/// Stock-related endpoints client
pub struct StockClient<'a> {
    client: &'a RestClient,
}

impl<'a> StockClient<'a> {
    /// Access intraday (real-time) endpoints
    ///
    /// # Example
    /// ```
    /// use marketdata_core::{RestClient, Auth};
    ///
    /// let client = RestClient::new(Auth::SdkToken("my-token".to_string()));
    /// let intraday = client.stock().intraday();
    /// ```
    pub fn intraday(&self) -> IntradayClient<'a> {
        IntradayClient {
            client: self.client,
        }
    }

    /// Access historical data endpoints
    ///
    /// # Example
    /// ```
    /// use marketdata_core::{RestClient, Auth};
    ///
    /// let client = RestClient::new(Auth::SdkToken("my-token".to_string()));
    /// let historical = client.stock().historical();
    /// ```
    pub fn historical(&self) -> HistoricalClient<'a> {
        HistoricalClient {
            client: self.client,
        }
    }

    /// Access technical indicator endpoints
    ///
    /// # Example
    /// ```
    /// use marketdata_core::{RestClient, Auth};
    ///
    /// let client = RestClient::new(Auth::SdkToken("my-token".to_string()));
    /// let technical = client.stock().technical();
    /// ```
    pub fn technical(&self) -> crate::rest::stock::technical::TechnicalClient<'a> {
        crate::rest::stock::technical::TechnicalClient::new(self.client)
    }

    /// Access snapshot endpoints for market-wide data
    ///
    /// # Example
    /// ```
    /// use marketdata_core::{RestClient, Auth};
    ///
    /// let client = RestClient::new(Auth::SdkToken("my-token".to_string()));
    /// let snapshot = client.stock().snapshot();
    /// ```
    pub fn snapshot(&self) -> crate::rest::stock::snapshot::SnapshotClient<'a> {
        crate::rest::stock::snapshot::SnapshotClient::new(self.client)
    }

    /// Access corporate actions endpoints (capital changes, dividends, IPO listings)
    ///
    /// # Example
    /// ```
    /// use marketdata_core::{RestClient, Auth};
    ///
    /// let client = RestClient::new(Auth::SdkToken("my-token".to_string()));
    /// let corporate_actions = client.stock().corporate_actions();
    /// ```
    pub fn corporate_actions(&self) -> CorporateActionsClient<'a> {
        CorporateActionsClient {
            client: self.client,
        }
    }

    /// Access ownership endpoints (ETF holdings)
    ///
    /// # Example
    /// ```
    /// use marketdata_core::{RestClient, Auth};
    ///
    /// let client = RestClient::new(Auth::SdkToken("my-token".to_string()));
    /// let ownership = client.stock().ownership();
    /// ```
    pub fn ownership(&self) -> OwnershipClient<'a> {
        OwnershipClient {
            client: self.client,
        }
    }
}

/// Ownership endpoints client
pub struct OwnershipClient<'a> {
    client: &'a RestClient,
}

impl<'a> OwnershipClient<'a> {
    /// Get the constituents an ETF held over a date range.
    ///
    /// # Example
    /// ```no_run
    /// use marketdata_core::{RestClient, Auth};
    ///
    /// let client = RestClient::new(Auth::SdkToken("my-token".to_string()));
    /// let holdings = client
    ///     .stock()
    ///     .ownership()
    ///     .etf_holdings()
    ///     .symbol("0050")
    ///     .send()?;
    /// # Ok::<(), marketdata_core::MarketDataError>(())
    /// ```
    pub fn etf_holdings(
        &self,
    ) -> crate::rest::stock::ownership::EtfHoldingsRequestBuilder<'_> {
        crate::rest::stock::ownership::EtfHoldingsRequestBuilder::new(self.client)
    }

    /// Get daily trading by the three major institutional investors
    /// (foreign, investment trust, dealer) over a date range.
    ///
    /// # Example
    /// ```no_run
    /// use marketdata_core::{RestClient, Auth};
    ///
    /// let client = RestClient::new(Auth::SdkToken("my-token".to_string()));
    /// let trades = client
    ///     .stock()
    ///     .ownership()
    ///     .institutional_trades()
    ///     .symbol("2330")
    ///     .send()?;
    /// # Ok::<(), marketdata_core::MarketDataError>(())
    /// ```
    pub fn institutional_trades(
        &self,
    ) -> crate::rest::stock::ownership::InstitutionalTradesRequestBuilder<'_> {
        crate::rest::stock::ownership::InstitutionalTradesRequestBuilder::new(self.client)
    }

    /// Get the monthly holdings and pledges disclosed by a company's directors
    /// and supervisors over a date range.
    ///
    /// # Example
    /// ```no_run
    /// use marketdata_core::{RestClient, Auth};
    ///
    /// let client = RestClient::new(Auth::SdkToken("my-token".to_string()));
    /// let holdings = client
    ///     .stock()
    ///     .ownership()
    ///     .director_holdings()
    ///     .symbol("2330")
    ///     .send()?;
    /// # Ok::<(), marketdata_core::MarketDataError>(())
    /// ```
    pub fn director_holdings(
        &self,
    ) -> crate::rest::stock::ownership::DirectorHoldingsRequestBuilder<'_> {
        crate::rest::stock::ownership::DirectorHoldingsRequestBuilder::new(self.client)
    }

    /// Get the weekly TDCC shareholder distribution by holding-size bracket
    /// over a date range.
    ///
    /// # Example
    /// ```no_run
    /// use marketdata_core::{RestClient, Auth};
    ///
    /// let client = RestClient::new(Auth::SdkToken("my-token".to_string()));
    /// let distribution = client
    ///     .stock()
    ///     .ownership()
    ///     .tdcc_distribution()
    ///     .symbol("2330")
    ///     .send()?;
    /// # Ok::<(), marketdata_core::MarketDataError>(())
    /// ```
    pub fn tdcc_distribution(
        &self,
    ) -> crate::rest::stock::ownership::TdccDistributionRequestBuilder<'_> {
        crate::rest::stock::ownership::TdccDistributionRequestBuilder::new(self.client)
    }
}

/// Corporate actions endpoints client
pub struct CorporateActionsClient<'a> {
    client: &'a RestClient,
}

impl<'a> CorporateActionsClient<'a> {
    /// Get capital structure changes
    ///
    /// # Example
    /// ```no_run
    /// use marketdata_core::{RestClient, Auth};
    ///
    /// let client = RestClient::new(Auth::SdkToken("my-token".to_string()));
    /// let changes = client.stock().corporate_actions().capital_changes().send()?;
    /// # Ok::<(), marketdata_core::MarketDataError>(())
    /// ```
    pub fn capital_changes(&self) -> crate::rest::stock::corporate_actions::CapitalChangesRequestBuilder<'_> {
        crate::rest::stock::corporate_actions::CapitalChangesRequestBuilder::new(self.client)
    }

    /// Get dividend announcements
    ///
    /// # Example
    /// ```no_run
    /// use marketdata_core::{RestClient, Auth};
    ///
    /// let client = RestClient::new(Auth::SdkToken("my-token".to_string()));
    /// let dividends = client.stock().corporate_actions().dividends().send()?;
    /// # Ok::<(), marketdata_core::MarketDataError>(())
    /// ```
    pub fn dividends(&self) -> crate::rest::stock::corporate_actions::DividendsRequestBuilder<'_> {
        crate::rest::stock::corporate_actions::DividendsRequestBuilder::new(self.client)
    }

    /// Get IPO listing applicants
    ///
    /// # Example
    /// ```no_run
    /// use marketdata_core::{RestClient, Auth};
    ///
    /// let client = RestClient::new(Auth::SdkToken("my-token".to_string()));
    /// let applicants = client.stock().corporate_actions().listing_applicants().send()?;
    /// # Ok::<(), marketdata_core::MarketDataError>(())
    /// ```
    pub fn listing_applicants(&self) -> crate::rest::stock::corporate_actions::ListingApplicantsRequestBuilder<'_> {
        crate::rest::stock::corporate_actions::ListingApplicantsRequestBuilder::new(self.client)
    }
}

/// Historical data endpoints client
pub struct HistoricalClient<'a> {
    client: &'a RestClient,
}

impl<'a> HistoricalClient<'a> {
    /// Get historical candles for a symbol
    ///
    /// # Example
    /// ```no_run
    /// use marketdata_core::{RestClient, Auth};
    ///
    /// let client = RestClient::new(Auth::SdkToken("my-token".to_string()));
    /// let candles = client.stock().historical().candles()
    ///     .symbol("2330")
    ///     .from("2024-01-01")
    ///     .to("2024-01-31")
    ///     .send()?;
    /// # Ok::<(), marketdata_core::MarketDataError>(())
    /// ```
    pub fn candles(&self) -> crate::rest::stock::historical::HistoricalCandlesRequestBuilder<'_> {
        crate::rest::stock::historical::HistoricalCandlesRequestBuilder::new(self.client)
    }

    /// Get historical stats for a symbol
    ///
    /// # Example
    /// ```no_run
    /// use marketdata_core::{RestClient, Auth};
    ///
    /// let client = RestClient::new(Auth::SdkToken("my-token".to_string()));
    /// let stats = client.stock().historical().stats()
    ///     .symbol("2330")
    ///     .send()?;
    /// # Ok::<(), marketdata_core::MarketDataError>(())
    /// ```
    pub fn stats(&self) -> crate::rest::stock::historical::StatsRequestBuilder<'_> {
        crate::rest::stock::historical::StatsRequestBuilder::new(self.client)
    }
}

/// Intraday (real-time) endpoints client
pub struct IntradayClient<'a> {
    client: &'a RestClient,
}

impl<'a> IntradayClient<'a> {
    /// Get intraday quote for a symbol
    ///
    /// # Example
    /// ```no_run
    /// use marketdata_core::{RestClient, Auth};
    ///
    /// let client = RestClient::new(Auth::SdkToken("my-token".to_string()));
    /// let quote = client.stock().intraday().quote().symbol("2330").send()?;
    /// # Ok::<(), marketdata_core::MarketDataError>(())
    /// ```
    pub fn quote(&self) -> crate::rest::stock::intraday::QuoteRequestBuilder<'_> {
        crate::rest::stock::intraday::QuoteRequestBuilder::new(self.client)
    }

    /// Get intraday ticker info for a symbol
    ///
    /// # Example
    /// ```no_run
    /// use marketdata_core::{RestClient, Auth};
    ///
    /// let client = RestClient::new(Auth::SdkToken("my-token".to_string()));
    /// let ticker = client.stock().intraday().ticker().symbol("2330").send()?;
    /// # Ok::<(), marketdata_core::MarketDataError>(())
    /// ```
    pub fn ticker(&self) -> crate::rest::stock::intraday::TickerRequestBuilder<'_> {
        crate::rest::stock::intraday::TickerRequestBuilder::new(self.client)
    }

    /// Get intraday tickers (batch list) for a security type
    ///
    /// # Example
    /// ```no_run
    /// use marketdata_core::{RestClient, Auth};
    ///
    /// let client = RestClient::new(Auth::SdkToken("my-token".to_string()));
    /// let tickers = client.stock().intraday().tickers().typ("EQUITY").send()?;
    /// # Ok::<(), marketdata_core::MarketDataError>(())
    /// ```
    pub fn tickers(&self) -> crate::rest::stock::intraday::TickersRequestBuilder<'_> {
        crate::rest::stock::intraday::TickersRequestBuilder::new(self.client)
    }

    /// Get intraday candles for a symbol
    ///
    /// # Example
    /// ```no_run
    /// use marketdata_core::{RestClient, Auth};
    ///
    /// let client = RestClient::new(Auth::SdkToken("my-token".to_string()));
    /// let candles = client.stock().intraday().candles().symbol("2330").timeframe("5").send()?;
    /// # Ok::<(), marketdata_core::MarketDataError>(())
    /// ```
    pub fn candles(&self) -> crate::rest::stock::intraday::CandlesRequestBuilder<'_> {
        crate::rest::stock::intraday::CandlesRequestBuilder::new(self.client)
    }

    /// Get intraday trades for a symbol
    ///
    /// # Example
    /// ```no_run
    /// use marketdata_core::{RestClient, Auth};
    ///
    /// let client = RestClient::new(Auth::SdkToken("my-token".to_string()));
    /// let trades = client.stock().intraday().trades().symbol("2330").send()?;
    /// # Ok::<(), marketdata_core::MarketDataError>(())
    /// ```
    pub fn trades(&self) -> crate::rest::stock::intraday::TradesRequestBuilder<'_> {
        crate::rest::stock::intraday::TradesRequestBuilder::new(self.client)
    }

    /// Get intraday volumes for a symbol
    ///
    /// # Example
    /// ```no_run
    /// use marketdata_core::{RestClient, Auth};
    ///
    /// let client = RestClient::new(Auth::SdkToken("my-token".to_string()));
    /// let volumes = client.stock().intraday().volumes().symbol("2330").send()?;
    /// # Ok::<(), marketdata_core::MarketDataError>(())
    /// ```
    pub fn volumes(&self) -> crate::rest::stock::intraday::VolumesRequestBuilder<'_> {
        crate::rest::stock::intraday::VolumesRequestBuilder::new(self.client)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rest_client_creation() {
        let client = RestClient::new(Auth::SdkToken("test-token".to_string()));
        assert_eq!(client.get_base_url(), "https://api.fugle.tw/marketdata/v1.0");
    }

    #[test]
    fn test_endpoint_url_encodes_segments_and_query() {
        let client = RestClient::new(Auth::SdkToken("t".to_string()));
        let url = client.endpoint_url(
            &["futopt", "intraday", "quote", "TXFC4/TXFD4"],
            &[("session", "afterhours"), ("a b", "x&y=z")],
        );
        assert_eq!(
            url,
            "https://api.fugle.tw/marketdata/v1.0/futopt/intraday/quote/TXFC4%2FTXFD4?session=afterhours&a+b=x%26y%3Dz"
        );
    }

    #[test]
    fn test_endpoint_url_without_query() {
        let client = RestClient::new(Auth::SdkToken("t".to_string()));
        let url = client.endpoint_url::<&str, &str>(&["stock", "corporate-actions", "dividends"], &[]);
        assert_eq!(url, "https://api.fugle.tw/marketdata/v1.0/stock/corporate-actions/dividends");
    }

    #[test]
    fn test_rest_client_custom_base_url_gets_version_appended() {
        // 0.8.0: the caller passes host + prefix; the SDK owns /v1.0.
        let client = RestClient::new(Auth::SdkToken("test-token".to_string()))
            .base_url("https://custom.example.com");
        assert_eq!(client.get_base_url(), "https://custom.example.com/v1.0");
        assert_eq!(client.resolved_base_url(), "https://custom.example.com/v1.0");
    }

    #[test]
    fn test_rest_client_base_url_strips_trailing_slashes() {
        let client = RestClient::new(Auth::SdkToken("t".to_string()))
            .base_url("https://custom.example.com/marketdata///");
        assert_eq!(
            client.resolved_base_url(),
            "https://custom.example.com/marketdata/v1.0"
        );
    }

    #[test]
    fn test_legacy_0_6_base_url_is_rejected_on_request() {
        // 0.6.0-0.7.x required base_url to INCLUDE /v1.0. That form is now
        // rejected — but the setter stays chainable, so the rejection is
        // parked until the request. Pins the deferred-error contract.
        let client = RestClient::new(Auth::SdkToken("t".to_string()))
            .base_url("https://api.fugle.tw/marketdata/v1.0"); // <-- 0.6-era form

        let msg = match client.get("https://example.invalid/unused") {
            Err(err) => err.to_string(),
            Ok(_) => panic!("a poisoned base_url must not reach the network"),
        };
        assert!(msg.contains("/v1.0"), "names the offending segment: {msg}");
        assert!(
            msg.contains("'https://api.fugle.tw/marketdata'"),
            "names the prefix to use instead: {msg}"
        );
    }

    #[test]
    fn test_try_base_url_reports_rejection_immediately() {
        let err = RestClient::new(Auth::SdkToken("t".to_string()))
            .try_base_url("https://api.fugle.tw/marketdata/v1.0")
            .err()
            .expect("0.6-era base_url must be rejected");
        let msg = err.to_string();
        assert!(msg.contains("must not include a version segment"));
        assert_eq!(
            msg.matches("Configuration error").count(),
            1,
            "the stored message must not carry its own prefix — \
             `execute` adds one: {msg}"
        );
    }

    #[test]
    fn test_try_base_url_accepts_host_prefix() {
        let client = RestClient::new(Auth::SdkToken("t".to_string()))
            .try_base_url("https://staging.fugle.tw/marketdata")
            .expect("host + prefix is the accepted form");
        assert_eq!(
            client.resolved_base_url(),
            "https://staging.fugle.tw/marketdata/v1.0"
        );
    }

    #[test]
    fn test_default_client_has_no_config_error() {
        // The default base URL already carries /v1.0, but it is set directly
        // rather than through `base_url`, so it must not self-reject.
        let client = RestClient::new(Auth::SdkToken("t".to_string()));
        assert!(client.config_error.is_none());
        assert_eq!(client.resolved_base_url(), "https://api.fugle.tw/marketdata/v1.0");
    }

    #[test]
    fn test_stock_client_creation() {
        let client = RestClient::new(Auth::ApiKey("test-key".to_string()));
        let stock_client = client.stock();
        assert_eq!(stock_client.client.get_base_url(), "https://api.fugle.tw/marketdata/v1.0");
    }

    #[test]
    fn test_intraday_client_creation() {
        let client = RestClient::new(Auth::BearerToken("test-bearer".to_string()));
        let intraday = client.stock().intraday();
        assert_eq!(intraday.client.get_base_url(), "https://api.fugle.tw/marketdata/v1.0");
    }

    #[test]
    fn test_chained_client_access() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let _intraday = client.stock().intraday();
        // Compilation success proves the chaining works
    }

    #[test]
    fn test_auth_types() {
        // Test all three auth types
        let _client1 = RestClient::new(Auth::ApiKey("key".to_string()));
        let _client2 = RestClient::new(Auth::BearerToken("token".to_string()));
        let _client3 = RestClient::new(Auth::SdkToken("sdk".to_string()));
    }

    #[test]
    fn test_client_clone() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let cloned = client.clone();

        // Cloned client should have same base URL and auth
        assert_eq!(client.get_base_url(), cloned.get_base_url());
    }

    #[test]
    fn test_connection_pool_sharing() {
        // Create client with connection pool
        let client = RestClient::new(Auth::SdkToken("test".to_string()));

        // Clone shares the same connection pool (via Arc in ureq::Agent)
        let cloned = client.clone();

        // Both clients should be usable
        let _stock1 = client.stock().intraday();
        let _stock2 = cloned.stock().intraday();

        // Compilation and execution success proves connection pool works
    }

    #[test]
    fn test_custom_base_url_preserved_in_clone() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()))
            .base_url("https://custom.example.com");

        let cloned = client.clone();
        assert_eq!(cloned.get_base_url(), "https://custom.example.com/v1.0");
    }

    #[test]
    fn test_config_error_survives_clone() {
        // A poisoned client must not launder its rejection through a clone.
        let client = RestClient::new(Auth::SdkToken("t".to_string()))
            .base_url("https://custom.example.com/v1.0");
        let cloned = client.clone();

        assert!(cloned.get("https://example.invalid/unused").is_err());
    }

    #[test]
    fn test_futopt_client_creation() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let futopt = client.futopt();
        assert_eq!(futopt.client.get_base_url(), "https://api.fugle.tw/marketdata/v1.0");
    }

    #[test]
    fn test_futopt_intraday_client_creation() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let intraday = client.futopt().intraday();
        assert_eq!(intraday.client.get_base_url(), "https://api.fugle.tw/marketdata/v1.0");
    }

    #[test]
    fn test_futopt_chained_client_access() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let _intraday = client.futopt().intraday();
        // Compilation success proves the chaining works
    }

    #[test]
    fn test_both_stock_and_futopt() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));

        // Both stock and futopt should be accessible from the same client
        let _stock = client.stock().intraday();
        let _futopt = client.futopt().intraday();
    }

    #[test]
    fn test_corporate_actions_client_creation() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        let corporate_actions = client.stock().corporate_actions();
        assert_eq!(corporate_actions.client.get_base_url(), "https://api.fugle.tw/marketdata/v1.0");
    }

    #[test]
    fn test_corporate_actions_chained_access() {
        let client = RestClient::new(Auth::SdkToken("test".to_string()));
        // Test that all corporate actions endpoints are accessible
        let _capital_changes = client.stock().corporate_actions().capital_changes();
        let _dividends = client.stock().corporate_actions().dividends();
        let _listing_applicants = client.stock().corporate_actions().listing_applicants();
    }
}
