package tw.com.fugle.marketdata;

/**
 * Exception thrown for authentication errors.
 *
 * This occurs when the API key, bearer token, or SDK token is
 * invalid, expired, or missing required permissions.
 *
 * Extends FugleException as an unchecked exception.
 */
public class AuthException extends FugleException {

    public AuthException(String message) {
        super(message);
    }

    public AuthException(String message, Throwable cause) {
        super(message, cause);
    }

    public AuthException(Throwable cause) {
        super(cause);
    }
}
