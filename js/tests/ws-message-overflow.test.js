/**
 * `messageOverflow` / `messageBuffer`, `messagesDropped` and
 * `messagesDroppedTotal` (#46).
 *
 * Frames queued for the JS thread count against `messageBuffer` until their
 * listener has run: a slow listener leaves the backlog to the SDK's queue,
 * which drops per `messageOverflow`, rather than growing the dispatch queue
 * without bound.
 */

const { Worker } = require('worker_threads');
const { WebSocketClient } = require('../');

const BURST = 500;

/**
 * Loopback server on its own thread, so a busy listener on this thread does
 * not slow it down. Auth is acked; `subscribe` is answered with `BURST`
 * `data` frames numbered from 0.
 */
function startBurstServer() {
  const worker = new Worker(
    `
    const { parentPort } = require('worker_threads');
    const { WebSocketServer } = require(${JSON.stringify(require.resolve('ws'))});
    const wss = new WebSocketServer({ host: '127.0.0.1', port: 0 });
    wss.on('connection', (socket) => {
      socket.on('message', (raw) => {
        const frame = JSON.parse(raw.toString());
        if (frame.event === 'auth') {
          socket.send(JSON.stringify({ event: 'authenticated', data: {} }));
        } else if (frame.event === 'subscribe') {
          for (let i = 0; i < ${BURST}; i += 1) {
            socket.send(JSON.stringify({ event: 'data', data: { i }, channel: 'trades' }));
          }
        }
      });
    });
    wss.on('listening', () => parentPort.postMessage(wss.address().port));
    `,
    { eval: true },
  );
  return new Promise((resolve) => {
    worker.once('message', (port) => resolve({ worker, url: `ws://127.0.0.1:${port}` }));
  });
}

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function waitFor(predicate, what, timeoutMs = 10000) {
  const deadline = Date.now() + timeoutMs;
  while (!predicate()) {
    if (Date.now() > deadline) throw new Error(`timed out waiting for ${what}`);
    await sleep(10);
  }
}

/** A listener that holds the JS thread for `ms` per frame. */
function slowListener(ms, onFrame) {
  return (raw) => {
    const until = Date.now() + ms;
    while (Date.now() < until);
    onFrame(JSON.parse(raw));
  };
}

describe('messageOverflow options', () => {
  test.each([
    [{ messageOverflow: 'drop_newest' }],
    [{ messageOverflow: '' }],
    [{ messageBuffer: 0 }],
  ])('rejects %p', (options) => {
    expect(() => new WebSocketClient({ apiKey: 'k', ...options })).toThrow();
  });

  test('accepts both policies', () => {
    expect(() => new WebSocketClient({ apiKey: 'k', messageOverflow: 'dropNewest', messageBuffer: 1 })).not.toThrow();
    expect(() => new WebSocketClient({ apiKey: 'k', messageOverflow: 'unbounded' })).not.toThrow();
  });
});

describe.each(['stock', 'futopt'])('%s message backpressure (#46)', (product) => {
  let server;
  let ws;

  afterEach(async () => {
    try {
      ws.disconnect();
    } catch (e) {
      // already gone
    }
    await server.worker.terminate();
  });

  async function run(options, listenerMs) {
    server = await startBurstServer();
    ws = new WebSocketClient({ apiKey: 'k', baseUrl: server.url, ...options })[product];
    const data = [];
    const reports = [];
    let disconnected = false;
    ws.on('message', slowListener(listenerMs, (msg) => {
      if (msg.event === 'data') data.push(msg.data.i);
    }));
    ws.on('messagesDropped', (event) => reports.push(event));
    ws.on('disconnect', () => {
      disconnected = true;
    });

    expect(ws.messagesDroppedTotal).toBe(0);
    await ws.connect();
    ws.subscribe({ channel: 'trades', symbol: '2330' });
    // The burst is over once every frame is either delivered or dropped.
    await waitFor(() => data.length + ws.messagesDroppedTotal === BURST, 'the burst to settle', 20000);
    ws.disconnect();
    await waitFor(() => disconnected, 'disconnect');
    return { data, reports };
  }

  test('a slow listener makes dropNewest drop, and reports every drop', async () => {
    const { data, reports } = await run({ messageBuffer: 16 }, 20);

    const total = ws.messagesDroppedTotal;
    // At most `messageBuffer` frames wait in the SDK and as many more are in
    // flight to the listener, plus the few it finished while the burst came
    // in. Without in-flight accounting nearly all 500 would be queued for the
    // JS thread instead.
    expect(data.length).toBeLessThan(BURST / 4);
    expect(total).toBeGreaterThan(0);
    expect(data.length + total).toBe(BURST);
    expect(reports.reduce((sum, r) => sum + r.dropped, 0)).toBe(total);
    expect(reports[reports.length - 1].total).toBe(total);
  }, 30000);

  test('unbounded delivers every frame to a slow listener', async () => {
    const { data, reports } = await run({ messageOverflow: 'unbounded', messageBuffer: 16 }, 2);

    expect(data).toEqual(Array.from({ length: BURST }, (_, i) => i));
    expect(reports).toEqual([]);
    expect(ws.messagesDroppedTotal).toBe(0);
  }, 30000);
});
