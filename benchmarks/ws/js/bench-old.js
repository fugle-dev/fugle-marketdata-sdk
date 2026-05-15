#!/usr/bin/env node
/**
 * WebSocket benchmark client — old @fugle/marketdata@1.4.2 SDK.
 *
 * Same measurement logic as ws-bench-new.js for fair comparison.
 *
 * Usage:
 *   node ws-bench-old.js --url ws://localhost:8765 --timeout 30000
 */

// Load old SDK from local node_modules (installed via benchmarks/package.json)
const { WebSocketClient } = require('@fugle/marketdata');

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------
const args = process.argv.slice(2);
function flag(name, fallback) {
  const idx = args.indexOf(`--${name}`);
  return idx !== -1 && args[idx + 1] != null ? args[idx + 1] : fallback;
}

const BASE_URL = flag('url', 'ws://localhost:8765');
const TIMEOUT  = Number(flag('timeout', 30000));

// ---------------------------------------------------------------------------
// Metrics state
// ---------------------------------------------------------------------------
let received = 0;
let t0 = null;
const latencies = new Float64Array(500000);
let latIdx = 0;
let maxSerial = -1;
let serverStats = null;

const startCpu = process.cpuUsage();
const startMem = process.memoryUsage();

if (global.gc) global.gc();

// ---------------------------------------------------------------------------
// Client — old SDK uses same API shape
// ---------------------------------------------------------------------------
const ws = new WebSocketClient({ apiKey: 'bench-key', baseUrl: BASE_URL });

ws.stock.on('message', (data) => {
  const msg = JSON.parse(data);

  if (msg.event === 'warmup') return;

  if (msg.event === 'bench_done') {
    serverStats = msg.data;
    finish();
    return;
  }

  if (msg.event === 'data') {
    if (t0 === null) t0 = Date.now();
    received++;
    if (msg.data && msg.data.server_ts) {
      latencies[latIdx++] = Date.now() - msg.data.server_ts;
    }
    if (msg.data && msg.data.serial > maxSerial) {
      maxSerial = msg.data.serial;
    }
  }
});

ws.stock.on('error', (err) => {
  console.error('error:', err);
});

ws.stock.connect().then(() => {
  ws.stock.subscribe({ channel: 'trades', symbol: '2330' });
});

const timer = setTimeout(() => {
  console.error('TIMEOUT: did not receive bench_done within', TIMEOUT, 'ms');
  finish();
}, TIMEOUT);

// ---------------------------------------------------------------------------
// Report
// ---------------------------------------------------------------------------
function finish() {
  clearTimeout(timer);
  const elapsed = t0 !== null ? Date.now() - t0 : 0;
  const endMem = process.memoryUsage();
  const endCpu = process.cpuUsage(startCpu);

  const lats = latencies.subarray(0, latIdx);
  const sorted = Float64Array.from(lats).sort();

  console.log(JSON.stringify({
    sdk: 'old-js-1.4.2',
    count: received,
    expected: serverStats ? serverStats.count : null,
    lost: serverStats ? serverStats.count - received : null,
    elapsed_ms: elapsed,
    msgs_per_sec: elapsed > 0 ? Number((received / elapsed * 1000).toFixed(0)) : 0,
    latency_p50_ms: percentile(sorted, 50),
    latency_p99_ms: percentile(sorted, 99),
    latency_min_ms: sorted.length > 0 ? sorted[0] : null,
    latency_max_ms: sorted.length > 0 ? sorted[sorted.length - 1] : null,
    mem_rss_delta_mb: Number(((endMem.rss - startMem.rss) / 1e6).toFixed(1)),
    cpu_user_ms: endCpu.user / 1000,
    cpu_system_ms: endCpu.system / 1000,
    server_msgs_per_sec: serverStats ? serverStats.server_msgs_per_sec : null,
  }));

  ws.stock.disconnect();
  setTimeout(() => process.exit(0), 200);
}

function percentile(sorted, p) {
  if (sorted.length === 0) return null;
  const idx = Math.min(Math.ceil((p / 100) * sorted.length) - 1, sorted.length - 1);
  return Number(sorted[Math.max(0, idx)].toFixed(2));
}
