//! Inbound message queue between the network loop and its consumer (#46).
//!
//! Runtime-free: a `Mutex<VecDeque>` read through a `Condvar` by blocking
//! consumers ([`MessageReceiver`](crate::websocket::MessageReceiver)) and
//! through a stored [`Waker`] by async ones
//! ([`MessageStream`](crate::websocket::MessageStream)). Both clients push
//! into it straight from their network loop, so no forwarding task sits in
//! between and the configured capacity is the only place a message can be
//! dropped.
//!
//! The queue also keeps the drop bookkeeping behind
//! [`ConnectionEvent::MessagesDropped`](crate::websocket::ConnectionEvent::MessagesDropped):
//! drops and the report that covers them are counted under the same lock.

use crate::metrics_compat::DropCounter;
use std::collections::VecDeque;
use std::sync::mpsc::{RecvTimeoutError, TryRecvError};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::task::{Context, Poll, Waker};
use std::time::{Duration, Instant};

/// Minimum spacing between two throttled drop reports on one connection.
pub(crate) const DROP_REPORT_INTERVAL: Duration = Duration::from_secs(1);

/// Drops not yet reported, as carried by `ConnectionEvent::MessagesDropped`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DropReport {
    /// Messages dropped since the previous report.
    pub dropped: u64,
    /// Messages dropped since the client was constructed.
    pub total: u64,
}

/// What [`QueueSender::push`] did with a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Pushed {
    /// Appended to the queue.
    Queued,
    /// Discarded because the queue was full; counted as dropped.
    Dropped,
    /// Discarded because the receiver is gone; not counted.
    NoReceiver,
}

struct State<T> {
    items: VecDeque<T>,
    senders: usize,
    receiver_alive: bool,
    waker: Option<Waker>,
    /// Drops not covered by a report yet.
    unreported: u64,
    /// When the last report was taken on the current connection.
    last_report: Option<Instant>,
    /// Inside the lock: a bare `DropCounter` (it may hold a `metrics`
    /// handle) would make the public receivers `!RefUnwindSafe`.
    dropped: DropCounter,
}

struct Shared<T> {
    state: Mutex<State<T>>,
    available: Condvar,
    /// `None` means unbounded.
    capacity: Option<usize>,
}

impl<T> Shared<T> {
    /// Every critical section leaves the state consistent, so a poisoned
    /// lock still yields a usable value.
    fn lock(&self) -> MutexGuard<'_, State<T>> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Create a queue holding at most `capacity` items (`None`: unbounded).
/// `dropped` is bumped once per item discarded for lack of room.
pub(crate) fn queue<T>(
    capacity: Option<usize>,
    dropped: DropCounter,
) -> (QueueSender<T>, QueueReceiver<T>) {
    let shared = Arc::new(Shared {
        state: Mutex::new(State {
            items: VecDeque::new(),
            senders: 1,
            receiver_alive: true,
            waker: None,
            unreported: 0,
            last_report: None,
            dropped,
        }),
        available: Condvar::new(),
        capacity,
    });
    (
        QueueSender {
            shared: Arc::clone(&shared),
        },
        QueueReceiver { shared },
    )
}

/// Producer end. Cloneable; the queue reports closed once every clone is
/// dropped and the remaining items have been read.
pub(crate) struct QueueSender<T> {
    shared: Arc<Shared<T>>,
}

impl<T> Clone for QueueSender<T> {
    fn clone(&self) -> Self {
        self.shared.lock().senders += 1;
        Self {
            shared: Arc::clone(&self.shared),
        }
    }
}

impl<T> Drop for QueueSender<T> {
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

impl<T> QueueSender<T> {
    /// Append `item`, or discard it if the queue is full. Drops are left
    /// for a later report; use this where no report may be emitted yet
    /// (the auth handshake runs before `Authenticated`).
    pub(crate) fn push(&self, item: T) -> Pushed {
        self.push_inner(item, false).0
    }

    /// [`push`](Self::push), then take a report of the drops so far if one
    /// is due: the first on this connection, or [`DROP_REPORT_INTERVAL`]
    /// after the previous one. Checking after successful pushes too means
    /// the tail of a drop burst is reported within the interval.
    pub(crate) fn push_and_report(&self, item: T) -> (Pushed, Option<DropReport>) {
        self.push_inner(item, true)
    }

    fn push_inner(&self, item: T, report: bool) -> (Pushed, Option<DropReport>) {
        let mut state = self.shared.lock();
        if !state.receiver_alive {
            return (Pushed::NoReceiver, None);
        }
        let full = self
            .shared
            .capacity
            .is_some_and(|capacity| state.items.len() >= capacity);
        let (pushed, waker) = if full {
            state.unreported += 1;
            state.dropped.bump();
            (Pushed::Dropped, None)
        } else {
            state.items.push_back(item);
            (Pushed::Queued, state.waker.take())
        };
        let report = if report {
            self.take_report(&mut state, false)
        } else {
            None
        };
        drop(state);
        if pushed == Pushed::Queued {
            self.shared.available.notify_one();
            if let Some(waker) = waker {
                waker.wake();
            }
        }
        (pushed, report)
    }

    /// Take a report of every unreported drop, due or not. Used right before
    /// a connection's `Disconnected` so its drops are all accounted for.
    pub(crate) fn take_unreported(&self) -> Option<DropReport> {
        let mut state = self.shared.lock();
        self.take_report(&mut state, true)
    }

    /// Start a new connection: its first drop report is not held back by a
    /// report taken on the previous one.
    pub(crate) fn start_connection(&self) {
        self.shared.lock().last_report = None;
    }

    fn take_report(&self, state: &mut State<T>, force: bool) -> Option<DropReport> {
        if state.unreported == 0 {
            return None;
        }
        let now = Instant::now();
        let throttled = state
            .last_report
            .is_some_and(|last| now.duration_since(last) < DROP_REPORT_INTERVAL);
        if throttled && !force {
            return None;
        }
        state.last_report = Some(now);
        Some(DropReport {
            dropped: std::mem::take(&mut state.unreported),
            total: state.dropped.load(),
        })
    }
}

/// Consumer end. Every method takes `&self`, so it can be shared between
/// threads; each item is delivered to exactly one caller.
pub(crate) struct QueueReceiver<T> {
    shared: Arc<Shared<T>>,
}

impl<T> Drop for QueueReceiver<T> {
    fn drop(&mut self) {
        let mut state = self.shared.lock();
        state.receiver_alive = false;
        state.waker = None;
        let items = std::mem::take(&mut state.items);
        drop(state);
        drop(items);
    }
}

impl<T> QueueReceiver<T> {
    /// Next item without waiting.
    pub(crate) fn try_recv(&self) -> Result<T, TryRecvError> {
        let mut state = self.shared.lock();
        match state.items.pop_front() {
            Some(item) => Ok(item),
            None if state.senders == 0 => Err(TryRecvError::Disconnected),
            None => Err(TryRecvError::Empty),
        }
    }

    /// Next item, waiting for one. `None` once the queue is closed and empty.
    pub(crate) fn recv(&self) -> Option<T> {
        let mut state = self.shared.lock();
        loop {
            if let Some(item) = state.items.pop_front() {
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
    pub(crate) fn recv_timeout(&self, timeout: Duration) -> Result<T, RecvTimeoutError> {
        let deadline = Instant::now() + timeout;
        let mut state = self.shared.lock();
        loop {
            if let Some(item) = state.items.pop_front() {
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
    pub(crate) fn poll_recv(&self, cx: &mut Context<'_>) -> Poll<Option<T>> {
        let mut state = self.shared.lock();
        if let Some(item) = state.items.pop_front() {
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
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::task::Wake;
    use std::thread;

    fn counter() -> DropCounter {
        DropCounter::new("test_messages_dropped", "localhost", "test")
    }

    struct CountingWaker(AtomicUsize);

    impl Wake for CountingWaker {
        fn wake(self: Arc<Self>) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn bounded_queue_drops_newest_and_counts() {
        let dropped = counter();
        let (tx, rx) = queue(Some(2), dropped.clone());
        assert_eq!(tx.push(1), Pushed::Queued);
        assert_eq!(tx.push(2), Pushed::Queued);
        assert_eq!(tx.push(3), Pushed::Dropped);
        assert_eq!(dropped.load(), 1);
        assert_eq!(rx.try_recv(), Ok(1));
        assert_eq!(tx.push(4), Pushed::Queued);
        assert_eq!(rx.try_recv(), Ok(2));
        assert_eq!(rx.try_recv(), Ok(4));
        assert_eq!(rx.try_recv(), Err(TryRecvError::Empty));
    }

    #[test]
    fn unbounded_queue_never_drops() {
        let dropped = counter();
        let (tx, rx) = queue(None, dropped.clone());
        for i in 0..10_000 {
            assert_eq!(tx.push(i), Pushed::Queued);
        }
        assert_eq!(dropped.load(), 0);
        assert_eq!((0..10_000).map(|_| rx.try_recv().unwrap()).sum::<i32>(), (0..10_000).sum::<i32>());
    }

    #[test]
    fn closes_only_when_every_sender_is_gone_and_items_are_read() {
        let (tx, rx) = queue(None, counter());
        let tx2 = tx.clone();
        tx.push(1);
        drop(tx);
        assert_eq!(rx.try_recv(), Ok(1));
        assert_eq!(rx.try_recv(), Err(TryRecvError::Empty));
        tx2.push(2);
        drop(tx2);
        assert_eq!(rx.recv(), Some(2));
        assert_eq!(rx.recv(), None);
        assert_eq!(rx.try_recv(), Err(TryRecvError::Disconnected));
        assert_eq!(
            rx.recv_timeout(Duration::from_secs(5)),
            Err(RecvTimeoutError::Disconnected)
        );
    }

    #[test]
    fn push_without_receiver_is_not_a_drop() {
        let dropped = counter();
        let (tx, rx) = queue(Some(1), dropped.clone());
        drop(rx);
        assert_eq!(tx.push(1), Pushed::NoReceiver);
        assert_eq!(tx.push(2), Pushed::NoReceiver);
        assert_eq!(dropped.load(), 0);
        assert_eq!(tx.take_unreported(), None);
    }

    #[test]
    fn blocking_recv_wakes_on_push_and_on_close() {
        let (tx, rx) = queue::<u32>(None, counter());
        let rx = Arc::new(rx);
        let reader = {
            let rx = Arc::clone(&rx);
            thread::spawn(move || (rx.recv(), rx.recv()))
        };
        thread::sleep(Duration::from_millis(50));
        tx.push(7);
        thread::sleep(Duration::from_millis(50));
        drop(tx);
        assert_eq!(reader.join().unwrap(), (Some(7), None));
    }

    #[test]
    fn recv_timeout_times_out_while_open() {
        let (_tx, rx) = queue::<u32>(None, counter());
        assert_eq!(
            rx.recv_timeout(Duration::from_millis(20)),
            Err(RecvTimeoutError::Timeout)
        );
    }

    #[test]
    fn poll_recv_registers_waker_and_is_woken_on_push_and_close() {
        let (tx, rx) = queue::<u32>(None, counter());
        let wakes = Arc::new(CountingWaker(AtomicUsize::new(0)));
        let waker = Waker::from(Arc::clone(&wakes));
        let mut cx = Context::from_waker(&waker);

        assert_eq!(rx.poll_recv(&mut cx), Poll::Pending);
        tx.push(1);
        assert_eq!(wakes.0.load(Ordering::SeqCst), 1);
        assert_eq!(rx.poll_recv(&mut cx), Poll::Ready(Some(1)));

        assert_eq!(rx.poll_recv(&mut cx), Poll::Pending);
        drop(tx);
        assert_eq!(wakes.0.load(Ordering::SeqCst), 2);
        assert_eq!(rx.poll_recv(&mut cx), Poll::Ready(None));
    }

    #[test]
    fn poll_recv_wakes_the_latest_waker() {
        let (tx, rx) = queue::<u32>(None, counter());
        let first = Arc::new(CountingWaker(AtomicUsize::new(0)));
        let second = Arc::new(CountingWaker(AtomicUsize::new(0)));
        let first_waker = Waker::from(Arc::clone(&first));
        let second_waker = Waker::from(Arc::clone(&second));

        assert!(rx.poll_recv(&mut Context::from_waker(&first_waker)).is_pending());
        assert!(rx.poll_recv(&mut Context::from_waker(&second_waker)).is_pending());
        tx.push(1);
        assert_eq!(first.0.load(Ordering::SeqCst), 0);
        assert_eq!(second.0.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn first_drop_is_reported_at_once_then_throttled() {
        let (tx, _rx) = queue(Some(1), counter());
        tx.push(0);
        assert_eq!(
            tx.push_and_report(1),
            (Pushed::Dropped, Some(DropReport { dropped: 1, total: 1 }))
        );
        assert_eq!(tx.push_and_report(2), (Pushed::Dropped, None));
        assert_eq!(tx.push_and_report(3), (Pushed::Dropped, None));
        assert_eq!(
            tx.take_unreported(),
            Some(DropReport { dropped: 2, total: 3 })
        );
        assert_eq!(tx.take_unreported(), None);
    }

    #[test]
    fn report_comes_due_after_the_interval_even_on_a_successful_push() {
        let (tx, rx) = queue(Some(1), counter());
        tx.push(0);
        assert!(tx.push_and_report(1).1.is_some());
        assert_eq!(tx.push_and_report(2), (Pushed::Dropped, None));
        thread::sleep(DROP_REPORT_INTERVAL);
        rx.try_recv().unwrap();
        assert_eq!(
            tx.push_and_report(3),
            (Pushed::Queued, Some(DropReport { dropped: 1, total: 2 }))
        );
    }

    #[test]
    fn handshake_drops_wait_for_a_reporting_push() {
        let (tx, rx) = queue(Some(1), counter());
        tx.push(0);
        assert_eq!(tx.push(1), Pushed::Dropped);
        rx.try_recv().unwrap();
        assert_eq!(
            tx.push_and_report(2),
            (Pushed::Queued, Some(DropReport { dropped: 1, total: 1 }))
        );
    }

    #[test]
    fn a_new_connection_reports_its_first_drop_at_once() {
        let (tx, _rx) = queue(Some(1), counter());
        tx.push(0);
        assert!(tx.push_and_report(1).1.is_some());
        tx.start_connection();
        assert_eq!(
            tx.push_and_report(2),
            (Pushed::Dropped, Some(DropReport { dropped: 1, total: 2 }))
        );
    }
}
