//! Refusing a `connect()` while another one on the same client is still
//! running (#119).

use std::sync::atomic::{AtomicBool, Ordering};

/// Admits one `connect()` at a time.
#[derive(Debug, Default)]
pub(crate) struct ConnectGate {
    busy: AtomicBool,
}

impl ConnectGate {
    /// Claim the gate, or `None` while another `connect()` holds it. The
    /// claim releases the gate when dropped, including when an async
    /// `connect()` is cancelled mid-handshake.
    pub(crate) fn try_claim(&self) -> Option<ConnectClaim<'_>> {
        self.busy
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .ok()
            .map(|_| ConnectClaim { gate: self })
    }

    /// Whether a `connect()` holds the gate.
    pub(crate) fn is_busy(&self) -> bool {
        self.busy.load(Ordering::SeqCst)
    }
}

/// A held [`ConnectGate`]; see [`ConnectGate::try_claim`].
pub(crate) struct ConnectClaim<'a> {
    gate: &'a ConnectGate,
}

impl Drop for ConnectClaim<'_> {
    fn drop(&mut self) {
        self.gate.busy.store(false, Ordering::SeqCst);
    }
}
