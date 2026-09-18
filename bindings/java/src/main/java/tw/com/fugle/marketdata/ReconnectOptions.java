package tw.com.fugle.marketdata;

/**
 * Configuration options for WebSocket reconnection behavior.
 *
 * <p>This immutable class defines reconnection parameters using the builder pattern.
 * All fields are optional - null values indicate the default should be used by the client.
 * The client auto-reconnects when no options are given; set {@code enabled(false)} to turn
 * it off.
 *
 * <p><b>Default values:</b>
 * <ul>
 *   <li>enabled: true</li>
 *   <li>maxAttempts: 0 (unlimited reconnection attempts)</li>
 *   <li>initialDelayMs: 1000 (starting delay in milliseconds)</li>
 *   <li>maxDelayMs: 60000 (maximum delay cap in milliseconds)</li>
 * </ul>
 *
 * <p><b>Example usage:</b>
 * <pre>{@code
 * ReconnectOptions options = ReconnectOptions.builder()
 *     .maxAttempts(10)
 *     .initialDelayMs(2000L)
 *     .maxDelayMs(120000L)
 *     .build();
 * }</pre>
 */
public final class ReconnectOptions {

    private final Boolean enabled;
    private final Integer maxAttempts;
    private final Long initialDelayMs;
    private final Long maxDelayMs;

    private ReconnectOptions(Boolean enabled, Integer maxAttempts, Long initialDelayMs, Long maxDelayMs) {
        this.enabled = enabled;
        this.maxAttempts = maxAttempts;
        this.initialDelayMs = initialDelayMs;
        this.maxDelayMs = maxDelayMs;
    }

    /**
     * Get whether auto-reconnect is enabled.
     *
     * @return Whether auto-reconnect is enabled, or null if using default (true)
     */
    public Boolean getEnabled() {
        return enabled;
    }

    /**
     * Get the maximum number of reconnection attempts.
     *
     * @return Maximum attempts (0 = unlimited), or null if using default (0)
     */
    public Integer getMaxAttempts() {
        return maxAttempts;
    }

    /**
     * Get the initial delay in milliseconds before first reconnection attempt.
     *
     * @return Initial delay in milliseconds, or null if using default (1000)
     */
    public Long getInitialDelayMs() {
        return initialDelayMs;
    }

    /**
     * Get the maximum delay cap in milliseconds between reconnection attempts.
     *
     * @return Maximum delay in milliseconds, or null if using default (60000)
     */
    public Long getMaxDelayMs() {
        return maxDelayMs;
    }

    /**
     * Create a new builder for constructing ReconnectOptions.
     *
     * @return A new builder instance
     */
    public static Builder builder() {
        return new Builder();
    }

    /**
     * Builder for creating immutable ReconnectOptions instances.
     */
    public static class Builder {
        private Boolean enabled;
        private Integer maxAttempts;
        private Long initialDelayMs;
        private Long maxDelayMs;

        private Builder() {}

        /**
         * Set whether auto-reconnect is enabled.
         *
         * @param enabled {@code false} turns auto-reconnect off (default: true)
         * @return This builder for chaining
         */
        public Builder enabled(Boolean enabled) {
            this.enabled = enabled;
            return this;
        }

        /**
         * Set the maximum number of reconnection attempts.
         *
         * @param maxAttempts Maximum attempts; 0 means unlimited (default: 0)
         * @return This builder for chaining
         */
        public Builder maxAttempts(Integer maxAttempts) {
            this.maxAttempts = maxAttempts;
            return this;
        }

        /**
         * Set the initial delay in milliseconds before first reconnection attempt.
         *
         * @param initialDelayMs Initial delay in milliseconds (default: 1000)
         * @return This builder for chaining
         */
        public Builder initialDelayMs(Long initialDelayMs) {
            this.initialDelayMs = initialDelayMs;
            return this;
        }

        /**
         * Set the maximum delay cap in milliseconds between reconnection attempts.
         *
         * @param maxDelayMs Maximum delay in milliseconds (default: 60000)
         * @return This builder for chaining
         */
        public Builder maxDelayMs(Long maxDelayMs) {
            this.maxDelayMs = maxDelayMs;
            return this;
        }

        /**
         * Build the immutable ReconnectOptions instance.
         *
         * <p>Validation is performed by the client builder, not here.
         *
         * @return Immutable ReconnectOptions instance
         */
        public ReconnectOptions build() {
            return new ReconnectOptions(enabled, maxAttempts, initialDelayMs, maxDelayMs);
        }
    }
}
