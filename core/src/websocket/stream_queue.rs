//! The client's ordered stream of messages and connection events (#46, #68).
//!
//! Runtime-free: a `Mutex<VecDeque<StreamItem>>` read through a `Condvar` by
//! blocking consumers ([`StreamReceiver`](crate::websocket::StreamReceiver))
//! and through a stored [`Waker`] by async ones
//! ([`ConnectionStream`](crate::websocket::ConnectionStream)). Everything the
//! client reports goes through one [`StreamSender`], so the order items are
//! queued in is the order they are delivered in.
//!
//! Messages and events have separate capacities: messages are bounded by
//! `message_buffer` under [`MessageOverflow::DropNewest`], events by
//! `event_buffer`. A full message allowance never costs an event.
//!
//! Per connection, the lock also guards:
//! - the window in which messages are accepted: opened together with
//!   `Authenticated`, closed together with `Disconnected`, so no message of a
//!   connection is delivered before its `Authenticated` or after its
//!   `Disconnected`;
//! - the claim on reporting that connection's `Disconnected` (#41);
//! - the drop bookkeeping behind `MessagesDropped`.
//!
//! [`MessageOverflow::DropNewest`]: crate::websocket::MessageOverflow::DropNewest

use crate::metrics_compat::DropCounter;
use crate::models::WebSocketMessage;
use crate::websocket::report_throttle::ReportThrottle;
#[cfg(test)]
use crate::websocket::report_throttle::REPORT_INTERVAL;
use crate::websocket::stream::StreamItem;
use crate::websocket::{ConnectionConfig, ConnectionEvent, ConnectionState, DisconnectIntent};
use std::collections::VecDeque;
use std::sync::mpsc::{RecvTimeoutError, TryRecvError};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError, RwLock};
use std::task::{Context, Poll, Waker};
use std::time::{Duration, Instant};

struct State {
    items: VecDeque<StreamItem>,
    /// Messages currently in `items`.
    messages: usize,
    /// Events currently in `items`.
    events: usize,
    senders: usize,
    receiver_alive: bool,
    waker: Option<Waker>,
    /// Messages are accepted: the current connection has authenticated and
    /// has not been reported closed.
    open: bool,
    /// The current connection's `Disconnected` has been reported (#41).
    disconnect_claimed: bool,
    /// Message drops not covered by a `MessagesDropped` yet, and when the
    /// last one was queued on the current connection.
    drop_reports: ReportThrottle,
    /// Inside the lock: a bare `DropCounter` (it may hold a `metrics`
    /// handle) would make the public receivers `!RefUnwindSafe`.
    messages_dropped: DropCounter,
    events_dropped: DropCounter,
}

struct Shared {
    state: Mutex<State>,
    available: Condvar,
    /// `None` means unbounded.
    message_capacity: Option<usize>,
    event_capacity: usize,
}

impl Shared {
    /// Every critical section leaves the state consistent, so a poisoned
    /// lock still yields a usable value.
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Create the stream for a client built from `config`.
pub(crate) fn stream(
    config: &ConnectionConfig,
    messages_dropped: DropCounter,
    events_dropped: DropCounter,
) -> (StreamSender, QueueReceiver) {
    let shared = Arc::new(Shared {
        state: Mutex::new(State {
            items: VecDeque::new(),
            messages: 0,
            events: 0,
            senders: 1,
            receiver_alive: true,
            waker: None,
            open: false,
            disconnect_claimed: false,
            drop_reports: ReportThrottle::new(),
            messages_dropped,
            events_dropped,
        }),
        available: Condvar::new(),
        message_capacity: config.message_capacity(),
        event_capacity: config.event_buffer,
    });
    (
        StreamSender {
            shared: Arc::clone(&shared),
        },
        QueueReceiver { shared },
    )
}

/// What a push did besides queueing, reported after the lock is released.
#[derive(Default)]
struct Outcome {
    queued: bool,
    dropped_event: Option<ConnectionEvent>,
    report: Option<(u64, u64)>,
}

/// Producer end, shared by everything that reports on a client. Cloneable;
/// the stream reports closed once every clone is dropped and the remaining
/// items have been read.
pub(crate) struct StreamSender {
    shared: Arc<Shared>,
}

impl Clone for StreamSender {
    fn clone(&self) -> Self {
        self.shared.lock().senders += 1;
        Self {
            shared: Arc::clone(&self.shared),
        }
    }
}

impl Drop for StreamSender {
    fn drop(&mut self) {
        let mut state = self.shared.lock();
        state.senders -= 1;
        if state.senders > 0 {
            return;
        }
        let waker = state.waker.take();
        drop(state);
        self.shared.available.notify_all();
        if let Some(waker) = waker {
            waker.wake();
        }
    }
}

impl StreamSender {
    /// Queue `event`, or drop it (counted in `events_dropped`) if the event
    /// allowance is full.
    pub(crate) fn emit(&self, event: ConnectionEvent) {
        let mut state = self.shared.lock();
        let mut outcome = Outcome::default();
        self.push_event(&mut state, event, &mut outcome);
        self.finish(state, outcome);
    }

    /// Queue a message of the open connection. Outside a connection's window
    /// it is discarded uncounted; while the message allowance is full it is
    /// dropped and counted. Queues a `MessagesDropped` right after it when
    /// one is due: the first on this connection, or [`REPORT_INTERVAL`](crate::websocket::REPORT_INTERVAL)
    /// after the previous one. Checking after successful pushes too reports
    /// the tail of a drop burst within the interval.
    pub(crate) fn push_message(&self, message: WebSocketMessage) {
        let mut state = self.shared.lock();
        if !state.open {
            return;
        }
        let mut outcome = Outcome::default();
        self.push_message_locked(&mut state, message, &mut outcome);
        self.push_report(&mut state, false, &mut outcome);
        self.finish(state, outcome);
    }

    /// Start a connection attempt, before its auth handshake: the drop count
    /// restarts from zero, and the first drop report is not held back by a
    /// report on the previous connection. Until then the count of the
    /// previous connection stays readable. Messages of whatever connection
    /// came before are no longer accepted.
    pub(crate) fn start_connection(&self) {
        let mut state = self.shared.lock();
        state.open = false;
        state.messages_dropped.reset();
        state.drop_reports.reset();
    }

    /// Report the connection authenticated: queue `Authenticated`, then the
    /// frames the handshake read, and start accepting its messages.
    pub(crate) fn authenticated(&self, data: serde_json::Value, frames: Vec<WebSocketMessage>) {
        let mut state = self.shared.lock();
        let mut outcome = Outcome::default();
        state.disconnect_claimed = false;
        state.open = true;
        self.push_event(&mut state, ConnectionEvent::Authenticated { data }, &mut outcome);
        for frame in frames {
            self.push_message_locked(&mut state, frame, &mut outcome);
        }
        self.finish(state, outcome);
    }

    /// Report the credentials rejected: queue `Unauthenticated`, then the
    /// frames the handshake read. The connection never opens.
    pub(crate) fn unauthenticated(
        &self,
        message: String,
        data: serde_json::Value,
        frames: Vec<WebSocketMessage>,
    ) {
        let mut state = self.shared.lock();
        let mut outcome = Outcome::default();
        self.push_event(
            &mut state,
            ConnectionEvent::Unauthenticated { message, data },
            &mut outcome,
        );
        for frame in frames {
            self.push_message_locked(&mut state, frame, &mut outcome);
        }
        self.finish(state, outcome);
    }

    /// Queue the current connection's `Disconnected`, unless it has one
    /// already. Drops not reported yet are reported first, and the
    /// connection stops accepting messages, all under one lock.
    #[cfg(test)]
    pub(crate) fn emit_disconnected(
        &self,
        code: Option<u16>,
        reason: String,
        intent: DisconnectIntent,
        will_reconnect: bool,
    ) {
        self.claim_disconnected(code, reason, intent, will_reconnect, |_| {});
    }

    /// Queue the `Disconnected` of a connection lost without the caller
    /// asking, unless it has one already. Under the same claim, and before
    /// the event is queued, `connection` is set to what the event reports: a
    /// consumer handling the `Disconnected` reads a matching state (#86).
    /// That is [`ConnectionState::Disconnected`] if the client reconnects next
    /// (its reconnect loop moves on to `Reconnecting`), otherwise `Closed`
    /// with the event's code, reason and intent.
    ///
    /// Takes `connection`'s write lock inside the stream's lock: nothing may
    /// call into the stream while holding `connection`'s lock.
    pub(crate) fn connection_lost(
        &self,
        connection: &RwLock<ConnectionState>,
        code: Option<u16>,
        reason: String,
        intent: DisconnectIntent,
        will_reconnect: bool,
    ) {
        self.claim_disconnected(code, reason, intent, will_reconnect, |reason| {
            let next = if will_reconnect {
                ConnectionState::Disconnected
            } else {
                ConnectionState::Closed {
                    code,
                    reason: reason.to_string(),
                    intent,
                }
            };
            // Writers only assign, so a poisoned lock holds a whole value.
            *connection.write().unwrap_or_else(PoisonError::into_inner) = next;
        });
    }

    /// Close the client on the caller's request (`disconnect()`,
    /// `force_close()`): set `connection` to `Closed` with
    /// [`DisconnectIntent::Client`] and queue the matching `Disconnected`,
    /// unless the connection has reported its close already (#93). Then
    /// that report stands:
    /// - a `Closed` state it recorded is left as it is, so the state keeps
    ///   agreeing with the event;
    /// - any other state (the client was reconnecting) is set to `Closed`
    ///   without a further event.
    ///
    /// Same lock order as [`connection_lost`](Self::connection_lost).
    pub(crate) fn client_closed(
        &self,
        connection: &RwLock<ConnectionState>,
        code: u16,
        reason: String,
    ) {
        let state = self.shared.lock();
        {
            // Writers only assign, so a poisoned lock holds a whole value.
            let mut current = connection.write().unwrap_or_else(PoisonError::into_inner);
            let reported_closed =
                state.disconnect_claimed && matches!(*current, ConnectionState::Closed { .. });
            if !reported_closed {
                *current = ConnectionState::Closed {
                    code: Some(code),
                    reason: reason.clone(),
                    intent: DisconnectIntent::Client,
                };
            }
        }
        if !state.disconnect_claimed {
            self.queue_disconnected(state, Some(code), reason, DisconnectIntent::Client, false);
        }
    }

    /// Claim and queue the connection's `Disconnected`, running `record`
    /// with its reason under the claim first.
    fn claim_disconnected(
        &self,
        code: Option<u16>,
        reason: String,
        intent: DisconnectIntent,
        will_reconnect: bool,
        record: impl FnOnce(&str),
    ) {
        let state = self.shared.lock();
        if state.disconnect_claimed {
            return;
        }
        record(&reason);
        self.queue_disconnected(state, code, reason, intent, will_reconnect);
    }

    /// Claim the connection's `Disconnected` and queue it under `state`,
    /// whose claim the caller has checked is free.
    fn queue_disconnected(
        &self,
        mut state: MutexGuard<'_, State>,
        code: Option<u16>,
        reason: String,
        intent: DisconnectIntent,
        will_reconnect: bool,
    ) {
        state.disconnect_claimed = true;
        state.open = false;
        let mut outcome = Outcome::default();
        self.push_report(&mut state, true, &mut outcome);
        self.push_event(
            &mut state,
            ConnectionEvent::Disconnected {
                code,
                reason,
                intent,
                will_reconnect,
            },
            &mut outcome,
        );
        self.finish(state, outcome);
    }

    fn push_event(&self, state: &mut State, event: ConnectionEvent, outcome: &mut Outcome) {
        if !state.receiver_alive {
            return;
        }
        if state.events >= self.shared.event_capacity {
            state.events_dropped.bump();
            outcome.dropped_event = Some(event);
            return;
        }
        state.events += 1;
        state.items.push_back(StreamItem::Event(event));
        outcome.queued = true;
    }

    fn push_message_locked(
        &self,
        state: &mut State,
        message: WebSocketMessage,
        outcome: &mut Outcome,
    ) {
        if !state.receiver_alive {
            return;
        }
        let full = self
            .shared
            .message_capacity
            .is_some_and(|capacity| state.messages >= capacity);
        if full {
            state.drop_reports.count();
            state.messages_dropped.bump();
            return;
        }
        state.messages += 1;
        state.items.push_back(StreamItem::Message(message));
        outcome.queued = true;
    }

    /// Queue a `MessagesDropped` for the unreported drops if one is due
    /// (`force`: regardless of the throttle).
    fn push_report(&self, state: &mut State, force: bool, outcome: &mut Outcome) {
        let Some(dropped) = state.drop_reports.take_due(Instant::now(), force) else {
            return;
        };
        let total = state.messages_dropped.load();
        outcome.report = Some((dropped, total));
        self.push_event(state, ConnectionEvent::MessagesDropped { dropped, total }, outcome);
    }

    /// Release the lock, wake the consumer if anything was queued, and log.
    fn finish(&self, mut state: MutexGuard<'_, State>, outcome: Outcome) {
        let waker = if outcome.queued {
            state.waker.take()
        } else {
            None
        };
        drop(state);
        if outcome.queued {
            self.shared.available.notify_all();
            if let Some(waker) = waker {
                waker.wake();
            }
        }
        if let Some((_dropped, _total)) = outcome.report {
            crate::tracing_compat::warn!(
                target: "fugle_marketdata::ws",
                dropped = _dropped,
                dropped_total = _total,
                "message queue saturated; dropping frames (drop-newest)"
            );
        }
        if let Some(_event) = outcome.dropped_event {
            crate::tracing_compat::warn!(
                target: "fugle_marketdata::ws",
                dropped = ?_event,
                "event allowance saturated; consumer is likely stuck"
            );
        }
    }
}

/// Consumer end. Every method takes `&self`, so it can be shared between
/// threads; each item is delivered to exactly one caller.
pub(crate) struct QueueReceiver {
    shared: Arc<Shared>,
}

impl Drop for QueueReceiver {
    fn drop(&mut self) {
        let mut state = self.shared.lock();
        state.receiver_alive = false;
        state.waker = None;
        state.messages = 0;
        state.events = 0;
        let items = std::mem::take(&mut state.items);
        drop(state);
        drop(items);
    }
}

impl QueueReceiver {
    fn pop(state: &mut State) -> Option<StreamItem> {
        let item = state.items.pop_front()?;
        match item {
            StreamItem::Message(_) => state.messages -= 1,
            StreamItem::Event(_) => state.events -= 1,
        }
        Some(item)
    }

    /// Next item without waiting.
    pub(crate) fn try_recv(&self) -> Result<StreamItem, TryRecvError> {
        let mut state = self.shared.lock();
        match Self::pop(&mut state) {
            Some(item) => Ok(item),
            None if state.senders == 0 => Err(TryRecvError::Disconnected),
            None => Err(TryRecvError::Empty),
        }
    }

    /// Next item, waiting for one. `None` once the stream is closed and empty.
    pub(crate) fn recv(&self) -> Option<StreamItem> {
        let mut state = self.shared.lock();
        loop {
            if let Some(item) = Self::pop(&mut state) {
                return Some(item);
            }
            if state.senders == 0 {
                return None;
            }
            state = self
                .shared
                .available
                .wait(state)
                .unwrap_or_else(PoisonError::into_inner);
        }
    }

    /// Next item, waiting at most `timeout` for one.
    pub(crate) fn recv_timeout(&self, timeout: Duration) -> Result<StreamItem, RecvTimeoutError> {
        let deadline = Instant::now() + timeout;
        let mut state = self.shared.lock();
        loop {
            if let Some(item) = Self::pop(&mut state) {
                return Ok(item);
            }
            if state.senders == 0 {
                return Err(RecvTimeoutError::Disconnected);
            }
            let now = Instant::now();
            if now >= deadline {
                return Err(RecvTimeoutError::Timeout);
            }
            state = self
                .shared
                .available
                .wait_timeout(state, deadline - now)
                .unwrap_or_else(PoisonError::into_inner)
                .0;
        }
    }

    /// Poll for the next item; `Ready(None)` once closed and empty. Only the
    /// most recent task to poll is woken.
    pub(crate) fn poll_recv(&self, cx: &mut Context<'_>) -> Poll<Option<StreamItem>> {
        let mut state = self.shared.lock();
        if let Some(item) = Self::pop(&mut state) {
            return Poll::Ready(Some(item));
        }
        if state.senders == 0 {
            return Poll::Ready(None);
        }
        match &state.waker {
            Some(waker) if waker.will_wake(cx.waker()) => {}
            _ => state.waker = Some(cx.waker().clone()),
        }
        Poll::Pending
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AuthRequest;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::task::Wake;
    use std::thread;

    fn counter(name: &'static str) -> DropCounter {
        DropCounter::new(name, "localhost", "test")
    }

    struct Fixture {
        tx: StreamSender,
        rx: QueueReceiver,
        messages_dropped: DropCounter,
        events_dropped: DropCounter,
    }

    fn fixture(message_buffer: usize, event_buffer: usize) -> Fixture {
        let config = ConnectionConfig::builder("ws://localhost", AuthRequest::with_api_key("k"))
            .message_buffer(message_buffer)
            .event_buffer(event_buffer)
            .build();
        let messages_dropped = counter("test_messages_dropped");
        let events_dropped = counter("test_events_dropped");
        let (tx, rx) = stream(&config, messages_dropped.clone(), events_dropped.clone());
        Fixture {
            tx,
            rx,
            messages_dropped,
            events_dropped,
        }
    }

    fn message(id: u32) -> WebSocketMessage {
        WebSocketMessage {
            event: "data".into(),
            data: None,
            channel: None,
            symbol: None,
            id: Some(id.to_string()),
            raw: String::new(),
        }
    }

    fn open(tx: &StreamSender) {
        tx.start_connection();
        tx.authenticated(serde_json::Value::Null, Vec::new());
    }

    /// Everything queued, as `m<id>` for messages and the event's `Debug`.
    fn drain(rx: &QueueReceiver) -> Vec<String> {
        std::iter::from_fn(|| rx.try_recv().ok())
            .map(|item| match item {
                StreamItem::Message(m) => format!("m{}", m.id.unwrap_or_default()),
                StreamItem::Event(e) => format!("{e:?}"),
            })
            .collect()
    }

    fn closed(code: Option<u16>) -> ConnectionEvent {
        ConnectionEvent::Disconnected {
            code,
            reason: "bye".into(),
            intent: DisconnectIntent::Server,
            will_reconnect: false,
        }
    }

    struct CountingWaker(AtomicUsize);

    impl Wake for CountingWaker {
        fn wake(self: Arc<Self>) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn items_come_out_in_the_order_they_were_queued() {
        let f = fixture(8, 8);
        f.tx.emit(ConnectionEvent::Connecting);
        f.tx.start_connection();
        f.tx.authenticated(serde_json::Value::Null, vec![message(0)]);
        f.tx.push_message(message(1));
        f.tx.emit_disconnected(Some(1000), "bye".into(), DisconnectIntent::Server, false);
        assert_eq!(
            drain(&f.rx),
            vec![
                "Connecting".to_string(),
                "Authenticated { data: Null }".into(),
                "m0".into(),
                "m1".into(),
                format!("{:?}", closed(Some(1000))),
            ]
        );
    }

    #[test]
    fn connection_lost_records_the_state_before_queuing_disconnected() {
        let f = fixture(8, 8);
        let connection = RwLock::new(ConnectionState::Connected);
        open(&f.tx);
        f.tx.connection_lost(&connection, Some(4001), "bye".into(), DisconnectIntent::Server, false);
        assert_eq!(
            *connection.read().unwrap(),
            ConnectionState::Closed {
                code: Some(4001),
                reason: "bye".into(),
                intent: DisconnectIntent::Server,
            }
        );
        assert_eq!(drain(&f.rx).last(), Some(&format!("{:?}", closed(Some(4001)))));

        // Reconnecting next: not connected, not closed either.
        let connection = RwLock::new(ConnectionState::Connected);
        open(&f.tx);
        f.tx.connection_lost(&connection, None, "lost".into(), DisconnectIntent::Network, true);
        assert_eq!(*connection.read().unwrap(), ConnectionState::Disconnected);
    }

    #[test]
    fn connection_lost_leaves_the_state_alone_once_disconnected_is_claimed() {
        let f = fixture(8, 8);
        open(&f.tx);
        let client_close = ConnectionState::Closed {
            code: Some(1000),
            reason: "Normal closure".into(),
            intent: DisconnectIntent::Client,
        };
        let connection = RwLock::new(client_close.clone());
        f.tx.emit_disconnected(Some(1000), "Normal closure".into(), DisconnectIntent::Client, false);
        f.tx.connection_lost(&connection, None, "lost".into(), DisconnectIntent::Network, false);
        assert_eq!(*connection.read().unwrap(), client_close);
    }

    fn client_close(code: u16, reason: &str) -> ConnectionState {
        ConnectionState::Closed {
            code: Some(code),
            reason: reason.into(),
            intent: DisconnectIntent::Client,
        }
    }

    #[test]
    fn client_closed_records_the_state_before_queuing_disconnected() {
        let f = fixture(8, 8);
        let connection = Arc::new(RwLock::new(ConnectionState::Connected));
        open(&f.tx);
        drain(&f.rx);
        let reader = {
            let connection = Arc::clone(&connection);
            thread::spawn(move || {
                let item = f.rx.recv().expect("an item");
                (format!("{item:?}"), connection.read().unwrap().clone())
            })
        };
        f.tx.client_closed(&connection, 1000, "Normal closure".into());
        let (item, state) = reader.join().unwrap();
        assert!(item.contains("Disconnected") && item.contains("Client"), "{item}");
        assert_eq!(state, client_close(1000, "Normal closure"));
    }

    #[test]
    fn client_closed_keeps_the_closed_state_of_a_reported_close() {
        let f = fixture(8, 8);
        open(&f.tx);
        let connection = RwLock::new(ConnectionState::Connected);
        f.tx.connection_lost(&connection, Some(4001), "bye".into(), DisconnectIntent::Server, false);
        let server_close = connection.read().unwrap().clone();
        drain(&f.rx);

        f.tx.client_closed(&connection, 1000, "Normal closure".into());
        f.tx.client_closed(&connection, 1006, "Force closed".into());
        assert_eq!(*connection.read().unwrap(), server_close);
        assert!(drain(&f.rx).is_empty());
    }

    #[test]
    fn client_closed_while_reconnecting_closes_without_a_second_disconnected() {
        let f = fixture(8, 8);
        open(&f.tx);
        let connection = RwLock::new(ConnectionState::Connected);
        f.tx.connection_lost(&connection, None, "lost".into(), DisconnectIntent::Network, true);
        *connection.write().unwrap() = ConnectionState::Reconnecting { attempt: 1 };
        drain(&f.rx);

        f.tx.client_closed(&connection, 1000, "Normal closure".into());
        assert_eq!(*connection.read().unwrap(), client_close(1000, "Normal closure"));
        assert!(drain(&f.rx).is_empty());
    }

    #[test]
    fn a_consumer_woken_by_disconnected_reads_the_recorded_state() {
        let f = fixture(8, 8);
        let connection = Arc::new(RwLock::new(ConnectionState::Connected));
        open(&f.tx);
        drain(&f.rx);
        let reader = {
            let connection = Arc::clone(&connection);
            thread::spawn(move || {
                let item = f.rx.recv().expect("an item");
                assert!(matches!(item, StreamItem::Event(ConnectionEvent::Disconnected { .. })));
                connection.read().unwrap().clone()
            })
        };
        f.tx.connection_lost(&connection, Some(1000), "bye".into(), DisconnectIntent::Server, false);
        assert!(matches!(reader.join().unwrap(), ConnectionState::Closed { .. }));
    }

    #[test]
    fn messages_outside_a_connection_are_discarded_uncounted() {
        let f = fixture(8, 8);
        f.tx.push_message(message(0));
        open(&f.tx);
        f.tx.push_message(message(1));
        f.tx.emit_disconnected(None, "bye".into(), DisconnectIntent::Server, false);
        f.tx.push_message(message(2));
        f.tx.start_connection();
        f.tx.push_message(message(3));
        let items = drain(&f.rx);
        assert!(items.contains(&"m1".to_string()), "{items:?}");
        for late in ["m0", "m2", "m3"] {
            assert!(!items.contains(&late.to_string()), "{items:?}");
        }
        assert_eq!(f.messages_dropped.load(), 0);
    }

    #[test]
    fn full_message_allowance_drops_newest_but_never_an_event() {
        let f = fixture(2, 8);
        open(&f.tx);
        drain(&f.rx);
        for id in 0..5 {
            f.tx.push_message(message(id));
        }
        f.tx.emit(ConnectionEvent::Reconnecting { attempt: 1 });
        f.tx.emit_disconnected(None, "bye".into(), DisconnectIntent::Network, false);
        assert_eq!(f.messages_dropped.load(), 3);
        assert_eq!(f.events_dropped.load(), 0);
        let items = drain(&f.rx);
        assert_eq!(
            items,
            vec![
                "m0".to_string(),
                "m1".into(),
                // The first drop is reported at once, right where it happened.
                format!("{:?}", ConnectionEvent::MessagesDropped { dropped: 1, total: 1 }),
                "Reconnecting { attempt: 1 }".into(),
                format!("{:?}", ConnectionEvent::MessagesDropped { dropped: 2, total: 3 }),
                "Disconnected { code: None, reason: \"bye\", intent: Network, will_reconnect: false }"
                    .into(),
            ]
        );
    }

    #[test]
    fn full_event_allowance_drops_events_only() {
        let f = fixture(8, 2);
        f.tx.emit(ConnectionEvent::Connecting);
        f.tx.emit(ConnectionEvent::Connected);
        f.tx.emit(ConnectionEvent::Reconnecting { attempt: 1 });
        assert_eq!(f.events_dropped.load(), 1);
        // Reading makes room again.
        assert!(f.rx.try_recv().is_ok());
        f.tx.emit(ConnectionEvent::Reconnecting { attempt: 2 });
        assert_eq!(f.events_dropped.load(), 1);
        assert_eq!(f.messages_dropped.load(), 0);
    }

    #[test]
    fn unbounded_message_allowance_never_drops() {
        let config = ConnectionConfig::builder("ws://localhost", AuthRequest::with_api_key("k"))
            .message_buffer(1)
            .message_overflow(crate::websocket::MessageOverflow::Unbounded)
            .build();
        let dropped = counter("test_messages_dropped");
        let (tx, rx) = stream(&config, dropped.clone(), counter("test_events_dropped"));
        open(&tx);
        for id in 0..1000 {
            tx.push_message(message(id));
        }
        assert_eq!(dropped.load(), 0);
        assert_eq!(drain(&rx).len(), 1001);
    }

    #[test]
    fn disconnected_is_reported_once_per_connection() {
        let f = fixture(8, 8);
        open(&f.tx);
        f.tx.emit_disconnected(None, "server".into(), DisconnectIntent::Server, true);
        f.tx.emit_disconnected(Some(1000), "client".into(), DisconnectIntent::Client, false);
        assert_eq!(drain(&f.rx).iter().filter(|e| e.starts_with("Disconnected")).count(), 1);
        // A new connection may report its own close.
        f.tx.start_connection();
        f.tx.authenticated(serde_json::Value::Null, Vec::new());
        f.tx.emit_disconnected(Some(1000), "client".into(), DisconnectIntent::Client, false);
        assert_eq!(drain(&f.rx).iter().filter(|e| e.starts_with("Disconnected")).count(), 1);
    }

    #[test]
    fn concurrent_closers_report_exactly_one_disconnected() {
        for _ in 0..200 {
            let f = fixture(8, 8);
            open(&f.tx);
            let closers: Vec<_> = (0..4)
                .map(|_| {
                    let tx = f.tx.clone();
                    thread::spawn(move || {
                        tx.emit_disconnected(None, "bye".into(), DisconnectIntent::Client, false)
                    })
                })
                .collect();
            for closer in closers {
                closer.join().expect("closer");
            }
            let items = drain(&f.rx);
            assert_eq!(items.iter().filter(|e| e.starts_with("Disconnected")).count(), 1);
        }
    }

    #[test]
    fn rejected_connection_queues_its_frames_after_unauthenticated_and_stays_closed() {
        let f = fixture(8, 8);
        f.tx.start_connection();
        f.tx.unauthenticated("no".into(), serde_json::Value::Null, vec![message(0)]);
        f.tx.push_message(message(1));
        assert_eq!(
            drain(&f.rx),
            vec![
                "Unauthenticated { message: \"no\", data: Null }".to_string(),
                "m0".into(),
            ]
        );
    }

    #[test]
    fn report_comes_due_after_the_interval_even_on_a_successful_push() {
        let f = fixture(1, 8);
        open(&f.tx);
        drain(&f.rx);
        f.tx.push_message(message(0));
        f.tx.push_message(message(1)); // dropped, reported at once
        f.tx.push_message(message(2)); // dropped, throttled
        drain(&f.rx);
        thread::sleep(REPORT_INTERVAL);
        f.tx.push_message(message(3));
        assert_eq!(
            drain(&f.rx),
            vec![
                "m3".to_string(),
                format!("{:?}", ConnectionEvent::MessagesDropped { dropped: 1, total: 2 }),
            ]
        );
    }

    #[test]
    fn a_new_connection_counts_from_zero_and_reports_its_first_drop_at_once() {
        let f = fixture(1, 8);
        open(&f.tx);
        drain(&f.rx);
        f.tx.push_message(message(0));
        f.tx.push_message(message(1));
        f.tx.push_message(message(2));
        assert_eq!(f.messages_dropped.load(), 2);
        f.tx.emit_disconnected(None, "bye".into(), DisconnectIntent::Network, true);
        assert_eq!(f.messages_dropped.load(), 2, "readable after the connection ended");
        drain(&f.rx);

        f.tx.start_connection();
        assert_eq!(f.messages_dropped.load(), 0);
        f.tx.authenticated(serde_json::Value::Null, vec![message(3)]);
        drain(&f.rx);
        f.tx.push_message(message(4));
        f.tx.push_message(message(5));
        assert_eq!(
            drain(&f.rx),
            vec![
                "m4".to_string(),
                format!("{:?}", ConnectionEvent::MessagesDropped { dropped: 1, total: 1 }),
            ]
        );
    }

    #[test]
    fn frames_of_a_full_handshake_are_dropped_and_reported_on_the_next_push() {
        let f = fixture(1, 8);
        f.tx.start_connection();
        f.tx.authenticated(serde_json::Value::Null, vec![message(0), message(1)]);
        assert_eq!(f.messages_dropped.load(), 1);
        drain(&f.rx);
        f.tx.push_message(message(2));
        assert_eq!(
            drain(&f.rx),
            vec![
                "m2".to_string(),
                format!("{:?}", ConnectionEvent::MessagesDropped { dropped: 1, total: 1 }),
            ]
        );
    }

    #[test]
    fn closes_only_when_every_sender_is_gone_and_items_are_read() {
        let f = fixture(8, 8);
        let tx2 = f.tx.clone();
        f.tx.emit(ConnectionEvent::Connecting);
        drop(f.tx);
        assert!(f.rx.try_recv().is_ok());
        assert_eq!(f.rx.try_recv().err(), Some(TryRecvError::Empty));
        tx2.emit(ConnectionEvent::Connected);
        drop(tx2);
        assert!(f.rx.recv().is_some());
        assert!(f.rx.recv().is_none());
        assert_eq!(f.rx.try_recv().err(), Some(TryRecvError::Disconnected));
        assert_eq!(
            f.rx.recv_timeout(Duration::from_secs(5)).err(),
            Some(RecvTimeoutError::Disconnected)
        );
    }

    #[test]
    fn nothing_is_queued_once_the_receiver_is_gone() {
        let f = fixture(1, 1);
        drop(f.rx);
        open(&f.tx);
        f.tx.push_message(message(0));
        f.tx.push_message(message(1));
        f.tx.emit(ConnectionEvent::Connecting);
        assert_eq!((f.messages_dropped.load(), f.events_dropped.load()), (0, 0));
    }

    #[test]
    fn blocking_recv_wakes_on_push_and_on_close() {
        let f = fixture(8, 8);
        let rx = Arc::new(f.rx);
        let reader = {
            let rx = Arc::clone(&rx);
            thread::spawn(move || (rx.recv().is_some(), rx.recv().is_none()))
        };
        thread::sleep(Duration::from_millis(50));
        f.tx.emit(ConnectionEvent::Connecting);
        thread::sleep(Duration::from_millis(50));
        drop(f.tx);
        assert_eq!(reader.join().unwrap(), (true, true));
    }

    #[test]
    fn recv_timeout_times_out_while_open() {
        let f = fixture(8, 8);
        assert_eq!(
            f.rx.recv_timeout(Duration::from_millis(20)).err(),
            Some(RecvTimeoutError::Timeout)
        );
    }

    #[test]
    fn poll_recv_wakes_the_latest_waker_on_push_and_close() {
        let f = fixture(8, 8);
        let first = Arc::new(CountingWaker(AtomicUsize::new(0)));
        let second = Arc::new(CountingWaker(AtomicUsize::new(0)));
        let first_waker = Waker::from(Arc::clone(&first));
        let second_waker = Waker::from(Arc::clone(&second));

        assert!(f.rx.poll_recv(&mut Context::from_waker(&first_waker)).is_pending());
        assert!(f.rx.poll_recv(&mut Context::from_waker(&second_waker)).is_pending());
        f.tx.emit(ConnectionEvent::Connecting);
        assert_eq!(first.0.load(Ordering::SeqCst), 0);
        assert_eq!(second.0.load(Ordering::SeqCst), 1);
        assert!(f.rx.poll_recv(&mut Context::from_waker(&second_waker)).is_ready());

        assert!(f.rx.poll_recv(&mut Context::from_waker(&second_waker)).is_pending());
        drop(f.tx);
        assert_eq!(second.0.load(Ordering::SeqCst), 2);
        assert!(matches!(
            f.rx.poll_recv(&mut Context::from_waker(&second_waker)),
            Poll::Ready(None)
        ));
    }
}
