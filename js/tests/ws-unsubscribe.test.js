/**
 * `unsubscribe()` sends the id the server issued (#136).
 *
 * Besides the server id (`'id'`, `{ id }`, `{ ids }`), it takes the options
 * passed to `subscribe()`: `{ channel, symbol | symbols }` plus the product's
 * modifier. Either way the frame carries the id from the `subscribed` ack.
 * Combining `channel` with `id` / `ids` is 1005 `INVALID_PARAMETER`.
 */

const { WebSocketServer } = require('ws');
const { WebSocketClient } = require('../');

/** The id the server issues for one subscribed symbol. */
function serverId(data, symbol) {
  const suffix = (data.intradayOddLot ? '-odd' : '') + (data.afterHours ? '-ah' : '');
  return `id-${data.channel}-${symbol}${suffix}`;
}

/**
 * Start a server that acks auth and every `subscribe` (as the Fugle server
 * does: an object for `symbol`, an array for `symbols`), and records the
 * `data` of every `unsubscribe` frame.
 */
function startServer(unsubscribes) {
  return new Promise((resolve) => {
    const wss = new WebSocketServer({ host: '127.0.0.1', port: 0 });
    wss.on('connection', (socket) => {
      socket.on('message', (raw) => {
        const frame = JSON.parse(raw.toString());
        if (frame.event === 'auth') {
          socket.send(JSON.stringify({ event: 'authenticated', data: { message: 'Authenticated successfully' } }));
        } else if (frame.event === 'subscribe') {
          const { symbols, ...rest } = frame.data;
          const entry = (symbol) => ({ ...rest, symbol, id: serverId(rest, symbol) });
          const data = symbols ? symbols.map(entry) : entry(frame.data.symbol);
          socket.send(JSON.stringify({ event: 'subscribed', data }));
        } else if (frame.event === 'unsubscribe') {
          unsubscribes.push(frame.data);
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

const PRODUCTS = [
  ['stock', 'intradayOddLot', 'odd'],
  ['futopt', 'afterHours', 'ah'],
];

describe.each(PRODUCTS)('%s unsubscribe()', (product, modifier, suffix) => {
  let wss;
  let ws;
  let unsubscribes;

  beforeEach(async () => {
    unsubscribes = [];
    wss = await startServer(unsubscribes);
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

  test('by channel and symbol sends the server id', async () => {
    ws.subscribe({ channel: 'trades', symbol: '2330' });
    ws.unsubscribe({ channel: 'trades', symbol: '2330' });

    await waitFor(() => unsubscribes.length > 0);
    expect(unsubscribes).toEqual([{ id: 'id-trades-2330' }]);
  });

  test('by channel and symbols sends the server ids', async () => {
    ws.subscribe({ channel: 'books', symbols: ['2330', '2317'] });
    ws.unsubscribe({ channel: 'books', symbols: ['2330', '2317'] });

    await waitFor(() => unsubscribes.length > 0);
    expect(unsubscribes).toEqual([{ ids: ['id-books-2330', 'id-books-2317'] }]);
  });

  test(`${modifier} names its own subscription`, async () => {
    ws.subscribe({ channel: 'trades', symbol: '2330', [modifier]: true });
    ws.subscribe({ channel: 'trades', symbol: '2330' });
    ws.unsubscribe({ channel: 'trades', symbol: '2330', [modifier]: true });

    await waitFor(() => unsubscribes.length > 0);
    expect(unsubscribes).toEqual([{ id: `id-trades-2330-${suffix}` }]);
  });

  test('by server id sends it as given', async () => {
    ws.unsubscribe('id-a');
    ws.unsubscribe({ id: 'id-b' });
    ws.unsubscribe({ ids: ['id-c', 'id-d'] });

    await waitFor(() => unsubscribes.length >= 3);
    expect(unsubscribes).toEqual([{ id: 'id-a' }, { id: 'id-b' }, { ids: ['id-c', 'id-d'] }]);
  });
});

describe.each(PRODUCTS)('%s unsubscribe() arguments', (product) => {
  const client = () => new WebSocketClient({ apiKey: 'test-key' })[product];

  test('channel with id or ids is INVALID_PARAMETER', () => {
    const ws = client();
    for (const options of [
      { channel: 'trades', symbol: '2330', id: 'id-a' },
      { channel: 'trades', symbol: '2330', ids: ['id-a'] },
    ]) {
      expect(() => ws.unsubscribe(options)).toThrow(
        expect.objectContaining({
          code: 1005,
          message: "Invalid parameter 'channel': cannot be combined with 'id' or 'ids'",
        }),
      );
    }
  });

  test('an unknown channel is INVALID_PARAMETER', () => {
    expect(() => client().unsubscribe({ channel: 'trade', symbol: '2330' })).toThrow(
      expect.objectContaining({ code: 1005 }),
    );
  });

  test('channel without symbol or symbols throws', () => {
    expect(() => client().unsubscribe({ channel: 'trades' })).toThrow(
      "unsubscribe() requires 'symbol' or 'symbols'",
    );
  });
});
