/**
 * A panic on a connection's worker or event thread is reported instead of
 * leaving the connection silently dead (#25).
 *
 * Debug builds panic on demand where `FUGLE_MARKETDATA_TEST_PANIC` says,
 * read from the real process environment when `connect()` is called. Jest's
 * `process.env` is a sandbox copy that never reaches it, so each case runs in
 * a child process started with the variable set, and reports what it saw as
 * one JSON line.
 */

const { spawn } = require('child_process');
const path = require('path');
const { WebSocketServer } = require('ws');

const PKG = path.resolve(__dirname, '..');

/** Loopback server that acks auth; lives in the Jest process. */
function startServer() {
  return new Promise((resolve) => {
    const wss = new WebSocketServer({ host: '127.0.0.1', port: 0 });
    wss.on('connection', (socket) => {
      socket.on('message', (raw) => {
        if (JSON.parse(raw.toString()).event === 'auth') {
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

/**
 * Run `script` in a child Node process with `site` as the panic site and
 * resolve with the object it prints on its `RESULT ` line (killed after
 * `timeoutMs`).
 */
function runChild(script, { url, site, timeoutMs = 10000 }) {
  return new Promise((resolve) => {
    const child = spawn(process.execPath, ['-e', script], {
      cwd: PKG,
      env: { ...process.env, URL: url, FUGLE_MARKETDATA_TEST_PANIC: site },
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
    child.on('exit', (code, signal) => {
      clearTimeout(timer);
      const line = stdout.split('\n').find((l) => l.startsWith('RESULT '));
      resolve({
        code,
        signal,
        result: line ? JSON.parse(line.slice('RESULT '.length)) : undefined,
        stdout,
        stderr,
      });
    });
  });
}

/** Shared child-side helpers, prepended to every script. */
const PRELUDE = `
  const { WebSocketClient } = require('./');
  const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
  async function waitFor(predicate, timeoutMs = 2000) {
    const deadline = Date.now() + timeoutMs;
    while (!predicate() && Date.now() < deadline) await sleep(20);
  }
  const describeError = (err) => ({
    isError: Object.prototype.toString.call(err) === '[object Error]',
    code: err && err.code,
    sourceKind: err && err.sourceKind,
    message: err && err.message,
  });
  function record(ws) {
    const events = { error: [], disconnect: [], authenticated: [] };
    ws.on('error', (err) => events.error.push(describeError(err)));
    ws.on('disconnect', (event) => events.disconnect.push(event));
    ws.on('authenticated', (data) => events.authenticated.push(data));
    return events;
  }
`;

const PRODUCTS = [
  ['stock', { channel: 'trades', symbol: '2330' }],
  ['futopt', { channel: 'trades', symbol: 'TXF1!', afterHours: true }],
];

describe.each(PRODUCTS)('%s thread panic supervision (#25)', (product, subscription) => {
  let wss;
  let url;

  beforeEach(async () => {
    wss = await startServer();
    url = `ws://127.0.0.1:${wss.address().port}`;
  });

  afterEach(async () => {
    await closeServer(wss);
  });

  test('a worker panic after connecting fires error and disconnect and marks the client disconnected', async () => {
    const run = await runChild(
      `${PRELUDE}
      (async () => {
        const ws = new WebSocketClient({ apiKey: 'test-key', baseUrl: process.env.URL })[${JSON.stringify(product)}];
        const events = record(ws);
        await ws.connect();
        await waitFor(() => events.error.length > 0 && events.disconnect.length > 0);
        await sleep(300);
        let subscribeThrew = false;
        try { ws.subscribe(${JSON.stringify(subscription)}); } catch (e) { subscribeThrew = true; }
        const afterPanic = { isConnected: ws.isConnected, isClosed: ws.isClosed, subscribeThrew };
        console.log('RESULT ' + JSON.stringify({ events, afterPanic }));
      })();
    `,
      { url, site: 'ws_worker' },
    );

    const detail = `stdout:\n${run.stdout}\nstderr:\n${run.stderr}`;
    expect({ code: run.code, detail }).toMatchObject({ code: 0 });
    const { events, afterPanic } = run.result;
    expect(events.error).toHaveLength(1);
    expect(events.error[0]).toMatchObject({ isError: true, code: -1, sourceKind: 'protocol' });
    expect(events.error[0].message).toMatch(/^WebSocket worker thread panicked: injected test panic at ws_worker/);
    expect(events.disconnect).toEqual([{ code: null, reason: events.error[0].message }]);
    expect(afterPanic).toEqual({ isConnected: false, isClosed: true, subscribeThrew: true });
  });

  test('connect() works again after a worker panic', async () => {
    const run = await runChild(
      `${PRELUDE}
      (async () => {
        const ws = new WebSocketClient({ apiKey: 'test-key', baseUrl: process.env.URL })[${JSON.stringify(product)}];
        const events = record(ws);
        await ws.connect();
        await waitFor(() => events.disconnect.length > 0);
        // Only the connection being started reads the variable.
        delete process.env.FUGLE_MARKETDATA_TEST_PANIC;
        await ws.connect();
        await sleep(300);
        console.log('RESULT ' + JSON.stringify({ isConnected: ws.isConnected, errors: events.error.length }));
        ws.disconnect();
      })();
    `,
      { url, site: 'ws_worker' },
    );

    expect({ code: run.code, stderr: run.stderr }).toMatchObject({ code: 0 });
    expect(run.result).toEqual({ isConnected: true, errors: 1 });
  });

  test('a worker panic right after authenticating does not turn the reported connect() into a rejection', async () => {
    // Hold the JS thread while the connection authenticates and the worker
    // panics, so the authenticated and error listeners are both pending when
    // it frees up; error's was registered first and runs first.
    const run = await runChild(
      `${PRELUDE}
      (async () => {
        const ws = new WebSocketClient({ apiKey: 'test-key', baseUrl: process.env.URL })[${JSON.stringify(product)}];
        const events = record(ws);
        const pending = ws.connect();
        const until = Date.now() + 800;
        while (Date.now() < until) {}
        const outcome = await pending.then(() => 'resolved', (e) => 'rejected: ' + (e && e.message));
        await waitFor(() => events.disconnect.length > 0);
        console.log('RESULT ' + JSON.stringify({ outcome, errors: events.error.length, disconnects: events.disconnect.length, authenticated: events.authenticated.length }));
      })();
    `,
      { url, site: 'ws_worker' },
    );

    expect({ code: run.code, stderr: run.stderr }).toMatchObject({ code: 0 });
    expect(run.result).toEqual({ outcome: 'resolved', errors: 1, disconnects: 1, authenticated: 1 });
  });

  test('an event thread panic rejects connect(), fires error and shuts the connection down', async () => {
    const run = await runChild(
      `${PRELUDE}
      (async () => {
        const ws = new WebSocketClient({ apiKey: 'test-key', baseUrl: process.env.URL })[${JSON.stringify(product)}];
        const events = record(ws);
        const rejection = await ws.connect().then(() => null, (reason) => describeError(reason));
        await sleep(300);
        const afterPanic = { isConnected: ws.isConnected, isClosed: ws.isClosed };
        const panicked = JSON.parse(JSON.stringify(events));
        delete process.env.FUGLE_MARKETDATA_TEST_PANIC;
        await ws.connect();
        console.log('RESULT ' + JSON.stringify({ rejection, events: panicked, afterPanic, reconnected: ws.isConnected }));
        ws.disconnect();
      })();
    `,
      { url, site: 'ws_events' },
    );

    expect({ code: run.code, stderr: run.stderr }).toMatchObject({ code: 0 });
    const { rejection, events, afterPanic, reconnected } = run.result;
    expect(rejection).toMatchObject({ isError: true });
    expect(rejection).toMatchObject({ code: -1, sourceKind: 'protocol' });
    expect(rejection.message).toMatch(/^WebSocket event thread panicked: injected test panic at ws_events/);
    expect(events.error).toHaveLength(1);
    expect(events.error[0]).toMatchObject({ isError: true, code: -1, sourceKind: 'protocol' });
    expect(events.authenticated).toEqual([]);
    // Never reported connected, so no disconnect either.
    expect(events.disconnect).toEqual([]);
    expect(afterPanic).toEqual({ isConnected: false, isClosed: true });
    expect(reconnected).toBe(true);
  });
});
