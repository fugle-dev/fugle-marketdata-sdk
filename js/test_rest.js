/**
 * Test script for marketdata-js REST client
 *
 * This script tests:
 * 1. Module loading
 * 2. RestClient creation
 * 3. Client chain access (stock/futopt -> intraday)
 * 4. Async/Promise API behavior
 * 5. Error handling with error codes
 */

const { RestClient } = require('./index.js');

console.log('=== marketdata-js REST Client Tests ===\n');

// Test 1: Module loading
console.log('Test 1: Module loading');
try {
    console.log('  - RestClient class:', typeof RestClient === 'function' ? 'OK' : 'FAIL');
} catch (e) {
    console.log('  - FAIL:', e.message);
}

// Test 2: Client creation
console.log('\nTest 2: Client creation');
try {
    const client = new RestClient('test-api-key');
    console.log('  - new RestClient():', client ? 'OK' : 'FAIL');
} catch (e) {
    console.log('  - FAIL:', e.message);
}

// Test 3: Client chain access
console.log('\nTest 3: Client chain access');
try {
    const client = new RestClient('test-api-key');

    // Stock chain
    const stock = client.stock;
    console.log('  - client.stock:', typeof stock === 'object' ? 'OK' : 'FAIL');

    const stockIntraday = stock.intraday;
    console.log('  - client.stock.intraday:', typeof stockIntraday === 'object' ? 'OK' : 'FAIL');

    // FutOpt chain
    const futopt = client.futopt;
    console.log('  - client.futopt:', typeof futopt === 'object' ? 'OK' : 'FAIL');

    const futoptIntraday = futopt.intraday;
    console.log('  - client.futopt.intraday:', typeof futoptIntraday === 'object' ? 'OK' : 'FAIL');

    // Method existence
    console.log('  - quote method:', typeof stockIntraday.quote === 'function' ? 'OK' : 'FAIL');
    console.log('  - ticker method:', typeof stockIntraday.ticker === 'function' ? 'OK' : 'FAIL');
    console.log('  - candles method:', typeof stockIntraday.candles === 'function' ? 'OK' : 'FAIL');
    console.log('  - trades method:', typeof stockIntraday.trades === 'function' ? 'OK' : 'FAIL');
    console.log('  - volumes method:', typeof stockIntraday.volumes === 'function' ? 'OK' : 'FAIL');
    console.log('  - futopt.products method:', typeof futoptIntraday.products === 'function' ? 'OK' : 'FAIL');
} catch (e) {
    console.log('  - FAIL:', e.message);
}

// Test 4: Async/Promise API
console.log('\nTest 4: Async/Promise API');
(async () => {
    try {
        const client = new RestClient('test-api-key');

        // Test stock methods return Promises
        const stockQuote = client.stock.intraday.quote('2330');
        const stockTicker = client.stock.intraday.ticker('2330');
        const stockCandles = client.stock.intraday.candles('2330', '5');
        const stockTrades = client.stock.intraday.trades('2330');
        const stockVolumes = client.stock.intraday.volumes('2330');

        console.log('  - stock.quote() returns Promise:', stockQuote instanceof Promise ? 'OK' : 'FAIL');
        console.log('  - stock.ticker() returns Promise:', stockTicker instanceof Promise ? 'OK' : 'FAIL');
        console.log('  - stock.candles() returns Promise:', stockCandles instanceof Promise ? 'OK' : 'FAIL');
        console.log('  - stock.trades() returns Promise:', stockTrades instanceof Promise ? 'OK' : 'FAIL');
        console.log('  - stock.volumes() returns Promise:', stockVolumes instanceof Promise ? 'OK' : 'FAIL');

        // Suppress rejections
        stockQuote.catch(() => {});
        stockTicker.catch(() => {});
        stockCandles.catch(() => {});
        stockTrades.catch(() => {});
        stockVolumes.catch(() => {});

        // Test futopt methods return Promises
        const futoptQuote = client.futopt.intraday.quote('TXFC4');
        const futoptTicker = client.futopt.intraday.ticker('TXFC4');
        const futoptCandles = client.futopt.intraday.candles('TXFC4', '5');
        const futoptTrades = client.futopt.intraday.trades('TXFC4');
        const futoptVolumes = client.futopt.intraday.volumes('TXFC4');
        const futoptProducts = client.futopt.intraday.products('FUTURE');

        console.log('  - futopt.quote() returns Promise:', futoptQuote instanceof Promise ? 'OK' : 'FAIL');
        console.log('  - futopt.ticker() returns Promise:', futoptTicker instanceof Promise ? 'OK' : 'FAIL');
        console.log('  - futopt.candles() returns Promise:', futoptCandles instanceof Promise ? 'OK' : 'FAIL');
        console.log('  - futopt.trades() returns Promise:', futoptTrades instanceof Promise ? 'OK' : 'FAIL');
        console.log('  - futopt.volumes() returns Promise:', futoptVolumes instanceof Promise ? 'OK' : 'FAIL');
        console.log('  - futopt.products() returns Promise:', futoptProducts instanceof Promise ? 'OK' : 'FAIL');

        // Suppress rejections
        futoptQuote.catch(() => {});
        futoptTicker.catch(() => {});
        futoptCandles.catch(() => {});
        futoptTrades.catch(() => {});
        futoptVolumes.catch(() => {});
        futoptProducts.catch(() => {});

        // Test 5: Error handling with error codes
        console.log('\nTest 5: Error handling');

        // Test products with invalid type - should return validation error synchronously
        try {
            const result = await client.futopt.intraday.products('INVALID_TYPE');
            console.log('  - Invalid type validation: Unexpected success');
        } catch (err) {
            const isValidationError = err.message.includes('FUTURE') || err.message.includes('OPTION');
            console.log('  - Invalid type validation:', isValidationError ? 'OK' : 'FAIL');
            console.log('    Error:', err.message);
        }

        // API call will fail with invalid API key
        try {
            await client.stock.intraday.quote('2330');
            console.log('  - Auth error handling: Unexpected success');
        } catch (err) {
            const hasErrorCode = err.message.match(/\[\d+\]/);
            console.log('  - Error thrown properly:', err instanceof Error ? 'OK' : 'FAIL');
            console.log('  - Error has code format [XXXX]:', hasErrorCode ? 'OK' : 'FAIL');
            console.log('    Error:', err.message.substring(0, 80));
        }

        // Test 6: TypeScript types check (index.d.ts exists)
        console.log('\nTest 6: TypeScript definitions');
        const fs = require('fs');
        const path = require('path');
        const dtsPath = path.join(__dirname, 'index.d.ts');
        const exists = fs.existsSync(dtsPath);
        console.log('  - index.d.ts exists:', exists ? 'OK' : 'FAIL');

        if (exists) {
            const content = fs.readFileSync(dtsPath, 'utf8');
            console.log('  - Contains RestClient:', content.includes('class RestClient') ? 'OK' : 'FAIL');
            console.log('  - Contains StockClient:', content.includes('class StockClient') ? 'OK' : 'FAIL');
            console.log('  - Contains FutOptClient:', content.includes('class FutOptClient') ? 'OK' : 'FAIL');
            console.log('  - Contains async quote method:', content.includes('quote(symbol: string): Promise') ? 'OK' : 'FAIL');
        }

        console.log('\n=== Tests Complete ===');
    } catch (e) {
        console.log('  - Test setup FAIL:', e.message);
    }
})();
