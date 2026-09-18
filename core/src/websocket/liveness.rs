//! Liveness state shared by the async and sync clients.
//!
//! [`Liveness`] decides, from the time of the last inbound frame, when to
//! send a probe and when to declare the connection dead (see
//! [`HealthCheckConfig`]). [`LatencyWaiters`] matches the server's `pong`
//! replies to pending `measure_latency()` calls. Both are clock-agnostic so
//! the async client can drive them with tokio's clock and the sync client
//! with std's.

use crate::models::{WebSocketMessage, WebSocketRequest};
use crate::websocket::protocol::frame_request;
use crate::websocket::HealthCheckConfig;
use crate::MarketDataError;
use std::ops::{Add, Sub};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Prefix of the `state` of every ping the SDK sends on its own. The pongs
/// that echo it are the SDK's and are not delivered to the caller.
const SDK_STATE_PREFIX: &str = "fugle-sdk:";

/// `state` of the health check's probe.
const PROBE_STATE: &str = "fugle-sdk:probe";

/// Default wait for `measure_latency()`'s pong, in milliseconds.
pub const DEFAULT_LATENCY_TIMEOUT_MS: u64 = 5_000;

/// `measure_latency()`'s timeout, defaulted. A zero timeout could never
/// succeed and is `InvalidParameter`.
pub(crate) fn latency_timeout_or_default(
    timeout: Option<Duration>,
) -> Result<Duration, MarketDataError> {
    match timeout {
        None => Ok(Duration::from_millis(DEFAULT_LATENCY_TIMEOUT_MS)),
        Some(timeout) if timeout.is_zero() => Err(MarketDataError::InvalidParameter {
            name: "timeout".to_string(),
            reason: "must be greater than 0".to_string(),
        }),
        Some(timeout) => Ok(timeout),
    }
}

/// The frame of a health-check probe.
pub(crate) fn probe_frame() -> String {
    frame_request(&WebSocketRequest::ping(Some(PROBE_STATE.to_string())))
        .expect("a ping request always serializes")
}

/// What the read site does next.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum LivenessAction<I> {
    /// Read until this instant; call [`Liveness::poll`] again if it passes.
    Wait(I),
    /// Send a probe now (it is already accounted as sent), then poll again.
    SendProbe,
    /// Declare the connection dead. Carries the detection window —
    /// `heartbeat_timeout`, or `idle_probe_after + probe_timeout` — as
    /// reported in `HeartbeatTimeout`.
    Dead(Duration),
}

#[derive(Debug, Clone, Copy)]
enum Plan {
    Passive { timeout: Duration },
    Probe { idle: Duration, timeout: Duration },
}

/// Silence tracking for one connection. Any inbound frame resets it.
#[derive(Debug)]
pub(crate) struct Liveness<I> {
    plan: Plan,
    last_inbound: I,
    probe_sent_at: Option<I>,
}

impl<I> Liveness<I>
where
    I: Copy + Ord + Add<Duration, Output = I> + Sub<I, Output = Duration>,
{
    /// `None` when liveness detection is disabled.
    pub(crate) fn new(config: &HealthCheckConfig, now: I) -> Option<Self> {
        if !config.enabled {
            return None;
        }
        let plan = if config.probe_enabled {
            Plan::Probe {
                idle: config.idle_probe_after_or_default(),
                timeout: config.probe_timeout_or_default(),
            }
        } else {
            Plan::Passive { timeout: config.heartbeat_timeout }
        };
        Some(Self { plan, last_inbound: now, probe_sent_at: None })
    }

    /// Any inbound frame proves the connection alive.
    pub(crate) fn on_inbound(&mut self, now: I) {
        self.last_inbound = now;
        self.probe_sent_at = None;
    }

    pub(crate) fn poll(&mut self, now: I) -> LivenessAction<I> {
        let deadline = match self.plan {
            Plan::Passive { timeout } => self.last_inbound + timeout,
            Plan::Probe { idle, timeout } => match self.probe_sent_at {
                Some(sent) => sent + timeout,
                None => {
                    let probe_at = self.last_inbound + idle;
                    if now < probe_at {
                        return LivenessAction::Wait(probe_at);
                    }
                    self.probe_sent_at = Some(now);
                    return LivenessAction::SendProbe;
                }
            },
        };
        if now < deadline {
            LivenessAction::Wait(deadline)
        } else {
            LivenessAction::Dead(self.window())
        }
    }

    /// The detection window: how much silence declares the connection dead.
    pub(crate) fn window(&self) -> Duration {
        match self.plan {
            Plan::Passive { timeout } => timeout,
            Plan::Probe { idle, timeout } => idle + timeout,
        }
    }

    /// Whether this is probe mode.
    pub(crate) fn probes(&self) -> bool {
        matches!(self.plan, Plan::Probe { .. })
    }

    /// When a probe is out, the instant it goes unanswered.
    pub(crate) fn probe_deadline(&self) -> Option<I> {
        match (self.plan, self.probe_sent_at) {
            (Plan::Probe { timeout, .. }, Some(sent)) => Some(sent + timeout),
            _ => None,
        }
    }
}

type Respond = Box<dyn FnOnce(Instant) + Send>;

/// Pending `measure_latency()` calls, keyed by the `state` of their ping.
#[derive(Default)]
pub(crate) struct LatencyWaiters {
    next_id: AtomicU64,
    waiting: Mutex<Vec<(String, Respond)>>,
}

impl LatencyWaiters {
    /// Register a waiter; `respond` gets the instant its pong arrived.
    /// Returns the `state` to send in the ping.
    pub(crate) fn register(&self, respond: impl FnOnce(Instant) + Send + 'static) -> String {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let state = format!("{SDK_STATE_PREFIX}latency:{id}");
        self.lock().push((state.clone(), Box::new(respond)));
        state
    }

    /// Forget a waiter that gave up.
    pub(crate) fn cancel(&self, state: &str) {
        self.lock().retain(|(s, _)| s != state);
    }

    /// Drop every waiter: the connection they were sent on is gone, so each
    /// sees its channel closed.
    pub(crate) fn fail_all(&self) {
        self.lock().clear();
    }

    /// Take `msg` if it is the pong of a ping the SDK sent — a probe or a
    /// `measure_latency()` — answering its waiter. Returns true when `msg`
    /// is the SDK's and must not reach the caller; any other pong (from the
    /// caller's own `ping()`) is theirs.
    pub(crate) fn intercept_pong(&self, msg: &WebSocketMessage, arrived: Instant) -> bool {
        if !msg.is_pong() {
            return false;
        }
        let Some(state) = msg
            .data
            .as_ref()
            .and_then(|data| data.get("state"))
            .and_then(|state| state.as_str())
        else {
            return false;
        };
        if !state.starts_with(SDK_STATE_PREFIX) {
            return false;
        }
        let respond = {
            let mut waiting = self.lock();
            waiting
                .iter()
                .position(|(s, _)| s == state)
                .map(|i| waiting.swap_remove(i).1)
        };
        if let Some(respond) = respond {
            respond(arrived);
        }
        true
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<(String, Respond)>> {
        self.waiting.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// Clears a [`LatencyWaiters`] when the read loop of its connection ends,
/// however it ends — the async one can be aborted at any `.await`.
pub(crate) struct FailWaitersOnDrop<'a>(pub(crate) &'a LatencyWaiters);

impl Drop for FailWaitersOnDrop<'_> {
    fn drop(&mut self) {
        self.0.fail_all();
    }
}

/// The ping frame of a `measure_latency()` call.
pub(crate) fn latency_frame(state: String) -> Result<String, MarketDataError> {
    frame_request(&WebSocketRequest::ping(Some(state)))
}

/// The error of a `measure_latency()` that got no pong in time.
pub(crate) fn latency_timeout() -> MarketDataError {
    MarketDataError::TimeoutError { operation: "measure_latency".to_string() }
}

/// The error of a `measure_latency()` whose connection closed first.
pub(crate) fn latency_connection_lost() -> MarketDataError {
    MarketDataError::ConnectionError {
        msg: "Connection closed before the pong arrived".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secs(s: u64) -> Duration {
        Duration::from_secs(s)
    }

    fn passive(timeout: u64) -> HealthCheckConfig {
        HealthCheckConfig::with_timeout(secs(timeout)).unwrap()
    }

    fn probe(idle: u64, timeout: u64) -> HealthCheckConfig {
        HealthCheckConfig::with_probe(secs(idle), secs(timeout)).unwrap()
    }

    #[test]
    fn disabled_has_no_liveness() {
        assert!(Liveness::new(&HealthCheckConfig::disabled(), Instant::now()).is_none());
    }

    #[test]
    fn passive_dies_at_heartbeat_timeout() {
        let t0 = Instant::now();
        let mut l = Liveness::new(&passive(35), t0).unwrap();
        assert_eq!(l.poll(t0), LivenessAction::Wait(t0 + secs(35)));
        assert_eq!(l.poll(t0 + secs(35)), LivenessAction::Dead(secs(35)));
    }

    #[test]
    fn passive_never_probes() {
        let t0 = Instant::now();
        let mut l = Liveness::new(&passive(35), t0).unwrap();
        assert_eq!(l.poll(t0 + secs(34)), LivenessAction::Wait(t0 + secs(35)));
        assert_eq!(l.probe_deadline(), None);
    }

    #[test]
    fn probe_mode_ignores_heartbeat_timeout() {
        let t0 = Instant::now();
        let mut config = probe(30, 5);
        config.heartbeat_timeout = secs(5);
        let mut l = Liveness::new(&config, t0).unwrap();
        assert_eq!(l.poll(t0 + secs(10)), LivenessAction::Wait(t0 + secs(30)));
    }

    #[test]
    fn probe_then_dead_without_inbound() {
        let t0 = Instant::now();
        let mut l = Liveness::new(&probe(30, 5), t0).unwrap();
        assert_eq!(l.poll(t0), LivenessAction::Wait(t0 + secs(30)));
        assert_eq!(l.poll(t0 + secs(30)), LivenessAction::SendProbe);
        assert_eq!(l.probe_deadline(), Some(t0 + secs(35)));
        // Only one probe per silence.
        assert_eq!(l.poll(t0 + secs(31)), LivenessAction::Wait(t0 + secs(35)));
        assert_eq!(l.poll(t0 + secs(35)), LivenessAction::Dead(secs(35)));
    }

    #[test]
    fn inbound_after_probe_resets() {
        let t0 = Instant::now();
        let mut l = Liveness::new(&probe(30, 5), t0).unwrap();
        assert_eq!(l.poll(t0 + secs(30)), LivenessAction::SendProbe);
        l.on_inbound(t0 + secs(32));
        assert_eq!(l.probe_deadline(), None);
        assert_eq!(l.poll(t0 + secs(40)), LivenessAction::Wait(t0 + secs(62)));
    }

    #[test]
    fn steady_inbound_never_probes() {
        let t0 = Instant::now();
        let mut l = Liveness::new(&probe(5, 1), t0).unwrap();
        for s in 1..100 {
            l.on_inbound(t0 + secs(s));
            assert!(matches!(l.poll(t0 + secs(s)), LivenessAction::Wait(_)));
        }
    }

    fn pong(state: serde_json::Value) -> WebSocketMessage {
        serde_json::from_value(serde_json::json!({"event": "pong", "data": {"time": 1, "state": state}}))
            .unwrap()
    }

    #[test]
    fn probe_pong_is_intercepted() {
        let waiters = LatencyWaiters::default();
        assert!(waiters.intercept_pong(&pong(PROBE_STATE.into()), Instant::now()));
    }

    #[test]
    fn caller_pong_is_not_intercepted() {
        let waiters = LatencyWaiters::default();
        assert!(!waiters.intercept_pong(&pong("mine".into()), Instant::now()));
        assert!(!waiters.intercept_pong(&pong(serde_json::json!({"a": 1})), Instant::now()));
    }

    #[test]
    fn latency_pong_answers_its_waiter() {
        let waiters = LatencyWaiters::default();
        let (tx, rx) = std::sync::mpsc::channel();
        let state = waiters.register(move |at| tx.send(at).unwrap());
        let other = waiters.register(|_| panic!("wrong waiter answered"));
        let at = Instant::now();
        assert!(waiters.intercept_pong(&pong(state.into()), at));
        assert_eq!(rx.try_recv().unwrap(), at);
        waiters.cancel(&other);
    }

    #[test]
    fn fail_all_closes_waiters() {
        let waiters = LatencyWaiters::default();
        let (tx, rx) = std::sync::mpsc::channel::<Instant>();
        waiters.register(move |at| tx.send(at).unwrap());
        drop(FailWaitersOnDrop(&waiters));
        assert!(rx.recv().is_err());
    }
}
