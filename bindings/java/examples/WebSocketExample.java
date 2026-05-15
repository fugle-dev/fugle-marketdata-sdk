// WebSocket 串流範例 - 即時行情
//
// 執行方式:
//   1. 先 build native library:
//      cargo build -p marketdata-uniffi --release
//
//   2. 設定環境變數:
//      export FUGLE_API_KEY='your-api-key'
//      export JAVA_HOME=/path/to/java21
//
//   3. 編譯並執行:
//      cd bindings/java
//      ./gradlew compileJava
//      java -cp "build/classes/java/main:$(find ~/.gradle -name 'jna-*.jar' | head -1)" \
//           -Djna.library.path=../../target/release \
//           tw.com.fugle.marketdata.examples.WebSocketExample
//
// 注意: 按 Ctrl+C 停止，或等待 30 秒自動結束

package tw.com.fugle.marketdata.examples;

import tw.com.fugle.marketdata.FugleWebSocketClient;
import tw.com.fugle.marketdata.FugleException;
import tw.com.fugle.marketdata.generated.StreamMessage;

import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicBoolean;

public class WebSocketExample {

    private static final AtomicBoolean running = new AtomicBoolean(true);

    public static void main(String[] args) {
        // 從環境變數取得 API Key
        String apiKey = System.getenv("FUGLE_API_KEY");
        if (apiKey == null || apiKey.isEmpty()) {
            System.out.println("請設定 FUGLE_API_KEY 環境變數");
            System.out.println("  export FUGLE_API_KEY='your-api-key'");
            System.exit(1);
        }

        // 設定中斷信號處理 (Ctrl+C)
        Runtime.getRuntime().addShutdownHook(new Thread(() -> {
            System.out.println("\n收到中斷信號，結束程式...");
            running.set(false);
        }));

        FugleWebSocketClient client = null;
        int messageCount = 0;

        try {
            // 1. 建立 WebSocket Client (使用 Builder 模式 + Pull 模式)
            System.out.println("1. 建立 WebSocket Client...");
            client = FugleWebSocketClient.builder()
                .apiKey(apiKey)
                .stock()              // 股票市場 (預設)
                .queueCapacity(100)   // 訊息佇列容量
                .build();

            // 2. 連線
            System.out.println("2. 連線中...");
            client.connect().join();
            System.out.println("   已連線!");

            // 3. 訂閱 2330 成交明細
            System.out.println("3. 訂閱 2330 trades...");
            client.subscribe("trades", "2330").join();

            // 4. 接收訊息 (Pull 模式: 用 poll 取得訊息)
            System.out.println("4. 等待訊息 (按 Ctrl+C 停止，或等待 30 秒)...\n");

            long startTime = System.currentTimeMillis();
            long timeoutMs = 30_000; // 30 秒超時

            while (running.get()) {
                // 檢查是否超時
                if (System.currentTimeMillis() - startTime > timeoutMs) {
                    System.out.println("\n30 秒到，自動結束...");
                    break;
                }

                // 用 poll 取得訊息 (1 秒超時)
                StreamMessage msg = client.poll(1, TimeUnit.SECONDS);

                if (msg != null) {
                    messageCount++;
                    printMessage(msg);
                }

                // 檢查錯誤
                if (client.hasErrors()) {
                    String error = client.pollError();
                    if (error != null) {
                        System.out.println("  [錯誤] " + error);
                    }
                }
            }

        } catch (FugleException e) {
            System.err.println("Fugle API 錯誤: " + e.getMessage());
            e.printStackTrace();
        } catch (InterruptedException e) {
            System.out.println("被中斷");
            Thread.currentThread().interrupt();
        } catch (Exception e) {
            System.err.println("錯誤: " + e.getMessage());
            e.printStackTrace();
        } finally {
            // 清理
            System.out.printf("%n=== 總共收到 %d 則訊息 ===%n", messageCount);
            System.out.println("斷線中...");

            if (client != null) {
                try {
                    // 用 timeout 避免 disconnect 卡住
                    client.disconnect()
                        .orTimeout(3, TimeUnit.SECONDS)
                        .handle((v, e) -> {
                            if (e != null) {
                                System.out.println("斷線 timeout，強制結束");
                            }
                            return null;
                        })
                        .join();
                } catch (Exception e) {
                    // ignore
                }
            }
            System.out.println("完成!");
        }
    }

    private static void printMessage(StreamMessage msg) {
        String event = msg.event();
        String channel = msg.channel() != null ? msg.channel() : "";
        String symbol = msg.symbol() != null ? msg.symbol() : "";

        switch (event) {
            case "subscribed":
                System.out.printf("  [訂閱成功] channel=%s, symbol=%s%n", channel, symbol);
                break;

            case "snapshot":
            case "data":
                String prefix = event.equals("snapshot") ? "[快照]" : "[即時]";
                System.out.printf("  %s %s:%s", prefix, channel, symbol);

                // 解析 dataJson 取得詳細資料
                String dataJson = msg.dataJson();
                if (dataJson != null && !dataJson.isEmpty()) {
                    // 簡單解析 price 和 volume (實際使用建議用 Jackson/Gson)
                    if (dataJson.contains("\"price\"")) {
                        String price = extractJsonValue(dataJson, "price");
                        if (price != null) {
                            System.out.printf(" 價格=%s", price);
                        }
                    }
                    if (dataJson.contains("\"volume\"")) {
                        String volume = extractJsonValue(dataJson, "volume");
                        if (volume != null) {
                            System.out.printf(" 量=%s", volume);
                        }
                    }
                }
                System.out.println();
                break;

            case "heartbeat":
                System.out.println("  [心跳]");
                break;

            default:
                System.out.printf("  [%s] channel=%s symbol=%s%n", event, channel, symbol);
                if (msg.dataJson() != null) {
                    System.out.printf("    data: %s%n", msg.dataJson());
                }
        }
    }

    // 簡單的 JSON value 提取 (實際使用建議用 Jackson/Gson)
    private static String extractJsonValue(String json, String key) {
        String pattern = "\"" + key + "\":";
        int idx = json.indexOf(pattern);
        if (idx < 0) return null;

        int start = idx + pattern.length();
        // 跳過空白
        while (start < json.length() && Character.isWhitespace(json.charAt(start))) {
            start++;
        }
        if (start >= json.length()) return null;

        int end = start;
        char c = json.charAt(start);

        if (c == '"') {
            // 字串值
            start++;
            end = json.indexOf('"', start);
            if (end < 0) return null;
            return json.substring(start, end);
        } else {
            // 數值
            while (end < json.length()) {
                c = json.charAt(end);
                if (c == ',' || c == '}' || c == ']' || Character.isWhitespace(c)) {
                    break;
                }
                end++;
            }
            return json.substring(start, end);
        }
    }
}
