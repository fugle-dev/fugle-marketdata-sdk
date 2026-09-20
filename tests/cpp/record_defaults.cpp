// A zero-initialized config record must be the full default (#158).
//
// C++ has no wrapper over the generated records, so `ReconnectConfigRecord{}`
// is what users write. When `enabled` was a plain `bool`, `Record{}` meant
// `enabled = false` and silently turned auto-reconnect and health check off.
//
// The checks are split between two tests:
// - Here: `enabled` is `std::optional<bool>` and a zero-initialized record
//   leaves it unset. Against the old generated header this file does not
//   compile, which is the regression guard on the C++ side.
// - Rust, `zero_valued_records_are_the_core_defaults` in
//   uniffi/src/websocket.rs: an unset `enabled` resolves to the core default
//   (on). C++ cannot reach that conversion, so it is not checked here.
//
// The REST params records (#202) follow the same rule: every field is
// `std::optional` and `Record{}` sends nothing, so the method calls below
// only have to compile — `candles_sync("2330", std::nullopt)` is the
// no-timeframe call that used to be impossible. They are not executed: the
// client's base URL points at a closed port and nothing is sent.
//
// Build and run against the library built with the `cpp` feature:
//   c++ -std=c++20 -Ibindings/cpp tests/cpp/record_defaults.cpp \
//     bindings/cpp/marketdata_uniffi.cpp -L<lib dir> -lmarketdata_uniffi
#include "marketdata_uniffi.hpp"

#include <cstdlib>
#include <iostream>

using namespace marketdata_uniffi;

namespace {

int failures = 0;

void check(bool ok, const char *what) {
    if (!ok) {
        std::cerr << "FAIL: " << what << '\n';
        ++failures;
    }
}

class NoopListener : public WebSocketListener {
public:
    void on_connected() override {}
    void on_authenticated(std::optional<std::string>) override {}
    void on_unauthenticated(std::optional<std::string>) override {}
    void on_disconnected(bool) override {}
    void on_error(const ErrorInfo &) override {}
    void on_reconnecting(uint32_t) override {}
    void on_reconnect_failed(uint32_t) override {}
    void on_messages_dropped(uint64_t) override {}
    void on_message(const StreamMessage &) override {}
};

// Crosses the FFI boundary with the given records; construction only, no
// connection is made.
void construct(std::optional<ReconnectConfigRecord> reconnect,
               std::optional<HealthCheckConfigRecord> health_check) {
    auto client = WebSocketClient::new_with_config(
        "test-key", std::make_shared<NoopListener>(), WebSocketEndpoint::kStock,
        reconnect, health_check);
    check(client != nullptr, "client constructed");
}

} // namespace

int main() {
    ReconnectConfigRecord reconnect{};
    check(!reconnect.enabled.has_value(), "ReconnectConfigRecord{} leaves enabled unset");

    HealthCheckConfigRecord health_check{};
    check(!health_check.enabled.has_value(), "HealthCheckConfigRecord{} leaves enabled unset");
    check(!health_check.probe_enabled, "HealthCheckConfigRecord{} leaves probing off");

    // Only some fields set: `enabled` stays unset.
    ReconnectConfigRecord partial{.max_attempts = 3};
    check(!partial.enabled.has_value(), "a partial ReconnectConfigRecord leaves enabled unset");

    construct(std::nullopt, std::nullopt);
    construct(reconnect, health_check);
    construct(ReconnectConfigRecord{.enabled = false}, HealthCheckConfigRecord{.enabled = false});
    construct(ReconnectConfigRecord{.enabled = true}, HealthCheckConfigRecord{.enabled = true});

    // REST params records: `Record{}` leaves every field unset (#202).
    StockTradesParams trades{};
    check(!trades.odd_lot.has_value() && !trades.offset.has_value() && !trades.limit.has_value() &&
              !trades.sort.has_value() && !trades.is_trial.has_value(),
          "StockTradesParams{} leaves every field unset");
    StockTradesParams five{.limit = 5};
    check(!five.odd_lot.has_value() && five.limit == 5u, "a partial StockTradesParams leaves the rest unset");
    MoversParams movers{};
    check(!movers.gt.has_value() && !movers.eq.has_value(), "MoversParams{} leaves the price bounds unset");
    SubscribeOptions opts{};
    check(!opts.after_hours.has_value() && !opts.intraday_odd_lot.has_value(),
          "SubscribeOptions{} leaves both sessions unset");

    // The record-taking signatures compile with `std::nullopt` and with an
    // aggregate. Not run: `if (false)` keeps the calls out of the way while
    // the compiler still checks them.
    if (false) {
        auto rest = new_rest_client_with_api_key_and_tls("test-key", std::string("http://127.0.0.1:9"),
                                                        TlsConfigRecord{});
        auto intraday = rest->stock()->intraday();
        (void)intraday->candles_sync("2330", std::nullopt);
        (void)intraday->trades_sync("2330", StockTradesParams{.limit = 5});
        (void)intraday->tickers_sync("EQUITY", StockTickersParams{});
        (void)rest->stock()->snapshot()->movers_sync("TSE", "up", "percent", std::nullopt);
        (void)rest->futopt()->intraday()->quote_sync("TXFE6", AfterHoursParams{.after_hours = true});
        auto ws = WebSocketClient::new_with_config("test-key", std::make_shared<NoopListener>(),
                                                   WebSocketEndpoint::kStock, std::nullopt, std::nullopt);
        ws->subscribe_sync("trades", {"2330", "2317"}, std::nullopt);
        ws->subscribe_sync("trades", {"2330"}, SubscribeOptions{.intraday_odd_lot = true});
    }

    if (failures > 0) {
        return EXIT_FAILURE;
    }
    std::cout << "OK\n";
    return EXIT_SUCCESS;
}
