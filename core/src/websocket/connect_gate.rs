//! Refusing a `connect()` while another one on the same client is still
//! opening a connection (#119). An async `connect()` that waits on a
//! reconnect does not go through the gate (#268).
//!
//! Public as `aio::admission::ConnectGate`, for code that opens each
//! connection on a new client and needs one gate across them (#271).

use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Admits one `connect()` that opens a connection at a time. An async
/// `connect()` that waits on a reconnect does not take it (#268).
///
/// Clones share the gate.
#[derive(Clone, Debug, Default)]
pub struct ConnectGate(Arc<AtomicBool>);

impl ConnectGate {
    /// Claim the gate, or `None` while another `connect()` holds it. The
    /// claim releases the gate when dropped, including when an async
    /// `connect()` is cancelled mid-handshake.
    pub fn try_claim(&self) -> Option<ConnectClaim> {
        self.0
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .ok()
            .map(|_| ConnectClaim(Arc::clone(&self.0)))
    }

    /// Whether a `connect()` holds the gate.
    pub fn is_busy(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// A held [`ConnectGate`]; see [`ConnectGate::try_claim`].
#[must_use = "dropping the claim releases the gate: hold it until the connection is installed or has failed"]
pub struct ConnectClaim(Arc<AtomicBool>);

impl fmt::Debug for ConnectClaim {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ConnectClaim").finish_non_exhaustive()
    }
}

impl Drop for ConnectClaim {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}
