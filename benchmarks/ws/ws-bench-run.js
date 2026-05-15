#!/usr/bin/env node
/**
 * WebSocket Benchmark Runner
 *
 * Starts the mock server, runs old SDK then new SDK, compares results.
 * Written in Node.js (not bash) to avoid shell escaping issues with JSON.
 *
 * Usage:
 *   node ws-bench-run.js [count] [rate] [warmup] [runs]
 *
 * Examples:
 *   node ws-bench-run.js                   # 10K messages, burst, 3 runs
 *   node ws-bench-run.js 50000             # 50K burst
 *   node ws-bench-run.js 10000 5000        # 10K at 5000 msg/s
 *   node ws-bench-run.js 10000 0 1000 5    # 10K burst, 1K warmup, 5 runs
 */

const { spawn } = require('child_process');
const path = require('path');
const fs = require('fs');

const COUNT  = Number(process.argv[2]) || 10000;
const RATE   = Number(process.argv[3]) || 0;
const WARMUP = Number(process.argv[4]) || 1000;
const RUNS   = Number(process.argv[5]) || 3;
const PORT   = 8765;

const BENCH_DIR = __dirname;
const SERVER_SCRIPT  = path.join(BENCH_DIR, 'ws-mock-server.js');
const CLIENT_NEW_JS  = path.join(BENCH_DIR, 'js', 'bench-new.js');
const CLIENT_OLD_JS  = path.join(BENCH_DIR, 'js', 'bench-old.js');
const CLIENT_NEW_PY  = path.join(BENCH_DIR, 'py', 'bench-new.py');
const CLIENT_OLD_PY  = path.join(BENCH_DIR, 'py', 'bench-old.py');
const CLIENT_NEW_CS  = path.join(BENCH_DIR, 'cs');
const CLIENT_NEW_GO  = path.join(BENCH_DIR, 'go', 'ws-bench-go');
const CLIENT_NEW_JAVA = path.join(BENCH_DIR, 'java', 'run.sh');
const CLIENT_NEW_CPP  = path.join(BENCH_DIR, 'cpp', 'run.sh');

// Determine which SDKs to benchmark based on --lang flag
const langIdx = process.argv.indexOf('--lang');
const LANG = langIdx !== -1 ? process.argv[langIdx + 1] : 'all';  // js, py, cs, go, java, cpp, all

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------
function sleep(ms) {
  return new Promise(r => setTimeout(r, ms));
}

/** Spawn the mock server; resolves when it prints "READY". */
function startServer() {
  return new Promise((resolve, reject) => {
    const proc = spawn('node', [
      SERVER_SCRIPT,
      '--port', String(PORT),
      '--count', String(COUNT),
      '--rate', String(RATE),
      '--warmup', String(WARMUP),
    ], { stdio: ['ignore', 'pipe', 'pipe'] });

    let ready = false;
    proc.stdout.on('data', (chunk) => {
      if (!ready && chunk.toString().includes('READY')) {
        ready = true;
        resolve(proc);
      }
    });
    proc.stderr.on('data', (chunk) => {
      // Server logs go to stderr — ignore during normal operation
    });
    proc.on('error', reject);
    proc.on('exit', (code) => {
      if (!ready) reject(new Error(`Server exited with code ${code}`));
    });
  });
}

/** Run a benchmark client; resolves with parsed JSON result. */
function runClient(script, label) {
  return new Promise((resolve, reject) => {
    const isPython = script.endsWith('.py');
    const isShell = script.endsWith('.sh');
    const isDotnet = script.includes('/cs') || script.includes('\\cs');
    const isGo = !isShell && !isPython && !isDotnet && !script.endsWith('.js');
    let cmd, cmdArgs;
    if (isShell) {
      cmd = 'bash';
      cmdArgs = [script, '--url', `ws://localhost:${PORT}`, '--timeout', '60000'];
    } else if (isGo) {
      cmd = script;
      cmdArgs = ['--url', `ws://localhost:${PORT}`, '--timeout', '60000'];
    } else if (isDotnet) {
      cmd = 'dotnet';
      cmdArgs = ['run', '--project', script, '--configuration', 'Release', '--', '--url', `ws://localhost:${PORT}`, '--timeout', '60000'];
    } else if (isPython) {
      cmd = 'python3';
      cmdArgs = [script, '--url', `ws://localhost:${PORT}`, '--timeout', '60'];
    } else {
      cmd = 'node';
      cmdArgs = ['--expose-gc', script, '--url', `ws://localhost:${PORT}`, '--timeout', '60000'];
    }
    const proc = spawn(cmd, cmdArgs, {
      stdio: ['ignore', 'pipe', 'pipe'],
    });

    let stdout = '';
    let stderr = '';
    proc.stdout.on('data', (d) => { stdout += d; });
    proc.stderr.on('data', (d) => { stderr += d; });

    proc.on('exit', (code) => {
      if (stderr.trim()) {
        process.stderr.write(`  [${label} stderr] ${stderr.trim()}\n`);
      }
      const lines = stdout.trim().split('\n');
      const jsonLine = lines.find(l => l.startsWith('{'));
      if (!jsonLine) {
        reject(new Error(`${label}: no JSON output. stdout=${stdout}, code=${code}`));
        return;
      }
      try {
        resolve(JSON.parse(jsonLine));
      } catch (e) {
        reject(new Error(`${label}: invalid JSON: ${jsonLine}`));
      }
    });

    proc.on('error', reject);
  });
}

function killServer(proc) {
  if (proc && !proc.killed) {
    proc.kill('SIGTERM');
  }
}

function median(arr) {
  if (arr.length === 0) return 0;
  const sorted = [...arr].sort((a, b) => a - b);
  return sorted[Math.floor(sorted.length / 2)];
}

function fmt(n) {
  if (n == null) return 'N/A';
  return typeof n === 'number' ? n.toLocaleString() : String(n);
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------
async function main() {
  // Check dependencies
  try {
    require.resolve('@fugle/marketdata');
  } catch {
    console.log('Installing benchmark dependencies...');
    const { execSync } = require('child_process');
    execSync('npm install', { cwd: BENCH_DIR, stdio: 'inherit' });
  }

  try {
    require.resolve('ws');
  } catch {
    const { execSync } = require('child_process');
    execSync('npm install', { cwd: BENCH_DIR, stdio: 'inherit' });
  }

  console.log('='.repeat(64));
  console.log(`WebSocket Benchmark: ${COUNT} messages, rate=${RATE || 'burst'}, warmup=${WARMUP}, runs=${RUNS}, lang=${LANG}`);
  console.log('='.repeat(64));

  // Define SDK pairs to benchmark
  const pairs = [];
  if (LANG === 'js' || LANG === 'all') {
    pairs.push({ label: 'JS', oldScript: CLIENT_OLD_JS, newScript: CLIENT_NEW_JS,
                 oldName: '@fugle/marketdata@1.4.2', newName: 'rust-core (JS)' });
  }
  if (LANG === 'py' || LANG === 'all') {
    pairs.push({ label: 'Python', oldScript: CLIENT_OLD_PY, newScript: CLIENT_NEW_PY,
                 oldName: 'fugle-marketdata@2.4.1', newName: 'rust-core (Py)' });
  }
  if (LANG === 'cs' || LANG === 'all') {
    pairs.push({ label: 'C#', oldScript: null, newScript: CLIENT_NEW_CS,
                 oldName: null, newName: 'rust-core (C#)', newOnly: true });
  }
  if (LANG === 'go' || LANG === 'all') {
    pairs.push({ label: 'Go', oldScript: null, newScript: CLIENT_NEW_GO,
                 oldName: null, newName: 'rust-core (Go)', newOnly: true });
  }
  if (LANG === 'java' || LANG === 'all') {
    pairs.push({ label: 'Java', oldScript: null, newScript: CLIENT_NEW_JAVA,
                 oldName: null, newName: 'rust-core (Java)', newOnly: true });
  }
  if (LANG === 'cpp' || LANG === 'all') {
    pairs.push({ label: 'C++', oldScript: null, newScript: CLIENT_NEW_CPP,
                 oldName: null, newName: 'rust-core (C++)', newOnly: true });
  }

  const allResults = {};  // { 'JS-old': [...], 'JS-new': [...], ... }

  for (const pair of pairs) {
    const oldKey = `${pair.label}-old`;
    const newKey = `${pair.label}-new`;
    if (!pair.newOnly) allResults[oldKey] = [];
    allResults[newKey] = [];

    console.log(`\n${'~'.repeat(64)}`);
    console.log(`  ${pair.label} SDK Benchmark`);
    console.log(`${'~'.repeat(64)}`);

    for (let run = 1; run <= RUNS; run++) {
      console.log(`\n--- ${pair.label} Run ${run}/${RUNS} ---`);

      // Old SDK (skip for new-only benchmarks like C#)
      if (!pair.newOnly) {
        console.log(`  Starting server for old ${pair.label} SDK...`);
        const server = await startServer();
        await sleep(300);
        console.log(`  Running old SDK (${pair.oldName})...`);
        try {
          const result = await runClient(pair.oldScript, `${pair.label}-old`);
          allResults[oldKey].push(result);
          console.log(`  Old: ${fmt(result.msgs_per_sec)} msg/s, p50=${fmt(result.latency_p50_ms)}ms, p99=${fmt(result.latency_p99_ms)}ms`);
        } catch (e) {
          console.error(`  Old SDK failed: ${e.message}`);
        }
        killServer(server);
        await sleep(500);
      }

      // New SDK
      {
        console.log(`  Starting server for new ${pair.label} SDK...`);
        const server = await startServer();
        await sleep(300);
        console.log(`  Running new SDK (${pair.newName})...`);
        try {
          const result = await runClient(pair.newScript, `${pair.label}-new`);
          allResults[newKey].push(result);
          console.log(`  New: ${fmt(result.msgs_per_sec)} msg/s, p50=${fmt(result.latency_p50_ms)}ms, p99=${fmt(result.latency_p99_ms)}ms`);
        } catch (e) {
          console.error(`  New SDK failed: ${e.message}`);
        }
        killServer(server);
        await sleep(500);
      }
    }
  }

  // ---------------------------------------------------------------------------
  // Summary
  // ---------------------------------------------------------------------------
  console.log('\n' + '='.repeat(64));
  console.log('RESULTS (median of', RUNS, 'runs)');
  console.log('='.repeat(64));

  for (const pair of pairs) {
    const newRuns = allResults[`${pair.label}-new`];

    if (pair.newOnly) {
      // New-only benchmark (e.g., C#): just show absolute numbers
      if (newRuns.length === 0) {
        console.log(`\n  ${pair.label}: No successful runs.`);
        continue;
      }
      const nTput = median(newRuns.map(r => r.msgs_per_sec));
      const nP50  = median(newRuns.map(r => r.latency_p50_ms));
      const nP99  = median(newRuns.map(r => r.latency_p99_ms));
      const nCpu  = median(newRuns.map(r => r.cpu_user_ms));

      console.log(`\n  --- ${pair.label} (new SDK only — no legacy SDK to compare) ---`);
      console.log(`  ${'Metric'.padEnd(24)} ${'New SDK'.padStart(12)}`);
      console.log('  ' + '-'.repeat(36));
      console.log(`  ${'Throughput (msg/s)'.padEnd(24)} ${fmt(nTput).padStart(12)}`);
      console.log(`  ${'Latency p50 (ms)'.padEnd(24)} ${fmt(nP50).padStart(12)}`);
      console.log(`  ${'Latency p99 (ms)'.padEnd(24)} ${fmt(nP99).padStart(12)}`);
      console.log(`  ${'CPU user (ms)'.padEnd(24)} ${fmt(nCpu).padStart(12)}`);
      continue;
    }

    const oldRuns = allResults[`${pair.label}-old`];
    if (oldRuns.length === 0 || newRuns.length === 0) {
      console.log(`\n  ${pair.label}: Not enough successful runs to compare.`);
      continue;
    }

    const oTput = median(oldRuns.map(r => r.msgs_per_sec));
    const nTput = median(newRuns.map(r => r.msgs_per_sec));
    const oP50  = median(oldRuns.map(r => r.latency_p50_ms));
    const nP50  = median(newRuns.map(r => r.latency_p50_ms));
    const oP99  = median(oldRuns.map(r => r.latency_p99_ms));
    const nP99  = median(newRuns.map(r => r.latency_p99_ms));
    const oCpu  = median(oldRuns.map(r => r.cpu_user_ms));
    const nCpu  = median(newRuns.map(r => r.cpu_user_ms));
    const tputDelta = oTput ? (((nTput - oTput) / oTput) * 100).toFixed(1) : 'N/A';

    console.log(`\n  --- ${pair.label} ---`);
    console.log(`  ${'Metric'.padEnd(24)} ${'Old SDK'.padStart(12)} ${'New SDK'.padStart(12)} ${'Delta'.padStart(10)}`);
    console.log('  ' + '-'.repeat(58));
    console.log(`  ${'Throughput (msg/s)'.padEnd(24)} ${fmt(oTput).padStart(12)} ${fmt(nTput).padStart(12)} ${(tputDelta + '%').padStart(10)}`);
    console.log(`  ${'Latency p50 (ms)'.padEnd(24)} ${fmt(oP50).padStart(12)} ${fmt(nP50).padStart(12)}`);
    console.log(`  ${'Latency p99 (ms)'.padEnd(24)} ${fmt(oP99).padStart(12)} ${fmt(nP99).padStart(12)}`);
    console.log(`  ${'CPU user (ms)'.padEnd(24)} ${fmt(oCpu).padStart(12)} ${fmt(nCpu).padStart(12)}`);
  }
  console.log('');

  const allRuns = Object.values(allResults).flat();
  const lost = allRuns.filter(r => r.lost > 0);
  if (lost.length > 0) {
    console.log('  WARNING: Some runs had message loss:');
    lost.forEach(r => console.log(`    ${r.sdk}: lost ${r.lost}/${r.expected}`));
  }

  // Write full results to file
  const outPath = path.join(BENCH_DIR, 'ws-benchmark-results.json');
  fs.writeFileSync(outPath, JSON.stringify({
    config: { count: COUNT, rate: RATE, warmup: WARMUP, runs: RUNS, lang: LANG },
    timestamp: new Date().toISOString(),
    results: allResults,
  }, null, 2));
  console.log(`  Full results written to: ${outPath}`);
}

main().catch((e) => {
  console.error('FATAL:', e);
  process.exit(1);
});
