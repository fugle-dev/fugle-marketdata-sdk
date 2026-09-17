// ws_already_connected_test.go - Connect while connected is refused (#119).
//
// It used to open a second connection and replace the first one's event
// forwarding.
//
// Run: CGO_ENABLED=1 go test -run AlreadyConnected -short .

package marketdata_uniffi

import "testing"

func TestWebSocketConnect_AlreadyConnectedIsRefused(t *testing.T) {
	srv := newAuthFrameServer(t)
	client, err := NewFugleWebSocketClient(nil, WithApiKey("the-key"), WithBaseUrl(srv.url()))
	if err != nil {
		t.Fatalf("NewFugleWebSocketClient: %v", err)
	}
	defer client.Close()
	if err := client.Connect(); err != nil {
		t.Fatalf("Connect: %v", err)
	}

	err = client.Connect()
	info, ok := ErrorInfoOf(err)
	if !ok || info.Code != 2011 {
		t.Fatalf("second Connect: got %v (info %+v), want code 2011", err, info)
	}
	if !client.IsConnected() {
		t.Fatal("IsConnected() = false after the refused Connect")
	}
	if n := len(srv.authData()); n != 1 {
		t.Fatalf("server saw %d auth frames, want 1", n)
	}

	client.client.Disconnect()
	if err := client.Connect(); err != nil {
		t.Fatalf("Connect after Disconnect: %v", err)
	}
	if !client.IsConnected() {
		t.Fatal("IsConnected() = false after reconnecting")
	}
}
