package tw.com.fugle.marketdata;

import tw.com.fugle.marketdata.generated.MarketDataException;

import java.util.concurrent.CompletableFuture;
import java.util.concurrent.CompletionException;
import java.util.concurrent.ExecutionException;

/**
 * Base exception for all Fugle MarketData SDK errors.
 *
 * This is an unchecked exception (extends RuntimeException) following
 * the decision from CONTEXT.md to match C# binding behavior.
 */
public class FugleException extends RuntimeException {

    public FugleException(String message) {
        super(message);
    }

    public FugleException(String message, Throwable cause) {
        super(message, cause);
    }

    public FugleException(Throwable cause) {
        super(cause);
    }

    /**
     * Convert a generated MarketDataException to the appropriate FugleException subclass.
     *
     * This method maps the flat UniFFI exception hierarchy to our two-level hierarchy:
     * - FugleException (base)
     *   - ApiException (API-specific errors including rate limits)
     *   - AuthException (authentication errors)
     *
     * @param e The generated exception to convert
     * @return The appropriate FugleException subclass
     */
    public static FugleException from(MarketDataException e) {
        if (e instanceof MarketDataException.AuthException) {
            return new AuthException(e.getMessage(), e);
        } else if (e instanceof MarketDataException.RateLimitException) {
            MarketDataException.RateLimitException rle = (MarketDataException.RateLimitException) e;
            return new RateLimitException(rle.msg(), e);
        } else if (e instanceof MarketDataException.ApiException ||
                   e instanceof MarketDataException.NetworkException ||
                   e instanceof MarketDataException.InvalidSymbol ||
                   e instanceof MarketDataException.ParseException ||
                   e instanceof MarketDataException.TimeoutException ||
                   e instanceof MarketDataException.ConfigException) {
            return new ApiException(e.getMessage(), e);
        } else {
            // Other, ClientClosed, WebSocketException, etc.
            return new FugleException(e.getMessage(), e);
        }
    }

    /**
     * Unwrap a CompletableFuture exception and convert to FugleException.
     *
     * This helper method extracts the underlying MarketDataException from
     * CompletionException or ExecutionException wrappers and converts it
     * to the appropriate FugleException subclass.
     *
     * @param e The exception from a CompletableFuture operation
     * @return The appropriate FugleException subclass
     */
    public static FugleException unwrap(Throwable e) {
        Throwable cause = e;

        // Unwrap CompletionException and ExecutionException
        if (e instanceof CompletionException || e instanceof ExecutionException) {
            cause = e.getCause();
        }

        // Convert MarketDataException to FugleException
        if (cause instanceof MarketDataException) {
            return from((MarketDataException) cause);
        }

        // If it's already a FugleException, return as-is
        if (cause instanceof FugleException) {
            return (FugleException) cause;
        }

        // Otherwise wrap in generic FugleException
        return new FugleException(cause);
    }
}
