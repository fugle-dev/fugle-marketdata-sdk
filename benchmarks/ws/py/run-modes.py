#!/usr/bin/env python3
"""Compare the new Python SDK's message paths (#246) and update REPORT.md.

Runs py/bench-new.py in each --mode (dict, raw-loads, dict-count, raw) and the null
client (js/bench-null.js, the control) at 10K and 50K burst, a fresh mock
server per run, modes interleaved. Writes the rows to
results/<date>/{10k,50k}-py-modes.jsonl and rewrites the two generated
blocks of REPORT.md (between the `py-modes` markers).

Usage (from anywhere; needs `npm install` in benchmarks/ws and the new SDK
built with `maturin develop --release` into py/.venv, or BENCH_PY_NEW):

    python3 benchmarks/ws/py/run-modes.py
    python3 benchmarks/ws/py/run-modes.py --report-only --date 2026-09-30

Run it while the load average is below 4, on a clean commit, so the result
can be reproduced from its SHA. The report says so when the load was higher,
and when the tree had uncommitted changes (results/ and REPORT.md, which this
script writes, do not count).
"""

import argparse
import datetime
import json
import os
import statistics
import subprocess
import time

WS_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
REPO = os.path.dirname(os.path.dirname(WS_DIR))
REPORT = os.path.join(WS_DIR, 'REPORT.md')
PORT = 8790
WARMUP = 1000
SIZES = (('10k', 10000), ('50k', 50000))
MODES = ('dict', 'raw-loads', 'dict-count', 'raw')
# Above this the throughput and latency of a run are not trusted (REPORT.md,
# "Measurement Validity").
LOAD_GATE = 4.0
# The null client on a quiet machine (2026-09-20), msg/s.
NULL_QUIET = {'10k': 196000, '50k': 269000}


def run_once(cmd, count):
    server = subprocess.Popen(
        ['node', os.path.join(WS_DIR, 'ws-mock-server.js'), '--port', str(PORT),
         '--count', str(count), '--rate', '0', '--warmup', str(WARMUP)],
        stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
    try:
        if not server.stdout.readline().startswith('READY'):
            raise RuntimeError('mock server did not start')
        done = subprocess.run(cmd, capture_output=True, text=True, timeout=180)
        lines = [line for line in done.stdout.splitlines() if line.startswith('{')]
        if not lines:
            raise RuntimeError(f'no result from {cmd}: {done.stderr.strip()}')
        return json.loads(lines[-1])
    finally:
        server.kill()
        server.wait()
        time.sleep(0.5)


def measure(out_dir, runs):
    python = os.environ.get('BENCH_PY_NEW') or os.path.join(REPO, 'py', '.venv', 'bin', 'python3')
    url = f'ws://localhost:{PORT}'
    for name, count in SIZES:
        path = os.path.join(out_dir, f'{name}-py-modes.jsonl')
        with open(path, 'w') as out:
            for run in range(runs):
                for mode in ('null',) + MODES:
                    load = os.getloadavg()
                    if mode == 'null':
                        result = run_once(['node', os.path.join(WS_DIR, 'js', 'bench-null.js'), '--url', url], count)
                        result['mode'] = 'null'
                    else:
                        result = run_once([python, os.path.join(WS_DIR, 'py', 'bench-new.py'), '--url', url,
                                           '--timeout', '60', '--mode', mode], count)
                    result.update(run=run, n=count, load1=round(load[0], 2), load5=round(load[1], 2))
                    out.write(json.dumps(result) + '\n')
                    out.flush()
                    print(f"{name} run {run + 1}/{runs} {mode:9} {result.get('msgs_per_sec'):>8} msg/s "
                          f"cpu {result.get('cpu_user_ms')} ms  load {result['load1']}", flush=True)


def git(*args):
    return subprocess.run(['git', '-C', REPO, *args], capture_output=True, text=True).stdout.strip()


def load_rows(out_dir):
    rows = {}
    for name, _ in SIZES:
        with open(os.path.join(out_dir, f'{name}-py-modes.jsonl')) as lines:
            rows[name] = [json.loads(line) for line in lines if line.strip()]
    return rows


def values(rows, mode, key):
    return [row[key] for row in rows if row['mode'] == mode and row.get(key) is not None]


def median(rows, mode, key):
    found = values(rows, mode, key)
    return statistics.median(found) if found else None


PENDING = 'pending rerun'


def spread(rows, key):
    """Widest (max - min) / min of `key` among the SDK modes, in percent."""
    widest = 0
    for mode in MODES:
        found = values(rows, mode, key)
        if found and min(found) > 0:
            widest = max(widest, (max(found) - min(found)) / min(found) * 100)
    return widest


def cpu_cell(rows, mode):
    cpu = median(rows, mode, 'cpu_user_ms')
    return PENDING if cpu is None else f'{cpu:.0f} ms'


def delta(rows, mode, base):
    """User CPU of `mode` against `base` in percent, None without both."""
    cpu, reference = median(rows, mode, 'cpu_user_ms'), median(rows, base, 'cpu_user_ms')
    if cpu is None or not reference:
        return None
    return (cpu / reference - 1) * 100


def summary_block(rows, meta):
    every = [row for name in rows for row in rows[name]]
    loads = [row['load1'] for row in every] + [row['load5'] for row in every]
    loaded = max(loads) >= LOAD_GATE
    null = {name: median(rows[name], 'null', 'msgs_per_sec') for name in rows}
    out = ['### Python: `message` (dict) vs `raw_message` (str) (#246)', '']
    if loaded:
        out += [
            '**These runs were made on a loaded machine: only their CPU column is',
            'usable, and they are to be repeated on a quiet one.** The load average',
            f'was {min(loads):.1f}-{max(loads):.1f} during the runs (the gate for this report is',
            f'< {LOAD_GATE:.0f}). The null client, the control for that, read',
            f"{null['10k']:,.0f} msg/s at 10K and {null['50k']:,.0f} at 50K against",
            f"{NULL_QUIET['10k'] // 1000}K / {NULL_QUIET['50k'] // 1000}K on a quiet machine, so throughput and latency say",
            'nothing about the SDK and are left out of the table; they are in',
            '[Per-run data](#per-run-data).',
            '',
        ]
    if meta.get('dirty'):
        out += [
            '**The tree had uncommitted changes when these runs were made, so they',
            'cannot be reproduced from a commit SHA; rerun on a clean commit.**',
            '',
        ]
    out += [
        '`py/bench-new.py --mode` picks the path and how much the callback does:',
        '',
        '- `dict`: `message` callback (the binding re-parses `raw` and builds the',
        '  dict) with the full bookkeeping of the other clients: `time.time()`,',
        '  `server_ts`, the latency list, the serial check.',
        '- `raw-loads`: `raw_message` callback that calls `json.loads` itself, as',
        '  2.x code does, then the same bookkeeping as `dict`.',
        '- `dict-count`: `message` callback that only checks the event and counts.',
        '- `raw`: `raw_message` callback that only checks the frame\'s prefix and',
        '  counts, the forward-or-store case.',
        '',
        'The like-for-like pairs are `raw-loads` against `dict` and `raw` against',
        '`dict-count`; `dict-count` and `raw` report no latency.',
        '',
        f"Run {meta['date']} on `{meta['tree']}`, {meta['runs']} runs per mode, a fresh mock server",
        'per run, modes interleaved; medians below. Raw output:',
        f"[`results/{meta['date']}/`](results/{meta['date']}/). Rerun and regenerate this section",
        'with `python3 benchmarks/ws/py/run-modes.py`, on a clean commit.',
        '',
    ]
    if loaded:
        out += ['| CPU user | ' + ' | '.join(f'`{mode}`' for mode in MODES) + ' |',
                '|----------|' + '|'.join('-' * (len(mode) + 4) for mode in MODES) + '|']
        for name, _ in SIZES:
            out.append(f'| {name.upper()} burst | ' + ' | '.join(cpu_cell(rows[name], mode) for mode in MODES) + ' |')
    else:
        out += ['| Burst | Mode | Throughput | Latency p50 | Latency p99 | CPU user |',
                '|-------|------|------------|-------------|-------------|----------|']
        for name, _ in SIZES:
            for mode in MODES:
                rate = median(rows[name], mode, 'msgs_per_sec')
                p50, p99 = median(rows[name], mode, 'latency_p50_ms'), median(rows[name], mode, 'latency_p99_ms')
                out.append(
                    f"| {name.upper()} | `{mode}` | {PENDING if rate is None else f'{rate:,.0f} msg/s'} | "
                    f"{'--' if p50 is None else f'{p50:.0f} ms'} | {'--' if p99 is None else f'{p99:.0f} ms'} | "
                    f'{cpu_cell(rows[name], mode)} |')
        out += ['', f"Null client (the consumer ceiling): {null['10k']:,.0f} msg/s at 10K, {null['50k']:,.0f} at 50K."]

    big = rows['50k']
    out.append('')
    isolated = delta(big, 'raw', 'dict-count')
    if isolated is None:
        out += [
            '- **The cost of the SDK building the dict is not measured yet.** `raw`',
            '  against `dict-count` is the pair that isolates it; `dict-count` was',
            f'  added after these runs and has no data ({PENDING}).',
        ]
    else:
        saved = median(big, 'dict-count', 'cpu_user_ms') - median(big, 'raw', 'cpu_user_ms')
        out += [
            f'- `raw` against `dict-count`, the same callback work with and without the',
            f'  dict: {isolated:+.1f}% user CPU at 50K, {saved / (SIZES[1][1] + WARMUP) * 1000:.1f} µs per message. This is',
            '  the cost of the SDK building the dict.',
        ]
    same_work = delta(big, 'raw-loads', 'dict')
    if same_work is not None:
        out += [
            f'- `raw-loads` against `dict`, the same callback work with the parse done by',
            f'  `json.loads` instead of the binding: {same_work:+.1f}% user CPU at 50K.',
        ]
    whole = delta(big, 'raw', 'dict')
    if whole is not None:
        out += [
            f'- `raw` against `dict` is {whole:+.0f}% user CPU at 50K, but the two callbacks do',
            '  different work: that is what a callback that does not parse saves in',
            '  total, the benchmark callback\'s own bookkeeping included, not the cost',
            '  of building the dict.',
        ]
    lost = {row.get('lost') for row in every} | {row.get('dropped', 0) for row in every}
    out += [
        f"- The per-run spread of CPU user is up to {spread(rows['10k'], 'cpu_user_ms'):.0f}% at 10K and"
        f" {spread(big, 'cpu_user_ms'):.0f}% at 50K;",
        '  differences inside it mean nothing.',
        '- ' + ('No run lost or dropped a message' if lost <= {0, None} else '**Some runs lost or dropped messages**')
        + ' (`message_overflow = unbounded`).',
    ]
    return '\n'.join(out)


def runs_block(rows, meta):
    every = [row for name in rows for row in rows[name]]
    loads = [row['load1'] for row in every]
    out = [
        f"### {meta['date']}: Python message paths (#246)",
        '',
        f"Run order, {meta['runs']} runs each; 1-minute load average {min(loads):.1f}-{max(loads):.1f}.",
        '',
        '| Client | 10K msg/s | 10K CPU user (ms) | 50K msg/s | 50K CPU user (ms) |',
        '|--------|-----------|-------------------|-----------|-------------------|',
    ]
    for mode in ('null',) + MODES:
        cells = []
        for name, _ in SIZES:
            rates, cpu = values(rows[name], mode, 'msgs_per_sec'), values(rows[name], mode, 'cpu_user_ms')
            cells.append(' / '.join(f'{value:,}' for value in rates) or PENDING)
            cells.append(' / '.join(f'{value:.0f}' for value in cpu) or PENDING)
        label = 'null client' if mode == 'null' else f'`{mode}`'
        out.append(f'| {label} | ' + ' | '.join(cells) + ' |')
    return '\n'.join(out)


def replace_block(text, name, body):
    begin, end = f'<!-- py-modes:{name}:begin -->', f'<!-- py-modes:{name}:end -->'
    if begin not in text or end not in text:
        raise SystemExit(f'REPORT.md has no {begin} ... {end} block')
    head, rest = text.split(begin, 1)
    _, tail = rest.split(end, 1)
    return f'{head}{begin}\n<!-- Generated by py/run-modes.py; edit the script, not this block. -->\n{body}\n{end}{tail}'


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('--runs', type=int, default=5)
    parser.add_argument('--date', default=datetime.date.today().isoformat(), help='results/<date>/ to write or read')
    parser.add_argument('--report-only', action='store_true', help='regenerate REPORT.md from results/<date>/')
    args = parser.parse_args()

    out_dir = os.path.join(WS_DIR, 'results', args.date)
    meta_path = os.path.join(out_dir, 'py-modes-meta.json')
    if not args.report_only:
        os.makedirs(out_dir, exist_ok=True)
        dirty = bool(git('status', '--porcelain', '--', '.', ':!benchmarks/ws/results', ':!benchmarks/ws/REPORT.md'))
        tree = git('rev-parse', '--short', 'HEAD') + (' + uncommitted changes' if dirty else '')
        meta = {'date': args.date, 'tree': tree, 'dirty': dirty, 'runs': args.runs}
        measure(out_dir, args.runs)
        with open(meta_path, 'w') as out:
            json.dump(meta, out, indent=2)
            out.write('\n')
    with open(meta_path) as source:
        meta = json.load(source)

    rows = load_rows(out_dir)
    with open(REPORT) as source:
        text = source.read()
    text = replace_block(text, 'summary', summary_block(rows, meta))
    text = replace_block(text, 'runs', runs_block(rows, meta))
    with open(REPORT, 'w') as out:
        out.write(text)
    print(f'REPORT.md updated from {os.path.relpath(out_dir, REPO)}')


if __name__ == '__main__':
    main()
