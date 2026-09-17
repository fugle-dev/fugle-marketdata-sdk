//! Single-writer task that drains outbound JSON frames into the WS sink.

use crate::websocket::aio::WsSink;
use crate::websocket::stream_queue::StreamSender;
use crate::websocket::ConnectionEvent;
use crate::MarketDataError;
use futures_util::SinkExt;
use std::sync::atomic::{AtomicU64, Ordering};
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

/// Generation of the connection whose writer is current. Each new writer
/// takes the next generation; [`retire_writer`] moves past the current one
/// once its connection is gone, so its writer stays silent (#105).
pub(crate) type WriterGeneration = Arc<AtomicU64>;

/// Make the current writer stale: a failed write it has yet to report is
/// dropped instead of reported as `Error`.
pub(crate) fn retire_writer(generation: &WriterGeneration) {
    generation.fetch_add(1, Ordering::SeqCst);
}

/// Install `sink` as the connection's write half and spawn its writer.
///
/// The previous writer is stopped and awaited before `sink` is installed,
/// so none of its queued frames can reach the new socket (#105). The new
/// sender goes into `write_tx`, the task into `writer_handle`. Returns the
/// new sender, and the receiver of the new writer's failed write for its
/// dispatch loop.
pub(crate) async fn start_writer(
    sink: WsSink,
    ws_sink: &Arc<Mutex<Option<WsSink>>>,
    write_tx: &Mutex<Option<tokio_mpsc::Sender<String>>>,
    writer_handle: &Mutex<Option<JoinHandle<()>>>,
    generation: &WriterGeneration,
    stream: StreamSender,
) -> (tokio_mpsc::Sender<String>, oneshot::Receiver<WriteFailure>) {
    // Both slots are held from here on, so nothing awaits once the writer is
    // spawned: a caller aborted inside this call cannot lose its handle.
    let mut write_tx = write_tx.lock().await;
    let mut writer_handle = writer_handle.lock().await;
    if let Some(previous) = writer_handle.take() {
        previous.abort();
        let _ = previous.await;
    }
    *ws_sink.lock().await = Some(sink);

    let (tx, rx) = tokio_mpsc::channel(WRITE_QUEUE_CAPACITY);
    let (write_failed_tx, write_failed_rx) = oneshot::channel();
    let current = generation.fetch_add(1, Ordering::SeqCst) + 1;
    *writer_handle = Some(tokio::spawn(run_writer_task(
        rx,
        Arc::clone(ws_sink),
        stream,
        write_failed_tx,
        Arc::clone(generation),
        current,
    )));
    *write_tx = Some(tx.clone());
    (tx, write_failed_rx)
}

/// Single-writer task body. Drains pre-serialized JSON strings from `rx`
/// and writes them as text frames to the shared `ws_sink`. Exits when the
/// channel closes or when a write fails.
///
/// A failed write ends the connection, as it does on the sync client: the
/// error goes to this connection's dispatch loop through `write_failed`,
/// which reports `Error` then `Disconnected` and decides on reconnecting
/// (#97). With no dispatch loop left to take it, `Error` is reported here,
/// unless this writer's connection was retired or replaced (#105).
async fn run_writer_task(
    mut rx: tokio_mpsc::Receiver<String>,
    ws_sink: Arc<Mutex<Option<WsSink>>>,
    stream: StreamSender,
    write_failed: oneshot::Sender<WriteFailure>,
    generation: WriterGeneration,
    current: u64,
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
                if generation.load(Ordering::SeqCst) == current {
                    stream.emit(ConnectionEvent::error_with_message(
                        &failure.error,
                        failure.message,
                    ));
                }
            }
            break;
        }
    }
}
