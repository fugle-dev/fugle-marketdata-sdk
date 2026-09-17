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
    @DisplayName("Builder has messageOverflow() method")
    void builderHasMessageOverflowMethod() throws NoSuchMethodException {
        Method method = FugleWebSocketClient.Builder.class.getMethod("messageOverflow", MessageOverflow.class);
        assertNotNull(method);
        assertEquals(FugleWebSocketClient.Builder.class, method.getReturnType());
    }

    @Test
    @DisplayName("Builder has messageBuffer() method")
    void builderHasMessageBufferMethod() throws NoSuchMethodException {
        Method method = FugleWebSocketClient.Builder.class.getMethod("messageBuffer", int.class);
        assertNotNull(method);
        assertEquals(FugleWebSocketClient.Builder.class, method.getReturnType());
    }

    @Test
    @DisplayName("FugleWebSocketClient has messagesDroppedTotal() method")
    void hasMessagesDroppedTotalMethod() throws NoSuchMethodException {
        Method method = FugleWebSocketClient.class.getMethod("messagesDroppedTotal");
        assertNotNull(method);
        assertEquals(long.class, method.getReturnType());
    }

    @Test
    @DisplayName("WebSocketListener has required callback methods")
    void webSocketListenerHasMethods() throws NoSuchMethodException {
        assertNotNull(WebSocketListener.class.getMethod("onConnected"));
        assertNotNull(WebSocketListener.class.getMethod("onAuthenticated", String.class));
        assertNotNull(WebSocketListener.class.getMethod("onUnauthenticated", String.class));
        assertNotNull(WebSocketListener.class.getMethod("onDisconnected", Boolean.class));
        assertNotNull(WebSocketListener.class.getMethod("onMessage", StreamMessage.class));
        assertNotNull(WebSocketListener.class.getMethod("onError", ErrorInfo.class));
        assertNotNull(WebSocketListener.class.getMethod("onMessagesDropped", Long.class));
    }

    // ========== Constructor Tests (require native library) ==========

    @Test
    @DisplayName("Builder with apiKey in pull mode succeeds")
    void builderPullModeSucceeds() {
        NativeLibrary.assumeAvailable();

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
        NativeLibrary.assumeAvailable();

        WebSocketListener listener = new WebSocketListener() {
            @Override
            public void onConnected() {}

            @Override
            public void onAuthenticated(String dataJson) {}

            @Override
            public void onUnauthenticated(String dataJson) {}

            @Override
            public void onDisconnected(Boolean willReconnect) {}

            @Override
            public void onMessage(StreamMessage message) {}

            @Override
            public void onError(ErrorInfo error) {}

            @Override
            public void onReconnecting(Integer attempt) {}

            @Override
            public void onReconnectFailed(Integer attempts) {}

            @Override
            public void onMessagesDropped(Long count) {}
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
    @DisplayName("Builder without credentials throws exception")
    void builderWithoutCredentialsThrows() {
        NativeLibrary.assumeAvailable();

        FugleException e = assertThrows(FugleException.class, () ->
                FugleWebSocketClient.builder().build()
        );
        assertEquals(Integer.valueOf(1004), e.getCode());
        assertTrue(e.getMessage().contains("exactly one non-empty credential"));
    }

    @Test
    @DisplayName("Builder rejects an empty apiKey with code 1004")
    void builderWithEmptyApiKeyIsRejected() {
        NativeLibrary.assumeAvailable();

        FugleException e = assertThrows(FugleException.class, () ->
                FugleWebSocketClient.builder().apiKey("").build()
        );
        assertEquals(Integer.valueOf(1004), e.getCode());
    }

    @Test
    @DisplayName("Pull mode methods throw exception in callback mode")
    void pullMethodsThrowInCallbackMode() {
        NativeLibrary.assumeAvailable();

        WebSocketListener listener = new WebSocketListener() {
            @Override
            public void onConnected() {}

            @Override
            public void onAuthenticated(String dataJson) {}

            @Override
            public void onUnauthenticated(String dataJson) {}

            @Override
            public void onDisconnected(Boolean willReconnect) {}

            @Override
            public void onMessage(StreamMessage message) {}

            @Override
            public void onError(ErrorInfo error) {}

            @Override
            public void onReconnecting(Integer attempt) {}

            @Override
            public void onReconnectFailed(Integer attempts) {}

            @Override
            public void onMessagesDropped(Long count) {}
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
        NativeLibrary.assumeAvailable();

        try (FugleWebSocketClient client = FugleWebSocketClient.builder()
                .apiKey("test-api-key")
                .build()) {
            assertFalse(client.isConnected());
        }
    }

    @Test
    @DisplayName("Custom queue capacity is respected")
    void customQueueCapacity() {
        NativeLibrary.assumeAvailable();

        try (FugleWebSocketClient client = FugleWebSocketClient.builder()
                .apiKey("test-api-key")
                .queueCapacity(100)
                .build()) {
            assertNotNull(client);
            assertEquals(0, client.queueSize());
        }
    }

    @Test
    @DisplayName("messagesDroppedTotal() is 0 before connect")
    void messagesDroppedTotalStartsAtZero() {
        NativeLibrary.assumeAvailable();

        try (FugleWebSocketClient client = FugleWebSocketClient.builder()
                .apiKey("test-api-key")
                .build()) {
            assertEquals(0L, client.messagesDroppedTotal());
        }
    }

    @Test
    @DisplayName("Builder accepts messageOverflow and messageBuffer")
    void builderAcceptsMessageQueueOptions() {
        NativeLibrary.assumeAvailable();

        try (FugleWebSocketClient client = FugleWebSocketClient.builder()
                .apiKey("test-api-key")
                .messageOverflow(MessageOverflow.UNBOUNDED)
                .messageBuffer(256)
                .build()) {
            assertNotNull(client);
        }
    }

    // ========== Integration Tests (require FUGLE_API_KEY) ==========

    @Test
    @Tag("integration")
    @DisplayName("Connect with valid API key succeeds")
    void connectWithValidKey() throws Exception {
        NativeLibrary.assumeAvailable();

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
        NativeLibrary.assumeAvailable();

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
        NativeLibrary.assumeAvailable();

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
            public void onAuthenticated(String dataJson) {
                System.out.println("Authenticated");
            }

            @Override
            public void onUnauthenticated(String dataJson) {
                System.err.println("Rejected: " + dataJson);
            }

            @Override
            public void onDisconnected(Boolean willReconnect) {
                System.out.println("Disconnected (will reconnect: " + willReconnect + ")");
            }

            @Override
            public void onMessage(StreamMessage message) {
                System.out.println("Received: " + message.event());
                messageReceived[0] = true;
            }

            @Override
            public void onError(ErrorInfo error) {
                System.err.println("Error: " + error.message());
            }

            @Override
            public void onReconnecting(Integer attempt) {}

            @Override
            public void onReconnectFailed(Integer attempts) {}

            @Override
            public void onMessagesDropped(Long count) {}
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
