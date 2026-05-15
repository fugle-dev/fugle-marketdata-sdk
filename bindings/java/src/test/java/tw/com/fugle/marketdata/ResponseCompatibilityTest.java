package tw.com.fugle.marketdata;

import org.junit.jupiter.api.*;
import static org.junit.jupiter.api.Assertions.*;
import static org.junit.jupiter.api.Assumptions.*;

import java.lang.reflect.Method;
import java.lang.reflect.Field;

/**
 * Response Compatibility Tests
 *
 * Validates response object structure matches expected Fugle API format.
 * Uses reflection for structural tests (no native library required).
 */
public class ResponseCompatibilityTest {

    private static boolean nativeLibraryAvailable = false;

    @BeforeAll
    static void checkNativeLibrary() {
        try {
            try (FugleRestClient client = FugleRestClient.builder()
                    .apiKey("test-key")
                    .build()) {
                nativeLibraryAvailable = true;
            }
        } catch (UnsatisfiedLinkError | NoClassDefFoundError e) {
            nativeLibraryAvailable = false;
        } catch (Exception e) {
            nativeLibraryAvailable = true;
        }
    }

    private void assumeNativeLibraryAvailable() {
        assumeTrue(nativeLibraryAvailable, "Native library not available. Build with: cargo build -p marketdata-uniffi --release");
    }

    // ========== Quote Response Structure ==========

    @Test
    @DisplayName("Quote record has symbol field")
    void quoteHasSymbolField() {
        // Check if Quote record/class exists with symbol method
        try {
            Class<?> quoteClass = Class.forName("uniffi.marketdata_uniffi.Quote");
            Method symbolMethod = quoteClass.getMethod("symbol");
            assertNotNull(symbolMethod, "Quote should have symbol() method");
            assertEquals(String.class, symbolMethod.getReturnType());
        } catch (ClassNotFoundException e) {
            fail("Quote class not found in uniffi bindings");
        } catch (NoSuchMethodException e) {
            fail("Quote.symbol() method not found");
        }
    }

    @Test
    @DisplayName("Quote record has date field")
    void quoteHasDateField() {
        try {
            Class<?> quoteClass = Class.forName("uniffi.marketdata_uniffi.Quote");
            Method dateMethod = quoteClass.getMethod("date");
            assertNotNull(dateMethod, "Quote should have date() method");
            assertEquals(String.class, dateMethod.getReturnType());
        } catch (ClassNotFoundException e) {
            fail("Quote class not found");
        } catch (NoSuchMethodException e) {
            fail("Quote.date() method not found");
        }
    }

    @Test
    @DisplayName("Quote record has expected fields")
    void quoteHasExpectedFields() {
        try {
            Class<?> quoteClass = Class.forName("uniffi.marketdata_uniffi.Quote");

            // Required fields
            assertNotNull(quoteClass.getMethod("symbol"), "Quote should have symbol()");
            assertNotNull(quoteClass.getMethod("date"), "Quote should have date()");

            // Optional fields (should exist in type)
            assertNotNull(quoteClass.getMethod("name"), "Quote should have name()");
            assertNotNull(quoteClass.getMethod("exchange"), "Quote should have exchange()");
            assertNotNull(quoteClass.getMethod("market"), "Quote should have market()");
        } catch (ClassNotFoundException e) {
            fail("Quote class not found");
        } catch (NoSuchMethodException e) {
            fail("Quote missing expected method: " + e.getMessage());
        }
    }

    // ========== Ticker Response Structure ==========

    @Test
    @DisplayName("Ticker record has symbol field")
    void tickerHasSymbolField() {
        try {
            Class<?> tickerClass = Class.forName("uniffi.marketdata_uniffi.Ticker");
            Method symbolMethod = tickerClass.getMethod("symbol");
            assertNotNull(symbolMethod, "Ticker should have symbol() method");
            assertEquals(String.class, symbolMethod.getReturnType());
        } catch (ClassNotFoundException e) {
            fail("Ticker class not found");
        } catch (NoSuchMethodException e) {
            fail("Ticker.symbol() method not found");
        }
    }

    @Test
    @DisplayName("Ticker record has expected fields")
    void tickerHasExpectedFields() {
        try {
            Class<?> tickerClass = Class.forName("uniffi.marketdata_uniffi.Ticker");

            assertNotNull(tickerClass.getMethod("symbol"), "Ticker should have symbol()");
            assertNotNull(tickerClass.getMethod("date"), "Ticker should have date()");
            assertNotNull(tickerClass.getMethod("name"), "Ticker should have name()");
        } catch (ClassNotFoundException e) {
            fail("Ticker class not found");
        } catch (NoSuchMethodException e) {
            fail("Ticker missing expected method: " + e.getMessage());
        }
    }

    // ========== Trades Response Structure ==========

    @Test
    @DisplayName("TradesResponse record has expected fields")
    void tradesResponseHasExpectedFields() {
        try {
            Class<?> tradesClass = Class.forName("uniffi.marketdata_uniffi.TradesResponse");

            assertNotNull(tradesClass.getMethod("symbol"), "TradesResponse should have symbol()");
            assertNotNull(tradesClass.getMethod("date"), "TradesResponse should have date()");
            assertNotNull(tradesClass.getMethod("data"), "TradesResponse should have data()");
        } catch (ClassNotFoundException e) {
            fail("TradesResponse class not found");
        } catch (NoSuchMethodException e) {
            fail("TradesResponse missing expected method: " + e.getMessage());
        }
    }

    // ========== Historical Response Structure (Phase 7) ==========

    @Test
    @Tag("structural")
    @DisplayName("HistoricalCandlesResponse class exists")
    void historicalCandlesResponseExists() {
        try {
            Class<?> responseClass = Class.forName("uniffi.marketdata_uniffi.HistoricalCandlesResponse");
            assertNotNull(responseClass, "HistoricalCandlesResponse should exist");
            assertNotNull(responseClass.getMethod("symbol"), "HistoricalCandlesResponse should have symbol()");
            assertNotNull(responseClass.getMethod("data"), "HistoricalCandlesResponse should have data()");
        } catch (ClassNotFoundException e) {
            fail("HistoricalCandlesResponse class not found");
        } catch (NoSuchMethodException e) {
            fail("HistoricalCandlesResponse missing expected method: " + e.getMessage());
        }
    }

    @Test
    @Tag("structural")
    @DisplayName("StatsResponse class exists")
    void statsResponseExists() {
        try {
            Class<?> responseClass = Class.forName("uniffi.marketdata_uniffi.StatsResponse");
            assertNotNull(responseClass, "StatsResponse should exist");
            assertNotNull(responseClass.getMethod("symbol"), "StatsResponse should have symbol()");
            assertNotNull(responseClass.getMethod("date"), "StatsResponse should have date()");
            assertNotNull(responseClass.getMethod("name"), "StatsResponse should have name()");
        } catch (ClassNotFoundException e) {
            fail("StatsResponse class not found");
        } catch (NoSuchMethodException e) {
            fail("StatsResponse missing expected method: " + e.getMessage());
        }
    }

    // ========== Snapshot Response Structure (Phase 7) ==========

    @Test
    @Tag("structural")
    @DisplayName("SnapshotQuotesResponse class exists")
    void snapshotQuotesResponseExists() {
        try {
            Class<?> responseClass = Class.forName("uniffi.marketdata_uniffi.SnapshotQuotesResponse");
            assertNotNull(responseClass, "SnapshotQuotesResponse should exist");
            assertNotNull(responseClass.getMethod("date"), "SnapshotQuotesResponse should have date()");
            assertNotNull(responseClass.getMethod("time"), "SnapshotQuotesResponse should have time()");
            assertNotNull(responseClass.getMethod("market"), "SnapshotQuotesResponse should have market()");
            assertNotNull(responseClass.getMethod("data"), "SnapshotQuotesResponse should have data()");
        } catch (ClassNotFoundException e) {
            fail("SnapshotQuotesResponse class not found");
        } catch (NoSuchMethodException e) {
            fail("SnapshotQuotesResponse missing expected method: " + e.getMessage());
        }
    }

    @Test
    @Tag("structural")
    @DisplayName("MoversResponse class exists")
    void moversResponseExists() {
        try {
            Class<?> responseClass = Class.forName("uniffi.marketdata_uniffi.MoversResponse");
            assertNotNull(responseClass, "MoversResponse should exist");
            assertNotNull(responseClass.getMethod("date"), "MoversResponse should have date()");
            assertNotNull(responseClass.getMethod("data"), "MoversResponse should have data()");
        } catch (ClassNotFoundException e) {
            fail("MoversResponse class not found");
        } catch (NoSuchMethodException e) {
            fail("MoversResponse missing expected method: " + e.getMessage());
        }
    }

    @Test
    @Tag("structural")
    @DisplayName("ActivesResponse class exists")
    void activesResponseExists() {
        try {
            Class<?> responseClass = Class.forName("uniffi.marketdata_uniffi.ActivesResponse");
            assertNotNull(responseClass, "ActivesResponse should exist");
            assertNotNull(responseClass.getMethod("data"), "ActivesResponse should have data()");
        } catch (ClassNotFoundException e) {
            fail("ActivesResponse class not found");
        } catch (NoSuchMethodException e) {
            fail("ActivesResponse missing expected method: " + e.getMessage());
        }
    }

    // ========== Technical Indicator Response Structure (Phase 7) ==========

    @Test
    @Tag("structural")
    @DisplayName("SmaResponse class exists")
    void smaResponseExists() {
        try {
            Class<?> responseClass = Class.forName("uniffi.marketdata_uniffi.SmaResponse");
            assertNotNull(responseClass, "SmaResponse should exist");
            assertNotNull(responseClass.getMethod("symbol"), "SmaResponse should have symbol()");
            assertNotNull(responseClass.getMethod("data"), "SmaResponse should have data()");
        } catch (ClassNotFoundException e) {
            fail("SmaResponse class not found");
        } catch (NoSuchMethodException e) {
            fail("SmaResponse missing expected method: " + e.getMessage());
        }
    }

    @Test
    @Tag("structural")
    @DisplayName("RsiResponse class exists")
    void rsiResponseExists() {
        try {
            Class<?> responseClass = Class.forName("uniffi.marketdata_uniffi.RsiResponse");
            assertNotNull(responseClass, "RsiResponse should exist");
        } catch (ClassNotFoundException e) {
            fail("RsiResponse class not found");
        }
    }

    @Test
    @Tag("structural")
    @DisplayName("KdjResponse class exists")
    void kdjResponseExists() {
        try {
            Class<?> responseClass = Class.forName("uniffi.marketdata_uniffi.KdjResponse");
            assertNotNull(responseClass, "KdjResponse should exist");
        } catch (ClassNotFoundException e) {
            fail("KdjResponse class not found");
        }
    }

    @Test
    @Tag("structural")
    @DisplayName("MacdResponse class exists")
    void macdResponseExists() {
        try {
            Class<?> responseClass = Class.forName("uniffi.marketdata_uniffi.MacdResponse");
            assertNotNull(responseClass, "MacdResponse should exist");
        } catch (ClassNotFoundException e) {
            fail("MacdResponse class not found");
        }
    }

    @Test
    @Tag("structural")
    @DisplayName("BbResponse class exists")
    void bbResponseExists() {
        try {
            Class<?> responseClass = Class.forName("uniffi.marketdata_uniffi.BbResponse");
            assertNotNull(responseClass, "BbResponse should exist");
        } catch (ClassNotFoundException e) {
            fail("BbResponse class not found");
        }
    }

    // ========== Corporate Actions Response Structure (Phase 7) ==========

    @Test
    @Tag("structural")
    @DisplayName("CapitalChangesResponse class exists")
    void capitalChangesResponseExists() {
        try {
            Class<?> responseClass = Class.forName("uniffi.marketdata_uniffi.CapitalChangesResponse");
            assertNotNull(responseClass, "CapitalChangesResponse should exist");
            assertNotNull(responseClass.getMethod("data"), "CapitalChangesResponse should have data()");
        } catch (ClassNotFoundException e) {
            fail("CapitalChangesResponse class not found");
        } catch (NoSuchMethodException e) {
            fail("CapitalChangesResponse missing expected method: " + e.getMessage());
        }
    }

    @Test
    @Tag("structural")
    @DisplayName("DividendsResponse class exists")
    void dividendsResponseExists() {
        try {
            Class<?> responseClass = Class.forName("uniffi.marketdata_uniffi.DividendsResponse");
            assertNotNull(responseClass, "DividendsResponse should exist");
        } catch (ClassNotFoundException e) {
            fail("DividendsResponse class not found");
        }
    }

    @Test
    @Tag("structural")
    @DisplayName("ListingApplicantsResponse class exists")
    void listingApplicantsResponseExists() {
        try {
            Class<?> responseClass = Class.forName("uniffi.marketdata_uniffi.ListingApplicantsResponse");
            assertNotNull(responseClass, "ListingApplicantsResponse should exist");
        } catch (ClassNotFoundException e) {
            fail("ListingApplicantsResponse class not found");
        }
    }

    // ========== FutOpt Historical Response Structure (Phase 7) ==========

    @Test
    @Tag("structural")
    @DisplayName("FutOptHistoricalCandlesResponse class exists")
    void futOptHistoricalCandlesResponseExists() {
        try {
            Class<?> responseClass = Class.forName("uniffi.marketdata_uniffi.FutOptHistoricalCandlesResponse");
            assertNotNull(responseClass, "FutOptHistoricalCandlesResponse should exist");
            assertNotNull(responseClass.getMethod("symbol"), "FutOptHistoricalCandlesResponse should have symbol()");
            // FutOptHistoricalCandlesResponse uses candles() not data()
            assertNotNull(responseClass.getMethod("candles"), "FutOptHistoricalCandlesResponse should have candles()");
        } catch (ClassNotFoundException e) {
            fail("FutOptHistoricalCandlesResponse class not found");
        } catch (NoSuchMethodException e) {
            fail("FutOptHistoricalCandlesResponse missing expected method: " + e.getMessage());
        }
    }

    @Test
    @Tag("structural")
    @DisplayName("FutOptDailyResponse class exists")
    void futOptDailyResponseExists() {
        try {
            Class<?> responseClass = Class.forName("uniffi.marketdata_uniffi.FutOptDailyResponse");
            assertNotNull(responseClass, "FutOptDailyResponse should exist");
            assertNotNull(responseClass.getMethod("symbol"), "FutOptDailyResponse should have symbol()");
            assertNotNull(responseClass.getMethod("data"), "FutOptDailyResponse should have data()");
        } catch (ClassNotFoundException e) {
            fail("FutOptDailyResponse class not found");
        } catch (NoSuchMethodException e) {
            fail("FutOptDailyResponse missing expected method: " + e.getMessage());
        }
    }

    // ========== Integration Response Tests ==========

    @Test
    @Tag("integration")
    @DisplayName("Quote response has required fields (live)")
    void quoteResponseHasRequiredFields() {
        assumeNativeLibraryAvailable();

        String apiKey = System.getenv("FUGLE_API_KEY");
        assumeTrue(apiKey != null && !apiKey.isEmpty(), "FUGLE_API_KEY not set");

        try (FugleRestClient client = FugleRestClient.builder()
                .apiKey(apiKey)
                .build()) {

            var quote = client.stock().intraday().getQuote("2330");

            assertNotNull(quote);
            assertNotNull(quote.symbol(), "Quote must have symbol");
            assertEquals("2330", quote.symbol());
            assertNotNull(quote.date(), "Quote must have date");
            assertFalse(quote.date().isEmpty(), "Quote date should not be empty");
        }
    }

    @Test
    @Tag("integration")
    @DisplayName("Ticker response has required fields (live)")
    void tickerResponseHasRequiredFields() {
        assumeNativeLibraryAvailable();

        String apiKey = System.getenv("FUGLE_API_KEY");
        assumeTrue(apiKey != null && !apiKey.isEmpty(), "FUGLE_API_KEY not set");

        try (FugleRestClient client = FugleRestClient.builder()
                .apiKey(apiKey)
                .build()) {

            var ticker = client.stock().intraday().getTicker("2330");

            assertNotNull(ticker);
            assertNotNull(ticker.symbol(), "Ticker must have symbol");
            assertEquals("2330", ticker.symbol());
            assertNotNull(ticker.name(), "Ticker must have name");
            assertFalse(ticker.name().isEmpty(), "Ticker name should not be empty");
        }
    }

    @Test
    @Tag("integration")
    @DisplayName("Trades response has required fields (live)")
    void tradesResponseHasRequiredFields() {
        assumeNativeLibraryAvailable();

        String apiKey = System.getenv("FUGLE_API_KEY");
        assumeTrue(apiKey != null && !apiKey.isEmpty(), "FUGLE_API_KEY not set");

        try (FugleRestClient client = FugleRestClient.builder()
                .apiKey(apiKey)
                .build()) {

            var trades = client.stock().intraday().getTrades("2330");

            assertNotNull(trades);
            assertNotNull(trades.symbol(), "Trades must have symbol");
            assertEquals("2330", trades.symbol());
            assertNotNull(trades.date(), "Trades must have date");
            assertNotNull(trades.data(), "Trades must have data array");
        }
    }
}
