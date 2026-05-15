//! Single-writer task that drains outbound JSON frames into the WS sink.

use crate::websocket::aio::WsSink;
use crate::websocket::connection_event::emit_event;
use crate::websocket::ConnectionEvent;
use crate::MarketDataError;
use futures_util::SinkExt;
use std::sync::mpsc;
use std::sync::Arc;
use tokio::sync::mpsc as tokio_mpsc;
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::Message;

/// Single-writer task body. Drains pre-serialized JSON strings from `rx`
/// and writes them as text frames to the shared `ws_sink`. Exits when the
/// channel closes or when a write fails. Errors are reported via `event_tx`.
pub(crate) async fn run_writer_task(
    mut rx: tokio_mpsc::Receiver<String>,
    ws_sink: Arc<Mutex<Option<WsSink>>>,
    event_tx: mpsc::SyncSender<ConnectionEvent>,
) {
    while let Some(text) = rx.recv().await {
        let mut sink_guard = ws_sink.lock().await;
        let Some(sink) = sink_guard.as_mut() else {
            // Sink has been cleared (disconnect/force_close). Stop draining.
            break;
        };
        if let Err(e) = sink.send(Message::Text(text.into())).await {
            let err: MarketDataError = e.into();
            emit_event(&event_tx, ConnectionEvent::Error {
                message: format!("Writer error: {}", err),
                code: err.to_error_code(),
            });
            break;
        }
    }
}
