#!/usr/bin/env python3
"""WebSocket benchmark client - new Rust-core Python SDK.

Connects to the mock server, subscribes, receives data messages, and
reports throughput / latency / memory metrics as a single JSON line on stdout.

Usage:
    python ws-bench-new-py.py --url ws://localhost:8765 --timeout 30
"""

import argparse
import json
import os
import resource
import sys
import threading
import time

# Add the py directory to path so we can import the built module
SDK_DIR = os.path.join(os.path.dirname(__file__), '..', '..', '..', 'py')
sys.path.insert(0, SDK_DIR)

from marketdata_py import WebSocketClient


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

    ws = WebSocketClient(api_key='bench-key', base_url=args.url)
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
    end_mem = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
    end_time_cpu = time.process_time()

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
        'cpu_user_ms': round((end_time_cpu - start_time_cpu) * 1000, 1),
        'server_msgs_per_sec': server_stats.get('server_msgs_per_sec') if server_stats else None,
    }

    print(json.dumps(result), flush=True)

    stock.disconnect()
    time.sleep(0.3)
    os._exit(0)


if __name__ == '__main__':
    main()
