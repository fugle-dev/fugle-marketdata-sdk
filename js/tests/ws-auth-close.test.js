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
