package tw.com.fugle.marketdata;

import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.*;

import tw.com.fugle.marketdata.generated.DisconnectInfo;
import tw.com.fugle.marketdata.generated.DisconnectIntent;

import java.util.List;
import java.util.concurrent.TimeUnit;
import java.util.function.BooleanSupplier;

/**
 * {@code lastDisconnect()} says who closed the connection and whether a
 * reconnect follows (#293); {@code onDisconnected(Boolean)} is unchanged.
 */
public class WebSocketLastDisconnectTest {

    private static LoopbackWsServer authAckingServer() throws Exception {
        return new LoopbackWsServer(text -> text.contains("\"event\":\"auth\"")
            ? List.of("{\"event\":\"authenticated\",\"data\":{}}")
            : List.of());
    }

    private static FugleWebSocketClient client(LoopbackWsServer server) {
        return FugleWebSocketClient.builder()
                .apiKey("the-key")
                .stock()
                .baseUrl(server.url())
                .reconnect(ReconnectOptions.builder().enabled(false).build())
                .build();
    }

    @Test
    void lastDisconnectAfterDisconnectIsClient() throws Exception {
        NativeLibrary.assumeAvailable();

        try (LoopbackWsServer server = authAckingServer();
             FugleWebSocketClient client = client(server)) {
            assertNull(client.lastDisconnect(), "null before connect()");
            client.connect().get(10, TimeUnit.SECONDS);
            client.disconnect().get(10, TimeUnit.SECONDS);

            assertEquals(
                    new DisconnectInfo((short) 1000, "Normal closure", DisconnectIntent.CLIENT, false),
                    client.lastDisconnect());
        }
    }

    @Test
    void lastDisconnectAfterServerCloseIsServer() throws Exception {
        NativeLibrary.assumeAvailable();

        try (LoopbackWsServer server = authAckingServer();
             FugleWebSocketClient client = client(server)) {
            client.connect().get(10, TimeUnit.SECONDS);
            server.closeConnections(1001, "going away");
            waitUntil(() -> client.lastDisconnect() != null, "lastDisconnect() never set");

            assertEquals(
                    new DisconnectInfo((short) 1001, "going away", DisconnectIntent.SERVER, false),
                    client.lastDisconnect());
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
