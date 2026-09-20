#!/usr/bin/env python3
"""WebSocket benchmark client - new Rust-core Python SDK (fugle-marketdata 3.x).

Connects to the mock server, subscribes, receives data messages, and
reports throughput / latency / memory metrics as a single JSON line on stdout.

Usage:
    python py/bench-new.py --url ws://localhost:8765 --timeout 30
"""

import argparse
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


def parse_args():
    p = argparse.ArgumentParser()
    p.add_argument('--url', default='ws://localhost:8765')
    p.add_argument('--timeout', type=int, default=30)
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

    def on_error(message, code):
        print(f'error: {message} (code={code})', file=sys.stderr)

    # Register handlers BEFORE connect() — connect() is blocking (returns
    # after auth), so 'connect' event may fire during the call.
    stock.on('message', on_message)
    stock.on('error', on_error)

    stock.connect()
    # After connect() returns, auth is done — subscribe immediately
    stock.subscribe('trades', '2330')

    # Wait for bench_done sentinel or timeout
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

    print(json.dumps(result), flush=True)

    stock.disconnect()
    time.sleep(0.3)
    os._exit(0)


if __name__ == '__main__':
    main()
