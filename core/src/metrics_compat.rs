//! Optional `metrics` crate integration for drop counters.
//!
//! Provides [`DropCounter`] — a clone-cheap wrapper around an
//! `Arc<AtomicU64>` that, when the `metrics` feature is enabled, also
//! bumps a per-client `metrics::Counter` registered with the supplied
//! `endpoint` / `client_id` labels. Without the feature, `DropCounter`
//! compiles to the same single-Arc allocation it always was, with no
//! extra `metrics`-crate code.
//!
//! The polling getters on `WebSocketClient` (`messages_dropped_total`,
//! `events_dropped_total`) read the underlying atomic directly via
//! [`DropCounter::load`] so consumers without `metrics-exporter-prometheus`
//! see no behaviour change.
//!
//! # Counter names
//!
//! Two counters are registered per `WebSocketClient::new` (when the
//! `metrics` feature is enabled):
//!
//! - [`COUNTER_MESSAGES_DROPPED`] — channel back-pressure on `messages()`.
//! - [`COUNTER_EVENTS_DROPPED`] — broadcast back-pressure on
//!   `connection_events()`.
//!
//! Both carry `endpoint` (URL host) and `client_id` labels, where
//! `client_id` defaults to the empty string when unset on
//! [`crate::ConnectionConfig`].

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Counter name for messages dropped by inbound channel back-pressure.
pub(crate) const COUNTER_MESSAGES_DROPPED: &str =
    "fugle_marketdata_ws_messages_dropped_total";

/// Counter name for lifecycle events dropped by event channel back-pressure.
pub(crate) const COUNTER_EVENTS_DROPPED: &str =
    "fugle_marketdata_ws_events_dropped_total";

/// Clone-cheap wrapper around `Arc<AtomicU64>` plus an optional
/// `metrics::Counter`. Cloning shares the same underlying atomic; the
/// metric handle is `Clone` and shares the same recorder slot too.
#[derive(Clone)]
pub(crate) struct DropCounter {
    inner: Arc<DropCounterInner>,
}

struct DropCounterInner {
    atomic: AtomicU64,
    #[cfg(feature = "metrics")]
    metric: ::metrics::Counter,
}

impl DropCounter {
    /// Construct a counter bound to the given `(metric_name, endpoint,
    /// client_id)` triple. When the `metrics` feature is enabled, registers
    /// the counter with the active recorder and clones a cheap handle
    /// stored alongside the atomic. Without the feature, the labels are
    /// ignored (no allocation) and only the atomic is allocated.
    pub(crate) fn new(metric_name: &'static str, endpoint: &str, client_id: &str) -> Self {
        // Without the feature, the args are unused. Suppress the warning
        // without `#[allow(unused_variables)]` so the typo case is caught.
        #[cfg(not(feature = "metrics"))]
        let _ = (metric_name, endpoint, client_id);

        let inner = DropCounterInner {
            atomic: AtomicU64::new(0),
            #[cfg(feature = "metrics")]
            metric: ::metrics::counter!(
                metric_name,
                "endpoint" => endpoint.to_owned(),
                "client_id" => client_id.to_owned()
            ),
        };
        Self {
            inner: Arc::new(inner),
        }
    }

    /// Increment the counter. Atomic bump is always performed; the
    /// `metrics::Counter` increment is only compiled when the `metrics`
    /// feature is enabled.
    #[inline]
    pub(crate) fn bump(&self) {
        self.inner.atomic.fetch_add(1, Ordering::Relaxed);
        #[cfg(feature = "metrics")]
        self.inner.metric.increment(1);
    }

    /// Read the underlying atomic value. Used by the polling getters
    /// (`messages_dropped_total`, `events_dropped_total`) so consumers
    /// without the `metrics` feature see the authoritative count.
    #[inline]
    pub(crate) fn load(&self) -> u64 {
        self.inner.atomic.load(Ordering::Relaxed)
    }
}

/// Register human-readable descriptions for the SDK's drop counters with
/// the active `metrics` recorder. Called once per `WebSocketClient`
/// construction. No-op when the `metrics` feature is disabled.
#[inline]
pub(crate) fn describe_drop_counters() {
    #[cfg(feature = "metrics")]
    {
        ::metrics::describe_counter!(
            COUNTER_MESSAGES_DROPPED,
            ::metrics::Unit::Count,
            "Inbound messages dropped because the consumer's `messages()` channel was saturated."
        );
        ::metrics::describe_counter!(
            COUNTER_EVENTS_DROPPED,
            ::metrics::Unit::Count,
            "Lifecycle events dropped because the consumer's `connection_events()` channel was saturated."
        );
    }
}

/// Extract the URL host as the `endpoint` metric label. Falls back to an
/// empty string for unparseable URLs (test mocks occasionally use synthetic
/// strings; an empty label is acceptable per metrics convention).
pub(crate) fn endpoint_label(url: &str) -> String {
    ::url::Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(str::to_owned))
        .unwrap_or_default()
}

/// Build the `(messages_dropped, events_dropped)` counter pair for a fresh
/// `WebSocketClient`. Calls [`describe_drop_counters`] once and constructs
/// both counters with the same `(endpoint, client_id)` labels derived from
/// `config`. Both `aio::WebSocketClient::with_full_config` and
/// `sync::WebSocketClient::with_full_config` go through this so the two
/// clients cannot drift on counter labels.
pub(crate) fn build_drop_counters(
    config: &crate::websocket::ConnectionConfig,
) -> (DropCounter, DropCounter) {
    describe_drop_counters();
    let endpoint = endpoint_label(&config.url);
    let client_id_label = config.client_id.clone().unwrap_or_default();
    let messages = DropCounter::new(COUNTER_MESSAGES_DROPPED, &endpoint, &client_id_label);
    let events = DropCounter::new(COUNTER_EVENTS_DROPPED, &endpoint, &client_id_label);
    (messages, events)
}
