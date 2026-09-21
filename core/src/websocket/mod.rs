//! WebSocket client for real-time market data streaming
//!
//! This module provides:
//! - WebSocket connection lifecycle management
//! - Configuration for connection parameters
//! - State machine for connection states
//! - Event notifications for connection events
//! - Channel-specific subscription and parsing

pub mod channels;
pub mod config;
pub(crate) mod connect_gate;
pub mod connection_event;
pub mod factory;
pub mod health_check;
pub(crate) mod liveness;
pub mod stream;
pub(crate) mod stream_queue;
pub(crate) mod protocol;
pub mod reconnection;
pub mod report_throttle;
pub mod subscription;
pub mod sync;
pub mod version;

#[cfg(feature = "tokio-comp")]
#[cfg_attr(docsrs, doc(cfg(feature = "tokio-comp")))]
pub mod aio;

// Default `WebSocketClient` is the sync implementation. Async users opt in via
// `marketdata_core::aio::WebSocketClient` with `--features tokio-comp`.
pub use channels::StockSubscription;
pub use config::{
    auth_timeout_from_millis, ConnectionConfig, ConnectionConfigBuilder, MessageOverflow,
    DEFAULT_AUTH_TIMEOUT, DEFAULT_EVENT_BUFFER, DEFAULT_MESSAGE_BUFFER,
};
pub use factory::WebSocketFactory;
pub use connection_event::{ConnectionEvent, ConnectionState, ConnectionStateHandle, DisconnectIntent};
pub use health_check::HealthCheckConfig;
pub use stream::{ConnectionStream, MessagesDroppedHandle, StreamItem, StreamReceiver};
pub use reconnection::{ReconnectionConfig, ReconnectionManager};
pub use stream_queue::ReconnectConflictHandle;
pub use report_throttle::{ReportThrottle, REPORT_INTERVAL};
pub use subscription::SubscriptionManager;
pub use sync::WebSocketClient;
pub use version::{FutOptVersion, StockVersion};
