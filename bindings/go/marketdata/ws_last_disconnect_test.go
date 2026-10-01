// ws_last_disconnect_test.go - LastDisconnect says who closed the connection
// and whether a reconnect follows (#293).
//
// Run: CGO_ENABLED=1 go test -run LastDisconnect -short .

package marketdata_uniffi

import (
	"reflect"
	"testing"
)

func TestWebSocketLastDisconnect_AfterDisconnect_IsClient(t *testing.T) {
	srv := newAuthFrameServer(t)
	client, err := NewFugleWebSocketClient(nil,
		WithApiKey("the-key"),
		WithBaseUrl(srv.url()),
		WithoutReconnect(),
	)
	if err != nil {
		t.Fatalf("NewFugleWebSocketClient: %v", err)
	}
	defer client.Close()
	if got := client.LastDisconnect(); got != nil {
		t.Fatalf("LastDisconnect() = %+v before Connect, want nil", *got)
	}
	if err := client.Connect(); err != nil {
		t.Fatalf("Connect: %v", err)
	}

	// Close destroys the client, so disconnect through it to read after.
	client.client.Disconnect()

	code := uint16(1000)
	want := DisconnectInfo{Code: &code, Reason: "Normal closure", Intent: DisconnectIntentClient, WillReconnect: false}
	got := client.LastDisconnect()
	if got == nil || !reflect.DeepEqual(*got, want) {
		t.Fatalf("LastDisconnect() = %+v, want %+v", got, want)
	}
}

func TestWebSocketLastDisconnect_AfterServerClose_IsServer(t *testing.T) {
	srv := newAuthFrameServer(t)
	client, err := NewFugleWebSocketClient(nil,
		WithApiKey("the-key"),
		WithBaseUrl(srv.url()),
		WithoutReconnect(),
	)
	if err != nil {
		t.Fatalf("NewFugleWebSocketClient: %v", err)
	}
	defer client.Close()
	if err := client.Connect(); err != nil {
		t.Fatalf("Connect: %v", err)
	}

	srv.closeConnections(1001, "going away")
	waitUntil(t, func() bool { return client.LastDisconnect() != nil }, "LastDisconnect() never set")

	code := uint16(1001)
	want := DisconnectInfo{Code: &code, Reason: "going away", Intent: DisconnectIntentServer, WillReconnect: false}
	if got := client.LastDisconnect(); !reflect.DeepEqual(*got, want) {
		t.Fatalf("LastDisconnect() = %+v, want %+v", *got, want)
	}
}
