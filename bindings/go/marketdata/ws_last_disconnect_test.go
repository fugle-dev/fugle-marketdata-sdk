// ws_last_disconnect_test.go - LastDisconnect says who closed the connection
// and whether a reconnect follows (#293).
//
// Run: CGO_ENABLED=1 go test -run LastDisconnect -short .

package marketdata_uniffi

import (
	"reflect"
	"sync"
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

	client.Disconnect()

	code := uint16(1000)
	want := DisconnectInfo{Code: &code, Reason: "Normal closure", Intent: DisconnectIntentClient, WillReconnect: false}
	got := client.LastDisconnect()
	if got == nil || !reflect.DeepEqual(*got, want) {
		t.Fatalf("LastDisconnect() = %+v, want %+v", got, want)
	}
	// Closed, past what was buffered before Disconnect.
	within(t, "Messages() closing", func() {
		for range client.Messages() {
		}
	})
	within(t, "Errors() closing", func() {
		for range client.Errors() {
		}
	})
	if _, ok := <-client.Messages(); ok {
		t.Fatal("Messages() still open after Disconnect")
	}
	if _, ok := <-client.Errors(); ok {
		t.Fatal("Errors() still open after Disconnect")
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

func TestWebSocketLastDisconnect_AfterClose_KeepsRecord(t *testing.T) {
	srv := newAuthFrameServer(t)
	client, err := NewFugleWebSocketClient(nil,
		WithApiKey("the-key"),
		WithBaseUrl(srv.url()),
		WithoutReconnect(),
	)
	if err != nil {
		t.Fatalf("NewFugleWebSocketClient: %v", err)
	}
	if err := client.Connect(); err != nil {
		t.Fatalf("Connect: %v", err)
	}
	if err := client.Close(); err != nil {
		t.Fatalf("Close: %v", err)
	}

	code := uint16(1000)
	want := DisconnectInfo{Code: &code, Reason: "Normal closure", Intent: DisconnectIntentClient, WillReconnect: false}
	got := client.LastDisconnect()
	if got == nil || !reflect.DeepEqual(*got, want) {
		t.Fatalf("LastDisconnect() after Close = %+v, want %+v", got, want)
	}

	// A second Close is a no-op, not a panic on the destroyed client.
	if err := client.Close(); err != nil {
		t.Fatalf("second Close: %v", err)
	}
	if got := client.LastDisconnect(); got == nil || !reflect.DeepEqual(*got, want) {
		t.Fatalf("LastDisconnect() after second Close = %+v, want %+v", got, want)
	}
}

func TestWebSocketLastDisconnect_CloseWithoutConnect_IsNil(t *testing.T) {
	client, err := NewFugleWebSocketClient(nil,
		WithApiKey("the-key"),
		WithBaseUrl("ws://127.0.0.1:1"),
		WithoutReconnect(),
	)
	if err != nil {
		t.Fatalf("NewFugleWebSocketClient: %v", err)
	}
	if err := client.Close(); err != nil {
		t.Fatalf("Close: %v", err)
	}
	if got := client.LastDisconnect(); got != nil {
		t.Fatalf("LastDisconnect() after Close without Connect = %+v, want nil", *got)
	}
}

func TestWebSocketLastDisconnect_ConcurrentWithClose(t *testing.T) {
	srv := newAuthFrameServer(t)
	client, err := NewFugleWebSocketClient(nil,
		WithApiKey("the-key"),
		WithBaseUrl(srv.url()),
		WithoutReconnect(),
	)
	if err != nil {
		t.Fatalf("NewFugleWebSocketClient: %v", err)
	}
	if err := client.Connect(); err != nil {
		t.Fatalf("Connect: %v", err)
	}

	start := make(chan struct{})
	var readers, closers sync.WaitGroup
	for i := 0; i < 8; i++ {
		readers.Add(1)
		go func() {
			defer readers.Done()
			<-start
			for j := 0; j < 200; j++ {
				client.LastDisconnect()
			}
		}()
	}
	for i := 0; i < 2; i++ {
		closers.Add(1)
		go func() {
			defer closers.Done()
			<-start
			if err := client.Close(); err != nil {
				t.Errorf("Close: %v", err)
			}
		}()
	}
	close(start)
	closers.Wait()
	readers.Wait()

	code := uint16(1000)
	want := DisconnectInfo{Code: &code, Reason: "Normal closure", Intent: DisconnectIntentClient, WillReconnect: false}
	if got := client.LastDisconnect(); got == nil || !reflect.DeepEqual(*got, want) {
		t.Fatalf("LastDisconnect() after concurrent Close = %+v, want %+v", got, want)
	}
}

func connectRefusedAsClientClosed(t *testing.T, client *StreamingClient, after string) {
	t.Helper()
	err := client.Connect()
	info, ok := ErrorInfoOf(err)
	if !ok || info.Code != 2010 {
		t.Fatalf("Connect after %s: got %v (info %+v), want code 2010", after, err, info)
	}
	if client.IsConnected() {
		t.Fatalf("IsConnected() = true after the refused Connect (after %s)", after)
	}
}

func TestWebSocketConnect_AfterDisconnect_IsClientClosed(t *testing.T) {
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
	client.Disconnect()

	connectRefusedAsClientClosed(t, client, "Disconnect")
	if n := len(srv.authData()); n != 1 {
		t.Fatalf("server saw %d auth frames, want 1", n)
	}
}

func TestWebSocketConnect_AfterServerClose_IsClientClosed(t *testing.T) {
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
	within(t, "Messages() closing", func() {
		for range client.Messages() {
		}
	})

	connectRefusedAsClientClosed(t, client, "a server close")
	if n := len(srv.authData()); n != 1 {
		t.Fatalf("server saw %d auth frames, want 1", n)
	}
}
