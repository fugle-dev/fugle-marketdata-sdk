// REST API 範例 - 取得股票報價
//
// 執行方式:
//   export FUGLE_API_KEY='your-api-key'
//   cd bindings/go/examples
//   go run rest_example.go
//
// 注意: 需要先 build native library:
//   cargo build -p marketdata-uniffi --release
//
// REST 方法回傳的是伺服器原始的 JSON 字串，用 encoding/json 解到自己
// 定義的 struct 即可。只宣告你要用的欄位，其餘會被忽略。

package main

import (
	"encoding/json"
	"fmt"
	"log"
	"os"

	mkt "github.com/fugle-dev/fugle-marketdata-go"
)

// Quote 只宣告這個範例會用到的欄位。
//
// referencePrice 是交易所計算漲跌與漲跌停價的基準，跟 previousClose 不
// 一定相同（除權息、停牌復牌後就會不同），要自己算漲跌幅請用它。
type Quote struct {
	Date           string   `json:"date"`
	Symbol         string   `json:"symbol"`
	Name           string   `json:"name"`
	LastPrice      *float64 `json:"lastPrice"`
	Change         *float64 `json:"change"`
	ChangePercent  *float64 `json:"changePercent"`
	ReferencePrice *float64 `json:"referencePrice"`
	OpenPrice      *float64 `json:"openPrice"`
	HighPrice      *float64 `json:"highPrice"`
	LowPrice       *float64 `json:"lowPrice"`
	Total          *struct {
		TradeVolume int64   `json:"tradeVolume"`
		TradeValue  float64 `json:"tradeValue"`
	} `json:"total"`
}

type Ticker struct {
	Symbol         string   `json:"symbol"`
	Name           string   `json:"name"`
	ReferencePrice *float64 `json:"referencePrice"`
	LimitUpPrice   *float64 `json:"limitUpPrice"`
	LimitDownPrice *float64 `json:"limitDownPrice"`
}

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
	//
	// 必填參數是位置參數，其餘篩選條件放進 *Params record；不需要篩選就傳 nil。
	fmt.Println("\n2. 取得 2330 報價...")
	body, err := client.Stock().Intraday().GetQuote("2330", nil)
	if err != nil {
		log.Fatalf("取得報價失敗: %v", err)
	}

	var quote Quote
	if err := json.Unmarshal([]byte(body), &quote); err != nil {
		log.Fatalf("解析報價失敗: %v", err)
	}

	// 3. 顯示報價資訊
	fmt.Println("\n=== 2330 台積電 報價 ===")
	fmt.Printf("日期: %s\n", quote.Date)
	fmt.Printf("代號: %s\n", quote.Symbol)
	fmt.Printf("名稱: %s\n", quote.Name)
	printPrice("參考價", quote.ReferencePrice)
	printPrice("最新價", quote.LastPrice)
	printPrice("漲跌", quote.Change)
	printPrice("漲跌幅", quote.ChangePercent)
	printPrice("開盤價", quote.OpenPrice)
	printPrice("最高價", quote.HighPrice)
	printPrice("最低價", quote.LowPrice)
	if quote.Total != nil {
		fmt.Printf("成交量: %d\n", quote.Total.TradeVolume)
		fmt.Printf("成交值: %.0f\n", quote.Total.TradeValue)
	}

	// 4. 取得 Ticker 資訊
	fmt.Println("\n3. 取得 2330 Ticker...")
	body, err = client.Stock().Intraday().GetTicker("2330", nil)
	if err != nil {
		log.Fatalf("取得 ticker 失敗: %v", err)
	}

	var ticker Ticker
	if err := json.Unmarshal([]byte(body), &ticker); err != nil {
		log.Fatalf("解析 ticker 失敗: %v", err)
	}

	fmt.Println("\n=== 2330 Ticker ===")
	fmt.Printf("代號: %s\n", ticker.Symbol)
	fmt.Printf("名稱: %s\n", ticker.Name)
	printPrice("參考價", ticker.ReferencePrice)
	printPrice("漲停價", ticker.LimitUpPrice)
	printPrice("跌停價", ticker.LimitDownPrice)

	// 5. 取得成交明細，只要最新 5 筆
	//
	// *Params record 的欄位全是指標；mkt.Uint32/Bool/String/Float64 是給字面值用的 helper。
	fmt.Println("\n4. 取得 2330 最新 5 筆成交明細...")
	body, err = client.Stock().Intraday().GetTrades("2330", &mkt.StockTradesParams{Limit: mkt.Uint32(5)})
	if err != nil {
		log.Fatalf("取得成交明細失敗: %v", err)
	}
	fmt.Println(body)

	fmt.Println("\n完成!")
}

// printPrice 印出一個可能不存在的價格欄位。
//
// 盤前、停牌或非交易時段，伺服器不會送出某些欄位，解出來就是 nil。
func printPrice(label string, v *float64) {
	if v == nil {
		return
	}
	fmt.Printf("%s: %.2f\n", label, *v)
}
