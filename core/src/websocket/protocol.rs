//! Wire-protocol framing and parsing helpers.
//!
//! Runtime-free pure functions. Shared by sync and async WebSocket clients.
//! Wraps the model-layer constructors (`WebSocketRequest::{auth, subscribe,
//! unsubscribe}` in `crate::models::subscription`) with `serde_json`
//! serialization, plus extracts subscription-ack bookkeeping that was
//! previously inlined in the async dispatch loop.

use crate::models::{
    AuthRequest, SubscribeRequest, SymbolSpec, UnsubscribeRequest, WebSocketMessage,
    WebSocketRequest,
};
use crate::websocket::channels::{FutOptSubscription, StockSubscription};
use crate::websocket::SubscriptionManager;
use crate::MarketDataError;

/// Classification of the inbound auth response.
pub(crate) enum AuthOutcome {
    /// Server accepted the credentials. Caller should transition to `Connected`.
    Authenticated,
    /// Server rejected the credentials. Caller should emit `Unauthenticated`.
    Failed(String),
    /// Frame is not an auth-related event. Caller should keep reading.
    Pending,
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
        SymbolSpec::Single(s) => wire_req.symbol = Some(s.clone()),
        SymbolSpec::Many(v) => wire_req.symbols = Some(v.clone()),
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
        SymbolSpec::Single(s) => wire_req.symbol = Some(s.clone()),
        SymbolSpec::Many(v) => wire_req.symbols = Some(v.clone()),
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

/// Re-serialize an arbitrary stored `SubscribeRequest` row for the
/// resubscribe-after-reconnect path.
pub(crate) fn frame_subscribe_raw(req: SubscribeRequest) -> Result<String, MarketDataError> {
    let msg = WebSocketRequest::subscribe(req);
    serde_json::to_string(&msg).map_err(|e| MarketDataError::DeserializationError { source: e })
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

/// Serialize any [`WebSocketRequest`] (used by the public `send()` API).
pub(crate) fn frame_request(req: &WebSocketRequest) -> Result<String, MarketDataError> {
    serde_json::to_string(req).map_err(|e| MarketDataError::DeserializationError { source: e })
}

/// Parse an inbound text frame into a typed `WebSocketMessage`.
pub(crate) fn parse_text_frame(text: &str) -> Result<WebSocketMessage, MarketDataError> {
    serde_json::from_str(text).map_err(|e| MarketDataError::DeserializationError { source: e })
}

/// Parse an inbound binary frame into a typed `WebSocketMessage`.
pub(crate) fn parse_binary_frame(data: &[u8]) -> Result<WebSocketMessage, MarketDataError> {
    serde_json::from_slice(data).map_err(|e| MarketDataError::DeserializationError { source: e })
}

/// Classify a frame received during the auth handshake.
pub(crate) fn classify_auth_response(msg: &WebSocketMessage) -> AuthOutcome {
    if msg.is_authenticated() {
        AuthOutcome::Authenticated
    } else if msg.is_error() {
        AuthOutcome::Failed(
            msg.error_message().unwrap_or_else(|| "Unknown error".to_string()),
        )
    } else {
        AuthOutcome::Pending
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
/// Any shape we can't parse is silently ignored — the unsub fallback path
/// (sending the local key as id) keeps the wire format valid even without
/// a recorded server id.
pub(crate) fn handle_subscribed_event(
    subscriptions: &SubscriptionManager,
    msg: &WebSocketMessage,
) {
    if msg.event != "subscribed" {
        return;
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
            subscriptions.record_server_id(
                build_sub_key(channel, symbol, after_hours, odd_lot),
                id.to_string(),
            );
        }
        return;
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
        subscriptions.record_server_id(
            build_sub_key(&channel, &symbol, after_hours, odd_lot),
            id,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_msg(json: &str) -> WebSocketMessage {
        serde_json::from_str(json).unwrap()
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
}
