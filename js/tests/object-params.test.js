/**
 * Object-form REST params — the call shape of the legacy `@fugle/marketdata`
 * 1.x SDK and of the examples on developer.fugle.tw.
 *
 * The path param (`symbol` / `market`) goes into the path and every other key
 * is checked against core's table and sent under the API name. Asserted
 * against the request a real loopback server receives: no API key or network
 * needed.
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
  ['futopt.historical.candles', (c) => c.futopt.historical.candles({ symbol: 'TXF', from: '2026-08-01', contractMonth: '1!', fields: 'open,close', sort: 'desc', session: 'AFTERHOURS' }),
    '/futopt/historical/candles/TXF', { from: '2026-08-01', contractMonth: '1!', fields: 'open,close', sort: 'desc', session: 'AFTERHOURS' }],
  ['futopt.historical.candles with product', (c) => c.futopt.historical.candles({ product: 'TXF', contractMonth: '202609' }),
    '/futopt/historical/candles/TXF', { contractMonth: '202609' }],
  ['futopt.historical.daily', (c) => c.futopt.historical.daily({ symbol: 'TXF', date: '2026-09-15', session: 'afterhours' }),
    '/futopt/historical/daily/TXF', { date: '2026-09-15', session: 'afterhours' }],
  ['futopt.historical.daily with product', (c) => c.futopt.historical.daily({ product: 'TXF' }),
    '/futopt/historical/daily/TXF', {}],
];

describe('object params are sent under the API names', () => {
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

// Keys are checked against core's table (see core::rest::params, #164);
// the server would ignore a typo or answer 400 depending on the endpoint.
describe('unknown keys are rejected', () => {
  // Errors come from the native module's realm, so `instanceof Error` is
  // false under jest; check the brand instead.
  const rejection = async (promise) => {
    const err = await promise.then(() => undefined, (e) => e);
    expect(Object.prototype.toString.call(err)).toBe('[object Error]');
    return err;
  };

  test('a typo names the endpoint, the suggestion and the accepted keys', async () => {
    const err = await rejection(ctx.client.futopt.intraday.tickers({ type: 'FUTURE', prodcut: 'TXF' }));
    expect(err.message).toBe(
      "Invalid parameter 'prodcut': `futopt.intraday.tickers` does not accept `prodcut`; " +
        'accepted keys: type, exchange, session, product, contractType, isSpread'
    );
  });

  test('a near miss in case or underscores suggests the API name', async () => {
    const err = await rejection(ctx.client.stock.intraday.trades({ symbol: '2330', istrial: true }));
    expect(err.message).toContain('did you mean `isTrial`?');
    expect(err.message).toContain('accepted keys: symbol, type, offset, limit, sort, isTrial');
  });

  test.each([
    ['oddlot', (c) => c.stock.intraday.ticker({ symbol: '2330', oddlot: true }), 'oddLot'],
    ['odd_Lot', (c) => c.stock.intraday.candles({ symbol: '2330', odd_Lot: true }), 'odd_lot'],
    ['afterhours', (c) => c.futopt.intraday.quote({ symbol: 'TXFD6', afterhours: true }), 'after_hours'],
    ['AfterHours', (c) => c.futopt.intraday.products({ type: 'FUTURE', AfterHours: true }), 'after_hours'],
  ])('a near miss of a flag (%s) suggests the flag, not the wire name it sets', async (_key, call, suggestion) => {
    // `type: true` / `session: true` would be wrong; the suggested spelling
    // takes the boolean the caller already wrote.
    const err = await rejection(call(ctx.client));
    expect(err.message).toContain(`did you mean \`${suggestion}\`?`);
    expect(err.message).not.toContain('did you mean `type`');
    expect(err.message).not.toContain('did you mean `session`');
  });

  test('the rejection carries the unified error fields', async () => {
    const err = await rejection(ctx.client.stock.intraday.ticker({ symbol: '2330', Type: 'oddlot' }));
    expect(err).toMatchObject({ code: 1005, sourceKind: 'client', status: null, body: null, requestId: null });
    expect(err.message).toContain('did you mean `type`?');
  });

  test('the path param given under two names is refused', async () => {
    await expect(
      ctx.client.futopt.historical.daily({ product: 'TXF', symbol: 'TXF' })
    ).rejects.toThrow('`symbol` and `product` both name the path param; give one');
    const err = await rejection(ctx.client.futopt.historical.daily({ product: 'TXF', dat: '2026-09-01' }));
    expect(err.message).toContain('accepted keys: symbol, product, date, session');
  });

  test.each([
    ['products does not take product', (c) => c.futopt.intraday.products({ type: 'FUTURE', product: 'TXF' }), '`product`'],
    ['tickers does not take status', (c) => c.futopt.intraday.tickers({ type: 'FUTURE', status: 'N' }), '`status`'],
    ['capital-changes does not take exchange', (c) => c.stock.corporateActions.capitalChanges({ exchange: 'TWSE' }), '`exchange`'],
    ['dividends does not take date', (c) => c.stock.corporateActions.dividends({ date: '2026-09-01' }), '`date`'],
    ['stats takes nothing but symbol', (c) => c.stock.historical.stats({ symbol: '2330', from: '2026-01-01' }), '`from`'],
    ['a key valid elsewhere is still unknown here', (c) => c.stock.intraday.quote({ symbol: '2330', timeframe: '5' }), '`timeframe`'],
  ])('%s', async (_name, call, key) => {
    const err = await rejection(call(ctx.client));
    expect(err.message).toContain(`does not accept ${key}`);
    expect(err.code).toBe(1005);
  });

  test('an unknown key is refused even when its value is null', async () => {
    await expect(ctx.client.stock.intraday.quote({ symbol: '2330', tpye: null })).rejects.toThrow('does not accept `tpye`');
  });

  test('the same parameter given under two spellings is refused', async () => {
    await expect(
      ctx.client.stock.intraday.trades({ symbol: '2330', isTrial: true, is_trial: false })
    ).rejects.toThrow(/`(isTrial|is_trial)` is `isTrial`, already given as `(is_trial|isTrial)`/);
    // Which spelling is reported first follows napi's property enumeration,
    // not the literal's order, so only the pairing is pinned.
  });

  test('a flag form must be a boolean', async () => {
    await expect(ctx.client.stock.intraday.quote({ symbol: '2330', oddLot: 'yes' })).rejects.toThrow('`oddLot` must be a boolean');
  });

  test('values are not checked: the server answers those', async () => {
    await ctx.client.stock.intraday.quote({ symbol: '2330', type: 'ODDLOT' });
    expect(ctx.lastRequest().query).toEqual({ type: 'ODDLOT' });
    await ctx.client.stock.snapshot.movers({ market: 'TSE', direction: 'sideways', change: 'percent' });
    expect(ctx.lastRequest().query).toEqual({ direction: 'sideways', change: 'percent' });
  });
});

describe('snake_case and flag aliases resolve through the table', () => {
  test.each([
    ['is_trial → isTrial', (c) => c.stock.intraday.trades({ symbol: '2330', is_trial: true, limit: 5 }),
      '/stock/intraday/trades/2330', { isTrial: 'true', limit: '5' }],
    ['contract_month → contractMonth', (c) => c.futopt.historical.candles({ product: 'TXF', contract_month: '2!' }),
      '/futopt/historical/candles/TXF', { contractMonth: '2!' }],
    ['r_period/k_period/d_period', (c) => c.stock.technical.kdj({ symbol: '2330', r_period: 9, k_period: 3, d_period: 3 }),
      '/stock/technical/kdj/2330', { rPeriod: '9', kPeriod: '3', dPeriod: '3' }],
    ['odd_lot: true on ticker', (c) => c.stock.intraday.ticker({ symbol: '2330', odd_lot: true }),
      '/stock/intraday/ticker/2330', { type: 'oddlot' }],
    ['oddLot: true on candles (was only special-cased on quote)', (c) => c.stock.intraday.candles({ symbol: '2330', oddLot: true }),
      '/stock/intraday/candles/2330', { type: 'oddlot' }],
    ['odd_lot: false sends nothing', (c) => c.stock.intraday.volumes({ symbol: '2330', odd_lot: false }),
      '/stock/intraday/volumes/2330', {}],
    ['after_hours: true on a single contract is lower case', (c) => c.futopt.intraday.quote({ symbol: 'TXFD6', after_hours: true }),
      '/futopt/intraday/quote/TXFD6', { session: 'afterhours' }],
    ['after_hours: true on a list endpoint is upper case', (c) => c.futopt.intraday.products({ type: 'FUTURE', after_hours: true }),
      '/futopt/intraday/products', { type: 'FUTURE', session: 'AFTERHOURS' }],
    ['session given verbatim is sent as given', (c) => c.futopt.intraday.tickers({ type: 'FUTURE', session: 'afterhours' }),
      '/futopt/intraday/tickers', { type: 'FUTURE', session: 'afterhours' }],
    ['positional oddLot with the object form', (c) => c.stock.intraday.quote({ symbol: '2330' }, true),
      '/stock/intraday/quote/2330', { type: 'oddlot' }],
    ['object oddLot: false wins over positional true', (c) => c.stock.intraday.quote({ symbol: '2330', oddLot: false }, true),
      '/stock/intraday/quote/2330', {}],
    ['object type: oddlot with positional true is not a duplicate', (c) => c.stock.intraday.quote({ symbol: '2330', type: 'oddlot' }, true),
      '/stock/intraday/quote/2330', { type: 'oddlot' }],
    ['object odd_lot: true with positional true is not a duplicate', (c) => c.stock.intraday.quote({ symbol: '2330', odd_lot: true }, true),
      '/stock/intraday/quote/2330', { type: 'oddlot' }],
    ['object oddLot: null with positional true', (c) => c.stock.intraday.quote({ symbol: '2330', oddLot: null }, true),
      '/stock/intraday/quote/2330', { type: 'oddlot' }],
  ])('%s', async (_name, call, path, query) => {
    await call(ctx.client);
    expect(ctx.lastRequest()).toEqual({ path: `/v1.0${path}`, query });
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
    ['dividends with range', (c) => c.stock.corporateActions.dividends('2026-08-01', '2026-09-30'),
      '/stock/corporate-actions/dividends', { start_date: '2026-08-01', end_date: '2026-09-30' }],
    ['capital changes with start only', (c) => c.stock.corporateActions.capitalChanges('2026-08-01'),
      '/stock/corporate-actions/capital-changes', { start_date: '2026-08-01' }],
    ['listing applicants with end only (object form)', (c) => c.stock.corporateActions.listingApplicants({ end_date: '2026-09-30' }),
      '/stock/corporate-actions/listing-applicants', { end_date: '2026-09-30' }],
    ['futopt tickers', (c) => c.futopt.intraday.tickers('FUTURE'), '/futopt/intraday/tickers', { type: 'FUTURE' }],
    ['futopt historical candles', (c) => c.futopt.historical.candles('TXF', '2026-09-01', '2026-09-15', '5', true, '2!', 'close', 'asc'),
      '/futopt/historical/candles/TXF',
      { from: '2026-09-01', to: '2026-09-15', contractMonth: '2!', fields: 'close', timeframe: '5', sort: 'asc', session: 'afterhours' }],
    ['futopt historical daily', (c) => c.futopt.historical.daily('TXF', '2026-09-15', true),
      '/futopt/historical/daily/TXF', { date: '2026-09-15', session: 'afterhours' }],
    ['futopt historical daily regular session', (c) => c.futopt.historical.daily('TXF', undefined, false),
      '/futopt/historical/daily/TXF', {}],
  ])('%s', async (_name, call, path, query) => {
    await call(ctx.client);
    expect(ctx.lastRequest()).toEqual({ path: `/v1.0${path}`, query });
  });

  test('a missing first argument is rejected', async () => {
    await expect(ctx.client.stock.intraday.ticker()).rejects.toThrow('`symbol` is required');
  });
});

// `date` was the first positional argument of the corporate-actions methods
// until the server turned out never to accept it (#168). `startDate` now sits
// in that slot, so the old `(date, startDate, endDate)` and `(date, startDate)`
// calls would silently run with a shifted range; both are refused with
// directions instead. The second one also covers a new call that only wants
// `endDate`, which the object form handles.
describe('corporate-actions legacy date argument', () => {
  test.each([
    ['capitalChanges', (c) => c.stock.corporateActions.capitalChanges(undefined, '2026-08-01', '2026-09-30')],
    ['dividends', (c) => c.stock.corporateActions.dividends('2026-08-15', '2026-08-01', '2026-09-30')],
    ['listingApplicants', (c) => c.stock.corporateActions.listingApplicants(null, '2026-08-01', '2026-09-30')],
  ])('%s(date, startDate, endDate) is rejected with directions', async (name, call) => {
    await expect(call(ctx.client)).rejects.toThrow(`\`${name}\` no longer takes \`date\``);
    await expect(call(ctx.client)).rejects.toThrow(`${name}(startDate, endDate)`);
  });

  test.each([
    ['capitalChanges', (c) => c.stock.corporateActions.capitalChanges(undefined, '2026-08-01')],
    ['dividends', (c) => c.stock.corporateActions.dividends(null, '2026-08-01')],
    ['listingApplicants', (c) => c.stock.corporateActions.listingApplicants(undefined, '2026-08-01')],
  ])('%s(undefined, startDate) is rejected and pointed at the object form', async (name, call) => {
    await expect(call(ctx.client)).rejects.toThrow(`\`${name}\` got an undefined first argument`);
    await expect(call(ctx.client)).rejects.toThrow(`${name}({ end_date })`);
  });

  test('an explicit undefined third argument is not a legacy call', async () => {
    await ctx.client.stock.corporateActions.dividends('2026-08-01', '2026-09-30', undefined);
    expect(ctx.lastRequest()).toEqual({
      path: '/v1.0/stock/corporate-actions/dividends',
      query: { start_date: '2026-08-01', end_date: '2026-09-30' },
    });
  });
});
