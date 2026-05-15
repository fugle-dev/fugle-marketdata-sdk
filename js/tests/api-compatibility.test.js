/**
 * API Compatibility Tests
 *
 * Verifies that the SDK structure matches the official @fugle/marketdata SDK.
 * These tests do NOT require network access or a valid API key - they only test
 * that the expected methods and properties exist with the correct types.
 */
const { RestClient, WebSocketClient } = require('../');

/**
 * Helper to check if a value is Promise-like (has .then and .catch methods)
 * This handles both native Promises and napi-rs Promises which may have different
 * constructors but behave identically.
 */
function isPromiseLike(value) {
  return value && typeof value.then === 'function' && typeof value.catch === 'function';
}

describe('API Compatibility', () => {
  describe('RestClient structure', () => {
    let client;

    beforeAll(() => {
      client = new RestClient({ apiKey: 'test-api-key' });
    });

    test('RestClient constructor accepts options object', () => {
      expect(() => new RestClient({ apiKey: 'any-key' })).not.toThrow();
    });

    test('RestClient has stock property', () => {
      expect(client.stock).toBeDefined();
    });

    test('RestClient has futopt property', () => {
      expect(client.futopt).toBeDefined();
    });

    describe('stock.intraday methods', () => {
      let intraday;

      beforeAll(() => {
        intraday = client.stock.intraday;
      });

      test('stock.intraday exists', () => {
        expect(intraday).toBeDefined();
      });

      test('quote method exists and is a function', () => {
        expect(typeof intraday.quote).toBe('function');
      });

      test('ticker method exists and is a function', () => {
        expect(typeof intraday.ticker).toBe('function');
      });

      test('candles method exists and is a function', () => {
        expect(typeof intraday.candles).toBe('function');
      });

      test('trades method exists and is a function', () => {
        expect(typeof intraday.trades).toBe('function');
      });

      test('volumes method exists and is a function', () => {
        expect(typeof intraday.volumes).toBe('function');
      });

      test('quote returns a Promise-like object', async () => {
        const result = intraday.quote('2330');
        expect(isPromiseLike(result)).toBe(true);
        // Wait for rejection and suppress it
        await expect(result).rejects.toThrow();
      });

      test('ticker returns a Promise-like object', async () => {
        const result = intraday.ticker('2330');
        expect(isPromiseLike(result)).toBe(true);
        await expect(result).rejects.toThrow();
      });

      test('candles returns a Promise-like object', async () => {
        const result = intraday.candles('2330', '5');
        expect(isPromiseLike(result)).toBe(true);
        await expect(result).rejects.toThrow();
      });

      test('trades returns a Promise-like object', async () => {
        const result = intraday.trades('2330');
        expect(isPromiseLike(result)).toBe(true);
        await expect(result).rejects.toThrow();
      });

      test('volumes returns a Promise-like object', async () => {
        const result = intraday.volumes('2330');
        expect(isPromiseLike(result)).toBe(true);
        await expect(result).rejects.toThrow();
      });
    });

    describe('futopt.intraday methods', () => {
      let intraday;

      beforeAll(() => {
        intraday = client.futopt.intraday;
      });

      test('futopt.intraday exists', () => {
        expect(intraday).toBeDefined();
      });

      test('quote method exists and is a function', () => {
        expect(typeof intraday.quote).toBe('function');
      });

      test('ticker method exists and is a function', () => {
        expect(typeof intraday.ticker).toBe('function');
      });

      test('candles method exists and is a function', () => {
        expect(typeof intraday.candles).toBe('function');
      });

      test('trades method exists and is a function', () => {
        expect(typeof intraday.trades).toBe('function');
      });

      test('volumes method exists and is a function', () => {
        expect(typeof intraday.volumes).toBe('function');
      });

      test('products method exists and is a function', () => {
        expect(typeof intraday.products).toBe('function');
      });

      test('products returns a Promise-like object', async () => {
        const result = intraday.products('FUTURE');
        expect(isPromiseLike(result)).toBe(true);
        await expect(result).rejects.toThrow();
      });
    });

    // ========== New Endpoints (Phase 7) ==========

    describe('stock.historical methods', () => {
      let historical;

      beforeAll(() => {
        historical = client.stock.historical;
      });

      test('stock.historical exists', () => {
        expect(historical).toBeDefined();
      });

      test('candles method exists and is a function', () => {
        expect(typeof historical.candles).toBe('function');
      });

      test('candles returns a Promise-like object', async () => {
        const result = historical.candles('2330');
        expect(isPromiseLike(result)).toBe(true);
        await expect(result).rejects.toThrow();
      });

      test('stats method exists and is a function', () => {
        expect(typeof historical.stats).toBe('function');
      });

      test('stats returns a Promise-like object', async () => {
        const result = historical.stats('2330');
        expect(isPromiseLike(result)).toBe(true);
        await expect(result).rejects.toThrow();
      });
    });

    describe('stock.snapshot methods', () => {
      let snapshot;

      beforeAll(() => {
        snapshot = client.stock.snapshot;
      });

      test('stock.snapshot exists', () => {
        expect(snapshot).toBeDefined();
      });

      test('quotes method exists and is a function', () => {
        expect(typeof snapshot.quotes).toBe('function');
      });

      test('quotes returns a Promise-like object', async () => {
        const result = snapshot.quotes('TSE');
        expect(isPromiseLike(result)).toBe(true);
        await expect(result).rejects.toThrow();
      });

      test('movers method exists and is a function', () => {
        expect(typeof snapshot.movers).toBe('function');
      });

      test('movers returns a Promise-like object', async () => {
        const result = snapshot.movers('TSE');
        expect(isPromiseLike(result)).toBe(true);
        await expect(result).rejects.toThrow();
      });

      test('actives method exists and is a function', () => {
        expect(typeof snapshot.actives).toBe('function');
      });

      test('actives returns a Promise-like object', async () => {
        const result = snapshot.actives('TSE');
        expect(isPromiseLike(result)).toBe(true);
        await expect(result).rejects.toThrow();
      });
    });

    describe('stock.technical methods', () => {
      let technical;

      beforeAll(() => {
        technical = client.stock.technical;
      });

      test('stock.technical exists', () => {
        expect(technical).toBeDefined();
      });

      test('sma method exists and is a function', () => {
        expect(typeof technical.sma).toBe('function');
      });

      test('sma returns a Promise-like object', async () => {
        const result = technical.sma('2330');
        expect(isPromiseLike(result)).toBe(true);
        await expect(result).rejects.toThrow();
      });

      test('rsi method exists and is a function', () => {
        expect(typeof technical.rsi).toBe('function');
      });

      test('rsi returns a Promise-like object', async () => {
        const result = technical.rsi('2330');
        expect(isPromiseLike(result)).toBe(true);
        await expect(result).rejects.toThrow();
      });

      test('kdj method exists and is a function', () => {
        expect(typeof technical.kdj).toBe('function');
      });

      test('kdj returns a Promise-like object', async () => {
        const result = technical.kdj('2330');
        expect(isPromiseLike(result)).toBe(true);
        await expect(result).rejects.toThrow();
      });

      test('macd method exists and is a function', () => {
        expect(typeof technical.macd).toBe('function');
      });

      test('macd returns a Promise-like object', async () => {
        const result = technical.macd('2330');
        expect(isPromiseLike(result)).toBe(true);
        await expect(result).rejects.toThrow();
      });

      test('bb method exists and is a function', () => {
        expect(typeof technical.bb).toBe('function');
      });

      test('bb returns a Promise-like object', async () => {
        const result = technical.bb('2330');
        expect(isPromiseLike(result)).toBe(true);
        await expect(result).rejects.toThrow();
      });
    });

    describe('stock.corporateActions methods', () => {
      let corporateActions;

      beforeAll(() => {
        corporateActions = client.stock.corporateActions;
      });

      test('stock.corporateActions exists', () => {
        expect(corporateActions).toBeDefined();
      });

      test('capitalChanges method exists and is a function', () => {
        expect(typeof corporateActions.capitalChanges).toBe('function');
      });

      test('capitalChanges returns a Promise-like object', async () => {
        const result = corporateActions.capitalChanges();
        expect(isPromiseLike(result)).toBe(true);
        await expect(result).rejects.toThrow();
      });

      test('dividends method exists and is a function', () => {
        expect(typeof corporateActions.dividends).toBe('function');
      });

      test('dividends returns a Promise-like object', async () => {
        const result = corporateActions.dividends();
        expect(isPromiseLike(result)).toBe(true);
        await expect(result).rejects.toThrow();
      });

      test('listingApplicants method exists and is a function', () => {
        expect(typeof corporateActions.listingApplicants).toBe('function');
      });

      test('listingApplicants returns a Promise-like object', async () => {
        const result = corporateActions.listingApplicants();
        expect(isPromiseLike(result)).toBe(true);
        await expect(result).rejects.toThrow();
      });
    });

    describe('futopt.historical methods', () => {
      let historical;

      beforeAll(() => {
        historical = client.futopt.historical;
      });

      test('futopt.historical exists', () => {
        expect(historical).toBeDefined();
      });

      test('candles method exists and is a function', () => {
        expect(typeof historical.candles).toBe('function');
      });

      test('candles returns a Promise-like object', async () => {
        const result = historical.candles('TXFC4');
        expect(isPromiseLike(result)).toBe(true);
        await expect(result).rejects.toThrow();
      });

      test('daily method exists and is a function', () => {
        expect(typeof historical.daily).toBe('function');
      });

      test('daily returns a Promise-like object', async () => {
        const result = historical.daily('TXFC4');
        expect(isPromiseLike(result)).toBe(true);
        await expect(result).rejects.toThrow();
      });
    });
  });

  describe('WebSocketClient structure', () => {
    let client;

    beforeAll(() => {
      client = new WebSocketClient({ apiKey: 'test-api-key' });
    });

    test('WebSocketClient constructor accepts options object', () => {
      expect(() => new WebSocketClient({ apiKey: 'any-key' })).not.toThrow();
    });

    test('WebSocketClient has stock property', () => {
      expect(client.stock).toBeDefined();
    });

    test('WebSocketClient has futopt property', () => {
      expect(client.futopt).toBeDefined();
    });

    describe('stock WebSocket methods', () => {
      let stock;

      beforeAll(() => {
        stock = client.stock;
      });

      test('on method exists and is a function', () => {
        expect(typeof stock.on).toBe('function');
      });

      test('connect method exists and is a function', () => {
        expect(typeof stock.connect).toBe('function');
      });

      test('subscribe method exists and is a function', () => {
        expect(typeof stock.subscribe).toBe('function');
      });

      test('unsubscribe method exists and is a function', () => {
        expect(typeof stock.unsubscribe).toBe('function');
      });

      test('disconnect method exists and is a function', () => {
        expect(typeof stock.disconnect).toBe('function');
      });

      test('isConnected is a boolean', () => {
        expect(typeof stock.isConnected).toBe('boolean');
      });

      test('isClosed is a boolean', () => {
        expect(typeof stock.isClosed).toBe('boolean');
      });

      test('initial state is disconnected', () => {
        expect(stock.isConnected).toBe(false);
      });
    });

    describe('futopt WebSocket methods', () => {
      let futopt;

      beforeAll(() => {
        futopt = client.futopt;
      });

      test('on method exists and is a function', () => {
        expect(typeof futopt.on).toBe('function');
      });

      test('connect method exists and is a function', () => {
        expect(typeof futopt.connect).toBe('function');
      });

      test('subscribe method exists and is a function', () => {
        expect(typeof futopt.subscribe).toBe('function');
      });

      test('unsubscribe method exists and is a function', () => {
        expect(typeof futopt.unsubscribe).toBe('function');
      });

      test('disconnect method exists and is a function', () => {
        expect(typeof futopt.disconnect).toBe('function');
      });

      test('isConnected is a boolean', () => {
        expect(typeof futopt.isConnected).toBe('boolean');
      });

      test('isClosed is a boolean', () => {
        expect(typeof futopt.isClosed).toBe('boolean');
      });

      test('initial state is disconnected', () => {
        expect(futopt.isConnected).toBe(false);
      });
    });
  });

  describe('Event callback registration', () => {
    test('stock WebSocket accepts event callbacks without error', () => {
      const client = new WebSocketClient({ apiKey: 'test-key' });

      expect(() => {
        client.stock.on('message', (data) => {});
        client.stock.on('connect', (info) => {});
        client.stock.on('disconnect', (reason) => {});
        client.stock.on('reconnect', (info) => {});
        client.stock.on('error', (err) => {});
      }).not.toThrow();
    });

    test('futopt WebSocket accepts event callbacks without error', () => {
      const client = new WebSocketClient({ apiKey: 'test-key' });

      expect(() => {
        client.futopt.on('message', (data) => {});
        client.futopt.on('connect', (info) => {});
        client.futopt.on('disconnect', (reason) => {});
        client.futopt.on('reconnect', (info) => {});
        client.futopt.on('error', (err) => {});
      }).not.toThrow();
    });
  });
});
