/**
 * `url` and chainable `on()` match `@fugle/marketdata` 1.x (#245).
 */

const { WebSocketClient } = require('../');

describe('url (#245)', () => {
  test('defaults to the production endpoint of each product, before connect()', () => {
    const ws = new WebSocketClient({ apiKey: 'test-key' });
    expect(ws.stock.url).toBe('wss://api.fugle.tw/marketdata/v1.0/stock/streaming');
    expect(ws.futopt.url).toBe('wss://api.fugle.tw/marketdata/v1.1/futopt/streaming');
  });

  test('reflects version', () => {
    const ws = new WebSocketClient({ apiKey: 'test-key', version: { futopt: 'v1.0' } });
    expect(ws.futopt.url.endsWith('/v1.0/futopt/streaming')).toBe(true);
    expect(ws.stock.url.endsWith('/v1.0/stock/streaming')).toBe(true);
  });

  test('reflects baseUrl', () => {
    const ws = new WebSocketClient({ apiKey: 'test-key', baseUrl: 'wss://custom.ws/marketdata' });
    expect(ws.stock.url).toBe('wss://custom.ws/marketdata/v1.0/stock/streaming');
    expect(ws.futopt.url).toBe('wss://custom.ws/marketdata/v1.1/futopt/streaming');
  });
});

describe.each(['stock', 'futopt'])('%s on() chaining (#245)', (product) => {
  test('on() returns the client it was called on', () => {
    const client = new WebSocketClient({ apiKey: 'test-key' })[product];
    const returned = client.on('message', () => {});
    expect(returned).toBe(client);
    expect(returned.on('error', () => {}).on('connect', () => {})).toBe(client);
  });

  test('a method chained on on() reaches the client', () => {
    const ws = new WebSocketClient({ apiKey: 'test-key' });
    expect(ws[product].on('message', () => {}).isConnected).toBe(false);
    expect(typeof ws[product].on('message', () => {}).subscribe).toBe('function');
  });

  test('subscribe() chained on on() before connect() does not throw TypeError', () => {
    const ws = new WebSocketClient({ apiKey: 'test-key' });
    const symbol = product === 'stock' ? '2330' : 'TXFC4';
    let caught;
    try {
      ws[product].on('message', () => {}).subscribe({ channel: 'trades', symbol });
    } catch (err) {
      caught = err;
    }
    // Not connected, so subscribe() may refuse; what 3.0.0-rc.9 threw was
    // `TypeError: Cannot read properties of undefined (reading 'subscribe')`.
    expect(caught instanceof TypeError).toBe(false);
    expect(caught === undefined ? undefined : caught.name).not.toBe('TypeError');
  });

  test('an unknown event still throws', () => {
    const client = new WebSocketClient({ apiKey: 'test-key' })[product];
    expect(() => client.on('nope', () => {})).toThrow();
  });
});
