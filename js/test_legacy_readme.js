#!/usr/bin/env node
/**
 * Smoke test mirroring fugle-marketdata-node's README examples.
 *
 * Each section corresponds to a code block in the legacy README so you can
 * eyeball-compare API surface and runtime behavior.
 *
 * Run individual sections, or all of them:
 *
 *   export FUGLE_API_KEY='your-key'
 *   node test_legacy_readme.js rest
 *   node test_legacy_readme.js ws
 *   node test_legacy_readme.js                # all sections
 */

const { RestClient, WebSocketClient } = require('./index.js');

const API_KEY = process.env.FUGLE_API_KEY;
if (!API_KEY) {
  console.error('Set FUGLE_API_KEY environment variable first.');
  process.exit(1);
}

// ---------------------------------------------------------------------------
// Section 1 — REST API (legacy README "REST API" block)
// ---------------------------------------------------------------------------
async function sectionRest() {
  console.log('='.repeat(60));
  console.log('Section 1: REST API');
  console.log('='.repeat(60));
  console.log(">>> client.stock.intraday.quote({ symbol: '2330' })");

  const client = new RestClient({ apiKey: API_KEY });
  const stock = client.stock;

  const data = await stock.intraday.quote({ symbol: '2330' });
  console.log(data);
  console.log();
}

// ---------------------------------------------------------------------------
// Section 2 — WebSocket API (legacy README "WebSocket API" block)
// ---------------------------------------------------------------------------
async function sectionWs() {
  console.log('='.repeat(60));
  console.log('Section 2: WebSocket API');
  console.log('='.repeat(60));

  const client = new WebSocketClient({ apiKey: API_KEY });
  const stock = client.stock;

  stock.on('connect', () => console.log('connected'));
  stock.on('message', (message) => {
    // Legacy README: `const data = JSON.parse(message); console.log(data);`
    const data = JSON.parse(message);
    console.log('message:', data);
  });
  stock.on('disconnect', (reason) => console.log('disconnect:', reason));
  stock.on('error', (err) => console.log('error:', err));

  console.log(">>> stock.connect().then(() => stock.subscribe({ channel: 'trades', symbol: '2330' }))");
  await stock.connect().then(() => {
    stock.subscribe({ channel: 'trades', symbol: '2330' });
  });

  // Stream for ~10 seconds then disconnect (the README example never
  // disconnects; we add a bounded wait so the script can finish).
  await new Promise((resolve) => setTimeout(resolve, 10_000));
  stock.disconnect();
  // Give the disconnect event time to flush
  await new Promise((resolve) => setTimeout(resolve, 500));
  console.log();
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------
const SECTIONS = {
  rest: sectionRest,
  ws: sectionWs,
};

async function main() {
  const requested = process.argv.slice(2);
  const selected = requested.length > 0 ? requested : Object.keys(SECTIONS);

  for (const name of selected) {
    if (!SECTIONS[name]) {
      console.error(`Unknown section: ${name}`);
      console.error(`Available: ${Object.keys(SECTIONS).join(', ')}`);
      process.exit(1);
    }
  }

  for (const name of selected) {
    await SECTIONS[name]();
  }
}

main().catch((err) => {
  console.error('FATAL:', err);
  process.exit(1);
});
