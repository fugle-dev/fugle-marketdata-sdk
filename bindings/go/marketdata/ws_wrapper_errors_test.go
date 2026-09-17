// ws_wrapper_errors_test.go - StreamingClient errors keep the SDK error (#119).
//
// Connect, Ping and QuerySubscriptions flattened it with %v, so ErrorInfoOf
// found no ErrorInfo. Connect is covered by ws_already_connected_test.go.
//
// Run: CGO_ENABLED=1 go test -run WrapperErrors -short .

package marketdata_uniffi

import "testing"

func TestStreamingClient_WrapperErrorsCarryErrorInfo(t *testing.T) {
	client, err := NewFugleWebSocketClient(nil, WithApiKey("the-key"))
	if err != nil {
		t.Fatalf("NewFugleWebSocketClient: %v", err)
	}
	defer client.Close()

	for name, call := range map[string]func() error{
		"Ping":               func() error { return client.Ping(nil) },
		"QuerySubscriptions": client.QuerySubscriptions,
	} {
		err := call()
		info, ok := ErrorInfoOf(err)
		if !ok || info.Code != 2001 {
			t.Errorf("%s before Connect: got %v (info %+v), want code 2001", name, err, info)
		}
	}
}
