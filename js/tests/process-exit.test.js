/**
 * Event-loop lifetime of WebSocket clients (#30).
 *
 * Registering a listener must not keep Node alive (1.x EventEmitter
 * semantics); an open connection must, and closing it must let the process
 * exit. Each case runs in a child process so a leaked handle shows up as a
 * process that never exits, instead of hanging Jest itself.
 */

const { spawn } = require('child_process');
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

function closeServer(wss) {
  for (const socket of wss.clients) socket.terminate();
  return new Promise((resolve) => wss.close(resolve));
}

/**
 * Run `script` in a child Node process. Resolves with `exited: false` if it is
 * still running after `timeoutMs` (the child is then killed).
 */
function runChild(script, { env = {}, timeoutMs = 5000, onLine } = {}) {
  return new Promise((resolve) => {
    const child = spawn(process.execPath, ['-e', script], {
      cwd: PKG,
      env: { ...process.env, ...env },
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    const lines = [];
    let stderr = '';
    child.stdout.on('data', (chunk) => {
      for (const line of chunk.toString().split('\n').filter(Boolean)) {
        lines.push(line);
        if (onLine) onLine(line);
      }
    });
    child.stderr.on('data', (chunk) => {
      stderr += chunk;
    });
    const timer = setTimeout(() => child.kill('SIGKILL'), timeoutMs);
    child.on('exit', (code, signal) => {
      clearTimeout(timer);
      resolve({ exited: signal === null, code, lines, stderr });
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
    err.message += `\n\nchild exited=${result.exited} code=${result.code}`
      + `\nstdout:\n${result.lines.join('\n')}\nstderr:\n${result.stderr}`;
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

  test('an open connection keeps the process alive', async () => {
    const result = await runChild(
      `
      const { WebSocketClient } = require('./');
      const ws = new WebSocketClient({ apiKey: 'test-key', baseUrl: process.env.URL })[${JSON.stringify(product)}];
      ws.on('message', () => {});
      ws.connect().then(() => console.log('CONNECTED'));
    `,
      { env: { URL: url }, timeoutMs: 2000 },
    );

    expectChild(result, () => {
      expect(result.lines).toContain('CONNECTED');
      expect(result.exited).toBe(false);
    });
  });

  test('disconnect() lets the process exit after delivering the disconnect event', async () => {
    const result = await runChild(
      `
      const { WebSocketClient } = require('./');
      const ws = new WebSocketClient({ apiKey: 'test-key', baseUrl: process.env.URL })[${JSON.stringify(product)}];
      ws.on('disconnect', (reason) => console.log('DISCONNECT ' + reason));
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
      ws.on('disconnect', (reason) => console.log('DISCONNECT ' + reason));
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

  test('a failed connect lets the process exit', async () => {
    const result = await runChild(`
      const { WebSocketClient } = require('./');
      const ws = new WebSocketClient({ apiKey: 'test-key', baseUrl: 'ws://127.0.0.1:1' })[${JSON.stringify(product)}];
      ws.on('error', () => {});
      ws.connect().catch(() => console.log('REJECTED'));
    `);

    expectChild(result, () => {
      expect(result.lines).toContain('REJECTED');
      expect(result).toMatchObject({ exited: true, code: 0 });
    });
  });
});
