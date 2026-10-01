/**
 * For 1.x compatibility, a failed connect() does not end the process when the
 * client has an `error` listener and the code only chains `.then(f)` on it,
 * as the 1.x README does (#307; `main.js`). Everything else keeps 3.0's
 * rejection: these cases lock which ways of using the Promise are swallowed
 * and which still reject unhandled.
 *
 * Each case runs in a child process with Node's default
 * `--unhandled-rejections=throw`: an unhandled rejection ends the child with
 * exit code 1 and prints the reason on stderr.
 */

const { spawn } = require('child_process');
const net = require('net');
const path = require('path');
const { WebSocketServer } = require('ws');

const PKG = path.resolve(__dirname, '..');
/** A port nothing listens on: connect() fails at once with a transport error. */
let REFUSED;

beforeAll(async () => {
  // A port just released rather than a fixed one, which a firewall dropping
  // packets to it would turn into a long connect timeout instead.
  const server = net.createServer();
  await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
  const { port } = server.address();
  await new Promise((resolve) => server.close(resolve));
  REFUSED = `ws://127.0.0.1:${port}`;
});

/** `rejectAuth`: answer auth with the server's rejection, as in ws-legacy-compat. */
function startServer({ rejectAuth = false } = {}) {
  return new Promise((resolve) => {
    const wss = new WebSocketServer({ host: '127.0.0.1', port: 0 });
    wss.on('connection', (socket) => {
      socket.on('message', (raw) => {
        if (JSON.parse(raw.toString()).event !== 'auth') return;
        if (rejectAuth) {
          socket.send(JSON.stringify({ event: 'error', code: 1000, data: { message: 'Invalid authentication credentials' } }));
          socket.close();
        } else {
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
 * Run `body` in a child with `ws` bound to `product`'s client of `url`, with
 * `nodeArgs` before the script. Resolves with its exit code, stdout lines and
 * stderr.
 */
function runChild(product, url, body, nodeArgs = []) {
  const script = `
    const { WebSocketClient } = require('./');
    const ws = new WebSocketClient({
      apiKey: 'test-key',
      baseUrl: ${JSON.stringify(url)},
      reconnect: { enabled: false },
    })[${JSON.stringify(product)}];
    ${body}
  `;
  return new Promise((resolve) => {
    const child = spawn(process.execPath, [...nodeArgs, '-e', script], { cwd: PKG, stdio: ['ignore', 'pipe', 'pipe'] });
    let stdout = '';
    let stderr = '';
    child.stdout.on('data', (chunk) => { stdout += chunk; });
    child.stderr.on('data', (chunk) => { stderr += chunk; });
    const timer = setTimeout(() => child.kill('SIGKILL'), 15000);
    child.on('exit', (code, signal) => {
      clearTimeout(timer);
      resolve({ code, signal, lines: stdout.split('\n').filter(Boolean), stderr });
    });
  });
}

const ON_ERROR = "ws.on('error', () => console.log('ERROR EVENT'));";

describe.each(['stock', 'futopt'])('%s: the 1.x README pattern', (product) => {
  test('connect().then(f) with an error listener: the failure does not end the process', async () => {
    const result = await runChild(product, REFUSED, `
      ${ON_ERROR}
      ws.connect().then(() => console.log('F'));
    `);
    expect(result).toMatchObject({ code: 0, lines: ['ERROR EVENT'] });
  });
});

describe('stock: which uses of a failed connect() reject unhandled', () => {
  // Swallowed: the error listener has the failure, and nothing handles the
  // rejection, as in 1.x where the Promise never settled.
  test.each([
    ['connect() alone', `${ON_ERROR} ws.connect();`],
    ['.then(f).then(g)', `${ON_ERROR} ws.connect().then(() => console.log('F')).then(() => console.log('G'));`],
    ['an error listener added after connect()', `ws.connect().then(() => console.log('F')); ${ON_ERROR}`],
    // The `error` event comes first and consumes it: it counts as present when connect() was called.
    ['a once() error listener', "ws.once('error', () => console.log('ERROR EVENT')); ws.connect().then(() => console.log('F'));"],
  ])('swallowed: %s', async (_, body) => {
    const result = await runChild('stock', REFUSED, body);
    expect(result).toMatchObject({ code: 0, lines: ['ERROR EVENT'] });
  });

  // Handled by the caller: it still receives the rejection, as 3.0 documents.
  test.each([
    ['await in try/catch', `
      (async () => {
        try { await ws.connect(); } catch (err) { console.log('CAUGHT ' + err.code); }
      })();
    `],
    ['.catch(r)', "ws.connect().catch((err) => console.log('CAUGHT ' + err.code));"],
    ['.then(f, r)', "ws.connect().then(() => console.log('F'), (err) => console.log('CAUGHT ' + err.code));"],
    ['.then(f).catch(r)', "ws.connect().then(() => console.log('F')).catch((err) => console.log('CAUGHT ' + err.code));"],
  ])('received by the caller: %s', async (_, body) => {
    const result = await runChild('stock', REFUSED, `${ON_ERROR}\n${body}`);
    expect(result.code).toBe(0);
    expect(result.lines).toEqual(expect.arrayContaining(['ERROR EVENT']));
    expect(result.lines).toContainEqual(expect.stringMatching(/^CAUGHT \d+$/));
  });

  // Still unhandled: these end the process as in 3.0 before #307.
  test.each([
    ['no error listener', "ws.connect().then(() => console.log('F'));", false],
    ['await without catch', "(async () => { await ws.connect(); console.log('F'); })();", true],
    ['.finally(f)', "ws.connect().finally(() => console.log('FINALLY'));", true],
    ['Promise.all([connect()])', "Promise.all([ws.connect()]).then(() => console.log('F'));", true],
    ['Promise.race([connect()])', "Promise.race([ws.connect()]).then(() => console.log('F'));", true],
    ['.finally(f).then(g)', "ws.connect().finally(() => console.log('FINALLY')).then(() => console.log('F'));", true],
    ['.catch(r) that rethrows, then .then(g)', "ws.connect().catch((err) => { throw err; }).then(() => console.log('F'));", true],
    // The async function's promise adopts connect()'s through then(resolve, reject).
    ['an async function returning connect()', "(async () => ws.connect())().then(() => console.log('F'));", true],
  ])('unhandled: %s', async (_, body, withErrorListener) => {
    const result = await runChild('stock', REFUSED, `${withErrorListener ? ON_ERROR : ''}\n${body}`);
    expect(result.code).toBe(1);
    expect(result.lines).not.toContain('F');
    expect(result.stderr).toMatch(/Connection refused|transport error/i);
  });
});

describe('stock: rejections that are not a connect failure stay unhandled', () => {
  let wss;
  afterEach(() => wss && closeServer(wss));

  test('an error thrown by f in connect().then(f)', async () => {
    wss = await startServer();
    const result = await runChild('stock', `ws://127.0.0.1:${wss.address().port}`, `
      ${ON_ERROR}
      ws.connect().then(() => { throw new Error('thrown by f'); });
    `);
    expect(result.code).toBe(1);
    expect(result.stderr).toContain('thrown by f');
  });

  test('rejected credentials: connect() rejects with the server data, as in 1.x', async () => {
    wss = await startServer({ rejectAuth: true });
    const result = await runChild('stock', `ws://127.0.0.1:${wss.address().port}`, `
      ${ON_ERROR}
      ws.on('unauthenticated', () => console.log('UNAUTHENTICATED'));
      ws.connect().then(() => console.log('F'));
    `);
    expect(result.code).toBe(1);
    expect(result.lines).toContain('UNAUTHENTICATED');
    expect(result.lines).not.toContain('F');
    // Not an Error: Node reports the plain `data` object as an unhandled rejection.
    expect(result.stderr).toContain('UnhandledPromiseRejection');
  });
});

describe('stock: --unhandled-rejections=strict', () => {
  test('the 1.x README pattern with an error listener still does not end the process', async () => {
    const result = await runChild('stock', REFUSED, `
      ${ON_ERROR}
      ws.connect().then(() => console.log('F'));
    `, ['--unhandled-rejections=strict']);
    expect(result).toMatchObject({ code: 0, lines: ['ERROR EVENT'] });
  });

  test('without an error listener the failure ends the process', async () => {
    const result = await runChild('stock', REFUSED, "ws.connect().then(() => console.log('F'));", [
      '--unhandled-rejections=strict',
    ]);
    expect(result.code).toBe(1);
    expect(result.lines).not.toContain('F');
  });
});

describe('stock: rejections that only reject, without an error event', () => {
  let wss;
  afterEach(() => wss && closeServer(wss));

  test('2010, connect() aborted by disconnect(): swallowed with an error listener', async () => {
    wss = await startServer();
    const result = await runChild('stock', `ws://127.0.0.1:${wss.address().port}`, `
      ${ON_ERROR}
      // disconnect() before authentication can complete, as in ws-connect-reuse.
      ws.connect().then(() => console.log('F'));
      ws.disconnect();
    `);
    expect(result.code).toBe(0);
    expect(result.lines).not.toContain('F');
  });

  test('2010 is what that connect() rejects with', async () => {
    wss = await startServer();
    const result = await runChild('stock', `ws://127.0.0.1:${wss.address().port}`, `
      ws.connect().catch((err) => console.log('CAUGHT ' + err.code));
      ws.disconnect();
    `);
    expect(result).toMatchObject({ code: 0, lines: ['CAUGHT 2010'] });
  });

  test('2011, already connected: unhandled even with an error listener', async () => {
    wss = await startServer();
    const result = await runChild('stock', `ws://127.0.0.1:${wss.address().port}`, `
      ${ON_ERROR}
      ws.connect().then(() => console.log('FIRST'));
      ws.connect().then(() => console.log('SECOND'));
    `);
    expect(result.code).toBe(1);
    expect(result.lines).not.toContain('SECOND');
    expect(result.stderr).toMatch(/Already connected/);
  });
});
