package tw.com.fugle.marketdata;

import tw.com.fugle.marketdata.generated.*;
import org.junit.jupiter.api.*;
import static org.junit.jupiter.api.Assertions.*;

import java.io.ByteArrayOutputStream;
import java.io.DataInputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.net.InetAddress;
import java.net.ServerSocket;
import java.net.Socket;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.util.Base64;
import java.util.List;
import java.util.concurrent.*;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

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

        try (BurstServer server = new BurstServer();
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
            for (String error; (error = client.pollError()) != null; ) {
                Matcher m = dropped.matcher(error);
                if (m.find()) {
                    reported += Long.parseLong(m.group(1));
                }
            }
            assertEquals(total, reported, "every drop is reported on the error queue");
        }
    }

    /**
     * Loopback WebSocket server on the JDK alone: acks {@code auth}, answers
     * {@code subscribe} with {@link #BURST} {@code data} frames and echoes a
     * Close.
     */
    private static final class BurstServer implements AutoCloseable {
        private final ServerSocket socket = new ServerSocket(0, 1, InetAddress.getLoopbackAddress());
        private final List<Socket> clients = new CopyOnWriteArrayList<>();
        private final Thread acceptor = new Thread(this::acceptLoop, "burst-server");

        BurstServer() throws IOException {
            acceptor.setDaemon(true);
            acceptor.start();
        }

        String url() {
            return "ws://127.0.0.1:" + socket.getLocalPort();
        }

        private void acceptLoop() {
            while (!socket.isClosed()) {
                try {
                    Socket client = socket.accept();
                    clients.add(client);
                    Thread t = new Thread(() -> serve(client), "burst-server-conn");
                    t.setDaemon(true);
                    t.start();
                } catch (IOException e) {
                    return;
                }
            }
        }

        private void serve(Socket client) {
            try (Socket c = client) {
                DataInputStream in = new DataInputStream(c.getInputStream());
                OutputStream out = c.getOutputStream();
                handshake(in, out);
                while (true) {
                    int b0 = in.readUnsignedByte();
                    int b1 = in.readUnsignedByte();
                    long len = b1 & 0x7f;
                    if (len == 126) {
                        len = in.readUnsignedShort();
                    } else if (len == 127) {
                        len = in.readLong();
                    }
                    byte[] mask = new byte[4];
                    if ((b1 & 0x80) != 0) {
                        in.readFully(mask);
                    }
                    byte[] payload = new byte[(int) len];
                    in.readFully(payload);
                    for (int i = 0; i < payload.length; i++) {
                        payload[i] ^= mask[i % 4];
                    }
                    int opcode = b0 & 0x0f;
                    if (opcode == 0x8) {
                        writeFrame(out, 0x8, payload);
                        return;
                    }
                    if (opcode != 0x1) {
                        continue;
                    }
                    String text = new String(payload, StandardCharsets.UTF_8);
                    if (text.contains("\"auth\"")) {
                        sendText(out, "{\"event\":\"authenticated\",\"data\":{}}");
                    } else if (text.contains("\"subscribe\"")) {
                        for (int i = 0; i < BURST; i++) {
                            sendText(out, "{\"event\":\"data\",\"data\":{\"i\":" + i + "},\"channel\":\"trades\"}");
                        }
                    }
                }
            } catch (Exception e) {
                // connection gone
            }
        }

        private static void handshake(InputStream in, OutputStream out) throws Exception {
            ByteArrayOutputStream head = new ByteArrayOutputStream();
            int matched = 0;
            byte[] end = {'\r', '\n', '\r', '\n'};
            while (matched < 4) {
                int b = in.read();
                if (b < 0) {
                    throw new IOException("closed during handshake");
                }
                head.write(b);
                matched = b == end[matched] ? matched + 1 : (b == '\r' ? 1 : 0);
            }
            Matcher m = Pattern.compile("(?i)Sec-WebSocket-Key:\\s*(\\S+)")
                    .matcher(head.toString(StandardCharsets.US_ASCII.name()));
            if (!m.find()) {
                throw new IOException("no Sec-WebSocket-Key");
            }
            byte[] digest = MessageDigest.getInstance("SHA-1").digest(
                    (m.group(1) + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11").getBytes(StandardCharsets.US_ASCII));
            String response = "HTTP/1.1 101 Switching Protocols\r\n"
                    + "Upgrade: websocket\r\nConnection: Upgrade\r\n"
                    + "Sec-WebSocket-Accept: " + Base64.getEncoder().encodeToString(digest) + "\r\n\r\n";
            out.write(response.getBytes(StandardCharsets.US_ASCII));
            out.flush();
        }

        private static void sendText(OutputStream out, String text) throws IOException {
            writeFrame(out, 0x1, text.getBytes(StandardCharsets.UTF_8));
        }

        private static synchronized void writeFrame(OutputStream out, int opcode, byte[] payload)
                throws IOException {
            ByteArrayOutputStream frame = new ByteArrayOutputStream();
            frame.write(0x80 | opcode);
            if (payload.length < 126) {
                frame.write(payload.length);
            } else {
                frame.write(126);
                frame.write(payload.length >> 8);
                frame.write(payload.length & 0xff);
            }
            frame.write(payload);
            out.write(frame.toByteArray());
            out.flush();
        }

        @Override
        public void close() throws IOException {
            socket.close();
            for (Socket c : clients) {
                c.close();
            }
        }
    }
}
