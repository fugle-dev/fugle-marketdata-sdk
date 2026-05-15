/**
 * Jest Configuration for @fugle/marketdata SDK
 *
 * API compatibility tests run without API key.
 * Integration tests are automatically skipped when FUGLE_API_KEY is not set.
 */
module.exports = {
  testEnvironment: 'node',
  testMatch: ['**/tests/**/*.test.js'],
  testTimeout: 30000,  // 30s for integration tests with network calls
  verbose: true,
  // Collect coverage from source files
  collectCoverageFrom: [
    'index.js',
  ],
  // Coverage thresholds (optional, can enable later)
  // coverageThreshold: {
  //   global: {
  //     branches: 50,
  //     functions: 50,
  //     lines: 50,
  //     statements: 50,
  //   },
  // },
};
