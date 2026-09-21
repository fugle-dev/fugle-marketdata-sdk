/**
 * `connect()` on a client that is already connected or connecting rejects
 * instead of starting a second worker that shares `isConnected` and the
 * callbacks (#44). A connection that has ended — `disconnect()`, a server
 * close with no reconnect left, a failed auth — can be connected again, even
 * from the callback or rejection that reports the end.
 */

const { spawn } = require('child_process');
const path = require('path');
const { WebSocketServer } = require('ws');
const { WebSocketClient } = require('../');

const PKG = path.resolve(__dirname, '..');

/**
 * Run `script` in a child Node process with `env` added to the real process
 * environment (Jest's `process.env` is a sandbox copy the native addon never
 * sees) and resolve with the object printed on its `RESULT ` line.
 */
function runChild(script, env, timeoutMs = 10000) {
  return new Promise((resolve) => {
    const child = spawn(process.execPath, ['-e', script], {
      cwd: PKG,
      env: { ...process.env, ...env },
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    let stdout = '';
    let stderr = '';
    child.stdout.on('data', (chunk) => {
      stdout += chunk;
    });
    child.stderr.on('data', (chunk) => {
      stderr += chunk;
    });
    const timer = setTimeout(() => child.kill('SIGKILL'), timeoutMs);
    child.on('exit', (code) => {
      clearTimeout(timer);
      const line = stdout.split('\n').find((l) => l.startsWith('RESULT '));
      resolve({ code, result: line && JSON.parse(line.slice('RESULT '.length)), stdout, stderr });
    });
  });
}

/**
 * Loopback server that acks auth and answers `subscribe` with `subscribed`
 * plus one `data` frame. The first `failAuth` auth attempts are rejected;
 * the first `slowAuth` are answered only after `authDelayMs`.
 *
 * Set at run time: `rejectAuth` rejects every auth from then on, and the
 * next `refuse` new connections are closed at once. `subscribes` records each
 * `subscribe` frame's `data` with the number of the connection it came on.
 */
function startServer({ failAuth = 0, slowAuth = 0, authDelayMs = 300 } = {}) {
  return new Promise((resolve) => {
    const wss = new WebSocketServer({ host: '127.0.0.1', port: 0 });
    wss.accepted = 0;
    wss.subscribes = [];
    wss.rejectAuth = false;
    wss.refuse = 0;
    let authFailuresLeft = failAuth;
    let slowAuthsLeft = slowAuth;
    wss.on('connection', (socket) => {
      if (wss.refuse > 0) {
        wss.refuse -= 1;
        socket.terminate();
        return;
      }
      wss.accepted += 1;
      const conn = wss.accepted;
      socket.on('message', (raw) => {
        const frame = JSON.parse(raw.toString());
        switch (frame.event) {
          case 'auth':
            if (wss.rejectAuth || authFailuresLeft > 0) {
              authFailuresLeft -= 1;
              // The server's rejection: `error` code 1000, then a Close without a code (#201).
              socket.send(JSON.stringify({ event: 'error', code: 1000, data: { message: 'Invalid API key' } }));
              socket.close();
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
            wss.subscribes.push({ conn, data: frame.data });
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

/** Drop every connection without a Close frame, as a network failure does. */
function dropConnections(wss) {
  for (const socket of wss.clients) socket.terminate();
}

const PRODUCTS = [
  ['stock', { channel: 'trades', symbol: '2330' }, { channel: 'trades', symbol: '2317' }],
  ['futopt', { channel: 'trades', symbol: 'TXF1!', afterHours: true }, { channel: 'trades', symbol: 'MXF1!', afterHours: true }],
];

describe.each(PRODUCTS)('%s connect() reuse (#44)', (product, subscription, other) => {
  let wss;
  let ws;

  async function setup(serverOptions, clientOptions = {}) {
    wss = await startServer(serverOptions);
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
    await expect(ws.connect()).rejects.toMatchObject({ code: 2011, sourceKind: 'client', message: expect.stringMatching(/^Already connected/) });
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
    await expect(ws.connect()).rejects.toMatchObject({ code: 2011, sourceKind: 'client', message: expect.stringMatching(/^Already connected/) });

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
    // Reconnecting by hand needs auto-reconnect off (it is on by default, #149).
    await setup(undefined, { reconnect: { enabled: false } });
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

    await expect(pending).rejects.toMatchObject({ code: 2010, message: expect.stringMatching(/^Connection aborted/) });
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

    await expect(abandoned).rejects.toMatchObject({ code: 2010, message: expect.stringMatching(/^Connection aborted/) });
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

  test('disconnect() from the authenticated listener before the worker checks for an abort is an ordinary disconnect', async () => {
    await setup();
    // disconnect() from the authenticated listener, which then holds the JS
    // thread: connect() cannot settle until it returns, and meanwhile the
    // worker (held briefly after authenticating) reaches its abort check
    // with `ending` set. authenticated has already fired, so the worker must
    // not abort the connection it reported.
    const run = await runChild(
      `
      const { WebSocketClient } = require('./');
      (async () => {
        const ws = new WebSocketClient({ apiKey: 'test-key', baseUrl: process.env.URL })[${JSON.stringify(product)}];
        const order = [];
        ws.on('authenticated', () => {
          order.push('authenticated');
          ws.disconnect();
          const until = Date.now() + 600;
          while (Date.now() < until) {}
        });
        ws.on('disconnect', () => order.push('disconnect'));
        const outcome = await ws.connect().then(
          () => 'resolved',
          (e) => 'rejected: ' + (e && e.message),
        );
        order.push(outcome);
        const deadline = Date.now() + 3000;
        while (!order.includes('disconnect') && Date.now() < deadline) {
          await new Promise((r) => setTimeout(r, 20));
        }
        console.log('RESULT ' + JSON.stringify({ order, isConnected: ws.isConnected }));
      })();
    `,
      { URL: `ws://127.0.0.1:${wss.address().port}`, FUGLE_MARKETDATA_TEST_DELAY_AFTER_CONNECT_MS: '100' },
    );

    expect({ code: run.code, stderr: run.stderr }).toMatchObject({ code: 0 });
    const { order, isConnected } = run.result;
    // The listener's own disconnect() may be delivered before the awaiting
    // code resumes; what matters is that the reported connection resolved.
    expect(order[0]).toBe('authenticated');
    expect(order.filter((e) => e.startsWith('re'))).toEqual(['resolved']);
    expect(order.filter((e) => e === 'disconnect')).toHaveLength(1);
    expect(isConnected).toBe(false);
  });
});

/**
 * `connect()` while an automatic reconnect is in progress — typically from a
 * `disconnect` listener, as 1.x code reconnected by hand — waits for the
 * reconnect instead of rejecting with 2011 (#230).
 */
describe.each(PRODUCTS)('%s connect() during an auto-reconnect (#230)', (product, subscription, other) => {
  const AUTH = { message: 'Authenticated successfully' };
  let wss;
  let ws;

  async function setup(reconnect = {}) {
    wss = await startServer();
    const { port } = wss.address();
    const client = new WebSocketClient({
      apiKey: 'test-key',
      baseUrl: `ws://127.0.0.1:${port}`,
      reconnect: { initialDelayMs: 100, ...reconnect },
    });
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

  /**
   * Connect and subscribe, run `beforeDrop`, then drop the connection.
   * `onDisconnect` runs in the `disconnect` listener, for this drop only.
   */
  async function connectThenDrop(onDisconnect, beforeDrop = () => {}) {
    let armed = true;
    ws.on('disconnect', () => {
      if (armed) {
        armed = false;
        onDisconnect();
      }
    });
    await ws.connect();
    ws.subscribe(subscription);
    await waitFor(() => wss.subscribes.length === 1, 'subscribe');
    beforeDrop();
    dropConnections(wss);
    await waitFor(() => !armed, 'disconnect event');
  }

  /** `connect()`, its rejection marked handled until the test awaits it. */
  function join() {
    const joined = ws.connect();
    joined.catch(() => {});
    return joined;
  }

  test('resolves each waiting connect() with the reconnect\'s data, after the replay', async () => {
    await setup();
    const authenticated = [];
    ws.on('authenticated', (data) => authenticated.push(data));
    let joined;
    await connectThenDrop(() => {
      joined = Promise.all([ws.connect(), ws.connect()]).then((results) => {
        ws.subscribe(other);
        return results;
      });
    });

    expect(await joined).toEqual([AUTH, AUTH]);
    expect(authenticated).toEqual([AUTH, AUTH]);
    expect(ws.isConnected).toBe(true);

    const onReconnect = () => wss.subscribes.filter((s) => s.conn === 2).map((s) => s.data);
    await waitFor(() => onReconnect().length === 2, 'replay and new subscribe');
    await sleep(200);
    expect(wss.accepted).toBe(2);
    // The replay alone, then the subscribe made once connect() resolved.
    expect(onReconnect()).toEqual([
      expect.objectContaining({ symbol: subscription.symbol }),
      expect.objectContaining({ symbol: other.symbol }),
    ]);

    // Reconnected, as the listeners have seen: refused again.
    await expect(ws.connect()).rejects.toMatchObject({ code: 2011 });
  });

  test('resolves at once when core reconnected while the JS thread was busy', async () => {
    await setup();
    // In a child process: the listener holds the child's JS thread past the
    // reconnect, which this process's server must be free to answer. The
    // reconnect's `authenticated` is then queued behind the listener when
    // connect() is called.
    const run = runChild(
      `
      const { WebSocketClient } = require('./');
      const ws = new WebSocketClient({ apiKey: 'test-key', baseUrl: process.env.URL, reconnect: { initialDelayMs: 100 } })[${JSON.stringify(product)}];
      let armed = true;
      ws.on('disconnect', () => {
        if (!armed) return;
        armed = false;
        const until = Date.now() + 1000;
        while (Date.now() < until);
        ws.connect().then(
          (data) => {
            ws.subscribe(${JSON.stringify(other)});
            setTimeout(() => {
              console.log('RESULT ' + JSON.stringify({ data }));
              ws.disconnect();
            }, 300);
          },
          (e) => {
            console.log('RESULT ' + JSON.stringify({ error: e.code }));
            ws.disconnect();
          },
        );
      });
      ws.connect().then(() => ws.subscribe(${JSON.stringify(subscription)}));
    `,
      { URL: `ws://127.0.0.1:${wss.address().port}` },
    );
    await waitFor(() => wss.subscribes.length === 1, 'subscribe');
    dropConnections(wss);
    const { code, result, stderr } = await run;

    expect({ code, stderr, result }).toMatchObject({ code: 0, result: { data: AUTH } });
    expect(wss.accepted).toBe(2);
    expect(wss.subscribes.filter((s) => s.conn === 2).map((s) => s.data)).toEqual([
      expect.objectContaining({ symbol: subscription.symbol }),
      expect.objectContaining({ symbol: other.symbol }),
    ]);
  });

  test('keeps waiting through a failed attempt', async () => {
    await setup({ maxAttempts: 2 });
    let joined;
    await connectThenDrop(
      () => {
        joined = join();
      },
      () => {
        wss.refuse = 1;
      },
    );

    expect(await joined).toEqual(AUTH);
    expect(wss.accepted).toBe(2);
  });

  test('disconnect() while waiting rejects it with 2010', async () => {
    await setup({ initialDelayMs: 2000 });
    let joined;
    await connectThenDrop(() => {
      joined = join();
    });

    ws.disconnect();

    await expect(joined).rejects.toMatchObject({ code: 2010, message: expect.stringMatching(/^Connection aborted/) });
    expect(wss.accepted).toBe(1);
  });

  test('rejects with 3005 when the attempts run out', async () => {
    await setup({ maxAttempts: 1 });
    let joined;
    await connectThenDrop(
      () => {
        joined = join();
      },
      () => {
        wss.refuse = Infinity;
      },
    );

    await expect(joined).rejects.toMatchObject({ code: 3005, sourceKind: 'network' });
  });

  test('rejects with the server\'s data when the reconnect is refused', async () => {
    await setup();
    let joined;
    await connectThenDrop(
      () => {
        joined = join();
      },
      () => {
        wss.rejectAuth = true;
      },
    );

    await expect(joined).rejects.toEqual({ message: 'Invalid API key' });
  });
});
