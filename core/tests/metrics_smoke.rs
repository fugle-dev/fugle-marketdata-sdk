//! Smoke tests for the optional `metrics` feature integration.
//!
//! Gated behind `feature = "metrics"`. Verifies that:
//!
//! 1. `WebSocketClient::new` registers both drop-counter descriptions on
//!    the active `metrics` recorder (`describe_drop_counters`).
//! 2. The atomic polling getters remain authoritative when the feature is
//!    enabled.
//! 3. The `metrics` counter increments in lock-step with the atomic when
//!    a drop occurs.
//!
//! Tests use `metrics-util::debugging::DebuggingRecorder` to install a
//! recorder local to the current thread so multiple tests don't fight
//! over a global recorder.

#![cfg(feature = "metrics")]

use marketdata_core::websocket::ConnectionConfig;
use marketdata_core::AuthRequest;
use metrics_util::debugging::{DebugValue, DebuggingRecorder};

const COUNTER_MESSAGES: &str = "fugle_marketdata_ws_messages_dropped_total";
const COUNTER_EVENTS: &str = "fugle_marketdata_ws_events_dropped_total";

#[cfg(feature = "tokio-comp")]
#[test]
fn aio_client_describes_both_counters_on_construction() {
    // Construction is synchronous; no async runtime needed. Plain test +
    // `with_local_recorder` keeps the recorder thread-local without any
    // runtime nesting.
    let recorder = DebuggingRecorder::new();
    let snapshotter = recorder.snapshotter();
    metrics::with_local_recorder(&recorder, || {
        let auth = AuthRequest::with_api_key("smoke");
        let config = ConnectionConfig::builder("wss://example.invalid/marketdata/v1.0/stock/streaming", auth)
            .client_id("metrics-smoke-aio")
            .build();
        let _client = marketdata_core::aio::WebSocketClient::new(config);
    });

    let snapshot = snapshotter.snapshot().into_vec();
    let descriptions: Vec<&str> = snapshot
        .iter()
        .filter_map(|(key, _unit, description, _value)| {
            description.as_ref().map(|_| key.key().name())
        })
        .collect();

    assert!(
        descriptions.iter().any(|n| *n == COUNTER_MESSAGES),
        "missing description for {COUNTER_MESSAGES}; got {descriptions:?}"
    );
    assert!(
        descriptions.iter().any(|n| *n == COUNTER_EVENTS),
        "missing description for {COUNTER_EVENTS}; got {descriptions:?}"
    );
}

#[cfg(all(feature = "tokio-comp", feature = "test-utils"))]
#[test]
fn metrics_counter_increments_with_atomic_on_drop() {
    // Sync test (not `#[tokio::test]`) so we can install the local
    // recorder on this thread and drive the runtime from the same scope.
    // `with_local_recorder` ties the recorder to the current thread for
    // the duration of the closure; tokio's current-thread runtime stays
    // on the same thread.
    use marketdata_core::models::streaming::StreamMessage;
    use marketdata_core::testing::aio_pair;

    let recorder = DebuggingRecorder::new();
    let snapshotter = recorder.snapshotter();
    let atomic_total = metrics::with_local_recorder(&recorder, || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("build current-thread runtime");

        rt.block_on(async {
            let (server, client) = aio_pair().await;
            client.connect().await.expect("connect");
            // Default message buffer is 4096; inject many more frames
            // than the buffer to guarantee at least one drop. The drain
            // sleep gives the dispatch task time to enqueue everything
            // before the consumer reads.
            for _ in 0..8192 {
                server
                    .inject_frame(StreamMessage::Pong { state: Some("saturate".into()) })
                    .await;
            }
            tokio::time::sleep(std::time::Duration::from_millis(150)).await;

            client.messages_dropped_total()
        })
    });

    let snapshot = snapshotter.snapshot().into_vec();
    let messages_metric_total: u64 = snapshot
        .iter()
        .find_map(|(key, _, _, value)| {
            if key.key().name() == COUNTER_MESSAGES {
                if let DebugValue::Counter(v) = value {
                    Some(*v)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .unwrap_or(0);

    assert_eq!(
        messages_metric_total, atomic_total,
        "metrics counter ({messages_metric_total}) and atomic ({atomic_total}) MUST agree"
    );
}
