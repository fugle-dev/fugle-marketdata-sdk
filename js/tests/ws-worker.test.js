/**
 * WebSocket worker thread tests against a loopback server.
 *
 * `connect()` hands the core client to a dedicated worker thread. Anything
 * that thread does outside its tokio runtime — `client.messages()` spawning
 * the bridge task, for one — panics there, but only after the connect Promise
 * has already resolved. The core crate's tests all run inside `#[tokio::test]`
 * and cannot see that, so these tests go through the real napi worker (#13).
 */

const { WebSocketServer } = require('ws');
const { WebSocketClient } = require('../');

/**
 * Start a server that acks auth, answers `subscribe` with `subscribed` plus
 * one `data` frame.
 */
function startServer() {
  return new Promise((resolve) => {
    const wss = new WebSocketServer({ host: '127.0.0.1', port: 0 });
    wss.on('connection', (socket) => {
      socket.on('message', (raw) => {
        const frame = JSON.parse(raw.toString());
        switch (frame.event) {
          case 'auth':
            socket.send(JSON.stringify({ event: 'authenticated', data: { message: 'Authenticated successfully' } }));
            break;
          case 'subscribe': {
            const { channel, symbol } = frame.data;
            const id = `${channel}-${symbol}`;
            socket.send(JSON.stringify({ event: 'subscribed', data: { id, channel, symbol } }));
            socket.send(JSON.stringify({ event: 'data', data: { symbol, price: 100 }, id, channel }));
            break;
          }
          default:
            break;
        }
      });
    });
    wss.on('listening', () => resolve(wss));
  });
}

function closeServer(wss) {
  for (const socket of wss.clients) socket.terminate();
  return new Promise((resolve) => wss.close(resolve));
}

/** Resolve with the first `message` payload whose event is `event`. */
function waitForEvent(seen, event, timeoutMs = 5000) {
  return new Promise((resolve, reject) => {
    const deadline = Date.now() + timeoutMs;
    const poll = () => {
      const hit = seen.find((msg) => msg.event === event);
      if (hit) return resolve(hit);
      if (Date.now() > deadline) {
        return reject(new Error(`no "${event}" message within ${timeoutMs}ms; got ${JSON.stringify(seen)}`));
      }
      setTimeout(poll, 20);
    };
    poll();
  });
}

const PRODUCTS = [
  ['stock', { channel: 'trades', symbol: '2330' }],
  ['futopt', { channel: 'trades', symbol: 'TXF1!', afterHours: true }],
];

describe.each(PRODUCTS)('%s worker thread', (product, subscription) => {
  let wss;
  let ws;

  beforeEach(async () => {
    wss = await startServer();
    const { port } = wss.address();
    const client = new WebSocketClient({ apiKey: 'test-key', baseUrl: `ws://127.0.0.1:${port}` });
    ws = client[product];
  });

  afterEach(async () => {
    try {
      ws.disconnect();
    } catch (e) {
      // already gone
    }
    await closeServer(wss);
  });

  test('delivers messages after connect', async () => {
    const seen = [];
    ws.on('message', (msg) => seen.push(JSON.parse(msg)));

    await ws.connect();
    ws.subscribe(subscription);

    await waitForEvent(seen, 'subscribed');
    const data = await waitForEvent(seen, 'data');
    expect(data.data.symbol).toBe(subscription.symbol);
  });

  test('still accepts commands well after connect', async () => {
    const seen = [];
    ws.on('message', (msg) => seen.push(JSON.parse(msg)));

    await ws.connect();
    // A dead worker drops its command receiver; give it time to die.
    await new Promise((r) => setTimeout(r, 500));

    expect(() => ws.subscribe(subscription)).not.toThrow();
    await waitForEvent(seen, 'subscribed');
    expect(ws.isConnected).toBe(true);
  });
});
