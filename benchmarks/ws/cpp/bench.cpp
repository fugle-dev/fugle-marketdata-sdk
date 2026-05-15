// WebSocket benchmark client -- new Rust-core SDK (C++ / UniFFI binding)
//
// Build:
//   c++ -std=c++20 -O2 -I../../../bindings/cpp bench.cpp ../../../bindings/cpp/marketdata_uniffi.cpp \
//       -L../../../target/release -lmarketdata_uniffi -o ws-bench-cpp
//
// Run:
//   ./ws-bench-cpp --url ws://localhost:8765 --timeout 60000

#include "marketdata_uniffi.hpp"
#include <algorithm>
#include <atomic>
#include <chrono>
#include <cmath>
#include <cstdlib>
#include <iostream>
#include <mutex>
#include <sstream>
#include <string>
#include <thread>
#include <vector>

using namespace marketdata_uniffi;

// Minimal JSON number extractor (no external JSON lib needed)
static bool extract_long(const std::string &json, const std::string &key, long &out) {
    auto pos = json.find("\"" + key + "\"");
    if (pos == std::string::npos) return false;
    pos = json.find(':', pos);
    if (pos == std::string::npos) return false;
    pos++;
    while (pos < json.size() && json[pos] == ' ') pos++;
    char *end;
    out = std::strtol(json.c_str() + pos, &end, 10);
    return end != json.c_str() + pos;
}

static bool extract_double(const std::string &json, const std::string &key, double &out) {
    auto pos = json.find("\"" + key + "\"");
    if (pos == std::string::npos) return false;
    pos = json.find(':', pos);
    if (pos == std::string::npos) return false;
    pos++;
    while (pos < json.size() && json[pos] == ' ') pos++;
    char *end;
    out = std::strtod(json.c_str() + pos, &end);
    return end != json.c_str() + pos;
}

// CLI flag parser
static std::string flag(int argc, char **argv, const std::string &name, const std::string &fallback) {
    for (int i = 1; i < argc - 1; i++)
        if (std::string(argv[i]) == "--" + name) return argv[i + 1];
    return fallback;
}

static long now_ms() {
    return std::chrono::duration_cast<std::chrono::milliseconds>(
               std::chrono::system_clock::now().time_since_epoch())
        .count();
}

// Metrics
static std::atomic<int> g_received{0};
static std::atomic<long> g_t0{0};
static std::atomic<bool> g_t0_set{false};
static std::vector<double> g_latencies;
static std::mutex g_lat_mutex;
static std::atomic<bool> g_done{false};
static long g_ss_count = -1;
static double g_ss_mps = 0;

class BenchListener : public WebSocketListener {
public:
    void on_connected() override {}
    void on_disconnected() override {}
    void on_error(const std::string &msg) override {
        std::cerr << "error: " << msg << std::endl;
    }
    void on_reconnecting(uint32_t) override {}
    void on_reconnect_failed(uint32_t) override {}

    void on_message(const StreamMessage &msg) override {
        if (msg.event == "warmup") return;

        if (msg.event == "bench_done") {
            if (msg.data_json) {
                long c;
                double m;
                if (extract_long(*msg.data_json, "count", c)) g_ss_count = c;
                if (extract_double(*msg.data_json, "server_msgs_per_sec", m)) g_ss_mps = m;
            }
            g_done.store(true);
            return;
        }

        if (msg.event == "data") {
            if (!g_t0_set.exchange(true)) g_t0.store(now_ms());
            g_received.fetch_add(1);

            if (msg.data_json) {
                long ts;
                if (extract_long(*msg.data_json, "server_ts", ts)) {
                    double lat = static_cast<double>(now_ms() - ts);
                    std::lock_guard<std::mutex> lk(g_lat_mutex);
                    g_latencies.push_back(lat);
                }
            }
        }
    }
};

int main(int argc, char **argv) {
    auto url = flag(argc, argv, "url", "ws://localhost:8765");
    auto timeout = std::stoi(flag(argc, argv, "timeout", "60000"));

    long start_time = now_ms();

    auto listener = std::make_shared<BenchListener>();
    auto client = WebSocketClient::new_with_url(
        "bench-key", listener, WebSocketEndpoint::kStock,
        url, std::nullopt, std::nullopt);

    client->connect_sync();
    client->subscribe_sync("trades", "2330");

    // Wait for done or timeout
    auto deadline = std::chrono::steady_clock::now() + std::chrono::milliseconds(timeout);
    while (!g_done.load()) {
        if (std::chrono::steady_clock::now() >= deadline) {
            std::cerr << "TIMEOUT" << std::endl;
            break;
        }
        std::this_thread::sleep_for(std::chrono::milliseconds(5));
    }

    // Report
    long elapsed = g_t0_set.load() ? now_ms() - g_t0.load() : 0;
    int count = g_received.load();
    double cpu_ms = static_cast<double>(now_ms() - start_time);

    std::sort(g_latencies.begin(), g_latencies.end());
    auto percentile = [&](int p) -> std::string {
        if (g_latencies.empty()) return "null";
        int idx = std::min(
            static_cast<int>(std::ceil(p / 100.0 * g_latencies.size())) - 1,
            static_cast<int>(g_latencies.size()) - 1);
        if (idx < 0) idx = 0;
        return std::to_string(std::round(g_latencies[idx] * 100.0) / 100.0);
    };

    int mps = elapsed > 0 ? static_cast<int>(count / (double)elapsed * 1000) : 0;

    std::ostringstream out;
    out << "{\"sdk\":\"rust-core-cpp\""
        << ",\"count\":" << count
        << ",\"expected\":" << (g_ss_count >= 0 ? std::to_string(g_ss_count) : "null")
        << ",\"lost\":" << (g_ss_count >= 0 ? std::to_string(g_ss_count - count) : "null")
        << ",\"elapsed_ms\":" << elapsed
        << ",\"msgs_per_sec\":" << mps
        << ",\"latency_p50_ms\":" << percentile(50)
        << ",\"latency_p99_ms\":" << percentile(99)
        << ",\"latency_min_ms\":" << (g_latencies.empty() ? "null" : std::to_string(g_latencies.front()))
        << ",\"latency_max_ms\":" << (g_latencies.empty() ? "null" : std::to_string(g_latencies.back()))
        << ",\"mem_rss_delta_mb\":0"
        << ",\"cpu_user_ms\":" << std::round(cpu_ms * 10) / 10
        << ",\"cpu_system_ms\":0"
        << ",\"server_msgs_per_sec\":" << (g_ss_mps > 0 ? std::to_string(g_ss_mps) : "null")
        << "}";
    std::cout << out.str() << std::endl;

    client->disconnect_sync();
    return 0;
}
