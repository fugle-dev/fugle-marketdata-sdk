// channel_wrapper.go - Idiomatic Go channel wrapper for WebSocket streaming
//
// This wrapper provides a channel-based API for consuming WebSocket messages,
// which is more idiomatic in Go than callback-based patterns.
//
// Usage:
//
//	client, err := marketdata_uniffi.NewStreamingClient(apiKey, 100)
//	if err != nil {
//	    log.Fatal(err)
//	}
//	defer client.Close()
//
//	if err := client.Connect(); err != nil {
//	    log.Fatal(err)
//	}
//
//	if err := client.Subscribe("trades", "2330"); err != nil {
//	    log.Fatal(err)
//	}
//
//	for {
//	    select {
//	    case msg, ok := <-client.Messages():
//	        if !ok {
//	            return
//	        }
//	        fmt.Printf("Got: %s %s\n", msg.Event, *msg.Symbol)
//	    case err := <-client.Errors():
//	        log.Println(err)
//	    }
//	}
//
// Read Errors() alongside Messages(): an unread error holds up delivery of
// the messages behind it once Errors() is full. Only messages-dropped
// reports are skipped instead of waiting (see OnMessagesDropped).
package marketdata_uniffi

import (
	"fmt"
	"sync"
)

// MessageChannel provides Go channel-based access to WebSocket messages
type MessageChannel struct {
	messages chan StreamMessage
	errors   chan error
	done     chan struct{}
	once     sync.Once
	// mu keeps Close from closing messages/errors while a callback is
	// sending on them: senders hold the read lock, Close takes the write
	// lock after closing done, which releases any sender blocked on a full
	// buffer.
	mu sync.RWMutex
}

// NewMessageChannel creates a channel-based message receiver
func NewMessageChannel(bufferSize int) *MessageChannel {
	if bufferSize <= 0 {
		bufferSize = 100
	}
	return &MessageChannel{
		messages: make(chan StreamMessage, bufferSize),
		errors:   make(chan error, 10),
		done:     make(chan struct{}),
	}
}

// Messages returns the channel for receiving messages
func (mc *MessageChannel) Messages() <-chan StreamMessage {
	return mc.messages
}

// Errors returns the channel for receiving errors
func (mc *MessageChannel) Errors() <-chan error {
	return mc.errors
}

// Close closes all channels
func (mc *MessageChannel) Close() {
	mc.once.Do(func() {
		close(mc.done)
		mc.mu.Lock()
		defer mc.mu.Unlock()
		close(mc.messages)
		close(mc.errors)
	})
}

// sendMessage delivers message unless the channel has been closed.
func (mc *MessageChannel) sendMessage(message StreamMessage) {
	mc.mu.RLock()
	defer mc.mu.RUnlock()
	select {
	case <-mc.done:
		return
	default:
	}
	select {
	case mc.messages <- message:
	case <-mc.done:
		// Channel closed, drop message
	}
}

// trySendError delivers err if Errors() has room, and otherwise skips it.
func (mc *MessageChannel) trySendError(err error) {
	mc.mu.RLock()
	defer mc.mu.RUnlock()
	select {
	case <-mc.done:
		return
	default:
	}
	select {
	case mc.errors <- err:
	default:
		// Errors() is full: skip rather than hold up Messages()
	}
}

// sendError delivers err unless the channel has been closed.
func (mc *MessageChannel) sendError(err error) {
	mc.mu.RLock()
	defer mc.mu.RUnlock()
	select {
	case <-mc.done:
		return
	default:
	}
	select {
	case mc.errors <- err:
	case <-mc.done:
		// Channel closed, drop error
	}
}

// channelListener implements WebSocketListener, forwarding to channels
type channelListener struct {
	ch *MessageChannel
}

// Ensure channelListener implements WebSocketListener
var _ WebSocketListener = (*channelListener)(nil)

// OnConnected implements WebSocketListener
func (l *channelListener) OnConnected() {
	// Transport established - authentication follows
}

// OnAuthenticated implements WebSocketListener
func (l *channelListener) OnAuthenticated(dataJson *string) {
	// Connection usable - could send event on separate channel if needed
}

// OnUnauthenticated implements WebSocketListener
func (l *channelListener) OnUnauthenticated(dataJson *string) {
	data := ""
	if dataJson != nil {
		data = *dataJson
	}
	l.ch.sendError(fmt.Errorf("unauthenticated: %s", data))
}

// OnDisconnected implements WebSocketListener
//
// The channels stay open while the client reconnects, so a range over
// Messages() keeps receiving once the connection is restored.
func (l *channelListener) OnDisconnected(willReconnect bool) {
	if !willReconnect {
		l.ch.Close()
	}
}

// OnMessage implements WebSocketListener
func (l *channelListener) OnMessage(message StreamMessage) {
	l.ch.sendMessage(message)
}

// OnError implements WebSocketListener
//
// Delivers a *StreamError carrying the unified ErrorInfo, instead of a
// formatted string, so callers reading Errors() can branch on
// ErrorInfoOf(err) (code, source kind, HTTP status) without parsing text.
func (l *channelListener) OnError(info ErrorInfo) {
	l.ch.sendError(&StreamError{Info: info})
}

// OnReconnecting implements WebSocketListener
func (l *channelListener) OnReconnecting(attempt uint32) {
	// Could send reconnecting event on error channel
}

// OnReconnectFailed implements WebSocketListener
//
// Terminal: no Disconnected follows, so the channels are closed here.
func (l *channelListener) OnReconnectFailed(attempts uint32) {
	l.ch.sendError(fmt.Errorf("all %d reconnection attempts exhausted", attempts))
	l.ch.Close()
}

// OnMessagesDropped implements WebSocketListener
//
// Reported when messages were dropped because Messages() fell behind while
// the queue held its configured buffer of unread messages
// (MessageOverflowDropNewest). Not terminal, so like OnError and
// OnUnauthenticated it is forwarded on Errors() without closing the
// channels; count is the number dropped since the previous report.
//
// Unlike other errors it never waits for room on Errors(): a caller that
// reads only Messages() keeps receiving, and the report is skipped.
// MessagesDroppedTotal() still counts every drop.
func (l *channelListener) OnMessagesDropped(count uint64) {
	l.ch.trySendError(fmt.Errorf("messages dropped: %d", count))
}

// StreamingClient wraps WebSocketClient with channel-based API
//
// This provides an idiomatic Go interface for consuming WebSocket messages
// using channels and range loops instead of callbacks.
type StreamingClient struct {
	client   *WebSocketClient
	channel  *MessageChannel
	listener *channelListener
}

// NewStreamingClient creates a channel-based streaming client for stock market data
//
// The bufferSize parameter controls how many messages can be buffered in the channel.
// Use a larger buffer if message processing may be slower than message arrival.
func NewStreamingClient(apiKey string, bufferSize int) (*StreamingClient, error) {
	ch := NewMessageChannel(bufferSize)
	listener := &channelListener{ch: ch}

	// Create WebSocket client with our channel listener
	client := NewWebSocketClient(apiKey, listener)

	return &StreamingClient{
		client:   client,
		channel:  ch,
		listener: listener,
	}, nil
}

// NewStreamingClientWithEndpoint creates a channel-based streaming client for a specific endpoint
//
// Use WebSocketEndpointStock for stock market data or WebSocketEndpointFutOpt for futures/options.
func NewStreamingClientWithEndpoint(apiKey string, endpoint WebSocketEndpoint, bufferSize int) (*StreamingClient, error) {
	ch := NewMessageChannel(bufferSize)
	listener := &channelListener{ch: ch}

	// Create WebSocket client with endpoint specification
	client := WebSocketClientNewWithEndpoint(apiKey, listener, endpoint)

	return &StreamingClient{
		client:   client,
		channel:  ch,
		listener: listener,
	}, nil
}

// Connect establishes WebSocket connection
func (sc *StreamingClient) Connect() error {
	err := sc.client.Connect()
	if err != nil {
		return fmt.Errorf("connect failed: %w", err)
	}
	return nil
}

// Subscribe adds a subscription to a channel/symbol pair
//
// Valid channels: "trades", "candles", "books", "aggregates", "indices" on the
// Stock endpoint; "trades", "candles", "books", "aggregates" on FutOpt.
// WithAfterHours is FutOpt only; on the Stock endpoint it is error 1005.
func (sc *StreamingClient) Subscribe(channel, symbol string, opts ...SubscribeOption) error {
	err := sc.client.Subscribe(channel, symbol, subscribeAfterHours(opts))
	if err != nil {
		return fmt.Errorf("subscribe failed: %w", err)
	}
	return nil
}

// Unsubscribe removes a subscription
//
// Pass the same options as the Subscribe call: an after-hours subscription
// is separate from the regular one.
func (sc *StreamingClient) Unsubscribe(channel, symbol string, opts ...SubscribeOption) error {
	err := sc.client.Unsubscribe(channel, symbol, subscribeAfterHours(opts))
	if err != nil {
		return fmt.Errorf("unsubscribe failed: %w", err)
	}
	return nil
}

// Messages returns the message channel for range iteration
//
// This channel will be closed when the WebSocket connection is closed.
func (sc *StreamingClient) Messages() <-chan StreamMessage {
	return sc.channel.Messages()
}

// Errors returns the error channel
//
// Monitor this channel to handle WebSocket errors.
func (sc *StreamingClient) Errors() <-chan error {
	return sc.channel.Errors()
}

// IsConnected returns true if the WebSocket is connected
func (sc *StreamingClient) IsConnected() bool {
	return sc.client.IsConnected()
}

// IsClosed returns true once the server has closed the connection with no
// reconnect to follow. False while reconnecting.
func (sc *StreamingClient) IsClosed() bool {
	return sc.client.IsClosed()
}

// MessagesDroppedTotal returns the number of messages dropped because
// Messages() fell behind while the queue held its configured buffer of
// unread messages (MessageOverflowDropNewest).
//
// Counted from the start of the current connection (every Connect() or
// reconnect restarts it); after disconnecting it still reads the last
// connection's count. 0 before the first Connect().
func (sc *StreamingClient) MessagesDroppedTotal() uint64 {
	return sc.client.MessagesDroppedTotal()
}

// Ping sends a ping message to the server.
// The optional state string will be echoed back in the pong response.
func (sc *StreamingClient) Ping(state *string) error {
	err := sc.client.Ping(state)
	if err != nil {
		return fmt.Errorf("ping failed: %w", err)
	}
	return nil
}

// QuerySubscriptions sends a query to the server for current subscriptions.
// The response arrives via the Messages() channel.
func (sc *StreamingClient) QuerySubscriptions() error {
	err := sc.client.QuerySubscriptions()
	if err != nil {
		return fmt.Errorf("query subscriptions failed: %w", err)
	}
	return nil
}

// Close disconnects and closes all channels
func (sc *StreamingClient) Close() error {
	sc.client.Disconnect()
	sc.channel.Close()
	sc.client.Destroy()
	return nil
}
