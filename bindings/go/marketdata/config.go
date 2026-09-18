package marketdata_uniffi

// ReconnectConfig tunes WebSocket auto-reconnect, which is on by default;
// pass it with WithReconnect. Zero values for fields mean "use default". To
// turn auto-reconnect off use WithoutReconnect.
type ReconnectConfig struct {
	// MaxAttempts is the maximum number of reconnection attempts; zero means
	// unlimited (the default), retrying at most MaxDelayMs apart
	MaxAttempts uint32
	// InitialDelayMs is the initial reconnection delay in milliseconds (default: 1000, min: 100)
	InitialDelayMs uint64
	// MaxDelayMs is the maximum reconnection delay in milliseconds (default: 60000)
	MaxDelayMs uint64
}

// HealthCheckConfig configures WebSocket liveness detection.
//
// The client declares the connection dead when no inbound frame (data,
// heartbeat or pong) arrives within HeartbeatTimeoutMs. The server sends a
// heartbeat every 30 seconds.
//
// Without WithHealthCheck the client uses the core defaults, which enable
// detection. Passing a HealthCheckConfig sets Enabled explicitly, so the Go
// zero value (false) turns detection off.
type HealthCheckConfig struct {
	// Enabled controls whether liveness detection is active
	Enabled bool
	// HeartbeatTimeoutMs is the maximum gap between inbound frames in
	// milliseconds (default: 35000, min: 5000). Zero means "use default".
	HeartbeatTimeoutMs uint64
}
