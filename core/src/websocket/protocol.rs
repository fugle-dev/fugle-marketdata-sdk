//! Wire-protocol framing and parsing helpers.
//!
//! Runtime-free pure functions. Shared by sync and async WebSocket clients.
//! Wraps the model-layer constructors (`WebSocketRequest::{auth, subscribe,
//! unsubscribe}` in `crate::models::subscription`) with `serde_json`
//! serialization, plus extracts subscription-ack bookkeeping that was
//! previously inlined in the async dispatch loop.

use crate::models::{
    AuthRequest, SubscribeRequest, Symbols, UnsubscribeRequest, WebSocketMessage,
    WebSocketRequest,
};
use crate::websocket::channels::{FutOptSubscription, StockSubscription};
use crate::websocket::SubscriptionManager;
use crate::MarketDataError;
use indexmap::IndexMap;
use std::collections::HashSet;

/// The server's error code for rejected credentials
/// (`{"event":"error","code":1000,...}`). The only auth-phase error that
/// ends the handshake as a rejection; every other code is a failure the
/// reconnect loop retries (#201).
pub(crate) const AUTH_REJECTED_CODE: i32 = 1000;

/// Classification of the inbound auth response.
#[derive(Debug)]
pub(crate) enum AuthOutcome {
    /// Server accepted the credentials. Carries the frame's `data`
    /// (`Null` when absent). Caller should transition to `Connected`.
    Authenticated(serde_json::Value),
    /// Server rejected the credentials (`error` code
    /// [`AUTH_REJECTED_CODE`]). Caller should emit `Unauthenticated`.
    Rejected {
        /// Server-provided rejection message.
        message: String,
        /// The frame's `data` (`Null` when absent).
        data: serde_json::Value,
    },
    /// Server answered the auth frame with an `error` of any other code (or
    /// none): `1011` auth service unavailable, `1004` no auth request seen,
    /// an unknown code. Not a verdict on the credentials, so the caller
    /// should emit `Error` and treat the attempt as failed.
    Failed(MarketDataError),
    /// Frame is not an auth-related event. Caller should keep reading.
    Pending,
}

/// Result of a complete auth handshake. Shared by the async and sync clients
/// so both map it to the same lifecycle events.
#[derive(Debug)]
pub(crate) enum AuthHandshake {
    /// Server accepted the credentials; report `Authenticated { data }`
    /// followed by `frames`.
    Authenticated {
        /// The `authenticated` frame's `data` (`Null` when absent).
        data: serde_json::Value,
        /// Every frame read during the handshake, in order, so it can be
        /// queued after `Authenticated` rather than before it (#68).
        frames: Vec<WebSocketMessage>,
    },
    /// Server rejected the credentials; report
    /// `Unauthenticated { message, data }` followed by `frames`, and fail
    /// `connect()` with `AuthError`.
    Rejected {
        /// Server-provided rejection message.
        message: String,
        /// The rejection frame's `data` (`Null` when absent).
        data: serde_json::Value,
        /// Every frame read during the handshake, in order.
        frames: Vec<WebSocketMessage>,
    },
    /// Transport, timeout or protocol failure before a verdict; emit `Error`.
    Failed(MarketDataError),
}

/// Serialize an auth request frame.
pub(crate) fn frame_auth(auth: AuthRequest) -> Result<String, MarketDataError> {
    let msg = WebSocketRequest::auth(auth);
    serde_json::to_string(&msg).map_err(|e| MarketDataError::DeserializationError { source: e })
}

/// Build the wire `SubscribeRequest` and the per-symbol expansion rows from a
/// `StockSubscription`. The wire request is sent as one frame; the expansion
/// list is what `SubscriptionManager` stores so each symbol gets its own
/// local key.
pub(crate) fn frame_subscribe(
    sub: StockSubscription,
) -> Result<(String, Vec<SubscribeRequest>), MarketDataError> {
    let mut wire_req = SubscribeRequest {
        channel: sub.channel.as_str().to_string(),
        ..Default::default()
    };
    match &sub.symbols {
        Symbols::Single(s) => wire_req.symbol = Some(s.clone()),
        Symbols::Many(v) => wire_req.symbols = Some(v.clone()),
    }
    if sub.intraday_odd_lot {
        wire_req.intraday_odd_lot = Some(true);
    }

    let expanded = wire_req.clone().expand();
    let msg = WebSocketRequest::subscribe(wire_req);
    let json = serde_json::to_string(&msg)
        .map_err(|e| MarketDataError::DeserializationError { source: e })?;
    Ok((json, expanded))
}

/// FutOpt counterpart of [`frame_subscribe`]. Same single/batch semantics; the
/// modifier is `after_hours` instead of `intraday_odd_lot`.
pub(crate) fn frame_subscribe_futopt(
    sub: FutOptSubscription,
) -> Result<(String, Vec<SubscribeRequest>), MarketDataError> {
    let mut wire_req = SubscribeRequest {
        channel: sub.channel.as_str().to_string(),
        ..Default::default()
    };
    match &sub.symbols {
        Symbols::Single(s) => wire_req.symbol = Some(s.clone()),
        Symbols::Many(v) => wire_req.symbols = Some(v.clone()),
    }
    if sub.after_hours {
        wire_req.after_hours = Some(true);
    }

    let expanded = wire_req.clone().expand();
    let msg = WebSocketRequest::subscribe(wire_req);
    let json = serde_json::to_string(&msg)
        .map_err(|e| MarketDataError::DeserializationError { source: e })?;
    Ok((json, expanded))
}

/// One subscribe frame to re-send after a reconnect.
#[derive(Debug)]
pub(crate) struct ResubscribeFrame {
    /// Names what the frame subscribes, for the `Error` reported when it
    /// cannot be sent: the key for a single subscription, otherwise the
    /// channel, modifier and symbol count (`trades:oddlot (3 symbols)`).
    pub(crate) label: String,
    /// The serialized frame.
    pub(crate) frame: Result<String, MarketDataError>,
}

/// Fold the stored per-symbol rows back into one subscribe frame per channel
/// and modifier (#111), so a reconnect re-sends a 1000-symbol batch as one
/// frame rather than 1000.
///
/// Groups keep the order in which their channel/modifier first appears, and
/// symbols keep their order within a group. A group of one is sent with
/// `symbol`, larger groups with `symbols`. A row without a symbol is sent on
/// its own, unchanged.
pub(crate) fn frame_resubscribe(rows: Vec<SubscribeRequest>) -> Vec<ResubscribeFrame> {
    #[derive(PartialEq, Eq, Hash)]
    enum Group {
        /// channel, after hours, intraday odd lot
        Batch(String, bool, bool),
        /// A symbol-less row, identified by its position.
        Alone(usize),
    }

    let mut groups: IndexMap<Group, Vec<SubscribeRequest>> = IndexMap::new();
    for (i, row) in rows.into_iter().enumerate() {
        let group = match row.symbol {
            Some(_) => Group::Batch(
                row.channel.clone(),
                row.after_hours == Some(true),
                row.intraday_odd_lot == Some(true),
            ),
            None => Group::Alone(i),
        };
        groups.entry(group).or_default().push(row);
    }

    groups
        .into_values()
        .map(|mut rows| {
            let (label, req) = if rows.len() == 1 {
                let row = rows.remove(0);
                (row.key(), row)
            } else {
                let after_hours = rows[0].after_hours == Some(true);
                let odd_lot = rows[0].intraday_odd_lot == Some(true);
                // Same precedence as `SubscribeRequest::key()`: a row can only
                // carry both flags when built by hand, and then the label
                // names after-hours while the frame still sends both.
                let modifier = match (after_hours, odd_lot) {
                    (true, _) => ":afterhours",
                    (false, true) => ":oddlot",
                    _ => "",
                };
                let channel = rows[0].channel.clone();
                let label = format!("{channel}{modifier} ({} symbols)", rows.len());
                let req = SubscribeRequest {
                    channel,
                    symbol: None,
                    symbols: Some(rows.into_iter().filter_map(|row| row.symbol).collect()),
                    after_hours: after_hours.then_some(true),
                    intraday_odd_lot: odd_lot.then_some(true),
                };
                (label, req)
            };
            let frame = serde_json::to_string(&WebSocketRequest::subscribe(req))
                .map_err(|e| MarketDataError::DeserializationError { source: e });
            ResubscribeFrame { label, frame }
        })
        .collect()
}

/// Build the unsubscribe frame from a list of server ids. Sends
/// `{data:{id:"..."}}` for a single id and `{data:{ids:[...]}}` for many —
/// both shapes accepted by the Fugle server.
pub(crate) fn frame_unsubscribe(wire_ids: Vec<String>) -> Result<String, MarketDataError> {
    let unsub_req = if wire_ids.len() == 1 {
        UnsubscribeRequest::by_id(wire_ids.into_iter().next().unwrap())
    } else {
        UnsubscribeRequest::by_ids(wire_ids)
    };
    let msg = WebSocketRequest::unsubscribe(unsub_req);
    serde_json::to_string(&msg).map_err(|e| MarketDataError::DeserializationError { source: e })
}

/// Remove what each of `targets` names from `subscriptions` and return the
/// server ids to send, without duplicates (see
/// [`SubscriptionManager::resolve_unsubscribe`]).
pub(crate) fn unsubscribe_wire_ids(
    subscriptions: &SubscriptionManager,
    targets: &[String],
) -> Vec<String> {
    let mut wire_ids: Vec<String> = Vec::with_capacity(targets.len());
    for target in targets {
        if let Some(id) = subscriptions.resolve_unsubscribe(target) {
            wire_ids.push(id);
        }
    }
    dedupe_in_place(&mut wire_ids);
    wire_ids
}

/// Drop repeated ids, keeping the first of each. Two keys can share one
/// server id (a FutOpt alias and the contract it resolves to), and a caller
/// may name the same subscription twice.
fn dedupe_in_place(ids: &mut Vec<String>) {
    if ids.len() < 2 {
        return;
    }
    let mut seen: HashSet<String> = HashSet::with_capacity(ids.len());
    ids.retain(|id| seen.insert(id.clone()));
}

/// Serialize any [`WebSocketRequest`] (used by the public `send()` API).
pub(crate) fn frame_request(req: &WebSocketRequest) -> Result<String, MarketDataError> {
    serde_json::to_string(req).map_err(|e| MarketDataError::DeserializationError { source: e })
}

/// Parse an inbound text frame into a typed `WebSocketMessage`.
pub(crate) fn parse_text_frame(text: &str) -> Result<WebSocketMessage, MarketDataError> {
    let mut msg: WebSocketMessage =
        serde_json::from_str(text).map_err(|e| MarketDataError::DeserializationError { source: e })?;
    msg.raw = text.to_string();
    Ok(msg)
}

/// Parse an inbound binary frame into a typed `WebSocketMessage`.
pub(crate) fn parse_binary_frame(data: &[u8]) -> Result<WebSocketMessage, MarketDataError> {
    let mut msg: WebSocketMessage =
        serde_json::from_slice(data).map_err(|e| MarketDataError::DeserializationError { source: e })?;
    msg.raw = String::from_utf8_lossy(data).into_owned();
    Ok(msg)
}

/// Classify a frame received during the auth handshake.
///
/// An `error` frame is a rejection only when its `code` is
/// [`AUTH_REJECTED_CODE`]; any other code, or none, is a failure of this
/// attempt (`ConnectionError` naming the code and the server's message), not
/// of the credentials (#201).
pub(crate) fn classify_auth_response(msg: &WebSocketMessage) -> AuthOutcome {
    let data = || msg.data.clone().unwrap_or(serde_json::Value::Null);
    if msg.is_authenticated() {
        return AuthOutcome::Authenticated(data());
    }
    if !msg.is_error() {
        return AuthOutcome::Pending;
    }
    let message = msg.error_message().unwrap_or_else(|| "Unknown error".to_string());
    match msg.error_code() {
        Some(AUTH_REJECTED_CODE) => AuthOutcome::Rejected { message, data: data() },
        Some(code) => AuthOutcome::Failed(MarketDataError::ConnectionError {
            msg: format!("Authentication failed (server error {code}): {message}"),
        }),
        None => AuthOutcome::Failed(MarketDataError::ConnectionError {
            msg: format!("Authentication failed: {message}"),
        }),
    }
}

/// Build the subscription key used by `SubscriptionManager`, mirroring the
/// suffix rules in `SubscribeRequest::key()`. The suffix only appears when
/// the respective flag is explicitly true — matching server ack shapes that
/// may omit the field for regular sessions.
fn build_sub_key(channel: &str, symbol: &str, after_hours: bool, odd_lot: bool) -> String {
    let base = format!("{}:{}", channel, symbol);
    if after_hours {
        format!("{base}:afterhours")
    } else if odd_lot {
        format!("{base}:oddlot")
    } else {
        base
    }
}

/// If `msg` is a `subscribed` ack, record the server-assigned id in the
/// subscription manager. Supports two wire shapes observed in the Fugle
/// protocol:
///
/// - single: top-level `{event, id, channel, symbol, afterHours?, intradayOddLot?}`
/// - batched: `{event, data: [{id, channel, symbol, afterHours?, intradayOddLot?}, ...]}`
///
/// Any shape we can't parse is silently ignored.
///
/// Returns the ids of subscriptions unsubscribed before this ack arrived;
/// the caller sends the unsubscribe frame for them (#136).
pub(crate) fn handle_subscribed_event(
    subscriptions: &SubscriptionManager,
    msg: &WebSocketMessage,
) -> Vec<String> {
    let mut cancels = Vec::new();
    if msg.event != "subscribed" {
        return cancels;
    }

    // Batched shape: data is an array of subscription entries.
    if let Some(arr) = msg.data.as_ref().and_then(|d| d.as_array()) {
        for entry in arr {
            let Some(id) = entry.get("id").and_then(|v| v.as_str()) else {
                continue;
            };
            let Some(channel) = entry.get("channel").and_then(|v| v.as_str()) else {
                continue;
            };
            let Some(symbol) = entry.get("symbol").and_then(|v| v.as_str()) else {
                continue;
            };
            let after_hours = entry
                .get("afterHours")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let odd_lot = entry
                .get("intradayOddLot")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            cancels.extend(subscriptions.record_ack(
                build_sub_key(channel, symbol, after_hours, odd_lot),
                id.to_string(),
            ));
        }
        // One ack can carry two keys the server issued the same id for.
        dedupe_in_place(&mut cancels);
        return cancels;
    }

    // Single shape: pull fields from data object when present, falling back
    // to the WebSocketMessage top-level fields the model already exposes.
    let data_obj = msg.data.as_ref().and_then(|d| d.as_object());
    let id = data_obj
        .and_then(|d| d.get("id"))
        .and_then(|v| v.as_str())
        .map(String::from)
        .or_else(|| msg.id.clone());
    let channel = data_obj
        .and_then(|d| d.get("channel"))
        .and_then(|v| v.as_str())
        .map(String::from)
        .or_else(|| msg.channel.clone());
    let symbol = data_obj
        .and_then(|d| d.get("symbol"))
        .and_then(|v| v.as_str())
        .map(String::from)
        .or_else(|| msg.symbol.clone());
    let after_hours = data_obj
        .and_then(|d| d.get("afterHours"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let odd_lot = data_obj
        .and_then(|d| d.get("intradayOddLot"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if let (Some(id), Some(channel), Some(symbol)) = (id, channel, symbol) {
        cancels.extend(subscriptions.record_ack(
            build_sub_key(&channel, &symbol, after_hours, odd_lot),
            id,
        ));
    }
    cancels
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Channel;
    use serde_json::json;

    fn parse_msg(json: &str) -> WebSocketMessage {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn classify_authenticated_with_data() {
        let msg = parse_msg(r#"{"event":"authenticated","data":{"message":"Authenticated successfully"}}"#);
        assert!(matches!(
            classify_auth_response(&msg),
            AuthOutcome::Authenticated(data)
                if data == serde_json::json!({"message":"Authenticated successfully"})
        ));
    }

    #[test]
    fn classify_authenticated_without_data_is_null() {
        let msg = parse_msg(r#"{"event":"authenticated"}"#);
        assert!(matches!(
            classify_auth_response(&msg),
            AuthOutcome::Authenticated(serde_json::Value::Null)
        ));
    }

    #[test]
    fn classify_error_1000_is_a_rejection() {
        let msg = parse_msg(r#"{"event":"error","code":1000,"data":{"message":"Invalid token"}}"#);
        assert!(matches!(
            classify_auth_response(&msg),
            AuthOutcome::Rejected { message, data }
                if message == "Invalid token" && data == serde_json::json!({"message":"Invalid token"})
        ));
    }

    #[test]
    fn classify_error_1000_without_data() {
        let msg = parse_msg(r#"{"event":"error","code":1000}"#);
        assert!(matches!(
            classify_auth_response(&msg),
            AuthOutcome::Rejected { message, data: serde_json::Value::Null }
                if message == "Unknown error"
        ));
    }

    /// `1011` (auth service unavailable), `1004` (no auth request seen) and
    /// any code the SDK does not know are failures of the attempt, reported
    /// as `ConnectionError`, never as a rejection (#201).
    #[test]
    fn classify_other_error_codes_are_failures() {
        for (code, message) in [
            (1011, "Auth service unavailable"),
            (1004, "No authentication request received"),
            (4242, "future code"),
        ] {
            let msg = parse_msg(&format!(
                r#"{{"event":"error","code":{code},"data":{{"message":"{message}"}}}}"#
            ));
            match classify_auth_response(&msg) {
                AuthOutcome::Failed(MarketDataError::ConnectionError { msg }) => {
                    assert_eq!(msg, format!("Authentication failed (server error {code}): {message}"));
                }
                other => panic!("{code}: expected Failed(ConnectionError), got {other:?}"),
            }
        }
    }

    /// An `error` without a code is not the server's rejection shape (a
    /// rejection always carries `code: 1000`), so it is a failure. The
    /// message is read from `data.message` or, for the server's code-less
    /// `{"event":"error","message":"…"}` shape, the top level (#209).
    #[test]
    fn classify_error_without_code_is_a_failure() {
        let msg = parse_msg(r#"{"event":"error","data":{"message":"Invalid token"}}"#);
        match classify_auth_response(&msg) {
            AuthOutcome::Failed(MarketDataError::ConnectionError { msg }) => {
                assert_eq!(msg, "Authentication failed: Invalid token");
            }
            other => panic!("expected Failed(ConnectionError), got {other:?}"),
        }
        let msg = parse_msg(r#"{"event":"error","message":"Unauthorized"}"#);
        match classify_auth_response(&msg) {
            AuthOutcome::Failed(MarketDataError::ConnectionError { msg }) => {
                assert_eq!(msg, "Authentication failed: Unauthorized");
            }
            other => panic!("expected Failed(ConnectionError), got {other:?}"),
        }
        let msg = parse_msg(r#"{"event":"error"}"#);
        match classify_auth_response(&msg) {
            AuthOutcome::Failed(MarketDataError::ConnectionError { msg }) => {
                assert_eq!(msg, "Authentication failed: Unknown error");
            }
            other => panic!("expected Failed(ConnectionError), got {other:?}"),
        }
    }

    #[test]
    fn classify_other_event_is_pending() {
        let msg = parse_msg(r#"{"event":"pong"}"#);
        assert!(matches!(classify_auth_response(&msg), AuthOutcome::Pending));
    }

    #[test]
    fn test_handle_subscribed_ignores_non_subscribed() {
        let manager = SubscriptionManager::new();
        let msg = parse_msg(
            r#"{"event":"data","id":"sub-1","channel":"trades","symbol":"2330"}"#,
        );
        handle_subscribed_event(&manager, &msg);
        assert!(manager.take_server_id("trades:2330").is_none());
    }

    #[test]
    fn test_handle_subscribed_single_top_level() {
        let manager = SubscriptionManager::new();
        let msg = parse_msg(
            r#"{"event":"subscribed","id":"sub-abc","channel":"trades","symbol":"2330"}"#,
        );
        handle_subscribed_event(&manager, &msg);
        assert_eq!(
            manager.take_server_id("trades:2330"),
            Some("sub-abc".to_string())
        );
    }

    #[test]
    fn test_handle_subscribed_single_with_after_hours() {
        let manager = SubscriptionManager::new();
        let msg = parse_msg(
            r#"{"event":"subscribed","data":{"id":"sub-ah","channel":"books","symbol":"TXFE6","afterHours":true}}"#,
        );
        handle_subscribed_event(&manager, &msg);
        assert_eq!(
            manager.take_server_id("books:TXFE6:afterhours"),
            Some("sub-ah".to_string())
        );
        // Without the suffix it's a different key — mustn't collide.
        assert!(manager.take_server_id("books:TXFE6").is_none());
    }

    #[test]
    fn test_handle_subscribed_single_with_odd_lot() {
        let manager = SubscriptionManager::new();
        let msg = parse_msg(
            r#"{"event":"subscribed","data":{"id":"sub-odd","channel":"trades","symbol":"2330","intradayOddLot":true}}"#,
        );
        handle_subscribed_event(&manager, &msg);
        assert_eq!(
            manager.take_server_id("trades:2330:oddlot"),
            Some("sub-odd".to_string())
        );
    }

    #[test]
    fn test_handle_subscribed_batched_array() {
        let manager = SubscriptionManager::new();
        let msg = parse_msg(
            r#"{"event":"subscribed","data":[
                {"id":"sub-1","channel":"trades","symbol":"2330"},
                {"id":"sub-2","channel":"books","symbol":"TXFE6","afterHours":true},
                {"id":"sub-3","channel":"trades","symbol":"2317","intradayOddLot":true}
            ]}"#,
        );
        handle_subscribed_event(&manager, &msg);
        assert_eq!(manager.take_server_id("trades:2330"), Some("sub-1".into()));
        assert_eq!(
            manager.take_server_id("books:TXFE6:afterhours"),
            Some("sub-2".into())
        );
        assert_eq!(
            manager.take_server_id("trades:2317:oddlot"),
            Some("sub-3".into())
        );
    }

    #[test]
    fn test_handle_subscribed_missing_fields_no_op() {
        let manager = SubscriptionManager::new();
        // No id, no channel — nothing to record.
        let msg = parse_msg(r#"{"event":"subscribed","symbol":"2330"}"#);
        handle_subscribed_event(&manager, &msg);
        assert!(manager.take_server_id("trades:2330").is_none());
    }

    #[test]
    fn handle_subscribed_returns_ids_unsubscribed_before_ack() {
        let manager = SubscriptionManager::new();
        manager.subscribe(SubscribeRequest::new(Channel::Trades, "2330"));
        manager.subscribe(SubscribeRequest::new(Channel::Trades, "2317"));
        assert_eq!(manager.resolve_unsubscribe("trades:2330"), None);

        let msg = parse_msg(
            r#"{"event":"subscribed","data":[
                {"id":"sub-1","channel":"trades","symbol":"2330"},
                {"id":"sub-2","channel":"trades","symbol":"2317"}
            ]}"#,
        );
        assert_eq!(handle_subscribed_event(&manager, &msg), ["sub-1"]);
        assert!(manager.take_server_id("trades:2330").is_none());
        assert_eq!(manager.take_server_id("trades:2317"), Some("sub-2".into()));
    }

    #[test]
    fn handle_subscribed_sends_one_cancel_per_shared_server_id() {
        let manager = SubscriptionManager::new();
        // A FutOpt alias and the contract it resolves to: the server issues
        // one id for both, so one unsubscribe covers them.
        manager.subscribe(SubscribeRequest::new(Channel::Trades, "TXF1"));
        manager.subscribe(SubscribeRequest::new(Channel::Trades, "TXFK6"));
        assert_eq!(manager.resolve_unsubscribe("trades:TXF1"), None);
        assert_eq!(manager.resolve_unsubscribe("trades:TXFK6"), None);

        let msg = parse_msg(
            r#"{"event":"subscribed","data":[
                {"id":"sub-1","channel":"trades","symbol":"TXF1"},
                {"id":"sub-1","channel":"trades","symbol":"TXFK6"}
            ]}"#,
        );
        assert_eq!(handle_subscribed_event(&manager, &msg), ["sub-1"]);
    }

    fn resubscribe(rows: Vec<SubscribeRequest>) -> Vec<(String, serde_json::Value)> {
        frame_resubscribe(rows)
            .into_iter()
            .map(|f| {
                let frame = serde_json::from_str(&f.frame.expect("frame serializes")).unwrap();
                (f.label, frame)
            })
            .collect()
    }

    fn odd_lot(symbol: &str) -> SubscribeRequest {
        SubscribeRequest {
            intraday_odd_lot: Some(true),
            ..SubscribeRequest::new(Channel::Trades, symbol)
        }
    }

    #[test]
    fn resubscribe_batches_by_channel_and_modifier_in_first_seen_order() {
        let frames = resubscribe(vec![
            SubscribeRequest::new(Channel::Trades, "2330"),
            SubscribeRequest::new(Channel::Books, "2317"),
            odd_lot("2330"),
            SubscribeRequest::new(Channel::Trades, "2454"),
            SubscribeRequest::new(Channel::Books, "0050"),
            odd_lot("2603"),
            SubscribeRequest::new(Channel::Trades, "2881"),
        ]);

        assert_eq!(frames.len(), 3);
        assert_eq!(frames[0].0, "trades (3 symbols)");
        assert_eq!(
            frames[0].1,
            json!({"event":"subscribe","data":{"channel":"trades","symbols":["2330","2454","2881"]}})
        );
        assert_eq!(frames[1].0, "books (2 symbols)");
        assert_eq!(
            frames[1].1,
            json!({"event":"subscribe","data":{"channel":"books","symbols":["2317","0050"]}})
        );
        assert_eq!(frames[2].0, "trades:oddlot (2 symbols)");
        assert_eq!(
            frames[2].1,
            json!({"event":"subscribe","data":{"channel":"trades","symbols":["2330","2603"],"intradayOddLot":true}})
        );
    }

    #[test]
    fn resubscribe_single_symbol_group_keeps_symbol_field_and_key_label() {
        let after_hours = SubscribeRequest {
            channel: "books".into(),
            symbol: Some("TXFE6".into()),
            after_hours: Some(true),
            ..Default::default()
        };
        let frames = resubscribe(vec![after_hours]);

        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].0, "books:TXFE6:afterhours");
        assert_eq!(
            frames[0].1,
            json!({"event":"subscribe","data":{"channel":"books","symbol":"TXFE6","afterHours":true}})
        );
    }

    #[test]
    fn resubscribe_treats_explicit_false_modifier_as_regular_session() {
        let explicit_false = SubscribeRequest {
            after_hours: Some(false),
            intraday_odd_lot: Some(false),
            ..SubscribeRequest::new(Channel::Trades, "2454")
        };
        let frames = resubscribe(vec![SubscribeRequest::new(Channel::Trades, "2330"), explicit_false]);

        assert_eq!(frames.len(), 1);
        assert_eq!(
            frames[0].1,
            json!({"event":"subscribe","data":{"channel":"trades","symbols":["2330","2454"]}})
        );
    }

    #[test]
    fn resubscribe_sends_symbol_less_rows_on_their_own() {
        let channel_only = || SubscribeRequest {
            channel: "indices".into(),
            ..Default::default()
        };
        let frames = resubscribe(vec![channel_only(), channel_only()]);

        assert_eq!(frames.len(), 2);
        for (label, frame) in &frames {
            assert_eq!(label, "indices");
            assert_eq!(frame, &json!({"event":"subscribe","data":{"channel":"indices"}}));
        }
    }

    #[test]
    fn resubscribe_nothing_stored_sends_nothing() {
        assert!(frame_resubscribe(Vec::new()).is_empty());
    }
}
