/**
 * `connect()` on a client that is already connected or connecting rejects
 * instead of starting a second worker that shares `isConnected` and the
 * callbacks (#44). A connection that has ended — `disconnect()`, a server
 * close with no reconnect left, a failed auth — can be connected again, even
 * from the callback or rejection that reports the end.
 */

const { WebSocketServer } = require('ws');
const { WebSocketClient } = require('../');

/**
 * Loopback server that acks auth and answers `subscribe` with `subscribed`
 * plus one `data` frame. The first `failAuth` auth attempts are rejected;
 * the first `slowAuth` are answered only after `authDelayMs`.
 */
function startServer({ failAuth = 0, slowAuth = 0, authDelayMs = 300 } = {}) {
  return new Promise((resolve) => {
    const wss = new WebSocketServer({ host: '127.0.0.1', port: 0 });
    wss.accepted = 0;
    let authFailuresLeft = failAuth;
    let slowAuthsLeft = slowAuth;
    wss.on('connection', (socket) => {
      wss.accepted += 1;
      socket.on('message', (raw) => {
        const frame = JSON.parse(raw.toString());
        switch (frame.event) {
          case 'auth':
            if (authFailuresLeft > 0) {
              authFailuresLeft -= 1;
              socket.send(JSON.stringify({ event: 'error', data: { message: 'Invalid API key' } }));
              socket.close(4001, 'unauthorized');
            } else {
              const ack = () => socket.send(JSON.stringify({ event: 'authenticated', data: { message: 'Authenticated successfully' } }));
              if (slowAuthsLeft > 0) {
                slowAuthsLeft -= 1;
                setTimeout(() => socket.readyState === socket.OPEN && ack(), authDelayMs);
              } else {
                ack();
              }
            }
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

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function waitFor(predicate, what, timeoutMs = 5000) {
  const deadline = Date.now() + timeoutMs;
  while (!predicate()) {
    if (Date.now() > deadline) throw new Error(`timed out waiting for ${what}`);
    await sleep(20);
  }
}

const openSockets = (wss) => [...wss.clients].filter((s) => s.readyState === s.OPEN).length;

const PRODUCTS = [
  ['stock', { channel: 'trades', symbol: '2330' }],
  ['futopt', { channel: 'trades', symbol: 'TXF1!', afterHours: true }],
];

describe.each(PRODUCTS)('%s connect() reuse (#44)', (product, subscription) => {
  let wss;
  let ws;

  async function setup(serverOptions) {
    wss = await startServer(serverOptions);
    const { port } = wss.address();
    const client = new WebSocketClient({ apiKey: 'test-key', baseUrl: `ws://127.0.0.1:${port}` });
    ws = client[product];
  }

  afterEach(async () => {
    try {
      ws.disconnect();
    } catch (e) {
      // already gone
    }
    await closeServer(wss);
  });

  test('second connect() while connecting rejects and opens no second connection', async () => {
    await setup();
    const connects = [];
    const data = [];
    ws.on('connect', () => connects.push(Date.now()));
    ws.on('message', (raw) => {
      const msg = JSON.parse(raw);
      if (msg.event === 'data') data.push(msg);
    });

    const first = ws.connect();
    await expect(ws.connect()).rejects.toThrow('[2011] Already connected');
    await first;

    ws.subscribe(subscription);
    await waitFor(() => data.length > 0, 'data frame');
    await sleep(300);

    expect(wss.accepted).toBe(1);
    expect(connects).toHaveLength(1);
    expect(data).toHaveLength(1);
  });

  test('connect() while connected rejects and leaves the connection working', async () => {
    await setup();
    const data = [];
    ws.on('message', (raw) => {
      const msg = JSON.parse(raw);
      if (msg.event === 'data') data.push(msg);
    });

    await ws.connect();
    await expect(ws.connect()).rejects.toThrow('[2011] Already connected');

    expect(ws.isConnected).toBe(true);
    ws.subscribe(subscription);
    await waitFor(() => data.length > 0, 'data frame');
    expect(wss.accepted).toBe(1);
  });

  test('connect() right after disconnect() reconnects', async () => {
    await setup();
    await ws.connect();

    ws.disconnect();
    await ws.connect();

    expect(ws.isConnected).toBe(true);
    expect(ws.isClosed).toBe(false);
    expect(wss.accepted).toBe(2);
    await waitFor(() => openSockets(wss) === 1, 'old socket to close');
  });

  test('connect() from a disconnect handler after a server close reconnects', async () => {
    await setup();
    let reconnect;
    ws.on('disconnect', () => {
      // Reconnect once: tearing the test down fires this handler again, and a
      // second connect() while the first is connecting rightly rejects.
      if (reconnect === undefined) reconnect = ws.connect();
    });
    await ws.connect();

    for (const socket of wss.clients) socket.close(1001, 'going away');
    await waitFor(() => reconnect !== undefined, 'disconnect event');
    await reconnect;

    expect(ws.isConnected).toBe(true);
    expect(ws.isClosed).toBe(false);
    expect(wss.accepted).toBe(2);
  });

  test('disconnect() before connect() resolves aborts that connect()', async () => {
    await setup({ slowAuth: 1 });
    // `connect` fires when the socket opens, so the aborted connection's
    // authentication is what must not be reported (#23).
    const authenticated = [];
    ws.on('authenticated', (data) => authenticated.push(data));

    const pending = ws.connect();
    ws.disconnect();

    await expect(pending).rejects.toThrow('[2010] Connection aborted');
    await sleep(200);
    expect(authenticated).toHaveLength(0);
    expect(ws.isConnected).toBe(false);
    expect(ws.isClosed).toBe(true);
    await waitFor(() => openSockets(wss) === 0, 'aborted socket to close');
  });

  test('connect() after disconnect() while still authenticating: only the new connect() resolves', async () => {
    await setup({ slowAuth: 1 });
    const authenticated = [];
    const data = [];
    ws.on('authenticated', (auth) => authenticated.push(auth));
    ws.on('message', (raw) => {
      const msg = JSON.parse(raw);
      if (msg.event === 'data') data.push(msg);
    });

    const abandoned = ws.connect();
    ws.disconnect();
    const current = ws.connect();

    await expect(abandoned).rejects.toThrow('[2010] Connection aborted');
    await current;

    expect(authenticated).toHaveLength(1);
    expect(ws.isConnected).toBe(true);
    expect(ws.isClosed).toBe(false);
    expect(wss.accepted).toBe(2);
    await waitFor(() => openSockets(wss) === 1, 'abandoned socket to close');

    ws.subscribe(subscription);
    await waitFor(() => data.length > 0, 'data frame');
    await sleep(300);
    expect(data).toHaveLength(1);
  });

  test('connect() retried after an auth failure is not rejected as already connected', async () => {
    await setup({ failAuth: 1 });

    await expect(ws.connect()).rejects.toEqual({ message: 'Invalid API key' });
    await ws.connect();

    expect(ws.isConnected).toBe(true);
    expect(wss.accepted).toBe(2);
  });
});
