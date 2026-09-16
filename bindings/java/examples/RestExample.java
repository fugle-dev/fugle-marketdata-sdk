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
//
// REST 方法回傳的是伺服器原始的 JSON 字串。本專案不綁定 JSON 函式庫，
// 這個範例直接印出原文；實際使用時用你慣用的函式庫解析即可，例如 Jackson:
//
//   ObjectMapper mapper = new ObjectMapper();
//   JsonNode quote = mapper.readTree(client.stock().intraday().getQuote("2330"));
//   // 伺服器沒送的欄位是不存在，而不是 null，讀之前先檢查
//   if (quote.has("lastPrice")) {
//       System.out.printf("最新價: %.2f%n", quote.get("lastPrice").asDouble());
//   }

package tw.com.fugle.marketdata.examples;

import tw.com.fugle.marketdata.FugleRestClient;
import tw.com.fugle.marketdata.FugleException;

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
            String quote = client.stock().intraday().getQuote("2330");

            System.out.println("\n=== 2330 台積電 報價 ===");
            System.out.println(quote);

            // 3. 取得 Ticker 資訊
            System.out.println("\n3. 取得 2330 Ticker...");
            String ticker = client.stock().intraday().getTicker("2330");

            System.out.println("\n=== 2330 Ticker ===");
            System.out.println(ticker);

            // 4. 使用 async 方式取得報價
            System.out.println("\n4. 使用 async 方式取得 2317 報價...");
            client.stock().intraday().getQuoteAsync("2317")
                .thenAccept(json -> {
                    System.out.println("\n=== 2317 鴻海 報價 (async) ===");
                    System.out.println(json);
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
