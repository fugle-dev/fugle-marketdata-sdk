// WebSocket benchmark client -- new Rust-core SDK (Java / UniFFI+JNA binding)
//
// Connects to the mock server, subscribes, receives data messages, and
// reports throughput / latency / memory metrics as a single JSON line on stdout.
//
// Compile:
//   javac -cp CLASSPATH WebSocketBenchmark.java
//
// Run:
//   java -cp CLASSPATH -Djna.library.path=... WebSocketBenchmark --url ws://localhost:8765 --timeout 60000

import tw.com.fugle.marketdata.*;
import tw.com.fugle.marketdata.generated.*;

import java.lang.management.ManagementFactory;
import java.lang.management.OperatingSystemMXBean;
import java.util.*;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.concurrent.atomic.AtomicLong;

public class WebSocketBenchmark {

    // Metrics state
    static final AtomicInteger received = new AtomicInteger(0);
    static final AtomicLong t0 = new AtomicLong(0);
    static final AtomicBoolean t0Set = new AtomicBoolean(false);
    static final double[] latencies = new double[500000];
    static final AtomicInteger latIdx = new AtomicInteger(0);
    static volatile Map<String, Object> serverStats = null;
    static final CountDownLatch done = new CountDownLatch(1);

    public static void main(String[] args) throws Exception {
        // Parse CLI args
        String url = flagValue(args, "url", "ws://localhost:8765");
        int timeout = Integer.parseInt(flagValue(args, "timeout", "60000"));

        // Force GC before benchmark
        System.gc();
        Thread.sleep(100);

        long startMem = Runtime.getRuntime().totalMemory() - Runtime.getRuntime().freeMemory();
        long startCpuNs = ManagementFactory.getThreadMXBean().getCurrentThreadCpuTime();
        long startTimeMs = System.currentTimeMillis();

        // Create WebSocket client with direct callback listener (no queue overhead)
        WebSocketListener listener = new WebSocketListener() {
            @Override
            public void onConnected() {}

            @Override
            public void onDisconnected() {}

            @Override
            @SuppressWarnings("unchecked")
            public void onMessage(StreamMessage msg) {
                String ev = msg.event();

                if ("warmup".equals(ev)) return;

                if ("bench_done".equals(ev)) {
                    String dataJson = msg.dataJson();
                    if (dataJson != null) {
                        try {
                            serverStats = parseJson(dataJson);
                        } catch (Exception e) { /* ignore */ }
                    }
                    done.countDown();
                    return;
                }

                if ("data".equals(ev)) {
                    if (t0Set.compareAndSet(false, true)) {
                        t0.set(System.currentTimeMillis());
                    }
                    received.incrementAndGet();

                    String dataJson = msg.dataJson();
                    if (dataJson != null) {
                        try {
                            Map<String, Object> data = parseJson(dataJson);
                            Object tsObj = data.get("server_ts");
                            if (tsObj instanceof Number) {
                                long serverTs = ((Number) tsObj).longValue();
                                long now = System.currentTimeMillis();
                                int idx = latIdx.getAndIncrement();
                                if (idx < latencies.length) {
                                    latencies[idx] = now - serverTs;
                                }
                            }
                        } catch (Exception e) { /* ignore */ }
                    }
                }
            }

            @Override
            public void onError(String errorMessage) {
                System.err.println("error: " + errorMessage);
            }

            @Override
            public void onReconnecting(Integer attempt) {}

            @Override
            public void onReconnectFailed(Integer attempts) {}
        };

        // Use raw UniFFI WebSocketClient with custom URL
        WebSocketClient client = WebSocketClient.newWithUrl(
            "bench-key", listener, WebSocketEndpoint.STOCK,
            url, null, null
        );

        client.connect().join();
        client.subscribe("trades", "2330").join();

        // Wait for bench_done or timeout
        if (!done.await(timeout, TimeUnit.MILLISECONDS)) {
            System.err.println("TIMEOUT: did not receive bench_done within " + timeout + " ms");
        }

        // Report
        long endTimeMs = System.currentTimeMillis();
        long endCpuNs = ManagementFactory.getThreadMXBean().getCurrentThreadCpuTime();
        long endMem = Runtime.getRuntime().totalMemory() - Runtime.getRuntime().freeMemory();

        long elapsed = t0Set.get() ? endTimeMs - t0.get() : 0;
        double cpuUserMs = (endCpuNs - startCpuNs) / 1_000_000.0;
        double memDeltaMb = (endMem - startMem) / 1_000_000.0;

        int count = received.get();
        int latCount = latIdx.get();
        double[] lats = Arrays.copyOf(latencies, latCount);
        Arrays.sort(lats);

        int msgsPerSec = elapsed > 0 ? (int) (count / (double) elapsed * 1000) : 0;

        Integer ssCount = null;
        Double ssMps = null;
        if (serverStats != null) {
            Object c = serverStats.get("count");
            if (c instanceof Number) ssCount = ((Number) c).intValue();
            Object m = serverStats.get("server_msgs_per_sec");
            if (m instanceof Number) ssMps = ((Number) m).doubleValue();
        }

        // Output JSON
        StringBuilder sb = new StringBuilder("{");
        sb.append("\"sdk\":\"rust-core-java\"");
        sb.append(",\"count\":").append(count);
        appendNullableInt(sb, "expected", ssCount);
        appendNullableInt(sb, "lost", ssCount != null ? ssCount - count : null);
        sb.append(",\"elapsed_ms\":").append(elapsed);
        sb.append(",\"msgs_per_sec\":").append(msgsPerSec);
        appendNullableDouble(sb, "latency_p50_ms", percentile(lats, 50));
        appendNullableDouble(sb, "latency_p99_ms", percentile(lats, 99));
        appendNullableDouble(sb, "latency_min_ms", lats.length > 0 ? lats[0] : null);
        appendNullableDouble(sb, "latency_max_ms", lats.length > 0 ? lats[lats.length - 1] : null);
        sb.append(",\"mem_rss_delta_mb\":").append(round1(memDeltaMb));
        sb.append(",\"cpu_user_ms\":").append(round1(cpuUserMs));
        sb.append(",\"cpu_system_ms\":0.0");
        appendNullableDouble(sb, "server_msgs_per_sec", ssMps);
        sb.append("}");

        System.out.println(sb);

        client.disconnect().join();
        Thread.sleep(200);
        System.exit(0);
    }

    static String flagValue(String[] args, String name, String fallback) {
        for (int i = 0; i < args.length - 1; i++) {
            if (args[i].equals("--" + name)) return args[i + 1];
        }
        return fallback;
    }

    static Double percentile(double[] sorted, int p) {
        if (sorted.length == 0) return null;
        int idx = Math.min((int) Math.ceil(p / 100.0 * sorted.length) - 1, sorted.length - 1);
        return Math.round(sorted[Math.max(0, idx)] * 100.0) / 100.0;
    }

    static double round1(double v) {
        return Math.round(v * 10.0) / 10.0;
    }

    static void appendNullableInt(StringBuilder sb, String key, Integer val) {
        sb.append(",\"").append(key).append("\":");
        sb.append(val != null ? val.toString() : "null");
    }

    static void appendNullableDouble(StringBuilder sb, String key, Double val) {
        sb.append(",\"").append(key).append("\":");
        sb.append(val != null ? val.toString() : "null");
    }

    @SuppressWarnings("unchecked")
    static Map<String, Object> parseJson(String json) {
        // Minimal JSON parser for benchmark — only handles flat objects with string/number values
        Map<String, Object> map = new HashMap<>();
        json = json.trim();
        if (json.startsWith("{")) json = json.substring(1);
        if (json.endsWith("}")) json = json.substring(0, json.length() - 1);

        for (String pair : json.split(",")) {
            String[] kv = pair.split(":", 2);
            if (kv.length != 2) continue;
            String key = kv[0].trim().replace("\"", "");
            String val = kv[1].trim();
            if (val.startsWith("\"") && val.endsWith("\"")) {
                map.put(key, val.substring(1, val.length() - 1));
            } else if ("null".equals(val)) {
                map.put(key, null);
            } else if ("true".equals(val)) {
                map.put(key, true);
            } else if ("false".equals(val)) {
                map.put(key, false);
            } else {
                try {
                    if (val.contains(".")) {
                        map.put(key, Double.parseDouble(val));
                    } else {
                        map.put(key, Long.parseLong(val));
                    }
                } catch (NumberFormatException e) {
                    map.put(key, val);
                }
            }
        }
        return map;
    }
}
