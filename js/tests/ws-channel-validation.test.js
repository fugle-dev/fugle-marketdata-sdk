/**
 * `subscribe()` checks the channel name on the spot (#113).
 *
 * An unknown channel used to reach the worker thread, which skipped it: no
 * frame was sent and no error reported. It now throws 1005
 * `INVALID_PARAMETER`, listing the valid channels, before anything is queued.
 */

const { WebSocketServer } = require('ws');
const { WebSocketClient } = require('../');

/** Start a server that acks auth and records every `subscribe` frame. */
function startServer(subscribes) {
  return new Promise((resolve) => {
    const wss = new WebSocketServer({ host: '127.0.0.1', port: 0 });
    wss.on('connection', (socket) => {
      socket.on('message', (raw) => {
        const frame = JSON.parse(raw.toString());
        if (frame.event === 'auth') {
          socket.send(JSON.stringify({ event: 'authenticated', data: { message: 'Authenticated successfully' } }));
        } else if (frame.event === 'subscribe') {
          subscribes.push(frame.data);
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

async function waitFor(predicate, timeoutMs = 5000) {
  const deadline = Date.now() + timeoutMs;
  while (!predicate()) {
    if (Date.now() > deadline) throw new Error(`condition not met within ${timeoutMs}ms`);
    await new Promise((r) => setTimeout(r, 20));
  }
}

function thrown(fn) {
  try {
    fn();
  } catch (err) {
    return err;
  }
  throw new Error('expected a throw');
}

const PRODUCTS = [
  ['stock', 'trades, candles, books, aggregates, indices', 'trade'],
  ['stock', 'trades, candles, books, aggregates, indices', 'quotes'],
  ['futopt', 'trades, candles, books, aggregates', 'trade'],
  ['futopt', 'trades, candles, books, aggregates', 'indices'],
];

describe.each(PRODUCTS)('%s subscribe() channel validation', (product, valid, channel) => {
  test(`throws INVALID_PARAMETER for '${channel}' before connect()`, () => {
    const ws = new WebSocketClient({ apiKey: 'test-key' })[product];

    const err = thrown(() => ws.subscribe({ channel, symbol: '2330' }));

    expect(err.code).toBe(1005);
    expect(err.sourceKind).toBe('client');
    expect(err.message).toBe(
      `Invalid parameter 'channel': unknown channel '${channel}'. Valid channels: ${valid}`,
    );
  });
});

describe.each(['stock', 'futopt'])('%s subscribe() while connected', (product) => {
  let wss;
  let ws;
  let subscribes;

  beforeEach(async () => {
    subscribes = [];
    wss = await startServer(subscribes);
    const { port } = wss.address();
    ws = new WebSocketClient({ apiKey: 'test-key', baseUrl: `ws://127.0.0.1:${port}` })[product];
    await ws.connect();
  });

  afterEach(async () => {
    try {
      ws.disconnect();
    } catch (e) {
      // already gone
    }
    await closeServer(wss);
  });

  test('an invalid channel throws and sends nothing', async () => {
    expect(() => ws.subscribe({ channel: 'trade', symbol: '2330' })).toThrow(
      expect.objectContaining({ code: 1005 }),
    );
    // A valid subscribe afterwards is the first frame the server sees.
    ws.subscribe({ channel: 'books', symbol: '2330' });
    await waitFor(() => subscribes.length > 0);
    expect(subscribes).toEqual([expect.objectContaining({ channel: 'books', symbol: '2330' })]);
  });

  test('channel names are case-insensitive', async () => {
    ws.subscribe({ channel: 'TRADES', symbol: '2330' });
    await waitFor(() => subscribes.length > 0);
    expect(subscribes[0].channel).toBe('trades');
  });
});
