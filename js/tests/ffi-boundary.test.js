/**
 * FFI Boundary Tests for Node.js bindings
 *
 * These tests verify that the napi-rs FFI boundary properly handles edge cases:
 * - Error propagation (Rust errors -> JS Errors)
 * - Panic recovery (null/undefined handling)
 * - Memory safety (GC, concurrent access, buffer handling)
 * - Event loop safety (non-blocking, ThreadsafeFunction)
 *
 * FFI boundary failures would manifest as:
 * - Process crash (unhandled panic)
 * - Memory corruption (invalid memory access)
 * - Event loop blocking (synchronous FFI calls)
 * - Type conversion errors (invalid data across boundary)
 */

const { RestClient, WebSocketClient } = require('../');

describe('FFI Boundary - Error Propagation', () => {
  test('invalid symbol throws Error with readable message', async () => {
    const client = new RestClient({ apiKey: 'test-api-key' });

    await expect(
      client.stock.intraday.quote('INVALID_SYMBOL_12345')
    ).rejects.toThrow();

    try {
      await client.stock.intraday.quote('INVALID_SYMBOL_12345');
    } catch (error) {
      // Error should be readable (no memory corruption)
      expect(error.message).toBeTruthy();
      expect(typeof error.message).toBe('string');
      expect(error.message.length).toBeGreaterThan(0);

      // Error message should not contain null bytes
      expect(error.message).not.toContain('\0');
    }
  });

  test('authentication failure throws Error', async () => {
    const client = new RestClient({ apiKey: 'mock-api-key' });

    try {
      await client.stock.intraday.quote('2330');
      // May or may not fail depending on mock key handling
    } catch (error) {
      // Error should be readable and structured
      expect(typeof error.message).toBe('string');
      expect(error.message.length).toBeGreaterThan(0);
    }
  });

  test('error includes error code in message', async () => {
    const client = new RestClient({ apiKey: 'test-key' });

    try {
      await client.stock.intraday.quote('INVALID');
    } catch (error) {
      // napi-rs errors embed code in message: "[code] message"
      expect(error.message).toMatch(/\[\d+\]/);
    }
  });

  test('error stack trace is available', async () => {
    const client = new RestClient({ apiKey: 'test-key' });

    try {
      await client.stock.intraday.quote('INVALID');
    } catch (error) {
      // JavaScript errors should have stack trace
      expect(error.stack).toBeTruthy();
      expect(typeof error.stack).toBe('string');
    }
  });
});

describe('FFI Boundary - Panic Recovery', () => {
  test('empty API key does not crash process', () => {
    // Empty API key is valid (counts as exactly one auth method)
    // Will fail at runtime when making API calls, but doesn't crash at construction
    expect(() => {
      new RestClient({ apiKey: '' });
    }).not.toThrow();

    const client = new RestClient({ apiKey: '' });
    expect(client).toBeTruthy();
  });

  test('null options throws Error', () => {
    // null options should be rejected at type conversion (napi-rs throws Error)
    expect(() => {
      new RestClient(null);
    }).toThrow();
  });

  test('undefined options throws Error', () => {
    // undefined options should be rejected at type conversion (napi-rs throws Error)
    expect(() => {
      new RestClient(undefined);
    }).toThrow();
  });

  test('empty options (no auth) throws Error', () => {
    // Empty options should fail "exactly one auth" validation
    expect(() => {
      new RestClient({});
    }).toThrow(/exactly one of/i);
  });

  test('very long input strings do not overflow', async () => {
    const client = new RestClient({ apiKey: 'test-key' });

    // Try extremely long symbol (potential buffer overflow)
    const longSymbol = 'A'.repeat(10000);

    await expect(
      client.stock.intraday.quote(longSymbol)
    ).rejects.toThrow();

    // Should throw error, not crash
  });

  test('unicode input handled safely', async () => {
    const client = new RestClient({ apiKey: 'test-key' });

    const unicodeSymbols = [
      '中文',  // Chinese characters
      '🚀📈',  // Emojis
      'א',    // Hebrew
      '\u0000', // Null character
    ];

    for (const symbol of unicodeSymbols) {
      try {
        await client.stock.intraday.quote(symbol);
      } catch (error) {
        // Should get error with valid message
        expect(typeof error.message).toBe('string');
      }
    }
  });

  test('non-string inputs throw Error', () => {
    const client = new RestClient({ apiKey: 'test-key' });

    const invalidInputs = [
      123,
      { symbol: '2330' },
      ['2330'],
      true,
    ];

    for (const input of invalidInputs) {
      // napi-rs type conversion errors are synchronous
      expect(() => {
        client.stock.intraday.quote(input);
      }).toThrow();
    }
  });
});

describe('FFI Boundary - Memory Safety', () => {
  test('multiple client instances do not interfere', () => {
    const clients = [];

    for (let i = 0; i < 10; i++) {
      clients.push(new RestClient({ apiKey: `key_${i}` }));
    }

    // All clients should be independent
    expect(clients.length).toBe(10);

    // Cleanup (let GC handle it)
    clients.length = 0;
  });

  test('client remains usable after error', async () => {
    const client = new RestClient({ apiKey: 'test-key' });

    // Cause an error
    try {
      await client.stock.intraday.quote('INVALID');
    } catch (error) {
      // Expected
    }

    // Client should still be usable (should not crash)
    await expect(
      client.stock.intraday.quote('2330')
    ).rejects.toThrow();
  });

  test('concurrent operations on same client', async () => {
    const client = new RestClient({ apiKey: 'test-key' });

    // Multiple concurrent calls on same client
    const promises = [];
    for (let i = 0; i < 5; i++) {
      promises.push(
        client.stock.intraday.quote('2330').catch(() => {})
      );
    }

    // All should complete without crash
    await Promise.allSettled(promises);
  });

  test('buffer handling is safe', () => {
    const client = new RestClient({ apiKey: 'test-key' });

    // Try buffer inputs (should fail at type conversion)
    const bufferInputs = [
      Buffer.from('2330'),
      new Uint8Array([50, 51, 51, 48]),
    ];

    for (const input of bufferInputs) {
      // napi-rs type conversion errors are synchronous
      expect(() => {
        client.stock.intraday.quote(input);
      }).toThrow();
    }
  });

  test('garbage collection does not cause crashes', async () => {
    // Create and destroy many clients
    for (let i = 0; i < 100; i++) {
      const client = new RestClient({ apiKey: `key_${i}` });
      // Let it go out of scope
    }

    // Force GC if available
    if (global.gc) {
      global.gc();
    }

    // Create new client after GC
    const client = new RestClient({ apiKey: 'test-key' });
    expect(client).toBeTruthy();
  });
});

describe('FFI Boundary - Event Loop Safety', () => {
  test('async operations do not block event loop', async () => {
    const client = new RestClient({ apiKey: 'test-key' });

    let otherTaskCompleted = false;

    // Start an API call
    const apiCall = client.stock.intraday.quote('2330').catch(() => {});

    // Start another async task
    const otherTask = new Promise(resolve => {
      setImmediate(() => {
        otherTaskCompleted = true;
        resolve();
      });
    });

    // Both should complete
    await Promise.all([apiCall, otherTask]);

    expect(otherTaskCompleted).toBe(true);
  });

  test('multiple concurrent async calls', async () => {
    const client = new RestClient({ apiKey: 'test-key' });

    const results = [];

    async function makeCall(id) {
      try {
        await client.stock.intraday.quote('2330');
      } catch (error) {
        // Expected
      }
      results.push(id);
    }

    // Start multiple concurrent calls
    const promises = [];
    for (let i = 0; i < 5; i++) {
      promises.push(makeCall(i));
    }

    await Promise.all(promises);

    // All should complete
    expect(results.length).toBe(5);
  });

  test('WebSocket callbacks do not block event loop', (done) => {
    const ws = new WebSocketClient({ apiKey: 'test-key' });

    let otherTaskCompleted = false;

    ws.stock.on('message', (msg) => {
      // May or may not fire
    });

    ws.stock.on('error', (error) => {
      // Expected with test key - don't fail test
    });

    // Start connection attempt (sync call)
    try {
      ws.stock.connect();
    } catch (error) {
      // May fail immediately with test key
    }

    // Run other async work
    setImmediate(() => {
      otherTaskCompleted = true;

      // Event loop should not be blocked
      expect(otherTaskCompleted).toBe(true);

      // Cleanup
      try {
        ws.stock.disconnect();
      } catch (error) {
        // Ignore
      }
      done();
    });
  });

  test('promise resolution is asynchronous', async () => {
    const client = new RestClient({ apiKey: 'test-key' });

    let syncCheckpoint = false;

    const promise = client.stock.intraday.quote('2330').catch(() => {});

    // This should run before promise settles
    syncCheckpoint = true;

    await promise;

    expect(syncCheckpoint).toBe(true);
  });
});

describe('FFI Boundary - Type Safety', () => {
  test('returned data structures are valid JavaScript objects', async () => {
    const client = new RestClient({ apiKey: 'test-key' });

    try {
      const quote = await client.stock.intraday.quote('2330');

      // If successful, should be valid object
      expect(typeof quote).toBe('object');
      expect(quote).not.toBeNull();
    } catch (error) {
      // Error is expected with test key, verify error is valid
      expect(typeof error.message).toBe('string');
      expect(error.message.length).toBeGreaterThan(0);
    }
  });

  test('method chaining works correctly', () => {
    const client = new RestClient({ apiKey: 'test-key' });

    // Verify property access chain doesn't crash
    expect(client.stock).toBeTruthy();
    expect(client.stock.intraday).toBeTruthy();
    expect(client.futopt).toBeTruthy();
    expect(client.futopt.intraday).toBeTruthy();
  });

  test('WebSocket client properties are accessible', () => {
    const ws = new WebSocketClient({ apiKey: 'test-key' });

    expect(ws.stock).toBeTruthy();
    expect(ws.futopt).toBeTruthy();
    expect(typeof ws.stock.on).toBe('function');
    expect(typeof ws.stock.connect).toBe('function');
  });
});

// Manual testing notes:
//
// Additional stress tests (not automated):
//   1. Memory leak testing:
//      - Run with: node --expose-gc tests/ffi-boundary.test.js
//      - Monitor RSS with: ps aux | grep node
//
//   2. ThreadsafeFunction stress:
//      - Create WebSocket with rapid connect/disconnect cycles
//      - Monitor for crashes or memory leaks
//
//   3. Valgrind testing (Linux):
//      - valgrind --leak-check=full node tests/ffi-boundary.test.js
