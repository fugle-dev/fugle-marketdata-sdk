# WebSocket Benchmark Report

Rust-core SDK vs legacy pure-JS / pure-Python SDKs, plus cross-language comparison.

Benchmark date: 2026-09-20 (previous run: 2026-04-12)
Hardware: Apple M3 Pro (macOS 26.5), localhost loopback
SDK under test: `main` @ `de76eca` (after #199 / #201 / #202 / #203 / #204 / #209), built from this tree
Raw output: [`results/2026-09-20/`](results/2026-09-20/) (per-run JSON + console log per language, `environment.txt`)

All numbers are the median of 3 runs; per-run values are listed in
[Per-run data](#per-run-data). "vs Apr" compares against the 2026-04-12 report
(`git show de76eca:benchmarks/ws/REPORT.md`), which ran on a different Apple
Silicon machine, so only differences well beyond run-to-run noise (a few
percent) mean anything.

## Summary (10K burst)

### JavaScript (Node.js)

| Metric | Old SDK (`@fugle/marketdata@1.6.0`) | New SDK (Rust core) | Delta | vs Apr (old / new) |
|--------|-------------------------------------|---------------------|-------|--------------------|
| Throughput | 192,308 msg/s | 172,414 msg/s | **-10.3%** | -7.7% / -6.9% |
| Latency p50 | 0 ms | 0 ms | -- | -- |
| Latency p99 | 1 ms | 1 ms | -- | -- |
| CPU user | 64 ms | 136 ms | +113% | -14% / **+92%** |

April's old SDK was `@fugle/marketdata@1.4.2`; 1.6.0 is the npm `latest`.

### Python

| Metric | Old SDK (`fugle-marketdata==2.7.0rc1`) | New SDK (Rust core) | Delta | vs Apr (old / new) |
|--------|----------------------------------------|---------------------|-------|--------------------|
| Throughput | 26,109 msg/s | 163,934 msg/s | **+528%** | +0.8% / **+57.4%** |
| Latency p50 | 185 ms | 3 ms | **-98%** | +2% / **-83%** |
| Latency p99 | 365 ms | 9 ms | **-98%** | -0.3% / **-84%** |
| CPU user | 937 ms | 190 ms | -80% | -9% / -5% |

April's old SDK was `fugle-marketdata==2.4.1`; 2.7.0rc1 is the version the
PM compares against (2.6.0 is the newest stable release on PyPI).

### C# (.NET 8)

No legacy C# SDK exists, so results are absolute (cross-language comparison only).

| Metric | New SDK (Rust core / UniFFI) | vs Apr |
|--------|:---------------------------:|:------:|
| Throughput | **185,185 msg/s** | +5.6% |
| Latency p50 | 0 ms | 2 → 0 ms |
| Latency p99 | 3 ms | 4 → 3 ms |
| CPU user | 159 ms | +73% |

### Go

No legacy Go SDK exists, so results are absolute (cross-language comparison only).

| Metric | New SDK (Rust core / UniFFI) | vs Apr |
|--------|:---------------------------:|:------:|
| Throughput | **178,571 msg/s** | **+12.5%** |
| Latency p50 | 2 ms | 4 → 2 ms |
| Latency p99 | 4 ms | 11 → 4 ms (**-64%**) |
| CPU user | 186 ms (wall clock, see note) | n/a |

### Java (JDK 21)

No legacy Java SDK exists, so results are absolute (cross-language comparison only).

| Metric | New SDK (Rust core / UniFFI+JNA) | vs Apr |
|--------|:-------------------------------:|:------:|
| Throughput | **22,321 msg/s** | +5.8% |
| Latency p50 | 322 ms | -5% |
| Latency p99 | 481 ms | -6% |
| CPU user | 107 ms (main thread only, see note) | n/a |

### C++ (C++20)

No legacy C++ SDK exists, so results are absolute (cross-language comparison only).

| Metric | New SDK (Rust core / UniFFI+C++) | vs Apr |
|--------|:-------------------------------:|:------:|
| Throughput | **169,491 msg/s** | +5.1% |
| Latency p50 | 0 ms | -- |
| Latency p99 | 2 ms | 3 → 2 ms |
| CPU user | 195 ms (wall clock, see note) | n/a |

### Cross-Language Comparison (New Rust-core SDK only, 10K burst)

| Metric | C# (UniFFI) | Go (UniFFI) | JS (napi-rs) | C++ (UniFFI) | Python (PyO3) | Java (UniFFI+JNA) |
|--------|:-----------:|:-----------:|:------------:|:------------:|:-------------:|:-----------------:|
| Throughput | 185,185 msg/s | 178,571 msg/s | 172,414 msg/s | 169,491 msg/s | 163,934 msg/s | 22,321 msg/s |
| Latency p50 | 0 ms | 2 ms | 0 ms | 0 ms | 3 ms | 322 ms |
| Latency p99 | 3 ms | 4 ms | 1 ms | 2 ms | 9 ms | 481 ms |
| CPU user | 159 ms | 186 ms\* | 136 ms | 195 ms\* | 190 ms | 107 ms\* |
| % of null client (196,078 msg/s) | 94% | 91% | 88% | 86% | 84% | 11% |

\* Not comparable: the Go and C++ clients report wall-clock time in this column
and the Java client reports the main thread's CPU only (#215). JS, C# and
Python report process-wide CPU.

The last row compares against a null client (`js/bench-null.js`: a raw `ws`
socket that counts frames and parses nothing), which is the most one consumer
can take from this mock server on this machine: 196,078 msg/s at 10K,
268,817 at 50K (`results/2026-09-20/*-null.jsonl`). The legacy JS SDK sits at
98% of it. The five fast bindings are within 6-16% of that ceiling, so the
spread between them is real but small; Java is the only binding that is
clearly client-bound.

### Heavy burst (50K)

| Metric | JS old | JS new | Py old | Py new | C# | Go | Java | C++ |
|--------|:------:|:------:|:------:|:------:|:--:|:--:|:----:|:---:|
| Throughput (msg/s) | 261,780 | 235,849 | 26,274 | 174,825 | 223,214 | 203,252 | 31,685 | 248,756 |
| Latency p50 (ms) | 0 | 0 | 928 | 52 | 12 | 28 | 895 | 0 |
| Latency p99 (ms) | 1 | 1 | 1,836 | 103 | 41 | 64 | 1,457 | 1 |
| CPU user (ms) | 188 | 339 | 2,462 | 536 | 396 | 378\* | 107\* | 338\* |
| % of null client (268,817 msg/s) | 97% | 88% | 10% | 65% | 83% | 76% | 12% | 93% |

JS old→new: -9.9%. Python old→new: +565%. No run lost a message (`lost` is
0 in every JSON) and no new-SDK client dropped one (`dropped` is 0).

With 50K the null client reaches 269K msg/s (the per-message cost of the Node
`ws` server and consumer amortises better) and the ranking separates: C++ and
JS keep up (p99 1 ms), C# and Go fall behind by 12-28 ms at p50 (their queue
fills while the callback parses JSON), Python by 52 ms, Java by ~0.9 s.

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

1. **Python: the Rust core is now 6.3x the pure-Python SDK** (was 4x in
   April). The old SDK is unchanged (26.1K vs 25.9K msg/s), the new one went
   from 104K to 164K msg/s and p50 from 18 ms to 3 ms. The machine explains
   at most ~12% of that: the Go client built from the earliest commit in this
   repo (`14fb9f6`, the 2026-05-15 v0.3.0 snapshot) runs at 178-189K on
   today's machine, the same as today's Go, and Go is +12.5% vs April. So most
   of the Python gain is code, and code from after `14fb9f6` (the control only
   covers May→September; April→May is not in this repo's history). The Python
   and JS clients from that snapshot panic on `connect()` (`no reactor
   running` in `aio/client.rs` `message_stream()`), so there is no runnable
   good end for a bisect; the candidates on the timeline are the core
   delivery-queue rewrite (`0f9894e` core-owned queue, `6079d5c` single
   ordered stream replacing the `receive_timeout(100ms)` poll) and the Python
   message-thread rewrite (`6685dce`). Tracked in #214.

2. **JS: the -10% gap to the legacy SDK is unchanged** (-10.3% vs -11.1% in
   April) even though the "redundant serde cycle" April blamed is gone: the
   binding now hands the frame to the listener verbatim (`message.raw`, no
   `serde_json::to_string`). The legacy SDK is a raw `ws` socket plus an
   EventEmitter and runs at 98% of the null client; the new binding is at 88%,
   and the difference is the extra hops: tokio reader thread → core queue →
   reader thread → `ThreadsafeFunction` → JS thread. Latency is identical
   (p50 0 ms, p99 1 ms, 10K and 50K).

3. **JS CPU per message roughly doubled** (71 → 136 ms for 10K; the old SDK
   went 74 → 64 ms on the same machine, so it is not the environment). It
   does not show in throughput (the binding is still within 12% of the null
   client), but it is the one regression in this run. `process.cpuUsage()`
   includes the Rust threads; candidates are the same queue rewrite plus the
   per-message `InFlight` permit (a mutex + `notify_all` per frame -- with
   `messageOverflow: 'unbounded'` the `wait_for_room` side returns at once)
   and liveness bookkeeping. Note the April client ran with the binding's
   defaults, before `messageOverflow` existed; this run sets `unbounded`, so
   the CPU comparison spans that setting change too. Not bisectable for the
   same reason as (1); tracked in #214. C# (+73%) moves the same way;
   Go/C++/Java CPU columns cannot be compared (see (6)).

4. **C#, Go, C++ and JS are within 10% of each other at 10K** (169-185K
   msg/s, 86-94% of the null client). April's ranking "C# 2nd, Go 3rd at
   ~70% of JS" is gone: Go is +12.5% and its p99 went 11 → 4 ms, and the May
   snapshot's Go client gives the same numbers as today's, so nothing in core
   since May changed Go; April→May is not in this repo's history, so whether
   the rest is the machine or an early core change cannot be told apart. At
   50K the UniFFI bindings with a JSON parse in the callback (C#, Go) start
   to queue (p50 12-28 ms) while C++ (string scan, no parser) and JS do not.

5. **Java is still ~12% of the others** (22.3K msg/s, p50 322 ms, ~5% better
   than April). Every `onMessage` crosses JNA's reflection-based callback
   dispatch with a `StreamMessage` record marshalled through `RustBuffer`. At
   production rates (100-500 msg/s) the callback takes well under the
   inter-arrival time, so this only matters for replay/burst workloads.

6. **The CPU column is not one metric.** JS/C#/Python report process-wide
   CPU; Go and C++ report wall-clock elapsed; Java reports the main thread
   only (which idles). April's table compared them anyway. Filed as #215; the
   numbers stay in this report so the next run has something to diff, but do
   not read Go/C++/Java CPU as CPU.

7. **The null client is the yardstick, not the server's self-reported rate.**
   `server_msgs_per_sec` in the JSON is `COUNT / elapsed` of the server's
   synchronous `ws.send()` loop, and it rises when the client reads slowly
   (sends become buffer copies: 725K msg/s against the pure-Python SDK,
   150-170K against the fast ones) and is understated by the warmup frames
   it includes in `elapsed` (~9% at 10K). It cannot separate server-bound
   from client-bound, and April's "server exceeds 500K msg/s" was read off
   it. A raw `ws` consumer that does no work (`js/bench-null.js`) tops out at
   196K msg/s (10K) / 269K (50K) on this machine; that is the number each
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
| CPU user (ms) | `process.cpuUsage()` (JS) or `time.process_time()` (Python) | |
| Message loss | `expected - received` | TCP guarantees delivery; loss = client bug |

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
  parses nothing: 196K msg/s at 10K, 269K at 50K on this machine.

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
  Throughput (msg/s)             26,109      163,934     527.9%
  Latency p50 (ms)                  185            3
  Latency p99 (ms)                  365            9
  CPU user (ms)                   937.4          190
```

### Output Files

- **Console**: summary table with median results across all runs
- **`ws-benchmark-results.json`**: full per-run data for the last run (gitignored)
- **`results/<date>/`**: the runs behind this report -- `<count>-<lang>.json`
  (per-run data), `<count>-<lang>.log` (console), `rate500-<lang>.*` (rate
  limited), `<count>-null.jsonl` (null client), `environment.txt` (versions,
  command, env), `may-snapshot-go.txt` (the `14fb9f6` control run)

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

| Client | 10K | 50K |
|--------|-----|-----|
| JS old | 196,078 / 192,308 / 192,308 | 261,780 / 259,067 / 264,550 |
| JS new | 178,571 / 172,414 / 172,414 | 235,849 / 233,645 / 235,849 |
| Python old | 25,445 / 26,109 / 26,246 | 26,028 / 26,315 / 26,274 |
| Python new | 153,846 / 163,934 / 166,666 | 170,648 / 174,825 / 176,056 |
| C# | 181,818 / 192,307 / 185,185 | 223,214 / 215,517 / 233,644 |
| Go | 181,818 / 178,571 / 178,571 | 202,429 / 203,252 / 204,081 |
| Java | 22,471 / 22,222 / 22,321 | 31,705 / 31,685 / 31,036 |
| C++ | 163,934 / 169,491 / 178,571 | 251,256 / 248,756 / 246,305 |

No run is more than 7% from its median (Python new 10K run 1 at -6.2%, C++
10K run 3 at +5.4% and C# 50K run 3 at +4.7% are the widest), so none was
excluded.

Null client (`js/bench-null.js`, raw `ws`, no parsing): 10K 196,078 / 196,078 /
200,000 msg/s; 50K 268,817 / 270,270 / 265,957 msg/s (`*-null.jsonl`).

Rate-limited (`5000 500 1000 3`) throughput per run: JS old 1,698 / 1,723 /
1,788; JS new 1,746 / 1,825 / 2,037; Python old 2,021 / 1,953 / 1,922; Python
new 2,016 / 1,991 / 2,014; C# 1,966 / 2,012 / 1,950; Go 1,992 / 1,924 / 1,970;
Java 1,975 / 2,085 / 2,069; C++ 1,971 / 1,967 / 1,963. JS new p99 per run is
10 / 8 / 1 ms (median 8) -- the widest latency spread in the rate-limited set.

Control run: the Go client built from `14fb9f6` (2026-05-15) against today's
mock server, 10K: 188,679 / 181,818 / 178,571 msg/s, p50 1-3 ms, p99 2-4 ms
(`results/2026-09-20/may-snapshot-go.txt`).
