// REST API 範例 - 取得股票報價
//
// 執行方式:
//   export FUGLE_API_KEY='your-api-key'
//   cd bindings/go/examples
//   go run rest_example.go
//
// 注意: 需要先 build native library:
//   cargo build -p marketdata-uniffi --release

package main

import (
	"fmt"
	"log"
	"os"

	mkt "github.com/fugle-dev/fugle-marketdata-go"
)

func main() {
	// 從環境變數取得 API Key
	apiKey := os.Getenv("FUGLE_API_KEY")
	if apiKey == "" {
		fmt.Println("請設定 FUGLE_API_KEY 環境變數")
		fmt.Println("  export FUGLE_API_KEY='your-api-key'")
		os.Exit(1)
	}

	// 1. 建立 REST Client
	fmt.Println("1. 建立 REST Client...")
	client, err := mkt.NewRestClientWithApiKey(apiKey)
	if err != nil {
		log.Fatalf("建立 client 失敗: %v", err)
	}
	defer client.Destroy()

	// 2. 取得股票報價 (TSMC 2330)
	fmt.Println("\n2. 取得 2330 報價...")
	quote, err := client.Stock().Intraday().GetQuote("2330")
	if err != nil {
		log.Fatalf("取得報價失敗: %v", err)
	}

	// 3. 顯示報價資訊
	fmt.Println("\n=== 2330 台積電 報價 ===")
	fmt.Printf("日期: %s\n", quote.Date)
	fmt.Printf("代號: %s\n", quote.Symbol)
	if quote.Name != nil {
		fmt.Printf("名稱: %s\n", *quote.Name)
	}
	if quote.LastPrice != nil {
		fmt.Printf("最新價: %.2f\n", *quote.LastPrice)
	}
	if quote.Change != nil {
		fmt.Printf("漲跌: %.2f\n", *quote.Change)
	}
	if quote.ChangePercent != nil {
		fmt.Printf("漲跌幅: %.2f%%\n", *quote.ChangePercent)
	}
	if quote.OpenPrice != nil {
		fmt.Printf("開盤價: %.2f\n", *quote.OpenPrice)
	}
	if quote.HighPrice != nil {
		fmt.Printf("最高價: %.2f\n", *quote.HighPrice)
	}
	if quote.LowPrice != nil {
		fmt.Printf("最低價: %.2f\n", *quote.LowPrice)
	}
	if quote.Total != nil {
		fmt.Printf("成交量: %d\n", quote.Total.TradeVolume)
		fmt.Printf("成交值: %.0f\n", quote.Total.TradeValue)
	}

	// 4. 取得 Ticker 資訊
	fmt.Println("\n3. 取得 2330 Ticker...")
	ticker, err := client.Stock().Intraday().GetTicker("2330")
	if err != nil {
		log.Fatalf("取得 ticker 失敗: %v", err)
	}

	fmt.Println("\n=== 2330 Ticker ===")
	fmt.Printf("代號: %s\n", ticker.Symbol)
	if ticker.Name != nil {
		fmt.Printf("名稱: %s\n", *ticker.Name)
	}
	if ticker.ReferencePrice != nil {
		fmt.Printf("參考價: %.2f\n", *ticker.ReferencePrice)
	}
	if ticker.LimitUpPrice != nil {
		fmt.Printf("漲停價: %.2f\n", *ticker.LimitUpPrice)
	}
	if ticker.LimitDownPrice != nil {
		fmt.Printf("跌停價: %.2f\n", *ticker.LimitDownPrice)
	}

	fmt.Println("\n完成!")
}
