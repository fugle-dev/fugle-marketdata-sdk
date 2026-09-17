package marketdata_uniffi

import (
	"errors"
	"fmt"
)

// NewFugleRestClient creates a REST client with functional options.
//
// Requires exactly one non-empty authentication option: WithApiKey,
// WithBearerToken, or WithSdkToken. Otherwise it returns a ConfigError
// (code 1004, see ErrorInfoOf).
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

	// Core requires exactly one non-blank credential (ConfigError, code 1004)
	// and reports which one to use.
	kind, err := ValidateCredentials(&cfg.apiKey, &cfg.bearerToken, &cfg.sdkToken)
	if err != nil {
		return nil, err
	}

	// Call appropriate UniFFI constructor based on auth method.
	//
	// uniffi-bindgen-go 0.5 returns `error` and an untyped nil on success.
	// Older generators returned a concrete *MarketDataError; wrapping a nil
	// pointer of that type in `error` produced a non-nil interface, which is
	// why every call used to fail with "failed to create client: <nil>".
	// The regression test in config_test.go guards against that shape.
	var baseUrl *string
	if cfg.baseUrl != "" {
		baseUrl = &cfg.baseUrl
	}
	tls := TlsConfigRecord{}

	var client *RestClient
	var uerr error

	switch kind {
	case CredentialKindApiKey:
		client, uerr = NewRestClientWithApiKeyAndTls(cfg.apiKey, baseUrl, tls)
	case CredentialKindBearerToken:
		client, uerr = NewRestClientWithBearerTokenAndTls(cfg.bearerToken, baseUrl, tls)
	default:
		client, uerr = NewRestClientWithSdkTokenAndTls(cfg.sdkToken, baseUrl, tls)
	}

	if uerr != nil {
		return nil, fmt.Errorf("failed to create client: %w", uerr)
	}

	return client, nil
}

// NewFugleWebSocketClient creates a WebSocket client with functional options.
//
// Requires exactly one non-empty authentication option: WithApiKey,
// WithBearerToken, or WithSdkToken. Otherwise it returns a ConfigError
// (code 1004, see ErrorInfoOf).
//
// The listener parameter receives WebSocket events (OnConnected, OnAuthenticated, OnUnauthenticated,
// OnMessage, OnError, OnDisconnected, OnReconnecting, OnReconnectFailed).
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

	// Core requires exactly one non-blank credential (ConfigError, code 1004)
	// and reports which one to use.
	kind, err := ValidateCredentials(&cfg.apiKey, &cfg.bearerToken, &cfg.sdkToken)
	if err != nil {
		return nil, err
	}

	// Create channel-based wrapper
	ch := NewMessageChannel(100)
	channelListener := &channelListener{ch: ch}

	// Call appropriate UniFFI constructor based on auth method and endpoint
	// NOTE: Current UniFFI WebSocketClient constructors only accept api_key string.
	// For bearerToken/sdkToken support, this would need additional UniFFI constructors.
	var client *WebSocketClient

	if kind == CredentialKindApiKey {
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
				Enabled:            cfg.healthCheck.Enabled,
				HeartbeatTimeoutMs: cfg.healthCheck.HeartbeatTimeoutMs,
			}
		}
		var messageQueueRecord *MessageQueueConfigRecord
		if cfg.messageOverflow != nil || cfg.messageBuffer != nil {
			overflow := MessageOverflowRecordDropNewest
			if cfg.messageOverflow != nil && *cfg.messageOverflow == MessageOverflowUnbounded {
				overflow = MessageOverflowRecordUnbounded
			}
			var buffer uint32
			if cfg.messageBuffer != nil {
				buffer = *cfg.messageBuffer
			}
			messageQueueRecord = &MessageQueueConfigRecord{
				Overflow: overflow,
				Buffer:   buffer,
			}
		}

		var baseUrl *string
		if cfg.baseUrl != "" {
			baseUrl = &cfg.baseUrl
		}

		client = WebSocketClientNewWithOptions(cfg.apiKey, channelListener, cfg.endpoint, baseUrl, reconnectRecord, healthCheckRecord, nil, nil, messageQueueRecord)
	} else {
		return nil, errors.New("bearer token and SDK token authentication not yet supported for WebSocket client")
	}

	return &StreamingClient{
		client:   client,
		channel:  ch,
		listener: channelListener,
	}, nil
}
