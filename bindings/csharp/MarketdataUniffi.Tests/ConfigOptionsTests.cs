using Microsoft.VisualStudio.TestTools.UnitTesting;
using System;

namespace MarketdataUniffi.Tests;

/// <summary>
/// Tests for RestClientOptions and WebSocketClientOptions configuration classes.
/// Verifies exactly-one-auth validation, options construction, and error handling.
/// </summary>
[TestClass]
public class ConfigOptionsTests
{
    private static bool _nativeLibraryAvailable;

    [ClassInitialize]
    public static void ClassInit(TestContext context)
    {
        // Check if native library is available by attempting to load it
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
            // Other exceptions mean the library loaded but failed validation
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

    // ========== RestClientOptions Tests ==========

    [TestMethod]
    public void RestClientOptions_ExactlyOneAuth_ApiKey_Succeeds()
    {
        SkipIfNativeLibraryUnavailable();

        var options = new FugleMarketData.RestClientOptions { ApiKey = "test-api-key" };

        try
        {
            using var client = new FugleMarketData.RestClient(options);
            Assert.IsNotNull(client);
            // If we get here, auth validation passed (UniFFI may still fail, but that's OK)
        }
        catch (ArgumentException ex) when (ex.Message.Contains("Provide exactly one of"))
        {
            Assert.Fail("Should not throw ArgumentException for single auth method");
        }
        catch
        {
            // Other exceptions (UniFFI errors) are acceptable - we're testing auth validation
        }
    }

    [TestMethod]
    public void RestClientOptions_ExactlyOneAuth_BearerToken_Succeeds()
    {
        SkipIfNativeLibraryUnavailable();

        var options = new FugleMarketData.RestClientOptions { BearerToken = "test-bearer-token" };

        try
        {
            using var client = new FugleMarketData.RestClient(options);
            Assert.IsNotNull(client);
        }
        catch (ArgumentException ex) when (ex.Message.Contains("Provide exactly one of"))
        {
            Assert.Fail("Should not throw ArgumentException for single auth method");
        }
        catch
        {
            // Other exceptions are acceptable
        }
    }

    [TestMethod]
    public void RestClientOptions_ExactlyOneAuth_SdkToken_Succeeds()
    {
        SkipIfNativeLibraryUnavailable();

        var options = new FugleMarketData.RestClientOptions { SdkToken = "test-sdk-token" };

        try
        {
            using var client = new FugleMarketData.RestClient(options);
            Assert.IsNotNull(client);
        }
        catch (ArgumentException ex) when (ex.Message.Contains("Provide exactly one of"))
        {
            Assert.Fail("Should not throw ArgumentException for single auth method");
        }
        catch
        {
            // Other exceptions are acceptable
        }
    }

    [TestMethod]
    public void RestClientOptions_NoAuth_ThrowsArgumentException()
    {
        var options = new FugleMarketData.RestClientOptions();

        var ex = Assert.ThrowsException<ArgumentException>(() =>
            new FugleMarketData.RestClient(options)
        );

        Assert.IsTrue(ex.Message.Contains("Provide exactly one of"));
        Assert.IsTrue(ex.Message.Contains("ApiKey"));
        Assert.IsTrue(ex.Message.Contains("BearerToken"));
        Assert.IsTrue(ex.Message.Contains("SdkToken"));
    }

    [TestMethod]
    public void RestClientOptions_MultipleAuth_ThrowsArgumentException()
    {
        var options = new FugleMarketData.RestClientOptions
        {
            ApiKey = "test-api-key",
            BearerToken = "test-bearer-token"
        };

        var ex = Assert.ThrowsException<ArgumentException>(() =>
            new FugleMarketData.RestClient(options)
        );

        Assert.IsTrue(ex.Message.Contains("Provide exactly one of"));
    }

    [TestMethod]
    public void RestClientOptions_NullOptions_ThrowsArgumentNullException()
    {
        Assert.ThrowsException<ArgumentNullException>(() =>
            new FugleMarketData.RestClient((FugleMarketData.RestClientOptions)null!)
        );
    }

    // ========== WebSocketClientOptions Tests ==========

    [TestMethod]
    public void WebSocketClientOptions_NoAuth_ThrowsArgumentException()
    {
        var options = new FugleMarketData.WebSocketClientOptions();
        var listener = new TestWebSocketListener();

        var ex = Assert.ThrowsException<ArgumentException>(() =>
            new FugleMarketData.WebSocketClient(options, listener)
        );

        Assert.IsTrue(ex.Message.Contains("Provide exactly one of"));
        Assert.IsTrue(ex.Message.Contains("ApiKey"));
        Assert.IsTrue(ex.Message.Contains("BearerToken"));
        Assert.IsTrue(ex.Message.Contains("SdkToken"));
    }

    [TestMethod]
    public void WebSocketClientOptions_MultipleAuth_ThrowsArgumentException()
    {
        var options = new FugleMarketData.WebSocketClientOptions
        {
            ApiKey = "test-api-key",
            SdkToken = "test-sdk-token"
        };
        var listener = new TestWebSocketListener();

        var ex = Assert.ThrowsException<ArgumentException>(() =>
            new FugleMarketData.WebSocketClient(options, listener)
        );

        Assert.IsTrue(ex.Message.Contains("Provide exactly one of"));
    }

    // ========== ReconnectOptions Tests ==========

    [TestMethod]
    public void ReconnectOptions_DefaultValues_AreNull()
    {
        var options = new FugleMarketData.ReconnectOptions();

        Assert.IsNull(options.MaxAttempts);
        Assert.IsNull(options.InitialDelayMs);
        Assert.IsNull(options.MaxDelayMs);
    }

    [TestMethod]
    public void ReconnectOptions_CustomValues_AreStored()
    {
        var options = new FugleMarketData.ReconnectOptions
        {
            MaxAttempts = 10,
            InitialDelayMs = 2000,
            MaxDelayMs = 120000
        };

        Assert.AreEqual(10u, options.MaxAttempts);
        Assert.AreEqual(2000ul, options.InitialDelayMs);
        Assert.AreEqual(120000ul, options.MaxDelayMs);
    }

    // ========== HealthCheckOptions Tests ==========

    [TestMethod]
    public void HealthCheckOptions_DefaultValues_AreNull()
    {
        var options = new FugleMarketData.HealthCheckOptions();

        Assert.IsNull(options.Enabled);
        Assert.IsNull(options.IntervalMs);
        Assert.IsNull(options.MaxMissedPongs);
    }

    [TestMethod]
    public void HealthCheckOptions_CustomValues_AreStored()
    {
        var options = new FugleMarketData.HealthCheckOptions
        {
            Enabled = true,
            IntervalMs = 20000,
            MaxMissedPongs = 3
        };

        Assert.AreEqual(true, options.Enabled);
        Assert.AreEqual(20000ul, options.IntervalMs);
        Assert.AreEqual(3ul, options.MaxMissedPongs);
    }

    // ========== WebSocketClientOptions with nested config Tests ==========

    [TestMethod]
    public void WebSocketClientOptions_AcceptsReconnectOptions()
    {
        SkipIfNativeLibraryUnavailable();

        var options = new FugleMarketData.WebSocketClientOptions
        {
            ApiKey = "test-api-key",
            Reconnect = new FugleMarketData.ReconnectOptions
            {
                MaxAttempts = 3,
                InitialDelayMs = 500,
                MaxDelayMs = 30000
            }
        };
        var listener = new TestWebSocketListener();

        try
        {
            using var client = new FugleMarketData.WebSocketClient(options, listener);
            Assert.IsNotNull(client);
            // Reconnect options stored for future use
        }
        catch (ArgumentException ex) when (ex.Message.Contains("Provide exactly one of"))
        {
            Assert.Fail("Should not throw ArgumentException for valid auth and reconnect config");
        }
        catch
        {
            // Other exceptions (UniFFI errors) are acceptable
        }
    }

    [TestMethod]
    public void WebSocketClientOptions_AcceptsHealthCheckOptions()
    {
        SkipIfNativeLibraryUnavailable();

        var options = new FugleMarketData.WebSocketClientOptions
        {
            ApiKey = "test-api-key",
            HealthCheck = new FugleMarketData.HealthCheckOptions
            {
                Enabled = true,
                IntervalMs = 15000,
                MaxMissedPongs = 2
            }
        };
        var listener = new TestWebSocketListener();

        try
        {
            using var client = new FugleMarketData.WebSocketClient(options, listener);
            Assert.IsNotNull(client);
            // Health check options stored for future use
        }
        catch (ArgumentException ex) when (ex.Message.Contains("Provide exactly one of"))
        {
            Assert.Fail("Should not throw ArgumentException for valid auth and health check config");
        }
        catch
        {
            // Other exceptions (UniFFI errors) are acceptable
        }
    }

    // ========== Helper Classes ==========

    private class TestWebSocketListener : FugleMarketData.IWebSocketListener
    {
        public void OnConnected() { }
        public void OnDisconnected() { }
        public void OnMessage(uniffi.marketdata_uniffi.StreamMessage message) { }
        public void OnError(string errorMessage) { }
        public void OnReconnecting(uint attempt) { }
        public void OnReconnectFailed(uint attempts) { }
    }
}
