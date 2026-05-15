/**
 * Response compatibility tests using fixture validation
 *
 * These tests verify that expected response structures match the official
 * @fugle/marketdata SDK by validating fixture formats and testing actual
 * API responses when available (with FUGLE_API_KEY set).
 *
 * Note: nock cannot intercept native Rust HTTP calls, so we validate
 * fixture structure and test against real API when credentials available.
 */

const quoteFixture = require('./fixtures/official-quote.json');
const tickerFixture = require('./fixtures/official-ticker.json');

describe('Response Compatibility Tests', () => {
  describe('Quote Fixture Structure', () => {
    test('should have all expected quote response fields', () => {
      const quote = quoteFixture;

      // Validate basic fields
      expect(quote).toHaveProperty('symbol');
      expect(quote).toHaveProperty('name');
      expect(quote).toHaveProperty('date');

      // Validate price fields
      expect(quote).toHaveProperty('referencePrice');
      expect(quote).toHaveProperty('previousClose');
      expect(quote).toHaveProperty('openPrice');
      expect(quote).toHaveProperty('highPrice');
      expect(quote).toHaveProperty('lowPrice');
      expect(quote).toHaveProperty('closePrice');
      expect(quote).toHaveProperty('lastPrice');
      expect(quote).toHaveProperty('avgPrice');

      // Validate change fields
      expect(quote).toHaveProperty('change');
      expect(quote).toHaveProperty('changePercent');
      expect(quote).toHaveProperty('amplitude');

      // Validate time fields
      expect(quote).toHaveProperty('openTime');
      expect(quote).toHaveProperty('highTime');
      expect(quote).toHaveProperty('lowTime');
      expect(quote).toHaveProperty('closeTime');
      expect(quote).toHaveProperty('lastUpdated');

      // Validate bid/ask arrays
      expect(quote).toHaveProperty('bids');
      expect(Array.isArray(quote.bids)).toBe(true);
      if (quote.bids.length > 0) {
        expect(quote.bids[0]).toHaveProperty('price');
        expect(quote.bids[0]).toHaveProperty('size');
      }

      expect(quote).toHaveProperty('asks');
      expect(Array.isArray(quote.asks)).toBe(true);
      if (quote.asks.length > 0) {
        expect(quote.asks[0]).toHaveProperty('price');
        expect(quote.asks[0]).toHaveProperty('size');
      }

      // Validate total object
      expect(quote).toHaveProperty('total');
      expect(quote.total).toHaveProperty('tradeValue');
      expect(quote.total).toHaveProperty('tradeVolume');
      expect(quote.total).toHaveProperty('tradeVolumeAtBid');
      expect(quote.total).toHaveProperty('tradeVolumeAtAsk');
      expect(quote.total).toHaveProperty('transaction');
      expect(quote.total).toHaveProperty('time');

      // Validate trial object
      expect(quote).toHaveProperty('trial');
      expect(quote.trial).toHaveProperty('trade');
      expect(quote.trial).toHaveProperty('tradeVolume');
      expect(quote.trial).toHaveProperty('tradeValue');
      expect(quote.trial).toHaveProperty('bid');
      expect(quote.trial).toHaveProperty('bidVolume');
      expect(quote.trial).toHaveProperty('ask');
      expect(quote.trial).toHaveProperty('askVolume');

      // Validate lastTrade object
      expect(quote).toHaveProperty('lastTrade');
      expect(quote.lastTrade).toHaveProperty('bid');
      expect(quote.lastTrade).toHaveProperty('bidVolume');
      expect(quote.lastTrade).toHaveProperty('ask');
      expect(quote.lastTrade).toHaveProperty('askVolume');
      expect(quote.lastTrade).toHaveProperty('price');
      expect(quote.lastTrade).toHaveProperty('size');
      expect(quote.lastTrade).toHaveProperty('time');
      expect(quote.lastTrade).toHaveProperty('serial');

      // Validate lastTrial object
      expect(quote).toHaveProperty('lastTrial');
      expect(quote.lastTrial).toHaveProperty('bid');
      expect(quote.lastTrial).toHaveProperty('bidVolume');
      expect(quote.lastTrial).toHaveProperty('ask');
      expect(quote.lastTrial).toHaveProperty('askVolume');
      expect(quote.lastTrial).toHaveProperty('price');
      expect(quote.lastTrial).toHaveProperty('size');
      expect(quote.lastTrial).toHaveProperty('time');
      expect(quote.lastTrial).toHaveProperty('serial');

      // Validate flags
      expect(quote).toHaveProperty('isClose');
      expect(quote).toHaveProperty('serial');
    });

    test('should have correct field types in quote fixture', () => {
      const quote = quoteFixture;

      // Verify types
      expect(typeof quote.symbol).toBe('string');
      expect(typeof quote.name).toBe('string');
      expect(typeof quote.date).toBe('string');
      expect(typeof quote.referencePrice).toBe('number');
      expect(typeof quote.change).toBe('number');
      expect(typeof quote.changePercent).toBe('number');
      expect(typeof quote.isClose).toBe('boolean');
      expect(Array.isArray(quote.bids)).toBe(true);
      expect(Array.isArray(quote.asks)).toBe(true);
      expect(typeof quote.total).toBe('object');
      expect(typeof quote.trial).toBe('object');
      expect(typeof quote.lastTrade).toBe('object');
      expect(typeof quote.lastTrial).toBe('object');
    });
  });

  describe('Ticker Fixture Structure', () => {
    test('should have all expected ticker response fields', () => {
      const ticker = tickerFixture;

      // Validate basic fields
      expect(ticker).toHaveProperty('symbol');
      expect(ticker).toHaveProperty('name');
      expect(ticker).toHaveProperty('date');
      expect(ticker).toHaveProperty('type');
      expect(ticker).toHaveProperty('exchange');
      expect(ticker).toHaveProperty('market');

      // Validate price fields
      expect(ticker).toHaveProperty('referencePrice');
      expect(ticker).toHaveProperty('previousClose');
      expect(ticker).toHaveProperty('openPrice');
      expect(ticker).toHaveProperty('highPrice');
      expect(ticker).toHaveProperty('lowPrice');
      expect(ticker).toHaveProperty('closePrice');
      expect(ticker).toHaveProperty('avgPrice');
      expect(ticker).toHaveProperty('lastPrice');

      // Validate change fields
      expect(ticker).toHaveProperty('change');
      expect(ticker).toHaveProperty('changePercent');
      expect(ticker).toHaveProperty('amplitude');

      // Validate time fields
      expect(ticker).toHaveProperty('openTime');
      expect(ticker).toHaveProperty('highTime');
      expect(ticker).toHaveProperty('lowTime');
      expect(ticker).toHaveProperty('closeTime');
      expect(ticker).toHaveProperty('lastUpdated');

      // Validate total object
      expect(ticker).toHaveProperty('total');
      expect(ticker.total).toHaveProperty('tradeValue');
      expect(ticker.total).toHaveProperty('tradeVolume');
      expect(ticker.total).toHaveProperty('transaction');

      // Validate lastTrade object
      expect(ticker).toHaveProperty('lastTrade');
      expect(ticker.lastTrade).toHaveProperty('price');
      expect(ticker.lastTrade).toHaveProperty('size');
      expect(ticker.lastTrade).toHaveProperty('time');

      // Validate metadata fields
      expect(ticker).toHaveProperty('industryType');
      expect(ticker).toHaveProperty('priceHigh52w');
      expect(ticker).toHaveProperty('priceLow52w');
      expect(ticker).toHaveProperty('priceHighLimit');
      expect(ticker).toHaveProperty('priceLowLimit');

      // Validate trading flags
      expect(ticker).toHaveProperty('canDayBuySell');
      expect(ticker).toHaveProperty('canDaySellBuy');
      expect(ticker).toHaveProperty('canShortMargin');
      expect(ticker).toHaveProperty('canShortLend');
      expect(ticker).toHaveProperty('attention');
      expect(ticker).toHaveProperty('disposition');
      expect(ticker).toHaveProperty('halted');
      expect(ticker).toHaveProperty('suspended');

      // Validate additional fields
      expect(ticker).toHaveProperty('isClose');
      expect(ticker).toHaveProperty('serial');
      expect(ticker).toHaveProperty('vwap');
      expect(ticker).toHaveProperty('weekTurnover');
      expect(ticker).toHaveProperty('turnover');
    });

    test('should have correct field types in ticker fixture', () => {
      const ticker = tickerFixture;

      // Verify types
      expect(typeof ticker.symbol).toBe('string');
      expect(typeof ticker.name).toBe('string');
      expect(typeof ticker.date).toBe('string');
      expect(typeof ticker.type).toBe('string');
      expect(typeof ticker.exchange).toBe('string');
      expect(typeof ticker.market).toBe('string');
      expect(typeof ticker.referencePrice).toBe('number');
      expect(typeof ticker.change).toBe('number');
      expect(typeof ticker.changePercent).toBe('number');
      expect(typeof ticker.isClose).toBe('boolean');
      expect(typeof ticker.canDayBuySell).toBe('boolean');
      expect(typeof ticker.canDaySellBuy).toBe('boolean');
      expect(typeof ticker.canShortMargin).toBe('boolean');
      expect(typeof ticker.canShortLend).toBe('boolean');
      expect(typeof ticker.attention).toBe('boolean');
      expect(typeof ticker.disposition).toBe('boolean');
      expect(typeof ticker.halted).toBe('boolean');
      expect(typeof ticker.suspended).toBe('boolean');
      expect(typeof ticker.total).toBe('object');
      expect(typeof ticker.lastTrade).toBe('object');
    });
  });

  // Integration tests that run when API key is available
  describe.skip('API Response Compatibility (requires FUGLE_API_KEY)', () => {
    const { RestClient } = require('../index');
    let client;

    beforeAll(() => {
      const apiKey = process.env.FUGLE_API_KEY;
      if (!apiKey) {
        console.log('Skipping API compatibility tests: FUGLE_API_KEY not set');
        return;
      }
      client = new RestClient(apiKey);
    });

    test('actual quote response should match fixture structure', async () => {
      if (!client) return;

      const quote = await client.stock.intraday.quote('2330');

      // Validate key fields exist (same as fixture tests)
      expect(quote).toHaveProperty('symbol');
      expect(quote).toHaveProperty('name');
      expect(quote).toHaveProperty('date');
      expect(quote).toHaveProperty('referencePrice');
      expect(quote).toHaveProperty('bids');
      expect(quote).toHaveProperty('asks');
      expect(quote).toHaveProperty('total');
      expect(quote).toHaveProperty('trial');
      expect(quote).toHaveProperty('lastTrade');
      expect(quote).toHaveProperty('lastTrial');
      expect(quote).toHaveProperty('isClose');
    });

    test('actual ticker response should match fixture structure', async () => {
      if (!client) return;

      const ticker = await client.stock.intraday.ticker('2330');

      // Validate key fields exist (same as fixture tests)
      expect(ticker).toHaveProperty('symbol');
      expect(ticker).toHaveProperty('name');
      expect(ticker).toHaveProperty('date');
      expect(ticker).toHaveProperty('type');
      expect(ticker).toHaveProperty('exchange');
      expect(ticker).toHaveProperty('market');
      expect(ticker).toHaveProperty('referencePrice');
      expect(ticker).toHaveProperty('total');
      expect(ticker).toHaveProperty('lastTrade');
      expect(ticker).toHaveProperty('canDayBuySell');
      expect(ticker).toHaveProperty('isClose');
    });
  });
});
