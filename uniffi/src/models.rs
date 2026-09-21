//! FFI model types.
//!
//! REST responses are handed to the caller as the server's raw JSON string
//! (see `crate::client`), so this module no longer mirrors the response
//! models. The mirrors were a hand-maintained second copy of
//! `marketdata_core::models` and had drifted badly: 22 fields were silently
//! missing, 11 of them on `FutOptQuote` alone, which made futures quotes
//! largely unusable from C#, Go, C++ and Java.
//!
//! What remains is the streaming envelope, which carries routing fields the
//! binding callbacks dispatch on.

use std::borrow::Cow;

use marketdata_core::models as core;

// ============================================================================
// WebSocket Message Model
// ============================================================================

/// An inbound streaming frame.
///
/// `raw` is the frame exactly as the server sent it — decode that when you
/// want the payload. The other fields are the routing subset this SDK parses
/// out so callbacks can dispatch without decoding the whole frame first; they
/// are a convenience, not the source of truth.
#[derive(Debug, Clone, uniffi::Record)]
pub struct StreamMessage {
    /// The frame verbatim, as received on the wire.
    pub raw: String,
    /// Event type: "data", "subscribed", "error", "authenticated", "pong".
    pub event: String,
    /// Channel name, for data events.
    pub channel: Option<String>,
    /// Symbol, for data events.
    pub symbol: Option<String>,
    /// Subscription id, for subscribed events.
    pub id: Option<String>,
    /// The `data` member of the frame, still encoded as JSON.
    pub data_json: Option<String>,
    /// Server error code, for error events: `1000` credentials rejected,
    /// `1001` subscription limit exceeded, `1002` command before
    /// authentication, `1003` request validation failed, `1004` no auth
    /// request within 60 s, `1011` auth service unavailable. Absent when the
    /// server sent an error frame without a code.
    pub error_code: Option<i32>,
    /// Error message, for error events: the frame's `data.message`, or its
    /// top-level `message` when the server sent the code-less shape.
    pub error_message: Option<String>,
}

impl From<core::WebSocketMessage> for StreamMessage {
    fn from(msg: core::WebSocketMessage) -> Self {
        // Core knows the frame shape (`code` at the top level, `message`
        // under `data` or at the top level); both return `None` off an
        // error frame.
        let error_code = msg.error_code();
        let error_message = msg.error_message();
        let data_json = msg.data_json().map(Cow::into_owned);

        Self {
            raw: msg.raw,
            event: msg.event,
            channel: msg.channel,
            symbol: msg.symbol,
            id: msg.id,
            data_json,
            error_code,
            error_message,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(json: &str) -> StreamMessage {
        core::WebSocketMessage::parse(json).unwrap().into()
    }

    #[test]
    fn error_code_comes_from_the_top_level() {
        // The server's shape (`ws-exception.filter.ts`, #209): `code` next to
        // `event`, the message under `data`.
        let msg = frame(
            r#"{"event":"error","code":1000,"data":{"message":"Invalid authentication credentials"}}"#,
        );
        assert_eq!(msg.event, "error");
        assert_eq!(msg.error_code, Some(1000));
        assert_eq!(
            msg.error_message.as_deref(),
            Some("Invalid authentication credentials")
        );
        assert_eq!(
            msg.data_json.as_deref(),
            Some(r#"{"message":"Invalid authentication credentials"}"#)
        );
    }

    #[test]
    fn error_without_code_reads_the_top_level_message() {
        let msg = frame(r#"{"event":"error","message":"Unauthorized"}"#);
        assert_eq!(msg.error_code, None);
        assert_eq!(msg.error_message.as_deref(), Some("Unauthorized"));
        assert_eq!(msg.data_json, None);
    }

    #[test]
    fn non_error_frames_carry_no_error_fields() {
        let json = r#"{"event":"data","code":7,"channel":"trades","symbol":"2330","data":{"price":1}}"#;
        let msg = frame(json);
        assert_eq!(msg.error_code, None);
        assert_eq!(msg.error_message, None);
        assert_eq!(msg.channel.as_deref(), Some("trades"));
        assert_eq!(msg.symbol.as_deref(), Some("2330"));
        assert_eq!(msg.raw, json);
    }

    #[test]
    fn data_json_is_the_frame_slice_as_sent() {
        // Re-serializing a `Value` would print `1e21` and drop the spaces.
        let msg = frame(r#"{"event":"data","data":{"price": 583.0, "n": 1e+21}}"#);
        assert_eq!(msg.data_json.as_deref(), Some(r#"{"price": 583.0, "n": 1e+21}"#));
    }
}
