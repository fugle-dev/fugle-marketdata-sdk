package tw.com.fugle.marketdata;

import org.junit.jupiter.api.*;
import static org.junit.jupiter.api.Assertions.*;

import java.lang.reflect.Method;

/**
 * Tests for Fugle exception hierarchy.
 *
 * <p>All tests are structural (using reflection) and pass without native library.
 */
public class ExceptionTest {

    // ========== Type Hierarchy Tests ==========

    @Test
    @DisplayName("FugleException is a RuntimeException")
    void fugleExceptionIsRuntimeException() {
        assertNotNull(FugleException.class);
        assertTrue(RuntimeException.class.isAssignableFrom(FugleException.class));
    }

    @Test
    @DisplayName("ApiException extends FugleException")
    void apiExceptionExtendsFugleException() {
        assertNotNull(ApiException.class);
        assertTrue(FugleException.class.isAssignableFrom(ApiException.class));
    }

    @Test
    @DisplayName("RateLimitException extends ApiException")
    void rateLimitExceptionExtendsApiException() {
        assertNotNull(RateLimitException.class);
        assertTrue(ApiException.class.isAssignableFrom(RateLimitException.class));
    }

    @Test
    @DisplayName("AuthException extends FugleException")
    void authExceptionExtendsFugleException() {
        assertNotNull(AuthException.class);
        assertTrue(FugleException.class.isAssignableFrom(AuthException.class));
    }

    // ========== Constructor Tests ==========

    @Test
    @DisplayName("FugleException has message constructor")
    void fugleExceptionHasMessageConstructor() throws NoSuchMethodException {
        assertNotNull(FugleException.class.getConstructor(String.class));
    }

    @Test
    @DisplayName("FugleException has message and cause constructor")
    void fugleExceptionHasMessageCauseConstructor() throws NoSuchMethodException {
        assertNotNull(FugleException.class.getConstructor(String.class, Throwable.class));
    }

    @Test
    @DisplayName("ApiException has message and statusCode constructor")
    void apiExceptionHasConstructor() throws NoSuchMethodException {
        assertNotNull(ApiException.class.getConstructor(String.class, int.class));
    }

    @Test
    @DisplayName("RateLimitException has constructor with retryAfter")
    void rateLimitExceptionHasConstructor() throws NoSuchMethodException {
        assertNotNull(RateLimitException.class.getConstructor(String.class, int.class, Long.class));
    }

    @Test
    @DisplayName("AuthException has message constructor")
    void authExceptionHasMessageConstructor() throws NoSuchMethodException {
        assertNotNull(AuthException.class.getConstructor(String.class));
    }

    // ========== Method Tests ==========

    @Test
    @DisplayName("FugleException has from() static method")
    void fugleExceptionHasFromMethod() throws NoSuchMethodException {
        Method method = FugleException.class.getMethod("from", Throwable.class);
        assertNotNull(method);
        assertEquals(RuntimeException.class, method.getReturnType());
    }

    @Test
    @DisplayName("FugleException has unwrap() static method")
    void fugleExceptionHasUnwrapMethod() throws NoSuchMethodException {
        Method method = FugleException.class.getMethod("unwrap", Throwable.class);
        assertNotNull(method);
        assertEquals(FugleException.class, method.getReturnType());
    }

    // ========== Behavior Tests ==========

    @Test
    @DisplayName("ApiException stores message")
    void apiExceptionStoresMessage() {
        ApiException ex = new ApiException("Not found");
        assertEquals("Not found", ex.getMessage());
    }

    @Test
    @DisplayName("ApiException stores cause")
    void apiExceptionStoresCause() {
        Exception cause = new Exception("root");
        ApiException ex = new ApiException("wrapped", cause);
        assertEquals("wrapped", ex.getMessage());
        assertEquals(cause, ex.getCause());
    }

    @Test
    @DisplayName("AuthException preserves message")
    void authExceptionPreservesMessage() {
        AuthException ex = new AuthException("Invalid API key");
        assertEquals("Invalid API key", ex.getMessage());
    }

    @Test
    @DisplayName("FugleException.unwrap() wraps generic exceptions")
    void unwrapWrapsGenericExceptions() {
        Exception cause = new Exception("Original error");
        FugleException wrapped = FugleException.unwrap(cause);

        assertInstanceOf(FugleException.class, wrapped);
        assertEquals(cause, wrapped.getCause());
    }

    @Test
    @DisplayName("FugleException.unwrap() preserves FugleException")
    void unwrapPreservesFugleException() {
        FugleException original = new FugleException("Already wrapped");
        FugleException result = FugleException.unwrap(original);

        assertSame(original, result);
    }

    @Test
    @DisplayName("FugleException.unwrap() wraps RuntimeException subclasses")
    void unwrapWrapsRuntimeException() {
        IllegalArgumentException original = new IllegalArgumentException("Invalid arg");
        FugleException result = FugleException.unwrap(original);

        assertInstanceOf(FugleException.class, result);
    }

    @Test
    @DisplayName("FugleException hierarchy is serializable")
    void exceptionsAreSerializable() {
        assertTrue(java.io.Serializable.class.isAssignableFrom(FugleException.class));
        assertTrue(java.io.Serializable.class.isAssignableFrom(ApiException.class));
        assertTrue(java.io.Serializable.class.isAssignableFrom(RateLimitException.class));
        assertTrue(java.io.Serializable.class.isAssignableFrom(AuthException.class));
    }
}
