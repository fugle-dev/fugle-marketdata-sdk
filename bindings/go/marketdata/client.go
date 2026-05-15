package marketdata_uniffi

import (
	"errors"
	"fmt"
)

// NewFugleRestClient creates a REST client with functional options.
//
// Requires exactly one authentication option: WithApiKey, WithBearerToken, or WithSdkToken.
//
// Example:
//
//	client, err := marketdata_uniffi.NewFugleRestClient(
//	    marketdata_uniffi.WithApiKey("your-api-key"),
//	)
func NewFugleRestClient(opts ...Option) (*RestClient, error) {
	cfg := &clientConfig{}

	// Apply all options
	for _, opt := range opts {
		if err := opt(cfg); err != nil {
			return nil, err
		}
	}

	// Validate exactly one auth method
	authCount := 0
	if cfg.apiKey != "" {
		authCount++
	}
	if cfg.bearerToken != "" {
		authCount++
	}
	if cfg.sdkToken != "" {
		authCount++
	}

	if authCount == 0 {
		return nil, errors.New("provide exactly one of: WithApiKey, WithBearerToken, or WithSdkToken")
	}
	if authCount > 1 {
		return nil, errors.New("provide exactly one of: WithApiKey, WithBearerToken, or WithSdkToken")
	}

	// Call appropriate UniFFI constructor based on auth method
	var client *RestClient
	var err error

	if cfg.apiKey != "" {
		client, err = NewRestClientWithApiKey(cfg.apiKey)
	} else if cfg.bearerToken != "" {
		client, err = NewRestClientWithBearerToken(cfg.bearerToken)
	} else if cfg.sdkToken != "" {
		client, err = NewRestClientWithSdkToken(cfg.sdkToken)
	}

	if err != nil {
		return nil, fmt.Errorf("failed to create client: %w", err)
	}

	// TODO: Apply baseUrl when RestClient exposes base_url setter
	// Currently baseUrl is stored but not applied (same as Python/Node.js)
	_ = cfg.baseUrl

	return client, nil
}

// NewFugleWebSocketClient creates a WebSocket client with functional options.
//
// Requires exactly one authentication option: WithApiKey, WithBearerToken, or WithSdkToken.
//
// The listener parameter receives WebSocket events (OnConnected, OnMessage, OnError, OnDisconnected).
//
// Example:
//
//	client, err := marketdata_uniffi.NewFugleWebSocketClient(
//	    listener,
//	    marketdata_uniffi.WithApiKey("your-api-key"),
//	    marketdata_uniffi.WithEndpoint(marketdata_uniffi.WebSocketEndpointStock),
//	)
func NewFugleWebSocketClient(listener WebSocketListener, opts ...Option) (*StreamingClient, error) {
	cfg := &clientConfig{
		endpoint: WebSocketEndpointStock, // Default endpoint
	}

	// Apply all options
	for _, opt := range opts {
		if err := opt(cfg); err != nil {
			return nil, err
		}
	}

	// Validate exactly one auth method
	authCount := 0
	if cfg.apiKey != "" {
		authCount++
	}
	if cfg.bearerToken != "" {
		authCount++
	}
	if cfg.sdkToken != "" {
		authCount++
	}

	if authCount == 0 {
		return nil, errors.New("provide exactly one of: WithApiKey, WithBearerToken, or WithSdkToken")
	}
	if authCount > 1 {
		return nil, errors.New("provide exactly one of: WithApiKey, WithBearerToken, or WithSdkToken")
	}

	// Create channel-based wrapper
	ch := NewMessageChannel(100)
	channelListener := &channelListener{ch: ch}

	// Call appropriate UniFFI constructor based on auth method and endpoint
	// NOTE: Current UniFFI WebSocketClient constructors only accept api_key string.
	// For bearerToken/sdkToken support, this would need additional UniFFI constructors.
	var client *WebSocketClient

	if cfg.apiKey != "" {
		var reconnectRecord *ReconnectConfigRecord
		if cfg.reconnect != nil {
			reconnectRecord = &ReconnectConfigRecord{
				MaxAttempts:    cfg.reconnect.MaxAttempts,
				InitialDelayMs: cfg.reconnect.InitialDelayMs,
				MaxDelayMs:     cfg.reconnect.MaxDelayMs,
			}
		}
		var healthCheckRecord *HealthCheckConfigRecord
		if cfg.healthCheck != nil {
			healthCheckRecord = &HealthCheckConfigRecord{
				Enabled:        cfg.healthCheck.Enabled,
				IntervalMs:     cfg.healthCheck.IntervalMs,
				MaxMissedPongs: cfg.healthCheck.MaxMissedPongs,
			}
		}

		if cfg.baseUrl != "" {
			// Use custom base URL constructor
			client = WebSocketClientNewWithUrl(cfg.apiKey, channelListener, cfg.endpoint, cfg.baseUrl, reconnectRecord, healthCheckRecord)
		} else if cfg.reconnect != nil || cfg.healthCheck != nil {
			client = WebSocketClientNewWithConfig(cfg.apiKey, channelListener, cfg.endpoint, reconnectRecord, healthCheckRecord)
		} else if cfg.endpoint == WebSocketEndpointStock {
			client = NewWebSocketClient(cfg.apiKey, channelListener)
		} else {
			client = WebSocketClientNewWithEndpoint(cfg.apiKey, channelListener, cfg.endpoint)
		}
	} else {
		return nil, errors.New("bearer token and SDK token authentication not yet supported for WebSocket client")
	}

	return &StreamingClient{
		client:   client,
		channel:  ch,
		listener: channelListener,
	}, nil
}
