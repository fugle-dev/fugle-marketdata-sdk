package tw.com.fugle.marketdata;

/**
 * Configuration options for WebSocket liveness detection.
 *
 * <p>The client declares a connection dead when no inbound frame arrives
 * within {@code heartbeatTimeoutMs}. The server sends a heartbeat every
 * 30 seconds, so any value above that works. {@code heartbeatTimeoutMs}
 * does not apply when {@code probeEnabled} is true.
 *
 * <p>All fields are optional; null means the core default.
 *
 * <p><b>Default values:</b>
 * <ul>
 *   <li>enabled: true</li>
 *   <li>heartbeatTimeoutMs: 35000 (minimum 5000)</li>
 *   <li>probeEnabled: false</li>
 *   <li>idleProbeAfterMs: 30000 (minimum 5000)</li>
 *   <li>probeTimeoutMs: 5000 (minimum 1000)</li>
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
    private final Boolean probeEnabled;
    private final Long idleProbeAfterMs;
    private final Long probeTimeoutMs;

    private HealthCheckOptions(
            Boolean enabled,
            Long heartbeatTimeoutMs,
            Boolean probeEnabled,
            Long idleProbeAfterMs,
            Long probeTimeoutMs) {
        this.enabled = enabled;
        this.heartbeatTimeoutMs = heartbeatTimeoutMs;
        this.probeEnabled = probeEnabled;
        this.idleProbeAfterMs = idleProbeAfterMs;
        this.probeTimeoutMs = probeTimeoutMs;
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
     * declared dead. Does not apply when {@link #getProbeEnabled()} is true.
     *
     * @return Timeout in milliseconds, or null if using the default (35000)
     */
    public Long getHeartbeatTimeoutMs() {
        return heartbeatTimeoutMs;
    }

    /**
     * Get whether a silent connection is confirmed with a ping before being
     * declared dead. After {@link #getIdleProbeAfterMs()} of silence one
     * ping is sent; if nothing arrives within {@link #getProbeTimeoutMs()}
     * the connection is declared dead.
     *
     * @return True if probing is enabled, or null if using the default (false)
     */
    public Boolean getProbeEnabled() {
        return probeEnabled;
    }

    /**
     * Get the silence before the probe.
     *
     * @return Duration in milliseconds, or null if using the default (30000)
     */
    public Long getIdleProbeAfterMs() {
        return idleProbeAfterMs;
    }

    /**
     * Get how long to wait for any inbound frame after the probe.
     *
     * @return Timeout in milliseconds, or null if using the default (5000)
     */
    public Long getProbeTimeoutMs() {
        return probeTimeoutMs;
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
        private Boolean probeEnabled;
        private Long idleProbeAfterMs;
        private Long probeTimeoutMs;

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
         * Set whether to confirm a silent connection with a ping before
         * declaring it dead.
         *
         * @param probeEnabled True to enable probing (default: false)
         * @return This builder for chaining
         */
        public Builder probeEnabled(Boolean probeEnabled) {
            this.probeEnabled = probeEnabled;
            return this;
        }

        /**
         * Set the silence before the probe, in milliseconds. Only used when
         * {@link #probeEnabled} is true.
         *
         * @param idleProbeAfterMs Duration in milliseconds (default: 30000, min: 5000)
         * @return This builder for chaining
         */
        public Builder idleProbeAfterMs(Long idleProbeAfterMs) {
            this.idleProbeAfterMs = idleProbeAfterMs;
            return this;
        }

        /**
         * Set how long to wait for any inbound frame after the probe, in
         * milliseconds. Only used when {@link #probeEnabled} is true.
         *
         * @param probeTimeoutMs Timeout in milliseconds (default: 5000, min: 1000)
         * @return This builder for chaining
         */
        public Builder probeTimeoutMs(Long probeTimeoutMs) {
            this.probeTimeoutMs = probeTimeoutMs;
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
            return new HealthCheckOptions(enabled, heartbeatTimeoutMs, probeEnabled, idleProbeAfterMs, probeTimeoutMs);
        }
    }
}
