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
    pub event: String,
    pub channel: Option<String>,
    pub symbol: Option<String>,
    pub id: Option<String>,
    pub data_json: Option<String>,
    pub error_code: Option<i32>,
    pub error_message: Option<String>,
}

impl From<core::WebSocketMessage> for StreamMessage {
    fn from(msg: core::WebSocketMessage) -> Self {
        // Extract error info from data if event is "error"
        let (error_code, error_message) = if msg.event == "error" {
            let code = msg
                .data
                .as_ref()
                .and_then(|d| d.get("code"))
                .and_then(|v| v.as_i64())
                .map(|c| c as i32);
            let message = msg
                .data
                .as_ref()
                .and_then(|d| d.get("message"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            (code, message)
        } else {
            (None, None)
        };

        Self {
            raw: msg.raw,
            event: msg.event,
            channel: msg.channel,
            symbol: msg.symbol,
            id: msg.id,
            data_json: msg.data.map(|d| d.to_string()),
            error_code,
            error_message,
        }
    }
}
