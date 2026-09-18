//! WebSocket client wrapper for JavaScript
//!
//! This module provides JavaScript-facing WebSocket clients using ThreadsafeFunction
//! for cross-thread callback invocation without blocking the Node.js event loop.
//!
//! Architecture:
//! - WebSocket connection runs in a dedicated background thread with its own tokio runtime
//! - Commands (connect, subscribe, disconnect) are sent via crossbeam channel
//! - Core connection events are delivered to listeners through the connection's
//!   [`EventSink`], with the argument shapes of `@fugle/marketdata` 1.x (#23)

use napi::bindgen_prelude::{Function, FunctionRef, JsValuesTupleIntoVec, PromiseRaw, ToNapiValue, Unknown};
use marketdata_core::{error_code, ErrorInfo, ErrorKind};
use napi::JsValue;
use napi::Env;
use napi::threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode};
use napi::{sys, Status};

/// An event listener, called on the JS thread with the event's own arguments
/// and no leading `err`, matching the legacy `@fugle/marketdata` 1.x
/// EventEmitter shape (#23):
/// ```js
/// stock.on('message', (data) => console.log(JSON.parse(data)));
/// stock.on('authenticated', (data) => console.log(data.message));
/// ```
///
/// A plain reference, not a threadsafe function: registering a listener does
/// not keep the Node event loop alive, matching an EventEmitter listener. An
/// open connection does, via the [`KeepAlive`] handle held for the
/// connection's lifetime (#30).
type Listener = FunctionRef<EventArgs, Unknown<'static>>;

/// Arguments an event listener is called with, built on the JS thread.
pub enum EventArgs {
    /// No arguments at all (`connect`), not a single `undefined`.
    None,
    /// A string (`message`: the frame verbatim).
    Text(String),
    /// A plain object; JSON `null` becomes `undefined`, as destructuring the
    /// frame's absent `data` did in 1.x.
    Json(serde_json::Value),
    /// An `Error` with the unified error fields (see `errors.rs`).
    Error(ErrorInfo),
}

impl JsValuesTupleIntoVec for EventArgs {
    // `env` comes from napi on the JS thread, as in napi-rs's own impls.
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn into_vec(self, env: sys::napi_env) -> napi::Result<Vec<sys::napi_value>> {
        let value = match self {
            EventArgs::None => return Ok(Vec::new()),
            EventArgs::Text(text) => unsafe { String::to_napi_value(env, text)? },
            EventArgs::Json(value) => json_to_napi(env, value)?,
            EventArgs::Error(info) => crate::errors::error_value(env, &info)?,
        };
        Ok(vec![value])
    }
}

fn json_to_napi(env: sys::napi_env, value: serde_json::Value) -> napi::Result<sys::napi_value> {
    unsafe {
        match value {
            serde_json::Value::Null => <()>::to_napi_value(env, ()),
            value => serde_json::Value::to_napi_value(env, value),
        }
    }
}

use napi_derive::napi;
use std::panic::AssertUnwindSafe;
use std::sync::{Arc, Mutex, atomic::{AtomicBool, AtomicU8, Ordering}};
use std::thread;
use std::time::Duration;

/// Keeps the Node event loop alive while a connection is open (#30).
///
/// Wraps a strong (ref'd) threadsafe function that is never called; the loop
/// is released when the last clone drops. The worker thread and the event
/// thread each hold a clone (inside their [`EventSink`]), and so does every
/// event the sink queues — dropped on the JS thread only after that event's
/// listener has returned. The final event (`disconnect`, a connect `error`, …)
/// therefore always runs before the process is allowed to exit, whichever
/// event turns out to be last and whether or not any listener is registered.
///
/// Kept apart from [`DispatchTsfn`]: a queued event holding the last clone of
/// the very threadsafe function it is queued on would release that function
/// from its own finalizer when Node discards the queue at teardown.
type KeepAlive = Arc<ThreadsafeFunction<(), (), (), Status, false>>;

/// The one queue a connection's events go through (#62): a weak threadsafe
/// function around a no-op, whose per-call completion runs the listener.
/// Node-API runs a single threadsafe function's calls in the order they were
/// queued, so listeners run in the order the events were emitted — which a
/// threadsafe function per listener did not guarantee.
type DispatchTsfn = ThreadsafeFunction<(), (), (), Status, false, true>;

/// Where a connection emits its events (#62).
///
/// Every event, and the `connect()` settlement that follows one, is queued on
/// one [`DispatchTsfn`] and runs on the JS thread in emission order.
///
/// The listener is looked up when the event runs, not when it is queued (see
/// [`call_listener`]): a listener registered after the event was queued still
/// receives it, and one that `on()` replaces while events are queued receives
/// none of them — they all go to its replacement.
///
/// `message` is the exception: a frame is only queued if a `message` listener
/// is registered when it arrives (see [`Self::emit_message`]).
///
/// Independent of where events come from: one reader per connection emits
/// core's ordered stream of messages and events here (#68).
#[derive(Clone)]
struct EventSink {
    dispatch: Arc<DispatchTsfn>,
    listeners: Arc<Listeners>,
    keep_alive: KeepAlive,
    in_flight: Arc<InFlight>,
}

/// `message` frames queued for the JS thread whose listener has not run yet
/// (#46). The stream reader waits while `limit` of them are, so a slow
/// listener leaves the backlog in core's queue, where `messageOverflow`
/// applies, instead of growing without bound in the dispatch queue.
///
/// Events wait behind a held-up reader too. Core keeps up to `event_buffer`
/// (1024) of them apart from messages and drops the rest, so a listener that
/// blocks for long enough loses events as well as messages.
struct InFlight {
    count: Mutex<usize>,
    room: std::sync::Condvar,
    /// `None`: no limit (`messageOverflow: 'unbounded'`).
    limit: Option<usize>,
}

impl InFlight {
    fn new(limit: Option<usize>) -> Self {
        Self {
            count: Mutex::new(0),
            room: std::sync::Condvar::new(),
            limit,
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, usize> {
        self.count.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Count a message as in flight until the returned permit is dropped.
    fn acquire(self: &Arc<Self>) -> InFlightPermit {
        *self.lock() += 1;
        InFlightPermit(Arc::clone(self))
    }

    fn release(&self) {
        let mut count = self.lock();
        *count = count.saturating_sub(1);
        drop(count);
        self.room.notify_all();
    }

    /// Wait until fewer than `limit` messages are in flight. Re-checks every
    /// 50 ms, so a missed wake-up only delays it.
    fn wait_for_room(&self) {
        let Some(limit) = self.limit else { return };
        let mut count = self.lock();
        while *count >= limit {
            count = self
                .room
                .wait_timeout(count, Duration::from_millis(50))
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .0;
        }
    }
}

/// One in-flight message. Dropping it releases the slot, so a call napi
/// discards without running (its closure is dropped with the permit) cannot
/// leave the reader waiting for room forever.
struct InFlightPermit(Arc<InFlight>);

impl Drop for InFlightPermit {
    fn drop(&mut self) {
        self.0.release();
    }
}

impl EventSink {
    /// Create a connection's sink. Must run on the JS thread.
    fn new(env: &Env, listeners: Arc<Listeners>, in_flight_limit: Option<usize>) -> napi::Result<Self> {
        let dispatch = env
            .create_function_from_closure::<(), (), _>("fugleWsDispatch", |_| Ok(()))?
            .build_threadsafe_function::<()>()
            .callee_handled::<false>()
            .weak::<true>()
            .build()?;
        Ok(Self {
            dispatch: Arc::new(dispatch),
            listeners,
            keep_alive: loop_keep_alive(env)?,
            in_flight: Arc::new(InFlight::new(in_flight_limit)),
        })
    }

    /// Queue `event` for its listener, if any is registered when it runs.
    fn emit(&self, event: &'static str, args: EventArgs) {
        self.emit_then(event, args, || {});
    }

    /// Queue a `message` frame, or skip it if no `message` listener is
    /// registered yet: under a flood, queuing frames nobody listens to costs a
    /// JS-thread call each (#62). A frame skipped this way is not replayed to a
    /// listener registered later, as an EventEmitter drops what it emits with
    /// no listener. Frames already queued still go to the current listener.
    ///
    /// A queued frame counts as in flight until its listener has run (see
    /// [`InFlight`]).
    fn emit_message(&self, frame: String) {
        if self.listeners.has_message.load(Ordering::SeqCst) {
            let permit = self.in_flight.acquire();
            self.emit_then("message", EventArgs::Text(frame), move || drop(permit));
        }
    }

    /// Wait until another `message` frame may be queued (see [`InFlight`]).
    fn wait_for_room(&self) {
        self.in_flight.wait_for_room();
    }

    /// [`Self::emit`], running `then` on the JS thread once the listener has
    /// returned (or right away when there is none) — or immediately, on this
    /// thread, if the event cannot be queued.
    ///
    /// The call carries a `keep_alive` clone, dropped after `then` (see
    /// [`KeepAlive`]). An exception the listener throws does not escape: it is
    /// reported as described in [`call_listener`], and `then` still runs.
    fn emit_then(&self, event: &'static str, args: EventArgs, then: impl FnOnce() + Send + 'static) {
        fn run_then<F: FnOnce()>(then: &Mutex<Option<F>>) {
            if let Some(then) = then.lock().ok().and_then(|mut guard| guard.take()) {
                then();
            }
        }

        let then = Arc::new(Mutex::new(Some(then)));
        let then_after_call = Arc::clone(&then);
        let listeners = Arc::clone(&self.listeners);
        let keep_alive = Arc::clone(&self.keep_alive);
        let status = self.dispatch.call_with_return_value(
            (),
            ThreadsafeFunctionCallMode::NonBlocking,
            move |_, env| {
                call_listener(&listeners, &env, event, args);
                run_then(&then_after_call);
                drop(keep_alive);
                Ok(())
            },
        );
        if status != Status::Ok {
            run_then(&then);
        }
    }
}

/// Call `event`'s listener, if one is registered. On the JS thread.
///
/// A listener that throws, or returns a thenable that rejects, neither
/// crashes the process nor stops later events (#83): see
/// [`report_listener_failure`].
fn call_listener(listeners: &Arc<Listeners>, env: &Env, event: &'static str, args: EventArgs) {
    // Looked up now, on delivery: whatever `on()` registered last, even after
    // this event was queued, gets it. Released before the call, so the
    // listener can register listeners. A panic while holding the lock must
    // not silence the events that report it (#25).
    let listener = listeners.get(event);
    let Some(listener) = listener else { return };
    let returned = listener
        .borrow_back(env)
        .and_then(|listener| listener.call(args));
    match returned {
        Ok(returned) => on_rejection(listeners, env, event, returned.raw()),
        Err(err) => {
            // The thrown value itself (napi-rs keeps a reference to it).
            if let Ok(thrown) = unsafe { napi::Error::to_napi_value(env.raw(), err) } {
                report_listener_failure(listeners, env, event, "threw", thrown);
            }
        }
    }
}

/// If `returned` is a thenable, report its rejection like a thrown exception
/// (#83). Nothing waits for it to settle.
fn on_rejection(listeners: &Arc<Listeners>, env: &Env, event: &'static str, returned: sys::napi_value) {
    let raw = env.raw();
    if !matches!(type_of(raw, returned), Some(sys::ValueType::napi_object | sys::ValueType::napi_function)) {
        return;
    }
    let Some(then) = named_property(raw, returned, c"then") else { return };
    if type_of(raw, then) != Some(sys::ValueType::napi_function) {
        return;
    }
    let reporter = Arc::clone(listeners);
    let on_rejected = env.create_function_from_closure::<(), (), _>("fugleListenerRejected", move |ctx| {
        let reason = ctx.get::<Unknown>(0).map(|reason| reason.raw());
        let reason = match reason {
            Ok(reason) => reason,
            Err(_) => undefined(ctx.env.raw()),
        };
        report_listener_failure(&reporter, ctx.env, event, "rejected", reason);
        Ok(())
    });
    let Ok(on_rejected) = on_rejected else { return };
    let args = [undefined(raw), on_rejected.raw()];
    let mut ignored = std::ptr::null_mut();
    let status = unsafe {
        sys::napi_call_function(raw, returned, then, args.len(), args.as_ptr(), &mut ignored)
    };
    if status != sys::Status::napi_ok {
        clear_exception(raw);
    }
}

/// Let the user know `event`'s listener failed (`how`: `threw` / `rejected`)
/// with `cause`, without crashing and without recursing (#83).
///
/// A failing `error` listener is only printed. Any other failure becomes an
/// `error` event (code [`error_code::CALLBACK_FAILED`], `cause`, `event`,
/// `count`) — throttled like `messagesDropped`: the first at once, later ones
/// at most once per second, counting the failures since the previous report.
/// With no `error` listener, or one that fails in turn, it is printed with
/// `console.error` instead.
fn report_listener_failure(
    listeners: &Arc<Listeners>,
    env: &Env,
    event: &'static str,
    how: &str,
    cause: sys::napi_value,
) {
    let raw = env.raw();
    if event == "error" {
        print_error(raw, &format!("'error' listener {how}:"), &[cause]);
        return;
    }
    let count = listeners
        .callback_failures
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .record(std::time::Instant::now());
    let Some(count) = count else { return };

    let message = format!("'{event}' listener {how}: {}", describe(raw, cause));
    let info = ErrorInfo::new(error_code::CALLBACK_FAILED, ErrorKind::Client, message);
    let Ok(error) = crate::errors::error_value(raw, &info) else { return };
    let _ = set_property(raw, error, c"cause", cause);
    if let Ok(name) = env.create_string(event) {
        let _ = set_property(raw, error, c"event", name.raw());
    }
    if let Ok(count) = env.create_double(count as f64) {
        let _ = set_property(raw, error, c"count", count.raw());
    }

    let Some(listener) = listeners.get("error") else {
        print_error(raw, "unhandled listener failure (no 'error' listener):", &[error]);
        return;
    };
    let Ok(function) = listener.borrow_back(env) else { return };
    match call_function(raw, function.raw(), &[error]) {
        // A rejection from the `error` listener is only printed.
        Ok(returned) => on_rejection(listeners, env, "error", returned),
        Err(thrown) => {
            print_error(raw, "'error' listener threw while handling a listener failure:", &[thrown, error]);
        }
    }
}

/// Call `function` with `this` undefined: its return value, or the value it
/// threw (cleared from the environment).
fn call_function(
    env: sys::napi_env,
    function: sys::napi_value,
    args: &[sys::napi_value],
) -> Result<sys::napi_value, sys::napi_value> {
    let mut returned = std::ptr::null_mut();
    let status = unsafe {
        sys::napi_call_function(env, undefined(env), function, args.len(), args.as_ptr(), &mut returned)
    };
    if status == sys::Status::napi_ok {
        return Ok(returned);
    }
    let mut thrown = undefined(env);
    let mut pending = false;
    if unsafe { sys::napi_is_exception_pending(env, &mut pending) } == sys::Status::napi_ok && pending {
        unsafe { sys::napi_get_and_clear_last_exception(env, &mut thrown) };
    }
    Err(thrown)
}

/// `value.message` if it is a string, otherwise `String(value)`.
fn describe(env: sys::napi_env, value: sys::napi_value) -> String {
    let message = match type_of(env, value) {
        Some(sys::ValueType::napi_object | sys::ValueType::napi_function) => named_property(env, value, c"message")
            .filter(|message| type_of(env, *message) == Some(sys::ValueType::napi_string)),
        _ => None,
    };
    let mut string = std::ptr::null_mut();
    let target = message.unwrap_or(value);
    if unsafe { sys::napi_coerce_to_string(env, target, &mut string) } != sys::Status::napi_ok {
        clear_exception(env);
        return "<unprintable value>".to_string();
    }
    unsafe { <String as napi::bindgen_prelude::FromNapiValue>::from_napi_value(env, string) }
        .unwrap_or_else(|_| "<unprintable value>".to_string())
}

/// `console.error('[@fugle/marketdata] <label>', ...values)`. Failures are
/// ignored: there is nowhere left to report them.
fn print_error(env: sys::napi_env, label: &str, values: &[sys::napi_value]) {
    let print = || -> Option<()> {
        let mut global = std::ptr::null_mut();
        if unsafe { sys::napi_get_global(env, &mut global) } != sys::Status::napi_ok {
            return None;
        }
        let console = named_property(env, global, c"console")?;
        let error = named_property(env, console, c"error")?;
        let label = unsafe { String::to_napi_value(env, format!("[@fugle/marketdata] {label}")) }.ok()?;
        let mut args = vec![label];
        args.extend_from_slice(values);
        let mut ignored = std::ptr::null_mut();
        let status = unsafe { sys::napi_call_function(env, console, error, args.len(), args.as_ptr(), &mut ignored) };
        (status == sys::Status::napi_ok).then_some(())
    };
    if print().is_none() {
        clear_exception(env);
    }
}

fn type_of(env: sys::napi_env, value: sys::napi_value) -> Option<sys::napi_valuetype> {
    let mut kind = 0;
    (unsafe { sys::napi_typeof(env, value, &mut kind) } == sys::Status::napi_ok).then_some(kind)
}

/// `object[name]`, or `None` (with any exception cleared) if reading it fails.
fn named_property(env: sys::napi_env, object: sys::napi_value, name: &std::ffi::CStr) -> Option<sys::napi_value> {
    let mut value = std::ptr::null_mut();
    if unsafe { sys::napi_get_named_property(env, object, name.as_ptr(), &mut value) } != sys::Status::napi_ok {
        clear_exception(env);
        return None;
    }
    Some(value)
}

fn set_property(env: sys::napi_env, object: sys::napi_value, name: &std::ffi::CStr, value: sys::napi_value) -> Option<()> {
    if unsafe { sys::napi_set_named_property(env, object, name.as_ptr(), value) } != sys::Status::napi_ok {
        clear_exception(env);
        return None;
    }
    Some(())
}

fn undefined(env: sys::napi_env) -> sys::napi_value {
    let mut value = std::ptr::null_mut();
    unsafe { sys::napi_get_undefined(env, &mut value) };
    value
}

/// Drop a pending exception a getter or callee left behind.
fn clear_exception(env: sys::napi_env) {
    let mut pending = false;
    if unsafe { sys::napi_is_exception_pending(env, &mut pending) } == sys::Status::napi_ok && pending {
        let mut ignored = std::ptr::null_mut();
        unsafe { sys::napi_get_and_clear_last_exception(env, &mut ignored) };
    }
}

/// How a `connect()` settles (#23).
enum AuthOutcome {
    /// Authenticated: resolve with the server's `data`.
    Authenticated(serde_json::Value),
    /// Credentials rejected: reject with the server's `data` object itself.
    Rejected(serde_json::Value),
    /// Any other failure: reject with an `Error`.
    Failed(Failure),
}

/// Why `connect()` failed, other than rejected credentials.
enum Failure {
    /// An SDK error: reject with the unified error fields.
    Coded(ErrorInfo),
    /// A failure with no error code: reject with `Error(message)`.
    Plain(String),
}

type AuthTx = tokio::sync::oneshot::Sender<AuthOutcome>;

/// Signal that authentication finished (or why it failed).
type AuthRx = tokio::sync::oneshot::Receiver<AuthOutcome>;

/// The pending `connect()` settlement, shared by the worker and the event
/// thread: whichever settles first takes it, so a Promise settles once.
type AuthSlot = Arc<Mutex<Option<AuthTx>>>;

/// Who decided how the initial authentication of a connection is reported
/// (#44): the forwarder on `Authenticated`, or the worker aborting because
/// disconnect() was called while authenticating. Whichever moves it off
/// `AUTH_PENDING` first wins, so `authenticated` never fires for a
/// connection whose `connect()` rejects as aborted.
type AuthDecision = Arc<AtomicU8>;
const AUTH_PENDING: u8 = 0;
const AUTH_REPORTED: u8 = 1;
const AUTH_ABORTED: u8 = 2;

/// Move `decision` from pending to `to`; false if the other side decided.
fn decide(decision: &AtomicU8, to: u8) -> bool {
    decision
        .compare_exchange(AUTH_PENDING, to, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok()
}

/// Test-only delay after the worker's `client.connect()` returns, before it
/// checks for an abort (#44): `FUGLE_MARKETDATA_TEST_DELAY_AFTER_CONNECT_MS`,
/// read on the JS thread when connecting. Debug builds only.
#[cfg(debug_assertions)]
fn test_delay_after_connect() -> Option<Duration> {
    std::env::var("FUGLE_MARKETDATA_TEST_DELAY_AFTER_CONNECT_MS")
        .ok()
        .and_then(|ms| ms.parse().ok())
        .map(Duration::from_millis)
}

#[cfg(not(debug_assertions))]
fn test_delay_after_connect() -> Option<Duration> {
    None
}

/// Settle the pending `connect()`, if it has not been settled already.
fn settle(slot: &AuthSlot, outcome: AuthOutcome) {
    if let Some(tx) = slot.lock().ok().and_then(|mut guard| guard.take()) {
        let _ = tx.send(outcome);
    }
}

/// Promise returned by `connect()`, settled by [`AuthOutcome`]. A failure to
/// start the worker rejects it too, so `connect()` never throws
/// synchronously.
fn auth_promise<'env>(
    env: &'env Env,
    started: napi::Result<AuthRx>,
) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
    env.spawn_future_with_callback(
        async move {
            Ok(started?.await.unwrap_or_else(|_| {
                AuthOutcome::Failed(Failure::Plain(
                    "Worker thread terminated before authentication signal".to_string(),
                ))
            }))
        },
        |env, outcome| match outcome {
            AuthOutcome::Authenticated(data) => {
                Ok(unsafe { Unknown::from_raw_unchecked(env.raw(), json_to_napi(env.raw(), data)?) })
            }
            AuthOutcome::Rejected(data) => {
                let data = unsafe { Unknown::from_raw_unchecked(env.raw(), json_to_napi(env.raw(), data)?) };
                Err(napi::Error::from(data))
            }
            AuthOutcome::Failed(Failure::Coded(info)) => Err(crate::errors::js_error(env, &info)),
            AuthOutcome::Failed(Failure::Plain(message)) => Err(napi::Error::from_reason(message)),
        },
    )
}

fn loop_keep_alive(env: &Env) -> napi::Result<KeepAlive> {
    env.create_function_from_closure::<(), (), _>("fugleWsKeepAlive", |_| Ok(()))?
        .build_threadsafe_function::<()>()
        .callee_handled::<false>()
        .build()
        .map(Arc::new)
}

/// Reconnection options for WebSocket clients
///
/// All fields are optional - defaults are applied when not specified:
/// - maxAttempts: 0 (unlimited)
/// - initialDelayMs: 1000
/// - maxDelayMs: 60000
#[napi(object)]
#[derive(Debug, Clone, Default)]
pub struct ReconnectOptions {
    /// Whether auto-reconnect is enabled (default: true, also when the
    /// `reconnect` option is omitted; set `false` to turn it off)
    pub enabled: Option<bool>,
    /// Maximum reconnection attempts; 0 means unlimited (default: 0, so the
    /// client keeps retrying at most `maxDelayMs` apart)
    pub max_attempts: Option<u32>,
    /// Initial reconnection delay in milliseconds (default: 1000, min: 100)
    pub initial_delay_ms: Option<f64>,
    /// Maximum reconnection delay in milliseconds (default: 60000)
    pub max_delay_ms: Option<f64>,
}

/// Health check options for WebSocket connections
///
/// All fields are optional - defaults are applied when not specified:
/// - enabled: false
/// - pingInterval: 30000
/// - maxMissedPongs: 2
///
/// Defaults: enabled=true, heartbeatTimeoutMs=35000.
#[napi(object)]
#[derive(Debug, Clone, Default)]
pub struct HealthCheckOptions {
    /// Whether liveness detection is active (default: true in 3.0)
    pub enabled: Option<bool>,
    /// Maximum allowed gap between inbound frames before declaring the
    /// connection dead, in milliseconds. Default 35000 (Fugle server's
    /// 30s heartbeat + 5s buffer); floor 5000.
    pub heartbeat_timeout_ms: Option<f64>,
}

/// REST client options
///
/// Exactly ONE non-empty apiKey, bearerToken, or sdkToken must be provided;
/// an empty or whitespace-only value counts as not provided.
/// baseUrl is optional for custom endpoint override.
#[napi(object)]
#[derive(Default)]
pub struct RestClientOptions {
    /// API key for authentication
    pub api_key: Option<String>,
    /// Bearer token for authentication
    pub bearer_token: Option<String>,
    /// SDK token for authentication
    pub sdk_token: Option<String>,
    /// Override base URL (optional)
    pub base_url: Option<String>,
    /// Additional root CA (PEM bytes). Appended to the OS trust store;
    /// chains signed by either this CA or an OS-trusted root are accepted.
    pub tls_root_cert_pem: Option<napi::bindgen_prelude::Uint8Array>,
    /// Disable ALL TLS verification (chain + hostname + expiry).
    /// Dev/testing only — exposes MITM risk. Defaults to false.
    pub tls_accept_invalid_certs: Option<bool>,
}

/// WebSocket client options
///
/// Exactly ONE non-empty apiKey, bearerToken, or sdkToken must be provided;
/// an empty or whitespace-only value counts as not provided.
/// reconnect and healthCheck are optional configuration objects.
#[napi(object)]
#[derive(Default)]
pub struct WebSocketClientOptions {
    /// API key for authentication
    pub api_key: Option<String>,
    /// Bearer token for authentication
    pub bearer_token: Option<String>,
    /// SDK token for authentication
    pub sdk_token: Option<String>,
    /// Override base URL (optional). Host and path prefix ONLY — the SDK
    /// appends the version segment.
    pub base_url: Option<String>,
    /// Per-product streaming version, e.g. `{ futopt: 'v1.0' }`.
    /// Omitted products get their latest: stock v1.0, futopt v1.1.
    pub version: Option<StreamingVersionOptions>,
    /// Reconnection configuration (optional)
    pub reconnect: Option<ReconnectOptions>,
    /// Health check configuration (optional)
    pub health_check: Option<HealthCheckOptions>,
    /// Additional root CA (PEM bytes). Appended to the OS trust store.
    pub tls_root_cert_pem: Option<napi::bindgen_prelude::Uint8Array>,
    /// Disable ALL TLS verification (chain + hostname + expiry).
    /// Dev/testing only — exposes MITM risk. Defaults to false.
    pub tls_accept_invalid_certs: Option<bool>,
    /// What happens while `messageBuffer` messages are unread: `'dropNewest'`
    /// (default) drops new ones and reports them with `messagesDropped`;
    /// `'unbounded'` never drops, and memory grows while listeners lag.
    #[napi(ts_type = "'dropNewest' | 'unbounded'")]
    pub message_overflow: Option<String>,
    /// Unread messages held before `messageOverflow` applies (default 4096).
    /// Up to this many wait in the SDK, and up to this many more may be
    /// queued for `message` listeners that have not run yet. While those
    /// listeners hold up delivery, events wait as well; beyond 1024 unread
    /// events the SDK drops them too.
    pub message_buffer: Option<u32>,
}

/// `messageOverflow` / `messageBuffer` of a `WebSocketClient` (#46).
#[derive(Clone, Copy)]
struct MessageQueueSettings {
    overflow: marketdata_core::MessageOverflow,
    buffer: usize,
}

impl MessageQueueSettings {
    fn parse(overflow: Option<&str>, buffer: Option<u32>) -> napi::Result<Self> {
        let overflow = match overflow {
            None | Some("dropNewest") => marketdata_core::MessageOverflow::DropNewest,
            Some("unbounded") => marketdata_core::MessageOverflow::Unbounded,
            Some(other) => {
                return Err(napi::Error::from_reason(format!(
                    "messageOverflow must be 'dropNewest' or 'unbounded', got {other:?}"
                )))
            }
        };
        let buffer = match buffer {
            None => marketdata_core::websocket::DEFAULT_MESSAGE_BUFFER,
            Some(0) => {
                return Err(napi::Error::from_reason(
                    "messageBuffer must be a positive integer",
                ))
            }
            Some(n) => n as usize,
        };
        Ok(Self { overflow, buffer })
    }

    fn apply(self, config: &mut marketdata_core::ConnectionConfig) {
        config.message_overflow = self.overflow;
        config.message_buffer = self.buffer;
    }

    /// Frames that may be in flight to `message` listeners at once.
    fn in_flight_limit(self) -> Option<usize> {
        match self.overflow {
            marketdata_core::MessageOverflow::Unbounded => None,
            _ => Some(self.buffer),
        }
    }
}

/// Read-only handles on a client's current or last core connection. They
/// outlive the core client, which the worker drops when the connection ends:
/// holding the client itself would keep its stream, and so the event thread,
/// alive.
struct ConnectionHandles {
    state: marketdata_core::ConnectionStateHandle,
    messages_dropped: marketdata_core::MessagesDroppedHandle,
}

/// The [`ConnectionHandles`] of a client's current or last connection; `None`
/// before the first `connect()`.
type ConnectionSlot = Arc<Mutex<Option<ConnectionHandles>>>;

/// Lock `slot`. A poisoned lock still yields the handles: writers only
/// assign, so they can never be left half-updated.
fn lock_connection(slot: &ConnectionSlot) -> std::sync::MutexGuard<'_, Option<ConnectionHandles>> {
    slot.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Read the handles in `slot`; `None` before the first `connect()`.
fn read_connection<T>(slot: &ConnectionSlot, read: impl FnOnce(&ConnectionHandles) -> T) -> Option<T> {
    lock_connection(slot).as_ref().map(read)
}

/// Per-product streaming version selection.
///
/// The official SDK takes a free-form map and validates at runtime; expressing
/// it as a struct lets TypeScript reject an unknown product at compile time,
/// while the string values still need checking here.
#[napi(object)]
pub struct StreamingVersionOptions {
    /// Stock streaming version. Only "v1.0" is served.
    pub stock: Option<String>,
    /// FutOpt streaming version: "v1.0" or "v1.1" (default).
    ///
    /// v1.1 adds trial-matching (試撮) frames on trades / books — branch on
    /// the frame's `isTrial` before acting on a price.
    pub futopt: Option<String>,
}

#[derive(Clone, Copy)]
pub(crate) enum WsProduct {
    Stock,
    FutOpt,
}

/// Validate the `version` option into core's per-product enums.
pub(crate) fn parse_ws_versions(
    version: &Option<StreamingVersionOptions>,
) -> napi::Result<(
    marketdata_core::websocket::StockVersion,
    marketdata_core::websocket::FutOptVersion,
)> {
    use marketdata_core::websocket::{FutOptVersion, StockVersion};

    let mut stock = StockVersion::default();
    let mut futopt = FutOptVersion::default();

    if let Some(opts) = version {
        if let Some(v) = opts.stock.as_deref() {
            stock = match v {
                "v1.0" => StockVersion::V1_0,
                other => {
                    return Err(napi::Error::from_reason(format!(
                        "stock streaming does not support {other} (supported: v1.0). \
                         Omit it to use v1.0."
                    )))
                }
            };
        }
        if let Some(v) = opts.futopt.as_deref() {
            futopt = match v {
                "v1.0" => FutOptVersion::V1_0,
                "v1.1" => FutOptVersion::V1_1,
                other => {
                    return Err(napi::Error::from_reason(format!(
                        "futopt streaming does not support {other} (supported: v1.0, v1.1). \
                         Omit it to use v1.1."
                    )))
                }
            };
        }
    }

    Ok((stock, futopt))
}

/// Resolve a streaming endpoint through core's factory.
///
/// Centralised so the two call sites cannot drift on base-URL semantics —
/// each used to hand-roll `format!("{base}/stock/streaming")`, which is
/// exactly the duplication 0.8.0 removes.
pub(crate) fn build_stream_config(
    auth: &marketdata_core::AuthRequest,
    base_url: Option<&str>,
    product: WsProduct,
    stock_version: marketdata_core::websocket::StockVersion,
    futopt_version: marketdata_core::websocket::FutOptVersion,
) -> Result<marketdata_core::ConnectionConfig, marketdata_core::MarketDataError> {
    let mut factory = marketdata_core::WebSocketFactory::new()
        .stock_version(stock_version)
        .futopt_version(futopt_version);
    if let Some(base) = base_url {
        factory = factory.base_url(base);
    }
    let factory = factory.auth(auth.clone());
    let builder = match product {
        WsProduct::Stock => factory.stock()?,
        WsProduct::FutOpt => factory.futopt()?,
    };
    Ok(builder.build())
}

/// Command sent to WebSocket worker thread
#[derive(Debug)]
enum WsCommand {
    Subscribe(Subscription),
    Unsubscribe { ids: Vec<String> },
    /// Send a `ping` frame carrying `data` (mirrors 1.x `ping(params)`)
    Ping { data: Option<serde_json::Value> },
    /// Ask the server for its current subscription list (response arrives via `message`)
    QuerySubscriptions,
    Disconnect,
}

/// A subscription whose channel `subscribe()` already parsed (#113).
#[derive(Debug)]
enum Subscription {
    Stock(marketdata_core::StockSubscription),
    FutOpt(marketdata_core::FutOptSubscription),
}

impl Subscription {
    async fn send(self, client: &marketdata_core::aio::WebSocketClient) -> Result<(), marketdata_core::MarketDataError> {
        match self {
            Subscription::Stock(sub) => client.subscribe(sub).await,
            Subscription::FutOpt(sub) => client.subscribe_futopt(sub).await,
        }
    }
}

/// A `ping()` argument as the frame's `data` (#23): an object (or any other
/// value) as-is, 1.x style; a string as `{ state }`, as rc.2 sent it; nothing
/// (or `null`) omits `data`.
/// The server ids `unsubscribe()` names: a string, `{ id }` or `{ ids }`.
/// `None` for options naming a `channel` instead; naming both is 1005.
fn unsubscribe_ids(env: &Env, options: &serde_json::Value) -> napi::Result<Option<Vec<String>>> {
    if let Some(s) = options.as_str() {
        return Ok(Some(vec![s.to_string()]));
    }
    let has_ids = options.get("id").is_some() || options.get("ids").is_some();
    if options.get("channel").is_some() {
        if has_ids {
            let err = marketdata_core::MarketDataError::InvalidParameter {
                name: "channel".to_string(),
                reason: "cannot be combined with 'id' or 'ids'".to_string(),
            };
            return Err(crate::errors::to_napi_error(env, err));
        }
        return Ok(None);
    }

    let single = options.get("id").and_then(|v| v.as_str()).map(String::from);
    let batch = options
        .get("ids")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect::<Vec<_>>()
        });

    match (single, batch) {
        (Some(id), None) => Ok(Some(vec![id])),
        (None, Some(list)) if !list.is_empty() => Ok(Some(list)),
        (None, Some(_)) => Err(napi::Error::from_reason(
            "unsubscribe({ids:[]}) is empty - provide at least one id",
        )),
        (Some(_), Some(_)) => Err(napi::Error::from_reason(
            "unsubscribe() accepts either 'id' or 'ids', not both",
        )),
        (None, None) => Err(napi::Error::from_reason(
            "unsubscribe() requires 'id', 'ids' or 'channel'",
        )),
    }
}

/// The `channel` of `unsubscribe()` options, parsed as for `subscribe()` (1005).
fn unsubscribe_channel<C>(env: &Env, options: &serde_json::Value) -> napi::Result<C>
where
    C: std::str::FromStr<Err = marketdata_core::MarketDataError>,
{
    options
        .get("channel")
        .and_then(|v| v.as_str())
        .ok_or_else(|| napi::Error::from_reason("Missing 'channel' field"))?
        .parse::<C>()
        .map_err(|e| crate::errors::to_napi_error(env, e))
}

/// The `symbol` or `symbols` of subscribe-shaped options — exactly one.
/// `call` names the method in error messages.
fn option_symbols(call: &str, options: &serde_json::Value) -> napi::Result<Vec<String>> {
    let single = options.get("symbol").and_then(|v| v.as_str()).map(String::from);
    let batch = options.get("symbols").and_then(|v| v.as_array()).map(|arr| {
        arr.iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect::<Vec<_>>()
    });
    match (single, batch) {
        (Some(s), None) => Ok(vec![s]),
        (None, Some(list)) if !list.is_empty() => Ok(list),
        (None, Some(_)) => Err(napi::Error::from_reason(format!(
            "{call}({{symbols:[]}}) is empty - provide at least one symbol"
        ))),
        (Some(_), Some(_)) => Err(napi::Error::from_reason(format!(
            "{call}() accepts either 'symbol' or 'symbols', not both"
        ))),
        (None, None) => Err(napi::Error::from_reason(format!(
            "{call}() requires 'symbol' or 'symbols'"
        ))),
    }
}

fn ping_data(params: Option<serde_json::Value>) -> Option<serde_json::Value> {
    match params? {
        serde_json::Value::Null => None,
        serde_json::Value::String(state) => Some(serde_json::json!({ "state": state })),
        value => Some(value),
    }
}

/// Rejection for `connect()` while a connection is open or still being
/// established (#44): core's `AlreadyConnected`, decided here from the
/// worker slot rather than core's client state (#119).
fn already_connected() -> ErrorInfo {
    marketdata_core::MarketDataError::AlreadyConnected.info()
}

/// Rejection for a `connect()` whose connection was given up because
/// `disconnect()` was called before the connection was established (#44):
/// core's `ConnectionAborted` (#121).
fn connect_aborted() -> Failure {
    Failure::Coded(marketdata_core::MarketDataError::ConnectionAborted.info())
}

/// The worker thread that owns a client's connection (#44).
struct Worker {
    tx: std::sync::mpsc::Sender<WsCommand>,
    handle: thread::JoinHandle<()>,
    /// Set once the connection is on its way out for good: `disconnect()` was
    /// called, authentication failed, or core stopped with no reconnect left.
    /// Always set *before* the JS side can observe the end (a `disconnect` /
    /// `error` callback, a rejected `connect()`), so calling `connect()` from
    /// there is accepted rather than racing the worker's exit.
    ending: Arc<AtomicBool>,
}

/// A client's current worker, shared by every `ws.stock` / `ws.futopt` wrapper.
type WorkerSlot = Arc<Mutex<Option<Worker>>>;

/// Make room in `slot` for a new worker.
///
/// Rejects while the current worker is connecting or connected. A worker
/// that is ending (or already gone) is taken out and its handle returned: the
/// new worker joins it before connecting, so the old connection is torn down
/// before the new one replaces it.
fn claim_worker_slot(
    slot: &mut Option<Worker>,
) -> Result<Option<thread::JoinHandle<()>>, ErrorInfo> {
    match slot.take() {
        None => Ok(None),
        Some(worker)
            if worker.ending.load(Ordering::SeqCst) || worker.handle.is_finished() =>
        {
            Ok(Some(worker.handle))
        }
        Some(worker) => {
            *slot = Some(worker);
            Err(already_connected())
        }
    }
}

/// Queue `command` for the running worker.
fn send_command(slot: &WorkerSlot, command: WsCommand, name: &str) -> napi::Result<()> {
    let guard = slot
        .lock()
        .map_err(|e| napi::Error::from_reason(format!("Lock error: {}", e)))?;
    let worker = guard
        .as_ref()
        .ok_or_else(|| napi::Error::from_reason("Not connected. Call connect() first."))?;
    worker
        .tx
        .send(command)
        .map_err(|_| napi::Error::from_reason(format!("Failed to send {} command", name)))
}

/// Mark the worker as ending and ask it to disconnect; no-op without one.
fn request_disconnect(slot: &WorkerSlot) -> napi::Result<()> {
    let guard = slot
        .lock()
        .map_err(|e| napi::Error::from_reason(format!("Lock error: {}", e)))?;
    if let Some(worker) = guard.as_ref() {
        worker.ending.store(true, Ordering::SeqCst);
        let _ = worker.tx.send(WsCommand::Disconnect);
    }
    Ok(())
}

/// The registered listener of each event, one per event.
///
/// Shared as `Arc`s so [`call_listener`] can release the lock before calling.
#[derive(Default)]
struct EventCallbacks {
    message: Option<Arc<Listener>>,
    connect: Option<Arc<Listener>>,
    disconnect: Option<Arc<Listener>>,
    reconnect: Option<Arc<Listener>>,
    error: Option<Arc<Listener>>,
    authenticated: Option<Arc<Listener>>,
    unauthenticated: Option<Arc<Listener>>,
    messages_dropped: Option<Arc<Listener>>,
}

impl EventCallbacks {
    fn slot(&mut self, event: &str) -> Option<&mut Option<Arc<Listener>>> {
        match event {
            "message" => Some(&mut self.message),
            "connect" => Some(&mut self.connect),
            "disconnect" => Some(&mut self.disconnect),
            "reconnect" => Some(&mut self.reconnect),
            "error" => Some(&mut self.error),
            "authenticated" => Some(&mut self.authenticated),
            "unauthenticated" => Some(&mut self.unauthenticated),
            "messagesDropped" => Some(&mut self.messages_dropped),
            _ => None,
        }
    }

    fn get(&mut self, event: &str) -> Option<Arc<Listener>> {
        self.slot(event).and_then(|slot| slot.clone())
    }
}

/// A client's listeners, shared by its connections' [`EventSink`]s.
#[derive(Default)]
struct Listeners {
    callbacks: Mutex<EventCallbacks>,
    /// Set once a `message` listener is registered; there is no way to remove
    /// one. Read by the worker for every frame (see [`EventSink::emit_message`]).
    has_message: AtomicBool,
    /// Throttles the reports of failed listeners (#83), across the client's
    /// connections.
    callback_failures: Mutex<marketdata_core::websocket::ReportThrottle>,
}

impl Listeners {
    /// The listener registered for `event`. A panic while holding the lock
    /// must not silence the events that report it (#25).
    fn get(&self, event: &str) -> Option<Arc<Listener>> {
        self.callbacks
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(event)
    }
}

/// `on(event, callback)`, shared by the stock and futopt clients.
fn register_listener(
    listeners: &Listeners,
    event: &str,
    callback: Function<'_, EventArgs, Unknown<'static>>,
) -> napi::Result<()> {
    let listener = Arc::new(callback.create_ref()?);
    let mut callbacks = listeners
        .callbacks
        .lock()
        .map_err(|e| napi::Error::from_reason(format!("Lock error: {}", e)))?;
    let slot = callbacks.slot(event).ok_or_else(|| {
        napi::Error::from_reason(format!(
            "Unknown event type: {}. Valid events: message, connect, disconnect, reconnect, error, authenticated, unauthenticated, messagesDropped",
            event
        ))
    })?;
    *slot = Some(listener);
    if event == "message" {
        listeners.has_message.store(true, Ordering::SeqCst);
    }
    Ok(())
}

/// WebSocket client for real-time market data (JavaScript wrapper)
///
/// # JavaScript Usage
///
/// ```javascript
/// const { WebSocketClient } = require('@fugle/marketdata');
///
/// // Create client with API key
/// const ws = new WebSocketClient('your-api-key');
///
/// // Register event handlers for stock data
/// ws.stock.on('message', (data) => console.log(JSON.parse(data)));
/// ws.stock.on('connect', () => console.log('Connected!'));
/// ws.stock.on('error', (err) => console.error(err));
///
/// // Connect and subscribe
/// ws.stock.connect();
/// ws.stock.subscribe({ channel: 'trades', symbol: '2330' });
/// ```
#[napi]
pub struct WebSocketClient {
    auth: marketdata_core::AuthRequest,
    base_url: Option<String>,
    stock_version: marketdata_core::websocket::StockVersion,
    futopt_version: marketdata_core::websocket::FutOptVersion,
    reconnect_config: marketdata_core::ReconnectionConfig,
    health_check_config: marketdata_core::HealthCheckConfig,
    tls_config: marketdata_core::TlsConfig,
    message_queue: MessageQueueSettings,
    // Shared state for child clients — created once in constructor so that
    // every `ws.stock` / `ws.futopt` getter access shares the same Arcs.
    stock_callbacks: Arc<Listeners>,
    stock_worker: WorkerSlot,
    stock_connection: ConnectionSlot,
    futopt_callbacks: Arc<Listeners>,
    futopt_worker: WorkerSlot,
    futopt_connection: ConnectionSlot,
}

#[napi]
impl WebSocketClient {
    /// Create a new WebSocket client with configuration
    ///
    /// @param options - Client configuration options
    /// @throws {Error} If validation fails (zero or multiple auth methods, invalid config values)
    ///
    /// @example
    /// ```javascript
    /// const { WebSocketClient } = require('@fugle/marketdata');
    ///
    /// // Simple usage with defaults
    /// const ws = new WebSocketClient({ apiKey: 'your-key' });
    ///
    /// // Custom reconnection config
    /// const ws = new WebSocketClient({
    ///   apiKey: 'your-key',
    ///   reconnect: { maxAttempts: 10, initialDelayMs: 2000 }
    /// });
    ///
    /// // Enable health check
    /// const ws = new WebSocketClient({
    ///   apiKey: 'your-key',
    ///   healthCheck: { enabled: true, pingInterval: 20000 }
    /// });
    /// ```
    #[napi(constructor)]
    pub fn new(env: Env, options: WebSocketClientOptions) -> napi::Result<Self> {
        use marketdata_core::{
            DEFAULT_MAX_ATTEMPTS, DEFAULT_INITIAL_DELAY_MS, DEFAULT_MAX_DELAY_MS,
            DEFAULT_HEALTH_CHECK_ENABLED, DEFAULT_HEARTBEAT_TIMEOUT_MS,
        };
        use std::time::Duration;

        // Core requires exactly one non-blank credential (ConfigError, 1004)
        // and sends it in the auth frame field matching its kind (#91).
        let auth = marketdata_core::AuthRequest::from(
            marketdata_core::Auth::from_credentials(
                options.api_key,
                options.bearer_token,
                options.sdk_token,
            )
            .map_err(|e| crate::errors::to_napi_error(&env, e))?,
        );

        let (stock_version, futopt_version) = parse_ws_versions(&options.version)?;
        let message_queue = MessageQueueSettings::parse(
            options.message_overflow.as_deref(),
            options.message_buffer,
        )?;

        // Resolve both endpoints now so a bad `baseUrl` throws from the
        // constructor rather than from `.stock.connect()` much later. Matches
        // the official SDK, which rejects a versioned baseUrl up front.
        for product in [WsProduct::Stock, WsProduct::FutOpt] {
            build_stream_config(
                &auth,
                options.base_url.as_deref(),
                product,
                stock_version,
                futopt_version,
            )
            .map_err(|e| crate::errors::to_napi_error(&env, e))?;
        }

        // Build reconnection config with validation via core. Omitting
        // `options.reconnect` uses the core default (auto-reconnect on,
        // unlimited attempts); `{ reconnect: { enabled: false } }` turns it off.
        let reconnect_cfg = if let Some(r) = &options.reconnect {
            let max = r.max_attempts.unwrap_or(DEFAULT_MAX_ATTEMPTS);
            let initial = Duration::from_millis(
                r.initial_delay_ms.map(|v| v as u64).unwrap_or(DEFAULT_INITIAL_DELAY_MS)
            );
            let max_delay = Duration::from_millis(
                r.max_delay_ms.map(|v| v as u64).unwrap_or(DEFAULT_MAX_DELAY_MS)
            );
            let mut cfg = marketdata_core::ReconnectionConfig::new(max, initial, max_delay)
                .map_err(|e| napi::Error::from_reason(e.to_string()))?;
            // Honor explicit opt-out: `{ reconnect: { enabled: false } }`.
            if let Some(enabled) = r.enabled {
                cfg.enabled = enabled;
            }
            cfg
        } else {
            marketdata_core::ReconnectionConfig::default()
        };

        // Build health check config with validation via core
        let health_check_cfg = if let Some(hc) = &options.health_check {
            let enabled = hc.enabled.unwrap_or(DEFAULT_HEALTH_CHECK_ENABLED);
            let timeout = Duration::from_millis(
                hc.heartbeat_timeout_ms.map(|v| v as u64).unwrap_or(DEFAULT_HEARTBEAT_TIMEOUT_MS)
            );
            let mut cfg = marketdata_core::HealthCheckConfig::with_timeout(timeout)
                .map_err(|e| napi::Error::from_reason(e.to_string()))?;
            cfg.enabled = enabled;
            cfg
        } else {
            marketdata_core::HealthCheckConfig::default()
        };

        // Build TLS config from options. Default config matches previous
        // behaviour (OS trust store, no overrides); custom CA / accept_invalid
        // flow through to the worker thread's ConnectionConfig.
        let tls_config = marketdata_core::TlsConfig {
            root_cert_pem: options.tls_root_cert_pem.map(|arr| arr.to_vec()),
            accept_invalid_certs: options.tls_accept_invalid_certs.unwrap_or(false),
        };

        Ok(Self {
            auth,
            base_url: options.base_url,
            stock_version,
            futopt_version,
            reconnect_config: reconnect_cfg,
            health_check_config: health_check_cfg,
            tls_config,
            message_queue,
            stock_callbacks: Arc::new(Listeners::default()),
            stock_worker: Arc::new(Mutex::new(None)),
            stock_connection: Arc::new(Mutex::new(None)),
            futopt_callbacks: Arc::new(Listeners::default()),
            futopt_worker: Arc::new(Mutex::new(None)),
            futopt_connection: Arc::new(Mutex::new(None)),
        })
    }

    /// Get the stock WebSocket client for real-time stock data.
    ///
    /// Every access returns a new JS wrapper but all wrappers share the same
    /// underlying state (callbacks, connection state, command channel), so the
    /// legacy `ws.stock.on(...); ws.stock.connect()` pattern works correctly.
    #[napi(getter)]
    pub fn stock(&self) -> StockWebSocketClient {
        StockWebSocketClient::from_shared(
            self.auth.clone(),
            self.base_url.clone(),
            self.stock_version,
            self.futopt_version,
            self.reconnect_config.clone(),
            self.health_check_config.clone(),
            self.tls_config.clone(),
            self.message_queue,
            Arc::clone(&self.stock_callbacks),
            Arc::clone(&self.stock_worker),
            Arc::clone(&self.stock_connection),
        )
    }

    /// Get the FutOpt WebSocket client for real-time futures/options data.
    ///
    /// Same shared-state semantics as `stock` — see its doc comment.
    #[napi(getter)]
    pub fn futopt(&self) -> FutOptWebSocketClient {
        FutOptWebSocketClient::from_shared(
            self.auth.clone(),
            self.base_url.clone(),
            self.stock_version,
            self.futopt_version,
            self.reconnect_config.clone(),
            self.health_check_config.clone(),
            self.tls_config.clone(),
            self.message_queue,
            Arc::clone(&self.futopt_callbacks),
            Arc::clone(&self.futopt_worker),
            Arc::clone(&self.futopt_connection),
        )
    }
}

/// Stock WebSocket client for real-time stock market data
///
/// # JavaScript Usage
///
/// ```javascript
/// // Event handlers
/// ws.stock.on('message', (data) => {
///   const msg = JSON.parse(data);
///   console.log(msg);
/// });
/// ws.stock.on('connect', () => console.log('Stock WebSocket connected'));
/// ws.stock.on('disconnect', (reason) => console.log('Disconnected:', reason));
/// ws.stock.on('reconnect', (info) => console.log('Reconnecting:', info));
/// ws.stock.on('error', (err) => console.error('Error:', err));
///
/// // Connect
/// ws.stock.connect();
///
/// // Subscribe to channels
/// ws.stock.subscribe({ channel: 'trades', symbol: '2330' });
/// ws.stock.subscribe({ channel: 'candles', symbol: '2330' });
/// ```
#[napi]
pub struct StockWebSocketClient {
    auth: marketdata_core::AuthRequest,
    base_url: Option<String>,
    stock_version: marketdata_core::websocket::StockVersion,
    futopt_version: marketdata_core::websocket::FutOptVersion,
    reconnect_config: marketdata_core::ReconnectionConfig,
    health_check_config: marketdata_core::HealthCheckConfig,
    tls_config: marketdata_core::TlsConfig,
    message_queue: MessageQueueSettings,
    callbacks: Arc<Listeners>,
    worker: WorkerSlot,
    connection: ConnectionSlot,
}

#[napi]
impl StockWebSocketClient {
    /// Create from pre-existing shared state (called by WebSocketClient getter).
    /// All mutable state lives behind Arc so multiple JS wrappers returned by
    /// the `ws.stock` getter share the same underlying callbacks, connection
    /// state, and command channel.
    fn from_shared(
        auth: marketdata_core::AuthRequest,
        base_url: Option<String>,
        stock_version: marketdata_core::websocket::StockVersion,
        futopt_version: marketdata_core::websocket::FutOptVersion,
        reconnect_config: marketdata_core::ReconnectionConfig,
        health_check_config: marketdata_core::HealthCheckConfig,
        tls_config: marketdata_core::TlsConfig,
        message_queue: MessageQueueSettings,
        callbacks: Arc<Listeners>,
        worker: WorkerSlot,
        connection: ConnectionSlot,
    ) -> Self {
        Self {
            auth,
            base_url,
            stock_version,
            futopt_version,
            reconnect_config,
            health_check_config,
            tls_config,
            message_queue,
            callbacks,
            worker,
            connection,
        }
    }

    /// Register an event handler
    ///
    /// Arguments match `@fugle/marketdata` 1.x (#23): `message(data: string)`,
    /// `connect()` when the socket opens, `authenticated(data)` /
    /// `unauthenticated(data)` with the server's `data`,
    /// `disconnect({ code, reason })`, `reconnect({ attempt })`, and
    /// `error(Error)` with a numeric `code` when core supplied one. Without an
    /// `error` listener errors are ignored rather than thrown.
    ///
    /// `message` frames that arrive before a `message` listener is registered
    /// are dropped, not delivered to it later (#62).
    ///
    /// @param event - Event type: "message", "connect", "authenticated",
    ///                "unauthenticated", "disconnect", "reconnect", "error"
    /// @param callback - Listener for that event
    ///
    /// @example
    /// ```javascript
    /// ws.stock.on('message', (data) => console.log(data));
    /// ws.stock.on('connect', () => console.log('Connected'));
    /// ws.stock.on('disconnect', ({ code, reason }) => console.log(code, reason));
    /// ws.stock.on('error', (err) => console.error(err.code, err.message));
    /// ```
    #[napi(
        ts_generic_types = "E extends WebSocketEvent",
        ts_args_type = "event: E, callback: WebSocketEventMap[E]"
    )]
    pub fn on(
        &self,
        event: String,
        callback: Function<'_, EventArgs, Unknown<'static>>,
    ) -> napi::Result<()> {
        register_listener(&self.callbacks, &event, callback)
    }

    /// Connect to the stock WebSocket server.
    ///
    /// Returns a Promise that resolves with the server's `authenticated`
    /// `data` once authentication completes, matching `@fugle/marketdata` 1.x
    /// (#23):
    ///
    /// ```js
    /// stock.connect().then((data) => {
    ///   stock.subscribe({ channel: 'trades', symbol: '2330' });
    /// });
    /// ```
    ///
    /// If the server rejects the credentials, the Promise rejects with the
    /// server's `data` object itself (after `unauthenticated` fires); any other
    /// failure rejects with a `MarketDataError` (`code`, `sourceKind`, … as
    /// properties; no `[code]` prefix in the message).
    ///
    /// Rejects with code `2011` (`Already connected`) while a connection is open or
    /// being established (#44). Call disconnect() first to reconnect; calling
    /// connect() right after disconnect(), or from a `disconnect` handler once
    /// no auto-reconnect will follow, is fine.
    #[napi(ts_return_type = "Promise<WebSocketAuthData | undefined>")]
    pub fn connect<'env>(&self, env: &'env Env) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        auth_promise(env, self.start_worker(env))
    }

    /// Spawn the worker thread; the receiver fires once it has authenticated.
    fn start_worker(&self, env: &Env) -> napi::Result<AuthRx> {
        // Held until the new worker is stored, so concurrent connect() calls
        // cannot both claim the slot (#44).
        let mut slot = self.worker.lock().map_err(|e| {
            napi::Error::from_reason(format!("Lock error: {}", e))
        })?;
        let previous = claim_worker_slot(&mut slot)
            .map_err(|info| crate::errors::js_error(env, &info))?;
        let sink = EventSink::new(
            env,
            Arc::clone(&self.callbacks),
            self.message_queue.in_flight_limit(),
        )?;
        let (cmd_tx, cmd_rx) = std::sync::mpsc::channel::<WsCommand>();
        let ending = Arc::new(AtomicBool::new(false));
        let decision: AuthDecision = Arc::new(AtomicU8::new(AUTH_PENDING));
        let panic_reported = Arc::new(AtomicBool::new(false));
        let delay_after_connect = test_delay_after_connect();

        let (auth_tx, auth_rx) = tokio::sync::oneshot::channel::<AuthOutcome>();
        let auth: AuthSlot = Arc::new(Mutex::new(Some(auth_tx)));

        // Clone data for the worker thread
        let auth_request = self.auth.clone();
        let base_url = self.base_url.clone();
        let stock_version = self.stock_version;
        let futopt_version = self.futopt_version;
        let reconnect_config = self.reconnect_config.clone();
        let health_check_config = self.health_check_config.clone();
        let tls_config = self.tls_config.clone();
        let message_queue = self.message_queue;
        let connection = Arc::clone(&self.connection);
        let ending_for_worker = Arc::clone(&ending);
        let test_panic = test_panic_site();

        // Spawn worker thread that owns WebSocketClient
        let handle = thread::Builder::new()
            .name("stock_ws_worker".to_string())
            .spawn(move || {
                use marketdata_core::aio::WebSocketClient as CoreClient;
                use marketdata_core::websocket::ConnectionConfig;

                let ending = ending_for_worker;
                // A connection being reused: let the previous worker finish
                // its teardown before this connection replaces it. Only the
                // worker is joined, not its stream reader, so the old
                // connection's `disconnect` callback may still arrive after
                // this connection's `connect`.
                if let Some(previous) = previous {
                    let _ = previous.join();
                }

                // Create tokio runtime
                // Multi-thread runtime so core's dispatch/writer/health-check
                // tasks keep running while the worker loop blocks on std::mpsc
                // receive_timeout. With a current_thread runtime those tasks
                // starve the moment the worker stops driving the executor,
                // causing incoming frames (subscribed, snapshot, heartbeat...)
                // to stall in tokio-tungstenite's buffer.
                let rt = match tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(2)
                    .enable_all()
                    .build()
                {
                    Ok(rt) => rt,
                    Err(e) => {
                        // No core client yet, so no event to forward: only
                        // the Promise reports it.
                        ending.store(true, Ordering::SeqCst);
                        settle(&auth, AuthOutcome::Failed(Failure::Plain(format!("Failed to create runtime: {}", e))));
                        return;
                    }
                };

                // `baseUrl` was validated in the constructor, so the only way
                // this can fail is a client built by other means — fall back to
                // production rather than kill the worker thread.
                let mut config = build_stream_config(
                    &auth_request,
                    base_url.as_deref(),
                    WsProduct::Stock,
                    stock_version,
                    futopt_version,
                )
                .unwrap_or_else(|_| {
                    ConnectionConfig::fugle_stock(auth_request.clone())
                });
                config.tls = tls_config;
                message_queue.apply(&mut config);
                let client = CoreClient::with_full_config(config, reconnect_config.clone(), health_check_config);
                // isConnected / isClosed read this connection's core state
                // from here on (#67).
                let state = client.state_handle();
                *lock_connection(&connection) = Some(ConnectionHandles {
                    state: state.clone(),
                    messages_dropped: client.messages_dropped_handle(),
                });

                // Supervised: a panic anywhere in here is reported instead of
                // leaving the connection silently dead (#25).
                let run = || {

                    // Forward core's stream from before connect(): `Connected` is
                    // emitted when the socket opens, ahead of authentication, and
                    // the authentication outcome settles the Promise (#23).
                    // Messages come through the same reader, in order (#68).
                    let dispatch_ended = Arc::new(AtomicBool::new(false));
                    spawn_stream_reader(
                        client.stream_receiver(),
                        sink.clone(),
                        Arc::clone(&auth),
                        state.clone(),
                        Arc::clone(&ending),
                        Arc::clone(&dispatch_ended),
                        test_panic.clone(),
                        Arc::clone(&decision),
                        Arc::clone(&panic_reported),
                    );

                    // Core's connect().await returns once authenticated. Its
                    // failures were already emitted as `Unauthenticated` or
                    // `Error`, which the forwarder turns into the rejection.
                    if rt.block_on(client.connect()).is_err() {
                        ending.store(true, Ordering::SeqCst);
                        return;
                    }

                    if let Some(delay) = delay_after_connect {
                        thread::sleep(delay);
                    }

                    // disconnect() arrived while authenticating, and connect() may
                    // already have started a newer worker that is joining this
                    // one: abandon the connection instead of reporting success
                    // (#44) — unless the forwarder already reported it, in which
                    // case the queued Disconnect ends it like any other.
                    if ending.load(Ordering::SeqCst) && decide(&decision, AUTH_ABORTED) {
                        let _ = rt.block_on(client.disconnect());
                        settle(&auth, AuthOutcome::Failed(connect_aborted()));
                        return;
                    }

                    // Main event loop
                    loop {
                        // Once connect() is reported resolved, not merely once
                        // core is connected, so the panic follows `authenticated`.
                        if decision.load(Ordering::SeqCst) == AUTH_REPORTED {
                            inject_test_panic(test_panic.as_deref(), "ws_worker");
                        }
                        if dispatch_ended.load(Ordering::SeqCst) {
                            // Core records its close before reporting it (#86),
                            // so only an event thread panic (#25) leaves the
                            // connection up. Closing it otherwise would overwrite
                            // the server's close in core's state with a client
                            // force close. This worker has not panicked, so
                            // `panic_reported` means the event thread did.
                            if panic_reported.load(Ordering::SeqCst) {
                                let _ = rt.block_on(client.force_close());
                            }
                            break;
                        }

                        // Wait briefly for a command, then re-check the flags.
                        match cmd_rx.recv_timeout(Duration::from_millis(50)) {
                            Ok(WsCommand::Subscribe(sub)) => {
                                let _ = rt.block_on(sub.send(&client));
                            }
                            Ok(WsCommand::Unsubscribe { ids }) => {
                                let _ = rt.block_on(client.unsubscribe(ids));
                            }
                            Ok(WsCommand::Ping { data }) => {
                                let request = marketdata_core::WebSocketRequest {
                                    event: "ping".to_string(),
                                    data,
                                };
                                let _ = rt.block_on(client.send(request));
                            }
                            Ok(WsCommand::QuerySubscriptions) => {
                                let request = marketdata_core::WebSocketRequest::subscriptions();
                                let _ = rt.block_on(client.send(request));
                            }
                            Ok(WsCommand::Disconnect) => {
                                let _ = rt.block_on(client.disconnect());
                                // Core's disconnect() emits `Disconnected` on its
                                // stream and the stream reader forwards it;
                                // firing here too duplicates the callback (#22).
                                break;
                            }
                            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                                // Command channel closed, cleanup
                                let _ = rt.block_on(client.disconnect());
                                break;
                            }
                        }
                    }
                    ending.store(true, Ordering::SeqCst);
                };
                if let Err(payload) = std::panic::catch_unwind(AssertUnwindSafe(run)) {
                    report_panic(
                        &PanicContext {
                            sink: &sink,
                            auth: &auth,
                            state: &state,
                            ending: &ending,
                            reported: &panic_reported,
                            decision: &decision,
                        },
                        "worker",
                        &*payload,
                    );
                    // The panic left core's connection up: close it. The event
                    // thread does not report its `Disconnected` again.
                    let _ = rt.block_on(client.force_close());
                }
            })
            .map_err(|e| napi::Error::from_reason(format!("Failed to spawn worker thread: {}", e)))?;

        *slot = Some(Worker { tx: cmd_tx, handle, ending });
        Ok(auth_rx)
    }

    /// Subscribe to a channel
    ///
    /// @param options - Subscription options. Provide either `symbol` (single)
    ///                  or `symbols` (batch list) — exactly one is required, matching
    ///                  the old `@fugle/marketdata` shape.
    ///                  Shape: `{ channel, symbol?, symbols?, intradayOddLot? }`
    #[napi(ts_args_type = "options: StockSubscribeOptions")]
    pub fn subscribe(&self, env: Env, options: serde_json::Value) -> napi::Result<()> {
        let channel = options
            .get("channel")
            .and_then(|v| v.as_str())
            .ok_or_else(|| napi::Error::from_reason("Missing 'channel' field"))?
            .parse::<marketdata_core::models::Channel>()
            .map_err(|e| crate::errors::to_napi_error(&env, e))?;

        let target_symbols = option_symbols("subscribe", &options)?;

        let odd_lot = options.get("intradayOddLot").and_then(|v| v.as_bool()).unwrap_or(false);
        let sub = marketdata_core::StockSubscription::new(channel, target_symbols).with_odd_lot(odd_lot);

        send_command(&self.worker, WsCommand::Subscribe(Subscription::Stock(sub)), "subscribe")
    }

    /// Unsubscribe from a channel
    ///
    /// Accepts the server id as a string, `{ id: "..." }` (single) or
    /// `{ ids: ["...", "..."] }` (batch), mirroring the old `@fugle/marketdata`
    /// Node SDK shape; or the `subscribe` options
    /// `{ channel, symbol | symbols, intradayOddLot? }`.
    #[napi(ts_args_type = "options: string | UnsubscribeOptions | StockUnsubscribeOptions")]
    pub fn unsubscribe(&self, env: Env, options: serde_json::Value) -> napi::Result<()> {
        // Accept legacy positional string for backward compat with the previous
        // `unsubscribe(id: string)` signature.
        let target_ids = match unsubscribe_ids(&env, &options)? {
            Some(ids) => ids,
            None => {
                let channel = unsubscribe_channel::<marketdata_core::models::Channel>(&env, &options)?;
                let odd_lot = options.get("intradayOddLot").and_then(|v| v.as_bool()).unwrap_or(false);
                marketdata_core::StockSubscription::new(channel, option_symbols("unsubscribe", &options)?)
                    .with_odd_lot(odd_lot)
                    .keys()
            }
        };

        send_command(&self.worker, WsCommand::Unsubscribe { ids: target_ids }, "unsubscribe")
    }

    /// Send a `ping` frame to the server.
    ///
    /// Mirrors the old `@fugle/marketdata` Node SDK. The server's `pong` reply
    /// is delivered via the `message` callback (or processed internally by the
    /// health check, if enabled).
    ///
    /// @param params - Sent as the frame's `data`, e.g. `{ state: 'x' }`, whose
    ///                 `state` the server echoes back in its pong. A string is
    ///                 accepted for compatibility and sent as `{ state }`.
    #[napi(ts_args_type = "params?: string | WebSocketPingParams")]
    pub fn ping(&self, params: Option<serde_json::Value>) -> napi::Result<()> {
        send_command(&self.worker, WsCommand::Ping { data: ping_data(params) }, "ping")
    }

    /// Ask the server for its current subscription list.
    ///
    /// Sends `{ event: "subscriptions" }` to the server. The reply is delivered
    /// asynchronously via the `message` callback, matching the old
    /// `@fugle/marketdata` Node SDK semantics.
    #[napi]
    pub fn subscriptions(&self) -> napi::Result<()> {
        send_command(&self.worker, WsCommand::QuerySubscriptions, "subscriptions")
    }

    /// Disconnect from the WebSocket server
    #[napi]
    pub fn disconnect(&self) -> napi::Result<()> {
        request_disconnect(&self.worker)
    }

    /// Messages dropped because they arrived while `messageBuffer` were
    /// unread (`messageOverflow: 'dropNewest'`).
    ///
    /// Counted from the start of the current connection (every `connect()` or
    /// reconnect restarts it); after `disconnect()` it still reads the last
    /// connection's count. 0 before the first `connect()`.
    #[napi(getter)]
    pub fn messages_dropped_total(&self) -> f64 {
        read_connection(&self.connection, |handles| handles.messages_dropped.total() as f64)
            .unwrap_or(0.0)
    }

    /// Check if connected
    ///
    /// True while the connection is authenticated; false while an
    /// auto-reconnect is in progress.
    #[napi(getter)]
    pub fn is_connected(&self) -> bool {
        read_connection(&self.connection, |handles| handles.state.is_connected()).unwrap_or(false)
    }

    /// Check if client has been closed
    ///
    /// Returns true once the connection has closed: after disconnect(), or
    /// after the server or network ended it with no reconnect left. A closed
    /// client can connect() again; isClosed turns false once the new
    /// connection starts.
    #[napi(getter)]
    pub fn is_closed(&self) -> bool {
        read_connection(&self.connection, |handles| handles.state.is_closed()).unwrap_or(false)
    }
}

/// FutOpt WebSocket client for real-time futures/options market data
///
/// # JavaScript Usage
///
/// ```javascript
/// // Event handlers
/// ws.futopt.on('message', (data) => {
///   const msg = JSON.parse(data);
///   console.log(msg);
/// });
/// ws.futopt.on('connect', () => console.log('FutOpt WebSocket connected'));
///
/// // Connect
/// ws.futopt.connect();
///
/// // Subscribe to channels
/// ws.futopt.subscribe({ channel: 'trades', symbol: 'TXFC4' });
/// ws.futopt.subscribe({ channel: 'books', symbol: 'MXFB4', afterHours: true });
/// ```
#[napi]
pub struct FutOptWebSocketClient {
    auth: marketdata_core::AuthRequest,
    base_url: Option<String>,
    stock_version: marketdata_core::websocket::StockVersion,
    futopt_version: marketdata_core::websocket::FutOptVersion,
    reconnect_config: marketdata_core::ReconnectionConfig,
    health_check_config: marketdata_core::HealthCheckConfig,
    tls_config: marketdata_core::TlsConfig,
    message_queue: MessageQueueSettings,
    callbacks: Arc<Listeners>,
    worker: WorkerSlot,
    connection: ConnectionSlot,
}

#[napi]
impl FutOptWebSocketClient {
    /// Create from pre-existing shared state (called by WebSocketClient getter).
    /// See StockWebSocketClient::from_shared for rationale.
    fn from_shared(
        auth: marketdata_core::AuthRequest,
        base_url: Option<String>,
        stock_version: marketdata_core::websocket::StockVersion,
        futopt_version: marketdata_core::websocket::FutOptVersion,
        reconnect_config: marketdata_core::ReconnectionConfig,
        health_check_config: marketdata_core::HealthCheckConfig,
        tls_config: marketdata_core::TlsConfig,
        message_queue: MessageQueueSettings,
        callbacks: Arc<Listeners>,
        worker: WorkerSlot,
        connection: ConnectionSlot,
    ) -> Self {
        Self {
            auth,
            base_url,
            stock_version,
            futopt_version,
            reconnect_config,
            health_check_config,
            tls_config,
            message_queue,
            callbacks,
            worker,
            connection,
        }
    }

    /// Register an event handler
    ///
    /// Same events and arguments as `StockWebSocketClient::on` (#23).
    #[napi(
        ts_generic_types = "E extends WebSocketEvent",
        ts_args_type = "event: E, callback: WebSocketEventMap[E]"
    )]
    pub fn on(
        &self,
        event: String,
        callback: Function<'_, EventArgs, Unknown<'static>>,
    ) -> napi::Result<()> {
        register_listener(&self.callbacks, &event, callback)
    }

    /// Connect to the FutOpt WebSocket server.
    ///
    /// Returns a Promise that resolves with the server's `authenticated`
    /// `data`. See `StockWebSocketClient::connect` for the rejections,
    /// including code `2011` (`Already connected`) (#44).
    #[napi(ts_return_type = "Promise<WebSocketAuthData | undefined>")]
    pub fn connect<'env>(&self, env: &'env Env) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        auth_promise(env, self.start_worker(env))
    }

    /// Spawn the worker thread; the receiver fires once it has authenticated.
    fn start_worker(&self, env: &Env) -> napi::Result<AuthRx> {
        // Held until the new worker is stored, so concurrent connect() calls
        // cannot both claim the slot (#44).
        let mut slot = self.worker.lock().map_err(|e| {
            napi::Error::from_reason(format!("Lock error: {}", e))
        })?;
        let previous = claim_worker_slot(&mut slot)
            .map_err(|info| crate::errors::js_error(env, &info))?;
        let sink = EventSink::new(
            env,
            Arc::clone(&self.callbacks),
            self.message_queue.in_flight_limit(),
        )?;
        let (cmd_tx, cmd_rx) = std::sync::mpsc::channel::<WsCommand>();
        let ending = Arc::new(AtomicBool::new(false));
        let decision: AuthDecision = Arc::new(AtomicU8::new(AUTH_PENDING));
        let panic_reported = Arc::new(AtomicBool::new(false));
        let delay_after_connect = test_delay_after_connect();

        let (auth_tx, auth_rx) = tokio::sync::oneshot::channel::<AuthOutcome>();
        let auth: AuthSlot = Arc::new(Mutex::new(Some(auth_tx)));

        let auth_request = self.auth.clone();
        let base_url = self.base_url.clone();
        let stock_version = self.stock_version;
        let futopt_version = self.futopt_version;
        let reconnect_config = self.reconnect_config.clone();
        let health_check_config = self.health_check_config.clone();
        let tls_config = self.tls_config.clone();
        let message_queue = self.message_queue;
        let connection = Arc::clone(&self.connection);
        let ending_for_worker = Arc::clone(&ending);
        let test_panic = test_panic_site();

        let handle = thread::Builder::new()
            .name("futopt_ws_worker".to_string())
            .spawn(move || {
                use marketdata_core::aio::WebSocketClient as CoreClient;
                use marketdata_core::websocket::ConnectionConfig;

                let ending = ending_for_worker;
                // A connection being reused: let the previous worker finish
                // its teardown before this connection replaces it. Only the
                // worker is joined, not its stream reader, so the old
                // connection's `disconnect` callback may still arrive after
                // this connection's `connect`.
                if let Some(previous) = previous {
                    let _ = previous.join();
                }

                // Multi-thread runtime so core's dispatch/writer/health-check
                // tasks keep running while the worker loop blocks on std::mpsc
                // receive_timeout. With a current_thread runtime those tasks
                // starve the moment the worker stops driving the executor,
                // causing incoming frames (subscribed, snapshot, heartbeat...)
                // to stall in tokio-tungstenite's buffer.
                let rt = match tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(2)
                    .enable_all()
                    .build()
                {
                    Ok(rt) => rt,
                    Err(e) => {
                        // No core client yet, so no event to forward: only
                        // the Promise reports it.
                        ending.store(true, Ordering::SeqCst);
                        settle(&auth, AuthOutcome::Failed(Failure::Plain(format!("Failed to create runtime: {}", e))));
                        return;
                    }
                };

                // See the stock sibling: `baseUrl` was validated in the
                // constructor, so this cannot fail for a normally-built client.
                let mut config = build_stream_config(
                    &auth_request,
                    base_url.as_deref(),
                    WsProduct::FutOpt,
                    stock_version,
                    futopt_version,
                )
                .unwrap_or_else(|_| {
                    ConnectionConfig::fugle_futopt(auth_request.clone())
                });
                config.tls = tls_config;
                message_queue.apply(&mut config);
                let client = CoreClient::with_full_config(config, reconnect_config.clone(), health_check_config);
                // isConnected / isClosed read this connection's core state
                // from here on (#67).
                let state = client.state_handle();
                *lock_connection(&connection) = Some(ConnectionHandles {
                    state: state.clone(),
                    messages_dropped: client.messages_dropped_handle(),
                });

                // Supervised: a panic anywhere in here is reported instead of
                // leaving the connection silently dead (#25).
                let run = || {

                    // Forward core's stream from before connect(): `Connected` is
                    // emitted when the socket opens, ahead of authentication, and
                    // the authentication outcome settles the Promise (#23).
                    // Messages come through the same reader, in order (#68).
                    let dispatch_ended = Arc::new(AtomicBool::new(false));
                    spawn_stream_reader(
                        client.stream_receiver(),
                        sink.clone(),
                        Arc::clone(&auth),
                        state.clone(),
                        Arc::clone(&ending),
                        Arc::clone(&dispatch_ended),
                        test_panic.clone(),
                        Arc::clone(&decision),
                        Arc::clone(&panic_reported),
                    );

                    // Core's connect().await returns once authenticated. Its
                    // failures were already emitted as `Unauthenticated` or
                    // `Error`, which the forwarder turns into the rejection.
                    if rt.block_on(client.connect()).is_err() {
                        ending.store(true, Ordering::SeqCst);
                        return;
                    }

                    if let Some(delay) = delay_after_connect {
                        thread::sleep(delay);
                    }

                    // disconnect() arrived while authenticating, and connect() may
                    // already have started a newer worker that is joining this
                    // one: abandon the connection instead of reporting success
                    // (#44) — unless the forwarder already reported it, in which
                    // case the queued Disconnect ends it like any other.
                    if ending.load(Ordering::SeqCst) && decide(&decision, AUTH_ABORTED) {
                        let _ = rt.block_on(client.disconnect());
                        settle(&auth, AuthOutcome::Failed(connect_aborted()));
                        return;
                    }

                    // Main event loop
                    loop {
                        // Once connect() is reported resolved, not merely once
                        // core is connected, so the panic follows `authenticated`.
                        if decision.load(Ordering::SeqCst) == AUTH_REPORTED {
                            inject_test_panic(test_panic.as_deref(), "ws_worker");
                        }
                        if dispatch_ended.load(Ordering::SeqCst) {
                            // Core records its close before reporting it (#86),
                            // so only an event thread panic (#25) leaves the
                            // connection up. Closing it otherwise would overwrite
                            // the server's close in core's state with a client
                            // force close. This worker has not panicked, so
                            // `panic_reported` means the event thread did.
                            if panic_reported.load(Ordering::SeqCst) {
                                let _ = rt.block_on(client.force_close());
                            }
                            break;
                        }

                        // Wait briefly for a command, then re-check the flags.
                        match cmd_rx.recv_timeout(Duration::from_millis(50)) {
                            Ok(WsCommand::Subscribe(sub)) => {
                                let _ = rt.block_on(sub.send(&client));
                            }
                            Ok(WsCommand::Unsubscribe { ids }) => {
                                let _ = rt.block_on(client.unsubscribe(ids));
                            }
                            Ok(WsCommand::Ping { data }) => {
                                let request = marketdata_core::WebSocketRequest {
                                    event: "ping".to_string(),
                                    data,
                                };
                                let _ = rt.block_on(client.send(request));
                            }
                            Ok(WsCommand::QuerySubscriptions) => {
                                let request = marketdata_core::WebSocketRequest::subscriptions();
                                let _ = rt.block_on(client.send(request));
                            }
                            Ok(WsCommand::Disconnect) => {
                                let _ = rt.block_on(client.disconnect());
                                // Core's disconnect() emits `Disconnected` on its
                                // stream and the stream reader forwards it;
                                // firing here too duplicates the callback (#22).
                                break;
                            }
                            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                                // Command channel closed, cleanup
                                let _ = rt.block_on(client.disconnect());
                                break;
                            }
                        }
                    }
                    ending.store(true, Ordering::SeqCst);
                };
                if let Err(payload) = std::panic::catch_unwind(AssertUnwindSafe(run)) {
                    report_panic(
                        &PanicContext {
                            sink: &sink,
                            auth: &auth,
                            state: &state,
                            ending: &ending,
                            reported: &panic_reported,
                            decision: &decision,
                        },
                        "worker",
                        &*payload,
                    );
                    // The panic left core's connection up: close it. The event
                    // thread does not report its `Disconnected` again.
                    let _ = rt.block_on(client.force_close());
                }
            })
            .map_err(|e| napi::Error::from_reason(format!("Failed to spawn worker thread: {}", e)))?;

        *slot = Some(Worker { tx: cmd_tx, handle, ending });
        Ok(auth_rx)
    }

    /// Subscribe to a channel
    ///
    /// @param options - Subscription options. Provide either `symbol` (single)
    ///                  or `symbols` (batch list) — exactly one is required.
    ///                  Shape: `{ channel, symbol?, symbols?, afterHours? }`
    #[napi(ts_args_type = "options: FutOptSubscribeOptions")]
    pub fn subscribe(&self, env: Env, options: serde_json::Value) -> napi::Result<()> {
        let channel = options
            .get("channel")
            .and_then(|v| v.as_str())
            .ok_or_else(|| napi::Error::from_reason("Missing 'channel' field"))?
            .parse::<marketdata_core::models::futopt::FutOptChannel>()
            .map_err(|e| crate::errors::to_napi_error(&env, e))?;

        let target_symbols = option_symbols("subscribe", &options)?;

        let after_hours = options.get("afterHours").and_then(|v| v.as_bool()).unwrap_or(false);
        let sub = marketdata_core::FutOptSubscription::new(channel, target_symbols).with_after_hours(after_hours);

        send_command(&self.worker, WsCommand::Subscribe(Subscription::FutOpt(sub)), "subscribe")
    }

    /// Unsubscribe from a channel
    ///
    /// Accepts the server id as a string, `{ id: "..." }` (single) or
    /// `{ ids: ["...", "..."] }` (batch); or the `subscribe` options
    /// `{ channel, symbol | symbols, afterHours? }`.
    #[napi(ts_args_type = "options: string | UnsubscribeOptions | FutOptUnsubscribeOptions")]
    pub fn unsubscribe(&self, env: Env, options: serde_json::Value) -> napi::Result<()> {
        let target_ids = match unsubscribe_ids(&env, &options)? {
            Some(ids) => ids,
            None => {
                let channel =
                    unsubscribe_channel::<marketdata_core::models::futopt::FutOptChannel>(&env, &options)?;
                let after_hours = options.get("afterHours").and_then(|v| v.as_bool()).unwrap_or(false);
                marketdata_core::FutOptSubscription::new(channel, option_symbols("unsubscribe", &options)?)
                    .with_after_hours(after_hours)
                    .keys()
            }
        };

        send_command(&self.worker, WsCommand::Unsubscribe { ids: target_ids }, "unsubscribe")
    }

    /// Send a `ping` frame to the server.
    ///
    /// Mirrors the old `@fugle/marketdata` Node SDK. The server's `pong` reply
    /// is delivered via the `message` callback (or processed internally by the
    /// health check, if enabled).
    ///
    /// @param params - Sent as the frame's `data`, e.g. `{ state: 'x' }`, whose
    ///                 `state` the server echoes back in its pong. A string is
    ///                 accepted for compatibility and sent as `{ state }`.
    #[napi(ts_args_type = "params?: string | WebSocketPingParams")]
    pub fn ping(&self, params: Option<serde_json::Value>) -> napi::Result<()> {
        send_command(&self.worker, WsCommand::Ping { data: ping_data(params) }, "ping")
    }

    /// Ask the server for its current subscription list.
    ///
    /// Sends `{ event: "subscriptions" }` to the server. The reply is delivered
    /// asynchronously via the `message` callback, matching the old
    /// `@fugle/marketdata` Node SDK semantics.
    #[napi]
    pub fn subscriptions(&self) -> napi::Result<()> {
        send_command(&self.worker, WsCommand::QuerySubscriptions, "subscriptions")
    }

    /// Disconnect from the WebSocket server
    #[napi]
    pub fn disconnect(&self) -> napi::Result<()> {
        request_disconnect(&self.worker)
    }

    /// Messages dropped because they arrived while `messageBuffer` were
    /// unread (`messageOverflow: 'dropNewest'`).
    ///
    /// Counted from the start of the current connection (every `connect()` or
    /// reconnect restarts it); after `disconnect()` it still reads the last
    /// connection's count. 0 before the first `connect()`.
    #[napi(getter)]
    pub fn messages_dropped_total(&self) -> f64 {
        read_connection(&self.connection, |handles| handles.messages_dropped.total() as f64)
            .unwrap_or(0.0)
    }

    /// Check if connected
    ///
    /// True while the connection is authenticated; false while an
    /// auto-reconnect is in progress.
    #[napi(getter)]
    pub fn is_connected(&self) -> bool {
        read_connection(&self.connection, |handles| handles.state.is_connected()).unwrap_or(false)
    }

    /// Check if client has been closed
    ///
    /// Returns true once the connection has closed: after disconnect(), or
    /// after the server or network ended it with no reconnect left. A closed
    /// client can connect() again; isClosed turns false once the new
    /// connection starts.
    #[napi(getter)]
    pub fn is_closed(&self) -> bool {
        read_connection(&self.connection, |handles| handles.state.is_closed()).unwrap_or(false)
    }
}

/// Forward a connection's core stream — its events and messages, in the
/// order core produced them (#68) — to the JS listeners until core closes the
/// stream (#23). Only core events are forwarded; the worker emits none of its
/// own.
///
/// The first authentication outcome settles `connect()`: `Authenticated`
/// resolves with the server's `data`, `Unauthenticated` rejects with it, and
/// an `Error` before either rejects with that error's `MarketDataError` — each
/// after its listener has run (see [`fire_and_settle`]).
///
/// A message is forwarded only between an `Authenticated` this reader
/// reported and the next `Disconnected`: frames of a rejected or abandoned
/// connection never reach `message`, and frames still arriving while
/// `disconnect()` closes the connection do, before its `disconnect`.
fn spawn_stream_reader(
    stream: Arc<marketdata_core::StreamReceiver>,
    sink: EventSink,
    auth: AuthSlot,
    state: marketdata_core::ConnectionStateHandle,
    ending: Arc<AtomicBool>,
    dispatch_ended: Arc<AtomicBool>,
    test_panic: Option<String>,
    decision: AuthDecision,
    panic_reported: Arc<AtomicBool>,
) {
    use marketdata_core::websocket::{ConnectionEvent, StreamItem};

    std::thread::spawn(move || {
        let run = || {
            let mut reported = false;
            loop {
                // A slow `message` listener holds the reader here, leaving the
                // backlog to core's queue (#46).
                sink.wait_for_room();
                let Ok(item) = stream.receive() else { break };
                let event = match item {
                    StreamItem::Event(event) => event,
                    StreamItem::Message(message) => {
                        if reported {
                            // The frame verbatim: re-serializing the routing
                            // struct would drop unknown fields and emit nulls
                            // for the ones the server omitted.
                            sink.emit_message(message.raw);
                        }
                        continue;
                    }
                    _ => continue,
                };
                match event {
                    ConnectionEvent::Connected => {
                        sink.emit("connect", EventArgs::None);
                    }
                    ConnectionEvent::Authenticated { data } => {
                        inject_test_panic(test_panic.as_deref(), "ws_events");
                        // The initial authentication: decide, atomically with the
                        // worker's abort, whether it is reported (#44).
                        if decision.load(Ordering::SeqCst) == AUTH_PENDING {
                            if ending.load(Ordering::SeqCst) {
                                // disconnect() came first. The queued Disconnect
                                // closes the connection; nothing else settles.
                                if decide(&decision, AUTH_ABORTED) {
                                    settle(&auth, AuthOutcome::Failed(connect_aborted()));
                                }
                                continue;
                            }
                            if !decide(&decision, AUTH_REPORTED) {
                                continue; // the worker aborted first
                            }
                        } else if decision.load(Ordering::SeqCst) == AUTH_ABORTED || ending.load(Ordering::SeqCst) {
                            continue; // a re-authentication after the connection was given up
                        }
                        reported = true;
                        fire_and_settle(
                            &sink,
                            "authenticated",
                            EventArgs::Json(data.clone()),
                            &auth,
                            AuthOutcome::Authenticated(data),
                            None,
                        );
                    }
                    ConnectionEvent::Unauthenticated { data, .. } => {
                        reported = false;
                        fire_and_settle(
                            &sink,
                            "unauthenticated",
                            EventArgs::Json(data.clone()),
                            &auth,
                            AuthOutcome::Rejected(data),
                            Some(&ending),
                        );
                    }
                    ConnectionEvent::Error(info) => {
                        let rejection = AuthOutcome::Failed(Failure::Coded(info.clone()));
                        fire_and_settle(
                            &sink,
                            "error",
                            EventArgs::Error(info),
                            &auth,
                            rejection,
                            Some(&ending),
                        );
                    }
                    ConnectionEvent::Disconnected { code, reason, will_reconnect, .. } => {
                        reported = false;
                        if panic_reported.load(Ordering::SeqCst) {
                            // The worker closing the connection after a panic
                            // that already reported its end (#25).
                            continue;
                        }
                        if !will_reconnect {
                            ending.store(true, Ordering::SeqCst);
                            // Core's dispatch task ends without reconnecting.
                            dispatch_ended.store(true, Ordering::SeqCst);
                        }
                        sink.emit(
                            "disconnect",
                            EventArgs::Json(serde_json::json!({ "code": code, "reason": reason })),
                        );
                    }
                    ConnectionEvent::MessagesDropped { dropped, total } => {
                        sink.emit(
                            "messagesDropped",
                            EventArgs::Json(serde_json::json!({ "dropped": dropped, "total": total })),
                        );
                    }
                    ConnectionEvent::Reconnecting { attempt } => {
                        sink.emit(
                            "reconnect",
                            EventArgs::Json(serde_json::json!({ "attempt": attempt })),
                        );
                    }
                    ConnectionEvent::ReconnectFailed { attempts } => {
                        ending.store(true, Ordering::SeqCst);
                        sink.emit(
                            "error",
                            EventArgs::Error(ErrorInfo::new(
                                error_code::RECONNECT_FAILED,
                                ErrorKind::Network,
                                format!("Reconnection failed after {} attempts", attempts),
                            )),
                        );
                        // Core's dispatch task has ended for good.
                        dispatch_ended.store(true, Ordering::SeqCst);
                    }
                    // `HeartbeatTimeout` is followed by `Disconnected`, which reports it.
                    _ => {}
                }
            }
        };
        if let Err(payload) = std::panic::catch_unwind(AssertUnwindSafe(run)) {
            report_panic(
                &PanicContext {
                    sink: &sink,
                    auth: &auth,
                    state: &state,
                    ending: &ending,
                    reported: &panic_reported,
                    decision: &decision,
                },
                "event",
                &*payload,
            );
            // Nothing reports the connection's end any more, and the worker
            // only exits once told core stopped: shut the connection down
            // rather than leave it (and the process) running unobserved.
            dispatch_ended.store(true, Ordering::SeqCst);
        }
    });
}

/// What a panicked worker or event thread needs to report it (#25).
struct PanicContext<'a> {
    sink: &'a EventSink,
    auth: &'a AuthSlot,
    state: &'a marketdata_core::ConnectionStateHandle,
    ending: &'a AtomicBool,
    /// Shared by the connection's worker and event thread, so a panic on
    /// both reports one `error`.
    reported: &'a AtomicBool,
    decision: &'a AtomicU8,
}

/// Report a panic on a connection's `thread` so the connection does not go
/// silently dead (#25): fire `error` (code -1) — rejecting `connect()` after
/// it unless its authentication was already reported — and `disconnect` if
/// it was reported and core has not closed it. The worker closes the core
/// connection.
/// Only the first panic of a connection fires them.
///
/// Both events go through the [`EventSink`], so each carries a keep-alive
/// clone and is delivered before the process may exit (#30). `ending` is set
/// first, so connect() may be called again from either listener (#44).
fn report_panic(ctx: &PanicContext<'_>, thread: &str, payload: &(dyn std::any::Any + Send)) {
    let detail = payload
        .downcast_ref::<&str>()
        .map(|message| message.to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "unknown panic".to_string());
    let reason = format!("WebSocket {} thread panicked: {}", thread, detail);

    ctx.ending.store(true, Ordering::SeqCst);
    // The other thread already reported this connection's end.
    if ctx.reported.swap(true, Ordering::SeqCst) {
        return;
    }

    let info = ErrorInfo::new(error_code::THREAD_PANIC, ErrorKind::Protocol, reason.clone());
    let error = EventArgs::Error(info.clone());
    // Reject connect() only if its authentication has not been reported yet,
    // taking that decision so it will not be (#44); once `authenticated` has
    // fired, connect() resolves and this is a disconnect like any other.
    if decide(ctx.decision, AUTH_ABORTED) {
        fire_and_settle(
            ctx.sink,
            "error",
            error,
            ctx.auth,
            AuthOutcome::Failed(Failure::Coded(info)),
            None,
        );
        return;
    }
    ctx.sink.emit("error", error);
    if !ctx.state.is_closed() {
        ctx.sink.emit(
            "disconnect",
            EventArgs::Json(serde_json::json!({ "code": null, "reason": reason })),
        );
    }
}

/// `FUGLE_MARKETDATA_TEST_PANIC`, naming where a test wants a WebSocket
/// thread to panic (#25). Read on the JS thread when connecting, so tests can
/// set it through `process.env`. Debug builds only.
#[cfg(debug_assertions)]
fn test_panic_site() -> Option<String> {
    std::env::var("FUGLE_MARKETDATA_TEST_PANIC").ok()
}

#[cfg(not(debug_assertions))]
fn test_panic_site() -> Option<String> {
    None
}

/// Panic if the test asked for one at `here` (`ws_worker`, `ws_events`).
#[cfg(debug_assertions)]
fn inject_test_panic(site: Option<&str>, here: &str) {
    if site == Some(here) {
        panic!("injected test panic at {}", here);
    }
}

#[cfg(not(debug_assertions))]
#[inline(always)]
fn inject_test_panic(_site: Option<&str>, _here: &str) {}

/// Emit `event`, then settle the pending `connect()` with `outcome` once the
/// listener has returned, so it runs before the Promise settles as it did in
/// 1.x, which emitted before settling (#23). Nothing is settled if
/// `connect()` already was.
///
/// `ending`, when given and `connect()` is still pending, is set before the
/// event is emitted, so calling connect() again from the listener or the
/// rejection is not refused as already connected (#44).
fn fire_and_settle(
    sink: &EventSink,
    event: &'static str,
    data: EventArgs,
    auth: &AuthSlot,
    outcome: AuthOutcome,
    ending: Option<&AtomicBool>,
) {
    let pending = auth.lock().map(|guard| guard.is_some()).unwrap_or(false);
    if !pending {
        sink.emit(event, data);
        return;
    }
    if let Some(ending) = ending {
        ending.store(true, Ordering::SeqCst);
    }
    let auth = Arc::clone(auth);
    sink.emit_then(event, data, move || settle(&auth, outcome));
}

// Unit tests are disabled because ThreadsafeFunction requires Node.js runtime
// Integration tests are done via JavaScript (test_websocket.js)

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    /// Whether `wait_for_room()` returns within `timeout`.
    fn room_within(in_flight: &Arc<InFlight>, timeout: Duration) -> mpsc::Receiver<()> {
        let (tx, rx) = mpsc::channel();
        let waiter = Arc::clone(in_flight);
        std::thread::spawn(move || {
            waiter.wait_for_room();
            let _ = tx.send(());
        });
        assert!(
            rx.recv_timeout(timeout).is_err(),
            "room while the only slot was taken"
        );
        rx
    }

    #[test]
    fn a_call_discarded_without_running_gives_back_its_in_flight_slot() {
        // napi drops a queued call's closure without running it when the
        // threadsafe function is torn down with the environment.
        let in_flight = Arc::new(InFlight::new(Some(1)));
        let permit = in_flight.acquire();
        let call = move || drop(permit);
        let room = room_within(&in_flight, Duration::from_millis(200));
        drop(call);
        room.recv_timeout(Duration::from_secs(5))
            .expect("a discarded call kept its slot, so the reader would wait forever");
    }

    #[test]
    fn unbounded_never_waits() {
        let in_flight = Arc::new(InFlight::new(None));
        let _permits: Vec<_> = (0..8).map(|_| in_flight.acquire()).collect();
        in_flight.wait_for_room();
    }
}
