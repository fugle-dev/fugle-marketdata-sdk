/**
 * The package's `main` is the hand-written `main.js`, which re-exports the
 * generated `index.js` and adds the 1.x compatibility layer (#307). Both
 * CommonJS and ESM imports must see every export of the generated file.
 */

const { spawnSync } = require('child_process');
const path = require('path');
const { pathToFileURL } = require('url');

const PKG = path.resolve(__dirname, '..');
const pkg = require('../package.json');

/** The names the generated index.js exports, as an ESM import must see them. */
const GENERATED = Object.keys(require('../index.js')).sort();

test('package.json points main at main.js and publishes it', () => {
  expect(pkg.main).toBe('main.js');
  expect(pkg.files).toEqual(expect.arrayContaining(['main.js', 'index.js', 'index.d.ts']));
});

test('require() of the package returns the generated exports, with connect() wrapped', () => {
  const main = require('..');
  expect(Object.keys(main).sort()).toEqual(GENERATED);
  expect(main).toBe(require('../index.js'));
  expect(main.StockWebSocketClient.prototype.connect.name).toBe('connect');
});

test('an ESM import sees every named export and the default export', () => {
  const url = pathToFileURL(path.join(PKG, pkg.main)).href;
  const script = `
    import * as ns from ${JSON.stringify(url)};
    import def, { WebSocketClient, RestClient } from ${JSON.stringify(url)};
    // Node >= 23 also exposes the whole of a CommonJS module as 'module.exports'.
    const named = Object.keys(ns).filter((name) => name !== 'default' && name !== 'module.exports').sort();
    console.log(JSON.stringify({
      named,
      sameDefault: def.WebSocketClient === WebSocketClient && def.RestClient === RestClient,
      wrapped: new WebSocketClient({ apiKey: 'k' }).stock.connect === def.StockWebSocketClient.prototype.connect,
    }));
  `;
  const result = spawnSync(process.execPath, ['--input-type=module', '-e', script], { cwd: PKG, encoding: 'utf8' });
  expect(result.stderr).toBe('');
  expect(JSON.parse(result.stdout)).toEqual({ named: GENERATED, sameDefault: true, wrapped: true });
});
