#!/usr/bin/env node
/**
 * Interactive WebSocket streaming — subscribe to channels via CLI arguments.
 *
 * Usage:
 *     export FUGLE_API_KEY='your-key'
 *     node ws_stream.js trades:2330 trades:2317 candles:2330
 *
 *     # Point at a custom WebSocket base URL (e.g. dev environment) so you
 *     # can kill it to exercise the health-check timeout / reconnect path:
 *     node ws_stream.js --url wss://api-dev.fugle.tw/marketdata/v1.0 trades:2330
 *
 * Press Enter to exit gracefully.
 *
 * Events surfaced through callbacks:
 *     message      — data, snapshot, subscribed, heartbeat, pong, etc. (raw JSON string)
 *     connect      — TCP+WS connection established
 *     disconnect   — connection closed
 *     reconnect    — auto-reconnect attempt
 *     error        — generic errors, reconnect failed
 *
 * WiFi-disconnect test flow (activity-timer health check):
 *     1. [connected] + [subscribed] + [data]/[heartbeat] messages streaming
 *     2. === WiFi off ===
 *     3. (silence — no frames arriving)
 *     4. [disconnected] reason="Health check timeout: no activity for ~90s"
 *     5. [reconnecting] attempt=1  (fails — no WiFi)
 *     6. [reconnecting] attempt=2  ...
 *     7. === WiFi back on ===
 *     8. [reconnecting] attempt=N  (succeeds)
 *     9. [data]/[heartbeat] messages resume
 */

const readline = require('readline');
const { WebSocketClient } = require('./index.js');

// ---------------------------------------------------------------------------
// Parse CLI arguments
// ---------------------------------------------------------------------------
function parseArgs(argv) {
  let baseUrl = null;
  const subs = [];

  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    if (arg === '--url') {
      baseUrl = argv[++i];
      if (!baseUrl) {
        console.error('--url requires a value');
        process.exit(1);
      }
      continue;
    }
    if (arg === '-h' || arg === '--help') {
      printHelp();
      process.exit(0);
    }
    if (!arg.includes(':')) {
      console.error(`Invalid subscription format: '${arg}' (expected channel:symbol)`);
      process.exit(1);
    }
    const [channel, symbol] = arg.split(':', 2);
    subs.push({ channel, symbol });
  }

  return { baseUrl, subs };
}

function printHelp() {
  console.log('Usage: node ws_stream.js [--url BASE_URL] channel:symbol [channel:symbol ...]');
  console.log('  e.g. node ws_stream.js trades:2330 candles:2330');
  console.log('  e.g. node ws_stream.js --url wss://api-dev.fugle.tw/marketdata/v1.0 trades:2330');
}

// ---------------------------------------------------------------------------
// Stats counter
// ---------------------------------------------------------------------------
class Stats {
  constructor() {
    this.messages = 0;
    this.heartbeats = 0;
    this.pongs = 0;
    this.disconnects = 0;
    this.reconnects = 0;
    this.errors = 0;
  }
  summary() {
    return {
      messages: this.messages,
      heartbeats: this.heartbeats,
      pongs: this.pongs,
      disconnects: this.disconnects,
      reconnects: this.reconnects,
      errors: this.errors,
    };
  }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------
async function main() {
  const { baseUrl, subs } = parseArgs(process.argv.slice(2));

  const apiKey = process.env.FUGLE_API_KEY;
  if (!apiKey) {
    console.error("Set FUGLE_API_KEY environment variable first:");
    console.error("  export FUGLE_API_KEY='your-key'");
    process.exit(1);
  }
  if (subs.length === 0) {
    printHelp();
    process.exit(1);
  }

  const stats = new Stats();

  // ---- Client setup ------------------------------------------------------
  const reconnectCfg = {
    enabled: true,
    maxAttempts: 5,
    initialDelayMs: 1000,
    maxDelayMs: 30_000,
  };
  const healthCheckCfg = {
    enabled: true,
    pingInterval: 15_000,
    maxMissedPongs: 3,
  };

  const ws = new WebSocketClient({
    apiKey,
    baseUrl,
    reconnect: reconnectCfg,
    healthCheck: healthCheckCfg,
  });
  const stock = ws.stock;

  // ---- Callbacks ---------------------------------------------------------
  stock.on('message', (raw) => {
    stats.messages++;
    let msg;
    try {
      msg = JSON.parse(raw);
    } catch (e) {
      console.log(`  [raw] ${raw}`);
      return;
    }
    const event = msg.event || '';
    const data = msg.data || {};
    if (event === 'pong') {
      stats.pongs++;
      console.log(`  [pong] ${JSON.stringify(data)}`);
    } else if (event === 'heartbeat') {
      stats.heartbeats++;
      console.log(`  [heartbeat] ${data.time}`);
    } else if (event === 'data') {
      console.log(`  [data] ${msg.channel}:${data.symbol} ${JSON.stringify(data)}`);
    } else if (event === 'snapshot') {
      console.log(`  [snapshot] ${msg.channel}:${data.symbol}`);
    } else if (event === 'subscribed') {
      console.log(`  [subscribed] ${data.channel}:${data.symbol}`);
    } else {
      console.log(`  [${event}] ${JSON.stringify(data)}`);
    }
  });

  stock.on('connect', () => {
    console.log('  [connected]');
    // Subscribe inside the connect callback so we know the worker has
    // finished authenticating. The current binding's connect() is
    // fire-and-forget (returns void); without this guard, subscribe()
    // races the worker thread and fires "Not connected" errors.
    // Once Gap #2 lands (connect() returning a Promise), this can be
    // moved back to the legacy `connect().then(() => subscribe(...))`
    // shape from the README.
    for (const { channel, symbol } of subs) {
      console.log(`Subscribing ${channel}:${symbol}`);
      stock.subscribe({ channel, symbol });
    }
  });

  stock.on('disconnect', (reason) => {
    stats.disconnects++;
    console.log(`  [disconnected] ${reason}`);
  });

  stock.on('reconnect', (info) => {
    stats.reconnects++;
    console.log(`  [reconnecting] ${info}`);
  });

  stock.on('error', (err) => {
    stats.errors++;
    console.log(`  [error] ${err}`);
  });

  // ---- Connect & subscribe -----------------------------------------------
  console.log('Connecting...');
  if (baseUrl) {
    console.log(`  url: ${baseUrl}`);
  } else {
    console.log('  url: (default Fugle stock streaming endpoint)');
  }
  console.log(`  reconnect: maxAttempts=${reconnectCfg.maxAttempts}, ` +
              `delay=${reconnectCfg.initialDelayMs}-${reconnectCfg.maxDelayMs}ms`);
  console.log(`  healthCheck: pingInterval=${healthCheckCfg.pingInterval}ms, ` +
              `maxMissedPongs=${healthCheckCfg.maxMissedPongs}`);

  stock.connect();
  console.log(`\nStreaming ${subs.length} subscription(s). Press Enter to exit.\n`);

  // ---- Wait for user to press Enter --------------------------------------
  const rl = readline.createInterface({ input: process.stdin, output: process.stdout });
  await new Promise((resolve) => {
    rl.question('', () => {
      rl.close();
      resolve();
    });
  });

  // ---- Disconnect & summary ----------------------------------------------
  console.log('\nDisconnecting...');
  stock.disconnect();
  // Give the disconnect callback a moment to fire
  await new Promise((resolve) => setTimeout(resolve, 500));

  const s = stats.summary();
  console.log('\nSummary:');
  console.log(`  messages:      ${s.messages}`);
  console.log(`  heartbeats:    ${s.heartbeats}`);
  console.log(`  pongs:         ${s.pongs}`);
  console.log(`  disconnects:   ${s.disconnects}`);
  console.log(`  reconnects:    ${s.reconnects}`);
  console.log(`  errors:        ${s.errors}`);
}

main().catch((err) => {
  console.error('FATAL:', err);
  process.exit(1);
});
