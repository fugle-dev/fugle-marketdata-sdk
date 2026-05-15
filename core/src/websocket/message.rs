//! FFI-safe message receiver for WebSocket messages.
//!
//! Provides blocking and timeout-based message reception suitable for FFI bindings.
//! Uses `std::sync::mpsc` (not tokio channels) for compatibility with non-async FFI consumers.
//! Runtime-free: shared by the sync `WebSocketClient` and the async
//! `aio::WebSocketClient`.

use crate::models::WebSocketMessage;
use crate::MarketDataError;
use std::sync::mpsc;
use std::sync::Mutex;
use std::time::Duration;

/// FFI-safe message receiver with blocking API.
///
/// Thread-safe: uses a `Mutex` internally so callers can share it across
/// threads. Background threads can receive messages while the main thread
/// handles other operations.
pub struct MessageReceiver {
    rx: Mutex<mpsc::Receiver<WebSocketMessage>>,
}

impl MessageReceiver {
    /// Create a new message receiver
    pub fn new(rx: mpsc::Receiver<WebSocketMessage>) -> Self {
        Self { rx: Mutex::new(rx) }
    }

    /// Receive a message (blocking)
    ///
    /// Blocks until a message is received or channel is closed.
    ///
    /// # Errors
    ///
    /// Returns `ConnectionError` if channel is closed
    pub fn receive(&self) -> Result<WebSocketMessage, MarketDataError> {
        let rx = self.rx.lock().map_err(|_| MarketDataError::ConnectionError {
            msg: "Message receiver lock poisoned".to_string(),
        })?;
        rx.recv().map_err(|_| MarketDataError::ConnectionError {
            msg: "Message channel closed".to_string(),
        })
    }

    /// Receive a message with timeout
    ///
    /// Returns:
    /// - `Ok(Some(msg))` if message received within timeout
    /// - `Ok(None)` if timeout elapsed with no message
    /// - `Err` if channel closed
    pub fn receive_timeout(
        &self,
        timeout: Duration,
    ) -> Result<Option<WebSocketMessage>, MarketDataError> {
        let rx = self.rx.lock().map_err(|_| MarketDataError::ConnectionError {
            msg: "Message receiver lock poisoned".to_string(),
        })?;
        match rx.recv_timeout(timeout) {
            Ok(msg) => Ok(Some(msg)),
            Err(mpsc::RecvTimeoutError::Timeout) => Ok(None),
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                Err(MarketDataError::ConnectionError {
                    msg: "Message channel closed".to_string(),
                })
            }
        }
    }

    /// Try to receive a message without blocking
    ///
    /// Returns:
    /// - `Some(msg)` if message available
    /// - `None` if no message available or channel closed
    pub fn try_receive(&self) -> Option<WebSocketMessage> {
        self.rx.lock().ok()?.try_recv().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_receive_blocking() {
        let (tx, rx) = mpsc::channel();
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
            };
            tx.send(msg).unwrap();
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
        let (_tx, rx) = mpsc::channel();
        let receiver = MessageReceiver::new(rx);

        // No message sent, should timeout
        let result = receiver.receive_timeout(Duration::from_millis(50));
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_receive_timeout_returns_message() {
        let (tx, rx) = mpsc::channel();
        let receiver = MessageReceiver::new(rx);

        // Send message immediately
        let msg = WebSocketMessage {
            event: "data".to_string(),
            data: None,
            channel: Some("trades".to_string()),
            symbol: Some("2330".to_string()),
            id: None,
        };
        tx.send(msg).unwrap();

        // Should receive before timeout
        let result = receiver.receive_timeout(Duration::from_secs(1));
        assert!(result.is_ok());
        let received = result.unwrap();
        assert!(received.is_some());
        assert_eq!(received.unwrap().event, "data");
    }

    #[test]
    fn test_try_receive_non_blocking() {
        let (tx, rx) = mpsc::channel();
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
        };
        tx.send(msg).unwrap();

        // Should receive immediately
        let received = receiver.try_receive();
        assert!(received.is_some());
        assert_eq!(received.unwrap().event, "data");
    }

    #[test]
    fn test_channel_closed_returns_error() {
        let (tx, rx) = mpsc::channel();
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
        let (tx, rx) = mpsc::channel();
        let receiver = MessageReceiver::new(rx);

        // Close channel
        drop(tx);

        // Should return error, not timeout
        let result = receiver.receive_timeout(Duration::from_secs(1));
        assert!(result.is_err());
    }

    #[test]
    fn test_try_receive_after_close() {
        let (tx, rx) = mpsc::channel();
        let receiver = MessageReceiver::new(rx);

        // Send message then close
        let msg = WebSocketMessage {
            event: "data".to_string(),
            data: None,
            channel: None,
            symbol: None,
            id: None,
        };
        tx.send(msg).unwrap();
        drop(tx);

        // Should still receive buffered message
        let received = receiver.try_receive();
        assert!(received.is_some());

        // Next try should return None (channel closed, no more messages)
        let received2 = receiver.try_receive();
        assert!(received2.is_none());
    }
}
