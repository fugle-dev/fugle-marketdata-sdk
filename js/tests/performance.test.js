/**
 * Performance Benchmark Tests for Node.js Binding
 *
 * Measures latency for REST client operations and compares
 * against official @fugle/marketdata SDK baseline.
 *
 * Validates SC #4: "Performance within 1.5x for Node.js"
 *
 * Run: npm test -- --testPathPattern=performance
 */

const fs = require('fs');
const path = require('path');
const { RestClient } = require('../');

// Load official SDK baseline if available
const BASELINE_PATH = path.join(__dirname, 'benchmarks', 'baseline.json');
let OFFICIAL_BASELINE = null;
try {
  OFFICIAL_BASELINE = JSON.parse(fs.readFileSync(BASELINE_PATH, 'utf8'));
} catch (e) {
  // Baseline not recorded yet
}

// Performance threshold from SC #4
const NODEJS_THRESHOLD_MULTIPLIER = 1.5; // Within 1.5x of official SDK

describe('Performance Benchmarks', () => {
  describe('Client Creation', () => {
    test('client creation latency', () => {
      const iterations = 100;
      const start = performance.now();

      for (let i = 0; i < iterations; i++) {
        const client = new RestClient({ apiKey: 'test-key' });
      }

      const elapsed = performance.now() - start;
      const avgMs = elapsed / iterations;

      console.log(`Client creation: ${avgMs.toFixed(2)}ms avg (${iterations} iterations)`);
      expect(avgMs).toBeLessThan(10); // Should be < 10ms per client
    });
  });

  describe('FFI Overhead', () => {
    test('quote method call overhead (no network)', async () => {
      const client = new RestClient({ apiKey: 'test-key' });
      const iterations = 10;
      const latencies = [];

      for (let i = 0; i < iterations; i++) {
        const start = performance.now();
        try {
          await client.stock.intraday.quote('2330');
        } catch (e) {
          // Expected without valid API key
        }
        latencies.push(performance.now() - start);
      }

      // Remove outliers and calculate median
      latencies.sort((a, b) => a - b);
      const median = latencies[Math.floor(latencies.length / 2)];

      console.log(`FFI overhead: ${median.toFixed(2)}ms median`);
      // FFI overhead should be reasonable
      expect(median).toBeLessThan(100);
    });
  });
});

describe('Integration Performance Benchmarks', () => {
  const apiKey = process.env.FUGLE_API_KEY;

  beforeAll(() => {
    if (!apiKey) {
      console.log('Skipping integration benchmarks: FUGLE_API_KEY not set');
    }
  });

  const describeOrSkip = apiKey ? describe : describe.skip;

  describeOrSkip('Live API Latency', () => {
    let client;

    beforeAll(() => {
      client = new RestClient({ apiKey });
    });

    test('quote latency', async () => {
      const latency = await measureLatency(() =>
        client.stock.intraday.quote('2330')
      );

      console.log(`Quote latency: ${latency.toFixed(2)}ms`);
      expect(latency).toBeLessThan(5000); // Should complete within 5s
    });

    test('ticker latency', async () => {
      const latency = await measureLatency(() =>
        client.stock.intraday.ticker('2330')
      );

      console.log(`Ticker latency: ${latency.toFixed(2)}ms`);
      expect(latency).toBeLessThan(5000);
    });
  });
});

describe('Official SDK Comparison', () => {
  const apiKey = process.env.FUGLE_API_KEY;

  beforeAll(() => {
    if (!apiKey) {
      console.log('Skipping SDK comparison: FUGLE_API_KEY not set');
    }
    if (!OFFICIAL_BASELINE) {
      console.log('Skipping SDK comparison: baseline not recorded');
      console.log('Run: node tests/benchmarks/record-official-baseline.js');
    }
  });

  const shouldRun = apiKey && OFFICIAL_BASELINE;
  const describeOrSkip = shouldRun ? describe : describe.skip;

  describeOrSkip('Latency Comparison', () => {
    let client;

    beforeAll(() => {
      client = new RestClient({ apiKey });
    });

    test('quote within 1.5x of official SDK', async () => {
      const officialMedian = OFFICIAL_BASELINE.operations.quote.median_ms;
      const ourMedian = await measureLatency(() =>
        client.stock.intraday.quote('2330'),
        { rounds: 10, warmup: 3 }
      );

      const ratio = ourMedian / officialMedian;
      const threshold = NODEJS_THRESHOLD_MULTIPLIER;

      console.log(`\nQuote Performance:`);
      console.log(`  Official SDK: ${officialMedian.toFixed(2)}ms`);
      console.log(`  Our SDK: ${ourMedian.toFixed(2)}ms`);
      console.log(`  Ratio: ${ratio.toFixed(2)}x (threshold: ${threshold}x)`);

      expect(ratio).toBeLessThanOrEqual(threshold);
    });

    test('ticker within 1.5x of official SDK', async () => {
      const officialMedian = OFFICIAL_BASELINE.operations.ticker.median_ms;
      const ourMedian = await measureLatency(() =>
        client.stock.intraday.ticker('2330'),
        { rounds: 10, warmup: 3 }
      );

      const ratio = ourMedian / officialMedian;
      const threshold = NODEJS_THRESHOLD_MULTIPLIER;

      console.log(`\nTicker Performance:`);
      console.log(`  Official SDK: ${officialMedian.toFixed(2)}ms`);
      console.log(`  Our SDK: ${ourMedian.toFixed(2)}ms`);
      console.log(`  Ratio: ${ratio.toFixed(2)}x (threshold: ${threshold}x)`);

      expect(ratio).toBeLessThanOrEqual(threshold);
    });

    test('trades within 1.5x of official SDK', async () => {
      const officialMedian = OFFICIAL_BASELINE.operations.trades.median_ms;
      const ourMedian = await measureLatency(() =>
        client.stock.intraday.trades('2330'),
        { rounds: 10, warmup: 3 }
      );

      const ratio = ourMedian / officialMedian;
      const threshold = NODEJS_THRESHOLD_MULTIPLIER;

      console.log(`\nTrades Performance:`);
      console.log(`  Official SDK: ${officialMedian.toFixed(2)}ms`);
      console.log(`  Our SDK: ${ourMedian.toFixed(2)}ms`);
      console.log(`  Ratio: ${ratio.toFixed(2)}x (threshold: ${threshold}x)`);

      expect(ratio).toBeLessThanOrEqual(threshold);
    });
  });
});

// Helper functions
async function measureLatency(operation, options = {}) {
  const { rounds = 5, warmup = 2 } = options;

  // Warmup
  for (let i = 0; i < warmup; i++) {
    try { await operation(); } catch (e) {}
    await sleep(500);
  }

  // Measure
  const latencies = [];
  for (let i = 0; i < rounds; i++) {
    const start = performance.now();
    try {
      await operation();
      latencies.push(performance.now() - start);
    } catch (e) {
      // Still record time for failed requests
    }
    await sleep(500);
  }

  // Return median
  latencies.sort((a, b) => a - b);
  return latencies[Math.floor(latencies.length / 2)];
}

function sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}
