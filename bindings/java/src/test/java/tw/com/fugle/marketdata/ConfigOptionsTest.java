package tw.com.fugle.marketdata;

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
            .intervalMs(60000L)
            .maxMissedPongs(3L)
            .build();

        assertNotNull(options);
        assertEquals(Boolean.TRUE, options.getEnabled());
        assertEquals(Long.valueOf(60000L), options.getIntervalMs());
        assertEquals(Long.valueOf(3L), options.getMaxMissedPongs());
    }

    @Test
    @DisplayName("HealthCheckOptions builder with no values (defaults)")
    void testHealthCheckOptionsDefaults() {
        HealthCheckOptions options = HealthCheckOptions.builder().build();

        assertNotNull(options);
        assertNull(options.getEnabled());
        assertNull(options.getIntervalMs());
        assertNull(options.getMaxMissedPongs());
    }

    // ========== RestClient Exactly-One-Auth Tests ==========

    @Test
    @DisplayName("RestClient with apiKey alone works (no auth validation error)")
    void testRestClientExactlyOneAuth_apiKey() {
        try {
            FugleRestClient client = FugleRestClient.builder()
                .apiKey("test-api-key")
                .build();

            // If we get here, auth validation passed
            // The actual UniFFI call will fail (no real API key), but that's expected
            client.close();
        } catch (FugleException e) {
            // Verify this is NOT from auth validation
            assertFalse(e.getMessage().contains("Provide exactly one of"),
                "Should not be an auth validation error");
        }
    }

    @Test
    @DisplayName("RestClient with bearerToken alone works (no auth validation error)")
    void testRestClientExactlyOneAuth_bearerToken() {
        try {
            FugleRestClient client = FugleRestClient.builder()
                .bearerToken("test-bearer-token")
                .build();

            client.close();
        } catch (FugleException e) {
            assertFalse(e.getMessage().contains("Provide exactly one of"),
                "Should not be an auth validation error");
        }
    }

    @Test
    @DisplayName("RestClient with sdkToken alone works (no auth validation error)")
    void testRestClientExactlyOneAuth_sdkToken() {
        try {
            FugleRestClient client = FugleRestClient.builder()
                .sdkToken("test-sdk-token")
                .build();

            client.close();
        } catch (FugleException e) {
            assertFalse(e.getMessage().contains("Provide exactly one of"),
                "Should not be an auth validation error");
        }
    }

    @Test
    @DisplayName("RestClient with no auth throws FugleException with correct message")
    void testRestClientNoAuth() {
        FugleException exception = assertThrows(FugleException.class, () -> {
            FugleRestClient.builder().build();
        });

        assertTrue(exception.getMessage().contains("Provide exactly one of"),
            "Error message should indicate exactly-one-auth requirement");
        assertTrue(exception.getMessage().contains("apiKey"),
            "Error message should list apiKey");
        assertTrue(exception.getMessage().contains("bearerToken"),
            "Error message should list bearerToken");
        assertTrue(exception.getMessage().contains("sdkToken"),
            "Error message should list sdkToken");
    }

    @Test
    @DisplayName("RestClient with multiple auth methods throws FugleException")
    void testRestClientMultipleAuth() {
        FugleException exception = assertThrows(FugleException.class, () -> {
            FugleRestClient.builder()
                .apiKey("test-api-key")
                .bearerToken("test-bearer-token")
                .build();
        });

        assertTrue(exception.getMessage().contains("Provide exactly one of"),
            "Error message should indicate exactly-one-auth requirement");
    }

    // ========== WebSocketClient Exactly-One-Auth Tests ==========

    @Test
    @DisplayName("WebSocketClient with apiKey alone works (no auth validation error)")
    void testWebSocketExactlyOneAuth_apiKey() {
        try {
            FugleWebSocketClient client = FugleWebSocketClient.builder()
                .apiKey("test-api-key")
                .stock()
                .build();

            // Auth validation passed, actual connection will fail (expected)
            client.close();
        } catch (FugleException e) {
            assertFalse(e.getMessage().contains("Provide exactly one of"),
                "Should not be an auth validation error");
        }
    }

    @Test
    @DisplayName("WebSocketClient with no auth throws FugleException with correct message")
    void testWebSocketNoAuth() {
        FugleException exception = assertThrows(FugleException.class, () -> {
            FugleWebSocketClient.builder()
                .stock()
                .build();
        });

        assertTrue(exception.getMessage().contains("Provide exactly one of"),
            "Error message should indicate exactly-one-auth requirement");
    }

    @Test
    @DisplayName("WebSocketClient with multiple auth methods throws FugleException")
    void testWebSocketMultipleAuth() {
        FugleException exception = assertThrows(FugleException.class, () -> {
            FugleWebSocketClient.builder()
                .apiKey("test-api-key")
                .bearerToken("test-bearer-token")
                .stock()
                .build();
        });

        assertTrue(exception.getMessage().contains("Provide exactly one of"),
            "Error message should indicate exactly-one-auth requirement");
    }

    // ========== WebSocketClient Config Options Tests ==========

    @Test
    @DisplayName("WebSocketClient builder accepts ReconnectOptions without error")
    void testWebSocketWithReconnectOptions() {
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
        HealthCheckOptions healthCheck = HealthCheckOptions.builder()
            .enabled(true)
            .intervalMs(60000L)
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
}
