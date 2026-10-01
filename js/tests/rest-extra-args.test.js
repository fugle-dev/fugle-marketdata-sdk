/**
 * Arguments a REST method does not use are refused (#294).
 *
 * napi drops arguments past the declared ones, and the object form ignored
 * the positional ones after it, so `trades('2330', { limit: 5 })` sent
 * `trades('2330')` and `candles({ symbol }, '5')` sent no timeframe. Both now
 * reject with a `TypeError` before any request is sent. `undefined` / `null`
 * still count as not given.
 */
const http = require('http');
const { RestClient } = require('../');

let server;
let client;
const urls = [];

beforeAll(async () => {
  server = http.createServer((req, res) => {
    urls.push(req.url);
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end('{}');
  });
  await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
  client = new RestClient({ apiKey: 'test-key', baseUrl: `http://127.0.0.1:${server.address().port}` });
});

afterAll(
  () =>
    new Promise((done) => {
      // The Rust client pools keep-alive connections; close them or jest hangs.
      server.closeAllConnections();
      server.close(done);
    }),
);

async function rejected(call) {
  const sent = urls.length;
  const error = await call().then(
    () => {
      throw new Error('expected a rejection');
    },
    (e) => e,
  );
  expect(urls.length).toBe(sent);
  // `name`, not `instanceof`: jest's sandbox has its own `TypeError`.
  return { name: error.name, code: error.code, message: error.message };
}

describe('one argument past the positional ones', () => {
  test.each([
    ['stock.intraday.trades', (c) => c.stock.intraday.trades('2330', { limit: 5 }), 1, 'symbol', 'object'],
    ['stock.intraday.ticker', (c) => c.stock.intraday.ticker('2330', 'x'), 1, 'symbol', 'string'],
    ['stock.intraday.candles', (c) => c.stock.intraday.candles('2330', '5', 'x'), 2, 'symbol, timeframe', 'string'],
    ['stock.historical.stats', (c) => c.stock.historical.stats('2330', 1), 1, 'symbol', 'number 1'],
    ['stock.snapshot.actives', (c) => c.stock.snapshot.actives('TSE', 'volume', true), 2, 'market, trade', 'boolean'],
    ['futopt.intraday.quote', (c) => c.futopt.intraday.quote('TXFA6', { session: 'afterhours' }), 1, 'symbol', 'object'],
    ['futopt.intraday.products', (c) => c.futopt.intraday.products('FUTURE', 'I', 'x'), 2, 'type, contractType', 'string'],
  ])('%s', async (method, call, count, names, kind) => {
    const short = method.split('.').pop();
    const first = names.split(', ')[0];
    const error = await rejected(() => call(client));
    expect(error.name).toBe('TypeError');
    expect(error.code).toBeUndefined();
    expect(error.message).toMatch(
      new RegExp(
        `^\`${method.replace(/\./g, '\\.')}\` takes at most ${count} positional arguments? \\(${names}\\), got a further ` +
          `${kind} one\\. Pass other parameters in the params object instead, e\\.g\\. ${short}\\(\\{ ${first}: \\.\\.\\., \\.\\.\\. \\}\\); accepted keys: ${first}\\b`,
      ),
    );
  });
});

describe('further arguments after the params object', () => {
  test.each([
    ['stock.intraday.ticker', (c) => c.stock.intraday.ticker({ symbol: '2330' }, 'x'), 'a further string argument'],
    ['stock.intraday.candles', (c) => c.stock.intraday.candles({ symbol: '2330' }, '5'), '`timeframe` as a further argument'],
    ['stock.historical.candles', (c) => c.stock.historical.candles({ symbol: '2330' }, undefined, '2026-01-31'), '`to` as a further argument'],
    ['stock.technical.sma', (c) => c.stock.technical.sma({ symbol: '2330' }, undefined, undefined, undefined, 5), '`period` as a further argument'],
    ['stock.corporateActions.dividends', (c) => c.stock.corporateActions.dividends({ start_date: '2026-01-01' }, '2026-02-01'), '`endDate` as a further argument'],
    ['futopt.historical.daily', (c) => c.futopt.historical.daily({ symbol: 'TXF' }, '2026-01-02'), '`date` as a further argument'],
  ])('%s', async (method, call, further) => {
    const error = await rejected(() => call(client));
    expect(error.name).toBe('TypeError');
    expect(error.message).toMatch(
      new RegExp(`^\`${method.replace(/\./g, '\\.')}\` got a params object and ${further.replace(/[`.]/g, '\\$&')}; the object form takes no other arguments\\. Put every parameter in the object; accepted keys: `),
    );
  });
});

test('corporateActions: a fourth argument', async () => {
  const error = await rejected(() => client.stock.corporateActions.dividends('2026-01-01', '2026-02-01', undefined, 'x'));
  expect(error.name).toBe('TypeError');
  expect(error.message).toMatch(/^`stock\.corporateActions\.dividends` takes at most 2 positional arguments \(startDate, endDate\), got a further string one/);
});

// A positional argument of the wrong type, or an integer napi would coerce,
// rejects the promise like the arguments above, instead of throwing napi's
// conversion error synchronously or sending a changed value (#294).
describe('positional arguments of the wrong type', () => {
  test.each([
    ['stock.intraday.candles', (c) => c.stock.intraday.candles('2330', 5), 'timeframe must be a string, got number 5'],
    ['stock.intraday.quote', (c) => c.stock.intraday.quote('2330', 'true'), 'oddLot must be a boolean, got string'],
    ['stock.intraday.tickers', (c) => c.stock.intraday.tickers('EQUITY', undefined, undefined, undefined, 'yes'), 'isNormal must be a boolean, got string'],
    ['stock.technical.sma', (c) => c.stock.technical.sma('2330', undefined, undefined, undefined, -1), 'period must be a non-negative integer, got number -1'],
    ['stock.technical.rsi', (c) => c.stock.technical.rsi('2330', undefined, undefined, undefined, 1.5), 'period must be a non-negative integer, got number 1.5'],
    ['stock.technical.kdj', (c) => c.stock.technical.kdj('2330', undefined, undefined, undefined, NaN), 'rPeriod must be a non-negative integer, got number NaN'],
    ['stock.technical.macd', (c) => c.stock.technical.macd('2330', undefined, undefined, undefined, undefined, Infinity), 'slow must be a non-negative integer, got number Infinity'],
    ['stock.technical.bb', (c) => c.stock.technical.bb('2330', undefined, undefined, undefined, '20'), 'period must be a non-negative integer, got string'],
    ['stock.corporateActions.dividends', (c) => c.stock.corporateActions.dividends('2026-01-01', 20260201), 'endDate must be a string, got number 20260201'],
    ['futopt.historical.daily', (c) => c.futopt.historical.daily('TXF', undefined, 1), 'afterHours must be a boolean, got number 1'],
    ['stock.intraday.trades', (c) => c.stock.intraday.trades(2330), 'symbol must be a string or a params object, got number 2330'],
    ['stock.intraday.ticker', (c) => c.stock.intraday.ticker(['2330']), 'symbol must be a string or a params object, got array'],
    ['futopt.intraday.products', (c) => c.futopt.intraday.products(new Date(0)), 'type must be a string or a params object, got object (Date)'],
  ])('%s', async (method, call, message) => {
    let result;
    // Not a synchronous throw: the call returns a promise that rejects.
    expect(() => {
      result = call(client);
    }).not.toThrow();
    const sent = urls.length;
    const error = await result.then(
      () => {
        throw new Error('expected a rejection');
      },
      (e) => e,
    );
    expect(urls.length).toBe(sent);
    expect({ name: error.name, code: error.code, message: error.message }).toEqual({
      name: 'TypeError',
      code: undefined,
      message: `\`${method}\`: ${message}`,
    });
  });

  test('ownership: a non-object rejects too', async () => {
    const error = await rejected(() => client.stock.ownership.etfHoldings(5));
    expect(error.name).toBe('TypeError');
  });

  test('a valid integer is sent as is', async () => {
    await client.stock.technical.sma('2330', undefined, undefined, undefined, 5);
    expect(urls[urls.length - 1]).toBe('/v1.0/stock/technical/sma/2330?period=5');
  });
});

describe('still accepted', () => {
  test('undefined and null in the extra positions count as not given', async () => {
    await client.stock.intraday.trades('2330', undefined, null);
    expect(urls[urls.length - 1]).toBe('/v1.0/stock/intraday/trades/2330');
    await client.stock.intraday.candles({ symbol: '2330' }, undefined, null);
    expect(urls[urls.length - 1]).toBe('/v1.0/stock/intraday/candles/2330');
  });

  test('every declared positional argument', async () => {
    await client.stock.historical.candles('2330', '2026-01-01', '2026-01-31', 'D');
    expect(urls[urls.length - 1]).toBe('/v1.0/stock/historical/candles/2330?from=2026-01-01&to=2026-01-31&timeframe=D');
  });

  test('quote: the positional oddLot flag also applies to the params object', async () => {
    await client.stock.intraday.quote({ symbol: '2330' }, true);
    expect(urls[urls.length - 1]).toBe('/v1.0/stock/intraday/quote/2330?type=oddlot');
  });
});
