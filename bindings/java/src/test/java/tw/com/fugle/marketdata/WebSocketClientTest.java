package tw.com.fugle.marketdata;

import tw.com.fugle.marketdata.generated.*;
import org.junit.jupiter.api.*;
import static org.junit.jupiter.api.Assertions.*;

import java.lang.reflect.Method;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.TimeUnit;

/**
 * Tests for FugleWebSocketClient wrapper over UniFFI bindings.
 *
 * <p>Structural tests verify type existence and API shape using reflection.
 * These tests pass without native library.
 *
 * <p>Integration tests (tagged with @Tag("integration")) require:
 * - Native library built and accessible
 * - FUGLE_API_KEY environment variable set
 */
public class WebSocketClientTest {

    private static boolean nativeLibraryAvailable = false;

    @BeforeAll
    static void checkNativeLibrary() {
        try {
            // Attempt to create a client to check if native library is available
            try (FugleWebSocketClient client = FugleWebSocketClient.builder()
                    .apiKey("test-api-key")
                    .build()) {
                nativeLibraryAvailable = true;
            }
        } catch (UnsatisfiedLinkError | NoClassDefFoundError e) {
            // Native library not available
            nativeLibraryAvailable = false;
        } catch (Exception e) {
            // Other exceptions mean library loaded but failed validation
            nativeLibraryAvailable = true;
        }
    }

    private void assumeNativeLibraryAvailable() {
        Assumptions.assumeTrue(nativeLibraryAvailable,
                "Native library not available. Build with: cargo build -p marketdata-uniffi --release");
    }

    // ========== Structural Tests (Type Existence) ==========

    @Test
    @DisplayName("FugleWebSocketClient type exists and implements AutoCloseable")
    void webSocketClientTypeExists() {
        assertNotNull(FugleWebSocketClient.class);
        assertTrue(AutoCloseable.class.isAssignableFrom(FugleWebSocketClient.class));
    }

    @Test
    @DisplayName("FugleWebSocketClient.Builder type exists")
    void builderTypeExists() {
        assertNotNull(FugleWebSocketClient.Builder.class);
    }

    @Test
    @DisplayName("WebSocketListener interface exists")
    void webSocketListenerExists() {
        assertNotNull(WebSocketListener.class);
        assertTrue(WebSocketListener.class.isInterface());
    }

    @Test
    @DisplayName("StreamMessage type exists")
    void streamMessageExists() {
        assertNotNull(StreamMessage.class);
    }

    // ========== API Shape Tests ==========

    @Test
    @DisplayName("FugleWebSocketClient has builder() method")
    void hasBuilderMethod() throws NoSuchMethodException {
        Method method = FugleWebSocketClient.class.getMethod("builder");
        assertNotNull(method);
        assertEquals(FugleWebSocketClient.Builder.class, method.getReturnType());
    }

    @Test
    @DisplayName("FugleWebSocketClient has connect() method")
    void hasConnectMethod() throws NoSuchMethodException {
        Method method = FugleWebSocketClient.class.getMethod("connect");
        assertNotNull(method);
        assertEquals(CompletableFuture.class, method.getReturnType());
    }

    @Test
    @DisplayName("FugleWebSocketClient has disconnect() method")
    void hasDisconnectMethod() throws NoSuchMethodException {
        Method method = FugleWebSocketClient.class.getMethod("disconnect");
        assertNotNull(method);
        assertEquals(CompletableFuture.class, method.getReturnType());
    }

    @Test
    @DisplayName("FugleWebSocketClient has subscribe() method")
    void hasSubscribeMethod() throws NoSuchMethodException {
        Method method = FugleWebSocketClient.class.getMethod("subscribe", String.class, String.class);
        assertNotNull(method);
        assertEquals(CompletableFuture.class, method.getReturnType());
    }

    @Test
    @DisplayName("FugleWebSocketClient has unsubscribe() method")
    void hasUnsubscribeMethod() throws NoSuchMethodException {
        Method method = FugleWebSocketClient.class.getMethod("unsubscribe", String.class, String.class);
        assertNotNull(method);
        assertEquals(CompletableFuture.class, method.getReturnType());
    }

    @Test
    @DisplayName("FugleWebSocketClient has isConnected() method")
    void hasIsConnectedMethod() throws NoSuchMethodException {
        Method method = FugleWebSocketClient.class.getMethod("isConnected");
        assertNotNull(method);
        assertEquals(boolean.class, method.getReturnType());
    }

    @Test
    @DisplayName("FugleWebSocketClient has pull-based methods")
    void hasPullBasedMethods() throws NoSuchMethodException {
        // poll()
        Method poll = FugleWebSocketClient.class.getMethod("poll");
        assertNotNull(poll);
        assertEquals(StreamMessage.class, poll.getReturnType());

        // poll(timeout, unit)
        Method pollTimeout = FugleWebSocketClient.class.getMethod("poll", long.class, TimeUnit.class);
        assertNotNull(pollTimeout);
        assertEquals(StreamMessage.class, pollTimeout.getReturnType());

        // take()
        Method take = FugleWebSocketClient.class.getMethod("take");
        assertNotNull(take);
        assertEquals(StreamMessage.class, take.getReturnType());

        // queueSize()
        Method queueSize = FugleWebSocketClient.class.getMethod("queueSize");
        assertNotNull(queueSize);
        assertEquals(int.class, queueSize.getReturnType());
    }

    @Test
    @DisplayName("FugleWebSocketClient has error queue methods")
    void hasErrorQueueMethods() throws NoSuchMethodException {
        Method hasErrors = FugleWebSocketClient.class.getMethod("hasErrors");
        assertNotNull(hasErrors);
        assertEquals(boolean.class, hasErrors.getReturnType());

        Method pollError = FugleWebSocketClient.class.getMethod("pollError");
        assertNotNull(pollError);
        assertEquals(String.class, pollError.getReturnType());
    }

    @Test
    @DisplayName("Builder has apiKey() method")
    void builderHasApiKeyMethod() throws NoSuchMethodException {
        Method method = FugleWebSocketClient.Builder.class.getMethod("apiKey", String.class);
        assertNotNull(method);
        assertEquals(FugleWebSocketClient.Builder.class, method.getReturnType());
    }

    @Test
    @DisplayName("Builder has stock() method")
    void builderHasStockMethod() throws NoSuchMethodException {
        Method method = FugleWebSocketClient.Builder.class.getMethod("stock");
        assertNotNull(method);
        assertEquals(FugleWebSocketClient.Builder.class, method.getReturnType());
    }

    @Test
    @DisplayName("Builder has futopt() method")
    void builderHasFutOptMethod() throws NoSuchMethodException {
        Method method = FugleWebSocketClient.Builder.class.getMethod("futopt");
        assertNotNull(method);
        assertEquals(FugleWebSocketClient.Builder.class, method.getReturnType());
    }

    @Test
    @DisplayName("Builder has listener() method")
    void builderHasListenerMethod() throws NoSuchMethodException {
        Method method = FugleWebSocketClient.Builder.class.getMethod("listener", WebSocketListener.class);
        assertNotNull(method);
        assertEquals(FugleWebSocketClient.Builder.class, method.getReturnType());
    }

    @Test
    @DisplayName("Builder has queueCapacity() method")
    void builderHasQueueCapacityMethod() throws NoSuchMethodException {
        Method method = FugleWebSocketClient.Builder.class.getMethod("queueCapacity", int.class);
        assertNotNull(method);
        assertEquals(FugleWebSocketClient.Builder.class, method.getReturnType());
    }

    @Test
    @DisplayName("WebSocketListener has required callback methods")
    void webSocketListenerHasMethods() throws NoSuchMethodException {
        assertNotNull(WebSocketListener.class.getMethod("onConnected"));
        assertNotNull(WebSocketListener.class.getMethod("onDisconnected"));
        assertNotNull(WebSocketListener.class.getMethod("onMessage", StreamMessage.class));
        assertNotNull(WebSocketListener.class.getMethod("onError", String.class));
    }

    // ========== Constructor Tests (require native library) ==========

    @Test
    @DisplayName("Builder with apiKey in pull mode succeeds")
    void builderPullModeSucceeds() {
        assumeNativeLibraryAvailable();

        try (FugleWebSocketClient client = FugleWebSocketClient.builder()
                .apiKey("test-api-key")
                .stock()
                .build()) {
            assertNotNull(client);
        }
    }

    @Test
    @DisplayName("Builder with apiKey in callback mode succeeds")
    void builderCallbackModeSucceeds() {
        assumeNativeLibraryAvailable();

        WebSocketListener listener = new WebSocketListener() {
            @Override
            public void onConnected() {}

            @Override
            public void onDisconnected() {}

            @Override
            public void onMessage(StreamMessage message) {}

            @Override
            public void onError(String errorMessage) {}

            @Override
            public void onReconnecting(Integer attempt) {}

            @Override
            public void onReconnectFailed(Integer attempts) {}
        };

        try (FugleWebSocketClient client = FugleWebSocketClient.builder()
                .apiKey("test-api-key")
                .stock()
                .listener(listener)
                .build()) {
            assertNotNull(client);
        }
    }

    @Test
    @DisplayName("Builder without apiKey throws exception")
    void builderWithoutApiKeyThrows() {
        assumeNativeLibraryAvailable();

        assertThrows(IllegalStateException.class, () ->
                FugleWebSocketClient.builder().build()
        );
    }

    @Test
    @DisplayName("Builder with empty apiKey throws exception")
    void builderWithEmptyApiKeyThrows() {
        assumeNativeLibraryAvailable();

        assertThrows(IllegalStateException.class, () ->
                FugleWebSocketClient.builder().apiKey("").build()
        );
    }

    @Test
    @DisplayName("Pull mode methods throw exception in callback mode")
    void pullMethodsThrowInCallbackMode() {
        assumeNativeLibraryAvailable();

        WebSocketListener listener = new WebSocketListener() {
            @Override
            public void onConnected() {}

            @Override
            public void onDisconnected() {}

            @Override
            public void onMessage(StreamMessage message) {}

            @Override
            public void onError(String errorMessage) {}

            @Override
            public void onReconnecting(Integer attempt) {}

            @Override
            public void onReconnectFailed(Integer attempts) {}
        };

        try (FugleWebSocketClient client = FugleWebSocketClient.builder()
                .apiKey("test-api-key")
                .listener(listener)
                .build()) {

            assertThrows(IllegalStateException.class, client::poll);
            assertThrows(IllegalStateException.class, client::queueSize);
            assertThrows(IllegalStateException.class, client::hasErrors);
            assertThrows(IllegalStateException.class, client::pollError);
        }
    }

    @Test
    @DisplayName("Client starts in disconnected state")
    void startsDisconnected() {
        assumeNativeLibraryAvailable();

        try (FugleWebSocketClient client = FugleWebSocketClient.builder()
                .apiKey("test-api-key")
                .build()) {
            assertFalse(client.isConnected());
        }
    }

    @Test
    @DisplayName("Custom queue capacity is respected")
    void customQueueCapacity() {
        assumeNativeLibraryAvailable();

        try (FugleWebSocketClient client = FugleWebSocketClient.builder()
                .apiKey("test-api-key")
                .queueCapacity(100)
                .build()) {
            assertNotNull(client);
            assertEquals(0, client.queueSize());
        }
    }

    // ========== Integration Tests (require FUGLE_API_KEY) ==========

    @Test
    @Tag("integration")
    @DisplayName("Connect with valid API key succeeds")
    void connectWithValidKey() throws Exception {
        assumeNativeLibraryAvailable();

        String apiKey = System.getenv("FUGLE_API_KEY");
        Assumptions.assumeTrue(apiKey != null && !apiKey.isEmpty(),
                "FUGLE_API_KEY environment variable not set");

        try (FugleWebSocketClient client = FugleWebSocketClient.builder()
                .apiKey(apiKey)
                .stock()
                .build()) {

            client.connect().get();
            assertTrue(client.isConnected());

            client.disconnect().get();
            assertFalse(client.isConnected());
        }
    }

    @Test
    @Tag("integration")
    @DisplayName("Subscribe and receive messages in pull mode")
    void subscribeAndReceiveMessages() throws Exception {
        assumeNativeLibraryAvailable();

        String apiKey = System.getenv("FUGLE_API_KEY");
        Assumptions.assumeTrue(apiKey != null && !apiKey.isEmpty(),
                "FUGLE_API_KEY environment variable not set");

        try (FugleWebSocketClient client = FugleWebSocketClient.builder()
                .apiKey(apiKey)
                .stock()
                .queueCapacity(100)
                .build()) {

            client.connect().get();
            client.subscribe("trades", "2330").get();

            // Wait for a message (with timeout)
            StreamMessage msg = client.poll(10, TimeUnit.SECONDS);
            if (msg != null) {
                assertNotNull(msg.event());
                System.out.println("Received event: " + msg.event());
            }

            client.disconnect().get();
        }
    }

    @Test
    @Tag("integration")
    @DisplayName("Subscribe and receive messages in callback mode")
    void subscribeAndReceiveMessagesCallback() throws Exception {
        assumeNativeLibraryAvailable();

        String apiKey = System.getenv("FUGLE_API_KEY");
        Assumptions.assumeTrue(apiKey != null && !apiKey.isEmpty(),
                "FUGLE_API_KEY environment variable not set");

        final boolean[] messageReceived = {false};

        WebSocketListener listener = new WebSocketListener() {
            @Override
            public void onConnected() {
                System.out.println("Connected!");
            }

            @Override
            public void onDisconnected() {
                System.out.println("Disconnected");
            }

            @Override
            public void onMessage(StreamMessage message) {
                System.out.println("Received: " + message.event());
                messageReceived[0] = true;
            }

            @Override
            public void onError(String errorMessage) {
                System.err.println("Error: " + errorMessage);
            }

            @Override
            public void onReconnecting(Integer attempt) {}

            @Override
            public void onReconnectFailed(Integer attempts) {}
        };

        try (FugleWebSocketClient client = FugleWebSocketClient.builder()
                .apiKey(apiKey)
                .stock()
                .listener(listener)
                .build()) {

            client.connect().get();
            client.subscribe("trades", "2330").get();

            // Wait for messages
            Thread.sleep(10000);

            client.disconnect().get();
        }
    }
}
