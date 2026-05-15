const { RestClient, WebSocketClient } = require('./index.js');

const API_KEY = process.env.FUGLE_API_KEY;
if (!API_KEY) {
  console.error('Set FUGLE_API_KEY environment variable first.');
  process.exit(1);
}

// Create client
const ws = new WebSocketClient({ apiKey: API_KEY });

// Register handlers
ws.stock.on('message', (data) => {
  const msg = JSON.parse(data);
  console.log('Message:', msg);
});

ws.stock.on('connect', () => {
  console.log('Connected!');
  ws.stock.subscribe({ channel: 'trades', symbol: '2330' });
});

ws.stock.on('disconnect', (reason) => {
  console.log('Disconnected:', reason);
});

ws.stock.on('error', (err) => {
  console.error('Error:', err);
});

// Connect
ws.stock.connect();

// Disconnect after 30 seconds
setTimeout(() => {
  ws.stock.disconnect();
}, 5000);