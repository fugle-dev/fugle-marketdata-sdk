package tw.com.fugle.marketdata;

import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.Arguments;
import org.junit.jupiter.params.provider.MethodSource;
import static org.junit.jupiter.api.Assertions.*;

import java.util.List;
import java.util.concurrent.CopyOnWriteArrayList;
import java.util.concurrent.TimeUnit;
import java.util.function.UnaryOperator;
import java.util.regex.Matcher;
import java.util.regex.Pattern;
import java.util.stream.Stream;

/**
 * The auth frame carries each credential in its own field (#91): the server
 * reads {@code apikey}, {@code token} or {@code sdkToken} and rejects a frame
 * with more than one. Bearer and SDK tokens used to be refused outright.
 */
public class WebSocketAuthFrameTest {

    private static final Pattern AUTH_DATA = Pattern.compile("^\\{\"event\":\"auth\",\"data\":(\\{.*\\})\\}$");

    static Stream<Arguments> credentials() {
        return Stream.of(
            Arguments.of("apiKey", (UnaryOperator<FugleWebSocketClient.Builder>) b -> b.apiKey("the-key"),
                "{\"apikey\":\"the-key\"}"),
            Arguments.of("bearerToken", (UnaryOperator<FugleWebSocketClient.Builder>) b -> b.bearerToken("the-token"),
                "{\"token\":\"the-token\"}"),
            Arguments.of("sdkToken", (UnaryOperator<FugleWebSocketClient.Builder>) b -> b.sdkToken("the-sdk-token"),
                "{\"sdkToken\":\"the-sdk-token\"}")
        );
    }

    @ParameterizedTest(name = "{0} is sent in its own field")
    @MethodSource("credentials")
    void connectSendsCredentialInItsField(
            String name, UnaryOperator<FugleWebSocketClient.Builder> credential, String expected) throws Exception {
        NativeLibrary.assumeAvailable();

        List<String> authData = new CopyOnWriteArrayList<>();
        try (LoopbackWsServer server = new LoopbackWsServer(text -> {
                 Matcher m = AUTH_DATA.matcher(text);
                 if (!m.matches()) {
                     return List.of();
                 }
                 authData.add(m.group(1));
                 return List.of("{\"event\":\"authenticated\",\"data\":{}}");
             });
             FugleWebSocketClient client = credential.apply(FugleWebSocketClient.builder())
                     .stock()
                     .baseUrl(server.url())
                     .build()) {
            client.connect().get(10, TimeUnit.SECONDS);
            client.disconnect().get(10, TimeUnit.SECONDS);

            assertEquals(List.of(expected), authData);
        }
    }
}
