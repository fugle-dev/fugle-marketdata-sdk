package tw.com.fugle.marketdata;

import org.junit.jupiter.api.Assertions;
import org.junit.jupiter.api.Assumptions;

/**
 * Detects whether the UniFFI native library can be loaded, so tests that
 * construct clients skip instead of failing where it is not built.
 */
final class NativeLibrary {

    private static Boolean available;

    private NativeLibrary() {}

    static synchronized boolean isAvailable() {
        if (available == null) {
            try (FugleRestClient client = FugleRestClient.builder()
                    .apiKey("test-api-key")
                    .build()) {
                available = true;
            } catch (UnsatisfiedLinkError | NoClassDefFoundError e) {
                available = false;
            } catch (Exception e) {
                // Other exceptions mean the library loaded but client creation failed
                available = true;
            }
        }
        return available;
    }

    static void assumeAvailable() {
        String message = "Native library not available. Build with: cargo build -p marketdata-uniffi --release";
        // Set by -PrequireNative (CI): a missing library is a failure, not a skip.
        if (Boolean.getBoolean("fugle.requireNative")) {
            Assertions.assertTrue(isAvailable(), message);
        } else {
            Assumptions.assumeTrue(isAvailable(), message);
        }
    }
}
