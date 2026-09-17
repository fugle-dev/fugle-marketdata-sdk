//! The ordered stream a WebSocket client reports through (#68).
//!
//! Every inbound message and every [`ConnectionEvent`] of a client arrives
//! as one [`StreamItem`], in the order the client produced them. See
//! [`connection_event`](crate::websocket::connection_event) for the
//! ordering guarantees. [`StreamReceiver`] has a blocking API for FFI
//! bindings and the sync client; [`ConnectionStream`] is the async one.
//! Runtime-free: shared by the sync `WebSocketClient` and the async
//! `aio::WebSocketClient`.

use crate::models::WebSocketMessage;
use crate::websocket::stream_queue::QueueReceiver;
use crate::websocket::ConnectionEvent;
use crate::MarketDataError;
use std::sync::mpsc::{RecvTimeoutError, TryRecvError};
use std::task::{Context, Poll};
use std::time::Duration;

/// One item of a client's stream.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum StreamItem {
    /// A frame the server sent.
    Message(WebSocketMessage),
    /// A change in the connection's lifecycle, or a diagnostic about it.
    Event(ConnectionEvent),
}

/// Error for a receiver whose client is gone and whose stream is drained.
fn closed() -> MarketDataError {
    MarketDataError::ConnectionError {
        msg: "Stream closed".to_string(),
    }
}

/// Blocking receiver of a client's stream, for FFI bindings and the sync
/// client.
///
/// Thread-safe: callers can share it across threads, and each item goes to
/// exactly one of them. The stream closes once the client has been dropped
/// and every queued item has been read.
pub struct StreamReceiver {
    rx: QueueReceiver,
}

impl StreamReceiver {
    pub(crate) fn new(rx: QueueReceiver) -> Self {
        Self { rx }
    }

    /// Receive the next item, blocking until there is one.
    ///
    /// # Errors
    ///
    /// Returns `ConnectionError` once the stream is closed.
    pub fn receive(&self) -> Result<StreamItem, MarketDataError> {
        self.rx.recv().ok_or_else(closed)
    }

    /// Receive the next item, waiting at most `timeout`.
    ///
    /// Returns `Ok(Some(item))` when one arrived and `Ok(None)` when the
    /// timeout elapsed first.
    ///
    /// # Errors
    ///
    /// Returns `ConnectionError` once the stream is closed.
    pub fn receive_timeout(&self, timeout: Duration) -> Result<Option<StreamItem>, MarketDataError> {
        match self.rx.recv_timeout(timeout) {
            Ok(item) => Ok(Some(item)),
            Err(RecvTimeoutError::Timeout) => Ok(None),
            Err(RecvTimeoutError::Disconnected) => Err(closed()),
        }
    }

    /// Take the next item without waiting: `None` when none is queued or the
    /// stream is closed.
    pub fn try_receive(&self) -> Option<StreamItem> {
        self.rx.try_recv().ok()
    }
}

/// Async stream of a client's items, returned by
/// [`aio::WebSocketClient::stream`](crate::aio::WebSocketClient::stream).
///
/// Offers [`recv`](Self::recv), [`try_recv`](Self::try_recv) and
/// [`poll_recv`](Self::poll_recv). With the `tokio-comp` feature it is also a
/// `futures::Stream`. It ends once the client has been dropped and every
/// queued item has been read.
pub struct ConnectionStream {
    rx: QueueReceiver,
}

impl ConnectionStream {
    #[cfg_attr(not(feature = "tokio-comp"), allow(dead_code))]
    pub(crate) fn new(rx: QueueReceiver) -> Self {
        Self { rx }
    }

    /// Wait for the next item; `None` once the stream has ended.
    ///
    /// Cancel safe: an item is only taken from the stream when this returns
    /// it.
    pub async fn recv(&mut self) -> Option<StreamItem> {
        std::future::poll_fn(|cx| self.rx.poll_recv(cx)).await
    }

    /// Take the next item without waiting.
    ///
    /// # Errors
    /// [`TryRecvError::Empty`] when no item is queued,
    /// [`TryRecvError::Disconnected`] once the stream has ended.
    pub fn try_recv(&mut self) -> Result<StreamItem, TryRecvError> {
        self.rx.try_recv()
    }

    /// Poll for the next item; `Ready(None)` once the stream has ended.
    pub fn poll_recv(&mut self, cx: &mut Context<'_>) -> Poll<Option<StreamItem>> {
        self.rx.poll_recv(cx)
    }
}

#[cfg(feature = "tokio-comp")]
impl futures_util::Stream for ConnectionStream {
    type Item = StreamItem;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        self.poll_recv(cx)
    }
}
