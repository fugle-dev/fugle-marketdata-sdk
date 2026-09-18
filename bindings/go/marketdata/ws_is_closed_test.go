// ws_is_closed_test.go - IsClosed reads core's connection state (#95).
//
// It used to read a flag only Disconnect set, so it stayed false after the
// server closed the connection with no reconnect to follow.
//
// Run: CGO_ENABLED=1 go test -run IsClosed -short .

package marketdata_uniffi

import (
	"testing"
	"time"
)

func TestWebSocketIsClosed_AfterServerCloseWithoutReconnect(t *testing.T) {
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
	if client.IsClosed() {
		t.Fatal("IsClosed() = true while connected")
	}

	srv.dropConnections()
	waitUntil(t, client.IsClosed, "IsClosed() never became true")
	if client.IsConnected() {
		t.Fatal("IsConnected() = true after the server closed the connection")
	}
}

func TestWebSocketIsClosed_FalseWhileReconnecting_TrueAfterDisconnect(t *testing.T) {
	srv := newAuthFrameServer(t)
	client, err := NewFugleWebSocketClient(nil,
		WithApiKey("the-key"),
		WithBaseUrl(srv.url()),
		WithReconnect(ReconnectConfig{MaxAttempts: 3, InitialDelayMs: 2000, MaxDelayMs: 2000}),
	)
	if err != nil {
		t.Fatalf("NewFugleWebSocketClient: %v", err)
	}
	defer client.Close()
	if err := client.Connect(); err != nil {
		t.Fatalf("Connect: %v", err)
	}

	srv.dropConnections()
	waitUntil(t, func() bool { return !client.IsConnected() }, "IsConnected() never became false")
	if client.IsClosed() {
		t.Fatal("IsClosed() = true while reconnecting")
	}

	// Close destroys the client, so disconnect through it to read IsClosed after.
	client.client.Disconnect()
	if !client.IsClosed() {
		t.Fatal("IsClosed() = false after Disconnect")
	}
}

func waitUntil(t *testing.T, condition func() bool, message string) {
	t.Helper()
	deadline := time.Now().Add(5 * time.Second)
	for !condition() {
		if time.Now().After(deadline) {
			t.Fatal(message)
		}
		time.Sleep(10 * time.Millisecond)
	}
}
