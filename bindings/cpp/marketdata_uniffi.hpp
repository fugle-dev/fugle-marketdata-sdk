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

#ifndef UNIFFI_CPP_RUST_STREAM
#define UNIFFI_CPP_RUST_STREAM
namespace uniffi {
struct RustStreamBuffer: std::basic_streambuf<char> {
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

}
#endif

namespace marketdata_uniffi {
struct FutOptClient;
struct FutOptHistoricalClient;
struct FutOptIntradayClient;
struct RestClient;
struct StockClient;
struct StockCorporateActionsClient;
struct StockHistoricalClient;
struct StockIntradayClient;
struct StockOwnershipClient;
struct StockSnapshotClient;
struct StockTechnicalClient;
struct WebSocketClient;
struct WebSocketListener;
struct CredentialsRecord;
struct ErrorInfo;
struct HealthCheckConfigRecord;
struct MessageQueueConfigRecord;
struct ReconnectConfigRecord;
struct StreamMessage;
struct StreamingVersionRecord;
struct TlsConfigRecord;
enum class CredentialKind;
enum class ErrorSourceKind;
struct MarketDataError;
enum class MessageOverflowRecord;
enum class WebSocketEndpoint;


/**
 * What the client does with an inbound message while its queue already
 * holds `buffer` unread messages.
 */
enum class MessageOverflowRecord: int32_t {
    /**
     * Drop new messages and report them through `on_messages_dropped`.
     */
    kDropNewest = 1,
    /**
     * Never drop: the queue grows while `on_message` lags.
     */
    kUnbounded = 2
};


/**
 * Coarse-grained classification of the source of a [`MarketDataError`].
 *
 * Mirrors `marketdata_core::ErrorKind`. That core enum is `#[non_exhaustive]`
 * so a future variant this crate doesn't know about yet maps to `Client`
 * (see the `From` impl below) rather than failing to compile.
 */
enum class ErrorSourceKind: int32_t {
    /**
     * Transport-level transient failure: connection reset, timeout,
     * heartbeat gap, server outage (5xx). Generally safe to retry with
     * backoff.
     */
    kNetwork = 1,
    /**
     * Protocol-level violation or unclassified WebSocket failure. Indicates
     * an SDK / version mismatch or a server-side bug; retry is unlikely to
     * help.
     */
    kProtocol = 2,
    /**
     * Authentication / authorization failure: bad credentials, 401/403,
     * expired token, TLS cert failure. Human intervention required.
     */
    kAuth = 3,
    /**
     * Server is rejecting requests because the caller is exceeding its
     * rate budget (HTTP 429).
     */
    kRateLimit = 4,
    /**
     * Caller-side problem: invalid input, configuration error, client
     * already closed, serialization failure, non-auth/non-throttle 4xx.
     */
    kClient = 5
};


/**
 * The cross-language view of an error: the fields every binding exposes
 * under the same names. Mirrors `marketdata_core::ErrorInfo`.
 */
struct ErrorInfo {
    /**
     * Numeric code from `marketdata_core::error_code`, stable across
     * languages and releases.
     */
    int32_t code;
    /**
     * Category of the failure.
     */
    ErrorSourceKind source_kind;
    /**
     * Human-readable message.
     */
    std::string message;
    /**
     * HTTP status, when the error came from an HTTP response (REST, or the
     * WebSocket upgrade).
     */
    std::optional<uint16_t> status;
    /**
     * Raw HTTP response body (REST only).
     */
    std::optional<std::string> body;
    /**
     * Server-assigned request id (`x-request-id`), when present.
     */
    std::optional<std::string> request_id;
    /**
     * HTTP response headers (REST only; empty otherwise).
     */
    std::unordered_map<std::string, std::string> headers;
};


/**
 * Message queue configuration record for FFI
 *
 * `buffer` is 0 for the default (4096).
 */
struct MessageQueueConfigRecord {
    /**
     * What happens to new messages while `buffer` are unread
     */
    MessageOverflowRecord overflow;
    /**
     * Unread messages held (default 4096; 0 means default)
     */
    uint32_t buffer;
};

namespace uniffi {
struct FfiConverterMarketDataError;
} // namespace uniffi

/**
 * Error type for UniFFI bindings
 *
 * Maps to MarketDataError in the UDL file. Each variant becomes an exception
 * in the target language with the error message preserved, plus an `info`
 * field carrying the unified [`ErrorInfo`].
 *
 * Note: This is a FLAT enum per UniFFI constraints - no nested error types.
 */
struct MarketDataError: std::runtime_error {
    friend uniffi::FfiConverterMarketDataError;

    MarketDataError() : std::runtime_error("") {}
    MarketDataError(const std::string &what_arg) : std::runtime_error(what_arg) {}

    virtual ~MarketDataError() = default;

    virtual void throw_underlying() {
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
    ErrorInfo info;

    NetworkError() : MarketDataError("") {}
    NetworkError(const std::string &what_arg) : MarketDataError(what_arg) {}

    void throw_underlying() override {
        throw *this;
    }

protected:
    int32_t get_variant_idx() const override {
        return 1;
    }
};

struct AuthError: MarketDataError {
    std::string msg;
    ErrorInfo info;

    AuthError() : MarketDataError("") {}
    AuthError(const std::string &what_arg) : MarketDataError(what_arg) {}

    void throw_underlying() override {
        throw *this;
    }

protected:
    int32_t get_variant_idx() const override {
        return 2;
    }
};

struct RateLimitError: MarketDataError {
    std::string msg;
    ErrorInfo info;

    RateLimitError() : MarketDataError("") {}
    RateLimitError(const std::string &what_arg) : MarketDataError(what_arg) {}

    void throw_underlying() override {
        throw *this;
    }

protected:
    int32_t get_variant_idx() const override {
        return 3;
    }
};

struct InvalidSymbol: MarketDataError {
    std::string msg;
    ErrorInfo info;

    InvalidSymbol() : MarketDataError("") {}
    InvalidSymbol(const std::string &what_arg) : MarketDataError(what_arg) {}

    void throw_underlying() override {
        throw *this;
    }

protected:
    int32_t get_variant_idx() const override {
        return 4;
    }
};

struct ParseError: MarketDataError {
    std::string msg;
    ErrorInfo info;

    ParseError() : MarketDataError("") {}
    ParseError(const std::string &what_arg) : MarketDataError(what_arg) {}

    void throw_underlying() override {
        throw *this;
    }

protected:
    int32_t get_variant_idx() const override {
        return 5;
    }
};

struct TimeoutError: MarketDataError {
    std::string msg;
    ErrorInfo info;

    TimeoutError() : MarketDataError("") {}
    TimeoutError(const std::string &what_arg) : MarketDataError(what_arg) {}

    void throw_underlying() override {
        throw *this;
    }

protected:
    int32_t get_variant_idx() const override {
        return 6;
    }
};

struct WebSocketError: MarketDataError {
    std::string msg;
    ErrorInfo info;

    WebSocketError() : MarketDataError("") {}
    WebSocketError(const std::string &what_arg) : MarketDataError(what_arg) {}

    void throw_underlying() override {
        throw *this;
    }

protected:
    int32_t get_variant_idx() const override {
        return 7;
    }
};

struct ClientClosed: MarketDataError {
    ErrorInfo info;

    ClientClosed() : MarketDataError("") {}
    ClientClosed(const std::string &what_arg) : MarketDataError(what_arg) {}

    void throw_underlying() override {
        throw *this;
    }

protected:
    int32_t get_variant_idx() const override {
        return 8;
    }
};

struct ConfigError: MarketDataError {
    std::string msg;
    ErrorInfo info;

    ConfigError() : MarketDataError("") {}
    ConfigError(const std::string &what_arg) : MarketDataError(what_arg) {}

    void throw_underlying() override {
        throw *this;
    }

protected:
    int32_t get_variant_idx() const override {
        return 9;
    }
};

struct ApiError: MarketDataError {
    std::string msg;
    ErrorInfo info;

    ApiError() : MarketDataError("") {}
    ApiError(const std::string &what_arg) : MarketDataError(what_arg) {}

    void throw_underlying() override {
        throw *this;
    }

protected:
    int32_t get_variant_idx() const override {
        return 10;
    }
};

struct Other: MarketDataError {
    std::string msg;
    ErrorInfo info;

    Other() : MarketDataError("") {}
    Other(const std::string &what_arg) : MarketDataError(what_arg) {}

    void throw_underlying() override {
        throw *this;
    }

protected:
    int32_t get_variant_idx() const override {
        return 11;
    }
};
} // namespace market_data_error


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
    /**
     * Get historical candles for a product such as "TXF" (sync/blocking)
     */
    std::string candles_sync(const std::string &symbol, std::optional<std::string> from, std::optional<std::string> to, std::optional<std::string> timeframe, bool after_hours, std::optional<std::string> contract_month, std::optional<std::string> fields, std::optional<std::string> sort);
    /**
     * Get one trading day's daily quotes for every contract month of a product such as "TXF" (sync/blocking)
     */
    std::string daily_sync(const std::string &symbol, std::optional<std::string> date, bool after_hours);

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
    /**
     * Get candlestick data for a contract (sync/blocking)
     */
    std::string candles_sync(const std::string &symbol, const std::string &timeframe);
    /**
     * Get available products list (sync/blocking)
     */
    std::string products_sync(const std::string &typ);
    /**
     * Get quote for a futures/options contract (sync/blocking)
     */
    std::string quote_sync(const std::string &symbol, bool after_hours);
    /**
     * Get ticker info for a contract (sync/blocking)
     */
    std::string ticker_sync(const std::string &symbol, bool after_hours);
    /**
     * Get batch tickers for futures/options (sync/blocking)
     *
     * typ: "F" for futures, "O" for options
     */
    std::string tickers_sync(const std::string &typ, std::optional<bool> is_spread);
    /**
     * Get trade history for a contract (sync/blocking)
     */
    std::string trades_sync(const std::string &symbol);
    /**
     * Get volume breakdown by price for a contract (sync/blocking)
     */
    std::string volumes_sync(const std::string &symbol);

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
     * The prefix every request from this client is built on, fully resolved —
     * host, path prefix and version segment.
     *
     * The version segment is chosen by the SDK rather than written by the
     * caller, so this is the only way to see what a client resolved to.
     */
    std::string base_url();
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
     * The fully resolved request prefix for this product client.
     */
    std::string base_url();
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
     * Access ownership endpoints (ETF holdings, institutional trades, director
     * holdings, TDCC distribution)
     */
    std::shared_ptr<StockOwnershipClient> ownership();
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
    /**
     * Get capital structure changes (sync/blocking)
     */
    std::string capital_changes_sync(std::optional<std::string> date, std::optional<std::string> start_date, std::optional<std::string> end_date);
    /**
     * Get dividend announcements (sync/blocking)
     */
    std::string dividends_sync(std::optional<std::string> date, std::optional<std::string> start_date, std::optional<std::string> end_date);
    /**
     * Get IPO listing applicants (sync/blocking)
     */
    std::string listing_applicants_sync(std::optional<std::string> date, std::optional<std::string> start_date, std::optional<std::string> end_date);

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
    /**
     * Get historical candles for a symbol (sync/blocking)
     */
    std::string candles_sync(const std::string &symbol, std::optional<std::string> from, std::optional<std::string> to, std::optional<std::string> timeframe);
    /**
     * Get historical stats for a symbol (sync/blocking)
     */
    std::string stats_sync(const std::string &symbol);

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
    /**
     * Get candlestick data for a symbol (sync/blocking)
     */
    std::string candles_sync(const std::string &symbol, const std::string &timeframe);
    /**
     * Get quote for a symbol (sync/blocking)
     */
    std::string quote_sync(const std::string &symbol);
    /**
     * Get ticker info for a symbol (sync/blocking)
     */
    std::string ticker_sync(const std::string &symbol);
    /**
     * Get batch tickers for a security type (sync/blocking)
     *
     * typ: Security type (e.g., "EQUITY", "INDEX", "ETF")
     */
    std::string tickers_sync(const std::string &typ);
    /**
     * Get trade history for a symbol (sync/blocking)
     */
    std::string trades_sync(const std::string &symbol);
    /**
     * Get volume breakdown for a symbol (sync/blocking)
     */
    std::string volumes_sync(const std::string &symbol);

    private:
    StockIntradayClient(const StockIntradayClient &);

    StockIntradayClient(void *);

    void *_uniffi_internal_clone_pointer() const;

    void *instance = nullptr;
};


namespace uniffi {
    struct FfiConverterStockOwnershipClient;
} // namespace uniffi

/**
 * Stock ownership endpoints client
 */
struct StockOwnershipClient



{
    friend uniffi::FfiConverterStockOwnershipClient;

    StockOwnershipClient() = delete;

    StockOwnershipClient(StockOwnershipClient &&) = delete;

    StockOwnershipClient &operator=(const StockOwnershipClient &) = delete;
    StockOwnershipClient &operator=(StockOwnershipClient &&) = delete;

    ~StockOwnershipClient();
    /**
     * Get monthly holdings and pledges disclosed by directors and supervisors (sync/blocking)
     */
    std::string director_holdings_sync(const std::string &symbol, std::optional<std::string> from, std::optional<std::string> to, std::optional<std::string> sort);
    /**
     * Get the constituents an ETF held over a date range (sync/blocking)
     */
    std::string etf_holdings_sync(const std::string &symbol, std::optional<std::string> from, std::optional<std::string> to, std::optional<std::string> sort);
    /**
     * Get daily trading by the three major institutional investors (sync/blocking)
     */
    std::string institutional_trades_sync(const std::string &symbol, std::optional<std::string> from, std::optional<std::string> to, std::optional<std::string> sort);
    /**
     * Get the weekly TDCC shareholder distribution by holding-size bracket (sync/blocking)
     */
    std::string tdcc_distribution_sync(const std::string &symbol, std::optional<std::string> from, std::optional<std::string> to, std::optional<std::string> sort);

    private:
    StockOwnershipClient(const StockOwnershipClient &);

    StockOwnershipClient(void *);

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
    /**
     * Get most actively traded stocks (sync/blocking)
     */
    std::string actives_sync(const std::string &market, std::optional<std::string> trade);
    /**
     * Get top movers (sync/blocking)
     */
    std::string movers_sync(const std::string &market, std::optional<std::string> direction, std::optional<std::string> change);
    /**
     * Get market-wide snapshot quotes (sync/blocking)
     */
    std::string quotes_sync(const std::string &market, std::optional<std::string> type_filter);

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
    /**
     * Get Bollinger Bands (sync/blocking)
     */
    std::string bb_sync(const std::string &symbol, std::optional<std::string> from, std::optional<std::string> to, std::optional<std::string> timeframe, std::optional<uint32_t> period, std::optional<double> stddev);
    /**
     * Get KDJ (sync/blocking)
     */
    std::string kdj_sync(const std::string &symbol, std::optional<std::string> from, std::optional<std::string> to, std::optional<std::string> timeframe, std::optional<uint32_t> r_period, std::optional<uint32_t> k_period, std::optional<uint32_t> d_period);
    /**
     * Get MACD (sync/blocking)
     */
    std::string macd_sync(const std::string &symbol, std::optional<std::string> from, std::optional<std::string> to, std::optional<std::string> timeframe, std::optional<uint32_t> fast, std::optional<uint32_t> slow, std::optional<uint32_t> signal);
    /**
     * Get Relative Strength Index (sync/blocking)
     */
    std::string rsi_sync(const std::string &symbol, std::optional<std::string> from, std::optional<std::string> to, std::optional<std::string> timeframe, std::optional<uint32_t> period);
    /**
     * Get Simple Moving Average (sync/blocking)
     */
    std::string sma_sync(const std::string &symbol, std::optional<std::string> from, std::optional<std::string> to, std::optional<std::string> timeframe, std::optional<uint32_t> period);

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
     * Create a new WebSocket client from whichever credential was given.
     *
     * Takes the same three credentials as the REST client: exactly one must
     * be non-empty (an empty or whitespace-only value counts as not
     * provided), otherwise this returns a `ConfigError` (code 1004). The
     * auth frame then carries it as `apikey`, `token` or `sdkToken`.
     * The other arguments are those of `new_with_options`.
     *
     * The credentials are one record rather than three arguments: with three
     * more buffers than `new_with_options` the Java binding (JNA) passed
     * garbage to Rust on macOS arm64.
     */
    static std::shared_ptr<WebSocketClient> new_with_credentials(const CredentialsRecord &credentials, const std::shared_ptr<WebSocketListener> &listener, const WebSocketEndpoint &endpoint, std::optional<std::string> base_url, std::optional<ReconnectConfigRecord> reconnect_config, std::optional<HealthCheckConfigRecord> health_check_config, std::optional<TlsConfigRecord> tls, std::optional<StreamingVersionRecord> version, std::optional<MessageQueueConfigRecord> message_queue);
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
    static std::shared_ptr<WebSocketClient> new_with_full_config(const std::string &api_key, const std::shared_ptr<WebSocketListener> &listener, const WebSocketEndpoint &endpoint, std::optional<std::string> base_url, std::optional<ReconnectConfigRecord> reconnect_config, std::optional<HealthCheckConfigRecord> health_check_config, std::optional<TlsConfigRecord> tls, std::optional<StreamingVersionRecord> version);
    /**
     * Create a new WebSocket client with full configuration plus the
     * message queue settings.
     *
     * Same as `new_with_full_config`, with `message_queue` choosing what
     * happens while `on_message` falls behind (None for the defaults:
     * `DropNewest`, 4096 messages).
     *
     * # Arguments
     * * `api_key` - Fugle API key for authentication
     * * `listener` - Callback interface for receiving WebSocket events
     * * `endpoint` - The market data endpoint (Stock or FutOpt)
     * * `base_url` - Optional base URL override
     * * `reconnect_config` - Optional reconnection configuration
     * * `health_check_config` - Optional health check configuration
     * * `tls` - Optional TLS customization (custom CA or accept_invalid_certs)
     * * `version` - Optional per-product streaming version
     * * `message_queue` - Optional message queue configuration
     */
    static std::shared_ptr<WebSocketClient> new_with_options(const std::string &api_key, const std::shared_ptr<WebSocketListener> &listener, const WebSocketEndpoint &endpoint, std::optional<std::string> base_url, std::optional<ReconnectConfigRecord> reconnect_config, std::optional<HealthCheckConfigRecord> health_check_config, std::optional<TlsConfigRecord> tls, std::optional<StreamingVersionRecord> version, std::optional<MessageQueueConfigRecord> message_queue);
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
     * Check if the connection has ended
     *
     * Reads core's connection state: true after `disconnect()`, and after
     * the server closes the connection when no reconnect follows (disabled
     * or attempts exhausted). False while reconnecting and before the first
     * `connect()`.
     */
    bool is_closed();
    /**
     * Check if the client is currently connected
     *
     * Reads core's connection state, so it is false while reconnecting and
     * right after the connection drops, without waiting for the event thread.
     */
    bool is_connected();
    /**
     * Messages dropped because they arrived while the message queue held
     * `buffer` unread messages (`MessageOverflowRecord::DropNewest`).
     *
     * Counted from the start of the current connection (every `connect()` or
     * reconnect restarts it); after `disconnect()` it still reads the last
     * connection's count. 0 before the first `connect()`.
     */
    uint64_t messages_dropped_total();
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
 * public void OnAuthenticated(string? dataJson) {
 * Console.WriteLine("Authenticated");
 * }
 * public void OnUnauthenticated(string? dataJson) {
 * Console.WriteLine($"Rejected: {dataJson}");
 * }
 * public void OnDisconnected(bool willReconnect) {
 * Console.WriteLine($"Disconnected (will reconnect: {willReconnect})");
 * }
 * public void OnMessage(StreamMessage message) {
 * Console.WriteLine($"Got {message.Event} for {message.Symbol}");
 * }
 * public void OnError(ErrorInfo error) {
 * Console.WriteLine($"Error: {error.Message}");
 * }
 * }
 * ```
 */
struct WebSocketListener {
    virtual ~WebSocketListener() {}
    /**
     * Called when the transport is established, before the server has
     * answered the auth frame. Fires again on every successful reconnect.
     * Wait for `on_authenticated` before treating the connection as usable.
     */
    virtual
    void on_connected() = 0;
    /**
     * Called when the server accepts the credentials.
     *
     * `data_json` is the `data` member of the server's `authenticated`
     * frame, still encoded as JSON, or `None` when the frame has none.
     */
    virtual
    void on_authenticated(std::optional<std::string> data_json) = 0;
    /**
     * Called when the server rejects the credentials. `connect()` also
     * fails with an auth error; no `on_error` is emitted for the rejection.
     *
     * `data_json` is the `data` member of the server's rejection frame
     * (the server's message is under `message`), still encoded as JSON, or
     * `None` when the frame has none.
     */
    virtual
    void on_unauthenticated(std::optional<std::string> data_json) = 0;
    /**
     * Called when the connection is closed, at most once per connection.
     *
     * `will_reconnect` is `true` when the client will try to reconnect
     * (`on_reconnecting` follows unless `disconnect()` is called first) and
     * `false` when this connection's lifecycle has ended.
     */
    virtual
    void on_disconnected(bool will_reconnect) = 0;
    /**
     * Called when a message is received
     */
    virtual
    void on_message(const StreamMessage &message) = 0;
    /**
     * Called when an error occurs
     */
    virtual
    void on_error(const ErrorInfo &error) = 0;
    /**
     * Called when a reconnection attempt starts
     */
    virtual
    void on_reconnecting(uint32_t attempt) = 0;
    /**
     * Called when all reconnection attempts are exhausted. Terminal: no
     * further lifecycle callbacks follow for this connection.
     */
    virtual
    void on_reconnect_failed(uint32_t attempts) = 0;
    /**
     * Called when messages were dropped because `on_message` fell behind
     * while the client's message queue held `buffer` unread messages
     * (`MessageOverflowRecord::DropNewest`).
     *
     * `count` is the number dropped since the previous call. The first drop
     * on a connection is reported at once, later ones at most once per
     * second, and the rest before `on_disconnected`. The connection's total
     * is `WebSocketClient::messages_dropped_total()`.
     */
    virtual
    void on_messages_dropped(uint64_t count) = 0;
};

namespace uniffi {
    struct UniffiCallbackInterfaceWebSocketListener {
        static void on_connected(uint64_t uniffi_handle,void * uniffi_out_return,RustCallStatus *out_status);
        static void on_authenticated(uint64_t uniffi_handle,RustBuffer data_json,void * uniffi_out_return,RustCallStatus *out_status);
        static void on_unauthenticated(uint64_t uniffi_handle,RustBuffer data_json,void * uniffi_out_return,RustCallStatus *out_status);
        static void on_disconnected(uint64_t uniffi_handle,int8_t will_reconnect,void * uniffi_out_return,RustCallStatus *out_status);
        static void on_message(uint64_t uniffi_handle,RustBuffer message,void * uniffi_out_return,RustCallStatus *out_status);
        static void on_error(uint64_t uniffi_handle,RustBuffer error,void * uniffi_out_return,RustCallStatus *out_status);
        static void on_reconnecting(uint64_t uniffi_handle,uint32_t attempt,void * uniffi_out_return,RustCallStatus *out_status);
        static void on_reconnect_failed(uint64_t uniffi_handle,uint32_t attempts,void * uniffi_out_return,RustCallStatus *out_status);
        static void on_messages_dropped(uint64_t uniffi_handle,uint64_t count,void * uniffi_out_return,RustCallStatus *out_status);

        static void uniffi_free(uint64_t uniffi_handle);
        static void init();
    private:
        static inline UniffiVTableCallbackInterfaceWebSocketListener vtable = UniffiVTableCallbackInterfaceWebSocketListener {
            .on_connected = reinterpret_cast<void *>(&on_connected),
            .on_authenticated = reinterpret_cast<void *>(&on_authenticated),
            .on_unauthenticated = reinterpret_cast<void *>(&on_unauthenticated),
            .on_disconnected = reinterpret_cast<void *>(&on_disconnected),
            .on_message = reinterpret_cast<void *>(&on_message),
            .on_error = reinterpret_cast<void *>(&on_error),
            .on_reconnecting = reinterpret_cast<void *>(&on_reconnecting),
            .on_reconnect_failed = reinterpret_cast<void *>(&on_reconnect_failed),
            .on_messages_dropped = reinterpret_cast<void *>(&on_messages_dropped),
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
 * public void OnAuthenticated(string? dataJson) {
 * Console.WriteLine("Authenticated");
 * }
 * public void OnUnauthenticated(string? dataJson) {
 * Console.WriteLine($"Rejected: {dataJson}");
 * }
 * public void OnDisconnected(bool willReconnect) {
 * Console.WriteLine($"Disconnected (will reconnect: {willReconnect})");
 * }
 * public void OnMessage(StreamMessage message) {
 * Console.WriteLine($"Got {message.Event} for {message.Symbol}");
 * }
 * public void OnError(ErrorInfo error) {
 * Console.WriteLine($"Error: {error.Message}");
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
     * Called when the transport is established, before the server has
     * answered the auth frame. Fires again on every successful reconnect.
     * Wait for `on_authenticated` before treating the connection as usable.
     */
    void on_connected();
    /**
     * Called when the server accepts the credentials.
     *
     * `data_json` is the `data` member of the server's `authenticated`
     * frame, still encoded as JSON, or `None` when the frame has none.
     */
    void on_authenticated(std::optional<std::string> data_json);
    /**
     * Called when the server rejects the credentials. `connect()` also
     * fails with an auth error; no `on_error` is emitted for the rejection.
     *
     * `data_json` is the `data` member of the server's rejection frame
     * (the server's message is under `message`), still encoded as JSON, or
     * `None` when the frame has none.
     */
    void on_unauthenticated(std::optional<std::string> data_json);
    /**
     * Called when the connection is closed, at most once per connection.
     *
     * `will_reconnect` is `true` when the client will try to reconnect
     * (`on_reconnecting` follows unless `disconnect()` is called first) and
     * `false` when this connection's lifecycle has ended.
     */
    void on_disconnected(bool will_reconnect);
    /**
     * Called when a message is received
     */
    void on_message(const StreamMessage &message);
    /**
     * Called when an error occurs
     */
    void on_error(const ErrorInfo &error);
    /**
     * Called when a reconnection attempt starts
     */
    void on_reconnecting(uint32_t attempt);
    /**
     * Called when all reconnection attempts are exhausted. Terminal: no
     * further lifecycle callbacks follow for this connection.
     */
    void on_reconnect_failed(uint32_t attempts);
    /**
     * Called when messages were dropped because `on_message` fell behind
     * while the client's message queue held `buffer` unread messages
     * (`MessageOverflowRecord::DropNewest`).
     *
     * `count` is the number dropped since the previous call. The first drop
     * on a connection is reported at once, later ones at most once per
     * second, and the rest before `on_disconnected`. The connection's total
     * is `WebSocketClient::messages_dropped_total()`.
     */
    void on_messages_dropped(uint64_t count);

    private:
    WebSocketListenerImpl(const WebSocketListenerImpl &);

    WebSocketListenerImpl(void *);

    void *_uniffi_internal_clone_pointer() const;

    void *instance = nullptr;
};


/**
 * The credentials a WebSocket client authenticates with.
 *
 * Exactly one must be non-empty; an empty or whitespace-only value counts
 * as not provided.
 *
 * Its fields are secrets: do not log this record. `Debug` here redacts
 * them, but the generated types may not — a C# record's `ToString()` and
 * Go's `fmt` `%v` print every field.
 */
struct CredentialsRecord {
    /**
     * Fugle API key, sent as `apikey`
     */
    std::optional<std::string> api_key;
    /**
     * OAuth bearer token, sent as `token`
     */
    std::optional<std::string> bearer_token;
    /**
     * Fugle SDK token, sent as `sdkToken`
     */
    std::optional<std::string> sdk_token;
};


/**
 * Health check configuration record for FFI
 *
 * All fields are optional — zero/false values mean "use default".
 */
struct HealthCheckConfigRecord {
    /**
     * Whether liveness detection is active (default: true in 3.0)
     */
    bool enabled;
    /**
     * Maximum allowed gap between inbound frames before declaring the
     * connection dead, in milliseconds. Default 35000; floor 5000.
     * Pass 0 to use the default.
     */
    uint64_t heartbeat_timeout_ms;
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
 * An inbound streaming frame.
 *
 * `raw` is the frame exactly as the server sent it — decode that when you
 * want the payload. The other fields are the routing subset this SDK parses
 * out so callbacks can dispatch without decoding the whole frame first; they
 * are a convenience, not the source of truth.
 */
struct StreamMessage {
    /**
     * The frame verbatim, as received on the wire.
     */
    std::string raw;
    /**
     * Event type: "data", "subscribed", "error", "authenticated", "pong".
     */
    std::string event;
    /**
     * Channel name, for data events.
     */
    std::optional<std::string> channel;
    /**
     * Symbol, for data events.
     */
    std::optional<std::string> symbol;
    /**
     * Subscription id, for subscribed events.
     */
    std::optional<std::string> id;
    /**
     * The `data` member of the frame, still encoded as JSON.
     */
    std::optional<std::string> data_json;
    /**
     * Error code, for error events.
     */
    std::optional<int32_t> error_code;
    /**
     * Error message, for error events.
     */
    std::optional<std::string> error_message;
};


/**
 * Per-product streaming version selection.
 *
 * UniFFI has no way to express core's one-enum-per-product typing across
 * C#/Go/Java/C++ at once, so this carries optional strings and validates
 * them — the same shape the official SDK's version map has.
 */
struct StreamingVersionRecord {
    /**
     * Stock streaming version. Only "v1.0" is served. None means latest.
     */
    std::optional<std::string> stock;
    /**
     * FutOpt streaming version: "v1.0" or "v1.1". None means latest (v1.1).
     *
     * v1.1 adds trial-matching (試撮) frames on trades / books — check the
     * frame's `isTrial` before acting on a price.
     */
    std::optional<std::string> futopt;
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


/**
 * Which credential [`validate_credentials`] accepted.
 */
enum class CredentialKind: int32_t {
    /**
     * `api_key` was the credential provided.
     */
    kApiKey = 1,
    /**
     * `bearer_token` was the credential provided.
     */
    kBearerToken = 2,
    /**
     * `sdk_token` was the credential provided.
     */
    kSdkToken = 3
};


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

namespace uniffi {
using ::uniffi::RustStream;
using ::uniffi::RustStreamBuffer;

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
struct FfiConverterUInt16 {
    static uint16_t lift(uint16_t);
    static uint16_t lower(uint16_t);
    static uint16_t read(RustStream &);
    static void write(RustStream &, uint16_t);
    static uint64_t allocation_size(uint16_t);
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


struct FfiConverterStockOwnershipClient {
    static std::shared_ptr<StockOwnershipClient> lift(void *);
    static void *lower(const std::shared_ptr<StockOwnershipClient> &);
    static std::shared_ptr<StockOwnershipClient> read(RustStream &);
    static void write(RustStream &, const std::shared_ptr<StockOwnershipClient> &);
    static uint64_t allocation_size(const std::shared_ptr<StockOwnershipClient> &);
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

struct FfiConverterTypeCredentialsRecord {
    static CredentialsRecord lift(RustBuffer);
    static RustBuffer lower(const CredentialsRecord &);
    static CredentialsRecord read(RustStream &);
    static void write(RustStream &, const CredentialsRecord &);
    static uint64_t allocation_size(const CredentialsRecord &);
};

struct FfiConverterTypeErrorInfo {
    static ErrorInfo lift(RustBuffer);
    static RustBuffer lower(const ErrorInfo &);
    static ErrorInfo read(RustStream &);
    static void write(RustStream &, const ErrorInfo &);
    static uint64_t allocation_size(const ErrorInfo &);
};

struct FfiConverterTypeHealthCheckConfigRecord {
    static HealthCheckConfigRecord lift(RustBuffer);
    static RustBuffer lower(const HealthCheckConfigRecord &);
    static HealthCheckConfigRecord read(RustStream &);
    static void write(RustStream &, const HealthCheckConfigRecord &);
    static uint64_t allocation_size(const HealthCheckConfigRecord &);
};

struct FfiConverterTypeMessageQueueConfigRecord {
    static MessageQueueConfigRecord lift(RustBuffer);
    static RustBuffer lower(const MessageQueueConfigRecord &);
    static MessageQueueConfigRecord read(RustStream &);
    static void write(RustStream &, const MessageQueueConfigRecord &);
    static uint64_t allocation_size(const MessageQueueConfigRecord &);
};

struct FfiConverterTypeReconnectConfigRecord {
    static ReconnectConfigRecord lift(RustBuffer);
    static RustBuffer lower(const ReconnectConfigRecord &);
    static ReconnectConfigRecord read(RustStream &);
    static void write(RustStream &, const ReconnectConfigRecord &);
    static uint64_t allocation_size(const ReconnectConfigRecord &);
};

struct FfiConverterTypeStreamMessage {
    static StreamMessage lift(RustBuffer);
    static RustBuffer lower(const StreamMessage &);
    static StreamMessage read(RustStream &);
    static void write(RustStream &, const StreamMessage &);
    static uint64_t allocation_size(const StreamMessage &);
};

struct FfiConverterTypeStreamingVersionRecord {
    static StreamingVersionRecord lift(RustBuffer);
    static RustBuffer lower(const StreamingVersionRecord &);
    static StreamingVersionRecord read(RustStream &);
    static void write(RustStream &, const StreamingVersionRecord &);
    static uint64_t allocation_size(const StreamingVersionRecord &);
};

struct FfiConverterTypeTlsConfigRecord {
    static TlsConfigRecord lift(RustBuffer);
    static RustBuffer lower(const TlsConfigRecord &);
    static TlsConfigRecord read(RustStream &);
    static void write(RustStream &, const TlsConfigRecord &);
    static uint64_t allocation_size(const TlsConfigRecord &);
};
struct FfiConverterCredentialKind {
    static CredentialKind lift(RustBuffer);
    static RustBuffer lower(const CredentialKind &);
    static CredentialKind read(RustStream &);
    static void write(RustStream &, const CredentialKind &);
    static uint64_t allocation_size(const CredentialKind &);
};
struct FfiConverterErrorSourceKind {
    static ErrorSourceKind lift(RustBuffer);
    static RustBuffer lower(const ErrorSourceKind &);
    static ErrorSourceKind read(RustStream &);
    static void write(RustStream &, const ErrorSourceKind &);
    static uint64_t allocation_size(const ErrorSourceKind &);
};

struct FfiConverterMarketDataError {
    static std::shared_ptr<MarketDataError> lift(RustBuffer buf);
    static RustBuffer lower(const MarketDataError &);
    static std::shared_ptr<MarketDataError> read(RustStream &stream);
    static void write(RustStream &stream, const MarketDataError &);
    static uint64_t allocation_size(const MarketDataError &);
};
struct FfiConverterMessageOverflowRecord {
    static MessageOverflowRecord lift(RustBuffer);
    static RustBuffer lower(const MessageOverflowRecord &);
    static MessageOverflowRecord read(RustStream &);
    static void write(RustStream &, const MessageOverflowRecord &);
    static uint64_t allocation_size(const MessageOverflowRecord &);
};
struct FfiConverterWebSocketEndpoint {
    static WebSocketEndpoint lift(RustBuffer);
    static RustBuffer lower(const WebSocketEndpoint &);
    static WebSocketEndpoint read(RustStream &);
    static void write(RustStream &, const WebSocketEndpoint &);
    static uint64_t allocation_size(const WebSocketEndpoint &);
};
struct FfiConverterOptionalUInt16 {
    static std::optional<uint16_t> lift(RustBuffer buf);
    static RustBuffer lower(const std::optional<uint16_t>& val);
    static std::optional<uint16_t> read(RustStream &stream);
    static void write(RustStream &stream, const std::optional<uint16_t>& value);
    static uint64_t allocation_size(const std::optional<uint16_t> &val);
};
struct FfiConverterOptionalUInt32 {
    static std::optional<uint32_t> lift(RustBuffer buf);
    static RustBuffer lower(const std::optional<uint32_t>& val);
    static std::optional<uint32_t> read(RustStream &stream);
    static void write(RustStream &stream, const std::optional<uint32_t>& value);
    static uint64_t allocation_size(const std::optional<uint32_t> &val);
};
struct FfiConverterOptionalInt32 {
    static std::optional<int32_t> lift(RustBuffer buf);
    static RustBuffer lower(const std::optional<int32_t>& val);
    static std::optional<int32_t> read(RustStream &stream);
    static void write(RustStream &stream, const std::optional<int32_t>& value);
    static uint64_t allocation_size(const std::optional<int32_t> &val);
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
struct FfiConverterOptionalTypeHealthCheckConfigRecord {
    static std::optional<HealthCheckConfigRecord> lift(RustBuffer buf);
    static RustBuffer lower(const std::optional<HealthCheckConfigRecord>& val);
    static std::optional<HealthCheckConfigRecord> read(RustStream &stream);
    static void write(RustStream &stream, const std::optional<HealthCheckConfigRecord>& value);
    static uint64_t allocation_size(const std::optional<HealthCheckConfigRecord> &val);
};
struct FfiConverterOptionalTypeMessageQueueConfigRecord {
    static std::optional<MessageQueueConfigRecord> lift(RustBuffer buf);
    static RustBuffer lower(const std::optional<MessageQueueConfigRecord>& val);
    static std::optional<MessageQueueConfigRecord> read(RustStream &stream);
    static void write(RustStream &stream, const std::optional<MessageQueueConfigRecord>& value);
    static uint64_t allocation_size(const std::optional<MessageQueueConfigRecord> &val);
};
struct FfiConverterOptionalTypeReconnectConfigRecord {
    static std::optional<ReconnectConfigRecord> lift(RustBuffer buf);
    static RustBuffer lower(const std::optional<ReconnectConfigRecord>& val);
    static std::optional<ReconnectConfigRecord> read(RustStream &stream);
    static void write(RustStream &stream, const std::optional<ReconnectConfigRecord>& value);
    static uint64_t allocation_size(const std::optional<ReconnectConfigRecord> &val);
};
struct FfiConverterOptionalTypeStreamingVersionRecord {
    static std::optional<StreamingVersionRecord> lift(RustBuffer buf);
    static RustBuffer lower(const std::optional<StreamingVersionRecord>& val);
    static std::optional<StreamingVersionRecord> read(RustStream &stream);
    static void write(RustStream &stream, const std::optional<StreamingVersionRecord>& value);
    static uint64_t allocation_size(const std::optional<StreamingVersionRecord> &val);
};
struct FfiConverterOptionalTypeTlsConfigRecord {
    static std::optional<TlsConfigRecord> lift(RustBuffer buf);
    static RustBuffer lower(const std::optional<TlsConfigRecord>& val);
    static std::optional<TlsConfigRecord> read(RustStream &stream);
    static void write(RustStream &stream, const std::optional<TlsConfigRecord>& value);
    static uint64_t allocation_size(const std::optional<TlsConfigRecord> &val);
};

struct FfiConverterMapStringString {
    static std::unordered_map<std::string, std::string> lift(RustBuffer);
    static RustBuffer lower(const std::unordered_map<std::string, std::string> &);
    static std::unordered_map<std::string, std::string> read(RustStream &);
    static void write(RustStream &, const std::unordered_map<std::string, std::string> &);
    static uint64_t allocation_size(const std::unordered_map<std::string, std::string> &);
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
/**
 * Check a set of credentials the way every client constructor does.
 *
 * A value that is empty or only whitespace counts as not provided; exactly
 * one of the three must remain. Wrappers that accept all three options call
 * this and pass the value of the returned kind to the matching constructor,
 * so the rule and the error (a `ConfigError`, code 1004) come from the core.
 */
CredentialKind validate_credentials(std::optional<std::string> api_key, std::optional<std::string> bearer_token, std::optional<std::string> sdk_token);
} // namespace marketdata_uniffi