/**
 * TypeScript type definitions for @fugle/marketdata SDK
 *
 * This file contains all interface and type definitions for REST API responses
 * and WebSocket events. These types provide full IDE autocomplete support.
 */

// ============================================================================
// Common Types
// ============================================================================

/** Price level for order book (bid/ask) */
export interface PriceLevel {
  /** Price at this level */
  price: number;
  /** Size (volume) at this level */
  size: number;
}

/** Trade execution info */
export interface TradeInfo {
  /** Best bid price at trade time */
  bid?: number;
  /** Best ask price at trade time */
  ask?: number;
  /** Trade price */
  price: number;
  /** Trade size */
  size: number;
  /** Trade timestamp (Unix milliseconds) */
  time: number;
  /**
   * Exchange sequence number. Stock sends a JSON number; futopt sends a
   * zero-padded string (e.g. "00379320") — the padding is significant.
   */
  serial?: string | number;
}

/** Total trading statistics */
export interface TotalStats {
  /** Total trade value. Absent on FutOpt aggregates (no `tradeValue` key). */
  tradeValue?: number;
  /** Total trade volume */
  tradeVolume?: number;
  /** Volume traded at bid */
  tradeVolumeAtBid?: number;
  /** Volume traded at ask */
  tradeVolumeAtAsk?: number;
  /** Number of transactions */
  transaction?: number;
  /** Timestamp */
  time?: number;
}

/** Trading halt status */
export interface TradingHalt {
  /** Whether trading is halted */
  isHalted: boolean;
  /** Halt timestamp */
  time?: number;
}

// ============================================================================
// REST Response Types - Stock/FutOpt Intraday
// ============================================================================

/**
 * Quote response from intraday/quote/{symbol}
 *
 * Contains real-time price, volume, and order book data.
 */
export interface QuoteResponse {
  /** Trading date (YYYY-MM-DD) */
  date: string;
  /** Security type (e.g., "EQUITY", "ODDLOT") */
  type?: string;
  /** Exchange code (e.g., "TWSE", "TPEx", "TAIFEX") */
  exchange?: string;
  /** Market (e.g., "TSE", "OTC") */
  market?: string;
  /** Symbol (e.g., "2330" for stock, "TXFC4" for futures) */
  symbol: string;
  /** Security name */
  name?: string;
  /**
   * Reference price for the session — the basis the exchange computes
   * `change`/`changePercent` and the limit-up/limit-down prices against.
   * Not sent by the official SDK's documented shape, but present on real
   * payloads and distinct from `previousClose` whenever the reference is
   * adjusted (ex-dividend, ex-rights, a resumed suspension).
   */
  referencePrice?: number;
  /** Previous trading day's close price */
  previousClose?: number;

  // OHLC prices with timestamps
  /** Open price */
  openPrice?: number;
  /** Open time (Unix milliseconds) */
  openTime?: number;
  /** High price */
  highPrice?: number;
  /** High time (Unix milliseconds) */
  highTime?: number;
  /** Low price */
  lowPrice?: number;
  /** Low time (Unix milliseconds) */
  lowTime?: number;
  /** Close price */
  closePrice?: number;
  /** Close time (Unix milliseconds) */
  closeTime?: number;

  // Current trading info
  /** Last traded price */
  lastPrice?: number;
  /** Last traded size */
  lastSize?: number;
  /** Average price */
  avgPrice?: number;
  /** Price change from previous close */
  change?: number;
  /** Percentage change from previous close */
  changePercent?: number;
  /** Price amplitude */
  amplitude?: number;

  // Order book
  /** Bid price levels */
  bids: PriceLevel[];
  /** Ask price levels */
  asks: PriceLevel[];

  // Aggregated stats
  /** Total trading statistics */
  total?: TotalStats;
  /** Last trade info */
  lastTrade?: TradeInfo;
  /** Last trial (simulated matching) info */
  lastTrial?: TradeInfo;
  /** Trading halt status */
  tradingHalt?: TradingHalt;

  // Limit price flags. All are booleans the server defaults to `false` when
  // absent — now that responses pass through raw, an absent flag is simply
  // not in the object, so these are optional rather than always-present.
  /** Is at limit down price */
  isLimitDownPrice?: boolean;
  /** Is at limit up price */
  isLimitUpPrice?: boolean;
  /** Is limit down bid */
  isLimitDownBid?: boolean;
  /** Is limit up bid */
  isLimitUpBid?: boolean;
  /** Is limit down ask */
  isLimitDownAsk?: boolean;
  /** Is limit up ask */
  isLimitUpAsk?: boolean;
  /** Is limit down halt */
  isLimitDownHalt?: boolean;
  /** Is limit up halt */
  isLimitUpHalt?: boolean;

  // Trading session flags (same "absent = false" caveat as above)
  /** Is in trial (simulated matching) period */
  isTrial?: boolean;
  /** Is delayed open */
  isDelayedOpen?: boolean;
  /** Is delayed close */
  isDelayedClose?: boolean;
  /** Is continuous trading */
  isContinuous?: boolean;
  /** Is market open */
  isOpen?: boolean;
  /** Is market closed */
  isClose?: boolean;

  /**
   * Exchange sequence number for this quote (distinct from
   * `lastTrade.serial`, which numbers trades, not quote updates).
   */
  serial?: number;
  /** Last updated timestamp (Unix milliseconds) */
  lastUpdated?: number;
}

/**
 * Ticker response from intraday/ticker/{symbol}
 *
 * Contains static security information and trading rules.
 */
export interface TickerResponse {
  /**
   * Trading date (YYYY-MM-DD). Absent on `intraday/tickers` list items
   * (only `symbol`/`industry`/`name` etc.); present on the single-ticker
   * endpoint.
   */
  date?: string;
  /** Security type (e.g., "EQUITY", "ODDLOT") */
  type?: string;
  /** Exchange code */
  exchange?: string;
  /** Market */
  market?: string;
  /** Symbol */
  symbol: string;

  // Stock info
  /** Stock name (Chinese) */
  name?: string;
  /** Stock name (English) */
  nameEn?: string;
  /** Industry category */
  industry?: string;
  /** Security type classification */
  securityType?: string;

  // Price limits
  /** Reference price (previous close) */
  referencePrice?: number;
  /** Limit up price */
  limitUpPrice?: number;
  /** Limit down price */
  limitDownPrice?: number;
  /** Previous close price */
  previousClose?: number;

  // Trading rules. Booleans the server defaults to `false` when absent;
  // under raw passthrough an absent flag is simply not in the object.
  /** Can day trade */
  canDayTrade?: boolean;
  /** Can buy day trade */
  canBuyDayTrade?: boolean;
  /** Can below flat margin short sell */
  canBelowFlatMarginShortSell?: boolean;
  /** Can below flat SBL short sell */
  canBelowFlatSBLShortSell?: boolean;

  // Attention flags (same "absent = false" caveat as above)
  /** Is attention stock */
  isAttention?: boolean;
  /** Is disposition stock */
  isDisposition?: boolean;
  /** Is unusually recommended */
  isUnusuallyRecommended?: boolean;
  /** Is specific abnormally */
  isSpecificAbnormally?: boolean;
  /** Is newly compiled */
  isNewlyCompiled?: boolean;

  // Trading parameters
  /** Matching interval (seconds) */
  matchingInterval?: number;
  /** Security status */
  securityStatus?: string;
  /** Board lot size */
  boardLot?: number;
  /** Trading currency */
  tradingCurrency?: string;

  // Warrant/ETN specific
  /** Exercise price */
  exercisePrice?: number;
  /** Exercised volume */
  exercisedVolume?: number;
  /** Cancelled volume */
  cancelledVolume?: number;
  /** Remaining volume */
  remainingVolume?: number;
  /** Exercise ratio */
  exerciseRatio?: number;
  /** Cap price */
  capPrice?: number;
  /** Floor price */
  floorPrice?: number;
  /** Maturity date */
  maturityDate?: string;

  // Session times
  /** Open time */
  openTime?: string;
  /** Close time */
  closeTime?: string;
}

/**
 * Tickers response from intraday/tickers (batch ticker list).
 *
 * Previously unwrapped to a bare array; the server actually returns an
 * envelope with sibling metadata alongside `data`.
 */
export interface TickersResponse {
  /** Trading date (YYYY-MM-DD) */
  date?: string;
  /** Security type queried (e.g., "EQUITY") */
  type?: string;
  /** Exchange code */
  exchange?: string;
  /** Market */
  market?: string;
  /** Ticker data */
  data: TickerResponse[];
}

/** A single intraday candlestick bar */
export interface IntradayCandle {
  /** Open price */
  open: number;
  /** High price */
  high: number;
  /** Low price */
  low: number;
  /** Close price */
  close: number;
  /** Volume */
  volume: number;
  /** Average price (VWAP for the candle period) */
  average?: number;
  /** Candle timestamp (ISO 8601 with timezone, e.g. "2026-04-17T09:00:00.000+08:00") */
  date: string;
}

/**
 * Candles response from intraday/candles/{symbol}
 *
 * Contains OHLCV candlestick data.
 */
export interface CandlesResponse {
  /** Trading date (YYYY-MM-DD) */
  date: string;
  /** Security type */
  type?: string;
  /** Exchange code */
  exchange?: string;
  /** Market */
  market?: string;
  /** Symbol */
  symbol: string;
  /** Timeframe (e.g., "1", "5", "10", "15", "30", "60") */
  timeframe?: string;
  /** Candle data */
  data: IntradayCandle[];
}

/** A single trade execution */
export interface Trade {
  /** Best bid price at trade time */
  bid?: number;
  /** Best ask price at trade time */
  ask?: number;
  /** Trade price */
  price: number;
  /** Trade size (volume) */
  size: number;
  /** Trade timestamp (Unix milliseconds) */
  time: number;
  /** Server-assigned monotonic sequence number (dedup / pagination anchor) */
  serial?: number;
  /** Cumulative volume at this trade (session total so far) */
  volume?: number;
}

/**
 * Trades response from intraday/trades/{symbol}
 *
 * Contains recent trade executions.
 */
export interface TradesResponse {
  /** Trading date (YYYY-MM-DD) */
  date: string;
  /** Security type */
  type?: string;
  /** Exchange code */
  exchange?: string;
  /** Market */
  market?: string;
  /** Symbol */
  symbol: string;
  /** Trade data */
  data: Trade[];
}

/** Volume at a specific price level */
export interface VolumeAtPrice {
  /** Price level */
  price: number;
  /** Total volume at this price */
  volume: number;
  /** Volume traded at bid */
  volumeAtBid?: number;
  /** Volume traded at ask */
  volumeAtAsk?: number;
}

/**
 * Volumes response from intraday/volumes/{symbol}
 *
 * Contains volume profile data at each price level.
 */
export interface VolumesResponse {
  /** Trading date (YYYY-MM-DD) */
  date: string;
  /** Security type */
  type?: string;
  /** Exchange code */
  exchange?: string;
  /** Market */
  market?: string;
  /** Symbol */
  symbol: string;
  /** Volume data at each price level */
  data: VolumeAtPrice[];
}

// ============================================================================
// REST Response Types - FutOpt Specific
// ============================================================================

/** Contract type for futures/options */
export type ContractType = 'I' | 'R' | 'B' | 'C' | 'S' | 'E';

/** FutOpt type */
export type FutOptType = 'FUTURE' | 'OPTION';

/**
 * A single product entry in FutOpt products response.
 *
 * `symbol` is the only field the server is guaranteed to send — every other
 * field is `Option`/defaulted on the core model, so treat all of them as
 * possibly absent.
 */
export interface FutOptProduct {
  /** Product type (FUTURE/OPTION) */
  type?: string;
  /** Exchange code */
  exchange?: string;
  /** Contract symbol */
  symbol: string;
  /** Contract name */
  name?: string;
  /** Underlying symbol */
  underlyingSymbol?: string;
  /** Contract type */
  contractType?: string;
  /** Contract size */
  contractSize?: number;
  /** Status code */
  statusCode?: string;
  /** Trading currency */
  tradingCurrency?: string;
  /** Whether quote is acceptable */
  quoteAcceptable?: boolean;
  /** Start date */
  startDate?: string;
  /** Whether block trade is allowed */
  canBlockTrade?: boolean;
  /** Expiry type */
  expiryType?: string;
  /** Underlying type */
  underlyingType?: string;
  /** Market close group */
  marketCloseGroup?: number;
  /** End session */
  endSession?: number;
}

/**
 * Products response from futopt/intraday/products
 *
 * Contains available futures/options contracts.
 */
export interface ProductsResponse {
  /** Trading date (YYYY-MM-DD) */
  date?: string;
  /** Product type */
  type?: string;
  /** Trading session */
  session?: string;
  /** Contract type filter applied */
  contractType?: string;
  /** Status filter applied */
  status?: string;
  /** Product list */
  data: FutOptProduct[];
}

/** Daily price limits and the reference prices they are derived from (FutOpt quote). */
export interface FutOptPriceLimits {
  /** Limit on the traded price */
  price?: number;
  /** Limit on the bid side */
  bid?: number;
  /** Limit on the ask side */
  ask?: number;
  /** Circuit-breaker (curb) level */
  curb?: number;
}

/**
 * Total trading statistics for a FutOpt quote.
 *
 * Which of these the server sends varies by endpoint and by session; a
 * missing field is not an error.
 */
export interface FutOptTotalStats {
  /** Total trade volume */
  tradeVolume?: number;
  /** Total traded value. Absent on FutOpt aggregates. */
  tradeValue?: number;
  /** Total volume matched at bid price */
  totalBidMatch?: number;
  /** Total volume matched at ask price */
  totalAskMatch?: number;
  /** Volume traded at the bid */
  tradeVolumeAtBid?: number;
  /** Volume traded at the ask */
  tradeVolumeAtAsk?: number;
  /** Number of transactions */
  transaction?: number;
  /** Timestamp (Unix milliseconds) */
  time?: number;
}

/** Trading halt status for a FutOpt quote. */
export interface FutOptTradingHalt {
  /** Whether trading is currently halted */
  isHalted?: boolean;
  /** Timestamp of the halt state (Unix milliseconds) */
  time?: number;
}

/**
 * Real-time FutOpt quote from futopt/intraday/quote/{symbol}.
 *
 * Distinct from the stock `QuoteResponse`: carries `priceLimits`/`serial`
 * and has no `isLimitUpPrice`-style limit-price flags (those are stock-only).
 */
export interface FutOptQuoteResponse {
  /** Trading date (YYYY-MM-DD) */
  date: string;
  /** Contract type (FUTURE or OPTION) */
  type?: string;
  /** Exchange code (TAIFEX) */
  exchange?: string;
  /** Market */
  market?: string;
  /** Contract symbol (e.g., "TXFC4", "TXO18000C4") */
  symbol: string;
  /** Contract name. Dropped by the official SDK in 1.5.0; kept if present. */
  name?: string;
  /** Previous close price. Dropped by the official SDK in 1.5.0; kept if present. */
  previousClose?: number;
  /** Daily price limits */
  priceLimits?: FutOptPriceLimits;

  // OHLC prices with timestamps
  /** Open price */
  openPrice?: number;
  /** Open time (Unix milliseconds) */
  openTime?: number;
  /** High price */
  highPrice?: number;
  /** High time (Unix milliseconds) */
  highTime?: number;
  /** Low price */
  lowPrice?: number;
  /** Low time (Unix milliseconds) */
  lowTime?: number;
  /** Close price */
  closePrice?: number;
  /** Close time (Unix milliseconds) */
  closeTime?: number;

  // Current trading info
  /** Last traded price */
  lastPrice?: number;
  /** Last traded size (number of contracts) */
  lastSize?: number;
  /** Average price */
  avgPrice?: number;
  /** Price change from previous close */
  change?: number;
  /** Percentage change from previous close */
  changePercent?: number;
  /** Price amplitude */
  amplitude?: number;

  // Order book
  /** Bid price levels (best to worst) */
  bids: PriceLevel[];
  /** Ask price levels (best to worst) */
  asks: PriceLevel[];

  // Aggregated stats
  /** Total trading statistics */
  total?: FutOptTotalStats;
  /** Last trade information */
  lastTrade?: TradeInfo;
  /** Last trial match (試撮). Absent outside a trial session. */
  lastTrial?: TradeInfo;
  /** Trading halt status */
  tradingHalt?: FutOptTradingHalt;

  // Session flags. Booleans the server defaults to `false` when absent.
  /**
   * Marks the quote as trial-matching (試撮) — a simulated match, not a
   * trade. Branch on this before acting on `lastPrice`/`lastSize`.
   */
  isTrial?: boolean;
  /** Is delayed open */
  isDelayedOpen?: boolean;
  /** Is delayed close */
  isDelayedClose?: boolean;
  /** Is in continuous trading */
  isContinuous?: boolean;
  /** Is the session open */
  isOpen?: boolean;
  /** Is the session closed */
  isClose?: boolean;

  /** Exchange sequence number for this quote */
  serial?: number;
  /** Last updated timestamp (Unix milliseconds) */
  lastUpdated?: number;
}

/**
 * FutOpt contract information from futopt/intraday/ticker/{symbol}
 * (also used as the item type of `futopt/intraday/tickers`' `data` array).
 */
export interface FutOptTickerResponse {
  /**
   * Trading date (YYYY-MM-DD). Absent on `intraday/tickers` list items;
   * present on the single-ticker endpoint.
   */
  date?: string;
  /** Contract type (FUTURE or OPTION) */
  type?: string;
  /** Exchange code (TAIFEX) */
  exchange?: string;
  /** Contract symbol (e.g., "TXFC4", "TXO18000C4") */
  symbol: string;
  /** Contract name (e.g., "臺股期貨 03", "臺指選擇權 18000C 03") */
  name?: string;
  /** Reference price (previous settlement price) */
  referencePrice?: number;
  /** Contract start date (YYYY-MM-DD) - when the contract starts trading */
  startDate?: string;
  /** Contract end date (YYYY-MM-DD) - last trading date */
  endDate?: string;
  /** Settlement date (YYYY-MM-DD) - when the contract settles */
  settlementDate?: string;
  /** Contract sub-type (e.g., "I" for Index) */
  contractType?: string;
  /** Whether dynamic price banding is enabled */
  isDynamicBanding?: boolean;
  /**
   * Whether this is a spread (價差) contract. Spread symbols carry a `/`
   * (e.g. "TXFC4/TXFD4").
   */
  isSpread?: boolean;
  /** Flow group for trading */
  flowGroup?: number;
}

/** Tickers response from futopt/intraday/tickers (batch ticker list). */
export interface FutOptTickersResponse {
  /** Queried product type (FUTURE or OPTION) */
  type?: string;
  /** Exchange code */
  exchange?: string;
  /** Queried session (REGULAR or AFTERHOURS) */
  session?: string;
  /** Ticker data */
  data: FutOptTickerResponse[];
}

// ============================================================================
// WebSocket Types
// ============================================================================

/** WebSocket message payload */
export interface WebSocketMessage {
  /** Event type */
  event: string;
  /** Message data */
  data: Record<string, unknown>;
  /** Channel name */
  channel?: string;
  /** Symbol */
  symbol?: string;
}

/** Stock WebSocket channel types */
export type StockChannel = 'trades' | 'books' | 'candles' | 'aggregates' | 'indices';

/** FutOpt WebSocket channel types */
export type FutOptChannel = 'trades' | 'books' | 'candles' | 'aggregates';

/**
 * Subscribe options for stock WebSocket
 */
export interface StockSubscribeOptions {
  /** Channel to subscribe to */
  channel: StockChannel;
  /** Stock symbol */
  symbol: string;
  /** Include intraday odd lot data */
  intradayOddLot?: boolean;
}

/**
 * Subscribe options for FutOpt WebSocket
 */
export interface FutOptSubscribeOptions {
  /** Channel to subscribe to */
  channel: FutOptChannel;
  /** Contract symbol */
  symbol: string;
  /** Include after-hours data */
  afterHours?: boolean;
}

/**
 * Options for `unsubscribe()`, accepted alongside a bare subscription-id
 * string. Provide either `id` (single) or `ids` (batch) — exactly one. The
 * ids are the ones the server issued in its `subscribed` message.
 */
export interface UnsubscribeOptions {
  /** Single subscription id to unsubscribe from */
  id?: string;
  /** Batch of subscription ids to unsubscribe from */
  ids?: string[];
}

/**
 * Stock `unsubscribe()` by the options passed to `subscribe()`. Provide
 * either `symbol` or `symbols`; combining `channel` with `id` / `ids` is
 * error 1005.
 */
export interface StockUnsubscribeOptions {
  /** Channel to unsubscribe from */
  channel: StockChannel;
  /** Stock symbol */
  symbol?: string;
  /** Batch of stock symbols */
  symbols?: string[];
  /** The value passed to `subscribe()`: odd-lot is a separate subscription */
  intradayOddLot?: boolean;
}

/**
 * FutOpt `unsubscribe()` by the options passed to `subscribe()`. Provide
 * either `symbol` or `symbols`; combining `channel` with `id` / `ids` is
 * error 1005.
 */
export interface FutOptUnsubscribeOptions {
  /** Channel to unsubscribe from */
  channel: FutOptChannel;
  /** Contract symbol */
  symbol?: string;
  /** Batch of contract symbols */
  symbols?: string[];
  /** The value passed to `subscribe()`: after-hours is a separate subscription */
  afterHours?: boolean;
}

/**
 * Parameters for `ping()`, sent as the ping frame's `data`. The server echoes
 * `state` back in its pong.
 */
export interface WebSocketPingParams {
  state?: unknown;
  [key: string]: unknown;
}

/**
 * The server's `data` from an `authenticated` or authentication `error`
 * frame, as delivered to `authenticated` / `unauthenticated` and to
 * `connect()`'s resolution or rejection.
 */
export interface WebSocketAuthData {
  message?: string;
  [key: string]: unknown;
}

/** Argument of the `disconnect` event. */
export interface WebSocketDisconnectEvent {
  /** WebSocket close code, or `null` when the connection ended without one */
  code: number | null;
  /** Close reason */
  reason: string;
}

/** Argument of the `reconnect` event. */
export interface WebSocketReconnectEvent {
  /** Reconnection attempt number, starting at 1 */
  attempt: number;
}

/**
 * Argument of the `messagesDropped` event: messages dropped because
 * `messageBuffer` were unread (`messageOverflow: 'dropNewest'`).
 */
export interface WebSocketMessagesDroppedEvent {
  /** Messages dropped since the previous `messagesDropped` */
  dropped: number;
  /** Messages dropped on this connection so far (see `messagesDroppedTotal`) */
  total: number;
}

/** Category of an SDK error (`docs/errors.md`). */
export type ErrorSourceKind = 'network' | 'protocol' | 'auth' | 'rate_limit' | 'client';

/** The fields every SDK error carries (`docs/errors.md`). */
export interface MarketDataErrorFields {
  /** Numeric error code (see the error code table) */
  code: number;
  /** Category of the failure */
  sourceKind: ErrorSourceKind;
  /** HTTP status, when the error came from an HTTP response */
  status: number | null;
  /** Raw HTTP response body (REST only) */
  body: string | null;
  /** Server-assigned request id (`x-request-id`), when present */
  requestId: string | null;
  /** HTTP response headers, lowercase names (REST only; empty otherwise) */
  headers: Record<string, string>;
}

/**
 * Error thrown by constructors and rejected by REST methods and
 * `connect()` (except an `unauthenticated` rejection, which rejects with the
 * server's data).
 *
 * ```js
 * try {
 *   await client.stock.intraday.quote('2330');
 * } catch (err) {
 *   if (err.code === 2002) console.error('auth failed', err.status, err.body);
 * }
 * ```
 */
export interface MarketDataError extends Error, MarketDataErrorFields {}

/**
 * Argument of the `error` event: a {@link MarketDataError}.
 *
 * A listener that throws, or returns a Promise that rejects, is reported with
 * code 3004 and `event`, `count` and `cause` set — the first failure at once,
 * later ones at most once per second (#83).
 */
export interface WebSocketError extends Error, Partial<MarketDataErrorFields> {
  /** Code 3004: the event whose listener failed, e.g. `'message'` */
  event?: WebSocketEvent;
  /** Code 3004: listener failures since the previous report (1 for the first) */
  count?: number;
  /** Code 3004: what the listener threw, or the Promise's rejection reason */
  cause?: unknown;
}

/**
 * Event map for typed WebSocket callbacks; argument shapes match
 * `@fugle/marketdata` 1.x.
 */
export interface WebSocketEventMap {
  /** Raw frame received from the server (JSON string) */
  message: (data: string) => void;
  /** Socket opened, before authentication */
  connect: () => void;
  /** Authentication succeeded */
  authenticated: (data?: WebSocketAuthData) => void;
  /**
   * Authentication was rejected by the server (`error` code 1000). During
   * an auto-reconnect this is terminal: an `error` with code 3005 follows
   * and the client stays closed (#201). Any other auth-phase server error
   * (1011 auth service unavailable, 1004 no auth request received) is an
   * `error` with code 2001 instead, and the reconnect goes on.
   */
  unauthenticated: (data?: WebSocketAuthData) => void;
  /** Disconnected from WebSocket server */
  disconnect: (event: WebSocketDisconnectEvent) => void;
  /** Reconnecting to WebSocket server */
  reconnect: (event: WebSocketReconnectEvent) => void;
  /**
   * Error occurred. With no listener, SDK errors are ignored and listener
   * failures (code 3004) are printed with `console.error`.
   */
  error: (error: WebSocketError) => void;
  /**
   * Messages were dropped because listeners fell behind. The first drop on a
   * connection is reported at once, later ones at most once per second, and
   * the rest before `disconnect`.
   */
  messagesDropped: (event: WebSocketMessagesDroppedEvent) => void;
}

/** Event names for WebSocket */
export type WebSocketEvent = keyof WebSocketEventMap;

// ============================================================================
// REST Response Types - Stock Historical
// ============================================================================

/** A single historical candlestick bar */
export interface HistoricalCandle {
  /** Date (YYYY-MM-DD) */
  date: string;
  /** Open price */
  open: number;
  /** High price */
  high: number;
  /** Low price */
  low: number;
  /** Close price */
  close: number;
  /** Volume */
  volume: number;
  /** Turnover (total value traded) */
  turnover?: number;
  /** Price change from previous close */
  change?: number;
}

/**
 * Historical candles response from historical/candles/{symbol}
 *
 * Contains OHLCV data for a date range.
 */
export interface HistoricalCandlesResponse {
  /** Stock symbol */
  symbol: string;
  /** Security type (e.g., "EQUITY") */
  type?: string;
  /** Exchange code */
  exchange?: string;
  /** Market */
  market?: string;
  /** Timeframe (e.g., "D", "W", "M", "1", "5", etc.) */
  timeframe?: string;
  /** Whether prices are adjusted for splits/dividends */
  adjusted?: boolean;
  /** Candle data */
  data: HistoricalCandle[];
}

/**
 * Stats response from historical/stats/{symbol}
 *
 * Contains statistical summary for a stock.
 */
export interface StatsResponse {
  /** Trading date (YYYY-MM-DD) */
  date: string;
  /** Security type (e.g., "EQUITY") */
  type: string;
  /** Exchange code */
  exchange: string;
  /** Market */
  market: string;
  /** Stock symbol */
  symbol: string;
  /** Stock name */
  name: string;
  /** Opening price */
  openPrice: number;
  /** High price */
  highPrice: number;
  /** Low price */
  lowPrice: number;
  /** Closing price */
  closePrice: number;
  /** Price change */
  change: number;
  /** Price change percentage */
  changePercent: number;
  /** Total trading volume */
  tradeVolume: number;
  /** Total trading value */
  tradeValue: number;
  /** Previous close price */
  previousClose: number;
  /** 52-week high price */
  week52High: number;
  /** 52-week low price */
  week52Low: number;
}

// ============================================================================
// REST Response Types - Stock Snapshot
// ============================================================================

/** A single stock in snapshot quotes response */
export interface SnapshotQuote {
  /** Security type (e.g., "EQUITY") */
  type?: string;
  /** Stock symbol */
  symbol: string;
  /** Stock name */
  name?: string;
  /** Opening price */
  openPrice?: number;
  /** Highest price of the day */
  highPrice?: number;
  /** Lowest price of the day */
  lowPrice?: number;
  /** Closing/last price */
  closePrice?: number;
  /** Price change from previous close */
  change?: number;
  /** Percentage change from previous close */
  changePercent?: number;
  /** Trading volume */
  tradeVolume?: number;
  /** Trading value */
  tradeValue?: number;
  /** Last updated timestamp (Unix milliseconds) */
  lastUpdated?: number;
}

/**
 * Snapshot quotes response from snapshot/quotes/{market}
 *
 * Contains market-wide quote data.
 */
export interface SnapshotQuotesResponse {
  /** Trading date (YYYY-MM-DD) */
  date: string;
  /** Time of snapshot (HH:MM:SS) */
  time: string;
  /** Market code (e.g., "TSE", "OTC") */
  market: string;
  /** Quote data for each stock */
  data: SnapshotQuote[];
}

/** A single stock in movers response */
export interface Mover {
  /** Security type (e.g., "EQUITY") */
  type?: string;
  /** Stock symbol */
  symbol: string;
  /** Stock name */
  name?: string;
  /** Opening price */
  openPrice?: number;
  /** Highest price of the day */
  highPrice?: number;
  /** Lowest price of the day */
  lowPrice?: number;
  /** Closing/last price */
  closePrice?: number;
  /** Price change from previous close */
  change?: number;
  /** Percentage change from previous close */
  changePercent?: number;
  /** Trading volume */
  tradeVolume?: number;
  /** Trading value */
  tradeValue?: number;
  /** Last updated timestamp */
  lastUpdated?: number;
}

/**
 * Movers response from snapshot/movers/{market}
 *
 * Contains top gainers or losers.
 */
export interface MoversResponse {
  /** Trading date (YYYY-MM-DD) */
  date: string;
  /** Time of snapshot (HH:MM:SS) */
  time: string;
  /** Market code */
  market: string;
  /** Mover data */
  data: Mover[];
}

/** A single stock in actives response */
export interface Active {
  /** Security type (e.g., "EQUITY") */
  type?: string;
  /** Stock symbol */
  symbol: string;
  /** Stock name */
  name?: string;
  /** Opening price */
  openPrice?: number;
  /** Highest price of the day */
  highPrice?: number;
  /** Lowest price of the day */
  lowPrice?: number;
  /** Closing/last price */
  closePrice?: number;
  /** Price change from previous close */
  change?: number;
  /** Percentage change from previous close */
  changePercent?: number;
  /** Trading volume */
  tradeVolume?: number;
  /** Trading value */
  tradeValue?: number;
  /** Last updated timestamp */
  lastUpdated?: number;
}

/**
 * Actives response from snapshot/actives/{market}
 *
 * Contains most actively traded stocks.
 */
export interface ActivesResponse {
  /** Trading date (YYYY-MM-DD) */
  date: string;
  /** Time of snapshot (HH:MM:SS) */
  time: string;
  /** Market code */
  market: string;
  /** Active stock data */
  data: Active[];
}

// ============================================================================
// REST Response Types - Technical Indicators
// ============================================================================

/** SMA data point */
export interface SmaDataPoint {
  /** Date (YYYY-MM-DD) */
  date: string;
  /** SMA value */
  sma: number;
}

/**
 * SMA response from technical/sma/{symbol}
 *
 * `type`/`exchange`/`market`/`timeframe` are optional: prod responses omit
 * them, returning only `symbol`/`period`/`data`.
 */
export interface SmaResponse {
  /** Stock symbol */
  symbol: string;
  /** Security type */
  type?: string;
  /** Exchange code */
  exchange?: string;
  /** Market */
  market?: string;
  /** Timeframe */
  timeframe?: string;
  /** SMA period */
  period: number;
  /** SMA data points */
  data: SmaDataPoint[];
}

/** RSI data point */
export interface RsiDataPoint {
  /** Date (YYYY-MM-DD) */
  date: string;
  /** RSI value (0-100) */
  rsi: number;
}

/**
 * RSI response from technical/rsi/{symbol}
 *
 * `type`/`exchange`/`market`/`timeframe` are optional: prod responses omit
 * them, returning only `symbol`/`period`/`data`.
 */
export interface RsiResponse {
  /** Stock symbol */
  symbol: string;
  /** Security type */
  type?: string;
  /** Exchange code */
  exchange?: string;
  /** Market */
  market?: string;
  /** Timeframe */
  timeframe?: string;
  /** RSI period */
  period: number;
  /** RSI data points */
  data: RsiDataPoint[];
}

/** KDJ data point */
export interface KdjDataPoint {
  /** Date (YYYY-MM-DD) */
  date: string;
  /** K value (Fast Stochastic) */
  k: number;
  /** D value (Slow Stochastic) */
  d: number;
  /** J value */
  j: number;
}

/**
 * KDJ response from technical/kdj/{symbol}
 *
 * `type`/`exchange`/`market`/`timeframe` are optional: prod omits them.
 * Prod also returns three independent periods (`rPeriod`/`kPeriod`/
 * `dPeriod`), not a single `period`.
 */
export interface KdjResponse {
  /** Stock symbol */
  symbol: string;
  /** Security type */
  type?: string;
  /** Exchange code */
  exchange?: string;
  /** Market */
  market?: string;
  /** Timeframe */
  timeframe?: string;
  /** RSV period */
  rPeriod?: number;
  /** K smoothing period */
  kPeriod?: number;
  /** D smoothing period */
  dPeriod?: number;
  /** KDJ data points */
  data: KdjDataPoint[];
}

/** MACD data point */
export interface MacdDataPoint {
  /** Date (YYYY-MM-DD) */
  date: string;
  /** MACD line value (fast EMA - slow EMA) */
  macdLine: number;
  /** Signal line value (EMA of MACD) */
  signalLine: number;
  /** Histogram value (MACD - Signal). Absent — prod does not return it. */
  histogram?: number;
}

/**
 * MACD response from technical/macd/{symbol}
 *
 * `type`/`exchange`/`market`/`timeframe` are optional: prod omits them.
 */
export interface MacdResponse {
  /** Stock symbol */
  symbol: string;
  /** Security type */
  type?: string;
  /** Exchange code */
  exchange?: string;
  /** Market */
  market?: string;
  /** Timeframe */
  timeframe?: string;
  /** Fast EMA period */
  fast: number;
  /** Slow EMA period */
  slow: number;
  /** Signal line period */
  signal: number;
  /** MACD data points */
  data: MacdDataPoint[];
}

/** Bollinger Bands data point */
export interface BbDataPoint {
  /** Date (YYYY-MM-DD) */
  date: string;
  /** Upper band value */
  upper: number;
  /** Middle band value (SMA) */
  middle: number;
  /** Lower band value */
  lower: number;
}

/**
 * Bollinger Bands response from technical/bb/{symbol}
 *
 * `type`/`exchange`/`market`/`timeframe`/`stddev` are optional: prod omits
 * them, returning only `symbol`/`period`/`data`.
 */
export interface BbResponse {
  /** Stock symbol */
  symbol: string;
  /** Security type */
  type?: string;
  /** Exchange code */
  exchange?: string;
  /** Market */
  market?: string;
  /** Timeframe */
  timeframe?: string;
  /** SMA period */
  period: number;
  /** Standard deviation multiplier */
  stddev?: number;
  /** Bollinger Bands data points */
  data: BbDataPoint[];
}

// ============================================================================
// REST Response Types - Corporate Actions
// ============================================================================

/** Capital change record */
export interface CapitalChange {
  /** Stock symbol */
  symbol: string;
  /** Company name */
  name?: string;
  /** Date of the capital change (YYYY-MM-DD) */
  date: string;
  /** Previous capital (in TWD) */
  previousCapital?: number;
  /** Current capital (in TWD) */
  currentCapital?: number;
  /** Type of change */
  changeType?: string;
  /** Reason for the change */
  reason?: string;
}

/**
 * Capital changes response from corporate-actions/capital-changes
 */
export interface CapitalChangesResponse {
  /** Response type */
  type: string;
  /** Exchange code */
  exchange: string;
  /** Market */
  market: string;
  /** Capital change records */
  data: CapitalChange[];
}

/** One constituent of an ETF's holdings on a given date */
export interface EtfHoldingComponent {
  /** Constituent symbol (e.g. "2330") */
  symbol: string;
  /** Constituent name */
  name: string;
  /** Number of shares held */
  quantity: number;
  /** Portfolio weight, in percent */
  weight: number;
  /**
   * Change in shares held versus the previous disclosure.
   * Absent on the first date in a series — there is nothing to compare against.
   */
  quantityChange?: number;
  /** Change in portfolio weight versus the previous disclosure */
  weightChange?: number;
}

/** Holdings disclosed on a single date */
export interface EtfHoldingsEntry {
  /** Disclosure date (YYYY-MM-DD) */
  date: string;
  /** Constituents held on this date */
  components: EtfHoldingComponent[];
}

/** Response for `stock.ownership.etfHoldings` */
export interface EtfHoldingsResponse {
  /** Security type */
  type?: string;
  /** Exchange code */
  exchange?: string;
  /** Market */
  market?: string;
  /** The ETF symbol these holdings belong to */
  symbol: string;
  /** Holdings by disclosure date */
  data: EtfHoldingsEntry[];
}

/**
 * Buy / sell / net shares traded by one class of institutional investor.
 * Fields may be null when the source has no figure for that day.
 */
export interface InstitutionalInvestorTrade {
  /** Shares bought */
  buy: number | null;
  /** Shares sold */
  sell: number | null;
  /** Net shares (buy - sell) */
  net: number | null;
}

/** Institutional investor trading on a single date */
export interface InstitutionalTradesEntry {
  /** Trading date (YYYY-MM-DD) */
  date: string;
  /** Foreign investors */
  foreign: InstitutionalInvestorTrade | null;
  /** Investment trusts */
  trust: InstitutionalInvestorTrade | null;
  /** Dealers */
  dealer: InstitutionalInvestorTrade | null;
  /** Combined net shares across all three investor classes */
  total: number | null;
}

/** Response for `stock.ownership.institutionalTrades` */
export interface InstitutionalTradesResponse {
  /** Security type */
  type?: string;
  /** Exchange code */
  exchange?: string;
  /** Market */
  market?: string;
  /** Stock symbol */
  symbol: string;
  /** Trading by date */
  data: InstitutionalTradesEntry[];
}

/** One director's or supervisor's disclosed holdings */
export interface DirectorHolding {
  /** Position of this director in the disclosure */
  order: number | null;
  /** Title (e.g. chairman, director, supervisor) */
  title: string;
  /** Name */
  name: string;
  /** Shares held when elected */
  electedShares: number | null;
  /** Shares currently held */
  heldShares: number | null;
  /** Shares pledged */
  pledgedShares: number | null;
  /** Pledged shares as a ratio of held shares */
  pledgeRatio: number | null;
  /** Shares held by related parties (spouse, minor children, nominees) */
  relatedHeldShares: number | null;
  /** Shares pledged by related parties */
  relatedPledgedShares: number | null;
  /** Related-party pledged shares as a ratio of related-party held shares */
  relatedPledgeRatio: number | null;
}

/** Director holdings disclosed for a single month */
export interface DirectorHoldingsEntry {
  /** Disclosure month (YYYY-MM) */
  date: string;
  /** Directors and supervisors disclosed for this month */
  directors: DirectorHolding[];
}

/** Response for `stock.ownership.directorHoldings` */
export interface DirectorHoldingsResponse {
  /** Security type */
  type?: string;
  /** Exchange code */
  exchange?: string;
  /** Market */
  market?: string;
  /** Stock symbol */
  symbol: string;
  /** Holdings by disclosure month */
  data: DirectorHoldingsEntry[];
}

/** One holding-size bracket of the TDCC shareholder distribution */
export interface TdccDistributionLevel {
  /** Holding-size bracket label, as returned by the API */
  range: string;
  /** Number of shareholders in this bracket */
  holders: number | null;
  /** Shares held by this bracket */
  shares: number | null;
  /** Share of total outstanding held by this bracket, in percent */
  proportion: number | null;
}

/** TDCC shareholder distribution on a single date */
export interface TdccDistributionEntry {
  /** Data date (YYYY-MM-DD) */
  date: string;
  /** Distribution by holding-size bracket */
  distributions: TdccDistributionLevel[];
}

/** Response for `stock.ownership.tdccDistribution` */
export interface TdccDistributionResponse {
  /** Security type */
  type?: string;
  /** Exchange code */
  exchange?: string;
  /** Market */
  market?: string;
  /** Stock symbol */
  symbol: string;
  /** Distribution by date */
  data: TdccDistributionEntry[];
}

/** Dividend record */
export interface Dividend {
  /** Stock symbol */
  symbol: string;
  /** Company name */
  name?: string;
  /** Ex-dividend date (YYYY-MM-DD) */
  exDividendDate?: string;
  /** Payment date (YYYY-MM-DD) */
  paymentDate?: string;
  /** Cash dividend amount per share */
  cashDividend?: number;
  /** Stock dividend ratio */
  stockDividend?: number;
  /** Dividend year (fiscal year) */
  dividendYear?: string;
}

/**
 * Dividends response from corporate-actions/dividends
 */
export interface DividendsResponse {
  /** Response type */
  type: string;
  /** Exchange code */
  exchange: string;
  /** Market */
  market: string;
  /** Dividend records */
  data: Dividend[];
}

/** Listing applicant record (IPO) */
export interface ListingApplicant {
  /** Stock symbol */
  symbol: string;
  /** Company name */
  name?: string;
  /** Application date (YYYY-MM-DD) */
  applicationDate?: string;
  /** Expected or actual listing date (YYYY-MM-DD) */
  listingDate?: string;
  /** Application status */
  status?: string;
  /** Industry classification */
  industry?: string;
}

/**
 * Listing applicants response from corporate-actions/listing-applicants
 */
export interface ListingApplicantsResponse {
  /** Response type */
  type: string;
  /** Exchange code */
  exchange: string;
  /** Market */
  market: string;
  /** Listing applicant records */
  data: ListingApplicant[];
}

// ============================================================================
// REST Response Types - FutOpt Historical
// ============================================================================

/**
 * A single FutOpt historical candlestick bar.
 *
 * Price fields are optional: the request's `fields` param chooses which ones
 * the server returns.
 */
export interface FutOptHistoricalCandle {
  /** Date (YYYY-MM-DD), or a timestamp for intraday timeframes */
  date: string;
  /**
   * Contract month this bar belongs to — differs bar to bar when a continuous
   * `contractMonth` (`1!`) rolls over
   */
  contractMonth?: string;
  /** Open price */
  open?: number;
  /** High price */
  high?: number;
  /** Low price */
  low?: number;
  /** Close price */
  close?: number;
  /** Volume (number of contracts) */
  volume?: number;
  /** Average price (intraday timeframes only) */
  average?: number;
  /** Number of transactions (intraday timeframes only) */
  transaction?: number;
  /** Price change from previous close */
  change?: number;
}

/**
 * FutOpt historical candles response from futopt/historical/candles/{product}
 */
export interface FutOptHistoricalCandlesResponse {
  /** Product code, echoed from the request (e.g., "TXF") */
  product: string;
  /** Contract month queried: "YYYYMM", or the continuous alias ("1!") as requested */
  contractMonth?: string;
  /** Exchange code (e.g., "TAIFEX") */
  exchange?: string;
  /** Trading session ("REGULAR" or "AFTERHOURS") */
  session?: string;
  /** Timeframe (e.g., "D", "W", "M", "5") */
  timeframe?: string;
  /** Sort order ("asc" or "desc") */
  sort?: string;
  /** Candle data */
  data: FutOptHistoricalCandle[];
}

/** One contract month's daily quote for FutOpt */
export interface FutOptDailyData {
  /** Contract month (e.g., "202609", or "202609/202610" for a spread) */
  contractMonth: string;
  /** Option right ("CALL" / "PUT"); null for futures */
  callPut?: string | null;
  /** Option strike price; null for futures */
  strikePrice?: number | null;
  /** Exchange code (e.g., "TAIFEX") */
  exchange?: string;
  /** Open price */
  openPrice?: number;
  /** High price */
  highPrice?: number;
  /** Low price */
  lowPrice?: number;
  /** Close price */
  closePrice?: number;
  /** Price change from previous close */
  change?: number;
  /** Percentage change from previous close */
  changePercent?: number;
  /** Volume (number of contracts) */
  volume?: number;
  /** Spread-order volume */
  volumeSpread?: number;
  /** Open interest (total outstanding contracts) */
  openInterest?: number;
  /** Settlement price (official closing price for margin calculation) */
  settlementPrice?: number;
}

/**
 * FutOpt daily response from futopt/historical/daily/{product}:
 * one trading day, one row per contract month.
 */
export interface FutOptDailyResponse {
  /** Trading date (YYYY-MM-DD) */
  date?: string;
  /** Product code, echoed from the request (e.g., "TXF") */
  product: string;
  /** Exchange code (e.g., "TAIFEX") */
  exchange?: string;
  /** Trading session ("REGULAR" or "AFTERHOURS") */
  session?: string;
  /** One row per contract month */
  data: FutOptDailyData[];
}

// ============================================================================
// REST Params (object form)
// ============================================================================
//
// Every REST method also accepts a single params object, the call shape of the
// legacy `@fugle/marketdata` 1.x SDK and of the examples on developer.fugle.tw.
// The path param (`symbol` or `market`) is taken out and every other key is
// checked against the table of what the endpoint accepts (core's
// `rest::params`, built from the server's DTOs) before it is sent under the
// API's own name; an unknown key rejects with the accepted keys in the
// message. Values are sent as given and checked by the server. At runtime the
// snake_case forms (`is_trial`, `contract_month`) are accepted too; the types
// list the API names.

type Timeframe = 'D' | 'W' | 'M' | '1' | '3' | '5' | '10' | '15' | '30' | '60';
type IntradayTimeframe = '1' | '5' | '10' | '15' | '30' | '60';
type SnapshotMarket = 'TSE' | 'OTC' | 'ESB' | 'TIB' | 'PSB';
type SnapshotType = 'ALL' | 'ALLBUT0999' | 'COMMONSTOCK';

/** The odd-lot switch of the single-symbol stock intraday endpoints. */
interface RestStockIntradayOddLot {
  type?: 'oddlot';
  /** Same as `type: 'oddlot'` when true; this SDK's own spelling from 3.0.0-rc. */
  oddLot?: boolean;
}

/** Params for `stock.intraday.tickers` */
export interface RestStockIntradayTickersParams {
  type: string;
  exchange?: string;
  market?: string;
  industry?: string;
  isNormal?: boolean;
  isAttention?: boolean;
  isDisposition?: boolean;
  isHalted?: boolean;
  /** Comma-separated symbols to restrict the list to, e.g. "2330,2317" */
  symbol?: string;
}

/** Params for `stock.intraday.ticker` */
export interface RestStockIntradayTickerParams extends RestStockIntradayOddLot {
  symbol: string;
}

/** Params for `stock.intraday.quote` */
export interface RestStockIntradayQuoteParams extends RestStockIntradayOddLot {
  symbol: string;
}

/** @deprecated Use `RestStockIntradayQuoteParams`. */
export type StockIntradayQuoteParams = RestStockIntradayQuoteParams;

/** Params for `stock.intraday.candles` */
export interface RestStockIntradayCandlesParams extends RestStockIntradayOddLot {
  symbol: string;
  timeframe?: IntradayTimeframe;
  sort?: 'asc' | 'desc';
}

/** Params for `stock.intraday.trades` */
export interface RestStockIntradayTradesParams extends RestStockIntradayOddLot {
  symbol: string;
  offset?: number;
  limit?: number;
  sort?: 'asc' | 'desc';
  isTrial?: boolean;
}

/** Params for `stock.intraday.volumes` */
export interface RestStockIntradayVolumesParams extends RestStockIntradayOddLot {
  symbol: string;
}

/** Params for `stock.historical.candles` */
export interface RestStockHistoricalCandlesParams {
  symbol: string;
  from?: string;
  to?: string;
  timeframe?: Timeframe;
  fields?: string;
  sort?: 'asc' | 'desc';
  adjusted?: boolean;
}

/** Params for `stock.historical.stats` */
export interface RestStockHistoricalStatsParams {
  symbol: string;
}

/** Params for `stock.snapshot.quotes` */
export interface RestStockSnapshotQuotesParams {
  market: SnapshotMarket;
  type?: SnapshotType;
}

/** Params for `stock.snapshot.movers` */
export interface RestStockSnapshotMoversParams {
  market: SnapshotMarket;
  direction: 'up' | 'down';
  change: 'percent' | 'value';
  type?: SnapshotType;
  gt?: number;
  gte?: number;
  lt?: number;
  lte?: number;
  eq?: number;
}

/** Params for `stock.snapshot.actives` */
export interface RestStockSnapshotActivesParams {
  market: SnapshotMarket;
  trade: 'volume' | 'value';
  type?: SnapshotType;
}

interface RestStockTechnicalBaseParams {
  symbol: string;
  from?: string;
  to?: string;
  timeframe?: Timeframe;
}

/** Params for `stock.technical.sma` */
export interface RestStockTechnicalSmaParams extends RestStockTechnicalBaseParams {
  period: number;
}

/** Params for `stock.technical.rsi` */
export interface RestStockTechnicalRsiParams extends RestStockTechnicalBaseParams {
  period: number;
}

/** Params for `stock.technical.kdj` */
export interface RestStockTechnicalKdjParams extends RestStockTechnicalBaseParams {
  rPeriod: number;
  kPeriod: number;
  dPeriod: number;
}

/** Params for `stock.technical.macd` */
export interface RestStockTechnicalMacdParams extends RestStockTechnicalBaseParams {
  fast: number;
  slow: number;
  signal: number;
}

/** Params for `stock.technical.bb` */
export interface RestStockTechnicalBbParams extends RestStockTechnicalBaseParams {
  period: number;
}

interface RestStockCorporateActionsDateRange {
  start_date?: string;
  end_date?: string;
  sort?: 'asc' | 'desc';
}

/** Params for `stock.corporateActions.capitalChanges` (no `exchange`: the server rejects it) */
export type RestStockCorporateActionsCapitalChangesParams = RestStockCorporateActionsDateRange;
/** Params for `stock.corporateActions.dividends` */
export interface RestStockCorporateActionsDividendsParams extends RestStockCorporateActionsDateRange {
  exchange?: 'TWSE' | 'TPEx';
}
/** Params for `stock.corporateActions.listingApplicants` */
export interface RestStockCorporateActionsListingApplicantsParams extends RestStockCorporateActionsDateRange {
  exchange?: 'TWSE' | 'TPEx';
}

/** Params for `futopt.intraday.products` (no `product`: that key belongs to `tickers`) */
export interface RestFutOptIntradayProductsParams {
  type: FutOptType;
  exchange?: 'TAIFEX';
  session?: 'REGULAR' | 'AFTERHOURS';
  contractType?: ContractType;
  status?: 'N' | 'P' | 'U';
}

/** Params for `futopt.intraday.tickers` (no `status`: that key belongs to `products`) */
export interface RestFutOptIntradayTickersParams {
  type: FutOptType;
  exchange?: 'TAIFEX';
  session?: 'REGULAR' | 'AFTERHOURS';
  product?: string;
  contractType?: ContractType;
  isSpread?: boolean;
}

interface RestFutOptIntradaySymbolParams {
  symbol: string;
  session?: 'afterhours';
}

/** Params for `futopt.intraday.quote` */
export type RestFutOptIntradayQuoteParams = RestFutOptIntradaySymbolParams;
/** Params for `futopt.intraday.ticker` */
export type RestFutOptIntradayTickerParams = RestFutOptIntradaySymbolParams;
/** Params for `futopt.intraday.volumes` */
export type RestFutOptIntradayVolumesParams = RestFutOptIntradaySymbolParams;

/** Params for `futopt.intraday.candles` */
export interface RestFutOptIntradayCandlesParams extends RestFutOptIntradaySymbolParams {
  timeframe?: IntradayTimeframe;
}

/** Params for `futopt.intraday.trades` */
export interface RestFutOptIntradayTradesParams extends RestFutOptIntradaySymbolParams {
  offset?: number;
  limit?: number;
  isTrial?: boolean;
}

/** FutOpt trading session; the server is case-insensitive. */
type FutOptHistoricalSession = 'REGULAR' | 'AFTERHOURS' | 'regular' | 'afterhours';

/**
 * The product code for `futopt.historical.*` (e.g. `TXF`, not a contract such
 * as `TXFC4`), under the API's own name `product` or the legacy `symbol`.
 */
type FutOptHistoricalProduct =
  | { product: string; symbol?: never }
  | { symbol: string; product?: never };

/** Params for `futopt.historical.candles` */
export type RestFutOptHistoricalCandlesParams = FutOptHistoricalProduct & {
  /** "YYYYMM", or a continuous contract: "1!" (default), "2!", "3!" */
  contractMonth?: string;
  from?: string;
  to?: string;
  timeframe?: '1' | '5' | '10' | '15' | '30' | '60' | 'D' | 'W' | 'M';
  /** Comma-separated, from `open,high,low,close,volume,average,transaction,change` */
  fields?: string;
  sort?: 'asc' | 'desc';
  /** Options only, with `callPut` */
  strikePrice?: number;
  /** Options only, with `strikePrice` */
  callPut?: 'CALL' | 'PUT';
  session?: FutOptHistoricalSession;
};

/** Params for `futopt.historical.daily` */
export type RestFutOptHistoricalDailyParams = FutOptHistoricalProduct & {
  /** Trading date (YYYY-MM-DD); the server defaults to today */
  date?: string;
  session?: FutOptHistoricalSession;
};

// ============================================================================
// Client Interfaces
// ============================================================================

/** Stock historical client interface */
export interface StockHistoricalClient {
  /** Get historical candles for a stock */
  candles(symbol: string | RestStockHistoricalCandlesParams, from?: string, to?: string, timeframe?: string): Promise<HistoricalCandlesResponse>;
  /** Get historical stats for a stock */
  stats(symbol: string | RestStockHistoricalStatsParams): Promise<StatsResponse>;
}

/** Stock snapshot client interface */
export interface StockSnapshotClient {
  /** Get snapshot quotes for a market */
  quotes(market: string | RestStockSnapshotQuotesParams, typeFilter?: string): Promise<SnapshotQuotesResponse>;
  /** Get movers (top gainers/losers) for a market */
  movers(market: string | RestStockSnapshotMoversParams, direction?: string, change?: string): Promise<MoversResponse>;
  /** Get most actively traded stocks for a market */
  actives(market: string | RestStockSnapshotActivesParams, trade?: string): Promise<ActivesResponse>;
}

/** Stock technical client interface */
export interface StockTechnicalClient {
  /** Get SMA for a stock */
  sma(symbol: string | RestStockTechnicalSmaParams, from?: string, to?: string, timeframe?: string, period?: number): Promise<SmaResponse>;
  /** Get RSI for a stock */
  rsi(symbol: string | RestStockTechnicalRsiParams, from?: string, to?: string, timeframe?: string, period?: number): Promise<RsiResponse>;
  /** Get KDJ for a stock */
  kdj(symbol: string | RestStockTechnicalKdjParams, from?: string, to?: string, timeframe?: string, rPeriod?: number, kPeriod?: number, dPeriod?: number): Promise<KdjResponse>;
  /** Get MACD for a stock */
  macd(symbol: string | RestStockTechnicalMacdParams, from?: string, to?: string, timeframe?: string, fast?: number, slow?: number, signal?: number): Promise<MacdResponse>;
  /** Get Bollinger Bands for a stock */
  bb(symbol: string | RestStockTechnicalBbParams, from?: string, to?: string, timeframe?: string, period?: number): Promise<BbResponse>;
}

/** Stock corporate actions client interface */
export interface StockCorporateActionsClient {
  /** Get capital changes */
  capitalChanges(startDate?: string | RestStockCorporateActionsCapitalChangesParams, endDate?: string): Promise<CapitalChangesResponse>;
  /** Get dividends */
  dividends(startDate?: string | RestStockCorporateActionsDividendsParams, endDate?: string): Promise<DividendsResponse>;
  /** Get listing applicants */
  listingApplicants(startDate?: string | RestStockCorporateActionsListingApplicantsParams, endDate?: string): Promise<ListingApplicantsResponse>;
}

/** FutOpt historical client interface */
export interface FutOptHistoricalClient {
  /** Get historical candles for a FutOpt product */
  candles(symbol: string | RestFutOptHistoricalCandlesParams, from?: string, to?: string, timeframe?: string, afterHours?: boolean, contractMonth?: string, fields?: string, sort?: 'asc' | 'desc'): Promise<FutOptHistoricalCandlesResponse>;
  /** Get one trading day's daily quotes for every contract month of a FutOpt product */
  daily(symbol: string | RestFutOptHistoricalDailyParams, date?: string, afterHours?: boolean): Promise<FutOptDailyResponse>;
}


/* auto-generated by NAPI-RS */
/* eslint-disable */

/** Futures and Options market data client */
export declare class FutOptClient {
  /** Get intraday client for real-time futures/options data */
  get intraday(): FutOptIntradayClient
  /** Get historical client for historical futures/options data */
  get historical(): FutOptHistoricalClient
}

/** FutOpt historical data client */
export declare class FutOptHistoricalClient {
  /**
   * Get historical candles for a futures/options product
   *
   * @param symbol - Product code (e.g., "TXF"); a contract code such as "TXFC4" returns 404
   * @param from - Start date (YYYY-MM-DD)
   * @param to - End date (YYYY-MM-DD)
   * @param timeframe - Timeframe ("D", "W", "M", "1", "5", "10", "15", "30", "60")
   * @param afterHours - Query the after-hours session
   * @param contractMonth - "YYYYMM", or a continuous contract: "1!" (default), "2!", "3!"
   * @param fields - Comma-separated fields, e.g. "open,high,low,close,volume"
   * @param sort - "asc" or "desc"
   * @returns Promise resolving to historical candles data
   */
  candles(symbol: string | RestFutOptHistoricalCandlesParams, from?: string | undefined | null, to?: string | undefined | null, timeframe?: string | undefined | null, afterHours?: boolean | undefined | null, contractMonth?: string | undefined | null, fields?: string | undefined | null, sort?: 'asc' | 'desc' | undefined | null): Promise<FutOptHistoricalCandlesResponse>
  /**
   * Get one trading day's daily quotes for every contract month of a futures/options product
   *
   * @param symbol - Product code (e.g., "TXF"); a contract code such as "TXFC4" returns 404
   * @param date - Trading date (YYYY-MM-DD); the server defaults to today
   * @param afterHours - Query the after-hours session
   * @returns Promise resolving to daily historical data
   */
  daily(symbol: string | RestFutOptHistoricalDailyParams, date?: string | undefined | null, afterHours?: boolean | undefined | null): Promise<FutOptDailyResponse>
}

/** FutOpt intraday data client */
export declare class FutOptIntradayClient {
  /**
   * Get intraday quote for a futures/options contract
   *
   * @param symbol - Contract symbol (e.g., "TXFC4" for TX futures, "TXO18000C4" for options)
   * @returns Promise resolving to Quote object with current price and volume data
   *
   * @example
   * ```javascript
   * const client = new RestClient('your-api-key');
   * const quote = await client.futopt.intraday.quote('TXFC4');
   * console.log(quote.lastPrice);  // 17550.0
   * console.log(quote.symbol);     // "TXFC4"
   * ```
   */
  quote(symbol: string | RestFutOptIntradayQuoteParams): Promise<FutOptQuoteResponse>
  /**
   * Get intraday ticker for a futures/options contract
   *
   * @param symbol - Contract symbol (e.g., "TXFC4")
   * @returns Promise resolving to Ticker object with last trade info
   */
  ticker(symbol: string | RestFutOptIntradayTickerParams): Promise<FutOptTickerResponse>
  /**
   * Get intraday candles for a futures/options contract
   *
   * @param symbol - Contract symbol (e.g., "TXFC4")
   * @param timeframe - Candle timeframe: "1", "5", "10", "15", "30", "60" (minutes)
   * @returns Promise resolving to Candles response with OHLCV data
   */
  candles(symbol: string | RestFutOptIntradayCandlesParams, timeframe?: string | undefined | null): Promise<CandlesResponse>
  /**
   * Get intraday trades for a futures/options contract
   *
   * @param symbol - Contract symbol (e.g., "TXFC4")
   * @returns Promise resolving to Trades response with recent trade history
   */
  trades(symbol: string | RestFutOptIntradayTradesParams): Promise<TradesResponse>
  /**
   * Get intraday volumes for a futures/options contract
   *
   * @param symbol - Contract symbol (e.g., "TXFC4")
   * @returns Promise resolving to Volumes response with volume at each price level
   */
  volumes(symbol: string | RestFutOptIntradayVolumesParams): Promise<VolumesResponse>
  /**
   * Get batch ticker list for a FutOpt contract type
   *
   * @param type - Contract type: "FUTURE" or "OPTION"
   * @param exchange - Optional exchange filter (e.g., "TAIFEX")
   * @param afterHours - Query after-hours session data
   * @param contractType - Optional contract type code: "I" / "R" / "B" / "C" / "S" / "E"
   * @returns Promise resolving to an array of FutOpt ticker info objects
   */
  tickers(type: FutOptType | RestFutOptIntradayTickersParams, exchange?: string | undefined | null, afterHours?: boolean | undefined | null, contractType?: ContractType | undefined | null, isSpread?: boolean | undefined | null): Promise<FutOptTickersResponse>
  /**
   * Get product list for futures/options
   *
   * @param typ - Type: "FUTURE" or "OPTION" (required)
   * @param contractType - Contract type filter (optional): "I" (index), "R" (rate), "B" (bond), "C" (currency), "S" (stock), "E" (ETF)
   * @returns Promise resolving to Products response with available contracts
   */
  products(type: FutOptType | RestFutOptIntradayProductsParams, contractType?: ContractType | undefined | null): Promise<ProductsResponse>
}

/**
 * FutOpt WebSocket client for real-time futures/options market data
 *
 * # JavaScript Usage
 *
 * ```javascript
 * // Event handlers
 * ws.futopt.on('message', (data) => {
 *   const msg = JSON.parse(data);
 *   console.log(msg);
 * });
 * ws.futopt.on('connect', () => console.log('FutOpt WebSocket connected'));
 *
 * // Connect
 * ws.futopt.connect();
 *
 * // Subscribe to channels
 * ws.futopt.subscribe({ channel: 'trades', symbol: 'TXFC4' });
 * ws.futopt.subscribe({ channel: 'books', symbol: 'MXFB4', afterHours: true });
 * ```
 */
export declare class FutOptWebSocketClient {
  /**
   * Register an event handler
   *
   * Same events and arguments as `StockWebSocketClient::on` (#23).
   */
  on<E extends WebSocketEvent>(event: E, callback: WebSocketEventMap[E]): void
  /**
   * Connect to the FutOpt WebSocket server.
   *
   * Returns a Promise that resolves with the server's `authenticated`
   * `data`. See `StockWebSocketClient::connect` for the rejections,
   * including code `2011` (`Already connected`) (#44), and for waiting on
   * an automatic reconnect (#230).
   */
  connect(): Promise<WebSocketAuthData | undefined>
  /**
   * Subscribe to a channel
   *
   * @param options - Subscription options. Provide either `symbol` (single)
   *                  or `symbols` (batch list) — exactly one is required.
   *                  Shape: `{ channel, symbol?, symbols?, afterHours? }`
   */
  subscribe(options: FutOptSubscribeOptions): void
  /**
   * Unsubscribe from a channel
   *
   * Accepts the server id as a string, `{ id: "..." }` (single) or
   * `{ ids: ["...", "..."] }` (batch); or the `subscribe` options
   * `{ channel, symbol | symbols, afterHours? }`.
   */
  unsubscribe(options: string | UnsubscribeOptions | FutOptUnsubscribeOptions): void
  /**
   * Send a `ping` frame to the server.
   *
   * Mirrors the old `@fugle/marketdata` Node SDK: fire and forget. The
   * server's `pong` reply is delivered via the `message` callback. To wait
   * for the pong and get the round trip, use `measureLatency()`.
   *
   * @param params - Sent as the frame's `data`, e.g. `{ state: 'x' }`, whose
   *                 `state` the server echoes back in its pong. A string is
   *                 accepted for compatibility and sent as `{ state }`.
   */
  ping(params?: string | WebSocketPingParams): void
  /**
   * Measure the round trip to the server: send a ping, wait for its pong,
   * and resolve with the time between the two in milliseconds.
   *
   * Works whether or not `healthCheck.probeEnabled` is set, and sends
   * nothing in the background. Its pong is not delivered to `message`.
   *
   * Rejects with `ClientClosed` (2010) when not connected,
   * `ConnectionError` (2001) when the connection closes before the pong,
   * and `TimeoutError` (3001) when no pong arrives within `timeoutMs`.
   *
   * @param timeoutMs - How long to wait for the pong (default: 5000).
   */
  measureLatency(timeoutMs?: number | undefined | null): Promise<number>
  /**
   * Ask the server for its current subscription list.
   *
   * Sends `{ event: "subscriptions" }` to the server. The reply is delivered
   * asynchronously via the `message` callback, matching the old
   * `@fugle/marketdata` Node SDK semantics.
   */
  subscriptions(): void
  /** Disconnect from the WebSocket server */
  disconnect(): void
  /**
   * Messages dropped because they arrived while `messageBuffer` were
   * unread (`messageOverflow: 'dropNewest'`).
   *
   * Counted from the start of the current connection (every `connect()` or
   * reconnect restarts it); after `disconnect()` it still reads the last
   * connection's count. 0 before the first `connect()`.
   */
  get messagesDroppedTotal(): number
  /**
   * Check if connected
   *
   * True while the connection is authenticated; false while an
   * auto-reconnect is in progress.
   */
  get isConnected(): boolean
  /**
   * Check if client has been closed
   *
   * Returns true once the connection has closed: after disconnect(), or
   * after the server or network ended it with no reconnect left. A closed
   * client can connect() again; isClosed turns false once the new
   * connection starts.
   */
  get isClosed(): boolean
}

/**
 * REST client for Fugle market data API (JavaScript wrapper)
 *
 * # JavaScript Usage
 *
 * ```javascript
 * const { RestClient } = require('@fugle/marketdata');
 *
 * // Create client with API key
 * const client = new RestClient('your-api-key');
 *
 * // Access stock market data
 * const quote = client.stock.intraday.quote('2330');
 * console.log(quote.lastPrice, quote.symbol);
 *
 * // Access futures/options market data
 * const futoptQuote = client.futopt.intraday.quote('TXFC4');
 * console.log(futoptQuote.lastPrice, futoptQuote.symbol);
 * ```
 */
export declare class RestClient {
  /**
   * Create a new REST client with options
   *
   * @param options - Client configuration options
   * @throws {Error} If validation fails (zero or multiple auth methods)
   *
   * @example
   * ```javascript
   * const { RestClient } = require('@fugle/marketdata');
   *
   * // API key auth
   * const client = new RestClient({ apiKey: 'your-key' });
   *
   * // Bearer token auth with custom base URL
   * const client = new RestClient({
   *   bearerToken: 'token',
   *   baseUrl: 'https://custom.api'
   * });
   * ```
   */
  constructor(options: RestClientOptions)
  /**
   * The prefix every request from this client is built on, fully resolved —
   * host, path prefix and version segment. Endpoints are appended to it.
   *
   * The version segment is chosen by the SDK rather than written by the
   * caller, so this is the only way to see what a client resolved to.
   */
  get baseUrl(): string
  /** Get the stock client for accessing stock market data */
  get stock(): StockClient
  /** Get the FutOpt client for accessing futures/options market data */
  get futopt(): FutOptClient
}

/** Stock market data client */
export declare class StockClient {
  /** Get intraday client for real-time stock data */
  get intraday(): StockIntradayClient
  /** Get historical client for historical stock data */
  get historical(): StockHistoricalClient
  /** Get snapshot client for market-wide data */
  get snapshot(): StockSnapshotClient
  /** Get technical indicators client */
  get technical(): StockTechnicalClient
  /** Get corporate actions client */
  get corporateActions(): StockCorporateActionsClient
  /** Get ownership client (ETF holdings, institutional trades, director holdings, TDCC distribution) */
  get ownership(): StockOwnershipClient
  /** The fully resolved request prefix for this product client. */
  get baseUrl(): string
}

/** Stock corporate actions client */
export declare class StockCorporateActionsClient {
  /**
   * Get capital changes (capital structure changes)
   *
   * @param startDate - Start date for range query (YYYY-MM-DD)
   * @param endDate - End date for range query (YYYY-MM-DD)
   * @returns Promise resolving to capital changes data
   */
  capitalChanges(startDate?: string | RestStockCorporateActionsCapitalChangesParams | undefined | null, endDate?: string | undefined | null): Promise<CapitalChangesResponse>
  /**
   * Get dividend announcements
   *
   * @param startDate - Start date for range query (YYYY-MM-DD)
   * @param endDate - End date for range query (YYYY-MM-DD)
   * @returns Promise resolving to dividends data
   */
  dividends(startDate?: string | RestStockCorporateActionsDividendsParams | undefined | null, endDate?: string | undefined | null): Promise<DividendsResponse>
  /**
   * Get IPO listing applicants
   *
   * @param startDate - Start date for range query (YYYY-MM-DD)
   * @param endDate - End date for range query (YYYY-MM-DD)
   * @returns Promise resolving to listing applicants data
   */
  listingApplicants(startDate?: string | RestStockCorporateActionsListingApplicantsParams | undefined | null, endDate?: string | undefined | null): Promise<ListingApplicantsResponse>
}

/** Stock historical data client */
export declare class StockHistoricalClient {
  /**
   * Get historical candles for a stock symbol
   *
   * @param symbol - Stock symbol (e.g., "2330")
   * @param from - Start date (YYYY-MM-DD)
   * @param to - End date (YYYY-MM-DD)
   * @param timeframe - Timeframe ("D", "W", "M", "1", "5", etc.)
   * @returns Promise resolving to historical candles data
   */
  candles(symbol: string | RestStockHistoricalCandlesParams, from?: string | undefined | null, to?: string | undefined | null, timeframe?: string | undefined | null): Promise<HistoricalCandlesResponse>
  /**
   * Get historical stats for a stock symbol
   *
   * @param symbol - Stock symbol (e.g., "2330")
   * @returns Promise resolving to historical stats data
   */
  stats(symbol: string | RestStockHistoricalStatsParams): Promise<StatsResponse>
}

/** Stock intraday data client */
export declare class StockIntradayClient {
  /**
   * Get intraday quote for a stock symbol.
   *
   * Two call shapes are supported (legacy fugle-marketdata-node parity):
   *
   * ```javascript
   * // Object shape (matches legacy SDK README)
   * await client.stock.intraday.quote({ symbol: '2330' });
   * await client.stock.intraday.quote({ symbol: '2330', type: 'oddlot' });
   *
   * // Positional shape
   * await client.stock.intraday.quote('2330');
   * await client.stock.intraday.quote('2330', true);
   * ```
   */
  quote(symbol: string | RestStockIntradayQuoteParams, oddLot?: boolean | undefined | null): Promise<QuoteResponse>
  /**
   * Get intraday ticker for a stock symbol
   *
   * @param symbol - Stock symbol (e.g., "2330" for TSMC)
   * @returns Promise resolving to Ticker object with last trade info
   */
  ticker(symbol: string | RestStockIntradayTickerParams): Promise<TickerResponse>
  /**
   * Get intraday candles for a stock symbol
   *
   * @param symbol - Stock symbol (e.g., "2330" for TSMC)
   * @param timeframe - Candle timeframe: "1", "5", "10", "15", "30", "60" (minutes)
   * @returns Promise resolving to Candles response with OHLCV data
   */
  candles(symbol: string | RestStockIntradayCandlesParams, timeframe?: string | undefined | null): Promise<CandlesResponse>
  /**
   * Get intraday trades for a stock symbol
   *
   * @param symbol - Stock symbol (e.g., "2330" for TSMC)
   * @returns Promise resolving to Trades response with recent trade history
   */
  trades(symbol: string | RestStockIntradayTradesParams): Promise<TradesResponse>
  /**
   * Get intraday volumes for a stock symbol
   *
   * @param symbol - Stock symbol (e.g., "2330" for TSMC)
   * @returns Promise resolving to Volumes response with volume at each price level
   */
  volumes(symbol: string | RestStockIntradayVolumesParams): Promise<VolumesResponse>
  /**
   * Get batch ticker list for a security type
   *
   * @param type - Security type ("EQUITY", "INDEX", "ETF", ...)
   * @param exchange - Optional exchange filter (e.g., "TWSE", "TPEx")
   * @param market - Optional market filter (e.g., "TSE", "OTC")
   * @param industry - Optional industry code filter
   * @param isNormal - Filter to normal-status tickers only
   * @returns Promise resolving to an array of ticker info objects
   */
  tickers(type: string | RestStockIntradayTickersParams, exchange?: string | undefined | null, market?: string | undefined | null, industry?: string | undefined | null, isNormal?: boolean | undefined | null): Promise<TickersResponse>
}

/** Stock ownership data client */
export declare class StockOwnershipClient {
  /**
   * Get the constituents an ETF held over a date range.
   *
   * ```javascript
   * await client.stock.ownership.etfHoldings({ symbol: '0050' });
   * await client.stock.ownership.etfHoldings({ symbol: '0050', from: '2026-01-01', sort: 'desc' });
   * ```
   */
  etfHoldings(params: EtfHoldingsParams): Promise<EtfHoldingsResponse>
  /**
   * Get daily trading by the three major institutional investors (foreign, investment trust, dealer).
   *
   * ```javascript
   * await client.stock.ownership.institutionalTrades({ symbol: '2330' });
   * await client.stock.ownership.institutionalTrades({ symbol: '2330', from: '2026-01-01', sort: 'desc' });
   * ```
   */
  institutionalTrades(params: InstitutionalTradesParams): Promise<InstitutionalTradesResponse>
  /**
   * Get monthly holdings and pledges disclosed by directors and supervisors.
   *
   * ```javascript
   * await client.stock.ownership.directorHoldings({ symbol: '2330' });
   * await client.stock.ownership.directorHoldings({ symbol: '2330', from: '2026-01-01', sort: 'desc' });
   * ```
   */
  directorHoldings(params: DirectorHoldingsParams): Promise<DirectorHoldingsResponse>
  /**
   * Get the weekly TDCC shareholder distribution by holding-size bracket.
   *
   * ```javascript
   * await client.stock.ownership.tdccDistribution({ symbol: '2330' });
   * await client.stock.ownership.tdccDistribution({ symbol: '2330', from: '2026-01-01', sort: 'desc' });
   * ```
   */
  tdccDistribution(params: TdccDistributionParams): Promise<TdccDistributionResponse>
}

/** Stock snapshot data client */
export declare class StockSnapshotClient {
  /**
   * Get snapshot quotes for a market
   *
   * @param market - Market code (e.g., "TSE", "OTC")
   * @param typeFilter - Optional type filter (e.g., "ALL", "COMMONSTOCK")
   * @returns Promise resolving to snapshot quotes data
   */
  quotes(market: string | RestStockSnapshotQuotesParams, typeFilter?: string | undefined | null): Promise<SnapshotQuotesResponse>
  /**
   * Get movers (top gainers/losers) for a market
   *
   * @param market - Market code (e.g., "TSE", "OTC")
   * @param direction - Direction filter ("up" or "down")
   * @param change - Change type ("percent" or "value")
   * @returns Promise resolving to movers data
   */
  movers(market: string | RestStockSnapshotMoversParams, direction?: string | undefined | null, change?: string | undefined | null): Promise<MoversResponse>
  /**
   * Get most actively traded stocks for a market
   *
   * @param market - Market code (e.g., "TSE", "OTC")
   * @param trade - Trade type filter ("volume" or "value")
   * @returns Promise resolving to actives data
   */
  actives(market: string | RestStockSnapshotActivesParams, trade?: string | undefined | null): Promise<ActivesResponse>
}

/** Stock technical indicators client */
export declare class StockTechnicalClient {
  /**
   * Get SMA (Simple Moving Average) for a stock
   *
   * @param symbol - Stock symbol (e.g., "2330")
   * @param from - Start date (YYYY-MM-DD)
   * @param to - End date (YYYY-MM-DD)
   * @param timeframe - Timeframe ("D", "W", "M")
   * @param period - SMA period (e.g., 20)
   * @returns Promise resolving to SMA data
   */
  sma(symbol: string | RestStockTechnicalSmaParams, from?: string | undefined | null, to?: string | undefined | null, timeframe?: string | undefined | null, period?: number | undefined | null): Promise<SmaResponse>
  /**
   * Get RSI (Relative Strength Index) for a stock
   *
   * @param symbol - Stock symbol (e.g., "2330")
   * @param from - Start date (YYYY-MM-DD)
   * @param to - End date (YYYY-MM-DD)
   * @param timeframe - Timeframe ("D", "W", "M")
   * @param period - RSI period (e.g., 14)
   * @returns Promise resolving to RSI data
   */
  rsi(symbol: string | RestStockTechnicalRsiParams, from?: string | undefined | null, to?: string | undefined | null, timeframe?: string | undefined | null, period?: number | undefined | null): Promise<RsiResponse>
  /**
   * Get KDJ (Stochastic Oscillator) for a stock
   *
   * @param symbol - Stock symbol (e.g., "2330")
   * @param from - Start date (YYYY-MM-DD)
   * @param to - End date (YYYY-MM-DD)
   * @param timeframe - Timeframe ("D", "W", "M")
   * @param rPeriod - RSV period (e.g., 9)
   * @param kPeriod - K smoothing period (e.g., 3)
   * @param dPeriod - D smoothing period (e.g., 3)
   * @returns Promise resolving to KDJ data
   */
  kdj(symbol: string | RestStockTechnicalKdjParams, from?: string | undefined | null, to?: string | undefined | null, timeframe?: string | undefined | null, rPeriod?: number | undefined | null, kPeriod?: number | undefined | null, dPeriod?: number | undefined | null): Promise<KdjResponse>
  /**
   * Get MACD (Moving Average Convergence Divergence) for a stock
   *
   * @param symbol - Stock symbol (e.g., "2330")
   * @param from - Start date (YYYY-MM-DD)
   * @param to - End date (YYYY-MM-DD)
   * @param timeframe - Timeframe ("D", "W", "M")
   * @param fast - Fast EMA period (default: 12)
   * @param slow - Slow EMA period (default: 26)
   * @param signal - Signal line period (default: 9)
   * @returns Promise resolving to MACD data
   */
  macd(symbol: string | RestStockTechnicalMacdParams, from?: string | undefined | null, to?: string | undefined | null, timeframe?: string | undefined | null, fast?: number | undefined | null, slow?: number | undefined | null, signal?: number | undefined | null): Promise<MacdResponse>
  /**
   * Get Bollinger Bands for a stock
   *
   * @param symbol - Stock symbol (e.g., "2330")
   * @param from - Start date (YYYY-MM-DD)
   * @param to - End date (YYYY-MM-DD)
   * @param timeframe - Timeframe ("D", "W", "M")
   * @param period - SMA period (default: 20)
   * @returns Promise resolving to Bollinger Bands data
   */
  bb(symbol: string | RestStockTechnicalBbParams, from?: string | undefined | null, to?: string | undefined | null, timeframe?: string | undefined | null, period?: number | undefined | null): Promise<BbResponse>
}

/**
 * Stock WebSocket client for real-time stock market data
 *
 * # JavaScript Usage
 *
 * ```javascript
 * // Event handlers
 * ws.stock.on('message', (data) => {
 *   const msg = JSON.parse(data);
 *   console.log(msg);
 * });
 * ws.stock.on('connect', () => console.log('Stock WebSocket connected'));
 * ws.stock.on('disconnect', (reason) => console.log('Disconnected:', reason));
 * ws.stock.on('reconnect', (info) => console.log('Reconnecting:', info));
 * ws.stock.on('error', (err) => console.error('Error:', err));
 *
 * // Connect
 * ws.stock.connect();
 *
 * // Subscribe to channels
 * ws.stock.subscribe({ channel: 'trades', symbol: '2330' });
 * ws.stock.subscribe({ channel: 'candles', symbol: '2330' });
 * ```
 */
export declare class StockWebSocketClient {
  /**
   * Register an event handler
   *
   * Arguments match `@fugle/marketdata` 1.x (#23): `message(data: string)`,
   * `connect()` when the socket opens, `authenticated(data)` /
   * `unauthenticated(data)` with the server's `data`,
   * `disconnect({ code, reason })`, `reconnect({ attempt })`, and
   * `error(Error)` with a numeric `code` when core supplied one. Without an
   * `error` listener errors are ignored rather than thrown.
   *
   * `message` frames that arrive before a `message` listener is registered
   * are dropped, not delivered to it later (#62).
   *
   * @param event - Event type: "message", "connect", "authenticated",
   *                "unauthenticated", "disconnect", "reconnect", "error"
   * @param callback - Listener for that event
   *
   * @example
   * ```javascript
   * ws.stock.on('message', (data) => console.log(data));
   * ws.stock.on('connect', () => console.log('Connected'));
   * ws.stock.on('disconnect', ({ code, reason }) => console.log(code, reason));
   * ws.stock.on('error', (err) => console.error(err.code, err.message));
   * ```
   */
  on<E extends WebSocketEvent>(event: E, callback: WebSocketEventMap[E]): void
  /**
   * Connect to the stock WebSocket server.
   *
   * Returns a Promise that resolves with the server's `authenticated`
   * `data` once authentication completes, matching `@fugle/marketdata` 1.x
   * (#23):
   *
   * ```js
   * stock.connect().then((data) => {
   *   stock.subscribe({ channel: 'trades', symbol: '2330' });
   * });
   * ```
   *
   * If the server rejects the credentials, the Promise rejects with the
   * server's `data` object itself (after `unauthenticated` fires); any other
   * failure rejects with a `MarketDataError` (`code`, `sourceKind`, … as
   * properties; no `[code]` prefix in the message).
   *
   * Rejects with code `2011` (`Already connected`) while a connection is open,
   * or while the first `connect()` is still in progress (#44). Call
   * disconnect() first to reconnect; calling connect() right after
   * disconnect(), or from a `disconnect` handler once no auto-reconnect will
   * follow, is fine.
   *
   * During an automatic reconnect — for instance from a `disconnect`
   * handler, as 1.x code often does — it waits for the reconnect instead
   * of starting another connection (#230). It then resolves with the
   * reconnect's `authenticated` `data` once the stored subscriptions have
   * been re-sent, so a `subscribe()` made then follows them. It rejects
   * with code `2010` (`Connection aborted`) if disconnect() is called or
   * the connection ends without reconnecting, `3005` (`ReconnectFailed`)
   * when the attempts run out, and with the server's `data` object when
   * the reconnect's credentials are rejected.
   */
  connect(): Promise<WebSocketAuthData | undefined>
  /**
   * Subscribe to a channel
   *
   * @param options - Subscription options. Provide either `symbol` (single)
   *                  or `symbols` (batch list) — exactly one is required, matching
   *                  the old `@fugle/marketdata` shape.
   *                  Shape: `{ channel, symbol?, symbols?, intradayOddLot? }`
   */
  subscribe(options: StockSubscribeOptions): void
  /**
   * Unsubscribe from a channel
   *
   * Accepts the server id as a string, `{ id: "..." }` (single) or
   * `{ ids: ["...", "..."] }` (batch), mirroring the old `@fugle/marketdata`
   * Node SDK shape; or the `subscribe` options
   * `{ channel, symbol | symbols, intradayOddLot? }`.
   */
  unsubscribe(options: string | UnsubscribeOptions | StockUnsubscribeOptions): void
  /**
   * Send a `ping` frame to the server.
   *
   * Mirrors the old `@fugle/marketdata` Node SDK: fire and forget. The
   * server's `pong` reply is delivered via the `message` callback. To wait
   * for the pong and get the round trip, use `measureLatency()`.
   *
   * @param params - Sent as the frame's `data`, e.g. `{ state: 'x' }`, whose
   *                 `state` the server echoes back in its pong. A string is
   *                 accepted for compatibility and sent as `{ state }`.
   */
  ping(params?: string | WebSocketPingParams): void
  /**
   * Measure the round trip to the server: send a ping, wait for its pong,
   * and resolve with the time between the two in milliseconds.
   *
   * Works whether or not `healthCheck.probeEnabled` is set, and sends
   * nothing in the background. Its pong is not delivered to `message`.
   *
   * Rejects with `ClientClosed` (2010) when not connected,
   * `ConnectionError` (2001) when the connection closes before the pong,
   * and `TimeoutError` (3001) when no pong arrives within `timeoutMs`.
   *
   * @param timeoutMs - How long to wait for the pong (default: 5000).
   */
  measureLatency(timeoutMs?: number | undefined | null): Promise<number>
  /**
   * Ask the server for its current subscription list.
   *
   * Sends `{ event: "subscriptions" }` to the server. The reply is delivered
   * asynchronously via the `message` callback, matching the old
   * `@fugle/marketdata` Node SDK semantics.
   */
  subscriptions(): void
  /** Disconnect from the WebSocket server */
  disconnect(): void
  /**
   * Messages dropped because they arrived while `messageBuffer` were
   * unread (`messageOverflow: 'dropNewest'`).
   *
   * Counted from the start of the current connection (every `connect()` or
   * reconnect restarts it); after `disconnect()` it still reads the last
   * connection's count. 0 before the first `connect()`.
   */
  get messagesDroppedTotal(): number
  /**
   * Check if connected
   *
   * True while the connection is authenticated; false while an
   * auto-reconnect is in progress.
   */
  get isConnected(): boolean
  /**
   * Check if client has been closed
   *
   * Returns true once the connection has closed: after disconnect(), or
   * after the server or network ended it with no reconnect left. A closed
   * client can connect() again; isClosed turns false once the new
   * connection starts.
   */
  get isClosed(): boolean
}

/**
 * WebSocket client for real-time market data (JavaScript wrapper)
 *
 * # JavaScript Usage
 *
 * ```javascript
 * const { WebSocketClient } = require('@fugle/marketdata');
 *
 * // Create client with API key
 * const ws = new WebSocketClient('your-api-key');
 *
 * // Register event handlers for stock data
 * ws.stock.on('message', (data) => console.log(JSON.parse(data)));
 * ws.stock.on('connect', () => console.log('Connected!'));
 * ws.stock.on('error', (err) => console.error(err));
 *
 * // Connect and subscribe
 * ws.stock.connect();
 * ws.stock.subscribe({ channel: 'trades', symbol: '2330' });
 * ```
 */
export declare class WebSocketClient {
  /**
   * Create a new WebSocket client with configuration
   *
   * @param options - Client configuration options
   * @throws {Error} If validation fails (zero or multiple auth methods, invalid config values)
   *
   * @example
   * ```javascript
   * const { WebSocketClient } = require('@fugle/marketdata');
   *
   * // Simple usage with defaults
   * const ws = new WebSocketClient({ apiKey: 'your-key' });
   *
   * // Custom reconnection config
   * const ws = new WebSocketClient({
   *   apiKey: 'your-key',
   *   reconnect: { maxAttempts: 10, initialDelayMs: 2000 }
   * });
   *
   * // Enable health check
   * const ws = new WebSocketClient({
   *   apiKey: 'your-key',
   *   healthCheck: { probeEnabled: true, idleProbeAfterMs: 10000 }
   * });
   * ```
   */
  constructor(options: WebSocketClientOptions)
  /**
   * Get the stock WebSocket client for real-time stock data.
   *
   * Every access returns a new JS wrapper but all wrappers share the same
   * underlying state (callbacks, connection state, command channel), so the
   * legacy `ws.stock.on(...); ws.stock.connect()` pattern works correctly.
   */
  get stock(): StockWebSocketClient
  /**
   * Get the FutOpt WebSocket client for real-time futures/options data.
   *
   * Same shared-state semantics as `stock` — see its doc comment.
   */
  get futopt(): FutOptWebSocketClient
}

/** `stock.ownership.directorHoldings` params (object form, matching the official SDK) */
export interface DirectorHoldingsParams {
  symbol: string
  from?: string
  to?: string
  sort?: string
}

/** `stock.ownership.etfHoldings` params (object form, matching the official SDK) */
export interface EtfHoldingsParams {
  symbol: string
  from?: string
  to?: string
  sort?: string
}

/**
 * Health check options for WebSocket connections
 *
 * All fields are optional. Defaults: enabled=true, heartbeatTimeoutMs=35000,
 * probeEnabled=false, idleProbeAfterMs=30000, probeTimeoutMs=5000.
 */
export interface HealthCheckOptions {
  /** Whether liveness detection is active (default: true in 3.0) */
  enabled?: boolean
  /**
   * Maximum allowed gap between inbound frames before declaring the
   * connection dead, in milliseconds. Default 35000 (Fugle server's
   * 30s heartbeat + 5s buffer); floor 5000.
   *
   * **Does not apply when `probeEnabled` is true**: detection is then
   * `idleProbeAfterMs + probeTimeoutMs`.
   */
  heartbeatTimeoutMs?: number
  /**
   * Confirm a silent connection with a ping before declaring it dead
   * (default: false). After `idleProbeAfterMs` without any inbound frame
   * one `{"event":"ping"}` is sent; if nothing arrives within
   * `probeTimeoutMs` the connection is declared dead. Turning this on
   * alone keeps detection at 35s and sends a ping only when the server's
   * 30s heartbeat is late. Does not detect a connection whose writes no
   * longer reach the server while the server still sends.
   */
  probeEnabled?: boolean
  /**
   * Silence before the probe, in milliseconds (default: 30000, the
   * server's heartbeat period; floor 5000). Probe mode only. Below 30000
   * a ping is sent in every gap between heartbeats while no data flows.
   */
  idleProbeAfterMs?: number
  /**
   * Wait for any inbound frame after the probe, in milliseconds
   * (default: 5000; floor 1000). Probe mode only.
   */
  probeTimeoutMs?: number
}

/** `stock.ownership.institutionalTrades` params (object form, matching the official SDK) */
export interface InstitutionalTradesParams {
  symbol: string
  from?: string
  to?: string
  sort?: string
}

/**
 * Reconnection options for WebSocket clients
 *
 * All fields are optional - defaults are applied when not specified:
 * - maxAttempts: 0 (unlimited)
 * - initialDelayMs: 1000
 * - maxDelayMs: 60000
 */
export interface ReconnectOptions {
  /**
   * Whether auto-reconnect is enabled (default: true, also when the
   * `reconnect` option is omitted; set `false` to turn it off)
   */
  enabled?: boolean
  /**
   * Maximum reconnection attempts; 0 means unlimited (default: 0, so the
   * client keeps retrying at most `maxDelayMs` apart)
   */
  maxAttempts?: number
  /** Initial reconnection delay in milliseconds (default: 1000, min: 100) */
  initialDelayMs?: number
  /** Maximum reconnection delay in milliseconds (default: 60000) */
  maxDelayMs?: number
}

/**
 * REST client options
 *
 * Exactly ONE non-empty apiKey, bearerToken, or sdkToken must be provided;
 * an empty or whitespace-only value counts as not provided.
 * baseUrl is optional for custom endpoint override.
 */
export interface RestClientOptions {
  /** API key for authentication */
  apiKey?: string
  /** Bearer token for authentication */
  bearerToken?: string
  /** SDK token for authentication */
  sdkToken?: string
  /** Override base URL (optional) */
  baseUrl?: string
  /**
   * Additional root CA (PEM bytes). Appended to the OS trust store;
   * chains signed by either this CA or an OS-trusted root are accepted.
   */
  tlsRootCertPem?: Uint8Array
  /**
   * Disable ALL TLS verification (chain + hostname + expiry).
   * Dev/testing only — exposes MITM risk. Defaults to false.
   */
  tlsAcceptInvalidCerts?: boolean
}

/**
 * Per-product streaming version selection.
 *
 * The official SDK takes a free-form map and validates at runtime; expressing
 * it as a struct lets TypeScript reject an unknown product at compile time,
 * while the string values still need checking here.
 */
export interface StreamingVersionOptions {
  /** Stock streaming version. Only "v1.0" is served. */
  stock?: string
  /**
   * FutOpt streaming version: "v1.0" or "v1.1" (default).
   *
   * v1.1 adds trial-matching (試撮) frames on trades / books — branch on
   * the frame's `isTrial` before acting on a price.
   */
  futopt?: string
}

/** `stock.ownership.tdccDistribution` params (object form, matching the official SDK) */
export interface TdccDistributionParams {
  symbol: string
  from?: string
  to?: string
  sort?: string
}

/**
 * WebSocket client options
 *
 * Exactly ONE non-empty apiKey, bearerToken, or sdkToken must be provided;
 * an empty or whitespace-only value counts as not provided.
 * reconnect and healthCheck are optional configuration objects.
 */
export interface WebSocketClientOptions {
  /** API key for authentication */
  apiKey?: string
  /** Bearer token for authentication */
  bearerToken?: string
  /** SDK token for authentication */
  sdkToken?: string
  /**
   * Override base URL (optional). Host and path prefix ONLY — the SDK
   * appends the version segment.
   */
  baseUrl?: string
  /**
   * Per-product streaming version, e.g. `{ futopt: 'v1.0' }`.
   * Omitted products get their latest: stock v1.0, futopt v1.1.
   */
  version?: StreamingVersionOptions
  /** Reconnection configuration (optional) */
  reconnect?: ReconnectOptions
  /** Health check configuration (optional) */
  healthCheck?: HealthCheckOptions
  /** Additional root CA (PEM bytes). Appended to the OS trust store. */
  tlsRootCertPem?: Uint8Array
  /**
   * Disable ALL TLS verification (chain + hostname + expiry).
   * Dev/testing only — exposes MITM risk. Defaults to false.
   */
  tlsAcceptInvalidCerts?: boolean
  /**
   * What happens while `messageBuffer` messages are unread: `'dropNewest'`
   * (default) drops new ones and reports them with `messagesDropped`;
   * `'unbounded'` never drops, and memory grows while listeners lag.
   */
  messageOverflow?: 'dropNewest' | 'unbounded'
  /**
   * Unread messages held before `messageOverflow` applies (default 4096).
   * Up to this many wait in the SDK, and up to this many more may be
   * queued for `message` listeners that have not run yet. While those
   * listeners hold up delivery, events wait as well; beyond 1024 unread
   * events the SDK drops them too.
   */
  messageBuffer?: number
  /**
   * How long the auth handshake may take once the WebSocket is open, in
   * milliseconds: from the auth frame being sent until the server's
   * verdict (default 10000). Applies to the first `connect()` and to
   * every reconnect; elapsing it fails the attempt with a `TimeoutError`
   * (code 3001). Must be greater than 0. The server itself allows 60 s.
   */
  authTimeoutMs?: number
}
