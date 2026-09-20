package tw.com.fugle.marketdata;

import org.junit.jupiter.api.*;
import static org.junit.jupiter.api.Assertions.*;

import tw.com.fugle.marketdata.generated.CorporateActionsParams;
import tw.com.fugle.marketdata.generated.OddLotParams;
import tw.com.fugle.marketdata.generated.OwnershipParams;
import tw.com.fugle.marketdata.generated.StockCandlesParams;
import tw.com.fugle.marketdata.generated.StockTradesParams;

import java.io.IOException;
import java.io.OutputStream;
import java.lang.reflect.Method;
import java.net.InetAddress;
import java.net.InetSocketAddress;
import java.util.Map;
import java.util.TreeMap;
import java.util.concurrent.CompletableFuture;

import com.sun.net.httpserver.HttpExchange;
import com.sun.net.httpserver.HttpServer;

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
        assertNotNull(FugleRestClient.StockIntradayClientWrapper.class.getMethod("getQuote", String.class, OddLotParams.class));
        assertNotNull(FugleRestClient.StockIntradayClientWrapper.class.getMethod("getTicker", String.class));
        assertNotNull(FugleRestClient.StockIntradayClientWrapper.class.getMethod("getTicker", String.class, OddLotParams.class));
        assertNotNull(FugleRestClient.StockIntradayClientWrapper.class.getMethod("getTrades", String.class));
        assertNotNull(FugleRestClient.StockIntradayClientWrapper.class.getMethod("getTrades", String.class, StockTradesParams.class));
        assertNotNull(FugleRestClient.StockIntradayClientWrapper.class.getMethod("getCandles", String.class));
        assertNotNull(FugleRestClient.StockIntradayClientWrapper.class.getMethod("getCandles", String.class, StockCandlesParams.class));
        assertNotNull(FugleRestClient.StockIntradayClientWrapper.class.getMethod("getVolumes", String.class));
        assertNotNull(FugleRestClient.StockIntradayClientWrapper.class.getMethod("getVolumes", String.class, OddLotParams.class));
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
        assertNotNull(FugleRestClient.FutOptIntradayClientWrapper.class.getMethod(
                "getQuote", String.class, tw.com.fugle.marketdata.generated.AfterHoursParams.class));
        assertNotNull(FugleRestClient.FutOptIntradayClientWrapper.class.getMethod("getTicker", String.class));
        assertNotNull(FugleRestClient.FutOptIntradayClientWrapper.class.getMethod(
                "getTicker", String.class, tw.com.fugle.marketdata.generated.AfterHoursParams.class));
        assertNotNull(FugleRestClient.FutOptIntradayClientWrapper.class.getMethod("getProducts", String.class));
        assertNotNull(FugleRestClient.FutOptIntradayClientWrapper.class.getMethod(
                "getProducts", String.class, tw.com.fugle.marketdata.generated.FutOptProductsParams.class));
    }

    @Test
    @DisplayName("FugleRestClient.FutOptIntradayClientWrapper has async methods")
    void intradayFutOptHasAsyncMethods() throws NoSuchMethodException {
        Method getQuoteAsync = FugleRestClient.FutOptIntradayClientWrapper.class.getMethod("getQuoteAsync", String.class);
        assertNotNull(getQuoteAsync);
        assertEquals(CompletableFuture.class, getQuoteAsync.getReturnType());

        Method getProductsAsync = FugleRestClient.FutOptIntradayClientWrapper.class.getMethod("getProductsAsync", String.class);
        assertNotNull(getProductsAsync);
        assertEquals(CompletableFuture.class, getProductsAsync.getReturnType());
    }

    @Test
    @DisplayName("FugleRestClient.StockClientWrapper exposes ownership()")
    void stockHasOwnershipMethod() throws NoSuchMethodException {
        Method method = FugleRestClient.StockClientWrapper.class.getMethod("ownership");
        assertEquals(FugleRestClient.StockOwnershipClientWrapper.class, method.getReturnType());
    }

    @Test
    @DisplayName("FugleRestClient.StockOwnershipClientWrapper has sync and async methods")
    void ownershipHasSyncAndAsyncMethods() throws NoSuchMethodException {
        Class<?> type = FugleRestClient.StockOwnershipClientWrapper.class;
        for (String name : new String[] {
                "getEtfHoldings", "getInstitutionalTrades", "getDirectorHoldings", "getTdccDistribution"}) {
            Method syncConvenience = type.getMethod(name, String.class);
            assertEquals(String.class, syncConvenience.getReturnType());

            Method sync = type.getMethod(name, String.class, OwnershipParams.class);
            assertEquals(String.class, sync.getReturnType());

            Method asyncConvenience = type.getMethod(name + "Async", String.class);
            assertEquals(CompletableFuture.class, asyncConvenience.getReturnType());

            Method async = type.getMethod(name + "Async", String.class, OwnershipParams.class);
            assertEquals(CompletableFuture.class, async.getReturnType());
        }
    }

    // ========== Constructor Tests (require native library) ==========

    @Test
    @DisplayName("Builder with apiKey succeeds")
    void builderWithApiKeySucceeds() {
        NativeLibrary.assumeAvailable();

        try (FugleRestClient client = FugleRestClient.builder()
                .apiKey("test-api-key")
                .build()) {
            assertNotNull(client);
        }
    }

    @Test
    @DisplayName("Builder without credentials throws exception")
    void builderWithoutCredentialsThrows() {
        NativeLibrary.assumeAvailable();

        FugleException e = assertThrows(FugleException.class, () ->
                FugleRestClient.builder().build()
        );
        assertEquals(Integer.valueOf(1004), e.getCode());
        assertTrue(e.getMessage().contains("exactly one non-empty credential"));
    }

    @Test
    @DisplayName("Builder rejects an empty apiKey with code 1004")
    void builderWithEmptyApiKeyIsRejected() {
        NativeLibrary.assumeAvailable();

        FugleException e = assertThrows(FugleException.class, () ->
                FugleRestClient.builder().apiKey("").build()
        );
        assertEquals(Integer.valueOf(1004), e.getCode());
    }

    @Test
    @DisplayName("Client stock() returns FugleRestClient.StockClientWrapper")
    void stockReturnsWrapper() {
        NativeLibrary.assumeAvailable();

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
        NativeLibrary.assumeAvailable();

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
        NativeLibrary.assumeAvailable();

        try (FugleRestClient client = FugleRestClient.builder()
                .apiKey("test-api-key")
                .build()) {
            assertNotNull(client.stock().intraday());
            assertInstanceOf(FugleRestClient.StockIntradayClientWrapper.class, client.stock().intraday());
        }
    }

    @Test
    @DisplayName("StockClient ownership() returns FugleRestClient.StockOwnershipClientWrapper")
    void ownershipReturnsWrapper() {
        NativeLibrary.assumeAvailable();

        try (FugleRestClient client = FugleRestClient.builder()
                .apiKey("test-api-key")
                .build()) {
            assertNotNull(client.stock().ownership());
            assertInstanceOf(FugleRestClient.StockOwnershipClientWrapper.class, client.stock().ownership());
        }
    }

    // ========== Query Parameter Tests (loopback, no network) ==========
    //
    // FugleRestClient.Builder cannot set a base URL yet (see TODO in build()),
    // so these go through the generated RestClient directly via
    // MarketdataUniffi.newRestClientWithApiKeyAndTls, which the wrapper's
    // builder uses internally once baseUrl() support lands.

    @Test
    @DisplayName("getTrades(symbol, StockTradesParams) sends oddLot+limit as query parameters")
    void getTradesSendsQueryParameters() throws Exception {
        NativeLibrary.assumeAvailable();

        java.util.concurrent.BlockingQueue<String> rawQueries = new java.util.concurrent.LinkedBlockingQueue<>();
        HttpServer server = HttpServer.create(new InetSocketAddress(InetAddress.getLoopbackAddress(), 0), 0);
        server.createContext("/", exchange -> {
            rawQueries.add(exchange.getRequestURI().getRawQuery());
            byte[] body = "{}".getBytes(java.nio.charset.StandardCharsets.UTF_8);
            exchange.getResponseHeaders().add("Content-Type", "application/json");
            exchange.sendResponseHeaders(200, body.length);
            try (OutputStream os = exchange.getResponseBody()) {
                os.write(body);
            }
        });
        server.start();
        try {
            tw.com.fugle.marketdata.generated.RestClient client =
                    tw.com.fugle.marketdata.generated.MarketdataUniffi.newRestClientWithApiKeyAndTls(
                            "test-api-key",
                            "http://127.0.0.1:" + server.getAddress().getPort(),
                            new tw.com.fugle.marketdata.generated.TlsConfigRecord(null, false));

            client.stock().intraday().tradesSync("2330", new StockTradesParams(true, null, 5, null, null));

            String rawQuery = rawQueries.poll(10, java.util.concurrent.TimeUnit.SECONDS);
            assertNotNull(rawQuery, "server did not receive a request");
            assertEquals(queryPairs("type=oddlot&limit=5"), queryPairs(rawQuery));
        } finally {
            server.stop(0);
        }
    }

    @Test
    @DisplayName("getCapitalChanges with exchange fails with code 1005 (unknown key)")
    void capitalChangesWithExchangeIs1005() {
        NativeLibrary.assumeAvailable();

        // Validation happens before the request is sent, so an unreachable
        // baseUrl (port 9, "discard") is enough - no server needed.
        tw.com.fugle.marketdata.generated.RestClient client;
        try {
            client = tw.com.fugle.marketdata.generated.MarketdataUniffi.newRestClientWithApiKeyAndTls(
                    "test-api-key", "http://127.0.0.1:9",
                    new tw.com.fugle.marketdata.generated.TlsConfigRecord(null, false));
        } catch (tw.com.fugle.marketdata.generated.MarketDataException e) {
            throw new AssertionError(e);
        }

        tw.com.fugle.marketdata.generated.MarketDataException e = assertThrows(
                tw.com.fugle.marketdata.generated.MarketDataException.class,
                () -> client.stock().corporateActions().capitalChangesSync(
                        new CorporateActionsParams(null, null, "TWSE", null)));
        assertEquals(Integer.valueOf(1005), FugleException.from(e).getCode());
    }

    /** Parse a raw query string ("a=b&c=d") into an order-independent set of pairs. */
    private static Map<String, String> queryPairs(String rawQuery) {
        Map<String, String> pairs = new TreeMap<>();
        if (rawQuery == null || rawQuery.isEmpty()) {
            return pairs;
        }
        for (String pair : rawQuery.split("&")) {
            String[] kv = pair.split("=", 2);
            pairs.put(kv[0], kv.length > 1 ? kv[1] : "");
        }
        return pairs;
    }

    // ========== Integration Tests (require FUGLE_API_KEY) ==========

    @Test
    @Tag("integration")
    @DisplayName("getQuoteAsync with valid API key returns quote")
    void getQuoteAsyncWithValidKey() throws Exception {
        NativeLibrary.assumeAvailable();

        String apiKey = System.getenv("FUGLE_API_KEY");
        Assumptions.assumeTrue(apiKey != null && !apiKey.isEmpty(),
                "FUGLE_API_KEY environment variable not set");

        try (FugleRestClient client = FugleRestClient.builder()
                .apiKey(apiKey)
                .build()) {

            var quote = client.stock().intraday().getQuoteAsync("2330").get();
            assertNotNull(quote);
            assertTrue(quote.matches("(?s).*\"symbol\"\\s*:\\s*\"2330\".*"), quote);
        }
    }

    @Test
    @Tag("integration")
    @DisplayName("getQuote (sync) with valid API key returns quote")
    void getQuoteWithValidKey() {
        NativeLibrary.assumeAvailable();

        String apiKey = System.getenv("FUGLE_API_KEY");
        Assumptions.assumeTrue(apiKey != null && !apiKey.isEmpty(),
                "FUGLE_API_KEY environment variable not set");

        try (FugleRestClient client = FugleRestClient.builder()
                .apiKey(apiKey)
                .build()) {

            var quote = client.stock().intraday().getQuote("2330");
            assertNotNull(quote);
            assertTrue(quote.matches("(?s).*\"symbol\"\\s*:\\s*\"2330\".*"), quote);
        }
    }

    @Test
    @Tag("integration")
    @DisplayName("getTickerAsync with valid API key returns ticker")
    void getTickerAsyncWithValidKey() throws Exception {
        NativeLibrary.assumeAvailable();

        String apiKey = System.getenv("FUGLE_API_KEY");
        Assumptions.assumeTrue(apiKey != null && !apiKey.isEmpty(),
                "FUGLE_API_KEY environment variable not set");

        try (FugleRestClient client = FugleRestClient.builder()
                .apiKey(apiKey)
                .build()) {

            var ticker = client.stock().intraday().getTickerAsync("2330").get();
            assertNotNull(ticker);
            assertTrue(ticker.matches("(?s).*\"symbol\"\\s*:\\s*\"2330\".*"), ticker);
        }
    }
}
