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
struct HealthCheckConfigRecord;
struct ReconnectConfigRecord;
struct StreamMessage;
struct StreamingVersionRecord;
struct TlsConfigRecord;
struct MarketDataError;
enum class WebSocketEndpoint;


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
    static std::shared_ptr<WebSocketClient> new_with_full_config(const std::string &api_key, const std::shared_ptr<WebSocketListener> &listener, const WebSocketEndpoint &endpoint, std::optional<std::string> base_url, std::optional<ReconnectConfigRecord> reconnect_config, std::optional<HealthCheckConfigRecord> health_check_config, std::optional<TlsConfigRecord> tls, std::optional<StreamingVersionRecord> version);
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

struct FfiConverterTypeHealthCheckConfigRecord {
    static HealthCheckConfigRecord lift(RustBuffer);
    static RustBuffer lower(const HealthCheckConfigRecord &);
    static HealthCheckConfigRecord read(RustStream &);
    static void write(RustStream &, const HealthCheckConfigRecord &);
    static uint64_t allocation_size(const HealthCheckConfigRecord &);
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