/**
 * Stock ownership endpoints — URL shape, sort validation and decoding.
 *
 * Runs against a local HTTP server: no API key or network needed.
 */
const http = require('http');
const { RestClient } = require('../');

const ENDPOINTS = {
  etfHoldings: 'etf-holdings',
  institutionalTrades: 'institutional-trades',
  directorHoldings: 'director-holdings',
  tdccDistribution: 'tdcc-distribution',
};

const BODIES = {
  'etf-holdings': {
    symbol: '0050',
    data: [{ date: '2026-07-31', components: [{ symbol: '2330', name: '台積電', quantity: 1, weight: 57.12 }] }],
  },
  'institutional-trades': {
    symbol: '2330',
    data: [{
      date: '2026-07-31',
      foreign: { buy: 30000000, sell: 25000000, net: 5000000 },
      trust: { buy: 1200000, sell: 800000, net: 400000 },
      dealer: { buy: null, sell: 900000, net: -400000 },
      total: 5000000,
    }],
  },
  'director-holdings': {
    symbol: '2330',
    data: [{
      date: '2026-05',
      directors: [{
        order: 1, title: '董事長', name: '某某', electedShares: 1000, heldShares: 1200,
        pledgedShares: 0, pledgeRatio: null, relatedHeldShares: 50, relatedPledgedShares: 0, relatedPledgeRatio: 0,
      }],
    }],
  },
  'tdcc-distribution': {
    symbol: '2330',
    data: [{ date: '2026-07-03', distributions: [{ range: '1-999', holders: 1500000, shares: 250000000, proportion: 0.96 }] }],
  },
};

describe('stock.ownership', () => {
  let server;
  let client;
  const paths = [];

  beforeAll(async () => {
    server = http.createServer((req, res) => {
      paths.push(req.url);
      const endpoint = req.url.split('/stock/ownership/')[1].split('/')[0];
      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify(BODIES[endpoint]));
    });
    await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
    client = new RestClient({ apiKey: 'test-key', baseUrl: `http://127.0.0.1:${server.address().port}` });
  });

  afterAll(() => new Promise((resolve) => server.close(resolve)));

  test('exposes every ownership endpoint', () => {
    for (const name of Object.keys(ENDPOINTS)) {
      expect(typeof client.stock.ownership[name]).toBe('function');
    }
  });

  test.each(Object.entries(ENDPOINTS))('%s requests the right path', async (name, path) => {
    await client.stock.ownership[name]({ symbol: '2330' });
    expect(paths[paths.length - 1]).toBe(`/v1.0/stock/ownership/${path}/2330`);
  });

  test.each(Object.entries(ENDPOINTS))('%s forwards from / to / sort', async (name, path) => {
    await client.stock.ownership[name]({ symbol: '2330', from: '2026-07-01', to: '2026-07-31', sort: 'desc' });
    expect(paths[paths.length - 1]).toBe(`/v1.0/stock/ownership/${path}/2330?from=2026-07-01&to=2026-07-31&sort=desc`);
  });

  test('rejects an unknown sort', async () => {
    await expect(client.stock.ownership.directorHoldings({ symbol: '2330', sort: 'newest' })).rejects.toThrow(/sort/);
  });

  test('decodes institutionalTrades', async () => {
    const data = await client.stock.ownership.institutionalTrades({ symbol: '2330' });
    expect(data.data[0].foreign.net).toBe(5000000);
    expect(data.data[0].dealer.buy).toBeNull();
    expect(data.data[0].total).toBe(5000000);
  });

  test('decodes directorHoldings', async () => {
    const data = await client.stock.ownership.directorHoldings({ symbol: '2330' });
    expect(data.data[0].date).toBe('2026-05');
    expect(data.data[0].directors[0].heldShares).toBe(1200);
    expect(data.data[0].directors[0].pledgeRatio).toBeNull();
  });

  test('decodes tdccDistribution', async () => {
    const data = await client.stock.ownership.tdccDistribution({ symbol: '2330' });
    expect(data.data[0].distributions[0]).toEqual({ range: '1-999', holders: 1500000, shares: 250000000, proportion: 0.96 });
  });
});
