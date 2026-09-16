/**
 * Passthrough tests: what the server sends is what the caller gets.
 *
 * These replace the old "response compatibility" suite, which asserted
 * `toHaveProperty` against the fixture JSON file itself rather than against
 * anything the SDK produced. Those assertions could never fail — three of them
 * were vouching for `referencePrice`, `previousClose` and `serial` while the
 * SDK was in fact dropping all three.
 *
 * The SDK makes real HTTP calls from Rust, so `nock` cannot intercept them.
 * We stand up a real loopback server instead and point `baseUrl` at it, which
 * exercises the whole chain: HTTP, JSON decode, and the conversion into JS
 * values.
 */

const http = require('http');
const { RestClient } = require('../');

/** A real `GET /stock/intraday/quote/2330` body, captured 2026-09-16. */
const QUOTE_2330 = {
  date: '2026-09-16',
  type: 'EQUITY',
  exchange: 'TWSE',
  market: 'TSE',
  symbol: '2330',
  name: '台積電',
  referencePrice: 2380,
  previousClose: 2385,
  openPrice: 2375,
  openTime: 1789520409721819,
  highPrice: 2385,
  highTime: 1789522040749587,
  lowPrice: 2375,
  lowTime: 1789520409721819,
  closePrice: 2385,
  closeTime: 1789525853959140,
  avgPrice: 2378.51,
  change: 5,
  changePercent: 0.21,
  amplitude: 0.42,
  lastPrice: 2385,
  lastSize: 1,
  bids: [
    { price: 2380, size: 185 },
    { price: 2375, size: 1407 },
  ],
  asks: [
    { price: 2385, size: 199 },
    { price: 2390, size: 556 },
  ],
  total: {
    tradeValue: 14566025000,
    tradeVolume: 6124,
    tradeVolumeAtBid: 1962,
    tradeVolumeAtAsk: 2686,
    transaction: 1922,
    time: 1789525853959140,
  },
  lastTrade: {
    bid: 2380,
    ask: 2385,
    price: 2385,
    size: 1,
    time: 1789525853959140,
    serial: 6132837,
  },
  isContinuous: true,
  serial: 6152257,
  lastUpdated: 1789525882301247,
};

/**
 * Serve `body` on every request and hand back a client pointed at it.
 * Returns the raw JSON text too, so tests can compare against the exact bytes.
 */
function serve(body) {
  const json = JSON.stringify(body);
  const server = http.createServer((req, res) => {
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(json);
  });

  return new Promise((resolve) => {
    server.listen(0, '127.0.0.1', () => {
      const { port } = server.address();
      resolve({
        server,
        json,
        client: new RestClient({
          apiKey: 'test-key',
          baseUrl: `http://127.0.0.1:${port}`,
        }),
        close: () =>
          new Promise((done) => {
            // The Rust client pools keep-alive connections, so the socket is
            // still open here; without this jest hangs after the run.
            server.closeAllConnections();
            server.close(done);
          }),
      });
    });
  });
}

describe('REST responses reach the caller untouched', () => {
  let ctx;

  afterEach(async () => {
    if (ctx) await ctx.close();
    ctx = null;
  });

  test('the response is byte-for-byte what the server sent', async () => {
    ctx = await serve(QUOTE_2330);
    const quote = await ctx.client.stock.intraday.quote('2330');

    expect(quote).toEqual(QUOTE_2330);
    // Nothing added, nothing removed — compare the key sets directly rather
    // than relying on toEqual's treatment of undefined.
    expect(Object.keys(quote).sort()).toEqual(Object.keys(QUOTE_2330).sort());
  });

  test('key order matches the server, not alphabetical order', async () => {
    ctx = await serve(QUOTE_2330);
    const quote = await ctx.client.stock.intraday.quote('2330');

    // Previously the JSON object was held in a sorted map, so callers saw
    // `amplitude, asks, avgPrice, bids, …` instead of the server's ordering.
    expect(Object.keys(quote)).toEqual(Object.keys(QUOTE_2330));
    expect(Object.keys(quote)[0]).toBe('date');
  });

  test('referencePrice survives, and it is what change is computed from', async () => {
    ctx = await serve(QUOTE_2330);
    const quote = await ctx.client.stock.intraday.quote('2330');

    expect(quote.referencePrice).toBe(2380);
    expect(quote.lastPrice - quote.referencePrice).toBe(quote.change);
    // Deriving it from previousClose would have given the wrong answer.
    expect(quote.lastPrice - quote.previousClose).not.toBe(quote.change);
  });

  test('fields the server omitted stay absent instead of defaulting to false', async () => {
    ctx = await serve(QUOTE_2330);
    const quote = await ctx.client.stock.intraday.quote('2330');

    // The server sent only `isContinuous`. The others used to be materialised
    // as `false`, which callers could not tell apart from a real `false`.
    expect(quote.isContinuous).toBe(true);
    expect('isOpen' in quote).toBe(false);
    expect('isClose' in quote).toBe(false);
    expect('isTrial' in quote).toBe(false);
    expect('isLimitUpPrice' in quote).toBe(false);
    expect('tradingHalt' in quote).toBe(false);
  });

  test('a field this SDK has never heard of still reaches the caller', async () => {
    ctx = await serve({ ...QUOTE_2330, someFieldAddedLater: { nested: [1, 2] } });
    const quote = await ctx.client.stock.intraday.quote('2330');

    expect(quote.someFieldAddedLater).toEqual({ nested: [1, 2] });
  });

  test('serial keeps the type the server used', async () => {
    ctx = await serve(QUOTE_2330);
    const quote = await ctx.client.stock.intraday.quote('2330');

    // The typed models normalise this to a string because futopt sends a
    // zero-padded one. Passthrough does not: the wire type is preserved.
    expect(typeof quote.serial).toBe('number');
    expect(typeof quote.lastTrade.serial).toBe('number');
  });

  test('tickers keeps the envelope instead of unwrapping it to an array', async () => {
    const envelope = {
      date: '2026-09-16',
      type: 'EQUITY',
      exchange: 'TWSE',
      market: 'TSE',
      data: [{ symbol: '2330', name: '台積電' }],
    };
    ctx = await serve(envelope);
    const result = await ctx.client.stock.intraday.tickers('EQUITY');

    // Earlier releases returned just `data`, losing the sibling metadata and
    // diverging from the official SDK.
    expect(result).toEqual(envelope);
    expect(Array.isArray(result)).toBe(false);
    expect(result.data).toHaveLength(1);
  });
});
