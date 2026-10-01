/**
 * A Close frame in place of the auth answer is reported with its code and
 * reason (#292).
 *
 * The server refuses a connection over its limit with
 * `Close(1001, "Maximum number of connections reached")`; the error used to
 * say only "Stream closed during authentication".
 */

const { WebSocketServer } = require('ws');
const { WebSocketClient } = require('../');

const REASON = 'Maximum number of connections reached';

/** Loopback server that answers every auth frame with the limit Close. */
function startServer() {
  return new Promise((resolve) => {
    const wss = new WebSocketServer({ host: '127.0.0.1', port: 0 });
    wss.on('connection', (socket) => {
      socket.on('message', (raw) => {
        if (JSON.parse(raw.toString()).event === 'auth') socket.close(1001, REASON);
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

  test('connect() rejects with the close code and reason', async () => {
    wss = await startServer();
    const { port } = wss.address();
    const ws = new WebSocketClient({
      apiKey: 'test-key',
      baseUrl: `ws://127.0.0.1:${port}`,
      reconnect: { enabled: false },
    })[product];

    const err = await ws.connect().then(
      () => {
        throw new Error('connect() resolved');
      },
      (e) => e,
    );

    expect(err.code).toBe(2001);
    expect(err.message).toContain(`Stream closed during authentication (close 1001: ${REASON})`);
  });
});
