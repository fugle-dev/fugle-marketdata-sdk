/**
 * Object-form REST params — the call shape of the legacy `@fugle/marketdata`
 * 1.x SDK and of the examples on developer.fugle.tw.
 *
 * The path param (`symbol` / `market`) goes into the path and every other key
 * is forwarded verbatim as a query param. Asserted against the request a real
 * loopback server receives: no API key or network needed.
 */
const http = require('http');
const { RestClient } = require('../');

/** Serve `{}` and record every request URL. */
function serve() {
  const urls = [];
  const server = http.createServer((req, res) => {
    urls.push(req.url);
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end('{}');
  });
  return new Promise((resolve) => {
    server.listen(0, '127.0.0.1', () => {
      const { port } = server.address();
      resolve({
        client: new RestClient({ apiKey: 'test-key', baseUrl: `http://127.0.0.1:${port}` }),
        lastRequest: () => {
          const url = new URL(urls[urls.length - 1], 'http://localhost');
          const query = {};
          for (const [key, value] of url.searchParams) {
            query[key] = key in query ? [].concat(query[key], value) : value;
          }
          return { path: url.pathname, query };
        },
        close: () =>
          new Promise((done) => {
            // The Rust client pools keep-alive connections; close them or jest hangs.
            server.closeAllConnections();
            server.close(done);
          }),
      });
    });
  });
}

let ctx;
beforeAll(async () => {
  ctx = await serve();
});
afterAll(async () => {
  await ctx.close();
});

// [name, call, expected path (after /v1.0), expected query]
const CASES = [
  // stock.intraday
  ['stock.intraday.quote type=oddlot', (c) => c.stock.intraday.quote({ symbol: '2330', type: 'oddlot' }),
    '/stock/intraday/quote/2330', { type: 'oddlot' }],
  ['stock.intraday.quote oddLot: true (rc spelling)', (c) => c.stock.intraday.quote({ symbol: '2330', oddLot: true }),
    '/stock/intraday/quote/2330', { type: 'oddlot' }],
  ['stock.intraday.quote oddLot: false', (c) => c.stock.intraday.quote({ symbol: '2330', oddLot: false }),
    '/stock/intraday/quote/2330', {}],
  ['stock.intraday.ticker', (c) => c.stock.intraday.ticker({ symbol: '2330', type: 'oddlot' }),
    '/stock/intraday/ticker/2330', { type: 'oddlot' }],
  ['stock.intraday.candles', (c) => c.stock.intraday.candles({ symbol: '2330', type: 'oddlot', timeframe: '5', sort: 'desc' }),
    '/stock/intraday/candles/2330', { type: 'oddlot', timeframe: '5', sort: 'desc' }],
  ['stock.intraday.trades', (c) => c.stock.intraday.trades({ symbol: '2330', offset: 0, limit: 5, sort: 'asc', isTrial: false }),
    '/stock/intraday/trades/2330', { offset: '0', limit: '5', sort: 'asc', isTrial: 'false' }],
  ['stock.intraday.volumes', (c) => c.stock.intraday.volumes({ symbol: '2330', type: 'oddlot' }),
    '/stock/intraday/volumes/2330', { type: 'oddlot' }],
  ['stock.intraday.tickers', (c) => c.stock.intraday.tickers({ type: 'EQUITY', isAttention: true, isDisposition: false, isHalted: true }),
    '/stock/intraday/tickers', { type: 'EQUITY', isAttention: 'true', isDisposition: 'false', isHalted: 'true' }],

  // stock.historical
  ['stock.historical.candles', (c) => c.stock.historical.candles({ symbol: '2330', fields: 'open,close,change', adjusted: true, sort: 'asc' }),
    '/stock/historical/candles/2330', { fields: 'open,close,change', adjusted: 'true', sort: 'asc' }],
  ['stock.historical.stats', (c) => c.stock.historical.stats({ symbol: '2330' }),
    '/stock/historical/stats/2330', {}],

  // stock.snapshot
  ['stock.snapshot.quotes', (c) => c.stock.snapshot.quotes({ market: 'TSE', type: 'COMMONSTOCK' }),
    '/stock/snapshot/quotes/TSE', { type: 'COMMONSTOCK' }],
  ['stock.snapshot.movers', (c) => c.stock.snapshot.movers({ market: 'TSE', direction: 'up', change: 'percent', type: 'ALL', gte: 5, lt: 9.5 }),
    '/stock/snapshot/movers/TSE', { direction: 'up', change: 'percent', type: 'ALL', gte: '5', lt: '9.5' }],
  ['stock.snapshot.actives', (c) => c.stock.snapshot.actives({ market: 'OTC', trade: 'value', type: 'ALLBUT0999' }),
    '/stock/snapshot/actives/OTC', { trade: 'value', type: 'ALLBUT0999' }],

  // stock.technical
  ['stock.technical.sma', (c) => c.stock.technical.sma({ symbol: '2330', from: '2026-08-01', to: '2026-09-10', timeframe: 'D', period: 20 }),
    '/stock/technical/sma/2330', { from: '2026-08-01', to: '2026-09-10', timeframe: 'D', period: '20' }],
  ['stock.technical.rsi', (c) => c.stock.technical.rsi({ symbol: '2330', period: 14 }),
    '/stock/technical/rsi/2330', { period: '14' }],
  ['stock.technical.kdj', (c) => c.stock.technical.kdj({ symbol: '2330', timeframe: 'D', rPeriod: 9, kPeriod: 3, dPeriod: 3 }),
    '/stock/technical/kdj/2330', { timeframe: 'D', rPeriod: '9', kPeriod: '3', dPeriod: '3' }],
  ['stock.technical.macd', (c) => c.stock.technical.macd({ symbol: '2330', fast: 12, slow: 26, signal: 9 }),
    '/stock/technical/macd/2330', { fast: '12', slow: '26', signal: '9' }],
  ['stock.technical.bb', (c) => c.stock.technical.bb({ symbol: '2330', period: 20 }),
    '/stock/technical/bb/2330', { period: '20' }],

  // stock.corporateActions
  ['stock.corporateActions.dividends', (c) => c.stock.corporateActions.dividends({ start_date: '2026-08-01', end_date: '2026-09-30', sort: 'desc' }),
    '/stock/corporate-actions/dividends', { start_date: '2026-08-01', end_date: '2026-09-30', sort: 'desc' }],
  ['stock.corporateActions.capitalChanges', (c) => c.stock.corporateActions.capitalChanges({ start_date: '2026-08-01', sort: 'asc' }),
    '/stock/corporate-actions/capital-changes', { start_date: '2026-08-01', sort: 'asc' }],
  ['stock.corporateActions.listingApplicants', (c) => c.stock.corporateActions.listingApplicants({ end_date: '2026-09-30' }),
    '/stock/corporate-actions/listing-applicants', { end_date: '2026-09-30' }],

  // futopt.intraday
  ['futopt.intraday.products', (c) => c.futopt.intraday.products({ type: 'FUTURE', exchange: 'TAIFEX', session: 'AFTERHOURS', status: 'N' }),
    '/futopt/intraday/products', { type: 'FUTURE', exchange: 'TAIFEX', session: 'AFTERHOURS', status: 'N' }],
  ['futopt.intraday.tickers', (c) => c.futopt.intraday.tickers({ type: 'FUTURE', product: 'TXF', session: 'AFTERHOURS' }),
    '/futopt/intraday/tickers', { type: 'FUTURE', product: 'TXF', session: 'AFTERHOURS' }],
  ['futopt.intraday.quote', (c) => c.futopt.intraday.quote({ symbol: 'TXFC4', session: 'afterhours' }),
    '/futopt/intraday/quote/TXFC4', { session: 'afterhours' }],
  ['futopt.intraday.ticker', (c) => c.futopt.intraday.ticker({ symbol: 'TXFC4', session: 'afterhours' }),
    '/futopt/intraday/ticker/TXFC4', { session: 'afterhours' }],
  ['futopt.intraday.candles', (c) => c.futopt.intraday.candles({ symbol: 'TXFC4', session: 'afterhours', timeframe: '5' }),
    '/futopt/intraday/candles/TXFC4', { session: 'afterhours', timeframe: '5' }],
  ['futopt.intraday.trades', (c) => c.futopt.intraday.trades({ symbol: 'TXFC4', session: 'afterhours', offset: 10, limit: 5, isTrial: true }),
    '/futopt/intraday/trades/TXFC4', { session: 'afterhours', offset: '10', limit: '5', isTrial: 'true' }],
  ['futopt.intraday.volumes', (c) => c.futopt.intraday.volumes({ symbol: 'TXFC4', session: 'afterhours' }),
    '/futopt/intraday/volumes/TXFC4', { session: 'afterhours' }],

  // futopt.historical
  ['futopt.historical.candles', (c) => c.futopt.historical.candles({ symbol: 'TXFC4', from: '2026-08-01', session: 'afterhours' }),
    '/futopt/historical/candles/TXFC4', { from: '2026-08-01', session: 'afterhours' }],
  ['futopt.historical.daily', (c) => c.futopt.historical.daily({ symbol: 'TXFC4', to: '2026-09-01' }),
    '/futopt/historical/daily/TXFC4', { to: '2026-09-01' }],
];

describe('object params are forwarded verbatim', () => {
  test.each(CASES)('%s', async (_name, call, path, query) => {
    await call(ctx.client);
    expect(ctx.lastRequest()).toEqual({ path: `/v1.0${path}`, query });
  });

  test('a spread symbol stays one path segment', async () => {
    await ctx.client.futopt.intraday.quote({ symbol: 'TXFC4/TXFD4' });
    expect(ctx.lastRequest().path).toBe('/v1.0/futopt/intraday/quote/TXFC4%2FTXFD4');
  });

  test('undefined and null are skipped; arrays repeat the key', async () => {
    await ctx.client.stock.intraday.trades({ symbol: '2330', limit: undefined, offset: null, sort: ['asc', 'desc'] });
    expect(ctx.lastRequest().query).toEqual({ sort: ['asc', 'desc'] });
  });

  test('query values are percent-encoded', async () => {
    await ctx.client.stock.historical.candles({ symbol: '2330', fields: 'open&close=1' });
    expect(ctx.lastRequest().query).toEqual({ fields: 'open&close=1' });
  });

  test('a missing path param is rejected', async () => {
    await expect(ctx.client.stock.intraday.ticker({ type: 'oddlot' })).rejects.toThrow('`symbol` is required');
    await expect(ctx.client.stock.snapshot.quotes({ type: 'ALL' })).rejects.toThrow('`market` is required');
  });

  test('a nested object value is rejected', async () => {
    await expect(ctx.client.stock.intraday.trades({ symbol: '2330', limit: { n: 5 } })).rejects.toThrow('`limit` must be');
  });
});

describe('positional calls are unchanged', () => {
  test.each([
    ['ticker', (c) => c.stock.intraday.ticker('2330'), '/stock/intraday/ticker/2330', {}],
    ['quote odd lot', (c) => c.stock.intraday.quote('2330', true), '/stock/intraday/quote/2330', { type: 'oddlot' }],
    ['candles without timeframe', (c) => c.stock.intraday.candles('2330'), '/stock/intraday/candles/2330', {}],
    ['candles with timeframe', (c) => c.stock.intraday.candles('2330', '5'), '/stock/intraday/candles/2330', { timeframe: '5' }],
    ['kdj', (c) => c.stock.technical.kdj('2330', undefined, undefined, 'D', 9, 3, 3),
      '/stock/technical/kdj/2330', { timeframe: 'D', rPeriod: '9', kPeriod: '3', dPeriod: '3' }],
    ['dividends without args', (c) => c.stock.corporateActions.dividends(), '/stock/corporate-actions/dividends', {}],
    ['dividends with range', (c) => c.stock.corporateActions.dividends(undefined, '2026-08-01', '2026-09-30'),
      '/stock/corporate-actions/dividends', { start_date: '2026-08-01', end_date: '2026-09-30' }],
    ['futopt tickers', (c) => c.futopt.intraday.tickers('FUTURE'), '/futopt/intraday/tickers', { type: 'FUTURE' }],
  ])('%s', async (_name, call, path, query) => {
    await call(ctx.client);
    expect(ctx.lastRequest()).toEqual({ path: `/v1.0${path}`, query });
  });

  test('a missing first argument is rejected', async () => {
    await expect(ctx.client.stock.intraday.ticker()).rejects.toThrow('`symbol` is required');
  });
});
