package tw.com.fugle.marketdata;

/**
 * Exception thrown when API rate limits are exceeded.
 *
 * This exception extends ApiException and provides additional
 * information about when the client can retry the request.
 *
 * Note: The generated MarketDataException.RateLimitException does not
 * currently provide retry-after information, so getRetryAfterSeconds()
 * returns null. This matches the pattern from C# where RetryAfterSeconds
 * is nullable and extracted from HTTP headers if available.
 */
public class RateLimitException extends ApiException {

    private final Integer retryAfterSeconds;

    public RateLimitException(String message) {
        super(message);
        this.retryAfterSeconds = null;
    }

    public RateLimitException(String message, Throwable cause) {
        super(message, cause);
        this.retryAfterSeconds = null;
    }

    public RateLimitException(String message, Integer retryAfterSeconds) {
        super(message);
        this.retryAfterSeconds = retryAfterSeconds;
    }

    public RateLimitException(String message, Integer retryAfterSeconds, Throwable cause) {
        super(message, cause);
        this.retryAfterSeconds = retryAfterSeconds;
    }

    /**
     * Get the number of seconds to wait before retrying.
     *
     * @return The retry delay in seconds, or null if not available
     */
    public Integer getRetryAfterSeconds() {
        return retryAfterSeconds;
    }
}
