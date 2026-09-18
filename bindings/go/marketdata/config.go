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

// HealthCheckConfig tunes WebSocket liveness detection, which is on by
// default; pass it with WithHealthCheck. Zero values for fields mean "use
// default". To turn detection off use WithoutHealthCheck.
//
// The client declares the connection dead when no inbound frame (data,
// heartbeat or pong) arrives within HeartbeatTimeoutMs. The server sends a
// heartbeat every 30 seconds.
type HealthCheckConfig struct {
	// HeartbeatTimeoutMs is the maximum gap between inbound frames in
	// milliseconds (default: 35000, min: 5000). Zero means "use default".
	// Does not apply when ProbeEnabled is true.
	HeartbeatTimeoutMs uint64
	// ProbeEnabled confirms a silent connection with a ping before
	// declaring it dead (default: false). After IdleProbeAfterMs of
	// silence one ping is sent; if nothing arrives within
	// ProbeTimeoutMs the connection is declared dead.
	ProbeEnabled bool
	// IdleProbeAfterMs is the silence before the probe, in milliseconds
	// (default: 30000, the server's heartbeat period; min: 5000). Zero
	// means "use default". Only used when ProbeEnabled is true.
	IdleProbeAfterMs uint64
	// ProbeTimeoutMs is how long to wait for any inbound frame after the
	// probe, in milliseconds (default: 5000, min: 1000). Zero means "use
	// default". Only used when ProbeEnabled is true.
	ProbeTimeoutMs uint64
}
