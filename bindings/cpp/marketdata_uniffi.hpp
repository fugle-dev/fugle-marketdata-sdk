#pragma once

#include <algorithm>
#include <bit>
#include <chrono>
#include <cstdint>
#include <exception>
#include <functional>
#include <iostream>
#include <map>
#include <memory>
#include <mutex>
#include <optional>
#include <stdexcept>
#include <streambuf>
#include <type_traits>
#include <variant>
#include <vector>

#include "marketdata_uniffi_scaffolding.hpp"

namespace marketdata_uniffi {
struct FutOptClient;
struct FutOptHistoricalClient;
struct FutOptIntradayClient;
struct RestClient;
struct StockClient;
struct StockCorporateActionsClient;
struct StockHistoricalClient;
struct StockIntradayClient;
struct StockSnapshotClient;
struct StockTechnicalClient;
struct WebSocketClient;
struct WebSocketListener;
struct Active;
struct ActivesResponse;
struct BbDataPoint;
struct BbResponse;
struct CapitalChange;
struct CapitalChangesResponse;
struct Dividend;
struct DividendsResponse;
struct FutOptDailyData;
struct FutOptDailyResponse;
struct FutOptHistoricalCandle;
struct FutOptHistoricalCandlesResponse;
struct FutOptLastTrade;
struct FutOptPriceLevel;
struct FutOptQuote;
struct FutOptTicker;
struct FutOptTotalStats;
struct HealthCheckConfigRecord;
struct HistoricalCandle;
struct HistoricalCandlesResponse;
struct IntradayCandle;
struct IntradayCandlesResponse;
struct KdjDataPoint;
struct KdjResponse;
struct ListingApplicant;
struct ListingApplicantsResponse;
struct MacdDataPoint;
struct MacdResponse;
struct Mover;
struct MoversResponse;
struct PriceLevel;
struct Product;
struct ProductsResponse;
struct Quote;
struct ReconnectConfigRecord;
struct RsiDataPoint;
struct RsiResponse;
struct SmaDataPoint;
struct SmaResponse;
struct SnapshotQuote;
struct SnapshotQuotesResponse;
struct StatsResponse;
struct StreamMessage;
struct Ticker;
struct TlsConfigRecord;
struct TotalStats;
struct Trade;
struct TradeInfo;
struct TradesResponse;
struct TradingHalt;
struct VolumeAtPrice;
struct VolumesResponse;
struct MarketDataError;
enum class WebSocketEndpoint;


/**
 * FutOpt last trade info
 */
struct FutOptLastTrade {
    double price;
    int64_t size;
    int64_t time;
};


/**
 * FutOpt daily data
 */
struct FutOptDailyData {
    std::string date;
    double open;
    double high;
    double low;
    double close;
    uint64_t volume;
    std::optional<uint64_t> open_interest;
    std::optional<double> settlement_price;
};


/**
 * Bid/Ask price level for order book
 */
struct PriceLevel {
    double price;
    int64_t size;
};


/**
 * FutOpt product
 */
struct Product {
    std::optional<std::string> product_type;
    std::optional<std::string> exchange;
    std::string symbol;
    std::optional<std::string> name;
    std::optional<std::string> underlying_symbol;
    std::optional<std::string> contract_type;
    std::optional<int64_t> contract_size;
    std::optional<std::string> underlying_type;
    std::optional<std::string> status_code;
    std::optional<std::string> trading_currency;
    bool quote_acceptable;
    bool can_block_trade;
    std::optional<std::string> start_date;
    std::optional<std::string> expiry_type;
    std::optional<int32_t> market_close_group;
    std::optional<int32_t> end_session;
};


/**
 * FutOpt total stats
 */
struct FutOptTotalStats {
    int64_t trade_volume;
    std::optional<int64_t> total_bid_match;
    std::optional<int64_t> total_ask_match;
};


/**
 * Dividend entry
 */
struct Dividend {
    std::string symbol;
    std::optional<std::string> name;
    std::optional<std::string> ex_dividend_date;
    std::optional<std::string> payment_date;
    std::optional<double> cash_dividend;
    std::optional<double> stock_dividend;
    std::optional<std::string> dividend_year;
};


/**
 * Single intraday candle
 */
struct IntradayCandle {
    double open;
    double high;
    double low;
    double close;
    int64_t volume;
    std::optional<double> average;
    std::string date;
};


/**
 * Capital change entry
 */
struct CapitalChange {
    std::string symbol;
    std::optional<std::string> name;
    std::string date;
    std::optional<double> previous_capital;
    std::optional<double> current_capital;
    std::optional<std::string> change_type;
    std::optional<std::string> reason;
};


/**
 * Total trading statistics
 */
struct TotalStats {
    double trade_value;
    int64_t trade_volume;
    std::optional<int64_t> trade_volume_at_bid;
    std::optional<int64_t> trade_volume_at_ask;
    std::optional<int64_t> transaction;
    std::optional<int64_t> time;
};


/**
 * Single snapshot quote
 */
struct SnapshotQuote {
    std::optional<std::string> data_type;
    std::string symbol;
    std::optional<std::string> name;
    std::optional<double> open_price;
    std::optional<double> high_price;
    std::optional<double> low_price;
    std::optional<double> close_price;
    std::optional<double> change;
    std::optional<double> change_percent;
    std::optional<int64_t> trade_volume;
    std::optional<double> trade_value;
    std::optional<int64_t> last_updated;
};


/**
 * Listing applicant entry
 */
struct ListingApplicant {
    std::string symbol;
    std::optional<std::string> name;
    std::optional<std::string> application_date;
    std::optional<std::string> listing_date;
    std::optional<std::string> status;
    std::optional<std::string> industry;
};


/**
 * RSI data point
 */
struct RsiDataPoint {
    std::string date;
    double rsi;
};


/**
 * Trade execution info
 */
struct TradeInfo {
    std::optional<double> bid;
    std::optional<double> ask;
    double price;
    int64_t size;
    int64_t time;
};


/**
 * SMA data point
 */
struct SmaDataPoint {
    std::string date;
    double sma;
};


/**
 * Single trade execution
 */
struct Trade {
    std::optional<double> bid;
    std::optional<double> ask;
    double price;
    int64_t size;
    int64_t time;
};


/**
 * Bollinger Bands data point
 */
struct BbDataPoint {
    std::string date;
    double upper;
    double middle;
    double lower;
};


/**
 * Single historical candle
 */
struct HistoricalCandle {
    std::string date;
    double open;
    double high;
    double low;
    double close;
    int64_t volume;
    std::optional<double> turnover;
    std::optional<double> change;
};


/**
 * MACD data point
 */
struct MacdDataPoint {
    std::string date;
    double macd;
    double signal_value;
    double histogram;
};


/**
 * Trading halt status
 */
struct TradingHalt {
    bool is_halted;
    std::optional<int64_t> time;
};


/**
 * Volume at a specific price level
 */
struct VolumeAtPrice {
    double price;
    int64_t volume;
    std::optional<int64_t> volume_at_bid;
    std::optional<int64_t> volume_at_ask;
};


/**
 * Single active entry
 */
struct Active {
    std::optional<std::string> data_type;
    std::string symbol;
    std::optional<std::string> name;
    std::optional<double> open_price;
    std::optional<double> high_price;
    std::optional<double> low_price;
    std::optional<double> close_price;
    std::optional<double> change;
    std::optional<double> change_percent;
    std::optional<int64_t> trade_volume;
    std::optional<double> trade_value;
    std::optional<int64_t> last_updated;
};


/**
 * Single mover entry
 */
struct Mover {
    std::optional<std::string> data_type;
    std::string symbol;
    std::optional<std::string> name;
    std::optional<double> open_price;
    std::optional<double> high_price;
    std::optional<double> low_price;
    std::optional<double> close_price;
    std::optional<double> change;
    std::optional<double> change_percent;
    std::optional<int64_t> trade_volume;
    std::optional<double> trade_value;
    std::optional<int64_t> last_updated;
};


/**
 * FutOpt price level
 */
struct FutOptPriceLevel {
    double price;
    int64_t size;
};


/**
 * FutOpt historical candle
 */
struct FutOptHistoricalCandle {
    std::string date;
    double open;
    double high;
    double low;
    double close;
    uint64_t volume;
    std::optional<uint64_t> open_interest;
    std::optional<double> change;
    std::optional<double> change_percent;
};


/**
 * KDJ data point
 */
struct KdjDataPoint {
    std::string date;
    double k;
    double d;
    double j;
};


/**
 * KDJ response
 */
struct KdjResponse {
    std::string symbol;
    std::string data_type;
    std::string exchange;
    std::string market;
    std::string timeframe;
    uint32_t period;
    std::vector<KdjDataPoint> data;
};


/**
 * FutOpt products response
 */
struct ProductsResponse {
    std::optional<std::string> date;
    std::optional<std::string> product_type;
    std::optional<std::string> session;
    std::optional<std::string> contract_type;
    std::optional<std::string> status;
    std::vector<Product> data;
};


/**
 * RSI response
 */
struct RsiResponse {
    std::string symbol;
    std::string data_type;
    std::string exchange;
    std::string market;
    std::string timeframe;
    uint32_t period;
    std::vector<RsiDataPoint> data;
};


/**
 * Listing applicants response
 */
struct ListingApplicantsResponse {
    std::string data_type;
    std::string exchange;
    std::string market;
    std::vector<ListingApplicant> data;
};


/**
 * Volumes response
 */
struct VolumesResponse {
    std::string date;
    std::optional<std::string> data_type;
    std::optional<std::string> exchange;
    std::optional<std::string> market;
    std::string symbol;
    std::vector<VolumeAtPrice> data;
};


/**
 * Trades response
 */
struct TradesResponse {
    std::string date;
    std::optional<std::string> data_type;
    std::optional<std::string> exchange;
    std::optional<std::string> market;
    std::string symbol;
    std::vector<Trade> data;
};


/**
 * Intraday candles response
 */
struct IntradayCandlesResponse {
    std::string date;
    std::optional<std::string> data_type;
    std::optional<std::string> exchange;
    std::optional<std::string> market;
    std::string symbol;
    std::optional<std::string> timeframe;
    std::vector<IntradayCandle> data;
};


/**
 * SMA response
 */
struct SmaResponse {
    std::string symbol;
    std::string data_type;
    std::string exchange;
    std::string market;
    std::string timeframe;
    uint32_t period;
    std::vector<SmaDataPoint> data;
};


/**
 * Historical candles response
 */
struct HistoricalCandlesResponse {
    std::string symbol;
    std::optional<std::string> data_type;
    std::optional<std::string> exchange;
    std::optional<std::string> market;
    std::optional<std::string> timeframe;
    std::optional<bool> adjusted;
    std::vector<HistoricalCandle> data;
};


/**
 * Movers response
 */
struct MoversResponse {
    std::string date;
    std::string time;
    std::string market;
    std::vector<Mover> data;
};


/**
 * FutOpt daily response
 */
struct FutOptDailyResponse {
    std::string symbol;
    std::optional<std::string> data_type;
    std::optional<std::string> exchange;
    std::vector<FutOptDailyData> data;
};


/**
 * FutOpt quote
 */
struct FutOptQuote {
    std::string date;
    std::optional<std::string> contract_type;
    std::optional<std::string> exchange;
    std::string symbol;
    std::optional<std::string> name;
    std::optional<double> previous_close;
    std::optional<double> open_price;
    std::optional<int64_t> open_time;
    std::optional<double> high_price;
    std::optional<int64_t> high_time;
    std::optional<double> low_price;
    std::optional<int64_t> low_time;
    std::optional<double> close_price;
    std::optional<int64_t> close_time;
    std::optional<double> last_price;
    std::optional<int64_t> last_size;
    std::optional<double> avg_price;
    std::optional<double> change;
    std::optional<double> change_percent;
    std::optional<double> amplitude;
    std::vector<FutOptPriceLevel> bids;
    std::vector<FutOptPriceLevel> asks;
    std::optional<FutOptTotalStats> total;
    std::optional<FutOptLastTrade> last_trade;
    std::optional<int64_t> last_updated;
};


/**
 * Snapshot quotes response
 */
struct SnapshotQuotesResponse {
    std::string date;
    std::string time;
    std::string market;
    std::vector<SnapshotQuote> data;
};


/**
 * Bollinger Bands response
 */
struct BbResponse {
    std::string symbol;
    std::string data_type;
    std::string exchange;
    std::string market;
    std::string timeframe;
    uint32_t period;
    double stddev;
    std::vector<BbDataPoint> data;
};


/**
 * Dividends response
 */
struct DividendsResponse {
    std::string data_type;
    std::string exchange;
    std::string market;
    std::vector<Dividend> data;
};


/**
 * Capital changes response
 */
struct CapitalChangesResponse {
    std::string data_type;
    std::string exchange;
    std::string market;
    std::vector<CapitalChange> data;
};


/**
 * Actives response
 */
struct ActivesResponse {
    std::string date;
    std::string time;
    std::string market;
    std::vector<Active> data;
};


/**
 * FutOpt historical candles response
 */
struct FutOptHistoricalCandlesResponse {
    std::string symbol;
    std::optional<std::string> data_type;
    std::optional<std::string> exchange;
    std::optional<std::string> timeframe;
    std::vector<FutOptHistoricalCandle> candles;
};


/**
 * MACD response
 */
struct MacdResponse {
    std::string symbol;
    std::string data_type;
    std::string exchange;
    std::string market;
    std::string timeframe;
    uint32_t fast;
    uint32_t slow;
    uint32_t signal;
    std::vector<MacdDataPoint> data;
};


/**
 * Real-time stock quote
 */
struct Quote {
    std::string date;
    std::optional<std::string> data_type;
    std::optional<std::string> exchange;
    std::optional<std::string> market;
    std::string symbol;
    std::optional<std::string> name;
    std::optional<double> open_price;
    std::optional<int64_t> open_time;
    std::optional<double> high_price;
    std::optional<int64_t> high_time;
    std::optional<double> low_price;
    std::optional<int64_t> low_time;
    std::optional<double> close_price;
    std::optional<int64_t> close_time;
    std::optional<double> last_price;
    std::optional<int64_t> last_size;
    std::optional<double> avg_price;
    std::optional<double> change;
    std::optional<double> change_percent;
    std::optional<double> amplitude;
    std::vector<PriceLevel> bids;
    std::vector<PriceLevel> asks;
    std::optional<TotalStats> total;
    std::optional<TradeInfo> last_trade;
    std::optional<TradeInfo> last_trial;
    std::optional<TradingHalt> trading_halt;
    bool is_limit_down_price;
    bool is_limit_up_price;
    bool is_limit_down_bid;
    bool is_limit_up_bid;
    bool is_limit_down_ask;
    bool is_limit_up_ask;
    bool is_limit_down_halt;
    bool is_limit_up_halt;
    bool is_trial;
    bool is_delayed_open;
    bool is_delayed_close;
    bool is_continuous;
    bool is_open;
    bool is_close;
    std::optional<int64_t> last_updated;
};


namespace uniffi {
    struct FfiConverterFutOptClient;
} // namespace uniffi

/**
 * FutOpt market data client
 */
struct FutOptClient



{
    friend uniffi::FfiConverterFutOptClient;

    FutOptClient() = delete;

    FutOptClient(FutOptClient &&) = delete;

    FutOptClient &operator=(const FutOptClient &) = delete;
    FutOptClient &operator=(FutOptClient &&) = delete;

    ~FutOptClient();
    /**
     * Access historical data endpoints
     */
    std::shared_ptr<FutOptHistoricalClient> historical();
    /**
     * Access intraday (real-time) endpoints
     */
    std::shared_ptr<FutOptIntradayClient> intraday();

    private:
    FutOptClient(const FutOptClient &);

    FutOptClient(void *);

    void *_uniffi_internal_clone_pointer() const;

    void *instance = nullptr;
};


namespace uniffi {
    struct FfiConverterFutOptHistoricalClient;
} // namespace uniffi

/**
 * FutOpt historical data endpoints
 *
 * Provides access to historical candles and daily data for futures and options.
 */
struct FutOptHistoricalClient



{
    friend uniffi::FfiConverterFutOptHistoricalClient;

    FutOptHistoricalClient() = delete;

    FutOptHistoricalClient(FutOptHistoricalClient &&) = delete;

    FutOptHistoricalClient &operator=(const FutOptHistoricalClient &) = delete;
    FutOptHistoricalClient &operator=(FutOptHistoricalClient &&) = delete;

    ~FutOptHistoricalClient();

    private:
    FutOptHistoricalClient(const FutOptHistoricalClient &);

    FutOptHistoricalClient(void *);

    void *_uniffi_internal_clone_pointer() const;

    void *instance = nullptr;
};


namespace uniffi {
    struct FfiConverterFutOptIntradayClient;
} // namespace uniffi

/**
 * FutOpt intraday endpoints with typed model returns
 */
struct FutOptIntradayClient



{
    friend uniffi::FfiConverterFutOptIntradayClient;

    FutOptIntradayClient() = delete;

    FutOptIntradayClient(FutOptIntradayClient &&) = delete;

    FutOptIntradayClient &operator=(const FutOptIntradayClient &) = delete;
    FutOptIntradayClient &operator=(FutOptIntradayClient &&) = delete;

    ~FutOptIntradayClient();

    private:
    FutOptIntradayClient(const FutOptIntradayClient &);

    FutOptIntradayClient(void *);

    void *_uniffi_internal_clone_pointer() const;

    void *instance = nullptr;
};


namespace uniffi {
    struct FfiConverterRestClient;
} // namespace uniffi

/**
 * REST client for UniFFI bindings
 *
 * Wraps the core RestClient and provides Arc-wrapped sub-clients for FFI safety.
 */
struct RestClient



{
    friend uniffi::FfiConverterRestClient;

    RestClient() = delete;

    RestClient(RestClient &&) = delete;

    RestClient &operator=(const RestClient &) = delete;
    RestClient &operator=(RestClient &&) = delete;

    ~RestClient();
    /**
     * Access FutOpt (futures and options) endpoints
     */
    std::shared_ptr<FutOptClient> futopt();
    /**
     * Access stock-related endpoints
     */
    std::shared_ptr<StockClient> stock();

    private:
    RestClient(const RestClient &);

    RestClient(void *);

    void *_uniffi_internal_clone_pointer() const;

    void *instance = nullptr;
};


namespace uniffi {
    struct FfiConverterStockClient;
} // namespace uniffi

/**
 * Stock market data client
 */
struct StockClient



{
    friend uniffi::FfiConverterStockClient;

    StockClient() = delete;

    StockClient(StockClient &&) = delete;

    StockClient &operator=(const StockClient &) = delete;
    StockClient &operator=(StockClient &&) = delete;

    ~StockClient();
    /**
     * Access corporate actions endpoints
     */
    std::shared_ptr<StockCorporateActionsClient> corporate_actions();
    /**
     * Access historical data endpoints
     */
    std::shared_ptr<StockHistoricalClient> historical();
    /**
     * Access intraday (real-time) endpoints
     */
    std::shared_ptr<StockIntradayClient> intraday();
    /**
     * Access snapshot (market-wide) endpoints
     */
    std::shared_ptr<StockSnapshotClient> snapshot();
    /**
     * Access technical indicator endpoints
     */
    std::shared_ptr<StockTechnicalClient> technical();

    private:
    StockClient(const StockClient &);

    StockClient(void *);

    void *_uniffi_internal_clone_pointer() const;

    void *instance = nullptr;
};


namespace uniffi {
    struct FfiConverterStockCorporateActionsClient;
} // namespace uniffi

/**
 * Stock corporate actions endpoints
 *
 * Provides access to capital changes, dividends, and listing applicants (IPO).
 */
struct StockCorporateActionsClient



{
    friend uniffi::FfiConverterStockCorporateActionsClient;

    StockCorporateActionsClient() = delete;

    StockCorporateActionsClient(StockCorporateActionsClient &&) = delete;

    StockCorporateActionsClient &operator=(const StockCorporateActionsClient &) = delete;
    StockCorporateActionsClient &operator=(StockCorporateActionsClient &&) = delete;

    ~StockCorporateActionsClient();

    private:
    StockCorporateActionsClient(const StockCorporateActionsClient &);

    StockCorporateActionsClient(void *);

    void *_uniffi_internal_clone_pointer() const;

    void *instance = nullptr;
};


namespace uniffi {
    struct FfiConverterStockHistoricalClient;
} // namespace uniffi

/**
 * Stock historical endpoints with typed model returns
 *
 * All methods have both async (get_*) and sync (*_sync) variants:
 * - Async methods are preferred for best performance (non-blocking)
 * - Sync methods block the calling thread (simpler API for scripting)
 */
struct StockHistoricalClient



{
    friend uniffi::FfiConverterStockHistoricalClient;

    StockHistoricalClient() = delete;

    StockHistoricalClient(StockHistoricalClient &&) = delete;

    StockHistoricalClient &operator=(const StockHistoricalClient &) = delete;
    StockHistoricalClient &operator=(StockHistoricalClient &&) = delete;

    ~StockHistoricalClient();

    private:
    StockHistoricalClient(const StockHistoricalClient &);

    StockHistoricalClient(void *);

    void *_uniffi_internal_clone_pointer() const;

    void *instance = nullptr;
};


namespace uniffi {
    struct FfiConverterStockIntradayClient;
} // namespace uniffi

/**
 * Stock intraday endpoints with typed model returns
 *
 * All methods have both async (get_*) and sync (*_sync) variants:
 * - Async methods are preferred for best performance (non-blocking)
 * - Sync methods block the calling thread (simpler API for scripting)
 */
struct StockIntradayClient



{
    friend uniffi::FfiConverterStockIntradayClient;

    StockIntradayClient() = delete;

    StockIntradayClient(StockIntradayClient &&) = delete;

    StockIntradayClient &operator=(const StockIntradayClient &) = delete;
    StockIntradayClient &operator=(StockIntradayClient &&) = delete;

    ~StockIntradayClient();

    private:
    StockIntradayClient(const StockIntradayClient &);

    StockIntradayClient(void *);

    void *_uniffi_internal_clone_pointer() const;

    void *instance = nullptr;
};


namespace uniffi {
    struct FfiConverterStockSnapshotClient;
} // namespace uniffi

/**
 * Stock snapshot endpoints for market-wide data
 *
 * Provides access to quotes, movers (gainers/losers), and most active stocks
 * across entire markets.
 */
struct StockSnapshotClient



{
    friend uniffi::FfiConverterStockSnapshotClient;

    StockSnapshotClient() = delete;

    StockSnapshotClient(StockSnapshotClient &&) = delete;

    StockSnapshotClient &operator=(const StockSnapshotClient &) = delete;
    StockSnapshotClient &operator=(StockSnapshotClient &&) = delete;

    ~StockSnapshotClient();

    private:
    StockSnapshotClient(const StockSnapshotClient &);

    StockSnapshotClient(void *);

    void *_uniffi_internal_clone_pointer() const;

    void *instance = nullptr;
};


namespace uniffi {
    struct FfiConverterStockTechnicalClient;
} // namespace uniffi

/**
 * Stock technical indicator endpoints
 *
 * Provides access to SMA, RSI, KDJ, MACD, and Bollinger Bands indicators.
 */
struct StockTechnicalClient



{
    friend uniffi::FfiConverterStockTechnicalClient;

    StockTechnicalClient() = delete;

    StockTechnicalClient(StockTechnicalClient &&) = delete;

    StockTechnicalClient &operator=(const StockTechnicalClient &) = delete;
    StockTechnicalClient &operator=(StockTechnicalClient &&) = delete;

    ~StockTechnicalClient();

    private:
    StockTechnicalClient(const StockTechnicalClient &);

    StockTechnicalClient(void *);

    void *_uniffi_internal_clone_pointer() const;

    void *instance = nullptr;
};


namespace uniffi {
    struct FfiConverterWebSocketClient;
} // namespace uniffi

/**
 * WebSocket client for real-time market data streaming
 *
 * Wraps the core WebSocketClient and forwards messages to the provided
 * WebSocketListener implementation via a background task.
 */
struct WebSocketClient



{
    friend uniffi::FfiConverterWebSocketClient;

    WebSocketClient() = delete;

    WebSocketClient(WebSocketClient &&) = delete;

    WebSocketClient &operator=(const WebSocketClient &) = delete;
    WebSocketClient &operator=(WebSocketClient &&) = delete;

    ~WebSocketClient();
    /**
     * Create a new WebSocket client for stock market data
     *
     * # Arguments
     * * `api_key` - Fugle API key for authentication
     * * `listener` - Callback interface for receiving WebSocket events
     */
    static std::shared_ptr<WebSocketClient> init(const std::string &api_key, const std::shared_ptr<WebSocketListener> &listener);
    /**
     * Create a new WebSocket client with full configuration
     *
     * # Arguments
     * * `api_key` - Fugle API key for authentication
     * * `listener` - Callback interface for receiving WebSocket events
     * * `endpoint` - The market data endpoint (Stock or FutOpt)
     * * `reconnect_config` - Optional reconnection configuration
     * * `health_check_config` - Optional health check configuration
     */
    static std::shared_ptr<WebSocketClient> new_with_config(const std::string &api_key, const std::shared_ptr<WebSocketListener> &listener, const WebSocketEndpoint &endpoint, std::optional<ReconnectConfigRecord> reconnect_config, std::optional<HealthCheckConfigRecord> health_check_config);
    /**
     * Create a new WebSocket client for a specific endpoint
     *
     * # Arguments
     * * `api_key` - Fugle API key for authentication
     * * `listener` - Callback interface for receiving WebSocket events
     * * `endpoint` - The market data endpoint (Stock or FutOpt)
     */
    static std::shared_ptr<WebSocketClient> new_with_endpoint(const std::string &api_key, const std::shared_ptr<WebSocketListener> &listener, const WebSocketEndpoint &endpoint);
    /**
     * Create a new WebSocket client with full configuration including TLS.
     *
     * All optional parameters can be None to use defaults. This is the
     * TLS-aware variant of `new_with_url` — use this when you need to
     * pin a custom CA or disable cert verification.
     *
     * # Arguments
     * * `api_key` - Fugle API key for authentication
     * * `listener` - Callback interface for receiving WebSocket events
     * * `endpoint` - The market data endpoint (Stock or FutOpt)
     * * `base_url` - Optional base URL override
     * * `reconnect_config` - Optional reconnection configuration
     * * `health_check_config` - Optional health check configuration
     * * `tls` - Optional TLS customization (custom CA or accept_invalid_certs)
     */
    static std::shared_ptr<WebSocketClient> new_with_full_config(const std::string &api_key, const std::shared_ptr<WebSocketListener> &listener, const WebSocketEndpoint &endpoint, std::optional<std::string> base_url, std::optional<ReconnectConfigRecord> reconnect_config, std::optional<HealthCheckConfigRecord> health_check_config, std::optional<TlsConfigRecord> tls);
    /**
     * Create a new WebSocket client with full configuration including custom base URL
     */
    static std::shared_ptr<WebSocketClient> new_with_url(const std::string &api_key, const std::shared_ptr<WebSocketListener> &listener, const WebSocketEndpoint &endpoint, const std::string &base_url, std::optional<ReconnectConfigRecord> reconnect_config, std::optional<HealthCheckConfigRecord> health_check_config);
    /**
     * Connect to the WebSocket server (blocking).
     */
    void connect_sync();
    /**
     * Disconnect from the WebSocket server (blocking).
     */
    void disconnect_sync();
    /**
     * Check if the client has been shut down
     */
    bool is_closed();
    /**
     * Check if the client is currently connected
     */
    bool is_connected();
    /**
     * Send a ping message (blocking).
     */
    void ping_sync(std::optional<std::string> state);
    /**
     * Query server subscriptions (blocking).
     */
    void query_subscriptions_sync();
    /**
     * Subscribe to a channel for a symbol (blocking).
     */
    void subscribe_sync(const std::string &channel, const std::string &symbol);
    /**
     * Unsubscribe from a channel for a symbol (blocking).
     */
    void unsubscribe_sync(const std::string &channel, const std::string &symbol);

    private:
    WebSocketClient(const WebSocketClient &);

    WebSocketClient(void *);

    void *_uniffi_internal_clone_pointer() const;

    void *instance = nullptr;
};




/**
 * Callback interface for WebSocket events
 *
 * Foreign code (C#, Go) implements this trait to receive WebSocket events.
 * The implementation must be thread-safe (Send + Sync) as callbacks may be
 * invoked from background tokio tasks.
 *
 * # Example (C#)
 *
 * ```csharp
 * class MyListener : IWebSocketListener {
 * public void OnConnected() {
 * Console.WriteLine("Connected!");
 * }
 * public void OnDisconnected() {
 * Console.WriteLine("Disconnected");
 * }
 * public void OnMessage(StreamMessage message) {
 * Console.WriteLine($"Got {message.Event} for {message.Symbol}");
 * }
 * public void OnError(string errorMessage) {
 * Console.WriteLine($"Error: {errorMessage}");
 * }
 * }
 * ```
 */
struct WebSocketListener {
    virtual ~WebSocketListener() {}
    /**
     * Called when WebSocket connection is established
     */
    virtual
    void on_connected() = 0;
    /**
     * Called when WebSocket connection is closed
     */
    virtual
    void on_disconnected() = 0;
    /**
     * Called when a message is received
     */
    virtual
    void on_message(const StreamMessage &message) = 0;
    /**
     * Called when an error occurs
     */
    virtual
    void on_error(const std::string &error_message) = 0;
    /**
     * Called when a reconnection attempt starts
     */
    virtual
    void on_reconnecting(uint32_t attempt) = 0;
    /**
     * Called when all reconnection attempts are exhausted
     */
    virtual
    void on_reconnect_failed(uint32_t attempts) = 0;
};

namespace uniffi {
    struct UniffiCallbackInterfaceWebSocketListener {
        static void on_connected(uint64_t uniffi_handle,void * uniffi_out_return,RustCallStatus *out_status);
        static void on_disconnected(uint64_t uniffi_handle,void * uniffi_out_return,RustCallStatus *out_status);
        static void on_message(uint64_t uniffi_handle,RustBuffer message,void * uniffi_out_return,RustCallStatus *out_status);
        static void on_error(uint64_t uniffi_handle,RustBuffer error_message,void * uniffi_out_return,RustCallStatus *out_status);
        static void on_reconnecting(uint64_t uniffi_handle,uint32_t attempt,void * uniffi_out_return,RustCallStatus *out_status);
        static void on_reconnect_failed(uint64_t uniffi_handle,uint32_t attempts,void * uniffi_out_return,RustCallStatus *out_status);

        static void uniffi_free(uint64_t uniffi_handle);
        static void init();
    private:
        static inline UniffiVTableCallbackInterfaceWebSocketListener vtable = UniffiVTableCallbackInterfaceWebSocketListener {
            .on_connected = reinterpret_cast<void *>(&on_connected),
            .on_disconnected = reinterpret_cast<void *>(&on_disconnected),
            .on_message = reinterpret_cast<void *>(&on_message),
            .on_error = reinterpret_cast<void *>(&on_error),
            .on_reconnecting = reinterpret_cast<void *>(&on_reconnecting),
            .on_reconnect_failed = reinterpret_cast<void *>(&on_reconnect_failed),
            .uniffi_free = reinterpret_cast<void *>(&uniffi_free)
        };
    };
}

namespace uniffi {
    struct FfiConverterWebSocketListener;
} // namespace uniffi

/**
 * Callback interface for WebSocket events
 *
 * Foreign code (C#, Go) implements this trait to receive WebSocket events.
 * The implementation must be thread-safe (Send + Sync) as callbacks may be
 * invoked from background tokio tasks.
 *
 * # Example (C#)
 *
 * ```csharp
 * class MyListener : IWebSocketListener {
 * public void OnConnected() {
 * Console.WriteLine("Connected!");
 * }
 * public void OnDisconnected() {
 * Console.WriteLine("Disconnected");
 * }
 * public void OnMessage(StreamMessage message) {
 * Console.WriteLine($"Got {message.Event} for {message.Symbol}");
 * }
 * public void OnError(string errorMessage) {
 * Console.WriteLine($"Error: {errorMessage}");
 * }
 * }
 * ```
 */
struct WebSocketListenerImpl

 : public WebSocketListener 

{
    friend uniffi::FfiConverterWebSocketListener;

    WebSocketListenerImpl() = delete;

    WebSocketListenerImpl(WebSocketListenerImpl &&) = delete;

    WebSocketListenerImpl &operator=(const WebSocketListenerImpl &) = delete;
    WebSocketListenerImpl &operator=(WebSocketListenerImpl &&) = delete;

    ~WebSocketListenerImpl();
    /**
     * Called when WebSocket connection is established
     */
    void on_connected();
    /**
     * Called when WebSocket connection is closed
     */
    void on_disconnected();
    /**
     * Called when a message is received
     */
    void on_message(const StreamMessage &message);
    /**
     * Called when an error occurs
     */
    void on_error(const std::string &error_message);
    /**
     * Called when a reconnection attempt starts
     */
    void on_reconnecting(uint32_t attempt);
    /**
     * Called when all reconnection attempts are exhausted
     */
    void on_reconnect_failed(uint32_t attempts);

    private:
    WebSocketListenerImpl(const WebSocketListenerImpl &);

    WebSocketListenerImpl(void *);

    void *_uniffi_internal_clone_pointer() const;

    void *instance = nullptr;
};


/**
 * FutOpt ticker
 */
struct FutOptTicker {
    std::string date;
    std::optional<std::string> contract_type;
    std::optional<std::string> exchange;
    std::string symbol;
    std::optional<std::string> name;
    std::optional<double> reference_price;
    std::optional<std::string> start_date;
    std::optional<std::string> end_date;
    std::optional<std::string> settlement_date;
    std::optional<std::string> contract_sub_type;
    bool is_dynamic_banding;
    std::optional<int32_t> flow_group;
};


/**
 * Health check configuration record for FFI
 *
 * All fields are optional — zero/false values mean "use default".
 */
struct HealthCheckConfigRecord {
    /**
     * Whether health check is enabled (default: false)
     */
    bool enabled;
    /**
     * Interval between ping messages in milliseconds (default: 30000, min: 5000)
     */
    uint64_t interval_ms;
    /**
     * Maximum missed pongs before disconnect (default: 2, min: 1)
     */
    uint64_t max_missed_pongs;
};


/**
 * Reconnection configuration record for FFI
 *
 * All fields are optional — zero/false values mean "use default".
 */
struct ReconnectConfigRecord {
    /**
     * Maximum reconnection attempts (default: 5, min: 1)
     */
    uint32_t max_attempts;
    /**
     * Initial reconnection delay in milliseconds (default: 1000, min: 100)
     */
    uint64_t initial_delay_ms;
    /**
     * Maximum reconnection delay in milliseconds (default: 60000)
     */
    uint64_t max_delay_ms;
};


/**
 * Historical stats response
 */
struct StatsResponse {
    std::string date;
    std::string data_type;
    std::string exchange;
    std::string market;
    std::string symbol;
    std::string name;
    double open_price;
    double high_price;
    double low_price;
    double close_price;
    double change;
    double change_percent;
    int64_t trade_volume;
    double trade_value;
    double previous_close;
    double week52_high;
    double week52_low;
};


/**
 * Streaming message (simplified for FFI)
 */
struct StreamMessage {
    std::string event;
    std::optional<std::string> channel;
    std::optional<std::string> symbol;
    std::optional<std::string> id;
    std::optional<std::string> data_json;
    std::optional<int32_t> error_code;
    std::optional<std::string> error_message;
};


/**
 * Stock ticker info
 */
struct Ticker {
    std::string date;
    std::optional<std::string> data_type;
    std::optional<std::string> exchange;
    std::optional<std::string> market;
    std::string symbol;
    std::optional<std::string> name;
    std::optional<std::string> name_en;
    std::optional<std::string> industry;
    std::optional<std::string> security_type;
    std::optional<double> reference_price;
    std::optional<double> limit_up_price;
    std::optional<double> limit_down_price;
    std::optional<double> previous_close;
    bool can_day_trade;
    bool can_buy_day_trade;
    bool can_below_flat_margin_short_sell;
    bool can_below_flat_sbl_short_sell;
    bool is_attention;
    bool is_disposition;
    bool is_unusually_recommended;
    bool is_specific_abnormally;
    bool is_newly_compiled;
    std::optional<int32_t> matching_interval;
    std::optional<std::string> security_status;
    std::optional<int32_t> board_lot;
    std::optional<std::string> trading_currency;
    std::optional<double> exercise_price;
    std::optional<int64_t> exercised_volume;
    std::optional<int64_t> cancelled_volume;
    std::optional<int64_t> remaining_volume;
    std::optional<double> exercise_ratio;
    std::optional<double> cap_price;
    std::optional<double> floor_price;
    std::optional<std::string> maturity_date;
    std::optional<std::string> open_time;
    std::optional<std::string> close_time;
};


/**
 * Optional TLS customization exposed to foreign languages.
 *
 * When all fields are default the SDK uses the OS trust store
 * (loaded by `rustls-native-certs`). Provide `root_cert_pem` to pin
 * an additional CA, or set `accept_invalid_certs` to disable all
 * verification (dev/testing only — exposes MITM risk).
 */
struct TlsConfigRecord {
    /**
     * PEM-encoded additional root CA bytes. Appended to the OS trust
     * store; chains signed by either this CA or any OS-trusted root
     * are accepted.
     */
    std::optional<std::vector<uint8_t>> root_cert_pem;
    /**
     * Disable ALL TLS verification (chain + hostname + expiry).
     * Equivalent to `curl -k` / `wscat --no-check`. Do not use in
     * production.
     */
    bool accept_invalid_certs;
};

namespace uniffi {
struct FfiConverterMarketDataError;
} // namespace uniffi

/**
 * Error type for UniFFI bindings
 *
 * Maps to MarketDataError in the UDL file. Each variant becomes an exception
 * in the target language with the error message preserved.
 *
 * Note: This is a FLAT enum per UniFFI constraints - no nested error types.
 */
struct MarketDataError: std::runtime_error {
    friend uniffi::FfiConverterMarketDataError;

    MarketDataError() : std::runtime_error("") {}
    MarketDataError(const std::string &what_arg) : std::runtime_error(what_arg) {}

    virtual ~MarketDataError() = default;

    // UniFFI internal function - do not call this manually!
    virtual void _uniffi_internal_throw_underlying() {
        throw *this;
    }

protected:
    virtual int32_t get_variant_idx() const {
        return 0;
    };
};
/**
 * Contains variants of MarketDataError
 */
namespace market_data_error {

struct NetworkError: MarketDataError {
    std::string msg;

    NetworkError() : MarketDataError("") {}
    NetworkError(const std::string &what_arg) : MarketDataError(what_arg) {}

    // UniFFI internal function - do not call this manually!
    void _uniffi_internal_throw_underlying() override {
        throw *this;
    }

protected:
    int32_t get_variant_idx() const override {
        return 1;
    }
};

struct AuthError: MarketDataError {
    std::string msg;

    AuthError() : MarketDataError("") {}
    AuthError(const std::string &what_arg) : MarketDataError(what_arg) {}

    // UniFFI internal function - do not call this manually!
    void _uniffi_internal_throw_underlying() override {
        throw *this;
    }

protected:
    int32_t get_variant_idx() const override {
        return 2;
    }
};

struct RateLimitError: MarketDataError {
    std::string msg;

    RateLimitError() : MarketDataError("") {}
    RateLimitError(const std::string &what_arg) : MarketDataError(what_arg) {}

    // UniFFI internal function - do not call this manually!
    void _uniffi_internal_throw_underlying() override {
        throw *this;
    }

protected:
    int32_t get_variant_idx() const override {
        return 3;
    }
};

struct InvalidSymbol: MarketDataError {
    std::string msg;

    InvalidSymbol() : MarketDataError("") {}
    InvalidSymbol(const std::string &what_arg) : MarketDataError(what_arg) {}

    // UniFFI internal function - do not call this manually!
    void _uniffi_internal_throw_underlying() override {
        throw *this;
    }

protected:
    int32_t get_variant_idx() const override {
        return 4;
    }
};

struct ParseError: MarketDataError {
    std::string msg;

    ParseError() : MarketDataError("") {}
    ParseError(const std::string &what_arg) : MarketDataError(what_arg) {}

    // UniFFI internal function - do not call this manually!
    void _uniffi_internal_throw_underlying() override {
        throw *this;
    }

protected:
    int32_t get_variant_idx() const override {
        return 5;
    }
};

struct TimeoutError: MarketDataError {
    std::string msg;

    TimeoutError() : MarketDataError("") {}
    TimeoutError(const std::string &what_arg) : MarketDataError(what_arg) {}

    // UniFFI internal function - do not call this manually!
    void _uniffi_internal_throw_underlying() override {
        throw *this;
    }

protected:
    int32_t get_variant_idx() const override {
        return 6;
    }
};

struct WebSocketError: MarketDataError {
    std::string msg;

    WebSocketError() : MarketDataError("") {}
    WebSocketError(const std::string &what_arg) : MarketDataError(what_arg) {}

    // UniFFI internal function - do not call this manually!
    void _uniffi_internal_throw_underlying() override {
        throw *this;
    }

protected:
    int32_t get_variant_idx() const override {
        return 7;
    }
};

struct ClientClosed: MarketDataError {

    ClientClosed() : MarketDataError("") {}
    ClientClosed(const std::string &what_arg) : MarketDataError(what_arg) {}

    // UniFFI internal function - do not call this manually!
    void _uniffi_internal_throw_underlying() override {
        throw *this;
    }

protected:
    int32_t get_variant_idx() const override {
        return 8;
    }
};

struct ConfigError: MarketDataError {
    std::string msg;

    ConfigError() : MarketDataError("") {}
    ConfigError(const std::string &what_arg) : MarketDataError(what_arg) {}

    // UniFFI internal function - do not call this manually!
    void _uniffi_internal_throw_underlying() override {
        throw *this;
    }

protected:
    int32_t get_variant_idx() const override {
        return 9;
    }
};

struct ApiError: MarketDataError {
    std::string msg;

    ApiError() : MarketDataError("") {}
    ApiError(const std::string &what_arg) : MarketDataError(what_arg) {}

    // UniFFI internal function - do not call this manually!
    void _uniffi_internal_throw_underlying() override {
        throw *this;
    }

protected:
    int32_t get_variant_idx() const override {
        return 10;
    }
};

struct Other: MarketDataError {
    std::string msg;

    Other() : MarketDataError("") {}
    Other(const std::string &what_arg) : MarketDataError(what_arg) {}

    // UniFFI internal function - do not call this manually!
    void _uniffi_internal_throw_underlying() override {
        throw *this;
    }

protected:
    int32_t get_variant_idx() const override {
        return 11;
    }
};
} // namespace market_data_error


/**
 * Endpoint type for WebSocket connection
 */
enum class WebSocketEndpoint: int32_t {
    /**
     * Stock market data endpoint
     */
    kStock = 1,
    /**
     * Futures and options market data endpoint
     */
    kFutOpt = 2
};

namespace uniffi {struct RustStreamBuffer: std::basic_streambuf<char> {
    RustStreamBuffer(RustBuffer *buf) {
        char* data = reinterpret_cast<char*>(buf->data);
        this->setg(data, data, data + buf->len);
        this->setp(data, data + buf->capacity);
    }
    ~RustStreamBuffer() = default;

private:
    RustStreamBuffer() = delete;
    RustStreamBuffer(const RustStreamBuffer &) = delete;
    RustStreamBuffer(RustStreamBuffer &&) = delete;

    RustStreamBuffer &operator=(const RustStreamBuffer &) = delete;
    RustStreamBuffer &operator=(RustStreamBuffer &&) = delete;
};

struct RustStream: std::basic_iostream<char> {
    RustStream(RustBuffer *buf):
        std::basic_iostream<char>(&streambuf), streambuf(RustStreamBuffer(buf)) { }

    template <typename T, typename = std::enable_if_t<std::is_arithmetic_v<T>>>
    RustStream &operator>>(T &val) {
        read(reinterpret_cast<char *>(&val), sizeof(T));

        if (std::endian::native != std::endian::big) {
            auto bytes = reinterpret_cast<char *>(&val);

            std::reverse(bytes, bytes + sizeof(T));
        }

        return *this;
    }

    template <typename T, typename = std::enable_if_t<std::is_arithmetic_v<T>>>
    RustStream &operator<<(T val) {
        if (std::endian::native != std::endian::big) {
            auto bytes = reinterpret_cast<char *>(&val);

            std::reverse(bytes, bytes + sizeof(T));
        }

        write(reinterpret_cast<char *>(&val), sizeof(T));

        return *this;
    }
private:
    RustStreamBuffer streambuf;
};


RustBuffer rustbuffer_alloc(uint64_t);
RustBuffer rustbuffer_from_bytes(const ForeignBytes &);
void rustbuffer_free(RustBuffer);
template <typename T> struct HandleMap {
    HandleMap() = default;

    std::shared_ptr<T> at(uint64_t handle) {
        std::lock_guard<std::mutex> guard(this->mutex);

        return this->map.at(handle);
    }

    uint64_t insert(std::shared_ptr<T> impl) {
        std::lock_guard<std::mutex> guard(this->mutex);

        auto handle = this->cur_handle;

        this->map.insert({ handle, impl });
        this->cur_handle += 1;

        return handle;
    }

    void erase(uint64_t handle) {
        // We store the object here to avoid re-entrant locking
        std::shared_ptr<T> cleanup;
        {
            std::lock_guard<std::mutex> guard(this->mutex);
            auto it = this->map.find(handle);
            if (it != this->map.end()) {
                cleanup = it->second;
                this->map.erase(it);
            }
        }
    }
    private:
        HandleMap(const HandleMap<T> &) = delete;
        HandleMap(HandleMap<T> &&) = delete;

        HandleMap<T> &operator=(const HandleMap<T> &) = delete;
        HandleMap<T> &operator=(HandleMap<T> &&) = delete;

        std::mutex mutex;
        uint64_t cur_handle = 0;
        std::map<uint64_t, std::shared_ptr<T>> map;
};
struct FfiConverterUInt32 {
    static uint32_t lift(uint32_t);
    static uint32_t lower(uint32_t);
    static uint32_t read(RustStream &);
    static void write(RustStream &, uint32_t);
    static uint64_t allocation_size(uint32_t);
};
struct FfiConverterInt32 {
    static int32_t lift(int32_t);
    static int32_t lower(int32_t);
    static int32_t read(RustStream &);
    static void write(RustStream &, int32_t);
    static uint64_t allocation_size(int32_t);
};
struct FfiConverterUInt64 {
    static uint64_t lift(uint64_t);
    static uint64_t lower(uint64_t);
    static uint64_t read(RustStream &);
    static void write(RustStream &, uint64_t);
    static uint64_t allocation_size(uint64_t);
};
struct FfiConverterInt64 {
    static int64_t lift(int64_t);
    static int64_t lower(int64_t);
    static int64_t read(RustStream &);
    static void write(RustStream &, int64_t);
    static uint64_t allocation_size(int64_t);
};
struct FfiConverterDouble {
    static double lift(double);
    static double lower(double);
    static double read(RustStream &);
    static void write(RustStream &, double);
    static uint64_t allocation_size(double);
};
struct FfiConverterBool {
    static bool lift(uint8_t);
    static uint8_t lower(bool);
    static bool read(RustStream &);
    static void write(RustStream &, bool);
    static uint64_t allocation_size(bool);
};
struct FfiConverterString {
    static std::string lift(RustBuffer buf);
    static RustBuffer lower(const std::string &);
    static std::string read(RustStream &);
    static void write(RustStream &, const std::string &);
    static uint64_t allocation_size(const std::string &);
};

struct FfiConverterBytes {
    static std::vector<uint8_t> lift(RustBuffer);
    static RustBuffer lower(const std::vector<uint8_t> &);
    static std::vector<uint8_t> read(RustStream &);
    static void write(RustStream &, const std::vector<uint8_t> &);
    static uint64_t allocation_size(const std::vector<uint8_t> &);
};


struct FfiConverterFutOptClient {
    static std::shared_ptr<FutOptClient> lift(void *);
    static void *lower(const std::shared_ptr<FutOptClient> &);
    static std::shared_ptr<FutOptClient> read(RustStream &);
    static void write(RustStream &, const std::shared_ptr<FutOptClient> &);
    static uint64_t allocation_size(const std::shared_ptr<FutOptClient> &);
private:
};


struct FfiConverterFutOptHistoricalClient {
    static std::shared_ptr<FutOptHistoricalClient> lift(void *);
    static void *lower(const std::shared_ptr<FutOptHistoricalClient> &);
    static std::shared_ptr<FutOptHistoricalClient> read(RustStream &);
    static void write(RustStream &, const std::shared_ptr<FutOptHistoricalClient> &);
    static uint64_t allocation_size(const std::shared_ptr<FutOptHistoricalClient> &);
private:
};


struct FfiConverterFutOptIntradayClient {
    static std::shared_ptr<FutOptIntradayClient> lift(void *);
    static void *lower(const std::shared_ptr<FutOptIntradayClient> &);
    static std::shared_ptr<FutOptIntradayClient> read(RustStream &);
    static void write(RustStream &, const std::shared_ptr<FutOptIntradayClient> &);
    static uint64_t allocation_size(const std::shared_ptr<FutOptIntradayClient> &);
private:
};


struct FfiConverterRestClient {
    static std::shared_ptr<RestClient> lift(void *);
    static void *lower(const std::shared_ptr<RestClient> &);
    static std::shared_ptr<RestClient> read(RustStream &);
    static void write(RustStream &, const std::shared_ptr<RestClient> &);
    static uint64_t allocation_size(const std::shared_ptr<RestClient> &);
private:
};


struct FfiConverterStockClient {
    static std::shared_ptr<StockClient> lift(void *);
    static void *lower(const std::shared_ptr<StockClient> &);
    static std::shared_ptr<StockClient> read(RustStream &);
    static void write(RustStream &, const std::shared_ptr<StockClient> &);
    static uint64_t allocation_size(const std::shared_ptr<StockClient> &);
private:
};


struct FfiConverterStockCorporateActionsClient {
    static std::shared_ptr<StockCorporateActionsClient> lift(void *);
    static void *lower(const std::shared_ptr<StockCorporateActionsClient> &);
    static std::shared_ptr<StockCorporateActionsClient> read(RustStream &);
    static void write(RustStream &, const std::shared_ptr<StockCorporateActionsClient> &);
    static uint64_t allocation_size(const std::shared_ptr<StockCorporateActionsClient> &);
private:
};


struct FfiConverterStockHistoricalClient {
    static std::shared_ptr<StockHistoricalClient> lift(void *);
    static void *lower(const std::shared_ptr<StockHistoricalClient> &);
    static std::shared_ptr<StockHistoricalClient> read(RustStream &);
    static void write(RustStream &, const std::shared_ptr<StockHistoricalClient> &);
    static uint64_t allocation_size(const std::shared_ptr<StockHistoricalClient> &);
private:
};


struct FfiConverterStockIntradayClient {
    static std::shared_ptr<StockIntradayClient> lift(void *);
    static void *lower(const std::shared_ptr<StockIntradayClient> &);
    static std::shared_ptr<StockIntradayClient> read(RustStream &);
    static void write(RustStream &, const std::shared_ptr<StockIntradayClient> &);
    static uint64_t allocation_size(const std::shared_ptr<StockIntradayClient> &);
private:
};


struct FfiConverterStockSnapshotClient {
    static std::shared_ptr<StockSnapshotClient> lift(void *);
    static void *lower(const std::shared_ptr<StockSnapshotClient> &);
    static std::shared_ptr<StockSnapshotClient> read(RustStream &);
    static void write(RustStream &, const std::shared_ptr<StockSnapshotClient> &);
    static uint64_t allocation_size(const std::shared_ptr<StockSnapshotClient> &);
private:
};


struct FfiConverterStockTechnicalClient {
    static std::shared_ptr<StockTechnicalClient> lift(void *);
    static void *lower(const std::shared_ptr<StockTechnicalClient> &);
    static std::shared_ptr<StockTechnicalClient> read(RustStream &);
    static void write(RustStream &, const std::shared_ptr<StockTechnicalClient> &);
    static uint64_t allocation_size(const std::shared_ptr<StockTechnicalClient> &);
private:
};


struct FfiConverterWebSocketClient {
    static std::shared_ptr<WebSocketClient> lift(void *);
    static void *lower(const std::shared_ptr<WebSocketClient> &);
    static std::shared_ptr<WebSocketClient> read(RustStream &);
    static void write(RustStream &, const std::shared_ptr<WebSocketClient> &);
    static uint64_t allocation_size(const std::shared_ptr<WebSocketClient> &);
private:
};


struct FfiConverterWebSocketListener {
    static std::shared_ptr<WebSocketListener> lift(void *);
    static void *lower(const std::shared_ptr<WebSocketListener> &);
    static std::shared_ptr<WebSocketListener> read(RustStream &);
    static void write(RustStream &, const std::shared_ptr<WebSocketListener> &);
    static uint64_t allocation_size(const std::shared_ptr<WebSocketListener> &);
private:
    friend struct UniffiCallbackInterfaceWebSocketListener;
    inline static HandleMap<WebSocketListener> handle_map = {};
};

struct FfiConverterTypeActive {
    static Active lift(RustBuffer);
    static RustBuffer lower(const Active &);
    static Active read(RustStream &);
    static void write(RustStream &, const Active &);
    static uint64_t allocation_size(const Active &);
};

struct FfiConverterTypeActivesResponse {
    static ActivesResponse lift(RustBuffer);
    static RustBuffer lower(const ActivesResponse &);
    static ActivesResponse read(RustStream &);
    static void write(RustStream &, const ActivesResponse &);
    static uint64_t allocation_size(const ActivesResponse &);
};

struct FfiConverterTypeBbDataPoint {
    static BbDataPoint lift(RustBuffer);
    static RustBuffer lower(const BbDataPoint &);
    static BbDataPoint read(RustStream &);
    static void write(RustStream &, const BbDataPoint &);
    static uint64_t allocation_size(const BbDataPoint &);
};

struct FfiConverterTypeBbResponse {
    static BbResponse lift(RustBuffer);
    static RustBuffer lower(const BbResponse &);
    static BbResponse read(RustStream &);
    static void write(RustStream &, const BbResponse &);
    static uint64_t allocation_size(const BbResponse &);
};

struct FfiConverterTypeCapitalChange {
    static CapitalChange lift(RustBuffer);
    static RustBuffer lower(const CapitalChange &);
    static CapitalChange read(RustStream &);
    static void write(RustStream &, const CapitalChange &);
    static uint64_t allocation_size(const CapitalChange &);
};

struct FfiConverterTypeCapitalChangesResponse {
    static CapitalChangesResponse lift(RustBuffer);
    static RustBuffer lower(const CapitalChangesResponse &);
    static CapitalChangesResponse read(RustStream &);
    static void write(RustStream &, const CapitalChangesResponse &);
    static uint64_t allocation_size(const CapitalChangesResponse &);
};

struct FfiConverterTypeDividend {
    static Dividend lift(RustBuffer);
    static RustBuffer lower(const Dividend &);
    static Dividend read(RustStream &);
    static void write(RustStream &, const Dividend &);
    static uint64_t allocation_size(const Dividend &);
};

struct FfiConverterTypeDividendsResponse {
    static DividendsResponse lift(RustBuffer);
    static RustBuffer lower(const DividendsResponse &);
    static DividendsResponse read(RustStream &);
    static void write(RustStream &, const DividendsResponse &);
    static uint64_t allocation_size(const DividendsResponse &);
};

struct FfiConverterTypeFutOptDailyData {
    static FutOptDailyData lift(RustBuffer);
    static RustBuffer lower(const FutOptDailyData &);
    static FutOptDailyData read(RustStream &);
    static void write(RustStream &, const FutOptDailyData &);
    static uint64_t allocation_size(const FutOptDailyData &);
};

struct FfiConverterTypeFutOptDailyResponse {
    static FutOptDailyResponse lift(RustBuffer);
    static RustBuffer lower(const FutOptDailyResponse &);
    static FutOptDailyResponse read(RustStream &);
    static void write(RustStream &, const FutOptDailyResponse &);
    static uint64_t allocation_size(const FutOptDailyResponse &);
};

struct FfiConverterTypeFutOptHistoricalCandle {
    static FutOptHistoricalCandle lift(RustBuffer);
    static RustBuffer lower(const FutOptHistoricalCandle &);
    static FutOptHistoricalCandle read(RustStream &);
    static void write(RustStream &, const FutOptHistoricalCandle &);
    static uint64_t allocation_size(const FutOptHistoricalCandle &);
};

struct FfiConverterTypeFutOptHistoricalCandlesResponse {
    static FutOptHistoricalCandlesResponse lift(RustBuffer);
    static RustBuffer lower(const FutOptHistoricalCandlesResponse &);
    static FutOptHistoricalCandlesResponse read(RustStream &);
    static void write(RustStream &, const FutOptHistoricalCandlesResponse &);
    static uint64_t allocation_size(const FutOptHistoricalCandlesResponse &);
};

struct FfiConverterTypeFutOptLastTrade {
    static FutOptLastTrade lift(RustBuffer);
    static RustBuffer lower(const FutOptLastTrade &);
    static FutOptLastTrade read(RustStream &);
    static void write(RustStream &, const FutOptLastTrade &);
    static uint64_t allocation_size(const FutOptLastTrade &);
};

struct FfiConverterTypeFutOptPriceLevel {
    static FutOptPriceLevel lift(RustBuffer);
    static RustBuffer lower(const FutOptPriceLevel &);
    static FutOptPriceLevel read(RustStream &);
    static void write(RustStream &, const FutOptPriceLevel &);
    static uint64_t allocation_size(const FutOptPriceLevel &);
};

struct FfiConverterTypeFutOptQuote {
    static FutOptQuote lift(RustBuffer);
    static RustBuffer lower(const FutOptQuote &);
    static FutOptQuote read(RustStream &);
    static void write(RustStream &, const FutOptQuote &);
    static uint64_t allocation_size(const FutOptQuote &);
};

struct FfiConverterTypeFutOptTicker {
    static FutOptTicker lift(RustBuffer);
    static RustBuffer lower(const FutOptTicker &);
    static FutOptTicker read(RustStream &);
    static void write(RustStream &, const FutOptTicker &);
    static uint64_t allocation_size(const FutOptTicker &);
};

struct FfiConverterTypeFutOptTotalStats {
    static FutOptTotalStats lift(RustBuffer);
    static RustBuffer lower(const FutOptTotalStats &);
    static FutOptTotalStats read(RustStream &);
    static void write(RustStream &, const FutOptTotalStats &);
    static uint64_t allocation_size(const FutOptTotalStats &);
};

struct FfiConverterTypeHealthCheckConfigRecord {
    static HealthCheckConfigRecord lift(RustBuffer);
    static RustBuffer lower(const HealthCheckConfigRecord &);
    static HealthCheckConfigRecord read(RustStream &);
    static void write(RustStream &, const HealthCheckConfigRecord &);
    static uint64_t allocation_size(const HealthCheckConfigRecord &);
};

struct FfiConverterTypeHistoricalCandle {
    static HistoricalCandle lift(RustBuffer);
    static RustBuffer lower(const HistoricalCandle &);
    static HistoricalCandle read(RustStream &);
    static void write(RustStream &, const HistoricalCandle &);
    static uint64_t allocation_size(const HistoricalCandle &);
};

struct FfiConverterTypeHistoricalCandlesResponse {
    static HistoricalCandlesResponse lift(RustBuffer);
    static RustBuffer lower(const HistoricalCandlesResponse &);
    static HistoricalCandlesResponse read(RustStream &);
    static void write(RustStream &, const HistoricalCandlesResponse &);
    static uint64_t allocation_size(const HistoricalCandlesResponse &);
};

struct FfiConverterTypeIntradayCandle {
    static IntradayCandle lift(RustBuffer);
    static RustBuffer lower(const IntradayCandle &);
    static IntradayCandle read(RustStream &);
    static void write(RustStream &, const IntradayCandle &);
    static uint64_t allocation_size(const IntradayCandle &);
};

struct FfiConverterTypeIntradayCandlesResponse {
    static IntradayCandlesResponse lift(RustBuffer);
    static RustBuffer lower(const IntradayCandlesResponse &);
    static IntradayCandlesResponse read(RustStream &);
    static void write(RustStream &, const IntradayCandlesResponse &);
    static uint64_t allocation_size(const IntradayCandlesResponse &);
};

struct FfiConverterTypeKdjDataPoint {
    static KdjDataPoint lift(RustBuffer);
    static RustBuffer lower(const KdjDataPoint &);
    static KdjDataPoint read(RustStream &);
    static void write(RustStream &, const KdjDataPoint &);
    static uint64_t allocation_size(const KdjDataPoint &);
};

struct FfiConverterTypeKdjResponse {
    static KdjResponse lift(RustBuffer);
    static RustBuffer lower(const KdjResponse &);
    static KdjResponse read(RustStream &);
    static void write(RustStream &, const KdjResponse &);
    static uint64_t allocation_size(const KdjResponse &);
};

struct FfiConverterTypeListingApplicant {
    static ListingApplicant lift(RustBuffer);
    static RustBuffer lower(const ListingApplicant &);
    static ListingApplicant read(RustStream &);
    static void write(RustStream &, const ListingApplicant &);
    static uint64_t allocation_size(const ListingApplicant &);
};

struct FfiConverterTypeListingApplicantsResponse {
    static ListingApplicantsResponse lift(RustBuffer);
    static RustBuffer lower(const ListingApplicantsResponse &);
    static ListingApplicantsResponse read(RustStream &);
    static void write(RustStream &, const ListingApplicantsResponse &);
    static uint64_t allocation_size(const ListingApplicantsResponse &);
};

struct FfiConverterTypeMacdDataPoint {
    static MacdDataPoint lift(RustBuffer);
    static RustBuffer lower(const MacdDataPoint &);
    static MacdDataPoint read(RustStream &);
    static void write(RustStream &, const MacdDataPoint &);
    static uint64_t allocation_size(const MacdDataPoint &);
};

struct FfiConverterTypeMacdResponse {
    static MacdResponse lift(RustBuffer);
    static RustBuffer lower(const MacdResponse &);
    static MacdResponse read(RustStream &);
    static void write(RustStream &, const MacdResponse &);
    static uint64_t allocation_size(const MacdResponse &);
};

struct FfiConverterTypeMover {
    static Mover lift(RustBuffer);
    static RustBuffer lower(const Mover &);
    static Mover read(RustStream &);
    static void write(RustStream &, const Mover &);
    static uint64_t allocation_size(const Mover &);
};

struct FfiConverterTypeMoversResponse {
    static MoversResponse lift(RustBuffer);
    static RustBuffer lower(const MoversResponse &);
    static MoversResponse read(RustStream &);
    static void write(RustStream &, const MoversResponse &);
    static uint64_t allocation_size(const MoversResponse &);
};

struct FfiConverterTypePriceLevel {
    static PriceLevel lift(RustBuffer);
    static RustBuffer lower(const PriceLevel &);
    static PriceLevel read(RustStream &);
    static void write(RustStream &, const PriceLevel &);
    static uint64_t allocation_size(const PriceLevel &);
};

struct FfiConverterTypeProduct {
    static Product lift(RustBuffer);
    static RustBuffer lower(const Product &);
    static Product read(RustStream &);
    static void write(RustStream &, const Product &);
    static uint64_t allocation_size(const Product &);
};

struct FfiConverterTypeProductsResponse {
    static ProductsResponse lift(RustBuffer);
    static RustBuffer lower(const ProductsResponse &);
    static ProductsResponse read(RustStream &);
    static void write(RustStream &, const ProductsResponse &);
    static uint64_t allocation_size(const ProductsResponse &);
};

struct FfiConverterTypeQuote {
    static Quote lift(RustBuffer);
    static RustBuffer lower(const Quote &);
    static Quote read(RustStream &);
    static void write(RustStream &, const Quote &);
    static uint64_t allocation_size(const Quote &);
};

struct FfiConverterTypeReconnectConfigRecord {
    static ReconnectConfigRecord lift(RustBuffer);
    static RustBuffer lower(const ReconnectConfigRecord &);
    static ReconnectConfigRecord read(RustStream &);
    static void write(RustStream &, const ReconnectConfigRecord &);
    static uint64_t allocation_size(const ReconnectConfigRecord &);
};

struct FfiConverterTypeRsiDataPoint {
    static RsiDataPoint lift(RustBuffer);
    static RustBuffer lower(const RsiDataPoint &);
    static RsiDataPoint read(RustStream &);
    static void write(RustStream &, const RsiDataPoint &);
    static uint64_t allocation_size(const RsiDataPoint &);
};

struct FfiConverterTypeRsiResponse {
    static RsiResponse lift(RustBuffer);
    static RustBuffer lower(const RsiResponse &);
    static RsiResponse read(RustStream &);
    static void write(RustStream &, const RsiResponse &);
    static uint64_t allocation_size(const RsiResponse &);
};

struct FfiConverterTypeSmaDataPoint {
    static SmaDataPoint lift(RustBuffer);
    static RustBuffer lower(const SmaDataPoint &);
    static SmaDataPoint read(RustStream &);
    static void write(RustStream &, const SmaDataPoint &);
    static uint64_t allocation_size(const SmaDataPoint &);
};

struct FfiConverterTypeSmaResponse {
    static SmaResponse lift(RustBuffer);
    static RustBuffer lower(const SmaResponse &);
    static SmaResponse read(RustStream &);
    static void write(RustStream &, const SmaResponse &);
    static uint64_t allocation_size(const SmaResponse &);
};

struct FfiConverterTypeSnapshotQuote {
    static SnapshotQuote lift(RustBuffer);
    static RustBuffer lower(const SnapshotQuote &);
    static SnapshotQuote read(RustStream &);
    static void write(RustStream &, const SnapshotQuote &);
    static uint64_t allocation_size(const SnapshotQuote &);
};

struct FfiConverterTypeSnapshotQuotesResponse {
    static SnapshotQuotesResponse lift(RustBuffer);
    static RustBuffer lower(const SnapshotQuotesResponse &);
    static SnapshotQuotesResponse read(RustStream &);
    static void write(RustStream &, const SnapshotQuotesResponse &);
    static uint64_t allocation_size(const SnapshotQuotesResponse &);
};

struct FfiConverterTypeStatsResponse {
    static StatsResponse lift(RustBuffer);
    static RustBuffer lower(const StatsResponse &);
    static StatsResponse read(RustStream &);
    static void write(RustStream &, const StatsResponse &);
    static uint64_t allocation_size(const StatsResponse &);
};

struct FfiConverterTypeStreamMessage {
    static StreamMessage lift(RustBuffer);
    static RustBuffer lower(const StreamMessage &);
    static StreamMessage read(RustStream &);
    static void write(RustStream &, const StreamMessage &);
    static uint64_t allocation_size(const StreamMessage &);
};

struct FfiConverterTypeTicker {
    static Ticker lift(RustBuffer);
    static RustBuffer lower(const Ticker &);
    static Ticker read(RustStream &);
    static void write(RustStream &, const Ticker &);
    static uint64_t allocation_size(const Ticker &);
};

struct FfiConverterTypeTlsConfigRecord {
    static TlsConfigRecord lift(RustBuffer);
    static RustBuffer lower(const TlsConfigRecord &);
    static TlsConfigRecord read(RustStream &);
    static void write(RustStream &, const TlsConfigRecord &);
    static uint64_t allocation_size(const TlsConfigRecord &);
};

struct FfiConverterTypeTotalStats {
    static TotalStats lift(RustBuffer);
    static RustBuffer lower(const TotalStats &);
    static TotalStats read(RustStream &);
    static void write(RustStream &, const TotalStats &);
    static uint64_t allocation_size(const TotalStats &);
};

struct FfiConverterTypeTrade {
    static Trade lift(RustBuffer);
    static RustBuffer lower(const Trade &);
    static Trade read(RustStream &);
    static void write(RustStream &, const Trade &);
    static uint64_t allocation_size(const Trade &);
};

struct FfiConverterTypeTradeInfo {
    static TradeInfo lift(RustBuffer);
    static RustBuffer lower(const TradeInfo &);
    static TradeInfo read(RustStream &);
    static void write(RustStream &, const TradeInfo &);
    static uint64_t allocation_size(const TradeInfo &);
};

struct FfiConverterTypeTradesResponse {
    static TradesResponse lift(RustBuffer);
    static RustBuffer lower(const TradesResponse &);
    static TradesResponse read(RustStream &);
    static void write(RustStream &, const TradesResponse &);
    static uint64_t allocation_size(const TradesResponse &);
};

struct FfiConverterTypeTradingHalt {
    static TradingHalt lift(RustBuffer);
    static RustBuffer lower(const TradingHalt &);
    static TradingHalt read(RustStream &);
    static void write(RustStream &, const TradingHalt &);
    static uint64_t allocation_size(const TradingHalt &);
};

struct FfiConverterTypeVolumeAtPrice {
    static VolumeAtPrice lift(RustBuffer);
    static RustBuffer lower(const VolumeAtPrice &);
    static VolumeAtPrice read(RustStream &);
    static void write(RustStream &, const VolumeAtPrice &);
    static uint64_t allocation_size(const VolumeAtPrice &);
};

struct FfiConverterTypeVolumesResponse {
    static VolumesResponse lift(RustBuffer);
    static RustBuffer lower(const VolumesResponse &);
    static VolumesResponse read(RustStream &);
    static void write(RustStream &, const VolumesResponse &);
    static uint64_t allocation_size(const VolumesResponse &);
};

struct FfiConverterMarketDataError {
    static std::shared_ptr<MarketDataError> lift(RustBuffer buf);
    static RustBuffer lower(const MarketDataError &);
    static std::shared_ptr<MarketDataError> read(RustStream &stream);
    static void write(RustStream &stream, const MarketDataError &);
    static uint64_t allocation_size(const MarketDataError &);
};
struct FfiConverterWebSocketEndpoint {
    static WebSocketEndpoint lift(RustBuffer);
    static RustBuffer lower(const WebSocketEndpoint &);
    static WebSocketEndpoint read(RustStream &);
    static void write(RustStream &, const WebSocketEndpoint &);
    static uint64_t allocation_size(const WebSocketEndpoint &);
};
struct FfiConverterOptionalInt32 {
    static std::optional<int32_t> lift(RustBuffer buf);
    static RustBuffer lower(const std::optional<int32_t>& val);
    static std::optional<int32_t> read(RustStream &stream);
    static void write(RustStream &stream, const std::optional<int32_t>& value);
    static uint64_t allocation_size(const std::optional<int32_t> &val);
};
struct FfiConverterOptionalUInt64 {
    static std::optional<uint64_t> lift(RustBuffer buf);
    static RustBuffer lower(const std::optional<uint64_t>& val);
    static std::optional<uint64_t> read(RustStream &stream);
    static void write(RustStream &stream, const std::optional<uint64_t>& value);
    static uint64_t allocation_size(const std::optional<uint64_t> &val);
};
struct FfiConverterOptionalInt64 {
    static std::optional<int64_t> lift(RustBuffer buf);
    static RustBuffer lower(const std::optional<int64_t>& val);
    static std::optional<int64_t> read(RustStream &stream);
    static void write(RustStream &stream, const std::optional<int64_t>& value);
    static uint64_t allocation_size(const std::optional<int64_t> &val);
};
struct FfiConverterOptionalDouble {
    static std::optional<double> lift(RustBuffer buf);
    static RustBuffer lower(const std::optional<double>& val);
    static std::optional<double> read(RustStream &stream);
    static void write(RustStream &stream, const std::optional<double>& value);
    static uint64_t allocation_size(const std::optional<double> &val);
};
struct FfiConverterOptionalBool {
    static std::optional<bool> lift(RustBuffer buf);
    static RustBuffer lower(const std::optional<bool>& val);
    static std::optional<bool> read(RustStream &stream);
    static void write(RustStream &stream, const std::optional<bool>& value);
    static uint64_t allocation_size(const std::optional<bool> &val);
};
struct FfiConverterOptionalString {
    static std::optional<std::string> lift(RustBuffer buf);
    static RustBuffer lower(const std::optional<std::string>& val);
    static std::optional<std::string> read(RustStream &stream);
    static void write(RustStream &stream, const std::optional<std::string>& value);
    static uint64_t allocation_size(const std::optional<std::string> &val);
};
struct FfiConverterOptionalBytes {
    static std::optional<std::vector<uint8_t>> lift(RustBuffer buf);
    static RustBuffer lower(const std::optional<std::vector<uint8_t>>& val);
    static std::optional<std::vector<uint8_t>> read(RustStream &stream);
    static void write(RustStream &stream, const std::optional<std::vector<uint8_t>>& value);
    static uint64_t allocation_size(const std::optional<std::vector<uint8_t>> &val);
};
struct FfiConverterOptionalTypeFutOptLastTrade {
    static std::optional<FutOptLastTrade> lift(RustBuffer buf);
    static RustBuffer lower(const std::optional<FutOptLastTrade>& val);
    static std::optional<FutOptLastTrade> read(RustStream &stream);
    static void write(RustStream &stream, const std::optional<FutOptLastTrade>& value);
    static uint64_t allocation_size(const std::optional<FutOptLastTrade> &val);
};
struct FfiConverterOptionalTypeFutOptTotalStats {
    static std::optional<FutOptTotalStats> lift(RustBuffer buf);
    static RustBuffer lower(const std::optional<FutOptTotalStats>& val);
    static std::optional<FutOptTotalStats> read(RustStream &stream);
    static void write(RustStream &stream, const std::optional<FutOptTotalStats>& value);
    static uint64_t allocation_size(const std::optional<FutOptTotalStats> &val);
};
struct FfiConverterOptionalTypeHealthCheckConfigRecord {
    static std::optional<HealthCheckConfigRecord> lift(RustBuffer buf);
    static RustBuffer lower(const std::optional<HealthCheckConfigRecord>& val);
    static std::optional<HealthCheckConfigRecord> read(RustStream &stream);
    static void write(RustStream &stream, const std::optional<HealthCheckConfigRecord>& value);
    static uint64_t allocation_size(const std::optional<HealthCheckConfigRecord> &val);
};
struct FfiConverterOptionalTypeReconnectConfigRecord {
    static std::optional<ReconnectConfigRecord> lift(RustBuffer buf);
    static RustBuffer lower(const std::optional<ReconnectConfigRecord>& val);
    static std::optional<ReconnectConfigRecord> read(RustStream &stream);
    static void write(RustStream &stream, const std::optional<ReconnectConfigRecord>& value);
    static uint64_t allocation_size(const std::optional<ReconnectConfigRecord> &val);
};
struct FfiConverterOptionalTypeTlsConfigRecord {
    static std::optional<TlsConfigRecord> lift(RustBuffer buf);
    static RustBuffer lower(const std::optional<TlsConfigRecord>& val);
    static std::optional<TlsConfigRecord> read(RustStream &stream);
    static void write(RustStream &stream, const std::optional<TlsConfigRecord>& value);
    static uint64_t allocation_size(const std::optional<TlsConfigRecord> &val);
};
struct FfiConverterOptionalTypeTotalStats {
    static std::optional<TotalStats> lift(RustBuffer buf);
    static RustBuffer lower(const std::optional<TotalStats>& val);
    static std::optional<TotalStats> read(RustStream &stream);
    static void write(RustStream &stream, const std::optional<TotalStats>& value);
    static uint64_t allocation_size(const std::optional<TotalStats> &val);
};
struct FfiConverterOptionalTypeTradeInfo {
    static std::optional<TradeInfo> lift(RustBuffer buf);
    static RustBuffer lower(const std::optional<TradeInfo>& val);
    static std::optional<TradeInfo> read(RustStream &stream);
    static void write(RustStream &stream, const std::optional<TradeInfo>& value);
    static uint64_t allocation_size(const std::optional<TradeInfo> &val);
};
struct FfiConverterOptionalTypeTradingHalt {
    static std::optional<TradingHalt> lift(RustBuffer buf);
    static RustBuffer lower(const std::optional<TradingHalt>& val);
    static std::optional<TradingHalt> read(RustStream &stream);
    static void write(RustStream &stream, const std::optional<TradingHalt>& value);
    static uint64_t allocation_size(const std::optional<TradingHalt> &val);
};

struct FfiConverterSequenceTypeActive {
    static std::vector<Active> lift(RustBuffer);
    static RustBuffer lower(const std::vector<Active> &);
    static std::vector<Active> read(RustStream &);
    static void write(RustStream &, const std::vector<Active> &);
    static uint64_t allocation_size(const std::vector<Active> &);
};

struct FfiConverterSequenceTypeBbDataPoint {
    static std::vector<BbDataPoint> lift(RustBuffer);
    static RustBuffer lower(const std::vector<BbDataPoint> &);
    static std::vector<BbDataPoint> read(RustStream &);
    static void write(RustStream &, const std::vector<BbDataPoint> &);
    static uint64_t allocation_size(const std::vector<BbDataPoint> &);
};

struct FfiConverterSequenceTypeCapitalChange {
    static std::vector<CapitalChange> lift(RustBuffer);
    static RustBuffer lower(const std::vector<CapitalChange> &);
    static std::vector<CapitalChange> read(RustStream &);
    static void write(RustStream &, const std::vector<CapitalChange> &);
    static uint64_t allocation_size(const std::vector<CapitalChange> &);
};

struct FfiConverterSequenceTypeDividend {
    static std::vector<Dividend> lift(RustBuffer);
    static RustBuffer lower(const std::vector<Dividend> &);
    static std::vector<Dividend> read(RustStream &);
    static void write(RustStream &, const std::vector<Dividend> &);
    static uint64_t allocation_size(const std::vector<Dividend> &);
};

struct FfiConverterSequenceTypeFutOptDailyData {
    static std::vector<FutOptDailyData> lift(RustBuffer);
    static RustBuffer lower(const std::vector<FutOptDailyData> &);
    static std::vector<FutOptDailyData> read(RustStream &);
    static void write(RustStream &, const std::vector<FutOptDailyData> &);
    static uint64_t allocation_size(const std::vector<FutOptDailyData> &);
};

struct FfiConverterSequenceTypeFutOptHistoricalCandle {
    static std::vector<FutOptHistoricalCandle> lift(RustBuffer);
    static RustBuffer lower(const std::vector<FutOptHistoricalCandle> &);
    static std::vector<FutOptHistoricalCandle> read(RustStream &);
    static void write(RustStream &, const std::vector<FutOptHistoricalCandle> &);
    static uint64_t allocation_size(const std::vector<FutOptHistoricalCandle> &);
};

struct FfiConverterSequenceTypeFutOptPriceLevel {
    static std::vector<FutOptPriceLevel> lift(RustBuffer);
    static RustBuffer lower(const std::vector<FutOptPriceLevel> &);
    static std::vector<FutOptPriceLevel> read(RustStream &);
    static void write(RustStream &, const std::vector<FutOptPriceLevel> &);
    static uint64_t allocation_size(const std::vector<FutOptPriceLevel> &);
};

struct FfiConverterSequenceTypeHistoricalCandle {
    static std::vector<HistoricalCandle> lift(RustBuffer);
    static RustBuffer lower(const std::vector<HistoricalCandle> &);
    static std::vector<HistoricalCandle> read(RustStream &);
    static void write(RustStream &, const std::vector<HistoricalCandle> &);
    static uint64_t allocation_size(const std::vector<HistoricalCandle> &);
};

struct FfiConverterSequenceTypeIntradayCandle {
    static std::vector<IntradayCandle> lift(RustBuffer);
    static RustBuffer lower(const std::vector<IntradayCandle> &);
    static std::vector<IntradayCandle> read(RustStream &);
    static void write(RustStream &, const std::vector<IntradayCandle> &);
    static uint64_t allocation_size(const std::vector<IntradayCandle> &);
};

struct FfiConverterSequenceTypeKdjDataPoint {
    static std::vector<KdjDataPoint> lift(RustBuffer);
    static RustBuffer lower(const std::vector<KdjDataPoint> &);
    static std::vector<KdjDataPoint> read(RustStream &);
    static void write(RustStream &, const std::vector<KdjDataPoint> &);
    static uint64_t allocation_size(const std::vector<KdjDataPoint> &);
};

struct FfiConverterSequenceTypeListingApplicant {
    static std::vector<ListingApplicant> lift(RustBuffer);
    static RustBuffer lower(const std::vector<ListingApplicant> &);
    static std::vector<ListingApplicant> read(RustStream &);
    static void write(RustStream &, const std::vector<ListingApplicant> &);
    static uint64_t allocation_size(const std::vector<ListingApplicant> &);
};

struct FfiConverterSequenceTypeMacdDataPoint {
    static std::vector<MacdDataPoint> lift(RustBuffer);
    static RustBuffer lower(const std::vector<MacdDataPoint> &);
    static std::vector<MacdDataPoint> read(RustStream &);
    static void write(RustStream &, const std::vector<MacdDataPoint> &);
    static uint64_t allocation_size(const std::vector<MacdDataPoint> &);
};

struct FfiConverterSequenceTypeMover {
    static std::vector<Mover> lift(RustBuffer);
    static RustBuffer lower(const std::vector<Mover> &);
    static std::vector<Mover> read(RustStream &);
    static void write(RustStream &, const std::vector<Mover> &);
    static uint64_t allocation_size(const std::vector<Mover> &);
};

struct FfiConverterSequenceTypePriceLevel {
    static std::vector<PriceLevel> lift(RustBuffer);
    static RustBuffer lower(const std::vector<PriceLevel> &);
    static std::vector<PriceLevel> read(RustStream &);
    static void write(RustStream &, const std::vector<PriceLevel> &);
    static uint64_t allocation_size(const std::vector<PriceLevel> &);
};

struct FfiConverterSequenceTypeProduct {
    static std::vector<Product> lift(RustBuffer);
    static RustBuffer lower(const std::vector<Product> &);
    static std::vector<Product> read(RustStream &);
    static void write(RustStream &, const std::vector<Product> &);
    static uint64_t allocation_size(const std::vector<Product> &);
};

struct FfiConverterSequenceTypeRsiDataPoint {
    static std::vector<RsiDataPoint> lift(RustBuffer);
    static RustBuffer lower(const std::vector<RsiDataPoint> &);
    static std::vector<RsiDataPoint> read(RustStream &);
    static void write(RustStream &, const std::vector<RsiDataPoint> &);
    static uint64_t allocation_size(const std::vector<RsiDataPoint> &);
};

struct FfiConverterSequenceTypeSmaDataPoint {
    static std::vector<SmaDataPoint> lift(RustBuffer);
    static RustBuffer lower(const std::vector<SmaDataPoint> &);
    static std::vector<SmaDataPoint> read(RustStream &);
    static void write(RustStream &, const std::vector<SmaDataPoint> &);
    static uint64_t allocation_size(const std::vector<SmaDataPoint> &);
};

struct FfiConverterSequenceTypeSnapshotQuote {
    static std::vector<SnapshotQuote> lift(RustBuffer);
    static RustBuffer lower(const std::vector<SnapshotQuote> &);
    static std::vector<SnapshotQuote> read(RustStream &);
    static void write(RustStream &, const std::vector<SnapshotQuote> &);
    static uint64_t allocation_size(const std::vector<SnapshotQuote> &);
};

struct FfiConverterSequenceTypeTrade {
    static std::vector<Trade> lift(RustBuffer);
    static RustBuffer lower(const std::vector<Trade> &);
    static std::vector<Trade> read(RustStream &);
    static void write(RustStream &, const std::vector<Trade> &);
    static uint64_t allocation_size(const std::vector<Trade> &);
};

struct FfiConverterSequenceTypeVolumeAtPrice {
    static std::vector<VolumeAtPrice> lift(RustBuffer);
    static RustBuffer lower(const std::vector<VolumeAtPrice> &);
    static std::vector<VolumeAtPrice> read(RustStream &);
    static void write(RustStream &, const std::vector<VolumeAtPrice> &);
    static uint64_t allocation_size(const std::vector<VolumeAtPrice> &);
};
} // namespace uniffi

/**
 * Create a REST client with API key authentication
 *
 * # Arguments
 * * `api_key` - The Fugle API key
 *
 * # Returns
 * A RestClient instance wrapped in Arc for thread-safe access
 */
std::shared_ptr<RestClient> new_rest_client_with_api_key(const std::string &api_key);
/**
 * Create a REST client with API key authentication, custom base URL, and TLS config
 */
std::shared_ptr<RestClient> new_rest_client_with_api_key_and_tls(const std::string &api_key, std::optional<std::string> base_url, const TlsConfigRecord &tls);
/**
 * Create a REST client with bearer token authentication
 *
 * # Arguments
 * * `bearer_token` - OAuth bearer token
 *
 * # Returns
 * A RestClient instance wrapped in Arc for thread-safe access
 */
std::shared_ptr<RestClient> new_rest_client_with_bearer_token(const std::string &bearer_token);
/**
 * Create a REST client with bearer token authentication, custom base URL, and TLS config
 */
std::shared_ptr<RestClient> new_rest_client_with_bearer_token_and_tls(const std::string &bearer_token, std::optional<std::string> base_url, const TlsConfigRecord &tls);
/**
 * Create a REST client with SDK token authentication
 *
 * # Arguments
 * * `sdk_token` - Fugle SDK token
 *
 * # Returns
 * A RestClient instance wrapped in Arc for thread-safe access
 */
std::shared_ptr<RestClient> new_rest_client_with_sdk_token(const std::string &sdk_token);
/**
 * Create a REST client with SDK token authentication, custom base URL, and TLS config
 */
std::shared_ptr<RestClient> new_rest_client_with_sdk_token_and_tls(const std::string &sdk_token, std::optional<std::string> base_url, const TlsConfigRecord &tls);
/**
 * Create a new WebSocket client for stock market data
 *
 * # Arguments
 * * `api_key` - Fugle API key for authentication
 * * `listener` - Callback interface for receiving WebSocket events
 *
 * # Returns
 * A WebSocketClient instance wrapped in Arc for thread-safe access
 */
std::shared_ptr<WebSocketClient> new_websocket_client(const std::string &api_key, const std::shared_ptr<WebSocketListener> &listener);
/**
 * Create a new WebSocket client with full configuration
 *
 * # Arguments
 * * `api_key` - Fugle API key for authentication
 * * `listener` - Callback interface for receiving WebSocket events
 * * `endpoint` - The market data endpoint (Stock or FutOpt)
 * * `reconnect_config` - Optional reconnection configuration
 * * `health_check_config` - Optional health check configuration
 *
 * # Returns
 * A WebSocketClient instance wrapped in Arc for thread-safe access
 */
std::shared_ptr<WebSocketClient> new_websocket_client_with_config(const std::string &api_key, const std::shared_ptr<WebSocketListener> &listener, const WebSocketEndpoint &endpoint, std::optional<ReconnectConfigRecord> reconnect_config, std::optional<HealthCheckConfigRecord> health_check_config);
/**
 * Create a new WebSocket client for a specific endpoint
 *
 * # Arguments
 * * `api_key` - Fugle API key for authentication
 * * `listener` - Callback interface for receiving WebSocket events
 * * `endpoint` - The market data endpoint (Stock or FutOpt)
 *
 * # Returns
 * A WebSocketClient instance wrapped in Arc for thread-safe access
 */
std::shared_ptr<WebSocketClient> new_websocket_client_with_endpoint(const std::string &api_key, const std::shared_ptr<WebSocketListener> &listener, const WebSocketEndpoint &endpoint);
} // namespace marketdata_uniffi