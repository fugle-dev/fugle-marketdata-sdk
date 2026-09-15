package tw.com.fugle.marketdata;

/**
 * Configuration options for WebSocket liveness detection.
 *
 * <p>The client declares a connection dead when no inbound frame arrives
 * within {@code heartbeatTimeoutMs}. The server sends a heartbeat every
 * 30 seconds, so any value above that works.
 *
 * <p>All fields are optional; null means the core default.
 *
 * <p><b>Default values:</b>
 * <ul>
 *   <li>enabled: true</li>
 *   <li>heartbeatTimeoutMs: 35000 (minimum 5000)</li>
 * </ul>
 *
 * <p><b>Example usage:</b>
 * <pre>{@code
 * HealthCheckOptions options = HealthCheckOptions.builder()
 *     .enabled(true)
 *     .heartbeatTimeoutMs(60000L)
 *     .build();
 * }</pre>
 */
public final class HealthCheckOptions {

    private final Boolean enabled;
    private final Long heartbeatTimeoutMs;

    private HealthCheckOptions(Boolean enabled, Long heartbeatTimeoutMs) {
        this.enabled = enabled;
        this.heartbeatTimeoutMs = heartbeatTimeoutMs;
    }

    /**
     * Get whether liveness detection is enabled.
     *
     * @return True if enabled, or null if using the default (true)
     */
    public Boolean getEnabled() {
        return enabled;
    }

    /**
     * Get the maximum gap between inbound frames before the connection is
     * declared dead.
     *
     * @return Timeout in milliseconds, or null if using the default (35000)
     */
    public Long getHeartbeatTimeoutMs() {
        return heartbeatTimeoutMs;
    }

    /**
     * Create a new builder for constructing HealthCheckOptions.
     *
     * @return A new builder instance
     */
    public static Builder builder() {
        return new Builder();
    }

    /**
     * Builder for creating immutable HealthCheckOptions instances.
     */
    public static class Builder {
        private Boolean enabled;
        private Long heartbeatTimeoutMs;

        private Builder() {}

        /**
         * Set whether liveness detection is enabled.
         *
         * @param enabled True to enable (default: true)
         * @return This builder for chaining
         */
        public Builder enabled(Boolean enabled) {
            this.enabled = enabled;
            return this;
        }

        /**
         * Set the maximum gap between inbound frames in milliseconds.
         *
         * @param heartbeatTimeoutMs Timeout in milliseconds (default: 35000, min: 5000)
         * @return This builder for chaining
         */
        public Builder heartbeatTimeoutMs(Long heartbeatTimeoutMs) {
            this.heartbeatTimeoutMs = heartbeatTimeoutMs;
            return this;
        }

        /**
         * Build the immutable HealthCheckOptions instance.
         *
         * <p>Validation is performed by the core, not here.
         *
         * @return Immutable HealthCheckOptions instance
         */
        public HealthCheckOptions build() {
            return new HealthCheckOptions(enabled, heartbeatTimeoutMs);
        }
    }
}
