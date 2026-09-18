/**
 * Jest Configuration for @fugle/marketdata SDK
 *
 * API compatibility tests run without API key.
 * Integration tests are automatically skipped when FUGLE_API_KEY is not set.
 */
module.exports = {
  testEnvironment: 'node',
  // Both extensions. config.test.ts is TypeScript so ts-jest type-checks it
  // against index.d.ts and a type error fails the suite; it had never run
  // while this pattern took `.test.js` only (#170). scripts/check-ci-coverage.py
  // verifies every tests/**/*.test.* file matches this pattern.
  testMatch: ['**/tests/**/*.test.[jt]s'],
  transform: {
    '^.+\\.ts$': ['ts-jest', { tsconfig: 'tsconfig.test.json' }],
  },
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
