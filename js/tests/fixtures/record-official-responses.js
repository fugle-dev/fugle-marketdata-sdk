/**
 * Instructions for recording official API responses
 *
 * This script documents the process for capturing actual API responses
 * from the official @fugle/marketdata SDK to create fixtures for testing.
 *
 * SETUP:
 * 1. Install the official SDK in a temporary directory:
 *    mkdir temp-recorder && cd temp-recorder
 *    npm init -y
 *    npm install @fugle/marketdata
 *
 * 2. Create a recording script (e.g., record.js):
 *
 * const fs = require('fs');
 * const { RestClient } = require('@fugle/marketdata');
 *
 * const client = new RestClient({ apiKey: process.env.FUGLE_API_KEY });
 *
 * async function recordResponses() {
 *   // Record quote response
 *   const quote = await client.stock.intraday.quote({ symbol: '2330' });
 *   fs.writeFileSync('official-quote.json', JSON.stringify(quote, null, 2));
 *
 *   // Record ticker response
 *   const ticker = await client.stock.intraday.ticker({ symbol: '2330' });
 *   fs.writeFileSync('official-ticker.json', JSON.stringify(ticker, null, 2));
 *
 *   // Record trades response
 *   const trades = await client.stock.intraday.trades({ symbol: '2330', limit: 10 });
 *   fs.writeFileSync('official-trades.json', JSON.stringify(trades, null, 2));
 *
 *   // Record candles response
 *   const candles = await client.stock.intraday.candles({ symbol: '2330', type: '1' });
 *   fs.writeFileSync('official-candles.json', JSON.stringify(candles, null, 2));
 *
 *   // Record volumes response
 *   const volumes = await client.stock.intraday.volumes({ symbol: '2330' });
 *   fs.writeFileSync('official-volumes.json', JSON.stringify(volumes, null, 2));
 * }
 *
 * recordResponses().catch(console.error);
 *
 * 3. Run with your API key:
 *    FUGLE_API_KEY=your_key_here node record.js
 *
 * 4. Copy the generated JSON files to this fixtures directory
 *
 * NOTE: The fixtures in this directory are mock responses created based on
 * Fugle API documentation. Replace them with real responses when available.
 */

console.log('See file header comments for instructions on recording official API responses');
