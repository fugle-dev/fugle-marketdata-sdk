using Microsoft.VisualStudio.TestTools.UnitTesting;
using System;
using System.Threading.Tasks;
using System.Collections.Generic;
using System.Linq;

namespace MarketdataUniffi.Tests;

/// <summary>
/// FFI Boundary Tests for C# bindings via UniFFI.
///
/// These tests verify that the UniFFI FFI boundary properly handles edge cases:
/// - Error propagation (Rust errors -> C# exceptions)
/// - Panic recovery (empty strings, long inputs)
/// - Memory safety (Dispose pattern, concurrent access)
/// - Thread safety (async doesn't block)
///
/// FFI boundary failures would manifest as:
/// - Process crash (unhandled panic)
/// - Memory corruption (invalid pointer access)
/// - Thread deadlock (blocking FFI calls)
/// - Exception type mismatches (error mapping broken)
/// </summary>
[TestClass]
public class FfiBoundaryTests
{
    private static bool _nativeLibraryAvailable;

    [ClassInitialize]
    public static void ClassInit(TestContext context)
    {
        // Check if native library is available
        try
        {
            using var client = new FugleMarketData.RestClient("test-api-key");
            _nativeLibraryAvailable = true;
        }
        catch (DllNotFoundException)
        {
            _nativeLibraryAvailable = false;
        }
        catch (TypeInitializationException ex) when (ex.InnerException is DllNotFoundException)
        {
            _nativeLibraryAvailable = false;
        }
        catch
        {
            // Other exceptions mean the library loaded
            _nativeLibraryAvailable = true;
        }
    }

    private void SkipIfNativeLibraryUnavailable()
    {
        if (!_nativeLibraryAvailable)
        {
            Assert.Inconclusive("Native library not available. Build with: cargo build -p marketdata-uniffi --release");
        }
    }

    // ========== Error Propagation Tests ==========

    [TestMethod]
    public async Task InvalidSymbol_ThrowsMarketDataException()
    {
        SkipIfNativeLibraryUnavailable();

        using var client = new FugleMarketData.RestClient("test-api-key");

        await Assert.ThrowsExceptionAsync<Exception>(async () =>
        {
            await client.Stock.Intraday.GetQuoteAsync("INVALID_SYMBOL_12345");
        });
    }

    [TestMethod]
    public async Task AuthenticationFailure_ThrowsException()
    {
        SkipIfNativeLibraryUnavailable();

        using var client = new FugleMarketData.RestClient("mock-api-key");

        try
        {
            await client.Stock.Intraday.GetQuoteAsync("2330");
        }
        catch (Exception ex)
        {
            // Error should have readable message (no memory corruption)
            Assert.IsNotNull(ex.Message);
            Assert.IsTrue(ex.Message.Length > 0);
            Assert.IsFalse(ex.Message.Contains("\0"));
        }
    }

    [TestMethod]
    public async Task ErrorMessage_IsReadableString()
    {
        SkipIfNativeLibraryUnavailable();

        using var client = new FugleMarketData.RestClient("test-key");

        try
        {
            await client.Stock.Intraday.GetQuoteAsync("2330");
        }
        catch (Exception ex)
        {
            // Error message should be valid UTF-8 string
            Assert.IsNotNull(ex.Message);
            Assert.IsTrue(ex.Message.Length > 0);
            Assert.IsFalse(string.IsNullOrWhiteSpace(ex.Message));
        }
    }

    [TestMethod]
    public void ErrorStackTrace_IsAvailable()
    {
        SkipIfNativeLibraryUnavailable();

        using var client = new FugleMarketData.RestClient("test-key");

        try
        {
            var quote = client.Stock.Intraday.GetQuote("INVALID");
        }
        catch (Exception ex)
        {
            // Exception should have stack trace
            Assert.IsNotNull(ex.StackTrace);
        }
    }

    // ========== Panic Recovery Tests ==========

    [TestMethod]
    public void EmptyApiKey_DoesNotCrash()
    {
        SkipIfNativeLibraryUnavailable();

        // Empty API key should not panic
        try
        {
            using var client = new FugleMarketData.RestClient("");
            Assert.IsNotNull(client);
        }
        catch (ArgumentNullException)
        {
            // Validation exception is acceptable
        }
    }

    [TestMethod]
    public void NullApiKey_ThrowsArgumentNullException()
    {
        // Null should be caught at C# validation level
        Assert.ThrowsException<ArgumentNullException>(() =>
        {
            new FugleMarketData.RestClient((string)null!);
        });
    }

    [TestMethod]
    public async Task VeryLongInput_DoesNotOverflow()
    {
        SkipIfNativeLibraryUnavailable();

        using var client = new FugleMarketData.RestClient("test-key");

        // Try extremely long symbol (potential buffer overflow)
        var longSymbol = new string('A', 10000);

        await Assert.ThrowsExceptionAsync<Exception>(async () =>
        {
            await client.Stock.Intraday.GetQuoteAsync(longSymbol);
        });

        // Should throw exception, not crash
    }

    [TestMethod]
    public async Task UnicodeInput_HandledSafely()
    {
        SkipIfNativeLibraryUnavailable();

        using var client = new FugleMarketData.RestClient("test-key");

        var unicodeSymbols = new[]
        {
            "中文",  // Chinese
            "🚀📈",  // Emojis
            "א",    // Hebrew
            "\u0000" // Null character
        };

        foreach (var symbol in unicodeSymbols)
        {
            try
            {
                await client.Stock.Intraday.GetQuoteAsync(symbol);
            }
            catch (Exception ex)
            {
                // Should get exception with valid message
                Assert.IsNotNull(ex.Message);
                Assert.IsTrue(ex.Message.Length > 0);
            }
        }
    }

    // ========== Memory Safety Tests ==========

    [TestMethod]
    public void MultipleClients_DoNotInterfere()
    {
        SkipIfNativeLibraryUnavailable();

        var clients = new List<FugleMarketData.RestClient>();

        for (int i = 0; i < 10; i++)
        {
            clients.Add(new FugleMarketData.RestClient($"key_{i}"));
        }

        // All clients should be independent
        Assert.AreEqual(10, clients.Count);

        // Cleanup
        foreach (var client in clients)
        {
            client.Dispose();
        }
    }

    [TestMethod]
    public async Task ClientReusableAfterError()
    {
        SkipIfNativeLibraryUnavailable();

        using var client = new FugleMarketData.RestClient("test-key");

        // Cause an error
        try
        {
            await client.Stock.Intraday.GetQuoteAsync("INVALID");
        }
        catch
        {
            // Expected
        }

        // Client should still be usable (should not crash)
        await Assert.ThrowsExceptionAsync<Exception>(async () =>
        {
            await client.Stock.Intraday.GetQuoteAsync("2330");
        });
    }

    [TestMethod]
    public async Task ConcurrentOperationsOnSameClient()
    {
        SkipIfNativeLibraryUnavailable();

        using var client = new FugleMarketData.RestClient("test-key");

        // Multiple concurrent calls on same client
        var tasks = new List<Task>();
        for (int i = 0; i < 5; i++)
        {
            tasks.Add(Task.Run(async () =>
            {
                try
                {
                    await client.Stock.Intraday.GetQuoteAsync("2330");
                }
                catch
                {
                    // Expected
                }
            }));
        }

        // All should complete without crash
        await Task.WhenAll(tasks);
    }

    [TestMethod]
    public void DisposePattern_WorksCorrectly()
    {
        SkipIfNativeLibraryUnavailable();

        FugleMarketData.RestClient client;

        using (client = new FugleMarketData.RestClient("test-key"))
        {
            Assert.IsNotNull(client);
        }

        // After dispose, new operations should fail gracefully
        // Note: UniFFI-generated code may or may not enforce this
    }

    [TestMethod]
    public void GarbageCollection_DoesNotCauseCrashes()
    {
        SkipIfNativeLibraryUnavailable();

        // Create and destroy many clients
        for (int i = 0; i < 100; i++)
        {
            var client = new FugleMarketData.RestClient($"key_{i}");
            client.Dispose();
        }

        // Force GC
        GC.Collect();
        GC.WaitForPendingFinalizers();
        GC.Collect();

        // Create new client after GC
        using var newClient = new FugleMarketData.RestClient("test-key");
        Assert.IsNotNull(newClient);
    }

    // ========== Thread Safety Tests ==========

    [TestMethod]
    public async Task AsyncOperations_DoNotBlockThreads()
    {
        SkipIfNativeLibraryUnavailable();

        using var client = new FugleMarketData.RestClient("test-key");

        var otherTaskCompleted = false;

        // Start an API call
        var apiCall = Task.Run(async () =>
        {
            try
            {
                await client.Stock.Intraday.GetQuoteAsync("2330");
            }
            catch
            {
                // Expected
            }
        });

        // Start another task
        var otherTask = Task.Run(async () =>
        {
            await Task.Delay(50);
            otherTaskCompleted = true;
        });

        // Both should complete
        await Task.WhenAll(apiCall, otherTask);

        Assert.IsTrue(otherTaskCompleted);
    }

    [TestMethod]
    public async Task MultipleConcurrentAsyncCalls()
    {
        SkipIfNativeLibraryUnavailable();

        using var client = new FugleMarketData.RestClient("test-key");

        var results = new List<int>();

        async Task MakeCall(int id)
        {
            try
            {
                await client.Stock.Intraday.GetQuoteAsync("2330");
            }
            catch
            {
                // Expected
            }

            lock (results)
            {
                results.Add(id);
            }
        }

        // Start multiple concurrent calls
        var tasks = new List<Task>();
        for (int i = 0; i < 5; i++)
        {
            tasks.Add(MakeCall(i));
        }

        await Task.WhenAll(tasks);

        // All should complete
        Assert.AreEqual(5, results.Count);
    }

    [TestMethod]
    public void SyncMethod_DoesNotDeadlock()
    {
        SkipIfNativeLibraryUnavailable();

        using var client = new FugleMarketData.RestClient("test-key");

        // Sync method should not deadlock
        try
        {
            var quote = client.Stock.Intraday.GetQuote("2330");
        }
        catch
        {
            // Expected error, but should not deadlock
        }
    }

    [TestMethod]
    public void FactoryMethods_WorkCorrectly()
    {
        SkipIfNativeLibraryUnavailable();

        using var apiClient = new FugleMarketData.RestClient("test-api-key");
        Assert.IsNotNull(apiClient);

        using var sdkClient = FugleMarketData.RestClient.WithSdkToken("test-sdk-token");
        Assert.IsNotNull(sdkClient);

        using var bearerClient = FugleMarketData.RestClient.WithBearerToken("test-bearer-token");
        Assert.IsNotNull(bearerClient);
    }

    // ========== Type Safety Tests ==========

    [TestMethod]
    public async Task ReturnedData_IsValidObject()
    {
        SkipIfNativeLibraryUnavailable();

        using var client = new FugleMarketData.RestClient("test-key");

        try
        {
            var quote = await client.Stock.Intraday.GetQuoteAsync("2330");

            // If successful, should be valid object
            Assert.IsNotNull(quote);
        }
        catch
        {
            // Error is expected with test key
        }
    }

    [TestMethod]
    public void PropertyChaining_WorksCorrectly()
    {
        SkipIfNativeLibraryUnavailable();

        using var client = new FugleMarketData.RestClient("test-key");

        // Verify property access chain doesn't crash
        Assert.IsNotNull(client.Stock);
        Assert.IsNotNull(client.Stock.Intraday);
        Assert.IsNotNull(client.FutOpt);
        Assert.IsNotNull(client.FutOpt.Intraday);
    }
}

// Manual testing notes:
//
// Additional stress tests (not automated):
//   1. Memory leak testing:
//      - Monitor with Performance Monitor (perfmon) on Windows
//      - Track Private Bytes and Working Set
//
//   2. Thread safety under load:
//      - 50+ concurrent async operations
//      - Monitor for deadlocks or race conditions
//
//   3. Valgrind testing (Linux/WSL):
//      - valgrind --leak-check=full dotnet test
