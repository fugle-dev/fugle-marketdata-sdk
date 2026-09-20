package tw.com.fugle.marketdata;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.ValueSource;
import static org.junit.jupiter.api.Assertions.*;

import tw.com.fugle.marketdata.generated.SubscribeOptions;

import java.util.List;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.CopyOnWriteArrayList;
import java.util.concurrent.ExecutionException;
import java.util.concurrent.TimeUnit;
import java.util.function.Supplier;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

/**
 * Subscribe and unsubscribe follow the client's endpoint (#123): the FutOpt
 * endpoint takes FutOpt channels and the after-hours session; the Stock
 * endpoint rejects after-hours with 1005. Unsubscribe sends the id the server
 * issued (#136).
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
                 return subscribedAck(text);
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
                "{\"event\":\"unsubscribe\",\"data\":{\"id\":\"id-books-TXFE6-ah\"}}");
            awaitFrames(frames, expected.size());
            client.disconnect().get(10, TimeUnit.SECONDS);

            assertEquals(expected, frames);
        }
    }

    @Test
    void unsubscribeIdsSendsIds() throws Exception {
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
                     .baseUrl(server.url())
                     .build()) {
            assertInvalidParameter(() -> client.unsubscribe(List.of()));
            client.connect().get(10, TimeUnit.SECONDS);
            client.unsubscribe(List.of("id-a")).get(10, TimeUnit.SECONDS);
            client.unsubscribe(List.of("id-b", "id-c")).get(10, TimeUnit.SECONDS);

            List<String> expected = List.of(
                "{\"event\":\"unsubscribe\",\"data\":{\"id\":\"id-a\"}}",
                "{\"event\":\"unsubscribe\",\"data\":{\"ids\":[\"id-b\",\"id-c\"]}}");
            awaitFrames(frames, expected.size());
            client.disconnect().get(10, TimeUnit.SECONDS);

            assertEquals(expected, frames);
        }
    }

    /**
     * A {@code subscribed} ack for a single-symbol {@code subscribe} frame, with
     * id {@code id-<channel>-<symbol>[-ah]}; nothing for other frames.
     */
    private static List<String> subscribedAck(String text) {
        Matcher m = SUBSCRIBE.matcher(text);
        if (!m.matches()) {
            return List.of();
        }
        String id = "id-" + m.group(2) + "-" + m.group(3)
                + (text.contains("\"afterHours\":true") ? "-ah" : "");
        return List.of("{\"event\":\"subscribed\",\"data\":{\"id\":\"" + id + "\"," + m.group(1));
    }

    private static final Pattern SUBSCRIBE = Pattern.compile(
            "\\{\"event\":\"subscribe\",\"data\":\\{(\"channel\":\"(\\w+)\",\"symbol\":\"(\\w+)\".*)");

    private static void awaitFrames(List<String> frames, int count) throws InterruptedException {
        long deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(10);
        while (frames.size() < count && System.nanoTime() < deadline) {
            Thread.sleep(20);
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

    @Test
    void subscribeSendsMultipleSymbolsInOneFrameAndSingleStillUsesSymbol() throws Exception {
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
                     .stock()
                     .baseUrl(server.url())
                     .build()) {
            client.connect().get(10, TimeUnit.SECONDS);

            client.subscribe("trades", List.of("2330", "2317")).get(10, TimeUnit.SECONDS);
            client.subscribe("trades", List.of("2330")).get(10, TimeUnit.SECONDS);

            List<String> expected = List.of(
                "{\"event\":\"subscribe\",\"data\":{\"channel\":\"trades\",\"symbols\":[\"2330\",\"2317\"]}}",
                "{\"event\":\"subscribe\",\"data\":{\"channel\":\"trades\",\"symbol\":\"2330\"}}");
            awaitFrames(frames, expected.size());
            client.disconnect().get(10, TimeUnit.SECONDS);

            assertEquals(expected, frames);
        }
    }

    @Test
    void subscribeSendsIntradayOddLotOnStockEndpoint() throws Exception {
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
                     .stock()
                     .baseUrl(server.url())
                     .build()) {
            client.connect().get(10, TimeUnit.SECONDS);

            client.subscribe("trades", List.of("2330"), new SubscribeOptions(null, true))
                    .get(10, TimeUnit.SECONDS);

            List<String> expected = List.of(
                "{\"event\":\"subscribe\",\"data\":{\"channel\":\"trades\",\"symbol\":\"2330\",\"intradayOddLot\":true}}");
            awaitFrames(frames, expected.size());
            client.disconnect().get(10, TimeUnit.SECONDS);

            assertEquals(expected, frames);
        }
    }

    @Test
    void futoptEndpointRejectsIntradayOddLot() {
        NativeLibrary.assumeAvailable();

        try (FugleWebSocketClient client = FugleWebSocketClient.builder().apiKey("the-key").futopt().build()) {
            assertInvalidParameter(() ->
                    client.subscribe("trades", List.of("TXFE6"), new SubscribeOptions(null, true)));
        }
    }

    @Test
    void emptySymbolsListIsRejected() {
        NativeLibrary.assumeAvailable();

        try (FugleWebSocketClient client = FugleWebSocketClient.builder().apiKey("the-key").stock().build()) {
            assertInvalidParameter(() -> client.subscribe("trades", List.of()));
            assertInvalidParameter(() -> client.unsubscribe("trades", List.of()));
        }
    }

    private static void assertInvalidParameter(Supplier<CompletableFuture<Void>> call) {
        ExecutionException e = assertThrows(ExecutionException.class,
                () -> call.get().get(10, TimeUnit.SECONDS));
        assertEquals(Integer.valueOf(1005), FugleException.unwrap(e).getCode());
    }
}
