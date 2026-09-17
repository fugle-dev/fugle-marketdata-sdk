//! Consumer handles for inbound WebSocket messages.
//!
//! [`MessageReceiver`] has a blocking API for FFI bindings; [`MessageStream`]
//! is the async one. Both read the client's inbound message queue directly.
//! Runtime-free: shared by the sync `WebSocketClient` and the async
//! `aio::WebSocketClient`.

use crate::models::WebSocketMessage;
use crate::websocket::message_queue::QueueReceiver;
use crate::MarketDataError;
use std::sync::mpsc::{RecvTimeoutError, TryRecvError};
use std::task::{Context, Poll};
use std::time::Duration;

/// Error for a receiver whose client is gone and whose queue is drained.
fn closed() -> MarketDataError {
    MarketDataError::ConnectionError {
        msg: "Message channel closed".to_string(),
    }
}

/// FFI-safe message receiver with blocking API.
///
/// Thread-safe: callers can share it across threads, and each message goes
/// to exactly one of them. The queue closes once the client has been
/// dropped and every queued message has been read.
pub struct MessageReceiver {
    rx: QueueReceiver<WebSocketMessage>,
}

impl MessageReceiver {
    pub(crate) fn new(rx: QueueReceiver<WebSocketMessage>) -> Self {
        Self { rx }
    }

    /// Receive a message (blocking)
    ///
    /// Blocks until a message is received or channel is closed.
    ///
    /// # Errors
    ///
    /// Returns `ConnectionError` if channel is closed
    pub fn receive(&self) -> Result<WebSocketMessage, MarketDataError> {
        self.rx.recv().ok_or_else(closed)
    }

    /// Receive a message with timeout
    ///
    /// Returns:
    /// - `Ok(Some(msg))` if message received within timeout
    /// - `Ok(None)` if timeout elapsed with no message
    /// - `Err` if channel closed
    ///
    /// # Errors
    /// Returns [`MarketDataError`] on transport, protocol, deserialization,
    /// validation, or peer-initiated failures.
    pub fn receive_timeout(
        &self,
        timeout: Duration,
    ) -> Result<Option<WebSocketMessage>, MarketDataError> {
        match self.rx.recv_timeout(timeout) {
            Ok(msg) => Ok(Some(msg)),
            Err(RecvTimeoutError::Timeout) => Ok(None),
            Err(RecvTimeoutError::Disconnected) => Err(closed()),
        }
    }

    /// Try to receive a message without blocking
    ///
    /// Returns:
    /// - `Some(msg)` if message available
    /// - `None` if no message available or channel closed
    pub fn try_receive(&self) -> Option<WebSocketMessage> {
        self.rx.try_recv().ok()
    }
}

/// Async stream of inbound messages, returned by
/// [`aio::WebSocketClient::message_stream`](crate::aio::WebSocketClient::message_stream).
///
/// Mirrors the parts of `tokio::sync::mpsc::Receiver` a consumer uses:
/// [`recv`](Self::recv), [`try_recv`](Self::try_recv) and
/// [`poll_recv`](Self::poll_recv). With the `tokio-comp` feature it is also
/// a `futures::Stream`. The stream ends once the client has been dropped
/// and every queued message has been read.
pub struct MessageStream {
    rx: QueueReceiver<WebSocketMessage>,
}

impl MessageStream {
    #[cfg_attr(not(feature = "tokio-comp"), allow(dead_code))]
    pub(crate) fn new(rx: QueueReceiver<WebSocketMessage>) -> Self {
        Self { rx }
    }

    /// Wait for the next message; `None` once the stream has ended.
    ///
    /// Cancel safe: a message is only taken from the queue when this
    /// returns it.
    pub async fn recv(&mut self) -> Option<WebSocketMessage> {
        std::future::poll_fn(|cx| self.rx.poll_recv(cx)).await
    }

    /// Take the next message without waiting.
    ///
    /// # Errors
    /// [`TryRecvError::Empty`] when no message is queued,
    /// [`TryRecvError::Disconnected`] once the stream has ended.
    pub fn try_recv(&mut self) -> Result<WebSocketMessage, TryRecvError> {
        self.rx.try_recv()
    }

    /// Poll for the next message; `Ready(None)` once the stream has ended.
    pub fn poll_recv(&mut self, cx: &mut Context<'_>) -> Poll<Option<WebSocketMessage>> {
        self.rx.poll_recv(cx)
    }
}

#[cfg(feature = "tokio-comp")]
impl futures_util::Stream for MessageStream {
    type Item = WebSocketMessage;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        self.poll_recv(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics_compat::DropCounter;
    use crate::websocket::message_queue::queue;

    #[test]
    fn test_receive_blocking() {
        let (tx, rx) = queue(None, DropCounter::new("test_messages_dropped", "localhost", "test"));
        let receiver = MessageReceiver::new(rx);

        // Spawn thread to send message
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(10));
            let msg = WebSocketMessage {
                event: "data".to_string(),
                data: None,
                channel: Some("trades".to_string()),
                symbol: Some("2330".to_string()),
                id: None,
                raw: String::new(),
            };
            tx.push(msg);
        });

        // Should block and receive
        let result = receiver.receive();
        assert!(result.is_ok());
        let msg = result.unwrap();
        assert_eq!(msg.event, "data");
        assert_eq!(msg.channel, Some("trades".to_string()));
    }

    #[test]
    fn test_receive_timeout_returns_none() {
        let (_tx, rx) = queue(None, DropCounter::new("test_messages_dropped", "localhost", "test"));
        let receiver = MessageReceiver::new(rx);

        // No message sent, should timeout
        let result = receiver.receive_timeout(Duration::from_millis(50));
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_receive_timeout_returns_message() {
        let (tx, rx) = queue(None, DropCounter::new("test_messages_dropped", "localhost", "test"));
        let receiver = MessageReceiver::new(rx);

        // Send message immediately
        let msg = WebSocketMessage {
            event: "data".to_string(),
            data: None,
            channel: Some("trades".to_string()),
            symbol: Some("2330".to_string()),
            id: None,
            raw: String::new(),
        };
        tx.push(msg);

        // Should receive before timeout
        let result = receiver.receive_timeout(Duration::from_secs(1));
        assert!(result.is_ok());
        let received = result.unwrap();
        assert!(received.is_some());
        assert_eq!(received.unwrap().event, "data");
    }

    #[test]
    fn test_try_receive_non_blocking() {
        let (tx, rx) = queue(None, DropCounter::new("test_messages_dropped", "localhost", "test"));
        let receiver = MessageReceiver::new(rx);

        // No message, should return None immediately
        assert!(receiver.try_receive().is_none());

        // Send message
        let msg = WebSocketMessage {
            event: "data".to_string(),
            data: None,
            channel: None,
            symbol: None,
            id: None,
            raw: String::new(),
        };
        tx.push(msg);

        // Should receive immediately
        let received = receiver.try_receive();
        assert!(received.is_some());
        assert_eq!(received.unwrap().event, "data");
    }

    #[test]
    fn test_channel_closed_returns_error() {
        let (tx, rx) = queue(None, DropCounter::new("test_messages_dropped", "localhost", "test"));
        let receiver = MessageReceiver::new(rx);

        // Close channel by dropping sender
        drop(tx);

        // Should return error
        let result = receiver.receive();
        assert!(result.is_err());
        match result {
            Err(MarketDataError::ConnectionError { msg }) => {
                assert!(msg.contains("closed"));
            }
            _ => panic!("Expected ConnectionError"),
        }
    }

    #[test]
    fn test_channel_closed_timeout_returns_error() {
        let (tx, rx) = queue(None, DropCounter::new("test_messages_dropped", "localhost", "test"));
        let receiver = MessageReceiver::new(rx);

        // Close channel
        drop(tx);

        // Should return error, not timeout
        let result = receiver.receive_timeout(Duration::from_secs(1));
        assert!(result.is_err());
    }

    #[test]
    fn test_try_receive_after_close() {
        let (tx, rx) = queue(None, DropCounter::new("test_messages_dropped", "localhost", "test"));
        let receiver = MessageReceiver::new(rx);

        // Send message then close
        let msg = WebSocketMessage {
            event: "data".to_string(),
            data: None,
            channel: None,
            symbol: None,
            id: None,
            raw: String::new(),
        };
        tx.push(msg);
        drop(tx);

        // Should still receive buffered message
        let received = receiver.try_receive();
        assert!(received.is_some());

        // Next try should return None (channel closed, no more messages)
        let received2 = receiver.try_receive();
        assert!(received2.is_none());
    }
}
