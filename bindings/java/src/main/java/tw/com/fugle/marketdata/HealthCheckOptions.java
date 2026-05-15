package tw.com.fugle.marketdata;

/**
 * Configuration options for WebSocket health check behavior.
 *
 * <p>This immutable class defines health check parameters using the builder pattern.
 * All fields are optional - null values indicate the default should be used by the client.
 *
 * <p><b>Default values:</b>
 * <ul>
 *   <li>enabled: false (health checks disabled by default)</li>
 *   <li>intervalMs: 30000 (interval between ping messages in milliseconds)</li>
 *   <li>maxMissedPongs: 2 (maximum missed pong responses before disconnect)</li>
 * </ul>
 *
 * <p><b>Example usage:</b>
 * <pre>{@code
 * HealthCheckOptions options = HealthCheckOptions.builder()
 *     .enabled(true)
 *     .intervalMs(60000L)
 *     .maxMissedPongs(3L)
 *     .build();
 * }</pre>
 */
public final class HealthCheckOptions {

    private final Boolean enabled;
    private final Long intervalMs;
    private final Long maxMissedPongs;

    private HealthCheckOptions(Boolean enabled, Long intervalMs, Long maxMissedPongs) {
        this.enabled = enabled;
        this.intervalMs = intervalMs;
        this.maxMissedPongs = maxMissedPongs;
    }

    /**
     * Get whether health checks are enabled.
     *
     * @return True if enabled, or null if using default (false)
     */
    public Boolean getEnabled() {
        return enabled;
    }

    /**
     * Get the interval in milliseconds between ping messages.
     *
     * @return Interval in milliseconds, or null if using default (30000)
     */
    public Long getIntervalMs() {
        return intervalMs;
    }

    /**
     * Get the maximum number of missed pong responses before disconnect.
     *
     * @return Maximum missed pongs, or null if using default (2)
     */
    public Long getMaxMissedPongs() {
        return maxMissedPongs;
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
        private Long intervalMs;
        private Long maxMissedPongs;

        private Builder() {}

        /**
         * Set whether health checks are enabled.
         *
         * @param enabled True to enable health checks (default: false)
         * @return This builder for chaining
         */
        public Builder enabled(Boolean enabled) {
            this.enabled = enabled;
            return this;
        }

        /**
         * Set the interval in milliseconds between ping messages.
         *
         * @param intervalMs Interval in milliseconds (default: 30000)
         * @return This builder for chaining
         */
        public Builder intervalMs(Long intervalMs) {
            this.intervalMs = intervalMs;
            return this;
        }

        /**
         * Set the maximum number of missed pong responses before disconnect.
         *
         * @param maxMissedPongs Maximum missed pongs (default: 2)
         * @return This builder for chaining
         */
        public Builder maxMissedPongs(Long maxMissedPongs) {
            this.maxMissedPongs = maxMissedPongs;
            return this;
        }

        /**
         * Build the immutable HealthCheckOptions instance.
         *
         * <p>Validation is performed by the client builder, not here.
         *
         * @return Immutable HealthCheckOptions instance
         */
        public HealthCheckOptions build() {
            return new HealthCheckOptions(enabled, intervalMs, maxMissedPongs);
        }
    }
}
