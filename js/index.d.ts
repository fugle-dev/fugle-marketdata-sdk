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
}

/** Total trading statistics */
export interface TotalStats {
  /** Total trade value */
  tradeValue: number;
  /** Total trade volume */
  tradeVolume: number;
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

  // Limit price flags
  /** Is at limit down price */
  isLimitDownPrice: boolean;
  /** Is at limit up price */
  isLimitUpPrice: boolean;
  /** Is limit down bid */
  isLimitDownBid: boolean;
  /** Is limit up bid */
  isLimitUpBid: boolean;
  /** Is limit down ask */
  isLimitDownAsk: boolean;
  /** Is limit up ask */
  isLimitUpAsk: boolean;
  /** Is limit down halt */
  isLimitDownHalt: boolean;
  /** Is limit up halt */
  isLimitUpHalt: boolean;

  // Trading session flags
  /** Is in trial (simulated matching) period */
  isTrial: boolean;
  /** Is delayed open */
  isDelayedOpen: boolean;
  /** Is delayed close */
  isDelayedClose: boolean;
  /** Is continuous trading */
  isContinuous: boolean;
  /** Is market open */
  isOpen: boolean;
  /** Is market closed */
  isClose: boolean;

  /** Last updated timestamp (Unix milliseconds) */
  lastUpdated?: number;
}

/**
 * Ticker response from intraday/ticker/{symbol}
 *
 * Contains static security information and trading rules.
 */
export interface TickerResponse {
  /** Trading date (YYYY-MM-DD) */
  date: string;
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

  // Trading rules
  /** Can day trade */
  canDayTrade: boolean;
  /** Can buy day trade */
  canBuyDayTrade: boolean;
  /** Can below flat margin short sell */
  canBelowFlatMarginShortSell: boolean;
  /** Can below flat SBL short sell */
  canBelowFlatSBLShortSell: boolean;

  // Attention flags
  /** Is attention stock */
  isAttention: boolean;
  /** Is disposition stock */
  isDisposition: boolean;
  /** Is unusually recommended */
  isUnusuallyRecommended: boolean;
  /** Is specific abnormally */
  isSpecificAbnormally: boolean;
  /** Is newly compiled */
  isNewlyCompiled: boolean;

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
  /** Candle timestamp (Unix milliseconds) */
  time: number;
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

/** A single product entry in FutOpt products response */
export interface FutOptProduct {
  /** Product type (FUTURE/OPTION) */
  type: string;
  /** Exchange code */
  exchange: string;
  /** Contract symbol */
  symbol: string;
  /** Contract name */
  name: string;
  /** Underlying symbol */
  underlyingSymbol: string;
  /** Contract type */
  contractType: string;
  /** Contract size */
  contractSize: number;
  /** Status code */
  statusCode: string;
  /** Trading currency */
  tradingCurrency: string;
  /** Whether quote is acceptable */
  quoteAcceptable: boolean;
  /** Start date */
  startDate: string;
  /** Whether block trade is allowed */
  canBlockTrade: boolean;
  /** Expiry type */
  expiryType: string;
  /** Underlying type */
  underlyingType: string;
  /** Market close group */
  marketCloseGroup: number;
  /** End session */
  endSession: number;
}

/**
 * Products response from futopt/intraday/products
 *
 * Contains available futures/options contracts.
 */
export interface ProductsResponse {
  /** Trading date (YYYY-MM-DD) */
  date: string;
  /** Product type */
  type: string;
  /** Trading session */
  session: string;
  /** Contract type filter applied */
  contractType: string;
  /** Status filter applied */
  status: string;
  /** Product list */
  data: FutOptProduct[];
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
 * Event map for typed WebSocket callbacks
 */
export interface WebSocketEventMap {
  /** Market data message received */
  message: (data: string) => void;
  /** Connected to WebSocket server */
  connect: (info: string) => void;
  /** Disconnected from WebSocket server */
  disconnect: (reason: string) => void;
  /** Reconnecting to WebSocket server */
  reconnect: (info: string) => void;
  /** Error occurred */
  error: (error: string) => void;
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
 */
export interface SmaResponse {
  /** Stock symbol */
  symbol: string;
  /** Security type */
  type: string;
  /** Exchange code */
  exchange: string;
  /** Market */
  market: string;
  /** Timeframe */
  timeframe: string;
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
 */
export interface RsiResponse {
  /** Stock symbol */
  symbol: string;
  /** Security type */
  type: string;
  /** Exchange code */
  exchange: string;
  /** Market */
  market: string;
  /** Timeframe */
  timeframe: string;
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
 */
export interface KdjResponse {
  /** Stock symbol */
  symbol: string;
  /** Security type */
  type: string;
  /** Exchange code */
  exchange: string;
  /** Market */
  market: string;
  /** Timeframe */
  timeframe: string;
  /** KDJ period */
  period: number;
  /** KDJ data points */
  data: KdjDataPoint[];
}

/** MACD data point */
export interface MacdDataPoint {
  /** Date (YYYY-MM-DD) */
  date: string;
  /** MACD line value */
  macd: number;
  /** Signal line value */
  signal: number;
  /** Histogram value (MACD - Signal) */
  histogram: number;
}

/**
 * MACD response from technical/macd/{symbol}
 */
export interface MacdResponse {
  /** Stock symbol */
  symbol: string;
  /** Security type */
  type: string;
  /** Exchange code */
  exchange: string;
  /** Market */
  market: string;
  /** Timeframe */
  timeframe: string;
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
 */
export interface BbResponse {
  /** Stock symbol */
  symbol: string;
  /** Security type */
  type: string;
  /** Exchange code */
  exchange: string;
  /** Market */
  market: string;
  /** Timeframe */
  timeframe: string;
  /** SMA period */
  period: number;
  /** Standard deviation multiplier */
  stddev: number;
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

/** A single FutOpt historical candlestick bar */
export interface FutOptHistoricalCandle {
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
  /** Volume (number of contracts) */
  volume: number;
  /** Open interest (total outstanding contracts) */
  openInterest?: number;
  /** Price change from previous close */
  change?: number;
  /** Percentage change from previous close */
  changePercent?: number;
}

/**
 * FutOpt historical candles response from futopt/historical/candles/{symbol}
 */
export interface FutOptHistoricalCandlesResponse {
  /** Contract symbol */
  symbol: string;
  /** Contract type (e.g., "FUTURE", "OPTION") */
  type?: string;
  /** Exchange code (e.g., "TAIFEX") */
  exchange?: string;
  /** Timeframe (e.g., "D", "W", "M") */
  timeframe?: string;
  /** Candle data (note: API returns as "data", renamed to "candles" in napi-rs) */
  candles: FutOptHistoricalCandle[];
}

/** A single FutOpt daily data point */
export interface FutOptDailyData {
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
  /** Volume (number of contracts) */
  volume: number;
  /** Open interest (total outstanding contracts) */
  openInterest?: number;
  /** Settlement price (official closing price for margin calculation) */
  settlementPrice?: number;
}

/**
 * FutOpt daily response from futopt/historical/daily/{symbol}
 */
export interface FutOptDailyResponse {
  /** Contract symbol */
  symbol: string;
  /** Contract type (e.g., "FUTURE", "OPTION") */
  type?: string;
  /** Exchange code (e.g., "TAIFEX") */
  exchange?: string;
  /** Daily data */
  data: FutOptDailyData[];
}

// ============================================================================
// Client Interfaces
// ============================================================================

/** Stock historical client interface */
export interface StockHistoricalClient {
  /** Get historical candles for a stock */
  candles(symbol: string, from?: string, to?: string, timeframe?: string): Promise<HistoricalCandlesResponse>;
  /** Get historical stats for a stock */
  stats(symbol: string): Promise<StatsResponse>;
}

/** Stock snapshot client interface */
export interface StockSnapshotClient {
  /** Get snapshot quotes for a market */
  quotes(market: string, typeFilter?: string): Promise<SnapshotQuotesResponse>;
  /** Get movers (top gainers/losers) for a market */
  movers(market: string, direction?: string, change?: string): Promise<MoversResponse>;
  /** Get most actively traded stocks for a market */
  actives(market: string, trade?: string): Promise<ActivesResponse>;
}

/** Stock technical client interface */
export interface StockTechnicalClient {
  /** Get SMA for a stock */
  sma(symbol: string, from?: string, to?: string, timeframe?: string, period?: number): Promise<SmaResponse>;
  /** Get RSI for a stock */
  rsi(symbol: string, from?: string, to?: string, timeframe?: string, period?: number): Promise<RsiResponse>;
  /** Get KDJ for a stock */
  kdj(symbol: string, from?: string, to?: string, timeframe?: string, period?: number): Promise<KdjResponse>;
  /** Get MACD for a stock */
  macd(symbol: string, from?: string, to?: string, timeframe?: string, fast?: number, slow?: number, signal?: number): Promise<MacdResponse>;
  /** Get Bollinger Bands for a stock */
  bb(symbol: string, from?: string, to?: string, timeframe?: string, period?: number, stddev?: number): Promise<BbResponse>;
}

/** Stock corporate actions client interface */
export interface StockCorporateActionsClient {
  /** Get capital changes */
  capitalChanges(date?: string, startDate?: string, endDate?: string): Promise<CapitalChangesResponse>;
  /** Get dividends */
  dividends(date?: string, startDate?: string, endDate?: string): Promise<DividendsResponse>;
  /** Get listing applicants */
  listingApplicants(date?: string, startDate?: string, endDate?: string): Promise<ListingApplicantsResponse>;
}

/** FutOpt historical client interface */
export interface FutOptHistoricalClient {
  /** Get historical candles for a FutOpt contract */
  candles(symbol: string, from?: string, to?: string, timeframe?: string, afterHours?: boolean): Promise<FutOptHistoricalCandlesResponse>;
  /** Get daily historical data for a FutOpt contract */
  daily(symbol: string, from?: string, to?: string, afterHours?: boolean): Promise<FutOptDailyResponse>;
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
   * Get historical candles for a futures/options contract
   *
   * @param symbol - Contract symbol (e.g., "TXFC4")
   * @param from - Start date (YYYY-MM-DD)
   * @param to - End date (YYYY-MM-DD)
   * @param timeframe - Timeframe ("D", "W", "M", "1", "5", etc.)
   * @param afterHours - Include after-hours data
   * @returns Promise resolving to historical candles data
   */
  candles(symbol: string, from?: string | undefined | null, to?: string | undefined | null, timeframe?: string | undefined | null, afterHours?: boolean | undefined | null): Promise<FutOptHistoricalCandlesResponse>
  /**
   * Get daily historical data for a futures/options contract
   *
   * @param symbol - Contract symbol (e.g., "TXFC4")
   * @param from - Start date (YYYY-MM-DD)
   * @param to - End date (YYYY-MM-DD)
   * @param afterHours - Include after-hours data
   * @returns Promise resolving to daily historical data
   */
  daily(symbol: string, from?: string | undefined | null, to?: string | undefined | null, afterHours?: boolean | undefined | null): Promise<FutOptDailyResponse>
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
  quote(symbol: string): Promise<QuoteResponse>
  /**
   * Get intraday ticker for a futures/options contract
   *
   * @param symbol - Contract symbol (e.g., "TXFC4")
   * @returns Promise resolving to Ticker object with last trade info
   */
  ticker(symbol: string): Promise<TickerResponse>
  /**
   * Get intraday candles for a futures/options contract
   *
   * @param symbol - Contract symbol (e.g., "TXFC4")
   * @param timeframe - Candle timeframe: "1", "5", "10", "15", "30", "60" (minutes)
   * @returns Promise resolving to Candles response with OHLCV data
   */
  candles(symbol: string, timeframe: string): Promise<CandlesResponse>
  /**
   * Get intraday trades for a futures/options contract
   *
   * @param symbol - Contract symbol (e.g., "TXFC4")
   * @returns Promise resolving to Trades response with recent trade history
   */
  trades(symbol: string): Promise<TradesResponse>
  /**
   * Get intraday volumes for a futures/options contract
   *
   * @param symbol - Contract symbol (e.g., "TXFC4")
   * @returns Promise resolving to Volumes response with volume at each price level
   */
  volumes(symbol: string): Promise<VolumesResponse>
  /**
   * Get batch ticker list for a FutOpt contract type
   *
   * @param type - Contract type: "FUTURE" or "OPTION"
   * @param exchange - Optional exchange filter (e.g., "TAIFEX")
   * @param afterHours - Query after-hours session data
   * @param contractType - Optional contract type code: "I" / "R" / "B" / "C" / "S" / "E"
   * @returns Promise resolving to an array of FutOpt ticker info objects
   */
  tickers(type: FutOptType, exchange?: string, afterHours?: boolean, contractType?: ContractType): Promise<FutOptTickerResponse[]>
  /**
   * Get product list for futures/options
   *
   * @param typ - Type: "FUTURE" or "OPTION" (required)
   * @param contractType - Contract type filter (optional): "I" (index), "R" (rate), "B" (bond), "C" (currency), "S" (stock), "E" (ETF)
   * @returns Promise resolving to Products response with available contracts
   */
  products(type: FutOptType, contractType?: ContractType): Promise<ProductsResponse>
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
   * @param event - Event type: "message", "connect", "disconnect", "reconnect", "error"
   * @param callback - JavaScript callback function receiving string data
   */
  on(event: WebSocketEvent, callback: (data: string) => void): void
  /**
   * Connect to the FutOpt WebSocket server.
   *
   * Returns a Promise that resolves when authentication completes.
   * See `StockWebSocketClient::connect` for the rationale and example.
   */
  connect(): Promise<void>
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
   * Accepts either `{ id: "..." }` (single) or `{ ids: ["...", "..."] }` (batch).
   */
  unsubscribe(options: string | UnsubscribeOptions): void
  /**
   * Send a `ping` frame to the server.
   *
   * Mirrors the old `@fugle/marketdata` Node SDK. The server's `pong` reply
   * is delivered via the `message` callback (or processed internally by the
   * health check, if enabled).
   *
   * @param state - Optional state string echoed back in the server's pong reply
   */
  ping(state?: string | undefined | null): void
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
  /** Check if connected */
  get isConnected(): boolean
  /**
   * Check if client has been closed
   *
   * Returns true if disconnect() has been called and client is closed.
   * Once closed, the client cannot be reused - create a new instance.
   */
  get isClosed(): boolean
}

/**
 * REST client for Fugle market data API (JavaScript wrapper)
 *
 * # JavaScript Usage
 *
 * ```javascript
 * const { RestClient } = require('@fubon/marketdata-js');
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
}

/** Stock corporate actions client */
export declare class StockCorporateActionsClient {
  /**
   * Get capital changes (capital structure changes)
   *
   * @param date - Specific date (YYYY-MM-DD)
   * @param startDate - Start date for range query (YYYY-MM-DD)
   * @param endDate - End date for range query (YYYY-MM-DD)
   * @returns Promise resolving to capital changes data
   */
  capitalChanges(date?: string | undefined | null, startDate?: string | undefined | null, endDate?: string | undefined | null): Promise<CapitalChangesResponse>
  /**
   * Get dividend announcements
   *
   * @param date - Specific date (YYYY-MM-DD)
   * @param startDate - Start date for range query (YYYY-MM-DD)
   * @param endDate - End date for range query (YYYY-MM-DD)
   * @returns Promise resolving to dividends data
   */
  dividends(date?: string | undefined | null, startDate?: string | undefined | null, endDate?: string | undefined | null): Promise<DividendsResponse>
  /**
   * Get IPO listing applicants
   *
   * @param date - Specific date (YYYY-MM-DD)
   * @param startDate - Start date for range query (YYYY-MM-DD)
   * @param endDate - End date for range query (YYYY-MM-DD)
   * @returns Promise resolving to listing applicants data
   */
  listingApplicants(date?: string | undefined | null, startDate?: string | undefined | null, endDate?: string | undefined | null): Promise<ListingApplicantsResponse>
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
  candles(symbol: string, from?: string | undefined | null, to?: string | undefined | null, timeframe?: string | undefined | null): Promise<HistoricalCandlesResponse>
  /**
   * Get historical stats for a stock symbol
   *
   * @param symbol - Stock symbol (e.g., "2330")
   * @returns Promise resolving to historical stats data
   */
  stats(symbol: string): Promise<StatsResponse>
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
   * await client.stock.intraday.quote({ symbol: '2330', oddLot: true });
   *
   * // Positional shape
   * await client.stock.intraday.quote('2330');
   * await client.stock.intraday.quote('2330', true);
   * ```
   */
  quote(symbol: string | StockIntradayQuoteParams, oddLot?: boolean | undefined | null): Promise<QuoteResponse>
  /**
   * Get intraday ticker for a stock symbol
   *
   * @param symbol - Stock symbol (e.g., "2330" for TSMC)
   * @returns Promise resolving to Ticker object with last trade info
   */
  ticker(symbol: string): Promise<TickerResponse>
  /**
   * Get intraday candles for a stock symbol
   *
   * @param symbol - Stock symbol (e.g., "2330" for TSMC)
   * @param timeframe - Candle timeframe: "1", "5", "10", "15", "30", "60" (minutes)
   * @returns Promise resolving to Candles response with OHLCV data
   */
  candles(symbol: string, timeframe: string): Promise<CandlesResponse>
  /**
   * Get intraday trades for a stock symbol
   *
   * @param symbol - Stock symbol (e.g., "2330" for TSMC)
   * @returns Promise resolving to Trades response with recent trade history
   */
  trades(symbol: string): Promise<TradesResponse>
  /**
   * Get intraday volumes for a stock symbol
   *
   * @param symbol - Stock symbol (e.g., "2330" for TSMC)
   * @returns Promise resolving to Volumes response with volume at each price level
   */
  volumes(symbol: string): Promise<VolumesResponse>
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
  tickers(r#type: string, exchange?: string | undefined | null, market?: string | undefined | null, industry?: string | undefined | null, isNormal?: boolean | undefined | null): Promise<TickerResponse[]>
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
  quotes(market: string, typeFilter?: string | undefined | null): Promise<SnapshotQuotesResponse>
  /**
   * Get movers (top gainers/losers) for a market
   *
   * @param market - Market code (e.g., "TSE", "OTC")
   * @param direction - Direction filter ("up" or "down")
   * @param change - Change type ("percent" or "value")
   * @returns Promise resolving to movers data
   */
  movers(market: string, direction?: string | undefined | null, change?: string | undefined | null): Promise<MoversResponse>
  /**
   * Get most actively traded stocks for a market
   *
   * @param market - Market code (e.g., "TSE", "OTC")
   * @param trade - Trade type filter ("volume" or "value")
   * @returns Promise resolving to actives data
   */
  actives(market: string, trade?: string | undefined | null): Promise<ActivesResponse>
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
  sma(symbol: string, from?: string | undefined | null, to?: string | undefined | null, timeframe?: string | undefined | null, period?: number | undefined | null): Promise<SmaResponse>
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
  rsi(symbol: string, from?: string | undefined | null, to?: string | undefined | null, timeframe?: string | undefined | null, period?: number | undefined | null): Promise<RsiResponse>
  /**
   * Get KDJ (Stochastic Oscillator) for a stock
   *
   * @param symbol - Stock symbol (e.g., "2330")
   * @param from - Start date (YYYY-MM-DD)
   * @param to - End date (YYYY-MM-DD)
   * @param timeframe - Timeframe ("D", "W", "M")
   * @param period - KDJ period (e.g., 9)
   * @returns Promise resolving to KDJ data
   */
  kdj(symbol: string, from?: string | undefined | null, to?: string | undefined | null, timeframe?: string | undefined | null, period?: number | undefined | null): Promise<KdjResponse>
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
  macd(symbol: string, from?: string | undefined | null, to?: string | undefined | null, timeframe?: string | undefined | null, fast?: number | undefined | null, slow?: number | undefined | null, signal?: number | undefined | null): Promise<MacdResponse>
  /**
   * Get Bollinger Bands for a stock
   *
   * @param symbol - Stock symbol (e.g., "2330")
   * @param from - Start date (YYYY-MM-DD)
   * @param to - End date (YYYY-MM-DD)
   * @param timeframe - Timeframe ("D", "W", "M")
   * @param period - SMA period (default: 20)
   * @param stddev - Standard deviation multiplier (default: 2.0)
   * @returns Promise resolving to Bollinger Bands data
   */
  bb(symbol: string, from?: string | undefined | null, to?: string | undefined | null, timeframe?: string | undefined | null, period?: number | undefined | null, stddev?: number | undefined | null): Promise<BbResponse>
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
   * @param event - Event type: "message", "connect", "disconnect", "reconnect", "error"
   * @param callback - JavaScript callback function receiving string data
   *
   * @example
   * ```javascript
   * ws.stock.on('message', (data) => console.log(data));
   * ws.stock.on('connect', () => console.log('Connected'));
   * ws.stock.on('error', (err) => console.error(err));
   * ```
   */
  on(event: WebSocketEvent, callback: (data: string) => void): void
  /**
   * Connect to the stock WebSocket server.
   *
   * Returns a Promise that resolves when authentication completes, matching
   * the legacy fugle-marketdata Node SDK shape:
   *
   * ```js
   * stock.connect().then(() => {
   *   stock.subscribe({ channel: 'trades', symbol: '2330' });
   * });
   * ```
   *
   * On rejection, the Promise carries the underlying error message. The
   * `connect` event callback also fires after the Promise resolves, so
   * existing callback-style code keeps working.
   */
  connect(): Promise<void>
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
   * Accepts either `{ id: "..." }` (single) or `{ ids: ["...", "..."] }` (batch).
   * Mirrors the old `@fugle/marketdata` Node SDK shape.
   */
  unsubscribe(options: string | UnsubscribeOptions): void
  /**
   * Send a `ping` frame to the server.
   *
   * Mirrors the old `@fugle/marketdata` Node SDK. The server's `pong` reply
   * is delivered via the `message` callback (or processed internally by the
   * health check, if enabled).
   *
   * @param state - Optional state string echoed back in the server's pong reply
   */
  ping(state?: string | undefined | null): void
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
  /** Check if connected */
  get isConnected(): boolean
  /**
   * Check if client has been closed
   *
   * Returns true if disconnect() has been called and client is closed.
   * Once closed, the client cannot be reused - create a new instance.
   */
  get isClosed(): boolean
}

/**
 * WebSocket client for real-time market data (JavaScript wrapper)
 *
 * # JavaScript Usage
 *
 * ```javascript
 * const { WebSocketClient } = require('@fubon/marketdata-js');
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
   *   healthCheck: { enabled: true, pingInterval: 20000 }
   * });
   * ```
   */
  constructor(options: WebSocketClientOptions)
  /**
   * Get the stock WebSocket client for real-time stock data.
   *
   * Every access returns a new JS wrapper but all wrappers share the same
   * underlying state (callbacks, connected flag, command channel), so the
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

/**
 * Health check options for WebSocket connections
 *
 * All fields are optional - defaults are applied when not specified:
 * - enabled: false
 * - pingInterval: 30000
 * - maxMissedPongs: 2
 *
 * Note: `pingInterval` is named to match the official `@fugle/marketdata` SDK.
 */
export interface HealthCheckOptions {
  /** Whether health check is enabled (default: false) */
  enabled?: boolean
  /** Interval between ping messages in milliseconds (default: 30000, min: 5000) */
  pingInterval?: number
  /** Maximum missed pongs before disconnect (default: 2, min: 1) */
  maxMissedPongs?: number
}

/**
 * Reconnection options for WebSocket clients
 *
 * All fields are optional - defaults are applied when not specified:
 * - maxAttempts: 5
 * - initialDelayMs: 1000
 * - maxDelayMs: 60000
 */
export interface ReconnectOptions {
  /** Maximum reconnection attempts (default: 5, min: 1) */
  maxAttempts?: number
  /** Initial reconnection delay in milliseconds (default: 1000, min: 100) */
  initialDelayMs?: number
  /** Maximum reconnection delay in milliseconds (default: 60000) */
  maxDelayMs?: number
}

/**
 * REST client options
 *
 * Exactly ONE of apiKey, bearerToken, or sdkToken must be provided.
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

/** Stock intraday quote params (object form) */
export interface StockIntradayQuoteParams {
  symbol: string
  oddLot?: boolean
}

/** Plain `{ symbol }` params reused by methods that take only a symbol. */
export interface SymbolParams {
  symbol: string
}

/**
 * WebSocket client options
 *
 * Exactly ONE of apiKey, bearerToken, or sdkToken must be provided.
 * reconnect and healthCheck are optional configuration objects.
 */
export interface WebSocketClientOptions {
  /** API key for authentication */
  apiKey?: string
  /** Bearer token for authentication */
  bearerToken?: string
  /** SDK token for authentication */
  sdkToken?: string
  /** Override base URL (optional) */
  baseUrl?: string
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
}
