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
	if cfg.IntervalMs != 0 {
		t.Errorf("expected IntervalMs default 0 (use core default), got %d", cfg.IntervalMs)
	}
	if cfg.MaxMissedPongs != 0 {
		t.Errorf("expected MaxMissedPongs default 0 (use core default), got %d", cfg.MaxMissedPongs)
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
		Enabled:        true,
		IntervalMs:     10000,
		MaxMissedPongs: 3,
	}

	if cfg.Enabled != true {
		t.Errorf("expected Enabled true, got %v", cfg.Enabled)
	}
	if cfg.IntervalMs != 10000 {
		t.Errorf("expected IntervalMs 10000, got %d", cfg.IntervalMs)
	}
	if cfg.MaxMissedPongs != 3 {
		t.Errorf("expected MaxMissedPongs 3, got %d", cfg.MaxMissedPongs)
	}
}

// Test 5: RestClient with ApiKey only (should not get auth error)
func TestRestClientExactlyOneAuth_ApiKey(t *testing.T) {
	_, err := NewFugleRestClient(WithApiKey("test-api-key"))

	// May fail with network error, but should NOT be an auth validation error
	if err != nil && strings.Contains(err.Error(), "provide exactly one of") {
		t.Errorf("got auth validation error, expected network error or success: %v", err)
	}
}

// Test 6: RestClient with BearerToken only (should not get auth error)
func TestRestClientExactlyOneAuth_BearerToken(t *testing.T) {
	_, err := NewFugleRestClient(WithBearerToken("test-bearer-token"))

	// May fail with network error, but should NOT be an auth validation error
	if err != nil && strings.Contains(err.Error(), "provide exactly one of") {
		t.Errorf("got auth validation error, expected network error or success: %v", err)
	}
}

// Test 7: RestClient with SdkToken only (should not get auth error)
func TestRestClientExactlyOneAuth_SdkToken(t *testing.T) {
	_, err := NewFugleRestClient(WithSdkToken("test-sdk-token"))

	// May fail with network error, but should NOT be an auth validation error
	if err != nil && strings.Contains(err.Error(), "provide exactly one of") {
		t.Errorf("got auth validation error, expected network error or success: %v", err)
	}
}

// Test 8: RestClient with no auth (should fail validation)
func TestRestClientNoAuth(t *testing.T) {
	_, err := NewFugleRestClient()

	if err == nil {
		t.Fatal("expected error, got nil")
	}
	if !strings.Contains(err.Error(), "provide exactly one of") {
		t.Errorf("expected 'provide exactly one of' error, got: %v", err)
	}
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
	if !strings.Contains(err.Error(), "provide exactly one of") {
		t.Errorf("expected 'provide exactly one of' error, got: %v", err)
	}
}

// Test 10: WithApiKey with empty string (should fail)
func TestWithApiKeyEmpty(t *testing.T) {
	_, err := NewFugleRestClient(WithApiKey(""))

	if err == nil {
		t.Fatal("expected error for empty API key, got nil")
	}
	if !strings.Contains(err.Error(), "cannot be empty") {
		t.Errorf("expected 'cannot be empty' error, got: %v", err)
	}
}

// Test 11: WebSocket with no auth (should fail validation)
func TestWebSocketNoAuth(t *testing.T) {
	listener := &mockListener{}
	_, err := NewFugleWebSocketClient(listener)

	if err == nil {
		t.Fatal("expected error, got nil")
	}
	if !strings.Contains(err.Error(), "provide exactly one of") {
		t.Errorf("expected 'provide exactly one of' error, got: %v", err)
	}
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
	if !strings.Contains(err.Error(), "provide exactly one of") {
		t.Errorf("expected 'provide exactly one of' error, got: %v", err)
	}
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
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if tt.opt == nil {
				t.Errorf("%s returned nil Option", tt.name)
			}
		})
	}
}

// Mock listener for WebSocket tests
type mockListener struct{}

func (m *mockListener) OnConnected()                      {}
func (m *mockListener) OnDisconnected()                   {}
func (m *mockListener) OnMessage(message StreamMessage)   {}
func (m *mockListener) OnError(errorMessage string)       {}
func (m *mockListener) OnReconnecting(attempt uint32)     {}
func (m *mockListener) OnReconnectFailed(attempts uint32) {}
