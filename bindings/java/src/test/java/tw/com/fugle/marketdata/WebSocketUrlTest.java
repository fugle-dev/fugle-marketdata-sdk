package tw.com.fugle.marketdata;

import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.*;

/**
 * {@code url()} is the endpoint core resolved from the base URL, the version
 * and the product, readable before connecting (#245).
 */
public class WebSocketUrlTest {

    @Test
    void urlDefaultsToTheProductionEndpointOfEachProduct() {
        NativeLibrary.assumeAvailable();

        try (FugleWebSocketClient stock = FugleWebSocketClient.builder().apiKey("the-key").stock().build();
             FugleWebSocketClient futopt = FugleWebSocketClient.builder().apiKey("the-key").futopt().build()) {
            assertEquals("wss://api.fugle.tw/marketdata/v1.0/stock/streaming", stock.url());
            assertEquals("wss://api.fugle.tw/marketdata/v1.1/futopt/streaming", futopt.url());
        }
    }

    @Test
    void urlReflectsBaseUrl() {
        NativeLibrary.assumeAvailable();

        try (FugleWebSocketClient client = FugleWebSocketClient.builder()
                .apiKey("the-key")
                .futopt()
                .baseUrl("wss://staging.fugle.tw/marketdata")
                .build()) {
            assertEquals("wss://staging.fugle.tw/marketdata/v1.1/futopt/streaming", client.url());
        }
    }

    @Test
    void urlWithVersionedBaseUrlThrowsConfigError() {
        NativeLibrary.assumeAvailable();

        try (FugleWebSocketClient client = FugleWebSocketClient.builder()
                .apiKey("the-key")
                .baseUrl("wss://staging.fugle.tw/marketdata/v1.0")
                .build()) {
            FugleException e = assertThrows(FugleException.class, client::url);
            assertEquals(Integer.valueOf(1004), e.getCode());
        }
    }
}
