/**
 * Event-loop lifetime of WebSocket clients (#30).
 *
 * Registering a listener must not keep Node alive (1.x EventEmitter
 * semantics); an open connection must, and closing it must let the process
 * exit. Each case runs in a child process so a leaked handle shows up as a
 * process that never exits, instead of hanging Jest itself.
 */

const { spawn } = require('child_process');
const os = require('os');
const path = require('path');
const { WebSocketServer } = require('ws');

const PKG = path.resolve(__dirname, '..');

/** Loopback server that acks auth. Lives in the Jest process, not the child. */
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

/**
 * Loopback server that acks auth and, once subscribed, sends `data` frames
 * without pause until the connection closes.
 */
function startFloodServer() {
  return new Promise((resolve) => {
    const wss = new WebSocketServer({ host: '127.0.0.1', port: 0 });
    wss.on('connection', (socket) => {
      let timer = null;
      socket.on('message', (raw) => {
        const frame = JSON.parse(raw.toString());
        if (frame.event === 'auth') {
          socket.send(JSON.stringify({ event: 'authenticated', data: { message: 'Authenticated successfully' } }));
        } else if (frame.event === 'subscribe' && timer === null) {
          let i = 0;
          timer = setInterval(() => {
            for (let n = 0; n < 50 && socket.readyState === socket.OPEN; n += 1) {
              socket.send(JSON.stringify({ event: 'data', data: { i: i++ }, channel: 'trades' }));
            }
          }, 1);
        }
      });
      socket.on('close', () => clearInterval(timer));
    });
    wss.on('listening', () => resolve(wss));
  });
}

function closeServer(wss) {
  for (const socket of wss.clients) socket.terminate();
  return new Promise((resolve) => wss.close(resolve));
}

/**
 * Run `script` in a child Node process. Resolves with `exited: false` if it is
 * still running after `timeoutMs`, or when `onLine(line, kill)` kills it; the
 * deadline is generous because a loaded machine can take seconds just to
 * start the child and connect (#63). `lineAt` maps each distinct stdout line
 * to when it first arrived; `exitAt` is exit time; `killedBy` is `'timeout'`,
 * `'test'` or null.
 */
function runChild(script, { env = {}, timeoutMs = 10000, onLine } = {}) {
  return new Promise((resolve) => {
    const startAt = Date.now();
    const child = spawn(process.execPath, ['-e', script], {
      cwd: PKG,
      env: { ...process.env, ...env },
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    const lines = [];
    const lineAt = {};
    const timeline = [];
    let stderr = '';
    let killedBy = null;
    const kill = (by) => {
      if (killedBy === null) killedBy = by;
      child.kill('SIGKILL');
    };
    child.stdout.on('data', (chunk) => {
      for (const line of chunk.toString().split('\n').filter(Boolean)) {
        const at = Date.now();
        lines.push(line);
        timeline.push(`+${at - startAt}ms ${line}`);
        if (!(line in lineAt)) lineAt[line] = at;
        if (onLine) onLine(line, () => kill('test'));
      }
    });
    child.stderr.on('data', (chunk) => {
      stderr += chunk;
    });
    const timer = setTimeout(() => kill('timeout'), timeoutMs);
    child.on('exit', (code, signal) => {
      clearTimeout(timer);
      const exitAt = Date.now();
      resolve({
        exited: signal === null, code, lines, lineAt, timeline, startAt, exitAt, killedBy, timeoutMs, stderr,
      });
    });
  });
}

/**
 * Run `assertions` against a child result; on failure, append the child's
 * stdout and stderr so an intermittent CI failure shows what the child did.
 */
function expectChild(result, assertions) {
  try {
    assertions();
  } catch (err) {
    // Elapsed time and load tell a loaded machine from a regression (#63).
    const killed = result.killedBy ? ` killedBy=${result.killedBy}` : '';
    err.message += `\n\nchild exited=${result.exited} code=${result.code}${killed}`
      + ` after ${result.exitAt - result.startAt}ms (timeout ${result.timeoutMs}ms)`
      + `, loadavg=${os.loadavg().map((n) => n.toFixed(1)).join(' ')}`
      + `\nstdout:\n${result.timeline.join('\n')}\nstderr:\n${result.stderr}`;
    throw err;
  }
}

const PRODUCTS = ['stock', 'futopt'];
const EVENTS = ['message', 'connect', 'disconnect', 'reconnect', 'error', 'authenticated', 'unauthenticated'];

describe.each(PRODUCTS)('%s process lifetime (#30)', (product) => {
  let wss;
  let url;

  beforeEach(async () => {
    wss = await startServer();
    url = `ws://127.0.0.1:${wss.address().port}`;
  });

  afterEach(async () => {
    await closeServer(wss);
  });

  test('registering listeners without connecting does not keep the process alive', async () => {
    const result = await runChild(`
      const { WebSocketClient } = require('./');
      const ws = new WebSocketClient({ apiKey: 'test-key' })[${JSON.stringify(product)}];
      for (const event of ${JSON.stringify(EVENTS)}) ws.on(event, () => {});
    `);

    expectChild(result, () => {
      expect(result).toMatchObject({ exited: true, code: 0 });
    });
  });

  // Connecting may take seconds on a loaded machine; staying alive is judged
  // only from CONNECTED on, over a fixed window (#63).
  test('an open connection keeps the process alive', async () => {
    const STAY_ALIVE_MS = 1000;
    const result = await runChild(
      `
      const { WebSocketClient } = require('./');
      const ws = new WebSocketClient({ apiKey: 'test-key', baseUrl: process.env.URL })[${JSON.stringify(product)}];
      ws.on('message', () => {});
      ws.connect().then(() => console.log('CONNECTED'));
    `,
      {
        env: { URL: url },
        onLine: (line, kill) => {
          if (line === 'CONNECTED') setTimeout(kill, STAY_ALIVE_MS);
        },
      },
    );

    expectChild(result, () => {
      expect(result.lines).toContain('CONNECTED');
      expect(result).toMatchObject({ exited: false, killedBy: 'test' });
      expect(result.exitAt - result.lineAt.CONNECTED).toBeGreaterThanOrEqual(STAY_ALIVE_MS);
    });
  });

  test('disconnect() lets the process exit after delivering the disconnect event', async () => {
    const result = await runChild(
      `
      const { WebSocketClient } = require('./');
      const ws = new WebSocketClient({ apiKey: 'test-key', baseUrl: process.env.URL })[${JSON.stringify(product)}];
      ws.on('disconnect', (event) => console.log('DISCONNECT ' + JSON.stringify(event)));
      ws.connect().then(() => ws.disconnect());
    `,
      { env: { URL: url } },
    );

    expectChild(result, () => {
      expect(result).toMatchObject({ exited: true, code: 0 });
      expect(result.lines).toEqual(['DISCONNECT {"code":1000,"reason":"Normal closure"}']);
    });
  });

  test('a server-initiated close lets the process exit', async () => {
    const result = await runChild(
      `
      const { WebSocketClient } = require('./');
      const ws = new WebSocketClient({ apiKey: 'test-key', baseUrl: process.env.URL })[${JSON.stringify(product)}];
      ws.on('disconnect', (event) => console.log('DISCONNECT ' + JSON.stringify(event)));
      ws.connect().then(() => console.log('CONNECTED'));
    `,
      {
        env: { URL: url },
        onLine: (line) => {
          if (line === 'CONNECTED') for (const socket of wss.clients) socket.close(1001, 'going away');
        },
      },
    );

    expectChild(result, () => {
      expect(result).toMatchObject({ exited: true, code: 0 });
      expect(result.lines).toContain('DISCONNECT {"code":1001,"reason":"going away"}');
    });
  });

  // The final `disconnect` callback is queued on a weak threadsafe function
  // just before the keep-alive handle is released. Between cycles nothing else
  // holds the event loop, so a callback lost to that ordering would end the
  // child early with fewer than CYCLES events.
  test('every disconnect event is delivered across repeated connect/disconnect cycles', async () => {
    const CYCLES = 50;
    const result = await runChild(
      `
      const { WebSocketClient } = require('./');
      const client = new WebSocketClient({ apiKey: 'test-key', baseUrl: process.env.URL });
      let delivered = 0;
      function cycle(n) {
        if (n === ${CYCLES}) return;
        const ws = client[${JSON.stringify(product)}];
        ws.on('disconnect', () => {
          delivered += 1;
          console.log('DELIVERED ' + delivered);
          cycle(n + 1);
        });
        ws.connect().then(() => ws.disconnect());
      }
      cycle(0);
    `,
      { env: { URL: url }, timeoutMs: 60000 },
    );

    expectChild(result, () => {
      expect(result).toMatchObject({ exited: true, code: 0 });
      expect(result.lines[result.lines.length - 1]).toBe(`DELIVERED ${CYCLES}`);
    });
  }, 70000);

  test('a failed connect lets the process exit after delivering the error event', async () => {
    const result = await runChild(`
      const { WebSocketClient } = require('./');
      const ws = new WebSocketClient({ apiKey: 'test-key', baseUrl: 'ws://127.0.0.1:1' })[${JSON.stringify(product)}];
      ws.on('error', () => console.log('ERROR'));
      ws.connect().catch(() => console.log('REJECTED'));
    `);

    expectChild(result, () => {
      expect(result.lines).toEqual(expect.arrayContaining(['REJECTED', 'ERROR']));
      expect(result).toMatchObject({ exited: true, code: 0 });
    });
  });

  // `dispatch_ended` must only follow `ReconnectFailed`: a disconnect that core
  // is still going to retry must not release the worker or the keep-alive.
  test('stays alive through the reconnect backoff, then exits once reconnection fails', async () => {
    const BACKOFF_MS = 1000;
    const result = await runChild(
      `
      const { WebSocketClient } = require('./');
      const ws = new WebSocketClient({
        apiKey: 'test-key',
        baseUrl: process.env.URL,
        reconnect: { enabled: true, maxAttempts: 1, initialDelayMs: ${BACKOFF_MS}, maxDelayMs: ${BACKOFF_MS} },
      })[${JSON.stringify(product)}];
      ws.on('disconnect', () => console.log('DISCONNECT'));
      ws.on('reconnect', () => console.log('RECONNECT'));
      ws.on('error', (err) => console.log('ERROR ' + err.message));
      ws.connect().then(() => console.log('CONNECTED'));
    `,
      {
        env: { URL: url },
        timeoutMs: 15000,
        // Drop the connection and stop listening, so the retry after the
        // backoff is refused and core gives up.
        onLine: (line) => {
          if (line === 'CONNECTED') {
            for (const socket of wss.clients) socket.terminate();
            wss.close();
          }
        },
      },
    );

    expectChild(result, () => {
      expect(result.lines).toEqual(expect.arrayContaining(['DISCONNECT', 'RECONNECT']));
      expect(result.lines[result.lines.length - 1]).toMatch(/^ERROR Reconnection failed/);
      expect(result).toMatchObject({ exited: true, code: 0 });
      // Alive for the whole backoff window after the drop.
      expect(result.exitAt - result.lineAt.DISCONNECT).toBeGreaterThanOrEqual(BACKOFF_MS * 0.8);
    });
  }, 20000);

  test('an exception thrown by a listener is reported through error, not uncaughtException (#83)', async () => {
    const result = await runChild(
      `
      const { WebSocketClient } = require('./');
      process.on('uncaughtException', (err) => console.log('UNCAUGHT ' + err.message));
      const ws = new WebSocketClient({ apiKey: 'test-key', baseUrl: process.env.URL })[${JSON.stringify(product)}];
      ws.on('disconnect', () => {
        const err = new Error('boom');
        err.custom = 42;
        throw err;
      });
      ws.on('error', (err) => console.log('ERROR ' + err.code + ' ' + err.event + ' ' + err.cause.custom));
      ws.connect().then(() => ws.disconnect());
    `,
      { env: { URL: url } },
    );

    expectChild(result, () => {
      expect(result.lines).toEqual(['ERROR 3004 disconnect 42']);
      expect(result).toMatchObject({ exited: true, code: 0 });
    });
  });

  test('a panicked worker thread lets the process exit after delivering error and disconnect (#25)', async () => {
    const result = await runChild(
      `
      const { WebSocketClient } = require('./');
      const ws = new WebSocketClient({ apiKey: 'test-key', baseUrl: process.env.URL })[${JSON.stringify(product)}];
      ws.on('error', (err) => console.log('ERROR ' + err.code));
      ws.on('disconnect', () => console.log('DISCONNECT'));
      ws.connect().then(() => console.log('CONNECTED'));
    `,
      { env: { URL: url, FUGLE_MARKETDATA_TEST_PANIC: 'ws_worker' } },
    );

    expectChild(result, () => {
      // The panic follows the connection immediately, so its error and
      // disconnect listeners may run before connect()'s continuation (#62).
      expect([...result.lines].sort()).toEqual(['CONNECTED', 'DISCONNECT', 'ERROR -1']);
      expect(result.lines.indexOf('ERROR -1')).toBeLessThan(result.lines.indexOf('DISCONNECT'));
      expect(result).toMatchObject({ exited: true, code: 0 });
    });
  });
});

// A slow `message` listener with a small `messageBuffer` fills the in-flight
// allowance, so the stream reader waits for room (#46). Ending the connection
// from there must still let the process exit: a lost in-flight slot would keep
// the reader, and the keep-alive it holds, waiting forever.
describe.each(PRODUCTS)('%s process lifetime under message backpressure (#46)', (product) => {
  let wss;
  let url;

  beforeEach(async () => {
    wss = await startFloodServer();
    url = `ws://127.0.0.1:${wss.address().port}`;
  });

  afterEach(async () => {
    await closeServer(wss);
  });

  /** Child-side client: prints FULL once the listener has fallen behind. */
  const slowClient = `
    const { WebSocketClient } = require('./');
    const ws = new WebSocketClient({ apiKey: 'test-key', baseUrl: process.env.URL, messageBuffer: 4 })[${JSON.stringify(product)}];
    let seen = 0;
    ws.on('message', () => {
      const until = Date.now() + 20;
      while (Date.now() < until);
      seen += 1;
      if (seen === 20) {
        console.log('FULL');
        if (typeof onFull === 'function') onFull();
      }
    });
    ws.on('disconnect', () => console.log('DISCONNECT dropped=' + (ws.messagesDroppedTotal > 0)));
    ws.connect().then(() => ws.subscribe({ channel: 'trades', symbol: '2330' }));
  `;

  test('disconnect() while the in-flight allowance is full lets the process exit', async () => {
    const result = await runChild(`let onFull = () => ws.disconnect();\n${slowClient}`, {
      env: { URL: url },
      timeoutMs: 20000,
    });

    expectChild(result, () => {
      expect(result).toMatchObject({ exited: true, code: 0 });
      // Dropped messages show the reader really was held back.
      expect(result.lines).toEqual(['FULL', 'DISCONNECT dropped=true']);
    });
  }, 30000);

  test('a server close while the in-flight allowance is full lets the process exit', async () => {
    const result = await runChild(`let onFull;\n${slowClient}`, {
      env: { URL: url },
      timeoutMs: 20000,
      onLine: (line) => {
        if (line === 'FULL') for (const socket of wss.clients) socket.close(1001, 'going away');
      },
    });

    expectChild(result, () => {
      expect(result).toMatchObject({ exited: true, code: 0 });
      expect(result.lines).toEqual(['FULL', 'DISCONNECT dropped=true']);
    });
  }, 30000);
});
