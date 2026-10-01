/**
 * `subscribe()` / `unsubscribe()` refuse what they would not use (#294).
 *
 * The methods used to read the keys they knew and drop the rest, so
 * `afterHours: true` on the stock client, `intradayOddLot: 'true'` or a
 * number in `symbols` changed what was subscribed without a word. Now each is
 * a `TypeError`, thrown before the connection is looked at, so these run on
 * a client that never connects: a valid call gets "Not connected" instead.
 */
const { WebSocketClient } = require('../');

const ws = new WebSocketClient({ apiKey: 'test-key' });

function thrown(fn) {
  try {
    fn();
  } catch (e) {
    // `name`, not `instanceof`: jest's sandbox has its own `TypeError`.
    return { name: e.name, message: e.message };
  }
  throw new Error('expected a throw');
}

const typeError = (message) => ({ name: 'TypeError', message });
const notConnected = { name: 'Error', message: 'Not connected. Call connect() first.' };

const STOCK_KEYS = 'channel, symbol, symbols, intradayOddLot';
const ID_FORM = '(without channel the object takes id or ids; to unsubscribe by channel and symbol, add channel)';
const FUTOPT_KEYS = 'channel, symbol, symbols, afterHours';

describe('subscribe()', () => {
  test.each([
    ['stock', { channel: 'trades', symbol: '2330', afterHours: true },
      `subscribe(): unknown key 'afterHours': it is a futopt option, the stock client takes intradayOddLot (accepted: ${STOCK_KEYS})`],
    ['futopt', { channel: 'trades', symbol: 'TXFA6', intradayOddLot: true },
      `subscribe(): unknown key 'intradayOddLot': it is a stock option, the futopt client takes afterHours (accepted: ${FUTOPT_KEYS})`],
    ['stock', { channel: 'trades', symbol: '2330', oddLot: true },
      `subscribe(): unknown key 'oddLot' (accepted: ${STOCK_KEYS})`],
    ['futopt', { channel: 'trades', symbol: 'TXFA6', foo: 1 }, `subscribe(): unknown key 'foo' (accepted: ${FUTOPT_KEYS})`],
    ['stock', { channel: 'trades', symbol: '2330', intradayOddLot: 'true' }, 'subscribe(): intradayOddLot must be a boolean, got string'],
    ['futopt', { channel: 'trades', symbol: 'TXFA6', afterHours: 1 }, 'subscribe(): afterHours must be a boolean, got number 1'],
    ['stock', { channel: 'trades', symbol: 2330 }, 'subscribe(): symbol must be a string, got number 2330'],
    ['stock', { channel: 'trades', symbols: '2330' }, 'subscribe(): symbols must be an array of strings, got string'],
    ['stock', { channel: 'trades', symbols: ['2330', 2317] }, 'subscribe(): symbols[1] must be a string, got number'],
    ['stock', { channel: 1, symbol: '2330' }, 'subscribe(): channel must be a string, got number 1'],
    ['stock', 'trades', "subscribe() takes an object like { channel: 'trades', symbol: '2330' }, got string"],
  ])('%s %j', (product, options, message) => {
    expect(thrown(() => ws[product].subscribe(options))).toEqual(typeError(message));
  });

  test.each([
    ['no argument', () => ws.stock.subscribe(), 'got undefined'],
    ['a function', () => ws.stock.subscribe(() => {}), 'got function'],
  ])('%s', (_label, call, tail) => {
    expect(thrown(call)).toEqual(typeError(`subscribe() takes an object like { channel: 'trades', symbol: '2330' }, ${tail}`));
  });

  test('a function value is named by its key', () => {
    expect(thrown(() => ws.stock.subscribe({ channel: 'trades', symbol: '2330', onData: () => {} }))).toEqual(
      typeError(`subscribe(): unknown key 'onData' (accepted: ${STOCK_KEYS})`),
    );
  });

  test('a further argument', () => {
    expect(thrown(() => ws.stock.subscribe({ channel: 'trades', symbol: '2330' }, { intradayOddLot: true }))).toEqual(
      typeError('subscribe() takes one argument, got a further object one'),
    );
  });

  test.each([
    ['stock', { channel: 'trades', symbol: '2330', intradayOddLot: true }],
    ['stock', { channel: 'trades', symbols: ['2330'], intradayOddLot: undefined }],
    ['futopt', { channel: 'books', symbol: 'TXFA6', afterHours: false }],
    // null counts as not given (#294).
    ['futopt', { channel: 'books', symbol: 'TXFA6', afterHours: null, symbols: null }],
  ])('%s %j passes the checks', (product, options) => {
    expect(thrown(() => ws[product].subscribe(options, undefined))).toEqual(notConnected);
  });
});

describe('unsubscribe()', () => {
  const stockAll = `${STOCK_KEYS}, id, ids`;
  test.each([
    ['stock', { id: 'abc', foo: 1 }, `unsubscribe(): unknown key 'foo' ${ID_FORM}`],
    ['stock', { id: 'abc', symbol: '2330' }, `unsubscribe(): unknown key 'symbol' ${ID_FORM}`],
    ['futopt', { ids: ['abc'], afterHours: true }, `unsubscribe(): unknown key 'afterHours' ${ID_FORM}`],
    ['stock', { channel: 'trades', symbol: '2330', foo: 1 }, `unsubscribe(): unknown key 'foo' (accepted: ${stockAll})`],
    ['stock', { channel: 'trades', symbol: '2330', afterHours: true },
      `unsubscribe(): unknown key 'afterHours': it is a futopt option, the stock client takes intradayOddLot (accepted: ${stockAll})`],
    ['futopt', { channel: 'trades', symbol: 'TXFA6', afterHours: 'yes' }, 'unsubscribe(): afterHours must be a boolean, got string'],
    ['stock', { id: 1 }, 'unsubscribe(): id must be a string, got number 1'],
    ['stock', { ids: ['abc', 1] }, 'unsubscribe(): ids[1] must be a string, got number'],
    ['stock', 1, "unsubscribe() takes a subscription id or an object like { id: '...' } or { channel: 'trades', symbol: '2330' }, got number 1"],
  ])('%s %j', (product, options, message) => {
    expect(thrown(() => ws[product].unsubscribe(options))).toEqual(typeError(message));
  });

  test('a further argument', () => {
    expect(thrown(() => ws.futopt.unsubscribe('abc', 'def'))).toEqual(typeError('unsubscribe() takes one argument, got a further string one'));
  });

  test.each([
    ['stock', 'abc'],
    ['stock', { ids: ['abc', 'def'] }],
    ['futopt', { channel: 'trades', symbol: 'TXFA6', afterHours: true }],
    // A null id next to channel is not given, so not the 1005 of both.
    ['stock', { channel: 'trades', symbol: '2330', id: null }],
  ])('%s %j passes the checks', (product, options) => {
    expect(thrown(() => ws[product].unsubscribe(options))).toEqual(notConnected);
  });
});
