package marketdata_uniffi

import (
	"sync"
	"testing"
)

func isClosed(ch *MessageChannel) bool {
	select {
	case <-ch.done:
		return true
	default:
		return false
	}
}

func TestChannelListener_DisconnectWithReconnectKeepsChannelsOpen(t *testing.T) {
	ch := NewMessageChannel(4)
	l := &channelListener{ch: ch}

	l.OnDisconnected(true)
	if isClosed(ch) {
		t.Fatal("channels closed on a disconnect that will reconnect")
	}

	l.OnMessage(StreamMessage{Event: "data"})
	if msg := <-ch.Messages(); msg.Event != "data" {
		t.Fatalf("got %q, want data", msg.Event)
	}

	l.OnDisconnected(false)
	if !isClosed(ch) {
		t.Fatal("channels still open after the final disconnect")
	}
	if _, ok := <-ch.Messages(); ok {
		t.Fatal("Messages() not closed")
	}
}

func TestChannelListener_ReconnectFailedReportsAndCloses(t *testing.T) {
	ch := NewMessageChannel(4)
	l := &channelListener{ch: ch}

	l.OnDisconnected(true)
	l.OnReconnectFailed(3)

	if err, ok := <-ch.Errors(); !ok || err == nil {
		t.Fatal("reconnect failure not reported on Errors()")
	}
	if !isClosed(ch) {
		t.Fatal("channels still open after reconnect failure")
	}
	if _, ok := <-ch.Errors(); ok {
		t.Fatal("Errors() not closed")
	}
}

func TestChannelListener_UnauthenticatedReportsError(t *testing.T) {
	ch := NewMessageChannel(4)
	l := &channelListener{ch: ch}

	data := `{"message":"Invalid token"}`
	l.OnUnauthenticated(&data)
	l.OnUnauthenticated(nil)

	for i := 0; i < 2; i++ {
		if err := <-ch.Errors(); err == nil {
			t.Fatalf("error %d missing", i)
		}
	}
}

// Callbacks run on the SDK's threads, so Close can race a send; neither may
// panic, including when the buffer is full.
func TestMessageChannel_CloseWhileSending(t *testing.T) {
	for i := 0; i < 50; i++ {
		ch := NewMessageChannel(1)
		l := &channelListener{ch: ch}
		var wg sync.WaitGroup
		for j := 0; j < 8; j++ {
			wg.Add(1)
			go func() {
				defer wg.Done()
				for k := 0; k < 20; k++ {
					l.OnMessage(StreamMessage{Event: "data"})
					l.OnError("boom")
				}
			}()
		}
		l.OnDisconnected(false)
		wg.Wait()
		l.OnMessage(StreamMessage{Event: "data"})
	}
}
