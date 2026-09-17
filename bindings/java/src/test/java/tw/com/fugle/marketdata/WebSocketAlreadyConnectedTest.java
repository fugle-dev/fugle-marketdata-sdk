package tw.com.fugle.marketdata;

import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.*;

import java.util.List;
import java.util.concurrent.ExecutionException;
import java.util.concurrent.TimeUnit;

/**
 * {@code connect()} while connected is refused with code 2011 and leaves the
 * connection up (#119); it used to open a second connection.
 */
public class WebSocketAlreadyConnectedTest {

    private static LoopbackWsServer authAckingServer() throws Exception {
        return new LoopbackWsServer(text -> text.contains("\"event\":\"auth\"")
            ? List.of("{\"event\":\"authenticated\",\"data\":{}}")
            : List.of());
    }

    @Test
    void connectWhileConnectedIsRefusedWith2011() throws Exception {
        NativeLibrary.assumeAvailable();

        try (LoopbackWsServer server = authAckingServer();
             FugleWebSocketClient client = FugleWebSocketClient.builder()
                     .apiKey("the-key")
                     .stock()
                     .baseUrl(server.url())
                     .build()) {
            client.connect().get(10, TimeUnit.SECONDS);

            ExecutionException thrown = assertThrows(ExecutionException.class,
                    () -> client.connect().get(10, TimeUnit.SECONDS));
            FugleException error = FugleException.unwrap(thrown);
            assertEquals(2011, error.getCode(), String.valueOf(error));
            assertTrue(client.isConnected(), "the first connection stays up");

            client.disconnect().get(10, TimeUnit.SECONDS);
            client.connect().get(10, TimeUnit.SECONDS);
            assertTrue(client.isConnected(), "connect() after disconnect() is allowed");
        }
    }
}
