/**
 * REST Integration Tests
 *
 * These tests require a valid FUGLE_API_KEY environment variable.
 * When API key is not available, all tests are automatically skipped.
 *
 * Run with API key:
 *   FUGLE_API_KEY=your-key npm test
 *
 * Run without API key (skipped):
 *   npm test
 */
const { RestClient } = require('../');

const API_KEY = process.env.FUGLE_API_KEY;
const describeWithApiKey = API_KEY ? describe : describe.skip;

describeWithApiKey('REST Integration Tests', () => {
  let client;

  beforeAll(() => {
    client = new RestClient({ apiKey: API_KEY });
  });

  describe('Stock Intraday', () => {
    test('quote returns valid data structure', async () => {
      const quote = await client.stock.intraday.quote('2330');

      expect(quote).toBeDefined();
      expect(quote.symbol).toBe('2330');
      expect(typeof quote.date).toBe('string');

      // These may be null outside trading hours, so just check they exist
      expect('bids' in quote).toBe(true);
      expect('asks' in quote).toBe(true);
      expect(Array.isArray(quote.bids)).toBe(true);
      expect(Array.isArray(quote.asks)).toBe(true);
    });

    test('ticker returns valid data structure', async () => {
      const ticker = await client.stock.intraday.ticker('2330');

      expect(ticker).toBeDefined();
      expect(ticker.symbol).toBe('2330');
      expect(typeof ticker.date).toBe('string');

      // Trading rule flags should be booleans
      expect(typeof ticker.canDayTrade).toBe('boolean');
    });

    test('candles returns valid data structure', async () => {
      const candles = await client.stock.intraday.candles('2330', '5');

      expect(candles).toBeDefined();
      expect(candles.symbol).toBe('2330');
      expect(Array.isArray(candles.data)).toBe(true);

      // If market is open and has data, check candle structure
      if (candles.data.length > 0) {
        const candle = candles.data[0];
        expect(typeof candle.open).toBe('number');
        expect(typeof candle.high).toBe('number');
        expect(typeof candle.low).toBe('number');
        expect(typeof candle.close).toBe('number');
        expect(typeof candle.volume).toBe('number');
      }
    });

    test('trades returns valid data structure', async () => {
      const trades = await client.stock.intraday.trades('2330');

      expect(trades).toBeDefined();
      expect(trades.symbol).toBe('2330');
      expect(Array.isArray(trades.data)).toBe(true);

      if (trades.data.length > 0) {
        const trade = trades.data[0];
        expect(typeof trade.price).toBe('number');
        expect(typeof trade.size).toBe('number');
        expect(typeof trade.time).toBe('number');
      }
    });

    test('volumes returns valid data structure', async () => {
      const volumes = await client.stock.intraday.volumes('2330');

      expect(volumes).toBeDefined();
      expect(volumes.symbol).toBe('2330');
      expect(Array.isArray(volumes.data)).toBe(true);

      if (volumes.data.length > 0) {
        const vol = volumes.data[0];
        expect(typeof vol.price).toBe('number');
        expect(typeof vol.volume).toBe('number');
      }
    });
  });

  describe('FutOpt Intraday', () => {
    // Note: FutOpt symbols change monthly, use a common front-month contract
    // or use the products endpoint to find valid symbols first

    test('products returns futures list', async () => {
      const products = await client.futopt.intraday.products('FUTURE');

      expect(products).toBeDefined();
      expect(typeof products.date).toBe('string');
      expect(Array.isArray(products.data)).toBe(true);

      if (products.data.length > 0) {
        const product = products.data[0];
        expect(typeof product.symbol).toBe('string');
        expect(typeof product.name).toBe('string');
      }
    });

    test('products returns options list', async () => {
      const products = await client.futopt.intraday.products('OPTION');

      expect(products).toBeDefined();
      expect(typeof products.date).toBe('string');
      expect(Array.isArray(products.data)).toBe(true);
    });

    test('products accepts contract type filter', async () => {
      const products = await client.futopt.intraday.products('FUTURE', 'I');

      expect(products).toBeDefined();
      expect(Array.isArray(products.data)).toBe(true);
    });
  });

  describe('Error handling', () => {
    test('invalid stock symbol returns error', async () => {
      await expect(
        client.stock.intraday.quote('INVALID_SYMBOL_12345')
      ).rejects.toThrow();
    });

    test('invalid futopt type returns error', async () => {
      await expect(
        client.futopt.intraday.products('INVALID_TYPE')
      ).rejects.toThrow();
    });
  });
});

// Always run: Verify skip behavior
describe('Integration test skip behavior', () => {
  test('FUGLE_API_KEY env var status', () => {
    if (API_KEY) {
      console.log('FUGLE_API_KEY is set - integration tests will run');
    } else {
      console.log('FUGLE_API_KEY not set - integration tests skipped');
    }
    // This test always passes - it's informational
    expect(true).toBe(true);
  });
});
