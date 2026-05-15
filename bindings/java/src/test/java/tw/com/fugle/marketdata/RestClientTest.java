package tw.com.fugle.marketdata;

import org.junit.jupiter.api.*;
import static org.junit.jupiter.api.Assertions.*;

import java.lang.reflect.Method;
import java.util.concurrent.CompletableFuture;

/**
 * Tests for FugleRestClient wrapper over UniFFI bindings.
 *
 * <p>Structural tests verify type existence and API shape using reflection.
 * These tests pass without native library.
 *
 * <p>Integration tests (tagged with @Tag("integration")) require:
 * - Native library built and accessible
 * - FUGLE_API_KEY environment variable set
 */
public class RestClientTest {

    private static boolean nativeLibraryAvailable = false;

    @BeforeAll
    static void checkNativeLibrary() {
        try {
            // Attempt to create a client to check if native library is available
            try (FugleRestClient client = FugleRestClient.builder()
                    .apiKey("test-api-key")
                    .build()) {
                nativeLibraryAvailable = true;
            }
        } catch (UnsatisfiedLinkError | NoClassDefFoundError e) {
            // Native library not available
            nativeLibraryAvailable = false;
        } catch (Exception e) {
            // Other exceptions mean library loaded but failed validation
            nativeLibraryAvailable = true;
        }
    }

    private void assumeNativeLibraryAvailable() {
        Assumptions.assumeTrue(nativeLibraryAvailable,
                "Native library not available. Build with: cargo build -p marketdata-uniffi --release");
    }

    // ========== Structural Tests (Type Existence) ==========

    @Test
    @DisplayName("FugleRestClient type exists and implements AutoCloseable")
    void restClientTypeExists() {
        assertNotNull(FugleRestClient.class);
        assertTrue(AutoCloseable.class.isAssignableFrom(FugleRestClient.class));
    }

    @Test
    @DisplayName("FugleRestClient.Builder type exists")
    void builderTypeExists() {
        assertNotNull(FugleRestClient.Builder.class);
    }

    @Test
    @DisplayName("FugleRestClient.StockClientWrapper type exists")
    void stockClientWrapperTypeExists() {
        assertNotNull(FugleRestClient.StockClientWrapper.class);
    }

    @Test
    @DisplayName("FugleRestClient.FutOptClientWrapper type exists")
    void futOptClientWrapperTypeExists() {
        assertNotNull(FugleRestClient.FutOptClientWrapper.class);
    }

    @Test
    @DisplayName("FugleRestClient.StockIntradayClientWrapper type exists")
    void stockIntradayClientWrapperTypeExists() {
        assertNotNull(FugleRestClient.StockIntradayClientWrapper.class);
    }

    @Test
    @DisplayName("FugleRestClient.FutOptIntradayClientWrapper type exists")
    void futOptIntradayClientWrapperTypeExists() {
        assertNotNull(FugleRestClient.FutOptIntradayClientWrapper.class);
    }

    // ========== API Shape Tests ==========

    @Test
    @DisplayName("FugleRestClient has builder() method")
    void hasBuilderMethod() throws NoSuchMethodException {
        Method method = FugleRestClient.class.getMethod("builder");
        assertNotNull(method);
        assertEquals(FugleRestClient.Builder.class, method.getReturnType());
    }

    @Test
    @DisplayName("FugleRestClient has stock() method")
    void hasStockMethod() throws NoSuchMethodException {
        Method method = FugleRestClient.class.getMethod("stock");
        assertNotNull(method);
        assertEquals(FugleRestClient.StockClientWrapper.class, method.getReturnType());
    }

    @Test
    @DisplayName("FugleRestClient has futopt() method")
    void hasFutOptMethod() throws NoSuchMethodException {
        Method method = FugleRestClient.class.getMethod("futopt");
        assertNotNull(method);
        assertEquals(FugleRestClient.FutOptClientWrapper.class, method.getReturnType());
    }

    @Test
    @DisplayName("Builder has apiKey() method")
    void builderHasApiKeyMethod() throws NoSuchMethodException {
        Method method = FugleRestClient.Builder.class.getMethod("apiKey", String.class);
        assertNotNull(method);
        assertEquals(FugleRestClient.Builder.class, method.getReturnType());
    }

    @Test
    @DisplayName("Builder has bearerToken() method")
    void builderHasBearerTokenMethod() throws NoSuchMethodException {
        Method method = FugleRestClient.Builder.class.getMethod("bearerToken", String.class);
        assertNotNull(method);
        assertEquals(FugleRestClient.Builder.class, method.getReturnType());
    }

    @Test
    @DisplayName("Builder has sdkToken() method")
    void builderHasSdkTokenMethod() throws NoSuchMethodException {
        Method method = FugleRestClient.Builder.class.getMethod("sdkToken", String.class);
        assertNotNull(method);
        assertEquals(FugleRestClient.Builder.class, method.getReturnType());
    }

    @Test
    @DisplayName("FugleRestClient.StockIntradayClientWrapper has sync methods")
    void intradayStockHasSyncMethods() throws NoSuchMethodException {
        assertNotNull(FugleRestClient.StockIntradayClientWrapper.class.getMethod("getQuote", String.class));
        assertNotNull(FugleRestClient.StockIntradayClientWrapper.class.getMethod("getTicker", String.class));
        assertNotNull(FugleRestClient.StockIntradayClientWrapper.class.getMethod("getTrades", String.class));
        assertNotNull(FugleRestClient.StockIntradayClientWrapper.class.getMethod("getCandles", String.class, String.class));
        assertNotNull(FugleRestClient.StockIntradayClientWrapper.class.getMethod("getVolumes", String.class));
    }

    @Test
    @DisplayName("FugleRestClient.StockIntradayClientWrapper has async methods")
    void intradayStockHasAsyncMethods() throws NoSuchMethodException {
        Method getQuoteAsync = FugleRestClient.StockIntradayClientWrapper.class.getMethod("getQuoteAsync", String.class);
        assertNotNull(getQuoteAsync);
        assertEquals(CompletableFuture.class, getQuoteAsync.getReturnType());

        Method getTickerAsync = FugleRestClient.StockIntradayClientWrapper.class.getMethod("getTickerAsync", String.class);
        assertNotNull(getTickerAsync);
        assertEquals(CompletableFuture.class, getTickerAsync.getReturnType());
    }

    @Test
    @DisplayName("FugleRestClient.FutOptIntradayClientWrapper has sync methods")
    void intradayFutOptHasSyncMethods() throws NoSuchMethodException {
        assertNotNull(FugleRestClient.FutOptIntradayClientWrapper.class.getMethod("getQuote", String.class));
        assertNotNull(FugleRestClient.FutOptIntradayClientWrapper.class.getMethod("getTicker", String.class));
        assertNotNull(FugleRestClient.FutOptIntradayClientWrapper.class.getMethod("getProducts"));
    }

    @Test
    @DisplayName("FugleRestClient.FutOptIntradayClientWrapper has async methods")
    void intradayFutOptHasAsyncMethods() throws NoSuchMethodException {
        Method getQuoteAsync = FugleRestClient.FutOptIntradayClientWrapper.class.getMethod("getQuoteAsync", String.class);
        assertNotNull(getQuoteAsync);
        assertEquals(CompletableFuture.class, getQuoteAsync.getReturnType());

        Method getProductsAsync = FugleRestClient.FutOptIntradayClientWrapper.class.getMethod("getProductsAsync");
        assertNotNull(getProductsAsync);
        assertEquals(CompletableFuture.class, getProductsAsync.getReturnType());
    }

    // ========== Constructor Tests (require native library) ==========

    @Test
    @DisplayName("Builder with apiKey succeeds")
    void builderWithApiKeySucceeds() {
        assumeNativeLibraryAvailable();

        try (FugleRestClient client = FugleRestClient.builder()
                .apiKey("test-api-key")
                .build()) {
            assertNotNull(client);
        }
    }

    @Test
    @DisplayName("Builder without credentials throws exception")
    void builderWithoutCredentialsThrows() {
        assumeNativeLibraryAvailable();

        assertThrows(IllegalStateException.class, () ->
                FugleRestClient.builder().build()
        );
    }

    @Test
    @DisplayName("Builder with empty apiKey throws exception")
    void builderWithEmptyApiKeyThrows() {
        assumeNativeLibraryAvailable();

        assertThrows(IllegalStateException.class, () ->
                FugleRestClient.builder().apiKey("").build()
        );
    }

    @Test
    @DisplayName("Client stock() returns FugleRestClient.StockClientWrapper")
    void stockReturnsWrapper() {
        assumeNativeLibraryAvailable();

        try (FugleRestClient client = FugleRestClient.builder()
                .apiKey("test-api-key")
                .build()) {
            assertNotNull(client.stock());
            assertInstanceOf(FugleRestClient.StockClientWrapper.class, client.stock());
        }
    }

    @Test
    @DisplayName("Client futopt() returns FugleRestClient.FutOptClientWrapper")
    void futOptReturnsWrapper() {
        assumeNativeLibraryAvailable();

        try (FugleRestClient client = FugleRestClient.builder()
                .apiKey("test-api-key")
                .build()) {
            assertNotNull(client.futopt());
            assertInstanceOf(FugleRestClient.FutOptClientWrapper.class, client.futopt());
        }
    }

    @Test
    @DisplayName("StockClient intraday() returns FugleRestClient.StockIntradayClientWrapper")
    void intradayReturnsWrapper() {
        assumeNativeLibraryAvailable();

        try (FugleRestClient client = FugleRestClient.builder()
                .apiKey("test-api-key")
                .build()) {
            assertNotNull(client.stock().intraday());
            assertInstanceOf(FugleRestClient.StockIntradayClientWrapper.class, client.stock().intraday());
        }
    }

    // ========== Integration Tests (require FUGLE_API_KEY) ==========

    @Test
    @Tag("integration")
    @DisplayName("getQuoteAsync with valid API key returns quote")
    void getQuoteAsyncWithValidKey() throws Exception {
        assumeNativeLibraryAvailable();

        String apiKey = System.getenv("FUGLE_API_KEY");
        Assumptions.assumeTrue(apiKey != null && !apiKey.isEmpty(),
                "FUGLE_API_KEY environment variable not set");

        try (FugleRestClient client = FugleRestClient.builder()
                .apiKey(apiKey)
                .build()) {

            var quote = client.stock().intraday().getQuoteAsync("2330").get();
            assertNotNull(quote);
            assertEquals("2330", quote.symbol());
        }
    }

    @Test
    @Tag("integration")
    @DisplayName("getQuote (sync) with valid API key returns quote")
    void getQuoteWithValidKey() {
        assumeNativeLibraryAvailable();

        String apiKey = System.getenv("FUGLE_API_KEY");
        Assumptions.assumeTrue(apiKey != null && !apiKey.isEmpty(),
                "FUGLE_API_KEY environment variable not set");

        try (FugleRestClient client = FugleRestClient.builder()
                .apiKey(apiKey)
                .build()) {

            var quote = client.stock().intraday().getQuote("2330");
            assertNotNull(quote);
            assertEquals("2330", quote.symbol());
        }
    }

    @Test
    @Tag("integration")
    @DisplayName("getTickerAsync with valid API key returns ticker")
    void getTickerAsyncWithValidKey() throws Exception {
        assumeNativeLibraryAvailable();

        String apiKey = System.getenv("FUGLE_API_KEY");
        Assumptions.assumeTrue(apiKey != null && !apiKey.isEmpty(),
                "FUGLE_API_KEY environment variable not set");

        try (FugleRestClient client = FugleRestClient.builder()
                .apiKey(apiKey)
                .build()) {

            var ticker = client.stock().intraday().getTickerAsync("2330").get();
            assertNotNull(ticker);
            assertEquals("2330", ticker.symbol());
        }
    }
}
