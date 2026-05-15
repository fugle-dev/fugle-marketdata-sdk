// WebSocket 串流範例 - 即時行情
//
// 執行方式:
//   export FUGLE_API_KEY='your-api-key'
//   cd bindings/go/examples
//   go run websocket_example.go
//
// 注意: 需要先 build native library:
//   cargo build -p marketdata-uniffi --release

package main

import (
	"encoding/json"
	"fmt"
	"log"
	"os"
	"os/signal"
	"syscall"
	"time"

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

	// 1. 建立 Streaming Client (使用 channel 模式，Go 風格)
	fmt.Println("1. 建立 WebSocket Client...")
	client, err := mkt.NewStreamingClient(apiKey, 100) // buffer size = 100
	if err != nil {
		log.Fatalf("建立 client 失敗: %v", err)
	}
	// 注意: 不用 defer，改在 cleanup 處理

	// 2. 連線
	fmt.Println("2. 連線中...")
	if err := client.Connect(); err != nil {
		log.Fatalf("連線失敗: %v", err)
	}
	fmt.Println("   已連線!")

	// 3. 訂閱 2330 成交明細
	fmt.Println("3. 訂閱 2330 trades...")
	if err := client.Subscribe("trades", "2330"); err != nil {
		log.Fatalf("訂閱失敗: %v", err)
	}

	// 4. 設定中斷信號處理 (Ctrl+C)
	sigChan := make(chan os.Signal, 1)
	signal.Notify(sigChan, syscall.SIGINT, syscall.SIGTERM)

	// 5. 接收訊息 (Go 風格: 用 channel + range)
	fmt.Println("4. 等待訊息 (按 Ctrl+C 停止)...\n")

	messageCount := 0
	timeout := time.After(30 * time.Second) // 30 秒後自動結束

	for {
		select {
		case msg, ok := <-client.Messages():
			if !ok {
				fmt.Println("Channel 已關閉")
				return
			}
			messageCount++
			printMessage(msg)

		case err := <-client.Errors():
			fmt.Printf("錯誤: %v\n", err)

		case <-sigChan:
			fmt.Println("\n收到中斷信號，結束程式...")
			goto cleanup

		case <-timeout:
			fmt.Println("\n30 秒到，自動結束...")
			goto cleanup
		}
	}

cleanup:
	fmt.Printf("\n=== 總共收到 %d 則訊息 ===\n", messageCount)
	fmt.Println("斷線中...")

	// 用 goroutine + timeout 避免 Close() 卡住
	done := make(chan struct{})
	go func() {
		client.Close()
		close(done)
	}()

	select {
	case <-done:
		fmt.Println("完成!")
	case <-time.After(3 * time.Second):
		fmt.Println("Close timeout，強制結束")
		os.Exit(0)
	}
}

func printMessage(msg mkt.StreamMessage) {
	event := msg.Event
	channel := ""
	symbol := ""

	if msg.Channel != nil {
		channel = *msg.Channel
	}
	if msg.Symbol != nil {
		symbol = *msg.Symbol
	}

	switch event {
	case "subscribed":
		fmt.Printf("  [訂閱成功] channel=%s, symbol=%s\n", channel, symbol)

	case "snapshot", "data":
		// 解析 DataJson 取得詳細資料
		prefix := "[快照]"
		if event == "data" {
			prefix = "[即時]"
		}
		fmt.Printf("  %s %s:%s", prefix, channel, symbol)

		if msg.DataJson != nil {
			var data map[string]interface{}
			if err := json.Unmarshal([]byte(*msg.DataJson), &data); err == nil {
				if price, ok := data["price"].(float64); ok {
					fmt.Printf(" 價格=%.2f", price)
				}
				if vol, ok := data["volume"].(float64); ok {
					fmt.Printf(" 量=%.0f", vol)
				}
			}
		}
		fmt.Println()

	case "heartbeat":
		fmt.Println("  [心跳]")

	default:
		fmt.Printf("  [%s] channel=%s symbol=%s\n", event, channel, symbol)
		if msg.DataJson != nil {
			fmt.Printf("    data: %s\n", *msg.DataJson)
		}
	}
}
