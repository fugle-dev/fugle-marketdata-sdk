package tw.com.fugle.marketdata;

/**
 * Exception thrown for API-level errors.
 *
 * This includes network errors, invalid symbols, parsing errors,
 * timeouts, configuration errors, and general API errors.
 *
 * Extends FugleException as an unchecked exception.
 */
public class ApiException extends FugleException {

    public ApiException(String message) {
        super(message);
    }

    public ApiException(String message, Throwable cause) {
        super(message, cause);
    }

    public ApiException(Throwable cause) {
        super(cause);
    }
}
