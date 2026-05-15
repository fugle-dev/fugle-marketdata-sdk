// REST API 範例 - 取得股票報價
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
//           tw.com.fugle.marketdata.examples.RestExample
//
// 或使用 Makefile:
//   make build-java
//   cd bindings/java && java -cp ... RestExample

package tw.com.fugle.marketdata.examples;

import tw.com.fugle.marketdata.FugleRestClient;
import tw.com.fugle.marketdata.FugleException;
import tw.com.fugle.marketdata.generated.Quote;
import tw.com.fugle.marketdata.generated.Ticker;
import tw.com.fugle.marketdata.generated.TotalStats;

public class RestExample {

    public static void main(String[] args) {
        // 從環境變數取得 API Key
        String apiKey = System.getenv("FUGLE_API_KEY");
        if (apiKey == null || apiKey.isEmpty()) {
            System.out.println("請設定 FUGLE_API_KEY 環境變數");
            System.out.println("  export FUGLE_API_KEY='your-api-key'");
            System.exit(1);
        }

        try {
            // 1. 建立 REST Client (使用 Builder 模式)
            System.out.println("1. 建立 REST Client...");
            FugleRestClient client = FugleRestClient.builder()
                .apiKey(apiKey)
                .build();

            // 2. 取得股票報價 (TSMC 2330)
            System.out.println("\n2. 取得 2330 報價...");
            Quote quote = client.stock().intraday().getQuote("2330");

            // 3. 顯示報價資訊
            System.out.println("\n=== 2330 台積電 報價 ===");
            System.out.printf("日期: %s%n", quote.date());
            System.out.printf("代號: %s%n", quote.symbol());

            if (quote.name() != null) {
                System.out.printf("名稱: %s%n", quote.name());
            }
            if (quote.lastPrice() != null) {
                System.out.printf("最新價: %.2f%n", quote.lastPrice());
            }
            if (quote.change() != null) {
                System.out.printf("漲跌: %.2f%n", quote.change());
            }
            if (quote.changePercent() != null) {
                System.out.printf("漲跌幅: %.2f%%%n", quote.changePercent());
            }
            if (quote.openPrice() != null) {
                System.out.printf("開盤價: %.2f%n", quote.openPrice());
            }
            if (quote.highPrice() != null) {
                System.out.printf("最高價: %.2f%n", quote.highPrice());
            }
            if (quote.lowPrice() != null) {
                System.out.printf("最低價: %.2f%n", quote.lowPrice());
            }

            TotalStats total = quote.total();
            if (total != null) {
                if (total.tradeVolume() != null) {
                    System.out.printf("成交量: %d%n", total.tradeVolume());
                }
                if (total.tradeValue() != null) {
                    System.out.printf("成交值: %.0f%n", total.tradeValue());
                }
            }

            // 4. 取得 Ticker 資訊
            System.out.println("\n3. 取得 2330 Ticker...");
            Ticker ticker = client.stock().intraday().getTicker("2330");

            System.out.println("\n=== 2330 Ticker ===");
            System.out.printf("代號: %s%n", ticker.symbol());

            if (ticker.name() != null) {
                System.out.printf("名稱: %s%n", ticker.name());
            }
            if (ticker.referencePrice() != null) {
                System.out.printf("參考價: %.2f%n", ticker.referencePrice());
            }
            if (ticker.limitUpPrice() != null) {
                System.out.printf("漲停價: %.2f%n", ticker.limitUpPrice());
            }
            if (ticker.limitDownPrice() != null) {
                System.out.printf("跌停價: %.2f%n", ticker.limitDownPrice());
            }

            // 5. 使用 async 方式取得報價
            System.out.println("\n4. 使用 async 方式取得 2317 報價...");
            client.stock().intraday().getQuoteAsync("2317")
                .thenAccept(q -> {
                    System.out.println("\n=== 2317 鴻海 報價 (async) ===");
                    System.out.printf("代號: %s%n", q.symbol());
                    if (q.name() != null) {
                        System.out.printf("名稱: %s%n", q.name());
                    }
                    if (q.lastPrice() != null) {
                        System.out.printf("最新價: %.2f%n", q.lastPrice());
                    }
                })
                .exceptionally(e -> {
                    System.err.println("Async 取得報價失敗: " + e.getMessage());
                    return null;
                })
                .join(); // 等待完成

            System.out.println("\n完成!");

        } catch (FugleException e) {
            System.err.println("Fugle API 錯誤: " + e.getMessage());
            e.printStackTrace();
            System.exit(1);
        } catch (Exception e) {
            System.err.println("錯誤: " + e.getMessage());
            e.printStackTrace();
            System.exit(1);
        }
    }
}
