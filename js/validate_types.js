/**
 * Runtime Type Validation Script
 *
 * Validates that TypeScript interfaces in index.d.ts match the actual JSON
 * structure returned by the Rust core via REST API.
 *
 * Usage:
 *   FUGLE_API_KEY=your_api_key node validate_types.js
 *
 * If FUGLE_API_KEY is not set, the script will skip validation and exit successfully.
 */

const { RestClient } = require('./');

const API_KEY = process.env.FUGLE_API_KEY;
if (!API_KEY) {
  console.log('SKIP: FUGLE_API_KEY not set');
  console.log('To run validation, set FUGLE_API_KEY environment variable');
  process.exit(0);
}

// ============================================================================
// Expected fields from TypeScript interfaces in index.d.ts
// ============================================================================

// From QuoteResponse interface
const QUOTE_REQUIRED_FIELDS = [
  'date', 'symbol', 'bids', 'asks',
  'isLimitDownPrice', 'isLimitUpPrice', 'isLimitDownBid', 'isLimitUpBid',
  'isLimitDownAsk', 'isLimitUpAsk', 'isLimitDownHalt', 'isLimitUpHalt',
  'isTrial', 'isDelayedOpen', 'isDelayedClose', 'isContinuous', 'isOpen', 'isClose'
];

const QUOTE_OPTIONAL_FIELDS = [
  'type', 'exchange', 'market', 'name',
  'openPrice', 'openTime', 'highPrice', 'highTime', 'lowPrice', 'lowTime',
  'closePrice', 'closeTime', 'lastPrice', 'lastSize', 'avgPrice',
  'change', 'changePercent', 'amplitude',
  'total', 'lastTrade', 'lastTrial', 'tradingHalt', 'lastUpdated'
];

// From TickerResponse interface
const TICKER_REQUIRED_FIELDS = [
  'date', 'symbol',
  'canDayTrade', 'canBuyDayTrade', 'canBelowFlatMarginShortSell', 'canBelowFlatSBLShortSell',
  'isAttention', 'isDisposition', 'isUnusuallyRecommended', 'isSpecificAbnormally', 'isNewlyCompiled'
];

const TICKER_OPTIONAL_FIELDS = [
  'type', 'exchange', 'market', 'name', 'nameEn', 'industry', 'securityType',
  'referencePrice', 'limitUpPrice', 'limitDownPrice', 'previousClose',
  'matchingInterval', 'securityStatus', 'boardLot', 'tradingCurrency',
  'exercisePrice', 'exercisedVolume', 'cancelledVolume', 'remainingVolume',
  'exerciseRatio', 'capPrice', 'floorPrice', 'maturityDate',
  'openTime', 'closeTime'
];

// From CandlesResponse interface
const CANDLES_REQUIRED_FIELDS = ['date', 'symbol', 'data'];
const CANDLES_OPTIONAL_FIELDS = ['type', 'exchange', 'market', 'timeframe'];

// From TradesResponse interface
const TRADES_REQUIRED_FIELDS = ['date', 'symbol', 'data'];
const TRADES_OPTIONAL_FIELDS = ['type', 'exchange', 'market'];

// From VolumesResponse interface
const VOLUMES_REQUIRED_FIELDS = ['date', 'symbol', 'data'];
const VOLUMES_OPTIONAL_FIELDS = ['type', 'exchange', 'market'];

// ============================================================================
// Validation Functions
// ============================================================================

function validateRequiredFields(obj, requiredFields, typeName) {
  const missing = requiredFields.filter(f => !(f in obj));
  if (missing.length > 0) {
    console.error(`  FAIL: ${typeName} missing required fields: ${missing.join(', ')}`);
    return false;
  }
  return true;
}

function warnExtraFields(obj, knownFields, typeName) {
  const extra = Object.keys(obj).filter(f => !knownFields.includes(f));
  if (extra.length > 0) {
    console.warn(`  WARN: ${typeName} has extra fields not in interface: ${extra.join(', ')}`);
  }
}

function validateTotalStats(total) {
  if (!total) return true;

  const requiredFields = ['tradeValue', 'tradeVolume'];
  const optionalFields = ['tradeVolumeAtBid', 'tradeVolumeAtAsk', 'transaction', 'time'];
  const allFields = [...requiredFields, ...optionalFields];

  const missing = requiredFields.filter(f => !(f in total));
  if (missing.length > 0) {
    console.error(`  FAIL: TotalStats missing required fields: ${missing.join(', ')}`);
    return false;
  }

  warnExtraFields(total, allFields, 'TotalStats');
  return true;
}

function validatePriceLevel(level, index) {
  if (typeof level.price !== 'number') {
    console.error(`  FAIL: PriceLevel[${index}].price is not a number`);
    return false;
  }
  if (typeof level.size !== 'number') {
    console.error(`  FAIL: PriceLevel[${index}].size is not a number`);
    return false;
  }
  return true;
}

function validateIntradayCandle(candle, index) {
  const requiredFields = ['open', 'high', 'low', 'close', 'volume', 'time'];
  const missing = requiredFields.filter(f => !(f in candle));
  if (missing.length > 0) {
    console.error(`  FAIL: IntradayCandle[${index}] missing: ${missing.join(', ')}`);
    return false;
  }
  return true;
}

function validateTrade(trade, index) {
  const requiredFields = ['price', 'size', 'time'];
  const missing = requiredFields.filter(f => !(f in trade));
  if (missing.length > 0) {
    console.error(`  FAIL: Trade[${index}] missing: ${missing.join(', ')}`);
    return false;
  }
  return true;
}

function validateVolumeAtPrice(vol, index) {
  const requiredFields = ['price', 'volume'];
  const missing = requiredFields.filter(f => !(f in vol));
  if (missing.length > 0) {
    console.error(`  FAIL: VolumeAtPrice[${index}] missing: ${missing.join(', ')}`);
    return false;
  }
  return true;
}

// ============================================================================
// Main Validation Logic
// ============================================================================

async function validateTypes() {
  const client = new RestClient(API_KEY);
  let allPassed = true;
  const symbol = '2330'; // TSMC - widely traded stock

  // -------------------------------------------------------------------------
  // Validate QuoteResponse
  // -------------------------------------------------------------------------
  console.log('\nValidating QuoteResponse...');
  try {
    const quote = await client.stock.intraday.quote(symbol);

    if (!validateRequiredFields(quote, QUOTE_REQUIRED_FIELDS, 'QuoteResponse')) {
      allPassed = false;
    }
    warnExtraFields(quote, [...QUOTE_REQUIRED_FIELDS, ...QUOTE_OPTIONAL_FIELDS], 'QuoteResponse');

    // Validate nested structures
    if (quote.total && !validateTotalStats(quote.total)) {
      allPassed = false;
    }

    // Validate bids array
    if (Array.isArray(quote.bids) && quote.bids.length > 0) {
      if (!validatePriceLevel(quote.bids[0], 0)) {
        allPassed = false;
      } else {
        console.log('  PASS: QuoteResponse.bids structure correct');
      }
    }

    // Validate asks array
    if (Array.isArray(quote.asks) && quote.asks.length > 0) {
      if (!validatePriceLevel(quote.asks[0], 0)) {
        allPassed = false;
      } else {
        console.log('  PASS: QuoteResponse.asks structure correct');
      }
    }

    console.log('  PASS: QuoteResponse validation complete');
  } catch (err) {
    console.error(`  ERROR: Failed to fetch quote: ${err.message}`);
    allPassed = false;
  }

  // -------------------------------------------------------------------------
  // Validate TickerResponse
  // -------------------------------------------------------------------------
  console.log('\nValidating TickerResponse...');
  try {
    const ticker = await client.stock.intraday.ticker(symbol);

    if (!validateRequiredFields(ticker, TICKER_REQUIRED_FIELDS, 'TickerResponse')) {
      allPassed = false;
    }
    warnExtraFields(ticker, [...TICKER_REQUIRED_FIELDS, ...TICKER_OPTIONAL_FIELDS], 'TickerResponse');

    console.log('  PASS: TickerResponse validation complete');
  } catch (err) {
    console.error(`  ERROR: Failed to fetch ticker: ${err.message}`);
    allPassed = false;
  }

  // -------------------------------------------------------------------------
  // Validate CandlesResponse
  // -------------------------------------------------------------------------
  console.log('\nValidating CandlesResponse...');
  try {
    const candles = await client.stock.intraday.candles(symbol, '5');

    if (!validateRequiredFields(candles, CANDLES_REQUIRED_FIELDS, 'CandlesResponse')) {
      allPassed = false;
    }
    warnExtraFields(candles, [...CANDLES_REQUIRED_FIELDS, ...CANDLES_OPTIONAL_FIELDS], 'CandlesResponse');

    // Validate candle data array
    if (Array.isArray(candles.data) && candles.data.length > 0) {
      if (!validateIntradayCandle(candles.data[0], 0)) {
        allPassed = false;
      } else {
        console.log('  PASS: CandlesResponse.data structure correct');
      }
    }

    console.log('  PASS: CandlesResponse validation complete');
  } catch (err) {
    console.error(`  ERROR: Failed to fetch candles: ${err.message}`);
    allPassed = false;
  }

  // -------------------------------------------------------------------------
  // Validate TradesResponse
  // -------------------------------------------------------------------------
  console.log('\nValidating TradesResponse...');
  try {
    const trades = await client.stock.intraday.trades(symbol);

    if (!validateRequiredFields(trades, TRADES_REQUIRED_FIELDS, 'TradesResponse')) {
      allPassed = false;
    }
    warnExtraFields(trades, [...TRADES_REQUIRED_FIELDS, ...TRADES_OPTIONAL_FIELDS], 'TradesResponse');

    // Validate trade data array
    if (Array.isArray(trades.data) && trades.data.length > 0) {
      if (!validateTrade(trades.data[0], 0)) {
        allPassed = false;
      } else {
        console.log('  PASS: TradesResponse.data structure correct');
      }
    }

    console.log('  PASS: TradesResponse validation complete');
  } catch (err) {
    console.error(`  ERROR: Failed to fetch trades: ${err.message}`);
    allPassed = false;
  }

  // -------------------------------------------------------------------------
  // Validate VolumesResponse
  // -------------------------------------------------------------------------
  console.log('\nValidating VolumesResponse...');
  try {
    const volumes = await client.stock.intraday.volumes(symbol);

    if (!validateRequiredFields(volumes, VOLUMES_REQUIRED_FIELDS, 'VolumesResponse')) {
      allPassed = false;
    }
    warnExtraFields(volumes, [...VOLUMES_REQUIRED_FIELDS, ...VOLUMES_OPTIONAL_FIELDS], 'VolumesResponse');

    // Validate volume data array
    if (Array.isArray(volumes.data) && volumes.data.length > 0) {
      if (!validateVolumeAtPrice(volumes.data[0], 0)) {
        allPassed = false;
      } else {
        console.log('  PASS: VolumesResponse.data structure correct');
      }
    }

    console.log('  PASS: VolumesResponse validation complete');
  } catch (err) {
    console.error(`  ERROR: Failed to fetch volumes: ${err.message}`);
    allPassed = false;
  }

  // -------------------------------------------------------------------------
  // Summary
  // -------------------------------------------------------------------------
  console.log('\n' + '='.repeat(60));
  if (allPassed) {
    console.log('SUCCESS: All TypeScript definitions match runtime JSON');
    process.exit(0);
  } else {
    console.error('FAILURE: TypeScript definitions do not match runtime JSON');
    console.error('Please update index.d.ts to match the actual API response');
    process.exit(1);
  }
}

// Run validation
validateTypes().catch(err => {
  console.error('Unexpected error:', err);
  process.exit(1);
});
