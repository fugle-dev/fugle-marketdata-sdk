package marketdata_uniffi

import (
	"errors"
	"time"
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
	noReconnect     bool
	healthCheck     *HealthCheckConfig
	noHealthCheck   bool
	messageOverflow *MessageOverflow
	messageBuffer   *uint32
	authTimeoutMs   *uint64
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

// WithReconnect tunes auto-reconnect for the WebSocket client. The client
// auto-reconnects without it too; see WithoutReconnect to turn it off.
// Between WithReconnect and WithoutReconnect, the last one given wins.
func WithReconnect(reconnect ReconnectConfig) Option {
	return func(cfg *clientConfig) error {
		cfg.reconnect = &reconnect
		cfg.noReconnect = false
		return nil
	}
}

// WithoutReconnect turns auto-reconnect off: once the connection drops the
// client stays closed until Connect is called again.
func WithoutReconnect() Option {
	return func(cfg *clientConfig) error {
		cfg.reconnect = nil
		cfg.noReconnect = true
		return nil
	}
}

// WithHealthCheck tunes liveness detection for the WebSocket client. The
// client detects a dead connection without it too; see WithoutHealthCheck to
// turn detection off. Between WithHealthCheck and WithoutHealthCheck, the
// last one given wins.
func WithHealthCheck(healthCheck HealthCheckConfig) Option {
	return func(cfg *clientConfig) error {
		cfg.healthCheck = &healthCheck
		cfg.noHealthCheck = false
		return nil
	}
}

// WithoutHealthCheck turns liveness detection off: a connection that goes
// silent without closing is not declared dead, so it is not reconnected.
func WithoutHealthCheck() Option {
	return func(cfg *clientConfig) error {
		cfg.healthCheck = nil
		cfg.noHealthCheck = true
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

// WithAuthTimeout sets how long the WebSocket auth handshake may take once
// the connection is open, from the auth frame being sent until the server's
// verdict (default: 10s). It applies to the first Connect and to every
// reconnect; elapsing it fails the attempt with a timeout error (code 3001).
// Must be at least one millisecond (core takes it in milliseconds). The
// server itself allows 60 seconds.
func WithAuthTimeout(d time.Duration) Option {
	return func(cfg *clientConfig) error {
		if d < time.Millisecond {
			return errors.New("auth timeout must be at least one millisecond")
		}
		ms := uint64(d / time.Millisecond)
		cfg.authTimeoutMs = &ms
		return nil
	}
}

// SubscribeOption configures a StreamingClient Subscribe or Unsubscribe call
type SubscribeOption func(*subscribeConfig)

type subscribeConfig struct {
	afterHours *bool
}

// WithAfterHours selects the after-hours (盤後) session. FutOpt endpoint only:
// on the Stock endpoint, Subscribe and Unsubscribe return error 1005.
func WithAfterHours(afterHours bool) SubscribeOption {
	return func(cfg *subscribeConfig) {
		cfg.afterHours = &afterHours
	}
}

// subscribeAfterHours is the after-hours value opts set, nil if none does.
func subscribeAfterHours(opts []SubscribeOption) *bool {
	var cfg subscribeConfig
	for _, opt := range opts {
		opt(&cfg)
	}
	return cfg.afterHours
}
