// marketdata_test.go - Tests for UniFFI Go bindings
//
// These tests verify the Go bindings work correctly:
// - Structural tests verify types exist and can be used
// - Integration tests (require FUGLE_API_KEY) verify live API calls
//
// Run all tests: CGO_ENABLED=1 go test -v .
// Run structural tests only: CGO_ENABLED=1 go test -v -short .
// Run integration tests: FUGLE_API_KEY=xxx CGO_ENABLED=1 go test -v .

package marketdata_uniffi

import (
	"os"
	"testing"
)

// ========== Structural Tests (Type Existence) ==========

func TestRestClient_Creation(t *testing.T) {
	client, err := NewRestClientWithApiKey("test-api-key")
	if err != nil {
		t.Fatalf("Failed to create client: %v", err)
	}
	if client == nil {
		t.Fatal("Client is nil")
	}
	defer client.Destroy()
}

func TestRestClient_WithSdkToken(t *testing.T) {
	client, err := NewRestClientWithSdkToken("test-sdk-token")
	if err != nil {
		t.Fatalf("Failed to create client with SDK token: %v", err)
	}
	if client == nil {
		t.Fatal("Client is nil")
	}
	defer client.Destroy()
}

func TestRestClient_WithBearerToken(t *testing.T) {
	client, err := NewRestClientWithBearerToken("test-bearer-token")
	if err != nil {
		t.Fatalf("Failed to create client with bearer token: %v", err)
	}
	if client == nil {
		t.Fatal("Client is nil")
	}
	defer client.Destroy()
}

func TestRestClient_Stock(t *testing.T) {
	client, err := NewRestClientWithApiKey("test-api-key")
	if err != nil {
		t.Fatalf("Failed to create client: %v", err)
	}
	defer client.Destroy()

	stock := client.Stock()
	if stock == nil {
		t.Fatal("StockClient is nil")
	}
}

func TestRestClient_Futopt(t *testing.T) {
	client, err := NewRestClientWithApiKey("test-api-key")
	if err != nil {
		t.Fatalf("Failed to create client: %v", err)
	}
	defer client.Destroy()

	futopt := client.Futopt()
	if futopt == nil {
		t.Fatal("FutOptClient is nil")
	}
}

func TestStockClient_Intraday(t *testing.T) {
	client, err := NewRestClientWithApiKey("test-api-key")
	if err != nil {
		t.Fatalf("Failed to create client: %v", err)
	}
	defer client.Destroy()

	intraday := client.Stock().Intraday()
	if intraday == nil {
		t.Fatal("StockIntradayClient is nil")
	}
}

// ========== Channel Wrapper Tests ==========

func TestNewStreamingClient(t *testing.T) {
	client, err := NewStreamingClient("test-api-key", 100)
	if err != nil {
		t.Fatalf("Failed to create streaming client: %v", err)
	}
	if client == nil {
		t.Fatal("StreamingClient is nil")
	}
	defer client.Close()
}

func TestNewStreamingClientWithEndpoint(t *testing.T) {
	client, err := NewStreamingClientWithEndpoint("test-api-key", WebSocketEndpointStock, 100)
	if err != nil {
		t.Fatalf("Failed to create streaming client with endpoint: %v", err)
	}
	if client == nil {
		t.Fatal("StreamingClient is nil")
	}
	defer client.Close()

	// Verify channels are accessible
	_ = client.Messages()
	_ = client.Errors()
}

func TestMessageChannel_Creation(t *testing.T) {
	ch := NewMessageChannel(50)
	if ch == nil {
		t.Fatal("MessageChannel is nil")
	}
	defer ch.Close()

	// Verify channels are not nil
	if ch.Messages() == nil {
		t.Fatal("Messages channel is nil")
	}
	if ch.Errors() == nil {
		t.Fatal("Errors channel is nil")
	}
}

func TestMessageChannel_DefaultBufferSize(t *testing.T) {
	// Zero buffer should use default (100)
	ch := NewMessageChannel(0)
	if ch == nil {
		t.Fatal("MessageChannel is nil with zero buffer")
	}
	defer ch.Close()

	// Negative buffer should use default (100)
	ch2 := NewMessageChannel(-10)
	if ch2 == nil {
		t.Fatal("MessageChannel is nil with negative buffer")
	}
	defer ch2.Close()
}

// ========== Integration Tests (require FUGLE_API_KEY) ==========

func TestRestClient_GetQuote_Integration(t *testing.T) {
	if testing.Short() {
		t.Skip("Skipping integration test in short mode")
	}

	apiKey := os.Getenv("FUGLE_API_KEY")
	if apiKey == "" {
		t.Skip("FUGLE_API_KEY not set")
	}

	client, err := NewRestClientWithApiKey(apiKey)
	if err != nil {
		t.Fatalf("Failed to create client: %v", err)
	}
	defer client.Destroy()

	quote, err := client.Stock().Intraday().GetQuote("2330")
	if err != nil {
		t.Fatalf("Failed to get quote: %v", err)
	}

	if quote.Symbol != "2330" {
		t.Errorf("Expected symbol 2330, got %s", quote.Symbol)
	}
}

func TestRestClient_GetTicker_Integration(t *testing.T) {
	if testing.Short() {
		t.Skip("Skipping integration test in short mode")
	}

	apiKey := os.Getenv("FUGLE_API_KEY")
	if apiKey == "" {
		t.Skip("FUGLE_API_KEY not set")
	}

	client, err := NewRestClientWithApiKey(apiKey)
	if err != nil {
		t.Fatalf("Failed to create client: %v", err)
	}
	defer client.Destroy()

	ticker, err := client.Stock().Intraday().GetTicker("2330")
	if err != nil {
		t.Fatalf("Failed to get ticker: %v", err)
	}

	if ticker.Symbol != "2330" {
		t.Errorf("Expected symbol 2330, got %s", ticker.Symbol)
	}
}

func TestRestClient_GetTrades_Integration(t *testing.T) {
	if testing.Short() {
		t.Skip("Skipping integration test in short mode")
	}

	apiKey := os.Getenv("FUGLE_API_KEY")
	if apiKey == "" {
		t.Skip("FUGLE_API_KEY not set")
	}

	client, err := NewRestClientWithApiKey(apiKey)
	if err != nil {
		t.Fatalf("Failed to create client: %v", err)
	}
	defer client.Destroy()

	trades, err := client.Stock().Intraday().GetTrades("2330")
	if err != nil {
		t.Fatalf("Failed to get trades: %v", err)
	}

	if trades.Symbol != "2330" {
		t.Errorf("Expected symbol 2330, got %s", trades.Symbol)
	}
}

func TestRestClient_FutOptProducts_Integration(t *testing.T) {
	if testing.Short() {
		t.Skip("Skipping integration test in short mode")
	}

	apiKey := os.Getenv("FUGLE_API_KEY")
	if apiKey == "" {
		t.Skip("FUGLE_API_KEY not set")
	}

	client, err := NewRestClientWithApiKey(apiKey)
	if err != nil {
		t.Fatalf("Failed to create client: %v", err)
	}
	defer client.Destroy()

	products, err := client.Futopt().Intraday().GetProducts("F")
	if err != nil {
		t.Fatalf("Failed to get products: %v", err)
	}

	// Should return at least one product
	if len(products.Data) == 0 {
		t.Error("Expected at least one futures product")
	}
}
