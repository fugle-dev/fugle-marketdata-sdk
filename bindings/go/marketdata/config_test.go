//go:build cgo
// +build cgo

package marketdata_uniffi

import (
	"strings"
	"testing"
)

// Test 1: ReconnectConfig zero-value defaults
func TestReconnectConfigDefaults(t *testing.T) {
	cfg := ReconnectConfig{}

	// Zero values should be interpreted as "use default" by the system
	// The struct itself doesn't enforce defaults, those come from core constants
	if cfg.MaxAttempts != 0 {
		t.Errorf("expected MaxAttempts default 0 (use core default), got %d", cfg.MaxAttempts)
	}
	if cfg.InitialDelayMs != 0 {
		t.Errorf("expected InitialDelayMs default 0 (use core default), got %d", cfg.InitialDelayMs)
	}
	if cfg.MaxDelayMs != 0 {
		t.Errorf("expected MaxDelayMs default 0 (use core default), got %d", cfg.MaxDelayMs)
	}
}

// Test 2: HealthCheckConfig zero-value defaults
func TestHealthCheckConfigDefaults(t *testing.T) {
	cfg := HealthCheckConfig{}

	// Enabled defaults to false (Go zero value matches DEFAULT_HEALTH_CHECK_ENABLED)
	if cfg.Enabled != false {
		t.Errorf("expected Enabled default false, got %v", cfg.Enabled)
	}
	if cfg.HeartbeatTimeoutMs != 0 {
		t.Errorf("expected HeartbeatTimeoutMs default 0 (use core default), got %d", cfg.HeartbeatTimeoutMs)
	}
}

// Test 3: ReconnectConfig custom values
func TestReconnectConfigCustomValues(t *testing.T) {
	cfg := ReconnectConfig{
		MaxAttempts:    3,
		InitialDelayMs: 2000,
		MaxDelayMs:     30000,
	}

	if cfg.MaxAttempts != 3 {
		t.Errorf("expected MaxAttempts 3, got %d", cfg.MaxAttempts)
	}
	if cfg.InitialDelayMs != 2000 {
		t.Errorf("expected InitialDelayMs 2000, got %d", cfg.InitialDelayMs)
	}
	if cfg.MaxDelayMs != 30000 {
		t.Errorf("expected MaxDelayMs 30000, got %d", cfg.MaxDelayMs)
	}
}

// Test 4: HealthCheckConfig custom values
func TestHealthCheckConfigCustomValues(t *testing.T) {
	cfg := HealthCheckConfig{
		Enabled:            true,
		HeartbeatTimeoutMs: 10000,
	}

	if cfg.Enabled != true {
		t.Errorf("expected Enabled true, got %v", cfg.Enabled)
	}
	if cfg.HeartbeatTimeoutMs != 10000 {
		t.Errorf("expected HeartbeatTimeoutMs 10000, got %d", cfg.HeartbeatTimeoutMs)
	}
}

// Test 5: RestClient with ApiKey only (should not get auth error)
func TestRestClientExactlyOneAuth_ApiKey(t *testing.T) {
	// Construction does not touch the network, so it must succeed.
	client, err := NewFugleRestClient(WithApiKey("test-api-key"))
	if err != nil {
		t.Fatalf("expected success, got error: %v", err)
	}
	if client == nil {
		t.Fatal("expected a client, got nil")
	}
}

// Test 6: RestClient with BearerToken only (should not get auth error)
func TestRestClientExactlyOneAuth_BearerToken(t *testing.T) {
	// Construction does not touch the network, so it must succeed.
	client, err := NewFugleRestClient(WithBearerToken("test-bearer-token"))
	if err != nil {
		t.Fatalf("expected success, got error: %v", err)
	}
	if client == nil {
		t.Fatal("expected a client, got nil")
	}
}

// Test 7: RestClient with SdkToken only (should not get auth error)
func TestRestClientExactlyOneAuth_SdkToken(t *testing.T) {
	// Construction does not touch the network, so it must succeed.
	client, err := NewFugleRestClient(WithSdkToken("test-sdk-token"))
	if err != nil {
		t.Fatalf("expected success, got error: %v", err)
	}
	if client == nil {
		t.Fatal("expected a client, got nil")
	}
}

// Test 7b: WithBaseUrl is applied instead of being ignored
func TestRestClientWithBaseUrl(t *testing.T) {
	client, err := NewFugleRestClient(WithApiKey("test-api-key"), WithBaseUrl("https://example.invalid/marketdata"))
	if err != nil {
		t.Fatalf("expected success, got error: %v", err)
	}
	// The SDK owns the version segment and appends it to the prefix.
	if got := client.BaseUrl(); got != "https://example.invalid/marketdata/v1.0" {
		t.Fatalf("expected base URL to be applied, got %q", got)
	}
}

// Test 8: RestClient with no auth (should fail validation)
func TestRestClientNoAuth(t *testing.T) {
	_, err := NewFugleRestClient()

	if err == nil {
		t.Fatal("expected error, got nil")
	}
	assertCredentialsRejected(t, err)
}

// Test 9: RestClient with multiple auth methods (should fail validation)
func TestRestClientMultipleAuth(t *testing.T) {
	_, err := NewFugleRestClient(
		WithApiKey("test-api-key"),
		WithBearerToken("test-bearer-token"),
	)

	if err == nil {
		t.Fatal("expected error, got nil")
	}
	assertCredentialsRejected(t, err)
}

// Test 10: empty or whitespace-only credentials count as not provided
func TestBlankCredentials(t *testing.T) {
	cases := map[string][]Option{
		"empty api key":      {WithApiKey("")},
		"blank bearer token": {WithBearerToken("   ")},
		"blank sdk token":    {WithSdkToken("\t")},
		"two blanks":         {WithApiKey(""), WithSdkToken(" ")},
	}
	for name, opts := range cases {
		t.Run(name, func(t *testing.T) {
			_, err := NewFugleRestClient(opts...)
			assertCredentialsRejected(t, err)
		})
	}
}

// assertCredentialsRejected checks the core's credential error: a
// ConfigError with code 1004.
func assertCredentialsRejected(t *testing.T, err error) {
	t.Helper()
	if err == nil {
		t.Fatal("expected error, got nil")
	}
	info, ok := ErrorInfoOf(err)
	if !ok {
		t.Fatalf("expected an SDK error, got: %v", err)
	}
	if info.Code != 1004 || info.SourceKind != ErrorSourceKindClient {
		t.Errorf("expected code 1004 / client, got %d / %v", info.Code, info.SourceKind)
	}
	if !strings.Contains(info.Message, "exactly one non-empty credential") {
		t.Errorf("unexpected message: %s", info.Message)
	}
}

// Test 11: WebSocket with no auth (should fail validation)
func TestWebSocketNoAuth(t *testing.T) {
	listener := &mockListener{}
	_, err := NewFugleWebSocketClient(listener)

	if err == nil {
		t.Fatal("expected error, got nil")
	}
	assertCredentialsRejected(t, err)
}

// Test 12: WebSocket with multiple auth methods (should fail validation)
func TestWebSocketMultipleAuth(t *testing.T) {
	listener := &mockListener{}
	_, err := NewFugleWebSocketClient(
		listener,
		WithApiKey("test-api-key"),
		WithSdkToken("test-sdk-token"),
	)

	if err == nil {
		t.Fatal("expected error, got nil")
	}
	assertCredentialsRejected(t, err)
}

// Test 13: Option functions return non-nil
func TestOptionFunctions(t *testing.T) {
	tests := []struct {
		name string
		opt  Option
	}{
		{"WithApiKey", WithApiKey("test-key")},
		{"WithBearerToken", WithBearerToken("test-token")},
		{"WithSdkToken", WithSdkToken("test-sdk")},
		{"WithBaseUrl", WithBaseUrl("https://test.example.com")},
		{"WithEndpoint", WithEndpoint(WebSocketEndpointStock)},
		{"WithReconnect", WithReconnect(ReconnectConfig{MaxAttempts: 3})},
		{"WithHealthCheck", WithHealthCheck(HealthCheckConfig{Enabled: true})},
		{"WithMessageOverflow", WithMessageOverflow(MessageOverflowUnbounded)},
		{"WithMessageBuffer", WithMessageBuffer(8192)},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if tt.opt == nil {
				t.Errorf("%s returned nil Option", tt.name)
			}
		})
	}
}

// Test 14: WithMessageBuffer rejects zero and negative values
func TestWithMessageBufferInvalid(t *testing.T) {
	for _, n := range []int{0, -1, -100} {
		cfg := &clientConfig{}
		err := WithMessageBuffer(n)(cfg)
		if err == nil {
			t.Fatalf("expected error for buffer %d, got nil", n)
		}
		if cfg.messageBuffer != nil {
			t.Fatalf("expected messageBuffer to stay unset for buffer %d", n)
		}
	}
}

// Test 15: WithMessageBuffer accepts positive values
func TestWithMessageBufferValid(t *testing.T) {
	cfg := &clientConfig{}
	if err := WithMessageBuffer(8192)(cfg); err != nil {
		t.Fatalf("expected success, got error: %v", err)
	}
	if cfg.messageBuffer == nil || *cfg.messageBuffer != 8192 {
		t.Fatalf("expected messageBuffer 8192, got %v", cfg.messageBuffer)
	}
}

// Test 16: clientConfig defaults to no explicit overflow/buffer (core default applies)
func TestMessageQueueDefaults(t *testing.T) {
	cfg := &clientConfig{}
	if cfg.messageOverflow != nil {
		t.Errorf("expected messageOverflow default nil (use core default), got %v", cfg.messageOverflow)
	}
	if cfg.messageBuffer != nil {
		t.Errorf("expected messageBuffer default nil (use core default), got %v", cfg.messageBuffer)
	}
}

// Test 17: WithMessageOverflow accepts both DropNewest and Unbounded
func TestWithMessageOverflowValues(t *testing.T) {
	tests := []MessageOverflow{MessageOverflowDropNewest, MessageOverflowUnbounded}

	for _, overflow := range tests {
		cfg := &clientConfig{}
		if err := WithMessageOverflow(overflow)(cfg); err != nil {
			t.Fatalf("expected success for %v, got error: %v", overflow, err)
		}
		if cfg.messageOverflow == nil || *cfg.messageOverflow != overflow {
			t.Fatalf("expected messageOverflow %v, got %v", overflow, cfg.messageOverflow)
		}
	}
}

// Test 18: WithMessageOverflow rejects unknown values
func TestWithMessageOverflowInvalid(t *testing.T) {
	cfg := &clientConfig{}
	err := WithMessageOverflow(MessageOverflow(99))(cfg)
	if err == nil {
		t.Fatal("expected error for invalid overflow policy, got nil")
	}
	if cfg.messageOverflow != nil {
		t.Fatal("expected messageOverflow to stay unset for an invalid policy")
	}
}

// Mock listener for WebSocket tests
type mockListener struct{}

func (m *mockListener) OnConnected()                       {}
func (m *mockListener) OnAuthenticated(dataJson *string)   {}
func (m *mockListener) OnUnauthenticated(dataJson *string) {}
func (m *mockListener) OnDisconnected(willReconnect bool)  {}
func (m *mockListener) OnMessage(message StreamMessage)    {}
func (m *mockListener) OnError(info ErrorInfo)             {}
func (m *mockListener) OnReconnecting(attempt uint32)      {}
func (m *mockListener) OnReconnectFailed(attempts uint32)  {}
func (m *mockListener) OnMessagesDropped(count uint64)     {}
