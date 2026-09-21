//! What a `connect()` / `wait_connected()` waiting on an automatic
//! reconnect needs to know about it (#230).

use crate::MarketDataError;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, PoisonError};
use tokio::sync::Notify;

/// How an automatic reconnect ended without reconnecting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ReconnectEnd {
    /// Every attempt failed.
    MaxAttempts {
        /// Number of attempts made.
        attempts: u32,
    },
    /// An attempt's credentials were rejected (#201).
    Rejected {
        /// The server's rejection message.
        message: String,
    },
}

impl ReconnectEnd {
    /// The error a waiting `connect()` returns for this end.
    pub(crate) fn into_error(self) -> MarketDataError {
        match self {
            Self::MaxAttempts { attempts } => MarketDataError::ReconnectFailed { attempts },
            Self::Rejected { message } => MarketDataError::AuthError { msg: message, http: None },
        }
    }
}

/// Shared by the client and its dispatch task.
#[derive(Debug, Default)]
pub(crate) struct ConnectWaiters {
    /// Woken when a reconnect has installed its connection and queued its
    /// subscription replay, when the dispatch task ends, when shutdown is
    /// requested, and when a fresh `connect()` returns.
    notify: Notify,
    /// Set from the moment the dispatch loop loses its connection until a
    /// reconnect has queued its replay or the dispatch task ends. The state
    /// is already `Connected` before the replay is queued, so `Connected`
    /// alone does not mean the reconnect is done.
    reconnecting: AtomicBool,
    /// How the last automatic reconnect ended, if it gave up. Cleared when a
    /// reconnect or a fresh `connect()` starts.
    end: Mutex<Option<ReconnectEnd>>,
    /// Test hook: held by a test to keep a reconnect between installing its
    /// connection and queuing its replay.
    #[cfg(test)]
    pub(crate) replay_hold: tokio::sync::Mutex<()>,
}

impl ConnectWaiters {
    /// A future that completes on the next [`wake`](Self::wake). Pin and
    /// `enable` it before checking the state, then await it.
    pub(crate) fn notified(&self) -> tokio::sync::futures::Notified<'_> {
        self.notify.notified()
    }

    /// Wake every waiter to check the state again.
    pub(crate) fn wake(&self) {
        self.notify.notify_waiters();
    }

    /// Whether an automatic reconnect is in progress.
    pub(crate) fn reconnecting(&self) -> bool {
        self.reconnecting.load(Ordering::SeqCst)
    }

    /// The dispatch loop lost its connection: a reconnect may follow.
    pub(crate) fn reconnect_started(&self) {
        self.set_end(None);
        self.reconnecting.store(true, Ordering::SeqCst);
    }

    /// The reconnect queued its replay, or the dispatch task is ending.
    pub(crate) fn reconnect_finished(&self) {
        self.reconnecting.store(false, Ordering::SeqCst);
        self.wake();
    }

    /// A fresh `connect()` is starting: no reconnect is in progress.
    pub(crate) fn fresh_connect(&self) {
        self.set_end(None);
        self.reconnecting.store(false, Ordering::SeqCst);
    }

    /// Record how the reconnect gave up, before its `Closed` state is set.
    pub(crate) fn set_end(&self, end: Option<ReconnectEnd>) {
        *self.end.lock().unwrap_or_else(PoisonError::into_inner) = end;
    }

    /// How the last reconnect gave up, if it did. Every waiter reads it.
    pub(crate) fn end(&self) -> Option<ReconnectEnd> {
        self.end.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }
}

/// Wakes the waiters when dropped: held by a fresh `connect()`, so that
/// whatever it ends in, a waiter checks the state again.
pub(crate) struct WakeOnDrop<'a>(pub(crate) &'a ConnectWaiters);

impl Drop for WakeOnDrop<'_> {
    fn drop(&mut self) {
        self.0.wake();
    }
}
