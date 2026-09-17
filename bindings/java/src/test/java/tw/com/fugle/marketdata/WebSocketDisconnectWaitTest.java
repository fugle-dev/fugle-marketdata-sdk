package tw.com.fugle.marketdata;

import tw.com.fugle.marketdata.generated.*;
import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.*;

import java.util.List;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.ExecutionException;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicReference;

/**
 * {@code disconnect()} completes once the listener has handled the
 * connection's remaining events (#126), and a listener method that calls it
 * does not wait for itself.
 */
public class WebSocketDisconnectWaitTest {

    private static LoopbackWsServer authAckingServer() throws Exception {
        return new LoopbackWsServer(text -> text.contains("\"event\":\"auth\"")
            ? List.of("{\"event\":\"authenticated\",\"data\":{}}")
            : List.of());
    }

    @Test
    void disconnectCompletesAfterOnDisconnected() throws Exception {
        NativeLibrary.assumeAvailable();

        CountDownLatch authenticated = new CountDownLatch(1);
        CountDownLatch disconnected = new CountDownLatch(1);
        Listener listener = new Listener() {
            @Override
            public void onAuthenticated(String dataJson) {
                authenticated.countDown();
            }

            @Override
            public void onDisconnected(Boolean willReconnect) {
                sleep(300);
                disconnected.countDown();
            }
        };
        try (LoopbackWsServer server = authAckingServer();
             FugleWebSocketClient client = FugleWebSocketClient.builder()
                     .apiKey("the-key")
                     .stock()
                     .baseUrl(server.url())
                     .listener(listener)
                     .build()) {
            client.connect().get(10, TimeUnit.SECONDS);
            assertTrue(authenticated.await(5, TimeUnit.SECONDS));

            client.disconnect().get(10, TimeUnit.SECONDS);
            assertEquals(0, disconnected.getCount(), "onDisconnected had not run when disconnect() completed");
        }
    }

    @Test
    void disconnectFromAListenerMethodDoesNotWaitForItself() throws Exception {
        NativeLibrary.assumeAvailable();

        AtomicReference<FugleWebSocketClient> self = new AtomicReference<>();
        CompletableFuture<Void> fromCallback = new CompletableFuture<>();
        CountDownLatch disconnected = new CountDownLatch(1);
        Listener listener = new Listener() {
            @Override
            public void onAuthenticated(String dataJson) {
                try {
                    self.get().disconnect().get(5, TimeUnit.SECONDS);
                    fromCallback.complete(null);
                } catch (Exception e) {
                    fromCallback.completeExceptionally(e);
                }
            }

            @Override
            public void onDisconnected(Boolean willReconnect) {
                disconnected.countDown();
            }
        };
        try (LoopbackWsServer server = authAckingServer();
             FugleWebSocketClient client = FugleWebSocketClient.builder()
                     .apiKey("the-key")
                     .stock()
                     .baseUrl(server.url())
                     .listener(listener)
                     .build()) {
            self.set(client);
            // The listener disconnects during the handshake, so connect()
            // either finds the connection stored and closes it, or gives it
            // up with code 2010 (#121). It ends either way.
            try {
                client.connect().get(10, TimeUnit.SECONDS);
            } catch (ExecutionException e) {
                assertEquals(2010, FugleException.unwrap(e).getCode(), String.valueOf(e.getCause()));
            }

            fromCallback.get(10, TimeUnit.SECONDS);
            assertTrue(disconnected.await(5, TimeUnit.SECONDS), "onDisconnected never ran");
            assertFalse(client.isConnected(), "still connected after the listener disconnected");
            client.disconnect().get(10, TimeUnit.SECONDS);
        }
    }

    private static void sleep(long ms) {
        try {
            Thread.sleep(ms);
        } catch (InterruptedException e) {
            Thread.currentThread().interrupt();
        }
    }

    /** A listener that ignores every event it does not override. */
    private abstract static class Listener implements WebSocketListener {
        @Override public void onConnected() {}
        @Override public void onAuthenticated(String dataJson) {}
        @Override public void onUnauthenticated(String dataJson) {}
        @Override public void onDisconnected(Boolean willReconnect) {}
        @Override public void onMessage(StreamMessage message) {}
        @Override public void onError(ErrorInfo error) {}
        @Override public void onReconnecting(Integer attempt) {}
        @Override public void onReconnectFailed(Integer attempts) {}
        @Override public void onMessagesDropped(Long count) {}
    }
}
