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

    if (failures > 0) {
        return EXIT_FAILURE;
    }
    std::cout << "OK\n";
    return EXIT_SUCCESS;
}
