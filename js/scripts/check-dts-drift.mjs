#!/usr/bin/env node
/**
 * check-dts-drift.mjs
 *
 * `js/index.d.ts` describes the *wire* shape of REST responses (the SDK now
 * passes server JSON through untouched instead of marshalling it through a
 * napi-rs struct). That means the interfaces in `index.d.ts` are documentation,
 * not compiler-checked bindings, and can silently drift from the Rust models
 * that were audited against real payloads.
 *
 * This script re-derives the wire field names from `core/src/models/**\/*.rs`
 * (the `#[serde(rename = "...")]` attribute, or the bare field name when
 * there's no rename — these structs do NOT use `rename_all`, so an unrenamed
 * field's wire name is its literal Rust name) and diffs them against the
 * matching `export interface` in `js/index.d.ts`.
 *
 * It only compares FIELD NAME SETS, not types or optionality — that's enough
 * to catch the class of bug this guards against (a renamed/typo'd/missing
 * field), without the parser needing to understand TypeScript types or serde
 * attribute value types.
 *
 * Usage: node scripts/check-dts-drift.mjs   (run from js/, or anywhere — paths
 * are resolved relative to this file)
 */

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const JS_DIR = path.resolve(__dirname, '..');
const REPO_ROOT = path.resolve(JS_DIR, '..');
const DTS_PATH = path.join(JS_DIR, 'index.d.ts');
const MODELS_DIR = path.join(REPO_ROOT, 'core', 'src', 'models');

// ============================================================================
// 1. Parse core/src/models/**/*.rs -> { StructName: [wireFieldName, ...] }
// ============================================================================

function listRustFiles(dir) {
  const out = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      out.push(...listRustFiles(full));
    } else if (entry.isFile() && entry.name.endsWith('.rs')) {
      out.push(full);
    }
  }
  return out;
}

// Given the text between a struct's `{` and its matching `}`, return the wire
// field names in declaration order. Handles multi-line `#[serde(...)]`
// attributes (including a `rename = "..."` buried among other keys).
function parseStructFields(body) {
  const fields = [];
  const lines = body.split('\n');
  let pendingAttr = '';

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i].trim();

    if (line === '' || line.startsWith('///') || line.startsWith('//')) {
      continue;
    }

    if (line.startsWith('#[')) {
      // Accumulate until the attribute's `[...]` brackets balance. These
      // attributes never contain nested `[`/`]` (only `(`, `)`, and quoted
      // strings), so simple bracket counting is safe.
      let attrText = line;
      let depth = (line.match(/\[/g) || []).length - (line.match(/\]/g) || []).length;
      while (depth > 0 && i + 1 < lines.length) {
        i++;
        attrText += '\n' + lines[i];
        depth += (lines[i].match(/\[/g) || []).length - (lines[i].match(/\]/g) || []).length;
      }
      pendingAttr += attrText + '\n';
      continue;
    }

    const fieldMatch = line.match(/^pub\s+(\w+)\s*:/);
    if (fieldMatch) {
      const fieldName = fieldMatch[1];
      const renameMatch = pendingAttr.match(/rename\s*=\s*"([^"]+)"/);
      fields.push(renameMatch ? renameMatch[1] : fieldName);
      pendingAttr = '';
      continue;
    }

    // Anything else (e.g. a stray line inside a doc example) doesn't carry a
    // pending attribute forward.
    pendingAttr = '';
  }

  return fields;
}

function findMatchingBrace(text, openBraceIdx) {
  let depth = 0;
  for (let i = openBraceIdx; i < text.length; i++) {
    if (text[i] === '{') depth++;
    else if (text[i] === '}') {
      depth--;
      if (depth === 0) return i;
    }
  }
  return -1;
}

function extractRustStructs(fileText) {
  const structs = {};
  const re = /pub struct (\w+)/g;
  let m;
  while ((m = re.exec(fileText)) !== null) {
    const name = m[1];
    const searchFrom = re.lastIndex;
    const braceIdx = fileText.indexOf('{', searchFrom);
    const semiIdx = fileText.indexOf(';', searchFrom);
    if (braceIdx === -1) continue; // no body found at all
    if (semiIdx !== -1 && semiIdx < braceIdx) continue; // tuple/unit struct
    const endIdx = findMatchingBrace(fileText, braceIdx);
    if (endIdx === -1) continue;
    const body = fileText.slice(braceIdx + 1, endIdx);
    // Last definition wins if a name repeats across files — shouldn't happen,
    // but don't silently merge if it does.
    structs[name] = parseStructFields(body);
  }
  return structs;
}

function loadRustStructs() {
  const structs = {};
  for (const file of listRustFiles(MODELS_DIR)) {
    const text = fs.readFileSync(file, 'utf8');
    const found = extractRustStructs(text);
    for (const [name, fields] of Object.entries(found)) {
      structs[name] = fields;
    }
  }
  return structs;
}

// ============================================================================
// 2. Parse js/index.d.ts -> { InterfaceName: [fieldName, ...] }
// ============================================================================

function parseInterfaceFields(body) {
  const fields = [];
  for (const raw of body.split('\n')) {
    const line = raw.trim();
    if (
      line === '' ||
      line.startsWith('//') ||
      line.startsWith('/*') ||
      line.startsWith('*')
    ) {
      continue;
    }
    // Matches `foo: Type;` and `foo?: Type;`. Method signatures like
    // `foo(a: string): Promise<X>` don't match (no `:`/`?:` immediately after
    // the identifier), so client interfaces naturally yield an empty field
    // list rather than false positives.
    const fieldMatch = line.match(/^(\w+)\??\s*:/);
    if (fieldMatch) fields.push(fieldMatch[1]);
  }
  return fields;
}

function extractDtsInterfaces(fileText) {
  const interfaces = {};
  const re = /export interface (\w+)/g;
  let m;
  while ((m = re.exec(fileText)) !== null) {
    const name = m[1];
    const braceIdx = fileText.indexOf('{', re.lastIndex);
    if (braceIdx === -1) continue;
    const endIdx = findMatchingBrace(fileText, braceIdx);
    if (endIdx === -1) continue;
    const body = fileText.slice(braceIdx + 1, endIdx);
    interfaces[name] = parseInterfaceFields(body);
  }
  return interfaces;
}

// ============================================================================
// 3. Interface <-> struct mapping, and interfaces intentionally left unmapped
// ============================================================================

// `js/index.d.ts` interface name -> core model struct name.
const INTERFACE_TO_STRUCT = {
  PriceLevel: 'PriceLevel',
  TradeInfo: 'TradeInfo',
  TotalStats: 'TotalStats',
  TradingHalt: 'TradingHalt',
  QuoteResponse: 'Quote',
  TickerResponse: 'Ticker',
  IntradayCandle: 'IntradayCandle',
  CandlesResponse: 'IntradayCandlesResponse',
  Trade: 'Trade',
  TradesResponse: 'TradesResponse',
  VolumeAtPrice: 'VolumeAtPrice',
  VolumesResponse: 'VolumesResponse',
  FutOptProduct: 'Product',
  ProductsResponse: 'ProductsResponse',
  FutOptPriceLimits: 'FutOptPriceLimits',
  FutOptTotalStats: 'FutOptTotalStats',
  FutOptTradingHalt: 'FutOptTradingHalt',
  FutOptQuoteResponse: 'FutOptQuote',
  FutOptTickerResponse: 'FutOptTicker',
  HistoricalCandle: 'HistoricalCandle',
  HistoricalCandlesResponse: 'HistoricalCandlesResponse',
  StatsResponse: 'StatsResponse',
  SnapshotQuote: 'SnapshotQuote',
  SnapshotQuotesResponse: 'SnapshotQuotesResponse',
  Mover: 'Mover',
  MoversResponse: 'MoversResponse',
  Active: 'Active',
  ActivesResponse: 'ActivesResponse',
  SmaDataPoint: 'SmaDataPoint',
  SmaResponse: 'SmaResponse',
  RsiDataPoint: 'RsiDataPoint',
  RsiResponse: 'RsiResponse',
  KdjDataPoint: 'KdjDataPoint',
  KdjResponse: 'KdjResponse',
  MacdDataPoint: 'MacdDataPoint',
  MacdResponse: 'MacdResponse',
  BbDataPoint: 'BbDataPoint',
  BbResponse: 'BbResponse',
  CapitalChange: 'CapitalChange',
  CapitalChangesResponse: 'CapitalChangesResponse',
  EtfHoldingComponent: 'EtfHoldingComponent',
  EtfHoldingsEntry: 'EtfHoldingsEntry',
  EtfHoldingsResponse: 'EtfHoldingsResponse',
  InstitutionalInvestorTrade: 'InstitutionalInvestorTrade',
  InstitutionalTradesEntry: 'InstitutionalTradesEntry',
  InstitutionalTradesResponse: 'InstitutionalTradesResponse',
  DirectorHolding: 'DirectorHolding',
  DirectorHoldingsEntry: 'DirectorHoldingsEntry',
  DirectorHoldingsResponse: 'DirectorHoldingsResponse',
  TdccDistributionLevel: 'TdccDistributionLevel',
  TdccDistributionEntry: 'TdccDistributionEntry',
  TdccDistributionResponse: 'TdccDistributionResponse',
  Dividend: 'Dividend',
  DividendsResponse: 'DividendsResponse',
  ListingApplicant: 'ListingApplicant',
  ListingApplicantsResponse: 'ListingApplicantsResponse',
  FutOptHistoricalCandle: 'FutOptHistoricalCandle',
  FutOptHistoricalCandlesResponse: 'FutOptHistoricalCandlesResponse',
  FutOptDailyData: 'FutOptDailyData',
  FutOptDailyResponse: 'FutOptDailyResponse',
};

// Interfaces that are intentionally NOT checked against a core struct,
// with the reason why. Keep this list honest — anything not here and not in
// INTERFACE_TO_STRUCT fails the build (see "unmapped interface" below).
const INTENTIONALLY_UNMAPPED = {
  // Envelope types: the server wraps a `data` array in metadata, but the
  // enclosing endpoint's own response struct differs per params, and these
  // envelopes are simple enough to eyeball. `data`'s element type IS checked
  // (via FutOptTickerResponse/TickerResponse).
  TickersResponse: 'envelope type, not a core struct',
  FutOptTickersResponse: 'envelope type, not a core struct',

  // Method-only interfaces (napi-rs client wrappers' hand-written TS
  // counterparts). No data fields to check — see also the empty-array short
  // circuit in parseInterfaceFields.
  StockHistoricalClient: 'method-only interface, no data fields',
  StockSnapshotClient: 'method-only interface, no data fields',
  StockTechnicalClient: 'method-only interface, no data fields',
  StockCorporateActionsClient: 'method-only interface, no data fields',
  FutOptHistoricalClient: 'method-only interface, no data fields',

  // WebSocket-side types. This script audits REST response shapes (the
  // raw-JSON-passthrough change); WebSocket framing is untouched by that
  // change and out of scope here.
  WebSocketMessage: 'WebSocket type, out of scope for the REST passthrough audit',
  StockChannel: 'type alias, not an interface with fields',
  FutOptChannel: 'type alias, not an interface with fields',
  StockSubscribeOptions: 'WebSocket param type, not a core response struct',
  FutOptSubscribeOptions: 'WebSocket param type, not a core response struct',
  UnsubscribeOptions: 'WebSocket param type, not a core response struct',
  WebSocketEventMap: 'callback signature map, no data fields',
  WebSocketPingParams: 'WebSocket param type, not a core response struct',
  WebSocketAuthData: 'server auth frame data, passed through verbatim',
  WebSocketDisconnectEvent: 'WebSocket event argument built by the binding',
  WebSocketReconnectEvent: 'WebSocket event argument built by the binding',
  WebSocketError: 'WebSocket event argument built by the binding',

  // JS-side request param / client-option types — inputs the caller
  // constructs, not server response shapes.
  ContractType: 'type alias, not an interface with fields',
  FutOptType: 'type alias, not an interface with fields',
  DirectorHoldingsParams: 'request param type, not a core response struct',
  EtfHoldingsParams: 'request param type, not a core response struct',
  HealthCheckOptions: 'client config type, not a core response struct',
  InstitutionalTradesParams: 'request param type, not a core response struct',
  ReconnectOptions: 'client config type, not a core response struct',
  RestClientOptions: 'client config type, not a core response struct',
  StockIntradayQuoteParams: 'request param type, not a core response struct',
  StreamingVersionOptions: 'client config type, not a core response struct',
  TdccDistributionParams: 'request param type, not a core response struct',
  WebSocketClientOptions: 'client config type, not a core response struct',
  WebSocketEvent: 'type alias, not an interface with fields',
};

// Per-interface fields that are known to be in the `.d.ts` but NOT (yet, or
// ever) on the mapped core struct, with the reason. Use sparingly — this is
// an escape hatch for cases where the wire response is known (from a real
// payload) to carry a field the core model doesn't model.
const KNOWN_EXTRA = {
  // The Fugle API returns `referencePrice` and `serial` on
  // `intraday/quote/{symbol}`. `core::models::quote::Quote` has carried both
  // since commit 9d7986f ("fix(core): Quote 補上 referencePrice 與
  // serial..."), so this entry is currently a no-op — kept so the check
  // survives if a future refactor drops either field from the struct again
  // without anyone noticing the API still sends it.
  QuoteResponse: ['referencePrice', 'serial'],
};

// ============================================================================
// 4. Diff
// ============================================================================

function main() {
  const rustStructs = loadRustStructs();
  const dtsText = fs.readFileSync(DTS_PATH, 'utf8');
  const dtsInterfaces = extractDtsInterfaces(dtsText);

  let failed = false;
  const problems = [];

  for (const [interfaceName, dtsFields] of Object.entries(dtsInterfaces)) {
    if (Object.prototype.hasOwnProperty.call(INTENTIONALLY_UNMAPPED, interfaceName)) {
      continue;
    }
    // `Rest*Params`: the legacy object-form request params, one per REST
    // method — inputs the caller constructs, not server response shapes.
    if (/^Rest\w+Params$/.test(interfaceName)) {
      continue;
    }

    const structName = INTERFACE_TO_STRUCT[interfaceName];
    if (!structName) {
      failed = true;
      problems.push(
        `UNMAPPED INTERFACE: "${interfaceName}" is not in INTERFACE_TO_STRUCT and not in ` +
          `INTENTIONALLY_UNMAPPED. Add it to one of them in scripts/check-dts-drift.mjs.`
      );
      continue;
    }

    const structFields = rustStructs[structName];
    if (!structFields) {
      failed = true;
      problems.push(
        `MISSING STRUCT: interface "${interfaceName}" maps to struct "${structName}", but no ` +
          `"pub struct ${structName}" was found under core/src/models/. Renamed or removed?`
      );
      continue;
    }

    const knownExtra = new Set(KNOWN_EXTRA[interfaceName] || []);
    const structSet = new Set(structFields);
    const dtsSet = new Set(dtsFields);

    const extraInDts = dtsFields.filter((f) => !structSet.has(f) && !knownExtra.has(f));
    const missingFromDts = structFields.filter((f) => !dtsSet.has(f));

    if (extraInDts.length > 0 || missingFromDts.length > 0) {
      failed = true;
      const lines = [`FIELD MISMATCH: interface "${interfaceName}" vs struct "${structName}"`];
      if (extraInDts.length > 0) {
        lines.push(`  in .d.ts but not in struct: ${extraInDts.join(', ')}`);
      }
      if (missingFromDts.length > 0) {
        lines.push(`  in struct but not in .d.ts: ${missingFromDts.join(', ')}`);
      }
      problems.push(lines.join('\n'));
    }
  }

  // Also flag mapping entries that reference a struct/interface that no
  // longer exists, so the table doesn't quietly rot the other direction.
  for (const [interfaceName, structName] of Object.entries(INTERFACE_TO_STRUCT)) {
    if (!dtsInterfaces[interfaceName]) {
      failed = true;
      problems.push(
        `STALE MAPPING: INTERFACE_TO_STRUCT has "${interfaceName}" -> "${structName}", but ` +
          `"${interfaceName}" is no longer an interface in js/index.d.ts.`
      );
    }
  }

  if (failed) {
    console.error('check-dts-drift: found js/index.d.ts drift from core/src/models/**/*.rs\n');
    console.error(problems.join('\n\n'));
    console.error(`\n${problems.length} problem(s) found.`);
    process.exit(1);
  }

  console.log(
    `check-dts-drift: OK (${Object.keys(INTERFACE_TO_STRUCT).length} interfaces checked, ` +
      `${Object.keys(INTENTIONALLY_UNMAPPED).length} intentionally unmapped)`
  );
}

main();
