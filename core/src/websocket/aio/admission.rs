//! What a `connect()` does when each connection is opened on a new
//! [`WebSocketClient`]: open one, wait on the automatic reconnect of the one
//! stored, or refuse (#119, #230, #268).
//!
//! For code that stores the client of its current connection and replaces it
//! on the next `connect()`, as the language bindings do. The client's own
//! [`connect()`](WebSocketClient::connect) cannot decide this: it does not
//! see the client before it.
//!
//! # Use
//!
//! The caller keeps one [`ConnectGate`] for all its connections, and with
//! each stored client a [`Delivered`] it feeds:
//!
//! - [`Delivered::observe`] with each [`ConnectionEvent`] of that client,
//!   before the event is handed to the callbacks.
//! - [`Delivered::connect_succeeded`] once, after the connection is
//!   installed (see below).
//!
//! A `connect()` then goes:
//!
//! 1. [`admit`], with a closure that reads the stored connection.
//! 2. [`Admission::Joined`], or an error: return it. Nothing was opened and
//!    the stored connection is as it was.
//! 3. [`Admission::Open`]: build a new client with a new `Delivered`,
//!    connect it, store the two in place of the old connection, call
//!    `connect_succeeded()`. Keep the [`ConnectClaim`] until the connection
//!    is stored, or the connect has failed; only then drop it. Dropped
//!    earlier, a second `connect()` would open a connection beside this one
//!    (#119).

use crate::websocket::aio::WebSocketClient;
use crate::websocket::ConnectionEvent;
use crate::MarketDataError;
use std::fmt;
use std::future::Future;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

pub use crate::websocket::connect_gate::{ConnectClaim, ConnectGate};

/// Whether the caller's callbacks were last handed the connection as up, as
/// a `connect()` decides between refusing and waiting on a reconnect (#230).
///
/// Read from what was delivered rather than the client's state: a
/// disconnect callback that calls `connect()` may run after the client has
/// already reconnected, and has to wait on that reconnect, not be refused.
///
/// One per connection; clones share the record.
#[derive(Clone, Default, Debug)]
pub struct Delivered(Arc<AtomicU8>);

impl Delivered {
    /// No `Authenticated` delivered yet for this connection.
    const PENDING: u8 = 0;
    /// `Authenticated` delivered, and nothing since that ends it.
    const AUTHENTICATED: u8 = 1;
    /// `Unauthenticated` or `Disconnected` delivered since.
    const LOST: u8 = 2;

    /// Record `event`. Call it before the callbacks run, so a callback
    /// calling `connect()` reads the event it is handling.
    pub fn observe(&self, event: &ConnectionEvent) {
        match event {
            ConnectionEvent::Authenticated { .. } => self.0.store(Self::AUTHENTICATED, Ordering::SeqCst),
            ConnectionEvent::Unauthenticated { .. } | ConnectionEvent::Disconnected { .. } => {
                self.0.store(Self::LOST, Ordering::SeqCst)
            }
            _ => {}
        }
    }

    /// The connect that opened this connection succeeded: it counts as
    /// delivered even if `Authenticated` has not been observed yet, so a
    /// second `connect()` right after is refused rather than waiting. Unless
    /// the connection's loss has already been observed.
    pub fn connect_succeeded(&self) {
        let _ = self.0.compare_exchange(
            Self::PENDING,
            Self::AUTHENTICATED,
            Ordering::SeqCst,
            Ordering::SeqCst,
        );
    }

    /// Whether the connection was last handed over as up.
    pub fn is_authenticated(&self) -> bool {
        self.0.load(Ordering::SeqCst) == Self::AUTHENTICATED
    }
}

/// The connection the caller has stored, as [`admit`] reads it.
///
/// `#[non_exhaustive]`: build it with [`new`](Self::new).
#[derive(Clone)]
#[non_exhaustive]
pub struct StoredConnection {
    /// The connection's client.
    pub client: Arc<WebSocketClient>,
    /// What has been delivered of it.
    pub delivered: Delivered,
}

impl StoredConnection {
    /// `client` as stored, with the record of what has been delivered of it.
    pub fn new(client: Arc<WebSocketClient>, delivered: Delivered) -> Self {
        Self { client, delivered }
    }
}

impl fmt::Debug for StoredConnection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StoredConnection")
            .field("state", &self.client.state())
            .field("delivered", &self.delivered)
            .finish_non_exhaustive()
    }
}

/// What [`admit`] decided.
///
/// Exhaustive on purpose, so a caller's `match` covers every outcome; a new
/// variant would be a breaking change.
#[derive(Debug)]
#[must_use = "`Open` carries the claim on the gate: dropping it lets another connect open a connection beside this one"]
pub enum Admission {
    /// Open a new connection. Hold the claim until it is stored, or has
    /// failed.
    Open(ConnectClaim),
    /// The stored connection's automatic reconnect is up; it is kept as is.
    Joined,
}

/// What the stored connection means for a `connect()`.
pub(crate) enum Stored {
    /// None, a closed one, or one whose wait already ended with nothing left
    /// to wait for: open a new connection, under the gate.
    Replace,
    /// It was last handed over as up: 2011.
    Refuse,
    /// Not closed and not handed over as up: it is reconnecting.
    Join(Arc<WebSocketClient>),
}

/// [`Stored`] for `current`, for a `connect()` that gave up waiting on
/// `gave_up`.
pub(crate) fn classify(current: Option<StoredConnection>, gave_up: Option<&Arc<WebSocketClient>>) -> Stored {
    match current {
        None => Stored::Replace,
        Some(c) if c.client.is_closed_sync() => Stored::Replace,
        Some(c) if gave_up.is_some_and(|g| Arc::ptr_eq(g, &c.client)) => Stored::Replace,
        Some(c) if c.delivered.is_authenticated() => Stored::Refuse,
        Some(c) => Stored::Join(c.client),
    }
}

/// The end of a join: `Ok(true)` once the reconnect is up, `Ok(false)` when
/// there is nothing left to wait for — the client closed without the
/// reconnect giving up, or it has no reconnect under way (`ConnectionError`)
/// — so a new connection is opened, otherwise the error: 2010 on
/// `disconnect()`, 3005 when the attempts ran out, `AuthError` when the
/// credentials were rejected.
pub(crate) fn join_ended(result: Result<(), MarketDataError>) -> Result<bool, MarketDataError> {
    match result {
        Ok(()) => Ok(true),
        Err(MarketDataError::ClientClosed | MarketDataError::ConnectionError { .. }) => Ok(false),
        Err(e) => Err(e),
    }
}

/// Decide what a `connect()` does, before anything of the stored connection
/// is replaced: open a new connection under `gate`, or wait here on the
/// automatic reconnect of the stored one.
///
/// `current` reads the stored connection. It may be called more than once,
/// and never across an await, so it may take a lock.
///
/// None stored, or a closed one, opens a new connection. One last handed
/// over as up is refused. Any other is reconnecting, and this waits on its
/// [`wait_connected()`](WebSocketClient::wait_connected). Not the client's
/// `is_active()`: between reconnect attempts the state is `Disconnected`,
/// and a connect let through there would leave the old reconnect loop
/// running beside a new connection (#230).
///
/// The stored connection is read before the gate, so a wait never holds it:
/// calls made at the same moment all wait, instead of one refusing the
/// others (#268).
///
/// What is stored has to be a client that connected, or one whose
/// `connect()` is under way. A wait on a client that never connected ends
/// at once in `ConnectionError`, and the decision is made again on what is
/// stored then; if the store keeps alternating between two such clients,
/// neither closed nor handed over as up, this does not return.
///
/// # Errors
///
/// - `AlreadyConnected` (2011) while the stored connection was last handed
///   over as up, or while another connect that opens a connection holds
///   `gate`.
/// - What the wait on a reconnect ended in: `ConnectionAborted` (2010) on
///   `disconnect()` / `force_close()`, `ReconnectFailed` (3005) when the
///   attempts ran out, `AuthError` when the credentials were rejected.
///
/// A wait that ends in `ClientClosed` or `ConnectionError` is not an error
/// here: nothing is left to wait for, so that client is treated as one to
/// replace and the decision is made again.
pub async fn admit(
    gate: &ConnectGate,
    current: impl FnMut() -> Option<StoredConnection>,
) -> Result<Admission, MarketDataError> {
    admit_with(gate, current, |client| async move { client.wait_connected().await }).await
}

/// [`admit`], waiting on a reconnect through `wait`.
pub(crate) async fn admit_with<W, F>(
    gate: &ConnectGate,
    mut current: impl FnMut() -> Option<StoredConnection>,
    mut wait: W,
) -> Result<Admission, MarketDataError>
where
    W: FnMut(Arc<WebSocketClient>) -> F,
    F: Future<Output = Result<(), MarketDataError>>,
{
    let mut gave_up: Option<Arc<WebSocketClient>> = None;
    loop {
        let mut stored = classify(current(), gave_up.as_ref());
        let mut claim = None;
        if matches!(stored, Stored::Replace) {
            claim = Some(gate.try_claim().ok_or(MarketDataError::AlreadyConnected)?);
            // Again under the claim: a connect that finished since the first
            // look has stored its connection.
            stored = classify(current(), gave_up.as_ref());
        }
        let client = match (stored, claim) {
            (Stored::Join(client), claim) => {
                // Released before the wait.
                drop(claim);
                client
            }
            (Stored::Replace, Some(claim)) => return Ok(Admission::Open(claim)),
            // `Replace` is only read under the claim.
            (Stored::Refuse | Stored::Replace, _) => return Err(MarketDataError::AlreadyConnected),
        };
        if join_ended(wait(Arc::clone(&client)).await)? {
            return Ok(Admission::Joined);
        }
        // Claimed next, as a fresh connect; another caller that got there
        // first is refused as usual.
        gave_up = Some(client);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::websocket::{ConnectionConfig, DisconnectIntent};
    use crate::AuthRequest;
    use std::cell::{Cell, RefCell};
    use std::collections::VecDeque;

    fn client() -> Arc<WebSocketClient> {
        let config = ConnectionConfig::fugle_stock(AuthRequest::with_api_key("test-key"));
        Arc::new(WebSocketClient::new(config))
    }

    async fn closed_client() -> Arc<WebSocketClient> {
        let client = client();
        client.force_close().await.unwrap();
        assert!(client.is_closed_sync());
        client
    }

    /// A stored connection not handed over as up: reconnecting.
    fn reconnecting(client: &Arc<WebSocketClient>) -> StoredConnection {
        StoredConnection::new(Arc::clone(client), Delivered::default())
    }

    /// A stored connection handed over as up.
    fn authenticated(client: &Arc<WebSocketClient>) -> StoredConnection {
        let stored = reconnecting(client);
        stored.delivered.connect_succeeded();
        stored
    }

    fn authenticated_event() -> ConnectionEvent {
        ConnectionEvent::Authenticated { data: serde_json::Value::Null }
    }

    fn disconnected_event() -> ConnectionEvent {
        ConnectionEvent::Disconnected {
            code: None,
            reason: String::new(),
            intent: DisconnectIntent::Server,
            will_reconnect: true,
        }
    }

    fn connection_error() -> MarketDataError {
        MarketDataError::ConnectionError { msg: "Not connected".to_string() }
    }

    /// A `wait` for a decision that must not wait.
    async fn no_wait(_: Arc<WebSocketClient>) -> Result<(), MarketDataError> {
        panic!("waited on a reconnect");
    }

    fn is_2011<T>(result: &Result<T, MarketDataError>) -> bool {
        matches!(result, Err(MarketDataError::AlreadyConnected))
    }

    // --- classify ---

    #[test]
    fn classify_nothing_stored_is_replace() {
        assert!(matches!(classify(None, None), Stored::Replace));
    }

    #[tokio::test]
    async fn classify_closed_is_replace_even_when_authenticated() {
        let closed = closed_client().await;
        assert!(matches!(classify(Some(reconnecting(&closed)), None), Stored::Replace));
        assert!(matches!(classify(Some(authenticated(&closed)), None), Stored::Replace));
    }

    #[test]
    fn classify_gave_up_is_replace_even_when_authenticated() {
        let client = client();
        assert!(matches!(classify(Some(reconnecting(&client)), Some(&client)), Stored::Replace));
        assert!(matches!(classify(Some(authenticated(&client)), Some(&client)), Stored::Replace));
    }

    #[test]
    fn classify_authenticated_is_refuse() {
        let (client, other) = (client(), client());
        assert!(matches!(classify(Some(authenticated(&client)), None), Stored::Refuse));
        // Giving up on another client does not change it.
        assert!(matches!(classify(Some(authenticated(&client)), Some(&other)), Stored::Refuse));
    }

    #[test]
    fn classify_otherwise_is_join_of_the_stored_client() {
        let (client, other) = (client(), client());
        for gave_up in [None, Some(&other)] {
            match classify(Some(reconnecting(&client)), gave_up) {
                Stored::Join(joined) => assert!(Arc::ptr_eq(&joined, &client)),
                _ => panic!("expected Join"),
            }
        }
    }

    // --- Delivered ---

    #[test]
    fn delivered_follows_the_events_observed() {
        let delivered = Delivered::default();
        assert!(!delivered.is_authenticated());

        for ignored in [ConnectionEvent::Connecting, ConnectionEvent::Connected] {
            delivered.observe(&ignored);
            assert!(!delivered.is_authenticated());
        }
        delivered.observe(&authenticated_event());
        assert!(delivered.is_authenticated());
        for ignored in [
            ConnectionEvent::Connecting,
            ConnectionEvent::Reconnecting { attempt: 1 },
            ConnectionEvent::ReconnectFailed { attempts: 1 },
        ] {
            delivered.observe(&ignored);
            assert!(delivered.is_authenticated());
        }

        delivered.observe(&disconnected_event());
        assert!(!delivered.is_authenticated());
        delivered.observe(&authenticated_event());
        assert!(delivered.is_authenticated());

        delivered.observe(&ConnectionEvent::Unauthenticated {
            message: "rejected".to_string(),
            data: serde_json::Value::Null,
        });
        assert!(!delivered.is_authenticated());
    }

    #[test]
    fn delivered_connect_succeeded_counts_unless_the_loss_was_observed() {
        let pending = Delivered::default();
        pending.connect_succeeded();
        assert!(pending.is_authenticated());
        // A loss observed afterwards still ends it.
        pending.observe(&disconnected_event());
        assert!(!pending.is_authenticated());

        let lost = Delivered::default();
        lost.observe(&disconnected_event());
        lost.connect_succeeded();
        assert!(!lost.is_authenticated());
    }

    #[test]
    fn delivered_clones_share_the_record() {
        let delivered = Delivered::default();
        let reader = delivered.clone();
        reader.observe(&authenticated_event());
        assert!(delivered.is_authenticated());
    }

    // --- ConnectGate ---

    #[test]
    fn a_cloned_gate_shares_the_claim() {
        let gate = ConnectGate::default();
        let clone = gate.clone();
        let claim = clone.try_claim().unwrap();
        assert!(gate.is_busy());
        assert!(gate.try_claim().is_none());
        drop(claim);
        assert!(!clone.is_busy());
        let _claim = gate.try_claim().unwrap();
        assert!(clone.is_busy());
    }

    #[test]
    fn debug_of_a_claim_and_an_admission() {
        let gate = ConnectGate::default();
        let claim = gate.try_claim().unwrap();
        assert_eq!(format!("{claim:?}"), "ConnectClaim { .. }");
        assert_eq!(format!("{:?}", Admission::Open(claim)), "Open(ConnectClaim { .. })");
        assert_eq!(format!("{:?}", Admission::Joined), "Joined");
    }

    // --- join_ended ---

    #[test]
    fn join_ended_sorts_the_end_of_a_wait() {
        assert!(matches!(join_ended(Ok(())), Ok(true)));
        assert!(matches!(join_ended(Err(MarketDataError::ClientClosed)), Ok(false)));
        assert!(matches!(join_ended(Err(connection_error())), Ok(false)));
        assert!(matches!(
            join_ended(Err(MarketDataError::ConnectionAborted)),
            Err(MarketDataError::ConnectionAborted)
        ));
    }

    // --- admit ---

    #[tokio::test]
    async fn nothing_stored_opens_and_holds_the_gate_until_the_claim_drops() {
        let gate = ConnectGate::default();
        let result = admit_with(&gate, || None, no_wait).await;
        let Ok(Admission::Open(claim)) = result else { panic!("expected Open") };
        assert!(gate.is_busy());
        drop(claim);
        assert!(!gate.is_busy());
    }

    #[tokio::test]
    async fn authenticated_is_refused_with_2011_without_taking_the_gate() {
        let gate = ConnectGate::default();
        let client = client();
        let busy_at_read = Cell::new(false);
        let result = admit_with(
            &gate,
            || {
                busy_at_read.set(busy_at_read.get() || gate.is_busy());
                Some(authenticated(&client))
            },
            no_wait,
        )
        .await;
        assert!(is_2011(&result));
        assert!(!busy_at_read.get());
        assert!(!gate.is_busy());
    }

    #[tokio::test]
    async fn not_authenticated_joins_without_taking_the_gate() {
        let gate = ConnectGate::default();
        let client = client();
        let waited = RefCell::new(Vec::new());
        let result = admit_with(
            &gate,
            || Some(reconnecting(&client)),
            |joined| {
                waited.borrow_mut().push((joined, gate.is_busy()));
                async { Ok(()) }
            },
        )
        .await;
        assert!(matches!(result, Ok(Admission::Joined)));
        let waited = waited.into_inner();
        assert_eq!(waited.len(), 1);
        assert!(Arc::ptr_eq(&waited[0].0, &client));
        assert!(!waited[0].1, "held the gate while waiting");
        assert!(!gate.is_busy());
    }

    #[tokio::test]
    async fn a_join_is_not_refused_while_another_connect_holds_the_gate() {
        let gate = ConnectGate::default();
        let _held = gate.try_claim().unwrap();
        let client = client();
        let result = admit_with(&gate, || Some(reconnecting(&client)), |_| async { Ok(()) }).await;
        assert!(matches!(result, Ok(Admission::Joined)));
    }

    #[tokio::test]
    async fn replace_is_refused_with_2011_while_the_gate_is_held() {
        let gate = ConnectGate::default();
        let held = gate.try_claim().unwrap();
        let reads = Cell::new(0);
        let result = admit_with(
            &gate,
            || {
                reads.set(reads.get() + 1);
                None
            },
            no_wait,
        )
        .await;
        assert!(is_2011(&result));
        // Refused at the gate, before the second look.
        assert_eq!(reads.get(), 1);
        // The other connect's claim is untouched.
        assert!(gate.is_busy());
        drop(held);
        assert!(!gate.is_busy());
    }

    /// The gap PR #270 left: the first look says `Replace`, the look under
    /// the claim finds a reconnecting connection.
    #[tokio::test]
    async fn a_join_found_under_the_claim_releases_the_gate_before_waiting() {
        let gate = ConnectGate::default();
        let client = client();
        let busy_at_read = RefCell::new(Vec::new());
        let busy_at_wait = Cell::new(None);
        let result = admit_with(
            &gate,
            || {
                busy_at_read.borrow_mut().push(gate.is_busy());
                (busy_at_read.borrow().len() > 1).then(|| reconnecting(&client))
            },
            |_| {
                busy_at_wait.set(Some(gate.is_busy()));
                // While this call waits, another connect can take the gate.
                let other = gate.try_claim();
                async move {
                    assert!(other.is_some());
                    Ok(())
                }
            },
        )
        .await;
        assert!(matches!(result, Ok(Admission::Joined)));
        assert_eq!(busy_at_read.into_inner(), [false, true]);
        assert_eq!(busy_at_wait.get(), Some(false), "held the gate while waiting");
        assert!(!gate.is_busy());
    }

    #[tokio::test]
    async fn authenticated_found_under_the_claim_is_refused_and_releases_the_gate() {
        let gate = ConnectGate::default();
        let client = client();
        let reads = Cell::new(0);
        let result = admit_with(
            &gate,
            || {
                reads.set(reads.get() + 1);
                (reads.get() > 1).then(|| authenticated(&client))
            },
            no_wait,
        )
        .await;
        assert!(is_2011(&result));
        assert_eq!(reads.get(), 2);
        assert!(!gate.is_busy());
    }

    #[tokio::test]
    async fn a_join_with_nothing_left_to_wait_for_opens_in_place_of_that_client() {
        for end in [MarketDataError::ClientClosed, connection_error()] {
            let gate = ConnectGate::default();
            let client = client();
            let mut ends = VecDeque::from([Err(end)]);
            let result = admit_with(
                &gate,
                // Still stored, and not closed as far as `is_closed_sync` tells.
                || Some(reconnecting(&client)),
                |_| {
                    let end = ends.pop_front().expect("waited on the client it gave up on");
                    async move { end }
                },
            )
            .await;
            let Ok(Admission::Open(claim)) = result else { panic!("expected Open") };
            assert!(gate.is_busy());
            drop(claim);
            assert!(!gate.is_busy());
        }
    }

    #[tokio::test]
    async fn a_join_with_nothing_left_is_refused_when_another_connect_holds_the_gate() {
        let gate = ConnectGate::default();
        let client = client();
        let held = RefCell::new(None);
        let result = admit_with(
            &gate,
            || Some(reconnecting(&client)),
            |_| {
                // Another connect gets there first.
                *held.borrow_mut() = gate.try_claim();
                async { Err(MarketDataError::ClientClosed) }
            },
        )
        .await;
        assert!(is_2011(&result));
        assert!(gate.is_busy());
    }

    #[tokio::test]
    async fn a_join_with_nothing_left_decides_again_on_a_replaced_connection() {
        // Replaced by a connection that is itself reconnecting: joined.
        let gate = ConnectGate::default();
        let (first, second) = (client(), client());
        let stored = RefCell::new(reconnecting(&first));
        let waited = RefCell::new(Vec::new());
        let result = admit_with(
            &gate,
            || Some(stored.borrow().clone()),
            |joined| {
                let was_first = Arc::ptr_eq(&joined, &first);
                waited.borrow_mut().push(joined);
                if was_first {
                    *stored.borrow_mut() = reconnecting(&second);
                }
                async move { if was_first { Err(connection_error()) } else { Ok(()) } }
            },
        )
        .await;
        assert!(matches!(result, Ok(Admission::Joined)));
        let waited = waited.into_inner();
        assert_eq!(waited.len(), 2);
        assert!(Arc::ptr_eq(&waited[1], &second));
        assert!(!gate.is_busy());

        // Replaced by one handed over as up: 2011.
        let stored = RefCell::new(reconnecting(&first));
        let result = admit_with(
            &gate,
            || Some(stored.borrow().clone()),
            |_| {
                *stored.borrow_mut() = authenticated(&second);
                async { Err(MarketDataError::ClientClosed) }
            },
        )
        .await;
        assert!(is_2011(&result));
        assert!(!gate.is_busy());
    }

    #[tokio::test]
    async fn the_errors_of_a_join_are_returned_as_they_are() {
        let ends = [
            MarketDataError::ConnectionAborted,
            MarketDataError::ReconnectFailed { attempts: 3 },
            MarketDataError::AuthError { msg: "rejected".to_string(), http: None },
        ];
        for end in ends {
            let expected = format!("{end:?}");
            let gate = ConnectGate::default();
            let client = client();
            let mut ends = VecDeque::from([Err(end)]);
            let result = admit_with(
                &gate,
                || Some(reconnecting(&client)),
                |_| {
                    let end = ends.pop_front().expect("waited again after an error");
                    async move { end }
                },
            )
            .await;
            let Err(error) = result else { panic!("expected {expected}") };
            assert_eq!(format!("{error:?}"), expected);
            assert!(!gate.is_busy());
        }
    }

    // --- admit, on the client's own wait_connected() ---

    #[tokio::test]
    async fn admit_opens_in_place_of_a_client_with_no_reconnect_under_way() {
        // Never connected: `wait_connected()` ends in `ConnectionError`.
        let gate = ConnectGate::default();
        let client = client();
        let result = admit(&gate, || Some(reconnecting(&client))).await;
        assert!(matches!(result, Ok(Admission::Open(_))));
    }

    #[tokio::test]
    async fn admit_opens_in_place_of_a_closed_client() {
        let gate = ConnectGate::default();
        let closed = closed_client().await;
        let result = admit(&gate, || Some(authenticated(&closed))).await;
        assert!(matches!(result, Ok(Admission::Open(_))));
    }

    #[test]
    fn admit_is_send_for_a_send_reader() {
        fn assert_send<T: Send>(_: &T) {}
        let gate = ConnectGate::default();
        let stored = std::sync::Mutex::new(None::<StoredConnection>);
        let admission = admit(&gate, || stored.lock().unwrap().clone());
        assert_send(&admission);
    }
}
