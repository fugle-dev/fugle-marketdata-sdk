#!/usr/bin/env python3
"""WebSocket benchmark client - old fugle-marketdata@2.4.1 Python SDK.

Same measurement logic as ws-bench-new-py.py for fair comparison.
Old SDK's message callback receives a raw JSON string (not a dict).

Usage:
    python ws-bench-old-py.py --url ws://localhost:8765 --timeout 30
"""

import argparse
import json
import os
import resource
import sys
import threading
import time

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

    start_mem = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
    start_time_cpu = time.process_time()

    # Old SDK takes api_key as kwarg and base_url for the WebSocket factory
    ws = WebSocketClient(api_key='bench-key', base_url=args.url)
    stock = ws.stock

    def on_message(raw):
        nonlocal received, t0, max_serial, server_stats

        # Old SDK delivers raw JSON string — must parse ourselves
        msg = json.loads(raw)
        event = msg.get('event', '')

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

    def on_error(err):
        print(f'error: {err}', file=sys.stderr)

    # Register handlers BEFORE connect() — old SDK's connect() blocks until
    # auth completes, so 'connect' event fires during connect() and we'd miss it.
    stock.on('message', on_message)
    stock.on('error', on_error)

    stock.connect()
    # After connect() returns, auth is done — subscribe immediately
    stock.subscribe({'channel': 'trades', 'symbol': '2330'})

    # Wait for bench_done sentinel or timeout
    done_event.wait(timeout=args.timeout)

    elapsed = (int(time.time() * 1000) - t0) if t0 is not None else 0
    end_mem = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
    end_time_cpu = time.process_time()

    latencies.sort()

    def percentile(arr, p):
        if not arr:
            return None
        idx = min(max(0, int((p / 100) * len(arr)) - 1), len(arr) - 1)
        return arr[idx]

    result = {
        'sdk': 'old-py-2.4.1',
        'count': received,
        'expected': server_stats.get('count') if server_stats else None,
        'lost': (server_stats.get('count', 0) - received) if server_stats else None,
        'elapsed_ms': elapsed,
        'msgs_per_sec': int(received / elapsed * 1000) if elapsed > 0 else 0,
        'latency_p50_ms': percentile(latencies, 50),
        'latency_p99_ms': percentile(latencies, 99),
        'latency_min_ms': latencies[0] if latencies else None,
        'latency_max_ms': latencies[-1] if latencies else None,
        'mem_rss_delta_kb': end_mem - start_mem,
        'cpu_user_ms': round((end_time_cpu - start_time_cpu) * 1000, 1),
        'server_msgs_per_sec': server_stats.get('server_msgs_per_sec') if server_stats else None,
    }

    print(json.dumps(result), flush=True)

    stock.disconnect()
    time.sleep(0.3)
    os._exit(0)


if __name__ == '__main__':
    main()
