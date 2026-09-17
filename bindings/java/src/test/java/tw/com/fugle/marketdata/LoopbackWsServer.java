package tw.com.fugle.marketdata;

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
import java.util.concurrent.CopyOnWriteArrayList;
import java.util.function.Function;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

/**
 * Loopback WebSocket server on the JDK alone, for tests that connect a real
 * client. Each text frame from the client goes to {@code replies}, whose
 * result is sent back as text frames in order; pings are answered and a
 * Close is echoed. {@link #dropConnections()} cuts the open connections
 * without a Close frame.
 */
final class LoopbackWsServer implements AutoCloseable {
    private final ServerSocket socket = new ServerSocket(0, 1, InetAddress.getLoopbackAddress());
    private final List<Socket> clients = new CopyOnWriteArrayList<>();
    private final Function<String, List<String>> replies;

    LoopbackWsServer(Function<String, List<String>> replies) throws IOException {
        this.replies = replies;
        Thread acceptor = new Thread(this::acceptLoop, "loopback-ws-server");
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
                Thread t = new Thread(() -> serve(client), "loopback-ws-server-conn");
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
                if (opcode == 0x9) {
                    writeFrame(out, 0xA, payload);
                    continue;
                }
                if (opcode != 0x1) {
                    continue;
                }
                for (String reply : replies.apply(new String(payload, StandardCharsets.UTF_8))) {
                    writeFrame(out, 0x1, reply.getBytes(StandardCharsets.UTF_8));
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

    /** Cut every open connection at the transport, as a network failure would. */
    void dropConnections() throws IOException {
        for (Socket c : clients) {
            clients.remove(c);
            c.close();
        }
    }

    @Override
    public void close() throws IOException {
        socket.close();
        for (Socket c : clients) {
            c.close();
        }
    }
}
