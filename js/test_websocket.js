/**
 * Test script for WebSocket bindings
 * Tests WebSocketClient, StockWebSocketClient, FutOptWebSocketClient
 */

const { WebSocketClient, StockWebSocketClient, FutOptWebSocketClient } = require('./index.js');

console.log('=== JavaScript WebSocket Bindings Test ===\n');

// Test 1: Module loading
console.log('Test 1: Module loading');
try {
  console.log('  - WebSocketClient:', typeof WebSocketClient);
  console.log('  - StockWebSocketClient:', typeof StockWebSocketClient);
  console.log('  - FutOptWebSocketClient:', typeof FutOptWebSocketClient);
  console.log('  Result: OK\n');
} catch (e) {
  console.log('  FAILED:', e.message, '\n');
  process.exit(1);
}

// Test 2: WebSocketClient creation
console.log('Test 2: WebSocketClient creation');
try {
  const ws = new WebSocketClient('test-api-key');
  console.log('  - Created WebSocketClient:', ws !== null);
  console.log('  Result: OK\n');
} catch (e) {
  console.log('  FAILED:', e.message, '\n');
  process.exit(1);
}

// Test 3: Stock/FutOpt client accessors
console.log('Test 3: Stock/FutOpt client accessors');
try {
  const ws = new WebSocketClient('test-api-key');
  const stock = ws.stock;
  const futopt = ws.futopt;

  console.log('  - ws.stock exists:', stock !== undefined);
  console.log('  - ws.stock.on:', typeof stock.on);
  console.log('  - ws.stock.connect:', typeof stock.connect);
  console.log('  - ws.stock.subscribe:', typeof stock.subscribe);
  console.log('  - ws.stock.unsubscribe:', typeof stock.unsubscribe);
  console.log('  - ws.stock.disconnect:', typeof stock.disconnect);
  console.log('  - ws.stock.isConnected:', typeof stock.isConnected);

  console.log('  - ws.futopt exists:', futopt !== undefined);
  console.log('  - ws.futopt.on:', typeof futopt.on);
  console.log('  - ws.futopt.connect:', typeof futopt.connect);
  console.log('  - ws.futopt.subscribe:', typeof futopt.subscribe);
  console.log('  Result: OK\n');
} catch (e) {
  console.log('  FAILED:', e.message, '\n');
  process.exit(1);
}

// Test 4: Event handler registration
console.log('Test 4: Event handler registration');
try {
  const ws = new WebSocketClient('test-api-key');

  // Register handlers
  ws.stock.on('message', (data) => console.log('  [stock] message:', data));
  ws.stock.on('connect', () => console.log('  [stock] connected'));
  ws.stock.on('disconnect', (reason) => console.log('  [stock] disconnected:', reason));
  ws.stock.on('error', (err) => console.log('  [stock] error:', err));

  ws.futopt.on('message', (data) => console.log('  [futopt] message:', data));
  ws.futopt.on('connect', () => console.log('  [futopt] connected'));
  ws.futopt.on('disconnect', (reason) => console.log('  [futopt] disconnected:', reason));
  ws.futopt.on('error', (err) => console.log('  [futopt] error:', err));

  console.log('  - All event handlers registered');
  console.log('  Result: OK\n');
} catch (e) {
  console.log('  FAILED:', e.message, '\n');
  process.exit(1);
}

// Test 5: Invalid event type
console.log('Test 5: Invalid event type handling');
try {
  const ws = new WebSocketClient('test-api-key');

  try {
    ws.stock.on('invalid_event', () => {});
    console.log('  FAILED: Should have thrown error\n');
    process.exit(1);
  } catch (e) {
    if (e.message.includes('Unknown event type')) {
      console.log('  - Correctly rejected invalid event type');
      console.log('  Result: OK\n');
    } else {
      console.log('  FAILED: Unexpected error:', e.message, '\n');
      process.exit(1);
    }
  }
} catch (e) {
  console.log('  FAILED:', e.message, '\n');
  process.exit(1);
}

// Test 6: isConnected getter
console.log('Test 6: isConnected getter');
try {
  const ws = new WebSocketClient('test-api-key');

  const stockConnected = ws.stock.isConnected;
  const futoptConnected = ws.futopt.isConnected;

  console.log('  - ws.stock.isConnected (initial):', stockConnected);
  console.log('  - ws.futopt.isConnected (initial):', futoptConnected);

  if (stockConnected === false && futoptConnected === false) {
    console.log('  Result: OK\n');
  } else {
    console.log('  FAILED: Expected false for isConnected\n');
    process.exit(1);
  }
} catch (e) {
  console.log('  FAILED:', e.message, '\n');
  process.exit(1);
}

// Test 7: Subscribe validation (without connection)
console.log('Test 7: Subscribe validation');
try {
  const ws = new WebSocketClient('test-api-key');

  // Should fail because not connected
  try {
    ws.stock.subscribe({ channel: 'trades', symbol: '2330' });
    console.log('  FAILED: Should have thrown error (not connected)\n');
    process.exit(1);
  } catch (e) {
    if (e.message.includes('Not connected')) {
      console.log('  - Correctly rejected subscribe before connect');
      console.log('  Result: OK\n');
    } else {
      console.log('  FAILED: Unexpected error:', e.message, '\n');
      process.exit(1);
    }
  }
} catch (e) {
  console.log('  FAILED:', e.message, '\n');
  process.exit(1);
}

// Test 8: TypeScript definitions check
console.log('Test 8: TypeScript definitions');
try {
  const fs = require('fs');
  const dts = fs.readFileSync('./index.d.ts', 'utf8');

  const hasWebSocketClient = dts.includes('export declare class WebSocketClient');
  const hasStockWebSocketClient = dts.includes('export declare class StockWebSocketClient');
  const hasFutOptWebSocketClient = dts.includes('export declare class FutOptWebSocketClient');
  const hasOnMethod = dts.includes('on(event: string');
  const hasConnectMethod = dts.includes('connect()');
  const hasSubscribeMethod = dts.includes('subscribe(options:');
  const hasIsConnected = dts.includes('get isConnected()');

  console.log('  - WebSocketClient class:', hasWebSocketClient);
  console.log('  - StockWebSocketClient class:', hasStockWebSocketClient);
  console.log('  - FutOptWebSocketClient class:', hasFutOptWebSocketClient);
  console.log('  - on() method:', hasOnMethod);
  console.log('  - connect() method:', hasConnectMethod);
  console.log('  - subscribe() method:', hasSubscribeMethod);
  console.log('  - isConnected getter:', hasIsConnected);

  if (hasWebSocketClient && hasStockWebSocketClient && hasFutOptWebSocketClient &&
      hasOnMethod && hasConnectMethod && hasSubscribeMethod && hasIsConnected) {
    console.log('  Result: All OK\n');
  } else {
    console.log('  FAILED: Missing some TypeScript definitions\n');
    process.exit(1);
  }
} catch (e) {
  console.log('  FAILED:', e.message, '\n');
  process.exit(1);
}

console.log('=== All Tests Passed ===\n');
console.log('Note: Connection tests require valid API key and network.');
console.log('Usage with real API key:');
console.log('  const ws = new WebSocketClient("your-api-key");');
console.log('  ws.stock.on("message", (data) => console.log(JSON.parse(data)));');
console.log('  ws.stock.on("connect", () => {');
console.log('    ws.stock.subscribe({ channel: "trades", symbol: "2330" });');
console.log('  });');
console.log('  ws.stock.connect();');
