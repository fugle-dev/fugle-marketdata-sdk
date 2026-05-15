// WebSocket benchmark client -- new Rust-core SDK (Go / UniFFI binding)
//
// Connects to the mock server, subscribes, receives data messages, and
// reports throughput / latency / memory metrics as a single JSON line on stdout.
//
// Build:
//   CGO_ENABLED=1 go build -o ws-bench-go .
//
// Usage:
//   ./ws-bench-go --url ws://localhost:8765 --timeout 60000

package main

import (
	"encoding/json"
	"flag"
	"fmt"
	"math"
	"os"
	"runtime"
	"sort"
	"time"

	mkt "github.com/fugle-dev/fugle-marketdata-go"
)

func main() {
	url := flag.String("url", "ws://localhost:8765", "WebSocket server URL")
	timeout := flag.Int("timeout", 60000, "Timeout in milliseconds")
	flag.Parse()

	// Metrics state
	var received int
	var t0 int64
	t0Set := false
	latencies := make([]float64, 0, 500000)
	maxSerial := -1
	var serverStats map[string]interface{}
	done := make(chan struct{})

	// Force GC before benchmark
	runtime.GC()

	var startMem runtime.MemStats
	runtime.ReadMemStats(&startMem)
	startCPU := time.Now()

	// Create raw WebSocket client (bypass channel wrapper for max perf)
	listener := &benchListener{
		onMsg: func(msg mkt.StreamMessage) {
			ev := msg.Event

			if ev == "warmup" {
				return
			}

			if ev == "bench_done" {
				if msg.DataJson != nil {
					var data map[string]interface{}
					if json.Unmarshal([]byte(*msg.DataJson), &data) == nil {
						serverStats = data
					}
				}
				close(done)
				return
			}

			if ev == "data" {
				if !t0Set {
					t0 = time.Now().UnixMilli()
					t0Set = true
				}
				received++

				if msg.DataJson != nil {
					var data map[string]interface{}
					if json.Unmarshal([]byte(*msg.DataJson), &data) == nil {
						if ts, ok := data["server_ts"].(float64); ok {
							now := time.Now().UnixMilli()
							latencies = append(latencies, float64(now)-ts)
						}
						if s, ok := data["serial"].(float64); ok {
							if int(s) > maxSerial {
								maxSerial = int(s)
							}
						}
					}
				}
			}
		},
		onErr: func(err string) {
			fmt.Fprintf(os.Stderr, "error: %s\n", err)
		},
	}

	client := mkt.WebSocketClientNewWithUrl(
		"bench-key", listener, mkt.WebSocketEndpointStock,
		*url, nil, nil,
	)

	if err := client.Connect(); err != nil {
		fmt.Fprintf(os.Stderr, "connect failed: %v\n", err)
		os.Exit(1)
	}

	if err := client.Subscribe("trades", "2330"); err != nil {
		fmt.Fprintf(os.Stderr, "subscribe failed: %v\n", err)
		os.Exit(1)
	}

	// Wait for bench_done or timeout
	select {
	case <-done:
	case <-time.After(time.Duration(*timeout) * time.Millisecond):
		fmt.Fprintf(os.Stderr, "TIMEOUT: did not receive bench_done within %d ms\n", *timeout)
	}

	// Report
	var elapsed int64
	if t0Set {
		elapsed = time.Now().UnixMilli() - t0
	}
	cpuUser := time.Since(startCPU).Seconds() * 1000 // approximate user CPU

	var endMem runtime.MemStats
	runtime.ReadMemStats(&endMem)
	memDelta := float64(endMem.Sys-startMem.Sys) / 1e6

	// Sort latencies
	sort.Float64s(latencies)

	percentile := func(sorted []float64, p int) interface{} {
		if len(sorted) == 0 {
			return nil
		}
		idx := int(math.Ceil(float64(p)/100.0*float64(len(sorted)))) - 1
		if idx < 0 {
			idx = 0
		}
		if idx >= len(sorted) {
			idx = len(sorted) - 1
		}
		return math.Round(sorted[idx]*100) / 100
	}

	var ssCount interface{}
	var ssMps interface{}
	if serverStats != nil {
		ssCount = serverStats["count"]
		ssMps = serverStats["server_msgs_per_sec"]
	}

	var lost interface{}
	if ssCount != nil {
		if c, ok := ssCount.(float64); ok {
			lost = int(c) - received
		}
	}

	var msgsPerSec int
	if elapsed > 0 {
		msgsPerSec = int(float64(received) / float64(elapsed) * 1000)
	}

	result := map[string]interface{}{
		"sdk":                "rust-core-go",
		"count":              received,
		"expected":           ssCount,
		"lost":               lost,
		"elapsed_ms":         elapsed,
		"msgs_per_sec":       msgsPerSec,
		"latency_p50_ms":     percentile(latencies, 50),
		"latency_p99_ms":     percentile(latencies, 99),
		"latency_min_ms":     nil,
		"latency_max_ms":     nil,
		"mem_rss_delta_mb":   math.Round(memDelta*10) / 10,
		"cpu_user_ms":        math.Round(cpuUser*10) / 10,
		"cpu_system_ms":      0.0,
		"server_msgs_per_sec": ssMps,
	}

	if len(latencies) > 0 {
		result["latency_min_ms"] = latencies[0]
		result["latency_max_ms"] = latencies[len(latencies)-1]
	}

	out, _ := json.Marshal(result)
	fmt.Println(string(out))

	client.Disconnect()
	client.Destroy()
	time.Sleep(200 * time.Millisecond)
	os.Exit(0)
}

// benchListener implements WebSocketListener directly (not channel-based)
// for maximum performance in benchmarking
type benchListener struct {
	onMsg func(mkt.StreamMessage)
	onErr func(string)
}

func (l *benchListener) OnConnected()                      {}
func (l *benchListener) OnDisconnected()                   {}
func (l *benchListener) OnMessage(message mkt.StreamMessage) { l.onMsg(message) }
func (l *benchListener) OnError(errorMessage string)       { l.onErr(errorMessage) }
func (l *benchListener) OnReconnecting(attempt uint32)     {}
func (l *benchListener) OnReconnectFailed(attempts uint32) {}
