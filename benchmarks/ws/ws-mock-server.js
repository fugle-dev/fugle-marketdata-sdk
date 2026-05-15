#!/usr/bin/env node
/**
 * Mock WebSocket server that simulates the Fugle market data protocol.
 *
 * Listens on /stock/streaming (matching what both old and new SDKs append to
 * baseUrl). On connection:
 *   1. Waits for {"event":"auth",...} from client
 *   2. Replies {"event":"authenticated"}
 *   3. Waits for {"event":"subscribe",...}
 *   4. Replies {"event":"subscribed",...}
 *   5. Sends --warmup messages (discardable, event="warmup")
 *   6. Sends --count data messages at --rate msgs/sec (0 = burst)
 *   7. Sends a final {"event":"bench_done"} sentinel so client knows to stop
 *
 * Usage:
 *   node ws-mock-server.js --port 8765 --count 10000 --rate 0 --warmup 1000
 */

const { WebSocketServer } = require('ws');
const http = require('http');

// ---------------------------------------------------------------------------
// CLI args
// ---------------------------------------------------------------------------
const args = process.argv.slice(2);
function flag(name, fallback) {
  const idx = args.indexOf(`--${name}`);
  return idx !== -1 && args[idx + 1] != null ? args[idx + 1] : fallback;
}

const PORT    = Number(flag('port', 8765));
const COUNT   = Number(flag('count', 10000));
const RATE    = Number(flag('rate', 0));       // 0 = burst (no throttle)
const WARMUP  = Number(flag('warmup', 1000));

// ---------------------------------------------------------------------------
// Pre-build message templates (avoid JSON.stringify per send)
// ---------------------------------------------------------------------------
function buildDataMsg(serial) {
  return JSON.stringify({
    event: 'data',
    id: 'sub-1',
    channel: 'trades',
    data: {
      symbol: '2330',
      type: 'Equity',
      exchange: 'TWSE',
      market: 'TSE',
      price: 583 + (serial % 10) * 0.5,
      size: 100 + (serial % 50),
      bid: 582.0,
      ask: 583.0,
      time: 1704067200123456,
      serial,
      server_ts: Date.now(),
    },
  });
}

function buildWarmupMsg(serial) {
  return JSON.stringify({
    event: 'warmup',
    data: { serial, server_ts: Date.now() },
  });
}

// ---------------------------------------------------------------------------
// Server
// ---------------------------------------------------------------------------
const server = http.createServer();
const wss = new WebSocketServer({ server, path: '/stock/streaming' });

wss.on('connection', (ws) => {
  let authenticated = false;
  let subscribed = false;

  ws.on('message', (raw) => {
    let msg;
    try { msg = JSON.parse(raw); } catch { return; }

    if (msg.event === 'auth' && !authenticated) {
      authenticated = true;
      ws.send(JSON.stringify({ event: 'authenticated' }));
      return;
    }

    if (msg.event === 'subscribe' && authenticated && !subscribed) {
      subscribed = true;
      ws.send(JSON.stringify({
        event: 'subscribed',
        id: 'sub-1',
        channel: msg.data?.channel || 'trades',
        symbol: msg.data?.symbol || '2330',
      }));
      // Start sending after a short tick to let client settle
      setImmediate(() => startSending(ws));
      return;
    }
  });
});

function startSending(ws) {
  const t0 = Date.now();

  // Phase 1: warmup messages (client should discard these)
  for (let i = 0; i < WARMUP; i++) {
    ws.send(buildWarmupMsg(i));
  }

  // Phase 2: measured data messages
  if (RATE === 0) {
    // Burst mode — send as fast as possible
    for (let i = 0; i < COUNT; i++) {
      ws.send(buildDataMsg(i));
    }
    finish(ws, t0);
  } else {
    // Rate-limited mode
    let sent = 0;
    const intervalMs = 1000 / RATE;
    const timer = setInterval(() => {
      const batchEnd = Math.min(sent + Math.ceil(RATE / 100), COUNT);
      while (sent < batchEnd) {
        ws.send(buildDataMsg(sent));
        sent++;
      }
      if (sent >= COUNT) {
        clearInterval(timer);
        finish(ws, t0);
      }
    }, Math.max(1, Math.floor(intervalMs)));
  }
}

function finish(ws, t0) {
  const elapsed = Date.now() - t0;
  const serverRate = (COUNT / elapsed * 1000).toFixed(0);
  // Send sentinel so client knows all data has been sent
  ws.send(JSON.stringify({
    event: 'bench_done',
    data: { count: COUNT, elapsed_ms: elapsed, server_msgs_per_sec: Number(serverRate) },
  }));
  // Log server-side stats
  console.error(JSON.stringify({
    role: 'server',
    count: COUNT,
    warmup: WARMUP,
    elapsed_ms: elapsed,
    server_msgs_per_sec: Number(serverRate),
  }));
}

server.listen(PORT, () => {
  // Signal to runner that server is ready (stdout)
  console.log(`READY ${PORT}`);
});
