package tw.com.fugle.marketdata;

/**
 * Configuration options for WebSocket reconnection behavior.
 *
 * <p>This immutable class defines reconnection parameters using the builder pattern.
 * All fields are optional - null values indicate the default should be used by the client.
 *
 * <p><b>Default values:</b>
 * <ul>
 *   <li>maxAttempts: 5 (maximum reconnection attempts)</li>
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

    private final Integer maxAttempts;
    private final Long initialDelayMs;
    private final Long maxDelayMs;

    private ReconnectOptions(Integer maxAttempts, Long initialDelayMs, Long maxDelayMs) {
        this.maxAttempts = maxAttempts;
        this.initialDelayMs = initialDelayMs;
        this.maxDelayMs = maxDelayMs;
    }

    /**
     * Get the maximum number of reconnection attempts.
     *
     * @return Maximum attempts, or null if using default (5)
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
        private Integer maxAttempts;
        private Long initialDelayMs;
        private Long maxDelayMs;

        private Builder() {}

        /**
         * Set the maximum number of reconnection attempts.
         *
         * @param maxAttempts Maximum attempts (default: 5)
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
            return new ReconnectOptions(maxAttempts, initialDelayMs, maxDelayMs);
        }
    }
}
