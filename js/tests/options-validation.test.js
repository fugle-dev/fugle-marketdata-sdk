/**
 * Constructor options are checked against their declared shape (#294).
 *
 * napi reads only the fields a `#[napi(object)]` declares, so `version: 'v1.0'`,
 * `reconnect: 'x'`, an unknown key or `messageBuffer: -1` used to pass and be
 * ignored or coerced. Now `undefined` counts as not given and anything else
 * that does not fit is a `TypeError` naming the field and what it takes.
 *
 * `name`, not `instanceof`, throughout: jest's sandbox has its own `TypeError`.
 */

const { WebSocketClient, RestClient } = require('../');

function thrown(fn) {
  try {
    fn();
  } catch (e) {
    return e;
  }
  throw new Error('expected a throw');
}

const ws = (extra) => new WebSocketClient({ apiKey: 'test-key', ...extra });
const rest = (extra) => new RestClient({ apiKey: 'test-key', ...extra });

function expectTypeError(fn, message) {
  const error = thrown(fn);
  expect({ name: error.name, code: error.code, message: error.message }).toEqual({
    name: 'TypeError',
    code: undefined,
    message,
  });
}

describe('the options argument itself', () => {
  test.each([
    ['undefined', undefined, 'got undefined.'],
    ['null', null, 'got null.'],
    ['a number', 1, 'got number 1.'],
    ['an array', [], 'got array.'],
    ['a string', 'key', "got string. To pass an API key, write { apiKey: '<key>' }."],
  ])('%s is a TypeError', (_label, options, tail) => {
    expectTypeError(() => new WebSocketClient(options), `WebSocketClient options must be an object like { apiKey: '<key>' }, ${tail}`);
    expectTypeError(() => new RestClient(options), `RestClient options must be an object like { apiKey: '<key>' }, ${tail}`);
  });
});

describe('WebSocketClient: unknown keys', () => {
  const known =
    'apiKey, bearerToken, sdkToken, baseUrl, version, reconnect, healthCheck, tlsRootCertPem, ' +
    'tlsAcceptInvalidCerts, messageOverflow, messageBuffer, authTimeoutMs';

  test.each([
    [{ foo: 1 }, `unknown option 'foo' (known: ${known})`],
    [{ url: 'wss://x' }, `unknown option 'url' (known: ${known})`],
    [{ reconnect: { maxRetries: 3 } }, "unknown option 'reconnect.maxRetries' (known: enabled, maxAttempts, initialDelayMs, maxDelayMs)"],
    [
      { healthCheck: { foo: 1 } },
      "unknown option 'healthCheck.foo' (known: enabled, heartbeatTimeoutMs, probeEnabled, idleProbeAfterMs, probeTimeoutMs)",
    ],
  ])('%j', (extra, message) => {
    expectTypeError(() => ws(extra), `WebSocketClient options: ${message}`);
  });

  test('a typo of the credential key names the key, not the credential rule', () => {
    expectTypeError(() => new WebSocketClient({ apikey: 'k' }), `WebSocketClient options: unknown option 'apikey' (known: ${known})`);
  });

  test('a key whose value is undefined counts as not given', () => {
    expect(() => ws({ foo: undefined, reconnect: { maxRetries: undefined } })).not.toThrow();
  });
});

describe('WebSocketClient: values of the wrong type', () => {
  const ms = 'must be a finite, non-negative number of milliseconds';
  test.each([
    [{ baseUrl: 1 }, 'baseUrl must be a string, got number 1'],
    [{ messageOverflow: null }, 'messageOverflow must be a string, got null'],
    [{ tlsAcceptInvalidCerts: 'true' }, 'tlsAcceptInvalidCerts must be a boolean, got string'],
    [{ tlsRootCertPem: '-----BEGIN' }, 'tlsRootCertPem must be a Uint8Array or Buffer holding PEM bytes, got string'],
    [{ tlsRootCertPem: new Uint16Array(1) }, 'tlsRootCertPem must be a Uint8Array or Buffer holding PEM bytes, got typed array'],
    [{ messageBuffer: -1 }, 'messageBuffer must be a non-negative integer, got number -1'],
    [{ messageBuffer: 1.5 }, 'messageBuffer must be a non-negative integer, got number 1.5'],
    [{ messageBuffer: '10' }, 'messageBuffer must be a non-negative integer, got string'],
    [{ authTimeoutMs: NaN }, `authTimeoutMs ${ms}, got number NaN`],
    [{ authTimeoutMs: -1 }, `authTimeoutMs ${ms}, got number -1`],
    [{ reconnect: 'x' }, 'reconnect must be an object like { enabled: false }, got string'],
    [{ reconnect: null }, 'reconnect must be an object like { enabled: false }, got null'],
    [{ reconnect: [] }, 'reconnect must be an object like { enabled: false }, got array'],
    [{ reconnect: { enabled: 1 } }, 'reconnect.enabled must be a boolean, got number 1'],
    [{ reconnect: { maxAttempts: NaN } }, 'reconnect.maxAttempts must be a non-negative integer, got number NaN'],
    [{ reconnect: { maxAttempts: -1 } }, 'reconnect.maxAttempts must be a non-negative integer, got number -1'],
    [{ reconnect: { initialDelayMs: Infinity } }, `reconnect.initialDelayMs ${ms}, got number Infinity`],
    [{ healthCheck: { heartbeatTimeoutMs: Infinity } }, `healthCheck.heartbeatTimeoutMs ${ms}, got number Infinity`],
  ])('%j', (extra, message) => {
    expectTypeError(() => ws(extra), `WebSocketClient options: ${message}`);
  });

  test('a Buffer is a valid tlsRootCertPem', () => {
    expect(() => ws({ tlsRootCertPem: Buffer.from('not checked here') })).not.toThrow();
  });

  test('values of the right type keep their own range errors', () => {
    expect(thrown(() => ws({ messageBuffer: 0 })).message).toBe('messageBuffer must be a positive integer');
    expect(thrown(() => ws({ messageOverflow: 'x' })).message).toMatch(/^messageOverflow must be 'dropNewest' or 'unbounded'/);
    expect(thrown(() => ws({ reconnect: { initialDelayMs: 1 } })).code).toBe(1004);
  });
});

describe('WebSocketClient: version', () => {
  test.each([
    ['v1.0', "version must be a per-product map, not the bare string 'v1.0'. Use version: { stock: 'v1.0', futopt: 'v1.0' }."],
    ['v1.1', "version must be a per-product map, not the bare string 'v1.1'. Use version: { futopt: 'v1.1' }."],
    ['v9', "version must be a per-product map, not the bare string 'v9'. No product serves v9."],
    [1, "version must be a per-product map like { futopt: 'v1.0' }, got number 1"],
    [true, "version must be a per-product map like { futopt: 'v1.0' }, got boolean"],
    [['v1.0'], "version must be a per-product map like { futopt: 'v1.0' }, got array"],
    [null, "version must be a per-product map like { futopt: 'v1.0' }, got null"],
    [{ foo: 'v1.0' }, "unknown product 'foo' in version map (known: stock, futopt)"],
    [{ futopt: 1 }, "version.futopt must be a version string, e.g. 'v1.1', got number 1"],
    [{ futopt: null }, "version.futopt must be a version string, e.g. 'v1.1', got null"],
    [{ futopt: 'v9' }, 'futopt streaming does not support v9 (supported: v1.0, v1.1). Remove it from the version map to use v1.1.'],
    [{ stock: 'v1.1' }, 'stock streaming does not support v1.1 (supported: v1.0). Remove it from the version map to use v1.0.'],
  ])('%j', (version, message) => {
    expectTypeError(() => ws({ version }), `WebSocketClient options: ${message}`);
  });

  test.each([
    [undefined, 'v1.1'],
    [{}, 'v1.1'],
    [{ futopt: undefined }, 'v1.1'],
    [{ futopt: 'v1.0' }, 'v1.0'],
    [{ futopt: 'v1.1', stock: 'v1.0' }, 'v1.1'],
  ])('%j selects futopt %s', (version, expected) => {
    const client = ws({ version });
    expect(client.futopt.url).toBe(`wss://api.fugle.tw/marketdata/${expected}/futopt/streaming`);
    expect(client.stock.url).toBe('wss://api.fugle.tw/marketdata/v1.0/stock/streaming');
  });
});

describe('RestClient', () => {
  test('an unknown key is a TypeError', () => {
    expectTypeError(
      () => rest({ foo: 1 }),
      "RestClient options: unknown option 'foo' (known: apiKey, bearerToken, sdkToken, baseUrl, tlsRootCertPem, tlsAcceptInvalidCerts)",
    );
  });

  test('values of its own fields are checked', () => {
    expectTypeError(() => rest({ baseUrl: null }), 'RestClient options: baseUrl must be a string, got null');
    expectTypeError(() => rest({ tlsAcceptInvalidCerts: 1 }), 'RestClient options: tlsAcceptInvalidCerts must be a boolean, got number 1');
  });

  // 1.x took one options object for both clients and documented that REST
  // ignores `version`; the 3.0 WebSocket-only keys follow the same rule.
  test('WebSocket-only keys are accepted and not read', () => {
    const shared = {
      apiKey: 'test-key',
      version: 'not checked here',
      healthCheck: { enabled: true, pingInterval: 30000 },
      reconnect: 'not checked here',
      messageOverflow: 'unbounded',
      messageBuffer: -1,
      authTimeoutMs: 5000,
    };
    expect(() => new RestClient(shared)).not.toThrow();
  });
});
