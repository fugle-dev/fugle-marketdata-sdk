#!/usr/bin/env node
/**
 * Record Official SDK Baseline Performance
 *
 * Records baseline latency metrics from the official @fugle/marketdata SDK.
 * Run this once with FUGLE_API_KEY to establish comparison baseline.
 *
 * Usage: node record-official-baseline.js
 * Output: baseline.json with median latencies for each operation
 */

const fs = require('fs');
const path = require('path');

async function main() {
  const apiKey = process.env.FUGLE_API_KEY;
  if (!apiKey) {
    console.log('Error: FUGLE_API_KEY not set');
    console.log('Set your API key to record official SDK baseline:');
    console.log('  export FUGLE_API_KEY=your_key');
    console.log('  node record-official-baseline.js');
    process.exit(1);
  }

  let RestClient;
  try {
    const fugle = require('@fugle/marketdata');
    RestClient = fugle.RestClient;
  } catch (e) {
    console.log('Error: @fugle/marketdata not installed');
    console.log('Install official SDK: npm install @fugle/marketdata');
    process.exit(1);
  }

  console.log('Recording official SDK baseline...');
  const client = new RestClient({ apiKey });

  const operations = {
    quote: () => client.stock.intraday.quote({ symbol: '2330' }),
    ticker: () => client.stock.intraday.ticker({ symbol: '2330' }),
    trades: () => client.stock.intraday.trades({ symbol: '2330' }),
  };

  const results = {};
  const warmupRounds = 3;
  const measureRounds = 10;

  for (const [name, operation] of Object.entries(operations)) {
    console.log(`  Benchmarking ${name}...`);

    // Warmup
    for (let i = 0; i < warmupRounds; i++) {
      try { await operation(); } catch (e) {}
      await sleep(500);
    }

    // Measure
    const latencies = [];
    for (let i = 0; i < measureRounds; i++) {
      const start = performance.now();
      try {
        await operation();
        const elapsed = performance.now() - start;
        latencies.push(elapsed);
      } catch (e) {
        console.log(`    Round ${i + 1} failed: ${e.message}`);
      }
      await sleep(500);
    }

    if (latencies.length > 0) {
      latencies.sort((a, b) => a - b);
      const median = latencies[Math.floor(latencies.length / 2)];
      const mean = latencies.reduce((a, b) => a + b, 0) / latencies.length;

      results[name] = {
        median_ms: median,
        mean_ms: mean,
        min_ms: Math.min(...latencies),
        max_ms: Math.max(...latencies),
        rounds: latencies.length,
      };
      console.log(`    Median: ${median.toFixed(2)}ms`);
    }
  }

  const baselinePath = path.join(__dirname, 'baseline.json');
  fs.writeFileSync(baselinePath, JSON.stringify({
    sdk: '@fugle/marketdata',
    recorded_at: new Date().toISOString(),
    operations: results,
  }, null, 2));

  console.log(`\nBaseline written to: ${baselinePath}`);
}

function sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

main().catch(console.error);
