#!/usr/bin/env python3
"""Compare one py/bench-new.py --mode between two builds of the new Python SDK.

Runs the mode with each interpreter and the null client (js/bench-null.js,
the control), a fresh mock server per run, interleaved, and prints the
medians. Writes nothing: it is for checking a branch against main before
merging (#260), not for REPORT.md.

Usage (needs `npm install` in benchmarks/ws; each interpreter's venv has its
checkout built with `maturin develop --release`):

    python3 benchmarks/ws/py/run-compare.py --mode aiter \\
        --base <main checkout>/py/.venv/bin/python3 --head py/.venv/bin/python3

Run it while the load average is below 4 (REPORT.md, "Measurement Validity").
"""

import argparse
import importlib.util
import os
import statistics

HERE = os.path.dirname(os.path.abspath(__file__))
_spec = importlib.util.spec_from_file_location('run_modes', os.path.join(HERE, 'run-modes.py'))
run_modes = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(run_modes)

KEYS = ('msgs_per_sec', 'latency_p50_ms', 'latency_p99_ms', 'cpu_user_ms', 'lost')


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--mode', default='aiter')
    parser.add_argument('--base', required=True, help='interpreter with the build to compare against')
    parser.add_argument('--head', required=True, help='interpreter with the build under test')
    parser.add_argument('--runs', type=int, default=5)
    args = parser.parse_args()

    url = f'ws://localhost:{run_modes.PORT}'
    bench = os.path.join(run_modes.WS_DIR, 'py', 'bench-new.py')
    clients = (
        ('null', ['node', os.path.join(run_modes.WS_DIR, 'js', 'bench-null.js'), '--url', url]),
        ('base', [os.path.abspath(args.base), bench, '--url', url, '--timeout', '60', '--mode', args.mode]),
        ('head', [os.path.abspath(args.head), bench, '--url', url, '--timeout', '60', '--mode', args.mode]),
    )
    for name, count in run_modes.SIZES:
        rows = {label: [] for label, _ in clients}
        for run in range(args.runs):
            for label, cmd in clients:
                load = os.getloadavg()
                result = run_modes.run_once(cmd, count)
                rows[label].append(result)
                print(f"{name} run {run + 1}/{args.runs} {label:4} {result.get('msgs_per_sec'):>8} msg/s "
                      f"load {load[0]:.2f}/{load[1]:.2f}", flush=True)
        print(f'\n{name.upper()} burst, `{args.mode}`, median of {args.runs}:')
        for label, _ in clients:
            cells = []
            for key in KEYS:
                found = [row[key] for row in rows[label] if row.get(key) is not None]
                cells.append(f'{key}={statistics.median(found):,.0f}' if found else f'{key}=-')
            print(f'  {label:4} ' + '  '.join(cells))
        print()
    load = os.getloadavg()
    if max(load[0], load[1]) >= run_modes.LOAD_GATE:
        print(f'load average {load[0]:.2f}/{load[1]:.2f} is above {run_modes.LOAD_GATE}: do not trust these figures')


if __name__ == '__main__':
    main()
