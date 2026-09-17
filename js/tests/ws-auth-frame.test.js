/**
 * The auth frame carries each credential in its own field (#91).
 *
 * The server reads `apikey`, `token` or `sdkToken` and rejects a frame with
 * more than one. Every kind used to go out as `apikey`.
 */

const { WebSocketServer } = require('ws');
const { WebSocketClient } = require('../');

/** Loopback server that records the `data` of every auth frame and acks it. */
function startServer() {
  return new Promise((resolve) => {
    const wss = new WebSocketServer({ host: '127.0.0.1', port: 0 });
    wss.authData = [];
    wss.on('connection', (socket) => {
      socket.on('message', (raw) => {
        const frame = JSON.parse(raw.toString());
        if (frame.event === 'auth') {
          wss.authData.push(frame.data);
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

const CREDENTIALS = [
  ['apiKey', { apiKey: 'the-key' }, { apikey: 'the-key' }],
  ['bearerToken', { bearerToken: 'the-token' }, { token: 'the-token' }],
  ['sdkToken', { sdkToken: 'the-sdk-token' }, { sdkToken: 'the-sdk-token' }],
];

describe.each(['stock', 'futopt'])('%s auth frame (#91)', (product) => {
  let wss;
  let ws;

  afterEach(async () => {
    try {
      ws.disconnect();
    } catch (e) {
      // already gone
    }
    await closeServer(wss);
  });

  test.each(CREDENTIALS)('%s is sent in its own field', async (_name, credential, expected) => {
    wss = await startServer();
    const { port } = wss.address();
    ws = new WebSocketClient({ ...credential, baseUrl: `ws://127.0.0.1:${port}` })[product];

    await ws.connect();

    expect(wss.authData).toStrictEqual([expected]);
  });
});
