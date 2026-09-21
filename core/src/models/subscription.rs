//! WebSocket subscription types - matches Fugle WebSocket API

use std::borrow::Cow;
use std::fmt;
use std::ops::Range;
use std::sync::OnceLock;

use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::value::RawValue;
use serde_json::Value;

use crate::MarketDataError;

use crate::models::symbols::Symbols;

/// WebSocket channel types for stock market data
///
/// These match the official Fugle WebSocket API channels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    /// Real-time trades
    Trades,
    /// Candlestick data
    Candles,
    /// Order book (bids/asks)
    Books,
    /// Aggregate data (quote-like)
    Aggregates,
    /// Index data
    Indices,
}

impl Channel {
    /// Get channel name as string
    pub fn as_str(&self) -> &'static str {
        match self {
            Channel::Trades => "trades",
            Channel::Candles => "candles",
            Channel::Books => "books",
            Channel::Aggregates => "aggregates",
            Channel::Indices => "indices",
        }
    }
}

/// Parses a channel name, ignoring case.
///
/// ```rust
/// use marketdata_core::models::Channel;
///
/// assert_eq!("Trades".parse::<Channel>().unwrap(), Channel::Trades);
/// assert!("trade".parse::<Channel>().is_err());
/// ```
impl std::str::FromStr for Channel {
    type Err = crate::MarketDataError;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        const ALL: [Channel; 5] = [
            Channel::Trades,
            Channel::Candles,
            Channel::Books,
            Channel::Aggregates,
            Channel::Indices,
        ];
        ALL.into_iter()
            .find(|channel| channel.as_str().eq_ignore_ascii_case(name))
            .ok_or_else(|| invalid_channel(name, &ALL.map(|channel| channel.as_str())))
    }
}

/// `InvalidParameter` for an unknown channel name, listing the valid ones.
pub(crate) fn invalid_channel(name: &str, valid: &[&str]) -> crate::MarketDataError {
    crate::MarketDataError::InvalidParameter {
        name: "channel".to_string(),
        reason: format!(
            "unknown channel '{}'. Valid channels: {}",
            name,
            valid.join(", ")
        ),
    }
}

/// Subscription request for WebSocket
///
/// Modifier flags (`after_hours`, `intraday_odd_lot`) are preserved across
/// reconnection so a 盤後 or 盤中零股 subscription comes back as the same
/// session — previous design stored only `{channel, symbol}` which silently
/// downgraded on resubscribe.
///
/// On the wire either `symbol` (single) or `symbols` (batch) is populated,
/// never both — see [`Symbols`]. The two fields are encoded separately
/// because the Fugle server protocol uses the field presence to drive its
/// ACK shape (`subscribed` event `data` is an object for single, array for
/// batch).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, bon::Builder)]
pub struct SubscribeRequest {
    /// Channel to subscribe to
    pub channel: String,

    /// Stock symbol for the single-symbol path. Mutually exclusive with
    /// `symbols`. Both `None` is allowed for channel-only subscriptions
    /// (e.g. indices) where the server doesn't require a symbol.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,

    /// Batch symbol list for the multi-symbol path. Mutually exclusive
    /// with `symbol`. Serializes as `symbols: [...]` on the wire.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbols: Option<Vec<String>>,

    /// FutOpt after-hours session flag. Sent as `afterHours: true` on wire
    /// when set; absent otherwise so stock path serializes unchanged.
    #[serde(rename = "afterHours", skip_serializing_if = "Option::is_none")]
    pub after_hours: Option<bool>,

    /// Stock intraday odd-lot flag. Sent as `intradayOddLot: true` on wire
    /// when set; absent otherwise.
    #[serde(rename = "intradayOddLot", skip_serializing_if = "Option::is_none")]
    pub intraday_odd_lot: Option<bool>,
}

impl SubscribeRequest {
    /// Create a new single-symbol subscription request.
    ///
    /// For the polymorphic single/batch variant, use [`SubscribeRequest::with_symbols`].
    pub fn new(channel: Channel, symbol: impl Into<String>) -> Self {
        Self {
            channel: channel.as_str().to_string(),
            symbol: Some(symbol.into()),
            ..Default::default()
        }
    }

    /// Create a subscription request from a [`Symbols`].
    ///
    /// Accepts `&str` / `String` / `Vec<String>` / array literal / slice via
    /// `impl Into<Symbols>`. Routes to the `symbol` or `symbols` wire field
    /// based on the variant.
    ///
    /// The input runs through [`Symbols::normalized`] before being attached
    /// to the request, so duplicate symbols collapse to one subscription
    /// and whitespace-only differences are squashed. Empty inputs produce a
    /// request with no symbols attached (the dispatch path treats this as
    /// a no-op on the wire).
    pub fn with_symbols(channel: Channel, symbols: impl Into<Symbols>) -> Self {
        let spec = symbols.into().normalized();
        let mut req = Self {
            channel: channel.as_str().to_string(),
            ..Default::default()
        };
        match spec {
            Symbols::Single(s) => req.symbol = Some(s),
            Symbols::Many(v) => {
                if !v.is_empty() {
                    req.symbols = Some(v);
                }
            }
        }
        req
    }

    /// Expand a batch request into N single-symbol requests.
    ///
    /// For wire transmission a batch `SubscribeRequest` is sent as a single
    /// frame with `symbols: [...]`, but for internal bookkeeping each symbol
    /// must occupy its own row in `SubscriptionManager` so that the server's
    /// per-symbol ACK can be recorded against a stable local key. This helper
    /// materializes the expansion; single-symbol requests pass through
    /// unchanged. A reconnect folds the rows back into one frame per channel
    /// and modifier.
    ///
    /// Modifier flags (`after_hours`, `intraday_odd_lot`) are duplicated to
    /// every expanded entry — batches always share their modifier flags
    /// across the symbols they enumerate.
    pub fn expand(self) -> Vec<SubscribeRequest> {
        match self.symbols {
            Some(symbols) => symbols
                .into_iter()
                .map(|s| SubscribeRequest {
                    channel: self.channel.clone(),
                    symbol: Some(s),
                    symbols: None,
                    after_hours: self.after_hours,
                    intraday_odd_lot: self.intraday_odd_lot,
                })
                .collect(),
            None => vec![self],
        }
    }

    // 0.4.0: Legacy per-channel constructors (`trades` / `candles` /
    // `books` / `aggregates`) were removed. They duplicated
    // `SubscribeRequest::new(Channel::*, symbol)` and complicated future
    // channel additions. See MIGRATION-0.4.md.

    /// Generate subscription key for tracking.
    ///
    /// Includes modifier suffix so 盤後/零股 subscriptions occupy distinct
    /// slots from their regular-session counterparts — the key is the
    /// identity used by `SubscriptionManager` for reconnect, replacement,
    /// and unsubscribe lookup.
    pub fn key(&self) -> String {
        let base = match &self.symbol {
            Some(symbol) => format!("{}:{}", self.channel, symbol),
            None => self.channel.clone(),
        };
        if self.after_hours == Some(true) {
            format!("{base}:afterhours")
        } else if self.intraday_odd_lot == Some(true) {
            format!("{base}:oddlot")
        } else {
            base
        }
    }
}

/// Unsubscribe request for WebSocket
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UnsubscribeRequest {
    /// Subscription ID to unsubscribe
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Multiple subscription IDs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ids: Option<Vec<String>>,
}

impl UnsubscribeRequest {
    /// Unsubscribe by single ID
    pub fn by_id(id: impl Into<String>) -> Self {
        Self {
            id: Some(id.into()),
            ids: None,
        }
    }

    /// Unsubscribe by multiple IDs
    pub fn by_ids(ids: Vec<String>) -> Self {
        Self {
            id: None,
            ids: Some(ids),
        }
    }
}

/// WebSocket message wrapper (incoming messages)
///
/// Built from a frame with [`parse`](Self::parse). The `data` payload is not
/// parsed up front: `parse` only records where it sits inside [`raw`](Self::raw),
/// and [`data`](Self::data) builds the `Value` the first time it is called.
/// Market-data frames go straight to the caller as `raw`, so the hot path never
/// builds (or drops) a `Value` tree it does not use (#236).
///
/// `Deserialize` still works (`serde_json::from_str` / `from_value`); a
/// message built that way has its `data` parsed eagerly and an empty `raw`.
#[derive(Clone)]
pub struct WebSocketMessage {
    /// Event type (e.g., "data", "subscribed", "error", "authenticated", "pong")
    pub event: String,

    /// Message data (varies by event type); read it with [`data`](Self::data)
    /// or [`data_json`](Self::data_json).
    data: DataSlot,

    /// Channel (for data events)
    pub channel: Option<String>,

    /// Symbol (for data events)
    pub symbol: Option<String>,

    /// Subscription ID (for subscribed events)
    pub id: Option<String>,

    /// Server error code (for error events), e.g. `1000` for rejected
    /// credentials. The server sends it at the top level of the frame, next
    /// to `event`, not inside `data`.
    pub code: Option<i32>,

    /// Top-level error message (for error events). The server normally puts
    /// the message under `data.message`; one shape (`ws-exception.filter.ts`)
    /// sends `{"event":"error","message":"…"}` with no `code` and no `data`,
    /// and this field catches it.
    pub message: Option<String>,

    /// The frame exactly as the server sent it.
    ///
    /// The fields above are the subset this SDK routes on, so re-serializing
    /// this struct would both drop anything the server added and materialise
    /// `null`s for the fields it left out. Bindings hand this string to the
    /// caller instead, so the payload the application sees is byte-for-byte
    /// what arrived on the wire.
    ///
    /// Skipped by serde: it is populated from the frame itself, never parsed
    /// out of it.
    ///
    /// [`data`](Self::data) and [`data_json`](Self::data_json) of a parsed
    /// message read their payload out of this string. Overwriting it leaves
    /// their result unspecified — typically `None`, never a panic; a `data()`
    /// already returned stays as it was.
    pub raw: String,
}

/// Where a message's `data` lives: a byte range of `raw` (from `parse`),
/// and the `Value` built from it on first use (or up front, from `Deserialize`).
#[derive(Clone, Default)]
struct DataSlot {
    span: Option<Range<u32>>,
    value: OnceLock<Option<Value>>,
}

impl DataSlot {
    fn parsed(value: Option<Value>) -> Self {
        Self { span: None, value: OnceLock::from(value) }
    }
}

/// The routed fields of a frame. `parse` reads `data` as a borrowed
/// `&RawValue`, `Deserialize` as a `Value`.
#[derive(Deserialize)]
struct Fields<D> {
    event: String,
    #[serde(default = "none")]
    data: Option<D>,
    #[serde(default)]
    channel: Option<String>,
    #[serde(default)]
    symbol: Option<String>,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    code: Option<i32>,
    #[serde(default)]
    message: Option<String>,
}

// `#[serde(default)]` on a generic field would require `D: Default`.
fn none<D>() -> Option<D> {
    None
}

impl<D> Fields<D> {
    fn into_message(self, data: DataSlot, raw: String) -> WebSocketMessage {
        WebSocketMessage {
            event: self.event,
            data,
            channel: self.channel,
            symbol: self.symbol,
            id: self.id,
            code: self.code,
            message: self.message,
            raw,
        }
    }
}

impl WebSocketMessage {
    /// Parse a frame. `raw` is `text`; `data` is located but not parsed.
    ///
    /// # Errors
    ///
    /// [`MarketDataError::DeserializationError`] when `text` is not a JSON
    /// object with a string `event`, or a routed field has the wrong type.
    pub fn parse(text: &str) -> Result<Self, MarketDataError> {
        let mut frame: Fields<&RawValue> =
            serde_json::from_str(text).map_err(|source| MarketDataError::DeserializationError { source })?;
        let data = match frame.data.take() {
            None => DataSlot::default(),
            Some(raw) => {
                // `raw` borrows from `text`, so its offset is its position there.
                let start = raw.get().as_ptr() as usize - text.as_ptr() as usize;
                let end = start + raw.get().len();
                match (u32::try_from(start), u32::try_from(end)) {
                    (Ok(start), Ok(end)) => DataSlot { span: Some(start..end), value: OnceLock::new() },
                    // A frame past 4 GiB: parse now rather than widen the span.
                    _ => DataSlot::parsed(Some(
                        serde_json::from_str(raw.get())
                            .map_err(|source| MarketDataError::DeserializationError { source })?,
                    )),
                }
            }
        };
        Ok(frame.into_message(data, text.to_string()))
    }

    /// The `data` payload, parsed on the first call; later calls return the
    /// same reference. `None` when the frame has no `data` (or `"data":null`).
    pub fn data(&self) -> Option<&Value> {
        self.data
            .value
            .get_or_init(|| serde_json::from_str(self.raw_data()?).ok())
            .as_ref()
    }

    /// The `data` payload as JSON text. For a parsed message this is the
    /// slice of [`raw`](Self::raw) the server sent, byte for byte; for one
    /// built by `Deserialize` it is the compact serialization of the value.
    pub fn data_json(&self) -> Option<Cow<'_, str>> {
        if self.data.span.is_some() {
            return self.raw_data().map(Cow::Borrowed);
        }
        self.data.value.get()?.as_ref().map(|value| Cow::Owned(value.to_string()))
    }

    fn raw_data(&self) -> Option<&str> {
        let span = self.data.span.as_ref()?;
        self.raw.get(span.start as usize..span.end as usize)
    }

    /// Check if this is an authentication success message
    pub fn is_authenticated(&self) -> bool {
        self.event == "authenticated"
    }

    /// Check if this is an error message
    pub fn is_error(&self) -> bool {
        self.event == "error"
    }

    /// Check if this is a data message
    pub fn is_data(&self) -> bool {
        self.event == "data"
    }

    /// Check if this is a pong message. With the activity-timer health check
    /// the SDK never sends internal pings, so any pong arriving on this
    /// connection is a response to a user-initiated `ping(state)` and is
    /// forwarded to user message callbacks unchanged.
    pub fn is_pong(&self) -> bool {
        self.event == "pong"
    }

    /// Check if this is a server-initiated heartbeat (`{"event":"heartbeat"}`).
    /// Heartbeats arrive every ~30 seconds and carry a microsecond timestamp
    /// in `data.time`. They are forwarded to user message callbacks so callers
    /// can use them for latency measurement or clock alignment.
    pub fn is_heartbeat(&self) -> bool {
        self.event == "heartbeat"
    }

    /// Check if this is a subscribed confirmation
    pub fn is_subscribed(&self) -> bool {
        self.event == "subscribed"
    }

    /// The server's error message if this is an error frame.
    ///
    /// Read from `data.message` (the usual shape, next to a top-level
    /// `code`), falling back to a top-level `message` for the `code`-less
    /// shape the server also sends.
    pub fn error_message(&self) -> Option<String> {
        if !self.is_error() {
            return None;
        }
        self.data()
            .and_then(|d| d.get("message"))
            .and_then(|m| m.as_str())
            .map(|s| s.to_string())
            .or_else(|| self.message.clone())
    }

    /// The server's error code if this is an error frame that carries one.
    ///
    /// Codes the server uses: `1000` credentials rejected, `1001`
    /// subscription limit exceeded, `1002` command before authentication,
    /// `1003` request validation failed, `1004` no auth request within 60 s,
    /// `1011` auth service unavailable. `1000` is the one the reconnect
    /// policy acts on (#201).
    pub fn error_code(&self) -> Option<i32> {
        if !self.is_error() {
            return None;
        }
        self.code
    }
}

/// Prints `data` without parsing it: the parsed value if `data()` has run,
/// otherwise the slice of `raw` it will be parsed from.
impl fmt::Debug for WebSocketMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct UnparsedData<'a>(&'a str);
        impl fmt::Debug for UnparsedData<'_> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.0)
            }
        }

        let mut s = f.debug_struct("WebSocketMessage");
        s.field("event", &self.event);
        match (self.data.value.get(), self.raw_data()) {
            (Some(value), _) => s.field("data", value),
            (None, Some(raw)) => s.field("data", &Some(UnparsedData(raw))),
            (None, None) => s.field("data", &None::<Value>),
        };
        s.field("channel", &self.channel)
            .field("symbol", &self.symbol)
            .field("id", &self.id)
            .field("code", &self.code)
            .field("message", &self.message)
            .field("raw", &self.raw)
            .finish()
    }
}

/// Same output as the derived impl before #236: `raw` is left out, and
/// `code` / `message` only appear when set.
impl Serialize for WebSocketMessage {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let len = 5 + usize::from(self.code.is_some()) + usize::from(self.message.is_some());
        let mut s = serializer.serialize_struct("WebSocketMessage", len)?;
        s.serialize_field("event", &self.event)?;
        s.serialize_field("data", &self.data())?;
        s.serialize_field("channel", &self.channel)?;
        s.serialize_field("symbol", &self.symbol)?;
        s.serialize_field("id", &self.id)?;
        if let Some(code) = &self.code {
            s.serialize_field("code", code)?;
        }
        if let Some(message) = &self.message {
            s.serialize_field("message", message)?;
        }
        s.end()
    }
}

/// Works from any serde source (not just a borrowed `&str`), so `data` is
/// parsed up front and `raw` is left empty. Frames off the wire go through
/// [`WebSocketMessage::parse`] instead.
impl<'de> Deserialize<'de> for WebSocketMessage {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let mut fields = Fields::<Value>::deserialize(deserializer)?;
        let data = DataSlot::parsed(fields.data.take());
        Ok(fields.into_message(data, String::new()))
    }
}

/// WebSocket authentication request
///
/// `Debug` is implemented manually to redact the credentials — a set field
/// prints as `Some(***)`, so logging a request (or anything holding one)
/// never leaks the secret, matching [`Auth`](crate::Auth).
#[derive(Clone, Serialize, Deserialize)]
pub struct AuthRequest {
    /// API key (if using API key auth)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apikey: Option<String>,

    /// Bearer token (if using token auth)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,

    /// SDK token (if using SDK token auth)
    #[serde(rename = "sdkToken", skip_serializing_if = "Option::is_none")]
    pub sdk_token: Option<String>,

    /// Optional client-requested heartbeat interval in milliseconds.
    /// Server may honor or clamp; absent value means "use server default".
    ///
    /// **Wire-only field** in 3.x — there is no public builder method
    /// yet because the server side does not honor this preference. Once
    /// server support lands (Phase 2.3 in the SDK roadmap), a
    /// `with_heartbeat_interval` builder will be exposed.
    /// Pre-shipping the wire field here means deployed v3.x clients
    /// can negotiate without needing a fresh release.
    #[serde(rename = "heartbeatIntervalMs", skip_serializing_if = "Option::is_none")]
    pub heartbeat_interval_ms: Option<u64>,
}

impl std::fmt::Debug for AuthRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        /// Prints `***` in place of a secret.
        struct Redacted;
        impl std::fmt::Debug for Redacted {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("***")
            }
        }
        let redact = |value: &Option<String>| value.as_ref().map(|_| Redacted);

        f.debug_struct("AuthRequest")
            .field("apikey", &redact(&self.apikey))
            .field("token", &redact(&self.token))
            .field("sdk_token", &redact(&self.sdk_token))
            .field("heartbeat_interval_ms", &self.heartbeat_interval_ms)
            .finish()
    }
}

impl AuthRequest {
    /// Create API key auth request
    pub fn with_api_key(api_key: impl Into<String>) -> Self {
        Self {
            apikey: Some(api_key.into()),
            token: None,
            sdk_token: None,
            heartbeat_interval_ms: None,
        }
    }

    /// Create bearer token auth request
    pub fn with_token(token: impl Into<String>) -> Self {
        Self {
            apikey: None,
            token: Some(token.into()),
            sdk_token: None,
            heartbeat_interval_ms: None,
        }
    }

    /// Create SDK token auth request
    pub fn with_sdk_token(sdk_token: impl Into<String>) -> Self {
        Self {
            apikey: None,
            token: None,
            sdk_token: Some(sdk_token.into()),
            heartbeat_interval_ms: None,
        }
    }

    /// Check that exactly one credential is set and it is not empty or only
    /// whitespace — the same rule as [`Auth::from_credentials`](crate::Auth::from_credentials).
    ///
    /// # Errors
    ///
    /// Returns [`MarketDataError::ConfigError`](crate::MarketDataError::ConfigError)
    /// when none or more than one credential is provided.
    pub fn validate(&self) -> Result<(), crate::MarketDataError> {
        crate::rest::auth::check_credentials(
            self.apikey.as_deref(),
            self.token.as_deref(),
            self.sdk_token.as_deref(),
        )
    }
}

impl From<crate::Auth> for AuthRequest {
    /// Send the credential in the field matching its kind.
    fn from(auth: crate::Auth) -> Self {
        match auth {
            crate::Auth::ApiKey(key) => Self::with_api_key(key),
            crate::Auth::BearerToken(token) => Self::with_token(token),
            crate::Auth::SdkToken(token) => Self::with_sdk_token(token),
        }
    }
}

/// WebSocket outgoing message (for sending to server)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketRequest {
    /// Event type
    pub event: String,

    /// Event data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl WebSocketRequest {
    /// Create auth request
    pub fn auth(auth: AuthRequest) -> Self {
        Self {
            event: "auth".to_string(),
            data: Some(serde_json::to_value(auth).unwrap()),
        }
    }

    /// Create subscribe request
    pub fn subscribe(sub: SubscribeRequest) -> Self {
        Self {
            event: "subscribe".to_string(),
            data: Some(serde_json::to_value(sub).unwrap()),
        }
    }

    /// Create unsubscribe request
    pub fn unsubscribe(unsub: UnsubscribeRequest) -> Self {
        Self {
            event: "unsubscribe".to_string(),
            data: Some(serde_json::to_value(unsub).unwrap()),
        }
    }

    /// Create ping request
    pub fn ping(state: Option<String>) -> Self {
        Self {
            event: "ping".to_string(),
            data: state.map(|s| serde_json::json!({"state": s})),
        }
    }

    /// Create subscriptions list request
    pub fn subscriptions() -> Self {
        Self {
            event: "subscriptions".to_string(),
            data: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_serialization() {
        let channel = Channel::Trades;
        let json = serde_json::to_string(&channel).unwrap();
        assert_eq!(json, "\"trades\"");
    }

    #[test]
    fn channel_parses_names_ignoring_case() {
        assert_eq!("trades".parse::<Channel>().unwrap(), Channel::Trades);
        assert_eq!("CANDLES".parse::<Channel>().unwrap(), Channel::Candles);
        assert_eq!("Books".parse::<Channel>().unwrap(), Channel::Books);
        assert_eq!(
            "aggregates".parse::<Channel>().unwrap(),
            Channel::Aggregates
        );
        assert_eq!("indices".parse::<Channel>().unwrap(), Channel::Indices);
    }

    #[test]
    fn channel_rejects_unknown_name_with_valid_list() {
        let err = "trade".parse::<Channel>().unwrap_err();
        let info = err.info();
        assert_eq!(info.code, crate::error_code::INVALID_PARAMETER);
        assert_eq!(
            info.message,
            "Invalid parameter 'channel': unknown channel 'trade'. \
             Valid channels: trades, candles, books, aggregates, indices"
        );
    }

    #[test]
    fn test_channel_deserialization() {
        let channel: Channel = serde_json::from_str("\"candles\"").unwrap();
        assert_eq!(channel, Channel::Candles);
    }

    #[test]
    fn test_subscribe_request() {
        let req = SubscribeRequest::new(Channel::Trades, "2330");
        assert_eq!(req.channel, "trades");
        assert_eq!(req.symbol.as_deref(), Some("2330"));
        assert_eq!(req.key(), "trades:2330");
    }

    #[test]
    fn test_subscribe_request_serialization() {
        let req = SubscribeRequest::new(Channel::Trades, "2330");
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"channel\":\"trades\""));
        assert!(json.contains("\"symbol\":\"2330\""));
        // Modifier flags absent when None — stock regular-session path
        // wire payload must stay byte-identical to pre-fix behavior.
        assert!(!json.contains("afterHours"));
        assert!(!json.contains("intradayOddLot"));
        // `symbols` array must not be emitted for the single-symbol path.
        assert!(!json.contains("\"symbols\""));
    }

    #[test]
    fn symbol_spec_accepts_common_input_shapes() {
        // &str, String, &String → Single
        let s1: Symbols = "2330".into();
        let s2: Symbols = "2330".to_string().into();
        let owned = "2330".to_string();
        let s3: Symbols = (&owned).into();
        assert!(matches!(s1, Symbols::Single(ref v) if v == "2330"));
        assert!(matches!(s2, Symbols::Single(ref v) if v == "2330"));
        assert!(matches!(s3, Symbols::Single(ref v) if v == "2330"));

        // Vec<String>, Vec<&str>, [&str; N], [String; N], slices → Many
        let m1: Symbols = vec!["A".to_string(), "B".to_string()].into();
        let m2: Symbols = vec!["A", "B"].into();
        let m3: Symbols = ["A", "B"].into();
        let m4: Symbols = ["A".to_string(), "B".to_string()].into();
        let arr: &[&str] = &["A", "B"];
        let m5: Symbols = arr.into();
        for v in [m1, m2, m3, m4, m5] {
            assert!(matches!(v, Symbols::Many(ref x) if x == &["A", "B"]));
        }
    }

    #[test]
    fn subscribe_request_with_symbols_serializes_batch() {
        let req = SubscribeRequest::with_symbols(Channel::Aggregates, vec!["2330", "0050", "2603"]);
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["channel"], "aggregates");
        assert_eq!(json["symbols"], serde_json::json!(["2330", "0050", "2603"]));
        // Single-symbol field must be absent.
        assert!(json.get("symbol").is_none());
    }

    #[test]
    fn subscribe_request_with_symbols_single_routes_to_symbol_field() {
        // Single-input variants land on `symbol`, not `symbols`.
        let req = SubscribeRequest::with_symbols(Channel::Trades, "2330");
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["symbol"], "2330");
        assert!(json.get("symbols").is_none());
    }

    #[test]
    fn bon_builder_round_trips_through_new() {
        // The derived builder is additive; the canonical constructors stay.
        let via_builder = SubscribeRequest::builder()
            .channel("trades".to_string())
            .symbol("2330".to_string())
            .build();
        let via_new = SubscribeRequest::new(Channel::Trades, "2330");
        assert_eq!(via_builder, via_new);
    }

    #[test]
    fn with_symbols_dedups_duplicates() {
        let req = SubscribeRequest::with_symbols(Channel::Trades, vec!["2330", "2330"]);
        // Many-of-one collapses to Single during normalization, so the
        // single-symbol wire path applies.
        assert_eq!(req.symbol.as_deref(), Some("2330"));
        assert!(req.symbols.is_none());
        assert_eq!(req.expand().len(), 1);
    }

    #[test]
    fn with_symbols_collapses_whitespace_differences() {
        let req = SubscribeRequest::with_symbols(Channel::Trades, vec!["2330", " 2330 ", "2330\n"]);
        assert_eq!(req.symbol.as_deref(), Some("2330"));
        assert!(req.symbols.is_none());
        assert_eq!(req.expand().len(), 1);
    }

    #[test]
    fn with_symbols_keeps_distinct_in_insertion_order() {
        let req =
            SubscribeRequest::with_symbols(Channel::Trades, vec!["2330", "2454", "2317"]);
        assert_eq!(
            req.symbols.as_deref(),
            Some(&["2330".to_string(), "2454".to_string(), "2317".to_string()][..])
        );
        assert_eq!(req.expand().len(), 3);
    }

    #[test]
    fn with_symbols_empty_input_yields_no_symbol_field() {
        // After normalization, an all-empty / all-whitespace input collapses
        // to `Many(vec![])`. We deliberately do NOT populate either wire
        // field in this case — the caller can detect and short-circuit.
        let req = SubscribeRequest::with_symbols(Channel::Trades, Vec::<String>::new());
        assert!(req.symbol.is_none());
        assert!(req.symbols.is_none());
    }

    #[test]
    fn expand_batch_into_per_symbol_requests() {
        let batch = SubscribeRequest::with_symbols(Channel::Aggregates, vec!["A", "B", "C"]);
        let expanded = batch.expand();
        assert_eq!(expanded.len(), 3);
        for (i, sym) in ["A", "B", "C"].iter().enumerate() {
            assert_eq!(expanded[i].channel, "aggregates");
            assert_eq!(expanded[i].symbol.as_deref(), Some(*sym));
            assert!(expanded[i].symbols.is_none());
        }
    }

    #[test]
    fn expand_preserves_modifier_flags_per_entry() {
        let mut batch = SubscribeRequest::with_symbols(Channel::Trades, ["2330", "2454"]);
        batch.intraday_odd_lot = Some(true);
        let expanded = batch.expand();
        for entry in &expanded {
            assert_eq!(entry.intraday_odd_lot, Some(true));
            assert!(entry.key().contains("oddlot"));
        }
    }

    #[test]
    fn expand_single_symbol_passes_through() {
        let single = SubscribeRequest::new(Channel::Trades, "2330");
        let expanded = single.expand();
        assert_eq!(expanded.len(), 1);
        assert_eq!(expanded[0].symbol.as_deref(), Some("2330"));
    }

    #[test]
    fn test_subscribe_request_after_hours_key_and_wire() {
        let req = SubscribeRequest {
            channel: "trades".to_string(),
            symbol: Some("TXF1!".to_string()),
            after_hours: Some(true),
            ..Default::default()
        };
        // Key preserves afterhours suffix → reconnect replays the correct session
        assert_eq!(req.key(), "trades:TXF1!:afterhours");
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"afterHours\":true"));
    }

    #[test]
    fn test_subscribe_request_oddlot_key_and_wire() {
        let req = SubscribeRequest {
            channel: "trades".to_string(),
            symbol: Some("2330".to_string()),
            intraday_odd_lot: Some(true),
            ..Default::default()
        };
        assert_eq!(req.key(), "trades:2330:oddlot");
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"intradayOddLot\":true"));
    }

    #[test]
    fn test_subscribe_request_deserialize_without_modifiers() {
        // Legacy payloads without the new fields must still deserialize.
        let json = r#"{"channel":"trades","symbol":"2330"}"#;
        let req: SubscribeRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.after_hours, None);
        assert_eq!(req.intraday_odd_lot, None);
        assert_eq!(req.key(), "trades:2330");
    }

    #[test]
    fn test_unsubscribe_request() {
        let req = UnsubscribeRequest::by_id("sub-123");
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"id\":\"sub-123\""));
    }

    #[test]
    fn test_websocket_message_deserialization() {
        let json = r#"{
            "event": "data",
            "channel": "trades",
            "symbol": "2330",
            "data": {"price": 583.0, "size": 1000}
        }"#;
        let msg: WebSocketMessage = serde_json::from_str(json).unwrap();
        assert!(msg.is_data());
        assert_eq!(msg.channel.as_deref(), Some("trades"));
        assert_eq!(msg.symbol.as_deref(), Some("2330"));
    }

    #[test]
    fn test_websocket_error_message() {
        let json = r#"{
            "event": "error",
            "data": {"message": "Unauthorized"}
        }"#;
        let msg: WebSocketMessage = serde_json::from_str(json).unwrap();
        assert!(msg.is_error());
        assert_eq!(msg.error_message(), Some("Unauthorized".to_string()));
        assert_eq!(msg.error_code(), None, "no code on the frame");
    }

    #[test]
    fn test_websocket_error_message_falls_back_to_top_level() {
        // The server's code-less shape: `message` next to `event`, no `data`.
        let json = r#"{"event":"error","message":"Unauthorized"}"#;
        let msg: WebSocketMessage = serde_json::from_str(json).unwrap();
        assert_eq!(msg.error_message(), Some("Unauthorized".to_string()));
        assert_eq!(msg.error_code(), None);

        // `data.message` wins when both are present.
        let json = r#"{"event":"error","message":"outer","data":{"message":"inner"}}"#;
        let msg: WebSocketMessage = serde_json::from_str(json).unwrap();
        assert_eq!(msg.error_message(), Some("inner".to_string()));

        // Only error frames report a message.
        let json = r#"{"event":"data","message":"x"}"#;
        let msg: WebSocketMessage = serde_json::from_str(json).unwrap();
        assert_eq!(msg.error_message(), None);
    }

    #[test]
    fn test_websocket_error_code_is_top_level() {
        // The server's shape (`ws-exception.filter.ts`): `code` next to
        // `event`, the message under `data`.
        let json = r#"{"event":"error","code":1000,"data":{"message":"Invalid authentication credentials"}}"#;
        let msg: WebSocketMessage = serde_json::from_str(json).unwrap();
        assert_eq!(msg.code, Some(1000));
        assert_eq!(msg.error_code(), Some(1000));
        assert_eq!(
            msg.error_message(),
            Some("Invalid authentication credentials".to_string())
        );

        // Only error frames report a code.
        let json = r#"{"event":"data","code":7}"#;
        let msg: WebSocketMessage = serde_json::from_str(json).unwrap();
        assert_eq!(msg.error_code(), None);
    }

    const DATA: &str = r#"{"price": 583.0, "n": 1e+21, "name":"\u53f0"}"#;

    fn data_frame() -> String {
        format!(r#"{{"event":"data","channel":"trades","data":{DATA},"id":"x"}}"#)
    }

    #[test]
    fn parsed_data_json_is_the_frame_slice() {
        let frame = data_frame();
        let msg = WebSocketMessage::parse(&frame).unwrap();
        assert_eq!(msg.raw, frame);
        assert!(matches!(msg.data_json(), Some(Cow::Borrowed(DATA))));
        assert_eq!(msg.channel.as_deref(), Some("trades"));
        assert_eq!(msg.id.as_deref(), Some("x"));
    }

    #[test]
    fn parsed_data_is_lazy_and_cached() {
        let msg = WebSocketMessage::parse(&data_frame()).unwrap();
        assert!(msg.data.value.get().is_none(), "not parsed by parse()");
        let first = msg.data().unwrap();
        assert_eq!(first, &serde_json::from_str::<Value>(DATA).unwrap());
        assert!(std::ptr::eq(first, msg.data().unwrap()));
    }

    #[test]
    fn frame_without_data_has_none() {
        for frame in [r#"{"event":"pong"}"#, r#"{"event":"pong","data":null}"#] {
            let msg = WebSocketMessage::parse(frame).unwrap();
            assert!(msg.data().is_none(), "{frame}");
            assert!(msg.data_json().is_none(), "{frame}");
        }
    }

    #[test]
    fn deserialized_data_is_eager_and_compact() {
        let msg: WebSocketMessage = serde_json::from_str(&data_frame()).unwrap();
        assert_eq!(msg.raw, "");
        assert_eq!(msg.data(), Some(&serde_json::from_str::<Value>(DATA).unwrap()));
        let json = msg.data_json().unwrap();
        assert!(matches!(json, Cow::Owned(_)));
        assert_eq!(json, serde_json::to_string(msg.data().unwrap()).unwrap());

        let msg: WebSocketMessage =
            serde_json::from_value(serde_json::json!({"event": "pong", "data": {"time": 1}})).unwrap();
        assert_eq!(msg.data_json().as_deref(), Some(r#"{"time":1}"#));
    }

    #[test]
    fn serialize_matches_the_derived_output() {
        // What the derived impl produced in 0.9.0-rc.6.
        let msg = WebSocketMessage::parse(r#"{"event":"pong","extra":1}"#).unwrap();
        assert_eq!(
            serde_json::to_string(&msg).unwrap(),
            r#"{"event":"pong","data":null,"channel":null,"symbol":null,"id":null}"#
        );
        let msg = WebSocketMessage::parse(
            r#"{"id":"s","event":"error","code":1000,"message":"m","data":{"b":1,"a":2}}"#,
        )
        .unwrap();
        assert_eq!(
            serde_json::to_string(&msg).unwrap(),
            r#"{"event":"error","data":{"b":1,"a":2},"channel":null,"symbol":null,"id":"s","code":1000,"message":"m"}"#
        );
    }

    #[test]
    fn clone_has_its_own_lazy_state() {
        let msg = WebSocketMessage::parse(&data_frame()).unwrap();
        let copy = msg.clone();
        assert!(msg.data().is_some());
        assert!(copy.data.value.get().is_none(), "clone taken before data() stays unparsed");
        assert!(!std::ptr::eq(msg.data().unwrap(), copy.data().unwrap()));
        assert_eq!(msg.data(), copy.data());
    }

    #[test]
    fn overwritten_raw_gives_none_not_a_panic() {
        let mut msg = WebSocketMessage::parse(&data_frame()).unwrap();
        msg.raw.truncate(10);
        assert!(msg.data_json().is_none());
        assert!(msg.data().is_none());
    }

    #[test]
    fn debug_does_not_parse_data() {
        let msg = WebSocketMessage::parse(&data_frame()).unwrap();
        let shown = format!("{msg:?}");
        assert!(shown.contains(&format!("data: Some({DATA})")), "{shown}");
        assert!(msg.data.value.get().is_none(), "Debug parsed data");
        msg.data();
        assert!(format!("{msg:?}").contains("data: Some(Object"), "parsed value shown once present");
    }

    #[test]
    fn test_websocket_authenticated() {
        let json = r#"{"event": "authenticated"}"#;
        let msg: WebSocketMessage = serde_json::from_str(json).unwrap();
        assert!(msg.is_authenticated());
    }

    #[test]
    fn test_auth_request_validate() {
        assert!(AuthRequest::with_api_key("k").validate().is_ok());
        assert!(AuthRequest::with_token("t").validate().is_ok());
        assert!(AuthRequest::with_sdk_token("s").validate().is_ok());

        let mut both = AuthRequest::with_api_key("k");
        both.token = Some("t".into());
        for req in [
            AuthRequest::with_api_key(""),
            AuthRequest::with_token("  "),
            AuthRequest::with_sdk_token("\n"),
            both,
        ] {
            let err = req.validate().expect_err("should be rejected");
            assert!(matches!(err, crate::MarketDataError::ConfigError(_)), "{err:?}");
        }
    }

    #[test]
    fn test_auth_request_debug_redacts_credentials() {
        for req in [
            AuthRequest::with_api_key("secret-api-key"),
            AuthRequest::with_token("secret-bearer-token"),
            AuthRequest::with_sdk_token("secret-sdk-token"),
        ] {
            let rendered = format!("{req:?}");
            assert!(!rendered.contains("secret"), "{rendered}");
            assert!(rendered.contains("Some(***)"), "{rendered}");
        }

        let rendered = format!("{:#?}", AuthRequest::with_token("secret-bearer-token"));
        assert!(!rendered.contains("secret"), "{rendered}");

        assert_eq!(
            format!("{:?}", AuthRequest::with_api_key("k")),
            "AuthRequest { apikey: Some(***), token: None, sdk_token: None, heartbeat_interval_ms: None }"
        );
    }

    #[test]
    fn test_auth_request_from_auth_keeps_kind() {
        let req = AuthRequest::from(crate::Auth::BearerToken("t".into()));
        assert_eq!(req.token.as_deref(), Some("t"));
        assert!(req.apikey.is_none() && req.sdk_token.is_none());
        let req = AuthRequest::from(crate::Auth::SdkToken("s".into()));
        assert_eq!(req.sdk_token.as_deref(), Some("s"));
        let req = AuthRequest::from(crate::Auth::ApiKey("k".into()));
        assert_eq!(req.apikey.as_deref(), Some("k"));
    }

    #[test]
    fn test_auth_request_api_key() {
        let req = AuthRequest::with_api_key("my-api-key");
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"apikey\":\"my-api-key\""));
        assert!(!json.contains("token"));
        assert!(!json.contains("sdkToken"));
    }

    #[test]
    fn test_auth_request_sdk_token() {
        let req = AuthRequest::with_sdk_token("my-sdk-token");
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"sdkToken\":\"my-sdk-token\""));
    }

    #[test]
    fn test_auth_request_heartbeat_interval_omitted_by_default() {
        // None must be skipped, preserving the existing wire format.
        let req = AuthRequest::with_api_key("k");
        let json = serde_json::to_string(&req).unwrap();
        assert!(!json.contains("heartbeatIntervalMs"));
    }

    #[test]
    fn test_auth_request_heartbeat_interval_serialized_when_set() {
        let mut req = AuthRequest::with_api_key("k");
        req.heartbeat_interval_ms = Some(30_000);
        let json: serde_json::Value = serde_json::from_str(
            &serde_json::to_string(&req).unwrap(),
        )
        .unwrap();
        assert_eq!(json["heartbeatIntervalMs"], 30_000);
        assert_eq!(json["apikey"], "k");
    }

    #[test]
    fn test_websocket_request_auth() {
        let req = WebSocketRequest::auth(AuthRequest::with_api_key("test"));
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"event\":\"auth\""));
        assert!(json.contains("\"apikey\":\"test\""));
    }

    #[test]
    fn test_websocket_request_subscribe() {
        let req = WebSocketRequest::subscribe(SubscribeRequest::new(Channel::Trades, "2330"));
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"event\":\"subscribe\""));
        assert!(json.contains("\"channel\":\"trades\""));
    }

    #[test]
    fn test_websocket_request_ping() {
        let req = WebSocketRequest::ping(Some("test-state".to_string()));
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"event\":\"ping\""));
        assert!(json.contains("\"state\":\"test-state\""));
    }
}
