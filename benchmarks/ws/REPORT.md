# WebSocket Benchmark Report

Rust-core SDK vs legacy pure-JS / pure-Python SDKs, plus cross-language comparison.

Benchmark date: 2026-09-20 (previous run: 2026-04-12)
Hardware: Apple M3 Pro (macOS 26.5), localhost loopback
SDK under test: `main` @ `de76eca` (after #199 / #201 / #202 / #203 / #204 / #209), built from this tree (the rerun was built from `bc36960`, which differs from it only under `benchmarks/`)
Raw output: [`results/2026-09-20/`](results/2026-09-20/) (per-run JSON + console log per language, `environment.txt`)

The 10K and 50K tables come from the evening rerun in
[`results/2026-09-20/cpu-rerun/`](results/2026-09-20/cpu-rerun/), made after
#215 changed the Go, C++ and Java clients to report process-wide CPU (see
[Metrics](#metrics)); throughput, latency and CPU in each table are from the
same runs. The morning run (`10k-*` / `50k-*` in the parent directory, same
tree, same machine) is kept for reference: its throughput and latency agree
with the rerun within noise (Java 10K is the one exception, +15%, see
[Per-run data](#per-run-data)), but its Go/C++/Java CPU column is not CPU.
The rate-limited runs and the May-snapshot control were not repeated.

The **Heavy burst (50K)** table and Key Takeaway 3 were updated on
2026-09-22 after #236 (core parses `WebSocketMessage.data` lazily), from
[`results/2026-09-22/`](results/2026-09-22/) (`main` @ `e0fc442`, same
machine and clients, Java not rerun). The 10K tables above are still the
2026-09-20 figures; the 2026-09-22 10K numbers are quoted in Key Takeaway 3.

All numbers are the median of 3 runs; per-run values are listed in
[Per-run data](#per-run-data). "vs Apr" compares against the 2026-04-12 report
(`git show de76eca:benchmarks/ws/REPORT.md`), which ran on a different Apple
Silicon machine, so only differences well beyond run-to-run noise (a few
percent) mean anything.

## Summary (10K burst)

### JavaScript (Node.js)

| Metric | Old SDK (`@fugle/marketdata@1.6.0`) | New SDK (Rust core) | Delta | vs Apr (old / new) |
|--------|-------------------------------------|---------------------|-------|--------------------|
| Throughput | 196,078 msg/s | 178,571 msg/s | **-8.9%** | -5.9% / -3.6% |
| Latency p50 | 0 ms | 0 ms | -- | -- |
| Latency p99 | 2 ms | 1 ms | -- | -- |
| CPU user | 64 ms | 135 ms | +111% | -14% / **+90%** |

April's old SDK was `@fugle/marketdata@1.4.2`; 1.6.0 is the npm `latest`.

### Python

| Metric | Old SDK (`fugle-marketdata==2.7.0rc1`) | New SDK (Rust core) | Delta | vs Apr (old / new) |
|--------|----------------------------------------|---------------------|-------|--------------------|
| Throughput | 25,839 msg/s | 166,666 msg/s | **+545%** | -0.3% / **+60.0%** |
| Latency p50 | 188 ms | 3 ms | **-98%** | +3% / **-83%** |
| Latency p99 | 367 ms | 9 ms | **-98%** | +0.3% / **-84%** |
| CPU user | 736 ms | 135 ms | -82% | -29% / -32% |

April's old SDK was `fugle-marketdata==2.4.1`; 2.7.0rc1 is the version the
PM compares against (2.6.0 is the newest stable release on PyPI). The Python
CPU figures are now user time only; April's and the morning run's
`time.process_time()` included system time (33 ms old / 53 ms new here). For
the new SDK that is the whole difference to the morning run (190 ms then,
135 + 53 now). The old SDK's 937 ms then vs 736 + 33 now is a wider gap than
its run-to-run spread (728-819 ms in these three runs); not investigated.

### C# (.NET 8)

No legacy C# SDK exists, so results are absolute (cross-language comparison only).

| Metric | New SDK (Rust core / UniFFI) | vs Apr |
|--------|:---------------------------:|:------:|
| Throughput | **185,185 msg/s** | +5.6% |
| Latency p50 | 0 ms | 2 → 0 ms |
| Latency p99 | 3 ms | 4 → 3 ms |
| CPU user | 156 ms | +70% |

### Go

No legacy Go SDK exists, so results are absolute (cross-language comparison only).

| Metric | New SDK (Rust core / UniFFI) | vs Apr |
|--------|:---------------------------:|:------:|
| Throughput | **181,818 msg/s** | **+14.5%** |
| Latency p50 | 1 ms | 4 → 1 ms |
| Latency p99 | 3 ms | 11 → 3 ms (**-73%**) |
| CPU user | 151 ms | n/a (April's 86 ms was wall clock, #215) |

### Java (JDK 21)

No legacy Java SDK exists, so results are absolute (cross-language comparison only).

| Metric | New SDK (Rust core / UniFFI+JNA) | vs Apr |
|--------|:-------------------------------:|:------:|
| Throughput | **25,641 msg/s** | **+21.5%** |
| Latency p50 | 293 ms | -14% |
| Latency p99 | 423 ms | -17% |
| CPU user | 1,926 ms (user + system, see note) | n/a (April's 98 ms was the main thread only, #215) |

The Java CPU figure is the whole process (`getProcessCpuTime()`), which is
the only process-level figure the JDK offers and includes system time and
the JIT compiler threads; the other five clients report user time only.

### C++ (C++20)

No legacy C++ SDK exists, so results are absolute (cross-language comparison only).

| Metric | New SDK (Rust core / UniFFI+C++) | vs Apr |
|--------|:-------------------------------:|:------:|
| Throughput | **172,413 msg/s** | +6.9% |
| Latency p50 | 0 ms | -- |
| Latency p99 | 2 ms | 3 → 2 ms |
| CPU user | 124 ms | n/a (April's 81 ms was wall clock, #215) |

### Cross-Language Comparison (New Rust-core SDK only, 10K burst)

| Metric | C# (UniFFI) | Go (UniFFI) | JS (napi-rs) | C++ (UniFFI) | Python (PyO3) | Java (UniFFI+JNA) |
|--------|:-----------:|:-----------:|:------------:|:------------:|:-------------:|:-----------------:|
| Throughput | 185,185 msg/s | 181,818 msg/s | 178,571 msg/s | 172,413 msg/s | 166,666 msg/s | 25,641 msg/s |
| Latency p50 | 0 ms | 1 ms | 0 ms | 0 ms | 3 ms | 293 ms |
| Latency p99 | 3 ms | 3 ms | 1 ms | 2 ms | 9 ms | 423 ms |
| CPU user | 156 ms | 151 ms | 135 ms | 124 ms | 135 ms | 1,926 ms\* |
| CPU system | 35 ms | 35 ms | 62 ms | 43 ms | 53 ms | -- |
| % of null client (192,308 msg/s) | 96% | 95% | 93% | 90% | 87% | 13% |

\* User + system (`getProcessCpuTime()`), including the JIT compiler threads;
the JDK has no process-level user-only figure. The other five are user time
only, process-wide (all threads, including the Rust runtime's).

The last row compares against a null client (`js/bench-null.js`: a raw `ws`
socket that counts frames and parses nothing), which is the most one consumer
can take from this mock server on this machine: 192,308 msg/s at 10K,
263,158 at 50K (`results/2026-09-20/cpu-rerun/*-null.jsonl`). The legacy JS
SDK is at that ceiling (196,078, within noise of it). The five fast bindings
are within 4-13% of the ceiling, so the spread between them is real but
small; Java is the only binding that is clearly client-bound.

The CPU columns are close for the five fast bindings (124-156 ms user for
11,000 frames, i.e. 11-14 µs per message including connect, warmup and the
listener's own JSON parse) and the ranking does not follow throughput: C++
(a string scan in the callback, no JSON parser) is the cheapest, C# and Go
(a full JSON parse in the callback) the most expensive, JS and Python in
between. JS spends the most system time (62 ms): every frame crosses two
thread boundaries there (core queue → reader thread → `ThreadsafeFunction` →
JS thread), see Key Takeaway 3.

### Heavy burst (50K)

2026-09-22, after #236 (`results/2026-09-22/50k-*`); "before" is the
2026-09-20 rerun (`results/2026-09-20/cpu-rerun/50k-*`).

| Metric | JS old | JS new | Py old | Py new | C# | Go | Java† | C++ |
|--------|:------:|:------:|:------:|:------:|:--:|:--:|:----:|:---:|
| Throughput (msg/s) | 253,807 | 227,273 | 25,920 | 174,216 | 251,256 | 230,414 | 32,154 | 233,644 |
| before #236 | 264,550 | 233,645 | 26,497 | 177,935 | 218,340 | 203,252 | | 246,305 |
| Latency p50 (ms) | 0 | 0 | 934 | 48 | **0** | **10** | 889 | 0 |
| before #236 | 0 | 0 | 922 | 49 | 19 | 28 | | 0 |
| Latency p99 (ms) | 1 | 1 | 1,860 | 86 | **3** | **24** | 1,440 | 2 |
| before #236 | 1 | 1 | 1,819 | 100 | 40 | 64 | | 1 |
| CPU user (ms) | 192 | **296** | 2,335 | **326** | **332** | **350** | 3,652\* | **212** |
| before #236 | 188 | 336 | 2,256 | 360 | 406 | 420 | | 298 |
| CPU system (ms) | 58 | 231 | 138 | 197 | 107 | 104 | -- | 148 |
| before #236 | 56 | 202 | 135 | 168 | 84 | 87 | | 118 |
| % of null client (259,067 msg/s) | 98% | 88% | 10% | 67% | 97% | 89% | 12% | 90% |

\* User + system, see the cross-language table.
† Not rerun for #236 (2026-09-20 figures); Java's `onMessage` parses
`dataJson` like C# and Go, so it takes the same core change.

JS old→new: -10.5%. Python old→new: +572%. No run lost a message (`lost` is
0 in every JSON) and no new-SDK client dropped one (`dropped` is 0). The
controls did not move: the legacy JS SDK is within 4% of its 2026-09-20
figures and the null client reaches 259K msg/s (263K then).

With 50K the null client reaches ~260K msg/s (the per-message cost of the
Node `ws` server and consumer amortises better) and the ranking separates.
Before #236, C++ and JS kept up (p99 1 ms), C# and Go fell behind by
19-28 ms at p50 (their queue filled while the reader thread turned each
`Value` tree back into `dataJson` and the callback parsed it), Python by
49 ms, Java by ~0.9 s. With `dataJson` now the frame's own slice, C# keeps up
(p50 0, p99 3 ms, 97% of the null client) and Go's p50 is 10 ms. Per frame
(51,000 including warmup) the user CPU is 4-7 µs for the fast bindings at
this size (C++ 4.2, JS 5.8, Python 6.4, C# 6.5, Go 6.9; before #236 5.8-8.2),
against 3.8 µs for the legacy JS SDK.

### Rate-limited (`rate=500`, 5K messages)

`ws-mock-server.js` implements the rate with a 2 ms `setInterval` sending
5-message batches, which on this machine delivers ~2,000 msg/s rather than
500; that is still the order of a busy live feed. No client falls behind
(p50 0 ms everywhere, `lost` 0); the JS clients' lower throughput figure is a
longer first-to-last elapsed with no missing frames, not investigated further.

| Metric | JS old | JS new | Py old | Py new | C# | Go | Java | C++ |
|--------|:------:|:------:|:------:|:------:|:--:|:--:|:----:|:---:|
| Throughput (msg/s) | 1,723 | 1,825 | 1,953 | 2,014 | 1,966 | 1,970 | 2,069 | 1,967 |
| Latency p50 (ms) | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| Latency p99 (ms) | 5 | 8 | 2 | 1 | 1 | 1 | 63 | 1 |
| Latency max (ms) | 10 | 12 | 9 | 1 | 4 | 3 | 82 | 3 |

Raw data: `results/2026-09-20/rate500-<lang>.json`.

## Key Takeaways

1. **Python: the Rust core is now 6.5x the pure-Python SDK** (was 4x in
   April). The old SDK is unchanged (25.8K vs 25.9K msg/s), the new one went
   from 104K to 167K msg/s and p50 from 18 ms to 3 ms. The machine explains
   at most ~15% of that: the Go client built from the earliest commit in this
   repo (`14fb9f6`, the 2026-05-15 v0.3.0 snapshot) runs at 178-189K on
   today's machine, the same as today's Go, and Go is +14.5% vs April. So most
   of the Python gain is code, and code from after `14fb9f6` (the control only
   covers May→September; April→May is not in this repo's history). The Python
   and JS clients from that snapshot panic on `connect()` (`no reactor
   running` in `aio/client.rs` `message_stream()`), so there is no runnable
   good end for a bisect; the candidates on the timeline are the core
   delivery-queue rewrite (`0f9894e` core-owned queue, `6079d5c` single
   ordered stream replacing the `receive_timeout(100ms)` poll) and the Python
   message-thread rewrite (`6685dce`). Tracked in #214.

2. **JS: the -10% gap to the legacy SDK is unchanged** (-8.9% at 10K and
   -11.7% at 50K, vs -11.1% in April; -10.5% at 50K after #236) even though
   the "redundant serde cycle" April blamed is gone: the binding now hands
   the frame to the listener verbatim (`message.raw`, no
   `serde_json::to_string`). The legacy SDK is a
   raw `ws` socket plus an EventEmitter and runs at the null client's
   ceiling; the new binding is at 93% (10K) / 89% (50K), and the difference
   is the extra hops: tokio reader thread → core queue → reader thread →
   `ThreadsafeFunction` → JS thread. Latency is identical (p50 0 ms, p99
   1-2 ms, 10K and 50K).

3. **Every Rust-core binding but Java costs about twice the legacy JS SDK's
   user CPU, and the cost is in core's shared path, not in any one binding**
   (#214). #236 (lazy `data`) took 5-16% of it off at 10K and 10-29% at
   50K; the rest is the architecture (see the end of this item). April
   read this as a JS regression (71 → 135 ms for 10K, while the
   old SDK went 74 → 64 ms on the same machine), but with all six clients
   measuring the same thing (see (6)) JS is not an outlier: C++ 124, JS 135,
   Python 135, Go 151, C# 156 ms against the legacy SDK's 64 (Java's 1,926 ms
   is its own story, see (5)). April→now is still not bisectable (see (1)),
   so instead the new JS client was profiled on this tree, with the results
   under [`results/2026-09-20/profile-214/`](results/2026-09-20/profile-214/):

   - **Per thread** (200K frames, `ps -M`, medians of 3): the legacy SDK does
     everything on the JS thread (0.59 s user). The new SDK's JS thread uses
     *less* (0.44 s: it no longer decodes WebSocket frames), and the rest is
     Rust: 0.46 s on the tokio worker running core's read loop and 0.16 s on
     the thread that moves items from core's queue to the
     `ThreadsafeFunction`. Per frame that is 2.3 µs + 0.8 µs of Rust on top
     of 2.2 µs of JS.
   - **Inside the read loop** (`sample`, share of the loop's on-CPU samples):
     ~26% the `recvfrom` syscall (system time) and ~18% tungstenite's frame
     decode, ~35% `parse_text_frame`, ~13% the queue push (mostly its
     `notify_all` syscall). The parse is the largest user-time item, and 80%
     of it is building `WebSocketMessage.data` as a `serde_json::Value` tree
     (an `IndexMap` per object, `preserve_order`); dropping that tree is
     another third of the queue-reader thread's CPU. JS hands `raw` to the
     listener and never reads the tree; Python re-parses `raw` into the dict
     it hands the callback (so it builds the tree twice); the UniFFI bindings
     only re-serialise it into `data_json`.
   - **What it would buy**: a measurement-only prototype that keeps `data` as
     the verbatim JSON slice for data events (`prototype-data-rawvalue.patch`,
     3 runs each, JS/Go/C# measured) takes 6-8% off user CPU at 10K and
     13-16% at 50K (JS 336 → 292, Go 420 → 355, C# 406 → 340 ms). System
     time rises at the same time (5-7 ms at 10K, 13-22 ms at 50K: the
     threads idle and wake more often), so the process total only drops
     1.5-4% at 10K and 4-10% at 50K. For the UniFFI bindings the prototype
     also removes the Value → string round trip on the reader thread, which
     is most of what was queueing them at 50K: Go p50 28 → 10 ms and C# 19 →
     1 ms, throughput +13-15%. JS throughput and latency are unchanged (its
     reader thread was not the bottleneck).
   - **What #236 bought** (2026-09-22, `main` @ `e0fc442`,
     [`results/2026-09-22/`](results/2026-09-22/)): core now keeps the byte
     range of `data` in `raw` and builds the `Value` only when `data()` is
     called; `dataJson` is that slice. User CPU against the 2026-09-20
     rerun, medians (10K: 5 runs for JS/C#/Go, 3 for Python/C++; 50K: 3):

     | | 10K before → after | 50K before → after |
     |---|---|---|
     | JS | 135 → 124 ms (-7.7%) | 336 → 296 ms (-12.0%) |
     | Python | 135 → 128 ms (-5.6%) | 360 → 326 ms (-9.6%) |
     | C# | 156 → 149 ms (-4.5%) | 406 → 332 ms (-18.2%) |
     | Go | 151 → 137 ms (-9.2%) | 420 → 350 ms (-16.6%) |
     | C++ | 124 → 104 ms (-16.2%) | 298 → 212 ms (-28.7%) |

     The 50K latency gain the prototype predicted is there: C# p50 19 → 0 ms
     and p99 40 → 3 ms, Go p50 28 → 10 ms and p99 64 → 24 ms, throughput
     +15% / +13% (see the 50K table). As with the prototype, system time
     rises (+5-12 ms at 10K, +17-30 ms at 50K), so the process total drops
     less than user time. Python gains although it still parses `raw` into
     its own dict: the tree it no longer gets from core was the second of the
     two it built. C++ gains the most, since its callback does no JSON work
     at all and core's parse was most of what it paid for.

     Against #236's thresholds (user CPU 10K JS ≤ 127, Go ≤ 142, C# ≤ 147;
     50K JS ≤ 300, Go ≤ 365, C# ≤ 350; 50K p50 Go ≤ 12, C# ≤ 3 ms; no
     `lost`/`dropped`) every figure passes except **C# at 10K: 149.3 ms
     against 147** (5 runs: 149.3 / 142.9 / 151.5 / 149.6 / 143.0; two of
     them under the line). The controls were at their 2026-09-20 values
     (legacy JS SDK +2.8%, null client 189K vs 192K), so the miss is not
     the machine; C#'s system time also rose the most at 10K (35 → 47 ms).
     A first 10K pass with the machine still settling (5-minute load ~6.7;
     `results/2026-09-22/10k-noisy/`) moved the legacy SDKs' CPU by +6% /
     +51% and is not used.

   The remainder (frame decode, the two thread hops, the queue) is the
   architecture: the legacy SDK is `ws` + an EventEmitter on one thread, the
   new one crosses two thread boundaries per frame. The `InFlight` permit and
   liveness bookkeeping named as candidates in the previous version of this
   report are negligible in the profile (4 samples of ~1,500). The next
   read-loop item on the list is the queue push's `notify_all` (only needed
   when a consumer is waiting), left out of #236.

4. **C#, Go, C++ and JS are within 8% of each other at 10K** (172-185K
   msg/s, 90-96% of the null client). April's ranking "C# 2nd, Go 3rd at
   ~70% of JS" is gone: Go is +14.5% and its p99 went 11 → 3 ms, and the May
   snapshot's Go client gives the same numbers as today's, so nothing in core
   since May changed Go; April→May is not in this repo's history, so whether
   the rest is the machine or an early core change cannot be told apart. At
   50K the UniFFI bindings with a JSON parse in the callback (C#, Go) start
   to queue (p50 19-28 ms) while C++ (string scan, no parser) and JS do not.

5. **Java is still ~13% of the others** (25.6K msg/s, p50 293 ms, ~20%
   better than April; the morning run measured 22.3K, see
   [Per-run data](#per-run-data)). Every `onMessage` crosses JNA's
   reflection-based callback
   dispatch with a `StreamMessage` record marshalled through `RustBuffer`. At
   production rates (100-500 msg/s) the callback takes well under the
   inter-arrival time, so this only matters for replay/burst workloads.

6. **The CPU column is now one metric** (#215). Until this rerun the six
   clients did not measure the same thing: JS/C#/Python reported process-wide
   CPU, Go and C++ reported wall-clock elapsed, Java the main thread only
   (which idles), and Python's figure included system time. April's table
   compared them anyway, and so did the morning run of this report. Every
   client now reports the process's user CPU across all threads (Java: user +
   system, the JDK offers nothing finer), with the system figure in
   `cpu_system_ms`; see [Metrics](#metrics) for the call each one uses. April's
   Go/C++/Java CPU figures therefore have no successor to compare with; the
   next run will.

7. **The null client is the yardstick, not the server's self-reported rate.**
   `server_msgs_per_sec` in the JSON is `COUNT / elapsed` of the server's
   synchronous `ws.send()` loop, and it rises when the client reads slowly
   (sends become buffer copies: 725K msg/s against the pure-Python SDK,
   150-170K against the fast ones) and is understated by the warmup frames
   it includes in `elapsed` (~9% at 10K). It cannot separate server-bound
   from client-bound, and April's "server exceeds 500K msg/s" was read off
   it. A raw `ws` consumer that does no work (`js/bench-null.js`) tops out at
   192K msg/s (10K) / 263K (50K) on this machine; that is the number each
   binding is compared against above. Pushing the ceiling higher (several
   sender connections, pre-serialised frames, a Rust/Go sender) would be
   needed to see how far the fast bindings can actually go.

8. **At live-feed rates latency is under 1 ms for every binding except JS
   and Java.** With the mock server delivering ~2,000 msg/s (the `rate=500`
   setting, see the rate-limited table), C++, C#, Go and the new Python
   binding are at p99 ≤ 1 ms and max ≤ 4 ms (the pure-Python SDK: p99 2 ms,
   max 9 ms). Both JS SDKs show p99 5-8 ms
   / max 10-12 ms, old and new alike, so it is Node's timer/event-loop
   granularity rather than the Rust core. Java is at p99 63 ms / max 82 ms
   even at 2K msg/s -- the JNA callback cost is not amortised by the lower
   rate -- so the burst-only caveat in April's report does not hold for Java.

## Methodology

### Architecture

```
+---------------------+
|  ws-mock-server.js  |   Mock Fugle WebSocket server (Node `ws` package)
|  (controlled rate)  |   Listens on /stock/streaming
+----+----------+-----+
     |ws://      |ws://
+----+----+ +----+----+
| old SDK | | new SDK |   Each runs as a separate process
| client  | | client  |   Identical measurement logic
+---------+ +---------+
     |           |
   JSON        JSON       Single-line JSON metrics on stdout
```

### Mock Server Protocol

The mock server (`ws-mock-server.js`) simulates the Fugle WebSocket handshake:

1. Client connects to `ws://localhost:PORT/stock/streaming`
2. Client sends `{"event":"auth","data":{"apikey":"..."}}`
3. Server replies `{"event":"authenticated"}`
4. Client sends `{"event":"subscribe","data":{...}}`
5. Server replies `{"event":"subscribed",...}`
6. Server sends N warmup messages (`event: "warmup"`, discarded by client)
7. Server sends N data messages (`event: "data"`) at the configured rate
8. Server sends `{"event":"bench_done","data":{...}}` sentinel

Each data message is a realistic ~300-byte trades payload with:
- `serial`: incrementing counter for loss detection
- `server_ts`: `Date.now()` timestamp for latency measurement

### Metrics

| Metric | How measured | Notes |
|--------|-------------|-------|
| Throughput (msg/s) | `count / elapsed`, where `elapsed` starts at first data message | Excludes connect/auth/subscribe overhead |
| Latency p50/p99 (ms) | `Date.now() - msg.data.server_ts` | Same system clock (localhost), ~1ms resolution |
| Memory delta (MB) | `process.memoryUsage().rss` before/after (JS) or `ru_maxrss` (Python) | |
| CPU user (ms) | Process-wide user CPU, all threads (the Rust runtime's included), from client start to `bench_done`; see the table below | Java is user + system |
| CPU system (ms) | The matching system CPU (`cpu_system_ms` in the JSON) | `null` for Java |
| Message loss | `expected - received` | TCP guarantees delivery; loss = client bug |

How each client measures CPU (#215; before this every client used something
different, see Key Takeaway 6):

| Client | `cpu_user_ms` | `cpu_system_ms` |
|--------|---------------|-----------------|
| JS (both SDKs) | `process.cpuUsage().user` | `.system` |
| Python (both SDKs) | `resource.getrusage(RUSAGE_SELF).ru_utime` | `.ru_stime` |
| C# | `Process.UserProcessorTime` | `Process.PrivilegedProcessorTime` |
| Go | `syscall.Getrusage(RUSAGE_SELF).Utime` | `.Stime` |
| C++ | `getrusage(RUSAGE_SELF).ru_utime` | `.ru_stime` |
| Java | `com.sun.management.OperatingSystemMXBean.getProcessCpuTime()` -- user + system, the JDK has no process-level user-only figure (`ThreadMXBean` cannot see JNA's and the Rust runtime's native threads) | `null` (already in `cpu_user_ms`) |

All figures include the connect/auth/subscribe handshake and the 1,000
warmup frames (the timer starts before the client is created), so the
per-message cost is `cpu / (count + warmup)`.

### Measurement Validity

- **Cross-process clock**: Both server and client use `Date.now()` (system wall clock).
  Since both processes run on localhost, the clock is shared. Resolution is ~1ms, which
  is sufficient for comparing SDK overhead in the 0.1-5ms range.
- **JIT warmup**: Server sends configurable warmup messages before the measured batch.
  Client discards these, ensuring V8/CPython are warmed up.
- **GC noise**: JS clients run with `--expose-gc`; latency arrays use pre-allocated
  `Float64Array` to minimize GC pressure. Both SDKs experience identical GC conditions.
- **Multiple runs**: Default 3 runs, results reported as median to reduce outlier impact.
- **Server bottleneck check**: the server's own `server_msgs_per_sec` is not a
  ceiling (it is coupled to how fast the client reads, see Key Takeaway 7).
  The ceiling is measured with `js/bench-null.js`, a raw `ws` consumer that
  parses nothing: 192K msg/s at 10K, 263K at 50K on this machine.

### Message Path Comparison

**JavaScript**:
```
Old:  server -> ws(C++) -> raw JSON string -> JS callback -> JSON.parse
New:  server -> tokio-tungstenite -> serde_json::from_str (routing fields only)
        -> core stream queue -> reader thread (stream.receive(), blocking)
        -> ThreadsafeFunction(raw frame string) -> JS callback -> JSON.parse
```
The April path re-serialised the parsed struct with `serde_json::to_string`;
the binding now passes `WebSocketMessage.raw` (the frame as received).

**Python**:
```
Old:  server -> websocket-client (pure Python) -> orjson.loads (internal)
        -> EventEmitter -> raw string -> callback -> json.loads
New:  server -> tokio-tungstenite (Rust) -> serde_json::from_str (Rust)
        -> core stream queue -> reader thread (stream.receive(), blocking)
        -> serde_json::from_str(raw) -> PyO3 dict -> callback (receives dict)
```

**C# / Go / Java / C++ (UniFFI)**:
```
New:  server -> tokio-tungstenite (Rust) -> serde_json::from_str (Rust)
        -> core stream queue -> ws_stream_reader thread (stream.receive(), blocking)
        -> StreamMessage { raw, event, channel, symbol, id, data_json, ... }
        -> UniFFI foreign callback (P/Invoke | CGO | JNA | C++)
        -> listener.OnMessage -> System.Text.Json | encoding/json | hand-rolled parse
```
The April path polled `receive_timeout(5ms)` on a std mpsc fed by a tokio
forwarding task; the reader now blocks on core's own queue.

All new-SDK clients run with `message_overflow = unbounded` so a burst is
measured as full delivery (the default `drop_newest` with a 4,096-message
buffer would drop frames -- possibly `bench_done` -- while the callback lags).
Every client reports `lost` and every new-SDK client reports `dropped`; both are 0 in every run here.

## How to Run

### Prerequisites

- **Node.js** >= 18 (mock server, JS clients, runner)
- **Python** >= 3.9 (Python clients)
- **Rust toolchain** (UniFFI native library, napi and PyO3 builds)
- **.NET 8.0 SDK** (C# client; pass `BENCH_DOTNET_ARGS=-p:TargetFrameworks=net8.0`
  if the installed SDK cannot build the binding's newest target)
- **Go** >= 1.23 with CGO, **JDK 21** + the binding's Gradle wrapper, a C++20 compiler

Build the new SDK from the tree you want to measure:

```bash
cargo build -p marketdata-uniffi --release          # C#, Go, Java (C++ builds its own in run.sh)
(cd js && npm ci && npm run build)                   # JS: js/index.js + native .node
(cd py && python3 -m venv .venv && .venv/bin/pip install maturin \
   && .venv/bin/maturin develop --release)           # Python: installs fugle_marketdata into py/.venv
```

The old and new Python SDKs share the `fugle-marketdata` distribution name,
so the old one needs its own venv:

```bash
(cd benchmarks/ws/py && python3 -m venv .venv-old \
   && .venv-old/bin/pip install "fugle-marketdata==2.7.0rc1")
```

### Quick Start

```bash
cd benchmarks/ws

# Install Node dependencies (ws package + old JS SDK @fugle/marketdata@1.6.0)
npm install

export BENCH_PY_NEW=../../py/.venv/bin/python3
export BENCH_PY_OLD=py/.venv-old/bin/python3
export BENCH_DOTNET_ARGS=-p:TargetFrameworks=net8.0   # only if `dotnet run` fails on net10.0

# Quick validation (1K messages, 1 run, one language)
node ws-bench-run.js 1000 0 100 1 --lang js

# Full benchmark, one language at a time (10K messages, burst, 3 runs)
for l in js py cs go java cpp; do node ws-bench-run.js 10000 0 1000 3 --lang $l; done

# Heavy burst (50K messages)
node ws-bench-run.js 50000 0 1000 3 --lang all

# Rate-limited (rate=500 setting; the server's 2 ms batches deliver ~2K msg/s)
node ws-bench-run.js 5000 500 1000 3 --lang all

# Consumer ceiling: raw ws client, no SDK (start the server yourself)
node ws-mock-server.js --port 8765 --count 10000 --rate 0 --warmup 1000 &
node js/bench-null.js --url ws://localhost:8765
```

### Arguments

```
node ws-bench-run.js [count] [rate] [warmup] [runs] [--lang js|py|cs|go|java|cpp|all]

  count    Number of measured data messages (default: 10000)
  rate     Messages per second, 0 = burst (default: 0)
  warmup   Warmup messages before measurement (default: 1000)
  runs     Number of runs, median reported (default: 3)
  --lang   Which SDK pairs to benchmark (default: all)

Environment:
  BENCH_PY_NEW / BENCH_PY_OLD   Python interpreter for the new / old client (default: python3)
  BENCH_DOTNET_ARGS             Extra flags for `dotnet run` (e.g. -p:TargetFrameworks=net8.0)
```

### Example Output

```
================================================================
WebSocket Benchmark: 10000 messages, rate=burst, warmup=1000, runs=3, lang=py
================================================================

  --- Python ---
  Metric                        Old SDK      New SDK      Delta
  ----------------------------------------------------------
  Throughput (msg/s)             25,839      166,666     545.0%
  Latency p50 (ms)                  188            3
  Latency p99 (ms)                  367            9
  CPU user (ms)                   736.4        135.3
```

### Output Files

- **Console**: summary table with median results across all runs
- **`ws-benchmark-results.json`**: full per-run data for the last run (gitignored)
- **`results/<date>/`**: the runs behind this report -- `<count>-<lang>.json`
  (per-run data), `<count>-<lang>.log` (console), `rate500-<lang>.*` (rate
  limited), `<count>-null.jsonl` (null client), `environment.txt` (versions,
  command, env), `may-snapshot-go.txt` (the `14fb9f6` control run);
  `results/2026-09-20/cpu-rerun/` holds the evening rerun the 10K and 50K
  tables are taken from (same layout, its own `environment.txt`);
  `results/2026-09-20/profile-214/` the per-thread and `sample` profiles and
  the prototype behind Key Takeaway 3 (see its `README.txt`)

### Running Individual Components

You can also run the mock server and clients separately for debugging:

```bash
# Start mock server standalone (sends 1000 messages at burst rate)
node ws-mock-server.js --port 8765 --count 1000 --rate 0 --warmup 100

# Run a single JS client against the server
node ws-bench-new.js --url ws://localhost:8765 --timeout 60000

# Run a single Python client against the server
python3 ws-bench-new-py.py --url ws://localhost:8765 --timeout 30
```

## Files

| File | Description |
|------|-------------|
| `ws-mock-server.js` | Mock Fugle WebSocket server with rate control |
| `js/bench-new.js` | New SDK (JS) benchmark client |
| `js/bench-old.js` | Old SDK (JS, `@fugle/marketdata@1.6.0`) benchmark client |
| `js/bench-null.js` | Null client: raw `ws`, no SDK, no parsing -- the consumer ceiling |
| `py/bench-new.py` | New SDK (Python) benchmark client |
| `py/bench-old.py` | Old SDK (Python, `fugle-marketdata==2.7.0rc1`) benchmark client |
| `cs/` | New SDK (C#, UniFFI) benchmark client (.NET 8 project) |
| `go/` | New SDK (Go, UniFFI) benchmark client |
| `java/` | New SDK (Java, UniFFI+JNA) benchmark client |
| `cpp/` | New SDK (C++, UniFFI) benchmark client |
| `ws-bench-run.js` | Runner: starts server, runs clients, compares results |
| `package.json` | Dependencies: `ws`, `@fugle/marketdata@1.6.0` |
| `results/` | Raw output of the runs behind each dated report |
| `REPORT.md` | This file |

## Test Conditions

| Parameter | Value |
|-----------|-------|
| Message count | 10,000 (standard), 50,000 (heavy) |
| Rate | burst (0 = no throttle) |
| Warmup | 1,000 messages |
| Runs | 3 (median reported) |
| Payload size | ~300 bytes (trades data message) |
| Transport | localhost loopback (no network latency) |
| Old JS SDK | `@fugle/marketdata@1.6.0` (C++ `ws` addon) |
| Old Py SDK | `fugle-marketdata==2.7.0rc1` (pure Python `websocket-client`; 2.6.0 is the newest stable) |
| New SDK | Rust core (`tokio-tungstenite` + `serde_json`) `main` @ `de76eca`, napi-rs (JS) / PyO3 (Py) / UniFFI (C#, Go, Java, C++), `message_overflow = unbounded` |
| Toolchain | Node v24.19.0, Python 3.12.2, rustc 1.95.0, .NET SDK 9.0.101 (net8.0 target), Go 1.23.4, OpenJDK 21.0.11 |

## Per-run data

Throughput per run (msg/s), in run order; the median is what the tables above use.
The table below is 2026-09-20 (before #236); the 2026-09-22 runs follow it.

| Client | 10K | 50K |
|--------|-----|-----|
| JS old | 200,000 / 196,078 / 192,308 | 264,550 / 267,380 / 263,158 |
| JS new | 178,571 / 181,818 / 178,571 | 232,558 / 239,234 / 233,645 |
| Python old | 25,706 / 25,839 / 26,315 | 26,497 / 26,666 / 26,329 |
| Python new | 166,666 / 166,666 / 169,491 | 177,304 / 177,935 / 178,571 |
| C# | 188,679 / 181,818 / 185,185 | 226,244 / 218,340 / 216,450 |
| Go | 185,185 / 178,571 / 181,818 | 204,918 / 203,252 / 199,203 |
| Java | 25,000 / 25,641 / 25,906 | 32,175 / 32,154 / 31,565 |
| C++ | 172,413 / 172,413 / 169,491 | 242,718 / 247,524 / 246,305 |

No run is more than 4% from its median (C# 50K run 1 at +3.6% is the widest),
so none was excluded. CPU user per run (ms), 10K: JS old 63.6 / 63.4 / 63.6,
JS new 134.5 / 134.6 / 137.4, Python old 728 / 819 / 736, Python new 132 /
135 / 136, C# 155 / 159 / 156, Go 150.5 / 151.1 / 151.3, Java 2,027 / 1,926 /
1,907, C++ 123.7 / 123.8 / 124.9.

The morning run (`results/2026-09-20/10k-*` and `50k-*`, same tree and
machine) had medians within 4% of these for every client at both sizes except
Java at 10K: 22,321 msg/s then (22,471 / 22,222 / 22,321), 25,641 now, with
the 50K Java runs agreeing (31,685 vs 32,154). The Java client changed only
its CPU calls between the two; the 10K Java measurement window is only
~0.4 s (10,000 frames at 25K msg/s) at the start of the JVM's life, so it is
the least stable number in this report.

Null client (`js/bench-null.js`, raw `ws`, no parsing): 10K 196,078 / 192,308 /
192,308 msg/s; 50K 265,957 / 263,158 / 263,158 msg/s (`cpu-rerun/*-null.jsonl`;
the morning run measured 196,078 / 268,817).

After #236 (2026-09-22, `results/2026-09-22/`), throughput per run (msg/s),
50K: JS old 253,807 / 257,732 / 252,525; JS new 223,214 / 228,311 / 227,273;
Python old 25,445 / 26,288 / 25,920; Python new 174,216 / 178,571 / 146,198;
C# 228,310 / 252,525 / 251,256; Go 229,357 / 230,414 / 234,741; C++ 233,644 /
233,644 / 238,095. CPU user per run (ms), 50K: JS old 192.4 / 191.9 / 194.1,
JS new 296.0 / 296.3 / 287.9, Python old 2,346 / 2,202 / 2,335, Python new
326.0 / 325.8 / 319.2, C# 350.3 / 328.6 / 332.4, Go 356.1 / 350.0 / 346.5, C++
211.6 / 214.6 / 212.4. 10K (5 runs for JS/C#/Go): JS old 65.3 / 64.3 / 64.3 /
66.7 / 66.6, JS new 123.8 / 124.5 / 127.3 / 124.2 / 124.3, Python old 984 /
801 / 932, Python new 126.3 / 129.2 / 127.7, C# 149.3 / 142.9 / 151.5 / 149.6 /
143.0, Go 138.5 / 142.6 / 137.2 / 136.5 / 134.2, C++ 101.8 / 103.7 / 107.3.
Null client with a fresh mock server per run: 10K 185,185 / 192,308 / 188,679;
50K 251,256 / 260,417 / 259,067 msg/s. Python new's third 50K run (146,198) is
the one outlier, 16% under its median.

Rate-limited (`5000 500 1000 3`) throughput per run: JS old 1,698 / 1,723 /
1,788; JS new 1,746 / 1,825 / 2,037; Python old 2,021 / 1,953 / 1,922; Python
new 2,016 / 1,991 / 2,014; C# 1,966 / 2,012 / 1,950; Go 1,992 / 1,924 / 1,970;
Java 1,975 / 2,085 / 2,069; C++ 1,971 / 1,967 / 1,963. JS new p99 per run is
10 / 8 / 1 ms (median 8) -- the widest latency spread in the rate-limited set.

Control run: the Go client built from `14fb9f6` (2026-05-15) against today's
mock server, 10K: 188,679 / 181,818 / 178,571 msg/s, p50 1-3 ms, p99 2-4 ms
(`results/2026-09-20/may-snapshot-go.txt`).
