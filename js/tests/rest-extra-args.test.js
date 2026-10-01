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
