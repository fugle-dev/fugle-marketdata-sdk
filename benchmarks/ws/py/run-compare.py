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

`--mode aiter-lag` adds the delay a second task on the event loop sees
(`lag_*`, see py/bench-new.py) to the rows. `--head-env NAME=V1,V2` runs the
head build once per value with that environment variable set, as separate
rows.

The clients take turns going first: each round starts one client later than
the one before, so none always runs last. A row shows how many runs it is
the median of (`n=`).

Run it while the load average is below 4 (REPORT.md, "Measurement Validity").
The 1- and 5-minute load are read before every run; each summary gives the
highest seen and says whether it passed the gate. With `--stop-above-gate`
it stops at the first run that would start at or above the gate instead,
after printing the medians of the runs made so far. The load average
includes what the benchmark itself adds, so on a machine that is close to
the gate the runs can trip it on their own.
"""

import argparse
import importlib.util
import os
import statistics
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
_spec = importlib.util.spec_from_file_location('run_modes', os.path.join(HERE, 'run-modes.py'))
run_modes = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(run_modes)

KEYS = ('msgs_per_sec', 'latency_p50_ms', 'latency_p99_ms', 'cpu_user_ms', 'lost')
# Only in the results of `--mode aiter-lag`; fractions of a millisecond matter.
LAG_KEYS = ('lag_wakeups', 'lag_p50_ms', 'lag_p99_ms', 'lag_max_ms')


def rate(result):
    found = result.get('msgs_per_sec')
    return '       -' if found is None else f'{found:>8}'


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--mode', default='aiter')
    parser.add_argument('--base', required=True, help='interpreter with the build to compare against')
    parser.add_argument('--head', required=True, help='interpreter with the build under test')
    parser.add_argument('--runs', type=int, default=5)
    parser.add_argument('--head-env', metavar='NAME=V1,V2',
                        help='run the head build once per value of this environment variable')
    parser.add_argument('--stop-above-gate', action='store_true',
                        help='stop before a run that would start above the load gate')
    args = parser.parse_args()

    url = f'ws://localhost:{run_modes.PORT}'
    bench = os.path.join(run_modes.WS_DIR, 'py', 'bench-new.py')
    head = [os.path.abspath(args.head), bench, '--url', url, '--timeout', '60', '--mode', args.mode]
    if args.head_env:
        env_name, _, values = args.head_env.partition('=')
        if not env_name or not all(values.split(',')):
            parser.error('--head-env takes NAME=V1[,V2...]')
        if len(set(values.split(','))) != len(values.split(',')):
            parser.error('--head-env values must differ: each one is a row')
        heads = [(f'head {value}', ['env', f'{env_name}={value}'] + head) for value in values.split(',')]
    else:
        heads = [('head', head)]
    clients = [
        ('null', ['node', os.path.join(run_modes.WS_DIR, 'js', 'bench-null.js'), '--url', url]),
        ('base', [os.path.abspath(args.base), bench, '--url', url, '--timeout', '60', '--mode', args.mode]),
        *heads,
    ]
    width = max(len(label) for label, _ in clients)
    for name, count in run_modes.SIZES:
        rows = {label: [] for label, _ in clients}
        loads = []
        stopped = None
        for run in range(args.runs):
            # Rotated, so no client always runs last in a round.
            for label, cmd in clients[run % len(clients):] + clients[:run % len(clients)]:
                load = os.getloadavg()
                if args.stop_above_gate and max(load[:2]) >= run_modes.LOAD_GATE:
                    stopped = (f'stopped before {name} run {run + 1} {label}: load {load[0]:.2f}/{load[1]:.2f} '
                               f'is not below the gate {run_modes.LOAD_GATE}')
                    break
                loads += load[:2]
                result = run_modes.run_once(cmd, count)
                rows[label].append(result)
                print(f"{name} run {run + 1}/{args.runs} {label:{width}} {rate(result)} msg/s "
                      f"load {load[0]:.2f}/{load[1]:.2f}", flush=True)
            if stopped:
                break
        print(f'\n{name.upper()} burst, `{args.mode}`, median of the runs made (n):')
        for label, _ in clients:
            cells = []
            for key in KEYS:
                found = [row[key] for row in rows[label] if row.get(key) is not None]
                cells.append(f'{key}={statistics.median(found):,.0f}' if found else f'{key}=-')
            for key in LAG_KEYS:
                found = [row[key] for row in rows[label] if row.get(key) is not None]
                if found:
                    digits = 0 if key == 'lag_wakeups' else 3
                    cells.append(f'{key}={statistics.median(found):,.{digits}f}')
            print(f'  {label:{width}} n={len(rows[label])}  ' + '  '.join(cells))
        if loads:
            loaded = max(loads) >= run_modes.LOAD_GATE
            print(f'  highest load average before a run: {max(loads):.2f}, gate {run_modes.LOAD_GATE}: '
                  + ('ABOVE THE GATE, do not trust these figures' if loaded else 'within the gate'))
        print()
        if stopped:
            sys.exit(stopped)


if __name__ == '__main__':
    main()
