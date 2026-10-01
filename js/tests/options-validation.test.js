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

  test('a key whose value is undefined or null counts as not given', () => {
    expect(() => ws({ foo: undefined, reconnect: { maxRetries: undefined } })).not.toThrow();
    // 1.x took `healthCheck: null`, `baseUrl: null` and the like.
    const client = ws({
      bar: null,
      baseUrl: null,
      bearerToken: null,
      healthCheck: null,
      reconnect: { enabled: null, maxAttempts: null },
      messageOverflow: null,
      authTimeoutMs: null,
      version: { futopt: null },
    });
    expect(client.futopt.url).toBe('wss://api.fugle.tw/marketdata/v1.1/futopt/streaming');
    expect(() => rest({ baseUrl: null, tlsRootCertPem: null, version: null })).not.toThrow();
  });
});

describe('WebSocketClient: values of the wrong type', () => {
  const ms = 'must be a number of milliseconds';
  test.each([
    [{ baseUrl: 1 }, 'baseUrl must be a string, got number 1'],
    [{ messageOverflow: 1 }, 'messageOverflow must be a string, got number 1'],
    [{ tlsAcceptInvalidCerts: 'true' }, 'tlsAcceptInvalidCerts must be a boolean, got string'],
    [{ tlsRootCertPem: '-----BEGIN' }, 'tlsRootCertPem must be a Uint8Array or Buffer holding PEM bytes, got string'],
    [{ tlsRootCertPem: new Uint16Array(1) }, 'tlsRootCertPem must be a Uint8Array or Buffer holding PEM bytes, got typed array'],
    [{ messageBuffer: -1 }, 'messageBuffer must be a non-negative integer, got number -1'],
    [{ messageBuffer: 1.5 }, 'messageBuffer must be a non-negative integer, got number 1.5'],
    [{ messageBuffer: '10' }, 'messageBuffer must be a non-negative integer, got string'],
    [{ authTimeoutMs: '5000' }, `authTimeoutMs ${ms}, got string`],
    [{ reconnect: 'x' }, 'reconnect must be an object like { enabled: false }, got string'],
    [{ reconnect: new Map() }, 'reconnect must be an object like { enabled: false }, got object (Map)'],
    [{ healthCheck: new Date(0) }, 'healthCheck must be an object like { enabled: true }, got object (Date)'],
    [{ reconnect: [] }, 'reconnect must be an object like { enabled: false }, got array'],
    [{ reconnect: { enabled: 1 } }, 'reconnect.enabled must be a boolean, got number 1'],
    [{ reconnect: { maxAttempts: NaN } }, 'reconnect.maxAttempts must be a non-negative integer, got number NaN'],
    [{ reconnect: { maxAttempts: -1 } }, 'reconnect.maxAttempts must be a non-negative integer, got number -1'],
    [{ reconnect: { initialDelayMs: '1000' } }, `reconnect.initialDelayMs ${ms}, got string`],
  ])('%j', (extra, message) => {
    expectTypeError(() => ws(extra), `WebSocketClient options: ${message}`);
  });

  test('a Buffer is a valid tlsRootCertPem', () => {
    expect(() => ws({ tlsRootCertPem: Buffer.from('not checked here') })).not.toThrow();
  });

  // A number out of range is core's to refuse (1004), whatever it is (#294).
  test.each([
    [{ authTimeoutMs: -1 }],
    [{ authTimeoutMs: NaN }],
    [{ reconnect: { initialDelayMs: Infinity } }],
    [{ healthCheck: { heartbeatTimeoutMs: Infinity } }],
    [{ healthCheck: { heartbeatTimeoutMs: -1 } }],
  ])('%j is 1004', (extra) => {
    const error = thrown(() => ws(extra));
    expect({ name: error.name, code: error.code }).toEqual({ name: 'Error', code: 1004 });
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
    [{ futopt: true }, "version.futopt must be a version string, e.g. 'v1.1', got boolean"],
    [{ futopt: 'v9' }, 'futopt streaming does not support v9 (supported: v1.0, v1.1). Remove it from the version map to use v1.1.'],
    [{ stock: 'v1.1' }, 'stock streaming does not support v1.1 (supported: v1.0). Remove it from the version map to use v1.0.'],
  ])('%j', (version, message) => {
    expectTypeError(() => ws({ version }), `WebSocketClient options: ${message}`);
  });

  test.each([
    [undefined, 'v1.1'],
    [{}, 'v1.1'],
    [{ futopt: undefined }, 'v1.1'],
    [{ futopt: null }, 'v1.1'],
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
    expectTypeError(() => rest({ baseUrl: 1 }), 'RestClient options: baseUrl must be a string, got number 1');
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

describe('edge cases of the check', () => {
  test('symbol keys are not options and are ignored, as napi ignores them', () => {
    expect(() => ws({ [Symbol('x')]: 1 })).not.toThrow();
  });

  test('an object without a prototype is a plain object', () => {
    const options = Object.assign(Object.create(null), { apiKey: 'k', reconnect: Object.assign(Object.create(null), { enabled: false }) });
    expect(() => new WebSocketClient(options)).not.toThrow();
  });

  test('an object inheriting from another object is not a plain object', () => {
    expectTypeError(
      () => new RestClient(Object.create({ apiKey: 'k' })),
      "RestClient options must be an object like { apiKey: '<key>' }, got an object inheriting from another object (spread it into a plain object: { ...value }).",
    );
  });

  // The one inheriting object taken as plain: its prototype has a null
  // prototype, like `Object.prototype`. Its inherited keys are napi's to
  // read, so they are checked too.
  test('inherited enumerable keys of a plain object are checked', () => {
    const base = Object.assign(Object.create(null), { foo: 1 });
    expectTypeError(
      () => new RestClient(Object.assign(Object.create(base), { apiKey: 'k' })),
      "RestClient options: unknown option 'foo' (known: apiKey, bearerToken, sdkToken, baseUrl, tlsRootCertPem, tlsAcceptInvalidCerts)",
    );
  });

  test('a class instance is not a plain object', () => {
    class Options {
      constructor() {
        this.apiKey = 'k';
      }
    }
    expectTypeError(() => new RestClient(new Options()), "RestClient options must be an object like { apiKey: '<key>' }, got object (Options).");
  });

  test('a getter that throws on a current field throws its own error', () => {
    const error = thrown(() => new WebSocketClient({ apiKey: 'k', get baseUrl() { throw new Error('boom'); } }));
    expect(error.message).toBe('boom');
  });

  test('a Proxy is read through its traps', () => {
    expect(() => new RestClient(new Proxy({ apiKey: 'k' }, {}))).not.toThrow();
    expectTypeError(
      () => new RestClient(new Proxy({ apiKey: 'k', foo: 1 }, {})),
      "RestClient options: unknown option 'foo' (known: apiKey, bearerToken, sdkToken, baseUrl, tlsRootCertPem, tlsAcceptInvalidCerts)",
    );
  });
});
