/**
 * WebSocket listener arguments, `connect()` settlement and `ping()` match
 * `@fugle/marketdata` 1.x (#23). Every event is forwarded from core's
 * connection events (#55); none is synthesized by the binding.
 */

const { WebSocketServer } = require('ws');
const { WebSocketClient } = require('../');

const AUTH_DATA = { message: 'Authenticated successfully' };
const AUTH_ERROR = { message: 'Invalid authentication credentials' };

/**
 * Loopback server. `failAuth` rejects the first that many auth attempts;
 * every frame the clients send is recorded in `wss.frames`.
 */
function startServer({ failAuth = 0 } = {}) {
  return new Promise((resolve) => {
    const wss = new WebSocketServer({ host: '127.0.0.1', port: 0 });
    wss.frames = [];
    let authFailuresLeft = failAuth;
    wss.on('connection', (socket) => {
      socket.on('message', (raw) => {
        const frame = JSON.parse(raw.toString());
        wss.frames.push(frame);
        switch (frame.event) {
          case 'auth':
            if (authFailuresLeft > 0) {
              authFailuresLeft -= 1;
              // The server's rejection: `error` code 1000, then a Close without a code (#201).
              socket.send(JSON.stringify({ event: 'error', code: 1000, data: AUTH_ERROR }));
              socket.close();
            } else {
              socket.send(JSON.stringify({ event: 'authenticated', data: AUTH_DATA }));
            }
            break;
          case 'ping':
            socket.send(JSON.stringify({ event: 'pong', data: { time: 1, ...(frame.data || {}) } }));
            break;
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

/**
 * Native errors come from Node's realm, not the jest sandbox's, so
 * `instanceof Error` cannot tell them apart; the internal tag can.
 */
const isError = (value) => Object.prototype.toString.call(value) === '[object Error]';

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function waitFor(predicate, what, timeoutMs = 5000) {
  const deadline = Date.now() + timeoutMs;
  while (!predicate()) {
    if (Date.now() > deadline) throw new Error(`timed out waiting for ${what}`);
    await sleep(20);
  }
}

/** Record every listener invocation as `[event, argumentsArray]`, in order. */
function recordEvents(ws) {
  const calls = [];
  for (const event of ['connect', 'authenticated', 'unauthenticated', 'disconnect', 'reconnect', 'error']) {
    ws.on(event, function listener() {
      calls.push([event, Array.from(arguments)]);
    });
  }
  return calls;
}

const names = (calls) => calls.map(([event]) => event);

describe.each(['stock', 'futopt'])('%s legacy-compatible WebSocket API (#23)', (product) => {
  let wss;
  let ws;

  async function setup(options = {}, serverOptions) {
    wss = await startServer(serverOptions);
    const { port } = wss.address();
    const client = new WebSocketClient({ apiKey: 'test-key', baseUrl: `ws://127.0.0.1:${port}`, ...options });
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

  test('connect fires without arguments, then authenticated with the server data; connect() resolves with it', async () => {
    await setup();
    const calls = recordEvents(ws);

    const resolved = await ws.connect();

    // Both listeners have run by the time connect() resolves, as in 1.x.
    expect(resolved).toEqual(AUTH_DATA);
    expect(calls).toEqual([
      ['connect', []],
      ['authenticated', [AUTH_DATA]],
    ]);
    expect(ws.isConnected).toBe(true);
  });

  test('rejected credentials fire connect then unauthenticated; connect() rejects with the server data', async () => {
    await setup({}, { failAuth: 1 });
    const calls = recordEvents(ws);

    const rejection = await ws.connect().then(
      () => { throw new Error('connect() resolved'); },
      (reason) => reason,
    );

    // The listeners have run by the time connect() rejects, as in 1.x.
    expect(calls).toEqual([
      ['connect', []],
      ['unauthenticated', [AUTH_ERROR]],
    ]);
    await sleep(200);

    expect(rejection).toEqual(AUTH_ERROR);
    expect(isError(rejection)).toBe(false);
    expect(names(calls)).not.toContain('error');
    expect(names(calls)).not.toContain('authenticated');
  });

  test('error listener receives an Error with a numeric code and no [code] prefix; connect() rejects with the same fields', async () => {
    await setup();
    const { port } = wss.address();
    await closeServer(wss);
    wss = await startServer();
    const unreachable = new WebSocketClient({ apiKey: 'test-key', baseUrl: `ws://127.0.0.1:${port}` })[product];
    const errors = [];
    unreachable.on('error', (err) => errors.push(err));

    const rejection = await unreachable.connect().catch((e) => e);
    await waitFor(() => errors.length > 0, 'error event');

    expect(isError(errors[0])).toBe(true);
    expect(typeof errors[0].code).toBe('number');
    expect(errors[0].message).not.toMatch(/^\[\d+\]/);
    expect(isError(rejection)).toBe(true);
    expect(rejection).toMatchObject({
      code: errors[0].code,
      sourceKind: errors[0].sourceKind,
      message: errors[0].message,
    });
    expect(['network', 'protocol', 'auth', 'rate_limit', 'client']).toContain(errors[0].sourceKind);
    expect(errors[0]).toMatchObject({ status: null, body: null, requestId: null, headers: {} });
  });

  test('a connection error without an error listener does not crash the process', async () => {
    await setup();
    const { port } = wss.address();
    await closeServer(wss);
    wss = await startServer();
    const unreachable = new WebSocketClient({ apiKey: 'test-key', baseUrl: `ws://127.0.0.1:${port}` })[product];

    const rejection = await unreachable.connect().catch((e) => e);
    expect(isError(rejection)).toBe(true);
    await sleep(200);
  });

  test('disconnect listener receives { code, reason }', async () => {
    await setup();
    const calls = recordEvents(ws);
    await ws.connect();

    ws.disconnect();
    await waitFor(() => names(calls).includes('disconnect'), 'disconnect');

    expect(calls.find(([event]) => event === 'disconnect')).toEqual([
      'disconnect',
      [{ code: 1000, reason: 'Normal closure' }],
    ]);
  });

  test('disconnect listener receives code null when the connection ends without a close code', async () => {
    await setup();
    const calls = recordEvents(ws);
    await ws.connect();

    // Drop the TCP connection without a close frame.
    for (const socket of wss.clients) socket.terminate();
    await waitFor(() => names(calls).includes('disconnect'), 'disconnect');

    const [, [event]] = calls.find(([name]) => name === 'disconnect');
    expect(event.code).toBeNull();
    expect(typeof event.reason).toBe('string');
    expect(Object.keys(event).sort()).toEqual(['code', 'reason']);
  });

  test('reconnect listener receives { attempt }; authenticated fires again without re-settling connect()', async () => {
    await setup({ reconnect: { enabled: true, maxAttempts: 3, initialDelayMs: 100, maxDelayMs: 100 } });
    const calls = recordEvents(ws);
    await ws.connect();

    for (const socket of wss.clients) socket.close(1001, 'going away');
    await waitFor(() => names(calls).filter((e) => e === 'authenticated').length === 2, 'second authenticated', 5000);

    expect(calls.find(([event]) => event === 'disconnect')).toEqual([
      'disconnect',
      [{ code: 1001, reason: 'going away' }],
    ]);
    expect(calls.find(([event]) => event === 'reconnect')).toEqual(['reconnect', [{ attempt: 1 }]]);
    expect(ws.isConnected).toBe(true);
  });

  test('ping(object) sends it as data; ping(string) sends { state }; ping() sends no data', async () => {
    await setup();
    const pongs = [];
    ws.on('message', (raw) => {
      const msg = JSON.parse(raw);
      if (msg.event === 'pong') pongs.push(msg);
    });
    await ws.connect();

    ws.ping({ state: 'obj', extra: 1 });
    ws.ping('str');
    ws.ping();
    await waitFor(() => pongs.length === 3, 'three pongs');

    const pings = wss.frames.filter((frame) => frame.event === 'ping');
    expect(pings).toEqual([
      { event: 'ping', data: { state: 'obj', extra: 1 } },
      { event: 'ping', data: { state: 'str' } },
      { event: 'ping' },
    ]);
    expect(pongs[0].data.state).toBe('obj');
  });

  test('measureLatency() resolves with the round trip; only ping() pongs reach message (#150)', async () => {
    await setup({ healthCheck: { probeEnabled: true } });
    const pongs = [];
    ws.on('message', (raw) => {
      const msg = JSON.parse(raw);
      if (msg.event === 'pong') pongs.push(msg);
    });
    await ws.connect();

    const latency = await ws.measureLatency();
    expect(typeof latency).toBe('number');
    expect(latency).toBeGreaterThanOrEqual(0);
    expect(latency).toBeLessThan(5000);

    ws.ping('mine');
    await waitFor(() => pongs.length === 1, 'the ping() pong');
    await sleep(100);
    expect(pongs.map((pong) => pong.data.state)).toEqual(['mine']);
  });

  test('measureLatency() rejects with ClientClosed before connect() (#150)', async () => {
    await setup();
    const error = await ws.measureLatency().catch((e) => e);
    expect(isError(error)).toBe(true);
    expect(error.code).toBe(2010);
  });
});

describe('healthCheck probe options (#150)', () => {
  test('accepts the probe options', () => {
    const ws = new WebSocketClient({
      apiKey: 'test-key',
      healthCheck: { probeEnabled: true, idleProbeAfterMs: 5000, probeTimeoutMs: 1000 },
    });
    expect(ws.stock).toBeDefined();
  });

  test.each([
    [{ heartbeatTimeoutMs: 4999 }],
    [{ probeEnabled: true, idleProbeAfterMs: 4999 }],
    [{ probeEnabled: true, probeTimeoutMs: 999 }],
    // Checked even with probing off, before it is switched on.
    [{ probeTimeoutMs: 10 }],
  ])('rejects %j below its floor with 1004', (healthCheck) => {
    let error;
    try {
      new WebSocketClient({ apiKey: 'test-key', healthCheck });
    } catch (e) {
      error = e;
    }
    expect(error && error.code).toBe(1004);
  });
});
