package marketdata_uniffi

import (
	"errors"
)

// Option configures a client (REST or WebSocket)
type Option func(*clientConfig) error

// clientConfig holds configuration for both REST and WebSocket clients
type clientConfig struct {
	apiKey      string
	bearerToken string
	sdkToken    string
	baseUrl     string
	endpoint    WebSocketEndpoint
	reconnect   *ReconnectConfig
	healthCheck *HealthCheckConfig
}

// WithApiKey sets API key authentication
func WithApiKey(key string) Option {
	return func(cfg *clientConfig) error {
		if key == "" {
			return errors.New("api key cannot be empty")
		}
		cfg.apiKey = key
		return nil
	}
}

// WithBearerToken sets bearer token authentication
func WithBearerToken(token string) Option {
	return func(cfg *clientConfig) error {
		if token == "" {
			return errors.New("bearer token cannot be empty")
		}
		cfg.bearerToken = token
		return nil
	}
}

// WithSdkToken sets SDK token authentication
func WithSdkToken(token string) Option {
	return func(cfg *clientConfig) error {
		if token == "" {
			return errors.New("sdk token cannot be empty")
		}
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
