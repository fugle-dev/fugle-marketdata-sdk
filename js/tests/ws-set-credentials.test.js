/**
 * `setCredentials()` changes what later connection attempts and requests
 * send (#322).
 *
 * The loopback server accepts only the new credential once the test asks it
 * to. A client connected with the old one, given the new one, authenticates
 * its automatic reconnect with it, in the field of its kind; a live
 * connection gets no new auth frame. `ws.setCredentials()` reaches both
 * product clients; `ws.stock.setCredentials()` only the stock client. After a
 * rejected `connect()`, setting the new credential and connecting again
 * succeeds. A REST client sends the new credential from its next request on,
 * through product clients taken before. Anything but exactly one non-blank
 * credential throws code 1004 and leaves the current one in place; a value
 * that is not a credentials object is a `TypeError`.
 */

const http = require('http');
const { WebSocketServer } = require('ws');
const { RestClient, WebSocketClient } = require('../');

const OLD = 'old-key';
const NEW = 'new-token';

/**
 * Loopback server that records the `data` of every auth frame. It acks every
 * auth, or, once `wss.required` is set, only one carrying that credential;
 * any other gets the server's rejection: `error` 1000, then a Close.
 */
function startServer() {
  return new Promise((resolve) => {
    const wss = new WebSocketServer({ host: '127.0.0.1', port: 0 });
    wss.authData = [];
    wss.required = null;
    wss.on('connection', (socket) => {
      socket.on('message', (raw) => {
        const frame = JSON.parse(raw.toString());
        if (frame.event !== 'auth') return;
        wss.authData.push(frame.data);
        const { apikey, token, sdkToken } = frame.data;
        if (wss.required !== null && ![apikey, token, sdkToken].includes(wss.required)) {
          socket.send(JSON.stringify({ event: 'error', code: 1000, data: { message: 'Invalid token' } }));
          socket.close();
          return;
        }
        socket.send(JSON.stringify({ event: 'authenticated', data: { message: 'Authenticated successfully' } }));
      });
    });
    wss.on('listening', () => resolve(wss));
  });
}

function closeServer(wss) {
  for (const socket of wss.clients) socket.terminate();
  return new Promise((resolve) => wss.close(resolve));
}

function dropConnections(wss) {
  for (const socket of wss.clients) socket.terminate();
}

function newClient(wss) {
  const { port } = wss.address();
  return new WebSocketClient({
    apiKey: OLD,
    baseUrl: `ws://127.0.0.1:${port}`,
    reconnect: { maxAttempts: 2, initialDelayMs: 100, maxDelayMs: 100 },
  });
}

function waitFor(predicate, what, timeoutMs = 5000) {
  return new Promise((resolve, reject) => {
    const deadline = Date.now() + timeoutMs;
    const poll = () => {
      if (predicate()) return resolve();
      if (Date.now() > deadline) return reject(new Error(`no ${what} within ${timeoutMs}ms`));
      setTimeout(poll, 20);
    };
    poll();
  });
}

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

function thrown(fn) {
  try {
    fn();
  } catch (e) {
    return e;
  }
  throw new Error('did not throw');
}

const BAD = [
  ['none', {}],
  ['two', { apiKey: 'a', sdkToken: 'b' }],
  ['blank', { bearerToken: '   ' }],
];

describe('WebSocket setCredentials (#322)', () => {
  let wss;
  const opened = [];

  beforeEach(async () => {
    wss = await startServer();
  });

  afterEach(async () => {
    for (const ws of opened.splice(0)) {
      try {
        ws.disconnect();
      } catch (e) {
        // never connected, or already gone
      }
    }
    await closeServer(wss);
  });

  describe.each(['stock', 'futopt'])('%s', (product) => {
    test.each([
      ['on the product client', false],
      ['on the parent', true],
    ])('the automatic reconnect sends the new credential of another kind, set %s', async (_name, onParent) => {
      const client = newClient(wss);
      const ws = client[product];
      opened.push(ws);
      let authenticated = 0;
      ws.on('authenticated', () => {
        authenticated += 1;
      });
      await ws.connect();

      wss.required = NEW;
      (onParent ? client : ws).setCredentials({ sdkToken: NEW });
      dropConnections(wss);
      await waitFor(() => authenticated === 2, 'second authenticated');

      expect(wss.authData).toStrictEqual([{ apikey: OLD }, { sdkToken: NEW }]);
    });

    test('a rejected connect() succeeds after the credential is set', async () => {
      wss.required = NEW;
      const ws = newClient(wss)[product];
      opened.push(ws);
      await expect(ws.connect()).rejects.toBeDefined();

      ws.setCredentials({ sdkToken: NEW });
      await ws.connect();

      expect(wss.authData).toStrictEqual([{ apikey: OLD }, { sdkToken: NEW }]);
    });

    test('connect() after a rejected reconnect sends the new credential', async () => {
      const ws = newClient(wss)[product];
      opened.push(ws);
      let rejected = false;
      ws.on('unauthenticated', () => {
        rejected = true;
      });
      await ws.connect();
      wss.required = NEW;
      dropConnections(wss);
      await waitFor(() => rejected && ws.isClosed, 'the reconnect to end');

      ws.setCredentials({ sdkToken: NEW });
      await ws.connect();

      expect(wss.authData.at(-1)).toStrictEqual({ sdkToken: NEW });
    });
  });

  test('a live connection gets no new auth frame', async () => {
    const ws = newClient(wss).stock;
    opened.push(ws);
    await ws.connect();

    ws.setCredentials({ bearerToken: NEW });
    await sleep(200);

    expect(wss.authData).toStrictEqual([{ apikey: OLD }]);
  });

  test('the parent reaches both product clients', async () => {
    const client = newClient(wss);
    client.setCredentials({ bearerToken: NEW });
    const { stock, futopt } = client;
    opened.push(stock, futopt);
    await stock.connect();
    await futopt.connect();

    expect(wss.authData).toStrictEqual([{ token: NEW }, { token: NEW }]);
  });

  test('a product client changes only its own', async () => {
    const client = newClient(wss);
    client.stock.setCredentials({ sdkToken: NEW });
    const { stock, futopt } = client;
    opened.push(stock, futopt);
    await stock.connect();
    await futopt.connect();

    expect(wss.authData).toStrictEqual([{ sdkToken: NEW }, { apikey: OLD }]);
  });

  test.each(BAD)('%s is code 1004 and keeps the current credential', async (_name, credentials) => {
    const client = newClient(wss);
    for (const target of [client, client.stock]) {
      expect(thrown(() => target.setCredentials(credentials))).toMatchObject({ code: 1004 });
    }
    const ws = client.stock;
    opened.push(ws);
    await ws.connect();

    expect(wss.authData).toStrictEqual([{ apikey: OLD }]);
  });

  test('a value that is not a credentials object is a TypeError', () => {
    const ws = newClient(wss);
    for (const value of [NEW, undefined, { apiKey: 1 }, { apikey: NEW }]) {
      expect(thrown(() => ws.setCredentials(value)).name).toBe('TypeError');
      expect(thrown(() => ws.stock.setCredentials(value)).name).toBe('TypeError');
    }
  });
});

describe('RestClient setCredentials (#322)', () => {
  let server;
  let headers;
  let baseUrl;

  beforeEach(async () => {
    headers = [];
    server = http.createServer((req, res) => {
      headers.push(req.headers);
      res.writeHead(401, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({ message: 'Unauthorized', statusCode: 401 }));
    });
    await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
    baseUrl = `http://127.0.0.1:${server.address().port}`;
  });

  afterEach(async () => {
    server.closeAllConnections();
    await new Promise((resolve) => server.close(resolve));
  });

  test('later requests send the new credential, through product clients taken before', async () => {
    const client = new RestClient({ apiKey: OLD, baseUrl });
    const intraday = client.stock.intraday;
    client.setCredentials({ sdkToken: NEW });

    // The server answers 401; only the header matters.
    await expect(intraday.quote({ symbol: '2330' })).rejects.toBeDefined();
    await expect(client.stock.intraday.quote({ symbol: '2330' })).rejects.toBeDefined();

    expect(headers.map((h) => h['x-sdk-token'])).toStrictEqual([NEW, NEW]);
    expect(headers.every((h) => !('x-api-key' in h))).toBe(true);
  });

  test('a value that is not a credentials object is a TypeError', () => {
    const client = new RestClient({ apiKey: OLD, baseUrl });
    for (const value of [NEW, undefined, { apiKey: 1 }, { apikey: NEW }]) {
      expect(thrown(() => client.setCredentials(value)).name).toBe('TypeError');
    }
  });

  test.each([...BAD, ['header', { apiKey: 'bad\nkey' }]])(
    '%s is code 1004 and keeps the current credential',
    async (_name, credentials) => {
      const client = new RestClient({ bearerToken: OLD, baseUrl });
      expect(thrown(() => client.setCredentials(credentials))).toMatchObject({ code: 1004 });

      await expect(client.stock.intraday.quote({ symbol: '2330' })).rejects.toBeDefined();

      expect(headers[0].authorization).toBe(`Bearer ${OLD}`);
    },
  );
});
