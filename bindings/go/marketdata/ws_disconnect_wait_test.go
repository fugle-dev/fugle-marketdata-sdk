// ws_disconnect_wait_test.go - Disconnect returns once the listener has
// handled the connection's remaining events (#126).
//
// Run: CGO_ENABLED=1 go test -run DisconnectWait -short .

package marketdata_uniffi

import (
	"sync/atomic"
	"testing"
	"time"
)

// hookListener runs onAuthenticated and onDisconnected, if set, and ignores
// every other event.
type hookListener struct {
	onAuthenticated func()
	onDisconnected  func()
}

func (l *hookListener) OnConnected() {}
func (l *hookListener) OnAuthenticated(dataJson *string) {
	if l.onAuthenticated != nil {
		l.onAuthenticated()
	}
}
func (l *hookListener) OnUnauthenticated(dataJson *string) {}
func (l *hookListener) OnDisconnected(willReconnect bool) {
	if l.onDisconnected != nil {
		l.onDisconnected()
	}
}
func (l *hookListener) OnMessage(message StreamMessage)   {}
func (l *hookListener) OnError(info ErrorInfo)            {}
func (l *hookListener) OnReconnecting(attempt uint32)     {}
func (l *hookListener) OnReconnectFailed(attempts uint32) {}
func (l *hookListener) OnMessagesDropped(count uint64)    {}

func newListenerClient(t *testing.T, srv *authFrameServer, listener WebSocketListener) *WebSocketClient {
	t.Helper()
	apiKey, url := "the-key", srv.url()
	client, err := WebSocketClientNewWithCredentials(
		CredentialsRecord{ApiKey: &apiKey},
		listener, WebSocketEndpointStock, &url, nil, nil, nil, nil, nil,
	)
	if err != nil {
		t.Fatalf("WebSocketClientNewWithCredentials: %v", err)
	}
	return client
}

// within fails the test if f has not returned after 10 seconds.
func within(t *testing.T, what string, f func()) {
	t.Helper()
	done := make(chan struct{})
	go func() {
		defer close(done)
		f()
	}()
	select {
	case <-done:
	case <-time.After(10 * time.Second):
		t.Fatalf("%s did not return", what)
	}
}

func TestDisconnectWait_ReturnsAfterOnDisconnected(t *testing.T) {
	srv := newAuthFrameServer(t)
	var authenticated, disconnected atomic.Bool
	client := newListenerClient(t, srv, &hookListener{
		onAuthenticated: func() { authenticated.Store(true) },
		onDisconnected: func() {
			time.Sleep(300 * time.Millisecond)
			disconnected.Store(true)
		},
	})
	defer client.Destroy()
	if err := client.Connect(); err != nil {
		t.Fatalf("Connect: %v", err)
	}
	waitUntil(t, authenticated.Load, "never authenticated")

	client.Disconnect()
	if !disconnected.Load() {
		t.Fatal("OnDisconnected had not run when Disconnect returned")
	}
}

func TestDisconnectWait_FromListenerDoesNotWaitForItself(t *testing.T) {
	srv := newAuthFrameServer(t)
	var client *WebSocketClient
	var returned, disconnected atomic.Bool
	client = newListenerClient(t, srv, &hookListener{
		onAuthenticated: func() {
			client.Disconnect()
			returned.Store(true)
		},
		onDisconnected: func() { disconnected.Store(true) },
	})
	defer client.Destroy()
	// The listener disconnects during the handshake, so Connect either finds
	// the connection stored and closes it, or gives it up with code 2010
	// (#121). It ends either way.
	if err := client.Connect(); err != nil {
		if info, ok := ErrorInfoOf(err); !ok || info.Code != 2010 {
			t.Fatalf("Connect: got %v (info %+v), want code 2010", err, info)
		}
	}
	waitUntil(t, returned.Load, "Disconnect in OnAuthenticated never returned")
	waitUntil(t, disconnected.Load, "OnDisconnected never ran")
	if client.IsConnected() {
		t.Fatal("IsConnected() = true after the listener disconnected")
	}
	within(t, "Disconnect", client.Disconnect)
}

func TestDisconnectWait_CloseWithUnreadMessagesReturns(t *testing.T) {
	srv := newAuthFrameServer(t)
	ch := NewMessageChannel(1)
	// Full, so delivering the `authenticated` message waits for a reader.
	ch.messages <- StreamMessage{Event: "unread"}
	listener := &channelListener{ch: ch}
	sc := &StreamingClient{client: newListenerClient(t, srv, listener), channel: ch, listener: listener}
	if err := sc.client.Connect(); err != nil {
		t.Fatalf("Connect: %v", err)
	}
	time.Sleep(200 * time.Millisecond)
	within(t, "Close", func() { _ = sc.Close() })
}
