// ws_url_test.go - URL() is the endpoint core resolved from the base URL,
// the version and the product, readable before Connect() (#245).
//
// Run: CGO_ENABLED=1 go test -run WebSocketURL -short .

package marketdata_uniffi

import "testing"

func TestWebSocketURL_DefaultsToTheProductionEndpointOfEachProduct(t *testing.T) {
	for endpoint, want := range map[WebSocketEndpoint]string{
		WebSocketEndpointStock:  "wss://api.fugle.tw/marketdata/v1.0/stock/streaming",
		WebSocketEndpointFutOpt: "wss://api.fugle.tw/marketdata/v1.1/futopt/streaming",
	} {
		client, err := NewFugleWebSocketClient(nil, WithApiKey("the-key"), WithEndpoint(endpoint))
		if err != nil {
			t.Fatalf("NewFugleWebSocketClient: %v", err)
		}
		got, err := client.URL()
		client.Close()
		if err != nil || got != want {
			t.Errorf("URL() = %q, %v; want %q", got, err, want)
		}
	}
}

func TestWebSocketURL_ReflectsBaseUrl(t *testing.T) {
	client, err := NewFugleWebSocketClient(nil,
		WithApiKey("the-key"),
		WithEndpoint(WebSocketEndpointFutOpt),
		WithBaseUrl("wss://staging.fugle.tw/marketdata"),
	)
	if err != nil {
		t.Fatalf("NewFugleWebSocketClient: %v", err)
	}
	defer client.Close()

	want := "wss://staging.fugle.tw/marketdata/v1.1/futopt/streaming"
	if got, err := client.URL(); err != nil || got != want {
		t.Errorf("URL() = %q, %v; want %q", got, err, want)
	}
}

func TestWebSocketURL_VersionedBaseUrlIsConfigError(t *testing.T) {
	client, err := NewFugleWebSocketClient(nil,
		WithApiKey("the-key"),
		WithBaseUrl("wss://staging.fugle.tw/marketdata/v1.0"),
	)
	if err != nil {
		t.Fatalf("NewFugleWebSocketClient: %v", err)
	}
	defer client.Close()

	_, err = client.URL()
	info, ok := ErrorInfoOf(err)
	if !ok || info.Code != 1004 {
		t.Errorf("URL() error = %v (info %+v), want code 1004", err, info)
	}
}
