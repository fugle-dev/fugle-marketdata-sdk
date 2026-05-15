# WebSocket Server Recommendations

A proposal for the Fugle market data WebSocket server team. This document collects connection-liveness, error-detection, and observability improvements that the SDK team would like the server to support, along with the rationale and priority for each.

> This is a derived view of an internal SDK roadmap. The full design (including parallel client-side changes) lives in the SDK team's planning docs. Source of truth: `.claude/plans/distributed-tumbling-rabbit.md` in the SDK contributor's working tree.

---

## Background

### What the server does today

- Sends `{"event":"heartbeat", "data":{"time":<microseconds>}}` to every active connection every 30 seconds.
- Sends `subscribed`, `data`, `error`, `authenticated`, `pong`, etc. as documented protocol events.
- Has no other connection liveness mechanism beyond this server-driven heartbeat.

### What the SDK does today

- Tracks "time since last inbound frame" (heartbeat counts, but so does any other event).
- Disconnects if 90 seconds pass with no inbound frame at all (default `30s × 3` policy).
- The 90s default is loose; Databento's reference implementation uses `30s + 5s = 35s`.

### What's about to change on the SDK side (context for these recommendations)

The SDK is moving to a tighter, async-native liveness model:
- Read-site `tokio::time::timeout(35s, ...)` replaces the polling task — no background timer, no atomics
- A new `HeartbeatTimeout` first-class error event surfaces explicitly to user code
- Default behaviour switches to "always on" instead of opt-in
- Auth payload will additionally carry an optional `heartbeat_interval_ms` field as a client preference (server may honor or ignore — see recommendation 3 below)

These client-side changes are non-breaking from the server's perspective: nothing on the wire changes that the server can't already handle. The server-side recommendations below are about what additional work on the **server** would multiply the value of those client changes.

---

## Recommendations

Ordered by recommended sequencing (priority + effort tradeoff). The first three are strongly recommended; the rest are conditional on metrics or future requirements.

### 1. TCP keepalive on listening sockets ⭐ (lowest effort, biggest backstop)

Set `SO_KEEPALIVE` plus tuned timing on every accepted client socket. The OS will detect dead TCP connections (route blackhole, client machine crash without a FIN, NAT timeout) and free server-side resources.

```
SO_KEEPALIVE       = on
TCP_KEEPIDLE       = 60s    # idle 60s before first probe
TCP_KEEPINTVL      = 10s    # interval between probes
TCP_KEEPCNT        = 3      # 3 failed probes -> RST
# Effective dead-TCP detection: ~90s
```

**Why it matters**

- Linux's default `TCP_KEEPIDLE = 7200s` (2 hours) means a stale subscriber connection ties up server state — subscription registration, dispatch loop slot, send buffer — for hours after the client is actually gone.
- This is purely an OS-level setting. **No protocol change.** All existing clients benefit immediately on next reconnect.
- It's also the safety net beneath every other mechanism in this document. If application-level heartbeat logic has a bug, the OS still cleans up the socket.

**Effort**

Tens of lines in the listener / accept loop. The exact API depends on the server's tech stack — see [Open questions](#open-questions-for-server-team).

**Blocking caveat**

Containerized deployments sometimes have these settings overridden at the host level. Verify the values actually take effect on production sockets — `ss -ti | grep keepalive` is the standard check.

---

### 2. Add a sequence number to heartbeat frames

Per-connection counter. Each heartbeat sent on a connection carries an incrementing `seq`:

```json
{"event":"heartbeat", "data":{"time": 1709876543000000, "seq": 4521}}
```

**Why it matters**

The SDK can detect missed heartbeats specifically (not just "any frame gap"). This produces a quantifiable metric: heartbeat delivery rate per connection. If the server-side metric reports "we sent 1000 heartbeats" and the client received "997", that's a 0.3% network loss rate that's invisible today.

**Why per-connection (not global)**

A global heartbeat counter would let one connection see another's seq jumps and falsely conclude there's a gap. Per-connection is the natural scope.

**Effort**

One `AtomicU64` per connection state object; increment-and-include in the heartbeat send path. The SDK side is similarly cheap.

---

### 3. Honor a client-requested `heartbeat_interval` ⭐⭐ (unlocks client-side feature)

Allow clients to negotiate their preferred heartbeat frequency at auth time, with the server clamping to safe bounds.

**Wire change** — additive, optional field in the auth request:

```json
{
  "event": "auth",
  "data": {
    "apikey": "...",
    "heartbeat_interval_ms": 30000
  }
}
```

**Server behaviour**:

1. Read `data.heartbeat_interval_ms` if present
2. Clamp to `[5000, 300000]` (5s minimum to prevent DoS by request flooding; 5min maximum to prevent clients from effectively disabling liveness detection)
3. If absent or invalid, fall back to current default (30s)
4. Apply the negotiated interval to that connection's heartbeat task period
5. **Echo the actual applied value** in the `authenticated` response:

```json
{
  "event": "authenticated",
  "data": {
    "heartbeat_interval_ms": 30000
  }
}
```

The echo is critical: if the server clamps a request from `1000` (client asked for 1 second) up to `5000`, the client must know — otherwise the client will use its requested 1s as the basis for its own timeout calculation and disconnect immediately every interval.

**Why it matters**

- Same server can serve HFT clients (who want 5s heartbeats for tight detection) and mobile/casual clients (who want 60s for battery/bandwidth) without forking endpoints
- This is the design used by [Databento's live API](https://github.com/databento/databento-rs/blob/main/src/live/protocol.rs) (`heartbeat_interval_s` in their auth string)
- Without this, the SDK's `heartbeat_interval` builder method (which the SDK already needs internally) cannot be exposed to end users — the field would silently do nothing

**Effort**

Auth handler: tens of lines (parse, validate, store on connection state). Heartbeat task: receive the per-connection interval instead of a global constant. Total: about a day of work, including tests.

**Coordination with SDK release**

The SDK team will hold off on exposing the corresponding builder method (`with_heartbeat_interval`) to end users until this server change is deployed. Once both ship together, users can opt into tighter or looser heartbeats per connection.

---

### 4. (Conditional) Per-stream sequence numbers on data frames

Add `seq` to every `data` event, scoped per `(channel, symbol)`:

```json
{"event":"data", "channel":"trades", "symbol":"2330", "seq":12345, "data":{...}}
```

The SDK then tracks `last_seq` per stream and can detect actual data loss (not just connection-layer gaps).

**Why this is conditional**

Adding sequence numbers without snapshot recovery is half the value: clients can detect a gap but can't recover from it. Full value requires the server to also buffer the last N data frames per stream so a reconnecting or gap-detecting client can request `replay_from_seq=12340` and have the missed frames replayed.

That's a significant server-side change: per-stream ring buffer, persistence guarantees on those buffers, replay protocol path. Not justified unless production metrics show meaningful data loss after recommendations 1–3 are deployed.

**Defer until metrics justify it.** Possible signal: client-side heartbeat-gap rate (from recommendation 2) is non-trivially higher than zero, suggesting the network path itself is dropping frames.

---

### 5. (Conditional) Server-side dead-client detection beyond TCP keepalive

Recommendation 1 (TCP keepalive) catches socket-layer death (peer machine off, NAT timeout, route black hole). It does not catch the rarer case where the client process is hung but the socket is still ack'd by the kernel — the connection looks alive at TCP layer, just nobody's reading data.

If this turns out to be a real problem (look for: sustained high open-connection counts during deploy windows where client crashes are common), two options exist:

**Option A — Application-layer client heartbeat** (Phase 3 in the SDK roadmap): require clients to actively send a `client_heartbeat` event periodically. Strong but **breaking** for all existing clients (including third-party ones); needs protocol version negotiation. Default not recommended.

**Option B — RFC 6455 protocol-level Ping**: server sends `Message::Ping` (WebSocket control frame, opcode 0x9) every 60s. Standard-compliant client stacks (browsers, `tokio-tungstenite`, Python `websockets`, Node `ws`) auto-pong without application-layer code. Server detects missed pongs and closes the connection.

Option B is non-breaking and probably sufficient. **Defer until metrics justify either.**

---

## Suggested rollout sequence

```
Sprint  ─── Action ──────────────────────────────────  Risk    Effort
  N      Recommendation 1: TCP keepalive                Low     Low
  N      Recommendation 2: heartbeat seq number          Low     Low
  N+1    Recommendation 3: honor heartbeat_interval      Low     Med
  ----- (collect metrics for 2-4 weeks) -----
  ?      Recommendation 4: data seq + replay            Med     High      (only if data loss observed)
  ?      Recommendation 5 Option B: RFC ping            Low     Med       (only if hung clients observed)
```

Recommendations 1–3 can ship in the same release. Recommendation 3 has the most coupling to client-side work — coordinate the deployment so the SDK release with the user-facing builder method lands shortly after.

---

## Open questions for server team

These need server team input before some of the recommendations above can be priced or sequenced:

1. **Tech stack** — Go / Node / Rust / Java / something else? This determines the exact API for setting `SO_KEEPALIVE` and friends.

2. **Heartbeat task topology** — Is heartbeat sent by a per-connection timer, or by a single ticker that fans out to all open connections? Per-connection makes recommendation 3 (variable interval per client) trivial. Fan-out makes it harder.

3. **Stale connection observability** — Does the server already report metrics like "currently open connections", "connections idle > 5 min", "connections closed due to TCP timeout"? These metrics are how we'll know whether recommendations 4 and 5 are necessary.

4. **Protocol version negotiation** — Is there an existing mechanism for clients to declare which protocol version they support? This is a prerequisite for any future breaking change (Phase 3).

5. **RFC 6455 control frame handling** — Does the WebSocket library in use already handle Ping/Pong control frames automatically (it should, by spec), or is there custom code that intercepts them? Confirms recommendation 5 Option B feasibility.

6. **Containerization caveats for keepalive** — If running in Kubernetes / Docker / etc., are there node-level overrides on `tcp_keepalive_*` sysctls that would prevent application settings from taking effect? Same question for any fronting load balancer (which may close idle connections on its own clock regardless of what the application does).

---

## References

- [Databento Rust live client — `client.rs`](https://github.com/databento/databento-rs/blob/main/src/live/client.rs) — read-site timeout pattern; `heartbeat_interval` builder
- [Databento Rust live client — `protocol.rs`](https://github.com/databento/databento-rs/blob/main/src/live/protocol.rs) — wire format for `heartbeat_interval_s` in auth
- [RabbitMQ docs — Detecting Dead TCP Connections with Heartbeats and TCP Keepalives](https://www.rabbitmq.com/docs/heartbeats) — clearest published explanation of the layered approach (heartbeat + keepalive)
- RFC 6455 §5.5.2 — WebSocket Ping/Pong control frames

---

*Contact: SDK team. Questions or counter-proposals welcome — this is a starting point, not a fixed spec.*
