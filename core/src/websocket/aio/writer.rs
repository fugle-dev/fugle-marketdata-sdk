//! Single-writer task that drains outbound JSON frames into the WS sink.

use crate::websocket::aio::WsSink;
use crate::websocket::stream_queue::StreamSender;
use crate::websocket::ConnectionEvent;
use crate::MarketDataError;
use futures_util::SinkExt;
use std::sync::Arc;
use tokio::sync::mpsc as tokio_mpsc;
use tokio::sync::oneshot;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::Message;

/// A failed write, handed from the writer task to its dispatch loop.
#[derive(Debug)]
pub(crate) struct WriteFailure {
    pub(crate) error: MarketDataError,
    pub(crate) message: String,
}

/// Outbound queue capacity. Generous for a ping-every-30s + occasional
/// sub/unsub workload while staying small enough to surface backpressure if
/// the sink stalls.
const WRITE_QUEUE_CAPACITY: usize = 64;

/// Spawn the writer task of a connection whose write half is in `ws_sink`.
/// Returns the sender that queues its frames, the task, and the receiver of
/// its failed write for the connection's dispatch loop.
pub(crate) fn spawn_writer(
    ws_sink: Arc<Mutex<Option<WsSink>>>,
    stream: StreamSender,
) -> (
    tokio_mpsc::Sender<String>,
    JoinHandle<()>,
    oneshot::Receiver<WriteFailure>,
) {
    let (write_tx, write_rx) = tokio_mpsc::channel(WRITE_QUEUE_CAPACITY);
    let (write_failed_tx, write_failed_rx) = oneshot::channel();
    let handle = tokio::spawn(run_writer_task(write_rx, ws_sink, stream, write_failed_tx));
    (write_tx, handle, write_failed_rx)
}

/// Single-writer task body. Drains pre-serialized JSON strings from `rx`
/// and writes them as text frames to the shared `ws_sink`. Exits when the
/// channel closes or when a write fails.
///
/// A failed write ends the connection, as it does on the sync client: the
/// error goes to this connection's dispatch loop through `write_failed`,
/// which reports `Error` then `Disconnected` and decides on reconnecting
/// (#97). With no dispatch loop left to take it, `Error` is reported here.
async fn run_writer_task(
    mut rx: tokio_mpsc::Receiver<String>,
    ws_sink: Arc<Mutex<Option<WsSink>>>,
    stream: StreamSender,
    write_failed: oneshot::Sender<WriteFailure>,
) {
    while let Some(text) = rx.recv().await {
        let mut sink_guard = ws_sink.lock().await;
        let Some(sink) = sink_guard.as_mut() else {
            // Sink has been cleared (disconnect/force_close). Stop draining.
            break;
        };
        if let Err(e) = sink.send(Message::Text(text.into())).await {
            // Same wording as the sync client's write error.
            let failure = WriteFailure {
                message: format!("WebSocket write error: {e}"),
                error: e.into(),
            };
            if let Err(failure) = write_failed.send(failure) {
                stream.emit(ConnectionEvent::error_with_message(
                    &failure.error,
                    failure.message,
                ));
            }
            break;
        }
    }
}
