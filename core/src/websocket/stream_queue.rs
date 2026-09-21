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
//! - the claim on reporting that connection's `Disconnected` (#41), and
//!   whether the client's close has been reported: one final event per
//!   disconnect, even when `disconnect()` stops a reconnect (#98), and
//!   nothing after it from the reconnect loop (#145) or from the connection
//!   that failed (#159);
//! - the drop bookkeeping behind `MessagesDropped`;
//! - when an automatic reconnect last restored the connection, behind the
//!   reconnect-conflict warning (#226).
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

/// The `Closed` reason recorded by [`StreamSender::reconnect_failed`] when
/// the reconnect loop ran out of attempts.
pub(crate) const MAX_ATTEMPTS_REASON: &str = "Max reconnection attempts reached";

/// The `Closed` reason recorded by [`StreamSender::reconnect_failed`] when
/// a reconnect attempt's credentials were rejected (#201): the server's
/// rejection message, so the state names why the loop stopped.
pub(crate) fn rejected_reason(message: &str) -> String {
    format!("Credentials rejected: {message}")
}

/// How soon after an automatic reconnect a caller's close is taken for code
/// that reconnects on its own as well, and warned about (#226). Such code
/// closes the connection the reconnect restored a few seconds later, and its
/// own `disconnect` handler then starts the next round.
pub(crate) const RECONNECT_CONFLICT_WINDOW: Duration = Duration::from_secs(30);

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
    /// The client's close has been reported: a `Disconnected` with
    /// `will_reconnect: false` or a `ReconnectFailed` is queued, and nothing
    /// reconnects until a connection authenticates again (#98). The reconnect
    /// loop (#145) and a failing connection (#159) report only while it is
    /// unset.
    close_reported: bool,
    /// When the reconnect loop authenticated the current connection; `None`
    /// once a caller's `connect()` has authenticated one since.
    reconnected_at: Option<Instant>,
    /// The reconnect-conflict warning has been queued: once per client.
    conflict_warned: bool,
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
            close_reported: false,
            reconnected_at: None,
            conflict_warned: false,
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

/// A connection's `Disconnected`, before it is queued.
struct Disconnect {
    code: Option<u16>,
    reason: String,
    intent: DisconnectIntent,
    will_reconnect: bool,
}

impl Disconnect {
    /// The state a consumer handling the event reads (#86); see
    /// [`StreamSender::connection_lost`].
    fn state(&self) -> ConnectionState {
        if self.will_reconnect {
            ConnectionState::Disconnected
        } else {
            ConnectionState::Closed {
                code: self.code,
                reason: self.reason.clone(),
                intent: self.intent,
            }
        }
    }

    fn event(self) -> ConnectionEvent {
        ConnectionEvent::Disconnected {
            code: self.code,
            reason: self.reason,
            intent: self.intent,
            will_reconnect: self.will_reconnect,
        }
    }
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
        state.reconnected_at = None;
        self.open_connection(&mut state, data, frames, &mut outcome);
        self.finish(state, outcome);
    }

    fn open_connection(
        &self,
        state: &mut State,
        data: serde_json::Value,
        frames: Vec<WebSocketMessage>,
        outcome: &mut Outcome,
    ) {
        state.disconnect_claimed = false;
        state.close_reported = false;
        state.open = true;
        self.push_event(state, ConnectionEvent::Authenticated { data }, outcome);
        for frame in frames {
            self.push_message_locked(state, frame, outcome);
        }
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
        self.push_rejection(&mut state, message, data, frames, &mut outcome);
        self.finish(state, outcome);
    }

    fn push_rejection(
        &self,
        state: &mut State,
        message: String,
        data: serde_json::Value,
        frames: Vec<WebSocketMessage>,
        outcome: &mut Outcome,
    ) {
        self.push_event(state, ConnectionEvent::Unauthenticated { message, data }, outcome);
        for frame in frames {
            self.push_message_locked(state, frame, outcome);
        }
    }

    /// Report a step of the reconnect loop: set `connection` to `next` and
    /// queue `event`, unless the client's close has been reported (#145).
    /// `false` means it has: nothing was changed and the loop must stop, so
    /// the final event stays final and its `Closed` state is kept.
    ///
    /// Only the reconnect loop reports through this: a `connect()` the caller
    /// starts is never held back (a reported close leaves the client
    /// `Closed`, which refuses `connect()` anyway). Same lock order as
    /// [`connection_lost`](Self::connection_lost).
    pub(crate) fn reconnect_step(
        &self,
        connection: &RwLock<ConnectionState>,
        next: ConnectionState,
        event: Option<ConnectionEvent>,
    ) -> bool {
        self.unless_closed(|state, outcome| {
            // Writers only assign, so a poisoned lock holds a whole value.
            *connection.write().unwrap_or_else(PoisonError::into_inner) = next;
            if let Some(event) = event {
                self.push_event(state, event, outcome);
            }
        })
    }

    /// Queue `event` unless the client's close has been reported: an
    /// `Error` of a reconnect attempt or a subscription replay (#145), or of
    /// a frame the closed connection still delivered (#159), is not reported
    /// after the final event. `false` if it was not queued.
    pub(crate) fn emit_unless_closed(&self, event: ConnectionEvent) -> bool {
        self.unless_closed(|state, outcome| self.push_event(state, event, outcome))
    }

    /// [`authenticated`](Self::authenticated) for the reconnect loop, with
    /// `connection` set to `Connected` first, unless the client's close has
    /// been reported (#145): a stopped reconnect never reopens. `false`
    /// means it has, and nothing was changed.
    pub(crate) fn reconnect_authenticated(
        &self,
        connection: &RwLock<ConnectionState>,
        data: serde_json::Value,
        frames: Vec<WebSocketMessage>,
    ) -> bool {
        self.unless_closed(|state, outcome| {
            // Writers only assign, so a poisoned lock holds a whole value.
            *connection.write().unwrap_or_else(PoisonError::into_inner) =
                ConnectionState::Connected;
            state.reconnected_at = Some(Instant::now());
            self.open_connection(state, data, frames, outcome);
        })
    }

    /// [`unauthenticated`](Self::unauthenticated) for the reconnect loop,
    /// unless the client's close has been reported (#145).
    pub(crate) fn reconnect_rejected(
        &self,
        message: String,
        data: serde_json::Value,
        frames: Vec<WebSocketMessage>,
    ) -> bool {
        self.unless_closed(|state, outcome| {
            self.push_rejection(state, message, data, frames, outcome);
        })
    }

    /// Run `report` under the lock unless the client's close has been
    /// reported; `false` if it has.
    fn unless_closed(&self, report: impl FnOnce(&mut State, &mut Outcome)) -> bool {
        let mut state = self.shared.lock();
        if state.close_reported {
            return false;
        }
        let mut outcome = Outcome::default();
        report(&mut state, &mut outcome);
        self.finish(state, outcome);
        true
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
        let mut state = self.shared.lock();
        if state.disconnect_claimed {
            return;
        }
        let mut outcome = Outcome::default();
        let disconnect = Disconnect { code, reason, intent, will_reconnect };
        self.push_disconnected(&mut state, disconnect, &mut outcome);
        self.finish(state, outcome);
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
        let mut state = self.shared.lock();
        let mut outcome = Outcome::default();
        let disconnect = Disconnect { code, reason, intent, will_reconnect };
        self.lose_connection(&mut state, connection, disconnect, &mut outcome);
        self.finish(state, outcome);
    }

    /// [`connection_lost`](Self::connection_lost) for a connection that
    /// failed: queue `event`, its `Error` or `HeartbeatTimeout`, then its
    /// `Disconnected` with [`DisconnectIntent::Network`] and no code, under
    /// one lock, unless the client's close has been reported (#159). `false`
    /// means it has: `disconnect()` / `force_close()` reported the close
    /// between the caller reading its stop flag and this, and nothing was
    /// queued, so the final event stays final.
    ///
    /// Same lock order as [`connection_lost`](Self::connection_lost).
    pub(crate) fn connection_failed(
        &self,
        connection: &RwLock<ConnectionState>,
        event: ConnectionEvent,
        reason: String,
        will_reconnect: bool,
    ) -> bool {
        self.unless_closed(|state, outcome| {
            self.push_event(state, event, outcome);
            // Past the gate, the claim below is free: the flags only differ
            // (`disconnect_claimed` set, `close_reported` unset) after a
            // `Disconnected { will_reconnect: true }`, which only the
            // connection's own reader queues, and it returns right after
            // instead of reporting a failure of the same connection. Every
            // other report leaves them equal: `client_closed` sets both,
            // `reconnect_failed` sets `close_reported` after that
            // `Disconnected` took the claim, and `open_connection` clears
            // both. Were the claim taken anyway, `event` would be queued and
            // the `Disconnected` skipped, as before #159: not after a final
            // event, since that would have set `close_reported`.
            let disconnect = Disconnect {
                code: None,
                reason,
                intent: DisconnectIntent::Network,
                will_reconnect,
            };
            self.lose_connection(state, connection, disconnect, outcome);
        })
    }

    /// Close the client on the caller's request (`disconnect()`,
    /// `force_close()`): set `connection` to `Closed` with
    /// [`DisconnectIntent::Client`] and queue the matching `Disconnected`
    /// with `will_reconnect: false`, unless the client's close has been
    /// reported already (#93). Then that report stands:
    /// - a `Closed` state it recorded is left as it is, so the state keeps
    ///   agreeing with the event;
    /// - any other state is set to `Closed` without a further event.
    ///
    /// A connection lost with `will_reconnect: true` has not reported the
    /// client's close: stopping its reconnect queues the final
    /// `Disconnected` here, so the stream itself says no reconnect follows
    /// (#98).
    ///
    /// Closing an open connection that the reconnect loop authenticated less
    /// than [`RECONNECT_CONFLICT_WINDOW`] ago first queues the
    /// reconnect-conflict warning, once per client (#226).
    ///
    /// Same lock order as [`connection_lost`](Self::connection_lost).
    pub(crate) fn client_closed(
        &self,
        connection: &RwLock<ConnectionState>,
        code: u16,
        reason: String,
    ) {
        let mut state = self.shared.lock();
        {
            // Writers only assign, so a poisoned lock holds a whole value.
            let mut current = connection.write().unwrap_or_else(PoisonError::into_inner);
            let keep = state.close_reported && matches!(*current, ConnectionState::Closed { .. });
            if !keep {
                *current = ConnectionState::Closed {
                    code: Some(code),
                    reason: reason.clone(),
                    intent: DisconnectIntent::Client,
                };
            }
        }
        let mut outcome = Outcome::default();
        if !state.close_reported {
            let elapsed = state.reconnected_at.map(|at| at.elapsed());
            if let Some(elapsed) = elapsed.filter(|elapsed| *elapsed < RECONNECT_CONFLICT_WINDOW) {
                if state.open && !state.conflict_warned {
                    state.conflict_warned = true;
                    crate::tracing_compat::warn!(
                        target: "fugle_marketdata::ws",
                        elapsed_ms = elapsed.as_millis() as u64,
                        "ws closed by the caller soon after an automatic reconnect"
                    );
                    self.push_event(&mut state, ConnectionEvent::reconnect_conflict(elapsed), &mut outcome);
                }
            }
            let disconnect = Disconnect {
                code: Some(code),
                reason,
                intent: DisconnectIntent::Client,
                will_reconnect: false,
            };
            self.push_disconnected(&mut state, disconnect, &mut outcome);
        }
        self.finish(state, outcome);
    }

    /// Report the reconnect loop giving up after `attempts`: set
    /// `connection` to `Closed` with `code`, the close that started the
    /// loop, `reason` and `intent` — [`DisconnectIntent::Network`] and
    /// [`MAX_ATTEMPTS_REASON`] when the attempts ran out,
    /// [`DisconnectIntent::Server`] and [`rejected_reason`] when the server
    /// rejected an attempt's credentials (#201) — then queue
    /// `ReconnectFailed`, unless the client's close has been reported
    /// already (a `disconnect()` got there first). Same lock order as
    /// [`connection_lost`](Self::connection_lost).
    pub(crate) fn reconnect_failed(
        &self,
        connection: &RwLock<ConnectionState>,
        code: Option<u16>,
        attempts: u32,
        reason: String,
        intent: DisconnectIntent,
    ) {
        let mut state = self.shared.lock();
        if state.close_reported {
            return;
        }
        state.close_reported = true;
        // Writers only assign, so a poisoned lock holds a whole value.
        *connection.write().unwrap_or_else(PoisonError::into_inner) =
            ConnectionState::Closed { code, reason, intent };
        let mut outcome = Outcome::default();
        self.push_event(&mut state, ConnectionEvent::ReconnectFailed { attempts }, &mut outcome);
        self.finish(state, outcome);
    }

    /// Under `state`: claim the connection's `Disconnected`, set
    /// `connection` to what it reports, and queue it, unless it is claimed
    /// already (see [`connection_lost`](Self::connection_lost)).
    fn lose_connection(
        &self,
        state: &mut State,
        connection: &RwLock<ConnectionState>,
        disconnect: Disconnect,
        outcome: &mut Outcome,
    ) {
        if state.disconnect_claimed {
            return;
        }
        // Writers only assign, so a poisoned lock holds a whole value.
        *connection.write().unwrap_or_else(PoisonError::into_inner) = disconnect.state();
        self.push_disconnected(state, disconnect, outcome);
    }

    /// Claim the connection's `Disconnected` and queue it under `state`.
    /// The caller has checked the claim is free, or, for the final event of
    /// a stopped reconnect, that the client's close is unreported.
    fn push_disconnected(&self, state: &mut State, disconnect: Disconnect, outcome: &mut Outcome) {
        state.disconnect_claimed = true;
        state.close_reported |= !disconnect.will_reconnect;
        state.open = false;
        self.push_report(state, true, outcome);
        self.push_event(state, disconnect.event(), outcome);
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
            code: None,
            message: None,
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

    /// A connection the reconnect loop authenticated, after one the caller
    /// opened was lost.
    fn reconnected(f: &Fixture, connection: &RwLock<ConnectionState>) {
        open(&f.tx);
        f.tx.connection_lost(connection, None, "lost".into(), DisconnectIntent::Network, true);
        f.tx.start_connection();
        assert!(f.tx.reconnect_authenticated(connection, serde_json::Value::Null, Vec::new()));
        drain(&f.rx);
    }

    fn is_conflict_warning(item: &str) -> bool {
        item.starts_with("Error") && item.contains("code: 3006")
    }

    #[test]
    fn closing_soon_after_a_reconnect_warns_before_the_disconnected() {
        let f = fixture(8, 8);
        let connection = RwLock::new(ConnectionState::Connected);
        reconnected(&f, &connection);

        f.tx.client_closed(&connection, 1000, "Normal closure".into());
        let items = drain(&f.rx);
        assert_eq!(items.len(), 2, "{items:?}");
        assert!(is_conflict_warning(&items[0]), "{items:?}");
        assert!(items[0].contains("source_kind: Client"), "{items:?}");
        assert!(items[1].starts_with("Disconnected") && items[1].contains("Client"), "{items:?}");
    }

    #[test]
    fn the_reconnect_conflict_is_warned_about_once_per_client() {
        let f = fixture(8, 8);
        let connection = RwLock::new(ConnectionState::Connected);
        reconnected(&f, &connection);
        f.tx.client_closed(&connection, 1000, "Normal closure".into());
        drain(&f.rx);

        // The caller's own connect() does not count; the next reconnect does,
        // but the warning was given already.
        reconnected(&f, &connection);
        f.tx.client_closed(&connection, 1000, "Normal closure".into());
        let items = drain(&f.rx);
        assert!(!items.iter().any(|i| is_conflict_warning(i)), "{items:?}");
        assert_eq!(disconnected_items(&items).len(), 1, "{items:?}");
    }

    #[test]
    fn no_reconnect_conflict_without_a_recent_reconnect() {
        // A connection the caller opened.
        let f = fixture(8, 8);
        let connection = RwLock::new(ConnectionState::Connected);
        reconnected(&f, &connection);
        f.tx.start_connection();
        f.tx.authenticated(serde_json::Value::Null, Vec::new());
        f.tx.client_closed(&connection, 1000, "Normal closure".into());
        assert!(!drain(&f.rx).iter().any(|i| is_conflict_warning(i)));

        // A reconnect longer ago than the window.
        let f = fixture(8, 8);
        let connection = RwLock::new(ConnectionState::Connected);
        reconnected(&f, &connection);
        f.tx.shared.lock().reconnected_at =
            Instant::now().checked_sub(RECONNECT_CONFLICT_WINDOW + Duration::from_secs(1));
        f.tx.client_closed(&connection, 1000, "Normal closure".into());
        let items = drain(&f.rx);
        assert!(!items.iter().any(|i| is_conflict_warning(i)), "{items:?}");
        assert_eq!(disconnected_items(&items).len(), 1, "{items:?}");

        // Stopping a reconnect in progress: the connection is not open.
        let f = fixture(8, 8);
        let connection = RwLock::new(ConnectionState::Connected);
        reconnected(&f, &connection);
        f.tx.connection_lost(&connection, None, "lost".into(), DisconnectIntent::Network, true);
        drain(&f.rx);
        f.tx.client_closed(&connection, 1000, "Normal closure".into());
        assert!(!drain(&f.rx).iter().any(|i| is_conflict_warning(i)));
    }

    fn disconnected_items(items: &[String]) -> Vec<&String> {
        items.iter().filter(|e| e.starts_with("Disconnected")).collect()
    }

    #[test]
    fn client_closed_while_reconnecting_reports_the_final_disconnected() {
        let f = fixture(8, 8);
        open(&f.tx);
        let connection = Arc::new(RwLock::new(ConnectionState::Connected));
        f.tx.connection_lost(&connection, None, "lost".into(), DisconnectIntent::Network, true);
        *connection.write().unwrap() = ConnectionState::Reconnecting { attempt: 1 };
        drain(&f.rx);

        // The state is recorded before the event is queued (#86).
        let tx = f.tx.clone();
        let reader = {
            let connection = Arc::clone(&connection);
            thread::spawn(move || {
                let item = f.rx.recv().expect("an item");
                let state = connection.read().unwrap().clone();
                (format!("{item:?}"), state, f)
            })
        };
        tx.client_closed(&connection, 1000, "Normal closure".into());
        let (item, state, f) = reader.join().unwrap();
        assert_eq!(
            item,
            format!(
                "{:?}",
                StreamItem::Event(ConnectionEvent::Disconnected {
                    code: Some(1000),
                    reason: "Normal closure".into(),
                    intent: DisconnectIntent::Client,
                    will_reconnect: false,
                })
            )
        );
        assert_eq!(state, client_close(1000, "Normal closure"));

        // Reported once: a second close, even after a racing writer moved
        // the state on, only restores `Closed`.
        *connection.write().unwrap() = ConnectionState::Reconnecting { attempt: 2 };
        f.tx.client_closed(&connection, 1006, "Force closed".into());
        assert_eq!(*connection.read().unwrap(), client_close(1006, "Force closed"));
        assert!(drain(&f.rx).is_empty());
    }

    #[test]
    fn reconnect_failed_records_the_state_and_ends_the_close() {
        let f = fixture(8, 8);
        open(&f.tx);
        let connection = RwLock::new(ConnectionState::Connected);
        f.tx.connection_lost(&connection, Some(1006), "lost".into(), DisconnectIntent::Network, true);
        drain(&f.rx);

        f.tx.reconnect_failed(&connection, Some(1006), 3, MAX_ATTEMPTS_REASON.to_string(), DisconnectIntent::Network);
        let failed = ConnectionState::Closed {
            code: Some(1006),
            reason: MAX_ATTEMPTS_REASON.into(),
            intent: DisconnectIntent::Network,
        };
        assert_eq!(*connection.read().unwrap(), failed);
        assert_eq!(drain(&f.rx), vec!["ReconnectFailed { attempts: 3 }".to_string()]);

        f.tx.client_closed(&connection, 1000, "Normal closure".into());
        f.tx.reconnect_failed(&connection, Some(1006), 3, MAX_ATTEMPTS_REASON.to_string(), DisconnectIntent::Network);
        assert_eq!(*connection.read().unwrap(), failed);
        assert!(drain(&f.rx).is_empty());
    }

    #[test]
    fn reconnect_failed_after_client_closed_reports_nothing() {
        let f = fixture(8, 8);
        open(&f.tx);
        let connection = RwLock::new(ConnectionState::Connected);
        f.tx.connection_lost(&connection, None, "lost".into(), DisconnectIntent::Network, true);
        drain(&f.rx);

        f.tx.client_closed(&connection, 1000, "Normal closure".into());
        f.tx.reconnect_failed(&connection, None, 1, MAX_ATTEMPTS_REASON.to_string(), DisconnectIntent::Network);
        assert_eq!(*connection.read().unwrap(), client_close(1000, "Normal closure"));
        assert_eq!(disconnected_items(&drain(&f.rx)).len(), 1);
    }

    #[test]
    fn a_stopped_reconnect_reports_exactly_one_final_event() {
        for _ in 0..200 {
            let f = fixture(8, 8);
            open(&f.tx);
            let connection = Arc::new(RwLock::new(ConnectionState::Connected));
            f.tx.connection_lost(&connection, None, "lost".into(), DisconnectIntent::Network, true);
            drain(&f.rx);
            let closers: Vec<_> = (0..4)
                .map(|i| {
                    let tx = f.tx.clone();
                    let connection = Arc::clone(&connection);
                    thread::spawn(move || {
                        if i == 0 {
                            tx.reconnect_failed(&connection, None, 1, MAX_ATTEMPTS_REASON.to_string(), DisconnectIntent::Network);
                        } else {
                            tx.client_closed(&connection, 1000, "Normal closure".into());
                        }
                    })
                })
                .collect();
            for closer in closers {
                closer.join().expect("closer");
            }
            let items = drain(&f.rx);
            let finals = items
                .iter()
                .filter(|e| e.starts_with("Disconnected") || e.starts_with("ReconnectFailed"))
                .count();
            assert_eq!(finals, 1, "{items:?}");
            assert!(matches!(*connection.read().unwrap(), ConnectionState::Closed { .. }));
        }
    }

    /// Every step the reconnect loop reports; `true` if all were reported.
    fn reconnect_steps(tx: &StreamSender, connection: &RwLock<ConnectionState>) -> Vec<bool> {
        vec![
            tx.reconnect_step(
                connection,
                ConnectionState::Reconnecting { attempt: 1 },
                Some(ConnectionEvent::Reconnecting { attempt: 1 }),
            ),
            tx.reconnect_step(connection, ConnectionState::Connecting, Some(ConnectionEvent::Connecting {})),
            tx.reconnect_step(connection, ConnectionState::Disconnected, None),
            tx.emit_unless_closed(ConnectionEvent::Connected {}),
            tx.reconnect_rejected("nope".into(), serde_json::Value::Null, vec![message(1)]),
            tx.reconnect_authenticated(connection, serde_json::Value::Null, vec![message(2)]),
        ]
    }

    #[test]
    fn a_reported_close_ends_the_reconnect_loop_s_reports() {
        // By the caller, and by the loop giving up.
        for close_by_client in [true, false] {
            let f = fixture(8, 8);
            open(&f.tx);
            let connection = RwLock::new(ConnectionState::Connected);
            f.tx.connection_lost(&connection, None, "lost".into(), DisconnectIntent::Network, true);
            if close_by_client {
                f.tx.client_closed(&connection, 1006, "Force closed".into());
            } else {
                f.tx.reconnect_failed(&connection, None, 1, MAX_ATTEMPTS_REASON.to_string(), DisconnectIntent::Network);
            }
            let closed_state = connection.read().unwrap().clone();
            let before = drain(&f.rx);

            assert_eq!(reconnect_steps(&f.tx, &connection), vec![false; 6]);
            assert!(drain(&f.rx).is_empty(), "nothing follows {before:?}");
            assert_eq!(*connection.read().unwrap(), closed_state);
            // Not reopened: messages are still refused.
            f.tx.push_message(message(3));
            assert!(drain(&f.rx).is_empty());
        }
    }

    fn timeout() -> ConnectionEvent {
        ConnectionEvent::HeartbeatTimeout {
            elapsed: Duration::from_secs(1),
        }
    }

    #[test]
    fn connection_failed_reports_the_event_and_the_disconnected_together() {
        for will_reconnect in [true, false] {
            let f = fixture(8, 8);
            open(&f.tx);
            let connection = RwLock::new(ConnectionState::Connected);
            drain(&f.rx);

            assert!(f.tx.connection_failed(&connection, timeout(), "dead".into(), will_reconnect));
            let expected_state = if will_reconnect {
                ConnectionState::Disconnected
            } else {
                ConnectionState::Closed {
                    code: None,
                    reason: "dead".into(),
                    intent: DisconnectIntent::Network,
                }
            };
            assert_eq!(*connection.read().unwrap(), expected_state);
            assert_eq!(
                drain(&f.rx),
                vec![
                    format!("{:?}", timeout()),
                    format!(
                        "{:?}",
                        ConnectionEvent::Disconnected {
                            code: None,
                            reason: "dead".into(),
                            intent: DisconnectIntent::Network,
                            will_reconnect,
                        }
                    ),
                ]
            );
            // The connection's close is reported once: a further failure of
            // it queues its event alone, or nothing once the close was final.
            let again = f.tx.connection_failed(&connection, timeout(), "dead".into(), will_reconnect);
            assert_eq!(again, will_reconnect);
            let expected = if will_reconnect { vec![format!("{:?}", timeout())] } else { vec![] };
            assert_eq!(drain(&f.rx), expected);
            assert_eq!(*connection.read().unwrap(), expected_state);
        }
    }

    /// The client's close reported between the caller's stop-flag check and
    /// its report (#159): neither the event nor a `Disconnected` follows.
    #[test]
    fn connection_failed_after_the_client_s_close_is_reported_reports_nothing() {
        let f = fixture(8, 8);
        open(&f.tx);
        let connection = RwLock::new(ConnectionState::Connected);
        f.tx.client_closed(&connection, 1006, "Force closed".into());
        let closed_state = connection.read().unwrap().clone();
        assert_eq!(disconnected_items(&drain(&f.rx)).len(), 1);

        for will_reconnect in [true, false] {
            assert!(!f.tx.connection_failed(&connection, timeout(), "dead".into(), will_reconnect));
        }
        assert!(drain(&f.rx).is_empty());
        assert_eq!(*connection.read().unwrap(), closed_state);
    }

    #[test]
    fn the_reconnect_loop_reports_until_the_close_is_reported() {
        let f = fixture(16, 16);
        open(&f.tx);
        let connection = RwLock::new(ConnectionState::Connected);
        f.tx.connection_lost(&connection, None, "lost".into(), DisconnectIntent::Network, true);
        drain(&f.rx);

        assert_eq!(reconnect_steps(&f.tx, &connection), vec![true; 6]);
        assert_eq!(*connection.read().unwrap(), ConnectionState::Connected);
        assert_eq!(
            drain(&f.rx),
            vec![
                "Reconnecting { attempt: 1 }".to_string(),
                "Connecting".into(),
                "Connected".into(),
                "Unauthenticated { message: \"nope\", data: Null }".into(),
                "m1".into(),
                "Authenticated { data: Null }".into(),
                "m2".into(),
            ]
        );
    }

    #[test]
    fn a_new_connection_lets_the_reconnect_loop_report_again() {
        let f = fixture(16, 16);
        open(&f.tx);
        let connection = RwLock::new(ConnectionState::Connected);
        f.tx.client_closed(&connection, 1000, "Normal closure".into());
        // A `connect()` reports through the unconditional calls.
        open(&f.tx);
        drain(&f.rx);
        f.tx.connection_lost(&connection, None, "lost".into(), DisconnectIntent::Network, true);
        assert!(f.tx.reconnect_step(
            &connection,
            ConnectionState::Reconnecting { attempt: 1 },
            Some(ConnectionEvent::Reconnecting { attempt: 1 }),
        ));
    }

    #[test]
    fn a_close_racing_the_reconnect_loop_is_the_last_event() {
        for _ in 0..200 {
            // Room for every step the loop can report, so none is dropped.
            let f = fixture(1_000, 4_000);
            open(&f.tx);
            let connection = Arc::new(RwLock::new(ConnectionState::Connected));
            f.tx.connection_lost(&connection, None, "lost".into(), DisconnectIntent::Network, true);
            drain(&f.rx);
            let reconnecting = {
                let tx = f.tx.clone();
                let connection = Arc::clone(&connection);
                thread::spawn(move || {
                    for _ in 0..500 {
                        if !reconnect_steps(&tx, &connection).iter().all(|&ok| ok) {
                            break;
                        }
                    }
                })
            };
            f.tx.client_closed(&connection, 1006, "Force closed".into());
            reconnecting.join().expect("reconnect loop");
            let items = drain(&f.rx);
            let last = items.last().expect("the final event");
            assert!(last.starts_with("Disconnected") && last.contains("will_reconnect: false"), "{items:?}");
            assert_eq!(*connection.read().unwrap(), client_close(1006, "Force closed"));
        }
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
