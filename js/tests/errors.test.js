/**
 * REST errors carry the unified error fields (#81): code, sourceKind,
 * message, status, body, requestId and headers. So does a config error
 * thrown by the WebSocket constructor.
 */

const http = require('http');
const { RestClient, WebSocketClient } = require('../');

// Errors come from the native module's realm, so `instanceof Error` is false
// under jest; check the brand instead.
const isError = (value) => Object.prototype.toString.call(value) === '[object Error]';

/** Answer every request with `status`, `body` and `headers`. */
function serve(status, body, headers = {}) {
  const server = http.createServer((req, res) => {
    res.writeHead(status, { 'Content-Type': 'application/json', ...headers });
    res.end(body);
  });
  return new Promise((resolve) => {
    server.listen(0, '127.0.0.1', () => {
      const { port } = server.address();
      resolve({
        client: new RestClient({ apiKey: 'test-key', baseUrl: `http://127.0.0.1:${port}` }),
        close: () =>
          new Promise((done) => {
            server.closeAllConnections();
            server.close(done);
          }),
      });
    });
  });
}

describe('REST errors', () => {
  let ctx;

  afterEach(async () => {
    if (ctx) await ctx.close();
    ctx = null;
  });

  test.each([
    [401, 2002, 'auth', 'Authentication error: '],
    [404, 2003, 'client', 'API error (status 404): '],
    [429, 2003, 'rate_limit', 'API error (status 429): '],
    [503, 2003, 'network', 'API error (status 503): '],
  ])('HTTP %i rejects with the response details', async (status, code, sourceKind, prefix) => {
    const body = JSON.stringify({ message: 'nope', statusCode: status });
    ctx = await serve(status, body, { 'X-Request-Id': 'req-1', 'X-RateLimit-Remaining': '0' });

    const err = await ctx.client.stock.intraday.quote('2330').catch((e) => e);

    expect(isError(err)).toBe(true);
    expect(err).toMatchObject({
      code,
      sourceKind,
      message: prefix + body,
      status,
      body,
      requestId: 'req-1',
    });
    expect(err.headers).toMatchObject({
      'content-type': 'application/json',
      'x-request-id': 'req-1',
      'x-ratelimit-remaining': '0',
    });
  });

  test('legacy object params reject the same way', async () => {
    ctx = await serve(404, '{"message":"nope"}');

    const err = await ctx.client.stock.intraday.trades({ symbol: '2330', limit: 5 }).catch((e) => e);

    expect(err).toMatchObject({ code: 2003, status: 404, body: '{"message":"nope"}', requestId: null });
  });

  test('a transport failure has no HTTP details', async () => {
    const client = new RestClient({ apiKey: 'test-key', baseUrl: 'http://127.0.0.1:9' });

    const err = await client.stock.intraday.quote('2330').catch((e) => e);

    expect(isError(err)).toBe(true);
    expect(err).toMatchObject({ code: 2001, sourceKind: 'network', status: null, body: null, requestId: null, headers: {} });
    expect(err.message).not.toMatch(/^\[\d+\]/);
  });

  test('a rejected baseUrl throws from the constructor with a code', () => {
    let err;
    try {
      new RestClient({ apiKey: 'test-key', baseUrl: 'https://api.fugle.tw/marketdata/v1.0' });
    } catch (e) {
      err = e;
    }
    expect(isError(err)).toBe(true);
    expect(err).toMatchObject({ code: 1004, sourceKind: 'client' });
  });
});

describe('WebSocket constructor errors', () => {
  // Core's ConfigError for `reconnect` carries the unified fields, like the
  // credential and `healthCheck` errors from the same constructor (#153).
  test.each([
    [{ initialDelayMs: 50 }, 'initial_delay must be >= 100ms (got 50ms)'],
    [{ initialDelayMs: 5000, maxDelayMs: 2000 }, 'max_delay (2000ms) must be >= initial_delay (5000ms)'],
    [{ enabled: false, initialDelayMs: 50 }, 'initial_delay must be >= 100ms (got 50ms)'],
  ])('an invalid reconnect option %p throws with a code', (reconnect, message) => {
    let err;
    try {
      new WebSocketClient({ apiKey: 'test-key', reconnect });
    } catch (e) {
      err = e;
    }
    expect(isError(err)).toBe(true);
    expect(err).toMatchObject({ code: 1004, sourceKind: 'client' });
    expect(err.message).toContain(message);
    expect(err.message).not.toMatch(/Configuration error: Configuration error/);
  });
});
