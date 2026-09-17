package tw.com.fugle.marketdata;

import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.*;

import java.util.List;
import java.util.concurrent.TimeUnit;
import java.util.function.BooleanSupplier;

/**
 * {@code isClosed()} reads core's connection state (#95): it used to read a
 * flag only {@code disconnect()} set, so it stayed false after the server
 * closed the connection with no reconnect to follow.
 */
public class WebSocketIsClosedTest {

    private static LoopbackWsServer authAckingServer() throws Exception {
        return new LoopbackWsServer(text -> text.contains("\"event\":\"auth\"")
            ? List.of("{\"event\":\"authenticated\",\"data\":{}}")
            : List.of());
    }

    @Test
    void isClosedAfterServerCloseWithoutReconnect() throws Exception {
        NativeLibrary.assumeAvailable();

        try (LoopbackWsServer server = authAckingServer();
             FugleWebSocketClient client = FugleWebSocketClient.builder()
                     .apiKey("the-key")
                     .stock()
                     .baseUrl(server.url())
                     .build()) {
            client.connect().get(10, TimeUnit.SECONDS);
            assertFalse(client.isClosed());

            server.dropConnections();
            waitUntil(client::isClosed, "isClosed() never became true");
            assertFalse(client.isConnected());
        }
    }

    @Test
    void isClosedIsFalseWhileReconnectingAndTrueAfterDisconnect() throws Exception {
        NativeLibrary.assumeAvailable();

        try (LoopbackWsServer server = authAckingServer();
             FugleWebSocketClient client = FugleWebSocketClient.builder()
                     .apiKey("the-key")
                     .stock()
                     .baseUrl(server.url())
                     .reconnect(ReconnectOptions.builder()
                             .maxAttempts(3)
                             .initialDelayMs(2000L)
                             .maxDelayMs(2000L)
                             .build())
                     .build()) {
            client.connect().get(10, TimeUnit.SECONDS);

            server.dropConnections();
            waitUntil(() -> !client.isConnected(), "isConnected() never became false");
            assertFalse(client.isClosed(), "false while reconnecting");

            client.disconnect().get(10, TimeUnit.SECONDS);
            assertTrue(client.isClosed(), "true once disconnect() completes");
        }
    }

    private static void waitUntil(BooleanSupplier condition, String message) throws InterruptedException {
        long deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(5);
        while (!condition.getAsBoolean()) {
            if (System.nanoTime() > deadline) {
                fail(message);
            }
            Thread.sleep(10);
        }
    }
}
