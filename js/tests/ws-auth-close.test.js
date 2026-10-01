/**
 * A Close frame in place of the auth answer is reported with its code and
 * reason (#292).
 *
 * The server refuses a connection over its limit with
 * `Close(1001, "Maximum number of connections reached")` (`Close(1013)` once
 * fugle-realtime !640 ships); the error used to say only "Stream closed
 * during authentication". The limit has its own code, 2012; a `1001` with
 * another reason (a restart) stays 2001 (#300).
 */

const { WebSocketServer } = require('ws');
const { WebSocketClient } = require('../');

const REASON = 'Maximum number of connections reached';

/** Loopback server that answers every auth frame with `Close(code, reason)`. */
function startServer(code = 1001, reason = REASON) {
  return new Promise((resolve) => {
    const wss = new WebSocketServer({ host: '127.0.0.1', port: 0 });
    wss.on('connection', (socket) => {
      socket.on('message', (raw) => {
        if (JSON.parse(raw.toString()).event === 'auth') socket.close(code, reason);
      });
    });
    wss.on('listening', () => resolve(wss));
  });
}

function closeServer(wss) {
  for (const socket of wss.clients) socket.terminate();
  return new Promise((resolve) => wss.close(resolve));
}

describe.each(['stock', 'futopt'])('%s close during auth (#292)', (product) => {
  let wss;

  afterEach(async () => {
    if (wss) await closeServer(wss);
  });

  async function connectError() {
    const { port } = wss.address();
    const ws = new WebSocketClient({
      apiKey: 'test-key',
      baseUrl: `ws://127.0.0.1:${port}`,
      reconnect: { enabled: false },
    })[product];
    return ws.connect().then(
      () => {
        throw new Error('connect() resolved');
      },
      (e) => e,
    );
  }

  test('connect() rejects with the close code and reason', async () => {
    wss = await startServer();
    const err = await connectError();
    expect(err.code).toBe(2012);
    expect(err.message).toContain(`Stream closed during authentication (close 1001: ${REASON})`);
  });

  test('Close 1013 is the connection limit (2012)', async () => {
    wss = await startServer(1013, '');
    const err = await connectError();
    expect(err.code).toBe(2012);
    expect(err.message).toContain('Stream closed during authentication (close 1013)');
  });

  test('error 1003 is the connection limit (2012)', async () => {
    wss = await new Promise((resolve) => {
      const server = new WebSocketServer({ host: '127.0.0.1', port: 0 });
      server.on('connection', (socket) => {
        socket.on('message', (raw) => {
          if (JSON.parse(raw.toString()).event !== 'auth') return;
          socket.send(JSON.stringify({ event: 'error', code: 1003, data: { message: REASON } }));
          socket.close(1013);
        });
      });
      server.on('listening', () => resolve(server));
    });
    const err = await connectError();
    expect(err.code).toBe(2012);
    expect(err.message).toContain(`Authentication failed (server error 1003): ${REASON}`);
  });

  test('Close 1001 with another reason stays 2001', async () => {
    wss = await startServer(1001, 'Server restarting');
    const err = await connectError();
    expect(err.code).toBe(2001);
    expect(err.message).toContain('Stream closed during authentication (close 1001: Server restarting)');
  });
});

describe.each(['stock', 'futopt'])('%s connection limit during a reconnect (#300)', (product) => {
  let wss;

  afterEach(async () => {
    if (wss) await closeServer(wss);
  });

  test('each refused attempt is an error event with code 2012, and is retried', async () => {
    // The first connection authenticates and is then dropped; every later
    // one is refused at the limit.
    let connections = 0;
    wss = await new Promise((resolve) => {
      const server = new WebSocketServer({ host: '127.0.0.1', port: 0 });
      server.on('connection', (socket) => {
        const first = connections++ === 0;
        socket.on('message', (raw) => {
          if (JSON.parse(raw.toString()).event !== 'auth') return;
          if (!first) {
            socket.close(1001, REASON);
            return;
          }
          socket.send(JSON.stringify({ event: 'authenticated', data: { message: 'ok' } }));
          setTimeout(() => socket.terminate(), 50);
        });
      });
      server.on('listening', () => resolve(server));
    });
    const { port } = wss.address();
    const ws = new WebSocketClient({
      apiKey: 'test-key',
      baseUrl: `ws://127.0.0.1:${port}`,
      reconnect: { enabled: true, maxAttempts: 2, initialDelayMs: 100, maxDelayMs: 100 },
    })[product];
    const codes = [];
    const gaveUp = new Promise((resolve) => {
      ws.on('error', (err) => {
        codes.push(err.code);
        if (err.code === 3005) resolve();
      });
    });

    await ws.connect();
    await gaveUp;

    // The drop itself is reported first (3002, the transport error); then
    // each of the two attempts, and the give-up.
    expect(codes.slice(-3)).toEqual([2012, 2012, 3005]);
    expect(codes.filter((code) => code === 2001)).toEqual([]);
  });
});
