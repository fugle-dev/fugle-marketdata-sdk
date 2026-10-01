/**
 * The `disconnect` event says who closed the connection (`intent`) and
 * whether a reconnect follows (`willReconnect`), forwarded from core's
 * `Disconnected` event (#293).
 */

const { WebSocketServer } = require('ws');
const { WebSocketClient } = require('../');

/**
 * Loopback server that authenticates every client and sends nothing else,
 * so a client with a heartbeat timeout sees the connection go silent.
 */
function startServer() {
  return new Promise((resolve) => {
    const wss = new WebSocketServer({ host: '127.0.0.1', port: 0 });
    wss.on('connection', (socket) => {
      socket.on('message', (raw) => {
        const frame = JSON.parse(raw.toString());
        if (frame.event === 'auth') {
          socket.send(JSON.stringify({ event: 'authenticated', data: { message: 'Authenticated successfully' } }));
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

const NO_RECONNECT = { reconnect: { enabled: false } };
const FAST_RECONNECT = { reconnect: { enabled: true, maxAttempts: 1, initialDelayMs: 100, maxDelayMs: 100 } };

describe.each(['stock', 'futopt'])('%s disconnect event intent / willReconnect (#293)', (product) => {
  let wss;
  let ws;
  let disconnects;
  let errors;

  async function setup(options = {}) {
    wss = await startServer();
    const { port } = wss.address();
    const client = new WebSocketClient({ apiKey: 'test-key', baseUrl: `ws://127.0.0.1:${port}`, ...options });
    ws = client[product];
    disconnects = [];
    errors = [];
    ws.on('disconnect', (event) => disconnects.push(event));
    ws.on('error', (err) => errors.push(err.code));
    await ws.connect();
  }

  afterEach(async () => {
    if (ws) {
      ws.disconnect();
      ws = undefined;
    }
    if (wss) {
      await closeServer(wss);
      wss = undefined;
    }
  });

  test('your disconnect(): code 1000, intent client, willReconnect false', async () => {
    await setup();
    ws.disconnect();
    await waitFor(() => disconnects.length === 1, 'disconnect');

    expect(disconnects).toEqual([{ code: 1000, reason: 'Normal closure', intent: 'client', willReconnect: false }]);
  });

  test('server Close 1000: intent server, willReconnect false', async () => {
    await setup(FAST_RECONNECT);
    for (const socket of wss.clients) socket.close(1000, 'bye');
    await waitFor(() => disconnects.length === 1, 'disconnect');

    expect(disconnects[0]).toEqual({ code: 1000, reason: 'bye', intent: 'server', willReconnect: false });
    await waitFor(() => ws.isClosed, 'isClosed');
  });

  test('server Close 1001 with reconnect on: intent server, willReconnect true; then your disconnect() is client', async () => {
    await setup(FAST_RECONNECT);
    const reconnects = [];
    ws.on('reconnect', (event) => reconnects.push(event));
    for (const socket of wss.clients) socket.close(1001, 'going away');
    await waitFor(() => reconnects.length === 1 && ws.isConnected, 'reconnected');

    ws.disconnect();
    await waitFor(() => disconnects.length === 2, 'second disconnect');

    expect(disconnects).toEqual([
      { code: 1001, reason: 'going away', intent: 'server', willReconnect: true },
      { code: 1000, reason: 'Normal closure', intent: 'client', willReconnect: false },
    ]);
  });

  test('server Close 1001 with reconnect off: willReconnect false', async () => {
    await setup(NO_RECONNECT);
    for (const socket of wss.clients) socket.close(1001, 'going away');
    await waitFor(() => disconnects.length === 1, 'disconnect');

    expect(disconnects[0]).toEqual({ code: 1001, reason: 'going away', intent: 'server', willReconnect: false });
  });

  test('TCP drop without a Close frame: code null, intent network', async () => {
    await setup(NO_RECONNECT);
    for (const socket of wss.clients) socket.terminate();
    await waitFor(() => disconnects.length === 1, 'disconnect');

    expect(disconnects[0]).toMatchObject({ code: null, intent: 'network', willReconnect: false });
  });

  test('reconnect given up: the drop said willReconnect true, the end is error 3005', async () => {
    await setup(FAST_RECONNECT);
    // Stop accepting and drop the connection: the one attempt fails.
    await closeServer(wss);
    wss = undefined;
    await waitFor(() => errors.includes(3005), 'error 3005', 10000);

    expect(disconnects).toHaveLength(1);
    expect(disconnects[0]).toMatchObject({ code: null, intent: 'network', willReconnect: true });
    expect(ws.isClosed).toBe(true);
  });

  test.each([
    ['off', NO_RECONNECT, false],
    ['on', FAST_RECONNECT, true],
  ])('heartbeat timeout, reconnect %s: reported by disconnect with intent network, not by error 3003', async (_, reconnect, willReconnect) => {
    await setup({ ...reconnect, healthCheck: { heartbeatTimeoutMs: 5000 } });
    await waitFor(() => disconnects.length >= 1, 'disconnect', 15000);

    expect(disconnects[0]).toMatchObject({ code: null, intent: 'network', willReconnect });
    expect(disconnects[0].reason).toMatch(/^Heartbeat timeout after/);
    expect(errors).not.toContain(3003);
  }, 20000);
});
