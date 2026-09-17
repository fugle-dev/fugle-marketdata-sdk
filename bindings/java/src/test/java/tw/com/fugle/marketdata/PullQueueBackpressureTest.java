package tw.com.fugle.marketdata;

import tw.com.fugle.marketdata.generated.*;
import org.junit.jupiter.api.*;
import static org.junit.jupiter.api.Assertions.*;

import java.io.IOException;
import java.util.List;
import java.util.concurrent.*;
import java.util.regex.Matcher;
import java.util.regex.Pattern;
import java.util.stream.IntStream;

/**
 * Pull mode with a full queue (#46): the wrapper waits for {@code poll()} to
 * make room rather than dropping silently, so the SDK's own queue drops,
 * counts and reports what the application does not keep up with.
 */
public class PullQueueBackpressureTest {

    private static final int BURST = 500;

    private static StreamMessage message(int i) {
        return new StreamMessage("{}", "data", "trades", "2330", null, "{\"i\":" + i + "}", null, null);
    }

    @Test
    @DisplayName("A full queue holds onMessage until poll() makes room")
    void fullQueueWaitsForRoom() throws Exception {
        BlockingQueue<StreamMessage> queue = new LinkedBlockingQueue<>(1);
        FugleWebSocketClient.InternalListener listener =
                new FugleWebSocketClient.InternalListener(queue, new LinkedBlockingQueue<>());
        listener.onMessage(message(0));

        CompletableFuture<Void> second = CompletableFuture.runAsync(() -> listener.onMessage(message(1)));
        Thread.sleep(300);
        assertFalse(second.isDone(), "onMessage returned while the queue was still full");

        assertEquals("{\"i\":0}", queue.take().dataJson());
        second.get(5, TimeUnit.SECONDS);
        assertEquals("{\"i\":1}", queue.take().dataJson(), "the waiting message was lost");
    }

    @Test
    @DisplayName("stop() releases a waiting onMessage, and resume() waits again")
    void stopReleasesTheWait() throws Exception {
        BlockingQueue<StreamMessage> queue = new LinkedBlockingQueue<>(1);
        FugleWebSocketClient.InternalListener listener =
                new FugleWebSocketClient.InternalListener(queue, new LinkedBlockingQueue<>());
        listener.onMessage(message(0));

        CompletableFuture<Void> waiting = CompletableFuture.runAsync(() -> listener.onMessage(message(1)));
        Thread.sleep(200);
        listener.stop();
        waiting.get(5, TimeUnit.SECONDS);

        listener.resume();
        CompletableFuture<Void> again = CompletableFuture.runAsync(() -> listener.onMessage(message(2)));
        Thread.sleep(300);
        assertFalse(again.isDone(), "resume() did not restore waiting");
        listener.stop();
        again.get(5, TimeUnit.SECONDS);
    }

    @Test
    @DisplayName("A wait from before disconnect() ends even if connect() follows at once")
    void reconnectRightAfterStopStillReleasesTheOldWait() throws Exception {
        BlockingQueue<StreamMessage> queue = new LinkedBlockingQueue<>(1);
        FugleWebSocketClient.InternalListener listener =
                new FugleWebSocketClient.InternalListener(queue, new LinkedBlockingQueue<>());
        listener.onMessage(message(0));

        CompletableFuture<Void> old = CompletableFuture.runAsync(() -> listener.onMessage(message(1)));
        Thread.sleep(150);
        // Both within one wait slice: the old wait never sees a stopped flag
        // unless each connection has its own.
        listener.stop();
        listener.resume();
        old.get(5, TimeUnit.SECONDS);
        assertEquals("{\"i\":0}", queue.poll().dataJson());
        assertNull(queue.poll(), "a message from the old connection was queued");
    }

    @Test
    @DisplayName("A slow poll makes the SDK drop, count and report, never the wrapper")
    void slowPollDropsAreCountedAndReported() throws Exception {
        NativeLibrary.assumeAvailable();

        try (LoopbackWsServer server = burstServer();
             FugleWebSocketClient client = FugleWebSocketClient.builder()
                     .apiKey("test-key")
                     .stock()
                     .baseUrl(server.url())
                     .queueCapacity(4)
                     .messageBuffer(16)
                     .build()) {
            assertEquals(0, client.messagesDroppedTotal());
            client.connect().get(10, TimeUnit.SECONDS);
            client.subscribe("trades", "2330").get(10, TimeUnit.SECONDS);

            int data = 0;
            long deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(30);
            // The burst is over once every frame is either delivered or dropped.
            while (data + client.messagesDroppedTotal() < BURST) {
                assertTrue(System.nanoTime() < deadline,
                        "burst did not settle: " + data + " delivered, "
                                + client.messagesDroppedTotal() + " dropped");
                StreamMessage msg = client.poll(100, TimeUnit.MILLISECONDS);
                if (msg != null && "data".equals(msg.event())) {
                    data++;
                    Thread.sleep(5);
                }
            }
            client.disconnect().get(10, TimeUnit.SECONDS);

            long total = client.messagesDroppedTotal();
            assertTrue(total > 0, "nothing was dropped; the poll was not slow enough");
            assertEquals(BURST, data + total);

            long reported = 0;
            Pattern dropped = Pattern.compile("^Dropped (\\d+) message");
            // The report covering the burst's tail is queued by disconnect()
            // and reaches the listener on its own thread, possibly after
            // disconnect() returns: wait for it rather than read once.
            deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(10);
            while (reported < total && System.nanoTime() < deadline) {
                String error = client.pollError();
                if (error == null) {
                    Thread.sleep(10);
                    continue;
                }
                Matcher m = dropped.matcher(error);
                if (m.find()) {
                    reported += Long.parseLong(m.group(1));
                }
            }
            assertEquals(total, reported, "every drop is reported on the error queue");
        }
    }

    /**
     * Acks {@code auth} and answers {@code subscribe} with {@link #BURST}
     * {@code data} frames.
     */
    private static LoopbackWsServer burstServer() throws IOException {
        return new LoopbackWsServer(text -> {
            if (text.contains("\"auth\"")) {
                return List.of("{\"event\":\"authenticated\",\"data\":{}}");
            }
            if (text.contains("\"subscribe\"")) {
                return IntStream.range(0, BURST)
                        .mapToObj(i -> "{\"event\":\"data\",\"data\":{\"i\":" + i + "},\"channel\":\"trades\"}")
                        .toList();
            }
            return List.of();
        });
    }
}
