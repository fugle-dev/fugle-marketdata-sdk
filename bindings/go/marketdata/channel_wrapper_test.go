package marketdata_uniffi

import (
	"errors"
	"sync"
	"testing"
	"time"
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

func TestChannelListener_MessagesDroppedReportsErrorWithoutClosing(t *testing.T) {
	ch := NewMessageChannel(4)
	l := &channelListener{ch: ch}

	l.OnMessagesDropped(3)

	err := <-ch.Errors()
	if err == nil {
		t.Fatal("dropped messages not reported on Errors()")
	}
	if isClosed(ch) {
		t.Fatal("channels closed after a non-terminal messages-dropped report")
	}

	// The connection stays usable: further messages still flow.
	l.OnMessage(StreamMessage{Event: "data"})
	if msg := <-ch.Messages(); msg.Event != "data" {
		t.Fatalf("got %q, want data", msg.Event)
	}
}

func TestChannelListener_MessagesDroppedNeverHoldsUpMessages(t *testing.T) {
	// A caller that reads only Messages(): drop reports pile up on Errors()
	// but must not stop delivery.
	const n = 100
	ch := NewMessageChannel(4)
	l := &channelListener{ch: ch}

	go func() {
		for i := 0; i < n; i++ {
			l.OnMessagesDropped(1)
			l.OnMessage(StreamMessage{Event: "data"})
		}
	}()

	timeout := time.After(5 * time.Second)
	for i := 0; i < n; i++ {
		select {
		case <-ch.Messages():
		case <-timeout:
			t.Fatalf("Messages() stalled after %d of %d while Errors() went unread", i, n)
		}
	}
	if got := len(ch.Errors()); got != cap(ch.Errors()) {
		t.Fatalf("Errors() holds %d reports, want it filled to %d", got, cap(ch.Errors()))
	}
}

func TestChannelListener_OnErrorDeliversStreamError(t *testing.T) {
	ch := NewMessageChannel(4)
	l := &channelListener{ch: ch}

	l.OnError(ErrorInfo{Code: 3002, Message: "boom"})

	err := <-ch.Errors()
	var se *StreamError
	if !errors.As(err, &se) {
		t.Fatalf("got %T, want *StreamError", err)
	}
	if se.Error() != "boom" {
		t.Fatalf("Error() = %q, want %q", se.Error(), "boom")
	}

	info, ok := ErrorInfoOf(err)
	if !ok {
		t.Fatal("ErrorInfoOf did not recognise the delivered error")
	}
	if info.Code != 3002 || info.Message != "boom" {
		t.Fatalf("got %+v, want code=3002 message=boom", info)
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
					l.OnError(ErrorInfo{Message: "boom"})
				}
			}()
		}
		l.OnDisconnected(false)
		wg.Wait()
		l.OnMessage(StreamMessage{Event: "data"})
	}
}
