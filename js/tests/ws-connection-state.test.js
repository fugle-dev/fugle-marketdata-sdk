/**
 * `isConnected` / `isClosed` read core's connection state rather than flags
 * the binding keeps (#67), so they follow auto-reconnects and server closes.
 */

const { WebSocketServer } = require('ws');
const { WebSocketClient } = require('../');

/** Loopback server that acks every auth. */
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

describe.each(['stock', 'futopt'])('%s isConnected / isClosed follow core connection state (#67)', (product) => {
  let wss;
  let ws;

  async function setup(clientOptions = {}) {
    wss = await startServer();
    const { port } = wss.address();
    const client = new WebSocketClient({ apiKey: 'test-key', baseUrl: `ws://127.0.0.1:${port}`, ...clientOptions });
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

  test('before connect() the client is neither connected nor closed', async () => {
    await setup();
    expect(ws.isConnected).toBe(false);
    expect(ws.isClosed).toBe(false);
  });

  test('without a reconnect option the client reconnects after a drop (#149)', async () => {
    await setup();
    let authenticated = 0;
    ws.on('authenticated', () => {
      authenticated += 1;
    });
    const reconnects = [];
    ws.on('reconnect', (event) => reconnects.push(event));
    await ws.connect();

    for (const socket of wss.clients) socket.terminate();
    await waitFor(() => authenticated === 2, 'reauthentication');
    expect(reconnects[0]).toEqual({ attempt: 1 });
    expect(ws.isConnected).toBe(true);
  });

  test('isConnected is false while auto-reconnecting and true once reconnected', async () => {
    await setup({ reconnect: { enabled: true, maxAttempts: 3, initialDelayMs: 300, maxDelayMs: 300 } });
    const duringReconnect = [];
    let authenticated = 0;
    ws.on('authenticated', () => {
      authenticated += 1;
    });
    ws.on('reconnect', () => duringReconnect.push({ isConnected: ws.isConnected, isClosed: ws.isClosed }));
    await ws.connect();
    expect(ws.isConnected).toBe(true);

    for (const socket of wss.clients) socket.close(1001, 'going away');
    await waitFor(() => duringReconnect.length > 0, 'reconnect event');
    expect(duringReconnect[0]).toEqual({ isConnected: false, isClosed: false });

    await waitFor(() => authenticated === 2, 'reauthentication');
    expect(ws.isConnected).toBe(true);
    expect(ws.isClosed).toBe(false);
  });

  test('a server close with no reconnect left marks the client closed by its disconnect listener (#86)', async () => {
    await setup({ reconnect: { enabled: false } });
    const inListener = [];
    ws.on('disconnect', () => inListener.push({ isConnected: ws.isConnected, isClosed: ws.isClosed }));
    await ws.connect();

    for (const socket of wss.clients) socket.close(1001, 'going away');
    await waitFor(() => inListener.length > 0, 'disconnect event');
    expect(inListener).toEqual([{ isConnected: false, isClosed: true }]);
  });

  test('a drop followed by a reconnect reads not connected in the disconnect listener (#86)', async () => {
    await setup({ reconnect: { enabled: true, maxAttempts: 3, initialDelayMs: 300, maxDelayMs: 300 } });
    const inListener = [];
    ws.on('disconnect', () => inListener.push({ isConnected: ws.isConnected, isClosed: ws.isClosed }));
    await ws.connect();

    for (const socket of wss.clients) socket.terminate();
    await waitFor(() => inListener.length > 0, 'disconnect event');
    expect(inListener[0]).toEqual({ isConnected: false, isClosed: false });
  });

  test('disconnect() during the reconnect backoff fires a final disconnect event (#98)', async () => {
    await setup({ reconnect: { enabled: true, maxAttempts: 3, initialDelayMs: 500, maxDelayMs: 500 } });
    const disconnects = [];
    const reconnects = [];
    ws.on('disconnect', (event) => disconnects.push({ ...event, isClosed: ws.isClosed }));
    ws.on('reconnect', (event) => reconnects.push(event));
    await ws.connect();

    for (const socket of wss.clients) socket.terminate();
    await waitFor(() => reconnects.length > 0, 'reconnect event');
    ws.disconnect();
    await waitFor(() => disconnects.length === 2, 'final disconnect event');
    // Past the backoff: nothing follows the final event.
    await sleep(700);

    expect(disconnects[0].isClosed).toBe(false);
    expect(disconnects[1]).toEqual({ code: 1000, reason: 'Normal closure', isClosed: true });
    expect(disconnects).toHaveLength(2);
    expect(reconnects).toEqual([{ attempt: 1 }]);
    expect(ws.isClosed).toBe(true);
  });
});
