package tw.com.fugle.marketdata;

import com.sun.net.httpserver.Headers;
import com.sun.net.httpserver.HttpServer;
import org.junit.jupiter.api.Test;
import tw.com.fugle.marketdata.generated.CredentialsRecord;
import static org.junit.jupiter.api.Assertions.*;

import java.io.OutputStream;
import java.net.InetAddress;
import java.net.InetSocketAddress;
import java.nio.charset.StandardCharsets;
import java.util.List;
import java.util.concurrent.CopyOnWriteArrayList;
import java.util.concurrent.TimeUnit;
import java.util.function.BooleanSupplier;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

/**
 * {@code setCredentials} changes what later connection attempts and requests
 * send (#322). The loopback server accepts only the new credential once the
 * test asks it to: the automatic reconnect after a drop authenticates with
 * it, in the field of its kind, and the live connection gets no new auth
 * frame. After a rejected connect, setting it and connecting again succeeds.
 * A REST client sends it from its next request on. Anything but exactly one
 * non-empty credential is 1004 and keeps the current one.
 */
public class SetCredentialsTest {

    private static final Pattern AUTH_DATA = Pattern.compile("^\\{\"event\":\"auth\",\"data\":(\\{.*\\})\\}$");
    private static final String OLD = "old-key";
    private static final String NEW = "new-token";

    /** Records each auth frame's data; acks it unless {@code required} is set and absent. */
    private static final class AuthServer {
        final List<String> authData = new CopyOnWriteArrayList<>();
        volatile String required;
        final LoopbackWsServer server;

        AuthServer() throws Exception {
            server = new LoopbackWsServer(text -> {
                Matcher m = AUTH_DATA.matcher(text);
                if (!m.matches()) {
                    return List.of();
                }
                authData.add(m.group(1));
                String need = required;
                if (need != null && !m.group(1).contains("\"" + need + "\"")) {
                    return List.of("{\"event\":\"error\",\"code\":1000,\"data\":{\"message\":\"Invalid token\"}}");
                }
                return List.of("{\"event\":\"authenticated\",\"data\":{}}");
            });
        }
    }

    private static FugleWebSocketClient client(AuthServer server) {
        return FugleWebSocketClient.builder()
                .apiKey(OLD)
                .stock()
                .baseUrl(server.server.url())
                .reconnect(ReconnectOptions.builder().maxAttempts(2).initialDelayMs(100L).maxDelayMs(100L).build())
                .build();
    }

    private static void assertConfigError(Runnable action) {
        FugleException ex = assertThrows(FugleException.class, action::run);
        assertEquals(Integer.valueOf(1004), ex.getCode());
    }

    @Test
    void reconnectSendsTheNewCredentialOfAnotherKindAndLeavesTheLiveConnectionAlone() throws Exception {
        NativeLibrary.assumeAvailable();

        AuthServer auth = new AuthServer();
        try (LoopbackWsServer server = auth.server; FugleWebSocketClient client = client(auth)) {
            client.connect().get(10, TimeUnit.SECONDS);

            auth.required = NEW;
            client.setCredentials(Credentials.sdkToken(NEW));
            Thread.sleep(100);
            assertEquals(1, auth.authData.size(), "no auth frame on the live connection");

            server.dropConnections();
            waitUntil(() -> auth.authData.size() == 2 && client.isConnected(), "the reconnect never authenticated");
            client.disconnect().get(10, TimeUnit.SECONDS);

            assertEquals(List.of("{\"apikey\":\"old-key\"}", "{\"sdkToken\":\"new-token\"}"), auth.authData);
        }
    }

    @Test
    void rejectedConnectSucceedsAfterTheCredentialIsSet() throws Exception {
        NativeLibrary.assumeAvailable();

        AuthServer auth = new AuthServer();
        auth.required = NEW;
        try (LoopbackWsServer server = auth.server; FugleWebSocketClient client = client(auth)) {
            assertThrows(Exception.class, () -> client.connect().get(10, TimeUnit.SECONDS));

            client.setCredentials(Credentials.bearerToken(NEW));
            client.connect().get(10, TimeUnit.SECONDS);
            client.disconnect().get(10, TimeUnit.SECONDS);

            assertEquals(List.of("{\"apikey\":\"old-key\"}", "{\"token\":\"new-token\"}"), auth.authData);
        }
    }

    @Test
    void invalidCredentialsAreConfigErrorAndKeepTheCurrentOne() throws Exception {
        NativeLibrary.assumeAvailable();

        AuthServer auth = new AuthServer();
        try (LoopbackWsServer server = auth.server; FugleWebSocketClient client = client(auth)) {
            assertConfigError(() -> client.setCredentials(Credentials.sdkToken("   ")));
            assertConfigError(() -> client.setCredentials(Credentials.apiKey(null)));
            assertThrows(NullPointerException.class, () -> client.setCredentials(null));
            client.connect().get(10, TimeUnit.SECONDS);
            client.disconnect().get(10, TimeUnit.SECONDS);

            assertEquals(List.of("{\"apikey\":\"old-key\"}"), auth.authData);
        }
    }

    @Test
    void restWrapperRefusesInvalidCredentials() {
        NativeLibrary.assumeAvailable();

        try (FugleRestClient client = FugleRestClient.builder().apiKey(OLD).build()) {
            assertConfigError(() -> client.setCredentials(Credentials.bearerToken("")));
            assertConfigError(() -> client.setCredentials(Credentials.apiKey("bad\nkey")));
            assertThrows(NullPointerException.class, () -> client.setCredentials(null));
            client.setCredentials(Credentials.sdkToken(NEW));
        }
    }

    @Test
    void credentialsToStringHidesTheSecret() {
        assertEquals("Credentials.sdkToken(***)", Credentials.sdkToken("secret").toString());
        assertEquals("Credentials.apiKey(***)", Credentials.apiKey("secret").toString());
    }

    @Test
    void restSendsTheNewCredentialFromTheNextRequest() throws Exception {
        NativeLibrary.assumeAvailable();

        List<Headers> headers = new CopyOnWriteArrayList<>();
        HttpServer server = HttpServer.create(new InetSocketAddress(InetAddress.getLoopbackAddress(), 0), 0);
        server.createContext("/", exchange -> {
            headers.add(exchange.getRequestHeaders());
            byte[] body = "{}".getBytes(StandardCharsets.UTF_8);
            exchange.getResponseHeaders().add("Content-Type", "application/json");
            exchange.sendResponseHeaders(200, body.length);
            try (OutputStream os = exchange.getResponseBody()) {
                os.write(body);
            }
        });
        server.start();
        // FugleRestClient.Builder does not apply baseUrl yet, so the request
        // goes through the generated client the wrapper delegates to.
        try (tw.com.fugle.marketdata.generated.RestClient client =
                     tw.com.fugle.marketdata.generated.MarketdataUniffi.newRestClientWithApiKeyAndTls(
                             OLD,
                             "http://127.0.0.1:" + server.getAddress().getPort(),
                             new tw.com.fugle.marketdata.generated.TlsConfigRecord(null, false))) {
            try (tw.com.fugle.marketdata.generated.StockClient stock = client.stock();
                 tw.com.fugle.marketdata.generated.StockIntradayClient intraday = stock.intraday()) {
                // Taken before the credential changes, and still sends the new one.
                client.setCredentials(new CredentialsRecord(null, null, NEW));
                intraday.quoteSync("2330", null);
            }

            assertEquals(1, headers.size());
            assertEquals(NEW, headers.get(0).getFirst("X-SDK-TOKEN"));
            assertNull(headers.get(0).getFirst("X-API-KEY"));
        } finally {
            server.stop(0);
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
