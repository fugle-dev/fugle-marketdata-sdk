/**
 * Registering a listener does not keep the client alive, and a listener
 * whose `this` wrapper was collected still runs with a client as `this`
 * (#307). Each case runs in a child with `--expose-gc` and watches the
 * objects with a FinalizationRegistry.
 */

const { spawn } = require('child_process');
const path = require('path');
const { WebSocketServer } = require('ws');

const PKG = path.resolve(__dirname, '..');

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

/** Prelude: `collected(name)` runs GC until `name`'s object is finalized, or gives up. */
const PRELUDE = `
  const { WebSocketClient, StockWebSocketClient, FutOptWebSocketClient } = require('./');
  const finalized = new Set();
  const registry = new FinalizationRegistry((name) => finalized.add(name));
  async function collected(name) {
    for (let i = 0; i < 200 && !finalized.has(name); i += 1) {
      global.gc();
      await new Promise((resolve) => setImmediate(resolve));
    }
    return finalized.has(name);
  }
`;

function runChild(body, env = {}) {
  return new Promise((resolve) => {
    const child = spawn(process.execPath, ['--expose-gc', '-e', PRELUDE + body], {
      cwd: PKG,
      env: { ...process.env, ...env },
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    let stdout = '';
    let stderr = '';
    child.stdout.on('data', (chunk) => { stdout += chunk; });
    child.stderr.on('data', (chunk) => { stderr += chunk; });
    const timer = setTimeout(() => child.kill('SIGKILL'), 20000);
    child.on('exit', (code) => {
      clearTimeout(timer);
      resolve({ code, lines: stdout.split('\n').filter(Boolean), stderr });
    });
  });
}

describe.each([
  ['stock', 'StockWebSocketClient'],
  ['futopt', 'FutOptWebSocketClient'],
])('%s', (product, className) => {
  test('a client with listeners is collected once nothing else holds it', async () => {
    const result = await runChild(`
      (async () => {
        let ws = new WebSocketClient({ apiKey: 'k' });
        let wrapper = ws[${JSON.stringify(product)}];
        registry.register(ws, 'client');
        registry.register(wrapper, 'wrapper');
        wrapper.on('message', () => {}).once('error', () => {});
        ws = null;
        wrapper = null;
        console.log('CLIENT ' + await collected('client'));
        console.log('WRAPPER ' + await collected('wrapper'));
      })();
    `);
    expect(result).toMatchObject({ code: 0, lines: ['CLIENT true', 'WRAPPER true'], stderr: '' });
  });

  test('on() keeps working after the globals it uses are replaced', async () => {
    const result = await runChild(`
      const ws = new WebSocketClient({ apiKey: 'k' })[${JSON.stringify(product)}];
      ws.on('message', () => {});
      const { WeakRef: weakRef } = globalThis;
      const { bind } = Function.prototype;
      globalThis.WeakRef = undefined;
      Function.prototype.bind = null;
      let outcome;
      try {
        ws.on('error', () => {}).once('connect', () => {});
        outcome = 'COUNT ' + (ws.listenerCount('message') + ws.listenerCount('error') + ws.listenerCount('connect'));
      } catch (err) {
        outcome = 'THREW ' + err.message;
      }
      // Restored before printing: Node's own code uses them.
      globalThis.WeakRef = weakRef;
      Function.prototype.bind = bind;
      console.log(outcome);
    `);
    expect(result).toMatchObject({ code: 0, lines: ['COUNT 3'], stderr: '' });
  });

  describe('with a server', () => {
    let wss;
    beforeEach(async () => { wss = await startServer(); });
    afterEach(() => closeServer(wss));

    test('a listener registered on a collected wrapper gets another wrapper of the same client as this', async () => {
      const result = await runChild(`
        (async () => {
          const ws = new WebSocketClient({ apiKey: 'k', baseUrl: process.env.URL, reconnect: { enabled: false } });
          let wrapper = ws[${JSON.stringify(product)}];
          registry.register(wrapper, 'wrapper');
          let seen = null;
          wrapper.on('authenticated', function () { seen = this; });
          wrapper = null;
          console.log('WRAPPER ' + await collected('wrapper'));

          const client = ws[${JSON.stringify(product)}];
          await client.connect();
          console.log('INSTANCE ' + (seen instanceof ${className}));
          // The same client's listeners, not a wrapper of its own.
          console.log('LISTENERS ' + seen.listenerCount('authenticated'));
          // Wrappers share their state: the connection opened through another one.
          console.log('CONNECTED ' + seen.isConnected);
          client.disconnect();
        })();
      `, { URL: `ws://127.0.0.1:${wss.address().port}` });
      expect(result).toMatchObject({
        code: 0,
        lines: ['WRAPPER true', 'INSTANCE true', 'LISTENERS 1', 'CONNECTED true'],
        stderr: '',
      });
    });
  });
});
