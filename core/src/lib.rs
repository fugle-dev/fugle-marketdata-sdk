#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!("../README.md")]
#![deny(missing_docs, rustdoc::broken_intra_doc_links, clippy::missing_errors_doc)]

pub mod errors;
pub mod models;
pub mod rest;
pub mod tls;
pub mod urls;
pub mod websocket;

// Internal: cfg-gated tracing macro re-exports. No-op when the `tracing`
// feature is disabled.
mod tracing_compat;

// Re-export error types
pub use errors::MarketDataError;

// Re-export TLS config
pub use tls::TlsConfig;

// Re-export REST client types
pub use rest::{Auth, RestClient, RetryPolicy};

// Re-export WebSocket types
pub use websocket::{
    ConnectionConfig, ConnectionEvent, ConnectionState, DisconnectIntent, HealthCheckConfig,
    MessageReceiver, ReconnectionConfig, WebSocketClient, WebSocketFactory,
};

// Re-export WebSocket config constants for binding layers (CON-01).
// 3.0 collapsed `interval × max_missed_pongs` into a single timeout
// window — bindings that previously exposed both fields now expose
// just `heartbeat_timeout_ms`.
pub use websocket::health_check::{
    DEFAULT_HEALTH_CHECK_ENABLED, DEFAULT_HEARTBEAT_TIMEOUT_MS, MIN_HEARTBEAT_TIMEOUT_MS,
};
pub use websocket::reconnection::{
    DEFAULT_INITIAL_DELAY_MS, DEFAULT_MAX_ATTEMPTS, DEFAULT_MAX_DELAY_MS, MIN_INITIAL_DELAY_MS,
};

// Re-export model types for convenience
pub use models::{
    // Common types
    PriceLevel, ResponseMeta, TotalStats, TradeInfo, TradingHalt,
    // REST response types
    HistoricalCandle, HistoricalCandlesResponse, IntradayCandle, IntradayCandlesResponse,
    Quote, Ticker, Trade, TradesResponse, VolumeAtPrice, VolumesResponse,
    // WebSocket types
    AuthRequest, Channel, SymbolSpec, UnsubscribeRequest, WebSocketMessage, WebSocketRequest,
};

// Re-export streaming message types (Phase 4)
pub use models::streaming::{
    AggregatesData, BooksData, CandleData, CandleHistoryItem, CandlesSnapshot, DataPayload,
    ErrorData, IndicesData, SnapshotPayload, StreamMessage, StreamTrade, SubscribedData,
    TradesData,
};

// Re-export channel subscription and parsing utilities (Phase 4)
pub use websocket::channels::{parse_channel_data, parse_stream_message, ChannelData};
pub use websocket::StockSubscription;

// Re-export FutOpt types (Phase 5)
pub use models::futopt::{
    ContractType, FutOptChannel, FutOptLastTrade, FutOptPriceLevel, FutOptQuote, FutOptSession,
    FutOptTicker, FutOptTotalStats, FutOptType, OptionRight, Product, ProductsResponse,
};
pub use websocket::channels::FutOptSubscription;

// Re-export the async module + runtime (gated behind `tokio-comp`).
#[cfg(feature = "tokio-comp")]
#[cfg_attr(docsrs, doc(cfg(feature = "tokio-comp")))]
pub use websocket::aio;
#[cfg(feature = "tokio-comp")]
#[cfg_attr(docsrs, doc(cfg(feature = "tokio-comp")))]
pub use websocket::aio::AsyncRuntime;

// Future modules (to be added in later phases):
// pub mod rest;
// pub mod ws;
