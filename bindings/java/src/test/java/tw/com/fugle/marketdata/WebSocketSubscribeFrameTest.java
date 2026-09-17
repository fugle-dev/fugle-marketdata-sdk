package tw.com.fugle.marketdata;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.ValueSource;
import static org.junit.jupiter.api.Assertions.*;

import java.util.List;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.CopyOnWriteArrayList;
import java.util.concurrent.ExecutionException;
import java.util.concurrent.TimeUnit;
import java.util.function.Supplier;

/**
 * Subscribe and unsubscribe follow the client's endpoint (#123): the FutOpt
 * endpoint takes FutOpt channels and the after-hours session; the Stock
 * endpoint rejects after-hours with 1005.
 */
public class WebSocketSubscribeFrameTest {

    @Test
    void futoptEndpointSendsAfterHours() throws Exception {
        NativeLibrary.assumeAvailable();

        List<String> frames = new CopyOnWriteArrayList<>();
        try (LoopbackWsServer server = new LoopbackWsServer(text -> {
                 if (text.startsWith("{\"event\":\"auth\"")) {
                     return List.of("{\"event\":\"authenticated\",\"data\":{}}");
                 }
                 frames.add(text);
                 return List.of();
             });
             FugleWebSocketClient client = FugleWebSocketClient.builder()
                     .apiKey("the-key")
                     .futopt()
                     .baseUrl(server.url())
                     .build()) {
            client.connect().get(10, TimeUnit.SECONDS);

            assertInvalidParameter(() -> client.subscribe("indices", "TXFE6"));
            client.subscribe("books", "TXFE6", true).get(10, TimeUnit.SECONDS);
            client.subscribe("trades", "TXFE6").get(10, TimeUnit.SECONDS);
            client.unsubscribe("books", "TXFE6", true).get(10, TimeUnit.SECONDS);

            List<String> expected = List.of(
                "{\"event\":\"subscribe\",\"data\":{\"channel\":\"books\",\"symbol\":\"TXFE6\",\"afterHours\":true}}",
                "{\"event\":\"subscribe\",\"data\":{\"channel\":\"trades\",\"symbol\":\"TXFE6\"}}",
                "{\"event\":\"unsubscribe\",\"data\":{\"id\":\"books:TXFE6:afterhours\"}}");
            long deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(10);
            while (frames.size() < expected.size() && System.nanoTime() < deadline) {
                Thread.sleep(20);
            }
            client.disconnect().get(10, TimeUnit.SECONDS);

            assertEquals(expected, frames);
        }
    }

    @ParameterizedTest(name = "afterHours={0} on the Stock endpoint is 1005")
    @ValueSource(booleans = {true, false})
    void stockEndpointRejectsAfterHours(boolean afterHours) throws Exception {
        NativeLibrary.assumeAvailable();

        try (FugleWebSocketClient client = FugleWebSocketClient.builder().apiKey("the-key").stock().build()) {
            assertInvalidParameter(() -> client.subscribe("trades", "2330", afterHours));
            assertInvalidParameter(() -> client.unsubscribe("trades", "2330", afterHours));
        }
    }

    private static void assertInvalidParameter(Supplier<CompletableFuture<Void>> call) {
        ExecutionException e = assertThrows(ExecutionException.class,
                () -> call.get().get(10, TimeUnit.SECONDS));
        assertEquals(Integer.valueOf(1005), FugleException.unwrap(e).getCode());
    }
}
