//! Messages handed from a connection's stream reader to `messages()`
//! iterators (#68).
//!
//! The reader is the only consumer of core's ordered stream, so messages no
//! `message` callback takes wait here for an iterator. The queue is bounded:
//! while it is full the reader stops taking items from core, whose own queue
//! then fills and drops (counted and reported by core). Lifecycle callbacks
//! queued behind those messages wait too, until an iterator reads or
//! `disconnect()` is called.

use marketdata_core::WebSocketMessage;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Condvar, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

/// How often a reader blocked on a full queue re-checks its stop flag.
const STOP_POLL: Duration = Duration::from_millis(50);

struct State {
    items: VecDeque<WebSocketMessage>,
    closed: bool,
}

/// Bounded queue from the stream reader to the iterators.
pub(crate) struct Handoff {
    state: Mutex<State>,
    /// Signalled when an item is pushed or the queue closes.
    readable: Condvar,
    /// Signalled when an item is taken.
    writable: Condvar,
    /// `None`: unbounded.
    capacity: Option<usize>,
}

impl Handoff {
    pub(crate) fn new(capacity: Option<usize>) -> Self {
        Self {
            state: Mutex::new(State {
                items: VecDeque::new(),
                closed: false,
            }),
            readable: Condvar::new(),
            writable: Condvar::new(),
            capacity,
        }
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Queue `message`, waiting while the queue is full. Once `stop` is set a
    /// message that would wait is discarded instead, so `disconnect()` never
    /// waits on an iterator nobody reads.
    pub(crate) fn push(&self, message: WebSocketMessage, stop: &AtomicBool) {
        let mut state = self.lock();
        while self.capacity.is_some_and(|capacity| state.items.len() >= capacity) {
            if stop.load(Ordering::SeqCst) || state.closed {
                return;
            }
            state = self
                .writable
                .wait_timeout(state, STOP_POLL)
                .unwrap_or_else(PoisonError::into_inner)
                .0;
        }
        state.items.push_back(message);
        drop(state);
        self.readable.notify_one();
    }

    /// No more messages will be pushed: iterators stop once they have read
    /// what is queued.
    pub(crate) fn close(&self) {
        self.lock().closed = true;
        self.readable.notify_all();
        self.writable.notify_all();
    }

    fn take(&self, state: &mut State) -> Option<WebSocketMessage> {
        let message = state.items.pop_front()?;
        self.writable.notify_one();
        Some(message)
    }

    /// Next message without waiting; `None` when none is queued.
    pub(crate) fn try_receive(&self) -> Option<WebSocketMessage> {
        let mut state = self.lock();
        self.take(&mut state)
    }

    /// Next message, waiting up to `timeout` (`None`: indefinitely).
    ///
    /// `Ok(None)` when the timeout elapsed first, `Err(())` once the queue is
    /// closed and drained.
    pub(crate) fn receive(&self, timeout: Option<Duration>) -> Result<Option<WebSocketMessage>, ()> {
        let deadline = timeout.map(|timeout| Instant::now() + timeout);
        let mut state = self.lock();
        loop {
            if let Some(message) = self.take(&mut state) {
                return Ok(Some(message));
            }
            if state.closed {
                return Err(());
            }
            state = match deadline {
                None => self.readable.wait(state).unwrap_or_else(PoisonError::into_inner),
                Some(deadline) => {
                    let now = Instant::now();
                    if now >= deadline {
                        return Ok(None);
                    }
                    self.readable
                        .wait_timeout(state, deadline - now)
                        .unwrap_or_else(PoisonError::into_inner)
                        .0
                }
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    fn message(id: u32) -> WebSocketMessage {
        WebSocketMessage {
            event: "data".into(),
            data: None,
            channel: None,
            symbol: None,
            id: Some(id.to_string()),
            code: None,
            raw: String::new(),
        }
    }

    #[test]
    fn full_queue_holds_the_reader_until_an_iterator_reads() {
        let handoff = Arc::new(Handoff::new(Some(1)));
        let stop = Arc::new(AtomicBool::new(false));
        handoff.push(message(0), &stop);
        let reader = {
            let (handoff, stop) = (Arc::clone(&handoff), Arc::clone(&stop));
            thread::spawn(move || handoff.push(message(1), &stop))
        };
        thread::sleep(Duration::from_millis(100));
        assert!(!reader.is_finished(), "push must wait while the queue is full");
        assert_eq!(handoff.try_receive().and_then(|m| m.id), Some("0".into()));
        reader.join().unwrap();
        assert_eq!(handoff.try_receive().and_then(|m| m.id), Some("1".into()));
    }

    #[test]
    fn stop_discards_a_message_that_would_wait() {
        let handoff = Arc::new(Handoff::new(Some(1)));
        let stop = Arc::new(AtomicBool::new(false));
        handoff.push(message(0), &stop);
        let reader = {
            let (handoff, stop) = (Arc::clone(&handoff), Arc::clone(&stop));
            thread::spawn(move || handoff.push(message(1), &stop))
        };
        stop.store(true, Ordering::SeqCst);
        reader.join().unwrap();
        // What was already queued stays readable.
        assert_eq!(handoff.try_receive().and_then(|m| m.id), Some("0".into()));
        assert!(handoff.try_receive().is_none());
    }

    #[test]
    fn receive_times_out_then_ends_once_closed_and_drained() {
        let handoff = Handoff::new(None);
        let stop = AtomicBool::new(false);
        assert_eq!(handoff.receive(Some(Duration::from_millis(20))).map(|m| m.is_none()), Ok(true));
        handoff.push(message(0), &stop);
        handoff.close();
        assert!(matches!(handoff.receive(None), Ok(Some(_))));
        assert!(handoff.receive(None).is_err());
    }

    #[test]
    fn close_wakes_a_waiting_iterator() {
        let handoff = Arc::new(Handoff::new(Some(4)));
        let iterator = {
            let handoff = Arc::clone(&handoff);
            thread::spawn(move || handoff.receive(None).is_err())
        };
        thread::sleep(Duration::from_millis(50));
        handoff.close();
        assert!(iterator.join().unwrap());
    }
}
