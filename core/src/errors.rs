//! Error types for marketdata-core
//!
//! Error code ranges:
//! - 1000-1999: Client errors (bad input, deserialization)
//! - 2000-2999: Server/API errors (auth, connection, HTTP)
//! - 3000-3999: Network errors (timeout, WebSocket)
//! - 9000-9999: Internal errors (unexpected failures)
//!
//! Every code is listed in [`error_code`]; `docs/errors.md` is the
//! cross-language reference built on [`ErrorInfo`].

use std::collections::BTreeMap;
use std::time::Duration;
use thiserror::Error;

/// Refined classification of a [`MarketDataError::WebSocketError`].
///
/// Mirrors `tungstenite::Error`'s own categorization without leaking the
/// upstream dependency type. Returned by pattern-matching the structured
/// `kind` field on the variant.
///
/// The enum is `#[non_exhaustive]` so future `tungstenite` releases or
/// new error sources can add variants without breaking exhaustive matches.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WebSocketErrorKind {
    /// Protocol-level violation: malformed frame, illegal state transition,
    /// reserved bits set. Never retryable — indicates an SDK / version
    /// mismatch or a server-side bug.
    Protocol,
    /// Frame exceeded the configured `max_message_size` / `max_frame_size`.
    /// Never retryable; the producer is misbehaving.
    Capacity,
    /// UTF-8 decoding failed on a text frame. Never retryable; the producer
    /// emitted invalid bytes.
    Utf8,
    /// TLS / certificate failure during the WebSocket handshake. Treated as
    /// authentication-adjacent — never retryable without operator action.
    Tls,
    /// Transport IO failure: connection reset, EOF, write error, etc.
    /// Retryable with backoff.
    Io,
    /// HTTP error during the WebSocket upgrade. `u16` is the status code.
    ///
    /// # Status code mapping
    ///
    /// Authoritative grid for monitor / incident-classifier code that
    /// branches on `WebSocketErrorKind::Http(_)`. [`MarketDataError::source_kind`]
    /// and [`MarketDataError::is_retryable`] honour this exact mapping; the
    /// `#[cfg(test)]` `http_mapping_consistency` module in this file pins
    /// the doc-vs-impl contract so silent drift fails CI.
    ///
    /// | Status range | [`ErrorKind`] | [`is_retryable`](MarketDataError::is_retryable) |
    /// |---|---|---|
    /// | `401`, `403` | [`Auth`](ErrorKind::Auth) | `false` |
    /// | `429` | [`RateLimit`](ErrorKind::RateLimit) | `true` |
    /// | `500..=599` | [`Network`](ErrorKind::Network) | `true` |
    /// | other 4xx (e.g. `404`) | [`Client`](ErrorKind::Client) | `false` |
    /// | anything else | [`Client`](ErrorKind::Client) | `false` |
    ///
    /// `Auth` and `Client` failures are non-retryable because the upstream
    /// rejection will not change between attempts; `RateLimit` and `5xx`
    /// retry with the configured backoff. Other 4xx codes (including `404`)
    /// surface as `Client` rather than `Network` because the server has
    /// stated, conclusively, that the request is wrong.
    Http(u16),
    /// Anything `tungstenite` adds in the future, or an error we can't
    /// classify. Retryable (conservative default).
    Other,
}

/// Coarse-grained classification of the source of a [`MarketDataError`].
///
/// Returned by [`MarketDataError::source_kind`] so downstream code can
/// branch on the *category* of failure (network glitch vs SDK / protocol
/// bug vs auth vs rate-limit vs caller-side validation) without
/// pattern-matching every variant or string-matching the embedded `msg`.
///
/// `#[non_exhaustive]` so future variants are non-breaking.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorKind {
    /// Transport-level transient failure: connection reset, timeout,
    /// heartbeat gap, server outage (5xx). Generally safe to retry with
    /// backoff.
    Network,
    /// Protocol-level violation or unclassified WebSocket failure. Indicates
    /// an SDK / version mismatch or a server-side bug; retry is unlikely
    /// to help.
    ///
    /// 0.5.1 maps **every** [`MarketDataError::WebSocketError`] to this
    /// kind because the variant is currently string-only. 0.6.0 refines
    /// the mapping when `WebSocketErrorKind` lands — IO failures will move
    /// to [`ErrorKind::Network`] and TLS failures to [`ErrorKind::Auth`].
    Protocol,
    /// Authentication / authorization failure: bad credentials, 401/403,
    /// expired token, TLS cert failure. Human intervention required.
    Auth,
    /// Server is rejecting requests because the caller is exceeding its
    /// rate budget (HTTP 429). Distinct from [`ErrorKind::Network`] —
    /// the correct response is to *reduce* request volume, not to assume
    /// the upstream is degraded. Adding parallel retries makes this
    /// strictly worse.
    RateLimit,
    /// Caller-side problem: invalid input, configuration error, client
    /// already closed, serialization failure, non-auth/non-throttle 4xx.
    /// The SDK can't recover from the caller's request without changes
    /// from the caller's side.
    Client,
}

impl ErrorKind {
    /// Stable lowercase name used by every binding: `"network"`,
    /// `"protocol"`, `"auth"`, `"rate_limit"` or `"client"`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Network => "network",
            Self::Protocol => "protocol",
            Self::Auth => "auth",
            Self::RateLimit => "rate_limit",
            Self::Client => "client",
        }
    }
}

impl std::fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Every error code the SDK reports, in core and in the bindings.
///
/// Values never change once released. New codes are added within the range
/// of their category (see the module docs); `docs/errors.md` lists them for
/// all languages.
pub mod error_code {
    /// [`MarketDataError::InvalidSymbol`](super::MarketDataError::InvalidSymbol).
    pub const INVALID_SYMBOL: i32 = 1001;
    /// [`MarketDataError::DeserializationError`](super::MarketDataError::DeserializationError),
    /// also a WebSocket frame that could not be parsed.
    pub const DESERIALIZATION: i32 = 1002;
    /// [`MarketDataError::RuntimeError`](super::MarketDataError::RuntimeError).
    pub const RUNTIME: i32 = 1003;
    /// [`MarketDataError::ConfigError`](super::MarketDataError::ConfigError).
    pub const CONFIG: i32 = 1004;
    /// [`MarketDataError::InvalidParameter`](super::MarketDataError::InvalidParameter).
    pub const INVALID_PARAMETER: i32 = 1005;
    /// [`MarketDataError::ConnectionError`](super::MarketDataError::ConnectionError).
    pub const CONNECTION: i32 = 2001;
    /// [`MarketDataError::AuthError`](super::MarketDataError::AuthError).
    pub const AUTH: i32 = 2002;
    /// [`MarketDataError::ApiError`](super::MarketDataError::ApiError).
    pub const API: i32 = 2003;
    /// [`MarketDataError::ClientClosed`](super::MarketDataError::ClientClosed),
    /// also a Node `connect()` aborted by `disconnect()`.
    pub const CLIENT_CLOSED: i32 = 2010;
    /// Node only: WebSocket `connect()` called while connected or connecting.
    pub const ALREADY_CONNECTED: i32 = 2011;
    /// [`MarketDataError::TimeoutError`](super::MarketDataError::TimeoutError).
    pub const TIMEOUT: i32 = 3001;
    /// [`MarketDataError::WebSocketError`](super::MarketDataError::WebSocketError).
    pub const WEBSOCKET: i32 = 3002;
    /// [`MarketDataError::HeartbeatTimeout`](super::MarketDataError::HeartbeatTimeout).
    pub const HEARTBEAT_TIMEOUT: i32 = 3003;
    /// [`MarketDataError::Other`](super::MarketDataError::Other).
    pub const OTHER: i32 = 9999;
    /// Node and Python: a binding's WebSocket worker thread panicked.
    pub const THREAD_PANIC: i32 = -1;
}

/// Header carrying the server-assigned request id, when the server sends one.
const REQUEST_ID_HEADER: &str = "x-request-id";

/// Response headers never kept in [`HttpErrorContext`]: they can carry
/// session credentials, and errors end up in logs.
const EXCLUDED_HEADERS: &[&str] = &["set-cookie"];

/// The HTTP response behind an error: status, raw body and headers.
///
/// Attached to [`MarketDataError::ApiError`] and to a REST
/// [`MarketDataError::AuthError`] (401 / 403) so a failed request can be
/// traced with the backend.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpErrorContext {
    /// HTTP status code.
    pub status: u16,
    /// Response body as received; `None` when it could not be read as text.
    pub body: Option<String>,
    /// Response headers with lowercase names, except `set-cookie`. Repeated
    /// headers are joined with `", "`.
    pub headers: BTreeMap<String, String>,
}

impl HttpErrorContext {
    /// Build a context. Header names are lowercased, repeated headers joined
    /// with `", "`, and `set-cookie` dropped.
    pub fn new<I, K, V>(status: u16, body: Option<String>, headers: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        let mut map: BTreeMap<String, String> = BTreeMap::new();
        for (name, value) in headers {
            let name = name.as_ref().to_ascii_lowercase();
            if EXCLUDED_HEADERS.contains(&name.as_str()) {
                continue;
            }
            map.entry(name)
                .and_modify(|joined| {
                    joined.push_str(", ");
                    joined.push_str(value.as_ref());
                })
                .or_insert_with(|| value.as_ref().to_string());
        }
        Self { status, body, headers: map }
    }

    /// Value of header `name` (case-insensitive).
    #[must_use]
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(&name.to_ascii_lowercase()).map(String::as_str)
    }

    /// The `x-request-id` header, if the server sent one.
    #[must_use]
    pub fn request_id(&self) -> Option<&str> {
        self.header(REQUEST_ID_HEADER)
    }
}

/// The cross-language view of an error: the fields every binding exposes
/// under the same names (cased per language).
///
/// Built from a [`MarketDataError`] with [`MarketDataError::info`], or with
/// [`ErrorInfo::new`] for errors that only exist in a binding.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorInfo {
    /// Numeric code from [`error_code`].
    pub code: i32,
    /// Category of the failure.
    pub source_kind: ErrorKind,
    /// Human-readable message.
    pub message: String,
    /// HTTP status, when the error came from an HTTP response (REST, or the
    /// WebSocket upgrade).
    pub status: Option<u16>,
    /// Raw HTTP response body (REST only).
    pub body: Option<String>,
    /// Server-assigned request id (`x-request-id`), when present.
    pub request_id: Option<String>,
    /// HTTP response headers (REST only; empty otherwise).
    pub headers: BTreeMap<String, String>,
}

impl ErrorInfo {
    /// An error without HTTP details.
    pub fn new(code: i32, source_kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            code,
            source_kind,
            message: message.into(),
            status: None,
            body: None,
            request_id: None,
            headers: BTreeMap::new(),
        }
    }

    fn with_http(mut self, http: &HttpErrorContext) -> Self {
        self.status = Some(http.status);
        self.body = http.body.clone();
        self.request_id = http.request_id().map(str::to_string);
        self.headers = http.headers.clone();
        self
    }
}

/// Main error type for marketdata-core operations
#[derive(Error, Debug)]
pub enum MarketDataError {
    /// Invalid symbol format or unsupported symbol
    #[error("Invalid symbol: {symbol}")]
    InvalidSymbol {
        /// The offending symbol string that failed validation.
        symbol: String,
    },

    /// Invalid or missing parameter
    #[error("Invalid parameter '{name}': {reason}")]
    InvalidParameter {
        /// Parameter name that failed validation.
        name: String,
        /// Human-readable explanation of why the parameter was rejected.
        reason: String,
    },

    /// JSON deserialization failed
    #[error("Deserialization failed: {source}")]
    DeserializationError {
        /// Underlying `serde_json` error.
        #[from]
        source: serde_json::Error,
    },

    /// Runtime operation failed
    #[error("Runtime error: {msg}")]
    RuntimeError {
        /// Diagnostic message describing the runtime failure.
        msg: String,
    },

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigError(
        /// Diagnostic message identifying the misconfiguration.
        String,
    ),

    /// Connection to server failed
    #[error("Connection error: {msg}")]
    ConnectionError {
        /// Diagnostic message describing the connection failure.
        msg: String,
    },

    /// Authentication failed
    #[error("Authentication error: {msg}")]
    AuthError {
        /// Diagnostic message describing the authentication failure.
        msg: String,
        /// The HTTP response, for a REST request rejected with 401 / 403.
        /// `None` for a WebSocket authentication rejection.
        http: Option<Box<HttpErrorContext>>,
    },

    /// API returned error response
    #[error("API error (status {status}): {message}")]
    ApiError {
        /// HTTP status code returned by the server.
        status: u16,
        /// Response body, or `HTTP <status>` when it could not be read.
        message: String,
        /// The HTTP response (its `status` equals `status`). `None` only for
        /// errors not built from a response.
        http: Option<Box<HttpErrorContext>>,
    },

    /// Operation timed out
    #[error("Timeout error: {operation}")]
    TimeoutError {
        /// Human-readable name of the operation that timed out.
        operation: String,
    },

    /// WebSocket error
    #[error("WebSocket error ({kind:?}): {msg}")]
    WebSocketError {
        /// Structured classification of the underlying WebSocket failure.
        /// Branch on this rather than substring-matching `msg` for
        /// programmatic decision-making (retry, alert, fail fast).
        kind: WebSocketErrorKind,
        /// Diagnostic message describing the WebSocket failure.
        msg: String,
    },

    /// Inbound activity timed out: no frame received within the
    /// configured `heartbeat_timeout` window.
    #[error("Heartbeat timeout: no inbound frames for {elapsed:?}")]
    HeartbeatTimeout {
        /// Wall-clock interval that elapsed since the last inbound frame.
        elapsed: Duration,
    },

    /// Client has been closed and cannot be reused
    #[error("Client already closed")]
    ClientClosed,

    /// Other unexpected errors
    #[error(transparent)]
    Other(
        /// Underlying error wrapped via `anyhow`.
        #[from]
        anyhow::Error,
    ),
}

impl From<tungstenite::Error> for MarketDataError {
    fn from(err: tungstenite::Error) -> Self {
        use tungstenite::Error as WsError;

        // Map each upstream variant to the right `WebSocketErrorKind`.
        // Previous behaviour collapsed everything into either
        // `ConnectionError` (IO), `WebSocketError` (protocol/capacity), or
        // `AuthError` (TLS/401/403) which conflated retry-policy with
        // fault-source. 0.6.0 retains `WebSocketError` as the single carrier
        // and exposes the source via the `kind` field.
        let (kind, msg) = match err {
            WsError::ConnectionClosed | WsError::AlreadyClosed | WsError::Io(_) => (
                WebSocketErrorKind::Io,
                format!("WebSocket transport error: {}", err),
            ),
            WsError::Protocol(_) => (
                WebSocketErrorKind::Protocol,
                format!("WebSocket protocol violation: {}", err),
            ),
            WsError::Capacity(_) => (
                WebSocketErrorKind::Capacity,
                format!("WebSocket capacity exceeded: {}", err),
            ),
            WsError::Utf8(_) => (
                WebSocketErrorKind::Utf8,
                format!("WebSocket UTF-8 decode failure: {}", err),
            ),
            WsError::Tls(_) => (
                WebSocketErrorKind::Tls,
                format!("TLS/certificate error: {}", err),
            ),
            WsError::Http(response) => {
                let status = response.status().as_u16();
                (
                    WebSocketErrorKind::Http(status),
                    format!("HTTP {} during WebSocket handshake", status),
                )
            }
            _ => (
                WebSocketErrorKind::Other,
                format!("WebSocket error: {}", err),
            ),
        };
        Self::WebSocketError { kind, msg }
    }
}

impl MarketDataError {
    /// Coarse-grained classification of the source of this error.
    ///
    /// Returns one of [`ErrorKind::Network`], [`ErrorKind::Protocol`],
    /// [`ErrorKind::Auth`], [`ErrorKind::RateLimit`], or [`ErrorKind::Client`]
    /// so downstream code can branch on category without pattern-matching
    /// every variant.
    ///
    /// # Mapping
    ///
    /// | `MarketDataError` variant | `ErrorKind` |
    /// |---|---|
    /// | `ConnectionError`, `TimeoutError`, `HeartbeatTimeout` | `Network` |
    /// | `WebSocketError { kind: Protocol \| Capacity \| Utf8 \| Other }` | `Protocol` |
    /// | `WebSocketError { kind: Tls }` | `Auth` |
    /// | `WebSocketError { kind: Io }` | `Network` |
    /// | `WebSocketError { kind: Http(_) }` | see [`WebSocketErrorKind::Http`] for the status-code mapping table |
    /// | `AuthError`, `ApiError { status: 401 \| 403 }` | `Auth` |
    /// | `ApiError { status: 429 }` | `RateLimit` |
    /// | `ApiError { status: 500..=599 }` | `Network` |
    /// | `ApiError { status: other 4xx }` | `Client` |
    /// | `InvalidSymbol`, `InvalidParameter`, `ConfigError`, `DeserializationError`, `ClientClosed` | `Client` |
    /// | `RuntimeError`, `Other` | `Client` |
    #[must_use]
    pub fn source_kind(&self) -> ErrorKind {
        match self {
            Self::ConnectionError { .. }
            | Self::TimeoutError { .. }
            | Self::HeartbeatTimeout { .. } => ErrorKind::Network,
            Self::WebSocketError { kind, .. } => match kind {
                WebSocketErrorKind::Protocol
                | WebSocketErrorKind::Capacity
                | WebSocketErrorKind::Utf8
                | WebSocketErrorKind::Other => ErrorKind::Protocol,
                WebSocketErrorKind::Tls => ErrorKind::Auth,
                WebSocketErrorKind::Io => ErrorKind::Network,
                WebSocketErrorKind::Http(status) => match *status {
                    401 | 403 => ErrorKind::Auth,
                    429 => ErrorKind::RateLimit,
                    500..=599 => ErrorKind::Network,
                    _ => ErrorKind::Client,
                },
            },
            Self::AuthError { .. } => ErrorKind::Auth,
            Self::ApiError { status, .. } => match *status {
                401 | 403 => ErrorKind::Auth,
                429 => ErrorKind::RateLimit,
                500..=599 => ErrorKind::Network,
                _ => ErrorKind::Client,
            },
            Self::InvalidSymbol { .. }
            | Self::InvalidParameter { .. }
            | Self::ConfigError(_)
            | Self::DeserializationError { .. }
            | Self::ClientClosed
            | Self::RuntimeError { .. }
            | Self::Other(_) => ErrorKind::Client,
        }
    }

    /// The cross-language view of this error; see [`ErrorInfo`].
    #[must_use]
    pub fn info(&self) -> ErrorInfo {
        let info = ErrorInfo::new(self.to_error_code(), self.source_kind(), self.to_string());
        match self {
            Self::ApiError { http: Some(http), .. } | Self::AuthError { http: Some(http), .. } => {
                info.with_http(http)
            }
            Self::ApiError { status, .. }
            | Self::WebSocketError { kind: WebSocketErrorKind::Http(status), .. } => ErrorInfo {
                status: Some(*status),
                ..info
            },
            _ => info,
        }
    }

    /// Get numeric error code for FFI consumers
    pub fn to_error_code(&self) -> i32 {
        match self {
            Self::InvalidSymbol { .. } => error_code::INVALID_SYMBOL,
            Self::InvalidParameter { .. } => error_code::INVALID_PARAMETER,
            Self::DeserializationError { .. } => error_code::DESERIALIZATION,
            Self::RuntimeError { .. } => error_code::RUNTIME,
            Self::ConfigError(_) => error_code::CONFIG,
            Self::ConnectionError { .. } => error_code::CONNECTION,
            Self::AuthError { .. } => error_code::AUTH,
            Self::ApiError { .. } => error_code::API,
            Self::TimeoutError { .. } => error_code::TIMEOUT,
            Self::WebSocketError { .. } => error_code::WEBSOCKET,
            Self::HeartbeatTimeout { .. } => error_code::HEARTBEAT_TIMEOUT,
            Self::ClientClosed => error_code::CLIENT_CLOSED,
            Self::Other(_) => error_code::OTHER,
        }
    }

    /// Check if error is retryable.
    ///
    /// # WebSocket retry verdict (0.6.0+)
    ///
    /// Refined to honour [`WebSocketErrorKind`]:
    ///
    /// | Kind | Retryable |
    /// |---|---|
    /// | `Protocol`, `Capacity`, `Utf8`, `Tls` | no |
    /// | `Io`, `Other` | yes |
    /// | `Http(429)`, `Http(500..=599)` | yes |
    /// | `Http(401 \| 403)` | no |
    /// | `Http(other)` | no |
    ///
    /// Protocol violations are now correctly non-retryable — retrying the
    /// same SDK against the same server will keep failing. Pre-0.6.0
    /// behaviour treated every WebSocket error as retryable, which was
    /// a footgun for monitor incident response.
    pub fn is_retryable(&self) -> bool {
        match self {
            // Network errors are retryable
            Self::ConnectionError { .. }
            | Self::TimeoutError { .. }
            | Self::HeartbeatTimeout { .. } => true,
            // WebSocket retry verdict driven by structured kind
            Self::WebSocketError { kind, .. } => match kind {
                WebSocketErrorKind::Io | WebSocketErrorKind::Other => true,
                WebSocketErrorKind::Http(status) => {
                    *status == 429 || (500..=599).contains(status)
                }
                WebSocketErrorKind::Protocol
                | WebSocketErrorKind::Capacity
                | WebSocketErrorKind::Utf8
                | WebSocketErrorKind::Tls => false,
            },
            // API errors with 429 or 5xx status codes are retryable
            Self::ApiError { status, .. } => *status == 429 || (500..=599).contains(status),
            // Parameter errors are never retryable (user must fix input)
            Self::InvalidParameter { .. } => false,
            // All other errors are not retryable
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = MarketDataError::InvalidSymbol {
            symbol: "INVALID".to_string(),
        };
        assert_eq!(err.to_string(), "Invalid symbol: INVALID");

        let err = MarketDataError::RuntimeError {
            msg: "test message".to_string(),
        };
        assert_eq!(err.to_string(), "Runtime error: test message");

        let err = MarketDataError::ConfigError("missing key".to_string());
        assert_eq!(err.to_string(), "Configuration error: missing key");

        let err = MarketDataError::ApiError {
            status: 404,
            message: "not found".to_string(),
            http: None,
        };
        assert_eq!(err.to_string(), "API error (status 404): not found");

        let err = MarketDataError::ClientClosed;
        assert_eq!(err.to_string(), "Client already closed");
    }

    #[test]
    fn test_error_codes() {
        let err = MarketDataError::InvalidSymbol {
            symbol: "test".to_string(),
        };
        assert_eq!(err.to_error_code(), 1001);

        let err = MarketDataError::RuntimeError {
            msg: "test".to_string(),
        };
        assert_eq!(err.to_error_code(), 1003);

        let err = MarketDataError::ConfigError("test".to_string());
        assert_eq!(err.to_error_code(), 1004);

        let err = MarketDataError::ConnectionError {
            msg: "test".to_string(),
        };
        assert_eq!(err.to_error_code(), 2001);

        let err = MarketDataError::AuthError {
            msg: "test".to_string(),
            http: None,
        };
        assert_eq!(err.to_error_code(), 2002);

        let err = MarketDataError::ApiError {
            status: 500,
            message: "test".to_string(),
            http: None,
        };
        assert_eq!(err.to_error_code(), 2003);

        let err = MarketDataError::TimeoutError {
            operation: "test".to_string(),
        };
        assert_eq!(err.to_error_code(), 3001);

        let err = MarketDataError::WebSocketError {
            kind: WebSocketErrorKind::Protocol,
            msg: "test".to_string(),
        };
        assert_eq!(err.to_error_code(), 3002);

        let err = MarketDataError::HeartbeatTimeout {
            elapsed: Duration::from_secs(35),
        };
        assert_eq!(err.to_error_code(), 3003);

        let err = MarketDataError::ClientClosed;
        assert_eq!(err.to_error_code(), 2010);

        let err = MarketDataError::Other(anyhow::anyhow!("test"));
        assert_eq!(err.to_error_code(), 9999);
    }

    #[test]
    fn test_retryable_classification() {
        // Retryable errors
        let err = MarketDataError::ConnectionError {
            msg: "test".to_string(),
        };
        assert!(err.is_retryable());

        let err = MarketDataError::TimeoutError {
            operation: "test".to_string(),
        };
        assert!(err.is_retryable());

        // Io kind is retryable; Protocol is not — test both.
        let err = MarketDataError::WebSocketError {
            kind: WebSocketErrorKind::Io,
            msg: "reset".to_string(),
        };
        assert!(err.is_retryable());
        let err = MarketDataError::WebSocketError {
            kind: WebSocketErrorKind::Protocol,
            msg: "frame".to_string(),
        };
        assert!(!err.is_retryable());

        let err = MarketDataError::HeartbeatTimeout {
            elapsed: Duration::from_secs(35),
        };
        assert!(err.is_retryable());

        // Non-retryable errors
        let err = MarketDataError::InvalidSymbol {
            symbol: "test".to_string(),
        };
        assert!(!err.is_retryable());

        let err = MarketDataError::RuntimeError {
            msg: "test".to_string(),
        };
        assert!(!err.is_retryable());

        let err = MarketDataError::ConfigError("test".to_string());
        assert!(!err.is_retryable());

        let err = MarketDataError::AuthError {
            msg: "test".to_string(),
            http: None,
        };
        assert!(!err.is_retryable());

        let err = MarketDataError::ApiError {
            status: 400,
            message: "test".to_string(),
            http: None,
        };
        assert!(!err.is_retryable());

        // ApiError with 429 should be retryable
        let err = MarketDataError::ApiError {
            status: 429,
            message: "rate limit".to_string(),
            http: None,
        };
        assert!(err.is_retryable());

        // ApiError with 5xx should be retryable
        let err = MarketDataError::ApiError {
            status: 503,
            message: "service unavailable".to_string(),
            http: None,
        };
        assert!(err.is_retryable());

        let err = MarketDataError::ClientClosed;
        assert!(!err.is_retryable());

        let err = MarketDataError::Other(anyhow::anyhow!("test"));
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_heartbeat_timeout_display() {
        let err = MarketDataError::HeartbeatTimeout {
            elapsed: Duration::from_secs(35),
        };
        assert!(err.to_string().contains("35s"));
        assert!(err.to_string().starts_with("Heartbeat timeout"));
    }

    #[test]
    fn test_from_serde_json_error() {
        let json_err = serde_json::from_str::<serde_json::Value>("{invalid json")
            .unwrap_err();
        let err: MarketDataError = json_err.into();

        assert_eq!(err.to_error_code(), 1002);
        assert!(matches!(err, MarketDataError::DeserializationError { .. }));
    }

    #[test]
    fn test_from_anyhow_error() {
        let anyhow_err = anyhow::anyhow!("test error");
        let err: MarketDataError = anyhow_err.into();

        assert_eq!(err.to_error_code(), 9999);
        assert!(matches!(err, MarketDataError::Other(_)));
    }

    #[test]
    fn test_from_tungstenite_connection_closed() {
        // 0.6.0: ConnectionClosed routes to WebSocketError { kind: Io }
        // (the old behaviour collapsed it into ConnectionError).
        use tokio_tungstenite::tungstenite::Error as WsError;

        let ws_err = WsError::ConnectionClosed;
        let err: MarketDataError = ws_err.into();

        assert_eq!(err.to_error_code(), 3002);
        assert!(matches!(
            err,
            MarketDataError::WebSocketError {
                kind: WebSocketErrorKind::Io,
                ..
            }
        ));
        assert!(err.is_retryable());
    }

    #[test]
    fn test_from_tungstenite_protocol_error() {
        // 0.6.0: Protocol kind is NOT retryable (was retryable in 0.5.x).
        use tokio_tungstenite::tungstenite::Error as WsError;
        use tokio_tungstenite::tungstenite::error::ProtocolError;

        let ws_err = WsError::Protocol(ProtocolError::ResetWithoutClosingHandshake);
        let err: MarketDataError = ws_err.into();

        assert_eq!(err.to_error_code(), 3002);
        assert!(matches!(
            err,
            MarketDataError::WebSocketError {
                kind: WebSocketErrorKind::Protocol,
                ..
            }
        ));
        assert!(
            !err.is_retryable(),
            "Protocol violations must not retry (0.6.0+); retry against the same SDK + server combo will keep failing"
        );
    }

    #[test]
    fn test_from_tungstenite_already_closed() {
        use tokio_tungstenite::tungstenite::Error as WsError;

        let ws_err = WsError::AlreadyClosed;
        let err: MarketDataError = ws_err.into();

        assert_eq!(err.to_error_code(), 3002);
        assert!(matches!(err, MarketDataError::WebSocketError { .. }));
    }

    // ----- source_kind() classification (0.5.1) -----

    #[test]
    fn source_kind_network_for_transport_failures() {
        let err = MarketDataError::ConnectionError {
            msg: "reset".to_string(),
        };
        assert_eq!(err.source_kind(), ErrorKind::Network);

        let err = MarketDataError::TimeoutError {
            operation: "read".to_string(),
        };
        assert_eq!(err.source_kind(), ErrorKind::Network);

        let err = MarketDataError::HeartbeatTimeout {
            elapsed: Duration::from_secs(35),
        };
        assert_eq!(err.source_kind(), ErrorKind::Network);
    }

    #[test]
    fn source_kind_for_websocket_protocol_kind() {
        let err = MarketDataError::WebSocketError {
            kind: WebSocketErrorKind::Protocol,
            msg: "frame".to_string(),
        };
        assert_eq!(err.source_kind(), ErrorKind::Protocol);
    }

    #[test]
    fn source_kind_for_websocket_io_routes_to_network() {
        let err = MarketDataError::WebSocketError {
            kind: WebSocketErrorKind::Io,
            msg: "reset".to_string(),
        };
        assert_eq!(err.source_kind(), ErrorKind::Network);
    }

    #[test]
    fn source_kind_for_websocket_tls_routes_to_auth() {
        let err = MarketDataError::WebSocketError {
            kind: WebSocketErrorKind::Tls,
            msg: "cert".to_string(),
        };
        assert_eq!(err.source_kind(), ErrorKind::Auth);
    }

    #[test]
    fn source_kind_for_websocket_http_401_routes_to_auth() {
        let err = MarketDataError::WebSocketError {
            kind: WebSocketErrorKind::Http(401),
            msg: "unauthorized".to_string(),
        };
        assert_eq!(err.source_kind(), ErrorKind::Auth);
    }

    #[test]
    fn source_kind_for_websocket_http_429_routes_to_rate_limit() {
        let err = MarketDataError::WebSocketError {
            kind: WebSocketErrorKind::Http(429),
            msg: "throttle".to_string(),
        };
        assert_eq!(err.source_kind(), ErrorKind::RateLimit);
    }

    #[test]
    fn tungstenite_protocol_routes_to_protocol_kind() {
        use tokio_tungstenite::tungstenite::error::ProtocolError;
        use tokio_tungstenite::tungstenite::Error as WsError;
        let ws_err = WsError::Protocol(ProtocolError::ResetWithoutClosingHandshake);
        let err: MarketDataError = ws_err.into();
        match err {
            MarketDataError::WebSocketError { kind, .. } => {
                assert_eq!(kind, WebSocketErrorKind::Protocol);
            }
            other => panic!("expected WebSocketError, got {other:?}"),
        }
    }

    #[test]
    fn tungstenite_io_routes_to_io_kind() {
        use std::io;
        use tokio_tungstenite::tungstenite::Error as WsError;
        let ws_err = WsError::Io(io::Error::new(io::ErrorKind::ConnectionReset, "reset"));
        let err: MarketDataError = ws_err.into();
        match err {
            MarketDataError::WebSocketError { kind, .. } => {
                assert_eq!(kind, WebSocketErrorKind::Io);
            }
            other => panic!("expected WebSocketError, got {other:?}"),
        }
    }

    #[test]
    fn source_kind_auth_for_401_403_api_errors() {
        let err = MarketDataError::ApiError {
            status: 401,
            message: "unauthorized".to_string(),
            http: None,
        };
        assert_eq!(err.source_kind(), ErrorKind::Auth);

        let err = MarketDataError::ApiError {
            status: 403,
            message: "forbidden".to_string(),
            http: None,
        };
        assert_eq!(err.source_kind(), ErrorKind::Auth);

        let err = MarketDataError::AuthError {
            msg: "bad token".to_string(),
            http: None,
        };
        assert_eq!(err.source_kind(), ErrorKind::Auth);
    }

    #[test]
    fn source_kind_network_for_5xx() {
        let err = MarketDataError::ApiError {
            status: 503,
            message: "service unavailable".to_string(),
            http: None,
        };
        assert_eq!(err.source_kind(), ErrorKind::Network);

        let err = MarketDataError::ApiError {
            status: 500,
            message: "internal".to_string(),
            http: None,
        };
        assert_eq!(err.source_kind(), ErrorKind::Network);
    }

    #[test]
    fn source_kind_rate_limit_for_429() {
        // 429 is distinct from Network: the correct response is to
        // *reduce* request volume, not to assume the upstream is down.
        // Monitor incident playbooks differ — keep them separable.
        let err = MarketDataError::ApiError {
            status: 429,
            message: "rate limit".to_string(),
            http: None,
        };
        assert_eq!(err.source_kind(), ErrorKind::RateLimit);
    }

    #[test]
    fn source_kind_client_for_validation_failures() {
        let err = MarketDataError::InvalidParameter {
            name: "symbol".to_string(),
            reason: "empty".to_string(),
        };
        assert_eq!(err.source_kind(), ErrorKind::Client);

        let err = MarketDataError::InvalidSymbol {
            symbol: "?".to_string(),
        };
        assert_eq!(err.source_kind(), ErrorKind::Client);

        let err = MarketDataError::ConfigError("bad".to_string());
        assert_eq!(err.source_kind(), ErrorKind::Client);

        let err = MarketDataError::ClientClosed;
        assert_eq!(err.source_kind(), ErrorKind::Client);
    }

    #[test]
    fn source_kind_client_for_4xx_excl_auth() {
        let err = MarketDataError::ApiError {
            status: 404,
            message: "not found".to_string(),
            http: None,
        };
        assert_eq!(err.source_kind(), ErrorKind::Client);

        let err = MarketDataError::ApiError {
            status: 400,
            message: "bad request".to_string(),
            http: None,
        };
        assert_eq!(err.source_kind(), ErrorKind::Client);
    }

    #[test]
    fn info_without_http_has_code_kind_and_message_only() {
        let err = MarketDataError::TimeoutError { operation: "read".to_string() };
        let info = err.info();
        assert_eq!(info.code, error_code::TIMEOUT);
        assert_eq!(info.source_kind, ErrorKind::Network);
        assert_eq!(info.message, "Timeout error: read");
        assert_eq!((info.status, info.body, info.request_id), (None, None, None));
        assert!(info.headers.is_empty());
    }

    #[test]
    fn info_carries_http_context() {
        let http = HttpErrorContext::new(
            401,
            Some("denied".to_string()),
            [("X-Request-Id", "abc"), ("Vary", "a"), ("vary", "b")],
        );
        let err = MarketDataError::AuthError { msg: "denied".to_string(), http: Some(Box::new(http)) };
        let info = err.info();
        assert_eq!(info.code, error_code::AUTH);
        assert_eq!(info.source_kind, ErrorKind::Auth);
        assert_eq!(info.message, "Authentication error: denied");
        assert_eq!(info.status, Some(401));
        assert_eq!(info.body.as_deref(), Some("denied"));
        assert_eq!(info.request_id.as_deref(), Some("abc"));
        assert_eq!(info.headers.get("vary").map(String::as_str), Some("a, b"));
    }

    #[test]
    fn http_context_drops_set_cookie_in_any_case() {
        let http = HttpErrorContext::new(
            401,
            None,
            [("Set-Cookie", "session=secret"), ("SET-COOKIE", "b=2"), ("set-cookie", "c=3"), ("Retry-After", "5")],
        );
        assert_eq!(http.header("set-cookie"), None);
        assert_eq!(http.headers.len(), 1);
        assert_eq!(http.header("retry-after"), Some("5"));
    }

    #[test]
    fn info_status_for_websocket_upgrade_and_bare_api_error() {
        let err = MarketDataError::WebSocketError {
            kind: WebSocketErrorKind::Http(429),
            msg: "HTTP 429 during WebSocket handshake".to_string(),
        };
        assert_eq!(err.info().status, Some(429));
        assert_eq!(err.info().source_kind, ErrorKind::RateLimit);
        let err = MarketDataError::ApiError { status: 404, message: "x".to_string(), http: None };
        assert_eq!(err.info().status, Some(404));
        assert_eq!(err.info().body, None);
    }

    #[test]
    fn error_kind_names_are_stable() {
        let names: Vec<_> = [
            ErrorKind::Network,
            ErrorKind::Protocol,
            ErrorKind::Auth,
            ErrorKind::RateLimit,
            ErrorKind::Client,
        ]
        .iter()
        .map(|k| k.to_string())
        .collect();
        assert_eq!(names, ["network", "protocol", "auth", "rate_limit", "client"]);
    }

    // `#[non_exhaustive]` only forces wildcard arms in OTHER crates. Same-
    // crate matches see every variant. We document the requirement here
    // for clarity; downstream cross-crate enforcement is verified by the
    // FFI binding builds (py / js / uniffi).
    #[test]
    fn error_kind_variants_exist() {
        fn classify(k: ErrorKind) -> u8 {
            match k {
                ErrorKind::Network => 1,
                ErrorKind::Protocol => 2,
                ErrorKind::Auth => 3,
                ErrorKind::RateLimit => 4,
                ErrorKind::Client => 5,
            }
        }
        assert_eq!(classify(ErrorKind::Network), 1);
        assert_eq!(classify(ErrorKind::Protocol), 2);
        assert_eq!(classify(ErrorKind::Auth), 3);
        assert_eq!(classify(ErrorKind::RateLimit), 4);
        assert_eq!(classify(ErrorKind::Client), 5);
    }
}

#[cfg(test)]
mod http_mapping_consistency {
    //! Pins the doc-vs-impl contract for `WebSocketErrorKind::Http(u16)`.
    //!
    //! The variant doc-comment lists the status-code → `ErrorKind` and
    //! `is_retryable()` mapping. This test exercises representative codes
    //! across every documented family so any silent drift between the
    //! doc table and the impl arms in `MarketDataError::source_kind` /
    //! `is_retryable` fails the suite.
    use super::{ErrorKind, MarketDataError, WebSocketErrorKind};

    fn ws_err(status: u16) -> MarketDataError {
        MarketDataError::WebSocketError {
            kind: WebSocketErrorKind::Http(status),
            msg: format!("HTTP {} during WebSocket handshake", status),
        }
    }

    /// Each row mirrors the rendered table on `WebSocketErrorKind::Http`.
    /// Update both at the same time — they are the doc-vs-impl contract.
    const HTTP_TABLE: &[(u16, ErrorKind, bool)] = &[
        (401, ErrorKind::Auth, false),
        (403, ErrorKind::Auth, false),
        (404, ErrorKind::Client, false),
        (429, ErrorKind::RateLimit, true),
        (500, ErrorKind::Network, true),
        (503, ErrorKind::Network, true),
        (999, ErrorKind::Client, false),
    ];

    #[test]
    fn http_status_mapping_matches_doc_table() {
        for &(status, expected_kind, expected_retryable) in HTTP_TABLE {
            let err = ws_err(status);
            assert_eq!(
                err.source_kind(),
                expected_kind,
                "HTTP {status}: source_kind() mismatch with documented table on WebSocketErrorKind::Http"
            );
            assert_eq!(
                err.is_retryable(),
                expected_retryable,
                "HTTP {status}: is_retryable() mismatch with documented table on WebSocketErrorKind::Http"
            );
        }
    }
}
