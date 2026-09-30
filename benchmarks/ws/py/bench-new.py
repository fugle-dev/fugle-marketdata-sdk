#!/usr/bin/env python3
"""WebSocket benchmark client - new Rust-core Python SDK (fugle-marketdata 3.x).

Connects to the mock server, subscribes, receives data messages, and
reports throughput / latency / memory metrics as a single JSON line on stdout.

Usage:
    python py/bench-new.py --url ws://localhost:8765 --timeout 30 [--mode dict]

--mode picks the message path (#246):
    dict       `message` callback, the SDK builds the dict (default)
    dict-count `message` callback that does only what `raw` does: it checks
               the event and counts, and reports no latency. `dict-count`
               against `raw` isolates the cost of the SDK building the dict
    raw-loads  `raw_message` callback that calls json.loads, as 2.x code does
    raw        `raw_message` callback that does not parse: it counts the
               data frames by prefix and reports no latency
    aiter      no callback: `async for` over `messages()`, doing what `dict`
               does per message. Measures the `__anext__` path (#260).
               For comparing two builds in this mode (py/run-compare.py),
               which the following does not affect. Not to be compared
               directly with `dict` or the other modes: connect() and
               subscribe() run before the event loop starts, so part of the
               burst is already queued when iteration begins, and the
               elapsed time is taken after asyncio.run() returns, so it
               includes closing the loop
    aiter-lag  `aiter` with a second task on the same event loop that sleeps
               1 ms at a time and records how late each wake-up is: what
               the iteration costs the other tasks on the loop (#267). Adds
               `lag_wakeups` and `lag_p50_ms` / `lag_p99_ms` / `lag_max_ms`
               to the result. The lag is about the number of messages read
               between two turns of the loop times what the loop body takes
               per message, so it grows with a slower body than this one.
               Its `msgs_per_sec` includes what the second task costs: not
               to be compared with the figures of `aiter`
"""

import argparse
import asyncio
import json
import os
import resource
import sys
import threading
import time

# Run with the interpreter that has the new SDK installed (`cd py &&
# maturin develop --release` into py/.venv); ws-bench-run.js takes it from
# BENCH_PY_NEW. The old SDK shares the distribution name, so it needs its
# own venv (BENCH_PY_OLD).
from fugle_marketdata import WebSocketClient

# How long the second task of `aiter-lag` sleeps between wake-ups.
LAG_SLEEP_S = 0.001


def parse_args():
    p = argparse.ArgumentParser()
    p.add_argument('--url', default='ws://localhost:8765')
    p.add_argument('--timeout', type=int, default=30)
    p.add_argument('--mode', choices=('dict', 'dict-count', 'raw-loads', 'raw', 'aiter', 'aiter-lag'), default='dict')
    return p.parse_args()


def main():
    args = parse_args()

    received = 0
    t0 = None
    latencies = []
    max_serial = -1
    server_stats = None
    done_event = threading.Event()

    # Process-wide CPU split into user / system (time.process_time() is the
    # sum of both), so the column is comparable with the other clients (#215).
    start_ru = resource.getrusage(resource.RUSAGE_SELF)
    start_mem = start_ru.ru_maxrss

    # The default (drop_newest, 4096 unread) drops messages -- and possibly
    # bench_done -- while the callback lags a burst; the benchmark measures
    # full delivery, same as the C#/Go/Java clients.
    ws = WebSocketClient(api_key='bench-key', base_url=args.url, message_overflow='unbounded')
    stock = ws.stock

    def on_message(msg):
        nonlocal received, t0, max_serial, server_stats

        event = msg.get('event', '') if isinstance(msg, dict) else ''

        if event == 'warmup':
            return

        if event == 'bench_done':
            server_stats = msg.get('data', {})
            done_event.set()
            return

        if event == 'data':
            now = int(time.time() * 1000)
            if t0 is None:
                t0 = now
            received += 1
            data = msg.get('data', {})
            server_ts = data.get('server_ts')
            if server_ts is not None:
                latencies.append(now - server_ts)
            serial = data.get('serial', -1)
            if serial > max_serial:
                max_serial = serial

    def on_raw_loads(raw):
        on_message(json.loads(raw))

    # `on_count` and `on_raw` do the same work per message, on a dict and
    # on the frame's text: no server_ts, no latency, no serial.
    def on_count(msg):
        nonlocal received, t0, server_stats

        event = msg.get('event')
        if event == 'data':
            if t0 is None:
                t0 = int(time.time() * 1000)
            received += 1
        elif event == 'bench_done':
            server_stats = msg.get('data', {})
            done_event.set()

    def on_raw(raw):
        nonlocal received, t0, server_stats

        # The mock server writes `event` first.
        if raw.startswith('{"event":"data"'):
            if t0 is None:
                t0 = int(time.time() * 1000)
            received += 1
        elif raw.startswith('{"event":"bench_done"'):
            server_stats = json.loads(raw).get('data', {})
            done_event.set()

    def on_error(message, code):
        print(f'error: {message} (code={code})', file=sys.stderr)

    # Register handlers BEFORE connect() — connect() is blocking (returns
    # after auth), so 'connect' event may fire during the call.
    if args.mode == 'dict':
        stock.on('message', on_message)
    elif args.mode == 'dict-count':
        stock.on('message', on_count)
    elif args.mode == 'raw-loads':
        stock.on('raw_message', on_raw_loads)
    elif args.mode == 'raw':
        stock.on('raw_message', on_raw)
    stock.on('error', on_error)

    stock.connect()
    # After connect() returns, auth is done — subscribe immediately
    stock.subscribe('trades', '2330')

    async def iterate():
        async for msg in stock.messages():
            on_message(msg)
            if done_event.is_set():
                return

    # How late each 1 ms sleep of the second task ended, in ms.
    lags = []

    async def measure_lag():
        while True:
            started = time.perf_counter()
            await asyncio.sleep(LAG_SLEEP_S)
            lags.append((time.perf_counter() - started - LAG_SLEEP_S) * 1000)

    async def iterate_until_timeout():
        lag_task = asyncio.ensure_future(measure_lag()) if args.mode == 'aiter-lag' else None
        try:
            await asyncio.wait_for(iterate(), args.timeout)
        except asyncio.TimeoutError:
            pass
        if lag_task is not None:
            lag_task.cancel()

    # Wait for bench_done sentinel or timeout
    if args.mode in ('aiter', 'aiter-lag'):
        asyncio.run(iterate_until_timeout())
    else:
        done_event.wait(timeout=args.timeout)

    elapsed = (int(time.time() * 1000) - t0) if t0 is not None else 0
    end_ru = resource.getrusage(resource.RUSAGE_SELF)
    end_mem = end_ru.ru_maxrss

    # Sort latencies for percentile
    latencies.sort()

    def percentile(arr, p):
        if not arr:
            return None
        idx = min(max(0, int((p / 100) * len(arr)) - 1), len(arr) - 1)
        return arr[idx]

    result = {
        'sdk': 'rust-core-py',
        'mode': args.mode,
        'count': received,
        'expected': server_stats.get('count') if server_stats else None,
        'lost': (server_stats.get('count', 0) - received) if server_stats else None,
        'elapsed_ms': elapsed,
        'msgs_per_sec': int(received / elapsed * 1000) if elapsed > 0 else 0,
        'latency_p50_ms': percentile(latencies, 50),
        'latency_p99_ms': percentile(latencies, 99),
        'latency_min_ms': latencies[0] if latencies else None,
        'latency_max_ms': latencies[-1] if latencies else None,
        # macOS ru_maxrss is in bytes, Linux in KB
        'mem_rss_delta_kb': end_mem - start_mem,
        'cpu_user_ms': round((end_ru.ru_utime - start_ru.ru_utime) * 1000, 1),
        'cpu_system_ms': round((end_ru.ru_stime - start_ru.ru_stime) * 1000, 1),
        'server_msgs_per_sec': server_stats.get('server_msgs_per_sec') if server_stats else None,
        'dropped': stock.messages_dropped_total(),
    }

    if args.mode == 'aiter-lag':
        lags.sort()
        result.update(
            lag_wakeups=len(lags),
            lag_p50_ms=round(percentile(lags, 50), 3) if lags else None,
            lag_p99_ms=round(percentile(lags, 99), 3) if lags else None,
            lag_max_ms=round(lags[-1], 3) if lags else None,
        )

    print(json.dumps(result), flush=True)

    stock.disconnect()
    time.sleep(0.3)
    os._exit(0)


if __name__ == '__main__':
    main()
