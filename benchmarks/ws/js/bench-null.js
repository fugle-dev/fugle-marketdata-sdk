#!/usr/bin/env node
/**
 * Null client: raw `ws` socket, no SDK, no JSON.parse on data frames. Measures
 * how fast the mock server can push frames into one consumer on this machine,
 * i.e. the ceiling any SDK client is compared against.
 *
 * Usage:
 *   node js/bench-null.js --url ws://localhost:8765 --timeout 30000
 */

const WebSocket = require('ws');

const args = process.argv.slice(2);
function flag(name, fallback) {
  const idx = args.indexOf(`--${name}`);
  return idx !== -1 && args[idx + 1] != null ? args[idx + 1] : fallback;
}

const BASE_URL = flag('url', 'ws://localhost:8765');
const TIMEOUT  = Number(flag('timeout', 30000));

let received = 0;
let t0 = null;
let serverStats = null;
const startCpu = process.cpuUsage();

const ws = new WebSocket(`${BASE_URL}/stock/streaming`);
ws.on('open', () => {
  ws.send(JSON.stringify({ event: 'auth', data: { apikey: 'bench-key' } }));
});
ws.on('message', (buf) => {
  const s = buf.toString();
  if (s.startsWith('{"event":"data"')) {
    if (t0 === null) t0 = Date.now();
    received++;
    return;
  }
  if (s.startsWith('{"event":"warmup"')) return;
  const msg = JSON.parse(s);
  if (msg.event === 'authenticated') {
    ws.send(JSON.stringify({ event: 'subscribe', data: { channel: 'trades', symbol: '2330' } }));
  } else if (msg.event === 'bench_done') {
    serverStats = msg.data;
    finish();
  }
});
ws.on('error', (err) => console.error('error:', err));

const timer = setTimeout(() => {
  console.error('TIMEOUT: did not receive bench_done within', TIMEOUT, 'ms');
  finish();
}, TIMEOUT);

function finish() {
  clearTimeout(timer);
  const elapsed = t0 !== null ? Date.now() - t0 : 0;
  const endCpu = process.cpuUsage(startCpu);
  console.log(JSON.stringify({
    sdk: 'null-ws',
    count: received,
    expected: serverStats ? serverStats.count : null,
    lost: serverStats ? serverStats.count - received : null,
    elapsed_ms: elapsed,
    msgs_per_sec: elapsed > 0 ? Number((received / elapsed * 1000).toFixed(0)) : 0,
    latency_p50_ms: null,
    latency_p99_ms: null,
    cpu_user_ms: endCpu.user / 1000,
    cpu_system_ms: endCpu.system / 1000,
    server_msgs_per_sec: serverStats ? serverStats.server_msgs_per_sec : null,
  }));
  ws.close();
  setTimeout(() => process.exit(0), 200);
}
