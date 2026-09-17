/**
 * A listener that throws, or whose Promise rejects, neither crashes the
 * process nor stops later events; the user is told through `error`
 * (code 3004), or `console.error` without an `error` listener (#83).
 *
 * Each case runs the client in a child process, so an uncaught exception
 * would show as a non-zero exit code rather than a Jest failure.
 */

const { spawn } = require('child_process');
const path = require('path');
const { WebSocketServer } = require('ws');

const PKG = path.resolve(__dirname, '..');

/** Run `body` in a child Node process; resolve with its `RESULT ` line. */
function runChild(body, env, timeoutMs = 15000) {
  const script = `
    const { WebSocketClient } = require('./');
    const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
    async function waitFor(predicate, what, timeoutMs = 5000) {
      const deadline = Date.now() + timeoutMs;
      while (!predicate()) {
        if (Date.now() > deadline) throw new Error('timed out waiting for ' + what);
        await sleep(10);
      }
    }
    // \`message\` also receives the authenticated frame.
    const isData = (raw) => JSON.parse(raw).event === 'data';
    const describeError = (err) => ({
      message: err.message,
      code: err.code,
      sourceKind: err.sourceKind,
      event: err.event,
      count: err.count,
      cause: err.cause instanceof Error ? err.cause.message : err.cause,
    });
    (async () => {
      ${body}
    })().catch((err) => {
      console.log('RESULT ' + JSON.stringify({ failed: String(err && err.stack) }));
      process.exit(2);
    });
  `;
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
 * Loopback server: acks auth; answers each `subscribe` with as many `data`
 * frames as its symbol says (`'3'` → three). With `dropAfterAuth`, the
 * socket is terminated right after the ack.
 */
function startServer({ dropAfterAuth = false } = {}) {
  return new Promise((resolve) => {
    const wss = new WebSocketServer({ host: '127.0.0.1', port: 0 });
    wss.on('connection', (socket) => {
      socket.on('message', (raw) => {
        const frame = JSON.parse(raw.toString());
        if (frame.event === 'auth') {
          socket.send(JSON.stringify({ event: 'authenticated', data: { message: 'Authenticated successfully' } }));
          if (dropAfterAuth) setTimeout(() => socket.terminate(), 20);
        } else if (frame.event === 'subscribe') {
          const { channel, symbol } = frame.data;
          for (let i = 0; i < Number(symbol); i += 1) {
            socket.send(JSON.stringify({ event: 'data', data: { symbol, i }, channel }));
          }
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

describe.each(['stock', 'futopt'])('%s listener failures (#83)', (product) => {
  let wss;
  let env;

  const client = `new WebSocketClient({ apiKey: 'test-key', baseUrl: process.env.URL })[${JSON.stringify(product)}]`;

  beforeEach(async () => {
    wss = await startServer();
    env = { URL: `ws://127.0.0.1:${wss.address().port}` };
  });

  afterEach(async () => {
    await closeServer(wss);
  });

  test('a throwing message listener is reported through error; later messages still arrive', async () => {
    const { code, result, stderr } = await runChild(
      `
      const ws = ${client};
      const received = [];
      const errors = [];
      ws.on('message', (raw) => {
        if (!isData(raw)) return;
        received.push(JSON.parse(raw).data.i);
        throw new Error('boom');
      });
      ws.on('error', (err) => errors.push(describeError(err)));
      await ws.connect();
      ws.subscribe({ channel: 'trades', symbol: '3' });
      await waitFor(() => received.length === 3, 'three messages');
      await sleep(100);
      ws.disconnect();
      console.log('RESULT ' + JSON.stringify({ received, errors }));
      `,
      env,
    );
    expect({ code, stderr }).toEqual({ code: 0, stderr: '' });
    expect(result.received).toEqual([0, 1, 2]);
    // Three failures within a second: the first is reported, the rest held back.
    expect(result.errors).toEqual([
      {
        message: "'message' listener threw: boom",
        code: 3004,
        sourceKind: 'client',
        event: 'message',
        count: 1,
        cause: 'boom',
      },
    ]);
  });

  test('later failures are reported at most once per second, with their count', async () => {
    const { code, result } = await runChild(
      `
      const ws = ${client};
      let received = 0;
      const errors = [];
      ws.on('message', (raw) => {
        if (!isData(raw)) return;
        received += 1;
        throw new Error('boom ' + received);
      });
      ws.on('error', (err) => errors.push(describeError(err)));
      await ws.connect();
      ws.subscribe({ channel: 'trades', symbol: '3' });
      await waitFor(() => received === 3, 'three messages');
      await sleep(1500);
      ws.subscribe({ channel: 'books', symbol: '1' });
      await waitFor(() => received === 4, 'fourth message');
      await sleep(100);
      ws.disconnect();
      console.log('RESULT ' + JSON.stringify({ errors }));
      `,
      env,
    );
    expect(code).toBe(0);
    expect(result.errors.map(({ count, cause }) => ({ count, cause }))).toEqual([
      { count: 1, cause: 'boom 1' },
      { count: 3, cause: 'boom 4' },
    ]);
  });

  test('without an error listener the failure is printed and the process keeps running', async () => {
    const { code, result, stderr } = await runChild(
      `
      const ws = ${client};
      let received = 0;
      ws.on('message', (raw) => {
        if (!isData(raw)) return;
        received += 1;
        throw new Error('unheard boom');
      });
      await ws.connect();
      ws.subscribe({ channel: 'trades', symbol: '2' });
      await waitFor(() => received === 2, 'two messages');
      ws.disconnect();
      console.log('RESULT ' + JSON.stringify({ received }));
      `,
      env,
    );
    expect(code).toBe(0);
    expect(result.received).toBe(2);
    expect(stderr).toContain('[@fugle/marketdata]');
    expect(stderr).toContain("'message' listener threw: unheard boom");
  });

  test('an error listener that throws is printed, not re-reported', async () => {
    const { code, result, stderr } = await runChild(
      `
      const ws = ${client};
      let received = 0;
      let errorCalls = 0;
      ws.on('message', (raw) => {
        if (!isData(raw)) return;
        received += 1;
        throw new Error('first boom');
      });
      ws.on('error', () => {
        errorCalls += 1;
        throw new Error('error listener boom');
      });
      await ws.connect();
      ws.subscribe({ channel: 'trades', symbol: '2' });
      await waitFor(() => received === 2, 'two messages');
      await sleep(100);
      ws.disconnect();
      console.log('RESULT ' + JSON.stringify({ received, errorCalls }));
      `,
      env,
    );
    expect(code).toBe(0);
    expect(result).toEqual({ received: 2, errorCalls: 1 });
    expect(stderr).toContain("'error' listener threw");
    expect(stderr).toContain('error listener boom');
  });

  test('a rejected Promise from a listener is reported like a throw', async () => {
    const { code, result, stderr } = await runChild(
      `
      const ws = ${client};
      const errors = [];
      let received = 0;
      ws.on('message', async (raw) => {
        if (!isData(raw)) return;
        received += 1;
        await sleep(1);
        throw new Error('async boom');
      });
      ws.on('error', (err) => errors.push(describeError(err)));
      await ws.connect();
      ws.subscribe({ channel: 'trades', symbol: '1' });
      await waitFor(() => errors.length === 1, 'the rejection');
      ws.disconnect();
      console.log('RESULT ' + JSON.stringify({ received, errors }));
      `,
      env,
    );
    expect({ code, stderr }).toEqual({ code: 0, stderr: '' });
    expect(result.errors).toEqual([
      {
        message: "'message' listener rejected: async boom",
        code: 3004,
        sourceKind: 'client',
        event: 'message',
        count: 1,
        cause: 'async boom',
      },
    ]);
  });

  test('a throwing connect listener does not keep connect() from resolving', async () => {
    const { code, result } = await runChild(
      `
      const ws = ${client};
      const errors = [];
      ws.on('connect', () => {
        throw new Error('connect boom');
      });
      ws.on('error', (err) => errors.push(describeError(err)));
      const data = await ws.connect();
      ws.disconnect();
      console.log('RESULT ' + JSON.stringify({ resolved: data.message, errors }));
      `,
      env,
    );
    expect(code).toBe(0);
    expect(result.resolved).toBe('Authenticated successfully');
    expect(result.errors.map((e) => [e.code, e.event, e.cause])).toEqual([[3004, 'connect', 'connect boom']]);
  });
});

describe('reconnection failure (#83)', () => {
  test('the error event carries code 3005', async () => {
    const wss = await startServer({ dropAfterAuth: true });
    const env = { URL: `ws://127.0.0.1:${wss.address().port}` };
    const pending = runChild(
      `
      const ws = new WebSocketClient({
        apiKey: 'test-key',
        baseUrl: process.env.URL,
        reconnect: { maxAttempts: 1, initialDelayMs: 100, maxDelayMs: 100 },
      }).stock;
      const errors = [];
      ws.on('error', (err) => errors.push(describeError(err)));
      await ws.connect();
      await waitFor(() => errors.some((e) => e.code === 3005), 'reconnection failure', 10000);
      console.log('RESULT ' + JSON.stringify({ failure: errors.find((e) => e.code === 3005) }));
      `,
      env,
    );
    // Stop accepting connections once the first one has authenticated, so
    // the reconnect fails.
    await new Promise((resolve) => wss.once('connection', resolve));
    await new Promise((resolve) => setTimeout(resolve, 50));
    await closeServer(wss);
    const { code, result } = await pending;
    expect(code).toBe(0);
    expect(result.failure).toMatchObject({
      message: 'Reconnection failed after 1 attempts',
      code: 3005,
      sourceKind: 'network',
    });
  });
});
