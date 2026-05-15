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
//	for msg := range client.Messages() {
//	    fmt.Printf("Got: %s %s\n", msg.Event, *msg.Symbol)
//	}
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
		close(mc.messages)
		close(mc.errors)
	})
}

// channelListener implements WebSocketListener, forwarding to channels
type channelListener struct {
	ch *MessageChannel
}

// Ensure channelListener implements WebSocketListener
var _ WebSocketListener = (*channelListener)(nil)

// OnConnected implements WebSocketListener
func (l *channelListener) OnConnected() {
	// Connection established - could send event on separate channel if needed
}

// OnDisconnected implements WebSocketListener
func (l *channelListener) OnDisconnected() {
	l.ch.Close()
}

// OnMessage implements WebSocketListener
func (l *channelListener) OnMessage(message StreamMessage) {
	select {
	case l.ch.messages <- message:
	case <-l.ch.done:
		// Channel closed, drop message
	}
}

// OnError implements WebSocketListener
func (l *channelListener) OnError(errorMessage string) {
	select {
	case l.ch.errors <- fmt.Errorf("websocket error: %s", errorMessage):
	case <-l.ch.done:
		// Channel closed, drop error
	}
}

// OnReconnecting implements WebSocketListener
func (l *channelListener) OnReconnecting(attempt uint32) {
	// Could send reconnecting event on error channel
}

// OnReconnectFailed implements WebSocketListener
func (l *channelListener) OnReconnectFailed(attempts uint32) {
	select {
	case l.ch.errors <- fmt.Errorf("all %d reconnection attempts exhausted", attempts):
	case <-l.ch.done:
	}
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
		return fmt.Errorf("connect failed: %v", err)
	}
	return nil
}

// Subscribe adds a subscription to a channel/symbol pair
//
// Valid channels: "trades", "candles", "books", "aggregates", "indices"
func (sc *StreamingClient) Subscribe(channel, symbol string) error {
	err := sc.client.Subscribe(channel, symbol)
	if err != nil {
		return fmt.Errorf("subscribe failed: %v", err)
	}
	return nil
}

// Unsubscribe removes a subscription
func (sc *StreamingClient) Unsubscribe(channel, symbol string) error {
	err := sc.client.Unsubscribe(channel, symbol)
	if err != nil {
		return fmt.Errorf("unsubscribe failed: %v", err)
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

// IsClosed returns true if the WebSocket client has been shut down
func (sc *StreamingClient) IsClosed() bool {
	return sc.client.IsClosed()
}

// Ping sends a ping message to the server.
// The optional state string will be echoed back in the pong response.
func (sc *StreamingClient) Ping(state *string) error {
	err := sc.client.Ping(state)
	if err != nil {
		return fmt.Errorf("ping failed: %v", err)
	}
	return nil
}

// QuerySubscriptions sends a query to the server for current subscriptions.
// The response arrives via the Messages() channel.
func (sc *StreamingClient) QuerySubscriptions() error {
	err := sc.client.QuerySubscriptions()
	if err != nil {
		return fmt.Errorf("query subscriptions failed: %v", err)
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
