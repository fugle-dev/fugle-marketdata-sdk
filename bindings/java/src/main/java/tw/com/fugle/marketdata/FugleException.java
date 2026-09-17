package tw.com.fugle.marketdata;

import tw.com.fugle.marketdata.generated.ErrorInfo;
import tw.com.fugle.marketdata.generated.ErrorSourceKind;
import tw.com.fugle.marketdata.generated.MarketDataException;

import java.util.Collections;
import java.util.Map;
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

    /**
     * The unified cross-language error fields (code, source kind, HTTP
     * details), or null when this exception did not come from a generated
     * {@link MarketDataException} (e.g. it was constructed directly).
     */
    private final ErrorInfo info;

    public FugleException(String message) {
        super(message);
        this.info = null;
    }

    public FugleException(String message, Throwable cause) {
        super(message, cause);
        this.info = null;
    }

    public FugleException(Throwable cause) {
        super(cause);
        this.info = null;
    }

    public FugleException(String message, ErrorInfo info) {
        super(message);
        this.info = info;
    }

    public FugleException(String message, ErrorInfo info, Throwable cause) {
        super(message, cause);
        this.info = info;
    }

    /**
     * The unified cross-language error info, or null when unavailable.
     *
     * @return the error info, or null
     */
    public ErrorInfo getInfo() {
        return info;
    }

    /**
     * Numeric error code from the unified error spec, or null when
     * {@link #getInfo()} is null.
     *
     * @return the error code, or null
     */
    public Integer getCode() {
        return info == null ? null : info.code();
    }

    /**
     * Coarse-grained classification of the error source, or null when
     * {@link #getInfo()} is null.
     *
     * @return the source kind, or null
     */
    public ErrorSourceKind getSourceKind() {
        return info == null ? null : info.sourceKind();
    }

    /**
     * HTTP status, when the error came from an HTTP response, or null.
     *
     * @return the HTTP status, or null
     */
    public Integer getStatus() {
        // The generated record holds the unsigned u16 in a Short.
        Short status = info == null ? null : info.status();
        return status == null ? null : Short.toUnsignedInt(status);
    }

    /**
     * Raw HTTP response body (REST only), or null.
     *
     * @return the response body, or null
     */
    public String getBody() {
        return info == null ? null : info.body();
    }

    /**
     * Server-assigned request id (`x-request-id`), or null when absent.
     *
     * @return the request id, or null
     */
    public String getRequestId() {
        return info == null ? null : info.requestId();
    }

    /**
     * HTTP response headers (REST only); an empty map when
     * {@link #getInfo()} is null or the error carries no headers.
     *
     * @return the response headers, never null
     */
    public Map<String, String> getHeaders() {
        return info == null ? Collections.emptyMap() : info.headers();
    }

    /**
     * Extract the {@link ErrorInfo} carried by a generated
     * {@link MarketDataException} variant.
     *
     * @param e The generated exception
     * @return The error info, or null if the variant carries none (should
     *         not happen post-0.2.0-rc.2, every variant carries one)
     */
    private static ErrorInfo extractInfo(MarketDataException e) {
        if (e instanceof MarketDataException.NetworkException) {
            return ((MarketDataException.NetworkException) e).info();
        } else if (e instanceof MarketDataException.AuthException) {
            return ((MarketDataException.AuthException) e).info();
        } else if (e instanceof MarketDataException.RateLimitException) {
            return ((MarketDataException.RateLimitException) e).info();
        } else if (e instanceof MarketDataException.InvalidSymbol) {
            return ((MarketDataException.InvalidSymbol) e).info();
        } else if (e instanceof MarketDataException.ParseException) {
            return ((MarketDataException.ParseException) e).info();
        } else if (e instanceof MarketDataException.TimeoutException) {
            return ((MarketDataException.TimeoutException) e).info();
        } else if (e instanceof MarketDataException.WebSocketException) {
            return ((MarketDataException.WebSocketException) e).info();
        } else if (e instanceof MarketDataException.ClientClosed) {
            return ((MarketDataException.ClientClosed) e).info();
        } else if (e instanceof MarketDataException.ConfigException) {
            return ((MarketDataException.ConfigException) e).info();
        } else if (e instanceof MarketDataException.ApiException) {
            return ((MarketDataException.ApiException) e).info();
        } else if (e instanceof MarketDataException.Other) {
            return ((MarketDataException.Other) e).info();
        }
        return null;
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
        ErrorInfo info = extractInfo(e);
        if (e instanceof MarketDataException.AuthException) {
            return new AuthException(e.getMessage(), info, e);
        } else if (e instanceof MarketDataException.RateLimitException) {
            MarketDataException.RateLimitException rle = (MarketDataException.RateLimitException) e;
            return new RateLimitException(rle.msg(), info, e);
        } else if (e instanceof MarketDataException.ApiException ||
                   e instanceof MarketDataException.NetworkException ||
                   e instanceof MarketDataException.InvalidSymbol ||
                   e instanceof MarketDataException.ParseException ||
                   e instanceof MarketDataException.TimeoutException ||
                   e instanceof MarketDataException.ConfigException) {
            return new ApiException(e.getMessage(), info, e);
        } else {
            // Other, ClientClosed, WebSocketException, etc.
            return new FugleException(e.getMessage(), info, e);
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
