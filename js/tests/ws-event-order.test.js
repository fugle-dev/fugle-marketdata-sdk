/**
 * Listeners run in the order core emits the connection events, and connect()
 * settles after the listener of the event that settles it (#62). Every event
 * goes through one threadsafe function per connection; with one per listener
 * Node-API did not guarantee their relative order.
 *
 * `message` keeps its place among the other events: core delivers messages
 * and events on one ordered stream, read by one reader (#68).
 */

const { spawn } = require('child_process');
const path = require('path');
const { WebSocketServer } = require('ws');
const { WebSocketClient } = require('../');

const PKG = path.resolve(__dirname, '..');

/** Run `script` in a child Node process; resolve with its `RESULT ` line. */
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
      resolve({ code, result: line && JSON.parse(line.slice('RESULT '.length)), stderr });
    });
  });
}

/**
 * Loopback server. Auth is acked, or rejected while `rejectAuth` is set.
 * With `dropAfterAuth`, the socket is terminated right after the ack. Each
 * `subscribe` is answered with one `data` frame carrying its symbol, or with
 * `floodThenClose` frames numbered from 0 followed by a Close.
 *
 * `wss.burst(n)` sends `n` `data` frames to every client right away;
 * `wss.sent` counts those written.
 */
function startServer({ rejectAuth = false, dropAfterAuth = false, floodThenClose = 0 } = {}) {
  return new Promise((resolve) => {
    const wss = new WebSocketServer({ host: '127.0.0.1', port: 0 });
    wss.sent = 0;
    wss.burst = (n) => {
      for (const socket of wss.clients) {
        for (let i = 0; i < n; i += 1) {
          socket.send(JSON.stringify({ event: 'data', data: { i }, channel: 'trades' }), (err) => {
            if (!err) wss.sent += 1;
          });
        }
      }
    };
    wss.on('connection', (socket) => {
      socket.on('message', (raw) => {
        if (JSON.parse(raw.toString()).event !== 'auth') return;
        if (rejectAuth) {
          // The server's rejection: `error` code 1000, then a Close without a code (#201).
          socket.send(JSON.stringify({ event: 'error', code: 1000, data: { message: 'Invalid API key' } }));
          socket.close();
          return;
        }
        socket.send(JSON.stringify({ event: 'authenticated', data: { message: 'Authenticated successfully' } }));
        if (dropAfterAuth) setTimeout(() => socket.terminate(), 20);
      });
      socket.on('message', (raw) => {
        const frame = JSON.parse(raw.toString());
        if (frame.event !== 'subscribe') return;
        const { channel, symbol } = frame.data;
        if (floodThenClose > 0) {
          for (let i = 0; i < floodThenClose; i += 1) {
            socket.send(JSON.stringify({ event: 'data', data: { symbol, i }, channel }));
          }
          socket.close(1000, 'done');
          return;
        }
        socket.send(JSON.stringify({ event: 'data', data: { symbol }, id: `${channel}-${symbol}`, channel }));
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

async function waitFor(predicate, what, timeoutMs = 10000) {
  const deadline = Date.now() + timeoutMs;
  while (!predicate()) {
    if (Date.now() > deadline) throw new Error(`timed out waiting for ${what}`);
    await sleep(10);
  }
}

/** Record every event (but `message`) into `events`, by name. */
function record(ws, events) {
  for (const name of ['connect', 'authenticated', 'unauthenticated', 'disconnect', 'reconnect', 'error']) {
    ws.on(name, () => events.push(name));
  }
}

/** Record every event, `message` included, as `message:<event>`. */
function recordAll(ws, events) {
  record(ws, events);
  ws.on('message', (raw) => events.push(`message:${JSON.parse(raw).event}`));
}

const CYCLES = 50;

describe.each(['stock', 'futopt'])('%s event order (#62)', (product) => {
  let wss;
  let ws;

  async function setup(serverOptions, clientOptions = {}) {
    wss = await startServer(serverOptions);
    const { port } = wss.address();
    ws = new WebSocketClient({ apiKey: 'test-key', baseUrl: `ws://127.0.0.1:${port}`, ...clientOptions })[product];
  }

  afterEach(async () => {
    try {
      ws.disconnect();
    } catch (e) {
      // already gone
    }
    await closeServer(wss);
  });

  test('connect, authenticated, then connect() resolves, on every connection', async () => {
    await setup();
    const events = [];
    record(ws, events);

    for (let i = 0; i < CYCLES; i += 1) {
      events.length = 0;
      await ws.connect().then(() => events.push('resolved'));
      ws.disconnect();
      await waitFor(() => events.includes('disconnect'), `disconnect of connection ${i}`);
      expect({ i, events }).toEqual({ i, events: ['connect', 'authenticated', 'resolved', 'disconnect'] });
    }
  }, 60000);

  test('connect, unauthenticated, then connect() rejects, on every attempt', async () => {
    await setup({ rejectAuth: true });
    const events = [];
    record(ws, events);

    for (let i = 0; i < CYCLES; i += 1) {
      events.length = 0;
      await ws.connect().then(
        () => events.push('resolved'),
        () => events.push('rejected'),
      );
      expect({ i, events: events.slice(0, 3) }).toEqual({ i, events: ['connect', 'unauthenticated', 'rejected'] });
    }
  }, 60000);

  test('each reconnect reports disconnect, reconnect, connect, authenticated in order', async () => {
    const DROPS = 10;
    await setup({ dropAfterAuth: true }, { reconnect: { maxAttempts: 3, initialDelayMs: 100, maxDelayMs: 100 } });
    const events = [];
    record(ws, events);

    await ws.connect();
    const authenticatedCount = () => events.filter((e) => e === 'authenticated').length;
    await waitFor(() => authenticatedCount() > DROPS, `${DROPS} reconnects`, 30000);
    ws.disconnect();

    // A dropped socket may also report a diagnostic `error` before its
    // `disconnect`; only the connection lifecycle is asserted.
    const lifecycle = events.filter((e) => e !== 'error');
    const cycle = ['disconnect', 'reconnect', 'connect', 'authenticated'];
    const expected = ['connect', 'authenticated'];
    for (let i = 0; i < DROPS; i += 1) expected.push(...cycle);
    expect(lifecycle.slice(0, expected.length)).toEqual(expected);
  }, 60000);

  test('events queued while the JS thread is busy run in order, with the listeners registered by then', async () => {
    // The client runs in a child process, so blocking its JS thread does not
    // also stall this loopback server.
    await setup();
    const { code, result, stderr } = await runChild(
      `
      const { WebSocketClient } = require('./');
      const ws = new WebSocketClient({ apiKey: 'test-key', baseUrl: process.env.URL })[${JSON.stringify(product)}];
      const events = [];
      const done = ws.connect().then(() => events.push('resolved'));
      // Authentication completes, and its events are queued, meanwhile.
      const until = Date.now() + 1000;
      while (Date.now() < until);
      ws.on('connect', () => events.push('connect'));
      ws.on('authenticated', () => events.push('authenticated'));
      done.then(() => {
        console.log('RESULT ' + JSON.stringify(events));
        ws.disconnect();
      });
    `,
      { URL: `ws://127.0.0.1:${wss.address().port}` },
    );
    expect({ code, stderr, result }).toEqual({ code: 0, stderr: '', result: ['connect', 'authenticated', 'resolved'] });
  });

  test('a listener replaced by on() while its events are queued never receives them', async () => {
    await setup();
    const { code, result, stderr } = await runChild(
      `
      const { WebSocketClient } = require('./');
      const ws = new WebSocketClient({ apiKey: 'test-key', baseUrl: process.env.URL })[${JSON.stringify(product)}];
      const events = [];
      ws.on('connect', () => events.push('old connect'));
      ws.on('authenticated', () => events.push('old authenticated'));
      const done = ws.connect().then(() => events.push('resolved'));
      // Authentication completes, and its events are queued, meanwhile.
      const until = Date.now() + 1000;
      while (Date.now() < until);
      ws.on('connect', () => events.push('new connect'));
      ws.on('authenticated', () => events.push('new authenticated'));
      done.then(() => {
        console.log('RESULT ' + JSON.stringify(events));
        ws.disconnect();
      });
    `,
      { URL: `ws://127.0.0.1:${wss.address().port}` },
    );
    expect({ code, stderr, result }).toEqual({
      code: 0,
      stderr: '',
      result: ['new connect', 'new authenticated', 'resolved'],
    });
  });

  test('message frames that arrive before a message listener is registered are not delivered to it', async () => {
    await setup();
    const { code, result, stderr } = await runChild(
      `
      const { WebSocketClient } = require('./');
      const ws = new WebSocketClient({ apiKey: 'test-key', baseUrl: process.env.URL })[${JSON.stringify(product)}];
      const symbols = [];
      ws.connect().then(() => {
        ws.subscribe(${JSON.stringify({ channel: 'trades', symbol: 'EARLY' })});
        // EARLY's frame arrives meanwhile, with no message listener yet.
        const until = Date.now() + 1000;
        while (Date.now() < until);
        ws.on('message', (raw) => {
          const { symbol } = JSON.parse(raw).data;
          symbols.push(symbol);
          if (symbol === 'LATE') {
            console.log('RESULT ' + JSON.stringify(symbols));
            ws.disconnect();
          }
        });
        ws.subscribe(${JSON.stringify({ channel: 'trades', symbol: 'LATE' })});
      });
    `,
      { URL: `ws://127.0.0.1:${wss.address().port}` },
    );
    expect({ code, stderr, result }).toEqual({ code: 0, stderr: '', result: ['LATE'] });
  });

  test('every message arrives between authenticated and disconnect (#68)', async () => {
    const FRAMES = 2000;
    await setup({ floodThenClose: FRAMES });
    const events = [];
    recordAll(ws, events);

    await ws.connect();
    ws.subscribe({ channel: 'trades', symbol: '2330' });
    await waitFor(() => events.includes('disconnect'), 'disconnect');
    await sleep(100);

    const data = Array(FRAMES).fill('message:data');
    expect(events).toEqual(['connect', 'authenticated', 'message:authenticated', ...data, 'disconnect']);
  });

  test('frames still arriving while disconnect() closes the connection reach message first (#68)', async () => {
    await setup();
    const events = [];
    recordAll(ws, events);

    await ws.connect();
    ws.disconnect();
    // Same tick: written after disconnect() was called, and before the
    // server can have read the client's Close, so they precede its reply.
    wss.burst(100);
    await waitFor(() => events.includes('disconnect'), 'disconnect');
    await sleep(200);

    // Every one of them reaches the listener, before `disconnect`, which
    // comes once and last.
    expect(wss.sent).toBe(100);
    expect(events.filter((e) => e === 'message:data').length).toBe(wss.sent);
    expect(events.filter((e) => e === 'disconnect')).toEqual(['disconnect']);
    expect(events[events.length - 1]).toBe('disconnect');
  });

  test('frames of a rejected authentication never reach message (#68)', async () => {
    await setup({ rejectAuth: true });
    const events = [];
    recordAll(ws, events);

    await ws.connect().catch(() => events.push('rejected'));
    await sleep(200);
    expect(events.filter((e) => e.startsWith('message'))).toEqual([]);
    expect(events.slice(0, 3)).toEqual(['connect', 'unauthenticated', 'rejected']);
  });

  test("each reconnect's frames arrive after its authenticated (#68)", async () => {
    const DROPS = 5;
    await setup({ dropAfterAuth: true }, { reconnect: { maxAttempts: 3, initialDelayMs: 100, maxDelayMs: 100 } });
    const events = [];
    recordAll(ws, events);

    await ws.connect();
    await waitFor(() => events.filter((e) => e === 'authenticated').length > DROPS, `${DROPS} reconnects`, 30000);
    ws.disconnect();

    const lifecycle = events.filter((e) => e !== 'error');
    const cycle = ['authenticated', 'message:authenticated', 'disconnect', 'reconnect', 'connect'];
    const expected = ['connect'];
    for (let i = 0; i < DROPS; i += 1) expected.push(...cycle);
    expect(lifecycle.slice(0, expected.length)).toEqual(expected);
  }, 60000);

  test('a listener may register listeners, which receive later events', async () => {
    await setup();
    const events = [];
    ws.on('connect', () => {
      events.push('connect');
      ws.on('authenticated', () => events.push('authenticated'));
    });

    await ws.connect();
    expect(events).toEqual(['connect', 'authenticated']);
  });
});
