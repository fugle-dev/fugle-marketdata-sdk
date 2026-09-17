package tw.com.fugle.marketdata;

import tw.com.fugle.marketdata.generated.ErrorSourceKind;
import org.junit.jupiter.api.*;
import static org.junit.jupiter.api.Assertions.*;

/**
 * Tests for configuration option classes and client builder auth validation.
 *
 * <p>These tests verify builder patterns, auth validation logic, and config acceptance.
 * Tests involving actual client construction will throw exceptions from UniFFI layer
 * (no real API key), which we catch to verify validation happened correctly.
 */
public class ConfigOptionsTest {

    // ========== ReconnectOptions Tests ==========

    @Test
    @DisplayName("ReconnectOptions builder with custom values")
    void testReconnectOptionsBuilder() {
        ReconnectOptions options = ReconnectOptions.builder()
            .maxAttempts(10)
            .initialDelayMs(2000L)
            .maxDelayMs(120000L)
            .build();

        assertNotNull(options);
        assertEquals(Integer.valueOf(10), options.getMaxAttempts());
        assertEquals(Long.valueOf(2000L), options.getInitialDelayMs());
        assertEquals(Long.valueOf(120000L), options.getMaxDelayMs());
    }

    @Test
    @DisplayName("ReconnectOptions builder with no values (defaults)")
    void testReconnectOptionsDefaults() {
        ReconnectOptions options = ReconnectOptions.builder().build();

        assertNotNull(options);
        assertNull(options.getMaxAttempts());
        assertNull(options.getInitialDelayMs());
        assertNull(options.getMaxDelayMs());
    }

    // ========== HealthCheckOptions Tests ==========

    @Test
    @DisplayName("HealthCheckOptions builder with custom values")
    void testHealthCheckOptionsBuilder() {
        HealthCheckOptions options = HealthCheckOptions.builder()
            .enabled(true)
            .heartbeatTimeoutMs(60000L)
            .build();

        assertNotNull(options);
        assertEquals(Boolean.TRUE, options.getEnabled());
        assertEquals(Long.valueOf(60000L), options.getHeartbeatTimeoutMs());
    }

    @Test
    @DisplayName("HealthCheckOptions builder with no values (defaults)")
    void testHealthCheckOptionsDefaults() {
        HealthCheckOptions options = HealthCheckOptions.builder().build();

        assertNotNull(options);
        assertNull(options.getEnabled());
        assertNull(options.getHeartbeatTimeoutMs());
    }

    // ========== RestClient Exactly-One-Auth Tests ==========

    @Test
    @DisplayName("RestClient with apiKey alone works (no auth validation error)")
    void testRestClientExactlyOneAuth_apiKey() {
        NativeLibrary.assumeAvailable();

        try {
            FugleRestClient client = FugleRestClient.builder()
                .apiKey("test-api-key")
                .build();

            // If we get here, auth validation passed
            // The actual UniFFI call will fail (no real API key), but that's expected
            client.close();
        } catch (FugleException e) {
            // Verify this is NOT from auth validation
            assertFalse(Integer.valueOf(1004).equals(e.getCode()), "Should not be an auth validation error");
        }
    }

    @Test
    @DisplayName("RestClient with bearerToken alone works (no auth validation error)")
    void testRestClientExactlyOneAuth_bearerToken() {
        NativeLibrary.assumeAvailable();

        try {
            FugleRestClient client = FugleRestClient.builder()
                .bearerToken("test-bearer-token")
                .build();

            client.close();
        } catch (FugleException e) {
            assertFalse(Integer.valueOf(1004).equals(e.getCode()), "Should not be an auth validation error");
        }
    }

    @Test
    @DisplayName("RestClient with sdkToken alone works (no auth validation error)")
    void testRestClientExactlyOneAuth_sdkToken() {
        NativeLibrary.assumeAvailable();

        try {
            FugleRestClient client = FugleRestClient.builder()
                .sdkToken("test-sdk-token")
                .build();

            client.close();
        } catch (FugleException e) {
            assertFalse(Integer.valueOf(1004).equals(e.getCode()), "Should not be an auth validation error");
        }
    }

    @Test
    @DisplayName("RestClient with no auth throws FugleException with code 1004")
    void testRestClientNoAuth() {
        NativeLibrary.assumeAvailable();

        assertCredentialsRejected(() -> FugleRestClient.builder().build());
    }

    @Test
    @DisplayName("RestClient with only empty or whitespace auth throws FugleException with code 1004")
    void testRestClientBlankAuth() {
        NativeLibrary.assumeAvailable();

        assertCredentialsRejected(() -> FugleRestClient.builder().apiKey("").build());
        assertCredentialsRejected(() -> FugleRestClient.builder().bearerToken("   ").build());
        assertCredentialsRejected(() -> FugleRestClient.builder().apiKey(" ").sdkToken("").build());
    }

    @Test
    @DisplayName("RestClient ignores a blank auth next to a real one")
    void testRestClientBlankAuthNextToRealOne() {
        NativeLibrary.assumeAvailable();

        try (FugleRestClient client = FugleRestClient.builder()
                .apiKey("  ")
                .sdkToken("test-sdk-token")
                .build()) {
            assertNotNull(client);
        }
    }

    @Test
    @DisplayName("RestClient with multiple auth methods throws FugleException with code 1004")
    void testRestClientMultipleAuth() {
        NativeLibrary.assumeAvailable();

        assertCredentialsRejected(() -> FugleRestClient.builder()
            .apiKey("test-api-key")
            .bearerToken("test-bearer-token")
            .build());
    }

    // ========== WebSocketClient Exactly-One-Auth Tests ==========

    @Test
    @DisplayName("WebSocketClient with apiKey alone works (no auth validation error)")
    void testWebSocketExactlyOneAuth_apiKey() {
        NativeLibrary.assumeAvailable();

        try {
            FugleWebSocketClient client = FugleWebSocketClient.builder()
                .apiKey("test-api-key")
                .stock()
                .build();

            // Auth validation passed, actual connection will fail (expected)
            client.close();
        } catch (FugleException e) {
            assertFalse(Integer.valueOf(1004).equals(e.getCode()), "Should not be an auth validation error");
        }
    }

    @Test
    @DisplayName("WebSocketClient with no or blank auth throws FugleException with code 1004")
    void testWebSocketNoAuth() {
        NativeLibrary.assumeAvailable();

        assertCredentialsRejected(() -> FugleWebSocketClient.builder().stock().build());
        assertCredentialsRejected(() -> FugleWebSocketClient.builder().apiKey("  ").stock().build());
    }

    @Test
    @DisplayName("WebSocketClient with multiple auth methods throws FugleException with code 1004")
    void testWebSocketMultipleAuth() {
        NativeLibrary.assumeAvailable();

        assertCredentialsRejected(() -> FugleWebSocketClient.builder()
            .apiKey("test-api-key")
            .bearerToken("test-bearer-token")
            .stock()
            .build());
    }

    // ========== WebSocketClient Config Options Tests ==========

    @Test
    @DisplayName("WebSocketClient builder accepts ReconnectOptions without error")
    void testWebSocketWithReconnectOptions() {
        NativeLibrary.assumeAvailable();

        ReconnectOptions reconnect = ReconnectOptions.builder()
            .maxAttempts(10)
            .initialDelayMs(2000L)
            .build();

        try {
            FugleWebSocketClient client = FugleWebSocketClient.builder()
                .apiKey("test-api-key")
                .stock()
                .reconnect(reconnect)
                .build();

            // If we get here, builder accepted ReconnectOptions
            client.close();
        } catch (FugleException e) {
            // Verify this is NOT from config acceptance
            assertFalse(e.getMessage().contains("reconnect"),
                "Should not reject ReconnectOptions");
        }
    }

    @Test
    @DisplayName("WebSocketClient builder accepts HealthCheckOptions without error")
    void testWebSocketWithHealthCheckOptions() {
        NativeLibrary.assumeAvailable();

        HealthCheckOptions healthCheck = HealthCheckOptions.builder()
            .enabled(true)
            .heartbeatTimeoutMs(60000L)
            .build();

        try {
            FugleWebSocketClient client = FugleWebSocketClient.builder()
                .apiKey("test-api-key")
                .stock()
                .healthCheck(healthCheck)
                .build();

            // If we get here, builder accepted HealthCheckOptions
            client.close();
        } catch (FugleException e) {
            // Verify this is NOT from config acceptance
            assertFalse(e.getMessage().contains("health"),
                "Should not reject HealthCheckOptions");
        }
    }

    // ========== WebSocketClient Message Queue Options Tests ==========

    @Test
    @DisplayName("WebSocketClient builder accepts messageOverflow(DROP_NEWEST) without error")
    void testWebSocketWithMessageOverflowDropNewest() {
        NativeLibrary.assumeAvailable();

        try {
            FugleWebSocketClient client = FugleWebSocketClient.builder()
                .apiKey("test-api-key")
                .stock()
                .messageOverflow(MessageOverflow.DROP_NEWEST)
                .build();

            client.close();
        } catch (FugleException e) {
            assertFalse(e.getMessage().contains("message"),
                "Should not reject messageOverflow(DROP_NEWEST)");
        }
    }

    @Test
    @DisplayName("WebSocketClient builder accepts messageOverflow(UNBOUNDED) without error")
    void testWebSocketWithMessageOverflowUnbounded() {
        NativeLibrary.assumeAvailable();

        try {
            FugleWebSocketClient client = FugleWebSocketClient.builder()
                .apiKey("test-api-key")
                .stock()
                .messageOverflow(MessageOverflow.UNBOUNDED)
                .build();

            client.close();
        } catch (FugleException e) {
            assertFalse(e.getMessage().contains("message"),
                "Should not reject messageOverflow(UNBOUNDED)");
        }
    }

    @Test
    @DisplayName("WebSocketClient builder accepts messageBuffer without error")
    void testWebSocketWithMessageBuffer() {
        NativeLibrary.assumeAvailable();

        try {
            FugleWebSocketClient client = FugleWebSocketClient.builder()
                .apiKey("test-api-key")
                .stock()
                .messageBuffer(256)
                .build();

            client.close();
        } catch (FugleException e) {
            assertFalse(e.getMessage().contains("message"),
                "Should not reject messageBuffer");
        }
    }

    @Test
    @DisplayName("WebSocketClient builder leaves messageOverflow/messageBuffer unset by default")
    void testWebSocketMessageQueueDefaults() {
        NativeLibrary.assumeAvailable();

        // No messageOverflow()/messageBuffer() call: the builder must pass
        // null through to newWithOptions so the core defaults (DropNewest,
        // 4096) apply. This just verifies building succeeds without them.
        try (FugleWebSocketClient client = FugleWebSocketClient.builder()
                .apiKey("test-api-key")
                .stock()
                .build()) {
            assertNotNull(client);
        }
    }

    @Test
    @DisplayName("WebSocketClient builder rejects messageBuffer(0)")
    void testWebSocketMessageBufferRejectsZero() {
        IllegalArgumentException exception = assertThrows(IllegalArgumentException.class, () ->
            FugleWebSocketClient.builder().messageBuffer(0)
        );
        assertTrue(exception.getMessage().contains("messageBuffer"));
    }

    @Test
    @DisplayName("WebSocketClient builder rejects negative messageBuffer")
    void testWebSocketMessageBufferRejectsNegative() {
        IllegalArgumentException exception = assertThrows(IllegalArgumentException.class, () ->
            FugleWebSocketClient.builder().messageBuffer(-1)
        );
        assertTrue(exception.getMessage().contains("messageBuffer"));
    }

    /** Credential errors come from core as a ConfigError (code 1004). */
    private static void assertCredentialsRejected(org.junit.jupiter.api.function.Executable build) {
        FugleException exception = assertThrows(FugleException.class, build);
        assertEquals(Integer.valueOf(1004), exception.getCode());
        assertEquals(ErrorSourceKind.CLIENT, exception.getSourceKind());
        assertTrue(exception.getMessage().contains("exactly one non-empty credential"),
            "Error message should indicate exactly-one-auth requirement: " + exception.getMessage());
    }
}
