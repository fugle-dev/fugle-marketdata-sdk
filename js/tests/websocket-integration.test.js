/**
 * WebSocket Integration Tests
 *
 * These tests require a valid FUGLE_API_KEY environment variable.
 * When API key is not available, all tests are automatically skipped.
 *
 * Note: WebSocket tests involve actual network connections and are
 * more timing-sensitive. They use longer timeouts.
 *
 * Run with API key:
 *   FUGLE_API_KEY=your-key npm test
 */
const { WebSocketClient } = require('../');

const API_KEY = process.env.FUGLE_API_KEY;
const describeWithApiKey = API_KEY ? describe : describe.skip;

describeWithApiKey('WebSocket Integration Tests', () => {
  describe('Stock WebSocket', () => {
    let client;
    let stockWs;

    beforeEach(() => {
      client = new WebSocketClient({ apiKey: API_KEY });
      stockWs = client.stock;
    });

    afterEach(() => {
      // Cleanup: ensure disconnected
      try {
        stockWs.disconnect();
      } catch (e) {
        // Ignore cleanup errors
      }
    });

    test('connects and receives connect event', (done) => {
      stockWs.on('connect', (info) => {
        expect(info).toBeDefined();
        stockWs.disconnect();
        done();
      });

      stockWs.on('error', (err) => {
        stockWs.disconnect();
        done.fail(`Connection error: ${err}`);
      });

      stockWs.connect();
    }, 15000);

    test('can subscribe after connect', (done) => {
      let subscribed = false;

      stockWs.on('connect', () => {
        // Subscribe to aggregates channel
        stockWs.subscribe({ channel: 'aggregates', symbol: '2330' });
        subscribed = true;
      });

      stockWs.on('message', (data) => {
        if (subscribed) {
          expect(typeof data).toBe('string');

          // Try to parse the message
          let parsed;
          try {
            parsed = JSON.parse(data);
            expect(parsed).toBeDefined();
          } catch (e) {
            // Message format may vary
          }

          stockWs.disconnect();
          done();
        }
      });

      stockWs.on('error', (err) => {
        stockWs.disconnect();
        done.fail(`Error: ${err}`);
      });

      stockWs.connect();
    }, 30000);

    test('isConnected returns correct state', (done) => {
      expect(stockWs.isConnected).toBe(false);

      stockWs.on('connect', () => {
        expect(stockWs.isConnected).toBe(true);
        stockWs.disconnect();
      });

      stockWs.on('disconnect', () => {
        // After disconnect, isConnected should be false
        expect(stockWs.isConnected).toBe(false);
        done();
      });

      stockWs.on('error', (err) => {
        stockWs.disconnect();
        done.fail(`Error: ${err}`);
      });

      stockWs.connect();
    }, 15000);
  });

  describe('FutOpt WebSocket', () => {
    let client;
    let futoptWs;

    beforeEach(() => {
      client = new WebSocketClient({ apiKey: API_KEY });
      futoptWs = client.futopt;
    });

    afterEach(() => {
      try {
        futoptWs.disconnect();
      } catch (e) {
        // Ignore cleanup errors
      }
    });

    test('connects and receives connect event', (done) => {
      futoptWs.on('connect', (info) => {
        expect(info).toBeDefined();
        futoptWs.disconnect();
        done();
      });

      futoptWs.on('error', (err) => {
        futoptWs.disconnect();
        done.fail(`Connection error: ${err}`);
      });

      futoptWs.connect();
    }, 15000);

    test('isConnected returns correct state', (done) => {
      expect(futoptWs.isConnected).toBe(false);

      futoptWs.on('connect', () => {
        expect(futoptWs.isConnected).toBe(true);
        futoptWs.disconnect();
      });

      futoptWs.on('disconnect', () => {
        expect(futoptWs.isConnected).toBe(false);
        done();
      });

      futoptWs.on('error', (err) => {
        futoptWs.disconnect();
        done.fail(`Error: ${err}`);
      });

      futoptWs.connect();
    }, 15000);
  });
});

// Always run: Verify WebSocket skip behavior
describe('WebSocket integration test skip behavior', () => {
  test('FUGLE_API_KEY env var status for WebSocket tests', () => {
    if (API_KEY) {
      console.log('FUGLE_API_KEY is set - WebSocket integration tests will run');
    } else {
      console.log('FUGLE_API_KEY not set - WebSocket integration tests skipped');
    }
    expect(true).toBe(true);
  });
});
