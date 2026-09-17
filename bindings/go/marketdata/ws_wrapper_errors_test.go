// ws_wrapper_errors_test.go - StreamingClient errors keep the SDK error (#119).
//
// They used to be flattened with %v, so ErrorInfoOf found no ErrorInfo.
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

	err = client.Subscribe("trade", "2330")
	info, ok := ErrorInfoOf(err)
	if !ok || info.Code != 1005 {
		t.Fatalf("Subscribe(unknown channel): got %v (info %+v), want code 1005", err, info)
	}
}
