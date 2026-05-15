package marketdata_uniffi

// ReconnectConfig configures WebSocket reconnection behavior.
// Zero values for fields mean "use default".
type ReconnectConfig struct {
	// MaxAttempts is the maximum number of reconnection attempts (default: 5, min: 1)
	MaxAttempts uint32
	// InitialDelayMs is the initial reconnection delay in milliseconds (default: 1000, min: 100)
	InitialDelayMs uint64
	// MaxDelayMs is the maximum reconnection delay in milliseconds (default: 60000)
	MaxDelayMs uint64
}

// HealthCheckConfig configures WebSocket health check behavior.
// Zero values for fields mean "use default".
type HealthCheckConfig struct {
	// Enabled controls whether health check is active (default: false)
	Enabled bool
	// IntervalMs is the interval between ping messages in milliseconds (default: 30000, min: 5000)
	IntervalMs uint64
	// MaxMissedPongs is the maximum missed pongs before disconnect (default: 2, min: 1)
	MaxMissedPongs uint64
}
