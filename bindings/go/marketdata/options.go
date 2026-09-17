package marketdata_uniffi

import (
	"errors"
)

// Option configures a client (REST or WebSocket)
type Option func(*clientConfig) error

// clientConfig holds configuration for both REST and WebSocket clients
type clientConfig struct {
	apiKey          string
	bearerToken     string
	sdkToken        string
	baseUrl         string
	endpoint        WebSocketEndpoint
	reconnect       *ReconnectConfig
	healthCheck     *HealthCheckConfig
	messageOverflow *MessageOverflow
	messageBuffer   *uint32
}

// MessageOverflow controls what happens to new WebSocket messages once the
// client's message queue holds MessageBuffer unread messages.
type MessageOverflow int

const (
	// MessageOverflowDropNewest discards new messages while the queue is
	// full, keeping earlier unread messages (default).
	MessageOverflowDropNewest MessageOverflow = iota
	// MessageOverflowUnbounded lets the queue grow without bound instead of
	// dropping messages.
	MessageOverflowUnbounded
)

// WithApiKey sets API key authentication.
//
// An empty or whitespace-only value counts as not provided; the client
// constructor rejects it unless another credential is given.
func WithApiKey(key string) Option {
	return func(cfg *clientConfig) error {
		cfg.apiKey = key
		return nil
	}
}

// WithBearerToken sets bearer token authentication.
//
// An empty or whitespace-only value counts as not provided; the client
// constructor rejects it unless another credential is given.
func WithBearerToken(token string) Option {
	return func(cfg *clientConfig) error {
		cfg.bearerToken = token
		return nil
	}
}

// WithSdkToken sets SDK token authentication.
//
// An empty or whitespace-only value counts as not provided; the client
// constructor rejects it unless another credential is given.
func WithSdkToken(token string) Option {
	return func(cfg *clientConfig) error {
		cfg.sdkToken = token
		return nil
	}
}

// WithBaseUrl sets custom base URL for REST client
func WithBaseUrl(url string) Option {
	return func(cfg *clientConfig) error {
		cfg.baseUrl = url
		return nil
	}
}

// WithEndpoint sets WebSocket endpoint (default: Stock)
func WithEndpoint(ep WebSocketEndpoint) Option {
	return func(cfg *clientConfig) error {
		cfg.endpoint = ep
		return nil
	}
}

// WithReconnect sets reconnection configuration for WebSocket client
func WithReconnect(reconnect ReconnectConfig) Option {
	return func(cfg *clientConfig) error {
		cfg.reconnect = &reconnect
		return nil
	}
}

// WithHealthCheck sets health check configuration for WebSocket client
func WithHealthCheck(healthCheck HealthCheckConfig) Option {
	return func(cfg *clientConfig) error {
		cfg.healthCheck = &healthCheck
		return nil
	}
}

// WithMessageOverflow sets what happens to new WebSocket messages once the
// message queue is full (default: MessageOverflowDropNewest).
func WithMessageOverflow(overflow MessageOverflow) Option {
	return func(cfg *clientConfig) error {
		switch overflow {
		case MessageOverflowDropNewest, MessageOverflowUnbounded:
		default:
			return errors.New("invalid message overflow policy")
		}
		cfg.messageOverflow = &overflow
		return nil
	}
}

// WithMessageBuffer sets how many unread WebSocket messages the client's
// message queue holds before the overflow policy kicks in (default: 4096).
func WithMessageBuffer(n int) Option {
	return func(cfg *clientConfig) error {
		if n <= 0 {
			return errors.New("message buffer must be greater than zero")
		}
		buffer := uint32(n)
		cfg.messageBuffer = &buffer
		return nil
	}
}
