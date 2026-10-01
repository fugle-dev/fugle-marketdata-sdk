/**
 * The WebSocket clients' listeners behave as the 1.x EventEmitter's did
 * (#307): `on()` adds a listener instead of replacing the previous one, and
 * `once` / `off` / `removeListener` / `removeAllListeners` / `listenerCount`
 * work as on an EventEmitter.
 */

const { WebSocketServer } = require('ws');
const { WebSocketClient } = require('../');

const AUTH_DATA = { message: 'Authenticated successfully' };

function startServer() {
  return new Promise((resolve) => {
    const wss = new WebSocketServer({ host: '127.0.0.1', port: 0 });
    wss.on('connection', (socket) => {
      socket.on('message', (raw) => {
        if (JSON.parse(raw.toString()).event === 'auth') {
          socket.send(JSON.stringify({ event: 'authenticated', data: AUTH_DATA }));
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

/** Resolves once `event` fires on `ws`. */
const next = (ws, event) => new Promise((resolve) => ws.once(event, (...args) => resolve(args)));

describe.each(['stock', 'futopt'])('%s listeners', (product) => {
  let wss;
  let client;
  let ws;

  beforeEach(async () => {
    wss = await startServer();
    client = new WebSocketClient({
      apiKey: 'test-key',
      baseUrl: `ws://127.0.0.1:${wss.address().port}`,
      reconnect: { enabled: false },
    });
    ws = client[product];
  });

  afterEach(async () => {
    ws.removeAllListeners();
    ws.disconnect();
    await closeServer(wss);
  });

  /** One connection: connect, then disconnect and wait for its event. */
  async function connectOnce() {
    await ws.connect();
    const closed = next(ws, 'disconnect');
    ws.disconnect();
    await closed;
  }

  test('every listener of an event is called, in registration order, with this set to the client', async () => {
    const calls = [];
    const thisValues = [];
    ws.on('authenticated', function first(data) {
      calls.push(['first', data]);
      thisValues.push(this);
    });
    ws.addListener('authenticated', function second(data) {
      calls.push(['second', data]);
      thisValues.push(this);
    });
    const messages = [];
    ws.on('message', (raw) => messages.push(['a', JSON.parse(raw).event]));
    ws.on('message', (raw) => messages.push(['b', JSON.parse(raw).event]));

    await ws.connect();

    expect(calls).toEqual([
      ['first', AUTH_DATA],
      ['second', AUTH_DATA],
    ]);
    // `this` is the client on() was called on, so its methods work on it.
    expect(thisValues[0]).toBe(ws);
    expect(thisValues[1]).toBe(ws);
    expect(typeof thisValues[0].subscribe).toBe('function');
    expect(messages).toEqual([
      ['a', 'authenticated'],
      ['b', 'authenticated'],
    ]);
  });

  test('a listener registered on another ws.stock / ws.futopt access is called too', async () => {
    const calls = [];
    client[product].on('authenticated', () => calls.push('first access'));
    client[product].on('authenticated', () => calls.push('second access'));
    await client[product].connect();
    expect(calls).toEqual(['first access', 'second access']);
  });

  test('once() listeners are called for the first emission only', async () => {
    const calls = [];
    ws.once('authenticated', () => calls.push('once'));
    ws.on('authenticated', () => calls.push('on'));
    expect(ws.listenerCount('authenticated')).toBe(2);

    await connectOnce();
    expect(ws.listenerCount('authenticated')).toBe(1);
    await connectOnce();

    expect(calls).toEqual(['once', 'on', 'on']);
  });

  test('off() and removeListener() remove a listener; removing one never added does nothing', async () => {
    const calls = [];
    const kept = () => calls.push('kept');
    const removed = () => calls.push('removed');
    const alsoRemoved = () => calls.push('also removed');
    ws.on('authenticated', removed).on('authenticated', kept).on('authenticated', alsoRemoved);

    expect(ws.off('authenticated', removed)).toBe(ws);
    expect(ws.removeListener('authenticated', alsoRemoved)).toBe(ws);
    ws.off('authenticated', () => {});
    ws.off('connect', kept);
    expect(ws.listenerCount('authenticated')).toBe(1);

    await ws.connect();
    expect(calls).toEqual(['kept']);
  });

  test('off() removes the most recent registration of a listener added twice', async () => {
    const calls = [];
    const listener = () => calls.push('listener');
    ws.on('authenticated', listener).on('authenticated', listener);
    ws.off('authenticated', listener);
    expect(ws.listenerCount('authenticated')).toBe(1);

    await ws.connect();
    expect(calls).toEqual(['listener']);
  });

  test('off() removes a once() listener by the function passed to once()', async () => {
    const calls = [];
    const listener = () => calls.push('once');
    ws.once('authenticated', listener);
    ws.off('authenticated', listener);
    expect(ws.listenerCount('authenticated')).toBe(0);

    await ws.connect();
    expect(calls).toEqual([]);
  });

  test('removeAllListeners() clears one event, or every event without an argument', async () => {
    const calls = [];
    ws.on('connect', () => calls.push('connect'));
    ws.on('authenticated', () => calls.push('authenticated'));
    ws.on('message', () => calls.push('message'));

    expect(ws.removeAllListeners('connect')).toBe(ws);
    expect(ws.listenerCount('connect')).toBe(0);
    expect(ws.listenerCount('authenticated')).toBe(1);
    await connectOnce();
    expect(calls).toEqual(['authenticated', 'message']);

    calls.length = 0;
    expect(ws.removeAllListeners()).toBe(ws);
    expect(ws.listenerCount('authenticated')).toBe(0);
    expect(ws.listenerCount('message')).toBe(0);
    await connectOnce();
    expect(calls).toEqual([]);
  });

  test('listenerCount() is 0 for an event without listeners, including an unknown one', () => {
    expect(ws.listenerCount('message')).toBe(0);
    expect(ws.listenerCount('nonexistent')).toBe(0);
  });

  test('on() and once() still refuse an unknown event', () => {
    expect(() => ws.on('nonexistent', () => {})).toThrow(/Unknown event type: nonexistent/);
    expect(() => ws.once('nonexistent', () => {})).toThrow(/Unknown event type: nonexistent/);
  });

  test('a listener that throws does not stop the listeners after it', async () => {
    const calls = [];
    const errors = [];
    ws.on('error', (err) => errors.push(err.code));
    ws.on('authenticated', () => {
      calls.push('throws');
      throw new Error('boom');
    });
    ws.on('authenticated', () => calls.push('after'));

    await ws.connect();
    expect(calls).toEqual(['throws', 'after']);
    expect(errors).toEqual([3004]);
  });

  test('a listener added or removed while an event is delivered only affects later emissions', async () => {
    const calls = [];
    const second = () => calls.push('second');
    ws.on('authenticated', () => {
      calls.push('first');
      ws.off('authenticated', second);
      ws.on('authenticated', () => calls.push('added'));
    });
    ws.on('authenticated', second);

    await connectOnce();
    expect(calls).toEqual(['first', 'second']);

    calls.length = 0;
    await connectOnce();
    expect(calls).toEqual(['first', 'added']);
  });
});
