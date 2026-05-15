/**
 * Configuration tests for v0.3.0 options-based constructors
 *
 * Tests validate:
 * 1. Options object constructor works
 * 2. Exactly-one-auth validation
 * 3. Config validation for reconnect/healthCheck
 * 4. TypeScript type inference
 */

// Note: These tests require the built native module
// Run: npm run build && npm test

import { RestClient, WebSocketClient } from '../index';
import type { RestClientOptions, WebSocketClientOptions, ReconnectOptions, HealthCheckOptions } from '../index';

describe('RestClient constructor', () => {
  describe('authentication', () => {
    it('accepts apiKey auth', () => {
      const client = new RestClient({ apiKey: 'test-key' });
      expect(client).toBeDefined();
      expect(client.stock).toBeDefined();
    });

    it('accepts bearerToken auth', () => {
      const client = new RestClient({ bearerToken: 'test-token' });
      expect(client).toBeDefined();
    });

    it('accepts sdkToken auth', () => {
      const client = new RestClient({ sdkToken: 'test-sdk-token' });
      expect(client).toBeDefined();
    });

    it('accepts baseUrl option', () => {
      const client = new RestClient({
        apiKey: 'test-key',
        baseUrl: 'https://custom.api'
      });
      expect(client).toBeDefined();
    });

    it('throws error when no auth provided', () => {
      expect(() => {
        // @ts-expect-error - Testing runtime validation for JS users
        new RestClient({});
      }).toThrow('exactly one');
    });

    it('throws error when multiple auth provided', () => {
      expect(() => {
        // @ts-expect-error - Testing runtime validation for JS users
        new RestClient({ apiKey: 'key', bearerToken: 'token' });
      }).toThrow('exactly one');
    });
  });
});

describe('WebSocketClient constructor', () => {
  describe('authentication', () => {
    it('accepts apiKey auth', () => {
      const ws = new WebSocketClient({ apiKey: 'test-key' });
      expect(ws).toBeDefined();
      expect(ws.stock).toBeDefined();
      expect(ws.futopt).toBeDefined();
    });

    it('accepts bearerToken auth', () => {
      const ws = new WebSocketClient({ bearerToken: 'test-token' });
      expect(ws).toBeDefined();
    });

    it('throws error when no auth provided', () => {
      expect(() => {
        // @ts-expect-error - Testing runtime validation for JS users
        new WebSocketClient({});
      }).toThrow('exactly one');
    });
  });

  describe('reconnect config', () => {
    it('accepts reconnect options', () => {
      const ws = new WebSocketClient({
        apiKey: 'test-key',
        reconnect: {
          maxAttempts: 10,
          initialDelayMs: 2000,
          maxDelayMs: 120000
        }
      });
      expect(ws).toBeDefined();
    });

    it('accepts partial reconnect options', () => {
      const ws = new WebSocketClient({
        apiKey: 'test-key',
        reconnect: { maxAttempts: 3 }
      });
      expect(ws).toBeDefined();
    });

    it('throws error for invalid maxAttempts', () => {
      expect(() => {
        new WebSocketClient({
          apiKey: 'test-key',
          reconnect: { maxAttempts: 0 }
        });
      }).toThrow();
    });

    it('throws error for invalid initialDelayMs', () => {
      expect(() => {
        new WebSocketClient({
          apiKey: 'test-key',
          reconnect: { initialDelayMs: 50 } // Below 100ms minimum
        });
      }).toThrow();
    });
  });

  describe('healthCheck config', () => {
    it('accepts healthCheck options', () => {
      const ws = new WebSocketClient({
        apiKey: 'test-key',
        healthCheck: {
          enabled: true,
          pingInterval: 20000,
          maxMissedPongs: 3
        }
      });
      expect(ws).toBeDefined();
    });

    it('accepts partial healthCheck options', () => {
      const ws = new WebSocketClient({
        apiKey: 'test-key',
        healthCheck: { enabled: true }
      });
      expect(ws).toBeDefined();
    });

    it('throws error for invalid pingInterval', () => {
      expect(() => {
        new WebSocketClient({
          apiKey: 'test-key',
          healthCheck: { pingInterval: 1000 } // Below 5000ms minimum
        });
      }).toThrow();
    });
  });

  describe('combined config', () => {
    it('accepts both reconnect and healthCheck', () => {
      const ws = new WebSocketClient({
        apiKey: 'test-key',
        reconnect: { maxAttempts: 10 },
        healthCheck: { enabled: true, pingInterval: 15000 }
      });
      expect(ws).toBeDefined();
    });
  });
});

describe('TypeScript type inference', () => {
  it('RestClientOptions type works correctly', () => {
    // These should compile without errors
    const opts1: RestClientOptions = { apiKey: 'key' };
    const opts2: RestClientOptions = { bearerToken: 'token' };
    const opts3: RestClientOptions = { sdkToken: 'sdk', baseUrl: 'url' };

    expect(opts1.apiKey).toBe('key');
    expect(opts2.bearerToken).toBe('token');
    expect(opts3.sdkToken).toBe('sdk');
  });

  it('WebSocketClientOptions type works correctly', () => {
    const opts: WebSocketClientOptions = {
      apiKey: 'key',
      reconnect: { maxAttempts: 5 },
      healthCheck: { enabled: false }
    };

    expect(opts.apiKey).toBe('key');
    expect(opts.reconnect?.maxAttempts).toBe(5);
    expect(opts.healthCheck?.enabled).toBe(false);
  });
});
