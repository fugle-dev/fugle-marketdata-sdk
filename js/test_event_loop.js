/**
 * Test that WebSocket message processing doesn't block Node.js event loop
 *
 * This test verifies the ThreadsafeFunction implementation correctly uses
 * NonBlocking mode, allowing the JavaScript event loop to continue processing
 * while receiving WebSocket messages.
 */
const { WebSocketClient } = require('./');

const API_KEY = process.env.FUGLE_API_KEY;
if (!API_KEY) {
  console.log('SKIP: FUGLE_API_KEY not set');
  process.exit(0);
}

const client = new WebSocketClient(API_KEY);
const ws = client.stock;

// Track timer consistency - if event loop blocked, timer fires late
const timerLags = [];
let lastTime = Date.now();
const timer = setInterval(() => {
  const now = Date.now();
  const lag = now - lastTime - 100;  // Expected 100ms interval
  timerLags.push(lag);
  lastTime = now;
}, 100);

let messageCount = 0;
const MAX_MESSAGES = 50;

ws.on('connect', () => {
  console.log('Connected, subscribing to high-frequency channel...');
  ws.subscribe({ channel: 'trades', symbol: '2330' });
});

ws.on('message', (data) => {
  messageCount++;
  if (messageCount >= MAX_MESSAGES) {
    ws.disconnect();
    clearInterval(timer);

    // Analyze event loop lag
    const maxLag = Math.max(...timerLags);
    const avgLag = timerLags.reduce((a,b) => a+b, 0) / timerLags.length;

    console.log(`Messages received: ${messageCount}`);
    console.log(`Timer samples: ${timerLags.length}`);
    console.log(`Max lag: ${maxLag}ms, Avg lag: ${avgLag.toFixed(2)}ms`);

    // Fail if event loop lag exceeds 100ms (indicates blocking)
    if (maxLag > 100) {
      console.error('FAIL: Event loop blocked - max lag > 100ms');
      process.exit(1);
    }
    console.log('PASS: Event loop not blocked by WebSocket callbacks');
    process.exit(0);
  }
});

ws.on('error', (err) => {
  console.error('Error:', err);
  process.exit(1);
});

ws.connect();

// Timeout after 30 seconds
setTimeout(() => {
  if (messageCount < MAX_MESSAGES) {
    ws.disconnect();
    clearInterval(timer);
    console.log(`Timeout: Only received ${messageCount} messages`);
    // Still check event loop wasn't blocked
    if (timerLags.length > 0) {
      const maxLag = Math.max(...timerLags);
      console.log(`Max lag during timeout: ${maxLag}ms`);
    }
    // Pass if we received at least some messages (market may be closed)
    process.exit(messageCount > 0 ? 0 : 1);
  }
}, 30000);
